import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
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
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

import { usePersistedToggle } from "../hooks/usePersistedToggle";
import {
  setAnthropicApiKey as persistApiKey,
  setCleanupAuthMode as persistAuthMode,
  setAiCleanupEnabled as persistEnabled,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupThresholds as persistThresholds,
} from "../lib/api";
import type { CleanupAuthMode } from "../lib/types";
import { CollapsibleCard } from "./CollapsibleCard";
import { InfoTip } from "./InfoTip";

const credentialSchema = z.object({ credential: z.string() });

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

type CredentialValues = z.infer<typeof credentialSchema>;
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
  defaultOpen?: boolean;
}

function formatSeconds(ms: number): string {
  const seconds = ms / 1000;
  return Number.isInteger(seconds) ? String(seconds) : seconds.toFixed(2);
}

const MODE_COPY: Record<
  CleanupAuthMode,
  { fieldLabel: string; placeholderEmpty: string; placeholderReplace: string }
> = {
  api_key: {
    fieldLabel: "Anthropic API Key",
    placeholderEmpty: "sk-ant-…",
    placeholderReplace: "Enter new key to replace…",
  },
  oauth: {
    fieldLabel: "Claude Code OAuth Token",
    placeholderEmpty: "sk-ant-oat…",
    placeholderReplace: "Enter new token to replace…",
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
  defaultOpen = false,
}: Props) {
  const enabledToggle = usePersistedToggle(
    enabled,
    persistEnabled,
    onEnabledChange,
  );

  const handleAuthModeChange = async (val: string) => {
    if (!val || val === authMode) return;
    const mode = val as CleanupAuthMode;
    try {
      await persistAuthMode(mode);
      onAuthModeChange(mode);
    } catch (e) {
      console.error("Failed to save auth mode", e);
    }
  };

  const credentialForm = useForm<CredentialValues>({
    resolver: zodResolver(credentialSchema),
    defaultValues: { credential: "" },
  });

  const [credSaving, setCredSaving] = useState(false);
  const [credSavedOk, setCredSavedOk] = useState(false);

  useEffect(() => {
    if (!credSavedOk) return;
    const t = setTimeout(() => setCredSavedOk(false), 1500);
    return () => clearTimeout(t);
  }, [credSavedOk]);

  // Switching modes wipes the unsaved draft so we don't accidentally try to
  // save a half-typed API key as an OAuth token (or vice versa).
  useEffect(() => {
    credentialForm.reset({ credential: "" });
  }, [authMode, credentialForm]);

  const persistCredential = async (raw: string) => {
    setCredSaving(true);
    credentialForm.clearErrors("credential");
    try {
      if (authMode === "api_key") {
        await persistApiKey(raw);
        onApiKeyConfiguredChange(raw.length > 0);
      } else {
        await persistOauthToken(raw);
        onOauthTokenConfiguredChange(raw.length > 0);
      }
      credentialForm.reset({ credential: "" });
      setCredSavedOk(true);
    } catch (e) {
      credentialForm.setError("credential", { message: String(e) });
    } finally {
      setCredSaving(false);
    }
  };

  const onCredentialSubmit = (values: CredentialValues) =>
    persistCredential(values.credential.trim());
  const handleCredentialClear = () => persistCredential("");

  const thresholdsForm = useForm<ThresholdsValues>({
    resolver: zodResolver(thresholdsSchema),
    values: {
      minWords: String(minWords),
      minDurationSec: formatSeconds(minDurationMs),
    },
  });

  const [threshSaving, setThreshSaving] = useState(false);
  const [threshSavedOk, setThreshSavedOk] = useState(false);

  useEffect(() => {
    if (!threshSavedOk) return;
    const t = setTimeout(() => setThreshSavedOk(false), 1500);
    return () => clearTimeout(t);
  }, [threshSavedOk]);

  const onThresholdsSubmit = async (values: ThresholdsValues) => {
    setThreshSaving(true);
    const wordsNum = Number(values.minWords);
    const ms = Math.round(Number(values.minDurationSec) * 1000);
    try {
      await persistThresholds(wordsNum, ms);
      onThresholdsChange(wordsNum, ms);
      setThreshSavedOk(true);
    } catch (e) {
      thresholdsForm.setError("root", { message: String(e) });
    } finally {
      setThreshSaving(false);
    }
  };

  const configured =
    authMode === "api_key" ? apiKeyConfigured : oauthTokenConfigured;
  const showWarning = enabledToggle.enabled && !configured;
  const copy = MODE_COPY[authMode];
  const placeholder = configured
    ? copy.placeholderReplace
    : copy.placeholderEmpty;
  const credentialValue = credentialForm.watch("credential");
  const credentialDirty = credentialValue.trim().length > 0;

  return (
    <CollapsibleCard title="AI Cleanup" defaultOpen={defaultOpen}>
      <label className="toggle-row">
        <span className="toggle-row-label label-with-info">
          Enable AI post-processing
          <InfoTip text="Removes filler words and applies spoken self-corrections via Claude Haiku 4.5. Adds ~500ms." />
        </span>
        <Switch
          checked={enabledToggle.enabled}
          onCheckedChange={enabledToggle.toggle}
        />
      </label>

      {enabledToggle.enabled && (
        <>
          <div className="field-group">
            <span className="field-label">Authentication</span>
            <ToggleGroup
              type="single"
              variant="outline"
              value={authMode}
              onValueChange={handleAuthModeChange}
              className="w-full"
            >
              <ToggleGroupItem value="api_key" className="flex-1 text-xs">
                <span className="label-with-info">
                  Anthropic API Key
                  <InfoTip text="Pay-as-you-go via console.anthropic.com." />
                </span>
              </ToggleGroupItem>
              <ToggleGroupItem value="oauth" className="flex-1 text-xs">
                <span className="label-with-info">
                  Claude Code OAuth
                  <InfoTip text="Uses your Claude subscription. Mint with `claude setup-token`." />
                </span>
              </ToggleGroupItem>
            </ToggleGroup>
          </div>

          <div className="field-group">
            <div className="row" style={{ alignItems: "baseline", gap: 8 }}>
              <span className="field-label" style={{ margin: 0 }}>
                {copy.fieldLabel}
              </span>
              {configured ? (
                <span className="status ok">Configured</span>
              ) : (
                <span className="status err">Not set</span>
              )}
            </div>
            <Form {...credentialForm}>
              <form onSubmit={credentialForm.handleSubmit(onCredentialSubmit)}>
                <FormField
                  control={credentialForm.control}
                  name="credential"
                  render={({ field }) => (
                    <FormItem>
                      <FormControl>
                        <div className="row">
                          <Input
                            {...field}
                            type="password"
                            placeholder={placeholder}
                            spellCheck={false}
                            autoComplete="off"
                            onChange={(e) => {
                              field.onChange(e);
                              credentialForm.clearErrors("credential");
                            }}
                          />
                          <Button
                            type="submit"
                            disabled={!credentialDirty || credSaving}
                          >
                            {credSaving ? "Saving…" : "Save"}
                          </Button>
                          {configured && (
                            <Button
                              type="button"
                              variant="outline"
                              onClick={handleCredentialClear}
                              disabled={credSaving}
                            >
                              Clear
                            </Button>
                          )}
                        </div>
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </form>
            </Form>
            {credSavedOk && <div className="status ok">Saved</div>}
            {showWarning && !credentialForm.formState.errors.credential && (
              <p className="hint-sm">
                Cleanup is bypassed until a credential is set.
              </p>
            )}
          </div>

          <div className="field-group">
            <div className="label-with-info" style={{ marginBottom: 0 }}>
              <span className="field-label" style={{ margin: 0 }}>
                Trigger thresholds
              </span>
              <InfoTip text="Both must be met for cleanup to run." />
            </div>
            <Form {...thresholdsForm}>
              <form onSubmit={thresholdsForm.handleSubmit(onThresholdsSubmit)}>
                <div className="row" style={{ alignItems: "flex-end" }}>
                  <FormField
                    control={thresholdsForm.control}
                    name="minWords"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="hint-sm">Min words</FormLabel>
                        <FormControl>
                          <Input
                            {...field}
                            type="number"
                            min={0}
                            step={1}
                            style={{ minWidth: 120 }}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={thresholdsForm.control}
                    name="minDurationSec"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="hint-sm">
                          Min duration (s)
                        </FormLabel>
                        <FormControl>
                          <Input
                            {...field}
                            type="number"
                            min={0}
                            step={0.5}
                            style={{ minWidth: 160 }}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <Button
                    type="submit"
                    disabled={!thresholdsForm.formState.isDirty || threshSaving}
                  >
                    {threshSaving ? "Saving…" : "Save"}
                  </Button>
                </div>
                {thresholdsForm.formState.errors.root && (
                  <div className="status err">
                    {thresholdsForm.formState.errors.root.message}
                  </div>
                )}
              </form>
            </Form>
            {threshSavedOk && <div className="status ok">Saved</div>}
          </div>
        </>
      )}
    </CollapsibleCard>
  );
}
