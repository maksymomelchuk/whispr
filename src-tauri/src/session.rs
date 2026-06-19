use crate::audio_level_meter::AudioLevelMeter;
use crate::config::CorrectionEntry;
use crate::engine::{Engine, EngineContext, Warning};
use crate::preview_throttle::PreviewThrottle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) const AUDIO_LEVEL_EVENT: &str = "audio-level";
pub(crate) const TRANSCRIPT_PARTIAL_EVENT: &str = "transcript-partial";
pub(crate) const PTT_ERROR_EVENT: &str = "ptt-error";
pub(crate) const TRANSCRIPTION_ERROR_EVENT: &str = "transcription-error";
const SOFT_WARNING_FLASH: Duration = Duration::from_millis(800);

/// Result of a transcription session. `speak_duration` is the time the user
/// held the key (recording start to stop). `audio` holds the full interleaved
/// mic buffer at capture rate when recording-to-disk is enabled.
pub struct SessionOutcome {
    pub transcript: String,
    pub speak_duration: Duration,
    pub audio: Option<Vec<i16>>,
}

pub struct Session<E: Engine> {
    engine: E,
    app: AppHandle,
    show_live_preview: bool,
    corrections: Vec<CorrectionEntry>,
    capture_audio: bool,
}

impl<E: Engine> Session<E> {
    pub fn new(
        engine: E,
        app: AppHandle,
        show_live_preview: bool,
        corrections: Vec<CorrectionEntry>,
    ) -> Self {
        Self {
            engine,
            app,
            show_live_preview,
            corrections,
            capture_audio: false,
        }
    }

    /// Tee a copy of every mic chunk so the session can be saved as FLAC after
    /// it finalizes. Off by default to avoid buffering audio nobody keeps.
    pub fn with_capture_audio(mut self, capture: bool) -> Self {
        self.capture_audio = capture;
        self
    }

    pub async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        ctx: EngineContext,
    ) -> Result<SessionOutcome, String> {
        let speak_start = Instant::now();

        let (engine_chunk_tx, engine_chunk_rx) = tokio::sync::mpsc::unbounded_channel();
        let (preview_tx, mut preview_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<Instant>();

        let app_meter = self.app.clone();
        let capture_audio = self.capture_audio;
        let meter_handle = tokio::spawn(async move {
            let mut level_meter = AudioLevelMeter::new();
            let mut recorded: Option<Vec<i16>> = capture_audio.then(Vec::new);
            while let Some(chunk) = chunks.recv().await {
                let now = Instant::now();
                if let Some(level) = level_meter.observe(now, &chunk) {
                    let _ = app_meter.emit(AUDIO_LEVEL_EVENT, level);
                }
                if let Some(buffer) = recorded.as_mut() {
                    buffer.extend_from_slice(&chunk);
                }
                let _ = engine_chunk_tx.send(chunk);
            }
            let _ = close_tx.send(Instant::now());
            let _ = app_meter.emit(AUDIO_LEVEL_EVENT, 0.0f32);
            recorded
        });

        let app_preview = self.app.clone();
        let show_preview = self.show_live_preview;
        let corrections = self.corrections;
        let preview_handle = tokio::spawn(async move {
            let mut throttle = PreviewThrottle::new();
            while let Some(raw) = preview_rx.recv().await {
                if !show_preview {
                    continue;
                }
                let now = Instant::now();
                if let Some(text) = throttle.offer(now, &raw, &corrections) {
                    let _ = app_preview.emit(TRANSCRIPT_PARTIAL_EVENT, &text);
                }
            }
        });

        let outcome = self.engine.run(engine_chunk_rx, preview_tx, ctx).await?;

        let audio = meter_handle.await.ok().flatten();
        let _ = preview_handle.await;

        let chunks_closed_at = close_rx.await.unwrap_or_else(|_| Instant::now());
        let speak_duration = chunks_closed_at.duration_since(speak_start);

        if let Some(Warning::FinalFailedUsedPreview) = outcome.warning {
            let _ = self.app.emit(PTT_ERROR_EVENT, ());
            let _ = self.app.emit(
                TRANSCRIPTION_ERROR_EVENT,
                "Final Groq transcription failed; pasting last live preview",
            );
            tokio::time::sleep(SOFT_WARNING_FLASH).await;
        }

        Ok(SessionOutcome {
            transcript: outcome.transcript,
            speak_duration,
            audio,
        })
    }
}
