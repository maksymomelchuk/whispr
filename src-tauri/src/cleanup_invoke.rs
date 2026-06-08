use crate::{cleanup, config};
#[cfg(test)]
use std::time::Duration;

/// Returns true if a credential is present for `provider` in `cleanup_settings`.
/// Used by the live pipeline to distinguish "no credential configured" (NoCredential)
/// from "credential rejected by API" (FailedCredential).
pub(crate) fn is_credential_configured(
    cleanup_settings: &config::AiCleanupSettings,
    provider: cleanup::AiProviderId,
) -> bool {
    use cleanup::AiProviderId;
    match provider {
        AiProviderId::Anthropic => match cleanup_settings.auth_mode {
            config::CleanupAuthMode::ApiKey => cleanup_settings
                .provider_keys
                .get("anthropic")
                .filter(|k| !k.is_empty())
                .is_some(),
            config::CleanupAuthMode::Oauth => cleanup_settings
                .anthropic_oauth_token
                .as_deref()
                .filter(|t| !t.is_empty())
                .is_some(),
        },
        AiProviderId::Custom => cleanup_settings
            .custom_provider
            .as_ref()
            .filter(|cp| !cp.base_url.is_empty())
            .is_some(),
        _ => cleanup_settings
            .provider_keys
            .get(provider.as_str())
            .filter(|k| !k.is_empty())
            .is_some(),
    }
}

/// Routes and executes an AI cleanup call: resolves credentials, selects the
/// Anthropic-native or OpenAI-compatible transport, applies the length-scaled
/// timeout, and maps transport outcomes to `CleanupError` variants.
///
/// No gating, no Tauri events, no window-focus side effects.
pub async fn invoke(
    cleanup_settings: &config::AiCleanupSettings,
    provider: cleanup::AiProviderId,
    model: &str,
    prompt_override: Option<&str>,
    transcript: &str,
) -> Result<(String, cleanup::Usage), cleanup::CleanupError> {
    use cleanup::AiProviderId;
    let prompt = cleanup::effective_prompt(prompt_override);
    match provider {
        AiProviderId::Anthropic => {
            let credential = resolve_anthropic_credential(cleanup_settings)?;
            cleanup::run(transcript, credential, model, &prompt).await
        }
        AiProviderId::Custom => {
            let (api_key, chat_url, custom_model) = resolve_custom_endpoint(cleanup_settings)?;
            cleanup::run_openai(transcript, &api_key, &chat_url, &custom_model, &prompt).await
        }
        _ => {
            let api_key = resolve_provider_key(cleanup_settings, provider)?;
            cleanup::run_openai(
                transcript,
                api_key,
                provider.openai_chat_url(),
                model,
                &prompt,
            )
            .await
        }
    }
}

/// Testable variant of `invoke` with injectable transport and explicit timeout.
#[cfg(test)]
pub(crate) async fn invoke_with_transport<T: cleanup::Transport>(
    cleanup_settings: &config::AiCleanupSettings,
    provider: cleanup::AiProviderId,
    model: &str,
    prompt_override: Option<&str>,
    transcript: &str,
    transport: &T,
    timeout: Duration,
) -> Result<(String, cleanup::Usage), cleanup::CleanupError> {
    use cleanup::AiProviderId;
    let prompt = cleanup::effective_prompt(prompt_override);
    match provider {
        AiProviderId::Anthropic => {
            let credential = resolve_anthropic_credential(cleanup_settings)?;
            cleanup::run_with_transport(transcript, credential, model, &prompt, transport, timeout)
                .await
        }
        AiProviderId::Custom => {
            let (api_key, chat_url, custom_model) = resolve_custom_endpoint(cleanup_settings)?;
            cleanup::run_openai_with_transport(
                transcript,
                &api_key,
                &chat_url,
                &custom_model,
                &prompt,
                transport,
                timeout,
            )
            .await
        }
        _ => {
            let api_key = resolve_provider_key(cleanup_settings, provider)?;
            cleanup::run_openai_with_transport(
                transcript,
                api_key,
                provider.openai_chat_url(),
                model,
                &prompt,
                transport,
                timeout,
            )
            .await
        }
    }
}

fn resolve_anthropic_credential<'a>(
    cleanup_settings: &'a config::AiCleanupSettings,
) -> Result<cleanup::Credential<'a>, cleanup::CleanupError> {
    match cleanup_settings.auth_mode {
        config::CleanupAuthMode::ApiKey => {
            match cleanup_settings
                .provider_keys
                .get("anthropic")
                .filter(|k| !k.is_empty())
            {
                Some(k) => Ok(cleanup::Credential::ApiKey(k)),
                None => Err(cleanup::CleanupError::Credential(
                    "AI cleanup is enabled but Anthropic API key is not set.".to_string(),
                )),
            }
        }
        config::CleanupAuthMode::Oauth => match cleanup_settings.anthropic_oauth_token.as_deref() {
            Some(t) if !t.is_empty() => Ok(cleanup::Credential::OauthToken(t)),
            _ => Err(cleanup::CleanupError::Credential(
                "AI cleanup is set to OAuth but no Claude Code token is configured.".to_string(),
            )),
        },
    }
}

fn resolve_custom_endpoint(
    cleanup_settings: &config::AiCleanupSettings,
) -> Result<(String, String, String), cleanup::CleanupError> {
    match &cleanup_settings.custom_provider {
        Some(cp) if !cp.base_url.is_empty() => {
            let chat_url = format!("{}/chat/completions", cp.base_url.trim_end_matches('/'));
            let api_key = cp.api_key.as_deref().unwrap_or("").to_string();
            Ok((api_key, chat_url, cp.model.clone()))
        }
        _ => Err(cleanup::CleanupError::Credential(
            "AI cleanup is enabled but the Custom provider is not configured.".to_string(),
        )),
    }
}

fn resolve_provider_key<'a>(
    cleanup_settings: &'a config::AiCleanupSettings,
    provider: cleanup::AiProviderId,
) -> Result<&'a str, cleanup::CleanupError> {
    match cleanup_settings
        .provider_keys
        .get(provider.as_str())
        .filter(|k| !k.is_empty())
    {
        Some(k) => Ok(k),
        None => Err(cleanup::CleanupError::Credential(format!(
            "AI cleanup is enabled but the {} API key is not set.",
            provider.as_str()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::{AiProviderId, CleanupError, Transport, TransportResponse};
    use crate::config::{AiCleanupSettings, CleanupAuthMode, CustomProvider};

    struct MockTransport {
        response: Box<dyn Fn() -> Result<TransportResponse, String> + Send + Sync>,
    }

    impl MockTransport {
        fn returning(status: u16, body: impl Into<String>) -> Self {
            let body = body.into();
            MockTransport {
                response: Box::new(move || {
                    Ok(TransportResponse {
                        status,
                        body: body.clone(),
                    })
                }),
            }
        }

        fn failing(err: impl Into<String>) -> Self {
            let err = err.into();
            MockTransport {
                response: Box::new(move || Err(err.clone())),
            }
        }
    }

    impl Transport for MockTransport {
        fn post<'a>(
            &'a self,
            _url: &'a str,
            _headers: &'a [(String, String)],
            _body: &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<TransportResponse, String>> + Send + 'a>,
        > {
            let result = (self.response)();
            Box::pin(async move { result })
        }
    }

    struct HangingTransport;

    impl Transport for HangingTransport {
        fn post<'a>(
            &'a self,
            _url: &'a str,
            _headers: &'a [(String, String)],
            _body: &'a serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<TransportResponse, String>> + Send + 'a>,
        > {
            Box::pin(std::future::pending())
        }
    }

    fn settings_with_anthropic_api_key(key: &str) -> AiCleanupSettings {
        let mut s = AiCleanupSettings::default();
        s.provider_keys
            .insert("anthropic".to_string(), key.to_string());
        s
    }

    fn settings_with_anthropic_oauth(token: &str) -> AiCleanupSettings {
        let mut s = AiCleanupSettings::default();
        s.auth_mode = CleanupAuthMode::Oauth;
        s.anthropic_oauth_token = Some(token.to_string());
        s
    }

    fn settings_with_provider_key(provider: &str, key: &str) -> AiCleanupSettings {
        let mut s = AiCleanupSettings::default();
        s.provider_keys
            .insert(provider.to_string(), key.to_string());
        s
    }

    fn settings_with_custom_provider(base_url: &str, model: &str) -> AiCleanupSettings {
        let mut s = AiCleanupSettings::default();
        s.custom_provider = Some(CustomProvider {
            base_url: base_url.to_string(),
            model: model.to_string(),
            api_key: Some("custom-key".to_string()),
        });
        s
    }

    fn anthropic_success_body(text: &str) -> String {
        serde_json::json!({
            "content": [{"type": "text", "text": text}],
            "usage": {
                "input_tokens": 10,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "output_tokens": 5
            }
        })
        .to_string()
    }

    fn openai_success_body(text: &str) -> String {
        serde_json::json!({
            "choices": [{"message": {"content": text}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        })
        .to_string()
    }

    fn error_body(message: &str) -> String {
        serde_json::json!({"error": {"message": message}}).to_string()
    }

    const FIVE_SECS: Duration = Duration::from_secs(5);

    // --- is_credential_configured tests ---

    #[test]
    fn api_key_missing_not_configured() {
        let s = AiCleanupSettings::default();
        assert!(!is_credential_configured(&s, AiProviderId::Anthropic));
    }

    #[test]
    fn api_key_present_is_configured() {
        let s = settings_with_anthropic_api_key("sk-ant-key");
        assert!(is_credential_configured(&s, AiProviderId::Anthropic));
    }

    #[test]
    fn oauth_no_token_not_configured() {
        let mut s = AiCleanupSettings::default();
        s.auth_mode = CleanupAuthMode::Oauth;
        assert!(!is_credential_configured(&s, AiProviderId::Anthropic));
    }

    #[test]
    fn oauth_token_present_is_configured() {
        let s = settings_with_anthropic_oauth("oauth-tok");
        assert!(is_credential_configured(&s, AiProviderId::Anthropic));
    }

    #[test]
    fn custom_no_base_url_not_configured() {
        let s = AiCleanupSettings::default();
        assert!(!is_credential_configured(&s, AiProviderId::Custom));
    }

    #[test]
    fn custom_base_url_present_is_configured() {
        let s = settings_with_custom_provider("https://example.com", "model");
        assert!(is_credential_configured(&s, AiProviderId::Custom));
    }

    #[test]
    fn openai_key_missing_not_configured() {
        let s = AiCleanupSettings::default();
        assert!(!is_credential_configured(&s, AiProviderId::OpenAi));
    }

    #[test]
    fn openai_key_present_is_configured() {
        let s = settings_with_provider_key("openai", "sk-openai-key");
        assert!(is_credential_configured(&s, AiProviderId::OpenAi));
    }

    // --- invoke_with_transport: routing tests ---

    #[tokio::test]
    async fn anthropic_api_key_returns_cleaned_text() {
        let settings = settings_with_anthropic_api_key("sk-ant-key");
        let transport = MockTransport::returning(200, anthropic_success_body("Clean output."));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        let (text, _) = result.expect("should succeed");
        assert_eq!(text, "Clean output.");
    }

    #[tokio::test]
    async fn anthropic_oauth_returns_cleaned_text() {
        let settings = settings_with_anthropic_oauth("oauth-token");
        let transport = MockTransport::returning(200, anthropic_success_body("OAuth clean."));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        let (text, _) = result.expect("should succeed");
        assert_eq!(text, "OAuth clean.");
    }

    #[tokio::test]
    async fn anthropic_missing_api_key_returns_credential_error() {
        let settings = AiCleanupSettings::default();
        let transport = MockTransport::returning(200, anthropic_success_body("irrelevant"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn anthropic_missing_oauth_returns_credential_error() {
        let mut settings = AiCleanupSettings::default();
        settings.auth_mode = CleanupAuthMode::Oauth;
        let transport = MockTransport::returning(200, anthropic_success_body("irrelevant"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn openai_provider_returns_cleaned_text() {
        let settings = settings_with_provider_key("openai", "sk-openai-key");
        let transport = MockTransport::returning(200, openai_success_body("OpenAI clean."));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::OpenAi,
            "gpt-4o-mini",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        let (text, _) = result.expect("should succeed");
        assert_eq!(text, "OpenAI clean.");
    }

    #[tokio::test]
    async fn openai_missing_key_returns_credential_error() {
        let settings = AiCleanupSettings::default();
        let transport = MockTransport::returning(200, openai_success_body("irrelevant"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::OpenAi,
            "gpt-4o-mini",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn custom_provider_returns_cleaned_text() {
        let settings = settings_with_custom_provider("https://my-llm.example.com", "my-model");
        let transport = MockTransport::returning(200, openai_success_body("Custom clean."));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Custom,
            "ignored-model",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        let (text, _) = result.expect("should succeed");
        assert_eq!(text, "Custom clean.");
    }

    #[tokio::test]
    async fn custom_provider_not_configured_returns_credential_error() {
        let settings = AiCleanupSettings::default();
        let transport = MockTransport::returning(200, openai_success_body("irrelevant"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Custom,
            "model",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    // --- invoke_with_transport: transport outcome mapping tests ---

    #[tokio::test]
    async fn transport_timeout_maps_to_timeout_error() {
        let settings = settings_with_anthropic_api_key("sk-ant-key");
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &HangingTransport,
            Duration::from_millis(50),
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Timeout(_))),
            "expected Timeout error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn transport_401_maps_to_credential_error() {
        let settings = settings_with_anthropic_api_key("sk-ant-bad-key");
        let transport = MockTransport::returning(401, error_body("invalid x-api-key"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Credential(_))),
            "expected Credential error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn transport_500_maps_to_transient_error() {
        let settings = settings_with_anthropic_api_key("sk-ant-key");
        let transport = MockTransport::returning(500, error_body("internal server error"));
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Transient(_))),
            "expected Transient error, got {result:?}"
        );
    }

    #[tokio::test]
    async fn network_failure_maps_to_transient_error() {
        let settings = settings_with_anthropic_api_key("sk-ant-key");
        let transport = MockTransport::failing("connection refused");
        let result = invoke_with_transport(
            &settings,
            AiProviderId::Anthropic,
            "claude-haiku-4-5",
            None,
            "raw transcript",
            &transport,
            FIVE_SECS,
        )
        .await;
        assert!(
            matches!(result, Err(CleanupError::Transient(_))),
            "expected Transient error, got {result:?}"
        );
    }
}
