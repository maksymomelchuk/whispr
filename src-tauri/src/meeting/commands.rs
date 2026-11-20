use super::capture;
use super::processing::{self, ProcessingConfig};
use super::store::{
    new_meeting_id, validate_meeting_id, Meeting, MeetingListItem, MeetingMeta, MeetingStatus,
    MeetingStore,
};
use super::summary::MeetingSummarySettings;
use crate::config;
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Releases the Start reservation however `start_meeting` exits.
struct StartReservation<'a>(&'a AtomicBool);

impl Drop for StartReservation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn reserve_start(state: &AppState) -> Result<StartReservation<'_>, String> {
    let already_starting = state
        .meeting_starting
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err();
    if already_starting || state.active_meeting.lock().unwrap().is_some() {
        if !already_starting {
            state.meeting_starting.store(false, Ordering::Release);
        }
        return Err("A meeting is already recording".to_string());
    }
    Ok(StartReservation(&state.meeting_starting))
}

#[tauri::command]
pub async fn start_meeting(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<MeetingMeta, String> {
    start(&app, &state).await
}

#[tauri::command]
pub async fn stop_meeting(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop(app, &state).await
}

pub async fn start(app: &AppHandle, state: &AppState) -> Result<MeetingMeta, String> {
    let _reservation = reserve_start(state)?;
    let settings = config::load(app);
    ProcessingConfig::from_settings(&settings)?;
    let store = meeting_store(app)?;
    let mut meta = MeetingMeta {
        id: new_meeting_id(),
        started_at: crate::history::now_unix_seconds(),
        duration_ms: 0,
        status: MeetingStatus::Recording,
        summary_model: None,
        system_audio_error: None,
        system_offset_ms: 0,
        speaker_names: Default::default(),
        ambiguous_speaker_names: Default::default(),
    };
    store.create(&meta)?;
    let started = match capture::start(&store, &meta.id, settings.input_device).await {
        Ok(started) => started,
        Err(error) => {
            store.delete(&meta.id)?;
            return Err(error);
        }
    };
    meta.system_audio_error = started.system_audio_error;
    meta.system_offset_ms = started.system_offset_ms;
    store.save_meta(&meta)?;
    *state.active_meeting.lock().unwrap() = Some(started.meeting);
    // A dictation that began before Start may have muted output; the call
    // must stay audible while it's recorded.
    crate::ptt::maybe_resume_media(state);
    notify_updated(app);
    Ok(meta)
}

pub async fn stop(app: AppHandle, state: &AppState) -> Result<(), String> {
    let active = state
        .active_meeting
        .lock()
        .unwrap()
        .take()
        .ok_or("No meeting is recording")?;
    let id = active.id.clone();
    let store = meeting_store(&app)?;
    let stopped = tauri::async_runtime::spawn_blocking(move || active.stop())
        .await
        .map_err(|e| format!("stopping the recording failed: {e}"))
        .and_then(|result| result);
    let duration = match stopped {
        Ok(duration) => duration,
        Err(error) => {
            mark_failed(&app, &store, &id, error.clone());
            return Err(error);
        }
    };
    let mut meta = store.load_meta(&id)?;
    meta.duration_ms = duration.as_millis() as u64;
    store.save_meta(&meta)?;
    spawn_processing(app, store, id);
    Ok(())
}

#[tauri::command]
pub fn retry_meeting(app: AppHandle, id: String) -> Result<(), String> {
    validate_meeting_id(&id)?;
    let store = meeting_store(&app)?;
    if !store.load_meta(&id)?.status.is_failed() {
        return Err("Only a failed meeting can be retried".to_string());
    }
    spawn_processing(app, store, id);
    Ok(())
}

fn mark_failed(app: &AppHandle, store: &MeetingStore, id: &str, error: String) {
    if let Err(save_error) = store.update_status(id, MeetingStatus::Failed(error)) {
        eprintln!("[meeting] recording failure for {id} failed: {save_error}");
    }
    notify_updated(app);
}

/// Reports a missing key as a Failed status rather than an error, because
/// processing runs detached from the command that started it.
fn spawn_processing(app: AppHandle, store: MeetingStore, id: String) {
    let config = match ProcessingConfig::from_settings(&config::load(&app)) {
        Ok(config) => config,
        Err(error) => return mark_failed(&app, &store, &id, error),
    };
    let processing = app.state::<AppState>().meetings_processing.clone();
    if !processing.lock().unwrap().insert(id.clone()) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let notifier = app.clone();
        processing::run(store, id.clone(), config, move || notify_updated(&notifier)).await;
        processing.lock().unwrap().remove(&id);
    });
}

#[tauri::command]
pub fn list_meetings(app: AppHandle) -> Result<Vec<MeetingListItem>, String> {
    Ok(meeting_store(&app)?.list_items())
}

#[tauri::command]
pub fn get_meeting(app: AppHandle, id: String) -> Result<Meeting, String> {
    validate_meeting_id(&id)?;
    meeting_store(&app)?.load(&id)
}

#[tauri::command]
pub fn rename_meeting_speaker(
    app: AppHandle,
    id: String,
    label: String,
    name: String,
) -> Result<(), String> {
    validate_meeting_id(&id)?;
    meeting_store(&app)?.rename_speaker(&id, &label, &name)?;
    notify_updated(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_meeting(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    validate_meeting_id(&id)?;
    if state.meetings_processing.lock().unwrap().contains(&id) {
        return Err("Wait for this meeting to finish processing before deleting it".to_string());
    }
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
