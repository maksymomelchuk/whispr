use crate::cleanup_stats::{self, CleanupStats, CLEANUP_STATS_UPDATED_EVENT};
use crate::config::{self, CleanupAuthMode, DeepgramSettings, Replacement, Shortcut};
use crate::history::{self, HistoryEntry, HISTORY_UPDATED_EVENT};
use crate::permissions;
use crate::state::AppState;
use crate::stats::{self, StatsRow, STATS_UPDATED_EVENT};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// Public projection of Settings for the webview. Omits both API keys so a
/// webview XSS (e.g., via a future supply-chain compromise) cannot read them
/// back over IPC. Keys are write-only from the frontend's perspective.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    pub api_key_configured: bool,
    pub shortcut: Shortcut,
    pub replacements: Vec<Replacement>,
    pub deepgram: DeepgramSettings,
    pub ai_cleanup_enabled: bool,
    pub ai_cleanup_auth_mode: CleanupAuthMode,
    pub ai_cleanup_key_configured: bool,
    pub ai_cleanup_oauth_token_configured: bool,
    pub ai_cleanup_min_words: usize,
    pub ai_cleanup_min_duration_ms: u64,
    pub input_device: Option<String>,
    pub pause_media_on_record: bool,
    pub history_limit: Option<usize>,
    pub show_in_dock: bool,
    pub show_live_preview: bool,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    let s = config::load(&app);
    SettingsView {
        api_key_configured: s.api_key.as_deref().is_some_and(|k| !k.is_empty()),
        shortcut: s.shortcut,
        replacements: s.replacements,
        deepgram: s.deepgram,
        ai_cleanup_enabled: s.ai_cleanup.enabled,
        ai_cleanup_auth_mode: s.ai_cleanup.auth_mode,
        ai_cleanup_key_configured: s
            .ai_cleanup
            .anthropic_api_key
            .as_deref()
            .is_some_and(|k| !k.is_empty()),
        ai_cleanup_oauth_token_configured: s
            .ai_cleanup
            .anthropic_oauth_token
            .as_deref()
            .is_some_and(|t| !t.is_empty()),
        ai_cleanup_min_words: s.ai_cleanup.min_words,
        ai_cleanup_min_duration_ms: s.ai_cleanup.min_duration_ms,
        input_device: s.input_device,
        pause_media_on_record: s.pause_media_on_record,
        history_limit: s.history_limit,
        show_in_dock: s.show_in_dock,
        show_live_preview: s.show_live_preview,
    }
}

#[tauri::command]
pub fn set_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.api_key = config::non_empty(api_key))
}

#[tauri::command]
pub fn set_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    shortcut: Shortcut,
) -> Result<(), String> {
    config::update(&app, |s| s.shortcut = shortcut.clone())?;
    // Live-update the listener's view of the shortcut so the change takes
    // effect immediately — no app restart needed.
    *state.shortcut.lock().unwrap() = shortcut;
    Ok(())
}

#[tauri::command]
pub fn set_replacements(
    app: AppHandle,
    replacements: Vec<Replacement>,
) -> Result<(), String> {
    config::update(&app, |s| s.replacements = replacements)
}

#[tauri::command]
pub fn set_deepgram_settings(
    app: AppHandle,
    deepgram: DeepgramSettings,
) -> Result<(), String> {
    config::update(&app, |s| s.deepgram = deepgram)
}

#[tauri::command]
pub fn set_ai_cleanup_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    config::update(&app, |s| s.ai_cleanup.enabled = enabled)
}

#[tauri::command]
pub fn set_anthropic_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.anthropic_api_key = config::non_empty(api_key)
    })
}

#[tauri::command]
pub fn set_anthropic_oauth_token(app: AppHandle, token: String) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.anthropic_oauth_token = config::non_empty(token)
    })
}

#[tauri::command]
pub fn set_cleanup_auth_mode(app: AppHandle, mode: CleanupAuthMode) -> Result<(), String> {
    config::update(&app, |s| s.ai_cleanup.auth_mode = mode)
}

#[tauri::command]
pub fn set_cleanup_thresholds(
    app: AppHandle,
    min_words: usize,
    min_duration_ms: u64,
) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.min_words = min_words;
        s.ai_cleanup.min_duration_ms = min_duration_ms;
    })
}

#[tauri::command]
pub fn set_pause_media_on_record(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    config::update(&app, |s| s.pause_media_on_record = enabled)?;
    *state.pause_media_on_record.lock().unwrap() = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_show_live_preview(app: AppHandle, enabled: bool) -> Result<(), String> {
    config::update(&app, |s| s.show_live_preview = enabled)
}

#[tauri::command]
pub fn set_show_in_dock(app: AppHandle, enabled: bool) -> Result<(), String> {
    config::update(&app, |s| s.show_in_dock = enabled)?;
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        let policy = if enabled {
            ActivationPolicy::Regular
        } else {
            ActivationPolicy::Accessory
        };
        app.set_activation_policy(policy)
            .map_err(|e| format!("Failed to update activation policy: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_input_devices() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        crate::recorder::Recorder::list_input_devices()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub fn set_input_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device: Option<String>,
) -> Result<(), String> {
    config::update(&app, |s| s.input_device = device.clone())?;
    *state.input_device.lock().unwrap() = device;
    Ok(())
}

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<HistoryEntry> {
    history::load(&app)
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    history::clear(&app)
}

#[tauri::command]
pub fn set_history_limit(app: AppHandle, limit: Option<usize>) -> Result<(), String> {
    config::update(&app, |s| s.history_limit = limit)?;
    history::enforce_limit(&app, limit)?;
    let _ = app.emit(HISTORY_UPDATED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub fn get_stats(app: AppHandle) -> Vec<StatsRow> {
    stats::load(&app)
}

#[tauri::command]
pub fn clear_stats(app: AppHandle) -> Result<(), String> {
    stats::clear(&app)?;
    cleanup_stats::clear(&app)?;
    let _ = app.emit(STATS_UPDATED_EVENT, ());
    let _ = app.emit(CLEANUP_STATS_UPDATED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub fn get_cleanup_stats(app: AppHandle) -> CleanupStats {
    cleanup_stats::load(&app)
}

#[tauri::command]
pub fn open_accessibility_settings() {
    permissions::open_accessibility_settings();
}
