import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useRef } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import * as z from "zod";

import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";

import { usePersistedToggle } from "../hooks/usePersistedToggle";
import {
  setAnthropicApiKey as persistApiKey,
  setCleanupAuthMode as persistAuthMode,
  setCleanupEnabled as persistEnabled,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupThresholds as persistThresholds,
} from "../lib/api";
import type { CleanupAuthMode } from "../lib/types";
import { CredentialField } from "./CredentialField";
import { InfoTip } from "./InfoTip";

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

interface Props {
  enabled: boolean;
  authMode: CleanupAuthMode;
  apiKeyConfigured: boolean;
  oauthTokenConfigured: boolean;
  minWords: number;
  minDurationMs: number;
  onEnabledChange: (enabled: boolean) => void;
  onAuthModeChange: (mode: CleanupAuthMode) => void;
  onApiKeyConfiguredChange: (configured: boolean) => void;
  onOauthTokenConfiguredChange: (configured: boolean) => void;
  onThresholdsChange: (minWords: number, minDurationMs: number) => void;
}

function formatSeconds(ms: number): string {
  const seconds = ms / 1000;
  return Number.isInteger(seconds) ? String(seconds) : seconds.toFixed(2);
}

const MODE_COPY: Record<
  CleanupAuthMode,
  { fieldLabel: string; placeholderEmpty: string; info: string }
> = {
  api_key: {
    fieldLabel: "Anthropic API key",
    placeholderEmpty: "sk-ant-…",
    info: "Pay-as-you-go via console.anthropic.com.",
  },
  oauth: {
    fieldLabel: "Claude Code OAuth token",
    placeholderEmpty: "sk-ant-oat…",
    info: "Uses your Claude subscription. Mint with `claude setup-token`.",
  },
};

export function AiCleanupField({
  enabled,
  authMode,
  apiKeyConfigured,
  oauthTokenConfigured,
  minWords,
  minDurationMs,
  onEnabledChange,
  onAuthModeChange,
  onApiKeyConfiguredChange,
  onOauthTokenConfiguredChange,
  onThresholdsChange,
}: Props) {
  const enabledToggle = usePersistedToggle(
    enabled,
    persistEnabled,
    onEnabledChange,
  );

  const handleAuthModeChange = async (val: string) => {
    if (!val || val === authMode) return;
    const mode = val as CleanupAuthMode;
    const previous = authMode;
    onAuthModeChange(mode);
    try {
      await persistAuthMode(mode);
    } catch (e) {
      onAuthModeChange(previous);
      toast.error("Couldn't change auth mode", { description: String(e) });
    }
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
    if (!enabledToggle.enabled) return;
    const valid = thresholdsSchema.safeParse(watched);
    if (!valid.success) return;
    const wordsNum = Number(watched.minWords);
    const ms = Math.round(Number(watched.minDurationSec) * 1000);
    if (
      wordsNum === lastPersistedRef.current.minWords &&
      ms === lastPersistedRef.current.minDurationMs
    ) {
      return;
    }
    const t = setTimeout(async () => {
      try {
        await persistThresholds(wordsNum, ms);
        lastPersistedRef.current = { minWords: wordsNum, minDurationMs: ms };
        onThresholdsChange(wordsNum, ms);
      } catch (e) {
        toast.error("Couldn't save thresholds", { description: String(e) });
      }
    }, 450);
    return () => clearTimeout(t);
  }, [
    watched.minWords,
    watched.minDurationSec,
    enabledToggle.enabled,
    onThresholdsChange,
  ]);

  const configured =
    authMode === "api_key" ? apiKeyConfigured : oauthTokenConfigured;
  const showWarning = enabledToggle.enabled && !configured;
  const copy = MODE_COPY[authMode];

  return (
    <section data-slot="ai-cleanup" className="flex flex-col gap-2.5">
      <header className="flex items-center gap-3 pb-1.5 border-b border-border/40">
        <h3 className="text-[14px] font-semibold text-foreground tracking-[-0.005em]">
          AI Cleanup
        </h3>
        <InfoTip text="Removes filler words and applies spoken self-corrections via Claude Haiku 4.5. Adds ~500ms." />
        <div className="ml-auto inline-flex items-center gap-2">
          <span
            className={cn(
              "text-[11px] tracking-[0.2px]",
              enabledToggle.enabled
                ? "text-foreground"
                : "text-muted-foreground/70",
            )}
          >
            {enabledToggle.enabled ? "On" : "Off"}
          </span>
          <Switch
            id="ai-cleanup-enabled"
            aria-label="Enable AI post-processing"
            checked={enabledToggle.enabled}
            onCheckedChange={enabledToggle.toggle}
          />
        </div>
      </header>

      {!enabledToggle.enabled ? (
        <p className="text-[12px] text-muted-foreground/80">
          Off — transcriptions are inserted as Deepgram returns them.
        </p>
      ) : (
        <div className="flex flex-col gap-3.5 pt-1">
          <div className="flex flex-col gap-[6px]">
            <span className="text-[11px] font-medium tracking-[0.2px] text-muted-foreground">
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
                <span className="inline-flex items-center gap-2">
                  Anthropic API key
                  <InfoTip text={MODE_COPY.api_key.info} />
                </span>
              </ToggleGroupItem>
              <ToggleGroupItem value="oauth" className="flex-1 text-xs">
                <span className="inline-flex items-center gap-2">
                  Claude Code OAuth
                  <InfoTip text={MODE_COPY.oauth.info} />
                </span>
              </ToggleGroupItem>
            </ToggleGroup>
          </div>

          <CredentialField
            label={copy.fieldLabel}
            placeholder={copy.placeholderEmpty}
            isConfigured={configured}
            persist={authMode === "api_key" ? persistApiKey : persistOauthToken}
            onConfiguredChange={
              authMode === "api_key"
                ? onApiKeyConfiguredChange
                : onOauthTokenConfiguredChange
            }
          />
          {showWarning && (
            <p className="-mt-1.5 text-[11px] text-muted-foreground/80">
              Cleanup is bypassed until a credential is set.
            </p>
          )}

          <div className="flex flex-col gap-[6px]">
            <div className="inline-flex items-center gap-2">
              <span className="text-[11px] font-medium tracking-[0.2px] text-muted-foreground">
                Trigger thresholds
              </span>
              <InfoTip text="Cleanup runs only when both are met. Lower values clean shorter dictations; higher values save tokens." />
            </div>
            <Form {...thresholdsForm}>
              <form onSubmit={(e) => e.preventDefault()}>
                <div className="flex items-end gap-2">
                  <FormField
                    control={thresholdsForm.control}
                    name="minWords"
                    render={({ field }) => (
                      <FormItem className="flex-1">
                        <FormLabel className="text-[11px] text-muted-foreground/70">
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
                        <FormMessage className="mt-1.5 text-[11px]" />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={thresholdsForm.control}
                    name="minDurationSec"
                    render={({ field }) => (
                      <FormItem className="flex-1">
                        <FormLabel className="text-[11px] text-muted-foreground/70">
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
                        <FormMessage className="mt-1.5 text-[11px]" />
                      </FormItem>
                    )}
                  />
                </div>
              </form>
            </Form>
          </div>
        </div>
      )}
    </section>
  );
}
