use std::time::Duration;
use whispr_lib::{
    config::{CorrectionEntry, Settings, SnippetEntry},
    history::CleanupStatus,
    mode::SEED_MODE_DEFAULT_EN,
    pipeline::{self, CleanupOutput, Outcome},
};

/// Deadline enforced by the harness. Any pipeline run that exceeds this
/// causes the spawned task to be cancelled and the test to fail with a
/// clear "harness deadline exceeded" message rather than hanging the suite.
pub const HARNESS_DEADLINE: Duration = Duration::from_secs(5);

/// Builder-style fixture that runs the post-transcription pipeline stages
/// (snippet expansion, correction application, paste-text preparation)
/// against a preset raw transcript without any Tauri or macOS dependencies.
///
/// Use `PipelineHarness::new()` to start, chain `with_*` builders to
/// configure inputs, then call `run("raw transcript")` to get an `Outcome`.
#[derive(Default)]
pub struct PipelineHarness {
    settings: Settings,
    mode_id: String,
    cleanup: Option<String>,
}

impl PipelineHarness {
    pub fn new() -> Self {
        PipelineHarness {
            settings: Settings::default(),
            mode_id: SEED_MODE_DEFAULT_EN.to_string(),
            cleanup: None,
        }
    }

    /// Replace the active corrections list (discards the default correction
    /// entries seeded by `Settings::default()`).
    pub fn with_corrections(mut self, rules: &[(&str, &str)]) -> Self {
        self.settings.corrections = rules
            .iter()
            .map(|(from, to)| CorrectionEntry {
                from: from.to_string(),
                to: to.to_string(),
            })
            .collect();
        self
    }

    /// Replace the active terms list.
    pub fn with_terms(mut self, terms: &[&str]) -> Self {
        self.settings.terms = terms.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Replace the active snippets list.
    pub fn with_snippets(mut self, snippets: &[(&str, &str)]) -> Self {
        self.settings.snippets = snippets
            .iter()
            .enumerate()
            .map(|(i, (trigger, expansion))| SnippetEntry {
                id: format!("snippet-{i}"),
                trigger: trigger.to_string(),
                expansion: expansion.to_string(),
            })
            .collect();
        self
    }

    /// Simulate a successful cleanup run that returned `cleaned_text`.
    /// Without this call the harness treats cleanup as disabled.
    pub fn with_cleanup(mut self, cleaned_text: &str) -> Self {
        self.cleanup = Some(cleaned_text.to_string());
        self
    }

    /// Run the post-transcription pipeline stages against `raw_text` and
    /// return an `Outcome`. Intended to be called inside a
    /// `tokio::task::spawn_blocking` future wrapped with
    /// `tokio::time::timeout(HARNESS_DEADLINE, ...)` so that any infinite
    /// loop in the pipeline causes the test to fail cleanly.
    pub fn run(self, raw_text: &str) -> Outcome {
        let mode = self
            .settings
            .modes
            .iter()
            .find(|m| m.id == self.mode_id)
            .cloned()
            .expect("mode not found in settings");

        let cleanup_output = match self.cleanup {
            Some(cleaned) => CleanupOutput {
                replaced_text: cleaned,
                status: CleanupStatus::Ran,
            },
            None => CleanupOutput {
                replaced_text: raw_text.to_string(),
                status: CleanupStatus::Disabled,
            },
        };

        pipeline::run_stages(raw_text, Duration::from_secs(1), &mode, &self.settings, cleanup_output)
    }
}

/// Test-only stub implementing `TranscriptionSession` for integration tests
/// that exercise the full `run_session` path. Requires macOS because
/// `TranscriptionSession` and its dependencies are macOS-gated.
#[cfg(target_os = "macos")]
pub mod stub_transcription {
    use std::time::Duration;
    use tauri::AppHandle;
    use tokio::sync::mpsc::UnboundedReceiver;
    use whispr_lib::{
        mode::ModeLanguage,
        recorder::AudioFormat,
        transcription_session::TranscriptionSession,
    };

    /// A `TranscriptionSession` that returns a preset transcript or error
    /// without touching any audio hardware or network. The `AppHandle` passed
    /// to `run` is ignored (no events are emitted).
    pub struct StubTranscriptionSession {
        result: Result<(String, Duration), String>,
    }

    impl StubTranscriptionSession {
        pub fn returning(text: impl Into<String>, duration: Duration) -> Self {
            StubTranscriptionSession {
                result: Ok((text.into(), duration)),
            }
        }

        pub fn failing(message: impl Into<String>) -> Self {
            StubTranscriptionSession {
                result: Err(message.into()),
            }
        }
    }

    impl TranscriptionSession for StubTranscriptionSession {
        async fn run(
            self,
            _app: AppHandle,
            _format: AudioFormat,
            _chunks: UnboundedReceiver<Vec<i16>>,
            _language: ModeLanguage,
            _terms: Vec<String>,
        ) -> Result<(String, Duration), String> {
            self.result
        }
    }
}
