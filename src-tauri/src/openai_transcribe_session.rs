use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::encode_to_flac_16k_mono;
use crate::provider::OpenAiTranscribeModel;
use crate::terms;
use serde_json::Value;
use std::sync::OnceLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const OPENAI_TRANSCRIBE_URL: &str = "https://api.openai.com/v1/audio/transcriptions";

pub struct OpenAiTranscribeEngine {
    pub model: OpenAiTranscribeModel,
    pub key: String,
}

impl OpenAiTranscribeEngine {
    pub fn new(model: OpenAiTranscribeModel, key: String) -> Self {
        Self { model, key }
    }
}

impl Engine for OpenAiTranscribeEngine {
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
        let prompt = terms::whisper_prompt_hint(&ctx.terms);

        let text = post_to_openai(
            &self.key,
            self.model.api_id(),
            language.as_deref(),
            prompt.as_deref(),
            flac,
        )
        .await?;

        Ok(EngineOutcome {
            transcript: strip_prompt_echo(&text, prompt.as_deref()),
            warning: None,
        })
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn post_to_openai(
    key: &str,
    model: &str,
    // None for Auto — Whisper auto-detects the language.
    language: Option<&str>,
    prompt: Option<&str>,
    flac: Vec<u8>,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(flac)
        .file_name("audio.flac")
        .mime_str("audio/flac")
        .map_err(|e| format!("OpenAI mime build failed: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json");

    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }
    form = form.part("file", part);
    if let Some(p) = prompt {
        form = form.text("prompt", p.to_string());
    }

    let resp = http_client()
        .post(OPENAI_TRANSCRIBE_URL)
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format_openai_error(status, &body));
    }

    parse_transcript(&body).map_err(|e| format!("OpenAI response parse failed: {e}"))
}

fn format_openai_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("OpenAI {status}")
    } else {
        format!("OpenAI {status}: {message}")
    }
}

// Whisper repeats its prompt when it can't transcribe short or silent audio.
// The vocabulary hint always begins with "Vocabulary: " — any transcript that
// starts with that prefix (or its comma variant produced by some Whisper
// variants) is an echo, not real speech.
fn strip_prompt_echo(text: &str, prompt: Option<&str>) -> String {
    if prompt.is_none() || text.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    if lower.starts_with("vocabulary:") || lower.starts_with("vocabulary,") {
        return String::new();
    }
    text.to_string()
}

fn parse_transcript(body: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("JSON parse failed: {e}"))?;
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
    fn engine_new_stores_model_and_key() {
        let engine = OpenAiTranscribeEngine::new(
            OpenAiTranscribeModel::Gpt4oTranscribe,
            "sk-test".to_string(),
        );
        assert_eq!(engine.key, "sk-test");
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
    fn extracts_openai_error_message_from_structured_body() {
        let body = r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#;
        let msg = format_openai_error(reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(
            msg.contains("Invalid API Key"),
            "expected message, got: {msg}"
        );
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_empty() {
        let msg = format_openai_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }

    #[test]
    fn strip_prompt_echo_discards_vocabulary_colon_prefix() {
        let text = "Vocabulary: Claude Code, OAuth, UUID, JWT";
        assert_eq!(strip_prompt_echo(text, Some("Vocabulary: Claude Code")), "");
    }

    #[test]
    fn strip_prompt_echo_discards_vocabulary_comma_prefix() {
        let text = "Vocabulary, Claude Code, OAuth, UUID, JWT";
        assert_eq!(strip_prompt_echo(text, Some("Vocabulary: Claude Code")), "");
    }

    #[test]
    fn strip_prompt_echo_preserves_real_speech() {
        let text = "Let's build something fancy";
        assert_eq!(
            strip_prompt_echo(text, Some("Vocabulary: Claude Code")),
            "Let's build something fancy"
        );
    }

    #[test]
    fn strip_prompt_echo_passthrough_when_no_prompt() {
        let text = "Vocabulary: whatever";
        assert_eq!(strip_prompt_echo(text, None), "Vocabulary: whatever");
    }

    #[test]
    fn strip_prompt_echo_passthrough_for_empty_text() {
        assert_eq!(strip_prompt_echo("", Some("Vocabulary: foo")), "");
    }
}
