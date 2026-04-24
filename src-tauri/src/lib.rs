/// Chatty lifecycle / hot-path log. Becomes a no-op in release builds so we
/// don't write user content (transcripts, keystroke metadata) to stdout in
/// shipped binaries. Use `eprintln!` directly for genuine error reporting.
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        { println!($($arg)*); }
    }};
}

mod commands;
mod config;
mod history;
mod permissions;
mod state;
mod transcription;

// Modules that wrap macOS-only APIs (CGEventTap, CGEventPost, CoreAudio via
// cpal, transparent overlay windows via macOSPrivateApi). Cross-platform
// ports live behind the same module names inside the cfg gates below.
#[cfg(target_os = "macos")]
mod overlay;
#[cfg(target_os = "macos")]
mod paste;
#[cfg(target_os = "macos")]
mod ptt;
#[cfg(target_os = "macos")]
mod recorder;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Triggers the macOS Accessibility prompt on first launch so the
            // user isn't left wondering why PTT silently does nothing. No-op
            // on other platforms.
            permissions::ensure_accessibility_trust();

            let settings = config::load(&app.handle());
            let app_state = AppState::default();
            *app_state.shortcut.lock().unwrap() = settings.shortcut;
            *app_state.input_device.lock().unwrap() = settings.input_device;

            #[cfg(target_os = "macos")]
            {
                let recorder = recorder::Recorder::spawn();
                ptt::start(app.handle().clone(), app_state.clone(), recorder);
                if let Err(e) = overlay::create(&app.handle()) {
                    eprintln!("Failed to create overlay window: {e}");
                }
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
            commands::get_history,
            commands::clear_history,
            commands::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
