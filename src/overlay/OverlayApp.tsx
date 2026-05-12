import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";

import "./OverlayApp.css";

type Mode = "recording" | "thinking" | "error";

const SPINNER_TICKS = 12;
// On overflow we slice to the tail and prepend an ellipsis so the newest
// word stays anchored at the right edge.
const PREVIEW_WINDOW = 120;
// Subsecond cadence keeps the displayed second within ~250ms of real time;
// the setter dedupes so we only rerender on second boundaries.
const TIMER_TICK_MS = 250;

function Spinner() {
  return (
    <svg
      className="overlay-spinner"
      xmlns="http://www.w3.org/2000/svg"
      width="14"
      height="14"
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

function ErrorIcon() {
  return (
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
  );
}

function Waveform() {
  return (
    <div className="overlay-wave" aria-hidden="true">
      <span className="overlay-bar" />
      <span className="overlay-bar" />
      <span className="overlay-bar" />
      <span className="overlay-bar" />
      <span className="overlay-bar" />
    </div>
  );
}

function formatElapsed(seconds: number) {
  const total = Math.max(0, seconds);
  const m = Math.floor(total / 60);
  const ss = (total % 60).toString().padStart(2, "0");
  return `${m}:${ss}`;
}

export function OverlayApp() {
  const [mode, setMode] = useState<Mode>("recording");
  const [partial, setPartial] = useState<string>("");
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [elapsedSec, setElapsedSec] = useState(0);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const id = requestAnimationFrame(() => setReady(true));
    return () => cancelAnimationFrame(id);
  }, []);

  useEffect(() => {
    if (mode !== "recording" || startedAt === null) return;
    const tick = () => {
      const next = Math.floor((Date.now() - startedAt) / 1000);
      setElapsedSec((prev) => (prev === next ? prev : next));
    };
    tick();
    const id = window.setInterval(tick, TIMER_TICK_MS);
    return () => window.clearInterval(id);
  }, [mode, startedAt]);

  useEffect(() => {
    let cancelled = false;
    let unsubs: (() => void)[] = [];
    Promise.all([
      listen("ptt-pressed", () => {
        setStartedAt(Date.now());
        setElapsedSec(0);
        setMode("recording");
        setPartial("");
      }),
      listen("ptt-thinking", () => {
        setMode("thinking");
        setPartial("");
      }),
      listen("ptt-error", () => {
        setMode("error");
        setPartial("");
      }),
      listen<string>("transcript-partial", (e) => {
        setPartial(e.payload ?? "");
      }),
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

  const trimmedPartial = partial.trim();
  const showPreview = mode === "recording" && trimmedPartial.length > 0;
  const displayText = useMemo(() => {
    if (!showPreview) return "";
    if (trimmedPartial.length <= PREVIEW_WINDOW) return trimmedPartial;
    return "…" + trimmedPartial.slice(-PREVIEW_WINDOW);
  }, [showPreview, trimmedPartial]);

  return (
    <div className="overlay-root">
      <div
        className={`overlay-pill ${mode}${showPreview ? " expanded" : ""}${ready ? " ready" : ""}`}
      >
        {showPreview && <div className="overlay-partial">{displayText}</div>}
        <div className="overlay-pill-footer">
          <span className="overlay-timer">{formatElapsed(elapsedSec)}</span>
          {mode === "recording" && <Waveform />}
          {mode === "thinking" && <Spinner />}
          {mode === "error" && <ErrorIcon />}
        </div>
      </div>
    </div>
  );
}
