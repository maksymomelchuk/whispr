import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { ApiKeyField } from "./components/ApiKeyField";
import { HistoryTab } from "./components/HistoryTab";
import { MicrophoneField } from "./components/MicrophoneField";
import { ReplacementsField } from "./components/ReplacementsField";
import { ShortcutField } from "./components/ShortcutField";
import { ShortcutRecorder } from "./components/ShortcutRecorder";
import { TranscriptionField } from "./components/TranscriptionField";
import { getSettings, setShortcut as persistShortcut } from "./lib/api";
import type {
  DeepgramSettings,
  Replacement,
  Settings,
  Shortcut,
} from "./lib/types";

import "./App.css";

type TabId = "general" | "shortcut" | "transcription" | "history";

const TABS: { id: TabId; label: string }[] = [
  { id: "general", label: "General" },
  { id: "shortcut", label: "Shortcut" },
  { id: "transcription", label: "Transcription" },
  { id: "history", label: "History" },
];

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [activeTab, setActiveTab] = useState<TabId>("general");
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e) => setLoadError(String(e)));
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("transcription-error", (e) => {
      setToast(e.payload || "Transcription failed");
    })
      .then((un) => {
        unlisten = un;
      })
      .catch((err) => console.error("listen(transcription-error) failed", err));
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 5000);
    return () => clearTimeout(t);
  }, [toast]);

  const handleShortcutSave = async (shortcut: Shortcut) => {
    try {
      await persistShortcut(shortcut);
      setSettings((s) => (s ? { ...s, shortcut } : s));
      setRecording(false);
    } catch (e) {
      console.error("Failed to save shortcut", e);
    }
  };

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
    <main className="app">
      <nav className="tabs" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`tab ${activeTab === tab.id ? "active" : ""}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className="tab-panel">
        {activeTab === "general" && (
          <>
            <ApiKeyField
              isConfigured={settings.api_key_configured}
              onSaved={(configured) =>
                setSettings((s) =>
                  s ? { ...s, api_key_configured: configured } : s,
                )
              }
            />
            <MicrophoneField
              initial={settings.input_device}
              onSaved={(input_device) =>
                setSettings((s) => (s ? { ...s, input_device } : s))
              }
            />
          </>
        )}
        {activeTab === "shortcut" && (
          <ShortcutField
            shortcut={settings.shortcut}
            onStartRecord={() => setRecording(true)}
          />
        )}
        {activeTab === "transcription" && (
          <>
            <TranscriptionField
              initial={settings.deepgram}
              defaultOpen={false}
              onSaved={(deepgram: DeepgramSettings) =>
                setSettings((s) => (s ? { ...s, deepgram } : s))
              }
            />
            <ReplacementsField
              initial={settings.replacements}
              defaultOpen={false}
              onSaved={(replacements: Replacement[]) =>
                setSettings((s) => (s ? { ...s, replacements } : s))
              }
            />
          </>
        )}
        {activeTab === "history" && <HistoryTab />}
      </div>

      {recording && (
        <ShortcutRecorder
          initial={settings.shortcut}
          onSave={handleShortcutSave}
          onCancel={() => setRecording(false)}
        />
      )}

      {toast && (
        <div className="toast err" role="alert" onClick={() => setToast(null)}>
          {toast}
        </div>
      )}
    </main>
  );
}

export default App;
