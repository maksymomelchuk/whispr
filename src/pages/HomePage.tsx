import { KeyboardIcon, MicrophoneIcon } from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

import { AbstractLoops } from "@/components/AbstractLoops";
import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";
import {
  checkPermissions,
  ensurePttStarted,
  openAccessibilitySettings,
  openMicrophoneSettings,
  type PermissionsStatus,
} from "../lib/api";

function timeGreeting(): string {
  const h = new Date().getHours();
  if (h < 12) return "Good morning.";
  if (h < 17) return "Good afternoon.";
  return "Good evening.";
}

export function HomePage() {
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let listenCancelled = false;

    const refresh = () => {
      checkPermissions()
        .then((perms) => {
          if (cancelled) return;
          setPermissions(perms);
          if (perms.accessibility) ensurePttStarted().catch(() => {});
        })
        .catch(() => {});
    };

    refresh();
    const intervalId = setInterval(refresh, 3000);

    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) refresh();
      })
      .then((un) => {
        if (listenCancelled) un();
        else unlisten = un;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      listenCancelled = true;
      clearInterval(intervalId);
      unlisten?.();
    };
  }, []);

  const allReady =
    permissions?.microphone === true && permissions?.accessibility === true;

  const subtitle =
    permissions === null
      ? " "
      : allReady
        ? "Your voice-to-text is ready."
        : "Grant permissions below to get started.";

  return (
    <div className="relative flex min-h-full items-center justify-center px-10 py-10 overflow-hidden">
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          maskImage:
            "radial-gradient(ellipse 22% 28% at 50% 50%, transparent 0%, rgba(0,0,0,0.15) 35%, black 75%)",
          WebkitMaskImage:
            "radial-gradient(ellipse 22% 28% at 50% 50%, transparent 0%, rgba(0,0,0,0.15) 35%, black 75%)",
        }}
        aria-hidden="true"
      >
        <AbstractLoops
          active={allReady}
          fillMode="cover"
          scale={1.6}
          className="absolute inset-0 w-full h-full opacity-70"
        />
      </div>

      <div className="relative w-full max-w-sm flex flex-col gap-7">
        <div className="flex flex-col gap-1.5">
          <h1 className="text-page-title text-foreground text-balance">
            {timeGreeting()}
          </h1>
          <p className="text-[13px] text-muted-foreground" style={{ minHeight: "1.45em" }}>
            {subtitle}
          </p>
        </div>

        <section className="flex flex-col gap-2">
          <SectionHeader title="Permissions" />
          <ul className="flex flex-col">
            <PermissionRow
              icon={MicrophoneIcon}
              label="Microphone"
              granted={permissions?.microphone}
              onGrant={openMicrophoneSettings}
            />
            <PermissionRow
              icon={KeyboardIcon}
              label="Accessibility"
              granted={permissions?.accessibility}
              onGrant={openAccessibilitySettings}
            />
          </ul>
        </section>
      </div>
    </div>
  );
}

interface PermissionRowProps {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  granted: boolean | undefined;
  onGrant: () => void;
}

function PermissionRow({ icon: Icon, label, granted, onGrant }: PermissionRowProps) {
  return (
    <li className="flex items-center gap-3 py-2.5 border-t border-border/60 last:border-b">
      <Icon size={15} className="text-muted-foreground shrink-0" />
      <span className="flex-1 text-[13px] text-foreground">{label}</span>
      {granted === undefined ? (
        <span className="text-[12px] text-muted-foreground/40">—</span>
      ) : granted ? (
        <span className="text-[12px] font-medium text-green-600 dark:text-green-500">
          Granted
        </span>
      ) : (
        <Button variant="ghost" size="sm" className="h-7 text-[12px]" onClick={onGrant}>
          Grant
        </Button>
      )}
    </li>
  );
}
