//! Benchmark clip definitions and the engines under test.
//!
//! References are the verbatim passages from `benchmark/passages.md`. If a clip
//! was misread during recording, edit the matching reference here so the error
//! rate reflects the model, not the misread.

use crate::mode::ModeLanguage;
use crate::provider::{AssemblyAiModel, GroqModel, OpenAiTranscribeModel};

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    /// WER/CER are meaningful — plain prose.
    Scored,
    /// Numbers/dates/units: WER is dominated by formatting, judge the transcript.
    FormattingEyeball,
    /// Translation target lives downstream in cleanup, not the engine — eyeball.
    TranslationEyeball,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Scored => "scored",
            Role::FormattingEyeball => "formatting (eyeball)",
            Role::TranslationEyeball => "translation (eyeball)",
        }
    }
}

pub struct ClipSpec {
    pub stem: &'static str,
    pub language: fn() -> ModeLanguage,
    pub role: Role,
    pub reference: &'static str,
}

pub fn clips() -> Vec<ClipSpec> {
    vec![
        ClipSpec {
            stem: "a_english",
            language: || ModeLanguage::exact("en"),
            role: Role::Scored,
            reference: "Hey Sarah, I wanted to follow up on yesterday's meeting. We've decided to push the launch to next Thursday, mostly because the design team isn't quite ready. Can you let Marcus know? I'll send over the updated roadmap this afternoon, and we should probably schedule a quick call before the weekend. Thanks so much — talk soon.",
        },
        ClipSpec {
            stem: "b_technical",
            language: || ModeLanguage::exact("en"),
            role: Role::Scored,
            reference: "The Wispr Tauri app routes audio chunks through the Engine trait before handing them to Deepgram over a WebSocket. We refactored the async session loop so the i16 samples are buffered as sixteen kilohertz mono FLAC. ElevenLabs Scribe and the gpt-4o-transcribe model both run as batch POST requests, while Groq polls every three seconds.",
        },
        ClipSpec {
            stem: "c_numbers",
            language: || ModeLanguage::exact("en"),
            role: Role::FormattingEyeball,
            reference: "The invoice came to twelve hundred fifty dollars and fifty cents, due on June fourth, twenty twenty-six. Our current version is one point eight point one. Please email the receipt to billing at serverless dot direct before three forty-five PM, and CC the finance team on the thread.",
        },
        ClipSpec {
            stem: "d_ukrainian",
            language: || ModeLanguage::exact("uk"),
            role: Role::Scored,
            reference: "Привіт! Сьогодні я хочу розповісти про новий застосунок для розпізнавання мовлення. Він працює дуже швидко і підтримує кілька мов одночасно. Минулого тижня ми додали підтримку української, і тепер можна диктувати листи, нотатки та повідомлення майже без помилок.",
        },
        ClipSpec {
            stem: "e_mixed",
            language: || ModeLanguage::Auto,
            role: Role::Scored,
            reference: "Я щойно задеплоїв новий реліз через GitHub Actions, але білд впав на етапі signing. Здається, проблема в Windows сертифікаті. Давай зробимо rollback і перевіримо логи в CI перед тим, як мерджити пул реквест у main.",
        },
        ClipSpec {
            stem: "f_translate",
            language: || ModeLanguage::exact("uk"),
            role: Role::TranslationEyeball,
            reference: "Доброго ранку! Дякую за вашу вчорашню допомогу з налаштуванням сервера. Усе працює ідеально, і команда дуже задоволена результатом.",
        },
    ]
}

#[derive(Clone, Copy)]
pub enum EngineSpec {
    Deepgram,
    Groq(GroqModel),
    AssemblyAi(AssemblyAiModel),
    OpenAi(OpenAiTranscribeModel),
    ElevenLabs,
}

impl EngineSpec {
    pub fn label(self) -> &'static str {
        match self {
            EngineSpec::Deepgram => "Deepgram nova-3",
            EngineSpec::Groq(GroqModel::WhisperLargeV3Turbo) => "Groq whisper-large-v3-turbo",
            EngineSpec::Groq(GroqModel::WhisperLargeV3) => "Groq whisper-large-v3",
            EngineSpec::AssemblyAi(_) => "AssemblyAI universal-streaming",
            EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oTranscribe) => {
                "OpenAI gpt-4o-transcribe"
            }
            EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oMiniTranscribe) => {
                "OpenAI gpt-4o-mini-transcribe"
            }
            EngineSpec::ElevenLabs => "ElevenLabs scribe_v2",
        }
    }

    /// Streaming engines transcribe audio as it arrives over a WebSocket, so
    /// the bench must feed them at real time rather than in one burst.
    pub fn is_streaming(self) -> bool {
        matches!(self, EngineSpec::Deepgram | EngineSpec::AssemblyAi(_))
    }

    pub fn env_var(self) -> &'static str {
        match self {
            EngineSpec::Deepgram => "DEEPGRAM_API_KEY",
            EngineSpec::Groq(_) => "GROQ_API_KEY",
            EngineSpec::AssemblyAi(_) => "ASSEMBLYAI_API_KEY",
            EngineSpec::OpenAi(_) => "OPENAI_API_KEY",
            EngineSpec::ElevenLabs => "ELEVENLABS_API_KEY",
        }
    }

    /// Approximate USD per audio-minute. EDIT THESE — provider pricing changes
    /// often and varies by plan/region. Treated as estimates in the report.
    pub fn usd_per_minute(self) -> f64 {
        match self {
            EngineSpec::Deepgram => 0.0077,
            EngineSpec::Groq(GroqModel::WhisperLargeV3Turbo) => 0.0007,
            EngineSpec::Groq(GroqModel::WhisperLargeV3) => 0.0019,
            EngineSpec::AssemblyAi(_) => 0.0150,
            EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oTranscribe) => 0.0060,
            EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oMiniTranscribe) => 0.0030,
            EngineSpec::ElevenLabs => 0.0067,
        }
    }
}

pub fn engines() -> Vec<EngineSpec> {
    vec![
        EngineSpec::Deepgram,
        EngineSpec::Groq(GroqModel::WhisperLargeV3Turbo),
        EngineSpec::Groq(GroqModel::WhisperLargeV3),
        EngineSpec::AssemblyAi(AssemblyAiModel::UniversalProStreaming),
        EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oTranscribe),
        EngineSpec::OpenAi(OpenAiTranscribeModel::Gpt4oMiniTranscribe),
        EngineSpec::ElevenLabs,
    ]
}
