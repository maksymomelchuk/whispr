#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDisplayServer {
    X11,
    Wayland,
    Unknown,
}

pub fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::MacOs
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "linux") {
        Os::Linux
    } else {
        Os::Other
    }
}

pub fn linux_display_server() -> LinuxDisplayServer {
    detect_linux_display_server(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").is_ok(),
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_os_matches_compile_target() {
        let os = current_os();
        #[cfg(target_os = "macos")]
        assert_eq!(os, Os::MacOs);
        #[cfg(target_os = "windows")]
        assert_eq!(os, Os::Windows);
        #[cfg(target_os = "linux")]
        assert_eq!(os, Os::Linux);
    }

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
