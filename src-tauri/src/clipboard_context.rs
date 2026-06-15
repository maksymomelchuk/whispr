use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

// Copy → refocus the target field → think → press PTT routinely exceeds a
// few seconds; 3 s (superwhisper's window) loses that race.
const RECENCY_WINDOW: Duration = Duration::from_secs(10);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);
// Must exceed RECENCY_WINDOW or no baseline sample survives past the cutoff.
const SAMPLE_RETENTION: Duration = Duration::from_secs(12);

// nspasteboard.org convention; set by 1Password, Bitwarden, etc.
#[cfg(target_os = "macos")]
const CONCEALED_PASTEBOARD_TYPE: &str = "org.nspasteboard.ConcealedType";
#[cfg(target_os = "windows")]
const CONCEALED_CLIPBOARD_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";

pub type SamplerWindow = Arc<Mutex<VecDeque<(i64, Instant)>>>;

/// Only the change-count integer is stored — clipboard content is never read here.
pub fn start_sampler(window: SamplerWindow) {
    std::thread::Builder::new()
        .name("clipboard-sampler".into())
        .spawn(move || loop {
            let count = platform_change_count();
            let now = Instant::now();
            {
                let mut w = window.lock().unwrap();
                w.push_back((count, now));
                if let Some(cutoff) = now.checked_sub(SAMPLE_RETENTION) {
                    while w.front().is_some_and(|&(_, t)| t < cutoff) {
                        w.pop_front();
                    }
                }
            }
            std::thread::sleep(SAMPLE_INTERVAL);
        })
        .ok();
}

pub fn is_recent_change(window: &SamplerWindow, current_count: i64) -> bool {
    let w = window.lock().unwrap();
    if w.is_empty() {
        return false;
    }
    let Some(cutoff) = Instant::now().checked_sub(RECENCY_WINDOW) else {
        return false;
    };
    let baseline = w
        .iter()
        .rev()
        .find(|&&(_, t)| t <= cutoff)
        .or_else(|| w.front())
        .map(|&(count, _)| count)
        .unwrap_or(current_count);
    current_count != baseline
}

pub fn capture(app: tauri::AppHandle) {
    use crate::state::AppState;
    let state = app.state::<AppState>();
    let window = state.clipboard_window.clone();
    let baseline = state.clipboard_count_at_ptt_down.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    *state.pending_clipboard_rx.lock().unwrap() = Some(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let current_count = platform_change_count();
        *baseline.lock().unwrap() = Some(current_count);
        let text = if is_recent_change(&window, current_count) {
            read_text_unless_concealed()
        } else {
            None
        };
        let _ = tx.send(text);
    });
}

/// Covers text copied while the user was speaking: the PTT-down capture has
/// already missed it, so re-check the change count after STT completes.
pub async fn read_if_copied_mid_session(app: &tauri::AppHandle) -> Option<String> {
    use crate::state::AppState;
    let baseline = app
        .state::<AppState>()
        .clipboard_count_at_ptt_down
        .lock()
        .unwrap()
        .take()?;
    tauri::async_runtime::spawn_blocking(move || {
        if platform_change_count() == baseline {
            return None;
        }
        read_text_unless_concealed()
    })
    .await
    .ok()
    .flatten()
}

fn read_text_unless_concealed() -> Option<String> {
    if platform_is_concealed() {
        return None;
    }
    platform_read_text()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn platform_change_count() -> i64 {
    use objc2_app_kit::NSPasteboard;
    NSPasteboard::generalPasteboard().changeCount() as i64
}

#[cfg(target_os = "macos")]
fn platform_is_concealed() -> bool {
    use objc2_app_kit::NSPasteboard;
    let pb = NSPasteboard::generalPasteboard();
    let Some(items) = pb.pasteboardItems() else {
        return false;
    };
    items.iter().any(|item| {
        item.types()
            .iter()
            .any(|t| t.to_string() == CONCEALED_PASTEBOARD_TYPE)
    })
}

#[cfg(target_os = "macos")]
pub fn platform_read_text() -> Option<String> {
    use objc2_app_kit::NSPasteboard;
    let pb = NSPasteboard::generalPasteboard();
    let items = pb.pasteboardItems()?;
    for item in items.iter() {
        for item_type in item.types().iter() {
            if item_type.to_string() == "public.utf8-plain-text" {
                if let Some(data) = item.dataForType(&item_type) {
                    return String::from_utf8(data.to_vec()).ok();
                }
            }
        }
    }
    None
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn platform_change_count() -> i64 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
    unsafe { GetClipboardSequenceNumber() as i64 }
}

#[cfg(target_os = "windows")]
fn platform_is_concealed() -> bool {
    use windows_sys::Win32::System::DataExchange::{
        IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };
    let mut name: Vec<u16> = CONCEALED_CLIPBOARD_FORMAT.encode_utf16().collect();
    name.push(0);
    unsafe {
        let format = RegisterClipboardFormatW(name.as_ptr());
        format != 0 && IsClipboardFormatAvailable(format) != 0
    }
}

#[cfg(target_os = "windows")]
pub fn platform_read_text() -> Option<String> {
    clipboard_win::get_clipboard_string().ok()
}

// ── Linux (no-op) ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn platform_change_count() -> i64 {
    0
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_is_concealed() -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn platform_read_text() -> Option<String> {
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn window_with_samples(samples: &[(i64, Duration)]) -> SamplerWindow {
        let now = Instant::now();
        let w = Arc::new(Mutex::new(VecDeque::new()));
        for &(count, age) in samples {
            if let Some(t) = now.checked_sub(age) {
                w.lock().unwrap().push_back((count, t));
            }
        }
        w
    }

    #[test]
    fn empty_window_returns_false() {
        let w = SamplerWindow::default();
        assert!(!is_recent_change(&w, 5));
    }

    #[test]
    fn same_count_as_baseline_returns_false() {
        // Baseline sample from 4s ago, current count matches → no change
        let w = window_with_samples(&[(10, Duration::from_secs(4))]);
        assert!(!is_recent_change(&w, 10));
    }

    #[test]
    fn count_changed_since_baseline_returns_true() {
        // Baseline from 4s ago had count 10, current is 11 → changed recently
        let w = window_with_samples(&[(10, Duration::from_secs(4))]);
        assert!(is_recent_change(&w, 11));
    }

    #[test]
    fn all_samples_within_window_uses_oldest_as_baseline() {
        // All samples are newer than 3s; oldest has count 5, current 7 → changed
        let w = window_with_samples(&[
            (5, Duration::from_millis(2900)),
            (6, Duration::from_millis(2000)),
            (7, Duration::from_millis(1000)),
        ]);
        assert!(is_recent_change(&w, 7));
    }

    #[test]
    fn all_samples_within_window_same_count_returns_false() {
        // All recent, count unchanged
        let w = window_with_samples(&[
            (10, Duration::from_millis(2500)),
            (10, Duration::from_millis(1500)),
        ]);
        assert!(!is_recent_change(&w, 10));
    }
}
