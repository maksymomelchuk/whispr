use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const OVERLAY_LABEL: &str = "overlay";
/// Logical size in points — the window's physical pixel size is this times
/// the monitor scale factor.
const OVERLAY_WIDTH: f64 = 110.0;
const OVERLAY_HEIGHT: f64 = 32.0;
/// Distance from the bottom edge of the primary monitor, in logical points.
const BOTTOM_MARGIN: f64 = 28.0;

pub fn create(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Wispr Overlay")
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .visible_on_all_workspaces(true)
    .accept_first_mouse(false)
    .shadow(false)
    .build()
    .map_err(|e| format!("overlay build: {e}"))?;

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
    let size = monitor.size();
    let pos = monitor.position();
    let win_w = (OVERLAY_WIDTH * scale) as i32;
    let win_h = (OVERLAY_HEIGHT * scale) as i32;
    let margin = (BOTTOM_MARGIN * scale) as i32;
    let x = pos.x + (size.width as i32 - win_w) / 2;
    let y = pos.y + size.height as i32 - win_h - margin;
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
        let _ = window.hide();
    }
}
