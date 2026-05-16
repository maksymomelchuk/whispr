import { Microphone, WarningCircle } from "@phosphor-icons/react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";

import { PageHeader } from "../components/PageHeader";
import { useSettings } from "../context/SettingsContext";
import { formatShortcut } from "../lib/api";

interface StatusItem {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  action: { to: string; label: string };
}

export function HomePage() {
  const { settings } = useSettings();

  if (!settings) return null;

  const missingApiKey =
    settings.transcription_provider === "deepgram"
      ? !settings.deepgram_api_key_configured
      : !settings.groq_api_key_configured;
  const missingMic = !settings.input_device;

  const statuses: StatusItem[] = [];
  if (missingApiKey) {
    statuses.push({
      icon: WarningCircle,
      label: `Set up your ${settings.transcription_provider === "deepgram" ? "Deepgram" : "Groq"} API key`,
      action: { to: "/transcription", label: "Open Transcription" },
    });
  }
  if (missingMic) {
    statuses.push({
      icon: Microphone,
      label: "Select a microphone",
      action: { to: "/general", label: "Open General" },
    });
  }

  const shortcuts = settings.hotkey_bindings.map((b) =>
    formatShortcut(b.shortcut),
  );

  return (
    <div className="flex flex-col gap-10 px-10 pt-9 pb-12 max-w-3xl">
      <PageHeader
        eyebrow="Workspace · Home"
        title="Whispr"
        subtitle="Hold a shortcut, speak, release. Transcription lands in the focused app."
      />

      <section className="flex flex-col gap-3">
        <span className="font-mono text-[10.5px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/80">
          Push to talk
        </span>
        {shortcuts.length > 0 ? (
          <div className="flex flex-wrap items-baseline gap-x-7 gap-y-2">
            {shortcuts.map((s, i) => (
              <span
                key={`${s}-${i}`}
                className="font-mono text-[34px] font-semibold leading-none tracking-[0.04em] text-foreground"
              >
                {s}
              </span>
            ))}
          </div>
        ) : (
          <p className="font-mono text-[24px] font-medium leading-none text-muted-foreground/70">
            No hotkeys set
          </p>
        )}
        <p className="text-[13px] text-muted-foreground">
          Hold to dictate · release to transcribe and paste
        </p>
      </section>

      {statuses.length > 0 && (
        <section className="flex flex-col gap-2">
          <span className="font-mono text-[10.5px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/80">
            Needs attention
          </span>
          <ul className="flex flex-col">
            {statuses.map(({ icon: Icon, label, action }, i) => (
              <li
                key={i}
                className="flex items-center gap-3 py-2.5 border-t border-border/60 last:border-b"
              >
                <span
                  aria-hidden
                  className="inline-flex size-1.5 rounded-full bg-[hsl(15_85%_55%)] shrink-0"
                />
                <Icon size={15} className="text-muted-foreground shrink-0" />
                <span className="flex-1 text-[13px] text-foreground">
                  {label}
                </span>
                <Button
                  asChild
                  variant="ghost"
                  size="sm"
                  className="h-7 text-[12px]"
                >
                  <Link to={action.to}>{action.label}</Link>
                </Button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
