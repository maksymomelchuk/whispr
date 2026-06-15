use std::time::{Duration, Instant};

/// Stage label for the time the user spends holding the key and speaking. It is
/// reported on its own line so it can be read apart from `latency` — the time
/// the app adds on top of speaking, which is the number worth optimizing.
const SPEAK_STAGE: &str = "speak";

/// Wall-clock breakdown of one dictation session, from session start to paste.
/// The caller records each pipeline stage as its `await` completes; `finish`
/// emits a single `[profile]` line to stderr.
pub struct SessionProfile {
    start: Instant,
    stages: Vec<(&'static str, Duration)>,
}

impl SessionProfile {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            stages: Vec::new(),
        }
    }

    pub fn record(&mut self, stage: &'static str, elapsed: Duration) {
        self.stages.push((stage, elapsed));
    }

    pub fn record_speak(&mut self, elapsed: Duration) {
        self.record(SPEAK_STAGE, elapsed);
    }

    pub fn finish(self) {
        let total = self.start.elapsed();
        let speak: Duration = self
            .stages
            .iter()
            .filter(|(label, _)| *label == SPEAK_STAGE)
            .map(|(_, dur)| *dur)
            .sum();
        let latency = total.saturating_sub(speak);

        let breakdown = self
            .stages
            .iter()
            .map(|(label, dur)| format!("{label}={}ms", dur.as_millis()))
            .collect::<Vec<_>>()
            .join(" ");

        eprintln!(
            "[profile] total={}ms speak={}ms latency={}ms | {breakdown}",
            total.as_millis(),
            speak.as_millis(),
            latency.as_millis(),
        );
    }
}
