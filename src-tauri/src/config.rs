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
        ("at", "@"),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiCleanupSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
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
    pub api_key: Option<String>,
    #[serde(default)]
    pub shortcut: Shortcut,
    #[serde(default = "default_replacements")]
    pub replacements: Vec<Replacement>,
    #[serde(default)]
    pub deepgram: DeepgramSettings,
    #[serde(default)]
    pub ai_cleanup: AiCleanupSettings,
    #[serde(default)]
    pub input_device: Option<String>,
    #[serde(default = "default_true")]
    pub pause_media_on_record: bool,
    #[serde(default = "default_history_limit")]
    pub history_limit: Option<usize>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: None,
            shortcut: Shortcut::default(),
            replacements: default_replacements(),
            deepgram: DeepgramSettings::default(),
            ai_cleanup: AiCleanupSettings::default(),
            input_device: None,
            pause_media_on_record: true,
            history_limit: default_history_limit(),
        }
    }
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
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Failed to parse {path:?}, using defaults: {e}");
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

pub fn save(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write {path:?}: {e}"))?;
    // Defense-in-depth: the Deepgram API key lives in this file. Tighten to
    // user-only read/write even though the parent dir is already 0700 on macOS.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
