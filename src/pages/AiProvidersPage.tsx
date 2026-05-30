import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import * as z from "zod";

import { AnthropicLogo } from "@/assets/AnthropicLogo";
import { Badge } from "@/components/ui/badge";
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
import { useSettings } from "../context/SettingsContext";
import {
  setAnthropicApiKey as persistApiKey,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupAuthMode as persistAuthMode,
  setCleanupThresholds as persistThresholds,
} from "../lib/api";
import type { EngineDescriptor } from "../lib/speechModelCatalog";
import type { Settings } from "../lib/types";

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
  // No client-side validation endpoint for Anthropic keys; auth failures surface at cleanup time.
  validate: async () => ({ kind: "valid" as const }),
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
  // No client-side validation endpoint for Anthropic keys; auth failures surface at cleanup time.
  validate: async () => ({ kind: "valid" as const }),
};

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

export function AiProvidersPage() {
  const { settings, setSettings, setSetting } = useSettings();
  const {
    ai_cleanup_auth_mode: authMode,
    ai_cleanup_min_words: minWords,
    ai_cleanup_min_duration_ms: minDurationMs,
  } = settings;
  const [dialogOpen, setDialogOpen] = useState(false);

  const descriptor =
    authMode === "api_key" ? ANTHROPIC_API_KEY_DESCRIPTOR : ANTHROPIC_OAUTH_DESCRIPTOR;
  const isConfigured = descriptor.selectConfigured(settings);

  const handleAuthModeChange = async (val: string) => {
    if (!val || val === authMode) return;
    if (val !== "api_key" && val !== "oauth") return;
    await setSetting(
      "ai_cleanup_auth_mode",
      val,
      () => persistAuthMode(val),
      (e) => toast.error("Couldn't change auth mode", { description: String(e) }),
    );
  };

  const handleConfiguredChange = (configured: boolean) => {
    const key =
      authMode === "api_key" ? "ai_cleanup_key_configured" : "ai_cleanup_oauth_token_configured";
    setSettings((s) => ({ ...s, [key]: configured }));
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
        toast.error("Couldn't save thresholds", { description: String(e) });
      }
    }, 450);
    return () => clearTimeout(t);
  }, [watched.minWords, watched.minDurationSec, setSettings]);

  return (
    <div className="p-6 flex flex-col gap-8">
      <div className="grid grid-cols-2 gap-3">
        <button
          type="button"
          onClick={() => setDialogOpen(true)}
          className={cn(
            "flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3",
            "text-left transition-colors hover:bg-accent/40 cursor-pointer w-full",
          )}
        >
          <AnthropicLogo className="h-8 w-8 shrink-0 rounded-md" />
          <div className="flex flex-1 flex-col gap-0.5 min-w-0">
            <span className="text-sm font-medium leading-tight">Anthropic</span>
            <Badge variant={isConfigured ? "accent" : "neutral"} className="w-fit">
              {isConfigured ? "Configured" : "Setup"}
            </Badge>
          </div>
        </button>
      </div>

      <ProviderSetupDialog
        descriptor={descriptor}
        isConfigured={isConfigured}
        onConfiguredChange={handleConfiguredChange}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
      >
        <div className="flex flex-col gap-[6px]">
          <span className="text-xs font-medium text-muted-foreground">Authentication</span>
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

      <div className="flex flex-col gap-3">
        <h2 className="font-mono text-eyebrow uppercase text-muted-foreground/60 tracking-wide text-xs">
          Cleanup Thresholds
        </h2>
        <Form {...thresholdsForm}>
          <form onSubmit={(e) => e.preventDefault()}>
            <div className="flex items-end gap-2">
              <FormField
                control={thresholdsForm.control}
                name="minWords"
                render={({ field }) => (
                  <FormItem className="flex-1">
                    <FormLabel className="text-muted-foreground/70">Min words</FormLabel>
                    <FormControl>
                      <Input {...field} type="number" min={0} step={1} inputMode="numeric" />
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
                    <FormLabel className="text-muted-foreground/70">Min duration (s)</FormLabel>
                    <FormControl>
                      <Input {...field} type="number" min={0} step={0.5} inputMode="decimal" />
                    </FormControl>
                    <FormMessage className="mt-1.5 text-help" />
                  </FormItem>
                )}
              />
            </div>
          </form>
        </Form>
        <p className="text-xs text-muted-foreground">
          Cleanup runs only when both thresholds are met and is enabled per-Profile in the Profiles
          page. There is no global toggle — enable cleanup per-Profile under Profiles.
        </p>
      </div>
    </div>
  );
}
