import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { type Control, useForm } from "react-hook-form";
import * as z from "zod";

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
import { TranscriptionField } from "./TranscriptionField";

const groqVariant = z.object({
  provider: z.literal("groq"),
  model: z.enum(["whisper_large_v3_turbo", "whisper_large_v3"]),
  language: z.string().min(1, "Language is required"),
});

const schema = z.discriminatedUnion("provider", [
  z.object({ provider: z.literal("deepgram") }),
  groqVariant,
]);

type FormValues = z.infer<typeof schema>;
type GroqFormValues = z.infer<typeof groqVariant>;

const PROVIDER_OPTIONS: { value: TranscriptionProvider; label: string }[] = [
  { value: "deepgram", label: "Deepgram" },
  { value: "groq", label: "Groq" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

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
  const [groqStatus, setGroqStatus] = useState<SaveStatus>("idle");
  const [groqError, setGroqError] = useState<string | null>(null);

  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    values:
      provider === "groq"
        ? { provider: "groq" as const, model: groq.model, language: groq.language }
        : { provider: "deepgram" as const },
  });

  // Path<FormValues> only includes "provider" (the common key across the discriminated union),
  // so groq-specific fields need a narrowed control type. These FormFields only mount when
  // provider === "groq", so the runtime variant is always the groq branch.
  const groqControl = form.control as unknown as Control<GroqFormValues>;

  useEffect(() => {
    if (groqStatus !== "saved") return;
    const t = setTimeout(() => setGroqStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [groqStatus]);

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

  const handleGroqSave = form.handleSubmit(async (values) => {
    if (values.provider !== "groq") return;
    const cleaned: GroqSettings = {
      model: values.model,
      language: values.language.trim() || "en",
    };
    setGroqStatus("saving");
    setGroqError(null);
    try {
      await persistGroqSettings(cleaned);
      onGroqSaved(cleaned);
      setGroqStatus("saved");
    } catch (e) {
      setGroqStatus("error");
      setGroqError(String(e));
    }
  });

  return (
    <Form {...form}>
      <section className="card">
        <FormField
          control={form.control}
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
        {providerSaveError && (
          <div className="status err">{providerSaveError}</div>
        )}
      </section>

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
          <section className="card">
            <div className="card-title-row">
              <h2 style={{ margin: 0 }}>Groq</h2>
              <InfoTip text="Whisper Large via Groq's transcription endpoint." />
            </div>
            <div className="field-group">
              <FormField
                control={groqControl}
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
                      <Select
                        value={field.value}
                        onValueChange={field.onChange}
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
                    </FormControl>
                  </FormItem>
                )}
              />
            </div>
            <div className="field-group">
              <FormField
                control={groqControl}
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
            <div className="row replacements-actions save-row">
              <div className="spacer" />
              <Button
                onClick={handleGroqSave}
                disabled={!form.formState.isDirty || groqStatus === "saving"}
              >
                {groqStatus === "saving" ? "Saving…" : "Save"}
              </Button>
            </div>
            {groqStatus === "saved" && <div className="status ok">Saved</div>}
            {groqStatus === "error" && (
              <div className="status err">{groqError}</div>
            )}
          </section>
        </>
      )}
    </Form>
  );
}
