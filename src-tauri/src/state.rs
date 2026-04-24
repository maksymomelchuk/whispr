use crate::config::Shortcut;
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
    pub shortcut: Arc<Mutex<Shortcut>>,
    pub modifiers: Arc<Mutex<ModifierState>>,
    pub ptt_active: Arc<Mutex<bool>>,
    pub input_device: Arc<Mutex<Option<String>>>,
}
