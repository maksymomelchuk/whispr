import { useState } from "react";

import { ShortcutField } from "../components/ShortcutField";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { useSettings } from "../context/SettingsContext";
import { setShortcut as persistShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";

export function ShortcutPage() {
  const { settings, setSettings } = useSettings();
  const [recording, setRecording] = useState(false);

  if (!settings) return null;

  const handleShortcutSave = async (shortcut: Shortcut) => {
    try {
      await persistShortcut(shortcut);
      setSettings((s) => (s ? { ...s, shortcut } : s));
      setRecording(false);
    } catch (e) {
      console.error("Failed to save shortcut", e);
    }
  };

  return (
    <div className="p-6 flex flex-col gap-4">
      <ShortcutField
        shortcut={settings.shortcut}
        onStartRecord={() => setRecording(true)}
      />
      <ShortcutRecorder
        open={recording}
        initial={settings.shortcut}
        onSave={handleShortcutSave}
        onCancel={() => setRecording(false)}
      />
    </div>
  );
}
