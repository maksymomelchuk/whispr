use crate::config::{HotkeyBinding, Shortcut};
use crate::provider::LocalWhisperModel;
use crate::recorder::Recorder;
#[cfg(target_os = "macos")]
use crate::target_app::FrontmostApp;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;
#[cfg(target_os = "macos")]
use tokio::sync::oneshot;

#[derive(Default, Debug, Clone, Copy)]
pub struct ModifierState {
    pub meta: bool,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

impl ModifierState {
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

pub enum LocalEngine {
    Whisper(transcribe_rs::whisper_cpp::WhisperEngine),
    Parakeet(transcribe_rs::onnx::parakeet::ParakeetModel),
}

pub struct LoadedModel {
    pub engine: LocalEngine,
    pub last_used: Instant,
}

/// All fields are Arcs so cloning is cheap and the
/// CGEventTap listener thread and the command handlers share the same data.
#[derive(Clone, Default)]
pub struct AppState {
    pub hotkey_bindings: Arc<Mutex<Vec<HotkeyBinding>>>,
    /// The shortcut that activated the current PTT session. Cleared when PTT ends.
    /// Kept separate so release detection uses only the active binding's keys.
    pub active_shortcut: Arc<Mutex<Option<Shortcut>>>,
    pub modifiers: Arc<Mutex<ModifierState>>,
    pub ptt_active: Arc<Mutex<bool>>,
    /// Set by the cancel orchestrator when the user presses Escape mid-Session.
    /// Checked by run_session after STT drains to short-circuit cleanup /
    /// paste / history / stats. Cleared at start_ptt.
    pub session_cancelled: Arc<AtomicBool>,
    pub input_device: Arc<Mutex<Option<String>>>,
    pub pause_media_on_record: Arc<Mutex<bool>>,
    /// Transient: did we actually pause media at the start of the current
    /// recording? If so, release should resume; otherwise leave it alone.
    pub did_pause_media: Arc<Mutex<bool>>,
    /// When true, the CGEventTap skips PTT matching so the settings UI can
    /// capture keystrokes for shortcut rebinding without firing dictation.
    pub shortcut_capture_paused: Arc<Mutex<bool>>,
    pub download_cancel_flags: Arc<Mutex<HashMap<LocalWhisperModel, Arc<AtomicBool>>>>,
    /// Receives the frontmost app captured at PTT press so the session task
    /// can record it in the history entry without a second osascript call.
    #[cfg(target_os = "macos")]
    pub pending_app_rx: Arc<Mutex<Option<oneshot::Receiver<Option<FrontmostApp>>>>>,
    /// True while the CGEventTap thread is alive. Used to detect when
    /// Accessibility permission was granted after startup so we can restart
    /// the tap without a full app relaunch.
    pub ptt_running: Arc<AtomicBool>,
    pub recorder: Arc<Mutex<Option<Recorder>>>,
    pub model_cache: Arc<Mutex<HashMap<LocalWhisperModel, LoadedModel>>>,
}
