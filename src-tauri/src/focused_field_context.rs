use tauri::Manager;

// Allow slightly more time than selected_text: Chromium/Electron apps need
// to build their accessibility tree on the first query, which typically adds
// 50–150 ms on top of a normal AX round-trip.
const CAPTURE_TIMEOUT_SECS: f32 = 0.2;

pub fn capture(app: tauri::AppHandle) {
    use crate::state::AppState;
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.state::<AppState>()
        .pending_focused_field_rx
        .lock()
        .unwrap() = Some(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = tx.send(platform_read_focused_field());
    });
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn platform_read_focused_field() -> Option<String> {
    use core_foundation::base::{CFRelease, CFTypeRef};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};
    use std::os::raw::c_int;
    use std::ptr;

    type AXUIElementRef = CFTypeRef;
    type AXError = i32;
    const AX_ERROR_SUCCESS: AXError = 0;
    const AX_SECURE_TEXT_FIELD_ROLE: &str = "AXSecureTextField";

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCreateApplication(pid: c_int) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetMessagingTimeout(
            element: AXUIElementRef,
            timeout_in_seconds: f32,
        ) -> AXError;
        fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut c_int) -> AXError;
    }

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(system_wide, CAPTURE_TIMEOUT_SECS);

        let focused_attr = CFString::from_static_string("AXFocusedUIElement");
        let mut focused_element: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            focused_attr.as_concrete_TypeRef(),
            &mut focused_element,
        );
        CFRelease(system_wide);

        if err != AX_ERROR_SUCCESS || focused_element.is_null() {
            return None;
        }

        // Pre-warm the Chromium/Electron accessibility tree via the app-level
        // AXEnhancedUserInterface attribute. On non-Chromium apps this is a
        // harmless no-op. On Chromium/Electron the first set triggers async
        // tree construction; subsequent reads land after it completes.
        let mut pid: c_int = 0;
        if AXUIElementGetPid(focused_element, &mut pid) == AX_ERROR_SUCCESS && pid > 0 {
            let app_element = AXUIElementCreateApplication(pid);
            if !app_element.is_null() {
                let enhanced_attr =
                    CFString::from_static_string("AXEnhancedUserInterface");
                let true_val = CFBoolean::true_value();
                // Ignore result: not all apps support this attribute.
                let _ = AXUIElementSetAttributeValue(
                    app_element,
                    enhanced_attr.as_concrete_TypeRef(),
                    true_val.as_CFTypeRef(),
                );
                CFRelease(app_element);
            }
        }

        let role_attr = CFString::from_static_string("AXRole");
        let mut role_value: CFTypeRef = ptr::null();
        let role_err = AXUIElementCopyAttributeValue(
            focused_element,
            role_attr.as_concrete_TypeRef(),
            &mut role_value,
        );
        if role_err == AX_ERROR_SUCCESS && !role_value.is_null() {
            let role = CFString::wrap_under_create_rule(role_value as CFStringRef);
            if role.to_string() == AX_SECURE_TEXT_FIELD_ROLE {
                CFRelease(focused_element);
                return None;
            }
        }

        let value_attr = CFString::from_static_string("AXValue");
        let mut field_value: CFTypeRef = ptr::null();
        let val_err = AXUIElementCopyAttributeValue(
            focused_element,
            value_attr.as_concrete_TypeRef(),
            &mut field_value,
        );
        CFRelease(focused_element);

        if val_err != AX_ERROR_SUCCESS || field_value.is_null() {
            return None;
        }

        let text = CFString::wrap_under_create_rule(field_value as CFStringRef).to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn platform_read_focused_field() -> Option<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::{VT_BOOL, VT_BSTR};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, UIA_IsPasswordPropertyId, UIA_ValueValuePropertyId,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let uia: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let cache_req = uia.CreateCacheRequest().ok()?;
        cache_req.AddProperty(UIA_IsPasswordPropertyId).ok()?;
        cache_req.AddProperty(UIA_ValueValuePropertyId).ok()?;

        let element = uia.GetFocusedElementBuildCache(&cache_req).ok()?;

        let pw_variant = element.GetCachedPropertyValue(UIA_IsPasswordPropertyId).ok()?;
        let pw_inner = &*pw_variant.0.Anonymous;
        if pw_inner.vt == VT_BOOL && pw_inner.Anonymous.boolVal.0 != 0 {
            return None;
        }

        let val_variant = element.GetCachedPropertyValue(UIA_ValueValuePropertyId).ok()?;
        let inner = &*val_variant.0.Anonymous;
        if inner.vt != VT_BSTR {
            return None;
        }
        let text = (&*inner.Anonymous.bstrVal).to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

// ── Linux (no-op) ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_read_focused_field() -> Option<String> {
    None
}
