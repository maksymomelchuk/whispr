use crate::api_key_validation::{self, ApiKeyValidation};
use crate::cleanup_stats::{self, CleanupStats, CLEANUP_STATS_UPDATED_EVENT};
use crate::config::{
    self, CleanupAuthMode, HotkeyAction, HotkeyBinding, LocalWhisperIdleTimeout, NamedCorrectionSet, NamedTermSet, Settings, SnippetEntry,
};
use crate::download::{self, LocalModelStatus, MODEL_DOWNLOAD_COMPLETE_EVENT, MODEL_DOWNLOAD_ERROR_EVENT};
use crate::history::{self, HistoryEntry, HISTORY_UPDATED_EVENT};
use crate::mode::{Mode, ModeId, SetId};
use crate::permissions;
use crate::provider::{local_model_path, GroqModel, LocalWhisperModel};
use crate::state::AppState;
use crate::stats::{self, StatsRow, STATS_UPDATED_EVENT};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Public projection of Settings for the webview. Omits API keys so a
/// webview XSS cannot read them back over IPC. Keys are write-only from the
/// frontend's perspective.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    pub deepgram_api_key_configured: bool,
    pub groq_api_key_configured: bool,
    pub assemblyai_api_key_configured: bool,
    pub hotkey_bindings: Vec<HotkeyBinding>,
    pub term_sets: Vec<NamedTermSet>,
    pub correction_sets: Vec<NamedCorrectionSet>,
    pub snippets: Vec<SnippetEntry>,
    pub modes: Vec<Mode>,
    pub default_mode_id: ModeId,
    pub ai_cleanup_auth_mode: CleanupAuthMode,
    pub ai_cleanup_key_configured: bool,
    pub ai_cleanup_oauth_token_configured: bool,
    pub ai_cleanup_min_words: usize,
    pub ai_cleanup_min_duration_ms: u64,
    pub input_device: Option<String>,
    pub pause_media_on_record: bool,
    pub history_limit: Option<usize>,
    pub show_in_dock: bool,
    pub start_at_login: bool,
    pub show_live_preview: bool,
    pub local_whisper_idle_timeout: LocalWhisperIdleTimeout,
}

impl From<Settings> for SettingsView {
    fn from(s: Settings) -> Self {
        let deepgram_api_key_configured = s
            .deepgram_api_key
            .as_deref()
            .or(s.api_key.as_deref())
            .is_some_and(|k| !k.is_empty());
        SettingsView {
            deepgram_api_key_configured,
            groq_api_key_configured: s.groq_api_key.as_deref().is_some_and(|k| !k.is_empty()),
            assemblyai_api_key_configured: s
                .assemblyai_api_key
                .as_deref()
                .is_some_and(|k| !k.is_empty()),
            hotkey_bindings: s.hotkey_bindings,
            term_sets: s.term_sets,
            correction_sets: s.correction_sets,
            snippets: s.snippets,
            modes: s.modes,
            default_mode_id: s.default_mode_id,
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
            start_at_login: s.start_at_login,
            show_live_preview: s.show_live_preview,
            local_whisper_idle_timeout: s.local_whisper.idle_timeout,
        }
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    config::load(&app).into()
}

#[tauri::command]
pub fn set_deepgram_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.deepgram_api_key = config::non_empty(api_key))
}

#[tauri::command]
pub fn set_groq_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.groq_api_key = config::non_empty(api_key))
}

#[tauri::command]
pub fn set_assemblyai_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.assemblyai_api_key = config::non_empty(api_key))
}

#[tauri::command]
pub async fn validate_assemblyai_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_assemblyai(&api_key).await
}

#[tauri::command]
pub async fn validate_deepgram_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_deepgram(&api_key).await
}

#[tauri::command]
pub async fn validate_groq_api_key(app: AppHandle, api_key: String) -> ApiKeyValidation {
    let settings = config::load(&app);
    let language = config::get_default_mode(&settings)
        .language
        .as_code()
        .unwrap_or("en")
        .to_string();
    api_key_validation::validate_groq(&api_key, GroqModel::default(), &language).await
}

#[tauri::command]
pub fn set_hotkey_bindings(
    app: AppHandle,
    state: State<'_, AppState>,
    bindings: Vec<HotkeyBinding>,
) -> Result<(), String> {
    config::check_hotkey_conflicts(&bindings)?;
    config::check_action_constraints(&bindings)?;
    config::update(&app, |s| s.hotkey_bindings = bindings.clone())?;
    // Live-update the PTT listener so the change takes effect immediately.
    *state.hotkey_bindings.lock().unwrap() = bindings;
    Ok(())
}

/// Opens System Settings to the Language & Region pane — the "Translation
/// Languages" section lives inside it on macOS 15/26. Surfaced to the UI as
/// the click target for the "Open Settings" action on missing-pack errors.
#[tauri::command]
pub fn open_translation_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.Localization-Settings.extension")
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Opening translation settings is only supported on macOS".to_string())
    }
}

#[tauri::command]
pub fn set_shortcut_capture_paused(state: State<'_, AppState>, paused: bool) {
    *state.shortcut_capture_paused.lock().unwrap() = paused;
}

#[tauri::command]
pub fn create_term_set(app: AppHandle, name: String) -> Result<NamedTermSet, String> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let set = NamedTermSet {
        id: format!("term-set-{ms}"),
        name: name.trim().to_string(),
        entries: vec![],
    };
    let result = set.clone();
    config::update(&app, |s| s.term_sets.push(set))?;
    Ok(result)
}

#[tauri::command]
pub fn rename_term_set(app: AppHandle, id: SetId, name: String) -> Result<(), String> {
    config::update(&app, |s| {
        if let Some(ts) = s.term_sets.iter_mut().find(|ts| ts.id == id) {
            ts.name = name.trim().to_string();
        }
    })
}

#[tauri::command]
pub fn update_term_set_entries(
    app: AppHandle,
    id: SetId,
    entries: Vec<String>,
) -> Result<(), String> {
    config::update(&app, |s| {
        if let Some(ts) = s.term_sets.iter_mut().find(|ts| ts.id == id) {
            ts.entries = entries;
        }
    })
}

#[tauri::command]
pub fn delete_term_set(app: AppHandle, id: SetId) -> Result<(), String> {
    config::update(&app, |s| {
        s.term_sets.retain(|ts| ts.id != id);
        for mode in s.modes.iter_mut() {
            mode.term_set_ids.retain(|tsid| *tsid != id);
        }
    })
}

#[tauri::command]
pub fn add_correction_set(app: AppHandle, set: NamedCorrectionSet) -> Result<(), String> {
    config::update(&app, |s| s.correction_sets.push(set))
}

#[tauri::command]
pub fn update_correction_set(app: AppHandle, set: NamedCorrectionSet) -> Result<(), String> {
    config::update(&app, |s| {
        if let Some(cs) = s.correction_sets.iter_mut().find(|cs| cs.id == set.id) {
            *cs = set;
        }
    })
}

#[tauri::command]
pub fn delete_correction_set(app: AppHandle, set_id: String) -> Result<(), String> {
    config::update(&app, |s| {
        s.correction_sets.retain(|cs| cs.id != set_id);
        for mode in s.modes.iter_mut() {
            mode.correction_set_ids.retain(|id| id != &set_id);
        }
    })
}

#[tauri::command]
pub fn set_snippets(
    app: AppHandle,
    snippets: Vec<SnippetEntry>,
) -> Result<(), String> {
    config::update(&app, |s| s.snippets = snippets)
}

#[tauri::command]
pub fn add_mode(app: AppHandle, mode: Mode) -> Result<(), String> {
    config::update(&app, |s| s.modes.push(mode))
}

#[tauri::command]
pub fn update_mode(app: AppHandle, mode: Mode) -> Result<(), String> {
    config::update(&app, |s| {
        let id = mode.id.clone();
        if let Some(m) = s.modes.iter_mut().find(|m| m.id == id) {
            *m = mode;
        }
    })
}

#[tauri::command]
pub fn delete_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    id: ModeId,
) -> Result<(), String> {
    config::update_fallible(&app, |s| {
        config::check_delete_mode(s, &id)?;
        s.modes.retain(|m| m.id != id);
        s.hotkey_bindings.retain(|b| !is_ptt_for_mode(&b.action, &id));
        Ok(())
    })?;
    // Live-update the PTT listener.
    state
        .hotkey_bindings
        .lock()
        .unwrap()
        .retain(|b| !is_ptt_for_mode(&b.action, &id));
    Ok(())
}

fn is_ptt_for_mode(action: &HotkeyAction, id: &str) -> bool {
    matches!(action, HotkeyAction::Ptt { mode_id } if mode_id == id)
}

#[tauri::command]
pub fn duplicate_mode(app: AppHandle, id: ModeId) -> Result<(), String> {
    config::update(&app, |s| {
        if let Some(source) = s.modes.iter().find(|m| m.id == id).cloned() {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            s.modes.push(Mode {
                id: format!("mode-{ms}"),
                name: format!("{} (copy)", source.name),
                ..source
            });
        }
    })
}

#[tauri::command]
pub fn set_default_mode(app: AppHandle, id: ModeId) -> Result<(), String> {
    config::update_fallible(&app, |s| {
        if !s.modes.iter().any(|m| m.id == id) {
            return Err(format!("Mode '{id}' not found."));
        }
        s.default_mode_id = id;
        Ok(())
    })
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
pub fn set_start_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| format!("Failed to enable autostart: {e}"))?;
    } else {
        manager.disable().map_err(|e| format!("Failed to disable autostart: {e}"))?;
    }
    config::update(&app, |s| s.start_at_login = enabled)
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
pub async fn get_app_icon(bundle_id: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || crate::target_app::resolve_icon(&bundle_id))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub fn get_cleanup_stats(app: AppHandle) -> CleanupStats {
    cleanup_stats::load(&app)
}

#[tauri::command]
pub fn open_accessibility_settings() {
    permissions::open_accessibility_settings();
}

#[derive(Debug, Clone, Serialize)]
pub struct PermissionsStatus {
    pub microphone: bool,
    pub accessibility: bool,
}

#[tauri::command]
pub fn check_permissions() -> PermissionsStatus {
    PermissionsStatus {
        microphone: permissions::check_microphone_permission(),
        accessibility: permissions::check_accessibility_permission(),
    }
}

#[tauri::command]
pub fn open_microphone_settings() {
    permissions::open_microphone_settings();
}

#[tauri::command]
pub fn ensure_ptt_started(app: AppHandle, state: State<'_, AppState>) {
    use std::sync::atomic::Ordering;
    if state.ptt_running.load(Ordering::Acquire) {
        return;
    }
    if !permissions::check_accessibility_permission() {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let recorder_opt = state.recorder.lock().unwrap().clone();
        if let Some(rec) = recorder_opt {
            crate::ptt::start(app, (*state).clone(), rec);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[tauri::command]
pub fn get_local_model_statuses(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Vec<LocalModelStatus> {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let flags = state.download_cancel_flags.lock().unwrap();
    [LocalWhisperModel::LargeV3, LocalWhisperModel::LargeV3Turbo]
        .iter()
        .map(|&model| {
            let path = local_model_path(&data_dir, model);
            LocalModelStatus {
                model,
                downloaded: path.exists(),
                downloading: flags.contains_key(&model),
                size_bytes: download::model_size_bytes(model),
            }
        })
        .collect()
}

#[tauri::command]
pub async fn start_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model: LocalWhisperModel,
) -> Result<(), String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut flags = state.download_cancel_flags.lock().unwrap();
        if flags.contains_key(&model) {
            return Err("Download already in progress".to_string());
        }
        flags.insert(model, cancel_flag.clone());
    }

    let cancel_flags = Arc::clone(&state.download_cancel_flags);
    let app_clone = app.clone();
    tokio::spawn(async move {
        let result = download::download_model(app_clone.clone(), model, cancel_flag).await;
        cancel_flags.lock().unwrap().remove(&model);
        match result {
            Ok(()) => {
                let _ = app_clone.emit(MODEL_DOWNLOAD_COMPLETE_EVENT, model);
            }
            Err(ref msg) if msg == "Cancelled" => {}
            Err(e) => {
                let _ = app_clone.emit(
                    MODEL_DOWNLOAD_ERROR_EVENT,
                    download::ModelDownloadError { model, message: e },
                );
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_model_download(
    state: State<'_, AppState>,
    model: LocalWhisperModel,
) -> Result<(), String> {
    let flags = state.download_cancel_flags.lock().unwrap();
    match flags.get(&model) {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            Ok(())
        }
        None => Err("No active download for this model".to_string()),
    }
}

#[tauri::command]
pub fn delete_local_model(app: AppHandle, model: LocalWhisperModel) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = local_model_path(&data_dir, model);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_local_model_path(app: AppHandle, model: LocalWhisperModel) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = local_model_path(&data_dir, model);
    if !path.exists() {
        return Err("Model file not found".to_string());
    }
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Model path is not valid UTF-8".to_string())
}

#[tauri::command]
pub fn set_local_whisper_idle_timeout(app: AppHandle, timeout: LocalWhisperIdleTimeout) -> Result<(), String> {
    config::update(&app, |s| {
        s.local_whisper.idle_timeout = timeout;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ModeLanguage;
    use crate::provider::ProviderModel;

    #[test]
    fn settings_view_defaults_match_fresh_install() {
        let view: SettingsView = Settings::default().into();
        assert!(!view.deepgram_api_key_configured);
        assert!(!view.groq_api_key_configured);
        assert_eq!(view.hotkey_bindings.len(), 1);
        assert_eq!(view.hotkey_bindings[0].shortcut.key, "AltRight");
        assert!(view.term_sets.is_empty());
    }

    #[test]
    fn settings_view_exposes_local_whisper_idle_timeout_with_fifteen_minute_default() {
        let view: SettingsView = Settings::default().into();
        assert_eq!(view.local_whisper_idle_timeout, LocalWhisperIdleTimeout::FifteenMinutes);
    }

    #[test]
    fn settings_view_exposes_modes_and_default_mode_id() {
        let view: SettingsView = Settings::default().into();
        assert_eq!(view.modes.len(), 4);
        assert_eq!(view.default_mode_id, crate::mode::SEED_MODE_DEFAULT_EN);
        let default = view.modes.iter().find(|m| m.id == crate::mode::SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.language, ModeLanguage::exact("en"));
    }

    #[test]
    fn settings_view_modes_default_to_deepgram_provider_model() {
        let view: SettingsView = Settings::default().into();
        for mode in &view.modes {
            assert_eq!(
                mode.provider_model,
                ProviderModel::Deepgram,
                "mode {} should default to Deepgram provider_model",
                mode.id
            );
        }
    }

    #[test]
    fn settings_view_exposes_independent_per_provider_configured_flags() {
        let view: SettingsView = Settings {
            deepgram_api_key: Some("dg-key".to_string()),
            groq_api_key: Some("gsk-key".to_string()),
            ..Settings::default()
        }
        .into();
        assert!(view.deepgram_api_key_configured);
        assert!(view.groq_api_key_configured);

        let view: SettingsView = Settings {
            deepgram_api_key: Some("dg-key".to_string()),
            ..Settings::default()
        }
        .into();
        assert!(view.deepgram_api_key_configured);
        assert!(!view.groq_api_key_configured);

        let view: SettingsView = Settings {
            groq_api_key: Some("gsk-key".to_string()),
            ..Settings::default()
        }
        .into();
        assert!(!view.deepgram_api_key_configured);
        assert!(view.groq_api_key_configured);
    }

    #[test]
    fn settings_view_treats_empty_keys_as_not_configured() {
        let view: SettingsView = Settings {
            deepgram_api_key: Some(String::new()),
            groq_api_key: Some(String::new()),
            ..Settings::default()
        }
        .into();
        assert!(!view.deepgram_api_key_configured);
        assert!(!view.groq_api_key_configured);
    }

    #[test]
    fn settings_view_falls_back_to_legacy_api_key_for_deepgram_configured() {
        let view: SettingsView = Settings {
            api_key: Some("legacy".to_string()),
            ..Settings::default()
        }
        .into();
        assert!(view.deepgram_api_key_configured);
    }

    #[test]
    fn settings_view_carries_per_mode_provider_model() {
        let mut settings = Settings::default();
        settings.modes[0].provider_model = ProviderModel::Groq {
            model: GroqModel::WhisperLargeV3,
        };
        let view: SettingsView = settings.into();
        assert_eq!(
            view.modes[0].provider_model,
            ProviderModel::Groq { model: GroqModel::WhisperLargeV3 }
        );
        // Remaining modes unchanged.
        assert_eq!(view.modes[1].provider_model, ProviderModel::Deepgram);
    }
}
