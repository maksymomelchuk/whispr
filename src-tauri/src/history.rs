use crate::cleanup::AiProviderId;
use crate::config;
use crate::mode::SetId;
use crate::provider::ProviderModel;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

const HISTORY_FILE: &str = "history.json";

pub const HISTORY_UPDATED_EVENT: &str = "history-updated";

static ENTRY_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_entry_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seq = ENTRY_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("entry-{ms}-{seq}")
}

/// Profile settings captured at dictation time so recovery can replay the
/// exact cleanup that ran, even if the user later edits or deletes the profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSnapshot {
    pub cleanup_provider: AiProviderId,
    pub cleanup_model: String,
    pub cleanup_prompt_override: Option<String>,
    pub use_snippets: bool,
    pub correction_set_ids: Vec<SetId>,
}

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
    RecoveredManually,
    FailedTimeout,
    FailedTransient(String),
    FailedCredential(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Stable unique id generated at dictation time. Absent in entries written
    /// before this field was added; those fall back to a composite key in the UI.
    #[serde(default)]
    pub id: String,
    pub timestamp: i64,
    pub speak_duration_ms: u64,
    pub raw_text: String,
    pub replaced_text: String,
    pub final_text: String,
    pub cleanup_status: CleanupStatus,
    /// Profile settings frozen at dictation time for recovery replay.
    #[serde(default)]
    pub profile_snapshot: Option<ProfileSnapshot>,
    // Optional so existing on-disk histories written before this field was added
    // still deserialize. Old entries fall back to a generic label in the UI.
    #[serde(default)]
    pub provider_model: Option<ProviderModel>,
    #[serde(default)]
    pub app_name: Option<String>,
    #[serde(default)]
    pub bundle_id: Option<String>,
    /// Channel flags only — captured content is never persisted.
    #[serde(default)]
    pub context_channels: Vec<String>,
    /// A saved mic recording exists at `recordings/{id}.flac`. Drives the
    /// inline player and the FLAC-deletes-with-the-row retention rule.
    #[serde(default)]
    pub has_audio: bool,
    /// Pinned by the user: exempt from every automatic retention cap and
    /// uncounted against both limits.
    #[serde(default)]
    pub favorite: bool,
}

fn history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(HISTORY_FILE))
}

const RECORDINGS_DIR: &str = "recordings";

pub fn recordings_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?
        .join(RECORDINGS_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings dir: {e}"))?;
    Ok(dir)
}

pub fn recording_path(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(recordings_dir(app)?.join(format!("{id}.flac")))
}

/// Encode the session's mic audio to 16 kHz mono FLAC and persist it locked to
/// the owning user. Called off the paste path — FLAC compression of a long clip
/// can take a noticeable amount of CPU.
pub fn save_recording(
    app: &tauri::AppHandle,
    id: &str,
    samples: &[i16],
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let flac = crate::groq_audio::encode_to_flac_16k_mono(samples, sample_rate, channels)?;
    let path = recording_path(app, id)?;
    fs::write(&path, flac).map_err(|e| format!("write {path:?}: {e}"))?;
    // Recordings can contain anything the user dictated — lock to the owner.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn delete_recording(app: &tauri::AppHandle, id: &str) {
    if let Ok(path) = recording_path(app, id) {
        let _ = fs::remove_file(path);
    }
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

/// Split a newest-first list into the entries to keep and the ids of evicted
/// entries that had audio (so their FLAC can be removed). A record carries its
/// optional audio, so one cap governs both: it counts only non-favorites, and an
/// evicted record drops its FLAC with it. Favorites are always kept and uncounted.
fn apply_retention(
    entries: Vec<HistoryEntry>,
    history_limit: Option<usize>,
) -> (Vec<HistoryEntry>, Vec<String>) {
    let mut evicted_audio = Vec::new();
    let mut seen = 0usize;
    let kept: Vec<HistoryEntry> = entries
        .into_iter()
        .filter(|entry| {
            if entry.favorite {
                return true;
            }
            seen += 1;
            let over = history_limit.is_some_and(|max| seen > max);
            if over && entry.has_audio {
                evicted_audio.push(entry.id.clone());
            }
            !over
        })
        .collect();

    (kept, evicted_audio)
}

fn retain_and_save(
    app: &tauri::AppHandle,
    entries: Vec<HistoryEntry>,
) -> Result<Vec<HistoryEntry>, String> {
    let cfg = config::load(app);
    let (kept, evicted_audio) = apply_retention(entries, cfg.history_limit);
    for id in &evicted_audio {
        delete_recording(app, id);
    }
    save(app, &kept)?;
    Ok(kept)
}

/// Prepend a new entry (newest-first) and apply the retention cap, deleting
/// the FLAC of any entry that gets evicted.
pub fn append(app: &tauri::AppHandle, entry: HistoryEntry) -> Result<Vec<HistoryEntry>, String> {
    let mut entries = load(app);
    entries.insert(0, entry);
    retain_and_save(app, entries)
}

pub fn clear(app: &tauri::AppHandle) -> Result<(), String> {
    for entry in load(app) {
        if entry.has_audio {
            delete_recording(app, &entry.id);
        }
    }
    save(app, &[])
}

/// Toggle the favorite flag, then re-apply retention: un-favoriting can push an
/// old entry back under a cap, favoriting exempts it.
pub fn set_favorite(app: &tauri::AppHandle, id: &str, favorite: bool) -> Result<(), String> {
    let mut entries = load(app);
    let entry = entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("history entry not found: {id}"))?;
    entry.favorite = favorite;
    retain_and_save(app, entries)?;
    Ok(())
}

/// History is stored newest-first, so this is just the head.
pub fn latest(app: &tauri::AppHandle) -> Option<HistoryEntry> {
    load(app).into_iter().next()
}

/// The text that was originally pasted for `entry` — `final_text` if cleanup
/// ran or the entry was recovered, otherwise `replaced_text`.
pub fn pasted_text(entry: &HistoryEntry) -> &str {
    match entry.cleanup_status {
        CleanupStatus::Ran | CleanupStatus::RecoveredManually => &entry.final_text,
        _ => &entry.replaced_text,
    }
}

pub fn update_by_id(
    app: &tauri::AppHandle,
    id: &str,
    replaced_text: String,
    final_text: String,
) -> Result<(), String> {
    let mut entries = load(app);
    let entry = entries
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("history entry not found: {id}"))?;
    entry.replaced_text = replaced_text;
    entry.final_text = final_text;
    entry.cleanup_status = CleanupStatus::RecoveredManually;
    save(app, &entries)
}

/// Re-apply the retention cap to the on-disk history. Used when the user
/// changes the limit so the change takes effect immediately rather than waiting
/// for the next dictation. Reads the limit from config (already saved by the
/// caller).
pub fn enforce_limits(app: &tauri::AppHandle) -> Result<(), String> {
    retain_and_save(app, load(app))?;
    Ok(())
}

pub fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, has_audio: bool, favorite: bool) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            timestamp: 0,
            speak_duration_ms: 0,
            raw_text: String::new(),
            replaced_text: String::new(),
            final_text: String::new(),
            cleanup_status: CleanupStatus::Disabled,
            profile_snapshot: None,
            provider_model: None,
            app_name: None,
            bundle_id: None,
            context_channels: vec![],
            has_audio,
            favorite,
        }
    }

    fn ids(entries: &[HistoryEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.id.as_str()).collect()
    }

    #[test]
    fn cap_trims_oldest_non_favorites() {
        let entries = vec![
            entry("a", false, false),
            entry("b", false, false),
            entry("c", false, false),
        ];
        let (kept, evicted) = apply_retention(entries, Some(2));
        assert_eq!(ids(&kept), vec!["a", "b"]);
        assert!(evicted.is_empty());
    }

    #[test]
    fn favorites_are_kept_and_uncounted() {
        let entries = vec![
            entry("a", false, false),
            entry("fav", false, true),
            entry("b", false, false),
            entry("c", false, false),
        ];
        let (kept, _) = apply_retention(entries, Some(2));
        // The two newest non-favorites (a, b) plus the favorite survive; c drops.
        assert_eq!(ids(&kept), vec!["a", "fav", "b"]);
    }

    #[test]
    fn audio_and_text_records_share_one_cap() {
        let entries = vec![
            entry("r1", true, false),
            entry("t1", false, false),
            entry("r2", true, false),
        ];
        let (kept, evicted) = apply_retention(entries, Some(2));
        assert_eq!(ids(&kept), vec!["r1", "t1"]);
        assert_eq!(evicted, vec!["r2"]);
    }

    #[test]
    fn favorite_record_is_exempt_from_the_cap() {
        let entries = vec![
            entry("r1", true, false),
            entry("fav", true, true),
            entry("r2", true, false),
        ];
        let (kept, evicted) = apply_retention(entries, Some(1));
        assert_eq!(ids(&kept), vec!["r1", "fav"]);
        assert_eq!(evicted, vec!["r2"]);
    }

    #[test]
    fn evicting_a_record_marks_its_audio_for_deletion() {
        let entries = vec![entry("t", false, false), entry("r", true, false)];
        let (kept, evicted) = apply_retention(entries, Some(1));
        assert_eq!(ids(&kept), vec!["t"]);
        assert_eq!(evicted, vec!["r"]);
    }
}
