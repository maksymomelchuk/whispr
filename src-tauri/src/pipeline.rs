use crate::config::Settings;
use crate::corrections::apply_corrections;
use crate::history::{CleanupStatus, HistoryEntry, now_unix_seconds};
use crate::mode::Mode;
use crate::snippets::expand_snippets;
use std::time::{Duration, Instant};

/// Result of running the post-transcription pipeline stages.
pub struct Outcome {
    /// Text that would be pasted (final text with a trailing space).
    pub pasted_text: String,
    /// History entry computed from this session.
    pub history_entry: HistoryEntry,
    /// Wall time from the start of `run_stages` to completion.
    pub elapsed: Duration,
    /// Events that the production pipeline would have emitted; always empty
    /// from `run_stages` itself (no AppHandle access). The harness may
    /// populate this from other sources.
    pub emitted_events: Vec<String>,
}

/// Pre-resolved output from the cleanup stage, fed into `run_stages`.
pub struct CleanupOutput {
    /// Text after cleanup ran, or the original text if cleanup was skipped.
    pub replaced_text: String,
    /// Whether and how cleanup ran.
    pub status: CleanupStatus,
}

/// Runs the post-transcription pipeline stages — snippet expansion, correction
/// application, and paste-text preparation — without any Tauri or macOS
/// dependencies, so it can be exercised from integration tests on any platform.
///
/// Production `run_session` in `ptt.rs` calls this after resolving translation
/// and cleanup. The `PipelineHarness` in integration tests calls it directly
/// with preset inputs to exercise these stages in isolation.
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
        emitted_events: Vec::new(),
    }
}
