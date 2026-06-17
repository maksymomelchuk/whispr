use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::to_pcm_16k_mono_bytes;
use crate::mode::ModeLanguage;
use crate::terms;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;

const SONIOX_WS_URL: &str = "wss://stt-rt.soniox.com/transcribe-websocket";
const SONIOX_RT_MODEL_ID: &str = "stt-rt-v5";

/// Capture audio is downmixed and resampled to 16 kHz mono PCM (signed 16-bit
/// little-endian) before it goes on the wire, so the config message declares a
/// fixed format regardless of the input device.
const REALTIME_SAMPLE_RATE: u32 = 16_000;
const REALTIME_NUM_CHANNELS: u32 = 1;
const AUDIO_FORMAT_PCM_S16LE: &str = "pcm_s16le";

/// Endpoint detection finalizes tokens progressively as the speaker pauses,
/// instead of leaving everything non-final until the PTT-release `finalize`.
/// This keeps the post-release paste near-instant and matches the Playground's
/// cadence. The delay is how long a pause must last before a segment is
/// committed; 500 ms (vs the 2000 ms default) trades a little stability for
/// dictation snappiness. Valid range is 500–3000 ms.
const MAX_ENDPOINT_DELAY_MS: u32 = 500;

/// Cap on how long we wait for Soniox to flush finalized tokens after we send
/// the finalize + end-of-audio frames, so a hung WS never blocks the paste.
const FINAL_RESULTS_TIMEOUT: Duration = Duration::from_secs(3);

/// Soniox emits these as standalone control tokens (endpoint / finalize
/// markers); they are never transcript content.
const CONTROL_TOKENS: [&str; 2] = ["<end>", "<fin>"];

pub struct SonioxRealtimeEngine {
    pub key: String,
    pub translate_to: Option<String>,
}

impl SonioxRealtimeEngine {
    pub fn new(key: String, translate_to: Option<String>) -> Self {
        Self { key, translate_to }
    }
}

impl Engine for SonioxRealtimeEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        let translating = self.translate_to.is_some();
        let config = build_config_message(
            &self.key,
            &ctx.language,
            self.translate_to.as_deref(),
            &ctx.terms,
        );

        let req = SONIOX_WS_URL
            .into_client_request()
            .map_err(|e| format!("bad WS URL: {e}"))?;
        let (ws, _resp) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("Soniox WS connect failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();

        sink.send(Message::Text(config))
            .await
            .map_err(|e| format!("Soniox config send failed: {e}"))?;

        let input_sample_rate = ctx.format.sample_rate;
        let input_channels = ctx.format.channels;
        let mut committed = String::new();

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => {
                    match maybe_chunk {
                        Some(chunk) => {
                            let pcm = to_pcm_16k_mono_bytes(&chunk, input_sample_rate, input_channels)
                                .map_err(|e| format!("Soniox audio resample failed: {e}"))?;
                            if let Err(e) = sink.send(Message::Binary(pcm)).await {
                                return Err(format!("Soniox WS send failed: {e}"));
                            }
                        }
                        None => break,
                    }
                }
                msg = stream.next() => {
                    let Some(msg) = msg else { return Err("Soniox WS closed mid-stream".into()); };
                    let msg = msg.map_err(|e| format!("Soniox WS recv failed: {e}"))?;
                    match msg {
                        Message::Text(t) => {
                            let update = extract_soniox_message(&t, translating)?;
                            committed.push_str(&update.final_text);
                            let _ = previews.send(compose_preview(&committed, &update.interim_text));
                        }
                        Message::Close(_) => return Err("Soniox WS closed mid-stream".into()),
                        _ => {}
                    }
                }
            }
        }

        if let Err(e) = sink.send(Message::Text(finalize_message())).await {
            eprintln!("[stream] Soniox finalize send failed: {e}");
        }
        // An empty *text* frame ("") is the documented end-of-audio signal;
        // Soniox finalizes remaining tokens and replies with `finished: true`.
        // A zero-length binary frame is silently ignored, which leaves the
        // stream open and burns the full drain timeout.
        let _ = sink.send(Message::Text(String::new())).await;

        let _ = tokio::time::timeout(FINAL_RESULTS_TIMEOUT, async {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(t)) => match extract_soniox_message(&t, translating) {
                        Ok(update) => {
                            committed.push_str(&update.final_text);
                            if update.finished {
                                break;
                            }
                        }
                        Err(e) => eprintln!("[stream] Soniox post-finalize parse error: {e}"),
                    },
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        eprintln!("[stream] Soniox post-finalize recv error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;

        let transcript = committed.trim().to_string();
        if !transcript.is_empty() {
            let _ = previews.send(transcript.clone());
        }

        Ok(EngineOutcome {
            transcript,
            warning: None,
        })
    }
}

fn build_config_message(
    key: &str,
    language: &ModeLanguage,
    translate_to: Option<&str>,
    terms: &[String],
) -> String {
    let mut config = serde_json::json!({
        "api_key": key,
        "model": SONIOX_RT_MODEL_ID,
        "audio_format": AUDIO_FORMAT_PCM_S16LE,
        "sample_rate": REALTIME_SAMPLE_RATE,
        "num_channels": REALTIME_NUM_CHANNELS,
        "enable_endpoint_detection": true,
        "max_endpoint_delay_ms": MAX_ENDPOINT_DELAY_MS,
    });

    let hints = language_hints(language);
    if !hints.is_empty() {
        config["language_hints"] = serde_json::json!(hints);
    }

    let context_terms = terms::soniox_context_terms(terms);
    if !context_terms.is_empty() {
        config["context"] = serde_json::json!({ "terms": context_terms });
    }

    if let Some(target) = translate_to {
        config["translation"] = serde_json::json!({
            "type": "one_way",
            "target_language": target,
        });
    }

    config.to_string()
}

/// Hints bias recognition without constraining it; `language_hints_strict` is
/// never set, because forcing a single language is exactly what breaks
/// intra-sentence code-switching. `Auto` sends no hints (full auto-detect).
fn language_hints(language: &ModeLanguage) -> Vec<String> {
    match language {
        ModeLanguage::Auto => vec![],
        ModeLanguage::Exact { code } => vec![code.clone()],
        ModeLanguage::Hints { codes } => codes.clone(),
    }
}

fn finalize_message() -> String {
    serde_json::json!({ "type": "finalize" }).to_string()
}

#[derive(Debug)]
struct SonioxUpdate {
    final_text: String,
    interim_text: String,
    finished: bool,
}

/// Accumulates token text from one Soniox response. Final tokens are appended
/// once (Soniox never re-sends them); non-final tokens are the revisable tail
/// and replace the previous interim. In a translation session only tokens
/// tagged `translation_status: "translation"` are kept; otherwise translation
/// tokens (if any) are dropped and transcription tokens kept.
fn extract_soniox_message(text: &str, translating: bool) -> Result<SonioxUpdate, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("JSON parse failed: {e}"))?;

    if let Some(code) = v.get("error_code").and_then(Value::as_i64) {
        let detail = v
            .get("error_message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(format!("Soniox error {code}: {detail}"));
    }

    let finished = v.get("finished").and_then(Value::as_bool).unwrap_or(false);
    let mut final_text = String::new();
    let mut interim_text = String::new();

    if let Some(tokens) = v.get("tokens").and_then(Value::as_array) {
        for token in tokens {
            let Some(piece) = token.get("text").and_then(Value::as_str) else {
                continue;
            };
            if CONTROL_TOKENS.contains(&piece) {
                continue;
            }
            let is_translation =
                token.get("translation_status").and_then(Value::as_str) == Some("translation");
            if translating != is_translation {
                continue;
            }
            if token
                .get("is_final")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                final_text.push_str(piece);
            } else {
                interim_text.push_str(piece);
            }
        }
    }

    Ok(SonioxUpdate {
        final_text,
        interim_text,
        finished,
    })
}

fn compose_preview(committed: &str, interim: &str) -> String {
    let mut preview = String::with_capacity(committed.len() + interim.len());
    preview.push_str(committed);
    preview.push_str(interim);
    preview.trim_start().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new_stores_key_and_target() {
        let engine = SonioxRealtimeEngine::new("k".to_string(), Some("en".to_string()));
        assert_eq!(engine.key, "k");
        assert_eq!(engine.translate_to.as_deref(), Some("en"));
    }

    #[test]
    fn auto_sends_no_language_hints() {
        assert!(language_hints(&ModeLanguage::Auto).is_empty());
    }

    #[test]
    fn exact_sends_single_hint() {
        assert_eq!(language_hints(&ModeLanguage::exact("uk")), vec!["uk"]);
    }

    #[test]
    fn hints_sends_all_codes() {
        let codes = vec!["uk".to_string(), "en".to_string()];
        assert_eq!(
            language_hints(&ModeLanguage::Hints {
                codes: codes.clone()
            }),
            codes
        );
    }

    #[test]
    fn config_never_sends_strict_hints() {
        let config = build_config_message("k", &ModeLanguage::exact("uk"), None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert!(v.get("language_hints_strict").is_none());
        assert_eq!(v["language_hints"], serde_json::json!(["uk"]));
    }

    #[test]
    fn config_declares_model_and_audio_format() {
        let config = build_config_message("secret", &ModeLanguage::Auto, None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert_eq!(v["api_key"], "secret");
        assert_eq!(v["model"], "stt-rt-v5");
        assert_eq!(v["audio_format"], "pcm_s16le");
        assert_eq!(v["sample_rate"], 16000);
        assert_eq!(v["num_channels"], 1);
    }

    #[test]
    fn config_enables_endpoint_detection_with_tuned_delay() {
        let config = build_config_message("k", &ModeLanguage::Auto, None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert_eq!(v["enable_endpoint_detection"], true);
        assert_eq!(v["max_endpoint_delay_ms"], 500);
    }

    #[test]
    fn config_omits_language_hints_for_auto() {
        let config = build_config_message("k", &ModeLanguage::Auto, None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert!(v.get("language_hints").is_none());
    }

    #[test]
    fn config_omits_translation_when_no_target() {
        let config = build_config_message("k", &ModeLanguage::Auto, None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert!(v.get("translation").is_none());
    }

    #[test]
    fn config_sends_one_way_translation_when_target_set() {
        let config = build_config_message("k", &ModeLanguage::Auto, Some("en"), &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert_eq!(v["translation"]["type"], "one_way");
        assert_eq!(v["translation"]["target_language"], "en");
    }

    #[test]
    fn config_includes_context_terms_when_present() {
        let terms = vec!["MongoDB".to_string(), "Tauri".to_string()];
        let config = build_config_message("k", &ModeLanguage::Auto, None, &terms);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            v["context"]["terms"],
            serde_json::json!(["MongoDB", "Tauri"])
        );
    }

    #[test]
    fn config_omits_context_when_no_terms() {
        let config = build_config_message("k", &ModeLanguage::Auto, None, &[]);
        let v: Value = serde_json::from_str(&config).unwrap();
        assert!(v.get("context").is_none());
    }

    #[test]
    fn finalize_message_has_finalize_type() {
        let v: Value = serde_json::from_str(&finalize_message()).unwrap();
        assert_eq!(v["type"], "finalize");
    }

    #[test]
    fn extracts_final_and_interim_tokens() {
        let body = r#"{"tokens":[
            {"text":"hello ","is_final":true},
            {"text":"wor","is_final":false}
        ]}"#;
        let update = extract_soniox_message(body, false).unwrap();
        assert_eq!(update.final_text, "hello ");
        assert_eq!(update.interim_text, "wor");
        assert!(!update.finished);
    }

    #[test]
    fn skips_control_tokens() {
        let body = r#"{"tokens":[
            {"text":"hi","is_final":true},
            {"text":"<end>","is_final":true},
            {"text":"<fin>","is_final":true}
        ]}"#;
        let update = extract_soniox_message(body, false).unwrap();
        assert_eq!(update.final_text, "hi");
    }

    #[test]
    fn verbatim_session_drops_translation_tokens() {
        let body = r#"{"tokens":[
            {"text":"привіт","is_final":true,"translation_status":"original"},
            {"text":"hello","is_final":true,"translation_status":"translation"}
        ]}"#;
        let update = extract_soniox_message(body, false).unwrap();
        assert_eq!(update.final_text, "привіт");
    }

    #[test]
    fn translating_session_keeps_only_translation_tokens() {
        let body = r#"{"tokens":[
            {"text":"привіт","is_final":true,"translation_status":"original"},
            {"text":"hello","is_final":true,"translation_status":"translation"}
        ]}"#;
        let update = extract_soniox_message(body, true).unwrap();
        assert_eq!(update.final_text, "hello");
    }

    #[test]
    fn detects_finished_flag() {
        let body = r#"{"tokens":[],"finished":true}"#;
        let update = extract_soniox_message(body, false).unwrap();
        assert!(update.finished);
    }

    #[test]
    fn surfaces_error_message() {
        let body = r#"{"tokens":[],"error_code":401,"error_message":"invalid api key"}"#;
        let err = extract_soniox_message(body, false).unwrap_err();
        assert!(err.contains("401"), "expected code, got: {err}");
        assert!(
            err.contains("invalid api key"),
            "expected detail, got: {err}"
        );
    }

    #[test]
    fn rejects_unparseable_message() {
        assert!(extract_soniox_message("not json", false).is_err());
    }

    #[test]
    fn compose_preview_concatenates_and_trims_leading_space() {
        assert_eq!(compose_preview(" hello ", "wor"), "hello wor");
        assert_eq!(compose_preview("hello", ""), "hello");
        assert_eq!(compose_preview("", ""), "");
    }
}
