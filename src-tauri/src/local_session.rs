use crate::audio_level_meter::AudioLevelMeter;
use crate::groq_audio::{self, AUDIO_LEVEL_EVENT};
use crate::mode::{Mode, ModeLanguage};
use crate::provider::{self, LocalWhisperModel};
use crate::recorder::AudioFormat;
use crate::state::{AppState, LoadedModel, LocalEngine};
use crate::terms;
use crate::transcription_session::TranscriptionSession;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::UnboundedReceiver;
use transcribe_rs::onnx::parakeet::ParakeetModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};
use transcribe_rs::{SpeechModel, TranscribeOptions};

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
        let mut level_meter = AudioLevelMeter::new();
        while let Some(chunk) = chunks.recv().await {
            let now = Instant::now();
            if let Some(level) = level_meter.observe(now, &chunk) {
                let _ = app.emit(AUDIO_LEVEL_EVENT, level);
            }
            all_samples.extend_from_slice(&chunk);
        }
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);
        let speak_duration = speak_start.elapsed();
        let audio_f32 =
            groq_audio::to_pcm_16k_mono_f32(&all_samples, format.sample_rate, format.channels)?;
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
            let models_dir = model_path
                .parent()
                .ok_or("Cannot resolve models directory")?;
            let engine = ParakeetModel::load(models_dir, &Quantization::Int8)
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
        guard.insert(
            model,
            LoadedModel {
                engine,
                last_used: Instant::now(),
            },
        );
    }

    let loaded = guard.get_mut(&model).unwrap();
    loaded.last_used = Instant::now();

    // `loaded.engine` borrows from `guard`; the guard must remain live for the
    // duration of inference. PTT sessions are serialized by the ptt_active flag,
    // so holding the mutex here does not cause contention.
    let text = match &mut loaded.engine {
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
                .transcribe(audio, &TranscribeOptions::default())
                .map_err(|e| format!("Parakeet inference failed: {e}"))?
                .text
        }
    };

    Ok(text.trim().to_string())
}
