//! Pure (event, state) → (state, actions) reducer for the Groq polling
//! session's control logic.
//!
//! Extracted from `groq_session` so the corner cases — in-flight-poll reuse on
//! PTT release, 429 skip-but-keep-schedule, final-POST failure fallback to the
//! last stable preview — can be exercised without an HTTP client or audio
//! thread. The driving session owns the buffer, the timer, and the HTTP
//! requests; it pumps events into `step()` and dispatches the returned actions.

use crate::groq_stabilizer::Stabilizer;
use std::time::Duration;

/// First poll fires at this offset from session start; subsequent polls every
/// `POLL_INTERVAL` after the prior dispatch tick.
pub const POLL_INTERVAL: Duration = Duration::from_secs(3);
/// Each poll uploads at most this much trailing audio.
pub const POLL_WINDOW_MAX: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// PTT still held; audio buffering and polling are live.
    Recording,
    /// PTT released; the in-flight poll already covers the full recording, so
    /// its result will be the authoritative final.
    AwaitingCoveringPoll,
    /// PTT released; a full-recording POST is in flight as the authoritative
    /// final.
    AwaitingFinalPost,
    /// Terminal state — `step` becomes a no-op.
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InFlight {
    pub id: u64,
    pub dispatched_at: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollFailure {
    RateLimited,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Time advanced; if `>= next_poll_at` and no poll is in flight, the
    /// reducer dispatches a poll. Called from either a 3 s timer or a buffer
    /// observation in the host session.
    Tick { elapsed: Duration },
    PollSucceeded { id: u64, text: String },
    PollFailed { id: u64, kind: PollFailure },
    PttReleased { elapsed: Duration },
    FinalSucceeded { text: String },
    FinalFailed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Dispatch a poll with the trailing `window` of buffered audio. `elapsed`
    /// is recorded on the in-flight slot so the coverage check at PTT release
    /// is exact.
    DispatchPoll {
        id: u64,
        window: Duration,
        elapsed: Duration,
    },
    /// Emit `transcript-partial` to the overlay with the stabilizer's combined
    /// preview.
    EmitPartial(String),
    /// POST the full recording from t=0 for the authoritative final.
    DispatchFinal,
    /// Paste this as the final transcript.
    PasteFinal(String),
    /// The final POST failed but at least one poll succeeded — paste the last
    /// stable preview and surface a soft warning to the overlay.
    PasteFallbackWithWarning(String),
    /// Final POST failed and no polls succeeded — paste nothing, hard error.
    HardError,
    /// A poll was rate-limited; the next scheduled poll still fires on time.
    LogRateLimited,
    /// A poll failed for a non-429 reason; logged the same way.
    LogPollError,
}

#[derive(Debug)]
pub struct State {
    elapsed: Duration,
    next_poll_at: Duration,
    in_flight: Option<InFlight>,
    successful_polls: u32,
    last_partial: String,
    phase: Phase,
    next_poll_id: u64,
    stabilizer: Stabilizer,
}

impl State {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            next_poll_at: POLL_INTERVAL,
            in_flight: None,
            successful_polls: 0,
            last_partial: String::new(),
            phase: Phase::Recording,
            next_poll_id: 1,
            stabilizer: Stabilizer::new(),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn in_flight(&self) -> Option<&InFlight> {
        self.in_flight.as_ref()
    }

    pub fn successful_polls(&self) -> u32 {
        self.successful_polls
    }

    pub fn last_partial(&self) -> &str {
        &self.last_partial
    }

    pub fn next_poll_at(&self) -> Duration {
        self.next_poll_at
    }

    pub fn step(&mut self, event: Event) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::Tick { elapsed } => {
                self.elapsed = elapsed;
                if self.phase == Phase::Recording
                    && self.in_flight.is_none()
                    && elapsed >= self.next_poll_at
                {
                    let id = self.next_poll_id;
                    self.next_poll_id += 1;
                    let window = elapsed.min(POLL_WINDOW_MAX);
                    self.in_flight = Some(InFlight {
                        id,
                        dispatched_at: elapsed,
                    });
                    self.next_poll_at += POLL_INTERVAL;
                    actions.push(Action::DispatchPoll {
                        id,
                        window,
                        elapsed,
                    });
                }
            }
            Event::PollSucceeded { id, text } => {
                if !self.in_flight.as_ref().is_some_and(|p| p.id == id) {
                    return actions;
                }
                self.in_flight = None;
                self.successful_polls += 1;
                let partial = self.stabilizer.ingest(&text);
                self.last_partial = partial.clone();

                if self.phase == Phase::AwaitingCoveringPoll {
                    // The covering poll IS the final transcript. Polling
                    // partials are preview-only — but here the poll's window
                    // already spanned the whole recording, so its text is the
                    // authoritative final.
                    self.phase = Phase::Done;
                    actions.push(Action::PasteFinal(text));
                } else {
                    actions.push(Action::EmitPartial(partial));
                }
            }
            Event::PollFailed { id, kind } => {
                if !self.in_flight.as_ref().is_some_and(|p| p.id == id) {
                    return actions;
                }
                self.in_flight = None;
                if self.phase == Phase::AwaitingCoveringPoll {
                    // The poll we were going to reuse as final never returned
                    // a transcript — fall back to a full-recording POST.
                    self.phase = Phase::AwaitingFinalPost;
                    actions.push(Action::DispatchFinal);
                }
                actions.push(match kind {
                    PollFailure::RateLimited => Action::LogRateLimited,
                    PollFailure::Other => Action::LogPollError,
                });
            }
            Event::PttReleased { elapsed } => {
                if self.phase != Phase::Recording {
                    return actions;
                }
                self.elapsed = elapsed;
                let covering = self
                    .in_flight
                    .as_ref()
                    .is_some_and(|p| p.dispatched_at >= elapsed);
                if covering {
                    self.phase = Phase::AwaitingCoveringPoll;
                } else {
                    self.phase = Phase::AwaitingFinalPost;
                    actions.push(Action::DispatchFinal);
                }
            }
            Event::FinalSucceeded { text } => {
                if self.phase == Phase::AwaitingFinalPost {
                    self.phase = Phase::Done;
                    actions.push(Action::PasteFinal(text));
                }
            }
            Event::FinalFailed => {
                if self.phase == Phase::AwaitingFinalPost {
                    self.phase = Phase::Done;
                    if self.successful_polls > 0 {
                        actions.push(Action::PasteFallbackWithWarning(
                            self.last_partial.clone(),
                        ));
                    } else {
                        actions.push(Action::HardError);
                    }
                }
            }
        }
        actions
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }
    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn no_poll_dispatched_before_three_seconds() {
        let mut s = State::new();
        let actions = s.step(Event::Tick {
            elapsed: millis(2_900),
        });
        assert!(actions.is_empty());
        assert!(s.in_flight().is_none());
    }

    #[test]
    fn first_tick_at_three_seconds_dispatches_a_poll() {
        let mut s = State::new();
        let actions = s.step(Event::Tick { elapsed: secs(3) });
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::DispatchPoll {
                id: 1,
                window,
                elapsed,
            } => {
                assert_eq!(*window, secs(3));
                assert_eq!(*elapsed, secs(3));
            }
            other => panic!("expected DispatchPoll, got: {other:?}"),
        }
        let inflight = s.in_flight().expect("in_flight set");
        assert_eq!(inflight.id, 1);
        assert_eq!(inflight.dispatched_at, secs(3));
    }

    #[test]
    fn poll_window_caps_at_ten_seconds_once_elapsed_exceeds_max() {
        let mut s = State::new();
        // 12 s in without any prior polls: window should clamp to 10 s.
        let actions = s.step(Event::Tick { elapsed: secs(12) });
        match &actions[0] {
            Action::DispatchPoll { window, .. } => assert_eq!(*window, secs(10)),
            other => panic!("expected DispatchPoll, got: {other:?}"),
        }
    }

    #[test]
    fn successful_poll_emits_stabilized_partial_and_increments_counter() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        let actions = s.step(Event::PollSucceeded {
            id: 1,
            text: "hello world".into(),
        });
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::EmitPartial(t) => assert_eq!(t, "hello world"),
            other => panic!("expected EmitPartial, got: {other:?}"),
        }
        assert_eq!(s.successful_polls(), 1);
        assert_eq!(s.last_partial(), "hello world");
        assert!(s.in_flight().is_none());
    }

    #[test]
    fn stabilizer_revises_partials_across_polls() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        let a = s.step(Event::PollSucceeded {
            id: 1,
            text: "the quick brown".into(),
        });
        assert!(matches!(&a[0], Action::EmitPartial(t) if t == "the quick brown"));
        s.step(Event::Tick { elapsed: secs(6) });
        let a = s.step(Event::PollSucceeded {
            id: 2,
            text: "the quick brown fox".into(),
        });
        assert!(matches!(&a[0], Action::EmitPartial(t) if t == "the quick brown fox"));
        assert_eq!(s.last_partial(), "the quick brown fox");
    }

    #[test]
    fn stale_poll_result_is_ignored() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        // Pretend a result from a never-dispatched id 99 arrived.
        let actions = s.step(Event::PollSucceeded {
            id: 99,
            text: "ghost".into(),
        });
        assert!(actions.is_empty());
        assert_eq!(s.successful_polls(), 0);
        assert!(s.in_flight().is_some(), "real poll 1 still in-flight");
    }

    #[test]
    fn ptt_release_at_500ms_skips_polling_and_dispatches_final() {
        let mut s = State::new();
        s.step(Event::Tick {
            elapsed: millis(500),
        });
        let actions = s.step(Event::PttReleased {
            elapsed: millis(500),
        });
        assert_eq!(actions, vec![Action::DispatchFinal]);
        assert_eq!(s.phase(), Phase::AwaitingFinalPost);
        assert_eq!(s.successful_polls(), 0);
    }

    #[test]
    fn ptt_release_at_2500ms_into_first_cycle_skips_polling_and_dispatches_final() {
        let mut s = State::new();
        s.step(Event::Tick {
            elapsed: millis(2_500),
        });
        let actions = s.step(Event::PttReleased {
            elapsed: millis(2_500),
        });
        assert_eq!(actions, vec![Action::DispatchFinal]);
        assert_eq!(s.phase(), Phase::AwaitingFinalPost);
    }

    #[test]
    fn ptt_release_with_covering_poll_waits_for_it_and_reuses_text_as_final() {
        let mut s = State::new();
        // Poll fired at exactly 3 s; PTT released at the same elapsed, so the
        // in-flight poll's window covers the full recording.
        s.step(Event::Tick { elapsed: secs(3) });
        let actions = s.step(Event::PttReleased { elapsed: secs(3) });
        assert!(
            actions.is_empty(),
            "covering poll → wait, no final POST yet",
        );
        assert_eq!(s.phase(), Phase::AwaitingCoveringPoll);

        let actions = s.step(Event::PollSucceeded {
            id: 1,
            text: "complete transcript".into(),
        });
        assert_eq!(
            actions,
            vec![Action::PasteFinal("complete transcript".into())],
        );
        assert_eq!(s.phase(), Phase::Done);
    }

    #[test]
    fn ptt_release_with_non_covering_poll_dispatches_final_post() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        // 0.5 s of audio arrived after dispatch — the in-flight poll's window
        // doesn't reach the latest sample. Full-recording POST is required.
        let actions = s.step(Event::PttReleased {
            elapsed: millis(3_500),
        });
        assert_eq!(actions, vec![Action::DispatchFinal]);
        assert_eq!(s.phase(), Phase::AwaitingFinalPost);
    }

    #[test]
    fn rate_limited_poll_is_skipped_and_leaves_schedule_intact() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        let actions = s.step(Event::PollFailed {
            id: 1,
            kind: PollFailure::RateLimited,
        });
        assert_eq!(actions, vec![Action::LogRateLimited]);
        assert_eq!(s.successful_polls(), 0);
        assert_eq!(
            s.next_poll_at(),
            secs(6),
            "429 must not push the schedule back — next poll still fires at 6 s"
        );
        assert!(s.in_flight().is_none());

        // Next tick at 6 s should dispatch poll 2 unchanged.
        let actions = s.step(Event::Tick { elapsed: secs(6) });
        assert!(matches!(
            &actions[0],
            Action::DispatchPoll { id: 2, window, .. } if *window == secs(6)
        ));
    }

    #[test]
    fn covering_poll_failure_falls_back_to_full_recording_post() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        s.step(Event::PttReleased { elapsed: secs(3) });
        let actions = s.step(Event::PollFailed {
            id: 1,
            kind: PollFailure::RateLimited,
        });
        assert!(actions.contains(&Action::DispatchFinal));
        assert!(actions.contains(&Action::LogRateLimited));
        assert_eq!(s.phase(), Phase::AwaitingFinalPost);
    }

    #[test]
    fn final_post_success_pastes_authoritative_text() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        s.step(Event::PollSucceeded {
            id: 1,
            text: "preview".into(),
        });
        s.step(Event::PttReleased { elapsed: secs(4) });
        let actions = s.step(Event::FinalSucceeded {
            text: "authoritative full transcript".into(),
        });
        assert_eq!(
            actions,
            vec![Action::PasteFinal("authoritative full transcript".into())],
        );
        assert_eq!(s.phase(), Phase::Done);
    }

    #[test]
    fn final_post_failure_after_successful_polls_pastes_last_partial_with_warning() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        s.step(Event::PollSucceeded {
            id: 1,
            text: "best preview we got".into(),
        });
        s.step(Event::PttReleased { elapsed: secs(4) });
        let actions = s.step(Event::FinalFailed);
        assert_eq!(
            actions,
            vec![Action::PasteFallbackWithWarning(
                "best preview we got".into()
            )],
        );
        assert_eq!(s.phase(), Phase::Done);
    }

    #[test]
    fn final_post_failure_with_no_successful_polls_hard_errors() {
        let mut s = State::new();
        s.step(Event::Tick {
            elapsed: millis(500),
        });
        s.step(Event::PttReleased {
            elapsed: millis(500),
        });
        let actions = s.step(Event::FinalFailed);
        assert_eq!(actions, vec![Action::HardError]);
        assert_eq!(s.phase(), Phase::Done);
    }

    #[test]
    fn second_poll_is_not_dispatched_while_first_is_in_flight() {
        let mut s = State::new();
        s.step(Event::Tick { elapsed: secs(3) });
        // 6 s tick while poll 1 is still in-flight — no new dispatch.
        let actions = s.step(Event::Tick { elapsed: secs(6) });
        assert!(actions.is_empty());
        // Once poll 1 returns, the next tick (at 9 s) catches up on the
        // skipped slot and dispatches poll 2.
        s.step(Event::PollSucceeded {
            id: 1,
            text: "first".into(),
        });
        let actions = s.step(Event::Tick { elapsed: secs(9) });
        assert!(matches!(&actions[0], Action::DispatchPoll { id: 2, .. }));
    }

    #[test]
    fn polls_repeat_on_three_second_cadence() {
        let mut s = State::new();
        let a = s.step(Event::Tick { elapsed: secs(3) });
        assert!(matches!(&a[0], Action::DispatchPoll { id: 1, .. }));
        s.step(Event::PollSucceeded {
            id: 1,
            text: "one".into(),
        });
        let a = s.step(Event::Tick { elapsed: secs(6) });
        assert!(matches!(
            &a[0],
            Action::DispatchPoll { id: 2, window, .. } if *window == secs(6)
        ));
        s.step(Event::PollSucceeded {
            id: 2,
            text: "one two".into(),
        });
        let a = s.step(Event::Tick { elapsed: secs(9) });
        assert!(matches!(
            &a[0],
            Action::DispatchPoll { id: 3, window, .. } if *window == secs(9)
        ));
    }

    #[test]
    fn done_phase_is_terminal_for_further_events() {
        let mut s = State::new();
        s.step(Event::Tick {
            elapsed: millis(500),
        });
        s.step(Event::PttReleased {
            elapsed: millis(500),
        });
        s.step(Event::FinalSucceeded { text: "ok".into() });
        assert_eq!(s.phase(), Phase::Done);
        let actions = s.step(Event::Tick { elapsed: secs(99) });
        assert!(
            actions.is_empty(),
            "no further dispatches once Done is reached"
        );
    }
}
