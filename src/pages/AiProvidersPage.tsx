import { zodResolver } from "@hookform/resolvers/zod";
import { CheckFatIcon, GearIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import * as z from "zod";

import { AnthropicLogo } from "@/assets/AnthropicLogo";
import { CerebrasLogo } from "@/assets/CerebrasLogo";
import { CustomLogo } from "@/assets/CustomLogo";
import { DeepSeekLogo } from "@/assets/DeepSeekLogo";
import { GoogleGeminiLogo } from "@/assets/GoogleGeminiLogo";
import { GroqLogo } from "@/assets/GroqLogo";
import { OpenAiLogo } from "@/assets/OpenAiLogo";
import { OpenRouterLogo } from "@/assets/OpenRouterLogo";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";

import { ProviderSetupDialog } from "../components/ProviderSetupDialog";
import { SectionCard } from "../components/SectionCard";
import { useSettings } from "../context/SettingsContext";
import {
  clearCustomProvider,
  setAnthropicApiKey as persistApiKey,
  setCleanupAuthMode as persistAuthMode,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupThresholds as persistThresholds,
  setCustomProvider,
  setProviderKey,
  validateCleanupProviderKey,
} from "../lib/api";
import type { EngineDescriptor } from "../lib/speechModelCatalog";
import { toastRetry } from "../lib/toastRetry";
import type { AiProviderId, Settings } from "../lib/types";

const ANTHROPIC_API_KEY_DESCRIPTOR: EngineDescriptor = {
  id: "anthropic",
  name: "Anthropic",
  logo: AnthropicLogo,
  description: "Claude Haiku for AI-powered transcription cleanup.",
  metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
  keyPlaceholder: "sk-ant-…",
  helpUrl: "https://console.anthropic.com/settings/keys",
  selectConfigured: (s: Settings) => s.ai_cleanup_key_configured,
  persist: persistApiKey,
  validate: (key: string) => validateCleanupProviderKey("anthropic", key),
};

const ANTHROPIC_OAUTH_DESCRIPTOR: EngineDescriptor = {
  id: "anthropic",
  name: "Anthropic",
  logo: AnthropicLogo,
  description: "Claude Haiku for AI-powered transcription cleanup.",
  metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
  keyPlaceholder: "sk-ant-oat…",
  helpUrl: "https://claude.ai/",
  selectConfigured: (s: Settings) => s.ai_cleanup_oauth_token_configured,
  persist: persistOauthToken,
  validate: async () => ({ kind: "valid" as const }),
};

const OPENAI_COMPAT_DESCRIPTORS: EngineDescriptor[] = [
  {
    id: "openai",
    name: "OpenAI",
    logo: OpenAiLogo,
    description: "GPT models for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "sk-…",
    helpUrl: "https://platform.openai.com/api-keys",
    selectConfigured: (s: Settings) =>
      s.configured_providers.includes("openai"),
    persist: (key: string) => setProviderKey("openai", key),
    validate: (key: string) => validateCleanupProviderKey("openai", key),
  },
  {
    id: "google",
    name: "Google Gemini",
    logo: GoogleGeminiLogo,
    description: "Gemini models for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "AIza…",
    helpUrl: "https://aistudio.google.com/apikey",
    selectConfigured: (s: Settings) =>
      s.configured_providers.includes("google"),
    persist: (key: string) => setProviderKey("google", key),
    validate: (key: string) => validateCleanupProviderKey("google", key),
  },
  {
    id: "groq",
    name: "Groq",
    logo: GroqLogo,
    description: "Llama models for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "gsk_…",
    helpUrl: "https://console.groq.com/keys",
    selectConfigured: (s: Settings) => s.configured_providers.includes("groq"),
    persist: (key: string) => setProviderKey("groq", key),
    validate: (key: string) => validateCleanupProviderKey("groq", key),
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    logo: DeepSeekLogo,
    description: "DeepSeek models for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "sk-…",
    helpUrl: "https://platform.deepseek.com/api_keys",
    selectConfigured: (s: Settings) =>
      s.configured_providers.includes("deepseek"),
    persist: (key: string) => setProviderKey("deepseek", key),
    validate: (key: string) => validateCleanupProviderKey("deepseek", key),
  },
  {
    id: "cerebras",
    name: "Cerebras",
    logo: CerebrasLogo,
    description:
      "Llama models on Cerebras hardware for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "csk-…",
    helpUrl: "https://cloud.cerebras.ai/",
    selectConfigured: (s: Settings) =>
      s.configured_providers.includes("cerebras"),
    persist: (key: string) => setProviderKey("cerebras", key),
    validate: (key: string) => validateCleanupProviderKey("cerebras", key),
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    logo: OpenRouterLogo,
    description:
      "Access 200+ models via OpenRouter for AI-powered transcription cleanup.",
    metadata: { languages: "100+ languages", streaming: "—", diarization: "—" },
    keyPlaceholder: "sk-or-…",
    helpUrl: "https://openrouter.ai/keys",
    selectConfigured: (s: Settings) =>
      s.configured_providers.includes("openrouter"),
    persist: (key: string) => setProviderKey("openrouter", key),
    validate: (key: string) => validateCleanupProviderKey("openrouter", key),
  },
];

const thresholdsSchema = z.object({
  minWords: z
    .string()
    .refine(
      (v) => /^\d+$/.test(v.trim()) && Number(v) >= 0,
      "Must be a non-negative integer",
    ),
  minDurationSec: z
    .string()
    .refine(
      (v) => !Number.isNaN(Number(v)) && Number(v) >= 0,
      "Must be a non-negative number",
    ),
});

type ThresholdsValues = z.infer<typeof thresholdsSchema>;

function formatSeconds(ms: number): string {
  const seconds = ms / 1000;
  return Number.isInteger(seconds) ? String(seconds) : seconds.toFixed(2);
}

function ProviderCard({
  descriptor,
  settings,
  onCardClick,
}: {
  descriptor: EngineDescriptor;
  settings: Settings;
  onCardClick: () => void;
}) {
  const isConfigured = descriptor.selectConfigured(settings);
  return (
    <button
      type="button"
      onClick={onCardClick}
      className={cn(
        "flex items-center gap-3 rounded-lg bg-card shadow-xs px-4 py-3",
        "text-left transition-colors hover:bg-accent/40 cursor-pointer w-full",
        "focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
      )}
    >
      <descriptor.logo className="h-8 w-8 shrink-0 rounded-md" />
      <span className="flex-1 min-w-0 truncate text-sm font-medium leading-tight">
        {descriptor.name}
      </span>
      {isConfigured ? (
        <CheckFatIcon
          size={16}
          weight="fill"
          role="img"
          aria-label="Configured"
          className="shrink-0 text-muted-foreground"
        />
      ) : (
        <GearIcon
          size={16}
          role="img"
          aria-label="Set up"
          className="shrink-0 text-muted-foreground/50"
        />
      )}
    </button>
  );
}

const customProviderSchema = z.object({
  baseUrl: z
    .string()
    .min(1, "Base URL is required")
    .refine((v) => {
      try {
        new URL(v.trim().replace(/\/$/, ""));
        return true;
      } catch {
        return false;
      }
    }, "Must be a valid URL (e.g. http://localhost:11434/v1)"),
  model: z.string(),
  apiKey: z.string(),
});

type CustomProviderValues = z.infer<typeof customProviderSchema>;

function CustomProviderDialog({
  open,
  onOpenChange,
  isConfigured,
  currentBaseUrl,
  currentModel,
  onSaved,
  onDisconnected,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isConfigured: boolean;
  currentBaseUrl: string | null;
  currentModel: string;
  onSaved: (baseUrl: string, model: string) => void;
  onDisconnected: () => void;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const form = useForm<CustomProviderValues>({
    resolver: zodResolver(customProviderSchema),
    defaultValues: { baseUrl: "", model: "", apiKey: "" },
  });

  useEffect(() => {
    if (open) {
      form.reset({
        baseUrl: currentBaseUrl ?? "",
        model: currentModel,
        apiKey: "",
      });
      setError(null);
      setSaving(false);
    }
  }, [open, currentBaseUrl, currentModel, form]);

  const handleSave = form.handleSubmit(async (values) => {
    setSaving(true);
    setError(null);
    const baseUrl = values.baseUrl.trim().replace(/\/$/, "");
    const model = values.model.trim();
    try {
      await setCustomProvider(baseUrl, model, values.apiKey.trim());
      onSaved(baseUrl, model);
      onOpenChange(false);
    } catch (e) {
      setError(`Couldn't save: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  });

  const handleDisconnect = async () => {
    setSaving(true);
    setError(null);
    try {
      await clearCustomProvider();
      onDisconnected();
      onOpenChange(false);
    } catch (e) {
      setError(`Couldn't disconnect: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" showCloseButton>
        <DialogHeader>
          <div className="flex items-center gap-3 mb-1">
            <CustomLogo className="h-8 w-8 shrink-0 rounded-md" />
            <DialogTitle className="text-base">Custom</DialogTitle>
          </div>
          <DialogDescription>
            Any OpenAI-compatible /chat/completions endpoint — local servers
            (Ollama, LM Studio, llama.cpp, vLLM) or any custom deployment.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={handleSave} className="flex flex-col gap-3">
            <FormField
              control={form.control}
              name="baseUrl"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs font-medium text-muted-foreground">
                    Base URL
                  </FormLabel>
                  <FormControl>
                    <Input
                      {...field}
                      placeholder="http://localhost:11434/v1"
                      disabled={saving}
                      spellCheck={false}
                      autoComplete="off"
                      aria-label="Base URL"
                    />
                  </FormControl>
                  <FormMessage className="text-xs" />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="model"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs font-medium text-muted-foreground">
                    Model
                  </FormLabel>
                  <FormControl>
                    <Input
                      {...field}
                      placeholder="llama3.2 (leave blank for single-model servers)"
                      disabled={saving}
                      spellCheck={false}
                      autoComplete="off"
                      aria-label="Model"
                    />
                  </FormControl>
                  <p className="text-xs text-muted-foreground">
                    Blank only works on single-model servers (e.g. LM Studio).
                    Ollama requires the exact pulled model name.
                  </p>
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="apiKey"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-xs font-medium text-muted-foreground">
                    API Key (optional)
                  </FormLabel>
                  <FormControl>
                    <Input
                      {...field}
                      type="password"
                      placeholder="Leave blank if not required"
                      disabled={saving}
                      spellCheck={false}
                      autoComplete="off"
                      aria-label="API Key"
                    />
                  </FormControl>
                  <p className="text-xs text-muted-foreground">
                    When blank, no Authorization header is sent. Local servers
                    usually need no key.
                  </p>
                </FormItem>
              )}
            />

            {error && (
              <p className="text-xs text-destructive" role="alert">
                {error}
              </p>
            )}

            <DialogFooter className="flex items-center justify-between sm:justify-between gap-2">
              <div className="flex gap-2">
                <DialogClose asChild>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={saving}
                  >
                    Cancel
                  </Button>
                </DialogClose>
                <Button type="submit" size="sm" disabled={saving}>
                  {saving ? "Saving…" : "Save"}
                </Button>
              </div>
              {isConfigured && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={saving}
                  onClick={handleDisconnect}
                  className="text-muted-foreground hover:text-destructive"
                >
                  Disconnect
                </Button>
              )}
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}

export function AiProvidersPage() {
  const { settings, setSettings, setSetting } = useSettings();
  const {
    ai_cleanup_auth_mode: authMode,
    ai_cleanup_min_words: minWords,
    ai_cleanup_min_duration_ms: minDurationMs,
  } = settings;

  const [openDialog, setOpenDialog] = useState<string | null>(null);

  const anthropicDescriptor =
    authMode === "api_key"
      ? ANTHROPIC_API_KEY_DESCRIPTOR
      : ANTHROPIC_OAUTH_DESCRIPTOR;
  const isAnthropicConfigured = anthropicDescriptor.selectConfigured(settings);

  const handleAuthModeChange = async (val: string) => {
    if (!val || val === authMode) return;
    if (val !== "api_key" && val !== "oauth") return;
    await setSetting(
      "ai_cleanup_auth_mode",
      val,
      () => persistAuthMode(val),
      (e) =>
        toast.error("Couldn't change auth mode", { description: String(e) }),
    );
  };

  const handleAnthropicConfiguredChange = (configured: boolean) => {
    const key =
      authMode === "api_key"
        ? "ai_cleanup_key_configured"
        : "ai_cleanup_oauth_token_configured";
    setSettings((s) => ({ ...s, [key]: configured }));
  };

  const handleProviderKeyConfiguredChange = (
    id: AiProviderId,
    configured: boolean,
  ) => {
    setSettings((s) => ({
      ...s,
      configured_providers: configured
        ? [...s.configured_providers.filter((p) => p !== id), id]
        : s.configured_providers.filter((p) => p !== id),
    }));
  };

  const handleCustomSaved = (baseUrl: string, model: string) => {
    setSettings((s) => ({
      ...s,
      custom_provider_configured: true,
      custom_provider_base_url: baseUrl,
      custom_provider_model: model,
    }));
  };

  const handleCustomDisconnected = () => {
    setSettings((s) => ({
      ...s,
      custom_provider_configured: false,
      custom_provider_base_url: null,
      custom_provider_model: "",
    }));
  };

  const thresholdsForm = useForm<ThresholdsValues>({
    resolver: zodResolver(thresholdsSchema),
    values: {
      minWords: String(minWords),
      minDurationSec: formatSeconds(minDurationMs),
    },
  });

  const lastPersistedRef = useRef({ minWords, minDurationMs });
  useEffect(() => {
    lastPersistedRef.current = { minWords, minDurationMs };
  }, [minWords, minDurationMs]);

  const watched = thresholdsForm.watch();
  useEffect(() => {
    const valid = thresholdsSchema.safeParse(watched);
    if (!valid.success) return;
    const wordsNum = Number(watched.minWords);
    const ms = Math.round(Number(watched.minDurationSec) * 1000);
    if (
      wordsNum === lastPersistedRef.current.minWords &&
      ms === lastPersistedRef.current.minDurationMs
    )
      return;
    const t = setTimeout(async () => {
      try {
        await persistThresholds(wordsNum, ms);
        lastPersistedRef.current = { minWords: wordsNum, minDurationMs: ms };
        setSettings((s) => ({
          ...s,
          ai_cleanup_min_words: wordsNum,
          ai_cleanup_min_duration_ms: ms,
        }));
      } catch (e) {
        toastRetry(
          "Couldn't save thresholds",
          async () => {
            await persistThresholds(wordsNum, ms);
            lastPersistedRef.current = {
              minWords: wordsNum,
              minDurationMs: ms,
            };
            setSettings((s) => ({
              ...s,
              ai_cleanup_min_words: wordsNum,
              ai_cleanup_min_duration_ms: ms,
            }));
          },
          String(e),
        );
      }
    }, 450);
    return () => clearTimeout(t);
  }, [watched.minWords, watched.minDurationSec, setSettings]);

  return (
    <PageShell
      title="Cleanup"
      description="Models that clean up transcriptions. Configure a provider, then enable cleanup per profile."
    >
      <SectionCard title="Provider">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <ProviderCard
            descriptor={anthropicDescriptor}
            settings={settings}
            onCardClick={() => setOpenDialog("anthropic")}
          />
          {OPENAI_COMPAT_DESCRIPTORS.map((descriptor) => (
            <ProviderCard
              key={descriptor.id}
              descriptor={descriptor}
              settings={settings}
              onCardClick={() => setOpenDialog(descriptor.id)}
            />
          ))}
          <button
            type="button"
            onClick={() => setOpenDialog("custom")}
            className={cn(
              "flex items-center gap-3 rounded-lg bg-card shadow-xs px-4 py-3",
              "text-left transition-colors hover:bg-accent/40 cursor-pointer w-full",
            )}
          >
            <CustomLogo className="h-8 w-8 shrink-0 rounded-md" />
            <span className="flex-1 min-w-0 truncate text-sm font-medium leading-tight">
              Custom
            </span>
            {settings.custom_provider_configured ? (
              <CheckFatIcon
                size={16}
                weight="fill"
                role="img"
                aria-label="Configured"
                className="shrink-0 text-muted-foreground"
              />
            ) : (
              <GearIcon
                size={16}
                role="img"
                aria-label="Set up"
                className="shrink-0 text-muted-foreground/50"
              />
            )}
          </button>
        </div>
      </SectionCard>

      <ProviderSetupDialog
        descriptor={anthropicDescriptor}
        isConfigured={isAnthropicConfigured}
        onConfiguredChange={handleAnthropicConfiguredChange}
        open={openDialog === "anthropic"}
        onOpenChange={(open) => setOpenDialog(open ? "anthropic" : null)}
      >
        <div className="flex flex-col gap-[6px]">
          <span className="text-xs font-medium text-muted-foreground">
            Authentication
          </span>
          <ToggleGroup
            type="single"
            variant="outline"
            value={authMode}
            onValueChange={handleAuthModeChange}
            className="w-full"
          >
            <ToggleGroupItem value="api_key" className="flex-1 text-xs">
              Anthropic API key
            </ToggleGroupItem>
            <ToggleGroupItem value="oauth" className="flex-1 text-xs">
              Claude Code OAuth
            </ToggleGroupItem>
          </ToggleGroup>
        </div>
      </ProviderSetupDialog>

      {OPENAI_COMPAT_DESCRIPTORS.map((descriptor) => (
        <ProviderSetupDialog
          key={descriptor.id}
          descriptor={descriptor}
          isConfigured={descriptor.selectConfigured(settings)}
          onConfiguredChange={(configured) =>
            handleProviderKeyConfiguredChange(
              descriptor.id as AiProviderId,
              configured,
            )
          }
          open={openDialog === descriptor.id}
          onOpenChange={(open) => setOpenDialog(open ? descriptor.id : null)}
        />
      ))}

      <CustomProviderDialog
        open={openDialog === "custom"}
        onOpenChange={(open) => setOpenDialog(open ? "custom" : null)}
        isConfigured={settings.custom_provider_configured}
        currentBaseUrl={settings.custom_provider_base_url}
        currentModel={settings.custom_provider_model}
        onSaved={handleCustomSaved}
        onDisconnected={handleCustomDisconnected}
      />

      <SectionCard title="Cleanup Thresholds">
        <div className="flex flex-col gap-3">
          <Form {...thresholdsForm}>
            <form onSubmit={(e) => e.preventDefault()}>
              <div className="flex items-end gap-2">
                <FormField
                  control={thresholdsForm.control}
                  name="minWords"
                  render={({ field }) => (
                    <FormItem className="flex-1">
                      <FormLabel className="text-muted-foreground/70">
                        Min words
                      </FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          type="number"
                          min={0}
                          step={1}
                          inputMode="numeric"
                        />
                      </FormControl>
                      <FormMessage className="mt-1.5 text-help" />
                    </FormItem>
                  )}
                />
                <FormField
                  control={thresholdsForm.control}
                  name="minDurationSec"
                  render={({ field }) => (
                    <FormItem className="flex-1">
                      <FormLabel className="text-muted-foreground/70">
                        Min duration (s)
                      </FormLabel>
                      <FormControl>
                        <Input
                          {...field}
                          type="number"
                          min={0}
                          step={0.5}
                          inputMode="decimal"
                        />
                      </FormControl>
                      <FormMessage className="mt-1.5 text-help" />
                    </FormItem>
                  )}
                />
              </div>
            </form>
          </Form>
          <p className="text-xs text-muted-foreground">
            Cleanup runs only when both thresholds are met. Enable it per
            profile under Profiles; there is no global toggle.
          </p>
        </div>
      </SectionCard>
    </PageShell>
  );
}
