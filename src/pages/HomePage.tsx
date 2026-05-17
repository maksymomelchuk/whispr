import { Keyboard, Microphone } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { ShortcutKeycaps } from "@/components/Keycap";

import { PageHeader } from "../components/PageHeader";
import { useSettings } from "../context/SettingsContext";
import {
  type PermissionsStatus,
  checkPermissions,
  openAccessibilitySettings,
  openMicrophoneSettings,
} from "../lib/api";

export function HomePage() {
  const { settings } = useSettings();
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(
    null,
  );

  useEffect(() => {
    checkPermissions().then(setPermissions).catch(() => {});
  }, []);

  if (!settings) return null;

  const defaultBindings = settings.hotkey_bindings.filter(
    (b) => b.mode_id === settings.default_mode_id,
  );

  return (
    <div className="flex flex-col gap-10 px-10 pt-9 pb-12 max-w-3xl">
      <PageHeader
        eyebrow="Workspace · Home"
        title="Whispr"
        subtitle="Hold a shortcut, speak, release. Transcription lands in the focused app."
      />

      <section className="flex flex-col gap-3">
        <span className="font-mono text-eyebrow uppercase text-muted-foreground/80">
          Push to talk
        </span>
        {defaultBindings.length > 0 ? (
          <div className="flex flex-wrap items-center gap-x-5 gap-y-2.5">
            {defaultBindings.map((b, i) => (
              <ShortcutKeycaps key={i} shortcut={b.shortcut} size="lg" />
            ))}
          </div>
        ) : (
          <p className="text-[14px] text-muted-foreground/70">
            No hotkeys set ·{" "}
            <Link to="/hotkeys" className="underline underline-offset-4">
              Open Hotkeys
            </Link>
          </p>
        )}
        <p className="text-[13px] text-muted-foreground">
          Hold to dictate · release to transcribe and paste
        </p>
      </section>

      {permissions && (
        <section className="flex flex-col gap-2">
          <span className="font-mono text-eyebrow uppercase text-muted-foreground/80">
            Permissions
          </span>
          <ul className="flex flex-col">
            <PermissionRow
              icon={Microphone}
              label="Microphone"
              granted={permissions.microphone}
              onGrant={openMicrophoneSettings}
            />
            <PermissionRow
              icon={Keyboard}
              label="Accessibility"
              granted={permissions.accessibility}
              onGrant={openAccessibilitySettings}
            />
          </ul>
        </section>
      )}
    </div>
  );
}

interface PermissionRowProps {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  granted: boolean;
  onGrant: () => void;
}

function PermissionRow({ icon: Icon, label, granted, onGrant }: PermissionRowProps) {
  return (
    <li className="flex items-center gap-3 py-2.5 border-t border-border/60 last:border-b">
      <Icon size={15} className="text-muted-foreground shrink-0" />
      <span className="flex-1 text-[13px] text-foreground">{label}</span>
      {granted ? (
        <span className="text-[12px] text-green-600 dark:text-green-500 font-medium">
          Granted
        </span>
      ) : (
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[12px]"
          onClick={onGrant}
        >
          Grant Permission
        </Button>
      )}
    </li>
  );
}
