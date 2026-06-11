use tauri::Manager;

const CAPTURE_TIMEOUT_SECS: f32 = 0.05;

pub fn capture(app: tauri::AppHandle) {
    use crate::state::AppState;
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.state::<AppState>()
        .pending_selected_text_rx
        .lock()
        .unwrap() = Some(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = tx.send(platform_read_selected_text());
    });
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn platform_read_selected_text() -> Option<String> {
    use core_foundation::base::{CFRelease, CFTypeRef};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ptr;

    type AXUIElementRef = CFTypeRef;
    type AXError = i32;
    const AX_ERROR_SUCCESS: AXError = 0;
    const AX_SECURE_TEXT_FIELD_ROLE: &str = "AXSecureTextField";

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetMessagingTimeout(
            element: AXUIElementRef,
            timeout_in_seconds: f32,
        ) -> AXError;
    }

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        // Bound all AX calls from this process to 50 ms so an unresponsive
        // target app cannot stall the session.
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

        let sel_attr = CFString::from_static_string("AXSelectedText");
        let mut sel_value: CFTypeRef = ptr::null();
        let sel_err = AXUIElementCopyAttributeValue(
            focused_element,
            sel_attr.as_concrete_TypeRef(),
            &mut sel_value,
        );
        CFRelease(focused_element);

        if sel_err != AX_ERROR_SUCCESS || sel_value.is_null() {
            return None;
        }

        let selected = CFString::wrap_under_create_rule(sel_value as CFStringRef);
        let text = selected.to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn platform_read_selected_text() -> Option<String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VT_BOOL;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_IsPasswordPropertyId,
        UIA_TextPatternId,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let uia: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let cache_req = uia.CreateCacheRequest().ok()?;
        cache_req.AddProperty(UIA_IsPasswordPropertyId).ok()?;
        cache_req.AddPattern(UIA_TextPatternId).ok()?;

        let element = uia.GetFocusedElementBuildCache(&cache_req).ok()?;

        let pw_variant = element.GetCachedPropertyValue(UIA_IsPasswordPropertyId).ok()?;
        let pw_inner = &*pw_variant.0.Anonymous;
        if pw_inner.vt == VT_BOOL && pw_inner.Anonymous.boolVal.0 != 0 {
            return None;
        }

        let pattern = element.GetCachedPattern(UIA_TextPatternId).ok()?;
        let text_pattern: IUIAutomationTextPattern = pattern.cast().ok()?;
        let ranges = text_pattern.GetSelection().ok()?;
        if ranges.Length().ok()? == 0 {
            return None;
        }
        let range = ranges.GetElement(0).ok()?;
        let bstr = range.GetText(-1).ok()?;
        let s = bstr.to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

// ── Linux (no-op) ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_read_selected_text() -> Option<String> {
    None
}
