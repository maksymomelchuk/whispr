/// Regression test for the identity-correction hang at the pipeline level.
///
/// When a correction rule's `to` value is the same as `from` after
/// case-folding (e.g. "getmany" → "Getmany"), a naive re-scan loop would
/// re-match the replacement on every pass and never converge.
/// `apply_corrections` handles these as a one-shot sweep with cursor
/// advancement, and this test proves the full pipeline still completes
/// within a hard deadline and produces the case-corrected output.
///
/// The harness wraps each pipeline call in `tokio::time::timeout` +
/// `spawn_blocking` so a truly infinite loop fails the test (via cancellation)
/// rather than hanging the suite indefinitely.
#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use std::time::Duration;

#[tokio::test]
async fn identity_correction_terminates_within_deadline() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("getmany", "Getmany")])
            .with_use_corrections(true)
            .run("I love Getmany.")
    })
    .await;

    // Input already has the target casing, so the rule is a no-op.
    assert_eq!(outcome.history_entry.final_text, "I love Getmany.");
    assert_eq!(outcome.pasted_text, "I love Getmany. ");
}

#[tokio::test]
async fn identity_correction_with_mixed_case_input_terminates() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("getmany", "Getmany"), ("hello", "Hello")])
            .with_use_corrections(true)
            .run("getmany hello world")
    })
    .await;

    // Both rules re-case their matches in the lowercase input.
    assert_eq!(outcome.history_entry.final_text, "Getmany Hello world");
}

#[tokio::test]
async fn non_identity_correction_applies_and_terminates() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("dash", "-")])
            .with_use_corrections(true)
            .run("dash dash help")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "--help");
    assert_eq!(outcome.pasted_text, "--help ");
}

#[tokio::test]
async fn empty_corrections_list_passes_text_through() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .run("some raw transcript")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "some raw transcript");
    assert_eq!(outcome.history_entry.raw_text, "some raw transcript");
}

#[tokio::test]
async fn cleanup_output_flows_through_pipeline() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("mongo", "MongoDB")])
            .with_use_corrections(true)
            .with_cleanup("I prefer Mongo")
            .run("raw transcript that cleanup replaced")
    })
    .await;

    // raw_text is the original, replaced_text is from cleanup, final_text
    // has corrections applied on top.
    assert_eq!(
        outcome.history_entry.raw_text,
        "raw transcript that cleanup replaced"
    );
    assert_eq!(outcome.history_entry.replaced_text, "I prefer Mongo");
    assert_eq!(outcome.history_entry.final_text, "I prefer MongoDB");
}

#[tokio::test]
async fn elapsed_time_is_populated() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("getmany", "Getmany")])
            .with_use_corrections(true)
            .run("I love Getmany.")
    })
    .await;

    // elapsed is measured by run_stages itself; it should be well within a
    // second for this trivial input.
    assert!(outcome.elapsed < Duration::from_secs(1));
}
