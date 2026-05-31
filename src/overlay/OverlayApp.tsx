import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";

import "./OverlayApp.css";

type Mode = "recording" | "thinking" | "error" | "cancelled";

type TargetApp = {
  bundleId: string;
  name: string;
  iconDataUrl?: string;
};

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

function CancelIcon() {
  return (
    <svg
      className="overlay-cancel-icon"
      xmlns="http://www.w3.org/2000/svg"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10" fill="currentColor" />
      <path
        d="M8 8 L16 16 M16 8 L8 16"
        stroke="#0f0f0f"
        strokeWidth="2.2"
        strokeLinecap="round"
      />
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

const BAR_COUNT = 5;
const BAR_MIN_HEIGHT = 4;
const BAR_MAX_HEIGHT = 16;

const Waveform = ({
  levelRef,
}: {
  levelRef: React.RefObject<HTMLDivElement | null>;
}) => (
  <div ref={levelRef} className="overlay-wave" aria-hidden="true">
    {Array.from({ length: BAR_COUNT }).map((_, i) => (
      <span key={i} className="overlay-bar" style={{ height: `var(--bar-${i})` }} />
    ))}
  </div>
);

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
  const [target, setTarget] = useState<TargetApp | null>(null);
  const waveRef = useRef<HTMLDivElement>(null);

  // Imperative write so 30 Hz mic frames don't churn React state.
  // Per-bar sine modulation: each bar's phase is fixed by its index, and the
  // level enters the sine argument so the standing wave shifts across the
  // bars as the level changes — porting AudioWaveformView from typewhisper-mac.
  const applyLevel = (level: number) => {
    const node = waveRef.current;
    if (!node) return;
    const clamped = Math.min(1, Math.max(0, level));
    for (let i = 0; i < BAR_COUNT; i++) {
      const phase = (i / BAR_COUNT) * Math.PI * 2;
      const waveOffset = Math.sin(phase + Math.PI * 0.75 + clamped * 3) * 0.12 + 0.88;
      let barLevel = clamped * waveOffset;
      if (i === 0) barLevel *= 0.85;
      const height = BAR_MIN_HEIGHT + barLevel * (BAR_MAX_HEIGHT - BAR_MIN_HEIGHT);
      node.style.setProperty(`--bar-${i}`, `${height.toFixed(2)}px`);
    }
  };

  useEffect(() => {
    if (ready) return;
    const id = requestAnimationFrame(() => setReady(true));
    return () => cancelAnimationFrame(id);
  }, [ready]);

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

    const resetSession = () => {
      setReady(false);
      setStartedAt(null);
      setElapsedSec(0);
      setMode("recording");
      setPartial("");
      setTarget(null);
      applyLevel(0);
    };
    Promise.all([
      listen("ptt-pressed", () => {
        resetSession();
        setStartedAt(Date.now());
      }),
      listen("overlay-reset", () => {
        resetSession();
      }),
      listen<TargetApp>("target-app", (e) => {
        if (e.payload) setTarget(e.payload);
      }),
      listen<number>("audio-level", (e) => {
        applyLevel(typeof e.payload === "number" ? e.payload : 0);
      }),
      // Release flips to the processing UI immediately. Without this the
      // overlay still shows the recording state through STT drain and any
      // translation step, which looks like recording never stopped.
      listen("ptt-released", () => {
        setMode("thinking");
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
      listen("ptt-cancelled", () => {
        setMode("cancelled");
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
          <div className="overlay-pill-leading">
            <div className="overlay-target-icon-slot">
              {target?.iconDataUrl && (
                <img
                  key={target.bundleId}
                  className="overlay-target-icon"
                  src={target.iconDataUrl}
                  alt=""
                  title={target.name}
                  draggable={false}
                />
              )}
            </div>
            <span className="overlay-timer">
              {mode === "cancelled" ? "Cancelled" : formatElapsed(elapsedSec)}
            </span>
          </div>
          {mode === "recording" && <Waveform levelRef={waveRef} />}
          {mode === "thinking" && <Spinner />}
          {mode === "error" && <ErrorIcon />}
          {mode === "cancelled" && <CancelIcon />}
        </div>
      </div>
    </div>
  );
}
