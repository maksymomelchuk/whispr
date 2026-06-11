use crate::cleanup::CleanupError;
#[cfg(test)]
use crate::cleanup::Transport;
use crate::cleanup_invoke;
use crate::config::Settings;
use crate::corrections::{apply_corrections, compose_corrections};
use crate::history::{CleanupStatus, HistoryEntry, ProfileSnapshot};
use crate::pipeline::Outcome;
use crate::snippets::expand_snippets;
#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

/// Returns true only when `entry` carries a stored Profile snapshot, a stable
/// id, and a cleanup status from which recovery is meaningful: one of the four
/// failure statuses. False for `Ran`, `Disabled`, both `Skipped*`,
/// `RecoveredManually`, and pre-upgrade entries without a snapshot.
pub fn is_recoverable(entry: &HistoryEntry) -> bool {
    let has_snapshot_and_id = entry.profile_snapshot.is_some() && !entry.id.is_empty();
    let is_failed = matches!(
        entry.cleanup_status,
        CleanupStatus::FailedTimeout
            | CleanupStatus::FailedTransient(_)
            | CleanupStatus::FailedCredential(_)
            | CleanupStatus::NoCredential
    );
    is_failed && has_snapshot_and_id
}

/// Re-runs AI cleanup and the full post-cleanup pipeline on `entry`, bypassing
/// min-words / min-duration / cleanup-enabled gates. Snapshot provider, model,
/// and prompt are used for the cleanup call; live Snippet and Correction
/// definitions are read from `settings`; which correction sets apply comes from
/// the stored snapshot. Returns the recovered `Outcome` on success; each
/// `CleanupError` variant propagates unchanged.
pub async fn recover_entry(
    entry: &HistoryEntry,
    settings: &Settings,
) -> Result<Outcome, CleanupError> {
    let snapshot = require_snapshot(entry)?;
    let start = Instant::now();

    let (replaced_text, _usage) = cleanup_invoke::invoke(
        &settings.ai_cleanup,
        snapshot.cleanup_provider.clone(),
        &snapshot.cleanup_model,
        snapshot.cleanup_prompt_override.as_deref(),
        None,
        &[],
        None,
        &entry.raw_text,
    )
    .await?;

    Ok(build_outcome(
        entry,
        snapshot,
        replaced_text,
        settings,
        start,
    ))
}

#[cfg(test)]
pub(crate) async fn recover_entry_with_transport<T: Transport>(
    entry: &HistoryEntry,
    settings: &Settings,
    transport: &T,
    timeout: Duration,
) -> Result<Outcome, CleanupError> {
    let snapshot = require_snapshot(entry)?;
    let start = Instant::now();

    let (replaced_text, _usage) = cleanup_invoke::invoke_with_transport(
        &settings.ai_cleanup,
        snapshot.cleanup_provider.clone(),
        &snapshot.cleanup_model,
        snapshot.cleanup_prompt_override.as_deref(),
        None,
        &[],
        None,
        &entry.raw_text,
        transport,
        timeout,
    )
    .await?;

    Ok(build_outcome(
        entry,
        snapshot,
        replaced_text,
        settings,
        start,
    ))
}

fn require_snapshot(entry: &HistoryEntry) -> Result<&ProfileSnapshot, CleanupError> {
    entry.profile_snapshot.as_ref().ok_or_else(|| {
        CleanupError::Transient("recovery requires a stored profile snapshot".to_string())
    })
}

fn build_outcome(
    entry: &HistoryEntry,
    snapshot: &ProfileSnapshot,
    replaced_text: String,
    settings: &Settings,
    start: Instant,
) -> Outcome {
    let mut final_text = replaced_text.clone();
    if snapshot.use_snippets {
        final_text = expand_snippets(&final_text, &settings.snippets);
    }
    if !snapshot.correction_set_ids.is_empty() || !settings.learned_entries.is_empty() {
        let corrections = compose_corrections(
            &snapshot.correction_set_ids,
            &settings.correction_sets,
            &settings.learned_entries,
        );
        final_text = apply_corrections(&final_text, &corrections);
    }

    let pasted_text = format!("{final_text} ");

    let history_entry = HistoryEntry {
        id: entry.id.clone(),
        timestamp: entry.timestamp,
        speak_duration_ms: entry.speak_duration_ms,
        raw_text: entry.raw_text.clone(),
        replaced_text,
        final_text,
        cleanup_status: CleanupStatus::RecoveredManually,
        profile_snapshot: entry.profile_snapshot.clone(),
        provider_model: entry.provider_model.clone(),
        app_name: entry.app_name.clone(),
        bundle_id: entry.bundle_id.clone(),
        context_channels: vec![],
    };

    Outcome {
        pasted_text,
        history_entry,
        elapsed: start.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::{AiProviderId, CleanupError, Transport, TransportResponse};
    use crate::config::{CorrectionEntry, NamedCorrectionSet, Settings, SnippetEntry};
    use crate::history::{CleanupStatus, HistoryEntry, ProfileSnapshot};

    struct MockTransport {
        response: Box<dyn Fn() -> Result<TransportResponse, String> + Send + Sync>,
    }

    impl MockTransport {
        fn returning(status: u16, body: impl Into<String>) -> Self {
            let body = body.into();
            MockTransport {
                response: Box::new(move || {
                    Ok(TransportResponse {
                        status,
                        body: body.clone(),
                    })
                }),
            }
        }

        fn failing(err: impl Into<String>) -> Self {
            let err = err.into();
            MockTransport {
                response: Box::new(move || Err(err.clone())),
            }
        }
    }

    impl Transport for MockTransport {
        fn post<'a>(
            &'a self,
            _url: &'a str,
            _headers: &'a [(String, String)],
            _body: &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<TransportResponse, String>> + Send + 'a>,
        > {
            let result = (self.response)();
            Box::pin(async move { result })
        }
    }

    struct HangingTransport;

    impl Transport for HangingTransport {
        fn post<'a>(
            &'a self,
            _url: &'a str,
            _headers: &'a [(String, String)],
            _body: &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<TransportResponse, String>> + Send + 'a>,
        > {
            Box::pin(std::future::pending())
        }
    }

    fn make_snapshot() -> ProfileSnapshot {
        ProfileSnapshot {
            cleanup_provider: AiProviderId::Anthropic,
            cleanup_model: "claude-haiku-4-5".to_string(),
            cleanup_prompt_override: None,
            use_snippets: false,
            correction_set_ids: vec![],
        }
    }

    fn make_entry(status: CleanupStatus) -> HistoryEntry {
        HistoryEntry {
            id: "test-entry-id".to_string(),
            timestamp: 1_000_000,
            speak_duration_ms: 5_000,
            raw_text: "raw transcript text".to_string(),
            replaced_text: "raw transcript text".to_string(),
            final_text: "raw transcript text".to_string(),
            cleanup_status: status,
            profile_snapshot: Some(make_snapshot()),
            provider_model: None,
            app_name: None,
            bundle_id: None,
            context_channels: vec![],
        }
    }

    fn settings_with_anthropic_key() -> Settings {
        let mut settings = Settings::default();
        settings
            .ai_cleanup
            .provider_keys
            .insert("anthropic".to_string(), "sk-ant-test".to_string());
        settings
    }

    fn anthropic_success_body(text: &str) -> String {
        serde_json::json!({
            "content": [{"type": "text", "text": text}],
            "usage": {
                "input_tokens": 10,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "output_tokens": 5
            }
        })
        .to_string()
    }

    const FIVE_SECS: Duration = Duration::from_secs(5);

    // --- is_recoverable: true cases ---

    #[test]
    fn failed_statuses_with_snapshot_and_id_are_recoverable() {
        let statuses = [
            CleanupStatus::FailedTimeout,
            CleanupStatus::FailedTransient("net error".to_string()),
            CleanupStatus::FailedCredential("bad key".to_string()),
            CleanupStatus::NoCredential,
        ];
        for status in statuses {
            let entry = make_entry(status);
            assert!(is_recoverable(&entry));
        }
    }

    // --- is_recoverable: false cases ---

    #[test]
    fn non_failed_statuses_are_not_recoverable() {
        let statuses = [
            CleanupStatus::Ran,
            CleanupStatus::Disabled,
            CleanupStatus::SkippedBelowMinWords,
            CleanupStatus::SkippedBelowMinDuration,
            CleanupStatus::RecoveredManually,
        ];
        for status in statuses {
            let entry = make_entry(status);
            assert!(!is_recoverable(&entry));
        }
    }

    #[test]
    fn failed_entry_without_snapshot_is_not_recoverable() {
        let mut entry = make_entry(CleanupStatus::FailedTimeout);
        entry.profile_snapshot = None;
        assert!(!is_recoverable(&entry));
    }

    #[test]
    fn failed_entry_without_id_is_not_recoverable() {
        let mut entry = make_entry(CleanupStatus::FailedTimeout);
        entry.id = String::new();
        assert!(!is_recoverable(&entry));
    }

    // --- recover_entry_with_transport: success cases ---

    #[tokio::test]
    async fn successful_recovery_returns_cleaned_text() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::FailedTimeout);
        let transport = MockTransport::returning(200, anthropic_success_body("Cleaned output."));

        let outcome = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS)
            .await
            .expect("recovery should succeed");

        assert_eq!(outcome.history_entry.final_text, "Cleaned output.");
        assert_eq!(outcome.pasted_text, "Cleaned output. ");
    }

    #[tokio::test]
    async fn recovery_sets_status_to_recovered_manually() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::FailedTimeout);
        let transport = MockTransport::returning(200, anthropic_success_body("text"));

        let outcome = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS)
            .await
            .expect("should succeed");

        assert!(matches!(
            outcome.history_entry.cleanup_status,
            CleanupStatus::RecoveredManually
        ));
    }

    #[tokio::test]
    async fn recovery_preserves_entry_id_and_raw_text() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::NoCredential);
        let transport = MockTransport::returning(200, anthropic_success_body("cleaned"));

        let outcome = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS)
            .await
            .expect("should succeed");

        assert_eq!(outcome.history_entry.id, "test-entry-id");
        assert_eq!(outcome.history_entry.raw_text, "raw transcript text");
    }

    #[tokio::test]
    async fn recovery_applies_snippets_when_snapshot_enables_them() {
        let mut settings = settings_with_anthropic_key();
        settings.snippets = vec![SnippetEntry {
            id: "s1".to_string(),
            trigger: "ty".to_string(),
            expansion: "thank you".to_string(),
        }];

        let mut entry = make_entry(CleanupStatus::FailedTimeout);
        if let Some(snap) = entry.profile_snapshot.as_mut() {
            snap.use_snippets = true;
        }

        let transport = MockTransport::returning(200, anthropic_success_body("ty for helping"));

        let outcome = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS)
            .await
            .expect("should succeed");

        assert_eq!(outcome.history_entry.final_text, "thank you for helping");
        assert_eq!(outcome.pasted_text, "thank you for helping ");
    }

    #[tokio::test]
    async fn recovery_applies_corrections_from_snapshot_sets() {
        let correction_set_id = "cs-test";
        let mut settings = settings_with_anthropic_key();
        settings.correction_sets = vec![NamedCorrectionSet {
            id: correction_set_id.to_string(),
            name: "Test".to_string(),
            entries: vec![CorrectionEntry {
                from: "teh".to_string(),
                to: "the".to_string(),
            }],
        }];

        let mut entry = make_entry(CleanupStatus::FailedTransient("net error".to_string()));
        if let Some(snap) = entry.profile_snapshot.as_mut() {
            snap.correction_set_ids = vec![correction_set_id.to_string()];
        }

        let transport =
            MockTransport::returning(200, anthropic_success_body("fix teh spelling here"));

        let outcome = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS)
            .await
            .expect("should succeed");

        assert_eq!(outcome.history_entry.final_text, "fix the spelling here");
    }

    // --- recover_entry_with_transport: error propagation ---

    #[tokio::test]
    async fn timeout_error_propagates_unchanged() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::FailedTimeout);

        let result = recover_entry_with_transport(
            &entry,
            &settings,
            &HangingTransport,
            Duration::from_millis(50),
        )
        .await;

        assert!(
            matches!(result, Err(CleanupError::Timeout(_))),
            "expected Timeout, got {result:?}"
        );
    }

    #[tokio::test]
    async fn credential_error_propagates_unchanged() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::FailedCredential("bad key".to_string()));
        let transport = MockTransport::returning(
            401,
            serde_json::json!({"error": {"message": "invalid key"}}).to_string(),
        );

        let result = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS).await;

        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn transient_error_propagates_unchanged() {
        let settings = settings_with_anthropic_key();
        let entry = make_entry(CleanupStatus::FailedTransient("net error".to_string()));
        let transport = MockTransport::failing("connection reset");

        let result = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS).await;

        assert!(
            matches!(result, Err(CleanupError::Transient(_))),
            "expected Transient error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn missing_snapshot_returns_transient_error() {
        let settings = settings_with_anthropic_key();
        let mut entry = make_entry(CleanupStatus::FailedTimeout);
        entry.profile_snapshot = None;
        let transport = MockTransport::returning(200, anthropic_success_body("irrelevant"));

        let result = recover_entry_with_transport(&entry, &settings, &transport, FIVE_SECS).await;

        assert!(
            matches!(result, Err(CleanupError::Transient(_))),
            "expected Transient error for missing snapshot, got {result:?}"
        );
    }
}
