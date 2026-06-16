use std::ffi::c_void;

use objc2_app_kit::{NSWindow, NSWindowButton};
use objc2_foundation::NSPoint;

// Gap from the top of the 44px drag header (`h-11` in AppShell) to the top of
// the buttons. With `titleBarStyle: Overlay`, macOS pins the traffic lights for
// a standard ~28px title bar, so they ride high in the taller header; this
// nudges them down to sit just above the header's vertical center.
const BUTTON_TOP_INSET: f64 = 12.0;

/// Vertically center the close/minimize/zoom buttons in the custom 44px header.
/// AppKit re-lays the buttons on resize, so this must run again on every
/// `Resized` event, not just at setup — it is idempotent.
pub fn reposition(ns_window: *mut c_void) {
    if ns_window.is_null() {
        return;
    }
    // Tauri hands us the main window's live NSWindow on the main thread; we
    // only borrow it for the duration of this call.
    let window: &NSWindow = unsafe { &*ns_window.cast::<NSWindow>() };

    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let Some(button) = window.standardWindowButton(kind) else {
            continue;
        };
        let Some(frame_view) = (unsafe { button.superview() }) else {
            continue;
        };
        // The buttons live in the theme frame, whose coordinate origin is the
        // window's bottom-left, so the target Y depends on the window height.
        // The buttons sit in the theme frame, whose origin is the window's
        // bottom-left, so the target Y is measured down from its top edge.
        let frame_view_height = frame_view.frame().size.height;
        let button_height = button.frame().size.height;
        let origin = NSPoint::new(
            button.frame().origin.x,
            frame_view_height - BUTTON_TOP_INSET - button_height,
        );
        button.setFrameOrigin(origin);
    }
}
