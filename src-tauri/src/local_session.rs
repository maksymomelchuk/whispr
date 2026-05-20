use crate::groq_audio::{self, AUDIO_LEVEL_EVENT};
use crate::mode::{Mode, ModeLanguage};
use crate::provider::{self, LocalWhisperModel};
use crate::recorder::AudioFormat;
use crate::terms;
use crate::transcription_session::TranscriptionSession;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

const LEVEL_THROTTLE: Duration = Duration::from_millis(33);

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
            let k = if raw > smoothed_level { 0.6 } else { 0.25 };
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
        let path_buf = provider::local_model_path(&data_dir, self.model);
        let model_path = path_buf
            .to_str()
            .ok_or_else(|| "Model path is not valid UTF-8".to_string())?
            .to_string();
        let language_code = language.as_code().map(str::to_string);
        let initial_prompt = terms::groq_prompt_hint(&terms);
        tokio::task::spawn_blocking(move || {
            run_whisper(
                &model_path,
                &audio_f32,
                language_code.as_deref(),
                initial_prompt.as_deref(),
            )
        })
        .await
        .map_err(|e| format!("Whisper inference thread panicked: {e}"))?
        .map(|t| (t, speak_duration))
    }
}

fn run_whisper(
    model_path: &str,
    audio: &[f32],
    language: Option<&str>,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("Failed to load whisper model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create whisper state: {e}"))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    if let Some(code) = language {
        params.set_language(Some(code));
    }
    if let Some(prompt) = initial_prompt {
        params.set_initial_prompt(prompt);
    }
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, audio)
        .map_err(|e| format!("Whisper inference failed: {e}"))?;

    let n = state
        .full_n_segments()
        .map_err(|e| format!("Failed to get segment count: {e}"))?;

    let mut transcript = String::new();
    for i in 0..n {
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| format!("Failed to get segment text: {e}"))?;
        transcript.push_str(text.trim_start());
    }

    Ok(transcript.trim().to_string())
}
