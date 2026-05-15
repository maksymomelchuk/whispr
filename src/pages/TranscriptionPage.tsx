import { AiCleanupField } from "../components/AiCleanupField";
import { ReplacementsField } from "../components/ReplacementsField";
import { TranscriptionProviderField } from "../components/TranscriptionProviderField";
import { useSettings } from "../context/SettingsContext";
import type { DeepgramSettings, GroqSettings, Replacement, TranscriptionProvider } from "../lib/types";

export function TranscriptionPage() {
  const { settings, setSettings } = useSettings();

  if (!settings) return null;

  return (
    <div className="p-6 flex flex-col gap-4">
      <TranscriptionProviderField
        provider={settings.transcription_provider}
        deepgram={settings.deepgram}
        groq={settings.groq}
        deepgramApiKeyConfigured={settings.deepgram_api_key_configured}
        groqApiKeyConfigured={settings.groq_api_key_configured}
        onProviderChange={(transcription_provider: TranscriptionProvider) =>
          setSettings((s) => (s ? { ...s, transcription_provider } : s))
        }
        onDeepgramSaved={(deepgram: DeepgramSettings) =>
          setSettings((s) => (s ? { ...s, deepgram } : s))
        }
        onGroqSaved={(groq: GroqSettings) =>
          setSettings((s) => (s ? { ...s, groq } : s))
        }
        onDeepgramApiKeyConfiguredChange={(configured) =>
          setSettings((s) =>
            s ? { ...s, deepgram_api_key_configured: configured } : s,
          )
        }
        onGroqApiKeyConfiguredChange={(configured) =>
          setSettings((s) =>
            s ? { ...s, groq_api_key_configured: configured } : s,
          )
        }
      />
      <AiCleanupField
        enabled={settings.ai_cleanup_enabled}
        authMode={settings.ai_cleanup_auth_mode}
        apiKeyConfigured={settings.ai_cleanup_key_configured}
        oauthTokenConfigured={settings.ai_cleanup_oauth_token_configured}
        onEnabledChange={(ai_cleanup_enabled) =>
          setSettings((s) => (s ? { ...s, ai_cleanup_enabled } : s))
        }
        onAuthModeChange={(ai_cleanup_auth_mode) =>
          setSettings((s) => (s ? { ...s, ai_cleanup_auth_mode } : s))
        }
        onApiKeyConfiguredChange={(ai_cleanup_key_configured) =>
          setSettings((s) => (s ? { ...s, ai_cleanup_key_configured } : s))
        }
        onOauthTokenConfiguredChange={(ai_cleanup_oauth_token_configured) =>
          setSettings((s) =>
            s ? { ...s, ai_cleanup_oauth_token_configured } : s,
          )
        }
        minWords={settings.ai_cleanup_min_words}
        minDurationMs={settings.ai_cleanup_min_duration_ms}
        onThresholdsChange={(ai_cleanup_min_words, ai_cleanup_min_duration_ms) =>
          setSettings((s) =>
            s ? { ...s, ai_cleanup_min_words, ai_cleanup_min_duration_ms } : s,
          )
        }
      />
      <ReplacementsField
        initial={settings.replacements}
        defaultOpen={false}
        onSaved={(replacements: Replacement[]) =>
          setSettings((s) => (s ? { ...s, replacements } : s))
        }
      />
    </div>
  );
}
