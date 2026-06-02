//! Polling Groq transcription session.
//!
//! Captured audio is buffered in memory; every `POLL_INTERVAL` seconds the
//! trailing window is sent to Groq for a live-preview transcription. A
//! longest-stable-prefix stabilizer (see `groq_stabilizer`) keeps the overlay
//! text from flickering. On PTT release, the authoritative final transcript
//! comes from either (a) the in-flight poll if its window already covers the
//! full recording or (b) a fresh full-recording POST. Polling partials are
//! never stitched into the final.
//!
//! State-machine corner cases live in `groq_session_state::State`: this module
//! owns the buffer, the timer, and the HTTP requests.

use crate::audio_level_meter::AudioLevelMeter;
use crate::config::{self, CorrectionEntry};
use crate::corrections::compose_corrections;
use crate::groq_audio::{encode_to_flac_16k_mono, AUDIO_LEVEL_EVENT, TRANSCRIPT_PARTIAL_EVENT};
use crate::groq_session_state::{self, Action, Event, Phase, PollFailure, State};
use crate::mode::{Mode, ModeLanguage};
use crate::preview_throttle::PreviewThrottle;
use crate::provider::GroqModel;
use crate::recorder::AudioFormat;
use crate::terms;
use crate::transcription_session::TranscriptionSession;
use serde_json::Value;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

const TRANSCRIPTION_ERROR_EVENT: &str = "transcription-error";
const PTT_ERROR_EVENT: &str = "ptt-error";
/// Hold the overlay's red error icon for this long when surfacing the
/// "final POST failed → pasted last preview" soft warning — same value
/// `ptt::ERROR_FLASH` uses for cleanup failures.
const SOFT_WARNING_FLASH: Duration = Duration::from_millis(800);

pub struct GroqSession {
    pub model: GroqModel,
}

enum Outcome {
    Poll {
        id: u64,
        result: Result<String, PollFailure>,
    },
    Final(Result<String, String>),
}

enum GroqHttpError {
    RateLimited,
    Other(String),
}

impl TranscriptionSession for GroqSession {
    async fn run(
        self,
        app: AppHandle,
        format: AudioFormat,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        language: ModeLanguage,
        terms: Vec<String>,
        mode: &Mode,
    ) -> Result<(String, Duration), String> {
        let speak_start = Instant::now();
        let settings = config::load(&app);
        let key = settings
            .groq_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| "API key missing for Groq".to_string())?;
        let model = self.model.api_id();
        let language = language.as_code().map(str::to_string);
        let prompt = terms::groq_prompt_hint(&terms);
        let show_live_preview = settings.show_live_preview;
        let corrections = if mode.use_corrections {
            compose_corrections(&mode.correction_set_ids, &settings.correction_sets)
        } else {
            Vec::new()
        };

        let buffered: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_per_second: u64 = format.sample_rate as u64 * format.channels as u64;
        let (outcome_tx, mut outcome_rx) = tokio::sync::mpsc::unbounded_channel::<Outcome>();

        let mut runner = Runner {
            state: State::new(),
            final_text: None,
            fallback_text: None,
            hard_error: false,
            app: app.clone(),
            buffered: buffered.clone(),
            samples_per_second,
            format,
            key,
            model,
            language,
            prompt,
            show_live_preview,
            corrections,
            preview_throttle: PreviewThrottle::new(),
            outcome_tx: outcome_tx.clone(),
        };

        let mut level_meter = AudioLevelMeter::new();
        // `Skip` so a stalled await never produces a burst of make-up ticks once
        // it resumes. When live preview is off there is nothing to render between
        // polls, so we skip the timer entirely and let the final POST carry the
        // only Groq request.
        let mut poll_timer = if show_live_preview {
            let mut t = tokio::time::interval_at(
                tokio::time::Instant::now() + groq_session_state::POLL_INTERVAL,
                groq_session_state::POLL_INTERVAL,
            );
            t.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            Some(t)
        } else {
            None
        };

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => match maybe_chunk {
                    Some(chunk) => {
                        buffered.lock().unwrap().extend_from_slice(&chunk);
                        let now = Instant::now();
                        if let Some(level) = level_meter.observe(now, &chunk) {
                            let _ = app.emit(AUDIO_LEVEL_EVENT, level);
                        }
                    }
                    None => break,
                },
                _ = async { poll_timer.as_mut().unwrap().tick().await }, if poll_timer.is_some() => {
                    runner.step(Event::Tick { elapsed: speak_start.elapsed() });
                }
                Some(outcome) = outcome_rx.recv() => {
                    runner.step(outcome_to_event(outcome));
                }
            }
        }
        let speak_duration = speak_start.elapsed();
        // Settle the wave to flat — the pill stays up through the final POST.
        let _ = app.emit(AUDIO_LEVEL_EVENT, 0.0f32);

        runner.step(Event::PttReleased {
            elapsed: speak_duration,
        });

        // Every spawned task ends with an outcome send, so each recv() resolves
        // to Some(...) until state advances to Done.
        while runner.state.phase() != Phase::Done {
            let Some(outcome) = outcome_rx.recv().await else {
                break;
            };
            runner.step(outcome_to_event(outcome));
        }

        if runner.hard_error {
            return Err("Groq transcription failed".into());
        }
        if let Some(text) = runner.fallback_text {
            let _ = app.emit(PTT_ERROR_EVENT, ());
            let _ = app.emit(
                TRANSCRIPTION_ERROR_EVENT,
                "Final Groq transcription failed; pasting last live preview".to_string(),
            );
            tokio::time::sleep(SOFT_WARNING_FLASH).await;
            return Ok((text, speak_duration));
        }
        Ok((runner.final_text.unwrap_or_default(), speak_duration))
    }
}

struct Runner {
    state: State,
    final_text: Option<String>,
    fallback_text: Option<String>,
    hard_error: bool,
    app: AppHandle,
    buffered: Arc<Mutex<Vec<i16>>>,
    samples_per_second: u64,
    format: AudioFormat,
    key: String,
    model: &'static str,
    language: Option<String>,
    prompt: Option<String>,
    show_live_preview: bool,
    corrections: Vec<CorrectionEntry>,
    preview_throttle: PreviewThrottle,
    outcome_tx: UnboundedSender<Outcome>,
}

impl Runner {
    fn step(&mut self, event: Event) {
        let actions = self.state.step(event);
        for action in actions {
            self.apply(action);
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::DispatchPoll { id, window, .. } => {
                let window_samples =
                    (window.as_secs_f64() * self.samples_per_second as f64) as usize;
                spawn_poll(
                    id,
                    self.buffered.clone(),
                    window_samples,
                    self.format,
                    self.key.clone(),
                    self.model,
                    self.language.clone(),
                    self.prompt.clone(),
                    self.outcome_tx.clone(),
                );
            }
            Action::EmitPartial(text) => {
                if self.show_live_preview {
                    let now = Instant::now();
                    if let Some(corrected) = self.preview_throttle.offer(now, &text, &self.corrections) {
                        let _ = self.app.emit(TRANSCRIPT_PARTIAL_EVENT, &corrected);
                    }
                }
            }
            Action::DispatchFinal => {
                spawn_final_post(
                    self.buffered.clone(),
                    self.format,
                    self.key.clone(),
                    self.model,
                    self.language.clone(),
                    self.prompt.clone(),
                    self.outcome_tx.clone(),
                );
            }
            Action::PasteFinal(text) => {
                self.final_text = Some(text);
            }
            Action::PasteFallbackWithWarning(text) => {
                self.fallback_text = Some(text);
            }
            Action::HardError => {
                self.hard_error = true;
            }
            Action::LogRateLimited => {
                eprintln!("[groq] poll rate-limited; skipping until next tick");
            }
            Action::LogPollError => {
                eprintln!("[groq] poll failed; skipping until next tick");
            }
        }
    }
}

fn outcome_to_event(o: Outcome) -> Event {
    match o {
        Outcome::Poll { id, result } => match result {
            Ok(text) => Event::PollSucceeded { id, text },
            Err(kind) => Event::PollFailed { id, kind },
        },
        Outcome::Final(Ok(text)) => Event::FinalSucceeded { text },
        Outcome::Final(Err(_)) => Event::FinalFailed,
    }
}

fn spawn_poll(
    id: u64,
    buffered: Arc<Mutex<Vec<i16>>>,
    window_samples: usize,
    format: AudioFormat,
    key: String,
    model: &'static str,
    language: Option<String>,
    prompt: Option<String>,
    outcome_tx: UnboundedSender<Outcome>,
) {
    tauri::async_runtime::spawn(async move {
        let snapshot: Vec<i16> = {
            let buf = buffered.lock().unwrap();
            let start = buf.len().saturating_sub(window_samples);
            buf[start..].to_vec()
        };
        if snapshot.is_empty() {
            let _ = outcome_tx.send(Outcome::Poll {
                id,
                result: Ok(String::new()),
            });
            return;
        }
        let result = match encode_to_flac_16k_mono(&snapshot, format.sample_rate, format.channels) {
            Ok(flac) => {
                match post_to_groq(&key, model, language.as_deref(), prompt.as_deref(), flac).await
                {
                    Ok(text) => Ok(text),
                    Err(GroqHttpError::RateLimited) => Err(PollFailure::RateLimited),
                    Err(GroqHttpError::Other(msg)) => {
                        eprintln!("[groq poll {id}] {msg}");
                        Err(PollFailure::Other)
                    }
                }
            }
            Err(e) => {
                eprintln!("[groq poll {id}] FLAC encode failed: {e}");
                Err(PollFailure::Other)
            }
        };
        let _ = outcome_tx.send(Outcome::Poll { id, result });
    });
}

fn spawn_final_post(
    buffered: Arc<Mutex<Vec<i16>>>,
    format: AudioFormat,
    key: String,
    model: &'static str,
    language: Option<String>,
    prompt: Option<String>,
    outcome_tx: UnboundedSender<Outcome>,
) {
    tauri::async_runtime::spawn(async move {
        let snapshot: Vec<i16> = buffered.lock().unwrap().clone();
        if snapshot.is_empty() {
            let _ = outcome_tx.send(Outcome::Final(Ok(String::new())));
            return;
        }
        let result = match encode_to_flac_16k_mono(&snapshot, format.sample_rate, format.channels) {
            Ok(flac) => {
                match post_to_groq(&key, model, language.as_deref(), prompt.as_deref(), flac).await
                {
                    Ok(text) => Ok(text),
                    Err(GroqHttpError::RateLimited) => {
                        Err("Groq rate-limited the final POST".to_string())
                    }
                    Err(GroqHttpError::Other(msg)) => Err(msg),
                }
            }
            Err(e) => Err(format!("FLAC encode failed: {e}")),
        };
        let _ = outcome_tx.send(Outcome::Final(result));
    });
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn post_to_groq(
    key: &str,
    model: &str,
    // None for Auto and Hints — Whisper falls back to its own auto-detect.
    language: Option<&str>,
    prompt: Option<&str>,
    flac: Vec<u8>,
) -> Result<String, GroqHttpError> {
    let part = reqwest::multipart::Part::bytes(flac)
        .file_name("audio.flac")
        .mime_str("audio/flac")
        .map_err(|e| GroqHttpError::Other(format!("Groq mime build failed: {e}")))?;
    let mut form = reqwest::multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json");
    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }
    form = form.part("file", part);
    if let Some(p) = prompt {
        form = form.text("prompt", p.to_string());
    }

    let resp = http_client()
        .post(GROQ_URL)
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| GroqHttpError::Other(format!("Groq request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(GroqHttpError::RateLimited);
    }
    if !status.is_success() {
        return Err(GroqHttpError::Other(format_groq_error(status, &body)));
    }
    let text = parse_transcript(&body).map_err(GroqHttpError::Other)?;
    Ok(strip_prompt_echo(&text, prompt))
}

fn format_groq_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("Groq {status}")
    } else {
        format!("Groq {status}: {message}")
    }
}

// Whisper repeats its prompt when it can't transcribe short or silent audio.
// The vocabulary hint always begins with "Vocabulary: " — any transcript that
// starts with that prefix (or its comma variant produced by some Whisper
// variants) is an echo, not real speech.
fn strip_prompt_echo(text: &str, prompt: Option<&str>) -> String {
    if prompt.is_none() || text.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    if lower.starts_with("vocabulary:") || lower.starts_with("vocabulary,") {
        return String::new();
    }
    text.to_string()
}

fn parse_transcript(body: &str) -> Result<String, String> {
    let v: Value =
        serde_json::from_str(body).map_err(|e| format!("Groq response parse failed: {e}"))?;
    let text = v
        .get("text")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Groq response missing `text` field".to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_transcript_text() {
        let body = r#"{"text":"hello world"}"#;
        assert_eq!(parse_transcript(body).unwrap(), "hello world");
    }

    #[test]
    fn trims_whitespace_around_transcript() {
        let body = r#"{"text":"  hello world  "}"#;
        assert_eq!(parse_transcript(body).unwrap(), "hello world");
    }

    #[test]
    fn rejects_missing_text_field() {
        let body = r#"{"error":"not authorized"}"#;
        assert!(parse_transcript(body).is_err());
    }

    #[test]
    fn rejects_unparseable_response_body() {
        let body = "not json at all";
        assert!(parse_transcript(body).is_err());
    }

    #[test]
    fn extracts_groq_error_message_from_structured_body() {
        let body = r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#;
        let msg = format_groq_error(reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(
            msg.contains("Invalid API Key"),
            "expected message, got: {msg}"
        );
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_empty() {
        let msg = format_groq_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_not_json() {
        let msg = format_groq_error(reqwest::StatusCode::BAD_GATEWAY, "<html>nope</html>");
        assert!(msg.contains("502"), "expected status, got: {msg}");
        assert!(
            msg.contains("<html>nope</html>"),
            "expected snippet, got: {msg}"
        );
    }

    #[test]
    fn strip_prompt_echo_discards_vocabulary_colon_prefix() {
        let text = "Vocabulary: Claude Code, OAuth, UUID, JWT";
        assert_eq!(strip_prompt_echo(text, Some("Vocabulary: Claude Code")), "");
    }

    #[test]
    fn strip_prompt_echo_discards_vocabulary_comma_prefix() {
        // Whisper sometimes renders the colon as a comma
        let text = "Vocabulary, Claude Code, OAuth, UUID, JWT";
        assert_eq!(strip_prompt_echo(text, Some("Vocabulary: Claude Code")), "");
    }

    #[test]
    fn strip_prompt_echo_discards_lowercase_variant() {
        let text = "vocabulary: Claude Code, OAuth";
        assert_eq!(strip_prompt_echo(text, Some("Vocabulary: Claude Code")), "");
    }

    #[test]
    fn strip_prompt_echo_preserves_real_speech() {
        let text = "Let's build something fancy";
        assert_eq!(
            strip_prompt_echo(text, Some("Vocabulary: Claude Code")),
            "Let's build something fancy"
        );
    }

    #[test]
    fn strip_prompt_echo_preserves_vocabulary_in_context() {
        // "vocabulary" as a word mid-sentence is not a hallucination
        let text = "expand your vocabulary every day";
        assert_eq!(
            strip_prompt_echo(text, Some("Vocabulary: Claude Code")),
            "expand your vocabulary every day"
        );
    }

    #[test]
    fn strip_prompt_echo_passthrough_when_no_prompt() {
        let text = "Vocabulary: whatever";
        assert_eq!(strip_prompt_echo(text, None), "Vocabulary: whatever");
    }

    #[test]
    fn strip_prompt_echo_passthrough_for_empty_text() {
        assert_eq!(strip_prompt_echo("", Some("Vocabulary: foo")), "");
    }
}
