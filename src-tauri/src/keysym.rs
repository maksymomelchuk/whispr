pub const KC_ALT_LEFT: u16 = 0x3A;
pub const KC_ALT_RIGHT: u16 = 0x3D;
pub const KC_META_LEFT: u16 = 0x37;
pub const KC_META_RIGHT: u16 = 0x36;
pub const KC_CONTROL_LEFT: u16 = 0x3B;
pub const KC_CONTROL_RIGHT: u16 = 0x3E;
pub const KC_SHIFT_LEFT: u16 = 0x38;
pub const KC_SHIFT_RIGHT: u16 = 0x3C;

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

pub fn keycode_to_code(kc: u16) -> Option<&'static str> {
    KEYSYM_MAP.iter().find(|(k, _)| *k == kc).map(|(_, c)| *c)
}

#[cfg(test)]
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
