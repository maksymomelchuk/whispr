//! Drives a single engine over a clip and times the final transcript.
//!
//! The feeder runs concurrently with `engine.run`, so streaming engines
//! (Deepgram, AssemblyAI) receive audio progressively and finalize exactly as
//! they do during live dictation. Streaming clips are paced at real time;
//! batch clips are fed at once since the engine only buffers until end-of-stream.
//!
//! Latency is measured from the last audio chunk (≈ push-to-talk release) to the
//! final transcript — finalization lag for streaming, full upload+process for
//! batch — so the two are comparable as "wait after release". The 100 ms chunk
//! size stays inside AssemblyAI's 50–1000 ms per-message window.

use crate::assemblyai_session::AssemblyAiEngine;
use crate::bench::clips::EngineSpec;
use crate::deepgram_session::DeepgramEngine;
use crate::elevenlabs_session::ElevenLabsEngine;
use crate::engine::{Engine, EngineContext};
use crate::groq_session::GroqEngine;
use crate::mode::ModeLanguage;
use crate::openai_transcribe_session::OpenAiTranscribeEngine;
use crate::recorder::AudioFormat;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::unbounded_channel;

const CHUNK_SAMPLES: usize = 1600;

pub struct EngineRun {
    pub transcript: String,
    pub latency: Duration,
}

pub async fn run(
    spec: EngineSpec,
    key: String,
    samples: &[i16],
    format: AudioFormat,
    language: ModeLanguage,
) -> Result<EngineRun, String> {
    let paced = spec.is_streaming();
    let (transcript, latency) = match spec {
        EngineSpec::Deepgram => {
            drive(DeepgramEngine::new(key), samples, format, language, paced).await
        }
        EngineSpec::Groq(model) => {
            drive(
                GroqEngine::new(model, key),
                samples,
                format,
                language,
                paced,
            )
            .await
        }
        EngineSpec::AssemblyAi(model) => {
            drive(
                AssemblyAiEngine::new(model, key),
                samples,
                format,
                language,
                paced,
            )
            .await
        }
        EngineSpec::OpenAi(model) => {
            drive(
                OpenAiTranscribeEngine::new(model, key),
                samples,
                format,
                language,
                paced,
            )
            .await
        }
        EngineSpec::ElevenLabs => {
            drive(ElevenLabsEngine::new(key), samples, format, language, paced).await
        }
    }?;
    Ok(EngineRun {
        transcript,
        latency,
    })
}

async fn drive<E: Engine>(
    engine: E,
    samples: &[i16],
    format: AudioFormat,
    language: ModeLanguage,
    paced: bool,
) -> Result<(String, Duration), String> {
    let (chunk_tx, chunk_rx) = unbounded_channel::<Vec<i16>>();
    let last_chunk_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    let chunks: Vec<Vec<i16>> = samples.chunks(CHUNK_SAMPLES).map(<[i16]>::to_vec).collect();
    let chunk_duration = chunk_playout(CHUNK_SAMPLES, format);
    let feeder_done = last_chunk_at.clone();
    let feeder = tokio::spawn(async move {
        for (index, chunk) in chunks.into_iter().enumerate() {
            if paced && index > 0 {
                tokio::time::sleep(chunk_duration).await;
            }
            if chunk_tx.send(chunk).is_err() {
                break;
            }
        }
        *feeder_done.lock().unwrap() = Some(Instant::now());
    });

    let (preview_tx, _preview_rx) = unbounded_channel::<String>();
    let context = EngineContext {
        format,
        language,
        terms: Vec::new(),
    };
    let outcome = engine.run(chunk_rx, preview_tx, context).await?;
    let _ = feeder.await;

    let released = last_chunk_at
        .lock()
        .unwrap()
        .ok_or("feeder never recorded the last chunk")?;
    Ok((outcome.transcript, released.elapsed()))
}

fn chunk_playout(samples: usize, format: AudioFormat) -> Duration {
    let frames = samples as f64 / format.channels.max(1) as f64;
    Duration::from_secs_f64(frames / format.sample_rate.max(1) as f64)
}
