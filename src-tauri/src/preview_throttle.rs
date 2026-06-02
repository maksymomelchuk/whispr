use crate::config::CorrectionEntry;
use crate::corrections::apply_corrections;
use std::time::{Duration, Instant};

/// Bounds the overlay re-render rate — interims can arrive faster than React
/// can usefully repaint.
const PARTIAL_THROTTLE: Duration = Duration::from_millis(100);

pub struct PreviewThrottle {
    last_emitted: String,
    last_emit: Option<Instant>,
}

impl PreviewThrottle {
    pub fn new() -> Self {
        Self {
            last_emitted: String::new(),
            last_emit: None,
        }
    }

    /// Applies `corrections` to `raw`, deduplicates against the last emission,
    /// and gates on the 100 ms throttle. Returns `Some(corrected)` when the
    /// caller should emit, `None` otherwise.
    pub fn offer(
        &mut self,
        now: Instant,
        raw: &str,
        corrections: &[CorrectionEntry],
    ) -> Option<String> {
        if self
            .last_emit
            .is_some_and(|t| now.duration_since(t) < PARTIAL_THROTTLE)
        {
            return None;
        }
        let corrected = apply_corrections(raw, corrections);
        if corrected == self.last_emitted {
            return None;
        }
        self.last_emitted = corrected.clone();
        self.last_emit = Some(now);
        Some(corrected)
    }
}

impl Default for PreviewThrottle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CorrectionEntry;

    fn entry(from: &str, to: &str) -> CorrectionEntry {
        CorrectionEntry {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn first_offer_emits() {
        let mut t = PreviewThrottle::new();
        assert_eq!(
            t.offer(Instant::now(), "hello world", &[]),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn duplicate_text_suppressed() {
        let mut t = PreviewThrottle::new();
        let t0 = Instant::now();
        t.offer(t0, "hello", &[]);
        assert_eq!(t.offer(t0 + millis(200), "hello", &[]), None);
    }

    #[test]
    fn within_throttle_window_suppressed() {
        let mut t = PreviewThrottle::new();
        let t0 = Instant::now();
        t.offer(t0, "hello", &[]);
        assert_eq!(t.offer(t0 + millis(50), "hello world", &[]), None);
    }

    #[test]
    fn emits_after_throttle_window_with_new_text() {
        let mut t = PreviewThrottle::new();
        let t0 = Instant::now();
        t.offer(t0, "hello", &[]);
        assert_eq!(
            t.offer(t0 + millis(101), "hello world", &[]),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn corrections_applied_to_raw_text() {
        let mut t = PreviewThrottle::new();
        let corrections = vec![entry("mongo", "MongoDB")];
        assert_eq!(
            t.offer(Instant::now(), "I use mongo", &corrections),
            Some("I use MongoDB".to_string())
        );
    }

    #[test]
    fn dedup_compares_corrected_text() {
        let mut t = PreviewThrottle::new();
        let t0 = Instant::now();
        let corrections = vec![entry("mongo", "MongoDB")];
        // Emits "I use MongoDB" (corrected)
        t.offer(t0, "I use mongo", &corrections);
        // Same corrected output — suppressed even though raw input matches
        assert_eq!(t.offer(t0 + millis(200), "I use mongo", &corrections), None);
    }

    #[test]
    fn no_corrections_passes_text_through() {
        let mut t = PreviewThrottle::new();
        assert_eq!(
            t.offer(Instant::now(), "plain text", &[]),
            Some("plain text".to_string())
        );
    }

    #[test]
    fn throttle_timer_resets_only_on_emission() {
        let mut t = PreviewThrottle::new();
        let t0 = Instant::now();
        t.offer(t0, "hello", &[]);
        // Within throttle — not emitted, timer not reset
        t.offer(t0 + millis(50), "hello world", &[]);
        // Past throttle from t0 — now emits
        assert_eq!(
            t.offer(t0 + millis(101), "hello world", &[]),
            Some("hello world".to_string())
        );
    }
}
