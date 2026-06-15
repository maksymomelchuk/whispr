pub const LEARNED_UPDATED_EVENT: &str = "learned-updated";

pub fn start(app: tauri::AppHandle, bundle_id: Option<String>) {
    platform_start(app, bundle_id);
}

#[cfg(target_os = "macos")]
fn platform_start(app: tauri::AppHandle, bundle_id: Option<String>) {
    std::thread::spawn(move || {
        macos::run_observation(app, bundle_id);
    });
}

#[cfg(target_os = "windows")]
fn platform_start(app: tauri::AppHandle, bundle_id: Option<String>) {
    std::thread::spawn(move || {
        windows_impl::run_observation(app, bundle_id);
    });
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_start(_app: tauri::AppHandle, _bundle_id: Option<String>) {}

#[cfg(target_os = "windows")]
mod windows_impl {
    use crate::{config, miner};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use tauri::Emitter;
    use windows::core::implement;
    use windows::core::BSTR;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
        IUIAutomationFocusChangedEventHandler_Impl, UIA_IsPasswordPropertyId,
        UIA_ValueValuePropertyId,
    };

    const WATCH_TIMEOUT: Duration = Duration::from_secs(30);
    // Let the synthesized paste keystroke land before snapshotting; otherwise the
    // baseline is the pre-paste field and every later read looks like a full rewrite.
    const PASTE_SETTLE: Duration = Duration::from_millis(400);
    const SLICE: Duration = Duration::from_millis(500);
    const IDLE_SETTLE: Duration = Duration::from_secs(3);

    #[implement(IUIAutomationFocusChangedEventHandler)]
    struct FocusChangedHandler(mpsc::SyncSender<()>);

    impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
        fn HandleFocusChangedEvent(
            &self,
            _sender: Option<&IUIAutomationElement>,
        ) -> windows::core::Result<()> {
            let _ = self.0.send(());
            Ok(())
        }
    }

    pub fn run_observation(app: tauri::AppHandle, bundle_id: Option<String>) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let uia: IUIAutomation =
                match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
                    Ok(u) => u,
                    Err(_) => return,
                };

            std::thread::sleep(PASTE_SETTLE);

            let (element, snapshot) = match snapshot_focused_field(&uia) {
                Some(pair) => pair,
                None => return,
            };

            // Focus-change fires on a COM thread-pool thread; it only nudges the
            // poll loop to re-check, so an app switch captures without waiting out
            // the idle window.
            let (focus_tx, focus_rx) = mpsc::sync_channel::<()>(4);
            let focus_handler: IUIAutomationFocusChangedEventHandler =
                FocusChangedHandler(focus_tx).into();
            let handler_added = uia
                .AddFocusChangedEventHandler(None, &focus_handler)
                .is_ok();

            let started = Instant::now();
            let mut last_value = snapshot.clone();
            let mut last_change = started;
            loop {
                if started.elapsed() >= WATCH_TIMEOUT {
                    break;
                }
                // User switched focus/app: capture now if they actually edited.
                if focus_rx.try_recv().is_ok() {
                    if let Some(current) = read_current_value(&element) {
                        if current != snapshot {
                            break;
                        }
                    }
                }
                if let Some(current) = read_current_value(&element) {
                    if current != last_value {
                        last_value = current;
                        last_change = Instant::now();
                    }
                }
                if last_value != snapshot && last_change.elapsed() >= IDLE_SETTLE {
                    break;
                }
                std::thread::sleep(SLICE);
            }

            if handler_added {
                let _ = uia.RemoveFocusChangedEventHandler(&focus_handler);
            }

            let final_text = match read_current_value(&element) {
                Some(t) => t,
                None => return,
            };

            let candidates = miner::mine(&snapshot, &final_text);
            if candidates.is_empty() {
                return;
            }

            let now_ms = miner::now_ms();
            let _ = config::update(&app, |s| {
                miner::observe_candidates(&candidates, s, bundle_id.as_deref(), now_ms);
            });
            let _ = app.emit(super::LEARNED_UPDATED_EVENT, ());
        }
    }

    unsafe fn snapshot_focused_field(
        uia: &IUIAutomation,
    ) -> Option<(IUIAutomationElement, String)> {
        let cache_req = uia.CreateCacheRequest().ok()?;
        cache_req.AddProperty(UIA_IsPasswordPropertyId).ok()?;
        cache_req.AddProperty(UIA_ValueValuePropertyId).ok()?;

        let element = uia.GetFocusedElementBuildCache(&cache_req).ok()?;

        let pw_var = element
            .GetCachedPropertyValue(UIA_IsPasswordPropertyId)
            .ok()?;
        // Fail-closed: skip unless we can confirm the field is not a password field.
        if bool::try_from(&pw_var).unwrap_or(true) {
            return None;
        }

        let val_var = element
            .GetCachedPropertyValue(UIA_ValueValuePropertyId)
            .ok()?;
        let snapshot = BSTR::try_from(&val_var).ok()?.to_string();

        Some((element, snapshot))
    }

    unsafe fn read_current_value(element: &IUIAutomationElement) -> Option<String> {
        let var = element
            .GetCurrentPropertyValue(UIA_ValueValuePropertyId)
            .ok()?;
        let text = BSTR::try_from(&var).ok()?.to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use crate::{config, miner};
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::os::raw::c_int;
    use std::ptr;
    use std::time::{Duration, Instant};
    use tauri::Emitter;

    type AXUIElementRef = CFTypeRef;
    type AXObserverRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type AXError = i32;
    const AX_ERROR_SUCCESS: AXError = 0;

    const WATCH_TIMEOUT_SECS: f64 = 30.0;
    // Let the synthesized paste keystroke land before snapshotting; otherwise the
    // baseline is the pre-paste field and every later read looks like a full rewrite.
    const PASTE_SETTLE: Duration = Duration::from_millis(400);
    const SLICE_SECS: f64 = 0.5;
    const IDLE_SETTLE_MS: u64 = 3000;
    const VALUE_CHANGED_NOTIF: &str = "AXValueChanged";
    const AX_SECURE_TEXT_FIELD_ROLE: &str = "AXSecureTextField";
    const FIELD_READ_TIMEOUT_SECS: f32 = 0.2;

    type AXObserverCallbackFn =
        unsafe extern "C" fn(AXObserverRef, AXUIElementRef, *const c_void, *mut c_void);

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
        focused: AXUIElementRef,
        snapshot: String,
        started: Instant,
        last_value: RefCell<String>,
        last_change_ms: Cell<u64>,
        stop_requested: Cell<bool>,
    }

    // The ctx pointer only crosses threads during Box::into_raw/from_raw handoff;
    // the callback always runs on the same thread as CFRunLoopRunInMode.
    unsafe impl Send for ObserverCtx {}

    unsafe extern "C" fn observer_callback(
        _observer: AXObserverRef,
        _element: AXUIElementRef,
        notification: *const c_void,
        refcon: *mut c_void,
    ) {
        if refcon.is_null() {
            return;
        }
        let ctx = &*(refcon as *const ObserverCtx);
        let name = if notification.is_null() {
            String::new()
        } else {
            CFString::wrap_under_get_rule(notification as CFStringRef).to_string()
        };

        if name == VALUE_CHANGED_NOTIF {
            if let Some(current) = read_value(ctx.focused) {
                let changed = { *ctx.last_value.borrow() != current };
                if changed {
                    ctx.last_change_ms
                        .set(ctx.started.elapsed().as_millis() as u64);
                    *ctx.last_value.borrow_mut() = current;
                }
            }
            return;
        }

        // Focus moved away / app deactivated / field destroyed: the user is done.
        // Stop only if they actually edited — guards against a stray settle event.
        if read_value(ctx.focused).as_deref() != Some(ctx.snapshot.as_str()) {
            ctx.stop_requested.set(true);
            CFRunLoopStop(ctx.run_loop_ref);
        }
    }

    unsafe fn read_value(element: AXUIElementRef) -> Option<String> {
        let value_attr = CFString::from_static_string("AXValue");
        let mut val: CFTypeRef = ptr::null();
        let err =
            AXUIElementCopyAttributeValue(element, value_attr.as_concrete_TypeRef(), &mut val);
        if err != AX_ERROR_SUCCESS || val.is_null() {
            return None;
        }
        Some(CFString::wrap_under_create_rule(val as CFStringRef).to_string())
    }

    pub fn run_observation(app: tauri::AppHandle, bundle_id: Option<String>) {
        unsafe {
            std::thread::sleep(PASTE_SETTLE);

            let Some((focused, snapshot)) = read_snapshot() else {
                return;
            };

            let mut pid: c_int = 0;
            if AXUIElementGetPid(focused, &mut pid) != AX_ERROR_SUCCESS || pid == 0 {
                CFRelease(focused);
                return;
            }

            let mut observer: AXObserverRef = ptr::null_mut();
            if AXObserverCreate(pid, observer_callback, &mut observer) != AX_ERROR_SUCCESS
                || observer.is_null()
            {
                CFRelease(focused);
                return;
            }

            let run_loop_ref = CFRunLoopGetCurrent();
            let ctx = Box::new(ObserverCtx {
                run_loop_ref,
                focused,
                snapshot: snapshot.clone(),
                started: Instant::now(),
                last_value: RefCell::new(snapshot.clone()),
                last_change_ms: Cell::new(0),
                stop_requested: Cell::new(false),
            });
            let ctx_ptr = Box::into_raw(ctx) as *mut c_void;
            let ctx_ref = &*(ctx_ptr as *const ObserverCtx);

            let value_notif = CFString::from_static_string(VALUE_CHANGED_NOTIF);
            AXObserverAddNotification(
                observer,
                focused,
                value_notif.as_concrete_TypeRef(),
                ctx_ptr,
            );

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

            let watch_timeout_ms = (WATCH_TIMEOUT_SECS * 1000.0) as u64;
            loop {
                if ctx_ref.started.elapsed().as_millis() as u64 >= watch_timeout_ms {
                    break;
                }
                CFRunLoopRunInMode(mode.as_concrete_TypeRef(), SLICE_SECS, 0);
                if ctx_ref.stop_requested.get() {
                    break;
                }
                // Poll fallback for apps that never post AXValueChanged.
                if let Some(current) = read_value(focused) {
                    let changed = { *ctx_ref.last_value.borrow() != current };
                    if changed {
                        ctx_ref
                            .last_change_ms
                            .set(ctx_ref.started.elapsed().as_millis() as u64);
                        *ctx_ref.last_value.borrow_mut() = current;
                    }
                }
                let edited = { *ctx_ref.last_value.borrow() != snapshot };
                if edited {
                    let idle =
                        ctx_ref.started.elapsed().as_millis() as u64 - ctx_ref.last_change_ms.get();
                    if idle >= IDLE_SETTLE_MS {
                        break;
                    }
                }
            }

            CFRunLoopRemoveSource(run_loop_ref, rl_source, mode.as_concrete_TypeRef());

            CFRelease(observer as *const c_void);
            let _ = Box::from_raw(ctx_ptr as *mut ObserverCtx);

            let final_text = read_value(focused);
            CFRelease(focused);

            let Some(final_text) = final_text else {
                return;
            };

            let candidates = miner::mine(&snapshot, &final_text);
            if candidates.is_empty() {
                return;
            }

            let now_ms = miner::now_ms();
            let _ = config::update(&app, |s| {
                miner::observe_candidates(&candidates, s, bundle_id.as_deref(), now_ms);
            });
            let _ = app.emit(super::LEARNED_UPDATED_EVENT, ());
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
        AXUIElementSetMessagingTimeout(focused, FIELD_READ_TIMEOUT_SECS);

        let role_attr = CFString::from_static_string("AXRole");
        let mut role_val: CFTypeRef = ptr::null();
        let role_err =
            AXUIElementCopyAttributeValue(focused, role_attr.as_concrete_TypeRef(), &mut role_val);
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
