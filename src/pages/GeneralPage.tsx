import { AppearanceField } from "../components/AppearanceField";
import { MicrophoneField } from "../components/MicrophoneField";
import { useSettings } from "../context/SettingsContext";

export function GeneralPage() {
  const {
    settings,
    setSettings,
    themePreference,
    setThemePreference,
    accent,
    setAccent,
  } = useSettings();

  if (!settings) return null;

  return (
    <div className="p-6 flex flex-col gap-8">
      <MicrophoneField
        initial={settings.input_device}
        onSaved={(input_device) =>
          setSettings((s) => (s ? { ...s, input_device } : s))
        }
        pauseMedia={settings.pause_media_on_record}
        onPauseMediaSaved={(pause_media_on_record) =>
          setSettings((s) => (s ? { ...s, pause_media_on_record } : s))
        }
      />
      <AppearanceField
        preference={themePreference}
        onChange={setThemePreference}
        accent={accent}
        onAccentChange={setAccent}
        showInDock={settings.show_in_dock}
        onShowInDockChange={(show_in_dock) =>
          setSettings((s) => (s ? { ...s, show_in_dock } : s))
        }
        showLivePreview={settings.show_live_preview}
        onShowLivePreviewChange={(show_live_preview) =>
          setSettings((s) => (s ? { ...s, show_live_preview } : s))
        }
      />
    </div>
  );
}
