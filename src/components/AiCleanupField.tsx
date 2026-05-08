import { useEffect, useState } from "react";

import {
  setAiCleanupEnabled as persistEnabled,
  setAnthropicApiKey as persistApiKey,
  setAnthropicOauthToken as persistOauthToken,
  setCleanupAuthMode as persistAuthMode,
  setCleanupThresholds as persistThresholds,
} from "../lib/api";
import type { CleanupAuthMode } from "../lib/types";
import { CollapsibleCard } from "./CollapsibleCard";
import { InfoTip } from "./InfoTip";

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

type SaveStatus = "idle" | "saving" | "saved" | "error";

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
  const [wordsDraft, setWordsDraft] = useState(String(minWords));
  const [secondsDraft, setSecondsDraft] = useState(
    formatSeconds(minDurationMs),
  );
  const [thresholdStatus, setThresholdStatus] = useState<SaveStatus>("idle");
  const [thresholdError, setThresholdError] = useState<string | null>(null);

  useEffect(() => {
    setWordsDraft(String(minWords));
    setSecondsDraft(formatSeconds(minDurationMs));
  }, [minWords, minDurationMs]);

  useEffect(() => {
    if (thresholdStatus !== "saved") return;
    const t = setTimeout(() => setThresholdStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [thresholdStatus]);

  const thresholdsDirty =
    Number(wordsDraft) !== minWords ||
    Math.round(Number(secondsDraft) * 1000) !== minDurationMs;

  const handleSaveThresholds = async () => {
    const wordsNum = Number(wordsDraft);
    const secondsNum = Number(secondsDraft);
    if (
      !Number.isFinite(wordsNum) ||
      wordsNum < 0 ||
      !Number.isInteger(wordsNum)
    ) {
      setThresholdStatus("error");
      setThresholdError("Min words must be a non-negative integer.");
      return;
    }
    if (!Number.isFinite(secondsNum) || secondsNum < 0) {
      setThresholdStatus("error");
      setThresholdError("Min duration must be a non-negative number.");
      return;
    }
    const ms = Math.round(secondsNum * 1000);
    setThresholdStatus("saving");
    setThresholdError(null);
    try {
      await persistThresholds(wordsNum, ms);
      onThresholdsChange(wordsNum, ms);
      setThresholdStatus("saved");
    } catch (e) {
      setThresholdStatus("error");
      setThresholdError(String(e));
    }
  };

  const [draft, setDraft] = useState("");
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [toggleSaving, setToggleSaving] = useState(false);

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  // Switching modes wipes the unsaved draft so we don't accidentally try to
  // save a half-typed API key as an OAuth token (or vice versa).
  useEffect(() => {
    setDraft("");
    setStatus("idle");
    setError(null);
  }, [authMode]);

  const handleToggle = async () => {
    const next = !enabled;
    setToggleSaving(true);
    try {
      await persistEnabled(next);
      onEnabledChange(next);
    } catch (e) {
      console.error("Failed to save AI cleanup toggle", e);
    } finally {
      setToggleSaving(false);
    }
  };

  const handleAuthModeChange = async (mode: CleanupAuthMode) => {
    if (mode === authMode) return;
    try {
      await persistAuthMode(mode);
      onAuthModeChange(mode);
    } catch (e) {
      console.error("Failed to save auth mode", e);
    }
  };

  const persistValue = async (value: string) => {
    setStatus("saving");
    setError(null);
    try {
      if (authMode === "api_key") {
        await persistApiKey(value);
        onApiKeyConfiguredChange(value.length > 0);
      } else {
        await persistOauthToken(value);
        onOauthTokenConfiguredChange(value.length > 0);
      }
      setDraft("");
      setStatus("saved");
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  };

  const handleSave = () => persistValue(draft.trim());
  const handleClear = () => persistValue("");

  const dirty = draft.trim().length > 0;
  const configured =
    authMode === "api_key" ? apiKeyConfigured : oauthTokenConfigured;
  const showWarning = enabled && !configured;
  const copy = MODE_COPY[authMode];
  const placeholder = configured
    ? copy.placeholderReplace
    : copy.placeholderEmpty;

  return (
    <CollapsibleCard title="AI Cleanup" defaultOpen={defaultOpen}>
      <div className="options-list">
        <label className="option-row">
          <input
            type="checkbox"
            checked={enabled}
            disabled={toggleSaving}
            onChange={handleToggle}
          />
          <div className="option-text">
            <div className="option-label label-with-info">
              Enable AI post-processing
              <InfoTip text="Removes filler words and applies spoken self-corrections via Claude Haiku 4.5. Adds ~500ms." />
            </div>
          </div>
        </label>
      </div>

      {enabled && (
        <>
          <div className="field-group">
            <label className="field-label">Authentication</label>
            <div className="options-list">
              <label className="option-row">
                <input
                  type="radio"
                  name="cleanup-auth-mode"
                  value="api_key"
                  checked={authMode === "api_key"}
                  onChange={() => handleAuthModeChange("api_key")}
                />
                <div className="option-text">
                  <div className="option-label label-with-info">
                    Anthropic API Key
                    <InfoTip text="Pay-as-you-go via console.anthropic.com." />
                  </div>
                </div>
              </label>
              <label className="option-row">
                <input
                  type="radio"
                  name="cleanup-auth-mode"
                  value="oauth"
                  checked={authMode === "oauth"}
                  onChange={() => handleAuthModeChange("oauth")}
                />
                <div className="option-text">
                  <div className="option-label label-with-info">
                    Claude Code OAuth token (experimental)
                    <InfoTip text="Uses your Claude subscription. Mint with `claude setup-token`." />
                  </div>
                </div>
              </label>
            </div>
          </div>

          <div className="field-group">
            <div
              className="row"
              style={{ alignItems: "baseline", gap: 8 }}
            >
              <label className="field-label" style={{ margin: 0 }}>
                {copy.fieldLabel}
              </label>
              {configured ? (
                <span className="status ok">Configured</span>
              ) : (
                <span className="status err">Not set</span>
              )}
            </div>
            <div className="row">
              <input
                type="password"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder={placeholder}
                spellCheck={false}
                autoComplete="off"
              />
              <button
                onClick={handleSave}
                disabled={!dirty || status === "saving"}
              >
                {status === "saving" ? "Saving…" : "Save"}
              </button>
              {configured && (
                <button
                  onClick={handleClear}
                  disabled={status === "saving"}
                  className="secondary"
                >
                  Clear
                </button>
              )}
            </div>
            {status === "saved" && <div className="status ok">Saved</div>}
            {status === "error" && <div className="status err">{error}</div>}
            {showWarning && status !== "error" && (
              <p className="hint-sm">
                Cleanup is bypassed until a credential is set.
              </p>
            )}
          </div>

          <div className="field-group">
            <div className="label-with-info" style={{ marginBottom: 0 }}>
              <label className="field-label" style={{ margin: 0 }}>
                Trigger thresholds
              </label>
              <InfoTip text="Both must be met for cleanup to run." />
            </div>
            <div className="row" style={{ alignItems: "flex-end" }}>
              <label
                className="hint-sm"
                style={{
                  minWidth: 120,
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                  textAlign: "left",
                }}
              >
                Min words
                <input
                  type="number"
                  min={0}
                  step={1}
                  value={wordsDraft}
                  onChange={(e) => setWordsDraft(e.target.value)}
                />
              </label>
              <label
                className="hint-sm"
                style={{
                  minWidth: 160,
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                  textAlign: "left",
                }}
              >
                Min duration (s)
                <input
                  type="number"
                  min={0}
                  step={0.5}
                  value={secondsDraft}
                  onChange={(e) => setSecondsDraft(e.target.value)}
                />
              </label>
              <button
                onClick={handleSaveThresholds}
                disabled={!thresholdsDirty || thresholdStatus === "saving"}
              >
                {thresholdStatus === "saving" ? "Saving…" : "Save"}
              </button>
            </div>
            {thresholdStatus === "saved" && (
              <div className="status ok">Saved</div>
            )}
            {thresholdStatus === "error" && (
              <div className="status err">{thresholdError}</div>
            )}
          </div>
        </>
      )}
    </CollapsibleCard>
  );
}
