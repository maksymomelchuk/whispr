import { useSettings } from "../context/SettingsContext";
import type { Mode, ModeLanguage } from "../lib/types";
import { SectionCard } from "../components/SectionCard";

function formatLanguage(lang: ModeLanguage): string {
  if (lang.kind === "auto") return "Auto";
  return lang.code;
}

function ModeCard({ mode }: { mode: Mode }) {
  const langLabel = formatLanguage(mode.language);
  const cleanupLabel = mode.ai_cleanup.enabled ? "Cleanup on" : "No cleanup";
  const summary = [langLabel, cleanupLabel].join(" · ");

  return (
    <SectionCard>
      <div className="flex flex-col gap-1">
        <span className="text-sm font-semibold text-foreground">
          {mode.name}
        </span>
        <span className="text-xs text-muted-foreground">{summary}</span>
      </div>
    </SectionCard>
  );
}

export function ModesPage() {
  const { settings } = useSettings();

  if (!settings) return null;

  const defaultMode =
    settings.modes.find((m) => m.id === settings.default_mode_id) ??
    settings.modes[0];

  return (
    <div className="p-6 flex flex-col gap-4">
      {defaultMode && <ModeCard mode={defaultMode} />}
    </div>
  );
}
