use std::thread;
use std::time::Duration;
use tauri::async_runtime::{self, JoinHandle};

#[cfg(target_os = "linux")]
use crate::platform::LinuxDisplayServer;

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

const CHUNK_SIZE: usize = 20;
const INTER_CHUNK_DELAY: Duration = Duration::from_millis(2);
const DRAIN_DELAY: Duration = Duration::from_millis(20);

#[cfg(target_os = "macos")]
const MODIFIER_SETTLE_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(target_os = "macos")]
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(5);

// core-graphics doesn't expose CGEventSourceFlagsState. Redeclare the symbol
// against the framework the crate already links — used to wait out any
// modifiers the user is still holding from the PTT shortcut before we inject
// keystrokes that would otherwise merge with them.
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: CGEventSourceStateID) -> u64;
}

/// CGEvent::post queues asynchronously at the HID layer; hold long enough
/// for queued events to land in the target app before the caller raises a
/// window.
#[cfg(target_os = "macos")]
fn wait_for_modifier_release() {
    let mask = (CGEventFlags::CGEventFlagCommand
        | CGEventFlags::CGEventFlagAlternate
        | CGEventFlags::CGEventFlagShift
        | CGEventFlags::CGEventFlagControl)
        .bits();
    let start = std::time::Instant::now();
    loop {
        let flags = unsafe { CGEventSourceFlagsState(CGEventSourceStateID::CombinedSessionState) };
        if flags & mask == 0 {
            return;
        }
        if start.elapsed() >= MODIFIER_SETTLE_TIMEOUT {
            eprintln!(
                "paste: modifiers still held after {:?} (flags=0x{:x}); typing anyway",
                MODIFIER_SETTLE_TIMEOUT, flags
            );
            return;
        }
        thread::sleep(MODIFIER_POLL_INTERVAL);
    }
}

/// CGEventKeyboardSetUnicodeString quietly drops or mangles long strings in
/// some targets (Electron apps in particular). Splitting into small chunks
/// keeps delivery reliable across Slack, VS Code, Safari, etc.
#[cfg(target_os = "macos")]
fn post_unicode(chunk: &str) -> Result<(), String> {
    for keydown in [true, false] {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "CGEventSource::new failed".to_string())?;
        let event = CGEvent::new_keyboard_event(source, 0, keydown)
            .map_err(|_| format!("new_keyboard_event(down={keydown}) failed"))?;
        event.set_flags(CGEventFlags::empty());
        event.set_string(chunk);
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

// ── Linux injector selection (pure) ──────────────────────────────────────────

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxInjector {
    Wtype,
    Dotool,
    Ydotool,
    Xdotool,
    Enigo,
}

#[cfg(target_os = "linux")]
pub struct AvailableTools {
    pub wtype: bool,
    pub dotool: bool,
    pub ydotool: bool,
    pub xdotool: bool,
}

/// Selects the Linux injector from the detected display server and which tools
/// are present on PATH. Wayland prefers native Wayland tools; X11 prefers
/// xdotool; both fall back to enigo when no native tool is available.
#[cfg(target_os = "linux")]
pub fn select_linux_injector(server: LinuxDisplayServer, tools: &AvailableTools) -> LinuxInjector {
    match server {
        LinuxDisplayServer::Wayland => {
            if tools.wtype {
                LinuxInjector::Wtype
            } else if tools.dotool {
                LinuxInjector::Dotool
            } else if tools.ydotool {
                LinuxInjector::Ydotool
            } else {
                LinuxInjector::Enigo
            }
        }
        LinuxDisplayServer::X11 => {
            if tools.xdotool {
                LinuxInjector::Xdotool
            } else {
                LinuxInjector::Enigo
            }
        }
        LinuxDisplayServer::Unknown => LinuxInjector::Enigo,
    }
}

#[cfg(target_os = "linux")]
fn tool_on_path(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn probe_available_tools() -> AvailableTools {
    AvailableTools {
        wtype: tool_on_path("wtype"),
        dotool: tool_on_path("dotool"),
        ydotool: tool_on_path("ydotool"),
        xdotool: tool_on_path("xdotool"),
    }
}

#[cfg(target_os = "linux")]
fn inject_wtype(text: &str) -> Result<(), String> {
    let status = std::process::Command::new("wtype")
        // -- terminates option parsing so text starting with '-' is typed
        // literally instead of being misread as a flag.
        .args(["--", text])
        .status()
        .map_err(|e| format!("wtype failed: {e}"))?;
    if !status.success() {
        return Err(format!("wtype exited with {status}"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_dotool(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = std::process::Command::new("dotool")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("dotool failed: {e}"))?;
    {
        // Drop stdin before wait() — dotool reads until EOF, so leaving the
        // pipe open would hang the process indefinitely.
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "dotool: no stdin".to_string())?;
        stdin
            .write_all(format!("type {text}\n").as_bytes())
            .map_err(|e| format!("dotool write failed: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("dotool wait failed: {e}"))?;
    if !status.success() {
        return Err(format!("dotool exited with {status}"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_ydotool(text: &str) -> Result<(), String> {
    let status = std::process::Command::new("ydotool")
        .args(["type", text])
        .status()
        .map_err(|e| format!("ydotool failed: {e}"))?;
    if !status.success() {
        return Err(format!("ydotool exited with {status}"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_xdotool(text: &str) -> Result<(), String> {
    // --clearmodifiers releases held PTT modifier keys before typing so they
    // don't corrupt the injected text. -- terminates option parsing so text
    // starting with '-' is typed literally instead of being misread as a flag.
    let status = std::process::Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--", text])
        .status()
        .map_err(|e| format!("xdotool failed: {e}"))?;
    if !status.success() {
        return Err(format!("xdotool exited with {status}"));
    }
    Ok(())
}

// Enigo is the fallback injector on Windows and Linux. It uses platform
// SendInput (Windows) / XSendEvent (Linux/X11) under the hood.
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn inject_enigo(text: &str) -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("enigo init failed: {e}"))?;
    // Release held PTT modifier keys before typing so they don't corrupt
    // the injected text.
    for key in [Key::Shift, Key::Control, Key::Alt, Key::Meta] {
        let _ = enigo.key(key, Direction::Release);
    }
    enigo
        .text(text)
        .map_err(|e| format!("enigo text failed: {e}"))
}

// ── Pure chunk boundary logic ─────────────────────────────────────────────────

/// Pick the chunk end: hard cap at CHUNK_SIZE, but back up to the last
/// whitespace within the chunk so boundaries land between words. Receivers
/// that occasionally inject a space at the boundary then drop it where a
/// space already belongs. Only a chunk containing no whitespace at all
/// (e.g. a long URL) falls back to the hard split.
fn next_chunk_end(chars: &[char], start: usize) -> usize {
    let hard_end = (start + CHUNK_SIZE).min(chars.len());
    if hard_end == chars.len() {
        return hard_end;
    }
    for i in ((start + 1)..hard_end).rev() {
        if chars[i - 1].is_whitespace() {
            return i;
        }
    }
    hard_end
}

// ── OS injection entry points ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn inject_text(text: &str) -> Result<(), String> {
    wait_for_modifier_release();
    // chunk by characters, not bytes — arbitrary UTF-8 byte splits would
    // corrupt multi-byte sequences when converted to UTF-16 downstream.
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    while start < chars.len() {
        let end = next_chunk_end(&chars, start);
        let chunk: String = chars[start..end].iter().collect();
        post_unicode(&chunk)?;
        start = end;
        thread::sleep(INTER_CHUNK_DELAY);
    }
    thread::sleep(DRAIN_DELAY);
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_text(text: &str) -> Result<(), String> {
    // chunk by characters, not bytes — arbitrary UTF-8 byte splits would
    // corrupt multi-byte sequences when converted to UTF-16 downstream.
    // Chunking prevents dropped characters in Electron and browser targets
    // that coalesce rapid SendInput events.
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    while start < chars.len() {
        let end = next_chunk_end(&chars, start);
        let chunk: String = chars[start..end].iter().collect();
        inject_enigo(&chunk)?;
        start = end;
        thread::sleep(INTER_CHUNK_DELAY);
    }
    thread::sleep(DRAIN_DELAY);
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_text(text: &str) -> Result<(), String> {
    let server = crate::platform::linux_display_server();
    let tools = probe_available_tools();
    match select_linux_injector(server, &tools) {
        LinuxInjector::Wtype => inject_wtype(text),
        LinuxInjector::Dotool => inject_dotool(text),
        LinuxInjector::Ydotool => inject_ydotool(text),
        LinuxInjector::Xdotool => inject_xdotool(text),
        // Enigo on Linux uses the same chunked path as Windows to avoid
        // dropped characters in Electron and browser targets.
        LinuxInjector::Enigo => {
            let chars: Vec<char> = text.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                let end = next_chunk_end(&chars, start);
                let chunk: String = chars[start..end].iter().collect();
                inject_enigo(&chunk)?;
                start = end;
                thread::sleep(INTER_CHUNK_DELAY);
            }
            thread::sleep(DRAIN_DELAY);
            Ok(())
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn inject_text(_text: &str) -> Result<(), String> {
    Err("paste not supported on this platform".to_string())
}

/// Caller must await before raising any window — modifier-release logic during
/// injection can take up to 250ms on macOS (and on Windows/Linux the injection
/// itself may take measurable time for long texts).
pub fn paste_text(text: String) -> JoinHandle<()> {
    async_runtime::spawn_blocking(move || {
        if text.is_empty() {
            return;
        }
        if let Err(e) = inject_text(&text) {
            eprintln!("[paste] {e}");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_end_returns_full_length_when_text_fits_in_one_chunk() {
        let chars: Vec<char> = "hello world".chars().collect();
        assert_eq!(next_chunk_end(&chars, 0), 11);
    }

    #[test]
    fn chunk_end_backs_up_to_last_word_boundary() {
        // "hello world foo bar baz quux" (28 chars)
        // hard_end = 20; chars[15]=' ' is the last space before position 20
        let chars: Vec<char> = "hello world foo bar baz quux".chars().collect();
        assert_eq!(next_chunk_end(&chars, 0), 16);
    }

    #[test]
    fn chunk_end_falls_back_to_hard_cap_when_no_whitespace() {
        let chars: Vec<char> = "a".repeat(30).chars().collect();
        assert_eq!(next_chunk_end(&chars, 0), CHUNK_SIZE);
    }

    #[test]
    fn chunk_end_advances_from_mid_string() {
        let chars: Vec<char> = "hello world foo bar".chars().collect();
        // start=6: "world foo bar" (13 chars, fits in 20) → returns len
        assert_eq!(next_chunk_end(&chars, 6), chars.len());
    }

    // ── Linux injector selection ──────────────────────────────────────────────

    #[cfg(target_os = "linux")]
    mod linux_injector {
        use super::*;
        use crate::platform::LinuxDisplayServer;

        fn no_tools() -> AvailableTools {
            AvailableTools {
                wtype: false,
                dotool: false,
                ydotool: false,
                xdotool: false,
            }
        }

        #[test]
        fn wayland_prefers_wtype() {
            let tools = AvailableTools {
                wtype: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &tools),
                LinuxInjector::Wtype
            );
        }

        #[test]
        fn wayland_falls_back_to_dotool_when_wtype_absent() {
            let tools = AvailableTools {
                dotool: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &tools),
                LinuxInjector::Dotool
            );
        }

        #[test]
        fn wayland_falls_back_to_ydotool_when_wtype_and_dotool_absent() {
            let tools = AvailableTools {
                ydotool: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &tools),
                LinuxInjector::Ydotool
            );
        }

        #[test]
        fn wayland_falls_back_to_enigo_when_no_wayland_tools_present() {
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &no_tools()),
                LinuxInjector::Enigo
            );
        }

        #[test]
        fn wayland_xdotool_present_does_not_select_xdotool() {
            // xdotool is an X11 tool; on Wayland it should not be chosen
            let tools = AvailableTools {
                xdotool: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &tools),
                LinuxInjector::Enigo
            );
        }

        #[test]
        fn wayland_prefers_wtype_when_all_tools_present() {
            let tools = AvailableTools {
                wtype: true,
                dotool: true,
                ydotool: true,
                xdotool: true,
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Wayland, &tools),
                LinuxInjector::Wtype
            );
        }

        #[test]
        fn x11_prefers_xdotool() {
            let tools = AvailableTools {
                xdotool: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::X11, &tools),
                LinuxInjector::Xdotool
            );
        }

        #[test]
        fn x11_falls_back_to_enigo_when_xdotool_absent() {
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::X11, &no_tools()),
                LinuxInjector::Enigo
            );
        }

        #[test]
        fn unknown_server_always_uses_enigo() {
            let tools = AvailableTools {
                wtype: true,
                xdotool: true,
                ..no_tools()
            };
            assert_eq!(
                select_linux_injector(LinuxDisplayServer::Unknown, &tools),
                LinuxInjector::Enigo
            );
        }
    }
}
