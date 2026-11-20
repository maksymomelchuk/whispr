use crate::cleanup::AiProviderId;
use crate::mode::Mode;

pub const DEFAULT_CLEANUP_MODEL: &str = "claude-haiku-5-5";

// Every cleanup model the picker has ever offered that it no longer does, with
// its successor. Some of these are no longer served by the provider at all, and
// keeping a delisted ID would leave the picker showing a blank value. IDs the
// user typed for a Custom provider never appear here, so they are left alone.
const DELISTED_CLEANUP_MODELS: &[(AiProviderId, &str, &str)] = &[
    (
        AiProviderId::Anthropic,
        "claude-haiku-4-5",
        "claude-haiku-5-5",
    ),
    (
        AiProviderId::Anthropic,
        "claude-sonnet-4-6",
        "claude-sonnet-5-5",
    ),
    (
        AiProviderId::Anthropic,
        "claude-opus-4-8",
        "claude-opus-5-5",
    ),
    (AiProviderId::OpenAi, "gpt-4o-mini", "gpt-6-luna"),
    (AiProviderId::OpenAi, "gpt-4o", "gpt-6.2-sol"),
    (AiProviderId::OpenAi, "gpt-5-mini", "gpt-6-luna"),
    (AiProviderId::OpenAi, "gpt-5-nano", "gpt-6-luna"),
    (AiProviderId::OpenAi, "gpt-5.4", "gpt-6.2-sol"),
    (AiProviderId::OpenAi, "gpt-5.5", "gpt-6.2-sol"),
    (
        AiProviderId::Google,
        "gemini-2.0-flash",
        "gemini-3.5-flash-lite",
    ),
    (
        AiProviderId::Google,
        "gemini-2.5-flash",
        "gemini-3.5-flash-lite",
    ),
    (AiProviderId::Google, "gemini-2.5-pro", "gemini-3.5-flash"),
    (
        AiProviderId::Google,
        "gemini-3.1-pro-preview",
        "gemini-3.5-flash",
    ),
    (
        AiProviderId::Groq,
        "llama-3.1-8b-instant",
        "openai/gpt-oss-20b",
    ),
    (
        AiProviderId::Groq,
        "llama-3.3-70b-versatile",
        "openai/gpt-oss-120b",
    ),
    (AiProviderId::Groq, "qwen/qwen3-32b", "qwen/qwen3.8-27b"),
    (AiProviderId::Groq, "qwen/qwen3.6-27b", "qwen/qwen3.8-27b"),
    (AiProviderId::DeepSeek, "deepseek-chat", "deepseek-flash"),
    (
        AiProviderId::DeepSeek,
        "deepseek-reasoner",
        "deepseek-flash",
    ),
    (
        AiProviderId::DeepSeek,
        "deepseek-v4-flash",
        "deepseek-flash",
    ),
    (AiProviderId::Cerebras, "llama-3.3-70b", "gpt-oss-120b"),
    (AiProviderId::Cerebras, "llama3.1-8b", "gpt-oss-120b"),
    (
        AiProviderId::Cerebras,
        "qwen-3-235b-a22b-instruct-2507",
        "qwen-3.8-27b",
    ),
    (
        AiProviderId::OpenRouter,
        "anthropic/claude-haiku-4.5",
        "anthropic/claude-haiku-5.5",
    ),
    (
        AiProviderId::OpenRouter,
        "openai/gpt-5-mini",
        "openai/gpt-6-luna",
    ),
    (
        AiProviderId::OpenRouter,
        "openai/gpt-5-nano",
        "openai/gpt-6-luna",
    ),
    (
        AiProviderId::OpenRouter,
        "google/gemini-2.0-flash-001",
        "google/gemini-3.5-flash-lite",
    ),
    (
        AiProviderId::OpenRouter,
        "google/gemini-2.5-flash",
        "google/gemini-3.5-flash-lite",
    ),
    (
        AiProviderId::OpenRouter,
        "meta-llama/llama-3.3-70b-instruct",
        "anthropic/claude-haiku-5.5",
    ),
];

fn successor_of(provider: AiProviderId, model: &str) -> Option<&'static str> {
    DELISTED_CLEANUP_MODELS
        .iter()
        .find(|(delisted_provider, delisted_model, _)| {
            *delisted_provider == provider && *delisted_model == model
        })
        .map(|(_, _, successor)| *successor)
}

/// Points every mode whose cleanup model was delisted at its successor.
/// Returns `true` if any mode changed.
pub fn replace_delisted_cleanup_models(modes: &mut [Mode]) -> bool {
    let mut changed = false;
    for mode in modes.iter_mut() {
        let cleanup = &mut mode.ai_cleanup;
        if let Some(successor) = successor_of(cleanup.provider, &cleanup.model) {
            cleanup.model = successor.to_string();
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_with_cleanup(provider: AiProviderId, model: &str) -> Mode {
        let mut mode = Mode::seed_default_en(true);
        mode.ai_cleanup.provider = provider;
        mode.ai_cleanup.model = model.to_string();
        mode
    }

    #[test]
    fn delisted_model_is_replaced_with_its_successor() {
        let mut modes = vec![mode_with_cleanup(AiProviderId::Cerebras, "llama-3.3-70b")];
        assert!(replace_delisted_cleanup_models(&mut modes));
        assert_eq!(modes[0].ai_cleanup.model, "gpt-oss-120b");
    }

    #[test]
    fn current_model_is_left_unchanged() {
        let mut modes = vec![mode_with_cleanup(AiProviderId::Groq, "openai/gpt-oss-120b")];
        assert!(!replace_delisted_cleanup_models(&mut modes));
        assert_eq!(modes[0].ai_cleanup.model, "openai/gpt-oss-120b");
    }

    #[test]
    fn delisted_id_under_another_provider_is_left_unchanged() {
        let mut modes = vec![mode_with_cleanup(AiProviderId::Custom, "gpt-4o-mini")];
        assert!(!replace_delisted_cleanup_models(&mut modes));
        assert_eq!(modes[0].ai_cleanup.model, "gpt-4o-mini");
    }

    #[test]
    fn no_successor_is_itself_delisted() {
        for (provider, _, successor) in DELISTED_CLEANUP_MODELS {
            assert_eq!(
                successor_of(*provider, successor),
                None,
                "{successor} is listed as both delisted and a successor"
            );
        }
    }
}
