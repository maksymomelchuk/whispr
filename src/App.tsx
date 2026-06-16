import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { BrowserRouter } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "./components/AppShell";
import { Alert, AlertDescription } from "./components/ui/alert";
import { Toaster } from "./components/ui/sonner";
import { TooltipProvider } from "./components/ui/tooltip";
import { SettingsContext } from "./context/SettingsContext";
import { SystemStatusProvider } from "./context/SystemStatusContext";
import { useAccent } from "./hooks/useAccent";
import { useTheme } from "./hooks/useTheme";
import { getSettings } from "./lib/api";
import type { Settings } from "./lib/types";

import "./globals.css";

function App() {
  const [settings, setRawSettings] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const setSettings = useCallback((updater: (prev: Settings) => Settings) => {
    setRawSettings((prev) => (prev ? updater(prev) : prev));
  }, []);

  const setSetting = useCallback(
    async <K extends keyof Settings>(
      key: K,
      value: Settings[K],
      persist: () => Promise<void>,
      onError?: (err: unknown) => void,
    ) => {
      if (!settings) return;
      const snapshot = settings;
      setSettings(() => ({ ...snapshot, [key]: value }));
      try {
        await persist();
      } catch (e) {
        setSettings(() => snapshot);
        onError?.(e);
      }
    },
    [settings, setSettings],
  );
  const { preference: themePreference, setPreference: setThemePreference } =
    useTheme();
  const { accent, setAccent } = useAccent();

  useEffect(() => {
    getSettings()
      .then(setRawSettings)
      .catch((e) => setLoadError(String(e)));
  }, []);

  useEffect(() => {
    // Race-safe: cleanup may run before listen() resolves (StrictMode
    // double-invoke). A `cancelled` flag tears the listener down as soon as
    // it arrives — without it we leak the first subscription and every
    // transcription-error fires the toast twice.
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen<string>("transcription-error", (e) => {
      const message = e.payload || "Transcription failed";
      toast.error(message, { duration: 6000 });
    })
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      })
      .catch((err) => console.error("listen(transcription-error) failed", err));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  if (loadError) {
    return (
      <main className="mx-auto flex max-w-[480px] flex-col gap-4 p-5">
        <Alert variant="destructive">
          <AlertDescription>
            Failed to load settings: {loadError}
          </AlertDescription>
        </Alert>
      </main>
    );
  }

  if (!settings) {
    return (
      <main className="mx-auto flex max-w-[480px] flex-col gap-4 p-5">
        <div className="py-10 text-center text-muted-foreground">Loading…</div>
      </main>
    );
  }

  return (
    <SettingsContext.Provider
      value={{
        settings,
        setSettings,
        setSetting,
        themePreference,
        setThemePreference,
        accent,
        setAccent,
      }}
    >
      <TooltipProvider>
        <BrowserRouter>
          <SystemStatusProvider>
            <AppShell />
          </SystemStatusProvider>
          <Toaster />
        </BrowserRouter>
      </TooltipProvider>
    </SettingsContext.Provider>
  );
}

export default App;
