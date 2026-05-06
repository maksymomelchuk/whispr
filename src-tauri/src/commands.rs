use crate::cleanup_stats::{self, CleanupStats, CLEANUP_STATS_UPDATED_EVENT};
use crate::config::{self, DeepgramSettings, Replacement, Shortcut};
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
    pub ai_cleanup_key_configured: bool,
    pub input_device: Option<String>,
    pub pause_media_on_record: bool,
    pub history_limit: Option<usize>,
    pub show_in_dock: bool,
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
        ai_cleanup_key_configured: s
            .ai_cleanup
            .anthropic_api_key
            .as_deref()
            .is_some_and(|k| !k.is_empty()),
        input_device: s.input_device,
        pause_media_on_record: s.pause_media_on_record,
        history_limit: s.history_limit,
        show_in_dock: s.show_in_dock,
    }
}

#[tauri::command]
pub fn set_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    config::save(&app, &settings)
}

#[tauri::command]
pub fn set_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    shortcut: Shortcut,
) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.shortcut = shortcut.clone();
    config::save(&app, &settings)?;
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
    let mut settings = config::load(&app);
    settings.replacements = replacements;
    config::save(&app, &settings)
}

#[tauri::command]
pub fn set_deepgram_settings(
    app: AppHandle,
    deepgram: DeepgramSettings,
) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.deepgram = deepgram;
    config::save(&app, &settings)
}

#[tauri::command]
pub fn set_ai_cleanup_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.ai_cleanup.enabled = enabled;
    config::save(&app, &settings)
}

#[tauri::command]
pub fn set_anthropic_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.ai_cleanup.anthropic_api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    config::save(&app, &settings)
}

#[tauri::command]
pub fn set_pause_media_on_record(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.pause_media_on_record = enabled;
    config::save(&app, &settings)?;
    *state.pause_media_on_record.lock().unwrap() = enabled;
    Ok(())
}

#[tauri::command]
pub fn set_show_in_dock(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = config::load(&app);
    settings.show_in_dock = enabled;
    config::save(&app, &settings)?;
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
    let mut settings = config::load(&app);
    settings.input_device = device.clone();
    config::save(&app, &settings)?;
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
    let mut settings = config::load(&app);
    settings.history_limit = limit;
    config::save(&app, &settings)?;
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
