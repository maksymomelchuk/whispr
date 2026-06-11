use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

const RECENCY_WINDOW: Duration = Duration::from_secs(3);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

pub type SamplerWindow = Arc<Mutex<VecDeque<(i64, Instant)>>>;

pub fn new_window() -> SamplerWindow {
    Arc::new(Mutex::new(VecDeque::new()))
}

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
                if let Some(cutoff) = now.checked_sub(Duration::from_secs(5)) {
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
    let window = app.state::<AppState>().clipboard_window.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.state::<AppState>().pending_clipboard_rx.lock().unwrap() = Some(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let current_count = platform_change_count();
        let text = if is_recent_change(&window, current_count) {
            platform_read_text()
        } else {
            None
        };
        let _ = tx.send(text);
    });
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn platform_change_count() -> i64 {
    use objc2_app_kit::NSPasteboard;
    NSPasteboard::generalPasteboard().changeCount() as i64
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
pub fn platform_read_text() -> Option<String> {
    clipboard_win::get_clipboard_string().ok()
}

// ── Linux (no-op) ─────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn platform_change_count() -> i64 {
    0
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
        let w = new_window();
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
