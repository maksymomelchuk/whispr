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

const PARAKEET_BASE_URL: &str =
    "https://huggingface.co/whispr-app/parakeet-tdt-0.6b-onnx/resolve/main";
const PARAKEET_ENCODER_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const PARAKEET_DECODER_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const PARAKEET_JOINER_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const PARAKEET_VOCAB_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const PARAKEET_ENCODER_SIZE_BYTES: u64 = 394_000_000;
const PARAKEET_DECODER_SIZE_BYTES: u64 = 100_000_000;
const PARAKEET_JOINER_SIZE_BYTES: u64 = 80_000_000;
const PARAKEET_VOCAB_SIZE_BYTES: u64 = 500_000;

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
        LocalWhisperModel::Parakeet => ModelSpec {
            files: vec![
                ModelFile {
                    filename: "parakeet-encoder.onnx".to_string(),
                    url: format!("{PARAKEET_BASE_URL}/encoder.onnx"),
                    sha256: PARAKEET_ENCODER_SHA256.to_string(),
                    size_bytes: PARAKEET_ENCODER_SIZE_BYTES,
                },
                ModelFile {
                    filename: "parakeet-decoder.onnx".to_string(),
                    url: format!("{PARAKEET_BASE_URL}/decoder.onnx"),
                    sha256: PARAKEET_DECODER_SHA256.to_string(),
                    size_bytes: PARAKEET_DECODER_SIZE_BYTES,
                },
                ModelFile {
                    filename: "parakeet-joiner.onnx".to_string(),
                    url: format!("{PARAKEET_BASE_URL}/joiner.onnx"),
                    sha256: PARAKEET_JOINER_SHA256.to_string(),
                    size_bytes: PARAKEET_JOINER_SIZE_BYTES,
                },
                ModelFile {
                    filename: "parakeet-vocab.json".to_string(),
                    url: format!("{PARAKEET_BASE_URL}/vocab.json"),
                    sha256: PARAKEET_VOCAB_SHA256.to_string(),
                    size_bytes: PARAKEET_VOCAB_SIZE_BYTES,
                },
            ],
        },
    }
}

pub fn total_size_bytes(spec: &ModelSpec) -> u64 {
    spec.files.iter().map(|f| f.size_bytes).sum()
}

pub fn is_placeholder_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b == b'0')
}

pub fn all_files_present(spec: &ModelSpec, models_dir: &Path) -> bool {
    spec.files.iter().all(|f| models_dir.join(&f.filename).exists())
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
    fn parakeet_spec_is_four_files() {
        let spec = catalog_for(LocalWhisperModel::Parakeet);
        assert_eq!(spec.files.len(), 4);
        let names: Vec<&str> = spec.files.iter().map(|f| f.filename.as_str()).collect();
        assert!(names.contains(&"parakeet-encoder.onnx"));
        assert!(names.contains(&"parakeet-decoder.onnx"));
        assert!(names.contains(&"parakeet-joiner.onnx"));
        assert!(names.contains(&"parakeet-vocab.json"));
    }

    #[test]
    fn parakeet_total_size_bytes_sums_all_four_files() {
        let spec = catalog_for(LocalWhisperModel::Parakeet);
        let expected = PARAKEET_ENCODER_SIZE_BYTES
            + PARAKEET_DECODER_SIZE_BYTES
            + PARAKEET_JOINER_SIZE_BYTES
            + PARAKEET_VOCAB_SIZE_BYTES;
        assert_eq!(total_size_bytes(&spec), expected);
    }

    #[test]
    fn parakeet_files_to_delete_covers_all_four_files_and_parts() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("parakeet-encoder.onnx"), b"enc").unwrap();
        std::fs::write(dir.path().join("parakeet-decoder.onnx.part"), b"dec-partial").unwrap();
        let spec = catalog_for(LocalWhisperModel::Parakeet);
        let mut paths = files_to_delete(&spec, dir.path());
        paths.sort();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("parakeet-encoder.onnx")));
        assert!(paths.iter().any(|p| p.ends_with("parakeet-decoder.onnx.part")));
    }

    #[test]
    fn parakeet_encoder_is_primary_file() {
        let spec = catalog_for(LocalWhisperModel::Parakeet);
        assert_eq!(spec.files[0].filename, "parakeet-encoder.onnx");
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

    #[test]
    fn is_placeholder_hash_true_for_64_zeros() {
        assert!(is_placeholder_hash(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn is_placeholder_hash_false_for_real_hash() {
        assert!(!is_placeholder_hash(
            "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2"
        ));
    }

    #[test]
    fn is_placeholder_hash_false_for_empty_string() {
        assert!(!is_placeholder_hash(""));
    }

    #[test]
    fn all_files_present_true_when_all_exist() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), b"data").unwrap();
        std::fs::write(dir.path().join("decoder.onnx"), b"data").unwrap();
        assert!(all_files_present(&two_file_spec(), dir.path()));
    }

    #[test]
    fn all_files_present_false_when_any_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("encoder.onnx"), b"data").unwrap();
        // decoder.onnx intentionally absent
        assert!(!all_files_present(&two_file_spec(), dir.path()));
    }

    #[test]
    fn all_files_present_false_when_none_exist() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(!all_files_present(&two_file_spec(), dir.path()));
    }

}
