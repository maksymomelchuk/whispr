use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::{encode_to_flac_16k_mono, to_pcm_16k_mono_bytes};
use crate::mode::ModeLanguage;
use crate::provider::AssemblyAiModel;
use crate::terms;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const ASSEMBLYAI_WS_BASE: &str = "wss://streaming.assemblyai.com/v3/ws";
/// Hard ceiling on the whole post-release wait (catch-up + forced finalize), so
/// a turn that never lands can't block the paste indefinitely.
const TERMINATION_TIMEOUT: Duration = Duration::from_secs(3);
/// In phase 2, once the forced final turn has landed and the socket stays quiet
/// this long, the finalize is complete.
const POST_TERMINATE_IDLE: Duration = Duration::from_millis(250);
/// Phase 1 quiet window: after the model has started emitting the buffered tail,
/// this much silence means it has caught up on all sent audio. Must exceed the
/// gap between consecutive partials while the server is actively transcribing.
const CATCHUP_IDLE: Duration = Duration::from_millis(500);
/// Phase 1 startup window: before any post-release turn has arrived, allow this
/// much quiet for the model to begin emitting the tail (it runs behind the
/// audio). Also bounds the no-op case where the turn already finalized and
/// there's nothing left to transcribe.
const FLUSH_START_GRACE: Duration = Duration::from_millis(700);
/// AssemblyAI rejects sends outside 50–1000 ms. 50 ms at 16 kHz mono 16-bit
/// = 800 samples * 2 bytes = 1600 bytes.
const MIN_SEND_BYTES: usize = 1600;
/// Duration one `MIN_SEND_BYTES` frame represents at 16 kHz mono 16-bit.
const FRAME_MS: usize = 50;
/// Trailing silence fed after the user releases, so the model crosses its
/// silence threshold (min_turn_silence default 400 ms) and finalizes the last
/// words instead of leaving them uncommitted. Comfortably above 400 ms so a
/// confident endpoint fires; also gives a fallback ForceEndpoint the context it
/// needs to commit the tail.
const SILENCE_FLUSH: Duration = Duration::from_millis(1000);

pub struct AssemblyAiEngine {
    pub model: AssemblyAiModel,
    pub key: String,
}

impl AssemblyAiEngine {
    pub fn new(model: AssemblyAiModel, key: String) -> Self {
        Self { model, key }
    }
}

impl Engine for AssemblyAiEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        if let ModeLanguage::Exact { code } = &ctx.language {
            if !self.model.supports_language(code) {
                return Err(format!(
                    "AssemblyAI model '{}' does not support language '{}'",
                    self.model.api_id(),
                    code
                ));
            }
        }

        let url = build_ws_url(self.model, &ctx.language, &ctx.terms)?;
        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("bad WS URL: {e}"))?;
        req.headers_mut().insert(
            "Authorization",
            self.key
                .parse()
                .map_err(|e| format!("bad auth header: {e}"))?,
        );

        let (ws, _) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("AssemblyAI WS connect failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();

        let mut completed_turns: Vec<String> = Vec::new();
        let mut current_partial = String::new();
        let mut close_reason: Option<String> = None;
        let mut ws_alive = true;
        let mut audio_buffer: Vec<u8> = Vec::with_capacity(MIN_SEND_BYTES * 2);

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => match maybe_chunk {
                    Some(chunk) => {
                        if ws_alive {
                            match to_pcm_16k_mono_bytes(&chunk, ctx.format.sample_rate, ctx.format.channels) {
                                Ok(bytes) => {
                                    audio_buffer.extend_from_slice(&bytes);
                                    while audio_buffer.len() >= MIN_SEND_BYTES {
                                        let to_send: Vec<u8> =
                                            audio_buffer.drain(..MIN_SEND_BYTES).collect();
                                        if sink.send(Message::Binary(to_send.into())).await.is_err() {
                                            ws_alive = false;
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {}
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
                        let _ = previews.send(compose_preview(&completed_turns, &current_partial));
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

        if ws_alive {
            if !audio_buffer.is_empty() {
                if audio_buffer.len() < MIN_SEND_BYTES {
                    audio_buffer.resize(MIN_SEND_BYTES, 0);
                }
                let tail = std::mem::take(&mut audio_buffer);
                let _ = sink.send(Message::Binary(tail)).await;
            }

            // Universal Streaming finalizes a turn on *detected silence*
            // (min_turn_silence, default 400 ms), not when audio stops. If we
            // just stop sending, the last words it received stay uncommitted and
            // ForceEndpoint cuts them off. Feed trailing silence so it transcribes
            // the buffered tail and finalizes naturally with the full transcript.
            let silence_frames = SILENCE_FLUSH.as_millis() as usize / FRAME_MS;
            let silence = vec![0u8; MIN_SEND_BYTES];
            for _ in 0..silence_frames {
                if sink.send(Message::Binary(silence.clone())).await.is_err() {
                    break;
                }
            }

            let deadline = tokio::time::sleep(TERMINATION_TIMEOUT);
            tokio::pin!(deadline);

            // Phase 1 — wait for the model to transcribe the buffered tail and the
            // trailing silence, which makes it commit the last words and usually
            // fire a natural end_of_turn. We read until it goes quiet (caught up)
            // or finalizes. Until the first post-release turn arrives we allow
            // FLUSH_START_GRACE for it to start; after that, CATCHUP_IDLE of quiet
            // means it's done.
            let mut natural_final = false;
            let mut flush_started = !current_partial.is_empty();
            loop {
                let idle = if flush_started {
                    CATCHUP_IDLE
                } else {
                    FLUSH_START_GRACE
                };
                tokio::select! {
                    _ = &mut deadline => break,
                    next = tokio::time::timeout(idle, stream.next()) => match next {
                        Err(_) => break,
                        Ok(None) | Ok(Some(Err(_))) => break,
                        Ok(Some(Ok(msg))) => {
                            if is_termination(&msg) {
                                break;
                            }
                            match handle_turn(&msg, &mut completed_turns, &mut current_partial) {
                                Some(true) => {
                                    natural_final = true;
                                    break;
                                }
                                Some(false) => flush_started = true,
                                None => {}
                            }
                        }
                    }
                }
            }

            // Phase 2 — now that the buffered audio is transcribed, force a clean
            // endpoint to finalize the turn, unless the server already finalized
            // it naturally. Exits on that final, idle, or the shared deadline.
            if !natural_final && !current_partial.is_empty() {
                let _ = sink.send(Message::Text(force_endpoint())).await;
                loop {
                    tokio::select! {
                        _ = &mut deadline => break,
                        next = tokio::time::timeout(POST_TERMINATE_IDLE, stream.next()) => match next {
                            Err(_) => break,
                            Ok(None) | Ok(Some(Err(_))) => break,
                            Ok(Some(Ok(msg))) => {
                                if is_termination(&msg) {
                                    break;
                                }
                                if handle_turn(&msg, &mut completed_turns, &mut current_partial)
                                    == Some(true)
                                {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let _ = sink.send(Message::Text(terminate())).await;
        }

        if let Some(reason) = close_reason {
            return Err(format!("AssemblyAI closed the connection: {reason}"));
        }

        // Fallback when the forced finalization didn't land within the deadline:
        // keep the last partial so the tail isn't dropped.
        if !current_partial.is_empty() {
            completed_turns.push(std::mem::take(&mut current_partial));
        }

        Ok(EngineOutcome {
            transcript: completed_turns.join(" "),
            warning: None,
        })
    }
}

const ASSEMBLYAI_UPLOAD_URL: &str = "https://api.assemblyai.com/v2/upload";
const ASSEMBLYAI_TRANSCRIPT_URL: &str = "https://api.assemblyai.com/v2/transcript";
const UNIVERSAL_2_SPEECH_MODEL: &str = "universal-2";
const POLL_INTERVAL: Duration = Duration::from_millis(400);
/// Ceiling on the async transcription round-trip so a stuck job can't hang the
/// paste indefinitely.
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(60);

/// Universal-2 has no streaming endpoint, so this engine buffers the full clip,
/// uploads it, submits an async transcript, and polls for the result. It exists
/// alongside the streaming engine because Universal-2 is the only AssemblyAI
/// model that covers Ukrainian (and 98 other languages).
pub struct AssemblyAiUniversalEngine {
    pub key: String,
}

impl AssemblyAiUniversalEngine {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl Engine for AssemblyAiUniversalEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        _previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        let mut all_samples: Vec<i16> = Vec::new();
        while let Some(chunk) = chunks.recv().await {
            all_samples.extend_from_slice(&chunk);
        }

        if all_samples.is_empty() {
            return Ok(EngineOutcome {
                transcript: String::new(),
                warning: None,
            });
        }

        let flac =
            encode_to_flac_16k_mono(&all_samples, ctx.format.sample_rate, ctx.format.channels)
                .map_err(|e| format!("FLAC encode failed: {e}"))?;

        let upload_url = upload_audio(&self.key, flac).await?;
        let id = submit_transcript(&self.key, &upload_url, &ctx.language, &ctx.terms).await?;
        let transcript = poll_transcript(&self.key, &id).await?;

        Ok(EngineOutcome {
            transcript,
            warning: None,
        })
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn upload_audio(key: &str, flac: Vec<u8>) -> Result<String, String> {
    let resp = http_client()
        .post(ASSEMBLYAI_UPLOAD_URL)
        .header("authorization", key)
        .body(flac)
        .send()
        .await
        .map_err(|e| format!("AssemblyAI upload request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_async_error(status, &body));
    }

    let v: Value =
        serde_json::from_str(&body).map_err(|e| format!("AssemblyAI upload parse failed: {e}"))?;
    v.get("upload_url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "AssemblyAI upload response missing `upload_url`".to_string())
}

async fn submit_transcript(
    key: &str,
    audio_url: &str,
    language: &ModeLanguage,
    terms: &[String],
) -> Result<String, String> {
    let mut body = json!({
        "audio_url": audio_url,
        "speech_models": [UNIVERSAL_2_SPEECH_MODEL],
    });
    apply_language(&mut body, language);
    let keyterms = terms::assemblyai_keyterms(terms);
    if !keyterms.is_empty() {
        body["keyterms_prompt"] = json!(keyterms);
    }

    let resp = http_client()
        .post(ASSEMBLYAI_TRANSCRIPT_URL)
        .header("authorization", key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AssemblyAI submit request failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_async_error(status, &text));
    }

    let v: Value =
        serde_json::from_str(&text).map_err(|e| format!("AssemblyAI submit parse failed: {e}"))?;
    v.get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "AssemblyAI submit response missing `id`".to_string())
}

async fn poll_transcript(key: &str, id: &str) -> Result<String, String> {
    let url = format!("{ASSEMBLYAI_TRANSCRIPT_URL}/{id}");
    let polled = tokio::time::timeout(TRANSCRIBE_TIMEOUT, async {
        loop {
            let resp = http_client()
                .get(&url)
                .header("authorization", key)
                .send()
                .await
                .map_err(|e| format!("AssemblyAI poll request failed: {e}"))?;

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(format_async_error(status, &body));
            }

            let v: Value = serde_json::from_str(&body)
                .map_err(|e| format!("AssemblyAI poll parse failed: {e}"))?;
            match v.get("status").and_then(Value::as_str) {
                Some("completed") => {
                    return Ok(v
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim()
                        .to_string());
                }
                Some("error") => {
                    let detail = v
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    return Err(format!("AssemblyAI transcription failed: {detail}"));
                }
                _ => tokio::time::sleep(POLL_INTERVAL).await,
            }
        }
    })
    .await;

    polled.unwrap_or_else(|_| Err("AssemblyAI transcription timed out".to_string()))
}

/// `language_code` forces a single language; `language_detection` auto-detects.
/// Hints bias detection toward the expected set via `expected_languages` rather
/// than forcing, so each clip is transcribed in the language actually spoken
/// instead of being coerced into the first listed code.
fn apply_language(body: &mut Value, language: &ModeLanguage) {
    match language {
        ModeLanguage::Exact { code } => {
            body["language_code"] = json!(code);
        }
        ModeLanguage::Hints { codes } => {
            body["language_detection"] = json!(true);
            body["language_detection_options"] = json!({ "expected_languages": codes });
        }
        ModeLanguage::Auto => {
            body["language_detection"] = json!(true);
        }
    }
}

fn format_async_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("AssemblyAI {status}")
    } else {
        format!("AssemblyAI {status}: {message}")
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

/// Applies a message to the running transcript. Returns `Some(true)` when it
/// finalized a turn (`end_of_turn`), `Some(false)` for an in-progress partial,
/// and `None` for a non-turn message — the post-release drain uses this to tell
/// "the forced flush landed" from "the server is still transcribing the tail".
fn handle_turn(msg: &Message, completed: &mut Vec<String>, partial: &mut String) -> Option<bool> {
    let (transcript, end_of_turn) = parse_turn(msg)?;
    if end_of_turn {
        if !transcript.is_empty() {
            completed.push(transcript);
        }
        partial.clear();
        return Some(true);
    }
    *partial = transcript;
    Some(false)
}

fn compose_preview(completed: &[String], partial: &str) -> String {
    let mut parts: Vec<&str> = completed.iter().map(String::as_str).collect();
    if !partial.is_empty() {
        parts.push(partial);
    }
    parts.join(" ")
}

fn force_endpoint() -> String {
    serde_json::json!({"type": "ForceEndpoint"}).to_string()
}

fn terminate() -> String {
    serde_json::json!({"type": "Terminate"}).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::AssemblyAiModel;
    use tokio_tungstenite::tungstenite::protocol::Message;

    #[test]
    fn universal_engine_new_stores_key() {
        let engine = AssemblyAiUniversalEngine::new("key123".to_string());
        assert_eq!(engine.key, "key123");
    }

    #[test]
    fn apply_language_sets_language_code_for_exact() {
        let mut body = json!({});
        apply_language(&mut body, &ModeLanguage::exact("uk"));
        assert_eq!(body["language_code"], "uk");
        assert!(body.get("language_codes").is_none());
        assert!(body.get("language_detection").is_none());
    }

    #[test]
    fn apply_language_biases_detection_for_hints() {
        let mut body = json!({});
        let lang = ModeLanguage::Hints {
            codes: vec!["uk".to_string(), "en".to_string()],
        };
        apply_language(&mut body, &lang);
        assert_eq!(body["language_detection"], true);
        assert_eq!(
            body["language_detection_options"]["expected_languages"],
            json!(["uk", "en"])
        );
        assert!(body.get("language_code").is_none());
    }

    #[test]
    fn apply_language_enables_detection_for_auto() {
        let mut body = json!({});
        apply_language(&mut body, &ModeLanguage::Auto);
        assert_eq!(body["language_detection"], true);
        assert!(body.get("language_code").is_none());
        assert!(body.get("language_detection_options").is_none());
    }

    #[test]
    fn format_async_error_extracts_error_field() {
        let msg = format_async_error(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"error":"Invalid API key"}"#,
        );
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(
            msg.contains("Invalid API key"),
            "expected detail, got: {msg}"
        );
    }

    #[test]
    fn format_async_error_falls_back_to_status_when_empty() {
        let msg = format_async_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }

    #[test]
    fn assemblyai_engine_new_stores_fields() {
        let engine = AssemblyAiEngine::new(AssemblyAiModel::WhisperStreaming, "key123".to_string());
        assert_eq!(engine.model, AssemblyAiModel::WhisperStreaming);
        assert_eq!(engine.key, "key123");
    }

    #[test]
    fn build_ws_url_sets_speech_model_and_sample_rate() {
        let url =
            build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            query.contains("speech_model=whisper-rt"),
            "query was: {query}"
        );
        assert!(query.contains("sample_rate=16000"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_includes_language_code_when_exact() {
        let url = build_ws_url(
            AssemblyAiModel::WhisperStreaming,
            &ModeLanguage::exact("uk"),
            &[],
        )
        .unwrap();
        let query = url.query().unwrap_or("");
        assert!(query.contains("language_code=uk"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_omits_language_code_when_auto() {
        let url =
            build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(!query.contains("language_code"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_includes_keyterms_prompt_when_terms_present() {
        let terms = vec!["foo".to_string(), "bar".to_string()];
        let url = build_ws_url(
            AssemblyAiModel::WhisperStreaming,
            &ModeLanguage::Auto,
            &terms,
        )
        .unwrap();
        let query = url.query().unwrap_or("");
        assert!(query.contains("keyterms_prompt"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_omits_keyterms_prompt_when_no_terms() {
        let url =
            build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(!query.contains("keyterms_prompt"), "query was: {query}");
    }

    #[test]
    fn compose_preview_joins_completed_and_partial() {
        let completed = vec!["Hello".to_string(), "world".to_string()];
        assert_eq!(compose_preview(&completed, "foo"), "Hello world foo");
    }

    #[test]
    fn compose_preview_excludes_empty_partial() {
        let completed = vec!["Hello".to_string()];
        assert_eq!(compose_preview(&completed, ""), "Hello");
    }

    #[test]
    fn compose_preview_returns_empty_when_nothing() {
        assert_eq!(compose_preview(&[], ""), "");
    }

    #[test]
    fn force_endpoint_message_has_force_endpoint_type() {
        let v: Value = serde_json::from_str(&force_endpoint()).unwrap();
        assert_eq!(v["type"], "ForceEndpoint");
    }

    #[test]
    fn terminate_message_has_terminate_type() {
        let v: Value = serde_json::from_str(&terminate()).unwrap();
        assert_eq!(v["type"], "Terminate");
    }

    #[test]
    fn handle_turn_reports_finalization_on_end_of_turn() {
        let mut completed = Vec::new();
        let mut partial = String::new();
        let msg = Message::Text(
            r#"{"type":"Turn","transcript":"hello world","end_of_turn":true}"#.into(),
        );
        assert_eq!(handle_turn(&msg, &mut completed, &mut partial), Some(true));
        assert_eq!(completed, vec!["hello world".to_string()]);
        assert!(partial.is_empty());
    }

    #[test]
    fn handle_turn_reports_partial_in_progress() {
        let mut completed = Vec::new();
        let mut partial = String::new();
        let msg =
            Message::Text(r#"{"type":"Turn","transcript":"hello","end_of_turn":false}"#.into());
        assert_eq!(handle_turn(&msg, &mut completed, &mut partial), Some(false));
        assert!(completed.is_empty());
        assert_eq!(partial, "hello");
    }

    #[test]
    fn handle_turn_reports_none_for_non_turn_message() {
        let mut completed = Vec::new();
        let mut partial = String::new();
        let msg = Message::Text(r#"{"type":"Termination"}"#.into());
        assert_eq!(handle_turn(&msg, &mut completed, &mut partial), None);
        assert!(completed.is_empty());
        assert!(partial.is_empty());
    }

    #[test]
    fn parse_turn_returns_none_for_binary_message() {
        let msg = Message::Binary(vec![1, 2, 3].into());
        assert!(parse_turn(&msg).is_none());
    }

    #[test]
    fn parse_turn_returns_none_for_non_turn_type() {
        let msg = Message::Text(r#"{"type":"Termination"}"#.into());
        assert!(parse_turn(&msg).is_none());
    }

    #[test]
    fn parse_turn_extracts_partial_transcript() {
        let msg = Message::Text(
            r#"{"type":"Turn","transcript":"hello world","end_of_turn":false}"#.into(),
        );
        let result = parse_turn(&msg).unwrap();
        assert_eq!(result.0, "hello world");
        assert!(!result.1);
    }

    #[test]
    fn parse_turn_extracts_final_transcript() {
        let msg = Message::Text(
            r#"{"type":"Turn","transcript":"hello world","end_of_turn":true}"#.into(),
        );
        let result = parse_turn(&msg).unwrap();
        assert_eq!(result.0, "hello world");
        assert!(result.1);
    }

    #[test]
    fn is_termination_detects_termination_type() {
        let msg = Message::Text(r#"{"type":"Termination"}"#.into());
        assert!(is_termination(&msg));
    }

    #[test]
    fn is_termination_returns_false_for_turn_type() {
        let msg = Message::Text(r#"{"type":"Turn","transcript":"hi","end_of_turn":false}"#.into());
        assert!(!is_termination(&msg));
    }

    #[test]
    fn is_termination_returns_false_for_binary() {
        let msg = Message::Binary(vec![].into());
        assert!(!is_termination(&msg));
    }
}
