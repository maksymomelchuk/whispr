#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::history::CleanupStatus;

/// When cleanup is disabled the raw transcript flows straight through as
/// the pasted text.
#[tokio::test]
async fn cleanup_disabled_raw_transcript_is_pasted() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .run("this is the raw transcript")
    })
    .await;

    assert!(
        matches!(outcome.history_entry.cleanup_status, CleanupStatus::Disabled),
        "expected Disabled, got {:?}",
        outcome.history_entry.cleanup_status,
    );
    assert_eq!(outcome.pasted_text, "this is the raw transcript ");
    assert_eq!(outcome.history_entry.raw_text, "this is the raw transcript");
    assert_eq!(outcome.history_entry.replaced_text, "this is the raw transcript");
}

/// When cleanup succeeds the pasted text comes from the cleaned output, not
/// the raw transcript. The raw transcript is still preserved in history.
#[tokio::test]
async fn cleanup_success_cleaned_text_is_pasted() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .with_cleanup("The cleaned and polished version.")
            .run("uh this is like the raw transcript")
    })
    .await;

    assert!(
        matches!(outcome.history_entry.cleanup_status, CleanupStatus::Ran),
        "expected Ran, got {:?}",
        outcome.history_entry.cleanup_status,
    );
    assert_eq!(outcome.pasted_text, "The cleaned and polished version. ");
    assert_eq!(outcome.history_entry.raw_text, "uh this is like the raw transcript");
    assert_eq!(outcome.history_entry.replaced_text, "The cleaned and polished version.");
}

/// On a transient cleanup error the pipeline falls back to the raw
/// transcript and records the failure status in the history entry.
#[tokio::test]
async fn cleanup_transient_error_falls_back_to_raw_transcript() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .with_cleanup_error(CleanupStatus::FailedTransient("network error".to_string()))
            .run("raw transcript on error")
    })
    .await;

    assert!(
        matches!(outcome.history_entry.cleanup_status, CleanupStatus::FailedTransient(_)),
        "expected FailedTransient, got {:?}",
        outcome.history_entry.cleanup_status,
    );
    assert_eq!(outcome.pasted_text, "raw transcript on error ");
    assert_eq!(outcome.history_entry.raw_text, "raw transcript on error");
}

/// On a cleanup timeout the same fallback applies: raw transcript pasted,
/// failure status recorded.
#[tokio::test]
async fn cleanup_timeout_falls_back_to_raw_transcript() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .with_cleanup_error(CleanupStatus::FailedTimeout)
            .run("raw transcript on timeout")
    })
    .await;

    assert!(
        matches!(outcome.history_entry.cleanup_status, CleanupStatus::FailedTimeout),
        "expected FailedTimeout, got {:?}",
        outcome.history_entry.cleanup_status,
    );
    assert_eq!(outcome.pasted_text, "raw transcript on timeout ");
}

/// Corrections still apply on top of successful cleanup output. The cleanup
/// output feeds the corrections stage, not the raw transcript.
#[tokio::test]
async fn cleanup_success_corrections_apply_on_cleaned_text() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("mongo", "MongoDB")])
            .with_cleanup("I prefer mongo for storage")
            .run("uh I like mongo")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I prefer MongoDB for storage");
    assert_eq!(outcome.history_entry.replaced_text, "I prefer mongo for storage");
    assert_eq!(outcome.history_entry.raw_text, "uh I like mongo");
}
