use arboard::Clipboard;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::thread;
use std::time::{Duration, Instant};

// core-graphics doesn't expose CGEventSourceFlagsState. Redeclare the symbol
// against the framework the crate already links — used below to wait out any
// modifiers the user is still holding from the PTT shortcut before we post
// Cmd+V.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: CGEventSourceStateID) -> u64;
}

const KEY_V: u16 = 0x09;
/// Tiny delay between writing the clipboard and firing Cmd+V. Without it,
/// fast Macs can deliver the paste before NSPasteboard has finished
/// publishing the new value, and the target app pastes whatever was there
/// before.
const CLIPBOARD_PROPAGATE_DELAY: Duration = Duration::from_millis(40);
/// How long to wait after Cmd+V before restoring the previous clipboard.
/// Long enough for the focused app to actually consume the paste, short
/// enough that the user won't notice their clipboard shimmering.
const RESTORE_DELAY: Duration = Duration::from_millis(200);
/// Upper bound on how long we'll stall the paste waiting for the user to
/// finish releasing their PTT modifiers. If they're genuinely still holding
/// something (e.g. Shift for some unrelated reason) we'd rather paste with
/// the wrong flags than hang indefinitely.
const MODIFIER_SETTLE_TIMEOUT: Duration = Duration::from_millis(250);
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(5);

fn read_clipboard() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

fn write_clipboard(text: &str) -> Result<(), String> {
    Clipboard::new()
        .map_err(|e| format!("Clipboard init failed: {e}"))?
        .set_text(text.to_owned())
        .map_err(|e| format!("Clipboard write failed: {e}"))
}

/// Post a single keyboard event at the HID level. Each call creates its own
/// event source — the core-graphics crate consumes the source when building
/// the event, so we can't reuse it across the down/up pair.
fn post_key(keycode: u16, keydown: bool, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "CGEventSource::new failed".to_string())?;
    let event = CGEvent::new_keyboard_event(source, keycode, keydown)
        .map_err(|_| format!("new_keyboard_event(down={keydown}) failed"))?;
    event.set_flags(flags);
    event.post(CGEventTapLocation::HID);
    Ok(())
}

fn post_cmd_v() -> Result<(), String> {
    let flags = CGEventFlags::CGEventFlagCommand;
    post_key(KEY_V, true, flags)?;
    post_key(KEY_V, false, flags)?;
    Ok(())
}

/// Wait until the user has released every non-Command modifier at the HID
/// level. We post the Cmd+V at HIDSystemState, which means the effective
/// flags on the event become our CGEventFlagCommand *OR'd with* whatever the
/// user is physically still holding. If the PTT shortcut was Option-based
/// (the default, AltRight) or used Shift/Ctrl, the target app sees
/// Option+Cmd+V / Shift+Cmd+V / Ctrl+Cmd+V — which most apps treat as no-op
/// or a different command, and the paste silently fails. Polling until the
/// physical state clears eliminates the race.
///
/// Command is intentionally excluded: the user may use a Cmd-based shortcut,
/// and our synthetic Cmd flag already covers the case where theirs is gone.
fn wait_for_modifier_release() {
    let mask = (CGEventFlags::CGEventFlagAlternate
        | CGEventFlags::CGEventFlagShift
        | CGEventFlags::CGEventFlagControl)
        .bits();
    let start = Instant::now();
    loop {
        let flags = unsafe { CGEventSourceFlagsState(CGEventSourceStateID::CombinedSessionState) };
        if flags & mask == 0 {
            return;
        }
        if start.elapsed() >= MODIFIER_SETTLE_TIMEOUT {
            eprintln!(
                "paste: modifiers still held after {:?} (flags=0x{:x}); pasting anyway",
                MODIFIER_SETTLE_TIMEOUT, flags
            );
            return;
        }
        thread::sleep(MODIFIER_POLL_INTERVAL);
    }
}

pub fn paste_text(text: String) -> Result<(), String> {
    println!("[paste] invoked, len={}", text.len());
    if text.is_empty() {
        println!("[paste] empty text, skipping");
        return Ok(());
    }

    let previous = read_clipboard();
    write_clipboard(&text)?;

    thread::spawn(move || {
        thread::sleep(CLIPBOARD_PROPAGATE_DELAY);
        wait_for_modifier_release();
        println!("[paste] posting Cmd+V");
        if let Err(e) = post_cmd_v() {
            eprintln!("CGEventPost paste failed: {e}");
        }
        thread::sleep(RESTORE_DELAY);
        if let Some(prev) = previous {
            let _ = write_clipboard(&prev);
        }
    });

    Ok(())
}
