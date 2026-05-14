import { useEffect, useState } from "react";

import { InfoTip } from "./InfoTip";

interface Props {
  title: string;
  info: string;
  placeholder?: string;
  isConfigured: boolean;
  persist: (apiKey: string) => Promise<void>;
  onSaved: (configured: boolean) => void;
}

type SaveStatus = "idle" | "saving" | "saved" | "error";

export function ApiKeyField({
  title,
  info,
  placeholder,
  isConfigured,
  persist,
  onSaved,
}: Props) {
  const [value, setValue] = useState("");
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);

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
    } catch (e) {
      setStatus("error");
      setError(String(e));
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
          onChange={(e) => setValue(e.target.value)}
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
      {status === "saved" && <div className="status ok">Saved</div>}
      {status === "error" && <div className="status err">{error}</div>}
    </section>
  );
}
