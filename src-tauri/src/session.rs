use crate::audio_level_meter::AudioLevelMeter;
use crate::config::CorrectionEntry;
use crate::engine::{Engine, EngineContext, Warning};
use crate::groq_audio::{AUDIO_LEVEL_EVENT, TRANSCRIPT_PARTIAL_EVENT};
use crate::preview_throttle::PreviewThrottle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedReceiver;

const PTT_ERROR_EVENT: &str = "ptt-error";
const WARNING_FLASH: Duration = Duration::from_millis(800);

pub struct Session<E: Engine> {
    engine: E,
    app: AppHandle,
    show_live_preview: bool,
    corrections: Vec<CorrectionEntry>,
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
        }
    }

    pub async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        ctx: EngineContext,
    ) -> Result<(String, Duration), String> {
        let speak_start = Instant::now();

        let (engine_chunk_tx, engine_chunk_rx) = tokio::sync::mpsc::unbounded_channel();
        let (preview_tx, mut preview_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<Instant>();

        let app_meter = self.app.clone();
        let meter_handle = tokio::spawn(async move {
            let mut level_meter = AudioLevelMeter::new();
            while let Some(chunk) = chunks.recv().await {
                let now = Instant::now();
                if let Some(level) = level_meter.observe(now, &chunk) {
                    let _ = app_meter.emit(AUDIO_LEVEL_EVENT, level);
                }
                let _ = engine_chunk_tx.send(chunk);
            }
            let _ = close_tx.send(Instant::now());
            let _ = app_meter.emit(AUDIO_LEVEL_EVENT, 0.0f32);
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

        let _ = meter_handle.await;
        let _ = preview_handle.await;

        let chunks_closed_at = close_rx.await.unwrap_or_else(|_| Instant::now());
        let speak_duration = chunks_closed_at.duration_since(speak_start);

        if let Some(warning) = outcome.warning {
            match warning {
                Warning::FinalFailedUsedPreview => {
                    let _ = self.app.emit(PTT_ERROR_EVENT, ());
                    tokio::time::sleep(WARNING_FLASH).await;
                }
            }
        }

        Ok((outcome.transcript, speak_duration))
    }
}
