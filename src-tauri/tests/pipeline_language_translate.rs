/// Integration tests for the language and translate axes of the pipeline matrix.
///
/// Covers:
/// - `TranslateTarget::Off` with `ModeLanguage::Exact`, `Hints`, and `Auto`:
///   raw transcript flows through the pipeline unchanged.
/// - `TranslateTarget::Apple` with a stubbed translator: the translated text
///   becomes `pasted_text`.
/// - Notice-merging precedence: `Focus` beats `Flash` beats `None`.
#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::mode::{ModeCleanup, ModeLanguage, Mode, TranslateTarget};
use whispr_lib::pipeline::{merge_notices, Notice};
use whispr_lib::provider::ProviderModel;

// ── Translate Off ────────────────────────────────────────────────────────────

#[tokio::test]
async fn translate_off_with_exact_language_passes_text_through() {
    // Default mode uses ModeLanguage::Exact { code: "en" } + TranslateTarget::Off.
    let outcome = run_under_deadline(|| PipelineHarness::new().run("hello exact language")).await;

    assert_eq!(outcome.pasted_text, "hello exact language ");
    assert_eq!(outcome.history_entry.raw_text, "hello exact language");
}

#[tokio::test]
async fn translate_off_with_hints_language_passes_text_through() {
    let mode = mode_with_language_off(ModeLanguage::Hints {
        codes: vec!["en".to_string(), "uk".to_string()],
    });
    let outcome = run_under_deadline(|| {
        PipelineHarness::new().with_mode(mode).run("hello hints language")
    })
    .await;

    assert_eq!(outcome.pasted_text, "hello hints language ");
    assert_eq!(outcome.history_entry.raw_text, "hello hints language");
}

#[tokio::test]
async fn translate_off_with_auto_language_passes_text_through() {
    let mode = mode_with_language_off(ModeLanguage::Auto);
    let outcome = run_under_deadline(|| {
        PipelineHarness::new().with_mode(mode).run("hello auto language")
    })
    .await;

    assert_eq!(outcome.pasted_text, "hello auto language ");
    assert_eq!(outcome.history_entry.raw_text, "hello auto language");
}

// ── Translate Apple (stubbed) ────────────────────────────────────────────────

/// Simulates the Apple translate branch: `with_translated_text` injects the
/// translator's output, and the pipeline should paste the translated text.
#[tokio::test]
async fn translate_apple_stub_output_becomes_pasted_text() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_translated_text("Hello World")
            .run("Привіт Світ")
    })
    .await;

    assert_eq!(outcome.pasted_text, "Hello World ");
    // raw_text is preserved as-is; only pasted_text reflects the translation.
    assert_eq!(outcome.history_entry.raw_text, "Привіт Світ");
}

/// Corrections and snippets applied AFTER translation — the translated text is
/// the input to those stages.
#[tokio::test]
async fn translate_apple_stub_with_downstream_corrections() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_translated_text("hello from ukraine")
            .with_corrections(&[("ukraine", "Ukraine")])
            .with_use_corrections(true)
            .run("привіт з україни")
    })
    .await;

    assert_eq!(outcome.pasted_text, "hello from Ukraine ");
    assert_eq!(outcome.history_entry.raw_text, "привіт з україни");
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
    // merge_notices is order-independent for the dominance check.
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

fn mode_with_language_off(language: ModeLanguage) -> Mode {
    Mode {
        id: "test-translate-off".to_string(),
        name: "Translate off test".to_string(),
        icon: None,
        language,
        translate: TranslateTarget::Off,
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
