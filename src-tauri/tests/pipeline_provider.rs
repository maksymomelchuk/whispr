/// Integration tests for the provider axis of the pipeline matrix.
///
/// Deepgram and Groq each deliver a final transcript to `run_stages` via
/// different mechanisms — Deepgram returns text in a single response;
/// Groq uses a polling state machine that may either (a) dispatch a separate
/// final POST or (b) reuse an in-flight poll that already covers the full
/// recording. These tests verify that, whatever path produced the transcript,
/// the post-transcription pipeline stages produce the expected `Outcome`.
#[path = "common/mod.rs"]
mod common;

use common::{run_under_deadline, PipelineHarness};
use std::time::Duration;
use whispr_lib::groq_session_state::{Action, Event, State as GroqState};

#[tokio::test]
async fn deepgram_preset_transcript_produces_pasted_text() {
    let outcome =
        run_under_deadline(|| PipelineHarness::new().run("transcribed by Deepgram")).await;

    assert_eq!(outcome.pasted_text, "transcribed by Deepgram ");
    assert_eq!(outcome.history_entry.raw_text, "transcribed by Deepgram");
}

#[tokio::test]
async fn deepgram_preset_with_corrections_applied() {
    let outcome = run_under_deadline(|| {
        PipelineHarness::new()
            .with_corrections(&[("deepgram", "Deepgram")])
            .with_use_corrections(true)
            .run("transcribed by deepgram provider")
    })
    .await;

    assert_eq!(outcome.pasted_text, "transcribed by Deepgram provider ");
}

#[tokio::test]
async fn groq_basic_preset_transcript_produces_pasted_text() {
    let outcome = run_under_deadline(|| PipelineHarness::new().run("transcribed by Groq")).await;

    assert_eq!(outcome.pasted_text, "transcribed by Groq ");
    assert_eq!(outcome.history_entry.raw_text, "transcribed by Groq");
}

/// Simulates the Groq per-poll state-machine variant where a poll dispatched at
/// 3 s covers the full recording (PTT released at exactly 3 s). The state
/// machine reuses that poll's text as the authoritative final transcript rather
/// than posting a separate full-recording request.
#[tokio::test]
async fn groq_covering_poll_produces_final_transcript_via_state_machine() {
    let mut state = GroqState::new();
    state.step(Event::Tick {
        elapsed: Duration::from_secs(3),
    });
    // PTT released at the same instant as the poll: the in-flight poll covers
    // the whole recording, so its result IS the final.
    state.step(Event::PttReleased {
        elapsed: Duration::from_secs(3),
    });
    let actions = state.step(Event::PollSucceeded {
        id: 1,
        text: "covering poll final transcript".into(),
    });

    let final_text = actions
        .into_iter()
        .find_map(|a| match a {
            Action::PasteFinal(t) => Some(t),
            _ => None,
        })
        .expect("covering poll should produce PasteFinal action");

    let outcome = run_under_deadline(move || PipelineHarness::new().run(&final_text)).await;

    assert_eq!(outcome.pasted_text, "covering poll final transcript ");
    assert_eq!(
        outcome.history_entry.raw_text,
        "covering poll final transcript"
    );
}

/// Simulates the Groq scenario where PTT is released before any poll fires,
/// so the state machine dispatches a full-recording final POST and its text
/// becomes the authoritative transcript.
#[tokio::test]
async fn groq_final_post_scenario_produces_pasted_text() {
    let mut state = GroqState::new();
    // No ticks — user releases PTT after only 500 ms (before the 3 s mark).
    state.step(Event::PttReleased {
        elapsed: Duration::from_millis(500),
    });
    let actions = state.step(Event::FinalSucceeded {
        text: "short Groq dictation".into(),
    });

    let final_text = actions
        .into_iter()
        .find_map(|a| match a {
            Action::PasteFinal(t) => Some(t),
            _ => None,
        })
        .expect("FinalSucceeded should produce PasteFinal action");

    let outcome = run_under_deadline(move || PipelineHarness::new().run(&final_text)).await;

    assert_eq!(outcome.pasted_text, "short Groq dictation ");
}
