use super::transcript::Token;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub const SONIOX_API_BASE_URL: &str = "https://api.soniox.com";
const ASYNC_MODEL: &str = "stt-async-v6";
const UPLOAD_FILE_NAME: &str = "meeting.flac";
const UPLOAD_MIME_TYPE: &str = "audio/flac";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Soniox caps files at 300 minutes; even that finishes well within this.
const MAX_TRANSCRIPTION_WAIT: Duration = Duration::from_secs(45 * 60);
const ERROR_BODY_SNIPPET_CHARS: usize = 300;
/// Uploading a 90-minute FLAC (~80 MB) on a slow uplink takes minutes.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const STATUS_COMPLETED: &str = "completed";
const STATUS_ERROR: &str = "error";

pub struct TranscriptionRequest {
    pub diarize: bool,
    pub terms: Vec<String>,
}

pub struct SonioxAsyncClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct IdResponse {
    id: String,
}

#[derive(Deserialize)]
struct StatusResponse {
    status: String,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Deserialize)]
struct TranscriptResponse {
    tokens: Vec<Token>,
}

impl SonioxAsyncClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Uploads the audio, waits for the async job, and returns its tokens.
    /// The uploaded file and the job are deleted afterwards either way, since
    /// Soniox otherwise keeps both for 30 days.
    pub async fn transcribe(
        &self,
        flac: Vec<u8>,
        request: &TranscriptionRequest,
    ) -> Result<Vec<Token>, String> {
        let file_id = self.upload(flac).await?;
        let result = self.transcribe_uploaded(&file_id, request).await;
        self.delete(&format!("/v1/files/{file_id}")).await;
        result
    }

    async fn transcribe_uploaded(
        &self,
        file_id: &str,
        request: &TranscriptionRequest,
    ) -> Result<Vec<Token>, String> {
        let transcription_id = self.create_transcription(file_id, request).await?;
        let result = self.wait_and_fetch(&transcription_id).await;
        self.delete(&format!("/v1/transcriptions/{transcription_id}"))
            .await;
        result
    }

    async fn wait_and_fetch(&self, transcription_id: &str) -> Result<Vec<Token>, String> {
        self.wait_until_completed(transcription_id).await?;
        let path = format!("/v1/transcriptions/{transcription_id}/transcript");
        let transcript: TranscriptResponse = self.get_json(&path).await?;
        Ok(transcript.tokens)
    }

    async fn upload(&self, flac: Vec<u8>) -> Result<String, String> {
        let part = Part::bytes(flac)
            .file_name(UPLOAD_FILE_NAME)
            .mime_str(UPLOAD_MIME_TYPE)
            .map_err(|e| e.to_string())?;
        let response = self
            .http
            .post(self.url("/v1/files"))
            .timeout(UPLOAD_TIMEOUT)
            .bearer_auth(&self.api_key)
            .multipart(Form::new().part("file", part))
            .send()
            .await
            .map_err(|e| format!("Soniox upload failed: {e}"))?;
        Ok(parse_json::<IdResponse>(response, "upload").await?.id)
    }

    async fn create_transcription(
        &self,
        file_id: &str,
        request: &TranscriptionRequest,
    ) -> Result<String, String> {
        let response = self
            .http
            .post(self.url("/v1/transcriptions"))
            .timeout(REQUEST_TIMEOUT)
            .bearer_auth(&self.api_key)
            .json(&transcription_body(file_id, request))
            .send()
            .await
            .map_err(|e| format!("Soniox transcription request failed: {e}"))?;
        Ok(parse_json::<IdResponse>(response, "create transcription")
            .await?
            .id)
    }

    async fn wait_until_completed(&self, transcription_id: &str) -> Result<(), String> {
        let deadline = Instant::now() + MAX_TRANSCRIPTION_WAIT;
        let path = format!("/v1/transcriptions/{transcription_id}");
        loop {
            let status: StatusResponse = self.get_json(&path).await?;
            match status.status.as_str() {
                STATUS_COMPLETED => return Ok(()),
                STATUS_ERROR => {
                    let reason = status.error_message.unwrap_or_default();
                    return Err(format!(
                        "Soniox transcription {transcription_id} failed: {reason}"
                    ));
                }
                _ if Instant::now() >= deadline => {
                    return Err(format!("Soniox transcription {transcription_id} timed out"));
                }
                _ => tokio::time::sleep(POLL_INTERVAL).await,
            }
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self
            .http
            .get(self.url(path))
            .timeout(REQUEST_TIMEOUT)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("Soniox request {path} failed: {e}"))?;
        parse_json(response, path).await
    }

    async fn delete(&self, path: &str) {
        let result = self
            .http
            .delete(self.url(path))
            .timeout(REQUEST_TIMEOUT)
            .bearer_auth(&self.api_key)
            .send()
            .await;
        if let Err(e) = result.and_then(|r| r.error_for_status()) {
            eprintln!("[meeting] Soniox cleanup {path} failed: {e}");
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

fn transcription_body(file_id: &str, request: &TranscriptionRequest) -> Value {
    let mut body = json!({
        "model": ASYNC_MODEL,
        "file_id": file_id,
        "enable_speaker_diarization": request.diarize,
        "enable_language_identification": true,
    });
    if !request.terms.is_empty() {
        body["context"] = json!({ "terms": request.terms });
    }
    body
}

async fn parse_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    step: &str,
) -> Result<T, String> {
    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = body.chars().take(ERROR_BODY_SNIPPET_CHARS).collect();
        return Err(format!("Soniox {step} returned HTTP {status}: {snippet}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("Soniox {step} response unreadable: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    fn request() -> TranscriptionRequest {
        TranscriptionRequest {
            diarize: true,
            terms: vec!["Tauri".into()],
        }
    }

    async fn mock_upload_and_create(server: &mut mockito::ServerGuard) {
        server
            .mock("POST", "/v1/files")
            .with_body(r#"{"id":"file-1"}"#)
            .create_async()
            .await;
        server
            .mock("POST", "/v1/transcriptions")
            .match_body(Matcher::PartialJson(json!({
                "file_id": "file-1",
                "enable_speaker_diarization": true,
                "context": {"terms": ["Tauri"]}
            })))
            .with_body(r#"{"id":"job-1"}"#)
            .create_async()
            .await;
        server.mock("DELETE", Matcher::Any).create_async().await;
    }

    #[tokio::test]
    async fn completed_job_returns_its_tokens() {
        let mut server = mockito::Server::new_async().await;
        mock_upload_and_create(&mut server).await;
        server
            .mock("GET", "/v1/transcriptions/job-1")
            .with_body(r#"{"status":"completed"}"#)
            .create_async()
            .await;
        server
            .mock("GET", "/v1/transcriptions/job-1/transcript")
            .with_body(r#"{"tokens":[{"text":"Hi","start_ms":0,"end_ms":200,"speaker":"1"}]}"#)
            .create_async()
            .await;
        let client = SonioxAsyncClient::new(&server.url(), "key");

        let tokens = client.transcribe(vec![0; 4], &request()).await.unwrap();

        assert_eq!(tokens[0].text, "Hi");
        assert_eq!(tokens[0].speaker.as_deref(), Some("1"));
    }

    #[tokio::test]
    async fn failed_job_reports_the_soniox_error_message() {
        let mut server = mockito::Server::new_async().await;
        mock_upload_and_create(&mut server).await;
        server
            .mock("GET", "/v1/transcriptions/job-1")
            .with_body(r#"{"status":"error","error_message":"audio too short"}"#)
            .create_async()
            .await;
        let client = SonioxAsyncClient::new(&server.url(), "key");

        let error = client.transcribe(vec![0; 4], &request()).await.unwrap_err();

        assert!(error.contains("audio too short"), "{error}");
    }

    #[tokio::test]
    async fn rejected_upload_reports_http_status() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/v1/files")
            .with_status(401)
            .with_body("invalid api key")
            .create_async()
            .await;
        let client = SonioxAsyncClient::new(&server.url(), "bad");

        let error = client.transcribe(vec![0; 4], &request()).await.unwrap_err();

        assert!(
            error.contains("401") && error.contains("invalid api key"),
            "{error}"
        );
    }
}
