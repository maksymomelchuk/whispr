import { useId } from "react";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AiProviderId, Settings } from "@/lib/types";

import { FieldColumn } from "./FieldColumn";

export const CLEANUP_PROVIDER_OPTIONS: {
  value: AiProviderId;
  label: string;
}[] = [
  { value: "anthropic", label: "Anthropic" },
  { value: "openai", label: "OpenAI" },
  { value: "google", label: "Google Gemini" },
  { value: "groq", label: "Groq" },
  { value: "deepseek", label: "DeepSeek" },
  { value: "cerebras", label: "Cerebras" },
  { value: "openrouter", label: "OpenRouter" },
  { value: "custom", label: "Custom" },
];

type CleanupModelOption = {
  value: string;
  label: string;
  recommended?: boolean;
};

export const CLEANUP_MODEL_OPTIONS: Record<
  Exclude<AiProviderId, "custom">,
  CleanupModelOption[]
> = {
  anthropic: [
    { value: "claude-haiku-5-5", label: "Claude Haiku 5.5", recommended: true },
    { value: "claude-sonnet-5-5", label: "Claude Sonnet 5.5" },
    { value: "claude-opus-5-5", label: "Claude Opus 5.5" },
    { value: "claude-fable-5-2", label: "Claude Fable 5.2" },
  ],
  openai: [
    { value: "gpt-6-luna", label: "GPT-6 Luna", recommended: true },
    { value: "gpt-5.4-mini", label: "GPT-5.4 mini" },
    { value: "gpt-5.4-nano", label: "GPT-5.4 nano" },
    { value: "gpt-6.2-sol", label: "GPT-6.2 Sol" },
    { value: "gpt-6.2-astra", label: "GPT-6.2 Astra" },
  ],
  google: [
    {
      value: "gemini-3.5-flash-lite",
      label: "Gemini 3.5 Flash-Lite",
      recommended: true,
    },
    { value: "gemini-3.5-flash", label: "Gemini 3.5 Flash" },
    { value: "gemini-3.1-flash-lite", label: "Gemini 3.1 Flash-Lite" },
  ],
  groq: [
    {
      value: "openai/gpt-oss-120b",
      label: "GPT-OSS 120B",
      recommended: true,
    },
    { value: "openai/gpt-oss-20b", label: "GPT-OSS 20B" },
    { value: "qwen/qwen3.8-27b", label: "Qwen3.8 27B" },
  ],
  deepseek: [
    {
      value: "deepseek-flash",
      label: "DeepSeek V4.1 Flash",
      recommended: true,
    },
    { value: "deepseek-v4-pro", label: "DeepSeek V4 Pro" },
  ],
  cerebras: [
    { value: "gpt-oss-120b", label: "GPT-OSS 120B", recommended: true },
    { value: "qwen-3.8-27b", label: "Qwen3.8 27B" },
  ],
  openrouter: [
    {
      value: "anthropic/claude-haiku-5.5",
      label: "Claude Haiku 5.5",
      recommended: true,
    },
    { value: "openai/gpt-6-luna", label: "GPT-6 Luna" },
    { value: "google/gemini-3.5-flash-lite", label: "Gemini 3.5 Flash-Lite" },
    { value: "google/gemini-3.5-flash", label: "Gemini 3.5 Flash" },
    { value: "deepseek/deepseek-v4.1-flash", label: "DeepSeek V4.1 Flash" },
    { value: "qwen/qwen3.8-27b", label: "Qwen3.8 27B" },
  ],
};

export interface AiProviderModel {
  provider: AiProviderId;
  model: string;
}

export function defaultModelFor(provider: AiProviderId): string {
  return provider === "custom" ? "" : CLEANUP_MODEL_OPTIONS[provider][0].value;
}

/** Providers a request can actually use. Anthropic counts only through the
 * credential its active auth mode selects, matching the backend's check. */
export function configuredAiProviders(settings: Settings): AiProviderId[] {
  const anthropicConfigured =
    settings.ai_cleanup_auth_mode === "oauth"
      ? settings.ai_cleanup_oauth_token_configured
      : settings.ai_cleanup_key_configured;
  return [
    ...(anthropicConfigured ? (["anthropic"] as AiProviderId[]) : []),
    ...settings.configured_providers.filter((id) => id !== "anthropic"),
    ...(settings.custom_provider_configured
      ? (["custom"] as AiProviderId[])
      : []),
  ];
}

interface AiProviderModelPickerProps {
  selection: AiProviderModel;
  onChange: (selection: AiProviderModel) => void;
  configuredProviders: AiProviderId[];
  customProviderModel: string;
}

export function AiProviderModelPicker({
  selection,
  onChange,
  configuredProviders,
  customProviderModel,
}: AiProviderModelPickerProps) {
  const providerFieldId = useId();
  const modelFieldId = useId();
  const selectProvider = (provider: AiProviderId) =>
    onChange({ provider, model: defaultModelFor(provider) });

  return (
    <div className="grid grid-cols-2 gap-x-5 gap-y-4">
      <FieldColumn label="Provider" htmlFor={providerFieldId}>
        <Select
          value={selection.provider}
          onValueChange={(v) => selectProvider(v as AiProviderId)}
        >
          <SelectTrigger id={providerFieldId} size="sm" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CLEANUP_PROVIDER_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
                {!configuredProviders.includes(opt.value) && " (needs setup)"}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </FieldColumn>
      {selection.provider !== "custom" ? (
        <FieldColumn label="Model" htmlFor={modelFieldId}>
          <Select
            value={selection.model}
            onValueChange={(model) => onChange({ ...selection, model })}
          >
            <SelectTrigger id={modelFieldId} size="sm" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CLEANUP_MODEL_OPTIONS[selection.provider].map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                  {opt.recommended && (
                    <span className="ml-1.5 text-muted-foreground">
                      (recommended)
                    </span>
                  )}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FieldColumn>
      ) : (
        <FieldColumn label="Model">
          <p className="text-[13px] text-muted-foreground">
            {customProviderModel || "(blank: single-model server)"}
          </p>
          <p className="text-xs text-muted-foreground">
            Configured on the Custom card under Cleanup.
          </p>
        </FieldColumn>
      )}
    </div>
  );
}
