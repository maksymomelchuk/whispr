use crate::config::{self, DeepgramSettings, Replacement};
use tauri::AppHandle;

const DEEPGRAM_ENDPOINT: &str = "https://api.deepgram.com/v1/listen";

fn build_query(dg: &DeepgramSettings) -> Vec<(&'static str, String)> {
    let language = if dg.language.trim().is_empty() {
        "en".to_string()
    } else {
        dg.language.trim().to_string()
    };
    let mut q: Vec<(&'static str, String)> = vec![
        ("model", "nova-3".into()),
        ("language", language),
    ];
    if dg.smart_format {
        q.push(("smart_format", "true".into()));
    }
    if dg.dictation {
        q.push(("dictation", "true".into()));
    }
    if dg.numerals {
        q.push(("numerals", "true".into()));
    }
    for kt in &dg.keyterms {
        let trimmed = kt.trim();
        if !trimmed.is_empty() {
            q.push(("keyterm", trimmed.to_string()));
        }
    }
    q
}

/// Punctuation whose replacement should glue to both neighbors with no spaces.
/// Example: "test dot ts" → "test.ts".
const COMPACT: &[char] = &['.', '/', '-', '_', '@'];
/// Punctuation whose replacement should lose the leading space but keep a
/// trailing one. Example: "hello comma world" → "hello, world".
const CLING_LEFT: &[char] = &[',', ';', ':', '?', '!'];

/// Case-insensitive whole-word replacement with a small spacing policy. We
/// pad the whole transcript with spaces on both ends, then look for " from "
/// so we only match full tokens (never substrings of other words), replace
/// with a spaced version of `to`, and finally collapse spacing for compact
/// and cling-left punctuation. The outer loop re-runs replacements until
/// stable so chains like "dash dash help" fully resolve to "--help".
fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    if replacements.is_empty() {
        return text.to_string();
    }

    let mut padded = format!(" {} ", text);

    // Phase 1: whole-word substitution. Keep spaces around `to` for now so
    // subsequent matches retain their boundaries.
    loop {
        let mut changed = false;
        for r in replacements {
            if r.from.is_empty() {
                continue;
            }
            let lower = padded.to_lowercase();
            let needle = format!(" {} ", r.from.to_lowercase());
            if let Some(pos) = lower.find(&needle) {
                let end = pos + needle.len();
                let replacement = format!(" {} ", r.to);
                padded.replace_range(pos..end, &replacement);
                changed = true;
                // Restart the scan from the top — replacement may have
                // exposed a new match earlier in the string.
                break;
            }
        }
        if !changed {
            break;
        }
    }

    // Phase 2: compact / cling-left spacing for punctuation.
    for &c in COMPACT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        padded = padded.replace(&middle, &c.to_string());
        padded = padded.replace(&tail, &c.to_string());
    }
    for &c in CLING_LEFT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        padded = padded.replace(&middle, &format!("{c} "));
        padded = padded.replace(&tail, &c.to_string());
    }

    // Collapse any runs of spaces that survived the passes above.
    padded.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub async fn transcribe(app: AppHandle, audio: Vec<u8>) -> Result<String, String> {
    println!("[transcribe] invoked, audio bytes={}", audio.len());
    if audio.is_empty() {
        println!("[transcribe] empty audio, returning early");
        return Ok(String::new());
    }

    let settings = config::load(&app);
    let key = settings
        .api_key
        .filter(|k| !k.is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;

    let query = build_query(&settings.deepgram);
    println!("[transcribe] query params: {query:?}");

    let client = reqwest::Client::new();
    let res = client
        .post(DEEPGRAM_ENDPOINT)
        .query(&query)
        .header("Authorization", format!("Token {key}"))
        .header("Content-Type", "audio/wav")
        .body(audio)
        .send()
        .await
        .map_err(|e| format!("Deepgram request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Deepgram API error {status}: {body}"));
    }

    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    let transcript = json["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .trim();

    println!("[transcribe] got transcript len={}", transcript.len());

    if transcript.is_empty() {
        return Ok(String::new());
    }

    let replaced = apply_replacements(transcript, &settings.replacements);
    println!("[transcribe] after replacements: {replaced:?}");

    // Trailing space so back-to-back dictations concatenate cleanly.
    Ok(format!("{replaced} "))
}
