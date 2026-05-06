//! Three rolling token counters for the LLM cleanup pass: today, this
//! calendar month, and all-time.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use time::OffsetDateTime;

const CLEANUP_STATS_FILE: &str = "cleanup_stats.json";
pub const CLEANUP_STATS_UPDATED_EVENT: &str = "cleanup-stats-updated";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeriodCounter {
    /// Period key — "YYYY-MM-DD" for today, "YYYY-MM" for the month
    /// counter. When this no longer matches the current period, the tokens
    /// reset to zero.
    pub period: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalCounter {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupStats {
    #[serde(default)]
    pub today: PeriodCounter,
    #[serde(default)]
    pub month: PeriodCounter,
    #[serde(default)]
    pub overall: TotalCounter,
}

fn stats_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(CLEANUP_STATS_FILE))
}

fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn current_day_key(now: &OffsetDateTime) -> String {
    now.date().to_string()
}

fn current_month_key(now: &OffsetDateTime) -> String {
    format!("{:04}-{:02}", now.year(), u8::from(now.month()))
}

/// Zero out a period counter (and stamp it with `current_key`) if its
/// stored key no longer matches the current period.
fn rollover(counter: &mut PeriodCounter, current_key: &str) {
    if counter.period != current_key {
        counter.period = current_key.to_string();
        counter.input_tokens = 0;
        counter.output_tokens = 0;
    }
}

fn read_from_disk(app: &tauri::AppHandle) -> CleanupStats {
    let path = match stats_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[cleanup_stats] path error: {e}");
            return CleanupStats::default();
        }
    };
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!(
                "[cleanup_stats] parse error on {path:?}, starting fresh: {e}"
            );
            CleanupStats::default()
        }),
        Err(_) => CleanupStats::default(),
    }
}

fn save(app: &tauri::AppHandle, stats: &CleanupStats) -> Result<(), String> {
    let path = stats_path(app)?;
    let json = serde_json::to_string_pretty(stats).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write {path:?}: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn load(app: &tauri::AppHandle) -> CleanupStats {
    let mut stats = read_from_disk(app);
    let now = local_now();
    rollover(&mut stats.today, &current_day_key(&now));
    rollover(&mut stats.month, &current_month_key(&now));
    stats
}

pub fn record(app: &tauri::AppHandle, input_tokens: u64, output_tokens: u64) {
    let mut stats = read_from_disk(app);
    let now = local_now();
    rollover(&mut stats.today, &current_day_key(&now));
    rollover(&mut stats.month, &current_month_key(&now));

    stats.today.input_tokens = stats.today.input_tokens.saturating_add(input_tokens);
    stats.today.output_tokens = stats.today.output_tokens.saturating_add(output_tokens);
    stats.month.input_tokens = stats.month.input_tokens.saturating_add(input_tokens);
    stats.month.output_tokens = stats.month.output_tokens.saturating_add(output_tokens);
    stats.overall.input_tokens = stats.overall.input_tokens.saturating_add(input_tokens);
    stats.overall.output_tokens = stats.overall.output_tokens.saturating_add(output_tokens);

    if let Err(e) = save(app, &stats) {
        eprintln!("[cleanup_stats] save failed: {e}");
        return;
    }
    let _ = app.emit(CLEANUP_STATS_UPDATED_EVENT, ());
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    save(app, &CleanupStats::default())
}
