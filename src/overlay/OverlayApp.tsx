import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import "./OverlayApp.css";

type Mode = "recording" | "thinking";

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
    const unsubs: (() => void)[] = [];
    let cancelled = false;
    listen("ptt-pressed", () => setMode("recording"))
      .then((u) => (cancelled ? u() : unsubs.push(u)))
      .catch((e) => console.error("listen(ptt-pressed) failed", e));
    listen("ptt-thinking", () => setMode("thinking"))
      .then((u) => (cancelled ? u() : unsubs.push(u)))
      .catch((e) => console.error("listen(ptt-thinking) failed", e));
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
      </div>
    </div>
  );
}
