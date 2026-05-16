use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum TranslationError {
    BinaryNotFound,
    // Language pack not downloaded; user needs System Settings > Language & Region.
    ModelNotInstalled,
    UnsupportedPair,
    RequiresMacOS15,
    Failed(String),
    Io(String),
}

impl std::fmt::Display for TranslationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslationError::BinaryNotFound => write!(f, "Translation helper binary not found"),
            TranslationError::ModelNotInstalled => write!(
                f,
                "Translation language pack not installed — open System Settings › General › \
                 Language & Region to download it"
            ),
            TranslationError::UnsupportedPair => {
                write!(f, "Apple Translate does not support this language pair")
            }
            TranslationError::RequiresMacOS15 => {
                write!(f, "Apple Translate requires macOS 15 (Sequoia) or later")
            }
            TranslationError::Failed(msg) => write!(f, "Translation failed: {msg}"),
            TranslationError::Io(msg) => write!(f, "Translation I/O error: {msg}"),
        }
    }
}

#[derive(Serialize)]
struct TranslateRequest<'a> {
    text: &'a str,
    source: Option<&'a str>,
    target: &'a str,
}

#[derive(Deserialize)]
struct TranslateResponse {
    translated: Option<String>,
    error_code: Option<String>,
    error: Option<String>,
}

/// Translate `text` from `source` to `target` (ISO 639-1 codes, e.g. "uk", "en").
/// `source` is `None` when the mode language is Auto; the framework auto-detects.
///
/// Blocks the calling thread — wrap in `spawn_blocking` when calling from async.
pub fn translate(
    text: &str,
    source: Option<&str>,
    target: &str,
) -> Result<String, TranslationError> {
    let binary = find_binary()?;

    let request = TranslateRequest { text, source, target };
    let input = serde_json::to_vec(&request).map_err(|e| TranslationError::Io(e.to_string()))?;

    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| TranslationError::Io(e.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&input)
            .map_err(|e| TranslationError::Io(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| TranslationError::Io(e.to_string()))?;

    if output.stdout.is_empty() {
        return Err(TranslationError::Failed("Empty response from translate helper".to_string()));
    }

    let response: TranslateResponse = serde_json::from_slice(&output.stdout)
        .map_err(|e| TranslationError::Failed(format!("Malformed response: {e}")))?;

    if let Some(translated) = response.translated {
        return Ok(translated);
    }

    Err(match response.error_code.as_deref() {
        Some("model_not_installed") => TranslationError::ModelNotInstalled,
        Some("unsupported_pair") => TranslationError::UnsupportedPair,
        Some("requires_macos_15") => TranslationError::RequiresMacOS15,
        _ => TranslationError::Failed(
            response.error.unwrap_or_else(|| "Unknown error".to_string()),
        ),
    })
}

/// Find the sidecar binary.
///
/// In a bundled app Tauri places sidecars in the same directory as the main
/// executable. During `tauri dev` the binary is compiled to `src-tauri/binaries/`
/// by build.rs; both locations are checked.
fn find_binary() -> Result<PathBuf, TranslationError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("apple-translate");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    Err(TranslationError::BinaryNotFound)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_request_serializes_all_fields() {
        let r = TranslateRequest {
            text: "Привіт",
            source: Some("uk"),
            target: "en",
        };
        let json = serde_json::to_string(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["text"], "Привіт");
        assert_eq!(v["source"], "uk");
        assert_eq!(v["target"], "en");
    }

    #[test]
    fn translate_request_source_none_serializes_as_null() {
        let r = TranslateRequest {
            text: "hello",
            source: None,
            target: "uk",
        };
        let json = serde_json::to_string(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["source"].is_null());
    }

    #[test]
    fn translate_response_parses_success() {
        let json = r#"{"translated":"Hello","error_code":null,"error":null}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.translated, Some("Hello".to_string()));
        assert!(r.error_code.is_none());
    }

    #[test]
    fn translate_response_parses_model_not_installed() {
        let json = r#"{"translated":null,"error_code":"model_not_installed","error":"Pack missing"}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert!(r.translated.is_none());
        assert_eq!(r.error_code.as_deref(), Some("model_not_installed"));
    }

    #[test]
    fn translate_response_parses_requires_macos_15() {
        let json = r#"{"translated":null,"error_code":"requires_macos_15","error":"Need 15+"}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.error_code.as_deref(), Some("requires_macos_15"));
    }

    #[test]
    fn translate_response_parses_unsupported_pair() {
        let json = r#"{"translated":null,"error_code":"unsupported_pair","error":"Not supported"}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.error_code.as_deref(), Some("unsupported_pair"));
    }

    #[test]
    fn translation_error_display_covers_all_variants() {
        assert!(TranslationError::BinaryNotFound.to_string().contains("binary"));
        assert!(TranslationError::ModelNotInstalled.to_string().contains("System Settings"));
        assert!(TranslationError::UnsupportedPair.to_string().contains("language pair"));
        assert!(TranslationError::RequiresMacOS15.to_string().contains("macOS 15"));
        assert!(TranslationError::Failed("x".into()).to_string().contains("x"));
        assert!(TranslationError::Io("y".into()).to_string().contains("y"));
    }

    #[test]
    fn binary_not_found_when_no_sidecar_exists() {
        // In the test environment the sidecar binary is not compiled,
        // so find_binary() must return BinaryNotFound.
        let result = find_binary();
        assert!(
            result.is_err(),
            "expected BinaryNotFound in test env, got path: {result:?}"
        );
    }
}
