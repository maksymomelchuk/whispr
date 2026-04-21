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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub api_key: Option<String>,
    #[serde(default)]
    pub shortcut: Shortcut,
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
    fs::write(&path, json).map_err(|e| format!("Failed to write {path:?}: {e}"))
}
