#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};

#[tokio::test]
async fn use_corrections_off_rules_not_applied() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_corrections(false)
            .with_corrections(&[("mongo", "MongoDB")])
            .run("I use mongo for storage")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use mongo for storage");
    assert_eq!(outcome.pasted_text, "I use mongo for storage ");
}

#[tokio::test]
async fn use_corrections_on_rule_applies() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_corrections(true)
            .with_corrections(&[("mongo", "MongoDB")])
            .run("I use mongo for storage")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use MongoDB for storage");
    assert_eq!(outcome.pasted_text, "I use MongoDB for storage ");
}

/// Multiple verbal-punctuation cues in one utterance all resolve. The
/// outer loop re-runs until stable so each occurrence is replaced.
#[tokio::test]
async fn use_corrections_on_chained_cue_resolves_fully() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_corrections(true)
            .with_corrections(&[("dash", "-")])
            .run("dash dash help")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "--help");
    assert_eq!(outcome.pasted_text, "--help ");
}

/// Multiple distinct rules can all fire in a single pipeline run.
#[tokio::test]
async fn use_corrections_on_multiple_rules_all_fire() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_corrections(true)
            .with_corrections(&[("mongo", "MongoDB"), ("js", "JavaScript")])
            .run("I use mongo and js")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use MongoDB and JavaScript");
}

/// When use_corrections is off the raw_text and final_text are identical
/// even if rules are configured.
#[tokio::test]
async fn use_corrections_off_raw_and_final_text_are_identical() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_use_corrections(false)
            .with_corrections(&[("dot", "."), ("at sign", "@")])
            .run("email at sign example dot com")
    })
    .await;

    assert_eq!(outcome.history_entry.raw_text, outcome.history_entry.final_text);
}

/// Rules from multiple named sets are all applied when the mode references them.
#[tokio::test]
async fn multiple_correction_sets_all_rules_fire() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_correction_set("set-a", &[("mongo", "MongoDB")])
            .with_correction_set("set-b", &[("js", "JavaScript")])
            .run("I use mongo and js")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use MongoDB and JavaScript");
}

/// When two sets define a rule for the same `from` term, the later set wins.
#[tokio::test]
async fn later_correction_set_overrides_earlier_on_collision() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_correction_set("set-a", &[("ts", "TypeScript-A")])
            .with_correction_set("set-b", &[("ts", "TypeScript-B")])
            .run("I write ts")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I write TypeScript-B");
}
