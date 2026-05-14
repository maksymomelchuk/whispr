//! One-shot Groq transcription session.
//!
//! Buffers all captured PCM for the dictation, encodes it to 16 kHz mono
//! FLAC on chunk-channel close, and POSTs a single multipart request to
//! Groq's OpenAI-compatible audio transcription endpoint. `audio-level`
//! events mirror the Deepgram session's cadence so the overlay wave still
//! animates.

use crate::config::{self, GroqModel};
use crate::groq_audio::encode_to_flac_16k_mono;
use crate::recorder::AudioFormat;
use crate::transcription_session::TranscriptionSession;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedReceiver;

const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const AUDIO_LEVEL_EVENT: &str = "audio-level";
/// Match the Deepgram session's cadence — the overlay wave should look
/// identical regardless of which provider is active.
const LEVEL_THROTTLE: Duration = Duration::from_millis(33);

pub struct GroqSession;

impl TranscriptionSession for GroqSession {
    async fn run(
        self,
        app: AppHandle,
        format: AudioFormat,
        mut chunks: UnboundedReceiver<Vec<i16>>,
    ) -> Result<(String, Duration), String> {
        let speak_start = Instant::now();
        let settings = config::load(&app);
        let key = settings
            .groq_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "Groq API key not configured".to_string())?;
        let model = groq_model_api_id(settings.groq.model);
        let language = if settings.groq.language.trim().is_empty() {
            "en"
        } else {
            settings.groq.language.trim()
        };

        let mut buffered: Vec<i16> = Vec::new();
        let mut smoothed_level: f32 = 0.0;
        let mut last_level_emit: Option<Instant> = None;

        while let Some(chunk) = chunks.recv().await {
            let raw_level = compute_level(&chunk);
            buffered.extend_from_slice(&chunk);
            // Asymmetric EMA matching DeepgramSession: fast attack so
            // vowels punch, slow decay so the wave doesn't snap to flat
            // between syllables.
            let k = if raw_level > smoothed_level { 0.6 } else { 0.25 };
            smoothed_level += (raw_level - smoothed_level) * k;
            let now = Instant::now();
            if last_level_emit
                .map_or(true, |t| now.duration_since(t) >= LEVEL_THROTTLE)
            {
                let _ = app.emit(AUDIO_LEVEL_EVENT, smoothed_level);
                last_level_emit = Some(now);
            }
        }
        let speak_duration = speak_start.elapsed();
        // Settle the wave — the pill stays up through the upload + parse
        // and we don't want it dancing on the last cached level.
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);

        if buffered.is_empty() {
            return Ok((String::new(), speak_duration));
        }

        let flac = encode_to_flac_16k_mono(&buffered, format.sample_rate, format.channels)?;
        let raw = post_to_groq(&key, model, language, flac).await?;
        Ok((raw, speak_duration))
    }
}

fn groq_model_api_id(model: GroqModel) -> &'static str {
    match model {
        GroqModel::WhisperLargeV3 => "whisper-large-v3",
        GroqModel::WhisperLargeV3Turbo => "whisper-large-v3-turbo",
    }
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn post_to_groq(
    key: &str,
    model: &str,
    language: &str,
    flac: Vec<u8>,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(flac)
        .file_name("audio.flac")
        .mime_str("audio/flac")
        .map_err(|e| format!("Groq mime build failed: {e}"))?;
    let form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json")
        .text("language", language.to_string())
        .part("file", part);

    let resp = http_client()
        .post(GROQ_URL)
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_groq_error(status, &body));
    }
    parse_transcript(&body)
}

fn format_groq_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("Groq {status}")
    } else {
        format!("Groq {status}: {message}")
    }
}

fn parse_transcript(body: &str) -> Result<String, String> {
    let v: Value =
        serde_json::from_str(body).map_err(|e| format!("Groq response parse failed: {e}"))?;
    let text = v
        .get("text")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Groq response missing `text` field".to_string())?;
    Ok(text.trim().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_groq_model_to_api_id() {
        assert_eq!(
            groq_model_api_id(GroqModel::WhisperLargeV3),
            "whisper-large-v3"
        );
        assert_eq!(
            groq_model_api_id(GroqModel::WhisperLargeV3Turbo),
            "whisper-large-v3-turbo"
        );
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
    fn extracts_groq_error_message_from_structured_body() {
        let body = r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#;
        let msg = format_groq_error(reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(msg.contains("Invalid API Key"), "expected message, got: {msg}");
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_empty() {
        let msg = format_groq_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_not_json() {
        let msg = format_groq_error(reqwest::StatusCode::BAD_GATEWAY, "<html>nope</html>");
        assert!(msg.contains("502"), "expected status, got: {msg}");
        assert!(msg.contains("<html>nope</html>"), "expected snippet, got: {msg}");
    }
}
