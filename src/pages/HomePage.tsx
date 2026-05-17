import { Keyboard, Microphone } from "@phosphor-icons/react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";

import { useSettings } from "../context/SettingsContext";
import {
  checkPermissions,
  openAccessibilitySettings,
  openMicrophoneSettings,
  type PermissionsStatus,
} from "../lib/api";

export function HomePage() {
  const { settings } = useSettings();
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(
    null,
  );

  useEffect(() => {
    checkPermissions()
      .then(setPermissions)
      .catch(() => {});
  }, []);

  if (!settings) return null;

  return (
    <div className="flex flex-col gap-10 px-10 pt-9 pb-12 max-w-3xl">
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

function PermissionRow({
  icon: Icon,
  label,
  granted,
  onGrant,
}: PermissionRowProps) {
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
