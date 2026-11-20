use super::commands;
use crate::state::AppState;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const OVERLAY_NOTICE_EVENT: &str = "overlay-notice";
/// Long enough to read a short sentence, short enough not to linger over work.
const NOTICE_DURATION: Duration = Duration::from_millis(1_800);
const ERROR_NOTICE_DURATION: Duration = Duration::from_millis(4_000);

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NoticeKind {
    MeetingStarted,
    MeetingStopped,
    Error,
}

#[derive(Clone, Serialize)]
struct OverlayNotice {
    text: String,
    kind: NoticeKind,
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
                .map(|()| ("Writing notes…", NoticeKind::MeetingStopped))
        } else {
            commands::start(&app, &state)
                .await
                .map(|_| ("Recording meeting", NoticeKind::MeetingStarted))
        };
        match notice {
            Ok((text, kind)) => flash(&app, text.to_string(), kind).await,
            Err(error) => flash(&app, format!("Meeting: {error}"), NoticeKind::Error).await,
        }
    });
}

async fn flash(app: &AppHandle, text: String, kind: NoticeKind) {
    crate::overlay::show(app);
    let _ = app.emit(OVERLAY_NOTICE_EVENT, OverlayNotice { text, kind });
    let duration = if kind == NoticeKind::Error {
        ERROR_NOTICE_DURATION
    } else {
        NOTICE_DURATION
    };
    tokio::time::sleep(duration).await;
    crate::overlay::hide(app);
}
