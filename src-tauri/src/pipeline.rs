use crate::config::Settings;
use crate::corrections::{apply_corrections, compose_corrections};
use crate::history::{
    new_entry_id, now_unix_seconds, CleanupStatus, HistoryEntry, ProfileSnapshot,
};
use crate::mode::Mode;
use crate::snippets::expand_snippets;
use std::time::{Duration, Instant};

/// Carries a user-visible notification produced by the cleanup stage.
/// `Focus` raises the main window (for errors requiring user action);
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
#[derive(Debug)]
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

/// Which paste action to take for a completed (possibly failed) cleanup run.
#[derive(Debug, PartialEq)]
pub enum PastePolicy {
    PasteRaw,
    SuppressAndClipboard,
}

/// Maps `(cleanup_status, paste_raw_on_failure)` to a paste action.
/// The suppress branch is taken only when a failure status is combined with
/// `paste_raw_on_failure = false`; non-failure statuses are always `PasteRaw`.
pub fn resolve_paste_policy(
    cleanup_status: &CleanupStatus,
    paste_raw_on_failure: bool,
) -> PastePolicy {
    let is_failure = matches!(
        cleanup_status,
        CleanupStatus::NoCredential
            | CleanupStatus::FailedTimeout
            | CleanupStatus::FailedTransient(_)
            | CleanupStatus::FailedCredential(_)
    );
    if is_failure && !paste_raw_on_failure {
        PastePolicy::SuppressAndClipboard
    } else {
        PastePolicy::PasteRaw
    }
}

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
    if !mode.correction_set_ids.is_empty() || !settings.learned_entries.is_empty() {
        let entries = compose_corrections(
            &mode.correction_set_ids,
            &settings.correction_sets,
            &settings.learned_entries,
        );
        final_text = apply_corrections(&final_text, &entries);
    }

    let pasted_text = format!("{final_text} ");

    let profile_snapshot = ProfileSnapshot {
        cleanup_provider: mode.ai_cleanup.provider.clone(),
        cleanup_model: mode.ai_cleanup.model.clone(),
        cleanup_prompt_override: mode.ai_cleanup.prompt_override.clone(),
        use_snippets: mode.use_snippets,
        correction_set_ids: mode.correction_set_ids.clone(),
    };

    let history_entry = HistoryEntry {
        id: new_entry_id(),
        timestamp: now_unix_seconds(),
        speak_duration_ms: speak_duration.as_millis() as u64,
        raw_text: raw_text.to_string(),
        replaced_text,
        final_text,
        cleanup_status,
        profile_snapshot: Some(profile_snapshot),
        provider_model: Some(mode.provider_model.clone()),
        app_name: None,
        bundle_id: None,
        context_channels: vec![],
    };

    Outcome {
        pasted_text,
        history_entry,
        elapsed: start.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::CleanupStatus;

    #[test]
    fn failure_with_toggle_off_suppresses_paste() {
        let cases = vec![
            CleanupStatus::FailedTimeout,
            CleanupStatus::FailedTransient("network error".to_string()),
            CleanupStatus::FailedCredential("bad key".to_string()),
            CleanupStatus::NoCredential,
        ];
        for status in &cases {
            assert_eq!(
                resolve_paste_policy(status, false),
                PastePolicy::SuppressAndClipboard,
                "expected SuppressAndClipboard for {status:?} with toggle off",
            );
        }
    }

    #[test]
    fn failure_with_toggle_on_pastes_raw() {
        let cases = vec![
            CleanupStatus::FailedTimeout,
            CleanupStatus::FailedTransient("network error".to_string()),
            CleanupStatus::FailedCredential("bad key".to_string()),
            CleanupStatus::NoCredential,
        ];
        for status in &cases {
            assert_eq!(
                resolve_paste_policy(status, true),
                PastePolicy::PasteRaw,
                "expected PasteRaw for {status:?} with toggle on",
            );
        }
    }

    #[test]
    fn non_failure_statuses_always_paste_raw() {
        let cases = vec![
            CleanupStatus::Disabled,
            CleanupStatus::SkippedBelowMinWords,
            CleanupStatus::SkippedBelowMinDuration,
            CleanupStatus::Ran,
        ];
        for toggle in [true, false] {
            for status in &cases {
                assert_eq!(
                    resolve_paste_policy(status, toggle),
                    PastePolicy::PasteRaw,
                    "expected PasteRaw for non-failure {status:?} with toggle={toggle}",
                );
            }
        }
    }
}
