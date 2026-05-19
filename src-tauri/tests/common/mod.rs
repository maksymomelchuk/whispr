#![allow(dead_code)]

use std::time::Duration;
use whispr_lib::{
    config::{CorrectionEntry, NamedCorrectionSet, Settings, SnippetEntry},
    history::CleanupStatus,
    mode::{Mode, SEED_MODE_DEFAULT_EN},
    pipeline::{self, CleanupOutput, Outcome},
};

const HARNESS_CORRECTION_SET_ID: &str = "harness-corrections";

/// Deadline enforced by the harness. Any pipeline run that exceeds this
/// causes the spawned task to be cancelled and the test to fail with a
/// clear "harness deadline exceeded" message rather than hanging the suite.
pub const HARNESS_DEADLINE: Duration = Duration::from_secs(5);

/// Runs `work` on a blocking thread, bounded by `HARNESS_DEADLINE`. Panics
/// with a clear message if the deadline is exceeded or `work` panics, so a
/// truly infinite loop in the pipeline fails the test cleanly instead of
/// hanging the suite.
pub async fn run_under_deadline<F>(work: F) -> Outcome
where
    F: FnOnce() -> Outcome + Send + 'static,
{
    tokio::time::timeout(HARNESS_DEADLINE, tokio::task::spawn_blocking(work))
        .await
        .expect("harness deadline exceeded")
        .expect("spawn_blocking panicked")
}

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
    cleanup_error: Option<CleanupStatus>,
    /// Pre-translated text. When set, this text is used as the base for the
    /// cleanup stage (simulating what Apple Translate would have produced).
    translated_text: Option<String>,
    /// Override the active mode entirely (e.g. to test non-default languages).
    custom_mode: Option<Mode>,
}

impl PipelineHarness {
    pub fn new() -> Self {
        PipelineHarness {
            settings: Settings::default(),
            mode_id: SEED_MODE_DEFAULT_EN.to_string(),
            cleanup: None,
            cleanup_error: None,
            translated_text: None,
            custom_mode: None,
        }
    }

    /// Append a named correction set and link its ID to the active mode's
    /// `correction_set_ids`. May be chained to build multi-set scenarios.
    pub fn with_correction_set(mut self, id: &str, rules: &[(&str, &str)]) -> Self {
        let entries = rules
            .iter()
            .map(|(from, to)| CorrectionEntry {
                from: from.to_string(),
                to: to.to_string(),
            })
            .collect();
        self.settings.correction_sets.push(NamedCorrectionSet {
            id: id.to_string(),
            name: id.to_string(),
            entries,
        });
        if let Some(m) = self.active_mode_mut() {
            m.correction_set_ids.push(id.to_string());
        }
        self
    }

    /// Replace the active correction sets with a single harness set containing
    /// the given rules. Does not change the active mode's correction_set_ids;
    /// combine with `with_use_corrections` to enable/disable.
    pub fn with_corrections(mut self, rules: &[(&str, &str)]) -> Self {
        let entries = rules
            .iter()
            .map(|(from, to)| CorrectionEntry {
                from: from.to_string(),
                to: to.to_string(),
            })
            .collect();
        self.settings.correction_sets = vec![NamedCorrectionSet {
            id: HARNESS_CORRECTION_SET_ID.to_string(),
            name: "Harness Corrections".to_string(),
            entries,
        }];
        self
    }

    /// Add a named term set to settings and reference it from the active mode.
    pub fn with_term_set(mut self, id: &str, entries: &[&str]) -> Self {
        use whispr_lib::config::NamedTermSet;
        self.settings.term_sets.push(NamedTermSet {
            id: id.to_string(),
            name: id.to_string(),
            entries: entries.iter().map(|s| s.to_string()).collect(),
        });
        if let Some(m) = self.active_mode_mut() {
            m.term_set_ids.push(id.to_string());
        }
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

    /// Simulate a cleanup run that failed with the given status. The pipeline
    /// falls back to the raw transcript and records the failure status.
    pub fn with_cleanup_error(mut self, status: CleanupStatus) -> Self {
        self.cleanup_error = Some(status);
        self
    }

    /// Set the active mode's correction_set_ids to point at the harness correction
    /// set (enabled=true) or clear them (enabled=false).
    pub fn with_use_corrections(mut self, enabled: bool) -> Self {
        if let Some(m) = self.active_mode_mut() {
            m.correction_set_ids = if enabled {
                vec![HARNESS_CORRECTION_SET_ID.to_string()]
            } else {
                vec![]
            };
        }
        self
    }

    /// Override the `use_snippets` flag on the active mode.
    pub fn with_use_snippets(mut self, enabled: bool) -> Self {
        if let Some(m) = self.active_mode_mut() {
            m.use_snippets = enabled;
        }
        self
    }

    fn active_mode_mut(&mut self) -> Option<&mut Mode> {
        self.settings.modes.iter_mut().find(|m| m.id == self.mode_id)
    }

    /// Simulate the Apple Translate stage returning `translated_text`. When
    /// set, this text is the base passed to the cleanup stage (mirroring the
    /// real pipeline where translate feeds into cleanup). Without this call the
    /// harness passes `raw_text` directly to cleanup, i.e. translate is a no-op.
    pub fn with_translated_text(mut self, translated_text: &str) -> Self {
        self.translated_text = Some(translated_text.to_string());
        self
    }

    /// Override the active `Mode` used by `run_stages`. Use this to test
    /// pipeline behaviour under non-default language or translate configurations
    /// without needing a matching entry in `settings.modes`.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.custom_mode = Some(mode);
        self
    }

    /// Run the post-transcription pipeline stages against `raw_text` and
    /// return an `Outcome`. Intended to be called inside a
    /// `tokio::task::spawn_blocking` future wrapped with
    /// `tokio::time::timeout(HARNESS_DEADLINE, ...)` so that any infinite
    /// loop in the pipeline causes the test to fail cleanly.
    pub fn run(self, raw_text: &str) -> Outcome {
        let mode = if let Some(m) = self.custom_mode {
            m
        } else {
            self.settings
                .modes
                .iter()
                .find(|m| m.id == self.mode_id)
                .cloned()
                .expect("mode not found in settings")
        };

        // Translation (if simulated) feeds into cleanup, mirroring the real
        // pipeline: raw_text → translate → translated_text → cleanup → replaced_text.
        let post_translate = self.translated_text.as_deref().unwrap_or(raw_text);

        let cleanup_output = match (self.cleanup_error, self.cleanup) {
            (Some(status), _) => CleanupOutput {
                replaced_text: post_translate.to_string(),
                status,
            },
            (None, Some(cleaned)) => CleanupOutput {
                replaced_text: cleaned,
                status: CleanupStatus::Ran,
            },
            (None, None) => CleanupOutput {
                replaced_text: post_translate.to_string(),
                status: CleanupStatus::Disabled,
            },
        };

        pipeline::run_stages(raw_text, Duration::from_secs(1), &mode, &self.settings, cleanup_output)
    }
}

