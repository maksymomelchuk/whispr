use crate::config;
use crate::provider::ProviderModel;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const HISTORY_FILE: &str = "history.json";

pub const HISTORY_UPDATED_EVENT: &str = "history-updated";

/// Outcome of the optional Anthropic cleanup step. Persisted so the History
/// tab can distinguish ran-with-no-change from skipped/failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CleanupStatus {
    Disabled,
    SkippedBelowMinWords,
    SkippedBelowMinDuration,
    NoCredential,
    Ran,
    FailedTimeout,
    FailedTransient(String),
    FailedCredential(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: i64,
    pub speak_duration_ms: u64,
    pub raw_text: String,
    pub replaced_text: String,
    pub final_text: String,
    pub cleanup_status: CleanupStatus,
    // Optional so existing on-disk histories written before this field was added
    // still deserialize. Old entries fall back to a generic label in the UI.
    #[serde(default)]
    pub provider_model: Option<ProviderModel>,
    #[serde(default)]
    pub app_name: Option<String>,
    #[serde(default)]
    pub bundle_id: Option<String>,
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
            eprintln!("[history] dropping incompatible history at {path:?}: {e}");
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

/// Prepend a new entry (newest-first) and trim to the configured history
/// limit.
pub fn append(app: &tauri::AppHandle, entry: HistoryEntry) -> Result<Vec<HistoryEntry>, String> {
    let limit = config::load(app).history_limit;
    if matches!(limit, Some(0)) {
        return Ok(Vec::new());
    }

    let mut entries = load(app);
    entries.insert(0, entry);
    if let Some(max) = limit {
        entries.truncate(max);
    }
    save(app, &entries)?;
    Ok(entries)
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    save(app, &[])
}

/// History is stored newest-first, so this is just the head.
pub fn latest(app: &tauri::AppHandle) -> Option<HistoryEntry> {
    load(app).into_iter().next()
}

/// The text that was originally pasted for `entry` — `final_text` if cleanup
/// ran, otherwise `replaced_text`. Matches the resolution used by the
/// dictation pipeline so "paste latest" reproduces what the user already saw.
pub fn pasted_text(entry: &HistoryEntry) -> &str {
    if matches!(entry.cleanup_status, CleanupStatus::Ran) {
        &entry.final_text
    } else {
        &entry.replaced_text
    }
}

/// Apply the limit to the existing on-disk history. Used when the user
/// changes the limit setting so the change takes effect immediately rather
/// than waiting for the next dictation.
pub fn enforce_limit(app: &tauri::AppHandle, limit: Option<usize>) -> Result<(), String> {
    match limit {
        Some(0) => clear(app),
        Some(n) => {
            let mut entries = load(app);
            if entries.len() > n {
                entries.truncate(n);
                save(app, &entries)?;
            }
            Ok(())
        }
        None => Ok(()),
    }
}

pub fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
