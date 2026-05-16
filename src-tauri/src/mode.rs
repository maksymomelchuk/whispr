use serde::{Deserialize, Serialize};

pub type ModeId = String;

pub const SEED_MODE_DEFAULT_EN: &str = "mode-default-en";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModeLanguage {
    Auto,
    #[serde(rename = "exact")]
    Exact { code: String },
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

    /// Returns the ISO code for `Exact`, `None` for `Auto`.
    pub fn as_code(&self) -> Option<&str> {
        match self {
            ModeLanguage::Exact { code } => Some(code.as_str()),
            ModeLanguage::Auto => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranslateTarget {
    Off,
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
    /// Creates the seeded default-English mode.
    pub fn seed_default_en(cleanup_enabled: bool) -> Self {
        Mode {
            id: SEED_MODE_DEFAULT_EN.to_string(),
            name: "Default".to_string(),
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
    fn mode_language_round_trips() {
        let cases = vec![
            ModeLanguage::Auto,
            ModeLanguage::exact("en"),
            ModeLanguage::exact("uk"),
            ModeLanguage::exact("fr"),
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
    fn mode_round_trips() {
        let mode = Mode::seed_default_en(false);
        let json = serde_json::to_string(&mode).unwrap();
        let decoded: Mode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, SEED_MODE_DEFAULT_EN);
        assert_eq!(decoded.name, "Default");
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
}
