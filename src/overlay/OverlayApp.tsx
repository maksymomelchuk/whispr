import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import "./OverlayApp.css";

type Mode = "recording" | "thinking";

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
      </div>
    </div>
  );
}
