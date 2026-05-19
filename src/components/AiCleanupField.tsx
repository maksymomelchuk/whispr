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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

import { useSettings } from "../context/SettingsContext";
import {
  setAnthropicApiKey as persistApiKey,
  setCleanupAuthMode as persistAuthMode,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupThresholds as persistThresholds,
} from "../lib/api";
import type { CleanupAuthMode } from "../lib/types";
import { CredentialField } from "./CredentialField";
import { InfoTip } from "./InfoTip";
import { SectionHeader } from "./SectionHeader";

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

export function AiCleanupField() {
  const { settings, setSetting, setSettings } = useSettings();
  const {
    ai_cleanup_auth_mode: authMode,
    ai_cleanup_key_configured: apiKeyConfigured,
    ai_cleanup_oauth_token_configured: oauthTokenConfigured,
    ai_cleanup_min_words: minWords,
    ai_cleanup_min_duration_ms: minDurationMs,
  } = settings;

  const handleAuthModeChange = async (val: string) => {
    if (!val || val === authMode) return;
    const mode = val as CleanupAuthMode;
    await setSetting(
      "ai_cleanup_auth_mode",
      mode,
      () => persistAuthMode(mode),
      (e) => toast.error("Couldn't change auth mode", { description: String(e) }),
    );
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
    ) {
      return;
    }
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

  const configured =
    authMode === "api_key" ? apiKeyConfigured : oauthTokenConfigured;
  const copy = MODE_COPY[authMode];

  return (
    <section data-slot="ai-cleanup" className="flex flex-col gap-2.5">
      <SectionHeader
        title="AI Cleanup"
        badge={
          <InfoTip text="Removes filler words and applies spoken self-corrections via Claude Haiku 4.5. Enable per-profile in the Profiles page. Adds ~500ms." />
        }
        className="items-center"
      />

      <div className="flex flex-col gap-3.5 pt-1">
        <div className="flex flex-col gap-[6px]">
          <div className="inline-flex items-center gap-2">
            <span className="text-form-label text-muted-foreground">
              Authentication
            </span>
            <InfoTip text="API key: pay-as-you-go via console.anthropic.com. OAuth: uses your Claude subscription — mint a token with `claude setup-token`." />
          </div>
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

        <CredentialField
          label={copy.fieldLabel}
          placeholder={copy.placeholderEmpty}
          isConfigured={configured}
          persist={authMode === "api_key" ? persistApiKey : persistOauthToken}
          onConfiguredChange={
            authMode === "api_key"
              ? (configured) =>
                  setSettings((s) => ({
                    ...s,
                    ai_cleanup_key_configured: configured,
                  }))
              : (configured) =>
                  setSettings((s) => ({
                    ...s,
                    ai_cleanup_oauth_token_configured: configured,
                  }))
          }
        />
        {!configured && (
          <p className="-mt-1.5 text-help text-muted-foreground/80">
            Set a credential to enable AI cleanup on a profile.
          </p>
        )}

        <div className="flex flex-col gap-[6px]">
          <div className="inline-flex items-center gap-2">
            <span className="text-form-label text-muted-foreground">
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
        </div>
      </div>
    </section>
  );
}
