import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import { useSettings } from "../context/SettingsContext";
import {
  setAssemblyAiApiKey as persistAssemblyAiApiKey,
  setAssemblyAiSettings as persistAssemblyAiSettings,
  setDeepgramApiKey as persistDeepgramApiKey,
  setGroqApiKey as persistGroqApiKey,
  setGroqSettings as persistGroqSettings,
  setTranscriptionProvider as persistProvider,
  validateAssemblyAiApiKey,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "../lib/api";
import type {
  AssemblyAiModel,
  AssemblyAiSettings,
  GroqModel,
  TranscriptionProvider,
} from "../lib/types";
import { ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES } from "../lib/types";
import { CredentialField } from "./CredentialField";
import { InfoTip } from "./InfoTip";
import { SectionCard } from "./SectionCard";

const PROVIDER_OPTIONS: { value: TranscriptionProvider; label: string }[] = [
  { value: "deepgram", label: "Deepgram" },
  { value: "groq", label: "Groq" },
  { value: "assembly_ai", label: "AssemblyAI" },
];

const ASSEMBLYAI_MODEL_OPTIONS: { value: AssemblyAiModel; label: string }[] = [
  { value: "universal_pro_streaming", label: "Universal-3 Pro" },
  { value: "universal_streaming_english", label: "Universal English" },
  { value: "universal_streaming_multilingual", label: "Universal Multilingual" },
  { value: "whisper_streaming", label: "Whisper Streaming" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

export function TranscriptionProviderField() {
  const { settings, setSetting, setSettings } = useSettings();
  const {
    transcription_provider: provider,
    groq,
    assemblyai,
    deepgram_api_key_configured,
    groq_api_key_configured,
    assemblyai_api_key_configured,
  } = settings;
  const [providerSaveError, setProviderSaveError] = useState<string | null>(
    null,
  );

  const providerForm = useForm<{ provider: TranscriptionProvider }>({
    values: { provider },
  });

  const handleProviderChange = async (next: TranscriptionProvider) => {
    if (next === provider) return;
    setProviderSaveError(null);
    await setSetting(
      "transcription_provider",
      next,
      () => persistProvider(next),
      (e) => setProviderSaveError(String(e)),
    );
  };

  const handleGroqModelChange = async (next: GroqModel) => {
    if (next === groq.model) return;
    await setSetting(
      "groq",
      { ...groq, model: next },
      () => persistGroqSettings({ ...groq, model: next }),
      (e) =>
        toast.error("Couldn't save Groq model", { description: String(e) }),
    );
  };

  const handleAssemblyAiModelChange = async (next: AssemblyAiModel) => {
    if (next === assemblyai.model) return;
    const updated: AssemblyAiSettings = { ...assemblyai, model: next };
    await setSetting(
      "assemblyai",
      updated,
      () => persistAssemblyAiSettings(updated),
      (e) =>
        toast.error("Couldn't save AssemblyAI model", { description: String(e) }),
    );
  };

  const assemblyAiSupportedLanguages =
    ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES[assemblyai.model];

  return (
    <SectionCard title="Provider">
      <Form {...providerForm}>
        <FormField
          control={providerForm.control}
          name="provider"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel>Service</FormLabel>
              <FormControl>
                <Select
                  value={field.value}
                  onValueChange={(val) =>
                    handleProviderChange(val as TranscriptionProvider)
                  }
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {PROVIDER_OPTIONS.map((opt) => (
                      <SelectItem key={opt.value} value={opt.value}>
                        {opt.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FormControl>
            </FormItem>
          )}
        />
      </Form>

      {provider === "deepgram" && (
        <CredentialField
          className="mt-3.5"
          label="API key"
          placeholder="dg_..."
          isConfigured={deepgram_api_key_configured}
          persist={persistDeepgramApiKey}
          validate={validateDeepgramApiKey}
          onConfiguredChange={(configured) =>
            setSettings((s) => ({
              ...s,
              deepgram_api_key_configured: configured,
            }))
          }
        />
      )}

      {provider === "groq" && (
        <>
          <CredentialField
            className="mt-3.5"
            label="API key"
            placeholder="gsk_..."
            isConfigured={groq_api_key_configured}
            persist={persistGroqApiKey}
            validate={validateGroqApiKey}
            onConfiguredChange={(configured) =>
              setSettings((s) => ({
                ...s,
                groq_api_key_configured: configured,
              }))
            }
          />
          <div className="mt-3.5 flex flex-col gap-[6px]">
            <div className="inline-flex items-center gap-2">
              <span className="text-form-label text-muted-foreground">
                Model
              </span>
              <InfoTip text="v3-turbo is cheapest and fastest. v3 is slightly more accurate." />
            </div>
            <Select
              value={groq.model}
              onValueChange={(val) => handleGroqModelChange(val as GroqModel)}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GROQ_MODEL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </>
      )}

      {provider === "assembly_ai" && (
        <>
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
          <div className="mt-3.5 flex flex-col gap-[6px]">
            <span className="text-form-label text-muted-foreground">Model</span>
            <Select
              value={assemblyai.model}
              onValueChange={(val) =>
                handleAssemblyAiModelChange(val as AssemblyAiModel)
              }
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ASSEMBLYAI_MODEL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {assemblyAiSupportedLanguages !== null && (
            <Alert className="mt-3">
              <AlertDescription>
                This model only supports:{" "}
                <strong>
                  {assemblyAiSupportedLanguages
                    .map((c) => c.toUpperCase())
                    .join(", ")}
                </strong>
                . Modes set to other languages will fail.
              </AlertDescription>
            </Alert>
          )}
        </>
      )}

      {providerSaveError && (
        <Alert variant="destructive" className="mt-3">
          <AlertDescription>{providerSaveError}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
