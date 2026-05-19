#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};

/// Terms are STT vocabulary hints resolved before the session starts.
/// The post-transcription pipeline stages do not modify text based on
/// term sets — whatever the STT produced passes through unchanged.
#[tokio::test]
async fn no_term_sets_raw_transcript_passes_through_unchanged() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[])
            .with_snippets(&[])
            .run("I use mongodb for storage")
    })
    .await;

    assert_eq!(outcome.pasted_text, "I use mongodb for storage ");
    assert_eq!(outcome.history_entry.final_text, "I use mongodb for storage");
    assert_eq!(outcome.history_entry.raw_text, "I use mongodb for storage");
}

/// Correct spelling produced by the STT (because it received vocabulary hints)
/// is preserved intact by the pipeline — no interference from the term sets.
#[tokio::test]
async fn correct_spelling_from_stt_preserved_through_pipeline() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_term_set("s1", &["MongoDB"])
            .with_corrections(&[])
            .with_snippets(&[])
            .run("I use MongoDB for storage")
    })
    .await;

    assert_eq!(outcome.pasted_text, "I use MongoDB for storage ");
    assert_eq!(outcome.history_entry.final_text, "I use MongoDB for storage");
}

/// Term sets do not suppress corrections — those are independent toggles.
/// Corrections still apply even when no term sets are referenced.
#[tokio::test]
async fn no_term_sets_does_not_suppress_corrections() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("mongo", "MongoDB")])
            .with_use_corrections(true)
            .with_snippets(&[])
            .run("I use mongo for storage")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use MongoDB for storage");
}

/// compose_term_hints deduplication: entries appearing in multiple sets are
/// only forwarded to the provider once, preserving first-seen order.
#[tokio::test]
async fn multi_set_dedup_preserves_first_seen_order() {
    use whispr_lib::terms::compose_term_hints;
    use whispr_lib::config::NamedTermSet;

    let sets = vec![
        NamedTermSet {
            id: "a".to_string(),
            name: "A".to_string(),
            entries: vec!["MongoDB".to_string(), "shared".to_string()],
        },
        NamedTermSet {
            id: "b".to_string(),
            name: "B".to_string(),
            entries: vec!["shared".to_string(), "TypeScript".to_string()],
        },
    ];
    let ids = vec!["a".to_string(), "b".to_string()];
    let result = compose_term_hints(&sets, &ids);
    assert_eq!(result, vec!["MongoDB", "shared", "TypeScript"]);
}
