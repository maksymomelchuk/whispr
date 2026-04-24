//! Minimal shim over Apple's private `MediaRemote.framework`.
//!
//! Lets us send play/pause commands to the current "Now Playing" app —
//! the same underlying mechanism behind the macOS media keys and tools
//! like BetterTouchTool. We dlopen the framework at runtime, so a future
//! macOS that removes or renames it degrades to a silent no-op (no crashes,
//! no false failures — the feature just stops working).

use crate::debug_log;
use libloading::Library;
use std::ffi::c_void;
use std::sync::OnceLock;

const MEDIAREMOTE_PATH: &str =
    "/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote";

// MRMediaRemoteCommand enum (integer-tagged in the header).
const CMD_PLAY: u32 = 0;
const CMD_PAUSE: u32 = 1;

fn lib() -> Option<&'static Library> {
    static LIB: OnceLock<Option<Library>> = OnceLock::new();
    LIB.get_or_init(|| unsafe { Library::new(MEDIAREMOTE_PATH).ok() })
        .as_ref()
}

fn send_command(cmd: u32) -> bool {
    let Some(l) = lib() else {
        debug_log!("[media] MediaRemote not loaded, skipping cmd {cmd}");
        return false;
    };
    type SendFn = unsafe extern "C" fn(u32, *const c_void) -> bool;
    unsafe {
        match l.get::<SendFn>(b"MRMediaRemoteSendCommand") {
            Ok(f) => f(cmd, std::ptr::null()),
            Err(e) => {
                debug_log!("[media] MRMediaRemoteSendCommand lookup failed: {e}");
                false
            }
        }
    }
}

pub fn pause() {
    let ok = send_command(CMD_PAUSE);
    debug_log!("[media] pause sent (ok={ok})");
}

pub fn play() {
    let ok = send_command(CMD_PLAY);
    debug_log!("[media] play sent (ok={ok})");
}
