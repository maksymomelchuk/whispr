use crate::engine::{Engine, EngineContext, EngineOutcome};
use crate::mode::ModeLanguage;
use crate::recorder::AudioFormat;
use crate::terms;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use url::Url;

const DEEPGRAM_WS_BASE: &str = "wss://api.deepgram.com/v1/listen";

/// How long to wait for Deepgram to flush remaining `is_final` results after
/// we send `CloseStream`. The server typically responds within a few hundred
/// ms; we cap it so a hung WS never blocks the paste indefinitely.
const FINAL_RESULTS_TIMEOUT: Duration = Duration::from_secs(3);

pub struct DeepgramEngine {
    pub key: String,
}

impl DeepgramEngine {
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl Engine for DeepgramEngine {
    async fn run(
        self,
        mut chunks: UnboundedReceiver<Vec<i16>>,
        previews: UnboundedSender<String>,
        ctx: EngineContext,
    ) -> Result<EngineOutcome, String> {
        let url = build_ws_url(&ctx.language, ctx.format, &ctx.terms)?;
        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| format!("bad WS URL: {e}"))?;
        req.headers_mut().insert(
            "Authorization",
            format!("Token {}", self.key)
                .parse()
                .map_err(|e| format!("bad auth header: {e}"))?,
        );

        let (ws, _resp) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| format!("Deepgram WS connect failed: {e}"))?;
        let (mut sink, mut stream) = ws.split();

        let mut transcript_pieces: Vec<String> = Vec::new();
        let mut current_interim: String = String::new();

        loop {
            tokio::select! {
                maybe_chunk = chunks.recv() => {
                    match maybe_chunk {
                        Some(chunk) => {
                            if let Err(e) = sink.send(Message::Binary(pcm_bytes(&chunk))).await {
                                return Err(format!("Deepgram WS send failed: {e}"));
                            }
                        }
                        None => break,
                    }
                }
                msg = stream.next() => {
                    let Some(msg) = msg else { return Err("Deepgram WS closed mid-stream".into()); };
                    let msg = msg.map_err(|e| format!("Deepgram WS recv failed: {e}"))?;
                    match msg {
                        Message::Text(t) => {
                            if let Some((is_final, piece)) = extract_transcript_message(&t) {
                                if is_final {
                                    if !piece.is_empty() {
                                        transcript_pieces.push(piece);
                                    }
                                    current_interim.clear();
                                } else {
                                    current_interim = piece;
                                }
                                let raw = raw_preview(&transcript_pieces, &current_interim);
                                let _ = previews.send(raw);
                            }
                        }
                        Message::Close(_) => return Err("Deepgram WS closed mid-stream".into()),
                        _ => {}
                    }
                }
            }
        }

        let close_msg = serde_json::json!({"type": "CloseStream"}).to_string();
        if let Err(e) = sink.send(Message::Text(close_msg)).await {
            eprintln!("[stream] CloseStream send failed: {e}");
        }

        // Drain remaining finals with a bounded timeout so a stuck server can't
        // block the paste. Preview emission is skipped here — the overlay holds
        // the last preview until the final transcript is ready.
        let _ = tokio::time::timeout(FINAL_RESULTS_TIMEOUT, async {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(t)) => {
                        if let Some((true, piece)) = extract_transcript_message(&t) {
                            if !piece.is_empty() {
                                transcript_pieces.push(piece);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        eprintln!("[stream] post-close recv error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        })
        .await;

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

fn build_ws_url(
    language: &ModeLanguage,
    format: AudioFormat,
    terms: &[String],
) -> Result<Url, String> {
    let mut url = Url::parse(DEEPGRAM_WS_BASE).map_err(|e| format!("base URL parse: {e}"))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("model", "nova-3");
        match language {
            ModeLanguage::Auto => {}
            ModeLanguage::Exact { code } => {
                q.append_pair("language", code);
            }
            // Deepgram's multi-language detection mode; individual codes are
            // informational only (Deepgram doesn't expose per-language hint params yet).
            ModeLanguage::Hints { .. } => {
                q.append_pair("language", "multi");
            }
        }
        q.append_pair("encoding", "linear16");
        q.append_pair("sample_rate", &format.sample_rate.to_string());
        q.append_pair("channels", &format.channels.to_string());
        q.append_pair("smart_format", "true");
        q.append_pair("numerals", "true");
    }
    // Budget computed after static params to stay within the 4 KB total-URL ceiling.
    let remaining = terms::DEEPGRAM_KEYTERM_BUDGET_BYTES.saturating_sub(url.as_str().len());
    {
        let mut q = url.query_pairs_mut();
        for term in terms::deepgram_keyterms(terms, remaining) {
            q.append_pair("keyterm", &term);
        }
    }
    Ok(url)
}

fn extract_transcript_message(text: &str) -> Option<(bool, String)> {
    let v: Value = serde_json::from_str(text).ok()?;
    if v.get("type").and_then(|x| x.as_str()) != Some("Results") {
        return None;
    }
    let is_final = v.get("is_final").and_then(|x| x.as_bool()).unwrap_or(false);
    let t = v["channel"]["alternatives"][0]["transcript"]
        .as_str()?
        .trim();
    Some((is_final, t.to_string()))
}

fn pcm_bytes(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::AudioFormat;

    fn fmt() -> AudioFormat {
        AudioFormat {
            sample_rate: 16000,
            channels: 1,
        }
    }

    #[test]
    fn deepgram_engine_new_stores_key() {
        let engine = DeepgramEngine::new("my-api-key".to_string());
        assert_eq!(engine.key, "my-api-key");
    }

    #[test]
    fn build_ws_url_hardcodes_smart_format_and_numerals() {
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &[]).unwrap();
        let query = url.query().unwrap_or("");
        assert!(
            query.contains("smart_format=true"),
            "URL must contain smart_format=true"
        );
        assert!(
            query.contains("numerals=true"),
            "URL must contain numerals=true"
        );
        assert!(
            !query.contains("dictation"),
            "URL must not contain dictation"
        );
        assert!(!query.contains("keyterm"), "URL must not contain keyterm");
    }

    #[test]
    fn url_omits_language_for_auto() {
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(
            !q.contains("language="),
            "Auto must not send language param: {q}"
        );
    }

    #[test]
    fn url_sends_exact_language_code() {
        let url = build_ws_url(&ModeLanguage::exact("uk"), fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(
            q.contains("language=uk"),
            "Exact must send language=uk: {q}"
        );
    }

    #[test]
    fn url_sends_multi_for_hints() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let url = build_ws_url(&lang, fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(
            q.contains("language=multi"),
            "Hints must send language=multi: {q}"
        );
    }

    #[test]
    fn url_includes_terms_as_keyterms() {
        let terms: Vec<String> = vec!["MongoDB".into(), "TypeScript".into()];
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(q.contains("keyterm=MongoDB"), "missing MongoDB: {q}");
        assert!(q.contains("keyterm=TypeScript"), "missing TypeScript: {q}");
    }

    #[test]
    fn url_has_no_keyterms_for_empty_terms() {
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &[]).unwrap();
        let q = url.query().unwrap_or("");
        assert!(!q.contains("keyterm="), "unexpected keyterm: {q}");
    }

    #[test]
    fn url_includes_keyterms_with_hints_language() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let terms: Vec<String> = vec!["MongoDB".into()];
        let url = build_ws_url(&lang, fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(
            q.contains("language=multi"),
            "Hints must send language=multi: {q}"
        );
        assert!(
            q.contains("keyterm=MongoDB"),
            "keyterm missing for Hints: {q}"
        );
    }

    #[test]
    fn url_includes_keyterms_with_auto_language() {
        let terms: Vec<String> = vec!["Kubernetes".into()];
        let url = build_ws_url(&ModeLanguage::Auto, fmt(), &terms).unwrap();
        let q = url.query().unwrap_or("");
        assert!(
            !q.contains("language="),
            "Auto must not send language param: {q}"
        );
        assert!(
            q.contains("keyterm=Kubernetes"),
            "keyterm missing for Auto: {q}"
        );
    }

    #[test]
    fn url_respects_keyterm_budget() {
        let terms: Vec<String> = (0..200).map(|i| format!("term{i:05}")).collect();
        let url = build_ws_url(&ModeLanguage::exact("en"), fmt(), &terms).unwrap();
        assert!(
            url.as_str().len() <= terms::DEEPGRAM_KEYTERM_BUDGET_BYTES,
            "URL too long: {} bytes",
            url.as_str().len()
        );
    }
}
