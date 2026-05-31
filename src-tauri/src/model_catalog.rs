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

pub fn total_size_bytes(spec: &ModelSpec) -> u64 {
    spec.files.iter().map(|f| f.size_bytes).sum()
}

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
    fn total_size_bytes_sums_all_files() {
        let spec = two_file_spec();
        assert_eq!(total_size_bytes(&spec), 1000);
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

}
