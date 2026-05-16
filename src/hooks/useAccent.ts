import { useEffect, useState } from "react";

export type Accent = "indigo" | "violet" | "coral" | "emerald" | "graphite";

export const ACCENTS: readonly Accent[] = [
  "indigo",
  "violet",
  "coral",
  "emerald",
  "graphite",
] as const;

const STORAGE_KEY = "whispr.accent";
const DATA_ATTR = "data-accent";
const DEFAULT_ACCENT: Accent = "indigo";

function readStored(): Accent {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v && (ACCENTS as readonly string[]).includes(v)) return v as Accent;
  } catch {
    /* localStorage may be unavailable in some webview contexts */
  }
  return DEFAULT_ACCENT;
}

function applyAccent(next: Accent) {
  const root = document.documentElement;
  root.classList.add("no-theme-transition");
  root.setAttribute(DATA_ATTR, next);
  void root.offsetHeight;
  requestAnimationFrame(() => {
    root.classList.remove("no-theme-transition");
  });
}

export function useAccent() {
  const [accent, setAccent] = useState<Accent>(readStored);

  useEffect(() => {
    applyAccent(accent);
    try {
      localStorage.setItem(STORAGE_KEY, accent);
    } catch {
      /* ignore */
    }
  }, [accent]);

  return { accent, setAccent };
}
