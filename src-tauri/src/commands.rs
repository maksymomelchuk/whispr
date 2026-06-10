use crate::api_key_validation::{self, ApiKeyValidation};
use crate::cleanup_stats::{self, CleanupStats, CLEANUP_STATS_UPDATED_EVENT};
use crate::config::{
    self, CleanupAuthMode, CorrectionEntry, HotkeyAction, HotkeyBinding, LocalWhisperIdleTimeout,
    NamedCorrectionSet, NamedTermSet, Settings, SnippetEntry,
};
use crate::download::{
    self, LocalModelStatus, MODEL_DOWNLOAD_COMPLETE_EVENT, MODEL_DOWNLOAD_ERROR_EVENT,
};
use crate::history::{self, HistoryEntry, HISTORY_UPDATED_EVENT};
use crate::mode::{Mode, ModeId, SetId};
use crate::model_catalog;
use crate::permissions;
use crate::provider::{local_model_path, GroqModel, LocalWhisperModel};
use crate::recovery;
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
    pub openai_api_key_configured: bool,
    pub elevenlabs_api_key_configured: bool,
    pub hotkey_bindings: Vec<HotkeyBinding>,
    pub term_sets: Vec<NamedTermSet>,
    pub correction_sets: Vec<NamedCorrectionSet>,
    pub snippets: Vec<SnippetEntry>,
    pub modes: Vec<Mode>,
    pub ai_cleanup_auth_mode: CleanupAuthMode,
    pub ai_cleanup_key_configured: bool,
    pub ai_cleanup_oauth_token_configured: bool,
    pub configured_providers: Vec<String>,
    pub custom_provider_configured: bool,
    pub custom_provider_base_url: Option<String>,
    pub custom_provider_model: String,
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
        let anthropic_key_configured = s
            .ai_cleanup
            .provider_keys
            .get("anthropic")
            .is_some_and(|k| !k.is_empty());
        let configured_providers: Vec<String> = s
            .ai_cleanup
            .provider_keys
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, _)| k.clone())
            .collect();
        let custom_provider_configured = s
            .ai_cleanup
            .custom_provider
            .as_ref()
            .is_some_and(|cp| !cp.base_url.is_empty());
        let custom_provider_base_url = s.ai_cleanup.custom_provider.as_ref().and_then(|cp| {
            if cp.base_url.is_empty() {
                None
            } else {
                Some(cp.base_url.clone())
            }
        });
        let custom_provider_model = s
            .ai_cleanup
            .custom_provider
            .as_ref()
            .map(|cp| cp.model.clone())
            .unwrap_or_default();
        SettingsView {
            deepgram_api_key_configured,
            groq_api_key_configured: s.groq_api_key.as_deref().is_some_and(|k| !k.is_empty()),
            assemblyai_api_key_configured: s
                .assemblyai_api_key
                .as_deref()
                .is_some_and(|k| !k.is_empty()),
            openai_api_key_configured: s.openai_api_key.as_deref().is_some_and(|k| !k.is_empty()),
            elevenlabs_api_key_configured: s
                .elevenlabs_api_key
                .as_deref()
                .is_some_and(|k| !k.is_empty()),
            hotkey_bindings: s.hotkey_bindings,
            term_sets: s.term_sets,
            correction_sets: s.correction_sets,
            snippets: s.snippets,
            modes: s.modes,
            ai_cleanup_auth_mode: s.ai_cleanup.auth_mode,
            ai_cleanup_key_configured: anthropic_key_configured,
            ai_cleanup_oauth_token_configured: s
                .ai_cleanup
                .anthropic_oauth_token
                .as_deref()
                .is_some_and(|t| !t.is_empty()),
            configured_providers,
            custom_provider_configured,
            custom_provider_base_url,
            custom_provider_model,
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
pub fn set_openai_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.openai_api_key = config::non_empty(api_key))
}

#[tauri::command]
pub fn set_elevenlabs_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| s.elevenlabs_api_key = config::non_empty(api_key))
}

#[tauri::command]
pub async fn validate_assemblyai_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_assemblyai(&api_key).await
}

#[tauri::command]
pub async fn validate_openai_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_openai(&api_key).await
}

#[tauri::command]
pub async fn validate_elevenlabs_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_elevenlabs(&api_key).await
}

#[tauri::command]
pub async fn validate_deepgram_api_key(api_key: String) -> ApiKeyValidation {
    api_key_validation::validate_deepgram(&api_key).await
}

#[tauri::command]
pub async fn validate_groq_api_key(app: AppHandle, api_key: String) -> ApiKeyValidation {
    let settings = config::load(&app);
    let language = settings
        .modes
        .first()
        .and_then(|m| m.language.as_code())
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

#[tauri::command]
pub fn set_shortcut_capture_paused(state: State<'_, AppState>, paused: bool) {
    *state.shortcut_capture_paused.lock().unwrap() = paused;
}

#[tauri::command]
pub fn create_term_set(app: AppHandle, name: String) -> Result<SettingsView, String> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let set = NamedTermSet {
        id: format!("term-set-{ms}"),
        name: name.trim().to_string(),
        entries: vec![],
    };
    config::update(&app, |s| s.term_sets.push(set))?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn rename_term_set(app: AppHandle, id: SetId, name: String) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        if let Some(ts) = s.term_sets.iter_mut().find(|ts| ts.id == id) {
            ts.name = name.trim().to_string();
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn update_term_set_entries(
    app: AppHandle,
    id: SetId,
    entries: Vec<String>,
) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        if let Some(ts) = s.term_sets.iter_mut().find(|ts| ts.id == id) {
            ts.entries = entries;
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn delete_term_set(app: AppHandle, id: SetId) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        s.term_sets.retain(|ts| ts.id != id);
        for mode in s.modes.iter_mut() {
            mode.term_set_ids.retain(|tsid| *tsid != id);
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn create_correction_set(app: AppHandle, name: String) -> Result<SettingsView, String> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let set = NamedCorrectionSet {
        id: format!("correction-set-{ms}"),
        name: name.trim().to_string(),
        entries: vec![],
    };
    config::update(&app, |s| s.correction_sets.push(set))?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn rename_correction_set(
    app: AppHandle,
    id: SetId,
    name: String,
) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        if let Some(cs) = s.correction_sets.iter_mut().find(|cs| cs.id == id) {
            cs.name = name.trim().to_string();
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn update_correction_set_entries(
    app: AppHandle,
    id: SetId,
    entries: Vec<CorrectionEntry>,
) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        if let Some(cs) = s.correction_sets.iter_mut().find(|cs| cs.id == id) {
            cs.entries = entries;
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn delete_correction_set(app: AppHandle, id: SetId) -> Result<SettingsView, String> {
    config::update(&app, |s| {
        s.correction_sets.retain(|cs| cs.id != id);
        for mode in s.modes.iter_mut() {
            mode.correction_set_ids.retain(|csid| *csid != id);
        }
    })?;
    Ok(config::load(&app).into())
}

#[tauri::command]
pub fn set_snippets(app: AppHandle, snippets: Vec<SnippetEntry>) -> Result<(), String> {
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
pub fn delete_mode(app: AppHandle, state: State<'_, AppState>, id: ModeId) -> Result<(), String> {
    config::update(&app, |s| {
        s.modes.retain(|m| m.id != id);
        s.hotkey_bindings
            .retain(|b| !is_ptt_for_mode(&b.action, &id));
    })?;
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
pub fn reorder_modes(app: AppHandle, ids: Vec<ModeId>) -> Result<(), String> {
    config::update(&app, |s| {
        let rank = |id: &str| ids.iter().position(|x| x == id).unwrap_or(usize::MAX);
        s.modes.sort_by_key(|m| rank(&m.id));
    })
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
pub fn set_anthropic_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    config::update(&app, |s| match config::non_empty(api_key) {
        Some(k) => {
            s.ai_cleanup
                .provider_keys
                .insert("anthropic".to_string(), k);
        }
        None => {
            s.ai_cleanup.provider_keys.remove("anthropic");
        }
    })
}

#[tauri::command]
pub fn set_anthropic_oauth_token(app: AppHandle, token: String) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.anthropic_oauth_token = config::non_empty(token)
    })
}

#[tauri::command]
pub fn set_provider_key(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    config::update(&app, |s| match config::non_empty(api_key) {
        Some(k) => {
            s.ai_cleanup.provider_keys.insert(provider_id, k);
        }
        None => {
            s.ai_cleanup.provider_keys.remove(&provider_id);
        }
    })
}

#[tauri::command]
pub fn clear_provider_key(app: AppHandle, provider_id: String) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.provider_keys.remove(&provider_id);
    })
}

#[tauri::command]
pub fn set_custom_provider(
    app: AppHandle,
    base_url: String,
    model: String,
    api_key: String,
) -> Result<(), String> {
    let trimmed_url = base_url.trim().trim_end_matches('/').to_string();
    if !trimmed_url.is_empty() {
        url::Url::parse(&trimmed_url).map_err(|_| format!("Invalid base URL: {trimmed_url}"))?;
    }
    config::update(&app, |s| {
        s.ai_cleanup.custom_provider = if trimmed_url.is_empty() {
            None
        } else {
            Some(config::CustomProvider {
                base_url: trimmed_url,
                model: model.trim().to_string(),
                api_key: config::non_empty(api_key.trim().to_string()),
            })
        };
    })
}

#[tauri::command]
pub fn clear_custom_provider(app: AppHandle) -> Result<(), String> {
    config::update(&app, |s| {
        s.ai_cleanup.custom_provider = None;
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
        manager
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {e}"))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {e}"))?;
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
    crate::recorder::Recorder::list_input_devices()
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
pub fn update_history_entry(
    app: AppHandle,
    id: String,
    replaced_text: String,
    final_text: String,
) -> Result<(), String> {
    history::update_by_id(&app, &id, replaced_text, final_text)?;
    let _ = app.emit(HISTORY_UPDATED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub async fn recover_cleanup(app: AppHandle, id: String) -> Result<String, String> {
    let entries = history::load(&app);
    let entry = entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("history entry not found: {id}"))?
        .clone();

    if !recovery::is_recoverable(&entry) {
        return Err("entry is not recoverable".to_string());
    }

    let settings = config::load(&app);
    let outcome = recovery::recover_entry(&entry, &settings)
        .await
        .map_err(|e| e.to_string())?;

    history::update_by_id(
        &app,
        &id,
        outcome.history_entry.replaced_text,
        outcome.history_entry.final_text.clone(),
    )?;

    let _ = app.emit(HISTORY_UPDATED_EVENT, ());

    Ok(outcome.history_entry.final_text)
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
    #[cfg(target_os = "macos")]
    return tauri::async_runtime::spawn_blocking(move || {
        crate::target_app::resolve_icon(&bundle_id)
    })
    .await
    .ok()
    .flatten();
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle_id;
        None
    }
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
    if !permissions::check_accessibility_permission() {
        return;
    }
    // CAS so concurrent polls can't double-spawn the tap thread.
    if state
        .ptt_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let recorder_opt = state.recorder.lock().unwrap().clone();
    match recorder_opt {
        Some(rec) => crate::ptt::start(app, (*state).clone(), rec),
        None => state.ptt_running.store(false, Ordering::Release),
    }
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
    let models_dir = data_dir.join("models");
    [
        LocalWhisperModel::LargeV3,
        LocalWhisperModel::LargeV3Turbo,
        LocalWhisperModel::Parakeet,
    ]
    .iter()
    .map(|&model| {
        let catalog = model_catalog::catalog_for(model);
        LocalModelStatus {
            model,
            downloaded: model_catalog::all_files_present(&catalog, &models_dir),
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
    let models_dir = data_dir.join("models");
    let spec = model_catalog::catalog_for(model);
    for path in model_catalog::files_to_delete(&spec, &models_dir) {
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
pub fn set_local_whisper_idle_timeout(
    app: AppHandle,
    timeout: LocalWhisperIdleTimeout,
) -> Result<(), String> {
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
        assert!(view.modes.is_empty());
        assert!(view.hotkey_bindings.is_empty());
        assert!(view.term_sets.is_empty());
    }

    #[test]
    fn settings_view_exposes_local_whisper_idle_timeout_with_fifteen_minute_default() {
        let view: SettingsView = Settings::default().into();
        assert_eq!(
            view.local_whisper_idle_timeout,
            LocalWhisperIdleTimeout::FifteenMinutes
        );
    }

    #[test]
    fn settings_view_exposes_modes() {
        let settings = Settings {
            modes: vec![Mode::seed_default_en(false), Mode::seed_cleaned_en()],
            ..Settings::default()
        };
        let view: SettingsView = settings.into();
        assert_eq!(view.modes.len(), 2);
        let default = view
            .modes
            .iter()
            .find(|m| m.id == crate::mode::SEED_MODE_DEFAULT_EN)
            .unwrap();
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
        let mut settings = Settings {
            modes: vec![Mode::seed_default_en(false), Mode::seed_cleaned_en()],
            ..Settings::default()
        };
        settings.modes[0].provider_model = ProviderModel::Groq {
            model: GroqModel::WhisperLargeV3,
        };
        let view: SettingsView = settings.into();
        assert_eq!(
            view.modes[0].provider_model,
            ProviderModel::Groq {
                model: GroqModel::WhisperLargeV3
            }
        );
        // Remaining modes unchanged.
        assert_eq!(view.modes[1].provider_model, ProviderModel::Deepgram);
    }
}
