mod commands;
mod config;
mod history;
mod permissions;
mod state;
mod stats;
mod tray;

#[cfg(target_os = "macos")]
mod transcription_stream;

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
mod recorder;

use state::AppState;
use tauri::{Manager, WindowEvent};

const MAIN_LABEL: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .on_window_event(|window, event| {
            // The settings window's red X should hide the app rather than
            // destroy the instance — the tray icon re-shows the same
            // window in place. Cmd+Q still routes through the default
            // macOS menu (app.exit), which does not trigger this event.
            if window.label() == MAIN_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            // Run as a menu bar app: no Dock icon, no Cmd+Tab entry. Set at
            // runtime rather than relying solely on Info.plist's LSUIElement
            // because `tauri dev` launches the raw binary and never reads
            // the bundle plist.
            #[cfg(target_os = "macos")]
            {
                let _ = app.set_activation_policy(
                    tauri::ActivationPolicy::Accessory,
                );
            }

            // Triggers the macOS Accessibility prompt on first launch so the
            // user isn't left wondering why PTT silently does nothing. No-op
            // on other platforms.
            permissions::ensure_accessibility_trust();

            let settings = config::load(&app.handle());
            let app_state = AppState::default();
            *app_state.shortcut.lock().unwrap() = settings.shortcut;
            *app_state.input_device.lock().unwrap() = settings.input_device;
            *app_state.pause_media_on_record.lock().unwrap() =
                settings.pause_media_on_record;

            #[cfg(target_os = "macos")]
            {
                let recorder = recorder::Recorder::spawn();
                ptt::start(app.handle().clone(), app_state.clone(), recorder);
                if let Err(e) = overlay::create(&app.handle()) {
                    eprintln!("Failed to create overlay window: {e}");
                }
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
            commands::set_api_key,
            commands::set_shortcut,
            commands::set_replacements,
            commands::set_deepgram_settings,
            commands::list_input_devices,
            commands::set_input_device,
            commands::set_pause_media_on_record,
            commands::get_history,
            commands::clear_history,
            commands::set_history_limit,
            commands::get_stats,
            commands::clear_stats,
            commands::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
