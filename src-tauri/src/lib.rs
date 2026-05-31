mod api_key_validation;
mod commands;
pub mod config;
pub mod download;
pub mod history;
pub mod hotkey;
pub mod keysym;
pub mod mode;
pub mod provider;
pub mod pipeline;
// corrections helpers are consumed by pipeline (cross-platform) and by
// macOS-only session modules; allow unused items on non-macOS so the
// module still ships and its tests run.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod corrections;
// terms helpers are consumed by macOS-only session modules; allow
// unused items on non-macOS so the module still ships and its tests run.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub mod terms;
// groq_audio is only used by groq_session (macOS-gated); allow unused
// items on non-macOS builds so the module still ships and its tests
// run, but the binary doesn't warn about dead code.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod groq_audio;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub mod groq_session_state;
mod groq_stabilizer;
mod permissions;
// snippets helpers are consumed by pipeline (cross-platform) and by
// macOS-only ptt; allow unused items on non-macOS so the module still
// ships and its tests run.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod snippets;
mod state;
mod stats;
mod tray;

#[cfg(target_os = "macos")]
mod cleanup;
#[cfg(target_os = "macos")]
mod cleanup_stats;
#[cfg(target_os = "macos")]
mod deepgram_session;
#[cfg(target_os = "macos")]
mod assemblyai_session;
#[cfg(target_os = "macos")]
mod groq_session;
#[cfg(target_os = "macos")]
mod local_session;
#[cfg(target_os = "macos")]
pub mod transcription_session;

// Modules that wrap macOS-only APIs (CGEventTap, CGEventPost, CoreAudio via
// cpal, transparent overlay windows via macOSPrivateApi). Cross-platform
// ports live behind the same module names inside the cfg gates below.
#[cfg(target_os = "macos")]
mod media;
#[cfg(target_os = "macos")]
mod overlay;
#[cfg(target_os = "macos")]
mod paste;
#[cfg(target_os = "macos")]
mod ptt;
#[cfg(target_os = "macos")]
pub mod recorder;
#[cfg(target_os = "macos")]
mod target_app;

use state::AppState;
use tauri::{Manager, WindowEvent};

const MAIN_LABEL: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .on_window_event(|window, event| {
            if window.label() != MAIN_LABEL {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .setup(|app| {
            // Run as a menu bar app by default: no Dock icon, no Cmd+Tab
            // entry. The user can opt back into a regular app presence via
            // the settings toggle. Set at runtime rather than relying solely
            // on Info.plist's LSUIElement because `tauri dev` launches the
            // raw binary and never reads the bundle plist.
            #[cfg(target_os = "macos")]
            {
                let policy = if config::load(&app.handle()).show_in_dock {
                    tauri::ActivationPolicy::Regular
                } else {
                    tauri::ActivationPolicy::Accessory
                };
                let _ = app.set_activation_policy(policy);
            }

            // Triggers the macOS Accessibility prompt on first launch so the
            // user isn't left wondering why PTT silently does nothing. No-op
            // on other platforms.
            permissions::ensure_accessibility_trust();
            // Prompts via AVFoundation so AVCaptureDevice's status cache
            // tracks the grant — cpal's CoreAudio prompt doesn't refresh it.
            permissions::ensure_microphone_trust();

            let settings = config::load(&app.handle());

            // Reconcile the OS-level Login Item with our persisted intent. The
            // user may have toggled it directly in System Settings; settings.json
            // is the source of truth.
            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.handle().autolaunch();
                let result = if settings.start_at_login {
                    manager.enable()
                } else {
                    manager.disable()
                };
                if let Err(e) = result {
                    eprintln!("Failed to reconcile autostart state: {e}");
                }
            }

            let app_state = AppState::default();
            *app_state.hotkey_bindings.lock().unwrap() = settings.hotkey_bindings;
            *app_state.input_device.lock().unwrap() = settings.input_device;
            *app_state.pause_media_on_record.lock().unwrap() =
                settings.pause_media_on_record;

            #[cfg(target_os = "macos")]
            {
                let recorder = recorder::Recorder::spawn();
                *app_state.recorder.lock().unwrap() = Some(recorder.clone());
                // CGEventTapCreate without Accessibility returns a tap that
                // only sees own-process events; that crippled state sticks
                // until relaunch. Defer creation until permission lands.
                if permissions::check_accessibility_permission() {
                    app_state
                        .ptt_running
                        .store(true, std::sync::atomic::Ordering::Release);
                    ptt::start(app.handle().clone(), app_state.clone(), recorder);
                }
                if let Err(e) = overlay::create(&app.handle()) {
                    eprintln!("Failed to create overlay window: {e}");
                }

                let eviction_cache = app_state.model_cache.clone();
                let eviction_app = app.handle().clone();
                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(60));
                        let idle_timeout = config::load(&eviction_app).local_whisper.idle_timeout;
                        let Some(threshold) = idle_timeout.as_duration() else {
                            continue;
                        };
                        eviction_cache.lock().unwrap().retain(|_, m| m.last_used.elapsed() < threshold);
                    }
                });
            }

            if let Err(e) = tray::setup(app.handle()) {
                eprintln!("Failed to create tray icon: {e}");
            }

            // LSUIElement apps don't foreground themselves on launch, so
            // the settings window can render behind whatever the user was
            // doing. Explicitly focus it.
            if let Some(window) = app.get_webview_window(MAIN_LABEL) {
                let _ = window.set_focus();
            }
            #[cfg(not(target_os = "macos"))]
            {
                eprintln!(
                    "[whispr] push-to-talk / audio capture / paste are not yet implemented \
                     on this platform; UI will run but dictation is disabled."
                );
            }

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_deepgram_api_key,
            commands::set_groq_api_key,
            commands::set_assemblyai_api_key,
            commands::validate_deepgram_api_key,
            commands::validate_groq_api_key,
            commands::validate_assemblyai_api_key,
            commands::set_hotkey_bindings,
            commands::set_shortcut_capture_paused,
            commands::create_term_set,
            commands::rename_term_set,
            commands::update_term_set_entries,
            commands::delete_term_set,
            commands::add_correction_set,
            commands::update_correction_set,
            commands::delete_correction_set,
            commands::set_snippets,
            commands::add_mode,
            commands::update_mode,
            commands::delete_mode,
            commands::duplicate_mode,
            commands::set_default_mode,
            commands::set_anthropic_api_key,
            commands::set_anthropic_oauth_token,
            commands::set_cleanup_auth_mode,
            commands::set_cleanup_thresholds,
            commands::list_input_devices,
            commands::set_input_device,
            commands::set_pause_media_on_record,
            commands::set_show_in_dock,
            commands::set_start_at_login,
            commands::set_show_live_preview,
            commands::get_history,
            commands::clear_history,
            commands::set_history_limit,
            commands::get_stats,
            commands::clear_stats,
            commands::get_app_icon,
            commands::get_cleanup_stats,
            commands::open_accessibility_settings,
            commands::check_permissions,
            commands::open_microphone_settings,
            commands::ensure_ptt_started,
            commands::get_local_model_statuses,
            commands::start_model_download,
            commands::cancel_model_download,
            commands::delete_local_model,
            commands::get_local_model_path,
            commands::set_local_whisper_idle_timeout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
