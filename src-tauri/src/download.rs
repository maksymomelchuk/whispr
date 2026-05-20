use crate::provider::LocalWhisperModel;
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub const MODEL_DOWNLOAD_PROGRESS_EVENT: &str = "model-download-progress";
pub const MODEL_DOWNLOAD_COMPLETE_EVENT: &str = "model-download-complete";
pub const MODEL_DOWNLOAD_ERROR_EVENT: &str = "model-download-error";

// SHA256 checksums from huggingface.co/ggerganov/whisper.cpp.
// Verify against the HuggingFace model card before shipping.
const LARGE_V3_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin";
const LARGE_V3_TURBO_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";
const LARGE_V3_SHA256: &str =
    "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2";
const LARGE_V3_TURBO_SHA256: &str =
    "4af2b29d7ec8dbbb9e7df5c8e52e4df6da4ec57fde8a6e71a25d70e9ac8dce53";
const LARGE_V3_SIZE_BYTES: u64 = 1_550_000_000;
const LARGE_V3_TURBO_SIZE_BYTES: u64 = 809_000_000;

#[derive(Debug, Serialize, Clone)]
pub struct DownloadProgress {
    pub model: LocalWhisperModel,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percentage: u8,
}

#[derive(Debug, Serialize, Clone)]
pub struct ModelDownloadError {
    pub model: LocalWhisperModel,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LocalModelStatus {
    pub model: LocalWhisperModel,
    pub downloaded: bool,
    pub downloading: bool,
    pub size_bytes: u64,
}

pub fn model_url(model: LocalWhisperModel) -> &'static str {
    match model {
        LocalWhisperModel::LargeV3 => LARGE_V3_URL,
        LocalWhisperModel::LargeV3Turbo => LARGE_V3_TURBO_URL,
    }
}

fn model_sha256(model: LocalWhisperModel) -> &'static str {
    match model {
        LocalWhisperModel::LargeV3 => LARGE_V3_SHA256,
        LocalWhisperModel::LargeV3Turbo => LARGE_V3_TURBO_SHA256,
    }
}

pub fn model_size_bytes(model: LocalWhisperModel) -> u64 {
    match model {
        LocalWhisperModel::LargeV3 => LARGE_V3_SIZE_BYTES,
        LocalWhisperModel::LargeV3Turbo => LARGE_V3_TURBO_SIZE_BYTES,
    }
}

struct DownloadCleanup {
    path: PathBuf,
    committed: bool,
}

impl DownloadCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path, committed: false }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for DownloadCleanup {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Core download logic; accepts an explicit URL so tests can point at a mock server.
pub async fn download_to_dir(
    models_dir: &Path,
    filename: &str,
    url: &str,
    expected_sha256: &str,
    cancel_flag: Arc<AtomicBool>,
    on_progress: impl Fn(u64, u64) + Send,
) -> Result<(), String> {
    std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;

    let final_path = models_dir.join(filename);
    let part_path = models_dir.join(format!("{}.part", filename));

    let mut cleanup = DownloadCleanup::new(part_path.clone());

    let existing_bytes = if part_path.exists() {
        std::fs::metadata(&part_path).map_err(|e| e.to_string())?.len()
    } else {
        0
    };

    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if existing_bytes > 0 {
        request = request.header("Range", format!("bytes={}-", existing_bytes));
    }

    let response = request.send().await.map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }

    // 206 Partial Content means the server honoured the Range header.
    // 200 means it returned the full file — discard any existing .part data.
    let actual_existing = if status.as_u16() == 206 { existing_bytes } else { 0 };

    let content_length = response.content_length().unwrap_or(0);
    let total_bytes = actual_existing + content_length;

    let mut hasher = Sha256::new();
    if actual_existing > 0 {
        let mut existing_file = std::fs::File::open(&part_path).map_err(|e| e.to_string())?;
        let mut buf = [0u8; 65536];
        loop {
            let n = existing_file.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    }

    let mut file = if actual_existing > 0 {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&part_path)
            .map_err(|e| e.to_string())?
    } else {
        std::fs::File::create(&part_path).map_err(|e| e.to_string())?
    };

    let mut downloaded = actual_existing;
    let mut stream = response.bytes_stream();

    while let Some(result) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err("Cancelled".to_string());
        }
        let chunk = result.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total_bytes);
    }

    drop(file);

    let computed = hex_encode(&hasher.finalize());
    if computed != expected_sha256 {
        return Err(format!("SHA256 mismatch: expected {expected_sha256}, got {computed}"));
    }

    std::fs::rename(&part_path, &final_path).map_err(|e| e.to_string())?;
    cleanup.commit();
    Ok(())
}

pub async fn download_model(
    app: AppHandle,
    model: LocalWhisperModel,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_dir = data_dir.join("models");
    let app_clone = app.clone();
    download_to_dir(
        &models_dir,
        model.filename(),
        model_url(model),
        model_sha256(model),
        cancel_flag,
        move |downloaded, total| {
            let percentage = if total > 0 {
                ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u8
            } else {
                0
            };
            let _ = app_clone.emit(
                MODEL_DOWNLOAD_PROGRESS_EVENT,
                DownloadProgress { model, bytes_downloaded: downloaded, total_bytes: total, percentage },
            );
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn sha256_of(data: &[u8]) -> String {
        hex_encode(&Sha256::digest(data))
    }

    #[tokio::test]
    async fn completed_download_renames_part_to_bin() {
        let dir = tempfile::TempDir::new().unwrap();
        let content = b"hello model";
        let expected_hash = sha256_of(content);

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/model.bin")
            .with_status(200)
            .with_body(content)
            .create_async()
            .await;

        let url = format!("{}/model.bin", server.url());
        let cancel = Arc::new(AtomicBool::new(false));
        download_to_dir(dir.path(), "model.bin", &url, &expected_hash, cancel, |_, _| {})
            .await
            .unwrap();

        assert!(dir.path().join("model.bin").exists(), ".bin file must exist after success");
        assert!(!dir.path().join("model.bin.part").exists(), ".part file must be removed after success");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn sha256_mismatch_cleans_up_part_and_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let content = b"wrong content";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/model.bin")
            .with_status(200)
            .with_body(content)
            .create_async()
            .await;

        let url = format!("{}/model.bin", server.url());
        let cancel = Arc::new(AtomicBool::new(false));
        let result =
            download_to_dir(dir.path(), "model.bin", &url, wrong_hash, cancel, |_, _| {}).await;

        assert!(result.is_err(), "should return an error on SHA256 mismatch");
        assert!(result.unwrap_err().contains("SHA256 mismatch"));
        assert!(!dir.path().join("model.bin").exists(), ".bin must not exist after mismatch");
        assert!(!dir.path().join("model.bin.part").exists(), ".part must be cleaned up after mismatch");
    }

    #[tokio::test]
    async fn cancellation_leaves_no_bin_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let content = b"some model data";

        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/model.bin")
            .with_status(200)
            .with_body(content)
            .create_async()
            .await;

        let url = format!("{}/model.bin", server.url());
        let cancel = Arc::new(AtomicBool::new(true)); // pre-cancelled
        let result =
            download_to_dir(dir.path(), "model.bin", &url, "any", cancel, |_, _| {}).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cancelled");
        assert!(!dir.path().join("model.bin").exists(), ".bin must not exist after cancellation");
        assert!(!dir.path().join("model.bin.part").exists(), ".part must be cleaned up after cancellation");
    }

    #[tokio::test]
    async fn resumed_download_sends_correct_range_header() {
        let dir = tempfile::TempDir::new().unwrap();
        let existing = b"first half ";
        let rest = b"second half";
        let full: Vec<u8> = existing.iter().chain(rest.iter()).copied().collect();
        let expected_hash = sha256_of(&full);

        // Pre-populate the .part file with the first half
        std::fs::write(dir.path().join("model.bin.part"), existing).unwrap();

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/model.bin")
            .match_header("range", format!("bytes={}-", existing.len()).as_str())
            .with_status(206)
            .with_body(rest)
            .create_async()
            .await;

        let url = format!("{}/model.bin", server.url());
        let cancel = Arc::new(AtomicBool::new(false));
        download_to_dir(dir.path(), "model.bin", &url, &expected_hash, cancel, |_, _| {})
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(dir.path().join("model.bin").exists());
        assert!(!dir.path().join("model.bin.part").exists());
    }
}
