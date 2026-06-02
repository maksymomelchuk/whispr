use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::to_pcm_16k_mono_bytes;
use crate::mode::ModeLanguage;
use crate::provider::AssemblyAiModel;
use crate::terms;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
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

        Ok(EngineOutcome {
            transcript: completed_turns.join(" "),
            warning: None,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::AssemblyAiModel;
    use tokio_tungstenite::tungstenite::protocol::Message;

    #[test]
    fn assemblyai_engine_new_stores_fields() {
        let engine = AssemblyAiEngine::new(AssemblyAiModel::WhisperStreaming, "key123".to_string());
        assert_eq!(engine.model, AssemblyAiModel::WhisperStreaming);
        assert_eq!(engine.key, "key123");
    }

    #[test]
    fn build_ws_url_sets_speech_model_and_sample_rate() {
        let url = build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[])
            .unwrap();
        let query = url.query().unwrap_or("");
        assert!(query.contains("speech_model=whisper-rt"), "query was: {query}");
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
        let url = build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[])
            .unwrap();
        let query = url.query().unwrap_or("");
        assert!(!query.contains("language_code"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_includes_keyterms_prompt_when_terms_present() {
        let terms = vec!["foo".to_string(), "bar".to_string()];
        let url =
            build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &terms).unwrap();
        let query = url.query().unwrap_or("");
        assert!(query.contains("keyterms_prompt"), "query was: {query}");
    }

    #[test]
    fn build_ws_url_omits_keyterms_prompt_when_no_terms() {
        let url = build_ws_url(AssemblyAiModel::WhisperStreaming, &ModeLanguage::Auto, &[])
            .unwrap();
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
