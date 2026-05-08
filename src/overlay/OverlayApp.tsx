import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import "./OverlayApp.css";

type Mode = "recording" | "thinking" | "error";

const SPINNER_TICKS = 12;

function Spinner() {
  return (
    <svg
      className="overlay-spinner"
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      {Array.from({ length: SPINNER_TICKS }).map((_, i) => (
        <rect
          key={i}
          x="11"
          y="2"
          width="2"
          height="5"
          rx="1"
          fill="currentColor"
          opacity={0.15 + (i / (SPINNER_TICKS - 1)) * 0.85}
          transform={`rotate(${i * (360 / SPINNER_TICKS)} 12 12)`}
        />
      ))}
    </svg>
  );
}

export function OverlayApp() {
  const [mode, setMode] = useState<Mode>("recording");

  useEffect(() => {
    let cancelled = false;
    let unsubs: (() => void)[] = [];
    Promise.all([
      listen("ptt-pressed", () => setMode("recording")),
      listen("ptt-thinking", () => setMode("thinking")),
      listen("ptt-error", () => setMode("error")),
    ])
      .then((handles) => {
        if (cancelled) handles.forEach((u) => u());
        else unsubs = handles;
      })
      .catch((e) => console.error("overlay listen() failed", e));
    return () => {
      cancelled = true;
      unsubs.forEach((u) => u());
    };
  }, []);

  return (
    <div className="overlay-root">
      <div className={`overlay-pill ${mode}`}>
        <div className="overlay-wave">
          <span className="overlay-bar" />
          <span className="overlay-bar" />
          <span className="overlay-bar" />
          <span className="overlay-bar" />
          <span className="overlay-bar" />
        </div>
        {mode === "thinking" && <Spinner />}
        {mode === "error" && (
          <svg
            className="overlay-error-icon"
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="10" fill="currentColor" />
            <rect x="11" y="6" width="2" height="8" rx="1" fill="#0f0f0f" />
            <rect x="11" y="16" width="2" height="2" rx="1" fill="#0f0f0f" />
          </svg>
        )}
      </div>
    </div>
  );
}
