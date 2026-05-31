use crate::config::{HotkeyAction, HotkeyBinding, Shortcut};
use crate::hotkey::{
    advance_tap_state, coex_advance_down, coex_timer_should_fire, is_cancel_event,
    key_has_both_kinds, shortcut_is_relevant, shortcut_matches, tap_state_key, CoexDown, Dispatch,
    TapEvent, TapState, DOUBLE_TAP_THRESHOLD,
};
use crate::provider::{self, LocalWhisperModel, ProviderModel, TranscriptionProvider};
use crate::assemblyai_session::AssemblyAiSession;
use crate::deepgram_session::DeepgramSession;
use crate::groq_session::GroqSession;
use crate::history::{self, CleanupStatus, HISTORY_UPDATED_EVENT};
use crate::pipeline::{self, CleanupOutput, Notice};
use crate::recorder::Recorder;
use crate::state::{AppState, ModifierState};
use crate::transcription_session::TranscriptionSession;
use crate::{cleanup, cleanup_stats, config, stats};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use crate::keysym::{
    keycode_to_code, KC_ALT_LEFT, KC_ALT_RIGHT, KC_CONTROL_LEFT, KC_CONTROL_RIGHT, KC_META_LEFT,
    KC_META_RIGHT, KC_SHIFT_LEFT, KC_SHIFT_RIGHT,
};
#[cfg(target_os = "macos")]
use crate::local_session::LocalSession;
use crate::paste;
#[cfg(target_os = "macos")]
use crate::{media, overlay, target_app};
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
#[cfg(target_os = "macos")]
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
#[cfg(target_os = "macos")]
use std::os::raw::c_void;
#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicPtr;

#[cfg(not(target_os = "macos"))]
use crate::keysym::rdev_key_to_code;
#[cfg(not(target_os = "macos"))]
use std::collections::HashSet;

const TRANSCRIPTION_ERROR_EVENT: &str = "transcription-error";
const PTT_PRESSED_EVENT: &str = "ptt-pressed";
const PTT_RELEASED_EVENT: &str = "ptt-released";
const PTT_THINKING_EVENT: &str = "ptt-thinking";
const PTT_ERROR_EVENT: &str = "ptt-error";
const PTT_CANCELLED_EVENT: &str = "ptt-cancelled";

const ERROR_FLASH: Duration = Duration::from_millis(800);
const CANCEL_FLASH: Duration = Duration::from_millis(800);
const MIN_SPEAK_DURATION: Duration = Duration::from_millis(300);

/// No window focus — caller still owns the target app's focus for paste.
fn notify_silent(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    eprintln!("[notify silent] {message}");
    let _ = app.emit(TRANSCRIPTION_ERROR_EVENT, &message);
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
}

// core-graphics keeps CGEventTapEnable private, but we need to call it from
// inside the tap's own callback (the only place we learn the system has
// disabled the tap). Redeclare the symbol — it resolves at link time against
// the CoreGraphics framework the crate already links.
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

/// Per-modifier press state. The CGEventFlags bitmask on each event is the
/// authoritative view of modifier-family state (alt / meta / ctrl / shift),
/// but it can't distinguish L vs R — so we track each side explicitly and
/// reconcile with the bitmask on every FlagsChanged to self-heal any drift.
#[cfg(target_os = "macos")]
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
#[cfg(target_os = "macos")]
fn modifier_family(keycode: u16) -> Option<CGEventFlags> {
    Some(match keycode {
        KC_ALT_LEFT | KC_ALT_RIGHT => CGEventFlags::CGEventFlagAlternate,
        KC_META_LEFT | KC_META_RIGHT => CGEventFlags::CGEventFlagCommand,
        KC_CONTROL_LEFT | KC_CONTROL_RIGHT => CGEventFlags::CGEventFlagControl,
        KC_SHIFT_LEFT | KC_SHIFT_RIGHT => CGEventFlags::CGEventFlagShift,
        _ => return None,
    })
}

/// Clear both sides of the modifier family a given keycode belongs to.
#[cfg(target_os = "macos")]
fn clear_family(state: &mut ModKeyState, keycode: u16) {
    match keycode {
        KC_ALT_LEFT | KC_ALT_RIGHT => {
            state.l_alt = false;
            state.r_alt = false;
        }
        KC_META_LEFT | KC_META_RIGHT => {
            state.l_meta = false;
            state.r_meta = false;
        }
        KC_CONTROL_LEFT | KC_CONTROL_RIGHT => {
            state.l_control = false;
            state.r_control = false;
        }
        KC_SHIFT_LEFT | KC_SHIFT_RIGHT => {
            state.l_shift = false;
            state.r_shift = false;
        }
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn side_mut(state: &mut ModKeyState, keycode: u16) -> Option<&mut bool> {
    Some(match keycode {
        KC_ALT_LEFT => &mut state.l_alt,
        KC_ALT_RIGHT => &mut state.r_alt,
        KC_META_LEFT => &mut state.l_meta,
        KC_META_RIGHT => &mut state.r_meta,
        KC_CONTROL_LEFT => &mut state.l_control,
        KC_CONTROL_RIGHT => &mut state.r_control,
        KC_SHIFT_LEFT => &mut state.l_shift,
        KC_SHIFT_RIGHT => &mut state.r_shift,
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
fn modifier_state_from_flags(flags: CGEventFlags) -> ModifierState {
    ModifierState {
        meta: flags.contains(CGEventFlags::CGEventFlagCommand),
        control: flags.contains(CGEventFlags::CGEventFlagControl),
        alt: flags.contains(CGEventFlags::CGEventFlagAlternate),
        shift: flags.contains(CGEventFlags::CGEventFlagShift),
    }
}

// Non-macOS: L/R modifier tracking fed from rdev key events.
#[cfg(not(target_os = "macos"))]
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

// Updates both the L/R side tracker and the shared ModifierState for a
// non-macOS key event. rdev reports each modifier side directly, so there
// is no FlagsChanged reconciliation step.
#[cfg(not(target_os = "macos"))]
fn update_modifier_state(state: &AppState, code: &str, is_press: bool, sides: &mut ModKeyState) {
    let changed = match code {
        "AltLeft" => { sides.l_alt = is_press; true }
        "AltRight" => { sides.r_alt = is_press; true }
        "MetaLeft" => { sides.l_meta = is_press; true }
        "MetaRight" => { sides.r_meta = is_press; true }
        "ControlLeft" => { sides.l_control = is_press; true }
        "ControlRight" => { sides.r_control = is_press; true }
        "ShiftLeft" => { sides.l_shift = is_press; true }
        "ShiftRight" => { sides.r_shift = is_press; true }
        _ => false,
    };
    if changed {
        *state.modifiers.lock().unwrap() = ModifierState {
            alt: sides.l_alt || sides.r_alt,
            meta: sides.l_meta || sides.r_meta,
            control: sides.l_control || sides.r_control,
            shift: sides.l_shift || sides.r_shift,
        };
    }
}

/// Tear down the recording side of a Session: mark cancelled, drop the active
/// shortcut, stop the recorder, resume muted media, emit the cancel event.
/// The downstream short-circuit in run_session is what skips paste / history /
/// stats / cleanup — this function only handles the immediate
/// mic-and-overlay teardown that mirrors a normal release.
fn cancel_session(app: &AppHandle, state: &AppState, recorder: &Recorder) {
    state.session_cancelled.store(true, Ordering::Release);
    *state.ptt_active.lock().unwrap() = false;
    *state.active_shortcut.lock().unwrap() = None;
    recorder.stop();
    maybe_resume_media(state);
    let _ = app.emit(PTT_CANCELLED_EVENT, ());
}

/// Mute system audio output when the user starts dictating. No-op on
/// non-macOS platforms where media control is not yet implemented.
fn maybe_pause_media(state: &AppState) {
    *state.did_pause_media.lock().unwrap() = false;
    #[cfg(target_os = "macos")]
    if *state.pause_media_on_record.lock().unwrap() {
        *state.did_pause_media.lock().unwrap() = true;
        tauri::async_runtime::spawn_blocking(media::mute_output);
    }
}

/// Mirror of maybe_pause_media. Unmutes only if this session was the one
/// that applied the mute.
fn maybe_resume_media(state: &AppState) {
    let mut flag = state.did_pause_media.lock().unwrap();
    if !*flag {
        return;
    }
    *flag = false;
    #[cfg(target_os = "macos")]
    tauri::async_runtime::spawn_blocking(media::unmute_output);
}

fn local_model_readiness(data_dir: &std::path::Path, model: LocalWhisperModel) -> Result<(), String> {
    let path = provider::local_model_path(data_dir, model);
    if path.exists() {
        Ok(())
    } else {
        Err("Local model not downloaded. Go to Settings → Local Models to download it.".to_string())
    }
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
        #[cfg(target_os = "macos")]
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
        Ok(Err(e)) => return Err(pipeline::recorder_failed_error(&e)),
        Err(_) => return Err(pipeline::RECORDER_THREAD_CRASHED_ERROR.to_string()),
    };

    let settings = config::load(app);

    // Resolve the active mode up front: the STT call needs its language hint,
    // otherwise the provider falls back to the default mode's language and
    // transcribes (e.g.) Ukrainian audio as English.
    let active_mode = settings
        .modes
        .iter()
        .find(|m| m.id == mode_id)
        .unwrap_or_else(|| config::get_default_mode(&settings));
    let mode_cleanup_enabled = active_mode.ai_cleanup.enabled;
    let mode_language = active_mode.language.clone();
    let mode_prompt_override = active_mode.ai_cleanup.prompt_override.clone();
    let session_terms =
        crate::terms::compose_term_hints(&settings.term_sets, &active_mode.term_set_ids);

    let missing_key = match active_mode.provider_model.provider() {
        TranscriptionProvider::Deepgram => settings
            .deepgram_api_key
            .as_deref()
            .is_none_or(|k| k.is_empty()),
        TranscriptionProvider::Groq => settings
            .groq_api_key
            .as_deref()
            .is_none_or(|k| k.is_empty()),
        TranscriptionProvider::AssemblyAi => settings
            .assemblyai_api_key
            .as_deref()
            .is_none_or(|k| k.is_empty()),
        TranscriptionProvider::Local => false,
    };
    if missing_key {
        recorder.stop();
        let name = match active_mode.provider_model.provider() {
            TranscriptionProvider::Deepgram => "Deepgram",
            TranscriptionProvider::Groq => "Groq",
            TranscriptionProvider::AssemblyAi => "AssemblyAI",
            TranscriptionProvider::Local => unreachable!(),
        };
        return Err(format!("API key missing for {name}"));
    }

    if let ProviderModel::Local { model } = &active_mode.provider_model {
        let check = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Cannot resolve app data directory: {e}"))
            .and_then(|dir| local_model_readiness(&dir, *model));
        if let Err(e) = check {
            recorder.stop();
            return Err(e);
        }
    }

    let session_result = match &active_mode.provider_model {
        ProviderModel::Deepgram => {
            DeepgramSession
                .run(app.clone(), format, chunk_rx, mode_language, session_terms, active_mode)
                .await
        }
        ProviderModel::Groq { model } => {
            GroqSession { model: *model }
                .run(app.clone(), format, chunk_rx, mode_language, session_terms, active_mode)
                .await
        }
        ProviderModel::AssemblyAi { model } => {
            AssemblyAiSession { model: *model }
                .run(app.clone(), format, chunk_rx, mode_language, session_terms, active_mode)
                .await
        }
        ProviderModel::Local { model } => {
            #[cfg(target_os = "macos")]
            {
                LocalSession { model: *model }
                    .run(app.clone(), format, chunk_rx, mode_language, session_terms, active_mode)
                    .await
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (model, format, chunk_rx, mode_language, session_terms);
                recorder.stop();
                return Err("Local transcription is not yet supported on this platform.".to_string());
            }
        }
    };

    if app
        .state::<AppState>()
        .session_cancelled
        .load(Ordering::Acquire)
    {
        // cancel_session already stopped the recorder; whatever session_result
        // we got is discarded. Hold the "Cancelled" pill visible for the flash
        // window before spawn_session hides the overlay.
        tokio::time::sleep(CANCEL_FLASH).await;
        return Ok(());
    }

    let (raw_text, speak_duration) = match session_result {
        Ok(r) => r,
        Err(e) => {
            // Stop the recorder if it's still running so an error doesn't
            // leak a live cpal stream.
            recorder.stop();
            return Err(e);
        }
    };
    if raw_text.is_empty() || speak_duration < MIN_SPEAK_DURATION {
        return Ok(());
    }

    let (replaced_text, cleanup_status, notice) = maybe_cleanup(
        app,
        &settings,
        mode_cleanup_enabled,
        &raw_text,
        speak_duration,
        mode_prompt_override.as_deref(),
    )
    .await;

    let pipeline::Outcome { pasted_text, mut history_entry, .. } = pipeline::run_stages(
        &raw_text,
        speak_duration,
        active_mode,
        &settings,
        CleanupOutput { replaced_text, status: cleanup_status },
    );

    #[cfg(target_os = "macos")]
    {
        let resolved_app = {
            let rx = app
                .state::<crate::state::AppState>()
                .pending_app_rx
                .lock()
                .unwrap()
                .take();
            match rx {
                Some(rx) => tokio::time::timeout(Duration::from_millis(500), rx)
                    .await
                    .ok()
                    .and_then(|r| r.ok())
                    .flatten(),
                None => None,
            }
        };
        history_entry.app_name = resolved_app.as_ref().map(|a| a.name.clone());
        history_entry.bundle_id = resolved_app.map(|a| a.bundle_id);
    }

    // paste_handle must complete before any notify_error: set_focus()
    // during the modifier-release wait would steal focus mid-paste.
    let paste_handle = paste::paste_text(pasted_text);

    let words = history_entry.final_text.split_whitespace().count() as u64;
    let seconds = speak_duration.as_secs() as u32;
    stats::record(app, words, seconds, history_entry.bundle_id.as_deref(), history_entry.app_name.as_deref());

    match history::append(app, history_entry) {
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
    match cleanup::run(transcript, credential, &prompt).await {
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

/// Acquires `ptt_active`, double-checks it's still false, then drives the full
/// PTT-start sequence: mark active, capture the target app (macOS only), spawn
/// the STT session, show the overlay (macOS only), notify the UI, and
/// (optionally) pause media.
fn start_ptt(
    app: &AppHandle,
    state: &AppState,
    recorder: &Recorder,
    shortcut: &Shortcut,
    mode_id: String,
) {
    let mut active = state.ptt_active.lock().unwrap();
    if *active {
        return;
    }
    *active = true;
    state.session_cancelled.store(false, Ordering::Release);
    *state.active_shortcut.lock().unwrap() = Some(shortcut.clone());
    let device = state.input_device.lock().unwrap().clone();
    // Capture frontmost app before overlay::show so the worker's
    // frontmostApplication() lookup can't race any window-server bookkeeping
    // that show() triggers.
    #[cfg(target_os = "macos")]
    target_app::capture(app.clone());
    // Spawn up front so the WS handshake overlaps with capture rather than
    // waiting for PTT release.
    spawn_session(app.clone(), recorder.clone(), device, mode_id);
    #[cfg(target_os = "macos")]
    overlay::show(app);
    let _ = app.emit(PTT_PRESSED_EVENT, shortcut);
    maybe_pause_media(state);
}

fn fire_paste_latest(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(entry) = history::latest(&app) else {
            return;
        };
        let text = history::pasted_text(&entry).to_string();
        if text.is_empty() {
            return;
        }
        paste::paste_text(text);
    });
}

/// Dispatches a matched binding on KeyDown. PTT bindings flip into recording
/// state and wait for the matching KeyUp; PasteLatest bindings fire-and-forget
/// against the latest history entry. PasteLatest is suppressed while a PTT
/// session is active so a stray double-tap during recording can't fire a
/// stale paste at the same target.
fn dispatch_binding(
    app: &AppHandle,
    state: &AppState,
    recorder: &Recorder,
    binding: &HotkeyBinding,
) {
    match &binding.action {
        HotkeyAction::Ptt { mode_id } => {
            start_ptt(app, state, recorder, &binding.shortcut, mode_id.clone());
        }
        HotkeyAction::PasteLatest => {
            if *state.ptt_active.lock().unwrap() {
                return;
            }
            fire_paste_latest(app);
        }
    }
}

// ── macOS event source: CGEventTap ───────────────────────────────────────────

/// Caller must set `state.ptt_running` to true (via CAS) before invoking.
/// This function only clears it on `CGEventTap::new` failure.
#[cfg(target_os = "macos")]
pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    std::thread::spawn(move || {
        let ptt_running = state.ptt_running.clone();

        let mod_state = Mutex::new(ModKeyState::default());
        // Shared handle to the tap's mach port so the callback can re-enable
        // itself. Populated after CGEventTap::new returns.
        let tap_port: Arc<AtomicPtr<c_void>> = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let tap_port_cb = tap_port.clone();
        // Per-shortcut tap state, keyed by (key, modifiers). Shared with
        // spawned coexistence timer tasks via Arc; single-press-only keys with
        // no double-tap sibling never touch this map.
        let tap_states: Arc<Mutex<HashMap<(String, Vec<String>), TapState>>> =
            Arc::new(Mutex::new(HashMap::new()));

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

                let Some(code) = keycode_to_code(keycode) else {
                    return None;
                };

                // KeyDown auto-repeat re-asserts a held key every ~30 ms once
                // the OS repeat delay elapses. Treating each repeat as a fresh
                // tap would defeat the coexistence timer (every repeat would
                // bump the generation and reschedule a new 400 ms timer that
                // never fires).
                if matches!(event_type, CGEventType::KeyDown)
                    && event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0
                {
                    return None;
                }

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

                // Cancel takes precedence over every other tap path. Bypasses
                // the relevance filter (Escape doesn't match any PTT shortcut)
                // and the coexistence timer (modifier state is irrelevant —
                // see is_cancel_event).
                if is_cancel_event(code, is_press, ptt_active_now) {
                    cancel_session(&app, &state, &recorder);
                    return None;
                }

                let bindings = state.hotkey_bindings.lock().unwrap().clone();
                let modifiers_val = *state.modifiers.lock().unwrap();

                // Reset pending tap-state for any shortcut whose key this event
                // doesn't match — "other key" cancels the gesture. Touches the
                // shared map: covers both bare double-tap bindings and the
                // coexistence pair (which keys on the same dt shortcut).
                if is_press && !ptt_active_now {
                    let mut tap_states_guard = tap_states.lock().unwrap();
                    for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                        if !shortcut_matches(code, &b.shortcut, modifiers_val) {
                            if let Some(ts) =
                                tap_states_guard.get_mut(&tap_state_key(&b.shortcut))
                            {
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

                if is_press && !ptt_active_now {
                    let sp = bindings.iter().find(|b| {
                        !b.shortcut.is_double_tap
                            && shortcut_matches(code, &b.shortcut, modifiers_val)
                    });
                    let dt = bindings.iter().find(|b| {
                        b.shortcut.is_double_tap
                            && shortcut_matches(code, &b.shortcut, modifiers_val)
                    });

                    match (sp, dt) {
                        (Some(sp_b), Some(dt_b))
                            if key_has_both_kinds(&bindings, &dt_b.shortcut) =>
                        {
                            let key = tap_state_key(&dt_b.shortcut);
                            let outcome = {
                                let mut guard = tap_states.lock().unwrap();
                                let ts = guard.entry(key.clone()).or_default();
                                coex_advance_down(ts, now)
                            };
                            match outcome {
                                CoexDown::FireDoubleTap => {
                                    dispatch_binding(&app, &state, &recorder, dt_b);
                                }
                                CoexDown::ScheduleSinglePress { captured_generation } => {
                                    let tap_states_for_timer = tap_states.clone();
                                    let app_for_timer = app.clone();
                                    let state_for_timer = state.clone();
                                    let recorder_for_timer = recorder.clone();
                                    let sp_for_timer = sp_b.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(DOUBLE_TAP_THRESHOLD).await;
                                        let should_fire = {
                                            let mut guard =
                                                tap_states_for_timer.lock().unwrap();
                                            let Some(ts) = guard.get_mut(&key) else {
                                                return;
                                            };
                                            if !coex_timer_should_fire(ts, captured_generation)
                                            {
                                                return;
                                            }
                                            ts.generation = ts.generation.wrapping_add(1);
                                            true
                                        };
                                        if should_fire {
                                            dispatch_binding(
                                                &app_for_timer,
                                                &state_for_timer,
                                                &recorder_for_timer,
                                                &sp_for_timer,
                                            );
                                        }
                                    });
                                }
                            }
                        }
                        (_, Some(dt_b)) => {
                            // Double-tap only: existing #40 behavior.
                            let dispatch = {
                                let mut guard = tap_states.lock().unwrap();
                                let ts = guard.entry(tap_state_key(&dt_b.shortcut)).or_default();
                                advance_tap_state(ts, TapEvent::Down, now)
                            };
                            if dispatch == Dispatch::StartPtt {
                                dispatch_binding(&app, &state, &recorder, dt_b);
                            }
                        }
                        (Some(sp_b), None) => {
                            // Single-press only: fire immediately, no regression.
                            dispatch_binding(&app, &state, &recorder, sp_b);
                        }
                        (None, None) => {}
                    }
                } else if !is_press {
                    let mut active = state.ptt_active.lock().unwrap();
                    if *active {
                        let sc_opt = state.active_shortcut.lock().unwrap().clone();
                        let should_stop = match sc_opt {
                            Some(ref sc) if sc.is_double_tap => {
                                let mut tap_states_guard = tap_states.lock().unwrap();
                                let ts =
                                    tap_states_guard.entry(tap_state_key(sc)).or_default();
                                advance_tap_state(ts, TapEvent::Up, now) == Dispatch::StopPtt
                            }
                            _ => true,
                        };
                        if should_stop {
                            *active = false;
                            *state.active_shortcut.lock().unwrap() = None;
                            // Overlay stays visible — spawn_session hides it
                            // after paste so the "still processing" state
                            // bridges STT drain and any LLM cleanup pass.
                            let _ = app.emit(PTT_RELEASED_EVENT, ());
                            maybe_resume_media(&state);
                            recorder.stop();
                        }
                    } else {
                        // Key-up while idle: record first-tap-up time for any
                        // double-tap (or coexistence) shortcut on this key.
                        let mut tap_states_guard = tap_states.lock().unwrap();
                        for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                            if shortcut_matches(code, &b.shortcut, modifiers_val) {
                                if let Some(ts) =
                                    tap_states_guard.get_mut(&tap_state_key(&b.shortcut))
                                {
                                    if ts.tap_count > 0 {
                                        advance_tap_state(ts, TapEvent::Up, now);
                                    }
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
                ptt_running.store(false, std::sync::atomic::Ordering::Release);
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

// ── Non-macOS event source: rdev ─────────────────────────────────────────────

/// Caller must set `state.ptt_running` to true (via CAS) before invoking.
/// Spawns a dedicated thread that runs rdev::listen; the callback translates
/// rdev events into (code, is_press) pairs and drives the same hotkey state
/// machine used by the macOS CGEventTap source.
///
// Keyboard-capture pattern adapted from the MIT-licensed Handy project
// (https://github.com/cjpais/Handy). Copyright (c) cjpais.
#[cfg(not(target_os = "macos"))]
pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    std::thread::spawn(move || {
        // Per-shortcut tap state, keyed by (key, modifiers). Shared with
        // spawned coexistence timer tasks via Arc.
        let tap_states: Arc<Mutex<HashMap<(String, Vec<String>), TapState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Tracks currently-held keys so OS key-repeat events (successive
        // KeyPress without an intervening KeyRelease) are ignored.
        let pressed_keys: Arc<Mutex<HashSet<String>>> =
            Arc::new(Mutex::new(HashSet::new()));
        // L/R per-side modifier tracking; the aggregated ModifierState is
        // written into state.modifiers after each modifier event.
        let mod_sides = Arc::new(Mutex::new(ModKeyState::default()));

        let callback = {
            let app = app.clone();
            let state = state.clone();
            let recorder = recorder.clone();

            move |event: rdev::Event| {
                let (code, is_press) = match event.event_type {
                    rdev::EventType::KeyPress(key) => match rdev_key_to_code(&key) {
                        Some(c) => (c, true),
                        None => return,
                    },
                    rdev::EventType::KeyRelease(key) => match rdev_key_to_code(&key) {
                        Some(c) => (c, false),
                        None => return,
                    },
                    _ => return,
                };

                // Auto-repeat suppression: skip a press if the key is already
                // in the held set. The set is cleared on release so a genuine
                // re-press after release goes through.
                {
                    let mut pressed = pressed_keys.lock().unwrap();
                    if is_press {
                        if !pressed.insert(code.to_string()) {
                            return;
                        }
                    } else {
                        pressed.remove(code);
                    }
                }

                update_modifier_state(&state, code, is_press, &mut mod_sides.lock().unwrap());

                if *state.shortcut_capture_paused.lock().unwrap() {
                    return;
                }

                let now = Instant::now();
                let ptt_active_now = *state.ptt_active.lock().unwrap();

                if is_cancel_event(code, is_press, ptt_active_now) {
                    cancel_session(&app, &state, &recorder);
                    return;
                }

                let bindings = state.hotkey_bindings.lock().unwrap().clone();
                let modifiers_val = *state.modifiers.lock().unwrap();

                if is_press && !ptt_active_now {
                    let mut tap_states_guard = tap_states.lock().unwrap();
                    for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                        if !shortcut_matches(code, &b.shortcut, modifiers_val) {
                            if let Some(ts) =
                                tap_states_guard.get_mut(&tap_state_key(&b.shortcut))
                            {
                                advance_tap_state(ts, TapEvent::OtherKey, now);
                            }
                        }
                    }
                }

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
                    return;
                }

                if is_press && !ptt_active_now {
                    let sp = bindings.iter().find(|b| {
                        !b.shortcut.is_double_tap
                            && shortcut_matches(code, &b.shortcut, modifiers_val)
                    });
                    let dt = bindings.iter().find(|b| {
                        b.shortcut.is_double_tap
                            && shortcut_matches(code, &b.shortcut, modifiers_val)
                    });

                    match (sp, dt) {
                        (Some(sp_b), Some(dt_b))
                            if key_has_both_kinds(&bindings, &dt_b.shortcut) =>
                        {
                            let key = tap_state_key(&dt_b.shortcut);
                            let outcome = {
                                let mut guard = tap_states.lock().unwrap();
                                let ts = guard.entry(key.clone()).or_default();
                                coex_advance_down(ts, now)
                            };
                            match outcome {
                                CoexDown::FireDoubleTap => {
                                    dispatch_binding(&app, &state, &recorder, dt_b);
                                }
                                CoexDown::ScheduleSinglePress { captured_generation } => {
                                    let tap_states_for_timer = tap_states.clone();
                                    let app_for_timer = app.clone();
                                    let state_for_timer = state.clone();
                                    let recorder_for_timer = recorder.clone();
                                    let sp_for_timer = sp_b.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(DOUBLE_TAP_THRESHOLD).await;
                                        let should_fire = {
                                            let mut guard =
                                                tap_states_for_timer.lock().unwrap();
                                            let Some(ts) = guard.get_mut(&key) else {
                                                return;
                                            };
                                            if !coex_timer_should_fire(ts, captured_generation) {
                                                return;
                                            }
                                            ts.generation = ts.generation.wrapping_add(1);
                                            true
                                        };
                                        if should_fire {
                                            dispatch_binding(
                                                &app_for_timer,
                                                &state_for_timer,
                                                &recorder_for_timer,
                                                &sp_for_timer,
                                            );
                                        }
                                    });
                                }
                            }
                        }
                        (_, Some(dt_b)) => {
                            let dispatch = {
                                let mut guard = tap_states.lock().unwrap();
                                let ts = guard.entry(tap_state_key(&dt_b.shortcut)).or_default();
                                advance_tap_state(ts, TapEvent::Down, now)
                            };
                            if dispatch == Dispatch::StartPtt {
                                dispatch_binding(&app, &state, &recorder, dt_b);
                            }
                        }
                        (Some(sp_b), None) => {
                            dispatch_binding(&app, &state, &recorder, sp_b);
                        }
                        (None, None) => {}
                    }
                } else if !is_press {
                    let mut active = state.ptt_active.lock().unwrap();
                    if *active {
                        let sc_opt = state.active_shortcut.lock().unwrap().clone();
                        let should_stop = match sc_opt {
                            Some(ref sc) if sc.is_double_tap => {
                                let mut tap_states_guard = tap_states.lock().unwrap();
                                let ts =
                                    tap_states_guard.entry(tap_state_key(sc)).or_default();
                                advance_tap_state(ts, TapEvent::Up, now) == Dispatch::StopPtt
                            }
                            _ => true,
                        };
                        if should_stop {
                            *active = false;
                            *state.active_shortcut.lock().unwrap() = None;
                            let _ = app.emit(PTT_RELEASED_EVENT, ());
                            maybe_resume_media(&state);
                            recorder.stop();
                        }
                    } else {
                        let mut tap_states_guard = tap_states.lock().unwrap();
                        for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                            if shortcut_matches(code, &b.shortcut, modifiers_val) {
                                if let Some(ts) =
                                    tap_states_guard.get_mut(&tap_state_key(&b.shortcut))
                                {
                                    if ts.tap_count > 0 {
                                        advance_tap_state(ts, TapEvent::Up, now);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        if let Err(e) = rdev::listen(callback) {
            eprintln!("[ptt] rdev listener failed: {e:?}");
            state.ptt_running.store(false, Ordering::Release);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_readiness_fails_when_model_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let result = local_model_readiness(dir.path(), LocalWhisperModel::LargeV3);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("Settings → Local Models"), "message was: {msg}");
    }

    #[test]
    fn local_readiness_passes_when_model_file_present() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = crate::provider::local_model_path(dir.path(), LocalWhisperModel::LargeV3Turbo);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"stub").unwrap();
        assert!(local_model_readiness(dir.path(), LocalWhisperModel::LargeV3Turbo).is_ok());
    }

    #[test]
    fn local_readiness_checks_correct_variant() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path_v3 = crate::provider::local_model_path(dir.path(), LocalWhisperModel::LargeV3);
        fs::create_dir_all(path_v3.parent().unwrap()).unwrap();
        fs::write(&path_v3, b"stub").unwrap();
        assert!(local_model_readiness(dir.path(), LocalWhisperModel::LargeV3).is_ok());
        assert!(local_model_readiness(dir.path(), LocalWhisperModel::LargeV3Turbo).is_err());
    }
}
