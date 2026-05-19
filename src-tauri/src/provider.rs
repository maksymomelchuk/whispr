use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    #[default]
    Deepgram,
    Groq,
    AssemblyAi,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroqModel {
    WhisperLargeV3,
    #[default]
    WhisperLargeV3Turbo,
}

impl GroqModel {
    pub fn api_id(self) -> &'static str {
        match self {
            Self::WhisperLargeV3 => "whisper-large-v3",
            Self::WhisperLargeV3Turbo => "whisper-large-v3-turbo",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyAiModel {
    #[default]
    UniversalProStreaming,
    UniversalStreamingEnglish,
    UniversalStreamingMultilingual,
    WhisperStreaming,
}

impl AssemblyAiModel {
    pub fn api_id(self) -> &'static str {
        match self {
            Self::UniversalProStreaming => "u3-rt-pro",
            Self::UniversalStreamingEnglish => "universal-streaming-english",
            Self::UniversalStreamingMultilingual => "universal-streaming-multilingual",
            Self::WhisperStreaming => "whisper-rt",
        }
    }

    pub fn supports_language(self, code: &str) -> bool {
        match self {
            Self::UniversalStreamingEnglish => code == "en",
            Self::UniversalProStreaming | Self::UniversalStreamingMultilingual => {
                matches!(code, "en" | "es" | "de" | "fr" | "pt" | "it")
            }
            Self::WhisperStreaming => true,
        }
    }

    pub fn supported_language_codes(self) -> Option<Vec<&'static str>> {
        match self {
            Self::UniversalStreamingEnglish => Some(vec!["en"]),
            Self::UniversalProStreaming | Self::UniversalStreamingMultilingual => {
                Some(vec!["en", "es", "de", "fr", "pt", "it"])
            }
            Self::WhisperStreaming => None,
        }
    }
}

/// Per-mode provider + model selection. The tag field doubles as the provider
/// identifier so the frontend only needs one discriminant for both.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderModel {
    #[default]
    Deepgram,
    Groq { model: GroqModel },
    AssemblyAi { model: AssemblyAiModel },
}

impl ProviderModel {
    pub fn provider(&self) -> TranscriptionProvider {
        match self {
            Self::Deepgram => TranscriptionProvider::Deepgram,
            Self::Groq { .. } => TranscriptionProvider::Groq,
            Self::AssemblyAi { .. } => TranscriptionProvider::AssemblyAi,
        }
    }

    pub fn from_legacy(
        provider: TranscriptionProvider,
        groq_model: GroqModel,
        assemblyai_model: AssemblyAiModel,
    ) -> Self {
        match provider {
            TranscriptionProvider::Deepgram => Self::Deepgram,
            TranscriptionProvider::Groq => Self::Groq { model: groq_model },
            TranscriptionProvider::AssemblyAi => Self::AssemblyAi { model: assemblyai_model },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_model_deepgram_serializes_with_provider_tag() {
        let m = ProviderModel::Deepgram;
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "deepgram");
        assert!(v.get("model").is_none());
    }

    #[test]
    fn provider_model_groq_serializes_with_provider_and_model() {
        let m = ProviderModel::Groq { model: GroqModel::WhisperLargeV3Turbo };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "groq");
        assert_eq!(v["model"], "whisper_large_v3_turbo");
    }

    #[test]
    fn provider_model_assemblyai_serializes_with_provider_and_model() {
        let m = ProviderModel::AssemblyAi { model: AssemblyAiModel::UniversalProStreaming };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "assembly_ai");
        assert_eq!(v["model"], "universal_pro_streaming");
    }

    #[test]
    fn provider_model_round_trips() {
        let cases = vec![
            ProviderModel::Deepgram,
            ProviderModel::Groq { model: GroqModel::WhisperLargeV3 },
            ProviderModel::Groq { model: GroqModel::WhisperLargeV3Turbo },
            ProviderModel::AssemblyAi { model: AssemblyAiModel::UniversalProStreaming },
            ProviderModel::AssemblyAi { model: AssemblyAiModel::WhisperStreaming },
        ];
        for case in cases {
            let json = serde_json::to_string(&case).unwrap();
            let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, case);
        }
    }

    #[test]
    fn provider_model_default_is_deepgram() {
        assert_eq!(ProviderModel::default(), ProviderModel::Deepgram);
    }

    #[test]
    fn from_legacy_groq_carries_model() {
        let pm = ProviderModel::from_legacy(
            TranscriptionProvider::Groq,
            GroqModel::WhisperLargeV3,
            AssemblyAiModel::default(),
        );
        assert_eq!(pm, ProviderModel::Groq { model: GroqModel::WhisperLargeV3 });
    }

    #[test]
    fn from_legacy_assemblyai_carries_model() {
        let pm = ProviderModel::from_legacy(
            TranscriptionProvider::AssemblyAi,
            GroqModel::default(),
            AssemblyAiModel::UniversalStreamingEnglish,
        );
        assert_eq!(pm, ProviderModel::AssemblyAi { model: AssemblyAiModel::UniversalStreamingEnglish });
    }

    #[test]
    fn assemblyai_model_supports_language_english_only() {
        assert!(AssemblyAiModel::UniversalStreamingEnglish.supports_language("en"));
        assert!(!AssemblyAiModel::UniversalStreamingEnglish.supports_language("uk"));
    }

    #[test]
    fn assemblyai_model_whisper_supports_all_languages() {
        assert!(AssemblyAiModel::WhisperStreaming.supports_language("uk"));
        assert!(AssemblyAiModel::WhisperStreaming.supports_language("zh"));
        assert!(AssemblyAiModel::WhisperStreaming.supported_language_codes().is_none());
    }

    #[test]
    fn groq_model_api_id_correct() {
        assert_eq!(GroqModel::WhisperLargeV3.api_id(), "whisper-large-v3");
        assert_eq!(GroqModel::WhisperLargeV3Turbo.api_id(), "whisper-large-v3-turbo");
    }
}
