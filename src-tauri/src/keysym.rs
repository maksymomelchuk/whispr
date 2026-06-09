#[cfg(target_os = "macos")]
pub const KC_ALT_LEFT: u16 = 0x3A;
#[cfg(target_os = "macos")]
pub const KC_ALT_RIGHT: u16 = 0x3D;
#[cfg(target_os = "macos")]
pub const KC_META_LEFT: u16 = 0x37;
#[cfg(target_os = "macos")]
pub const KC_META_RIGHT: u16 = 0x36;
#[cfg(target_os = "macos")]
pub const KC_CONTROL_LEFT: u16 = 0x3B;
#[cfg(target_os = "macos")]
pub const KC_CONTROL_RIGHT: u16 = 0x3E;
#[cfg(target_os = "macos")]
pub const KC_SHIFT_LEFT: u16 = 0x38;
#[cfg(target_os = "macos")]
pub const KC_SHIFT_RIGHT: u16 = 0x3C;

#[cfg(target_os = "macos")]
static KEYSYM_MAP: &[(u16, &str)] = &[
    (0x00, "KeyA"),
    (0x0B, "KeyB"),
    (0x08, "KeyC"),
    (0x02, "KeyD"),
    (0x0E, "KeyE"),
    (0x03, "KeyF"),
    (0x05, "KeyG"),
    (0x04, "KeyH"),
    (0x22, "KeyI"),
    (0x26, "KeyJ"),
    (0x28, "KeyK"),
    (0x25, "KeyL"),
    (0x2E, "KeyM"),
    (0x2D, "KeyN"),
    (0x1F, "KeyO"),
    (0x23, "KeyP"),
    (0x0C, "KeyQ"),
    (0x0F, "KeyR"),
    (0x01, "KeyS"),
    (0x11, "KeyT"),
    (0x20, "KeyU"),
    (0x09, "KeyV"),
    (0x0D, "KeyW"),
    (0x07, "KeyX"),
    (0x10, "KeyY"),
    (0x06, "KeyZ"),
    (0x1D, "Digit0"),
    (0x12, "Digit1"),
    (0x13, "Digit2"),
    (0x14, "Digit3"),
    (0x15, "Digit4"),
    (0x17, "Digit5"),
    (0x16, "Digit6"),
    (0x1A, "Digit7"),
    (0x1C, "Digit8"),
    (0x19, "Digit9"),
    (0x31, "Space"),
    (0x24, "Enter"),
    (0x30, "Tab"),
    (0x35, "Escape"),
    (0x33, "Backspace"),
    (0x7E, "ArrowUp"),
    (0x7D, "ArrowDown"),
    (0x7B, "ArrowLeft"),
    (0x7C, "ArrowRight"),
    (0x2C, "Slash"),
    (0x2B, "Comma"),
    (0x2F, "Period"),
    (0x29, "Semicolon"),
    (0x27, "Quote"),
    (0x32, "Backquote"),
    (0x2A, "Backslash"),
    (0x1B, "Minus"),
    (0x18, "Equal"),
    (0x21, "BracketLeft"),
    (0x1E, "BracketRight"),
    (KC_ALT_LEFT, "AltLeft"),
    (KC_ALT_RIGHT, "AltRight"),
    (KC_META_LEFT, "MetaLeft"),
    (KC_META_RIGHT, "MetaRight"),
    (KC_CONTROL_LEFT, "ControlLeft"),
    (KC_CONTROL_RIGHT, "ControlRight"),
    (KC_SHIFT_LEFT, "ShiftLeft"),
    (KC_SHIFT_RIGHT, "ShiftRight"),
    (0x7A, "F1"),
    (0x78, "F2"),
    (0x63, "F3"),
    (0x76, "F4"),
    (0x60, "F5"),
    (0x61, "F6"),
    (0x62, "F7"),
    (0x64, "F8"),
    (0x65, "F9"),
    (0x6D, "F10"),
    (0x67, "F11"),
    (0x6F, "F12"),
];

#[cfg(target_os = "macos")]
pub fn keycode_to_code(kc: u16) -> Option<&'static str> {
    KEYSYM_MAP.iter().find(|(k, _)| *k == kc).map(|(_, c)| *c)
}

/// rdev's Key::Alt is the left Alt key on most platforms; Key::AltGr is the
/// right Alt / AltGr key. Neither variant distinguishes hardware side the way
/// macOS keycodes do, so the mapping is best-effort on keyboards with two
/// symmetric Alt keys where the right one is not labelled AltGr.
#[cfg(target_os = "linux")]
pub fn rdev_key_to_code(key: &rdev::Key) -> Option<&'static str> {
    use rdev::Key;
    Some(match key {
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::Delete => "Delete",
        Key::DownArrow => "ArrowDown",
        Key::End => "End",
        Key::Escape => "Escape",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Home => "Home",
        Key::LeftArrow => "ArrowLeft",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Return => "Enter",
        Key::RightArrow => "ArrowRight",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "ArrowUp",
        Key::PrintScreen => "PrintScreen",
        Key::ScrollLock => "ScrollLock",
        Key::Pause => "Pause",
        Key::NumLock => "NumLock",
        Key::BackQuote => "Backquote",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::Num0 => "Digit0",
        Key::Minus => "Minus",
        Key::Equal => "Equal",
        Key::KeyA => "KeyA",
        Key::KeyB => "KeyB",
        Key::KeyC => "KeyC",
        Key::KeyD => "KeyD",
        Key::KeyE => "KeyE",
        Key::KeyF => "KeyF",
        Key::KeyG => "KeyG",
        Key::KeyH => "KeyH",
        Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ",
        Key::KeyK => "KeyK",
        Key::KeyL => "KeyL",
        Key::KeyM => "KeyM",
        Key::KeyN => "KeyN",
        Key::KeyO => "KeyO",
        Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ",
        Key::KeyR => "KeyR",
        Key::KeyS => "KeyS",
        Key::KeyT => "KeyT",
        Key::KeyU => "KeyU",
        Key::KeyV => "KeyV",
        Key::KeyW => "KeyW",
        Key::KeyX => "KeyX",
        Key::KeyY => "KeyY",
        Key::KeyZ => "KeyZ",
        Key::LeftBracket => "BracketLeft",
        Key::RightBracket => "BracketRight",
        Key::SemiColon => "Semicolon",
        Key::Quote => "Quote",
        Key::BackSlash => "Backslash",
        Key::IntlBackslash => "IntlBackslash",
        Key::Comma => "Comma",
        Key::Dot => "Period",
        Key::Slash => "Slash",
        Key::Insert => "Insert",
        Key::KpReturn => "NumpadEnter",
        Key::KpMinus => "NumpadSubtract",
        Key::KpPlus => "NumpadAdd",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",
        Key::KpDelete => "NumpadDecimal",
        Key::Function => "Fn",
        Key::Unknown(_) => return None,
    })
}

/// Maps Windows virtual-key codes to the web `KeyboardEvent.code` strings that
/// bindings are stored as. A low-level hook reports distinct VKs for the left
/// and right modifiers, so they resolve unambiguously (unlike rdev).
#[cfg(target_os = "windows")]
pub fn vk_to_code(vk: u32) -> Option<&'static str> {
    Some(match vk {
        0x41 => "KeyA",
        0x42 => "KeyB",
        0x43 => "KeyC",
        0x44 => "KeyD",
        0x45 => "KeyE",
        0x46 => "KeyF",
        0x47 => "KeyG",
        0x48 => "KeyH",
        0x49 => "KeyI",
        0x4A => "KeyJ",
        0x4B => "KeyK",
        0x4C => "KeyL",
        0x4D => "KeyM",
        0x4E => "KeyN",
        0x4F => "KeyO",
        0x50 => "KeyP",
        0x51 => "KeyQ",
        0x52 => "KeyR",
        0x53 => "KeyS",
        0x54 => "KeyT",
        0x55 => "KeyU",
        0x56 => "KeyV",
        0x57 => "KeyW",
        0x58 => "KeyX",
        0x59 => "KeyY",
        0x5A => "KeyZ",
        0x30 => "Digit0",
        0x31 => "Digit1",
        0x32 => "Digit2",
        0x33 => "Digit3",
        0x34 => "Digit4",
        0x35 => "Digit5",
        0x36 => "Digit6",
        0x37 => "Digit7",
        0x38 => "Digit8",
        0x39 => "Digit9",
        0x70 => "F1",
        0x71 => "F2",
        0x72 => "F3",
        0x73 => "F4",
        0x74 => "F5",
        0x75 => "F6",
        0x76 => "F7",
        0x77 => "F8",
        0x78 => "F9",
        0x79 => "F10",
        0x7A => "F11",
        0x7B => "F12",
        0x20 => "Space",
        0x0D => "Enter",
        0x09 => "Tab",
        0x1B => "Escape",
        0x08 => "Backspace",
        0x25 => "ArrowLeft",
        0x26 => "ArrowUp",
        0x27 => "ArrowRight",
        0x28 => "ArrowDown",
        0xA0 | 0x10 => "ShiftLeft",
        0xA1 => "ShiftRight",
        0xA2 | 0x11 => "ControlLeft",
        0xA3 => "ControlRight",
        0xA4 | 0x12 => "AltLeft",
        0xA5 => "AltRight",
        0x5B => "MetaLeft",
        0x5C => "MetaRight",
        0xC0 => "Backquote",
        0xBD => "Minus",
        0xBB => "Equal",
        0xDB => "BracketLeft",
        0xDD => "BracketRight",
        0xDC => "Backslash",
        0xBA => "Semicolon",
        0xDE => "Quote",
        0xBC => "Comma",
        0xBE => "Period",
        0xBF => "Slash",
        _ => return None,
    })
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn known_modifier_keycodes_map_to_correct_codes() {
        assert_eq!(keycode_to_code(KC_ALT_LEFT), Some("AltLeft"));
        assert_eq!(keycode_to_code(KC_ALT_RIGHT), Some("AltRight"));
        assert_eq!(keycode_to_code(KC_META_LEFT), Some("MetaLeft"));
        assert_eq!(keycode_to_code(KC_META_RIGHT), Some("MetaRight"));
        assert_eq!(keycode_to_code(KC_CONTROL_LEFT), Some("ControlLeft"));
        assert_eq!(keycode_to_code(KC_CONTROL_RIGHT), Some("ControlRight"));
        assert_eq!(keycode_to_code(KC_SHIFT_LEFT), Some("ShiftLeft"));
        assert_eq!(keycode_to_code(KC_SHIFT_RIGHT), Some("ShiftRight"));
    }

    #[test]
    fn known_letter_keycodes_map_to_correct_codes() {
        assert_eq!(keycode_to_code(0x00), Some("KeyA"));
        assert_eq!(keycode_to_code(0x01), Some("KeyS"));
        assert_eq!(keycode_to_code(0x35), Some("Escape"));
        assert_eq!(keycode_to_code(0x31), Some("Space"));
    }

    #[test]
    fn unknown_keycode_returns_none() {
        assert_eq!(keycode_to_code(0xFF), None);
    }

    #[test]
    fn all_keymap_entries_resolve() {
        for &(kc, code) in KEYSYM_MAP {
            assert_eq!(
                keycode_to_code(kc),
                Some(code),
                "keycode {kc:#04x} should map to {code}"
            );
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod rdev_tests {
    use super::*;
    use rdev::Key;

    #[test]
    fn escape_maps_to_escape() {
        assert_eq!(rdev_key_to_code(&Key::Escape), Some("Escape"));
    }

    #[test]
    fn modifier_keys_map_to_standard_codes() {
        assert_eq!(rdev_key_to_code(&Key::Alt), Some("AltLeft"));
        assert_eq!(rdev_key_to_code(&Key::AltGr), Some("AltRight"));
        assert_eq!(rdev_key_to_code(&Key::ShiftLeft), Some("ShiftLeft"));
        assert_eq!(rdev_key_to_code(&Key::ShiftRight), Some("ShiftRight"));
        assert_eq!(rdev_key_to_code(&Key::ControlLeft), Some("ControlLeft"));
        assert_eq!(rdev_key_to_code(&Key::ControlRight), Some("ControlRight"));
        assert_eq!(rdev_key_to_code(&Key::MetaLeft), Some("MetaLeft"));
        assert_eq!(rdev_key_to_code(&Key::MetaRight), Some("MetaRight"));
    }

    #[test]
    fn letter_keys_map_to_web_key_codes() {
        assert_eq!(rdev_key_to_code(&Key::KeyA), Some("KeyA"));
        assert_eq!(rdev_key_to_code(&Key::KeyZ), Some("KeyZ"));
    }

    #[test]
    fn top_row_numbers_map_to_digit_codes() {
        assert_eq!(rdev_key_to_code(&Key::Num1), Some("Digit1"));
        assert_eq!(rdev_key_to_code(&Key::Num0), Some("Digit0"));
    }

    #[test]
    fn arrow_keys_use_arrow_prefix() {
        assert_eq!(rdev_key_to_code(&Key::UpArrow), Some("ArrowUp"));
        assert_eq!(rdev_key_to_code(&Key::DownArrow), Some("ArrowDown"));
        assert_eq!(rdev_key_to_code(&Key::LeftArrow), Some("ArrowLeft"));
        assert_eq!(rdev_key_to_code(&Key::RightArrow), Some("ArrowRight"));
    }

    #[test]
    fn unknown_key_returns_none() {
        assert_eq!(rdev_key_to_code(&Key::Unknown(0x1234)), None);
    }
}
