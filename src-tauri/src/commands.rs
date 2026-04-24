use crate::config::{self, DeepgramSettings, Replacement, Settings, Shortcut};
use crate::history::{self, HistoryEntry};
use crate::permissions;
use crate::recorder::Recorder;
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    config::load(&app)
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
pub fn list_input_devices() -> Vec<String> {
    Recorder::list_input_devices()
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
pub fn open_accessibility_settings() {
    permissions::open_accessibility_settings();
}
