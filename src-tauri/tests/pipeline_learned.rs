#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::config::{LearnedEntry, LearnedEntryStatus, LearnedKind};
use whispr_lib::terms::compose_term_hints;

fn promoted_correction(id: &str, from: &str, word: &str) -> LearnedEntry {
    LearnedEntry {
        id: id.to_string(),
        word: word.to_string(),
        kind: LearnedKind::Correction {
            from: from.to_string(),
        },
        status: LearnedEntryStatus::Promoted,
        total_observations: 2,
        last_observed_ms: 1_000_000,
    }
}

fn candidate_correction(id: &str, from: &str, word: &str) -> LearnedEntry {
    LearnedEntry {
        id: id.to_string(),
        word: word.to_string(),
        kind: LearnedKind::Correction {
            from: from.to_string(),
        },
        status: LearnedEntryStatus::Candidate,
        total_observations: 1,
        last_observed_ms: 1_000_000,
    }
}

fn promoted_term(id: &str, word: &str) -> LearnedEntry {
    LearnedEntry {
        id: id.to_string(),
        word: word.to_string(),
        kind: LearnedKind::Term,
        status: LearnedEntryStatus::Promoted,
        total_observations: 2,
        last_observed_ms: 1_000_000,
    }
}

fn candidate_term(id: &str, word: &str) -> LearnedEntry {
    LearnedEntry {
        id: id.to_string(),
        word: word.to_string(),
        kind: LearnedKind::Term,
        status: LearnedEntryStatus::Candidate,
        total_observations: 1,
        last_observed_ms: 1_000_000,
    }
}

#[tokio::test]
async fn promoted_learned_correction_applies_post_stt() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_learned_entry(promoted_correction("c1", "tauri", "Tauri"))
            .run("I love tauri for building apps")
    })
    .await;

    assert_eq!(
        outcome.history_entry.final_text,
        "I love Tauri for building apps"
    );
}

#[tokio::test]
async fn promoted_learned_correction_applies_without_ai_cleanup() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_learned_entry(promoted_correction("c1", "tauri", "Tauri"))
            .run("tauri is awesome")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "Tauri is awesome");
    assert_eq!(outcome.pasted_text, "Tauri is awesome ");
}

#[tokio::test]
async fn candidate_learned_correction_never_applies() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_learned_entry(candidate_correction("c1", "tauri", "Tauri"))
            .run("I use tauri")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I use tauri");
}

#[tokio::test]
async fn manual_correction_beats_learned_on_collision() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_correction_set("manual", &[("tauri", "TAURI-MANUAL")])
            .with_learned_entry(promoted_correction("c1", "tauri", "Tauri-Learned"))
            .run("I love tauri")
    })
    .await;

    assert_eq!(outcome.history_entry.final_text, "I love TAURI-MANUAL");
}

#[test]
fn promoted_learned_term_included_in_hint_composition() {
    use whispr_lib::config::NamedTermSet;

    let manual_sets = vec![NamedTermSet {
        id: "s1".to_string(),
        name: "S1".to_string(),
        entries: vec!["MongoDB".to_string()],
    }];
    let set_ids = vec!["s1".to_string()];
    let learned = vec![promoted_term("t1", "Tauri")];

    let result = compose_term_hints(&manual_sets, &set_ids, &learned);
    assert!(result.contains(&"MongoDB".to_string()));
    assert!(result.contains(&"Tauri".to_string()));
    // Manual terms come first.
    assert_eq!(result[0], "MongoDB");
}

#[test]
fn candidate_learned_term_excluded_from_hint_composition() {
    use whispr_lib::config::NamedTermSet;

    let manual_sets = vec![NamedTermSet {
        id: "s1".to_string(),
        name: "S1".to_string(),
        entries: vec!["MongoDB".to_string()],
    }];
    let set_ids = vec!["s1".to_string()];
    let learned = vec![candidate_term("t1", "Tauri")];

    let result = compose_term_hints(&manual_sets, &set_ids, &learned);
    assert!(result.contains(&"MongoDB".to_string()));
    assert!(!result.contains(&"Tauri".to_string()));
}

#[test]
fn manual_term_beats_learned_on_budget_tie_via_ordering() {
    use whispr_lib::config::NamedTermSet;
    use whispr_lib::terms::deepgram_keyterms;
    use whispr_lib::terms::DEEPGRAM_KEYTERM_BUDGET_BYTES;

    // Build a manual term that fills all but ~12 bytes of the budget.
    // "&keyterm=".len() = 9; a 3-char term needs 12 bytes.
    let filler_len = DEEPGRAM_KEYTERM_BUDGET_BYTES - 12;
    let filler = "a".repeat(filler_len);

    let manual_sets = vec![NamedTermSet {
        id: "s1".to_string(),
        name: "S1".to_string(),
        entries: vec![filler.clone()],
    }];
    let set_ids = vec!["s1".to_string()];
    let learned = vec![promoted_term("t1", "Tauri")];

    let hints = compose_term_hints(&manual_sets, &set_ids, &learned);
    // Both filler and Tauri are in hints
    assert_eq!(hints[0], filler);
    assert_eq!(hints[1], "Tauri");

    // But budget truncation drops the learned term.
    let keyterms = deepgram_keyterms(&hints, DEEPGRAM_KEYTERM_BUDGET_BYTES);
    assert!(
        keyterms.contains(&filler),
        "manual term must survive budget truncation"
    );
    assert!(
        !keyterms.contains(&"Tauri".to_string()),
        "learned term must be dropped when budget exhausted"
    );
}
