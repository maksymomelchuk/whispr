import { useEffect, useState } from "react";

import {
  listInputDevices,
  setInputDevice as persistInputDevice,
  setPauseMediaOnRecord as persistPauseMediaOnRecord,
} from "../lib/api";

interface Props {
  initial: string | null;
  onSaved: (device: string | null) => void;
  pauseMedia: boolean;
  onPauseMediaSaved: (enabled: boolean) => void;
}

type LoadState = "loading" | "ready" | "error";
type SaveStatus = "idle" | "saving" | "saved" | "error";

const SYSTEM_DEFAULT = "";

export function MicrophoneField({
  initial,
  onSaved,
  pauseMedia,
  onPauseMediaSaved,
}: Props) {
  const [devices, setDevices] = useState<string[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [value, setValue] = useState<string>(initial ?? SYSTEM_DEFAULT);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [pauseEnabled, setPauseEnabled] = useState(pauseMedia);

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
    setPauseEnabled(pauseMedia);
  }, [pauseMedia]);

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

  const togglePauseMedia = async () => {
    const next = !pauseEnabled;
    setPauseEnabled(next);
    setSaveError(null);
    try {
      await persistPauseMediaOnRecord(next);
      onPauseMediaSaved(next);
    } catch (e) {
      setPauseEnabled(!next);
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
      <h2>Audio</h2>

      <div className="field">
        <label className="field-label" htmlFor="mic-device">
          Input device
        </label>
        <select
          id="mic-device"
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

      <label className="toggle-row">
        <span className="toggle-row-label">Pause media while recording</span>
        <input
          type="checkbox"
          role="switch"
          checked={pauseEnabled}
          onChange={togglePauseMedia}
        />
      </label>

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
