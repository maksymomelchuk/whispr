use crate::config::{Shortcut, TranscriptionProvider};
use crate::deepgram_session::DeepgramSession;
use crate::dictionary::apply_dictionary;
use crate::groq_session::GroqSession;
use crate::history::{self, CleanupStatus, HistoryEntry, HISTORY_UPDATED_EVENT};
use crate::mode::TranslateTarget;
use crate::recorder::Recorder;
use crate::snippets::expand_snippets;
use crate::state::{AppState, ModifierState};
use crate::transcription_session::TranscriptionSession;
use crate::{cleanup, cleanup_stats, config, media, overlay, paste, stats, target_app, translation};
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::os::raw::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

const TRANSCRIPTION_ERROR_EVENT: &str = "transcription-error";
const PTT_PRESSED_EVENT: &str = "ptt-pressed";
const PTT_RELEASED_EVENT: &str = "ptt-released";
const PTT_THINKING_EVENT: &str = "ptt-thinking";
const PTT_ERROR_EVENT: &str = "ptt-error";

const ERROR_FLASH: Duration = Duration::from_millis(800);
const DOUBLE_TAP_THRESHOLD: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TapEvent {
    Down,
    Up,
    OtherKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dispatch {
    StartPtt,
    StopPtt,
    Nothing,
}

#[derive(Default)]
pub struct TapState {
    pub tap_count: u8,
    pub last_tap_up_time: Option<Instant>,
}

pub fn advance_tap_state(state: &mut TapState, event: TapEvent, now: Instant) -> Dispatch {
    match event {
        TapEvent::Down => {
            if state.tap_count == 1 {
                if let Some(t) = state.last_tap_up_time {
                    if now.duration_since(t) < DOUBLE_TAP_THRESHOLD {
                        state.tap_count = 2;
                        return Dispatch::StartPtt;
                    }
                }
            }
            state.tap_count = 1;
            state.last_tap_up_time = None;
            Dispatch::Nothing
        }
        TapEvent::Up => {
            if state.tap_count == 2 {
                state.tap_count = 0;
                state.last_tap_up_time = None;
                return Dispatch::StopPtt;
            }
            if state.tap_count == 1 {
                state.last_tap_up_time = Some(now);
            }
            Dispatch::Nothing
        }
        TapEvent::OtherKey => {
            if state.tap_count == 1 {
                state.tap_count = 0;
                state.last_tap_up_time = None;
            }
            Dispatch::Nothing
        }
    }
}

/// No window focus — caller still owns the target app's focus for paste.
fn notify_silent(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    eprintln!("[notify silent] {message}");
    let _ = app.emit(TRANSCRIPTION_ERROR_EVENT, &message);
    macos_notification(&message);
}

/// Pops main window. Only safe after any pending paste has gone out.
fn notify_error(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    eprintln!("[notify focus] {message}");
    let _ = app.emit(TRANSCRIPTION_ERROR_EVENT, &message);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }

    macos_notification(&message);
}

/// osascript blocks 50–200ms — keep off the runtime worker.
#[cfg(target_os = "macos")]
fn macos_notification(message: &str) {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(r#"display notification "{escaped}" with title "Whispr""#);
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = Command::new("osascript").args(["-e", &script]).output() {
            eprintln!("[notify] osascript failed: {e}");
        }
    });
}
#[cfg(not(target_os = "macos"))]
fn macos_notification(_message: &str) {}

/// `Flash` paints the overlay red briefly; `Focus` raises the main window.
enum Notice {
    None,
    Flash(String),
    Focus(String),
}

// core-graphics keeps CGEventTapEnable private, but we need to call it from
// inside the tap's own callback (the only place we learn the system has
// disabled the tap). Redeclare the symbol — it resolves at link time against
// the CoreGraphics framework the crate already links.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

/// Per-modifier press state. The CGEventFlags bitmask on each event is the
/// authoritative view of modifier-family state (alt / meta / ctrl / shift),
/// but it can't distinguish L vs R — so we track each side explicitly and
/// reconcile with the bitmask on every FlagsChanged to self-heal any drift.
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

/// The CGEventFlags family bit that a given modifier keycode belongs to.
fn modifier_family(keycode: u16) -> Option<CGEventFlags> {
    Some(match keycode {
        0x3A | 0x3D => CGEventFlags::CGEventFlagAlternate,
        0x37 | 0x36 => CGEventFlags::CGEventFlagCommand,
        0x3B | 0x3E => CGEventFlags::CGEventFlagControl,
        0x38 | 0x3C => CGEventFlags::CGEventFlagShift,
        _ => return None,
    })
}

/// Clear both sides of the modifier family a given keycode belongs to.
fn clear_family(state: &mut ModKeyState, keycode: u16) {
    match keycode {
        0x3A | 0x3D => {
            state.l_alt = false;
            state.r_alt = false;
        }
        0x37 | 0x36 => {
            state.l_meta = false;
            state.r_meta = false;
        }
        0x3B | 0x3E => {
            state.l_control = false;
            state.r_control = false;
        }
        0x38 | 0x3C => {
            state.l_shift = false;
            state.r_shift = false;
        }
        _ => {}
    }
}

fn side_mut(state: &mut ModKeyState, keycode: u16) -> Option<&mut bool> {
    Some(match keycode {
        0x3A => &mut state.l_alt,
        0x3D => &mut state.r_alt,
        0x37 => &mut state.l_meta,
        0x36 => &mut state.r_meta,
        0x3B => &mut state.l_control,
        0x3E => &mut state.r_control,
        0x38 => &mut state.l_shift,
        0x3C => &mut state.r_shift,
        _ => return None,
    })
}

fn modifier_state_from_flags(flags: CGEventFlags) -> ModifierState {
    ModifierState {
        meta: flags.contains(CGEventFlags::CGEventFlagCommand),
        control: flags.contains(CGEventFlags::CGEventFlagControl),
        alt: flags.contains(CGEventFlags::CGEventFlagAlternate),
        shift: flags.contains(CGEventFlags::CGEventFlagShift),
    }
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

/// True if `code` is the shortcut's key or one of its required modifiers.
/// Used to decide whether an event is worth processing for PTT.
fn shortcut_is_relevant(code: &str, shortcut: &Shortcut) -> bool {
    if code == shortcut.key {
        return true;
    }
    shortcut.modifiers.iter().any(|m| match m.as_str() {
        "Meta" => matches!(code, "MetaLeft" | "MetaRight"),
        "Control" => matches!(code, "ControlLeft" | "ControlRight"),
        "Alt" => matches!(code, "AltLeft" | "AltRight"),
        "Shift" => matches!(code, "ShiftLeft" | "ShiftRight"),
        _ => false,
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

/// Mute system audio output when the user starts dictating. Runs in
/// spawn_blocking because we shell out to osascript, which can take tens of
/// milliseconds — too long to block the CGEventTap callback.
fn maybe_pause_media(state: &AppState) {
    if !*state.pause_media_on_record.lock().unwrap() {
        *state.did_pause_media.lock().unwrap() = false;
        return;
    }
    *state.did_pause_media.lock().unwrap() = true;
    tauri::async_runtime::spawn_blocking(media::mute_output);
}

/// Mirror of maybe_pause_media. Unmutes only if this session was the one
/// that applied the mute.
fn maybe_resume_media(state: &AppState) {
    let mut flag = state.did_pause_media.lock().unwrap();
    if !*flag {
        return;
    }
    *flag = false;
    tauri::async_runtime::spawn_blocking(media::unmute_output);
}

/// Spawned synchronously on PTT press so the Deepgram WS handshake overlaps
/// with the user's first words. Release closes the chunk channel; this task
/// drains STT, runs optional LLM cleanup, pastes, and only then hides the
/// overlay so it bridges the post-release processing.
fn spawn_session(app: AppHandle, recorder: Recorder, device: Option<String>, mode_id: String) {
    tauri::async_runtime::spawn(async move {
        let result = run_session(&app, recorder, device, &mode_id).await;
        if let Err(e) = result {
            eprintln!("[pipeline] {e}");
            notify_error(&app, e);
        }
        overlay::hide(&app);
    });
}

async fn run_session(
    app: &AppHandle,
    recorder: Recorder,
    device: Option<String>,
    mode_id: &str,
) -> Result<(), String> {
    let (chunk_tx, chunk_rx) = tokio::sync::mpsc::unbounded_channel();
    let (format_tx, format_rx) = tokio::sync::oneshot::channel();
    recorder.start(device, chunk_tx, format_tx);

    let format = match format_rx.await {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => return Err(format!("Recording failed: {e}")),
        Err(_) => return Err("Recording thread crashed".to_string()),
    };

    let settings = config::load(app);
    let session_result = match settings.transcription_provider {
        TranscriptionProvider::Deepgram => {
            DeepgramSession.run(app.clone(), format, chunk_rx).await
        }
        TranscriptionProvider::Groq => GroqSession.run(app.clone(), format, chunk_rx).await,
    };

    let (raw_text, speak_duration) = match session_result {
        Ok(r) => r,
        Err(e) => {
            // Stop the recorder if it's still running so an error doesn't
            // leak a live cpal stream.
            recorder.stop();
            return Err(e);
        }
    };
    if raw_text.is_empty() {
        return Ok(());
    }

    let active_mode = settings
        .modes
        .iter()
        .find(|m| m.id == mode_id)
        .unwrap_or_else(|| config::get_default_mode(&settings));
    let mode_cleanup_enabled = active_mode.ai_cleanup.enabled;
    let mode_use_snippets = active_mode.use_snippets;
    let mode_use_dictionary = active_mode.use_dictionary;
    let mode_translate = active_mode.translate.clone();
    let mode_source_lang = active_mode.language.as_code().map(str::to_string);
    let mode_prompt_override = active_mode.ai_cleanup.prompt_override.clone();

    let (translated_text, translate_notice) =
        maybe_translate(app, &raw_text, &mode_translate, mode_source_lang.as_deref()).await;

    let (replaced_text, cleanup_status, cleanup_notice) = maybe_cleanup(
        app,
        &settings,
        mode_cleanup_enabled,
        &translated_text,
        speak_duration,
        mode_prompt_override.as_deref(),
    )
    .await;

    let notice = merge_notices(translate_notice, cleanup_notice);

    let mut final_text = replaced_text.clone();
    if mode_use_snippets {
        final_text = expand_snippets(&final_text, &settings.snippets);
    }
    if mode_use_dictionary {
        final_text = apply_dictionary(&final_text, &settings.dictionary);
    }

    let words = final_text.split_whitespace().count() as u64;
    let seconds = speak_duration.as_secs() as u32;

    let entry = HistoryEntry {
        timestamp: history::now_unix_seconds(),
        speak_duration_ms: speak_duration.as_millis() as u64,
        raw_text,
        replaced_text,
        final_text: final_text.clone(),
        cleanup_status,
    };

    // paste_handle must complete before any notify_error: set_focus()
    // during the modifier-release wait would steal focus mid-paste.
    let paste_handle = paste::paste_text(format!("{final_text} "));

    stats::record(app, words, seconds);

    match history::append(app, entry) {
        Ok(_) => {
            let _ = app.emit(HISTORY_UPDATED_EVENT, ());
        }
        Err(e) => eprintln!("[pipeline] history append failed: {e}"),
    }

    if let Err(e) = paste_handle.await {
        eprintln!("[pipeline] paste worker failed: {e}");
        notify_error(app, format!("Paste failed: {e}"));
    }

    match notice {
        Notice::None => {}
        Notice::Flash(message) => {
            notify_silent(app, message);
            tokio::time::sleep(ERROR_FLASH).await;
        }
        Notice::Focus(message) => notify_error(app, message),
    }

    Ok(())
}

fn merge_notices(a: Notice, b: Notice) -> Notice {
    match (a, b) {
        (Notice::Focus(m), _) | (_, Notice::Focus(m)) => Notice::Focus(m),
        (Notice::Flash(m), _) | (_, Notice::Flash(m)) => Notice::Flash(m),
        _ => Notice::None,
    }
}

async fn maybe_translate(
    app: &AppHandle,
    text: &str,
    target: &TranslateTarget,
    source_lang: Option<&str>,
) -> (String, Notice) {
    let TranslateTarget::Apple { target: target_lang } = target else {
        return (text.to_string(), Notice::None);
    };

    let text_owned = text.to_string();
    let source_owned = source_lang.map(str::to_string);
    let target_owned = target_lang.clone();

    let result = tokio::task::spawn_blocking(move || {
        translation::translate(&text_owned, source_owned.as_deref(), &target_owned)
    })
    .await;

    match result {
        Ok(Ok(translated)) => (translated, Notice::None),
        Ok(Err(e)) => {
            let _ = app.emit(PTT_ERROR_EVENT, ());
            (text.to_string(), Notice::Flash(format!("Translation unavailable: {e}")))
        }
        Err(_) => {
            let _ = app.emit(PTT_ERROR_EVENT, ());
            (text.to_string(), Notice::Flash("Translation thread panicked".to_string()))
        }
    }
}

async fn maybe_cleanup(
    app: &AppHandle,
    settings: &config::Settings,
    mode_cleanup_enabled: bool,
    transcript: &str,
    speak_duration: Duration,
    prompt_override: Option<&str>,
) -> (String, CleanupStatus, Notice) {
    let cleanup_settings = &settings.ai_cleanup;

    if !mode_cleanup_enabled {
        return (transcript.to_string(), CleanupStatus::Disabled, Notice::None);
    }

    let credential = match cleanup_settings.auth_mode {
        config::CleanupAuthMode::ApiKey => match cleanup_settings.anthropic_api_key.as_deref() {
            Some(k) if !k.is_empty() => cleanup::Credential::ApiKey(k),
            _ => {
                return (
                    transcript.to_string(),
                    CleanupStatus::NoCredential,
                    Notice::Focus(
                        "AI cleanup is enabled but Anthropic API key is not set.".to_string(),
                    ),
                );
            }
        },
        config::CleanupAuthMode::Oauth => {
            match cleanup_settings.anthropic_oauth_token.as_deref() {
                Some(t) if !t.is_empty() => cleanup::Credential::OauthToken(t),
                _ => {
                    return (
                        transcript.to_string(),
                        CleanupStatus::NoCredential,
                        Notice::Focus(
                            "AI cleanup is set to OAuth but no Claude Code token is configured."
                                .to_string(),
                        ),
                    );
                }
            }
        }
    };

    let words = transcript.split_whitespace().count();
    if words < cleanup_settings.min_words {
        return (
            transcript.to_string(),
            CleanupStatus::SkippedBelowMinWords,
            Notice::None,
        );
    }
    let min_duration = Duration::from_millis(cleanup_settings.min_duration_ms);
    if speak_duration < min_duration {
        return (
            transcript.to_string(),
            CleanupStatus::SkippedBelowMinDuration,
            Notice::None,
        );
    }

    let _ = app.emit(PTT_THINKING_EVENT, ());

    let prompt = cleanup::effective_prompt(prompt_override);
    match cleanup::run(transcript, credential, prompt).await {
        Ok((cleaned, usage)) => {
            cleanup_stats::record(app, usage.input_tokens, usage.output_tokens);
            (cleaned, CleanupStatus::Ran, Notice::None)
        }
        Err(err) => {
            let message = format!("AI cleanup unavailable: {err}");
            let (status, notice) = match err {
                cleanup::CleanupError::Credential(m) => {
                    (CleanupStatus::FailedCredential(m), Notice::Focus(message))
                }
                cleanup::CleanupError::Timeout => {
                    let _ = app.emit(PTT_ERROR_EVENT, ());
                    (CleanupStatus::FailedTimeout, Notice::Flash(message))
                }
                cleanup::CleanupError::Transient(m) => {
                    let _ = app.emit(PTT_ERROR_EVENT, ());
                    (CleanupStatus::FailedTransient(m), Notice::Flash(message))
                }
            };
            (transcript.to_string(), status, notice)
        }
    }
}

pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    std::thread::spawn(move || {

        let mod_state = Mutex::new(ModKeyState::default());
        // Shared handle to the tap's mach port so the callback can re-enable
        // itself. Populated after CGEventTap::new returns.
        let tap_port: Arc<AtomicPtr<c_void>> = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let tap_port_cb = tap_port.clone();
        // Per double-tap binding state, keyed by (key, modifiers).
        // Single-press bindings never touch this map.
        let mut tap_states: HashMap<(String, Vec<String>), TapState> = HashMap::new();

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
                // macOS disables the tap out-of-band when the callback runs
                // too slowly OR when secure input (password fields, etc.)
                // takes over. Re-enable immediately and drop the event —
                // these don't carry a keycode so the parsing below would
                // misinterpret them.
                if matches!(
                    event_type,
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
                ) {
                    eprintln!("CGEventTap disabled ({event_type:?}); re-enabling");
                    let port = tap_port_cb.load(Ordering::Acquire);
                    if !port.is_null() {
                        unsafe { CGEventTapEnable(port, true) };
                    }
                    return None;
                }

                let keycode =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;

                let Some(code) = macos_keycode_to_code(keycode) else {
                    return None;
                };

                let is_press = match event_type {
                    CGEventType::KeyDown => true,
                    CGEventType::KeyUp => false,
                    CGEventType::FlagsChanged => {
                        let flags = event.get_flags();
                        // Aggregate modifier state comes straight from the
                        // bitmask — no drift possible, always matches the OS.
                        *state.modifiers.lock().unwrap() = modifier_state_from_flags(flags);

                        let family = modifier_family(keycode)?;
                        let family_on = flags.contains(family);
                        let mut mods = mod_state.lock().unwrap();

                        if !family_on {
                            // Family fully released: both L and R of this
                            // family must now be up. Force them so any drift
                            // (e.g. app started with a modifier already held,
                            // or another tap swallowed an event) self-heals.
                            clear_family(&mut mods, keycode);
                            false
                        } else {
                            // Family is still on after this event, so this
                            // specific side toggled. Flip our tracked bit.
                            let side = side_mut(&mut mods, keycode)?;
                            *side = !*side;
                            *side
                        }
                    }
                    _ => return None,
                };

                if *state.shortcut_capture_paused.lock().unwrap() {
                    return None;
                }

                let now = Instant::now();
                let ptt_active_now = *state.ptt_active.lock().unwrap();
                let bindings = state.hotkey_bindings.lock().unwrap().clone();
                let modifiers_val = *state.modifiers.lock().unwrap();

                // Reset pending double-tap state for any binding whose key
                // this event doesn't match — "other key" cancels the gesture.
                if is_press && !ptt_active_now {
                    for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                        let key_matches = code == b.shortcut.key
                            && (is_modifier_code(&b.shortcut.key)
                                || modifiers_val.matches(&b.shortcut.modifiers));
                        if !key_matches {
                            let tap_key =
                                (b.shortcut.key.clone(), b.shortcut.modifiers.clone());
                            if let Some(ts) = tap_states.get_mut(&tap_key) {
                                advance_tap_state(ts, TapEvent::OtherKey, now);
                            }
                        }
                    }
                }

                // For release events, check only against the shortcut that
                // started the active session so unrelated bindings don't
                // cause spurious releases.
                let relevant = if ptt_active_now {
                    state
                        .active_shortcut
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|sc| shortcut_is_relevant(code, sc))
                        .unwrap_or(false)
                } else {
                    bindings.iter().any(|b| shortcut_is_relevant(code, &b.shortcut))
                };
                if !relevant {
                    return None;
                }

                let mut active = state.ptt_active.lock().unwrap();

                if is_press && !*active {
                    // Advance double-tap state machines; fire if second tap within window.
                    let mut dt_start_idx: Option<usize> = None;
                    for (i, b) in bindings.iter().enumerate() {
                        if !b.shortcut.is_double_tap {
                            continue;
                        }
                        if code == b.shortcut.key
                            && (is_modifier_code(&b.shortcut.key)
                                || modifiers_val.matches(&b.shortcut.modifiers))
                        {
                            let tap_key =
                                (b.shortcut.key.clone(), b.shortcut.modifiers.clone());
                            let ts = tap_states.entry(tap_key).or_default();
                            if advance_tap_state(ts, TapEvent::Down, now) == Dispatch::StartPtt {
                                dt_start_idx = Some(i);
                                break;
                            }
                        }
                    }

                    let matched = if let Some(i) = dt_start_idx {
                        Some(&bindings[i])
                    } else {
                        bindings.iter().find(|b| {
                            !b.shortcut.is_double_tap
                                && code == b.shortcut.key
                                && (is_modifier_code(&b.shortcut.key)
                                    || modifiers_val.matches(&b.shortcut.modifiers))
                        })
                    };

                    if let Some(binding) = matched {
                        *active = true;
                        *state.active_shortcut.lock().unwrap() = Some(binding.shortcut.clone());
                        let mode_id = binding.mode_id.clone();
                        let device = state.input_device.lock().unwrap().clone();
                        // Capture frontmost app before overlay::show so the
                        // worker's frontmostApplication() lookup can't race
                        // any window-server bookkeeping that show() triggers.
                        target_app::capture(app.clone());
                        // Spawn up front so the WS handshake overlaps with
                        // capture rather than waiting for PTT release.
                        spawn_session(app.clone(), recorder.clone(), device, mode_id);
                        overlay::show(&app);
                        let _ = app.emit(PTT_PRESSED_EVENT, ());
                        maybe_pause_media(&state);
                    }
                } else if !is_press && *active {
                    let sc_opt = state.active_shortcut.lock().unwrap().clone();
                    let should_stop = match sc_opt {
                        Some(ref sc) if sc.is_double_tap => {
                            let tap_key = (sc.key.clone(), sc.modifiers.clone());
                            let ts = tap_states.entry(tap_key).or_default();
                            advance_tap_state(ts, TapEvent::Up, now) == Dispatch::StopPtt
                        }
                        _ => true,
                    };
                    if should_stop {
                        *active = false;
                        *state.active_shortcut.lock().unwrap() = None;
                        // Overlay stays visible — spawn_session hides it after
                        // paste so the "still processing" state bridges STT
                        // drain and any LLM cleanup pass.
                        let _ = app.emit(PTT_RELEASED_EVENT, ());
                        maybe_resume_media(&state);
                        recorder.stop();
                    }
                } else if !is_press && !*active {
                    // Key-up while idle: record first-tap-up time for double-tap bindings.
                    for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                        if code == b.shortcut.key
                            && (is_modifier_code(&b.shortcut.key)
                                || modifiers_val.matches(&b.shortcut.modifiers))
                        {
                            let tap_key =
                                (b.shortcut.key.clone(), b.shortcut.modifiers.clone());
                            if let Some(ts) = tap_states.get_mut(&tap_key) {
                                if ts.tap_count > 0 {
                                    advance_tap_state(ts, TapEvent::Up, now);
                                }
                            }
                        }
                    }
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

        // Publish the mach-port pointer so the callback can re-enable the
        // tap after a system-initiated disable. Safe: the tap (and therefore
        // the mach port) outlives the callback — both live until the
        // runloop exits, which only happens when this thread tears down.
        tap_port.store(
            tap.mach_port.as_concrete_TypeRef() as *mut c_void,
            Ordering::Release,
        );

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
            CFRunLoop::run_current();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn double_tap_within_window_starts_on_second_down_stops_on_second_up() {
        let base = t0();
        let mut state = TapState::default();

        // First down: tap_count → 1, no start
        assert_eq!(advance_tap_state(&mut state, TapEvent::Down, base), Dispatch::Nothing);
        assert_eq!(state.tap_count, 1);

        // First up: record timestamp
        let t1 = base + Duration::from_millis(50);
        assert_eq!(advance_tap_state(&mut state, TapEvent::Up, t1), Dispatch::Nothing);
        assert!(state.last_tap_up_time.is_some());

        // Second down within 400ms: start PTT
        let t2 = base + Duration::from_millis(150);
        assert_eq!(advance_tap_state(&mut state, TapEvent::Down, t2), Dispatch::StartPtt);
        assert_eq!(state.tap_count, 2);

        // Second up: stop PTT, reset
        let t3 = base + Duration::from_millis(300);
        assert_eq!(advance_tap_state(&mut state, TapEvent::Up, t3), Dispatch::StopPtt);
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());
    }

    #[test]
    fn double_tap_expired_window_second_down_treated_as_new_first_tap() {
        let base = t0();
        let mut state = TapState::default();

        assert_eq!(advance_tap_state(&mut state, TapEvent::Down, base), Dispatch::Nothing);
        let t1 = base + Duration::from_millis(50);
        advance_tap_state(&mut state, TapEvent::Up, t1);

        // Second down after threshold: resets to tap_count=1, no start
        let t2 = base + Duration::from_millis(500);
        assert_eq!(advance_tap_state(&mut state, TapEvent::Down, t2), Dispatch::Nothing);
        assert_eq!(state.tap_count, 1);
        assert!(state.last_tap_up_time.is_none());
    }

    #[test]
    fn other_key_between_taps_resets_state() {
        let base = t0();
        let mut state = TapState::default();

        advance_tap_state(&mut state, TapEvent::Down, base);
        let t1 = base + Duration::from_millis(50);
        advance_tap_state(&mut state, TapEvent::Up, t1);
        assert_eq!(state.tap_count, 1);

        // Unrelated key press cancels the pending double-tap
        let t2 = base + Duration::from_millis(100);
        assert_eq!(advance_tap_state(&mut state, TapEvent::OtherKey, t2), Dispatch::Nothing);
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());

        // Subsequent down within what would have been the window: no start
        let t3 = base + Duration::from_millis(150);
        assert_eq!(advance_tap_state(&mut state, TapEvent::Down, t3), Dispatch::Nothing);
        assert_eq!(state.tap_count, 1);
    }

    #[test]
    fn fresh_state_up_event_is_noop() {
        let mut state = TapState::default();
        assert_eq!(advance_tap_state(&mut state, TapEvent::Up, t0()), Dispatch::Nothing);
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());
    }
}
