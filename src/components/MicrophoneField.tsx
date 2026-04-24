import { useEffect, useState } from "react";

import {
  listInputDevices,
  setInputDevice as persistInputDevice,
} from "../lib/api";

interface Props {
  initial: string | null;
  onSaved: (device: string | null) => void;
}

type LoadState = "loading" | "ready" | "error";
type SaveStatus = "idle" | "saving" | "saved" | "error";

const SYSTEM_DEFAULT = "";

export function MicrophoneField({ initial, onSaved }: Props) {
  const [devices, setDevices] = useState<string[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [value, setValue] = useState<string>(initial ?? SYSTEM_DEFAULT);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    listInputDevices()
      .then((list) => {
        setDevices(list);
        setLoadState("ready");
      })
      .catch((e) => {
        setLoadState("error");
        setLoadError(String(e));
      });
  }, []);

  useEffect(() => {
    setValue(initial ?? SYSTEM_DEFAULT);
  }, [initial]);

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  const handleChange = async (next: string) => {
    const previous = value;
    setValue(next);
    const payload = next === SYSTEM_DEFAULT ? null : next;
    setStatus("saving");
    setSaveError(null);
    try {
      await persistInputDevice(payload);
      onSaved(payload);
      setStatus("saved");
    } catch (e) {
      setValue(previous);
      setStatus("error");
      setSaveError(String(e));
    }
  };

  const missing =
    loadState === "ready" &&
    initial !== null &&
    initial !== undefined &&
    !devices.includes(initial);

  return (
    <section className="card">
      <h2>Microphone</h2>
      <p className="hint">
        Pick the input device used for recording. System default follows
        whatever macOS currently considers the active input.
      </p>
      <div className="row">
        <select
          className="mic-select"
          value={value}
          disabled={loadState !== "ready" || status === "saving"}
          onChange={(e) => handleChange(e.target.value)}
        >
          <option value={SYSTEM_DEFAULT}>System default</option>
          {devices.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
          {missing && initial ? (
            <option value={initial}>{initial} (unavailable)</option>
          ) : null}
        </select>
      </div>
      {loadState === "loading" && (
        <div className="status">Enumerating devices…</div>
      )}
      {loadState === "error" && <div className="status err">{loadError}</div>}
      {missing && (
        <div className="status err">
          Saved device isn&rsquo;t currently available. Recording will use the
          system default until it&rsquo;s reconnected.
        </div>
      )}
      {status === "saved" && <div className="status ok">Saved</div>}
      {status === "error" && <div className="status err">{saveError}</div>}
    </section>
  );
}
