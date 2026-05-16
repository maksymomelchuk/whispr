import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import * as z from "zod";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
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
  DeepgramSettings,
  GroqModel,
  GroqSettings,
  TranscriptionProvider,
} from "../lib/types";
import { ApiKeyField } from "./ApiKeyField";
import { InfoTip } from "./InfoTip";
import { SectionCard } from "./SectionCard";
import { TranscriptionField } from "./TranscriptionField";

const PROVIDER_OPTIONS: { value: TranscriptionProvider; label: string }[] = [
  { value: "deepgram", label: "Deepgram" },
  { value: "groq", label: "Groq" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

const groqSchema = z.object({
  model: z.enum(["whisper_large_v3_turbo", "whisper_large_v3"]),
  language: z.string().min(1, "Language is required"),
});

type GroqFormValues = z.infer<typeof groqSchema>;
type SaveStatus = "idle" | "saving" | "saved" | "error";

interface Props {
  provider: TranscriptionProvider;
  deepgram: DeepgramSettings;
  groq: GroqSettings;
  deepgramApiKeyConfigured: boolean;
  groqApiKeyConfigured: boolean;
  onProviderChange: (provider: TranscriptionProvider) => void;
  onDeepgramSaved: (deepgram: DeepgramSettings) => void;
  onGroqSaved: (groq: GroqSettings) => void;
  onDeepgramApiKeyConfiguredChange: (configured: boolean) => void;
  onGroqApiKeyConfiguredChange: (configured: boolean) => void;
}

export function TranscriptionProviderField({
  provider,
  deepgram,
  groq,
  deepgramApiKeyConfigured,
  groqApiKeyConfigured,
  onProviderChange,
  onDeepgramSaved,
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

  return (
    <>
      <SectionCard>
        <Form {...providerForm}>
          <FormField
            control={providerForm.control}
            name="provider"
            render={({ field }) => (
              <FormItem className="gap-[6px]">
                <div className="label-with-info">
                  <FormLabel className="field-label" style={{ margin: 0 }}>
                    Transcription provider
                  </FormLabel>
                  <InfoTip text="Provider used for the next dictation. Each provider keeps its own API key and options." />
                </div>
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
        {providerSaveError && (
          <Alert variant="destructive" className="mt-2">
            <AlertDescription>{providerSaveError}</AlertDescription>
          </Alert>
        )}
      </SectionCard>

      {provider === "deepgram" && (
        <>
          <ApiKeyField
            title="Deepgram API Key"
            info="Required to transcribe audio with Deepgram. Paste your key from console.deepgram.com."
            placeholder="dg_..."
            isConfigured={deepgramApiKeyConfigured}
            persist={persistDeepgramApiKey}
            validate={validateDeepgramApiKey}
            onSaved={onDeepgramApiKeyConfiguredChange}
          />
          <TranscriptionField
            initial={deepgram}
            defaultOpen
            onSaved={onDeepgramSaved}
          />
        </>
      )}

      {provider === "groq" && (
        <>
          <ApiKeyField
            title="Groq API Key"
            info="Required to transcribe audio with Groq. Create one at console.groq.com."
            placeholder="gsk_..."
            isConfigured={groqApiKeyConfigured}
            persist={persistGroqApiKey}
            validate={validateGroqApiKey}
            onSaved={onGroqApiKeyConfiguredChange}
          />
          <GroqOptions initial={groq} onSaved={onGroqSaved} />
        </>
      )}
    </>
  );
}

interface GroqOptionsProps {
  initial: GroqSettings;
  onSaved: (groq: GroqSettings) => void;
}

function GroqOptions({ initial, onSaved }: GroqOptionsProps) {
  const form = useForm<GroqFormValues>({
    resolver: zodResolver(groqSchema),
    values: { model: initial.model, language: initial.language },
  });
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  const onSubmit = form.handleSubmit(async (values) => {
    const cleaned: GroqSettings = {
      model: values.model,
      language: values.language.trim() || "en",
    };
    setStatus("saving");
    setError(null);
    try {
      await persistGroqSettings(cleaned);
      onSaved(cleaned);
      setStatus("saved");
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  });

  return (
    <SectionCard
      title="Groq"
      info="Whisper Large via Groq's transcription endpoint."
    >
      <Form {...form}>
        <div className="field-group">
          <FormField
            control={form.control}
            name="model"
            render={({ field }) => (
              <FormItem className="gap-[6px]">
                <div className="label-with-info">
                  <FormLabel className="field-label" style={{ margin: 0 }}>
                    Model
                  </FormLabel>
                  <InfoTip text="v3-turbo is the cheapest and fastest. v3 is slightly more accurate." />
                </div>
                <FormControl>
                  <Select value={field.value} onValueChange={field.onChange}>
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
                </FormControl>
              </FormItem>
            )}
          />
        </div>
        <div className="field-group">
          <FormField
            control={form.control}
            name="language"
            render={({ field }) => (
              <FormItem className="gap-[6px]">
                <div className="label-with-info">
                  <FormLabel className="field-label" style={{ margin: 0 }}>
                    Language
                  </FormLabel>
                  <InfoTip text="ISO-639-1 language code (e.g. en, fr, es). Defaults to en." />
                </div>
                <FormControl>
                  <Input
                    placeholder="en"
                    spellCheck={false}
                    autoComplete="off"
                    {...field}
                  />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
        </div>
        <div className="mt-2 flex items-center justify-end">
          <Button
            onClick={onSubmit}
            disabled={!form.formState.isDirty || status === "saving"}
          >
            {status === "saving" ? "Saving…" : "Save"}
          </Button>
        </div>
      </Form>
      {status === "saved" && (
        <Alert variant="success" className="mt-2">
          <AlertDescription>Saved</AlertDescription>
        </Alert>
      )}
      {status === "error" && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
