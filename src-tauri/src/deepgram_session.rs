use crate::config::{self, Replacement};
use crate::mode::ModeLanguage;
use crate::recorder::AudioFormat;
use crate::replacements::apply_replacements;
use crate::transcription_session::TranscriptionSession;
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
const AUDIO_LEVEL_EVENT: &str = "audio-level";
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
        let replacements = settings.replacements.clone();

        let mode_language = config::get_default_mode(&settings).language.clone();
        let url = build_ws_url(&settings.deepgram, &mode_language, format)?;
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
                            let raw_level = compute_level(&chunk);
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
            let final_preview = apply_replacements(&raw, &replacements);
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

fn build_ws_url(
    dg: &config::DeepgramSettings,
    language: &ModeLanguage,
    format: AudioFormat,
) -> Result<Url, String> {
    let mut url = Url::parse(DEEPGRAM_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("model", "nova-3");
        // Auto omits the language parameter; Exact sends the ISO code.
        if let Some(code) = language.as_code() {
            q.append_pair("language", code);
        }
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

/// dB range maps perceived loudness — linear RMS leaves quiet mics barely
/// visible. FLOOR_DB clamps room tone to zero so the bars stay flat in
/// silence.
fn compute_level(chunk: &[i16]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = chunk.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_sq / chunk.len() as f64).sqrt() / i16::MAX as f64;
    if rms <= 0.0 {
        return 0.0;
    }
    const FLOOR_DB: f64 = -40.0;
    const CEIL_DB: f64 = -10.0;
    let db = 20.0 * rms.log10();
    let n = ((db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0);
    n as f32
}

fn pcm_bytes(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    bytes
}
