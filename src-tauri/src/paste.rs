use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::thread;
use std::time::{Duration, Instant};
use tauri::async_runtime::{self, JoinHandle};

// core-graphics doesn't expose CGEventSourceFlagsState. Redeclare the symbol
// against the framework the crate already links — used to wait out any
// modifiers the user is still holding from the PTT shortcut before we inject
// keystrokes that would otherwise merge with them.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: CGEventSourceStateID) -> u64;
}

/// CGEventKeyboardSetUnicodeString quietly drops or mangles long strings in
/// some targets (Electron apps in particular). Splitting into small chunks
/// keeps delivery reliable across Slack, VS Code, Safari, etc.
const CHUNK_SIZE: usize = 20;
/// Breathing room between chunks so busy receivers don't coalesce or drop
/// adjacent events.
const INTER_CHUNK_DELAY: Duration = Duration::from_millis(2);
/// CGEvent::post queues asynchronously at the HID layer; hold long enough
/// for queued events to land in the target app before the caller raises a
/// window.
const DRAIN_DELAY: Duration = Duration::from_millis(80);
/// Upper bound on how long we'll stall waiting for the user to finish
/// releasing their PTT modifiers. Better to type with stale modifiers than
/// hang forever if something is genuinely held.
const MODIFIER_SETTLE_TIMEOUT: Duration = Duration::from_millis(250);
const MODIFIER_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Wait until the user has released every modifier at the HID level. We post
/// at CGEventTapLocation::HID, which ORs our event's flags with the user's
/// currently-held modifiers. With clipboard+Cmd+V that only mattered for
/// non-Command modifiers; with direct Unicode injection *any* held modifier
/// can cause the target app to interpret the event as a shortcut instead of
/// typed text.
fn wait_for_modifier_release() {
    let mask = (CGEventFlags::CGEventFlagCommand
        | CGEventFlags::CGEventFlagAlternate
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
                "paste: modifiers still held after {:?} (flags=0x{:x}); typing anyway",
                MODIFIER_SETTLE_TIMEOUT, flags
            );
            return;
        }
        thread::sleep(MODIFIER_POLL_INTERVAL);
    }
}

/// Synthesize a key-down/key-up pair whose Unicode string is `chunk`. The
/// keycode we use doesn't matter — setting the Unicode string overrides the
/// layout translation — but some apps only react on key-down while others
/// need the pair, so we post both.
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

/// Caller must await before raising any window — set_focus() during
/// wait_for_modifier_release (up to 250ms) lands keystrokes in Whispr.
pub fn paste_text(text: String) -> JoinHandle<()> {
    async_runtime::spawn_blocking(move || {
        if text.is_empty() {
            return;
        }
        wait_for_modifier_release();

        // chunk by characters, not bytes — arbitrary UTF-8 byte splits would
        // corrupt multi-byte sequences when converted to UTF-16 downstream.
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;
        while start < chars.len() {
            let end = next_chunk_end(&chars, start);
            let chunk: String = chars[start..end].iter().collect();
            if let Err(e) = post_unicode(&chunk) {
                eprintln!("[paste] post_unicode failed: {e}");
                break;
            }
            start = end;
            thread::sleep(INTER_CHUNK_DELAY);
        }
        thread::sleep(DRAIN_DELAY);
    })
}
