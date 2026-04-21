use crate::config;
use tauri::AppHandle;

const DEEPGRAM_ENDPOINT: &str =
    "https://api.deepgram.com/v1/listen?model=nova-3&smart_format=true&language=multi";

#[tauri::command]
pub async fn transcribe(app: AppHandle, audio: Vec<u8>) -> Result<String, String> {
    if audio.is_empty() {
        return Ok(String::new());
    }

    let settings = config::load(&app);
    let key = settings
        .api_key
        .filter(|k| !k.is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;

    let client = reqwest::Client::new();
    let res = client
        .post(DEEPGRAM_ENDPOINT)
        .header("Authorization", format!("Token {key}"))
        .header("Content-Type", "audio/webm")
        .body(audio)
        .send()
        .await
        .map_err(|e| format!("Deepgram request failed: {e}"))?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Deepgram API error {status}: {body}"));
    }

    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    let transcript = json["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .trim();

    Ok(if transcript.is_empty() {
        String::new()
    } else {
        // Trailing space so back-to-back dictations concatenate cleanly.
        format!("{transcript} ")
    })
}
