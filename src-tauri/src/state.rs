use crate::config::{HotkeyBinding, Shortcut};
use crate::provider::LocalWhisperModel;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug, Clone, Copy)]
pub struct ModifierState {
    pub meta: bool,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

impl ModifierState {
    /// True when the set of held modifiers matches the required set exactly.
    pub fn matches(&self, required: &[String]) -> bool {
        let meta_req = required.iter().any(|m| m == "Meta");
        let control_req = required.iter().any(|m| m == "Control");
        let alt_req = required.iter().any(|m| m == "Alt");
        let shift_req = required.iter().any(|m| m == "Shift");
        self.meta == meta_req
            && self.control == control_req
            && self.alt == alt_req
            && self.shift == shift_req
    }
}

/// Tauri-managed state. All fields are Arcs so cloning is cheap and the
/// CGEventTap listener thread and the command handlers share the same data.
#[derive(Clone, Default)]
pub struct AppState {
    pub hotkey_bindings: Arc<Mutex<Vec<HotkeyBinding>>>,
    /// The shortcut that activated the current PTT session. Cleared when PTT ends.
    /// Kept separate so release detection uses only the active binding's keys.
    pub active_shortcut: Arc<Mutex<Option<Shortcut>>>,
    pub modifiers: Arc<Mutex<ModifierState>>,
    pub ptt_active: Arc<Mutex<bool>>,
    pub input_device: Arc<Mutex<Option<String>>>,
    /// User preference: pause the Now Playing app while dictating.
    pub pause_media_on_record: Arc<Mutex<bool>>,
    /// Transient: did we actually pause media at the start of the current
    /// recording? If so, release should resume; otherwise leave it alone.
    pub did_pause_media: Arc<Mutex<bool>>,
    /// When true, the CGEventTap skips PTT matching so the settings UI can
    /// capture keystrokes for shortcut rebinding without firing dictation.
    pub shortcut_capture_paused: Arc<Mutex<bool>>,
    pub download_cancel_flags: Arc<Mutex<HashMap<LocalWhisperModel, Arc<AtomicBool>>>>,
}
