use crate::state::{AppState, ModifierState};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    EventField,
};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// Per-modifier press state, updated by toggling on every FlagsChanged event.
/// macOS fires one FlagsChanged event each time a specific modifier key
/// transitions, so toggle-on-event gives us per-side (L/R) tracking that
/// the shared flag bitmask can't distinguish.
#[derive(Default)]
struct ModKeyState {
    l_alt: bool,
    r_alt: bool,
    l_meta: bool,
    r_meta: bool,
    l_control: bool,
    r_control: bool,
    l_shift: bool,
    r_shift: bool,
}

/// Map macOS virtual keycodes (from <Carbon/HIToolbox/Events.h>) to the
/// KeyboardEvent.code strings the frontend uses.
fn macos_keycode_to_code(kc: u16) -> Option<&'static str> {
    Some(match kc {
        0x00 => "KeyA",
        0x0B => "KeyB",
        0x08 => "KeyC",
        0x02 => "KeyD",
        0x0E => "KeyE",
        0x03 => "KeyF",
        0x05 => "KeyG",
        0x04 => "KeyH",
        0x22 => "KeyI",
        0x26 => "KeyJ",
        0x28 => "KeyK",
        0x25 => "KeyL",
        0x2E => "KeyM",
        0x2D => "KeyN",
        0x1F => "KeyO",
        0x23 => "KeyP",
        0x0C => "KeyQ",
        0x0F => "KeyR",
        0x01 => "KeyS",
        0x11 => "KeyT",
        0x20 => "KeyU",
        0x09 => "KeyV",
        0x0D => "KeyW",
        0x07 => "KeyX",
        0x10 => "KeyY",
        0x06 => "KeyZ",
        0x1D => "Digit0",
        0x12 => "Digit1",
        0x13 => "Digit2",
        0x14 => "Digit3",
        0x15 => "Digit4",
        0x17 => "Digit5",
        0x16 => "Digit6",
        0x1A => "Digit7",
        0x1C => "Digit8",
        0x19 => "Digit9",
        0x31 => "Space",
        0x24 => "Enter",
        0x30 => "Tab",
        0x35 => "Escape",
        0x33 => "Backspace",
        0x7E => "ArrowUp",
        0x7D => "ArrowDown",
        0x7B => "ArrowLeft",
        0x7C => "ArrowRight",
        // Punctuation
        0x2C => "Slash",
        0x2B => "Comma",
        0x2F => "Period",
        0x29 => "Semicolon",
        0x27 => "Quote",
        0x32 => "Backquote",
        0x2A => "Backslash",
        0x1B => "Minus",
        0x18 => "Equal",
        0x21 => "BracketLeft",
        0x1E => "BracketRight",
        // Modifiers (also produce FlagsChanged)
        0x3A => "AltLeft",
        0x3D => "AltRight",
        0x37 => "MetaLeft",
        0x36 => "MetaRight",
        0x3B => "ControlLeft",
        0x3E => "ControlRight",
        0x38 => "ShiftLeft",
        0x3C => "ShiftRight",
        // Function keys
        0x7A => "F1",
        0x78 => "F2",
        0x63 => "F3",
        0x76 => "F4",
        0x60 => "F5",
        0x61 => "F6",
        0x62 => "F7",
        0x64 => "F8",
        0x65 => "F9",
        0x6D => "F10",
        0x67 => "F11",
        0x6F => "F12",
        _ => return None,
    })
}

fn is_modifier_code(code: &str) -> bool {
    matches!(
        code,
        "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
            | "ControlLeft"
            | "ControlRight"
            | "ShiftLeft"
            | "ShiftRight"
    )
}

/// Toggle the given modifier's state and return the new value. macOS fires
/// one FlagsChanged per press or release, so flipping the bit each time is
/// accurate.
fn toggle_modifier(state: &mut ModKeyState, keycode: u16) -> Option<bool> {
    Some(match keycode {
        0x3A => {
            state.l_alt = !state.l_alt;
            state.l_alt
        }
        0x3D => {
            state.r_alt = !state.r_alt;
            state.r_alt
        }
        0x37 => {
            state.l_meta = !state.l_meta;
            state.l_meta
        }
        0x36 => {
            state.r_meta = !state.r_meta;
            state.r_meta
        }
        0x3B => {
            state.l_control = !state.l_control;
            state.l_control
        }
        0x3E => {
            state.r_control = !state.r_control;
            state.r_control
        }
        0x38 => {
            state.l_shift = !state.l_shift;
            state.l_shift
        }
        0x3C => {
            state.r_shift = !state.r_shift;
            state.r_shift
        }
        _ => return None,
    })
}

fn aggregate_modifiers(mods: &ModKeyState) -> ModifierState {
    ModifierState {
        meta: mods.l_meta || mods.r_meta,
        control: mods.l_control || mods.r_control,
        alt: mods.l_alt || mods.r_alt,
        shift: mods.l_shift || mods.r_shift,
    }
}

pub fn start(app: AppHandle, state: AppState) {
    std::thread::spawn(move || {
        println!("Starting CGEventTap keyboard listener…");

        let mod_state = Mutex::new(ModKeyState::default());

        let tap_result = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::FlagsChanged,
            ],
            move |_proxy, event_type, event| {
                let keycode =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;

                let Some(code) = macos_keycode_to_code(keycode) else {
                    return None;
                };

                let is_press = match event_type {
                    CGEventType::KeyDown => Some(true),
                    CGEventType::KeyUp => Some(false),
                    CGEventType::FlagsChanged => {
                        let mut mods = mod_state.lock().unwrap();
                        let pressed = toggle_modifier(&mut mods, keycode)?;
                        // Keep the aggregated modifier state in sync for the
                        // shortcut-matching logic below.
                        *state.modifiers.lock().unwrap() = aggregate_modifiers(&mods);
                        Some(pressed)
                    }
                    _ => return None,
                };
                let Some(is_press) = is_press else { return None };

                let shortcut = state.shortcut.lock().unwrap().clone();
                if code != shortcut.key {
                    return None;
                }

                let modifiers_ok = if is_modifier_code(&shortcut.key) {
                    true
                } else {
                    state.modifiers.lock().unwrap().matches(&shortcut.modifiers)
                };
                if !modifiers_ok {
                    return None;
                }

                let mut active = state.ptt_active.lock().unwrap();
                if is_press && !*active {
                    *active = true;
                    let _ = app.emit("ptt-pressed", ());
                } else if !is_press && *active {
                    *active = false;
                    let _ = app.emit("ptt-released", ());
                }

                None
            },
        );

        let tap = match tap_result {
            Ok(t) => t,
            Err(_) => {
                eprintln!(
                    "Failed to create CGEventTap. Grant Accessibility permission to this binary and relaunch."
                );
                return;
            }
        };

        // Attach the tap to a fresh CFRunLoop on this thread and run it.
        // This is the piece rdev gets wrong on modern macOS — it must happen
        // on the same thread that owns the tap.
        unsafe {
            let Ok(loop_source) = tap.mach_port.create_runloop_source(0) else {
                eprintln!("Failed to create CFRunLoop source for CGEventTap");
                return;
            };
            CFRunLoop::get_current().add_source(&loop_source, kCFRunLoopCommonModes);
            tap.enable();
            println!("CGEventTap enabled; entering run loop");
            CFRunLoop::run_current();
        }
    });
}
