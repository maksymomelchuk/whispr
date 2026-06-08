#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;

#[cfg(target_os = "macos")]
fn set_output_muted(muted: bool) {
    let script = format!(
        "set volume output muted {}",
        if muted { "true" } else { "false" }
    );
    // Fire-and-forget. osascript prints to stderr on failure, which is fine;
    // nothing we can do to recover if the user's system lacks osascript.
    let _ = Command::new("osascript").args(["-e", &script]).output();
}

// pactl is shipped with PulseAudio and PipeWire (as a compat shim). Fire-and-
// forget: if it isn't installed the spawn fails silently.
#[cfg(target_os = "linux")]
fn set_output_muted(muted: bool) {
    let value = if muted { "1" } else { "0" };
    let _ = Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", value])
        .output();
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn set_output_muted(_muted: bool) {}

pub fn mute_output() {
    set_output_muted(true);
}

pub fn unmute_output() {
    set_output_muted(false);
}
