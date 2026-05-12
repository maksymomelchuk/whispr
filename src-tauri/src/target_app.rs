//! Snapshot the frontmost app at PTT press so the overlay can show whose
//! window the dictated text will land in. A short System Events probe gets
//! the bundle id; a cache hit emits immediately, a miss falls through to a
//! second AppleScriptObjC pass that renders the icon via NSWorkspace. Both
//! passes run on a blocking worker — the CGEventTap callback must never
//! wait for osascript.

use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const TARGET_APP_EVENT: &str = "target-app";

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

/// `setSize:` on the source NSImage is only a hint, so we render explicitly
/// into a fresh 64×64 bitmap rep — otherwise TIFFRepresentation hands back
/// the multi-megabyte 1024px source.
const ICON_SCRIPT: &str = r#"use framework "AppKit"
use scripting additions
on run argv
  set bid to item 1 of argv
  set ws to current application's NSWorkspace's sharedWorkspace
  set appURL to ws's URLForApplicationWithBundleIdentifier:bid
  if appURL is missing value then return ""
  set appPath to (appURL's |path|() as text)
  set img to ws's iconForFile:appPath
  if img is missing value then return ""

  set bm to current application's NSBitmapImageRep's alloc()'s initWithBitmapDataPlanes:(missing value) pixelsWide:64 pixelsHigh:64 bitsPerSample:8 samplesPerPixel:4 hasAlpha:true isPlanar:false colorSpaceName:(current application's NSDeviceRGBColorSpace) bytesPerRow:0 bitsPerPixel:0
  bm's setSize:(current application's NSMakeSize(64, 64))

  set ctx to current application's NSGraphicsContext's graphicsContextWithBitmapImageRep:bm
  current application's NSGraphicsContext's saveGraphicsState()
  current application's NSGraphicsContext's setCurrentContext:ctx
  img's drawInRect:(current application's NSMakeRect(0, 0, 64, 64)) fromRect:(current application's NSZeroRect) operation:2 fraction:1.0
  current application's NSGraphicsContext's restoreGraphicsState()

  set png to (bm's representationUsingType:4 |properties|:(missing value))
  return ((png's base64EncodedStringWithOptions:0) as text)
end run
"#;

/// Safe to call from the CGEventTap callback — all work runs on a blocking
/// worker thread.
pub fn capture(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let (bundle_id, name) = match resolve_bundle() {
            Some(b) => b,
            None => return,
        };

        if let Some(cached) = cache().lock().unwrap().get(&bundle_id).cloned() {
            let _ = app.emit(TARGET_APP_EVENT, &cached);
            return;
        }

        let icon_data_url = match resolve_icon(&bundle_id) {
            Some(i) => i,
            None => return,
        };
        let target = TargetApp {
            bundle_id: bundle_id.clone(),
            name,
            icon_data_url,
        };
        cache().lock().unwrap().insert(bundle_id, target.clone());
        let _ = app.emit(TARGET_APP_EVENT, &target);
    });
}

fn resolve_bundle() -> Option<(String, String)> {
    let output = run_osascript(BUNDLE_SCRIPT, &[])?;
    let mut parts = output.splitn(2, "|||");
    let bundle_id = parts.next()?.trim().to_string();
    let name = parts.next()?.trim().to_string();
    if bundle_id.is_empty() {
        return None;
    }
    Some((bundle_id, name))
}

fn resolve_icon(bundle_id: &str) -> Option<String> {
    let b64 = run_osascript(ICON_SCRIPT, &[bundle_id])?;
    if b64.is_empty() {
        return None;
    }
    Some(format!("data:image/png;base64,{b64}"))
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
