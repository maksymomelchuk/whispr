use crate::config::{self, Replacement};
use crate::recorder::AudioFormat;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const DEEPGRAM_WS_BASE: &str = "wss://api.deepgram.com/v1/listen";

/// How long to wait for Deepgram to flush remaining `is_final` results after
/// we send `CloseStream`. The server typically responds within a few hundred
/// ms; we cap it so a hung WS never blocks the paste indefinitely.
const FINAL_RESULTS_TIMEOUT: Duration = Duration::from_secs(3);

const TRANSCRIPT_PARTIAL_EVENT: &str = "transcript-partial";
/// Bounds the overlay rerender rate — Deepgram interims arrive faster than
/// React can usefully repaint, and `compose_preview` is non-trivial.
const PARTIAL_THROTTLE: Duration = Duration::from_millis(100);

/// Open a Deepgram Live WebSocket, forward `chunks` as PCM frames until the
/// channel closes (recorder torn down by PTT release), then ask Deepgram for
/// final results and return the concatenated raw transcript. The returned
/// `Duration` is the time from session start to chunk channel close — the
/// user-perceived speaking duration, excluding the post-close final-results
/// drain. Replacements are applied by the caller so each pipeline stage is
/// observable for the history trace.
pub async fn run(
    app: AppHandle,
    format: AudioFormat,
    mut chunks: UnboundedReceiver<Vec<i16>>,
) -> Result<(String, Duration), String> {
    let speak_start = Instant::now();
    let settings = config::load(&app);
    let key = settings
        .api_key
        .clone()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;
    let show_live_preview = settings.show_live_preview;
    let replacements = settings.replacements.clone();

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
    let mut current_interim: String = String::new();
    let mut last_emitted: String = String::new();
    let mut last_emit: Option<Instant> = None;

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
                        if let Some((is_final, piece)) = extract_transcript_message(&t) {
                            if is_final {
                                if !piece.is_empty() {
                                    transcript_pieces.push(piece);
                                }
                                current_interim.clear();
                            } else {
                                current_interim = piece;
                            }
                            if show_live_preview {
                                let now = Instant::now();
                                let throttled = last_emit
                                    .is_some_and(|prev| now.duration_since(prev) < PARTIAL_THROTTLE);
                                if !throttled {
                                    let preview = compose_preview(
                                        &transcript_pieces,
                                        &current_interim,
                                        &replacements,
                                    );
                                    if preview != last_emitted {
                                        let _ = app.emit(TRANSCRIPT_PARTIAL_EVENT, &preview);
                                        last_emit = Some(now);
                                        last_emitted = preview;
                                    }
                                }
                            }
                        }
                    }
                    Message::Close(_) => return Err("Deepgram WS closed mid-stream".into()),
                    _ => {}
                }
            }
        }
    }
    let speak_duration = speak_start.elapsed();

    // Phase 2: ask Deepgram to flush, then drain remaining finals with a
    // bounded timeout so a stuck server can't block the paste. Partial
    // emission is deliberately skipped — the overlay holds the last
    // preview until we have the final.
    let close_msg = serde_json::json!({"type": "CloseStream"}).to_string();
    if let Err(e) = sink.send(Message::Text(close_msg)).await {
        eprintln!("[stream] CloseStream send failed: {e}");
    }

    let _ = tokio::time::timeout(FINAL_RESULTS_TIMEOUT, async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    if let Some((true, piece)) = extract_transcript_message(&t) {
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
    let raw = raw.trim().to_string();

    if show_live_preview && !raw.is_empty() {
        let final_preview = apply_replacements(&raw, &replacements);
        if final_preview != last_emitted {
            let _ = app.emit(TRANSCRIPT_PARTIAL_EVENT, &final_preview);
        }
    }

    Ok((raw, speak_duration))
}

fn compose_preview(
    finals: &[String],
    interim: &str,
    replacements: &[Replacement],
) -> String {
    let mut preview = finals.join(" ");
    if !interim.is_empty() {
        if !preview.is_empty() {
            preview.push(' ');
        }
        preview.push_str(interim);
    }
    apply_replacements(&preview, replacements)
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

fn extract_transcript_message(text: &str) -> Option<(bool, String)> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("type").and_then(|x| x.as_str()) != Some("Results") {
        return None;
    }
    let is_final = v.get("is_final").and_then(|x| x.as_bool()).unwrap_or(false);
    let t = v["channel"]["alternatives"][0]["transcript"]
        .as_str()?
        .trim();
    Some((is_final, t.to_string()))
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
/// pad the whole transcript with spaces on both ends, then search for `from`
/// flanked by non-word characters — so Deepgram's terminal punctuation
/// ("Design skill.", "design skill,") doesn't kill the right-hand boundary
/// the way a literal " from " search would. The replacement is spliced in
/// with surrounding spaces and collapsed by the phase-2 punctuation passes
/// below. The outer loop re-runs replacements until stable so chains like
/// "dash dash help" fully resolve to "--help".
pub fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    if replacements.is_empty() {
        return text.to_string();
    }

    let mut padded = format!(" {} ", text);
    let froms_lc: Vec<String> = replacements
        .iter()
        .map(|r| r.from.to_lowercase())
        .collect();

    loop {
        let lower = padded.to_lowercase();
        let mut changed = false;
        for (r, from_lc) in replacements.iter().zip(froms_lc.iter()) {
            if from_lc.is_empty() {
                continue;
            }
            if let Some((start, end)) = find_word_match(&lower, from_lc) {
                let replacement = format!(" {} ", r.to);
                padded.replace_range(start..end, &replacement);
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

/// Chars that count as part of a word for replacement boundary purposes.
/// Hyphen/underscore/apostrophe are included so a rule like "well" doesn't
/// match inside "well-being" or "don't".
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-' || c == '_'
}

/// Locate `needle` inside `haystack` as a whole word — flanked by non-word
/// characters or the string boundary. Both inputs are expected to already be
/// lowercased; byte offsets are returned for direct use with
/// `replace_range` on a same-length original-case string (ASCII-safe).
fn find_word_match(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let left_ok = haystack[..start]
            .chars()
            .next_back()
            .map_or(true, |c| !is_word_char(c));
        let right_ok = haystack[end..]
            .chars()
            .next()
            .map_or(true, |c| !is_word_char(c));
        if left_ok && right_ok {
            return Some((start, end));
        }
        from = start + needle.chars().next().map_or(1, |c| c.len_utf8());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(from: &str, to: &str) -> Replacement {
        Replacement {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn matches_with_trailing_period() {
        let out = apply_replacements(
            "Let me improve my design skill.",
            &[r("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Let me improve my /emil-design-engineering.");
    }

    #[test]
    fn matches_with_trailing_comma() {
        let out = apply_replacements(
            "I rely on my design skill, every day.",
            &[r("design skill", "/emil-design-engineering")],
        );
        assert_eq!(
            out,
            "I rely on my /emil-design-engineering, every day."
        );
    }

    #[test]
    fn matches_with_trailing_question_mark() {
        let out = apply_replacements(
            "Want to use my design skill?",
            &[r("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Want to use my /emil-design-engineering?");
    }

    #[test]
    fn does_not_match_inside_word() {
        let out = apply_replacements("I love design skills", &[r("design skill", "X")]);
        assert_eq!(out, "I love design skills");
    }

    #[test]
    fn does_not_match_with_hyphen_boundary() {
        let out = apply_replacements("well-being matters", &[r("well", "good")]);
        assert_eq!(out, "well-being matters");
    }

    #[test]
    fn case_insensitive_match() {
        let out = apply_replacements("Design Skill rules.", &[r("design skill", "X")]);
        assert_eq!(out, "X rules.");
    }
}
