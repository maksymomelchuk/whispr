pub fn start(app: tauri::AppHandle, bundle_id: Option<String>) {
    platform_start(app, bundle_id);
}

#[cfg(target_os = "macos")]
fn platform_start(app: tauri::AppHandle, bundle_id: Option<String>) {
    std::thread::spawn(move || {
        macos::run_observation(app, bundle_id);
    });
}

#[cfg(not(target_os = "macos"))]
fn platform_start(_app: tauri::AppHandle, _bundle_id: Option<String>) {}

#[cfg(target_os = "macos")]
mod macos {
    use crate::{config, miner};
    use core_foundation::base::{CFRelease, CFTypeRef};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::os::raw::c_int;
    use std::ptr;

    type AXUIElementRef = CFTypeRef;
    type AXObserverRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type AXError = i32;
    const AX_ERROR_SUCCESS: AXError = 0;

    const WATCH_TIMEOUT_SECS: f64 = 90.0;
    const AX_SECURE_TEXT_FIELD_ROLE: &str = "AXSecureTextField";
    const FIELD_READ_TIMEOUT_SECS: f32 = 0.2;

    type AXObserverCallbackFn = unsafe extern "C" fn(
        AXObserverRef,
        AXUIElementRef,
        *const c_void,
        *mut c_void,
    );

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCreateApplication(pid: c_int) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> AXError;
        fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut c_int) -> AXError;
        fn AXObserverCreate(
            pid: c_int,
            callback: AXObserverCallbackFn,
            outObserver: *mut AXObserverRef,
        ) -> AXError;
        fn AXObserverAddNotification(
            observer: AXObserverRef,
            element: AXUIElementRef,
            notification: CFStringRef,
            refcon: *mut c_void,
        ) -> AXError;
        fn AXObserverGetRunLoopSource(observer: AXObserverRef) -> CFRunLoopSourceRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopRunInMode(
            mode: CFStringRef,
            seconds: f64,
            return_after_source_handled: u8,
        ) -> i32;
        fn CFRunLoopStop(rl: CFRunLoopRef);
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
        fn CFRunLoopRemoveSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    }

    struct ObserverCtx {
        run_loop_ref: CFRunLoopRef,
    }

    // The ctx pointer only crosses threads during Box::into_raw/from_raw handoff;
    // the callback always runs on the same thread as CFRunLoopRunInMode.
    unsafe impl Send for ObserverCtx {}

    unsafe extern "C" fn stop_run_loop(
        _observer: AXObserverRef,
        _element: AXUIElementRef,
        _notification: *const c_void,
        refcon: *mut c_void,
    ) {
        if !refcon.is_null() {
            let ctx = &*(refcon as *const ObserverCtx);
            CFRunLoopStop(ctx.run_loop_ref);
        }
    }

    pub fn run_observation(app: tauri::AppHandle, bundle_id: Option<String>) {
        unsafe {
            let Some((focused, snapshot)) = read_snapshot() else {
                return;
            };

            let mut pid: c_int = 0;
            if AXUIElementGetPid(focused, &mut pid) != AX_ERROR_SUCCESS || pid == 0 {
                CFRelease(focused);
                return;
            }

            let mut observer: AXObserverRef = ptr::null_mut();
            if AXObserverCreate(pid, stop_run_loop, &mut observer) != AX_ERROR_SUCCESS
                || observer.is_null()
            {
                CFRelease(focused);
                return;
            }

            let run_loop_ref = CFRunLoopGetCurrent();
            let ctx = Box::new(ObserverCtx { run_loop_ref });
            let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

            let app_element = AXUIElementCreateApplication(pid);
            if !app_element.is_null() {
                let focus_notif = CFString::from_static_string("AXFocusedUIElementChanged");
                let deactivate_notif = CFString::from_static_string("AXApplicationDeactivated");
                AXObserverAddNotification(
                    observer,
                    app_element,
                    focus_notif.as_concrete_TypeRef(),
                    ctx_ptr,
                );
                AXObserverAddNotification(
                    observer,
                    app_element,
                    deactivate_notif.as_concrete_TypeRef(),
                    ctx_ptr,
                );
                CFRelease(app_element);
            }
            let destroyed_notif = CFString::from_static_string("AXUIElementDestroyed");
            AXObserverAddNotification(
                observer,
                focused,
                destroyed_notif.as_concrete_TypeRef(),
                ctx_ptr,
            );

            let rl_source = AXObserverGetRunLoopSource(observer);
            if rl_source.is_null() {
                CFRelease(observer as *const c_void);
                CFRelease(focused);
                let _ = Box::from_raw(ctx_ptr as *mut ObserverCtx);
                return;
            }

            let mode = CFString::from_static_string("kCFRunLoopDefaultMode");
            CFRunLoopAddSource(run_loop_ref, rl_source, mode.as_concrete_TypeRef());
            CFRunLoopRunInMode(mode.as_concrete_TypeRef(), WATCH_TIMEOUT_SECS, 0);
            CFRunLoopRemoveSource(run_loop_ref, rl_source, mode.as_concrete_TypeRef());

            CFRelease(observer as *const c_void);
            let _ = Box::from_raw(ctx_ptr as *mut ObserverCtx);

            let value_attr = CFString::from_static_string("AXValue");
            let mut final_val: CFTypeRef = ptr::null();
            let err = AXUIElementCopyAttributeValue(
                focused,
                value_attr.as_concrete_TypeRef(),
                &mut final_val,
            );
            CFRelease(focused);

            if err != AX_ERROR_SUCCESS || final_val.is_null() {
                return;
            }
            let final_text =
                CFString::wrap_under_create_rule(final_val as CFStringRef).to_string();

            let candidates = miner::mine(&snapshot, &final_text);
            if candidates.is_empty() {
                return;
            }

            let now_ms = miner::now_ms();
            let _ = config::update(&app, |s| {
                miner::observe_candidates(&candidates, s, bundle_id.as_deref(), now_ms);
            });
        }
    }

    unsafe fn read_snapshot() -> Option<(AXUIElementRef, String)> {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(system_wide, FIELD_READ_TIMEOUT_SECS);

        let focused_attr = CFString::from_static_string("AXFocusedUIElement");
        let mut focused: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            focused_attr.as_concrete_TypeRef(),
            &mut focused,
        );
        CFRelease(system_wide);

        if err != AX_ERROR_SUCCESS || focused.is_null() {
            return None;
        }

        let role_attr = CFString::from_static_string("AXRole");
        let mut role_val: CFTypeRef = ptr::null();
        let role_err = AXUIElementCopyAttributeValue(
            focused,
            role_attr.as_concrete_TypeRef(),
            &mut role_val,
        );
        if role_err == AX_ERROR_SUCCESS && !role_val.is_null() {
            let role = CFString::wrap_under_create_rule(role_val as CFStringRef).to_string();
            if role == AX_SECURE_TEXT_FIELD_ROLE {
                CFRelease(focused);
                return None;
            }
        }

        let value_attr = CFString::from_static_string("AXValue");
        let mut snapshot_val: CFTypeRef = ptr::null();
        let val_err = AXUIElementCopyAttributeValue(
            focused,
            value_attr.as_concrete_TypeRef(),
            &mut snapshot_val,
        );

        if val_err != AX_ERROR_SUCCESS || snapshot_val.is_null() {
            CFRelease(focused);
            return None;
        }

        let snapshot = CFString::wrap_under_create_rule(snapshot_val as CFStringRef).to_string();
        Some((focused, snapshot))
    }
}
