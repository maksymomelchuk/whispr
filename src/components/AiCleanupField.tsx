import { useEffect, useState } from "react";

import {
  setAiCleanupEnabled as persistEnabled,
  setAnthropicApiKey as persistKey,
} from "../lib/api";
import { CollapsibleCard } from "./CollapsibleCard";

interface Props {
  enabled: boolean;
  keyConfigured: boolean;
  onEnabledChange: (enabled: boolean) => void;
  onKeyConfiguredChange: (configured: boolean) => void;
  defaultOpen?: boolean;
}

type SaveStatus = "idle" | "saving" | "saved" | "error";

export function AiCleanupField({
  enabled,
  keyConfigured,
  onEnabledChange,
  onKeyConfiguredChange,
  defaultOpen = false,
}: Props) {
  const [keyValue, setKeyValue] = useState("");
  const [keyStatus, setKeyStatus] = useState<SaveStatus>("idle");
  const [keyError, setKeyError] = useState<string | null>(null);
  const [toggleSaving, setToggleSaving] = useState(false);

  useEffect(() => {
    if (keyStatus !== "saved") return;
    const t = setTimeout(() => setKeyStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [keyStatus]);

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

  const persistAndUpdate = async (value: string) => {
    setKeyStatus("saving");
    setKeyError(null);
    try {
      await persistKey(value);
      setKeyValue("");
      onKeyConfiguredChange(value.length > 0);
      setKeyStatus("saved");
    } catch (e) {
      setKeyStatus("error");
      setKeyError(String(e));
    }
  };

  const handleSaveKey = () => persistAndUpdate(keyValue.trim());
  const handleClearKey = () => persistAndUpdate("");

  const dirty = keyValue.trim().length > 0;
  const showKeyWarning = enabled && !keyConfigured;

  return (
    <CollapsibleCard title="AI Cleanup" defaultOpen={defaultOpen}>
      <p className="hint">
        Optional post-processing that removes filler words and handles spoken
        self-corrections via Anthropic Claude Haiku 4.5. Adds ~500ms after
        release for dictations longer than 3 seconds; short utterances stay
        instant.
      </p>

      <div className="options-list">
        <label className="option-row">
          <input
            type="checkbox"
            checked={enabled}
            disabled={toggleSaving}
            onChange={handleToggle}
          />
          <div className="option-text">
            <div className="option-label">Enable AI post-processing</div>
            <div className="option-description">
              When on, longer dictations are sent to Anthropic for verbatim
              cleanup. The transcript is otherwise preserved as spoken — the
              model never invents corrections.
            </div>
          </div>
        </label>
      </div>

      {enabled && (
        <div className="field-group">
          <label className="field-label">Anthropic API Key</label>
          <p className="hint">
            Required when enabled. Paste your key from{" "}
            <span className="mono">console.anthropic.com</span>.
          </p>
          <p className="hint-sm">
            Status:{" "}
            {keyConfigured ? (
              <span className="status ok">Configured</span>
            ) : (
              <span className="status err">Not set</span>
            )}
          </p>
          <div className="row">
            <input
              type="password"
              value={keyValue}
              onChange={(e) => setKeyValue(e.target.value)}
              placeholder={
                keyConfigured ? "Enter new key to replace…" : "sk-ant-…"
              }
              spellCheck={false}
              autoComplete="off"
            />
            <button
              onClick={handleSaveKey}
              disabled={!dirty || keyStatus === "saving"}
            >
              {keyStatus === "saving" ? "Saving…" : "Save"}
            </button>
            {keyConfigured && (
              <button
                onClick={handleClearKey}
                disabled={keyStatus === "saving"}
                className="secondary"
              >
                Clear
              </button>
            )}
          </div>
          {keyStatus === "saved" && <div className="status ok">Saved</div>}
          {keyStatus === "error" && (
            <div className="status err">{keyError}</div>
          )}
          {showKeyWarning && keyStatus !== "error" && (
            <p className="hint-sm">
              Cleanup is bypassed until a key is set — dictations will paste
              raw.
            </p>
          )}
        </div>
      )}
    </CollapsibleCard>
  );
}
