use serde::{Deserialize, Serialize};

pub type ModeId = String;

pub const SEED_MODE_DEFAULT_EN: &str = "mode-default-en";
pub const SEED_MODE_CLEANED_EN: &str = "mode-cleaned-en";
pub const SEED_MODE_UKRAINIAN: &str = "mode-ukrainian";
pub const SEED_MODE_UA_EN: &str = "mode-ua-en";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModeLanguage {
    Auto,
    #[serde(rename = "exact")]
    Exact { code: String },
    // Two or more language codes the user expects to speak; providers treat
    // this as a multi-language hint rather than a hard constraint.
    #[serde(rename = "hints")]
    Hints { codes: Vec<String> },
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranslateTarget {
    Off,
    Apple { target: String },
}

impl Default for TranslateTarget {
    fn default() -> Self {
        TranslateTarget::Off
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeCleanup {
    pub enabled: bool,
    pub prompt_override: Option<String>,
}

impl Default for ModeCleanup {
    fn default() -> Self {
        ModeCleanup {
            enabled: false,
            prompt_override: None,
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
    pub translate: TranslateTarget,
    pub ai_cleanup: ModeCleanup,
    #[serde(default = "default_true")]
    pub use_dictionary: bool,
    #[serde(default = "default_true")]
    pub use_snippets: bool,
}

impl Mode {
    /// Creates the seeded default-English mode. `cleanup_enabled` is carried
    /// over from the legacy flat toggle during migration; fresh installs pass false.
    pub fn seed_default_en(cleanup_enabled: bool) -> Self {
        Mode {
            id: SEED_MODE_DEFAULT_EN.to_string(),
            name: "Default English".to_string(),
            icon: None,
            language: ModeLanguage::exact("en"),
            translate: TranslateTarget::Off,
            ai_cleanup: ModeCleanup {
                enabled: cleanup_enabled,
                prompt_override: None,
            },
            use_dictionary: true,
            use_snippets: true,
        }
    }

    pub fn seed_cleaned_en() -> Self {
        Mode {
            id: SEED_MODE_CLEANED_EN.to_string(),
            name: "Cleaned English".to_string(),
            icon: None,
            language: ModeLanguage::exact("en"),
            translate: TranslateTarget::Off,
            ai_cleanup: ModeCleanup {
                enabled: true,
                prompt_override: None,
            },
            use_dictionary: true,
            use_snippets: true,
        }
    }

    pub fn seed_ukrainian() -> Self {
        Mode {
            id: SEED_MODE_UKRAINIAN.to_string(),
            name: "Ukrainian".to_string(),
            icon: None,
            language: ModeLanguage::exact("uk"),
            translate: TranslateTarget::Off,
            ai_cleanup: ModeCleanup {
                enabled: false,
                prompt_override: None,
            },
            use_dictionary: true,
            use_snippets: true,
        }
    }

    pub fn seed_ua_en() -> Self {
        Mode {
            id: SEED_MODE_UA_EN.to_string(),
            name: "UA \u{2192} EN".to_string(),
            icon: None,
            language: ModeLanguage::exact("uk"),
            translate: TranslateTarget::Apple {
                target: "en".to_string(),
            },
            ai_cleanup: ModeCleanup {
                enabled: true,
                prompt_override: None,
            },
            use_dictionary: true,
            use_snippets: true,
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
    fn translate_target_off_serializes_with_kind() {
        let t = TranslateTarget::Off;
        let json = serde_json::to_string(&t).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "off");
    }

    #[test]
    fn translate_target_apple_serializes_with_kind_and_target() {
        let t = TranslateTarget::Apple {
            target: "en".to_string(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "apple");
        assert_eq!(v["target"], "en");
    }

    #[test]
    fn translate_target_round_trips() {
        let cases = vec![
            TranslateTarget::Off,
            TranslateTarget::Apple {
                target: "en".to_string(),
            },
        ];
        for case in cases {
            let json = serde_json::to_string(&case).unwrap();
            let decoded: TranslateTarget = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, case);
        }
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
        assert!(decoded.use_dictionary);
        assert!(decoded.use_snippets);
    }

    #[test]
    fn seed_default_en_carries_cleanup_enabled_flag() {
        let on = Mode::seed_default_en(true);
        assert!(on.ai_cleanup.enabled);
        let off = Mode::seed_default_en(false);
        assert!(!off.ai_cleanup.enabled);
    }

    #[test]
    fn seed_ua_en_has_apple_translate_target() {
        let m = Mode::seed_ua_en();
        assert_eq!(m.id, SEED_MODE_UA_EN);
        assert_eq!(m.language, ModeLanguage::exact("uk"));
        assert_eq!(
            m.translate,
            TranslateTarget::Apple {
                target: "en".to_string()
            }
        );
        assert!(m.ai_cleanup.enabled);
    }

    #[test]
    fn mode_cleanup_disabled_with_prompt_override_carries_both_fields() {
        let c = ModeCleanup {
            enabled: false,
            prompt_override: Some("custom prompt".to_string()),
        };
        assert!(!c.enabled);
        assert_eq!(c.prompt_override.as_deref(), Some("custom prompt"));
    }

    #[test]
    fn mode_cleanup_prompt_override_none_is_default() {
        let c = ModeCleanup {
            enabled: true,
            prompt_override: None,
        };
        assert!(c.prompt_override.is_none());
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
