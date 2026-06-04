//! Live API-key validation for the transcription providers.
//!
//! Triggered from the Settings UI on paste/edit blur. Both providers
//! authenticate against the same key the dictation path uses, so a 200 here
//! means the key will work for real dictation.

use crate::provider::GroqModel;

const ASSEMBLYAI_ACCOUNT_URL: &str = "https://api.assemblyai.com/v2/account";
use crate::groq_audio::encode_to_flac_16k_mono;
use reqwest::StatusCode;
use serde::Serialize;

/// `/v1/projects` lists the projects a key can see. It authenticates with the
/// same `Authorization: Token <key>` header as the dictation WebSocket, so a
/// 200 here guarantees the key will pass the WS auth on the next session.
/// Cheaper to call than firing an empty transcription request.
const DEEPGRAM_AUTH_URL: &str = "https://api.deepgram.com/v1/projects";

const GROQ_TRANSCRIBE_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";
const ELEVENLABS_USER_URL: &str = "https://api.elevenlabs.io/v1/user";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiKeyValidation {
    Valid,
    Invalid,
    Error { message: String },
}

pub fn silent_groq_flac() -> Result<Vec<u8>, String> {
    let samples = vec![0i16; 16_000];
    encode_to_flac_16k_mono(&samples, 16_000, 1)
}

pub fn status_to_validation(status: StatusCode) -> ApiKeyValidation {
    if status.is_success() {
        ApiKeyValidation::Valid
    } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        ApiKeyValidation::Invalid
    } else {
        ApiKeyValidation::Error {
            message: format!("Provider returned HTTP {}", status.as_u16()),
        }
    }
}

pub fn groq_model_id(model: GroqModel) -> &'static str {
    match model {
        GroqModel::WhisperLargeV3 => "whisper-large-v3",
        GroqModel::WhisperLargeV3Turbo => "whisper-large-v3-turbo",
    }
}

pub async fn validate_assemblyai(api_key: &str) -> ApiKeyValidation {
    if api_key.is_empty() {
        return ApiKeyValidation::Invalid;
    }
    let client = reqwest::Client::new();
    match client
        .get(ASSEMBLYAI_ACCOUNT_URL)
        .header("Authorization", api_key)
        .send()
        .await
    {
        Ok(resp) => status_to_validation(resp.status()),
        Err(e) => ApiKeyValidation::Error {
            message: format!("Network error: {e}"),
        },
    }
}

pub async fn validate_deepgram(api_key: &str) -> ApiKeyValidation {
    if api_key.is_empty() {
        return ApiKeyValidation::Invalid;
    }
    let client = reqwest::Client::new();
    match client
        .get(DEEPGRAM_AUTH_URL)
        .header("Authorization", format!("Token {api_key}"))
        .send()
        .await
    {
        Ok(resp) => status_to_validation(resp.status()),
        Err(e) => ApiKeyValidation::Error {
            message: format!("Network error: {e}"),
        },
    }
}

pub async fn validate_openai(api_key: &str) -> ApiKeyValidation {
    if api_key.is_empty() {
        return ApiKeyValidation::Invalid;
    }
    let client = reqwest::Client::new();
    match client
        .get(OPENAI_MODELS_URL)
        .bearer_auth(api_key)
        .send()
        .await
    {
        Ok(resp) => status_to_validation(resp.status()),
        Err(e) => ApiKeyValidation::Error {
            message: format!("Network error: {e}"),
        },
    }
}

pub async fn validate_elevenlabs(api_key: &str) -> ApiKeyValidation {
    if api_key.is_empty() {
        return ApiKeyValidation::Invalid;
    }
    let client = reqwest::Client::new();
    match client
        .get(ELEVENLABS_USER_URL)
        .header("xi-api-key", api_key)
        .send()
        .await
    {
        Ok(resp) => status_to_validation(resp.status()),
        Err(e) => ApiKeyValidation::Error {
            message: format!("Network error: {e}"),
        },
    }
}

pub async fn validate_groq(api_key: &str, model: GroqModel, language: &str) -> ApiKeyValidation {
    if api_key.is_empty() {
        return ApiKeyValidation::Invalid;
    }
    let flac = match silent_groq_flac() {
        Ok(b) => b,
        Err(e) => {
            return ApiKeyValidation::Error {
                message: format!("Failed to build validation audio: {e}"),
            };
        }
    };
    let lang = language.trim();
    let lang = if lang.is_empty() { "en" } else { lang };

    let part = match reqwest::multipart::Part::bytes(flac)
        .file_name("silence.flac")
        .mime_str("audio/flac")
    {
        Ok(p) => p,
        Err(e) => {
            return ApiKeyValidation::Error {
                message: format!("Failed to build multipart: {e}"),
            };
        }
    };
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", groq_model_id(model))
        .text("response_format", "json")
        .text("language", lang.to_string());

    let client = reqwest::Client::new();
    match client
        .post(GROQ_TRANSCRIBE_URL)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(resp) => status_to_validation(resp.status()),
        Err(e) => ApiKeyValidation::Error {
            message: format!("Network error: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_2xx_to_valid() {
        assert_eq!(
            status_to_validation(StatusCode::OK),
            ApiKeyValidation::Valid
        );
        assert_eq!(
            status_to_validation(StatusCode::CREATED),
            ApiKeyValidation::Valid
        );
    }

    #[test]
    fn maps_401_and_403_to_invalid() {
        assert_eq!(
            status_to_validation(StatusCode::UNAUTHORIZED),
            ApiKeyValidation::Invalid
        );
        assert_eq!(
            status_to_validation(StatusCode::FORBIDDEN),
            ApiKeyValidation::Invalid
        );
    }

    #[test]
    fn maps_other_failures_to_error_not_valid() {
        let v = status_to_validation(StatusCode::TOO_MANY_REQUESTS);
        assert!(matches!(v, ApiKeyValidation::Error { .. }));
        let v = status_to_validation(StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(v, ApiKeyValidation::Error { .. }));
        let v = status_to_validation(StatusCode::BAD_REQUEST);
        assert!(matches!(v, ApiKeyValidation::Error { .. }));
    }

    #[test]
    fn silent_groq_flac_decodes_to_one_second_16k_mono() {
        let bytes = silent_groq_flac().expect("encode silence");
        let mut reader = claxon::FlacReader::new(&bytes[..]).expect("FlacReader::new");
        let info = reader.streaminfo();
        assert_eq!(info.sample_rate, 16_000);
        assert_eq!(info.channels, 1);

        let decoded: usize = reader.samples().filter_map(|s| s.ok()).count();
        let expected = 16_000usize;
        let block_tolerance = info.max_block_size as usize;
        let diff = decoded.abs_diff(expected);
        assert!(
            diff <= block_tolerance,
            "decoded {decoded} samples vs expected {expected} (diff {diff})"
        );
    }

    #[test]
    fn groq_model_id_uses_groq_api_naming() {
        assert_eq!(groq_model_id(GroqModel::WhisperLargeV3), "whisper-large-v3");
        assert_eq!(
            groq_model_id(GroqModel::WhisperLargeV3Turbo),
            "whisper-large-v3-turbo"
        );
    }

    #[tokio::test]
    async fn validate_deepgram_short_circuits_on_empty_key() {
        let v = validate_deepgram("").await;
        assert_eq!(v, ApiKeyValidation::Invalid);
    }

    #[tokio::test]
    async fn validate_groq_short_circuits_on_empty_key() {
        let v = validate_groq("", GroqModel::WhisperLargeV3Turbo, "en").await;
        assert_eq!(v, ApiKeyValidation::Invalid);
    }

    #[tokio::test]
    async fn validate_openai_short_circuits_on_empty_key() {
        let v = validate_openai("").await;
        assert_eq!(v, ApiKeyValidation::Invalid);
    }

    #[tokio::test]
    async fn validate_elevenlabs_short_circuits_on_empty_key() {
        let v = validate_elevenlabs("").await;
        assert_eq!(v, ApiKeyValidation::Invalid);
    }
}
