import { HistoryTab } from "../components/HistoryTab";
import { useSettings } from "../context/SettingsContext";

export function HistoryPage() {
  const { settings, setSettings } = useSettings();

  if (!settings) return null;

  return (
    <div className="p-6">
      <HistoryTab
        historyLimit={settings.history_limit}
        onHistoryLimitChange={(history_limit) =>
          setSettings((s) => (s ? { ...s, history_limit } : s))
        }
      />
    </div>
  );
}
