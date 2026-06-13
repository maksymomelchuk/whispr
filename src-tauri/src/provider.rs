use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionProvider {
    #[default]
    Deepgram,
    Groq,
    AssemblyAi,
    Local,
    OpenAi,
    ElevenLabs,
    Soniox,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAiTranscribeModel {
    #[default]
    Gpt4oTranscribe,
    Gpt4oMiniTranscribe,
}

impl OpenAiTranscribeModel {
    pub fn api_id(self) -> &'static str {
        match self {
            Self::Gpt4oTranscribe => "gpt-4o-transcribe",
            Self::Gpt4oMiniTranscribe => "gpt-4o-mini-transcribe",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElevenLabsModel {
    #[default]
    ScribeV2,
    ScribeV2Realtime,
}

impl ElevenLabsModel {
    pub fn api_id(self) -> &'static str {
        match self {
            Self::ScribeV2 => "scribe_v2",
            Self::ScribeV2Realtime => "scribe_v2_realtime",
        }
    }
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalWhisperModel {
    LargeV3,
    #[default]
    LargeV3Turbo,
    Parakeet,
}

impl LocalWhisperModel {
    pub fn filename(self) -> &'static str {
        match self {
            Self::LargeV3 => "ggml-large-v3.bin",
            Self::LargeV3Turbo => "ggml-large-v3-turbo.bin",
            Self::Parakeet => "encoder-model.int8.onnx",
        }
    }
}

pub fn local_model_path(data_dir: &Path, model: LocalWhisperModel) -> PathBuf {
    data_dir.join("models").join(model.filename())
}

/// Per-mode provider + model selection. The tag field doubles as the provider
/// identifier so the frontend only needs one discriminant for both.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderModel {
    #[default]
    Deepgram,
    Groq {
        model: GroqModel,
    },
    AssemblyAi {
        model: AssemblyAiModel,
    },
    Local {
        model: LocalWhisperModel,
    },
    OpenAi {
        model: OpenAiTranscribeModel,
    },
    ElevenLabs {
        // Existing configs predate per-model selection and omit this field.
        #[serde(default)]
        model: ElevenLabsModel,
    },
    // Single realtime model (stt-rt-v4), so no model sub-enum. `translate_to`
    // carries the one-way STT-layer translation target; None = verbatim
    // code-switching. The field lives here so it's unrepresentable for any
    // provider that can't honor it.
    Soniox {
        #[serde(default)]
        translate_to: Option<String>,
    },
}

impl ProviderModel {
    pub fn provider(&self) -> TranscriptionProvider {
        match self {
            Self::Deepgram => TranscriptionProvider::Deepgram,
            Self::Groq { .. } => TranscriptionProvider::Groq,
            Self::AssemblyAi { .. } => TranscriptionProvider::AssemblyAi,
            Self::Local { .. } => TranscriptionProvider::Local,
            Self::OpenAi { .. } => TranscriptionProvider::OpenAi,
            Self::ElevenLabs { .. } => TranscriptionProvider::ElevenLabs,
            Self::Soniox { .. } => TranscriptionProvider::Soniox,
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
            TranscriptionProvider::AssemblyAi => Self::AssemblyAi {
                model: assemblyai_model,
            },
            TranscriptionProvider::Local => Self::Local {
                model: LocalWhisperModel::default(),
            },
            TranscriptionProvider::OpenAi => Self::OpenAi {
                model: OpenAiTranscribeModel::default(),
            },
            TranscriptionProvider::ElevenLabs => Self::ElevenLabs {
                model: ElevenLabsModel::default(),
            },
            TranscriptionProvider::Soniox => Self::Soniox { translate_to: None },
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
        let m = ProviderModel::Groq {
            model: GroqModel::WhisperLargeV3Turbo,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "groq");
        assert_eq!(v["model"], "whisper_large_v3_turbo");
    }

    #[test]
    fn provider_model_assemblyai_serializes_with_provider_and_model() {
        let m = ProviderModel::AssemblyAi {
            model: AssemblyAiModel::UniversalProStreaming,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "assembly_ai");
        assert_eq!(v["model"], "universal_pro_streaming");
    }

    #[test]
    fn provider_model_round_trips() {
        let cases = vec![
            ProviderModel::Deepgram,
            ProviderModel::Groq {
                model: GroqModel::WhisperLargeV3,
            },
            ProviderModel::Groq {
                model: GroqModel::WhisperLargeV3Turbo,
            },
            ProviderModel::AssemblyAi {
                model: AssemblyAiModel::UniversalProStreaming,
            },
            ProviderModel::AssemblyAi {
                model: AssemblyAiModel::WhisperStreaming,
            },
            ProviderModel::OpenAi {
                model: OpenAiTranscribeModel::Gpt4oTranscribe,
            },
            ProviderModel::OpenAi {
                model: OpenAiTranscribeModel::Gpt4oMiniTranscribe,
            },
            ProviderModel::ElevenLabs {
                model: ElevenLabsModel::ScribeV2Realtime,
            },
        ];
        for case in cases {
            let json = serde_json::to_string(&case).unwrap();
            let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, case);
        }
    }

    #[test]
    fn provider_model_eleven_labs_serializes_with_provider_and_model() {
        let m = ProviderModel::ElevenLabs {
            model: ElevenLabsModel::ScribeV2Realtime,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "eleven_labs");
        assert_eq!(v["model"], "scribe_v2_realtime");
    }

    #[test]
    fn provider_model_eleven_labs_round_trips() {
        let pm = ProviderModel::ElevenLabs {
            model: ElevenLabsModel::ScribeV2Realtime,
        };
        let json = serde_json::to_string(&pm).unwrap();
        let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, pm);
    }

    #[test]
    fn provider_model_eleven_labs_legacy_json_without_model_defaults_to_scribe_v2() {
        let decoded: ProviderModel = serde_json::from_str(r#"{"provider":"eleven_labs"}"#).unwrap();
        assert_eq!(
            decoded,
            ProviderModel::ElevenLabs {
                model: ElevenLabsModel::ScribeV2,
            }
        );
    }

    #[test]
    fn eleven_labs_model_api_ids() {
        assert_eq!(ElevenLabsModel::ScribeV2.api_id(), "scribe_v2");
        assert_eq!(
            ElevenLabsModel::ScribeV2Realtime.api_id(),
            "scribe_v2_realtime"
        );
    }

    #[test]
    fn provider_model_default_is_deepgram() {
        assert_eq!(ProviderModel::default(), ProviderModel::Deepgram);
    }

    #[test]
    fn provider_model_soniox_verbatim_serializes_without_target() {
        let m = ProviderModel::Soniox { translate_to: None };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "soniox");
        assert!(v["translate_to"].is_null());
    }

    #[test]
    fn provider_model_soniox_translating_serializes_with_target() {
        let m = ProviderModel::Soniox {
            translate_to: Some("en".to_string()),
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "soniox");
        assert_eq!(v["translate_to"], "en");
    }

    #[test]
    fn provider_model_soniox_round_trips() {
        for translate_to in [None, Some("uk".to_string())] {
            let pm = ProviderModel::Soniox { translate_to };
            let json = serde_json::to_string(&pm).unwrap();
            let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, pm);
        }
    }

    #[test]
    fn provider_model_soniox_legacy_json_without_target_defaults_to_none() {
        let decoded: ProviderModel = serde_json::from_str(r#"{"provider":"soniox"}"#).unwrap();
        assert_eq!(decoded, ProviderModel::Soniox { translate_to: None });
    }

    #[test]
    fn provider_model_soniox_reports_soniox_provider() {
        let pm = ProviderModel::Soniox { translate_to: None };
        assert_eq!(pm.provider(), TranscriptionProvider::Soniox);
    }

    #[test]
    fn from_legacy_soniox_has_no_translation_target() {
        let pm = ProviderModel::from_legacy(
            TranscriptionProvider::Soniox,
            GroqModel::default(),
            AssemblyAiModel::default(),
        );
        assert_eq!(pm, ProviderModel::Soniox { translate_to: None });
    }

    #[test]
    fn from_legacy_groq_carries_model() {
        let pm = ProviderModel::from_legacy(
            TranscriptionProvider::Groq,
            GroqModel::WhisperLargeV3,
            AssemblyAiModel::default(),
        );
        assert_eq!(
            pm,
            ProviderModel::Groq {
                model: GroqModel::WhisperLargeV3
            }
        );
    }

    #[test]
    fn from_legacy_assemblyai_carries_model() {
        let pm = ProviderModel::from_legacy(
            TranscriptionProvider::AssemblyAi,
            GroqModel::default(),
            AssemblyAiModel::UniversalStreamingEnglish,
        );
        assert_eq!(
            pm,
            ProviderModel::AssemblyAi {
                model: AssemblyAiModel::UniversalStreamingEnglish
            }
        );
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
        assert!(AssemblyAiModel::WhisperStreaming
            .supported_language_codes()
            .is_none());
    }

    #[test]
    fn groq_model_api_id_correct() {
        assert_eq!(GroqModel::WhisperLargeV3.api_id(), "whisper-large-v3");
        assert_eq!(
            GroqModel::WhisperLargeV3Turbo.api_id(),
            "whisper-large-v3-turbo"
        );
    }

    #[test]
    fn openai_transcribe_model_api_id_correct() {
        assert_eq!(
            OpenAiTranscribeModel::Gpt4oTranscribe.api_id(),
            "gpt-4o-transcribe"
        );
        assert_eq!(
            OpenAiTranscribeModel::Gpt4oMiniTranscribe.api_id(),
            "gpt-4o-mini-transcribe"
        );
    }

    #[test]
    fn openai_transcribe_model_default_is_gpt4o_transcribe() {
        assert_eq!(
            OpenAiTranscribeModel::default(),
            OpenAiTranscribeModel::Gpt4oTranscribe
        );
    }

    #[test]
    fn openai_transcribe_model_serializes_as_snake_case() {
        let v: serde_json::Value =
            serde_json::to_value(OpenAiTranscribeModel::Gpt4oTranscribe).unwrap();
        assert_eq!(v, "gpt4o_transcribe");
        let v: serde_json::Value =
            serde_json::to_value(OpenAiTranscribeModel::Gpt4oMiniTranscribe).unwrap();
        assert_eq!(v, "gpt4o_mini_transcribe");
    }

    #[test]
    fn provider_model_openai_serializes_with_provider_and_model() {
        let m = ProviderModel::OpenAi {
            model: OpenAiTranscribeModel::Gpt4oTranscribe,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "open_ai");
        assert_eq!(v["model"], "gpt4o_transcribe");
    }

    #[test]
    fn provider_model_openai_round_trips() {
        for model in [
            OpenAiTranscribeModel::Gpt4oTranscribe,
            OpenAiTranscribeModel::Gpt4oMiniTranscribe,
        ] {
            let pm = ProviderModel::OpenAi { model };
            let json = serde_json::to_string(&pm).unwrap();
            let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, pm);
        }
    }

    #[test]
    fn local_whisper_model_large_v3_serializes_as_snake_case() {
        let v: serde_json::Value = serde_json::to_value(LocalWhisperModel::LargeV3).unwrap();
        assert_eq!(v, "large_v3");
    }

    #[test]
    fn local_whisper_model_large_v3_turbo_serializes_as_snake_case() {
        let v: serde_json::Value = serde_json::to_value(LocalWhisperModel::LargeV3Turbo).unwrap();
        assert_eq!(v, "large_v3_turbo");
    }

    #[test]
    fn local_whisper_model_round_trips() {
        for model in [
            LocalWhisperModel::LargeV3,
            LocalWhisperModel::LargeV3Turbo,
            LocalWhisperModel::Parakeet,
        ] {
            let json = serde_json::to_string(&model).unwrap();
            let decoded: LocalWhisperModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, model);
        }
    }

    #[test]
    fn local_whisper_model_parakeet_serializes_as_snake_case() {
        let v: serde_json::Value = serde_json::to_value(LocalWhisperModel::Parakeet).unwrap();
        assert_eq!(v, "parakeet");
    }

    #[test]
    fn provider_model_local_parakeet_round_trips() {
        let pm = ProviderModel::Local {
            model: LocalWhisperModel::Parakeet,
        };
        let json = serde_json::to_string(&pm).unwrap();
        let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, pm);
    }

    #[test]
    fn provider_model_local_parakeet_serializes_with_provider_and_model() {
        let m = ProviderModel::Local {
            model: LocalWhisperModel::Parakeet,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "local");
        assert_eq!(v["model"], "parakeet");
    }

    #[test]
    fn local_model_path_parakeet_primary_file_under_models_subdir() {
        let data_dir = Path::new("/app/data");
        let path = local_model_path(data_dir, LocalWhisperModel::Parakeet);
        assert_eq!(
            path,
            PathBuf::from("/app/data/models/encoder-model.int8.onnx")
        );
    }

    #[test]
    fn provider_model_local_serializes_with_provider_and_model() {
        let m = ProviderModel::Local {
            model: LocalWhisperModel::LargeV3,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "local");
        assert_eq!(v["model"], "large_v3");
    }

    #[test]
    fn provider_model_local_turbo_serializes() {
        let m = ProviderModel::Local {
            model: LocalWhisperModel::LargeV3Turbo,
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(v["provider"], "local");
        assert_eq!(v["model"], "large_v3_turbo");
    }

    #[test]
    fn provider_model_local_round_trips() {
        for model in [LocalWhisperModel::LargeV3, LocalWhisperModel::LargeV3Turbo] {
            let pm = ProviderModel::Local { model };
            let json = serde_json::to_string(&pm).unwrap();
            let decoded: ProviderModel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, pm);
        }
    }

    #[test]
    fn local_model_path_large_v3_under_models_subdir() {
        let data_dir = Path::new("/app/data");
        let path = local_model_path(data_dir, LocalWhisperModel::LargeV3);
        assert_eq!(path, PathBuf::from("/app/data/models/ggml-large-v3.bin"));
    }

    #[test]
    fn local_model_path_large_v3_turbo_under_models_subdir() {
        let data_dir = Path::new("/app/data");
        let path = local_model_path(data_dir, LocalWhisperModel::LargeV3Turbo);
        assert_eq!(
            path,
            PathBuf::from("/app/data/models/ggml-large-v3-turbo.bin")
        );
    }
}
