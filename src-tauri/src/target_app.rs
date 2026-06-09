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
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
fn icon_cache() -> &'static Mutex<HashMap<String, TargetApp>> {
    static CACHE: OnceLock<Mutex<HashMap<String, TargetApp>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

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
        let Some((frontmost, exe_path)) = resolve_foreground_window() else {
            return;
        };

        if let Some(cached) = icon_cache()
            .lock()
            .unwrap()
            .get(&frontmost.bundle_id)
            .cloned()
        {
            let _ = app.emit(TARGET_APP_EVENT, &cached);
            return;
        }

        let icon_data_url = extract_icon_data_url(&exe_path);
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

/// Returns the frontmost window's app plus a null-terminated wide path to its
/// executable — the path is what the Shell needs to resolve the app's icon.
#[cfg(target_os = "windows")]
fn resolve_foreground_window() -> Option<(FrontmostApp, Vec<u16>)> {
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

        let path_wide = &buf[..len as usize];
        let path = OsString::from_wide(path_wide);
        let name = Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        if name.is_empty() {
            return None;
        }

        let mut path_terminated = path_wide.to_vec();
        path_terminated.push(0);

        Some((
            FrontmostApp {
                bundle_id: name.clone(),
                name,
            },
            path_terminated,
        ))
    }
}

#[cfg(target_os = "windows")]
fn extract_icon_data_url(exe_path: &[u16]) -> Option<String> {
    use base64::Engine as _;
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon;

    unsafe {
        let mut info: SHFILEINFOW = zeroed();
        let ok = SHGetFileInfoW(
            exe_path.as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || info.hIcon.is_null() {
            return None;
        }

        let png = icon_to_png(info.hIcon);
        DestroyIcon(info.hIcon);

        let png = png?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        Some(format!("data:image/png;base64,{b64}"))
    }
}

#[cfg(target_os = "windows")]
unsafe fn icon_to_png(hicon: *mut std::ffi::c_void) -> Option<Vec<u8>> {
    use std::mem::zeroed;
    use windows_sys::Win32::Graphics::Gdi::DeleteObject;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut icon_info: ICONINFO = zeroed();
    if GetIconInfo(hicon, &mut icon_info) == 0 {
        return None;
    }

    let rgba = if icon_info.hbmColor.is_null() {
        None
    } else {
        bitmap_to_rgba(icon_info.hbmColor)
    };

    if !icon_info.hbmColor.is_null() {
        DeleteObject(icon_info.hbmColor);
    }
    if !icon_info.hbmMask.is_null() {
        DeleteObject(icon_info.hbmMask);
    }

    let (width, height, pixels) = rgba?;
    encode_png(&pixels, width, height)
}

/// Reads an HBITMAP into a top-down RGBA buffer. Older icons carry no alpha
/// channel in their color bitmap, leaving it all-zero (fully transparent); in
/// that case we force opaque rather than render an invisible icon.
#[cfg(target_os = "windows")]
unsafe fn bitmap_to_rgba(hbm: *mut std::ffi::c_void) -> Option<(u32, u32, Vec<u8>)> {
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS,
    };

    let mut bmp: BITMAP = zeroed();
    let read = GetObjectW(
        hbm,
        size_of::<BITMAP>() as i32,
        &mut bmp as *mut BITMAP as *mut std::ffi::c_void,
    );
    if read == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
        return None;
    }
    let width = bmp.bmWidth;
    let height = bmp.bmHeight;

    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut pixels = vec![0u8; (width * height) as usize * 4];

    let hdc = CreateCompatibleDC(null_mut());
    if hdc.is_null() {
        return None;
    }
    let scanlines = GetDIBits(
        hdc,
        hbm,
        0,
        height as u32,
        pixels.as_mut_ptr() as *mut std::ffi::c_void,
        &mut bmi,
        DIB_RGB_COLORS,
    );
    DeleteDC(hdc);

    if scanlines == 0 {
        return None;
    }

    let has_alpha = pixels.chunks_exact(4).any(|px| px[3] != 0);
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
        if !has_alpha {
            px[3] = 0xFF;
        }
    }

    Some((width as u32, height as u32, pixels))
}

#[cfg(target_os = "windows")]
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(out)
}

// ── Linux / other ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_capture(_app: AppHandle) {}
