import { useSettings } from "../context/SettingsContext";
import {
  setAssemblyAiApiKey as persistAssemblyAiApiKey,
  setDeepgramApiKey as persistDeepgramApiKey,
  setGroqApiKey as persistGroqApiKey,
  validateAssemblyAiApiKey,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "../lib/api";
import { CredentialField } from "./CredentialField";
import { SectionCard } from "./SectionCard";

export function ProvidersField() {
  const { settings, setSettings } = useSettings();
  const {
    deepgram_api_key_configured,
    groq_api_key_configured,
    assemblyai_api_key_configured,
  } = settings;

  return (
    <div className="flex flex-col gap-6">
      <SectionCard title="Deepgram">
        <CredentialField
          className="mt-3.5"
          label="API key"
          placeholder="dg_..."
          isConfigured={deepgram_api_key_configured}
          persist={persistDeepgramApiKey}
          validate={validateDeepgramApiKey}
          onConfiguredChange={(configured) =>
            setSettings((s) => ({ ...s, deepgram_api_key_configured: configured }))
          }
        />
      </SectionCard>

      <SectionCard title="Groq">
        <CredentialField
          className="mt-3.5"
          label="API key"
          placeholder="gsk_..."
          isConfigured={groq_api_key_configured}
          persist={persistGroqApiKey}
          validate={validateGroqApiKey}
          onConfiguredChange={(configured) =>
            setSettings((s) => ({ ...s, groq_api_key_configured: configured }))
          }
        />
      </SectionCard>

      <SectionCard title="AssemblyAI">
        <CredentialField
          className="mt-3.5"
          label="API key"
          placeholder="..."
          isConfigured={assemblyai_api_key_configured}
          persist={persistAssemblyAiApiKey}
          validate={validateAssemblyAiApiKey}
          onConfiguredChange={(configured) =>
            setSettings((s) => ({
              ...s,
              assemblyai_api_key_configured: configured,
            }))
          }
        />
      </SectionCard>
    </div>
  );
}
