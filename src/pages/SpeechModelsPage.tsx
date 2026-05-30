import { ProviderCard } from "../components/ProviderCard";
import { useSettings } from "../context/SettingsContext";
import { SPEECH_MODEL_CATALOG } from "../lib/speechModelCatalog";
import type { Settings } from "../lib/types";

const CONFIGURED_KEY: Record<string, keyof Settings> = {
  deepgram: "deepgram_api_key_configured",
  groq: "groq_api_key_configured",
  assemblyai: "assemblyai_api_key_configured",
};

export function SpeechModelsPage() {
  const { settings, setSettings } = useSettings();

  return (
    <div className="p-6 flex flex-col gap-8">
      <div className="flex flex-col gap-3">
        <h2 className="font-mono text-eyebrow uppercase text-muted-foreground/60 tracking-wide text-xs">
          Cloud
        </h2>
        <div className="grid grid-cols-2 gap-3">
          {SPEECH_MODEL_CATALOG.map((descriptor) => (
            <ProviderCard
              key={descriptor.id}
              descriptor={descriptor}
              isConfigured={descriptor.selectConfigured(settings)}
              onConfiguredChange={(configured) => {
                const key = CONFIGURED_KEY[descriptor.id];
                if (key) setSettings((s) => ({ ...s, [key]: configured }));
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
