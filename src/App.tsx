import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { BrowserRouter } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "./components/AppShell";
import { Toaster } from "./components/ui/sonner";
import { TooltipProvider } from "./components/ui/tooltip";
import { SettingsContext } from "./context/SettingsContext";
import { useTheme } from "./hooks/useTheme";
import { getSettings } from "./lib/api";
import type { Settings } from "./lib/types";

import "./App.css";
import "./globals.css";

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const { preference: themePreference, setPreference: setThemePreference } =
    useTheme();

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e) => setLoadError(String(e)));
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("transcription-error", (e) => {
      toast.error(e.payload || "Transcription failed");
    })
      .then((un) => {
        unlisten = un;
      })
      .catch((err) => console.error("listen(transcription-error) failed", err));
    return () => unlisten?.();
  }, []);

  if (loadError) {
    return (
      <main className="app">
        <div className="card err-card">
          Failed to load settings: {loadError}
        </div>
      </main>
    );
  }

  if (!settings) {
    return (
      <main className="app">
        <div className="loading">Loading…</div>
      </main>
    );
  }

  return (
    <SettingsContext.Provider
      value={{ settings, setSettings, themePreference, setThemePreference }}
    >
      <TooltipProvider>
        <BrowserRouter>
          <AppShell />
          <Toaster />
        </BrowserRouter>
      </TooltipProvider>
    </SettingsContext.Provider>
  );
}

export default App;
