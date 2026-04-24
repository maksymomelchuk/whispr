use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const HISTORY_FILE: &str = "history.json";
const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp: i64,
}

fn history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(HISTORY_FILE))
}

pub fn load(app: &tauri::AppHandle) -> Vec<HistoryEntry> {
    let path = match history_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[history] path error: {e}");
            return Vec::new();
        }
    };
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("[history] parse error on {path:?}, starting fresh: {e}");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

fn save(app: &tauri::AppHandle, entries: &[HistoryEntry]) -> Result<(), String> {
    let path = history_path(app)?;
    let json = serde_json::to_string_pretty(entries).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {path:?}: {e}"))?;
    // Transcripts may include anything the user dictated — passwords, PII,
    // secrets. Lock the file to the owning user.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Prepend a new entry (newest-first) and trim to MAX_ENTRIES. Trailing
/// whitespace from the dictation pipeline is stripped before persisting —
/// the pasted text keeps its space but history reads cleaner without it.
pub fn append(app: &tauri::AppHandle, text: &str) -> Result<Vec<HistoryEntry>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(load(app));
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut entries = load(app);
    entries.insert(
        0,
        HistoryEntry {
            text: trimmed.to_string(),
            timestamp,
        },
    );
    entries.truncate(MAX_ENTRIES);
    save(app, &entries)?;
    Ok(entries)
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    save(app, &[])
}
