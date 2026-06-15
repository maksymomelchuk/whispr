import { useEffect, useState } from "react";

import { PageShell } from "@/components/PageShell";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import { LocalModelCard } from "../components/LocalModelCard";
import { ProviderCard } from "../components/ProviderCard";
import { SectionCard } from "../components/SectionCard";
import { useSettings } from "../context/SettingsContext";
import { getLocalModelStatuses, setLocalWhisperIdleTimeout } from "../lib/api";
import { SPEECH_MODEL_CATALOG } from "../lib/speechModelCatalog";
import type {
  LocalModelStatus,
  LocalWhisperIdleTimeout,
  Settings,
} from "../lib/types";

const CONFIGURED_KEY: Record<string, keyof Settings> = {
  deepgram: "deepgram_api_key_configured",
  groq: "groq_api_key_configured",
  assemblyai: "assemblyai_api_key_configured",
  openai: "openai_api_key_configured",
  elevenlabs: "elevenlabs_api_key_configured",
  soniox: "soniox_api_key_configured",
};

const IDLE_TIMEOUT_OPTIONS: {
  value: LocalWhisperIdleTimeout;
  label: string;
}[] = [
  { value: "five_minutes", label: "5 min" },
  { value: "fifteen_minutes", label: "15 min" },
  { value: "thirty_minutes", label: "30 min" },
  { value: "one_hour", label: "1 hour" },
  { value: "never", label: "Never" },
];

export function SpeechModelsPage() {
  const { settings, setSettings, setSetting } = useSettings();
  const [localStatuses, setLocalStatuses] = useState<LocalModelStatus[]>([]);

  useEffect(() => {
    getLocalModelStatuses().then(setLocalStatuses);
  }, []);

  return (
    <PageShell
      title="Speech models"
      description="How speech is transcribed. Cloud providers need an API key; local models run on-device."
    >
      <SectionCard title="Cloud">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
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
      </SectionCard>

      {localStatuses.length > 0 && (
        <SectionCard title="Local">
          <div className="flex flex-col gap-3">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {localStatuses.map((status) => (
                <LocalModelCard key={status.model} status={status} />
              ))}
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                Idle timeout
              </span>
              <Select
                value={settings.local_whisper_idle_timeout}
                onValueChange={(value) =>
                  setSetting(
                    "local_whisper_idle_timeout",
                    value as LocalWhisperIdleTimeout,
                    () =>
                      setLocalWhisperIdleTimeout(
                        value as LocalWhisperIdleTimeout,
                      ),
                  )
                }
              >
                <SelectTrigger className="w-28 h-8 text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {IDLE_TIMEOUT_OPTIONS.map((opt) => (
                    <SelectItem
                      key={opt.value}
                      value={opt.value}
                      className="text-xs"
                    >
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </SectionCard>
      )}
    </PageShell>
  );
}
