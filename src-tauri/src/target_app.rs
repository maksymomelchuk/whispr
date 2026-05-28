//! Snapshot the frontmost app at PTT press so the overlay can show whose
//! window the dictated text will land in. A short System Events probe gets
//! the bundle id; a cache hit emits immediately, a miss falls through to a
//! native NSWorkspace call that renders the icon into a 64×64 bitmap.
//! Both passes run on a blocking worker — the CGEventTap callback must never
//! block.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

const TARGET_APP_EVENT: &str = "target-app";

/// The frontmost app at PTT-down, resolved by osascript. Plumbed from the
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
    icon_data_url: String,
}

fn cache() -> &'static Mutex<HashMap<String, TargetApp>> {
    static CACHE: OnceLock<Mutex<HashMap<String, TargetApp>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const BUNDLE_SCRIPT: &str = r#"tell application "System Events"
  set p to first application process whose frontmost is true
  return (bundle identifier of p) & "|||" & (name of p)
end tell
"#;


/// Safe to call from the CGEventTap callback — all work runs on a blocking
/// worker thread. Also wires up a oneshot so the session task can record the
/// resolved app in the history entry without a second osascript call.
pub fn capture(app: AppHandle) {
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

        if let Some(cached) = cache().lock().unwrap().get(&frontmost.bundle_id).cloned() {
            let _ = app.emit(TARGET_APP_EVENT, &cached);
            return;
        }

        let Some(icon_data_url) = resolve_icon(&frontmost.bundle_id) else {
            return;
        };
        let target = TargetApp {
            bundle_id: frontmost.bundle_id.clone(),
            name: frontmost.name,
            icon_data_url,
        };
        cache().lock().unwrap().insert(frontmost.bundle_id, target.clone());
        let _ = app.emit(TARGET_APP_EVENT, &target);
    });
}

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

        let size = NSSize { width: 64.0, height: 64.0 };
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
        let png_data = bitmap_rep.representationUsingType_properties(
            NSBitmapImageFileType::PNG,
            &props,
        )?;

        let b64 = png_data.base64EncodedStringWithOptions(NSDataBase64EncodingOptions(0));
        Some(format!("data:image/png;base64,{b64}"))
    }
}

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
