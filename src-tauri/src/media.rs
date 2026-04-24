//! Minimal shim over Apple's private `MediaRemote.framework`.
//!
//! Lets us send play/pause commands to the current "Now Playing" app —
//! the same underlying mechanism behind the macOS media keys and tools
//! like BetterTouchTool. We dlopen the framework at runtime, so a future
//! macOS that removes or renames it degrades to a silent no-op (no crashes,
//! no false failures — the feature just stops working).

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

fn send_command(cmd: u32) {
    let Some(l) = lib() else { return };
    type SendFn = unsafe extern "C" fn(u32, *const c_void) -> bool;
    unsafe {
        if let Ok(f) = l.get::<SendFn>(b"MRMediaRemoteSendCommand") {
            let _ = f(cmd, std::ptr::null());
        }
    }
}

pub fn pause() {
    send_command(CMD_PAUSE);
}

pub fn play() {
    send_command(CMD_PLAY);
}
