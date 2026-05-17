#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};

/// When `use_terms` is off the terms list is not passed to the STT provider,
/// so the transcript may carry the provider's uncorrected spelling. At the
/// pipeline stage level the text passes through unchanged — no terms-based
/// rewrite happens here (terms are STT vocabulary hints only).
#[tokio::test]
async fn use_terms_off_raw_transcript_passes_through_unchanged() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_terms(false)
            .with_terms(&["MongoDB"])
            .with_corrections(&[])
            .with_snippets(&[])
            .run("I use mongodb for storage")
    })
    .await;

    assert_eq!(outcome.pasted_text, "I use mongodb for storage ");
    assert_eq!(outcome.history_entry.final_text, "I use mongodb for storage");
    assert_eq!(outcome.history_entry.raw_text, "I use mongodb for storage");
}

/// When `use_terms` is on the STT provider receives the vocabulary list and
/// is more likely to produce the exact term spellings. The pipeline preserves
/// whatever the STT produced — here the correctly-cased term comes through
/// to `pasted_text` intact.
#[tokio::test]
async fn use_terms_on_correct_spelling_from_stt_preserved_through_pipeline() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_terms(true)
            .with_terms(&["MongoDB"])
            .with_corrections(&[])
            .with_snippets(&[])
            .run("I use MongoDB for storage")
    })
    .await;

    assert_eq!(outcome.pasted_text, "I use MongoDB for storage ");
    assert_eq!(outcome.history_entry.final_text, "I use MongoDB for storage");
}

/// use_terms=false does not suppress corrections or snippets — those are
/// independent toggles. Corrections still apply even when use_terms is off.
#[tokio::test]
async fn use_terms_off_does_not_suppress_corrections() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_terms(false)
            .with_terms(&["MongoDB"])
            .with_corrections(&[("mongo", "MongoDB")])
            .with_snippets(&[])
            .run("I use mongo for storage")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use MongoDB for storage");
}
