use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::encode_to_flac_16k_mono;
use crate::terms;
use serde_json::Value;
use std::sync::OnceLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const ELEVENLABS_STT_URL: &str = "https://api.elevenlabs.io/v1/speech-to-text";
const SCRIBE_V2_MODEL_ID: &str = "scribe_v2";

pub struct ElevenLabsEngine {
    pub key: String,
}

impl ElevenLabsEngine {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl Engine for ElevenLabsEngine {
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

        let language = ctx.language.as_code().map(str::to_string);
        let keyterms = terms::elevenlabs_keyterms(&ctx.terms);

        let text = post_to_elevenlabs(&self.key, language.as_deref(), &keyterms, flac).await?;

        Ok(EngineOutcome {
            transcript: text,
            warning: None,
        })
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn post_to_elevenlabs(
    key: &str,
    language: Option<&str>,
    keyterms: &[String],
    flac: Vec<u8>,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(flac)
        .file_name("audio.flac")
        .mime_str("audio/flac")
        .map_err(|e| format!("ElevenLabs mime build failed: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .text("model_id", SCRIBE_V2_MODEL_ID)
        .part("file", part);

    if let Some(lang) = language {
        form = form.text("language_code", lang.to_string());
    }

    for keyterm in keyterms {
        form = form.text("keyterm_prompts", keyterm.clone());
    }

    let resp = http_client()
        .post(ELEVENLABS_STT_URL)
        .header("xi-api-key", key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("ElevenLabs request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format_elevenlabs_error(status, &body));
    }

    parse_transcript(&body).map_err(|e| format!("ElevenLabs response parse failed: {e}"))
}

fn format_elevenlabs_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["detail"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("ElevenLabs {status}")
    } else {
        format!("ElevenLabs {status}: {message}")
    }
}

fn parse_transcript(body: &str) -> Result<String, String> {
    let v: Value =
        serde_json::from_str(body).map_err(|e| format!("JSON parse failed: {e}"))?;
    let text = v
        .get("text")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "response missing `text` field".to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new_stores_key() {
        let engine = ElevenLabsEngine::new("xi-test-key".to_string());
        assert_eq!(engine.key, "xi-test-key");
    }

    #[test]
    fn parses_transcript_text() {
        let body = r#"{"text":"hello world"}"#;
        assert_eq!(parse_transcript(body).unwrap(), "hello world");
    }

    #[test]
    fn trims_whitespace_around_transcript() {
        let body = r#"{"text":"  hello world  "}"#;
        assert_eq!(parse_transcript(body).unwrap(), "hello world");
    }

    #[test]
    fn rejects_missing_text_field() {
        let body = r#"{"error":"not authorized"}"#;
        assert!(parse_transcript(body).is_err());
    }

    #[test]
    fn rejects_unparseable_response_body() {
        let body = "not json at all";
        assert!(parse_transcript(body).is_err());
    }

    #[test]
    fn extracts_elevenlabs_error_detail_from_structured_body() {
        let body = r#"{"detail":"Invalid API key"}"#;
        let msg = format_elevenlabs_error(reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(msg.contains("Invalid API key"), "expected detail, got: {msg}");
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_empty() {
        let msg = format_elevenlabs_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }
}
