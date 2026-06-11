use crate::cleanup::AiProviderId;
use crate::provider::ProviderModel;
use serde::{Deserialize, Serialize};

pub type ModeId = String;
pub type SetId = String;

pub const SEED_MODE_DEFAULT_EN: &str = "mode-default-en";
pub const SEED_MODE_CLEANED_EN: &str = "mode-cleaned-en";
pub const SEED_MODE_UKRAINIAN: &str = "mode-ukrainian";
pub const SEED_MODE_UA_EN: &str = "mode-ua-en";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModeLanguage {
    Auto,
    #[serde(rename = "exact")]
    Exact {
        code: String,
    },
    // Two or more language codes the user expects to speak; providers treat
    // this as a multi-language hint rather than a hard constraint.
    #[serde(rename = "hints")]
    Hints {
        codes: Vec<String>,
    },
}

impl Default for ModeLanguage {
    fn default() -> Self {
        ModeLanguage::Exact {
            code: "en".to_string(),
        }
    }
}

impl ModeLanguage {
    pub fn exact(code: impl Into<String>) -> Self {
        ModeLanguage::Exact { code: code.into() }
    }

    #[cfg(test)]
    pub fn hints(codes: Vec<String>) -> Self {
        ModeLanguage::Hints { codes }
    }

    /// Returns the ISO code for `Exact`. Returns `None` for `Auto` and `Hints`
    /// because neither maps to a single authoritative code.
    pub fn as_code(&self) -> Option<&str> {
        match self {
            ModeLanguage::Exact { code } => Some(code.as_str()),
            ModeLanguage::Auto | ModeLanguage::Hints { .. } => None,
        }
    }
}

fn default_cleanup_model() -> String {
    "claude-haiku-4-5".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeCleanup {
    pub enabled: bool,
    pub prompt_override: Option<String>,
    #[serde(default)]
    pub provider: AiProviderId,
    #[serde(default = "default_cleanup_model")]
    pub model: String,
    #[serde(default = "default_true")]
    pub paste_raw_on_failure: bool,
    // Defaults to false so existing behaviour is unchanged after upgrade.
    #[serde(default)]
    pub clipboard_context_enabled: bool,
    // Defaults to false so existing behaviour is unchanged after upgrade.
    #[serde(default)]
    pub selected_text_context_enabled: bool,
    // Defaults to false so existing behaviour is unchanged after upgrade.
    #[serde(default)]
    pub focused_field_context_enabled: bool,
    // Defaults to false so existing behaviour is unchanged after upgrade.
    #[serde(default)]
    pub post_paste_observation_enabled: bool,
}

impl Default for ModeCleanup {
    fn default() -> Self {
        ModeCleanup {
            enabled: false,
            prompt_override: None,
            provider: AiProviderId::default(),
            model: default_cleanup_model(),
            paste_raw_on_failure: true,
            clipboard_context_enabled: false,
            selected_text_context_enabled: false,
            focused_field_context_enabled: false,
            post_paste_observation_enabled: false,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mode {
    pub id: ModeId,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub language: ModeLanguage,
    pub ai_cleanup: ModeCleanup,
    /// Legacy field; migrated to use_terms + use_corrections on first load.
    #[serde(rename = "use_dictionary", default, skip_serializing)]
    pub legacy_use_dictionary: Option<bool>,
    /// Legacy boolean; read on load to populate term_set_ids, then not written back.
    #[serde(default = "default_true", skip_serializing)]
    pub use_terms: bool,
    /// Legacy field; read during migration to seed correction_set_ids, then dropped.
    #[serde(default = "default_true", skip_serializing)]
    pub use_corrections: bool,
    #[serde(default = "default_true")]
    pub use_snippets: bool,
    #[serde(default)]
    pub provider_model: ProviderModel,
    #[serde(default)]
    pub term_set_ids: Vec<SetId>,
    #[serde(default)]
    pub correction_set_ids: Vec<SetId>,
}

impl Mode {
    /// `cleanup_enabled` is carried over from the legacy flat toggle during migration;
    /// fresh installs pass false.
    pub fn seed_default_en(cleanup_enabled: bool) -> Self {
        Mode {
            id: SEED_MODE_DEFAULT_EN.to_string(),
            name: "Default English".to_string(),
            icon: None,
            language: ModeLanguage::exact("en"),
            ai_cleanup: ModeCleanup {
                enabled: cleanup_enabled,
                ..ModeCleanup::default()
            },
            legacy_use_dictionary: None,
            use_terms: true,
            use_corrections: true,
            use_snippets: true,
            provider_model: ProviderModel::Deepgram,
            term_set_ids: vec![],
            correction_set_ids: vec![],
        }
    }

    pub fn seed_cleaned_en() -> Self {
        Mode {
            id: SEED_MODE_CLEANED_EN.to_string(),
            name: "Cleaned English".to_string(),
            icon: None,
            language: ModeLanguage::exact("en"),
            ai_cleanup: ModeCleanup {
                enabled: true,
                ..ModeCleanup::default()
            },
            legacy_use_dictionary: None,
            use_terms: true,
            use_corrections: true,
            use_snippets: true,
            provider_model: ProviderModel::Deepgram,
            term_set_ids: vec![],
            correction_set_ids: vec![],
        }
    }

    pub fn seed_ukrainian() -> Self {
        Mode {
            id: SEED_MODE_UKRAINIAN.to_string(),
            name: "Ukrainian".to_string(),
            icon: None,
            language: ModeLanguage::exact("uk"),
            ai_cleanup: ModeCleanup {
                enabled: false,
                ..ModeCleanup::default()
            },
            legacy_use_dictionary: None,
            use_terms: true,
            use_corrections: true,
            use_snippets: true,
            provider_model: ProviderModel::Deepgram,
            term_set_ids: vec![],
            correction_set_ids: vec![],
        }
    }

    pub fn seed_ua_en() -> Self {
        Mode {
            id: SEED_MODE_UA_EN.to_string(),
            name: "UA \u{2192} EN".to_string(),
            icon: None,
            language: ModeLanguage::exact("uk"),
            ai_cleanup: ModeCleanup {
                enabled: true,
                prompt_override: Some(
                    "Translate the following Ukrainian transcription to English. \
                     Output only the translated text, nothing else."
                        .to_string(),
                ),
                paste_raw_on_failure: false,
                ..ModeCleanup::default()
            },
            legacy_use_dictionary: None,
            use_terms: true,
            use_corrections: true,
            use_snippets: true,
            provider_model: ProviderModel::Deepgram,
            term_set_ids: vec![],
            correction_set_ids: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_language_exact_serializes_with_kind_and_code() {
        let lang = ModeLanguage::exact("en");
        let json = serde_json::to_string(&lang).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "exact");
        assert_eq!(v["code"], "en");
    }

    #[test]
    fn mode_language_auto_serializes_with_kind_only() {
        let lang = ModeLanguage::Auto;
        let json = serde_json::to_string(&lang).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "auto");
        assert!(v.get("code").is_none());
    }

    #[test]
    fn mode_language_hints_serializes_with_kind_and_codes() {
        let lang = ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]);
        let json = serde_json::to_string(&lang).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "hints");
        assert_eq!(v["codes"][0], "en");
        assert_eq!(v["codes"][1], "uk");
    }

    #[test]
    fn mode_language_round_trips() {
        let cases = vec![
            ModeLanguage::Auto,
            ModeLanguage::exact("en"),
            ModeLanguage::exact("uk"),
            ModeLanguage::exact("fr"),
            ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]),
        ];
        for case in cases {
            let json = serde_json::to_string(&case).unwrap();
            let decoded: ModeLanguage = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, case);
        }
    }

    #[test]
    fn mode_language_as_code_returns_none_for_auto() {
        assert_eq!(ModeLanguage::Auto.as_code(), None);
    }

    #[test]
    fn mode_language_as_code_returns_none_for_hints() {
        assert_eq!(
            ModeLanguage::hints(vec!["en".to_string(), "uk".to_string()]).as_code(),
            None
        );
    }

    #[test]
    fn mode_language_as_code_returns_code_for_exact() {
        assert_eq!(ModeLanguage::exact("uk").as_code(), Some("uk"));
    }

    #[test]
    fn mode_round_trips() {
        let mode = Mode::seed_default_en(false);
        let json = serde_json::to_string(&mode).unwrap();
        let decoded: Mode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, SEED_MODE_DEFAULT_EN);
        assert_eq!(decoded.name, "Default English");
        assert_eq!(decoded.language, ModeLanguage::exact("en"));
        assert!(!decoded.ai_cleanup.enabled);
        assert!(decoded.use_snippets);
        assert!(decoded.term_set_ids.is_empty());
        assert!(decoded.correction_set_ids.is_empty());
    }

    #[test]
    fn use_dictionary_false_deserializes_to_legacy_field() {
        let json = r#"{"id":"x","name":"X","language":{"kind":"exact","code":"en"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_dictionary":false,"use_snippets":true}"#;
        let mode: Mode = serde_json::from_str(json).unwrap();
        assert_eq!(mode.legacy_use_dictionary, Some(false));
        // use_terms and use_corrections default to true until migrated
        assert!(mode.use_terms);
        assert!(mode.use_corrections);
    }

    #[test]
    fn use_dictionary_true_deserializes_to_legacy_field() {
        let json = r#"{"id":"x","name":"X","language":{"kind":"exact","code":"en"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_dictionary":true,"use_snippets":true}"#;
        let mode: Mode = serde_json::from_str(json).unwrap();
        assert_eq!(mode.legacy_use_dictionary, Some(true));
    }

    #[test]
    fn use_dictionary_absent_gives_none() {
        let json = r#"{"id":"x","name":"X","language":{"kind":"exact","code":"en"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_terms":true,"use_corrections":true,"use_snippets":true}"#;
        let mode: Mode = serde_json::from_str(json).unwrap();
        assert_eq!(mode.legacy_use_dictionary, None);
    }

    #[test]
    fn seed_mode_does_not_serialize_legacy_fields() {
        let mode = Mode::seed_default_en(false);
        let json = serde_json::to_string(&mode).unwrap();
        assert!(
            !json.contains("use_dictionary"),
            "use_dictionary must not appear"
        );
        assert!(!json.contains("use_terms"), "use_terms is skip_serializing");
        assert!(
            !json.contains("use_corrections"),
            "use_corrections is skip_serializing"
        );
        assert!(json.contains("term_set_ids"));
        assert!(json.contains("correction_set_ids"));
    }

    #[test]
    fn seed_default_en_carries_cleanup_enabled_flag() {
        let on = Mode::seed_default_en(true);
        assert!(on.ai_cleanup.enabled);
        let off = Mode::seed_default_en(false);
        assert!(!off.ai_cleanup.enabled);
    }

    #[test]
    fn seed_ua_en_uses_ai_cleanup_with_translation_prompt() {
        let m = Mode::seed_ua_en();
        assert_eq!(m.id, SEED_MODE_UA_EN);
        assert_eq!(m.language, ModeLanguage::exact("uk"));
        assert!(m.ai_cleanup.enabled);
        assert!(
            m.ai_cleanup
                .prompt_override
                .as_deref()
                .unwrap_or("")
                .contains("Ukrainian"),
            "prompt_override must ask for Ukrainian → English translation"
        );
    }

    #[test]
    fn old_config_with_translate_field_deserializes_without_error() {
        let json = r#"{"id":"x","name":"X","language":{"kind":"exact","code":"en"},"translate":{"kind":"apple","target":"en"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}"#;
        let mode: Mode = serde_json::from_str(json).unwrap();
        assert_eq!(mode.id, "x");
        assert_eq!(mode.language, ModeLanguage::exact("en"));
    }

    #[test]
    fn mode_cleanup_disabled_with_prompt_override_carries_both_fields() {
        let c = ModeCleanup {
            enabled: false,
            prompt_override: Some("custom prompt".to_string()),
            ..ModeCleanup::default()
        };
        assert!(!c.enabled);
        assert_eq!(c.prompt_override.as_deref(), Some("custom prompt"));
    }

    #[test]
    fn mode_cleanup_prompt_override_none_is_default() {
        let c = ModeCleanup {
            enabled: true,
            prompt_override: None,
            ..ModeCleanup::default()
        };
        assert!(c.prompt_override.is_none());
    }

    #[test]
    fn mode_cleanup_defaults_to_anthropic_and_haiku() {
        let c = ModeCleanup::default();
        assert_eq!(c.provider, AiProviderId::Anthropic);
        assert_eq!(c.model, "claude-haiku-4-5");
    }

    #[test]
    fn mode_cleanup_missing_provider_and_model_deserialize_with_defaults() {
        let json = r#"{"enabled":false,"prompt_override":null}"#;
        let c: ModeCleanup = serde_json::from_str(json).unwrap();
        assert_eq!(c.provider, AiProviderId::Anthropic);
        assert_eq!(c.model, "claude-haiku-4-5");
    }

    #[test]
    fn mode_cleanup_provider_and_model_round_trip() {
        let c = ModeCleanup {
            enabled: true,
            prompt_override: None,
            provider: AiProviderId::OpenAi,
            model: "gpt-4o-mini".to_string(),
            paste_raw_on_failure: true,
            clipboard_context_enabled: false,
            selected_text_context_enabled: false,
            focused_field_context_enabled: false,
            post_paste_observation_enabled: false,
        };
        let json = serde_json::to_string(&c).unwrap();
        let decoded: ModeCleanup = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.provider, AiProviderId::OpenAi);
        assert_eq!(decoded.model, "gpt-4o-mini");
    }

    #[test]
    fn mode_cleanup_paste_raw_on_failure_defaults_to_true() {
        let c = ModeCleanup::default();
        assert!(c.paste_raw_on_failure);
    }

    #[test]
    fn mode_cleanup_paste_raw_on_failure_deserializes_to_true_when_absent() {
        let json = r#"{"enabled":false,"prompt_override":null}"#;
        let c: ModeCleanup = serde_json::from_str(json).unwrap();
        assert!(c.paste_raw_on_failure);
    }

    #[test]
    fn mode_cleanup_selected_text_context_defaults_to_false() {
        let c = ModeCleanup::default();
        assert!(!c.selected_text_context_enabled);
    }

    #[test]
    fn mode_cleanup_selected_text_context_deserializes_to_false_when_absent() {
        let json = r#"{"enabled":false,"prompt_override":null}"#;
        let c: ModeCleanup = serde_json::from_str(json).unwrap();
        assert!(!c.selected_text_context_enabled);
    }

    #[test]
    fn mode_cleanup_focused_field_context_defaults_to_false() {
        let c = ModeCleanup::default();
        assert!(!c.focused_field_context_enabled);
    }

    #[test]
    fn mode_cleanup_focused_field_context_deserializes_to_false_when_absent() {
        let json = r#"{"enabled":false,"prompt_override":null}"#;
        let c: ModeCleanup = serde_json::from_str(json).unwrap();
        assert!(!c.focused_field_context_enabled);
    }

    #[test]
    fn mode_cleanup_post_paste_observation_defaults_to_false() {
        let c = ModeCleanup::default();
        assert!(!c.post_paste_observation_enabled);
    }

    #[test]
    fn mode_cleanup_post_paste_observation_deserializes_to_false_when_absent() {
        let json = r#"{"enabled":false,"prompt_override":null}"#;
        let c: ModeCleanup = serde_json::from_str(json).unwrap();
        assert!(!c.post_paste_observation_enabled);
    }

    #[test]
    fn seed_ua_en_has_paste_raw_on_failure_false() {
        let m = Mode::seed_ua_en();
        assert!(!m.ai_cleanup.paste_raw_on_failure);
    }

    #[test]
    fn all_four_seed_ids_are_distinct() {
        let ids = [
            SEED_MODE_DEFAULT_EN,
            SEED_MODE_CLEANED_EN,
            SEED_MODE_UKRAINIAN,
            SEED_MODE_UA_EN,
        ];
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
    }
}
