use crate::groq_audio;
use std::time::{Duration, Instant};

const LEVEL_THROTTLE: Duration = Duration::from_millis(33);
// Asymmetric EMA: fast attack so vowels punch, slow decay so the wave
// doesn't snap to silent between syllables.
const LEVEL_SMOOTH_RISE: f32 = 0.6;
const LEVEL_SMOOTH_FALL: f32 = 0.25;

pub struct AudioLevelMeter {
    smoothed: f32,
    last_emit: Option<Instant>,
}

impl AudioLevelMeter {
    pub fn new() -> Self {
        Self {
            smoothed: 0.0,
            last_emit: None,
        }
    }

    /// Returns `Some(level)` when the 33 ms throttle allows an emit; `None`
    /// otherwise.
    pub fn observe(&mut self, now: Instant, chunk: &[i16]) -> Option<f32> {
        let raw = groq_audio::compute_level(chunk);
        let k = if raw > self.smoothed {
            LEVEL_SMOOTH_RISE
        } else {
            LEVEL_SMOOTH_FALL
        };
        self.smoothed += (raw - self.smoothed) * k;
        if self
            .last_emit
            .map_or(true, |t| now.duration_since(t) >= LEVEL_THROTTLE)
        {
            self.last_emit = Some(now);
            Some(self.smoothed)
        } else {
            None
        }
    }
}

impl Default for AudioLevelMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn first_observe_always_emits() {
        let mut meter = AudioLevelMeter::new();
        assert!(meter.observe(Instant::now(), &[0i16; 100]).is_some());
    }

    #[test]
    fn within_throttle_window_returns_none() {
        let mut meter = AudioLevelMeter::new();
        let t0 = Instant::now();
        meter.observe(t0, &[0i16; 100]);
        assert!(meter.observe(t0 + millis(10), &[0i16; 100]).is_none());
    }

    #[test]
    fn after_throttle_window_emits_again() {
        let mut meter = AudioLevelMeter::new();
        let t0 = Instant::now();
        meter.observe(t0, &[0i16; 100]);
        assert!(meter.observe(t0 + millis(34), &[0i16; 100]).is_some());
    }

    #[test]
    fn ema_rise_coefficient_is_0_6() {
        let chunk: Vec<i16> = vec![16384i16; 160];
        let expected_raw = groq_audio::compute_level(&chunk);

        let mut meter = AudioLevelMeter::new();
        let level = meter.observe(Instant::now(), &chunk).unwrap();

        let expected = expected_raw * LEVEL_SMOOTH_RISE;
        assert!(
            (level - expected).abs() < 0.001,
            "expected {expected}, got {level}"
        );
    }

    #[test]
    fn ema_fall_coefficient_is_0_25() {
        let loud: Vec<i16> = vec![16384i16; 160];
        let t0 = Instant::now();

        let mut meter = AudioLevelMeter::new();
        let initial = meter.observe(t0, &loud).unwrap();

        let after = meter.observe(t0 + millis(34), &[0i16; 160]).unwrap();
        let expected = initial * (1.0 - LEVEL_SMOOTH_FALL);
        assert!(
            (after - expected).abs() < 0.001,
            "expected {expected}, got {after}"
        );
    }

    #[test]
    fn empty_chunk_returns_zero_level() {
        let mut meter = AudioLevelMeter::new();
        let level = meter.observe(Instant::now(), &[]).unwrap();
        assert_eq!(level, 0.0);
    }
}
