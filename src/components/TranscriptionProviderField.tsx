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

import {
  setDeepgramApiKey as persistDeepgramApiKey,
  setGroqApiKey as persistGroqApiKey,
  setGroqSettings as persistGroqSettings,
  setTranscriptionProvider as persistProvider,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "../lib/api";
import type {
  GroqModel,
  GroqSettings,
  TranscriptionProvider,
} from "../lib/types";
import { CredentialField } from "./CredentialField";
import { InfoTip } from "./InfoTip";
import { SectionCard } from "./SectionCard";

const PROVIDER_OPTIONS: { value: TranscriptionProvider; label: string }[] = [
  { value: "deepgram", label: "Deepgram" },
  { value: "groq", label: "Groq" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

interface Props {
  provider: TranscriptionProvider;
  groq: GroqSettings;
  deepgramApiKeyConfigured: boolean;
  groqApiKeyConfigured: boolean;
  onProviderChange: (provider: TranscriptionProvider) => void;
  onGroqSaved: (groq: GroqSettings) => void;
  onDeepgramApiKeyConfiguredChange: (configured: boolean) => void;
  onGroqApiKeyConfiguredChange: (configured: boolean) => void;
}

export function TranscriptionProviderField({
  provider,
  groq,
  deepgramApiKeyConfigured,
  groqApiKeyConfigured,
  onProviderChange,
  onGroqSaved,
  onDeepgramApiKeyConfiguredChange,
  onGroqApiKeyConfiguredChange,
}: Props) {
  const [providerSaveError, setProviderSaveError] = useState<string | null>(
    null,
  );
  const providerForm = useForm<{ provider: TranscriptionProvider }>({
    values: { provider },
  });

  const handleProviderChange = async (next: TranscriptionProvider) => {
    if (next === provider) return;
    const previous = provider;
    onProviderChange(next);
    setProviderSaveError(null);
    try {
      await persistProvider(next);
    } catch (e) {
      onProviderChange(previous);
      setProviderSaveError(String(e));
    }
  };

  const handleGroqModelChange = async (next: GroqModel) => {
    if (next === groq.model) return;
    const previous = groq;
    const updated: GroqSettings = { ...groq, model: next };
    onGroqSaved(updated);
    try {
      await persistGroqSettings(updated);
    } catch (e) {
      onGroqSaved(previous);
      toast.error("Couldn't save Groq model", { description: String(e) });
    }
  };

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
          isConfigured={deepgramApiKeyConfigured}
          persist={persistDeepgramApiKey}
          validate={validateDeepgramApiKey}
          onConfiguredChange={onDeepgramApiKeyConfiguredChange}
        />
      )}

      {provider === "groq" && (
        <>
          <CredentialField
            className="mt-3.5"
            label="API key"
            placeholder="gsk_..."
            isConfigured={groqApiKeyConfigured}
            persist={persistGroqApiKey}
            validate={validateGroqApiKey}
            onConfiguredChange={onGroqApiKeyConfiguredChange}
          />
          <div className="mt-3.5 flex flex-col gap-[6px]">
            <div className="inline-flex items-center gap-2">
              <span className="text-form-label text-muted-foreground">Model</span>
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

      {providerSaveError && (
        <Alert variant="destructive" className="mt-3">
          <AlertDescription>{providerSaveError}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
