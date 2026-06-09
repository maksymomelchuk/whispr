import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "whispr.theme";
const DARK_CLASS = "dark";

function readStored(): ThemePreference {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    /* localStorage may be unavailable in some webview contexts */
  }
  return "system";
}

function applyDark(isDark: boolean) {
  const root = document.documentElement;
  // Suppress CSS transitions across the theme flip so colors that change
  // between modes (bg-input, text-muted-foreground, etc.) don't animate
  // their old → new value over ~150ms.
  root.classList.add("no-theme-transition");
  root.classList.toggle(DARK_CLASS, isDark);
  // Force the browser to apply the change before we drop the suppression.
  void root.offsetHeight;
  requestAnimationFrame(() => {
    root.classList.remove("no-theme-transition");
  });
}

function resolveDark(pref: ThemePreference): boolean {
  if (pref === "dark") return true;
  if (pref === "light") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

// The webview CSS class only themes our content; the OS-drawn title bar
// (and its close button) follows the native window theme. A null theme
// hands the window back to the system preference.
function syncWindowTheme(pref: ThemePreference) {
  const theme = pref === "system" ? null : pref;
  getCurrentWindow()
    .setTheme(theme)
    .catch(() => {});
}

export function useTheme() {
  const [preference, setPreference] = useState<ThemePreference>(readStored);

  useEffect(() => {
    applyDark(resolveDark(preference));
    syncWindowTheme(preference);
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      /* ignore */
    }
  }, [preference]);

  useEffect(() => {
    if (preference !== "system") return;
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyDark(mql.matches);
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [preference]);

  return { preference, setPreference };
}
