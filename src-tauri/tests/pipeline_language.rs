/// Integration tests for the language axis of the pipeline matrix.
///
/// Covers:
/// - `ModeLanguage::Exact`, `Hints`, and `Auto`: raw transcript flows through
///   the pipeline unchanged.
/// - Notice-merging precedence: `Focus` beats `Flash` beats `None`.
#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::mode::{Mode, ModeCleanup, ModeLanguage};
use whispr_lib::pipeline::{merge_notices, Notice};
use whispr_lib::provider::ProviderModel;

// ── Language pass-through ────────────────────────────────────────────────────

#[tokio::test]
async fn exact_language_passes_text_through() {
    let outcome = run_under_deadline(|| PipelineHarness::new().run("hello exact language")).await;

    assert_eq!(outcome.pasted_text, "hello exact language ");
    assert_eq!(outcome.history_entry.raw_text, "hello exact language");
}

#[tokio::test]
async fn hints_language_passes_text_through() {
    let mode = mode_with_language(ModeLanguage::Hints {
        codes: vec!["en".to_string(), "uk".to_string()],
    });
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_mode(mode)
            .run("hello hints language")
    })
    .await;

    assert_eq!(outcome.pasted_text, "hello hints language ");
    assert_eq!(outcome.history_entry.raw_text, "hello hints language");
}

#[tokio::test]
async fn auto_language_passes_text_through() {
    let mode = mode_with_language(ModeLanguage::Auto);
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_mode(mode)
            .run("hello auto language")
    })
    .await;

    assert_eq!(outcome.pasted_text, "hello auto language ");
    assert_eq!(outcome.history_entry.raw_text, "hello auto language");
}

// ── Notice-merging precedence ────────────────────────────────────────────────

#[test]
fn notice_focus_dominates_flash() {
    let result = merge_notices(
        Notice::Focus("critical error".to_string()),
        Notice::Flash("soft warning".to_string()),
    );
    assert_eq!(result, Notice::Focus("critical error".to_string()));
}

#[test]
fn notice_flash_dominates_when_other_is_focus_symmetric() {
    let result = merge_notices(
        Notice::Flash("soft".to_string()),
        Notice::Focus("critical".to_string()),
    );
    assert_eq!(result, Notice::Focus("critical".to_string()));
}

#[test]
fn notice_focus_dominates_none() {
    let result = merge_notices(Notice::None, Notice::Focus("focus error".to_string()));
    assert_eq!(result, Notice::Focus("focus error".to_string()));
}

#[test]
fn notice_flash_dominates_none() {
    let result = merge_notices(Notice::Flash("transient".to_string()), Notice::None);
    assert_eq!(result, Notice::Flash("transient".to_string()));
}

#[test]
fn notice_none_merged_with_none_yields_none() {
    assert_eq!(merge_notices(Notice::None, Notice::None), Notice::None);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn mode_with_language(language: ModeLanguage) -> Mode {
    Mode {
        id: "test-language".to_string(),
        name: "Language test".to_string(),
        icon: None,
        language,
        ai_cleanup: ModeCleanup::default(),
        legacy_use_dictionary: None,
        use_terms: true,
        use_corrections: true,
        use_snippets: true,
        provider_model: ProviderModel::Deepgram,
        term_set_ids: vec![],
        correction_set_ids: vec![],
    }
}
