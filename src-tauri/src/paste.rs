use arboard::Clipboard;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::thread;
use std::time::Duration;

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

#[tauri::command]
pub fn paste_text(text: String) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let previous = read_clipboard();
    write_clipboard(&text)?;

    thread::spawn(move || {
        thread::sleep(CLIPBOARD_PROPAGATE_DELAY);
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
