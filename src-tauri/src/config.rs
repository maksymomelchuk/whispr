use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<String>,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: "AltRight".to_string(),
            modifiers: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

pub fn default_replacements() -> Vec<Replacement> {
    [
        ("dot", "."),
        ("slash", "/"),
        ("dash", "-"),
        ("underscore", "_"),
        ("at sign", "@"),
        ("comma", ","),
        ("colon", ":"),
        ("semicolon", ";"),
        ("question mark", "?"),
        ("exclamation mark", "!"),
    ]
    .into_iter()
    .map(|(from, to)| Replacement {
        from: from.to_string(),
        to: to.to_string(),
    })
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramSettings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub smart_format: bool,
    #[serde(default)]
    pub dictation: bool,
    #[serde(default)]
    pub numerals: bool,
    #[serde(default)]
    pub keyterms: Vec<String>,
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for DeepgramSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            smart_format: false,
            dictation: false,
            numerals: false,
            keyterms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    #[default]
    Deepgram,
    Groq,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroqModel {
    WhisperLargeV3,
    #[default]
    WhisperLargeV3Turbo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqSettings {
    #[serde(default)]
    pub model: GroqModel,
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for GroqSettings {
    fn default() -> Self {
        Self {
            model: GroqModel::default(),
            language: default_language(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupAuthMode {
    #[default]
    ApiKey,
    Oauth,
}

pub const DEFAULT_CLEANUP_MIN_WORDS: usize = 9;
pub const DEFAULT_CLEANUP_MIN_DURATION_MS: u64 = 3000;

fn default_cleanup_min_words() -> usize {
    DEFAULT_CLEANUP_MIN_WORDS
}

fn default_cleanup_min_duration_ms() -> u64 {
    DEFAULT_CLEANUP_MIN_DURATION_MS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCleanupSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auth_mode: CleanupAuthMode,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    /// Long-lived `sk-ant-oat…` token from `claude setup-token`. Sent as
    /// `Authorization: Bearer` against the OAuth-gated Messages endpoint —
    /// outside Anthropic's recommended path; see README for caveats.
    #[serde(default)]
    pub anthropic_oauth_token: Option<String>,
    /// Minimum word count at which cleanup runs. Below this, dictations paste
    /// raw to preserve snappiness for short utterances.
    #[serde(default = "default_cleanup_min_words")]
    pub min_words: usize,
    /// Minimum spoken duration (ms) at which cleanup runs.
    #[serde(default = "default_cleanup_min_duration_ms")]
    pub min_duration_ms: u64,
}

impl Default for AiCleanupSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_mode: CleanupAuthMode::default(),
            anthropic_api_key: None,
            anthropic_oauth_token: None,
            min_words: DEFAULT_CLEANUP_MIN_WORDS,
            min_duration_ms: DEFAULT_CLEANUP_MIN_DURATION_MS,
        }
    }
}

fn default_true() -> bool {
    true
}

/// `None` = unlimited, `Some(0)` = off (no history kept), `Some(n)` = keep n.
fn default_history_limit() -> Option<usize> {
    Some(5)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Legacy single-provider field. Read on load for migration into
    /// `deepgram_api_key`, then cleared so it drops out of subsequent saves.
    /// New code should not write to this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub transcription_provider: TranscriptionProvider,
    #[serde(default)]
    pub deepgram_api_key: Option<String>,
    #[serde(default)]
    pub groq_api_key: Option<String>,
    #[serde(default)]
    pub shortcut: Shortcut,
    #[serde(default = "default_replacements")]
    pub replacements: Vec<Replacement>,
    #[serde(default)]
    pub deepgram: DeepgramSettings,
    #[serde(default)]
    pub groq: GroqSettings,
    #[serde(default)]
    pub ai_cleanup: AiCleanupSettings,
    #[serde(default)]
    pub input_device: Option<String>,
    #[serde(default = "default_true")]
    pub pause_media_on_record: bool,
    #[serde(default = "default_history_limit")]
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub show_in_dock: bool,
    #[serde(default = "default_true")]
    pub show_live_preview: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: None,
            transcription_provider: TranscriptionProvider::default(),
            deepgram_api_key: None,
            groq_api_key: None,
            shortcut: Shortcut::default(),
            replacements: default_replacements(),
            deepgram: DeepgramSettings::default(),
            groq: GroqSettings::default(),
            ai_cleanup: AiCleanupSettings::default(),
            input_device: None,
            pause_media_on_record: true,
            history_limit: default_history_limit(),
            show_in_dock: false,
            show_live_preview: true,
        }
    }
}

/// One-way migration from the legacy single-`api_key` schema to per-provider
/// keys. Returns `true` if any change was made so the caller can re-save.
///
/// Rules:
/// - If a non-empty legacy `api_key` is present and `deepgram_api_key` is
///   empty, copy the legacy value across.
/// - Always clear `api_key` so the field stops appearing in serialized form.
/// - Never overwrite an existing `deepgram_api_key`.
fn migrate(s: &mut Settings) -> bool {
    let Some(legacy) = s.api_key.take() else {
        return false;
    };
    let deepgram_already_set = s
        .deepgram_api_key
        .as_deref()
        .is_some_and(|k| !k.is_empty());
    if !legacy.is_empty() && !deepgram_already_set {
        s.deepgram_api_key = Some(legacy);
    }
    true
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(SETTINGS_FILE))
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Settings path error: {e}");
            return Settings::default();
        }
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    let mut settings: Settings = serde_json::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("Failed to parse {path:?}, using defaults: {e}");
        Settings::default()
    });
    if migrate(&mut settings) {
        if let Err(e) = save(app, &settings) {
            eprintln!("Failed to re-save migrated settings: {e}");
        }
    }
    settings
}

/// Convenience for the many `load → mutate → save` setter commands.
pub fn update<F: FnOnce(&mut Settings)>(
    app: &tauri::AppHandle,
    f: F,
) -> Result<(), String> {
    let mut settings = load(app);
    f(&mut settings);
    save(app, &settings)
}

/// Empty strings come from cleared inputs and should be persisted as `None`.
pub fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn save(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write {path:?}: {e}"))?;
    // Defense-in-depth: API keys live in this file. Tighten to user-only
    // read/write even though the parent dir is already 0700 on macOS.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_expected_provider_and_groq_defaults() {
        let s = Settings::default();
        assert_eq!(s.transcription_provider, TranscriptionProvider::Deepgram);
        assert_eq!(s.groq.model, GroqModel::WhisperLargeV3Turbo);
        assert_eq!(s.groq.language, "en");
        assert!(s.deepgram_api_key.is_none());
        assert!(s.groq_api_key.is_none());
    }

    #[test]
    fn provider_enum_serializes_as_snake_case() {
        let s = serde_json::to_string(&TranscriptionProvider::Deepgram).unwrap();
        assert_eq!(s, "\"deepgram\"");
        let s = serde_json::to_string(&TranscriptionProvider::Groq).unwrap();
        assert_eq!(s, "\"groq\"");
    }

    #[test]
    fn groq_model_serializes_as_snake_case() {
        let s = serde_json::to_string(&GroqModel::WhisperLargeV3).unwrap();
        assert_eq!(s, "\"whisper_large_v3\"");
        let s = serde_json::to_string(&GroqModel::WhisperLargeV3Turbo).unwrap();
        assert_eq!(s, "\"whisper_large_v3_turbo\"");
    }

    #[test]
    fn migration_copies_legacy_api_key_to_deepgram_and_drops_legacy_field() {
        let legacy = r#"{"api_key": "dg-legacy-key"}"#;
        let mut s: Settings = serde_json::from_str(legacy).unwrap();
        assert_eq!(s.api_key.as_deref(), Some("dg-legacy-key"));
        assert!(s.deepgram_api_key.is_none());

        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.deepgram_api_key.as_deref(), Some("dg-legacy-key"));
        assert!(s.api_key.is_none());

        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(
            v.get("api_key").is_none(),
            "legacy api_key must drop out of subsequent saves, got: {reserialized}"
        );
        assert_eq!(
            v.get("deepgram_api_key").and_then(|x| x.as_str()),
            Some("dg-legacy-key")
        );
    }

    #[test]
    fn migration_preserves_existing_deepgram_api_key() {
        let json = r#"{"api_key": "old-key", "deepgram_api_key": "new-key"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();

        let changed = migrate(&mut s);
        assert!(changed, "legacy api_key should be cleared on save");
        assert_eq!(
            s.deepgram_api_key.as_deref(),
            Some("new-key"),
            "deepgram_api_key must be left untouched when already set"
        );
        assert!(s.api_key.is_none());

        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(v.get("api_key").is_none());
        assert_eq!(
            v.get("deepgram_api_key").and_then(|x| x.as_str()),
            Some("new-key")
        );
    }

    #[test]
    fn migration_is_noop_for_fresh_settings() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(!changed);
        assert!(s.api_key.is_none());
        assert!(s.deepgram_api_key.is_none());
    }

    #[test]
    fn migration_clears_empty_legacy_api_key() {
        let json = r#"{"api_key": ""}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        assert!(s.api_key.is_none());
        assert!(
            s.deepgram_api_key.is_none(),
            "empty legacy key should not populate deepgram_api_key"
        );
    }

    #[test]
    fn legacy_settings_round_trip_loses_only_api_key_field() {
        let legacy = r#"{
            "api_key": "dg-key",
            "shortcut": {"key": "AltRight", "modifiers": []},
            "deepgram": {
                "language": "fr",
                "smart_format": true,
                "dictation": false,
                "numerals": false,
                "keyterms": []
            }
        }"#;
        let mut s: Settings = serde_json::from_str(legacy).unwrap();
        migrate(&mut s);
        assert_eq!(s.deepgram.language, "fr");
        assert!(s.deepgram.smart_format);
        assert_eq!(s.deepgram_api_key.as_deref(), Some("dg-key"));
        assert_eq!(s.transcription_provider, TranscriptionProvider::Deepgram);
        assert_eq!(s.groq.model, GroqModel::WhisperLargeV3Turbo);
        assert_eq!(s.groq.language, "en");
    }
}
