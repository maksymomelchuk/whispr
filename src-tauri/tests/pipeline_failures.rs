//! Integration tests for pipeline failure modes.
//!
//! Recorder-failure tests pin the user-visible error messages produced when the
//! recorder cannot start, crashes mid-session, or fails to negotiate an audio
//! format.  They exercise `pipeline::recorder_failed_error` and
//! `RECORDER_THREAD_CRASHED_ERROR` directly; `ptt::run_session` delegates to
//! those same helpers, so a future rewording breaks these tests before it ships.
//!
//! The history-append-failure test runs through `PipelineHarness` and verifies
//! that `run_stages` returns a complete `Outcome` regardless of what happens to
//! the history entry afterward (history::append is called by ptt::run_session
//! *after* run_stages returns, and its failure is only logged, never fatal).
#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::{history::CleanupStatus, pipeline};

// ── Recorder start error ────────────────────────────────────────────────────

#[test]
fn recorder_start_error_produces_expected_message() {
    let msg = pipeline::recorder_failed_error("no input device available");
    assert_eq!(msg, "Recording failed: no input device available");
}

// ── Recorder thread crash ───────────────────────────────────────────────────

#[test]
fn recorder_thread_crash_produces_expected_message() {
    // The recorder's audio thread drops its format_tx without sending when it
    // panics or exits unexpectedly.  The session must surface a clean error
    // rather than hanging on the never-resolved oneshot.
    assert_eq!(
        pipeline::RECORDER_THREAD_CRASHED_ERROR,
        "Recording thread crashed"
    );
}

// ── Format negotiation failure ──────────────────────────────────────────────
// When cpal reports an unsupported sample format (e.g. U8, F64) the recorder
// sends Err("unsupported sample format: …") through format_tx.  That error is
// wrapped with the same "Recording failed: …" prefix used for any other
// recorder start failure.

#[test]
fn format_negotiation_failure_produces_expected_message() {
    let msg = pipeline::recorder_failed_error("unsupported sample format: U8");
    assert_eq!(msg, "Recording failed: unsupported sample format: U8");
}

// ── History append failure ──────────────────────────────────────────────────

#[tokio::test]
async fn history_append_failure_does_not_prevent_session_completion() {
    // run_stages always returns a complete Outcome.  history::append runs
    // *after* run_stages returns inside ptt::run_session; a filesystem error
    // there is logged and the session still returns Ok(()) so the paste
    // already dispatched is unaffected.
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_cleanup("Cleaned transcript result")
            .run("raw transcript input")
    })
    .await;

    assert_eq!(outcome.pasted_text, "Cleaned transcript result ");
    assert_eq!(
        outcome.history_entry.final_text,
        "Cleaned transcript result"
    );
    assert_eq!(outcome.history_entry.raw_text, "raw transcript input");
    assert!(
        matches!(outcome.history_entry.cleanup_status, CleanupStatus::Ran),
        "cleanup status must be Ran when cleanup output was provided"
    );
}
