use serde::Serialize;
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::{AppHandle, Emitter};

const TARGET_APP_EVENT: &str = "target-app";

/// The frontmost app at PTT-down, resolved per-platform. Plumbed from the
/// capture worker to the session task via a oneshot so history can attribute
/// the dictation to a specific app without a second probe.
#[derive(Debug, Clone)]
pub struct FrontmostApp {
    pub bundle_id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TargetApp {
    bundle_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_data_url: Option<String>,
}

/// Safe to call from a PTT callback — all work runs on a blocking worker thread.
/// On macOS it also wires up a oneshot so the session task can record the
/// resolved app in history without a second OS probe.
pub fn capture(app: AppHandle) {
    platform_capture(app);
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::io::Write;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "macos")]
use tokio::sync::oneshot;

#[cfg(target_os = "macos")]
fn icon_cache() -> &'static Mutex<HashMap<String, TargetApp>> {
    static CACHE: OnceLock<Mutex<HashMap<String, TargetApp>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
const BUNDLE_SCRIPT: &str = r#"tell application "System Events"
  set p to first application process whose frontmost is true
  return (bundle identifier of p) & "|||" & (name of p)
end tell
"#;

#[cfg(target_os = "macos")]
fn platform_capture(app: AppHandle) {
    use crate::state::AppState;

    let (tx, rx) = oneshot::channel();
    if let Some(state) = app.try_state::<AppState>() {
        *state.pending_app_rx.lock().unwrap() = Some(rx);
    }

    tauri::async_runtime::spawn_blocking(move || {
        let Some(frontmost) = resolve_bundle() else {
            let _ = tx.send(None);
            return;
        };

        let _ = tx.send(Some(frontmost.clone()));

        if let Some(cached) = icon_cache()
            .lock()
            .unwrap()
            .get(&frontmost.bundle_id)
            .cloned()
        {
            let _ = app.emit(TARGET_APP_EVENT, &cached);
            return;
        }

        let icon_data_url = resolve_icon(&frontmost.bundle_id);
        let target = TargetApp {
            bundle_id: frontmost.bundle_id.clone(),
            name: frontmost.name,
            icon_data_url,
        };
        icon_cache()
            .lock()
            .unwrap()
            .insert(frontmost.bundle_id, target.clone());
        let _ = app.emit(TARGET_APP_EVENT, &target);
    });
}

#[cfg(target_os = "macos")]
fn resolve_bundle() -> Option<FrontmostApp> {
    let output = run_osascript(BUNDLE_SCRIPT, &[])?;
    let mut parts = output.splitn(2, "|||");
    let bundle_id = parts.next()?.trim().to_string();
    let name = parts.next()?.trim().to_string();
    if bundle_id.is_empty() {
        return None;
    }
    Some(FrontmostApp { bundle_id, name })
}

#[cfg(target_os = "macos")]
pub fn resolve_icon(bundle_id: &str) -> Option<String> {
    use objc2::runtime::AnyObject;
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext,
        NSWorkspace,
    };
    use objc2_foundation::{
        NSDataBase64EncodingOptions, NSDictionary, NSPoint, NSRect, NSSize, NSString,
    };

    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let bundle_id_ns = NSString::from_str(bundle_id);
        let app_url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id_ns)?;
        let app_path = app_url.path()?;
        let icon = workspace.iconForFile(&app_path);

        let bitmap_rep = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            64, 64, 8, 4, true, false,
            NSDeviceRGBColorSpace,
            0, 0,
        )?;

        let size = NSSize {
            width: 64.0,
            height: 64.0,
        };
        bitmap_rep.setSize(size);

        let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_rep)?;
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&ctx));

        let rect = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size,
        };
        icon.drawInRect(rect);

        NSGraphicsContext::restoreGraphicsState_class();

        let props = NSDictionary::<NSString, AnyObject>::new();
        let png_data =
            bitmap_rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)?;

        let b64 = png_data.base64EncodedStringWithOptions(NSDataBase64EncodingOptions(0));
        Some(format!("data:image/png;base64,{b64}"))
    }
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("osascript");
    cmd.arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for a in args {
        cmd.arg(a);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| eprintln!("[target_app] osascript spawn: {e}"))
        .ok()?;
    if let Some(stdin) = child.stdin.as_mut() {
        if let Err(e) = stdin.write_all(script.as_bytes()) {
            eprintln!("[target_app] osascript stdin: {e}");
            return None;
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| eprintln!("[target_app] osascript wait: {e}"))
        .ok()?;
    if !output.status.success() {
        eprintln!(
            "[target_app] osascript failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> *mut c_void;
        pub fn GetWindowThreadProcessId(hwnd: *mut c_void, lpdw_process_id: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn OpenProcess(
            dw_desired_access: u32,
            b_inherit_handle: i32,
            dw_process_id: u32,
        ) -> *mut c_void;
        pub fn CloseHandle(h_object: *mut c_void) -> i32;
        pub fn QueryFullProcessImageNameW(
            h_process: *mut c_void,
            dw_flags: u32,
            lp_exe_name: *mut u16,
            lpdw_size: *mut u32,
        ) -> i32;
    }
}

#[cfg(target_os = "windows")]
fn platform_capture(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(frontmost) = resolve_foreground_window() else {
            return;
        };
        let target = TargetApp {
            bundle_id: frontmost.bundle_id,
            name: frontmost.name,
            icon_data_url: None,
        };
        let _ = app.emit(TARGET_APP_EVENT, &target);
    });
}

#[cfg(target_os = "windows")]
fn resolve_foreground_window() -> Option<FrontmostApp> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    unsafe {
        let hwnd = win32::GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        win32::GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let handle = win32::OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut buf = vec![0u16; 260];
        let mut len = buf.len() as u32;
        let ok = win32::QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        win32::CloseHandle(handle);

        if ok == 0 {
            return None;
        }

        let path = OsString::from_wide(&buf[..len as usize]);
        let name = Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        if name.is_empty() {
            return None;
        }

        Some(FrontmostApp {
            bundle_id: name.clone(),
            name,
        })
    }
}

// ── Linux / other ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_capture(_app: AppHandle) {}
