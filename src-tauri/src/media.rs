//! System-audio mute helpers.
//!
//! We toggle the macOS output-mute state instead of sending Now Playing
//! pause/play commands. The mute approach is app-agnostic (works for
//! browser-based players, Spotify, YouTube, Discord, anything producing
//! audio) and — crucially — doesn't get the "paused media gets un-paused
//! on release" wrong, because we never touch playback state. If nothing
//! was playing, mute/unmute are audibly no-ops; if something was playing,
//! it's silenced during the recording and returns at the same spot.
//!
//! Tradeoff vs. a true pause: playback keeps advancing while we dictate,
//! so a long session eats into the song. That matches the `Handy` open-
//! source dictation app's behavior and avoids all the MediaRemote query
//! gating, TCC prompts, and Obj-C block plumbing we'd otherwise need for
//! real pause/resume with accurate state detection.

use std::process::Command;

fn set_output_muted(muted: bool) {
    let script = format!(
        "set volume output muted {}",
        if muted { "true" } else { "false" }
    );
    // Fire-and-forget. osascript prints to stderr on failure, which is fine;
    // nothing we can do to recover if the user's system lacks osascript.
    let _ = Command::new("osascript").args(["-e", &script]).output();
}

pub fn mute_output() {
    set_output_muted(true);
}

pub fn unmute_output() {
    set_output_muted(false);
}
