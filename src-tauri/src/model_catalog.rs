use crate::provider::LocalWhisperModel;
use std::path::{Path, PathBuf};

const LARGE_V3_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin";
const LARGE_V3_TURBO_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";
const LARGE_V3_SHA256: &str =
    "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2";
const LARGE_V3_TURBO_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const LARGE_V3_SIZE_BYTES: u64 = 3_095_033_483;
const LARGE_V3_TURBO_SIZE_BYTES: u64 = 1_624_555_275;

pub struct ModelFile {
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

pub struct ModelSpec {
    pub files: Vec<ModelFile>,
}

pub struct FileDownloadPlan<'a> {
    pub file: &'a ModelFile,
    pub resume_bytes: u64,
}

pub fn catalog_for(model: LocalWhisperModel) -> ModelSpec {
    match model {
        LocalWhisperModel::LargeV3 => ModelSpec {
            files: vec![ModelFile {
                filename: model.filename().to_string(),
                url: LARGE_V3_URL.to_string(),
                sha256: LARGE_V3_SHA256.to_string(),
                size_bytes: LARGE_V3_SIZE_BYTES,
            }],
        },
        LocalWhisperModel::LargeV3Turbo => ModelSpec {
            files: vec![ModelFile {
                filename: model.filename().to_string(),
                url: LARGE_V3_TURBO_URL.to_string(),
                sha256: LARGE_V3_TURBO_SHA256.to_string(),
                size_bytes: LARGE_V3_TURBO_SIZE_BYTES,
            }],
        },
    }
}

/// Returns a download task for each file that is not yet fully downloaded.
/// Files whose final path already exists are skipped. For files with a
/// `.part` file present, `resume_bytes` reflects how many bytes are already
/// on disk so the caller can issue an HTTP Range request.
pub fn plan_downloads<'a>(spec: &'a ModelSpec, models_dir: &Path) -> Vec<FileDownloadPlan<'a>> {
    spec.files
        .iter()
        .filter(|f| !models_dir.join(&f.filename).exists())
        .map(|f| {
            let part_path = models_dir.join(format!("{}.part", f.filename));
            let resume_bytes = std::fs::metadata(&part_path).map(|m| m.len()).unwrap_or(0);
            FileDownloadPlan { file: f, resume_bytes }
        })
        .collect()
}

pub fn total_size_bytes(spec: &ModelSpec) -> u64 {
    spec.files.iter().map(|f| f.size_bytes).sum()
}

/// Bytes currently on disk for this spec: sum of final files and any `.part`
/// files that represent interrupted downloads.
pub fn disk_usage(spec: &ModelSpec, models_dir: &Path) -> u64 {
    spec.files
        .iter()
        .map(|f| {
            let final_bytes = std::fs::metadata(models_dir.join(&f.filename))
                .map(|m| m.len())
                .unwrap_or(0);
            let part_bytes = std::fs::metadata(models_dir.join(format!("{}.part", f.filename)))
                .map(|m| m.len())
                .unwrap_or(0);
            final_bytes + part_bytes
        })
        .sum()
}

/// All on-disk paths (final and `.part`) belonging to this spec.
pub fn files_to_delete(spec: &ModelSpec, models_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for f in &spec.files {
        let final_path = models_dir.join(&f.filename);
        let part_path = models_dir.join(format!("{}.part", f.filename));
        if final_path.exists() {
            paths.push(final_path);
        }
        if part_path.exists() {
            paths.push(part_path);
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_file_spec() -> ModelSpec {
        ModelSpec {
            files: vec![
                ModelFile {
                    filename: "encoder.onnx".to_string(),
                    url: String::new(),
                    sha256: String::new(),
                    size_bytes: 300,
                },
                ModelFile {
                    filename: "decoder.onnx".to_string(),
                    url: String::new(),
                    sha256: String::new(),
                    size_bytes: 700,
                },
            ],
        }
    }

    #[test]
    fn large_v3_spec_is_single_file() {
        let spec = catalog_for(LocalWhisperModel::LargeV3);
        assert_eq!(spec.files.len(), 1);
        assert_eq!(spec.files[0].filename, "ggml-large-v3.bin");
        assert_eq!(spec.files[0].size_bytes, LARGE_V3_SIZE_BYTES);
    }

    #[test]
    fn large_v3_turbo_spec_is_single_file() {
        let spec = catalog_for(LocalWhisperModel::LargeV3Turbo);
        assert_eq!(spec.files.len(), 1);
        assert_eq!(spec.files[0].filename, "ggml-large-v3-turbo.bin");
        assert_eq!(spec.files[0].size_bytes, LARGE_V3_TURBO_SIZE_BYTES);
    }

    #[test]
    fn plan_downloads_returns_all_files_when_dir_is_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let spec = two_file_spec();
        let plans = plan_downloads(&spec, dir.path());
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].resume_bytes, 0);
        assert_eq!(plans[1].resume_bytes, 0);
    }

    #[test]
    fn plan_downloads_skips_complete_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), b"complete").unwrap();
        let spec = two_file_spec();
        let plans = plan_downloads(&spec, dir.path());
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].file.filename, "decoder.onnx");
    }

    #[test]
    fn plan_downloads_resumes_partial_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let partial = b"first half";
        std::fs::write(dir.path().join("encoder.onnx.part"), partial).unwrap();
        let spec = two_file_spec();
        let plans = plan_downloads(&spec, dir.path());
        assert_eq!(plans.len(), 2);
        let encoder_plan = plans.iter().find(|p| p.file.filename == "encoder.onnx").unwrap();
        assert_eq!(encoder_plan.resume_bytes, partial.len() as u64);
    }

    #[test]
    fn plan_downloads_returns_no_tasks_when_all_complete() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), b"done").unwrap();
        std::fs::write(dir.path().join("decoder.onnx"), b"done").unwrap();
        let spec = two_file_spec();
        let plans = plan_downloads(&spec, dir.path());
        assert!(plans.is_empty());
    }

    #[test]
    fn total_size_bytes_sums_all_files() {
        let spec = two_file_spec();
        assert_eq!(total_size_bytes(&spec), 1000);
    }

    #[test]
    fn disk_usage_counts_final_and_part_bytes() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.path().join("decoder.onnx.part"), vec![0u8; 50]).unwrap();
        let spec = two_file_spec();
        assert_eq!(disk_usage(&spec, dir.path()), 150);
    }

    #[test]
    fn disk_usage_is_zero_when_no_files_on_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let spec = two_file_spec();
        assert_eq!(disk_usage(&spec, dir.path()), 0);
    }

    #[test]
    fn files_to_delete_returns_final_and_part_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), b"done").unwrap();
        std::fs::write(dir.path().join("decoder.onnx.part"), b"partial").unwrap();
        let spec = two_file_spec();
        let mut paths = files_to_delete(&spec, dir.path());
        paths.sort();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("encoder.onnx")));
        assert!(paths.iter().any(|p| p.ends_with("decoder.onnx.part")));
    }

    #[test]
    fn files_to_delete_returns_empty_when_nothing_on_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let spec = two_file_spec();
        assert!(files_to_delete(&spec, dir.path()).is_empty());
    }

    #[test]
    fn plan_downloads_with_single_file_spec_matches_whisper_behavior() {
        let dir = tempfile::TempDir::new().unwrap();
        let spec = catalog_for(LocalWhisperModel::LargeV3Turbo);
        let plans = plan_downloads(&spec, dir.path());
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].file.filename, "ggml-large-v3-turbo.bin");
        assert_eq!(plans[0].resume_bytes, 0);
    }
}
