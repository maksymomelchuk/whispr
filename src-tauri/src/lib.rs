mod api_key_validation;
mod commands;
pub mod config;
mod corrections;
pub mod download;
mod groq_audio;
pub mod groq_session_state;
mod groq_stabilizer;
pub mod history;
pub(crate) mod hotkey;
pub(crate) mod keysym;
pub mod mode;
mod permissions;
pub mod pipeline;
#[cfg(target_os = "linux")]
pub(crate) mod platform;
pub mod provider;
mod snippets;
mod state;
mod stats;
pub mod terms;
mod tray;

// cleanup_stats is pure Rust (file I/O + token counters, no OS APIs) and
// is therefore compiled on all platforms.
mod cleanup_stats;

// Session modules depend only on tokio-tungstenite, reqwest, and cpal —
// all cross-platform. Un-gated so cloud transcription compiles everywhere.
mod assemblyai_session;
mod audio_level_meter;
mod deepgram_session;
mod elevenlabs_session;
mod engine;
mod groq_session;
mod local_engine;
mod openai_transcribe_session;
mod preview_throttle;
pub(crate) mod recorder;
mod session;

// cleanup is cross-platform HTTP (reqwest + serde_json, no OS APIs).
pub mod cleanup;
mod cleanup_invoke;
pub mod model_catalog;
pub mod recovery;

mod clipboard_context;
mod focused_field_context;
mod miner;
mod post_paste_observer;
mod selected_text_context;
pub(crate) mod selector;
mod tone;

// media, overlay, and target_app expose platform-neutral public APIs and
// select their OS implementation internally via cfg.
mod media;
mod overlay;
mod target_app;
// paste exposes a platform-neutral public API; per-OS injection is selected
// internally via cfg: CGEvent on macOS, enigo on Windows, native tools or
// enigo fallback on Linux.
#[cfg(target_os = "macos")]
mod mac_clipboard;
mod paste;
#[cfg(target_os = "windows")]
mod windows_clipboard;

// ptt compiles on all platforms; the event source is selected internally via
// cfg: CGEventTap on macOS, rdev on Windows/Linux.
mod ptt;

use state::AppState;
use tauri::{Manager, WindowEvent};

const MAIN_LABEL: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // tao registers RawInput for the app window (RIDEV_DEVNOTIFY). On
        // Windows, RawInput for the process's own window preempts the
        // WH_KEYBOARD_LL hook chain, so our global keyboard hook never fires
        // while our own window is focused. RIDEV_REMOVE unregisters it; we
        // don't consume tao DeviceEvents, so nothing is lost.
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ));

    // The default menu (File/Edit/Window/Help) is the system menu bar on macOS
    // — where it also hosts our safe Cmd+Q swap — but renders as a per-window
    // menu bar on Windows/Linux. Drop it on Windows only; macOS needs it and
    // Linux keeps its existing bar.
    #[cfg(not(target_os = "windows"))]
    let builder = builder
        .menu(tray::build_app_menu)
        .on_menu_event(tray::on_app_menu_event);

    builder
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
            clipboard_context::start_sampler(app_state.clipboard_window.clone());
            *app_state.hotkey_bindings.lock().unwrap() = settings.hotkey_bindings;
            *app_state.input_device.lock().unwrap() = settings.input_device;
            *app_state.pause_media_on_record.lock().unwrap() = settings.pause_media_on_record;

            // Spawn the audio capture thread on all platforms — cpal is
            // cross-platform and cloud engine sessions need it on Windows/Linux.
            {
                let recorder = recorder::Recorder::spawn();
                *app_state.recorder.lock().unwrap() = Some(recorder.clone());

                #[cfg(target_os = "macos")]
                {
                    // CGEventTapCreate without Accessibility returns a tap that
                    // only sees own-process events; that crippled state sticks
                    // until relaunch. Defer creation until permission lands.
                    if permissions::check_accessibility_permission() {
                        app_state
                            .ptt_running
                            .store(true, std::sync::atomic::Ordering::Release);
                        ptt::start(app.handle().clone(), app_state.clone(), recorder);
                    }
                }

                // On Windows and Linux, rdev provides global key capture with no
                // permission gate; start the listener unconditionally.
                #[cfg(not(target_os = "macos"))]
                {
                    app_state
                        .ptt_running
                        .store(true, std::sync::atomic::Ordering::Release);
                    ptt::start(app.handle().clone(), app_state.clone(), recorder);
                }
            }

            if let Err(e) = overlay::create(&app.handle()) {
                eprintln!("Failed to create overlay window: {e}");
            }

            {
                let eviction_cache = app_state.model_cache.clone();
                let eviction_app = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    let idle_timeout = config::load(&eviction_app).local_whisper.idle_timeout;
                    let Some(threshold) = idle_timeout.as_duration() else {
                        continue;
                    };
                    eviction_cache
                        .lock()
                        .unwrap()
                        .retain(|_, m| m.last_used.elapsed() < threshold);
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
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_deepgram_api_key,
            commands::set_groq_api_key,
            commands::set_assemblyai_api_key,
            commands::set_openai_api_key,
            commands::set_elevenlabs_api_key,
            commands::validate_deepgram_api_key,
            commands::validate_groq_api_key,
            commands::validate_assemblyai_api_key,
            commands::validate_openai_api_key,
            commands::validate_elevenlabs_api_key,
            commands::set_hotkey_bindings,
            commands::set_shortcut_capture_paused,
            commands::create_term_set,
            commands::rename_term_set,
            commands::update_term_set_entries,
            commands::delete_term_set,
            commands::create_correction_set,
            commands::rename_correction_set,
            commands::update_correction_set_entries,
            commands::delete_correction_set,
            commands::set_snippets,
            commands::add_mode,
            commands::update_mode,
            commands::delete_mode,
            commands::reorder_modes,
            commands::duplicate_mode,
            commands::set_anthropic_api_key,
            commands::set_anthropic_oauth_token,
            commands::set_provider_key,
            commands::clear_provider_key,
            commands::set_custom_provider,
            commands::clear_custom_provider,
            commands::set_cleanup_auth_mode,
            commands::set_cleanup_thresholds,
            commands::set_tone_overlay_enabled,
            commands::get_apps_seen_in_history,
            commands::set_tone_app_override,
            commands::set_tone_app_custom_prompt,
            commands::clear_tone_app_override,
            commands::list_input_devices,
            commands::set_input_device,
            commands::set_pause_media_on_record,
            commands::set_show_in_dock,
            commands::set_start_at_login,
            commands::set_show_live_preview,
            commands::get_history,
            commands::clear_history,
            commands::set_history_limit,
            commands::update_history_entry,
            commands::get_learned_entries,
            commands::delete_learned_entry,
            commands::promote_learned_entry,
            commands::set_learn_from_corrections,
            commands::recover_cleanup,
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
