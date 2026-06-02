use crate::audio_level_meter::AudioLevelMeter;
use crate::config;
use crate::corrections::compose_corrections;
use crate::groq_audio::{to_pcm_16k_mono_bytes, AUDIO_LEVEL_EVENT, TRANSCRIPT_PARTIAL_EVENT};
use crate::mode::{Mode, ModeLanguage};
use crate::preview_throttle::PreviewThrottle;
use crate::provider::AssemblyAiModel;
use crate::recorder::AudioFormat;
use crate::terms;
use crate::transcription_session::TranscriptionSession;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const ASSEMBLYAI_WS_BASE: &str = "wss://streaming.assemblyai.com/v3/ws";
/// Cap on how long to wait for AssemblyAI to flush remaining turns after
/// Terminate is sent. The server typically responds within a few hundred ms.
const TERMINATION_TIMEOUT: Duration = Duration::from_secs(3);
/// AssemblyAI rejects sends outside 50–1000 ms. 50 ms at 16 kHz mono 16-bit
/// = 800 samples * 2 bytes = 1600 bytes.
const MIN_SEND_BYTES: usize = 1600;

pub struct AssemblyAiSession {
    pub model: AssemblyAiModel,
}

impl TranscriptionSession for AssemblyAiSession {
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
        let key = settings
            .assemblyai_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "API key missing for AssemblyAI".to_string())?;
        let model = self.model;
        let show_live_preview = settings.show_live_preview;
        let corrections = if mode.use_corrections {
            compose_corrections(&mode.correction_set_ids, &settings.correction_sets)
        } else {
            Vec::new()
        };

        if let ModeLanguage::Exact { code } = &language {
            if !model.supports_language(code) {
                return Err(format!(
                    "AssemblyAI model '{}' does not support language '{}'",
                    model.api_id(),
                    code
                ));
            }
        }

        let url = build_ws_url(model, &language, &terms)?;
        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("bad WS URL: {e}"))?;
        req.headers_mut().insert(
            "Authorization",
            key.parse().map_err(|e| format!("bad auth header: {e}"))?,
        );

        let (ws, _) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("AssemblyAI WS connect failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();

        let mut completed_turns: Vec<String> = Vec::new();
        let mut current_partial = String::new();
        let mut level_meter = AudioLevelMeter::new();
        let mut preview_throttle = PreviewThrottle::new();
        let mut close_reason: Option<String> = None;
        let mut ws_alive = true;
        let mut audio_buffer: Vec<u8> = Vec::with_capacity(MIN_SEND_BYTES * 2);

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => match maybe_chunk {
                    Some(chunk) => {
                        let now = Instant::now();
                        if let Some(level) = level_meter.observe(now, &chunk) {
                            let _ = app.emit(AUDIO_LEVEL_EVENT, level);
                        }
                        if ws_alive {
                            match to_pcm_16k_mono_bytes(&chunk, format.sample_rate, format.channels) {
                                Ok(bytes) => {
                                    audio_buffer.extend_from_slice(&bytes);
                                    while audio_buffer.len() >= MIN_SEND_BYTES {
                                        let to_send: Vec<u8> =
                                            audio_buffer.drain(..MIN_SEND_BYTES).collect();
                                        if let Err(_) =
                                            sink.send(Message::Binary(to_send.into())).await
                                        {
                                            ws_alive = false;
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {},
                            }
                        }
                    }
                    None => break,
                },
                maybe_msg = stream.next(), if ws_alive => match maybe_msg {
                    Some(Ok(Message::Close(frame))) => {
                        close_reason = Some(
                            frame
                                .map(|f| format!("{} {}", f.code, f.reason))
                                .unwrap_or_else(|| "no frame".to_string()),
                        );
                        ws_alive = false;
                    }
                    Some(Ok(msg)) => {
                        handle_turn(&msg, &mut completed_turns, &mut current_partial);
                        if show_live_preview {
                            let now = Instant::now();
                            let raw = compose_preview(&completed_turns, &current_partial);
                            if let Some(corrected) = preview_throttle.offer(now, &raw, &corrections) {
                                let _ = app.emit(TRANSCRIPT_PARTIAL_EVENT, &corrected);
                            }
                        }
                    }
                    Some(Err(_)) => {
                        ws_alive = false;
                    }
                    None => {
                        ws_alive = false;
                    }
                }
            }
        }

        let speak_duration = speak_start.elapsed();
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);

        if ws_alive {
            if !audio_buffer.is_empty() {
                if audio_buffer.len() < MIN_SEND_BYTES {
                    audio_buffer.resize(MIN_SEND_BYTES, 0);
                }
                let tail = std::mem::take(&mut audio_buffer);
                let _ = sink.send(Message::Binary(tail.into())).await;
            }
            let terminate = serde_json::json!({"type": "Terminate"});
            let _ = sink.send(Message::Text(terminate.to_string().into())).await;

            let deadline = tokio::time::sleep(TERMINATION_TIMEOUT);
            tokio::pin!(deadline);

            loop {
                tokio::select! {
                    _ = &mut deadline => break,
                    maybe_msg = stream.next() => match maybe_msg {
                        None | Some(Err(_)) => break,
                        Some(Ok(msg)) => {
                            if is_termination(&msg) {
                                break;
                            }
                            handle_turn(&msg, &mut completed_turns, &mut current_partial);
                        }
                    }
                }
            }
        }

        if let Some(reason) = close_reason {
            return Err(format!("AssemblyAI closed the connection: {reason}"));
        }

        // end_of_turn fires on detected silence; short PTT clips often
        // terminate before silence detection, leaving only a partial.
        if !current_partial.is_empty() {
            completed_turns.push(std::mem::take(&mut current_partial));
        }
        Ok((completed_turns.join(" "), speak_duration))
    }
}

fn build_ws_url(
    model: AssemblyAiModel,
    language: &ModeLanguage,
    terms: &[String],
) -> Result<Url, String> {
    let mut url = Url::parse(ASSEMBLYAI_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("speech_model", model.api_id());
        q.append_pair("sample_rate", "16000");
        if let ModeLanguage::Exact { code } = language {
            q.append_pair("language_code", code);
        }
        if let Some(prompt) = terms::assemblyai_keyterms_prompt(terms) {
            q.append_pair("keyterms_prompt", &prompt);
        }
    }
    Ok(url)
}

fn handle_turn(msg: &Message, completed: &mut Vec<String>, partial: &mut String) {
    let Some((transcript, end_of_turn)) = parse_turn(msg) else {
        return;
    };
    if end_of_turn {
        if !transcript.is_empty() {
            completed.push(transcript);
        }
        partial.clear();
    } else {
        *partial = transcript;
    }
}

fn compose_preview(completed: &[String], partial: &str) -> String {
    let mut parts: Vec<&str> = completed.iter().map(String::as_str).collect();
    if !partial.is_empty() {
        parts.push(partial);
    }
    parts.join(" ")
}

fn parse_turn(msg: &Message) -> Option<(String, bool)> {
    let Message::Text(text) = msg else {
        return None;
    };
    let v: Value = serde_json::from_str(text.as_str()).ok()?;
    if v.get("type").and_then(|x| x.as_str()) != Some("Turn") {
        return None;
    }
    let transcript = v["transcript"].as_str().unwrap_or("").trim().to_string();
    let end_of_turn = v["end_of_turn"].as_bool().unwrap_or(false);
    Some((transcript, end_of_turn))
}

fn is_termination(msg: &Message) -> bool {
    let Message::Text(text) = msg else {
        return false;
    };
    serde_json::from_str::<Value>(text.as_str())
        .ok()
        .and_then(|v| {
            v.get("type")
                .and_then(|x| x.as_str())
                .map(|s| s == "Termination")
        })
        .unwrap_or(false)
}
