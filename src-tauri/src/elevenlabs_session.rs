use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::groq_audio::{encode_to_flac_16k_mono, to_pcm_16k_mono_bytes};
use crate::mode::ModeLanguage;
use crate::terms;
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const ELEVENLABS_STT_URL: &str = "https://api.elevenlabs.io/v1/speech-to-text";
const SCRIBE_V2_MODEL_ID: &str = "scribe_v2";

const ELEVENLABS_REALTIME_WS_BASE: &str = "wss://api.elevenlabs.io/v1/speech-to-text/realtime";
const SCRIBE_V2_REALTIME_MODEL_ID: &str = "scribe_v2_realtime";

/// The realtime API ingests 16 kHz mono PCM only; capture audio (often 48 kHz
/// and/or stereo) is downmixed and resampled before it goes on the wire.
const REALTIME_SAMPLE_RATE: u32 = 16_000;

/// Hard ceiling on the post-commit drain, so a flush that never starts can't
/// block the paste indefinitely.
const FINAL_RESULTS_TIMEOUT: Duration = Duration::from_secs(3);

/// ElevenLabs realtime sends no terminal/"finished" message and keeps the
/// socket open after a commit, so there's no close to wait for. Once the
/// committed flush has gone quiet for this long we treat it as complete. This
/// bounds the post-release tail to the idle window instead of always hitting
/// FINAL_RESULTS_TIMEOUT. The commit-to-first-segment latency is covered
/// separately by the `got_committed` guard, so this only needs to outlast the
/// gap between segments of a split final flush — those arrive in a sub-100ms
/// burst, so the window sits just above that to keep the post-release tail low.
const POST_COMMIT_IDLE: Duration = Duration::from_millis(120);

pub struct ElevenLabsEngine {
    pub key: String,
}

impl ElevenLabsEngine {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl Engine for ElevenLabsEngine {
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

        if all_samples.is_empty() {
            return Ok(EngineOutcome {
                transcript: String::new(),
                warning: None,
            });
        }

        let flac =
            encode_to_flac_16k_mono(&all_samples, ctx.format.sample_rate, ctx.format.channels)
                .map_err(|e| format!("FLAC encode failed: {e}"))?;

        let language = ctx.language.as_code().map(str::to_string);
        let keyterms = terms::elevenlabs_keyterms(&ctx.terms);

        let text = post_to_elevenlabs(&self.key, language.as_deref(), &keyterms, flac).await?;

        Ok(EngineOutcome {
            transcript: text,
            warning: None,
        })
    }
}

pub struct ElevenLabsRealtimeEngine {
    pub key: String,
}

impl ElevenLabsRealtimeEngine {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl Engine for ElevenLabsRealtimeEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        let url = build_ws_url(&ctx.language, &ctx.terms)?;
        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("bad WS URL: {e}"))?;
        req.headers_mut().insert(
            "xi-api-key",
            self.key
                .parse()
                .map_err(|e| format!("bad auth header: {e}"))?,
        );

        let (ws, _resp) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("ElevenLabs WS connect failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();

        let input_sample_rate = ctx.format.sample_rate;
        let input_channels = ctx.format.channels;
        let mut transcript_pieces: Vec<String> = Vec::new();
        let mut current_interim: String = String::new();

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => {
                    match maybe_chunk {
                        Some(chunk) => {
                            let pcm = to_pcm_16k_mono_bytes(&chunk, input_sample_rate, input_channels)
                                .map_err(|e| format!("ElevenLabs audio resample failed: {e}"))?;
                            if let Err(e) = sink.send(Message::Text(audio_chunk_message(&pcm))).await {
                                return Err(format!("ElevenLabs WS send failed: {e}"));
                            }
                        }
                        None => break,
                    }
                }
                msg = stream.next() => {
                    let Some(msg) = msg else { return Err("ElevenLabs WS closed mid-stream".into()); };
                    let msg = msg.map_err(|e| format!("ElevenLabs WS recv failed: {e}"))?;
                    match msg {
                        Message::Text(t) => {
                            match extract_realtime_message(&t)? {
                                Some((true, piece)) => {
                                    if !piece.is_empty() {
                                        transcript_pieces.push(piece);
                                    }
                                    current_interim.clear();
                                    let _ = previews.send(raw_preview(&transcript_pieces, &current_interim));
                                }
                                Some((false, piece)) => {
                                    current_interim = piece;
                                    let _ = previews.send(raw_preview(&transcript_pieces, &current_interim));
                                }
                                None => {}
                            }
                        }
                        Message::Close(_) => return Err("ElevenLabs WS closed mid-stream".into()),
                        _ => {}
                    }
                }
            }
        }

        if let Err(e) = sink.send(Message::Text(commit_message())).await {
            eprintln!("[stream] ElevenLabs commit send failed: {e}");
        }

        // No end-of-results signal and no server close after commit, so exit
        // once the committed flush goes idle rather than waiting on a Close that
        // never arrives. Preview emission is skipped while draining — the overlay
        // holds the last preview until the final transcript is ready.
        let deadline = tokio::time::sleep(FINAL_RESULTS_TIMEOUT);
        tokio::pin!(deadline);
        let mut got_committed = false;
        loop {
            tokio::select! {
                _ = &mut deadline => break,
                next = tokio::time::timeout(POST_COMMIT_IDLE, stream.next()) => {
                    match next {
                        // Idle window elapsed with no message: done once the first
                        // committed piece has arrived; otherwise keep waiting for
                        // the flush to start, up to the hard deadline.
                        Err(_) => {
                            if got_committed {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Ok(Some(Ok(Message::Text(t)))) => {
                            if let Ok(Some((true, piece))) = extract_realtime_message(&t) {
                                got_committed = true;
                                if !piece.is_empty() {
                                    transcript_pieces.push(piece);
                                }
                            }
                        }
                        Ok(Some(Ok(Message::Close(_)))) => break,
                        Ok(Some(Err(e))) => {
                            eprintln!("[stream] ElevenLabs post-commit recv error: {e}");
                            break;
                        }
                        Ok(Some(_)) => {}
                    }
                }
            }
        }

        let transcript = transcript_pieces.join(" ").trim().to_string();

        if !transcript.is_empty() {
            let _ = previews.send(transcript.clone());
        }

        Ok(EngineOutcome {
            transcript,
            warning: None,
        })
    }
}

fn build_ws_url(language: &ModeLanguage, terms: &[String]) -> Result<Url, String> {
    let mut url =
        Url::parse(ELEVENLABS_REALTIME_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("model_id", SCRIBE_V2_REALTIME_MODEL_ID);
        query.append_pair("no_verbatim", "true");
        // Realtime takes a single language code; Auto and Hints fall back to the
        // server's multilingual auto-detection (Hints can't be expressed as one
        // code, and forcing one would hurt code-switching).
        if let ModeLanguage::Exact { code } = language {
            query.append_pair("language_code", code);
        }
    }
    // Budget computed after static params to stay within the URL-length ceiling.
    let remaining =
        terms::ELEVENLABS_REALTIME_KEYTERM_BUDGET_BYTES.saturating_sub(url.as_str().len());
    {
        let mut query = url.query_pairs_mut();
        for term in terms::elevenlabs_realtime_keyterms(terms, remaining) {
            query.append_pair("keyterms", &term);
        }
    }
    Ok(url)
}

fn audio_chunk_message(pcm: &[u8]) -> String {
    let audio_base_64 = base64::engine::general_purpose::STANDARD.encode(pcm);
    serde_json::json!({
        "message_type": "input_audio_chunk",
        "audio_base_64": audio_base_64,
        "commit": false,
        "sample_rate": REALTIME_SAMPLE_RATE,
    })
    .to_string()
}

fn commit_message() -> String {
    serde_json::json!({
        "message_type": "input_audio_chunk",
        "audio_base_64": "",
        "commit": true,
        "sample_rate": REALTIME_SAMPLE_RATE,
    })
    .to_string()
}

/// Returns `Some((is_committed, text))` for transcript messages, `None` for
/// non-transcript messages (e.g. `session_started`). Errors on `input_error`.
fn extract_realtime_message(text: &str) -> Result<Option<(bool, String)>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("JSON parse failed: {e}"))?;
    match v.get("message_type").and_then(|x| x.as_str()) {
        Some("partial_transcript") => Ok(Some((false, transcript_text(&v)))),
        Some("committed_transcript") => Ok(Some((true, transcript_text(&v)))),
        Some("input_error") => Err(format_realtime_error(&v)),
        _ => Ok(None),
    }
}

fn transcript_text(v: &Value) -> String {
    v.get("text")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn format_realtime_error(v: &Value) -> String {
    let detail = v
        .get("error")
        .or_else(|| v.get("message"))
        .and_then(|x| x.as_str())
        .unwrap_or("unknown error");
    format!("ElevenLabs realtime error: {detail}")
}

fn raw_preview(finals: &[String], interim: &str) -> String {
    let mut preview = finals.join(" ");
    if !interim.is_empty() {
        if !preview.is_empty() {
            preview.push(' ');
        }
        preview.push_str(interim);
    }
    preview
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn post_to_elevenlabs(
    key: &str,
    language: Option<&str>,
    keyterms: &[String],
    flac: Vec<u8>,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(flac)
        .file_name("audio.flac")
        .mime_str("audio/flac")
        .map_err(|e| format!("ElevenLabs mime build failed: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .text("model_id", SCRIBE_V2_MODEL_ID)
        .text("no_verbatim", "true")
        .text("tag_audio_events", "false")
        .part("file", part);

    if let Some(lang) = language {
        form = form.text("language_code", lang.to_string());
    }

    for keyterm in keyterms {
        form = form.text("keyterms", keyterm.clone());
    }

    let resp = http_client()
        .post(ELEVENLABS_STT_URL)
        .header("xi-api-key", key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("ElevenLabs request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format_elevenlabs_error(status, &body));
    }

    parse_transcript(&body).map_err(|e| format!("ElevenLabs response parse failed: {e}"))
}

fn format_elevenlabs_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["detail"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    if message.is_empty() {
        format!("ElevenLabs {status}")
    } else {
        format!("ElevenLabs {status}: {message}")
    }
}

fn parse_transcript(body: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("JSON parse failed: {e}"))?;
    let text = v
        .get("text")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "response missing `text` field".to_string())?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new_stores_key() {
        let engine = ElevenLabsEngine::new("xi-test-key".to_string());
        assert_eq!(engine.key, "xi-test-key");
    }

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
    fn extracts_elevenlabs_error_detail_from_structured_body() {
        let body = r#"{"detail":"Invalid API key"}"#;
        let msg = format_elevenlabs_error(reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("401"), "expected status, got: {msg}");
        assert!(
            msg.contains("Invalid API key"),
            "expected detail, got: {msg}"
        );
    }

    #[test]
    fn falls_back_to_status_when_error_body_is_empty() {
        let msg = format_elevenlabs_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "");
        assert!(msg.contains("500"), "expected status, got: {msg}");
    }

    #[test]
    fn realtime_engine_new_stores_key() {
        let engine = ElevenLabsRealtimeEngine::new("xi-test-key".to_string());
        assert_eq!(engine.key, "xi-test-key");
    }

    #[test]
    fn partial_transcript_is_not_committed() {
        let body = r#"{"message_type":"partial_transcript","text":"hello"}"#;
        assert_eq!(
            extract_realtime_message(body).unwrap(),
            Some((false, "hello".to_string()))
        );
    }

    #[test]
    fn committed_transcript_is_committed() {
        let body = r#"{"message_type":"committed_transcript","text":"  hello world "}"#;
        assert_eq!(
            extract_realtime_message(body).unwrap(),
            Some((true, "hello world".to_string()))
        );
    }

    #[test]
    fn session_started_is_ignored() {
        let body = r#"{"message_type":"session_started"}"#;
        assert_eq!(extract_realtime_message(body).unwrap(), None);
    }

    #[test]
    fn input_error_message_surfaces_as_error() {
        let body = r#"{"message_type":"input_error","error":"bad audio"}"#;
        let err = extract_realtime_message(body).unwrap_err();
        assert!(err.contains("bad audio"), "expected detail, got: {err}");
    }

    #[test]
    fn audio_chunk_message_declares_16k_and_base64_encodes_payload() {
        let pcm = vec![1u8, 0u8, 255u8, 255u8];
        let msg = audio_chunk_message(&pcm);
        let v: Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["message_type"], "input_audio_chunk");
        assert_eq!(v["commit"], false);
        assert_eq!(v["sample_rate"], 16000);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(v["audio_base_64"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded, pcm);
    }

    #[test]
    fn commit_message_has_empty_audio_commit_true_and_16k_rate() {
        let v: Value = serde_json::from_str(&commit_message()).unwrap();
        assert_eq!(v["audio_base_64"], "");
        assert_eq!(v["commit"], true);
        assert_eq!(v["sample_rate"], 16000);
    }

    #[test]
    fn raw_preview_joins_finals_and_interim() {
        let finals = vec!["hello".to_string(), "there".to_string()];
        assert_eq!(raw_preview(&finals, "wor"), "hello there wor");
        assert_eq!(raw_preview(&finals, ""), "hello there");
        assert_eq!(raw_preview(&[], "wor"), "wor");
    }

    #[test]
    fn url_always_sends_model_and_no_verbatim() {
        let url = build_ws_url(&ModeLanguage::Auto, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            query.contains("model_id=scribe_v2_realtime"),
            "missing model_id: {query}"
        );
        assert!(
            query.contains("no_verbatim=true"),
            "missing no_verbatim: {query}"
        );
    }

    #[test]
    fn url_sends_language_code_for_exact() {
        let url = build_ws_url(&ModeLanguage::exact("uk"), &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            query.contains("language_code=uk"),
            "Exact must send language_code=uk: {query}"
        );
    }

    #[test]
    fn url_omits_language_code_for_auto() {
        let url = build_ws_url(&ModeLanguage::Auto, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            !query.contains("language_code"),
            "Auto must not send language_code: {query}"
        );
    }

    #[test]
    fn url_omits_language_code_for_hints() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let url = build_ws_url(&lang, &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            !query.contains("language_code"),
            "Hints must not send language_code: {query}"
        );
    }

    #[test]
    fn url_appends_keyterms() {
        let terms: Vec<String> = vec!["MongoDB".into(), "TypeScript".into()];
        let url = build_ws_url(&ModeLanguage::exact("en"), &terms).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            query.contains("keyterms=MongoDB"),
            "missing MongoDB: {query}"
        );
        assert!(
            query.contains("keyterms=TypeScript"),
            "missing TypeScript: {query}"
        );
    }

    #[test]
    fn url_has_no_keyterms_for_empty_terms() {
        let url = build_ws_url(&ModeLanguage::exact("en"), &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(!query.contains("keyterms="), "unexpected keyterms: {query}");
    }
}
