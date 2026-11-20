use super::commands;
use crate::state::AppState;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const OVERLAY_NOTICE_EVENT: &str = "overlay-notice";
/// Long enough to read a short sentence, short enough not to linger over work.
const NOTICE_DURATION: Duration = Duration::from_millis(1_800);
const ERROR_NOTICE_DURATION: Duration = Duration::from_millis(4_000);

#[derive(Clone, Serialize)]
struct OverlayNotice {
    text: String,
    is_error: bool,
}

/// Starts a meeting when none is recording, otherwise stops and summarizes
/// it. Feedback goes to the overlay pill because the settings window is
/// usually hidden while a call is on screen.
pub fn toggle(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let recording = state.active_meeting.lock().unwrap().is_some();
        let notice = if recording {
            commands::stop(app.clone(), &state)
                .await
                .map(|()| "Meeting stopped — writing notes".to_string())
        } else {
            commands::start(&app, &state)
                .await
                .map(|_| "Meeting recording".to_string())
        };
        match notice {
            Ok(text) => flash(&app, text, false).await,
            Err(error) => flash(&app, format!("Meeting: {error}"), true).await,
        }
    });
}

async fn flash(app: &AppHandle, text: String, is_error: bool) {
    crate::overlay::show(app);
    let _ = app.emit(OVERLAY_NOTICE_EVENT, OverlayNotice { text, is_error });
    let duration = if is_error {
        ERROR_NOTICE_DURATION
    } else {
        NOTICE_DURATION
    };
    tokio::time::sleep(duration).await;
    crate::overlay::hide(app);
}
