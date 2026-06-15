/// True when a password field anywhere has focus. macOS sets secure event
/// input (the keylogger-blocking mechanism) for native AND web password fields
/// — Chromium/WebKit `<input type=password>` report role `AXTextField`, not
/// `AXSecureTextField`, so role-matching alone fails open on browsers. Fails
/// safe: a stuck flag only over-blocks capture, never leaks.
#[cfg(target_os = "macos")]
pub fn is_secure_input_active() -> bool {
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn IsSecureEventInputEnabled() -> bool;
    }
    unsafe { IsSecureEventInputEnabled() }
}

/// Windows secure detection is per-element via `UIA_IsPasswordPropertyId` at the
/// capture site; there is no global flag to consult here.
#[cfg(not(target_os = "macos"))]
pub fn is_secure_input_active() -> bool {
    false
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDisplayServer {
    X11,
    Wayland,
    Unknown,
}

#[cfg(target_os = "linux")]
pub fn linux_display_server() -> LinuxDisplayServer {
    detect_linux_display_server(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").is_ok(),
    )
}

#[cfg(target_os = "linux")]
fn detect_linux_display_server(
    xdg_session_type: Option<&str>,
    wayland_display_present: bool,
) -> LinuxDisplayServer {
    // XDG_SESSION_TYPE is the authoritative source; WAYLAND_DISPLAY is a
    // fallback set by most compositors even when XDG_SESSION_TYPE is absent.
    match xdg_session_type {
        Some("wayland") => LinuxDisplayServer::Wayland,
        Some("x11") => LinuxDisplayServer::X11,
        _ => {
            if wayland_display_present {
                LinuxDisplayServer::Wayland
            } else {
                LinuxDisplayServer::Unknown
            }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn xdg_wayland_detected() {
        assert_eq!(
            detect_linux_display_server(Some("wayland"), false),
            LinuxDisplayServer::Wayland
        );
    }

    #[test]
    fn xdg_wayland_detected_regardless_of_wayland_display() {
        assert_eq!(
            detect_linux_display_server(Some("wayland"), true),
            LinuxDisplayServer::Wayland
        );
    }

    #[test]
    fn xdg_x11_detected() {
        assert_eq!(
            detect_linux_display_server(Some("x11"), false),
            LinuxDisplayServer::X11
        );
    }

    #[test]
    fn wayland_display_fallback_without_xdg() {
        assert_eq!(
            detect_linux_display_server(None, true),
            LinuxDisplayServer::Wayland
        );
    }

    #[test]
    fn no_env_vars_returns_unknown() {
        assert_eq!(
            detect_linux_display_server(None, false),
            LinuxDisplayServer::Unknown
        );
    }

    #[test]
    fn unknown_xdg_value_falls_through_to_wayland_display() {
        assert_eq!(
            detect_linux_display_server(Some("mir"), true),
            LinuxDisplayServer::Wayland
        );
        assert_eq!(
            detect_linux_display_server(Some("mir"), false),
            LinuxDisplayServer::Unknown
        );
    }

    #[test]
    fn empty_xdg_falls_through_to_wayland_display() {
        assert_eq!(
            detect_linux_display_server(Some(""), true),
            LinuxDisplayServer::Wayland
        );
        assert_eq!(
            detect_linux_display_server(Some(""), false),
            LinuxDisplayServer::Unknown
        );
    }
}
