#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};

#[tokio::test]
async fn use_snippets_off_trigger_not_expanded() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_snippets(false)
            .with_snippets(&[("sig", "Jane Smith")])
            .with_corrections(&[])
            .run("please send sig")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "please send sig");
    assert_eq!(outcome.pasted_text, "please send sig ");
}

#[tokio::test]
async fn use_snippets_on_trigger_expands_to_full_text() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_snippets(true)
            .with_snippets(&[("sig", "Jane Smith")])
            .with_corrections(&[])
            .run("please send sig")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "please send Jane Smith");
    assert_eq!(outcome.pasted_text, "please send Jane Smith ");
}

/// Snippets expand before corrections run. A correction rule that matches
/// text introduced by a snippet expansion will fire; a correction rule
/// present in the original transcript but not in the expansion won't affect
/// the expansion result. This pins the pipeline stage order.
#[tokio::test]
async fn use_snippets_on_expansion_is_subject_to_corrections() {
    // Snippet "sig" expands to "John Smith". Correction "john" → "Jane" then
    // fires on the expansion, producing "Jane Smith". If corrections had run
    // first, "john" wouldn't exist in the raw text and the final text would
    // be "John Smith" — proving that snippets run before corrections.
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_snippets(true)
            .with_use_corrections(true)
            .with_snippets(&[("sig", "John Smith")])
            .with_corrections(&[("john", "Jane")])
            .run("call sig please")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "call Jane Smith please");
}

/// When use_snippets is off, corrections still fire on the original text.
/// Snippet triggers present in the raw transcript are left unexpanded.
#[tokio::test]
async fn use_snippets_off_corrections_still_apply_to_raw_text() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_snippets(false)
            .with_use_corrections(true)
            .with_snippets(&[("sig", "John Smith")])
            .with_corrections(&[("dot", ".")])
            .run("send sig dot txt")
    })
    .await;

    // "sig" is not expanded (use_snippets=off); "dot" is corrected → ".".
    assert_eq!(outcome.history_entry.final_text, "send sig.txt");
}
