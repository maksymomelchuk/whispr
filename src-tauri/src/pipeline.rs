use crate::config::Settings;
use crate::corrections::apply_corrections;
use crate::history::{CleanupStatus, HistoryEntry, now_unix_seconds};
use crate::mode::Mode;
use crate::snippets::expand_snippets;
use std::time::{Duration, Instant};

/// Carries a user-visible notification produced by the translate or cleanup
/// stages. `Focus` raises the main window (for errors requiring user action);
/// `Flash` paints the overlay red briefly (transient soft warning).
#[derive(Debug, PartialEq)]
pub enum Notice {
    None,
    Flash(String),
    Focus(String),
}

/// `Focus` beats `Flash` beats `None` — when both a transient warning and an
/// actionable error are emitted, the actionable one wins.
pub fn merge_notices(a: Notice, b: Notice) -> Notice {
    match (a, b) {
        (Notice::Focus(m), _) | (_, Notice::Focus(m)) => Notice::Focus(m),
        (Notice::Flash(m), _) | (_, Notice::Flash(m)) => Notice::Flash(m),
        _ => Notice::None,
    }
}

/// Result of running the post-transcription pipeline stages.
pub struct Outcome {
    /// Text that would be pasted (final text with a trailing space).
    pub pasted_text: String,
    /// History entry computed from this session.
    pub history_entry: HistoryEntry,
    /// Wall time from the start of `run_stages` to completion.
    pub elapsed: Duration,
}

/// Pre-resolved output from the cleanup stage, fed into `run_stages`.
pub struct CleanupOutput {
    /// Text after cleanup ran, or the original text if cleanup was skipped.
    pub replaced_text: String,
    /// Whether and how cleanup ran.
    pub status: CleanupStatus,
}

/// Error string produced when the recorder fails to start or cannot negotiate an
/// audio format. The argument is the recorder's own error message.
///
/// Defined here (cross-platform) so the exact wording can be tested from
/// integration tests on any platform without a macOS environment or AppHandle.
pub fn recorder_failed_error(recorder_err: &str) -> String {
    format!("Recording failed: {recorder_err}")
}

/// Error string produced when the recorder's audio thread exits or panics
/// before resolving the format-negotiation oneshot.
pub const RECORDER_THREAD_CRASHED_ERROR: &str = "Recording thread crashed";

/// Runs the post-transcription pipeline stages — snippet expansion, correction
/// application, and paste-text preparation — without any Tauri or macOS
/// dependencies, so it can be exercised from integration tests on any platform.
pub fn run_stages(
    raw_text: &str,
    speak_duration: Duration,
    mode: &Mode,
    settings: &Settings,
    cleanup_output: CleanupOutput,
) -> Outcome {
    let start = Instant::now();

    let CleanupOutput {
        replaced_text,
        status: cleanup_status,
    } = cleanup_output;

    let mut final_text = replaced_text.clone();
    if mode.use_snippets {
        final_text = expand_snippets(&final_text, &settings.snippets);
    }
    if mode.use_corrections {
        final_text = apply_corrections(&final_text, &settings.corrections);
    }

    let pasted_text = format!("{final_text} ");

    let history_entry = HistoryEntry {
        timestamp: now_unix_seconds(),
        speak_duration_ms: speak_duration.as_millis() as u64,
        raw_text: raw_text.to_string(),
        replaced_text,
        final_text,
        cleanup_status,
    };

    Outcome {
        pasted_text,
        history_entry,
        elapsed: start.elapsed(),
    }
}
