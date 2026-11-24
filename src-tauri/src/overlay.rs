use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_RESET_EVENT: &str = "overlay-reset";
const OVERLAY_MEETING_EVENT: &str = "overlay-meeting";

/// While a meeting is pinned the pill rests on a recording indicator instead
/// of hiding, so a forgotten recording can't run unnoticed. Dictation and
/// notices are transient: they take over the pill and fall back to the
/// meeting indicator when they end, and pinning or unpinning never disturbs
/// one that is on screen.
struct OverlayState {
    pinned_meeting_started_at: Option<i64>,
    transient_shown: bool,
}

static STATE: Mutex<OverlayState> = Mutex::new(OverlayState {
    pinned_meeting_started_at: None,
    transient_shown: false,
});
/// Logical points (multiplied by monitor scale for physical pixels). The
/// inner pill collapses via CSS, so the slack here stays click-through.
const OVERLAY_WIDTH: f64 = 640.0;
const OVERLAY_HEIGHT: f64 = 120.0;
/// Distance from monitor bottom to window bottom — the pill anchors to the
/// window bottom in CSS so this also fixes the pill's resting position. Both
/// platforms anchor to the full monitor frame; on Windows the pill therefore
/// draws on top of the taskbar (the window is topmost and click-through, so the
/// taskbar stays usable), and this margin tunes how far it sits over it.
const BOTTOM_MARGIN: f64 = 16.0;

pub fn create(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let builder =
        WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("index.html".into()))
            .title("Whispr Overlay")
            .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .shadow(false);

    #[cfg(target_os = "macos")]
    let builder = builder
        .accept_first_mouse(false)
        .visible_on_all_workspaces(true);

    let window = builder.build().map_err(|e| format!("overlay build: {e}"))?;

    // Click-through — events pass to whatever app is underneath, and we never
    // steal focus when shown.
    let _ = window.set_ignore_cursor_events(true);
    reposition(&window);

    #[cfg(target_os = "macos")]
    pin_over_fullscreen_spaces(&window);

    Ok(())
}

/// `visible_on_all_workspaces` only sets `CanJoinAllSpaces`, which does not
/// cover a window that owns its Space in native full-screen — over such an app
/// the pill would otherwise render behind it and stay invisible. Adding
/// `FullScreenAuxiliary` lets the pill draw into that Space; the status level
/// keeps it above the Dock and the app's own windows. Set once at create: the
/// window keeps these until it's destroyed.
#[cfg(target_os = "macos")]
fn pin_over_fullscreen_spaces(window: &WebviewWindow) {
    use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let Ok(ptr) = window.ns_window() else {
        return;
    };
    // SAFETY: tauri returns this webview's live NSWindow, and create() runs on
    // the main thread — the only thread AppKit permits these calls from.
    let ns_window: &NSWindow = unsafe { &*(ptr as *const NSWindow) };

    ns_window.setLevel(NSStatusWindowLevel);
    let behavior = ns_window.collectionBehavior()
        | NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary;
    ns_window.setCollectionBehavior(behavior);
}

/// Recalculate position on the primary monitor's bottom center. Called both
/// on create and on every show — the user may have moved windows between
/// displays or changed resolution since the last recording.
fn reposition(window: &WebviewWindow) {
    let monitor = match window.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();

    let origin = *monitor.position();
    let extent = *monitor.size();

    let win_w = (OVERLAY_WIDTH * scale) as i32;
    let win_h = (OVERLAY_HEIGHT * scale) as i32;
    let margin = (BOTTOM_MARGIN * scale) as i32;
    let x = origin.x + (extent.width as i32 - win_w) / 2;
    let y = origin.y + extent.height as i32 - win_h - margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub fn show(app: &AppHandle) {
    STATE.lock().unwrap().transient_shown = true;
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        present(&window);
    }
}

fn present(window: &WebviewWindow) {
    reposition(window);
    let _ = window.show();
    raise_above_taskbar(window);
}

/// The Windows taskbar is also topmost; z-order within the topmost band goes to
/// whoever re-inserted last, so without this the taskbar's next repaint draws
/// over us. tao's set_always_on_top can't do this — it diffs window flags and
/// no-ops once ALWAYS_ON_TOP is set (it was, at build), so it never re-inserts.
/// SetWindowPos re-inserts unconditionally.
#[cfg(target_os = "windows")]
fn raise_above_taskbar(window: &WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    unsafe {
        SetWindowPos(
            hwnd.0 as _,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn raise_above_taskbar(_window: &WebviewWindow) {}

pub fn hide(app: &AppHandle) {
    let pinned = {
        let mut state = STATE.lock().unwrap();
        state.transient_shown = false;
        state.pinned_meeting_started_at
    };
    // Reset before hiding so the next show() paints a clean pill instead
    // of one frame of stale preview while ptt-pressed is still in flight.
    let _ = app.emit(OVERLAY_RESET_EVENT, ());
    match pinned {
        Some(started_at) => {
            let _ = app.emit(OVERLAY_MEETING_EVENT, started_at);
        }
        None => hide_window(app),
    }
}

pub fn pin_meeting(app: &AppHandle, started_at: i64) {
    let transient_shown = {
        let mut state = STATE.lock().unwrap();
        state.pinned_meeting_started_at = Some(started_at);
        state.transient_shown
    };
    if transient_shown {
        return;
    }
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        present(&window);
    }
    let _ = app.emit(OVERLAY_MEETING_EVENT, started_at);
}

pub fn unpin_meeting(app: &AppHandle) {
    let transient_shown = {
        let mut state = STATE.lock().unwrap();
        state.pinned_meeting_started_at = None;
        state.transient_shown
    };
    if transient_shown {
        return;
    }
    let _ = app.emit(OVERLAY_RESET_EVENT, ());
    hide_window(app);
}

/// The state is re-read after hiding rather than locked across it: window
/// calls hop to the main thread, which may itself be waiting on the lock. A
/// show or pin that landed while we were hiding re-presents the window.
fn hide_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let _ = window.hide();
    let (transient_shown, pinned) = {
        let state = STATE.lock().unwrap();
        (state.transient_shown, state.pinned_meeting_started_at)
    };
    if transient_shown {
        present(&window);
        return;
    }
    if let Some(started_at) = pinned {
        present(&window);
        let _ = app.emit(OVERLAY_MEETING_EVENT, started_at);
    }
}
