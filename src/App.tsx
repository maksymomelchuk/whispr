import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { BrowserRouter } from "react-router-dom";
import { toast } from "sonner";

import { AppShell } from "./components/AppShell";
import { Alert, AlertDescription } from "./components/ui/alert";
import { Toaster } from "./components/ui/sonner";
import { TooltipProvider } from "./components/ui/tooltip";
import { SettingsContext } from "./context/SettingsContext";
import { useAccent } from "./hooks/useAccent";
import { useTheme } from "./hooks/useTheme";
import { getSettings, openTranslationSettings } from "./lib/api";
import type { Settings } from "./lib/types";

import "./globals.css";

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const { preference: themePreference, setPreference: setThemePreference } =
    useTheme();
  const { accent, setAccent } = useAccent();

  useEffect(() => {
    getSettings()
      .then(setSettings)
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
      // The message itself is the only signal we have for which error kind
      // fired; backend wires "System Settings" only into actionable error
      // strings (missing language pack, etc.).
      const actionable = message.includes("System Settings");
      toast.error(message, {
        duration: actionable ? 12000 : 6000,
        action: actionable
          ? {
              label: "Open Settings",
              onClick: () => {
                openTranslationSettings().catch((err) =>
                  console.error("open_translation_settings failed", err),
                );
              },
            }
          : undefined,
      });
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
        themePreference,
        setThemePreference,
        accent,
        setAccent,
      }}
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
