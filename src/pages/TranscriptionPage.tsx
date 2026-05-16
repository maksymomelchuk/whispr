import { AiCleanupField } from "../components/AiCleanupField";
import { DictionaryField } from "../components/DictionaryField";
import { TranscriptionProviderField } from "../components/TranscriptionProviderField";
import { useSettings } from "../context/SettingsContext";
import type {
  DictionaryEntry,
  GroqSettings,
  TranscriptionProvider,
} from "../lib/types";

export function TranscriptionPage() {
  const { settings, setSettings } = useSettings();

  if (!settings) return null;

  const defaultMode = settings.modes.find(
    (m) => m.id === settings.default_mode_id,
  ) ?? settings.modes[0];

  return (
    <div className="p-6 flex flex-col gap-6">
      <TranscriptionProviderField
        provider={settings.transcription_provider}
        groq={settings.groq}
        deepgramApiKeyConfigured={settings.deepgram_api_key_configured}
        groqApiKeyConfigured={settings.groq_api_key_configured}
        onProviderChange={(transcription_provider: TranscriptionProvider) =>
          setSettings((s) => (s ? { ...s, transcription_provider } : s))
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
        enabled={defaultMode?.ai_cleanup.enabled ?? false}
        authMode={settings.ai_cleanup_auth_mode}
        apiKeyConfigured={settings.ai_cleanup_key_configured}
        oauthTokenConfigured={settings.ai_cleanup_oauth_token_configured}
        onEnabledChange={(enabled) =>
          setSettings((s) => {
            if (!s) return s;
            const modes = s.modes.map((m) =>
              m.id === s.default_mode_id
                ? { ...m, ai_cleanup: { ...m.ai_cleanup, enabled } }
                : m,
            );
            return { ...s, modes };
          })
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
        onThresholdsChange={(
          ai_cleanup_min_words,
          ai_cleanup_min_duration_ms,
        ) =>
          setSettings((s) =>
            s ? { ...s, ai_cleanup_min_words, ai_cleanup_min_duration_ms } : s,
          )
        }
      />
      <DictionaryField
        initial={settings.dictionary}
        defaultOpen={false}
        onSaved={(dictionary: DictionaryEntry[]) =>
          setSettings((s) => (s ? { ...s, dictionary } : s))
        }
      />
    </div>
  );
}
