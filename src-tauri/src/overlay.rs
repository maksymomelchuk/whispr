use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_RESET_EVENT: &str = "overlay-reset";
/// Logical points (multiplied by monitor scale for physical pixels). The
/// inner pill collapses via CSS, so the slack here stays click-through.
const OVERLAY_WIDTH: f64 = 640.0;
const OVERLAY_HEIGHT: f64 = 120.0;
/// Distance from monitor bottom to window bottom — the pill anchors to the
/// window bottom in CSS so this also fixes the pill's resting position.
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
    Ok(())
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

    // work_area excludes the taskbar/panel, so the pill rests above it rather
    // than clipped behind the always-on-top taskbar. macOS keeps the full frame
    // to preserve its tuned resting position.
    #[cfg(target_os = "macos")]
    let (origin, extent) = (*monitor.position(), *monitor.size());
    #[cfg(not(target_os = "macos"))]
    let (origin, extent) = {
        let area = monitor.work_area();
        (area.position, area.size)
    };

    let win_w = (OVERLAY_WIDTH * scale) as i32;
    let win_h = (OVERLAY_HEIGHT * scale) as i32;
    let margin = (BOTTOM_MARGIN * scale) as i32;
    let x = origin.x + (extent.width as i32 - win_w) / 2;
    let y = origin.y + extent.height as i32 - win_h - margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        reposition(&window);
        let _ = window.show();
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        // Reset before hiding so the next show() paints a clean pill instead
        // of one frame of stale preview while ptt-pressed is still in flight.
        let _ = app.emit(OVERLAY_RESET_EVENT, ());
        let _ = window.hide();
    }
}
