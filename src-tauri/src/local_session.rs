use crate::groq_audio::{self, AUDIO_LEVEL_EVENT};
use crate::mode::{Mode, ModeLanguage};
use crate::provider::{self, LocalWhisperModel};
use crate::recorder::AudioFormat;
use crate::state::{AppState, LocalEngine, LoadedModel};
use crate::terms;
use crate::transcription_session::TranscriptionSession;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::UnboundedReceiver;
use transcribe_rs::parakeet_onnx::ParakeetEngine;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

const LEVEL_THROTTLE: Duration = Duration::from_millis(33);
// Exponential smoothing coefficients for audio level: faster rise (0.6) to catch
// sudden loud sounds, slower decay (0.25) to avoid flickering on ambient noise.
const LEVEL_SMOOTH_RISE: f32 = 0.6;
const LEVEL_SMOOTH_FALL: f32 = 0.25;

pub struct LocalSession {
    pub model: LocalWhisperModel,
}

impl TranscriptionSession for LocalSession {
    async fn run(
        self,
        app: AppHandle,
        format: AudioFormat,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        language: ModeLanguage,
        terms: Vec<String>,
        _mode: &Mode,
    ) -> Result<(String, Duration), String> {
        let speak_start = Instant::now();
        let mut all_samples: Vec<i16> = Vec::new();
        let mut smoothed_level: f32 = 0.0;
        let mut last_level_emit: Option<Instant> = None;
        while let Some(chunk) = chunks.recv().await {
            let raw = groq_audio::compute_level(&chunk);
            let k = if raw > smoothed_level { LEVEL_SMOOTH_RISE } else { LEVEL_SMOOTH_FALL };
            smoothed_level += (raw - smoothed_level) * k;
            let now = Instant::now();
            if last_level_emit.map_or(true, |t| now.duration_since(t) >= LEVEL_THROTTLE) {
                let _ = app.emit(AUDIO_LEVEL_EVENT, smoothed_level);
                last_level_emit = Some(now);
            }
            all_samples.extend_from_slice(&chunk);
        }
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);
        let speak_duration = speak_start.elapsed();
        let audio_f32 = groq_audio::to_pcm_16k_mono_f32(
            &all_samples,
            format.sample_rate,
            format.channels,
        )?;
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Cannot resolve app data directory: {e}"))?;
        let model_path = provider::local_model_path(&data_dir, self.model);
        let language_code = language.as_code().map(str::to_string);
        let initial_prompt = terms::groq_prompt_hint(&terms);
        let model = self.model;
        let cache: Arc<Mutex<HashMap<LocalWhisperModel, LoadedModel>>> =
            app.state::<AppState>().model_cache.clone();
        tokio::task::spawn_blocking(move || {
            run_local_cached(
                &cache,
                model,
                &model_path,
                &audio_f32,
                language_code.as_deref(),
                initial_prompt.as_deref(),
            )
        })
        .await
        .map_err(|e| format!("Local inference thread panicked: {e}"))?
        .map(|t| (t, speak_duration))
    }
}

fn load_engine(model: LocalWhisperModel, model_path: &Path) -> Result<LocalEngine, String> {
    match model {
        LocalWhisperModel::Parakeet => {
            let models_dir = model_path.parent().ok_or("Cannot resolve models directory")?;
            let engine = ParakeetEngine::load(
                models_dir.join("parakeet-encoder.onnx"),
                models_dir.join("parakeet-decoder.onnx"),
                models_dir.join("parakeet-joiner.onnx"),
                models_dir.join("parakeet-vocab.json"),
            )
            .map_err(|e| format!("Failed to load Parakeet model: {e}"))?;
            Ok(LocalEngine::Parakeet(engine))
        }
        _ => {
            let engine = WhisperEngine::load(model_path)
                .map_err(|e| format!("Failed to load Whisper model: {e}"))?;
            Ok(LocalEngine::Whisper(engine))
        }
    }
}

fn run_local_cached(
    cache: &Mutex<HashMap<LocalWhisperModel, LoadedModel>>,
    model: LocalWhisperModel,
    model_path: &Path,
    audio: &[f32],
    language: Option<&str>,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let mut guard = cache.lock().unwrap();

    if !guard.contains_key(&model) {
        let engine = load_engine(model, model_path)?;
        guard.insert(model, LoadedModel { engine, last_used: Instant::now() });
    }

    let loaded = guard.get_mut(&model).unwrap();
    loaded.last_used = Instant::now();

    // `loaded.engine` borrows from `guard`; the guard must remain live for the
    // duration of inference. PTT sessions are serialized by the ptt_active flag,
    // so holding the mutex here does not cause contention.
    let text = match &loaded.engine {
        LocalEngine::Whisper(engine) => {
            let params = WhisperInferenceParams {
                language: language.map(str::to_string),
                initial_prompt: initial_prompt.map(str::to_string),
                ..Default::default()
            };
            engine
                .transcribe_with(audio, &params)
                .map_err(|e| format!("Whisper inference failed: {e}"))?
                .text
        }
        LocalEngine::Parakeet(engine) => {
            engine
                .transcribe(audio)
                .map_err(|e| format!("Parakeet inference failed: {e}"))?
                .text
        }
    };

    Ok(text.trim().to_string())
}
