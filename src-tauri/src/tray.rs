#[cfg(target_os = "macos")]
use libc;
#[cfg(target_os = "macos")]
use tauri::menu::MenuItemKind;
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const MAIN_LABEL: &str = "main";
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-icon@2x.png");
const SETTINGS_MENU_ID: &str = "open_settings";
const QUIT_MENU_ID: &str = "quit";

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(TRAY_ICON_BYTES)?;

    let open_item = MenuItemBuilder::with_id(SETTINGS_MENU_ID, "Open Settings").build(app)?;
    let quit_item = MenuItemBuilder::with_id(QUIT_MENU_ID, "Quit Whispr").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &quit_item])
        .build()?;

    TrayIconBuilder::with_id("whispr")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Whispr")
        .on_menu_event(|app, event| match event.id.as_ref() {
            SETTINGS_MENU_ID => show_main(app),
            QUIT_MENU_ID => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click toggles the settings window. Right-click falls
            // through to the native menu because show_menu_on_left_click
            // is false.
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_main(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// The standard macOS application menu's Quit item is wired to AppKit's
/// `terminate:`, which calls `exit()` and runs ggml's Metal static destructor —
/// that aborts (see [`quit`]). tao installs no `applicationShouldTerminate:`
/// hook, so this never surfaces as a Tauri `RunEvent` we could intercept. Build
/// the default menu, then swap that predefined Quit for a Cmd+Q item routed
/// through our `_exit(0)` path. Non-macOS keeps the default menu unchanged.
pub fn build_app_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    #[cfg(target_os = "macos")]
    replace_quit_with_safe_exit(app, &menu)?;
    Ok(menu)
}

#[cfg(target_os = "macos")]
fn replace_quit_with_safe_exit(app: &AppHandle, menu: &Menu<tauri::Wry>) -> tauri::Result<()> {
    let Some(MenuItemKind::Submenu(app_submenu)) = menu.items()?.into_iter().next() else {
        return Ok(());
    };
    // The predefined Quit is the app submenu's trailing item; drop it before
    // appending our replacement so Cmd+Q binds to ours, not terminate:.
    if let Some(MenuItemKind::Predefined(predefined_quit)) = app_submenu.items()?.into_iter().last()
    {
        app_submenu.remove(&predefined_quit)?;
    }
    let quit_item = MenuItemBuilder::with_id(QUIT_MENU_ID, "Quit Whispr")
        .accelerator("Cmd+Q")
        .build(app)?;
    app_submenu.append(&quit_item)
}

pub fn on_app_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id.as_ref() == QUIT_MENU_ID {
        quit(app);
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(target_os = "macos", allow(unused_variables))]
fn quit(app: &AppHandle) {
    // ggml's Metal device destructor (ggml_metal_rsets_free) aborts when run
    // via std::process::exit's C++ static-destructor pass. _exit(0) terminates
    // immediately with a clean exit code, bypassing those destructors entirely.
    // This is macOS-only; elsewhere app.exit(0) runs Tauri's normal shutdown
    // lifecycle (RunEvent::ExitRequested / Exit).
    #[cfg(target_os = "macos")]
    unsafe {
        libc::_exit(0);
    }
    #[cfg(not(target_os = "macos"))]
    app.exit(0);
}
