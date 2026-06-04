use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio;
use crate::provider::LocalWhisperModel;
use crate::state::{InferenceBackend, LoadedModel};
use crate::terms;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use transcribe_rs::onnx::parakeet::ParakeetModel;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};
use transcribe_rs::{SpeechModel, TranscribeOptions};

pub struct LocalWhisperEngine {
    pub model: LocalWhisperModel,
    pub model_cache: Arc<Mutex<HashMap<LocalWhisperModel, LoadedModel>>>,
    pub model_path: PathBuf,
}

impl LocalWhisperEngine {
    pub fn new(
        model: LocalWhisperModel,
        model_cache: Arc<Mutex<HashMap<LocalWhisperModel, LoadedModel>>>,
        model_path: PathBuf,
    ) -> Self {
        Self {
            model,
            model_cache,
            model_path,
        }
    }
}

impl Engine for LocalWhisperEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        _previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        let mut all_samples: Vec<i16> = Vec::new();
        while let Some(chunk) = chunks.recv().await {
            all_samples.extend_from_slice(&chunk);
        }
        let audio_f32 = groq_audio::to_pcm_16k_mono_f32(
            &all_samples,
            ctx.format.sample_rate,
            ctx.format.channels,
        )?;
        let language_code = ctx.language.as_code().map(str::to_string);
        let initial_prompt = terms::whisper_prompt_hint(&ctx.terms);
        let model = self.model;
        let model_path = self.model_path;
        let cache = self.model_cache;
        let transcript = tokio::task::spawn_blocking(move || {
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
        .map_err(|e| format!("Local inference thread panicked: {e}"))??;

        Ok(EngineOutcome {
            transcript,
            warning: None,
        })
    }
}

fn load_inference_backend(
    model: LocalWhisperModel,
    model_path: &Path,
) -> Result<InferenceBackend, String> {
    match model {
        LocalWhisperModel::Parakeet => {
            let models_dir = model_path
                .parent()
                .ok_or("Cannot resolve models directory")?;
            let engine = ParakeetModel::load(models_dir, &Quantization::Int8)
                .map_err(|e| format!("Failed to load Parakeet model: {e}"))?;
            Ok(InferenceBackend::Parakeet(engine))
        }
        _ => {
            let engine = WhisperEngine::load(model_path)
                .map_err(|e| format!("Failed to load Whisper model: {e}"))?;
            Ok(InferenceBackend::Whisper(engine))
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
        let backend = load_inference_backend(model, model_path)?;
        guard.insert(
            model,
            LoadedModel {
                engine: backend,
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
        InferenceBackend::Whisper(engine) => {
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
        InferenceBackend::Parakeet(engine) => {
            engine
                .transcribe(audio, &TranscribeOptions::default())
                .map_err(|e| format!("Parakeet inference failed: {e}"))?
                .text
        }
    };

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_whisper_engine_new_stores_fields() {
        let cache: Arc<Mutex<HashMap<LocalWhisperModel, LoadedModel>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let path = PathBuf::from("/tmp/model.bin");
        let engine = LocalWhisperEngine::new(LocalWhisperModel::LargeV3, cache, path.clone());
        assert_eq!(engine.model, LocalWhisperModel::LargeV3);
        assert_eq!(engine.model_path, path);
    }
}
