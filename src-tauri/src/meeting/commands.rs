use super::capture;
use super::processing::{self, ProcessingConfig};
use super::store::{Meeting, MeetingMeta, MeetingStatus, MeetingStore};
use super::summary::MeetingSummarySettings;
use crate::config;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State};

pub const MEETINGS_UPDATED_EVENT: &str = "meetings-updated";
const MEETINGS_DIR: &str = "meetings";

pub fn meeting_store(app: &AppHandle) -> Result<MeetingStore, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    Ok(MeetingStore::new(data_dir.join(MEETINGS_DIR)))
}

fn notify_updated(app: &AppHandle) {
    let _ = app.emit(MEETINGS_UPDATED_EVENT, ());
}

#[tauri::command]
pub async fn start_meeting(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<MeetingMeta, String> {
    if state.active_meeting.lock().unwrap().is_some() {
        return Err("A meeting is already recording".to_string());
    }
    let settings = config::load(&app);
    ProcessingConfig::from_settings(&settings)?;
    let store = meeting_store(&app)?;
    let mut meta = MeetingMeta {
        id: format!("meeting-{}", crate::history::now_unix_seconds()),
        started_at: crate::history::now_unix_seconds(),
        duration_ms: 0,
        status: MeetingStatus::Recording,
        summary_model: None,
        system_audio_error: None,
    };
    store.create(&meta)?;
    let started = capture::start(&store, &meta.id, settings.input_device).await;
    let started = match started {
        Ok(started) => started,
        Err(error) => {
            store.delete(&meta.id)?;
            return Err(error);
        }
    };
    meta.system_audio_error = started.system_audio_error;
    store.save_meta(&meta)?;
    *state.active_meeting.lock().unwrap() = Some(started.meeting);
    notify_updated(&app);
    Ok(meta)
}

#[tauri::command]
pub async fn stop_meeting(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let active = state
        .active_meeting
        .lock()
        .unwrap()
        .take()
        .ok_or("No meeting is recording")?;
    let id = active.id.clone();
    let duration = tauri::async_runtime::spawn_blocking(move || active.stop())
        .await
        .map_err(|e| format!("stopping the recording failed: {e}"))??;
    let store = meeting_store(&app)?;
    let mut meta = store.load_meta(&id)?;
    meta.duration_ms = duration.as_millis() as u64;
    store.save_meta(&meta)?;
    spawn_processing(app, store, id);
    Ok(())
}

#[tauri::command]
pub fn retry_meeting(app: AppHandle, id: String) -> Result<(), String> {
    let store = meeting_store(&app)?;
    store.load_meta(&id)?;
    spawn_processing(app, store, id);
    Ok(())
}

/// Reports a missing key as a Failed status rather than an error, because
/// processing runs detached from the command that started it.
fn spawn_processing(app: AppHandle, store: MeetingStore, id: String) {
    let config = match ProcessingConfig::from_settings(&config::load(&app)) {
        Ok(config) => config,
        Err(error) => {
            if let Err(save_error) = store.update_status(&id, MeetingStatus::Failed(error)) {
                eprintln!("[meeting] recording failure for {id} failed: {save_error}");
            }
            notify_updated(&app);
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        let notifier = app.clone();
        processing::run(store, id, config, move || notify_updated(&notifier)).await;
    });
}

#[tauri::command]
pub fn list_meetings(app: AppHandle) -> Result<Vec<MeetingMeta>, String> {
    Ok(meeting_store(&app)?.list())
}

#[tauri::command]
pub fn get_meeting(app: AppHandle, id: String) -> Result<Meeting, String> {
    meeting_store(&app)?.load(&id)
}

#[tauri::command]
pub fn delete_meeting(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let recording_id = state
        .active_meeting
        .lock()
        .unwrap()
        .as_ref()
        .map(|m| m.id.clone());
    if recording_id.as_deref() == Some(id.as_str()) {
        return Err("Stop the meeting before deleting it".to_string());
    }
    meeting_store(&app)?.delete(&id)?;
    notify_updated(&app);
    Ok(())
}

#[tauri::command]
pub fn set_meeting_summary(app: AppHandle, summary: MeetingSummarySettings) -> Result<(), String> {
    let custom_provider = summary.provider == crate::cleanup::AiProviderId::Custom;
    if summary.model.trim().is_empty() && !custom_provider {
        return Err(format!("No model chosen for {}", summary.provider.as_str()));
    }
    config::update(&app, |s| s.meeting_summary = summary)
}
