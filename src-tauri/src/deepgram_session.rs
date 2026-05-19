use crate::config::{self, CorrectionEntry};
use crate::corrections::{apply_corrections, compose_corrections};
use crate::mode::{Mode, ModeLanguage};
use crate::recorder::AudioFormat;
use crate::transcription_session::TranscriptionSession;
use crate::groq_audio::{self, AUDIO_LEVEL_EVENT, TRANSCRIPT_PARTIAL_EVENT};
use crate::terms;
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

/// Bounds the overlay rerender rate — Deepgram interims arrive faster than
/// React can usefully repaint, and `compose_preview` is non-trivial.
const PARTIAL_THROTTLE: Duration = Duration::from_millis(100);
/// 30 Hz is smooth enough for the wave; cpal callbacks fire 2–4× faster on
/// most input configs and would otherwise flood the IPC channel.
const LEVEL_THROTTLE: Duration = Duration::from_millis(33);

pub struct DeepgramSession;

impl TranscriptionSession for DeepgramSession {
    async fn run(
        self,
        app: AppHandle,
        format: AudioFormat,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        language: ModeLanguage,
        terms: Vec<String>,
        mode: &Mode,
    ) -> Result<(String, Duration), String> {
        let speak_start = Instant::now();
        let settings = config::load(&app);
        // Prefer the new per-provider key; fall back to the legacy single-key
        // field for the brief window before `load`'s migration has re-saved.
        let key = settings
            .deepgram_api_key
            .clone()
            .or_else(|| settings.api_key.clone())
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "API key not configured".to_string())?;
        let show_live_preview = settings.show_live_preview;
        let corrections = if mode.use_corrections {
            compose_corrections(&mode.correction_set_ids, &settings.correction_sets)
        } else {
            Vec::new()
        };

        let url = build_ws_url(&language, format, &terms)?;
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
        let mut smoothed_level: f32 = 0.0;
        let mut last_level_emit: Option<Instant> = None;

        // Phase 1: forward audio while it's still flowing. Process server
        // messages opportunistically so the WS receive buffer doesn't pile up.
        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => {
                    match maybe_chunk {
                        Some(chunk) => {
                            let raw_level = groq_audio::compute_level(&chunk);
                            if let Err(e) = sink.send(Message::Binary(pcm_bytes(&chunk))).await {
                                return Err(format!("Deepgram WS send failed: {e}"));
                            }
                            // Asymmetric EMA: fast attack so vowels punch, slow
                            // decay so the wave doesn't snap to silent between
                            // syllables.
                            let k = if raw_level > smoothed_level { 0.6 } else { 0.25 };
                            smoothed_level = smoothed_level + (raw_level - smoothed_level) * k;
                            let now = Instant::now();
                            if last_level_emit
                                .map_or(true, |t| now.duration_since(t) >= LEVEL_THROTTLE)
                            {
                                let _ = app.emit(AUDIO_LEVEL_EVENT, smoothed_level);
                                last_level_emit = Some(now);
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
                                            &corrections,
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
        // Settle the wave to flat — the pill stays up through "thinking" and
        // we don't want it dancing on the last cached level.
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);

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
            let final_preview = apply_corrections(&raw, &corrections);
            if final_preview != last_emitted {
                let _ = app.emit(TRANSCRIPT_PARTIAL_EVENT, &final_preview);
            }
        }

        Ok((raw, speak_duration))
    }
}

fn compose_preview(
    finals: &[String],
    interim: &str,
    corrections: &[CorrectionEntry],
) -> String {
    let mut preview = finals.join(" ");
    if !interim.is_empty() {
        if !preview.is_empty() {
            preview.push(' ');
        }
        preview.push_str(interim);
    }
    apply_corrections(&preview, corrections)
}

fn build_ws_url(
    language: &ModeLanguage,
    format: AudioFormat,
    terms: &[String],
) -> Result<Url, String> {
    let mut url = Url::parse(DEEPGRAM_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("model", "nova-3");
        match language {
            ModeLanguage::Auto => {}
            ModeLanguage::Exact { code } => {
                q.append_pair("language", code);
            }
            // Deepgram's multi-language detection mode; individual codes are
            // informational only (Deepgram doesn't expose per-language hint params yet).
            ModeLanguage::Hints { .. } => {
                q.append_pair("language", "multi");
            }
        }
        q.append_pair("encoding", "linear16");
        q.append_pair("sample_rate", &format.sample_rate.to_string());
        q.append_pair("channels", &format.channels.to_string());
        q.append_pair("smart_format", "true");
        q.append_pair("numerals", "true");
    }
    // Append terms as keyterms, staying within the 4 KB total-URL ceiling.
    // Budget is computed after all static params.
    let remaining =
        terms::DEEPGRAM_KEYTERM_BUDGET_BYTES.saturating_sub(url.as_str().len());
    {
        let mut q = url.query_pairs_mut();
        for term in terms::deepgram_keyterms(terms, remaining) {
            q.append_pair("keyterm", &term);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::AudioFormat;

    fn fmt() -> AudioFormat {
        AudioFormat {
            sample_rate: 16000,
            channels: 1,
        }
    }

    #[test]
    fn build_ws_url_hardcodes_smart_format_and_numerals() {
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(query.contains("smart_format=true"), "URL must contain smart_format=true");
        assert!(query.contains("numerals=true"), "URL must contain numerals=true");
        assert!(!query.contains("dictation"), "URL must not contain dictation");
        assert!(!query.contains("keyterm"), "URL must not contain keyterm");
    }

    #[test]
    fn url_omits_language_for_auto() {
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(!q.contains("language="), "Auto must not send language param: {q}");
    }

    #[test]
    fn url_sends_exact_language_code() {
        let url = build_ws_url(&ModeLanguage::exact("uk"), fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(q.contains("language=uk"), "Exact must send language=uk: {q}");
    }

    #[test]
    fn url_sends_multi_for_hints() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let url = build_ws_url(&lang, fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(q.contains("language=multi"), "Hints must send language=multi: {q}");
    }

    #[test]
    fn url_includes_terms_as_keyterms() {
        let terms: Vec<String> = vec!["MongoDB".into(), "TypeScript".into()];
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(q.contains("keyterm=MongoDB"), "missing MongoDB: {q}");
        assert!(q.contains("keyterm=TypeScript"), "missing TypeScript: {q}");
    }

    #[test]
    fn url_has_no_keyterms_for_empty_terms() {
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(!q.contains("keyterm="), "unexpected keyterm: {q}");
    }

    #[test]
    fn url_includes_keyterms_with_hints_language() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let terms: Vec<String> = vec!["MongoDB".into()];
        let url = build_ws_url(&lang, fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(q.contains("language=multi"), "Hints must send language=multi: {q}");
        assert!(q.contains("keyterm=MongoDB"), "keyterm missing for Hints: {q}");
    }

    #[test]
    fn url_includes_keyterms_with_auto_language() {
        let terms: Vec<String> = vec!["Kubernetes".into()];
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(!q.contains("language="), "Auto must not send language param: {q}");
        assert!(q.contains("keyterm=Kubernetes"), "keyterm missing for Auto: {q}");
    }

    #[test]
    fn url_respects_keyterm_budget() {
        let terms: Vec<String> = (0..200)
            .map(|i| format!("term{i:05}"))
            .collect();
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &terms).unwrap();
        assert!(
            url.as_str().len() <= terms::DEEPGRAM_KEYTERM_BUDGET_BYTES,
            "URL too long: {} bytes",
            url.as_str().len()
        );
    }
}
