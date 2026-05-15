import { useEffect, useState } from "react";

import type { ApiKeyValidation } from "../lib/types";
import { InfoTip } from "./InfoTip";

interface Props {
  title: string;
  info: string;
  placeholder?: string;
  isConfigured: boolean;
  persist: (apiKey: string) => Promise<void>;
  validate?: (apiKey: string) => Promise<ApiKeyValidation>;
  onSaved: (configured: boolean) => void;
}

type SaveStatus = "idle" | "saving" | "saved" | "error";

type ValidationState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "result"; result: ApiKeyValidation };

export function ApiKeyField({
  title,
  info,
  placeholder,
  isConfigured,
  persist,
  validate,
  onSaved,
}: Props) {
  const [value, setValue] = useState("");
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [validation, setValidation] = useState<ValidationState>({
    kind: "idle",
  });

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  const handleSave = async () => {
    setStatus("saving");
    setError(null);
    try {
      const trimmed = value.trim();
      await persist(trimmed);
      setValue("");
      onSaved(trimmed.length > 0);
      setStatus("saved");
      setValidation({ kind: "idle" });
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  };

  const handleClear = async () => {
    setStatus("saving");
    setError(null);
    try {
      await persist("");
      setValue("");
      onSaved(false);
      setStatus("saved");
      setValidation({ kind: "idle" });
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  };

  const handleBlur = async () => {
    if (!validate) return;
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      setValidation({ kind: "idle" });
      return;
    }
    setValidation({ kind: "checking" });
    try {
      const result = await validate(trimmed);
      setValidation({ kind: "result", result });
    } catch (e) {
      setValidation({
        kind: "result",
        result: { kind: "error", message: String(e) },
      });
    }
  };

  const handleChange = (next: string) => {
    setValue(next);
    if (validation.kind !== "idle") {
      setValidation({ kind: "idle" });
    }
  };

  const dirty = value.trim().length > 0;
  const inputPlaceholder = isConfigured
    ? "Enter new key to replace…"
    : (placeholder ?? "");

  return (
    <section className="card">
      <div className="card-title-row">
        <h2 style={{ margin: 0 }}>{title}</h2>
        <InfoTip text={info} />
        {isConfigured ? (
          <span className="status ok">Configured</span>
        ) : (
          <span className="status err">Not set</span>
        )}
      </div>
      <div className="row">
        <input
          type="password"
          value={value}
          onChange={(e) => handleChange(e.target.value)}
          onBlur={handleBlur}
          placeholder={inputPlaceholder}
          spellCheck={false}
          autoComplete="off"
        />
        <button onClick={handleSave} disabled={!dirty || status === "saving"}>
          {status === "saving" ? "Saving…" : "Save"}
        </button>
        {isConfigured && (
          <button
            onClick={handleClear}
            disabled={status === "saving"}
            className="secondary"
          >
            Clear
          </button>
        )}
      </div>
      {renderValidation(validation)}
      {status === "saved" && <div className="status ok">Saved</div>}
      {status === "error" && <div className="status err">{error}</div>}
    </section>
  );
}

function renderValidation(state: ValidationState) {
  switch (state.kind) {
    case "idle":
      return null;
    case "checking":
      return <div className="status">Checking key…</div>;
    case "result":
      switch (state.result.kind) {
        case "valid":
          return <div className="status ok">Key is valid</div>;
        case "invalid":
          return <div className="status err">Invalid API key</div>;
        case "error":
          return (
            <div className="status err">
              Could not validate key: {state.result.message}
            </div>
          );
      }
  }
}
