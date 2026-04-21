mod commands;
mod config;
mod permissions;
mod ptt;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Must be called before rdev/device_query; triggers the macOS
            // Accessibility prompt on first launch so the user isn't left
            // wondering why PTT silently does nothing.
            permissions::ensure_accessibility_trust();

            let settings = config::load(&app.handle());
            let app_state = AppState::default();
            *app_state.shortcut.lock().unwrap() = settings.shortcut;

            ptt::start(app.handle().clone(), app_state.clone());
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_api_key,
            commands::set_shortcut,
            commands::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
