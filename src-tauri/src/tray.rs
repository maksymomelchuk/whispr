use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const MAIN_LABEL: &str = "main";
const TRAY_ICON_BYTES: &[u8] =
    include_bytes!("../icons/tray-icon@2x.png");

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(TRAY_ICON_BYTES)?;

    let open_item =
        MenuItemBuilder::with_id("open_settings", "Open Settings").build(app)?;
    let quit_item =
        MenuItemBuilder::with_id("quit", "Quit Whispr").build(app)?;
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
            "open_settings" => show_main(app),
            "quit" => app.exit(0),
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
