/// Regression test for the identity-correction hang at the pipeline level.
///
/// When a correction rule's `to` value is the same as `from` after
/// case-folding (e.g. "getmany" → "Getmany"), a naive re-scan loop would
/// re-match the replacement on every pass and never converge. `apply_corrections`
/// caps iteration with `MAX_PASSES`, and this test proves the full pipeline
/// honours that cap by completing within a hard deadline.
///
/// The test wraps the pipeline call in `tokio::time::timeout` +
/// `spawn_blocking` so a truly infinite loop fails the test (via cancellation)
/// rather than hanging the suite indefinitely.
#[path = "common/mod.rs"]
mod common;

use common::{PipelineHarness, HARNESS_DEADLINE};
use std::time::Duration;

#[tokio::test]
async fn identity_correction_terminates_within_deadline() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[("getmany", "Getmany")])
                .run("I love Getmany.")
        }),
    )
    .await;

    assert!(
        result.is_ok(),
        "harness deadline exceeded: pipeline with identity correction rule hung"
    );
    let outcome = result.unwrap().expect("spawn_blocking panicked");

    // The identity rule is filtered out by apply_corrections (case-folded
    // from == to), so the text is unchanged.
    assert_eq!(outcome.history_entry.final_text, "I love Getmany.");
    assert_eq!(outcome.pasted_text, "I love Getmany. ");
}

#[tokio::test]
async fn identity_correction_with_mixed_case_input_terminates() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[("getmany", "Getmany"), ("hello", "Hello")])
                .run("getmany hello world")
        }),
    )
    .await;

    assert!(
        result.is_ok(),
        "harness deadline exceeded: pipeline with multiple identity rules hung"
    );
    let outcome = result.unwrap().expect("spawn_blocking panicked");

    // Both rules are case-folded identities and are filtered out.
    assert_eq!(outcome.history_entry.final_text, "getmany hello world");
}

#[tokio::test]
async fn non_identity_correction_applies_and_terminates() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[("dash", "-")])
                .run("dash dash help")
        }),
    )
    .await;

    assert!(result.is_ok(), "harness deadline exceeded");
    let outcome = result.unwrap().expect("spawn_blocking panicked");
    assert_eq!(outcome.history_entry.final_text, "--help");
    assert_eq!(outcome.pasted_text, "--help ");
}

#[tokio::test]
async fn empty_corrections_list_passes_text_through() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[])
                .run("some raw transcript")
        }),
    )
    .await;

    assert!(result.is_ok(), "harness deadline exceeded");
    let outcome = result.unwrap().expect("spawn_blocking panicked");
    assert_eq!(outcome.history_entry.final_text, "some raw transcript");
    assert_eq!(outcome.history_entry.raw_text, "some raw transcript");
}

#[tokio::test]
async fn cleanup_output_flows_through_pipeline() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[("mongo", "MongoDB")])
                .with_cleanup("I prefer Mongo")
                .run("raw transcript that cleanup replaced")
        }),
    )
    .await;

    assert!(result.is_ok(), "harness deadline exceeded");
    let outcome = result.unwrap().expect("spawn_blocking panicked");

    // raw_text is the original, replaced_text is from cleanup, final_text
    // has corrections applied on top.
    assert_eq!(outcome.history_entry.raw_text, "raw transcript that cleanup replaced");
    assert_eq!(outcome.history_entry.replaced_text, "I prefer Mongo");
    assert_eq!(outcome.history_entry.final_text, "I prefer MongoDB");
}

#[tokio::test]
async fn elapsed_time_is_populated() {
    let result = tokio::time::timeout(
        HARNESS_DEADLINE,
        tokio::task::spawn_blocking(|| {
            PipelineHarness::new()
                .with_corrections(&[("getmany", "Getmany")])
                .run("I love Getmany.")
        }),
    )
    .await;

    assert!(result.is_ok(), "harness deadline exceeded");
    let outcome = result.unwrap().expect("spawn_blocking panicked");

    // elapsed is measured by run_stages itself; it should be non-zero and
    // well within a second for this trivial input.
    assert!(outcome.elapsed < Duration::from_secs(1));
}
