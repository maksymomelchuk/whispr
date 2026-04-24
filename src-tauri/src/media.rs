//! Minimal shim over Apple's private `MediaRemote.framework`.
//!
//! Lets us observe the current "Now Playing" app's play/pause state and send
//! it commands — the same underlying mechanism behind the macOS media keys
//! and tools like BetterTouchTool. We dlopen the framework at runtime, so
//! a future macOS that removes or renames it degrades to a silent no-op
//! (no crashes, no false failures — the feature just stops working).

use block2::RcBlock;
use libloading::Library;
use std::ffi::c_void;
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

const MEDIAREMOTE_PATH: &str =
    "/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote";

// MRMediaRemoteCommand enum (integer-tagged in the header).
const CMD_PLAY: u32 = 0;
const CMD_PAUSE: u32 = 1;

// Bounded wait for the MediaRemote query. The callback is normally posted in
// a few ms; we never want this to stall the PTT pipeline even if MediaRemote
// misbehaves.
const STATE_QUERY_TIMEOUT: Duration = Duration::from_millis(150);

fn lib() -> Option<&'static Library> {
    static LIB: OnceLock<Option<Library>> = OnceLock::new();
    LIB.get_or_init(|| unsafe { Library::new(MEDIAREMOTE_PATH).ok() })
        .as_ref()
}

// dispatch_get_global_queue lives in libSystem; link through the System
// umbrella framework the app already pulls in.
#[link(name = "System", kind = "framework")]
extern "C" {
    fn dispatch_get_global_queue(identifier: isize, flags: usize) -> *mut c_void;
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

/// Synchronously ask MediaRemote whether any app is currently playing. The
/// underlying API is async — we spin a global-queue block and wait on a
/// channel. Returns false on missing framework, timeout, or any other
/// failure so callers can always fall through to "don't pause".
pub fn is_playing() -> bool {
    let Some(l) = lib() else { return false };
    // The completion block receives an Obj-C BOOL, which on modern ARM64
    // macOS is a 1-byte bool; using Rust u8 for the callback arg sidesteps
    // block2's stricter type-encoding for Rust bool.
    type GetIsPlayingFn =
        unsafe extern "C" fn(*mut c_void, *const block2::Block<dyn Fn(u8)>);
    let sym = unsafe { l.get::<GetIsPlayingFn>(b"MRMediaRemoteGetNowPlayingApplicationIsPlaying") };
    let Ok(sym) = sym else { return false };

    let queue = unsafe { dispatch_get_global_queue(0, 0) };
    let (tx, rx) = mpsc::channel::<bool>();
    // Guard so the block's single-shot send is idempotent — Apple's docs
    // don't guarantee the completion fires exactly once.
    let tx = Mutex::new(Some(tx));
    let block = RcBlock::new(move |playing: u8| {
        if let Ok(mut guard) = tx.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(playing != 0);
            }
        }
    });

    unsafe { sym(queue, &*block) };
    rx.recv_timeout(STATE_QUERY_TIMEOUT).unwrap_or(false)
}
