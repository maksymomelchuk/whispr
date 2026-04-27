use crate::config::{self, Replacement};
use crate::recorder::AudioFormat;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const DEEPGRAM_WS_BASE: &str = "wss://api.deepgram.com/v1/listen";

/// How long to wait for Deepgram to flush remaining `is_final` results after
/// we send `CloseStream`. The server typically responds within a few hundred
/// ms; we cap it so a hung WS never blocks the paste indefinitely.
const FINAL_RESULTS_TIMEOUT: Duration = Duration::from_secs(3);

/// Open a Deepgram Live WebSocket, forward `chunks` as PCM frames until the
/// channel closes (recorder torn down by PTT release), then ask Deepgram for
/// final results and return the concatenated transcript with replacements
/// applied.
pub async fn run(
    app: AppHandle,
    format: AudioFormat,
    mut chunks: UnboundedReceiver<Vec<i16>>,
) -> Result<String, String> {
    let settings = config::load(&app);
    let key = settings
        .api_key
        .clone()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;

    let url = build_ws_url(&settings.deepgram, format)?;
    let mut req = url
        .as_str()
        .into_client_request()
        .map_err(|e| format!("bad WS URL: {e}"))?;
    req.headers_mut().insert(
        "Authorization",
        format!("Token {key}")
            .parse()
            .map_err(|e| format!("bad auth header: {e}"))?,
    );

    let (ws, _resp) = tokio_tungstenite::connect_async(req)
        .await
        .map_err(|e| format!("Deepgram WS connect failed: {e}"))?;
    let (mut sink, mut stream) = ws.split();

    let mut transcript_pieces: Vec<String> = Vec::new();

    // Phase 1: forward audio while it's still flowing. Process server
    // messages opportunistically so the WS receive buffer doesn't pile up.
    loop {
        tokio::select! {
            maybe_chunk = chunks.recv() => {
                match maybe_chunk {
                    Some(chunk) => {
                        if let Err(e) = sink.send(Message::Binary(pcm_bytes(&chunk))).await {
                            return Err(format!("Deepgram WS send failed: {e}"));
                        }
                    }
                    None => break, // recorder torn down → end of audio
                }
            }
            msg = stream.next() => {
                let Some(msg) = msg else { return Err("Deepgram WS closed mid-stream".into()); };
                let msg = msg.map_err(|e| format!("Deepgram WS recv failed: {e}"))?;
                match msg {
                    Message::Text(t) => {
                        if let Some(piece) = extract_final_transcript(&t) {
                            if !piece.is_empty() {
                                transcript_pieces.push(piece);
                            }
                        }
                    }
                    Message::Close(_) => return Err("Deepgram WS closed mid-stream".into()),
                    _ => {}
                }
            }
        }
    }

    // Phase 2: ask Deepgram to flush, then drain remaining finals with a
    // bounded timeout so a stuck server can't block the paste.
    let close_msg = serde_json::json!({"type": "CloseStream"}).to_string();
    if let Err(e) = sink.send(Message::Text(close_msg)).await {
        eprintln!("[stream] CloseStream send failed: {e}");
    }

    let _ = tokio::time::timeout(FINAL_RESULTS_TIMEOUT, async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    if let Some(piece) = extract_final_transcript(&t) {
                        if !piece.is_empty() {
                            transcript_pieces.push(piece);
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    eprintln!("[stream] post-close recv error: {e}");
                    break;
                }
                _ => {}
            }
        }
    })
    .await;

    let raw = transcript_pieces.join(" ");
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let replaced = apply_replacements(trimmed, &settings.replacements);
    Ok(format!("{replaced} "))
}

fn build_ws_url(dg: &config::DeepgramSettings, format: AudioFormat) -> Result<Url, String> {
    let mut url = Url::parse(DEEPGRAM_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    let language = if dg.language.trim().is_empty() {
        "en"
    } else {
        dg.language.trim()
    };
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("model", "nova-3");
        q.append_pair("language", language);
        q.append_pair("encoding", "linear16");
        q.append_pair("sample_rate", &format.sample_rate.to_string());
        q.append_pair("channels", &format.channels.to_string());
        if dg.smart_format {
            q.append_pair("smart_format", "true");
        }
        if dg.dictation {
            q.append_pair("dictation", "true");
        }
        if dg.numerals {
            q.append_pair("numerals", "true");
        }
        for kt in &dg.keyterms {
            let trimmed = kt.trim();
            if !trimmed.is_empty() {
                q.append_pair("keyterm", trimmed);
            }
        }
    }
    Ok(url)
}

/// Pull the transcript out of a Deepgram Live `Results` message, but only
/// when `is_final` is true. Interim results would otherwise stack on top of
/// the final ones, duplicating speech.
fn extract_final_transcript(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("type").and_then(|x| x.as_str()) != Some("Results") {
        return None;
    }
    if !v.get("is_final").and_then(|x| x.as_bool()).unwrap_or(false) {
        return None;
    }
    let t = v["channel"]["alternatives"][0]["transcript"]
        .as_str()?
        .trim();
    Some(t.to_string())
}

fn pcm_bytes(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    bytes
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
