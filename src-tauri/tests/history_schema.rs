#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use whispr_lib::cleanup::AiProviderId;
use whispr_lib::history::{pasted_text, CleanupStatus, HistoryEntry, ProfileSnapshot};
use whispr_lib::mode::Mode;

/// New dictations carry a non-empty stable id.
#[tokio::test]
async fn pipeline_entry_has_non_empty_id() {
    let outcome = run_under_deadline(|| PipelineHarness::new().run("hello world")).await;
    assert!(
        !outcome.history_entry.id.is_empty(),
        "history entry id must not be empty"
    );
}

/// Two entries produced in the same process have distinct ids.
#[tokio::test]
async fn pipeline_entries_have_unique_ids() {
    let (a, b) = tokio::join!(
        run_under_deadline(|| PipelineHarness::new().run("first entry")),
        run_under_deadline(|| PipelineHarness::new().run("second entry")),
    );
    assert_ne!(
        a.history_entry.id, b.history_entry.id,
        "concurrent entries must have distinct ids"
    );
}

/// The pipeline captures a ProfileSnapshot from the active mode.
#[tokio::test]
async fn pipeline_entry_carries_profile_snapshot() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_cleanup("cleaned text")
            .run("raw text")
    })
    .await;
    assert!(
        outcome.history_entry.profile_snapshot.is_some(),
        "history entry must include a profile snapshot"
    );
}

/// The snapshot captures cleanup provider, model, and use_snippets from the mode.
#[tokio::test]
async fn pipeline_snapshot_captures_mode_cleanup_settings() {
    let mut mode = Mode::seed_cleaned_en();
    mode.use_snippets = false;

    let outcome = run_under_deadline(move || {
        PipelineHarness::new()
            .with_mode(mode)
            .with_cleanup("cleaned")
            .run("raw")
    })
    .await;

    let snap = outcome
        .history_entry
        .profile_snapshot
        .expect("snapshot must be present");

    assert_eq!(snap.cleanup_provider, AiProviderId::Anthropic);
    assert_eq!(snap.cleanup_model, "claude-haiku-4-5");
    assert!(!snap.use_snippets);
    assert_eq!(snap.cleanup_prompt_override, None);
}

/// RecoveredManually round-trips through serde.
#[test]
fn cleanup_status_recovered_manually_round_trips() {
    let status = CleanupStatus::RecoveredManually;
    let json = serde_json::to_string(&status).unwrap();
    assert!(
        json.contains("recovered_manually"),
        "must serialize as recovered_manually, got: {json}"
    );
    let decoded: CleanupStatus = serde_json::from_str(&json).unwrap();
    assert!(
        matches!(decoded, CleanupStatus::RecoveredManually),
        "must round-trip"
    );
}

/// pasted_text returns final_text for RecoveredManually, just like Ran.
#[test]
fn pasted_text_recovered_manually_returns_final_text() {
    let entry = HistoryEntry {
        id: "test-id".to_string(),
        timestamp: 0,
        speak_duration_ms: 1000,
        raw_text: "raw".to_string(),
        replaced_text: "replaced".to_string(),
        final_text: "final recovered".to_string(),
        cleanup_status: CleanupStatus::RecoveredManually,
        profile_snapshot: None,
        provider_model: None,
        app_name: None,
        bundle_id: None,
    };
    assert_eq!(pasted_text(&entry), "final recovered");
}

/// Old entries without an id or profile_snapshot deserialize with defaults.
#[test]
fn old_entry_without_id_and_snapshot_deserializes() {
    let json = r#"{
        "timestamp": 1000000,
        "speak_duration_ms": 5000,
        "raw_text": "hello",
        "replaced_text": "hello",
        "final_text": "hello",
        "cleanup_status": {"kind": "disabled"}
    }"#;
    let entry: HistoryEntry = serde_json::from_str(json).expect("must deserialize");
    assert!(entry.id.is_empty(), "id defaults to empty string");
    assert!(
        entry.profile_snapshot.is_none(),
        "profile_snapshot defaults to None"
    );
}

/// ProfileSnapshot round-trips through serde.
#[test]
fn profile_snapshot_round_trips() {
    let snap = ProfileSnapshot {
        cleanup_provider: AiProviderId::OpenAi,
        cleanup_model: "gpt-4o-mini".to_string(),
        cleanup_prompt_override: Some("translate to English".to_string()),
        use_snippets: true,
        correction_set_ids: vec!["cs-1".to_string(), "cs-2".to_string()],
    };
    let json = serde_json::to_string(&snap).unwrap();
    let decoded: ProfileSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.cleanup_provider, AiProviderId::OpenAi);
    assert_eq!(decoded.cleanup_model, "gpt-4o-mini");
    assert_eq!(
        decoded.cleanup_prompt_override.as_deref(),
        Some("translate to English")
    );
    assert!(decoded.use_snippets);
    assert_eq!(decoded.correction_set_ids, vec!["cs-1", "cs-2"]);
}
