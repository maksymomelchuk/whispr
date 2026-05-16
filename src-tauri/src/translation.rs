use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum TranslationError {
    BinaryNotFound,
    // Language pack not downloaded; user needs System Settings > Language & Region.
    ModelNotInstalled { from: String, to: String },
    UnsupportedPair,
    RequiresMacOS26,
    SourceRequired,
    Failed(String),
    Io(String),
}

/// Best-effort language code → human name for error messages. Falls back to
/// the code itself for codes not in this short table.
fn language_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "uk" => "Ukrainian",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "pl" => "Polish",
        "ru" => "Russian",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh" => "Chinese",
        "ar" => "Arabic",
        "tr" => "Turkish",
        "nl" => "Dutch",
        _ => code,
    }
}

impl std::fmt::Display for TranslationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslationError::BinaryNotFound => write!(f, "Translation helper binary not found"),
            TranslationError::ModelNotInstalled { from, to } => write!(
                f,
                "Translation pack missing. Add {} and {} in System Settings.",
                language_name(from),
                language_name(to),
            ),
            TranslationError::UnsupportedPair => {
                write!(f, "Apple Translate does not support this language pair")
            }
            TranslationError::RequiresMacOS26 => {
                write!(f, "Apple Translate requires macOS 26 (Tahoe) or later")
            }
            TranslationError::SourceRequired => write!(
                f,
                "Apple Translate requires an explicit source language — set the mode's Spoken \
                 Language to a specific language (not Auto)"
            ),
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
        // Surface swift-side diagnostics (the sidecar writes `[apple-translate]`
        // step logs to stderr) so a hang's last completed step is visible.
        .stderr(Stdio::inherit())
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
        Some("model_not_installed") => TranslationError::ModelNotInstalled {
            from: source.unwrap_or("").to_string(),
            to: target.to_string(),
        },
        Some("unsupported_pair") => TranslationError::UnsupportedPair,
        Some("requires_macos_26") => TranslationError::RequiresMacOS26,
        Some("source_required") => TranslationError::SourceRequired,
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
    fn translate_response_parses_requires_macos_26() {
        let json = r#"{"translated":null,"error_code":"requires_macos_26","error":"Need 26+"}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.error_code.as_deref(), Some("requires_macos_26"));
    }

    #[test]
    fn translate_response_parses_source_required() {
        let json = r#"{"translated":null,"error_code":"source_required","error":"Set source"}"#;
        let r: TranslateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.error_code.as_deref(), Some("source_required"));
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
        assert!(
            TranslationError::ModelNotInstalled {
                from: "uk".to_string(),
                to: "en".to_string(),
            }
            .to_string()
            .contains("System Settings")
        );
        assert!(TranslationError::UnsupportedPair.to_string().contains("language pair"));
        assert!(TranslationError::RequiresMacOS26.to_string().contains("macOS 26"));
        assert!(TranslationError::SourceRequired.to_string().contains("source language"));
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
