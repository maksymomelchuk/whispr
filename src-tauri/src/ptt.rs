use crate::assemblyai_session::AssemblyAiEngine;
use crate::config::{HotkeyAction, HotkeyBinding, Shortcut};
use crate::corrections::compose_corrections;
use crate::deepgram_session::DeepgramEngine;
use crate::elevenlabs_session::ElevenLabsEngine;
use crate::engine::EngineContext;
use crate::groq_session::GroqEngine;
use crate::history::{self, CleanupStatus, HISTORY_UPDATED_EVENT};
use crate::hotkey::{
    advance_tap_state, coex_advance_down, coex_timer_should_fire, is_cancel_event,
    key_has_both_kinds, shortcut_is_relevant, shortcut_matches, tap_state_key, CoexDown, Dispatch,
    TapEvent, TapState, DOUBLE_TAP_THRESHOLD,
};
use crate::local_engine::LocalWhisperEngine;
use crate::openai_transcribe_session::OpenAiTranscribeEngine;
use crate::pipeline::{self, CleanupOutput, Notice};
use crate::provider::{self, LocalWhisperModel, ProviderModel, TranscriptionProvider};
use crate::recorder::Recorder;
use crate::session::{Session, PTT_ERROR_EVENT, TRANSCRIPTION_ERROR_EVENT};
use crate::state::{AppState, ModifierState};
use crate::{cleanup, cleanup_invoke, cleanup_stats, config, model_catalog, recovery, stats};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
use crate::keysym::{
    keycode_to_code, KC_ALT_LEFT, KC_ALT_RIGHT, KC_CONTROL_LEFT, KC_CONTROL_RIGHT, KC_META_LEFT,
    KC_META_RIGHT, KC_SHIFT_LEFT, KC_SHIFT_RIGHT,
};
use crate::paste;
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

#[cfg(target_os = "linux")]
use crate::keysym::rdev_key_to_code;
#[cfg(not(target_os = "macos"))]
use std::collections::HashSet;

const PTT_PRESSED_EVENT: &str = "ptt-pressed";
const PTT_RELEASED_EVENT: &str = "ptt-released";
const PTT_THINKING_EVENT: &str = "ptt-thinking";
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

// rdev reports each modifier side directly, so there is no FlagsChanged reconciliation step.
#[cfg(not(target_os = "macos"))]
fn update_modifier_state(state: &AppState, code: &str, is_press: bool, sides: &mut ModKeyState) {
    let changed = match code {
        "AltLeft" => {
            sides.l_alt = is_press;
            true
        }
        "AltRight" => {
            sides.r_alt = is_press;
            true
        }
        "MetaLeft" => {
            sides.l_meta = is_press;
            true
        }
        "MetaRight" => {
            sides.r_meta = is_press;
            true
        }
        "ControlLeft" => {
            sides.l_control = is_press;
            true
        }
        "ControlRight" => {
            sides.r_control = is_press;
            true
        }
        "ShiftLeft" => {
            sides.l_shift = is_press;
            true
        }
        "ShiftRight" => {
            sides.r_shift = is_press;
            true
        }
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

fn maybe_pause_media(state: &AppState) {
    *state.did_pause_media.lock().unwrap() = false;
    if *state.pause_media_on_record.lock().unwrap() {
        *state.did_pause_media.lock().unwrap() = true;
        tauri::async_runtime::spawn_blocking(media::mute_output);
    }
}

/// Unmutes only if this session was the one that applied the mute.
fn maybe_resume_media(state: &AppState) {
    let mut flag = state.did_pause_media.lock().unwrap();
    if !*flag {
        return;
    }
    *flag = false;
    tauri::async_runtime::spawn_blocking(media::unmute_output);
}

fn local_model_readiness(
    data_dir: &std::path::Path,
    model: LocalWhisperModel,
) -> Result<(), String> {
    let catalog = model_catalog::catalog_for(model);
    let models_dir = data_dir.join("models");
    if model_catalog::all_files_present(&catalog, &models_dir) {
        Ok(())
    } else {
        Err("Local model not downloaded. Go to Settings → Local Models to download it.".to_string())
    }
}

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
        Ok(Err(e)) => return Err(pipeline::recorder_failed_error(&e)),
        Err(_) => return Err(pipeline::RECORDER_THREAD_CRASHED_ERROR.to_string()),
    };

    let settings = config::load(app);

    // Resolve the active mode up front: the STT call needs its language hint,
    // otherwise the provider falls back to the first mode's language and
    // transcribes (e.g.) Ukrainian audio as English.
    let active_mode = settings
        .modes
        .iter()
        .find(|m| m.id == mode_id)
        .or_else(|| settings.modes.first())
        .ok_or_else(|| format!("No profile found for mode '{mode_id}'."))?;
    let mode_cleanup_enabled = active_mode.ai_cleanup.enabled;
    let mode_language = active_mode.language.clone();
    let mode_prompt_override = active_mode.ai_cleanup.prompt_override.clone();
    let cleanup_provider = active_mode.ai_cleanup.provider;
    let cleanup_model = active_mode.ai_cleanup.model.clone();
    let paste_raw_on_failure = active_mode.ai_cleanup.paste_raw_on_failure;
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
        TranscriptionProvider::OpenAi => settings
            .openai_api_key
            .as_deref()
            .is_none_or(|k| k.is_empty()),
        TranscriptionProvider::ElevenLabs => settings
            .elevenlabs_api_key
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
            TranscriptionProvider::OpenAi => "OpenAI",
            TranscriptionProvider::ElevenLabs => "ElevenLabs",
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
            // Prefer the new per-provider key; fall back to the legacy single-key
            // field for the brief window before `load`'s migration has re-saved.
            let key = settings
                .deepgram_api_key
                .clone()
                .or_else(|| settings.api_key.clone())
                .filter(|k| !k.is_empty())
                .ok_or_else(|| "API key not configured".to_string())?;
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                DeepgramEngine::new(key),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
        }
        ProviderModel::Groq { model } => {
            let key = settings
                .groq_api_key
                .clone()
                .filter(|k| !k.is_empty())
                .ok_or_else(|| "API key not configured".to_string())?;
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                GroqEngine::new(*model, key),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
        }
        ProviderModel::AssemblyAi { model } => {
            let key = settings.assemblyai_api_key.clone().unwrap_or_default();
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                AssemblyAiEngine::new(*model, key),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
        }
        ProviderModel::Local { model } => {
            let cache = app.state::<AppState>().model_cache.clone();
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Cannot resolve app data directory: {e}"))?;
            let model_path = provider::local_model_path(&data_dir, *model);
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                LocalWhisperEngine::new(*model, cache, model_path),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
        }
        ProviderModel::OpenAi { model } => {
            let key = settings
                .openai_api_key
                .clone()
                .filter(|k| !k.is_empty())
                .ok_or_else(|| "API key not configured".to_string())?;
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                OpenAiTranscribeEngine::new(*model, key),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
        }
        ProviderModel::ElevenLabs => {
            let key = settings
                .elevenlabs_api_key
                .clone()
                .filter(|k| !k.is_empty())
                .ok_or_else(|| "API key not configured".to_string())?;
            let corrections =
                compose_corrections(&active_mode.correction_set_ids, &settings.correction_sets);
            let ctx = EngineContext {
                format,
                language: mode_language,
                terms: session_terms,
            };
            Session::new(
                ElevenLabsEngine::new(key),
                app.clone(),
                settings.show_live_preview,
                corrections,
            )
            .run(chunk_rx, ctx)
            .await
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
        cleanup_provider,
        &cleanup_model,
    )
    .await;

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let pipeline::Outcome {
        pasted_text,
        mut history_entry,
        ..
    } = pipeline::run_stages(
        &raw_text,
        speak_duration,
        active_mode,
        &settings,
        CleanupOutput {
            replaced_text,
            status: cleanup_status,
        },
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

    let paste_policy =
        pipeline::resolve_paste_policy(&history_entry.cleanup_status, paste_raw_on_failure);

    // paste_handle must complete before any notify_error: set_focus()
    // during the modifier-release wait would steal focus mid-paste.
    let paste_handle = match paste_policy {
        pipeline::PastePolicy::PasteRaw => paste::paste_text(pasted_text),
        pipeline::PastePolicy::SuppressAndClipboard => paste::write_to_clipboard(raw_text),
    };

    let words = history_entry.final_text.split_whitespace().count() as u64;
    let seconds = speak_duration.as_secs() as u32;
    stats::record(
        app,
        words,
        seconds,
        history_entry.bundle_id.as_deref(),
        history_entry.app_name.as_deref(),
    );

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
    cleanup_provider: cleanup::AiProviderId,
    cleanup_model: &str,
) -> (String, CleanupStatus, Notice) {
    let cleanup_settings = &settings.ai_cleanup;

    if !mode_cleanup_enabled {
        return (
            transcript.to_string(),
            CleanupStatus::Disabled,
            Notice::None,
        );
    }

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

    let result = cleanup_invoke::invoke(
        cleanup_settings,
        cleanup_provider,
        cleanup_model,
        prompt_override,
        transcript,
    )
    .await;

    match result {
        Ok((cleaned, usage)) => {
            cleanup_stats::record(app, usage.input_tokens, usage.output_tokens);
            (cleaned, CleanupStatus::Ran, Notice::None)
        }
        Err(err) => {
            let message = format!("AI cleanup unavailable: {err}");
            let (status, notice) = match err {
                cleanup::CleanupError::Credential(m) => {
                    if cleanup_invoke::is_credential_configured(cleanup_settings, cleanup_provider)
                    {
                        (CleanupStatus::FailedCredential(m), Notice::Focus(message))
                    } else {
                        (CleanupStatus::NoCredential, Notice::Focus(m))
                    }
                }
                cleanup::CleanupError::Timeout(_) => {
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
    // Capture the target app before showing our overlay: show() can make the
    // overlay the frontmost window, and the capture would then attribute the
    // dictation to our own overlay instead of the app the user is typing into.
    target_app::capture(app.clone());
    // Spawn up front so the WS handshake overlaps with capture rather than
    // waiting for PTT release.
    spawn_session(app.clone(), recorder.clone(), device, mode_id);
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

fn fire_recover_latest(app: &AppHandle, state: &AppState) {
    use std::sync::atomic::Ordering;
    if state.recover_in_flight.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let Some(entry) = history::latest(&app) else {
            state.recover_in_flight.store(false, Ordering::Release);
            return;
        };
        if !recovery::is_recoverable(&entry) {
            notify_silent(&app, "Nothing to recover");
            tokio::time::sleep(ERROR_FLASH).await;
            state.recover_in_flight.store(false, Ordering::Release);
            return;
        }
        let settings = config::load(&app);
        overlay::show(&app);
        let _ = app.emit(PTT_THINKING_EVENT, ());
        let result = recovery::recover_entry(&entry, &settings).await;
        overlay::hide(&app);
        match result {
            Ok(outcome) => {
                if let Err(e) = history::update_by_id(
                    &app,
                    &entry.id,
                    outcome.history_entry.replaced_text,
                    outcome.history_entry.final_text,
                ) {
                    eprintln!("[recovery] update_by_id failed: {e}");
                }
                let _ = app.emit(HISTORY_UPDATED_EVENT, ());
                paste::paste_text(outcome.pasted_text).await.ok();
            }
            Err(err) => {
                notify_silent(&app, format!("Recovery failed: {err}"));
                tokio::time::sleep(ERROR_FLASH).await;
            }
        }
        state.recover_in_flight.store(false, Ordering::Release);
    });
}

/// PasteLatest and RecoverLatest are suppressed while a PTT session is active
/// so a stray double-tap during recording can't fire a stale paste at the same
/// target.
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
        HotkeyAction::RecoverLatest => {
            if *state.ptt_active.lock().unwrap() {
                return;
            }
            fire_recover_latest(app, state);
        }
    }
}

// ── macOS event source: CGEventTap ───────────────────────────────────────────

/// Caller must set `state.ptt_running` to true (via CAS) before invoking.
#[cfg(target_os = "macos")]
pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    std::thread::spawn(move || {
        let ptt_running = state.ptt_running.clone();

        let mod_state = Mutex::new(ModKeyState::default());
        let tap_port: Arc<AtomicPtr<c_void>> = Arc::new(AtomicPtr::new(std::ptr::null_mut()));
        let tap_port_cb = tap_port.clone();
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

                if is_press && !ptt_active_now {
                    let mut tap_states_guard = tap_states.lock().unwrap();
                    for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                        if !shortcut_matches(code, &b.shortcut, modifiers_val) {
                            if let Some(ts) = tap_states_guard.get_mut(&tap_state_key(&b.shortcut))
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
                    bindings
                        .iter()
                        .any(|b| shortcut_is_relevant(code, &b.shortcut))
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
                                CoexDown::ScheduleSinglePress {
                                    captured_generation,
                                } => {
                                    let tap_states_for_timer = tap_states.clone();
                                    let app_for_timer = app.clone();
                                    let state_for_timer = state.clone();
                                    let recorder_for_timer = recorder.clone();
                                    let sp_for_timer = sp_b.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(DOUBLE_TAP_THRESHOLD).await;
                                        let should_fire = {
                                            let mut guard = tap_states_for_timer.lock().unwrap();
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
                                let ts = tap_states_guard.entry(tap_state_key(sc)).or_default();
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

// ── Shared keyboard-event handling (Windows + Linux) ─────────────────────────

#[cfg(not(target_os = "macos"))]
struct EventCtx {
    app: AppHandle,
    state: AppState,
    recorder: Recorder,
    tap_states: Arc<Mutex<HashMap<(String, Vec<String>), TapState>>>,
}

#[cfg(not(target_os = "macos"))]
#[derive(Default)]
struct EventTracking {
    /// Currently-held keys, so OS auto-repeat (successive press without an
    /// intervening release) is ignored.
    pressed_keys: HashSet<String>,
    mod_sides: ModKeyState,
    was_capture_paused: bool,
}

#[cfg(not(target_os = "macos"))]
fn handle_key_event(ctx: &EventCtx, tracking: &mut EventTracking, code: &str, is_press: bool) {
    let app = &ctx.app;
    let state = &ctx.state;
    let recorder = &ctx.recorder;
    let tap_states = &ctx.tap_states;

    // A key pressed while the settings UI binds a shortcut can lose its release
    // to the settings window, which would otherwise leave it stuck "down" in
    // held-key/modifier tracking — dead until restart.
    if *state.shortcut_capture_paused.lock().unwrap() {
        tracking.was_capture_paused = true;
        return;
    }
    if tracking.was_capture_paused {
        tracking.was_capture_paused = false;
        tracking.pressed_keys.clear();
        tracking.mod_sides = ModKeyState::default();
        *state.modifiers.lock().unwrap() = ModifierState::default();
    }

    if is_press {
        if !tracking.pressed_keys.insert(code.to_string()) {
            return;
        }
    } else {
        tracking.pressed_keys.remove(code);
    }

    update_modifier_state(state, code, is_press, &mut tracking.mod_sides);

    let now = Instant::now();
    let ptt_active_now = *state.ptt_active.lock().unwrap();

    if is_cancel_event(code, is_press, ptt_active_now) {
        cancel_session(app, state, recorder);
        return;
    }

    let bindings = state.hotkey_bindings.lock().unwrap().clone();
    let modifiers_val = *state.modifiers.lock().unwrap();

    if is_press && !ptt_active_now {
        let mut tap_states_guard = tap_states.lock().unwrap();
        for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
            if !shortcut_matches(code, &b.shortcut, modifiers_val) {
                if let Some(ts) = tap_states_guard.get_mut(&tap_state_key(&b.shortcut)) {
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
        bindings
            .iter()
            .any(|b| shortcut_is_relevant(code, &b.shortcut))
    };
    if !relevant {
        return;
    }

    if is_press && !ptt_active_now {
        let sp = bindings.iter().find(|b| {
            !b.shortcut.is_double_tap && shortcut_matches(code, &b.shortcut, modifiers_val)
        });
        let dt = bindings.iter().find(|b| {
            b.shortcut.is_double_tap && shortcut_matches(code, &b.shortcut, modifiers_val)
        });

        match (sp, dt) {
            (Some(sp_b), Some(dt_b)) if key_has_both_kinds(&bindings, &dt_b.shortcut) => {
                let key = tap_state_key(&dt_b.shortcut);
                let outcome = {
                    let mut guard = tap_states.lock().unwrap();
                    let ts = guard.entry(key.clone()).or_default();
                    coex_advance_down(ts, now)
                };
                match outcome {
                    CoexDown::FireDoubleTap => {
                        dispatch_binding(app, state, recorder, dt_b);
                    }
                    CoexDown::ScheduleSinglePress {
                        captured_generation,
                    } => {
                        let tap_states_for_timer = tap_states.clone();
                        let app_for_timer = app.clone();
                        let state_for_timer = state.clone();
                        let recorder_for_timer = recorder.clone();
                        let sp_for_timer = sp_b.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(DOUBLE_TAP_THRESHOLD).await;
                            let should_fire = {
                                let mut guard = tap_states_for_timer.lock().unwrap();
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
                    dispatch_binding(app, state, recorder, dt_b);
                }
            }
            (Some(sp_b), None) => {
                dispatch_binding(app, state, recorder, sp_b);
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
                    let ts = tap_states_guard.entry(tap_state_key(sc)).or_default();
                    advance_tap_state(ts, TapEvent::Up, now) == Dispatch::StopPtt
                }
                _ => true,
            };
            if should_stop {
                *active = false;
                *state.active_shortcut.lock().unwrap() = None;
                let _ = app.emit(PTT_RELEASED_EVENT, ());
                maybe_resume_media(state);
                recorder.stop();
            }
        } else {
            let mut tap_states_guard = tap_states.lock().unwrap();
            for b in bindings.iter().filter(|b| b.shortcut.is_double_tap) {
                if shortcut_matches(code, &b.shortcut, modifiers_val) {
                    if let Some(ts) = tap_states_guard.get_mut(&tap_state_key(&b.shortcut)) {
                        if ts.tap_count > 0 {
                            advance_tap_state(ts, TapEvent::Up, now);
                        }
                    }
                }
            }
        }
    }
}

// ── Linux event source: rdev ─────────────────────────────────────────────────

/// Caller must set `state.ptt_running` to true (via CAS) before invoking.
// Keyboard-capture pattern adapted from the MIT-licensed Handy project
// (https://github.com/cjpais/Handy). Copyright (c) cjpais.
#[cfg(target_os = "linux")]
pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    std::thread::spawn(move || {
        let ptt_running = state.ptt_running.clone();
        let ctx = EventCtx {
            app,
            state,
            recorder,
            tap_states: Arc::new(Mutex::new(HashMap::new())),
        };
        let mut tracking = EventTracking::default();

        let callback = move |event: rdev::Event| {
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
            handle_key_event(&ctx, &mut tracking, code, is_press);
        };

        if let Err(e) = rdev::listen(callback) {
            eprintln!("[ptt] rdev listener failed: {e:?}");
            ptt_running.store(false, Ordering::Release);
        }
    });
}

// ── Windows event source: WH_KEYBOARD_LL ──────────────────────────────────────
//
// We own the low-level hook directly instead of going through rdev: it reports
// distinct VKs for left/right modifiers (rdev collapses them) and lets us
// forward raw keycodes to a worker thread without resolving the character
// layer. Capture works while our own window is focused only because RawInput is
// disabled (DeviceEventFilter::Always in lib.rs) — tao's RawInput registration
// otherwise preempts this hook for our process's own focused window.

// FFI structs mirror the Win32 layout; several fields exist only for size and
// are never read.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod win32_hook {
    use std::ffi::c_void;

    pub const WH_KEYBOARD_LL: i32 = 13;
    pub const HC_ACTION: i32 = 0;
    pub const WM_KEYDOWN: usize = 0x0100;
    pub const WM_KEYUP: usize = 0x0101;
    pub const WM_SYSKEYDOWN: usize = 0x0104;
    pub const WM_SYSKEYUP: usize = 0x0105;
    pub const LLKHF_INJECTED: u32 = 0x10;

    #[repr(C)]
    pub struct KbdLlHookStruct {
        pub vk_code: u32,
        pub scan_code: u32,
        pub flags: u32,
        pub time: u32,
        pub dw_extra_info: usize,
    }

    #[repr(C)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    pub struct Msg {
        pub hwnd: *mut c_void,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt: Point,
    }

    pub type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;

    #[link(name = "user32")]
    extern "system" {
        pub fn SetWindowsHookExW(
            id_hook: i32,
            lpfn: HookProc,
            hmod: *mut c_void,
            dw_thread_id: u32,
        ) -> *mut c_void;
        pub fn CallNextHookEx(
            hhk: *mut c_void,
            n_code: i32,
            w_param: usize,
            l_param: isize,
        ) -> isize;
        pub fn GetMessageW(
            lp_msg: *mut Msg,
            hwnd: *mut c_void,
            w_msg_filter_min: u32,
            w_msg_filter_max: u32,
        ) -> i32;
        pub fn TranslateMessage(lp_msg: *const Msg) -> i32;
        pub fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    }
}

/// Maps Windows virtual-key codes to the web `KeyboardEvent.code` strings that
/// bindings are stored as. A low-level hook reports distinct VKs for the left
/// and right modifiers, so they resolve unambiguously (unlike rdev).
#[cfg(target_os = "windows")]
fn vk_to_code(vk: u32) -> Option<&'static str> {
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

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_SENDER: std::cell::RefCell<Option<std::sync::mpsc::Sender<(u32, u32, usize)>>> =
        const { std::cell::RefCell::new(None) };
}

// The callback must return fast: Windows drops the event and stalls the hook if
// it exceeds LowLevelHooksTimeout (~300ms), and dispatch marshals overlay/emit
// work to the main UI thread. So it only forwards the raw (vk, flags, message)
// and all filtering/mapping/matching runs on the worker thread.
#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_proc(code: i32, w_param: usize, l_param: isize) -> isize {
    if code == win32_hook::HC_ACTION {
        let info = &*(l_param as *const win32_hook::KbdLlHookStruct);
        HOOK_SENDER.with(|cell| {
            if let Some(tx) = cell.borrow().as_ref() {
                let _ = tx.send((info.vk_code, info.flags, w_param));
            }
        });
    }
    win32_hook::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param)
}

#[cfg(target_os = "windows")]
fn run_event_worker(ctx: EventCtx, rx: std::sync::mpsc::Receiver<(u32, u32, usize)>) {
    let mut tracking = EventTracking::default();
    while let Ok((vk, flags, message)) = rx.recv() {
        let injected = flags & win32_hook::LLKHF_INJECTED != 0;
        let is_press = message == win32_hook::WM_KEYDOWN || message == win32_hook::WM_SYSKEYDOWN;
        let is_release = message == win32_hook::WM_KEYUP || message == win32_hook::WM_SYSKEYUP;
        let mapped = vk_to_code(vk);
        // Skip our own synthetic paste keystrokes so they can't match a hotkey.
        if injected || !(is_press || is_release) {
            continue;
        }
        if let Some(code) = mapped {
            handle_key_event(&ctx, &mut tracking, code, is_press);
        }
    }
}

/// Caller must set `state.ptt_running` to true (via CAS) before invoking.
#[cfg(target_os = "windows")]
pub fn start(app: AppHandle, state: AppState, recorder: Recorder) {
    let ptt_running = state.ptt_running.clone();
    let (tx, rx) = std::sync::mpsc::channel::<(u32, u32, usize)>();

    let ctx = EventCtx {
        app,
        state,
        recorder,
        tap_states: Arc::new(Mutex::new(HashMap::new())),
    };
    std::thread::spawn(move || run_event_worker(ctx, rx));

    std::thread::spawn(move || {
        HOOK_SENDER.with(|cell| *cell.borrow_mut() = Some(tx));
        unsafe {
            let hook = win32_hook::SetWindowsHookExW(
                win32_hook::WH_KEYBOARD_LL,
                keyboard_proc,
                std::ptr::null_mut(),
                0,
            );
            if hook.is_null() {
                eprintln!("[ptt] SetWindowsHookExW failed");
                ptt_running.store(false, Ordering::Release);
                return;
            }
            eprintln!("[ptt diag] WH_KEYBOARD_LL hook installed");
            pump_messages();
        }
    });
}

// WH_KEYBOARD_LL callbacks are delivered to this thread while it pumps messages;
// a blocking GetMessageW services them as they arrive and returns 0 on WM_QUIT.
#[cfg(target_os = "windows")]
unsafe fn pump_messages() {
    let mut msg: win32_hook::Msg = std::mem::zeroed();
    while win32_hook::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        win32_hook::TranslateMessage(&msg);
        win32_hook::DispatchMessageW(&msg);
    }
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
        assert!(
            msg.contains("Settings → Local Models"),
            "message was: {msg}"
        );
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

    #[test]
    fn local_readiness_parakeet_requires_all_three_files() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let models_dir = dir.path().join("models");
        fs::create_dir_all(&models_dir).unwrap();

        // Only encoder present — should still fail.
        fs::write(models_dir.join("encoder-model.int8.onnx"), b"stub").unwrap();
        assert!(local_model_readiness(dir.path(), LocalWhisperModel::Parakeet).is_err());

        // Add remaining files — should now pass.
        fs::write(models_dir.join("decoder_joint-model.int8.onnx"), b"stub").unwrap();
        fs::write(models_dir.join("vocab.txt"), b"stub").unwrap();
        fs::write(models_dir.join("nemo128.onnx"), b"stub").unwrap();
        assert!(local_model_readiness(dir.path(), LocalWhisperModel::Parakeet).is_ok());
    }
}
