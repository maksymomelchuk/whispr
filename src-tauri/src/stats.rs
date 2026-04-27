use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use time::OffsetDateTime;

const STATS_FILE: &str = "stats.json";
const RETENTION_DAYS: i64 = 365;

pub const STATS_UPDATED_EVENT: &str = "stats-updated";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsRow {
    pub date: String,
    pub words: u64,
    pub dictations: u32,
    pub total_seconds: u32,
}

fn stats_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(STATS_FILE))
}

pub fn load(app: &tauri::AppHandle) -> Vec<StatsRow> {
    let path = match stats_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[stats] path error: {e}");
            return Vec::new();
        }
    };
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("[stats] parse error on {path:?}, starting fresh: {e}");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

fn save(app: &tauri::AppHandle, rows: &[StatsRow]) -> Result<(), String> {
    let path = stats_path(app)?;
    let json = serde_json::to_string_pretty(rows).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {path:?}: {e}"))?;
    // Counts aren't sensitive the way transcripts are, but match the
    // 0o600 lockdown of the rest of app data for consistency.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Tauri spawns a multi-threaded runtime; reading the local timezone via
/// libc is racy in that setting and the `time` crate may refuse. UTC is the
/// safe fallback — the resulting day boundary will just shift, not corrupt.
fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

pub fn record(app: &tauri::AppHandle, words: u64, seconds: u32) {
    let today = local_now().date();
    let today_str = today.to_string();
    let cutoff_str = (today - time::Duration::days(RETENTION_DAYS)).to_string();

    let mut rows = load(app);

    if let Some(row) = rows.iter_mut().find(|r| r.date == today_str) {
        row.words = row.words.saturating_add(words);
        row.dictations = row.dictations.saturating_add(1);
        row.total_seconds = row.total_seconds.saturating_add(seconds);
    } else {
        rows.push(StatsRow {
            date: today_str,
            words,
            dictations: 1,
            total_seconds: seconds,
        });
    }

    rows.retain(|r| r.date >= cutoff_str);
    rows.sort_by(|a, b| a.date.cmp(&b.date));

    if let Err(e) = save(app, &rows) {
        eprintln!("[stats] save failed: {e}");
        return;
    }
    let _ = app.emit(STATS_UPDATED_EVENT, ());
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    save(app, &[])
}
