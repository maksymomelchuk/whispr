import {
  CheckFatIcon,
  DiamondIcon,
  KeyboardIcon,
  MicrophoneIcon,
  XIcon,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import { AbstractLoops } from "@/components/AbstractLoops";
import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";

import { useSettings } from "../context/SettingsContext";
import {
  checkPermissions,
  ensurePttStarted,
  getHistory,
  getLocalModelStatuses,
  openAccessibilitySettings,
  openMicrophoneSettings,
  type PermissionsStatus,
} from "../lib/api";
import type { LocalModelStatus, Settings } from "../lib/types";

const GUIDE_DISMISSED_KEY = "whispr.setup-guide-dismissed";

function readGuideDismissed(): boolean {
  try {
    return localStorage.getItem(GUIDE_DISMISSED_KEY) === "true";
  } catch {
    /* localStorage may be unavailable in some webview contexts */
    return false;
  }
}

function writeGuideDismissed() {
  try {
    localStorage.setItem(GUIDE_DISMISSED_KEY, "true");
  } catch {
    /* localStorage may be unavailable in some webview contexts */
  }
}

function timeGreeting(): string {
  const h = new Date().getHours();
  if (h < 12) return "Good morning.";
  if (h < 17) return "Good afternoon.";
  return "Good evening.";
}

function isSpeechModelReady(
  settings: Settings,
  localStatuses: LocalModelStatus[],
): boolean {
  const activeMode = settings.modes[0];
  if (!activeMode) return false;
  const pm = activeMode.provider_model;
  switch (pm.provider) {
    case "deepgram":
      return settings.deepgram_api_key_configured;
    case "groq":
      return settings.groq_api_key_configured;
    case "assembly_ai":
      return settings.assemblyai_api_key_configured;
    case "open_ai":
      return settings.openai_api_key_configured;
    case "eleven_labs":
      return settings.elevenlabs_api_key_configured;
    case "soniox":
      return settings.soniox_api_key_configured;
    case "local": {
      const status = localStatuses.find((s) => s.model === pm.model);
      return Boolean(status?.downloaded && !status.load_failed);
    }
  }
}

export function HomePage() {
  const { settings } = useSettings();
  const navigate = useNavigate();
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(
    null,
  );
  const [localStatuses, setLocalStatuses] = useState<LocalModelStatus[]>([]);
  const [hasDictated, setHasDictated] = useState<boolean | null>(null);
  const [guideDismissed, setGuideDismissed] = useState(readGuideDismissed);

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
          if (perms.microphone && perms.accessibility) {
            getHistory()
              .then((entries) => {
                if (!cancelled) setHasDictated(entries.length > 0);
              })
              .catch(() => {});
            getLocalModelStatuses()
              .then((statuses) => {
                if (!cancelled) setLocalStatuses(statuses);
              })
              .catch(() => {});
          }
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

  const lifecycle = !allReady
    ? "pending"
    : hasDictated === null
      ? "loading"
      : hasDictated
        ? "activated"
        : "activating";

  const speechModelReady = allReady
    ? isSpeechModelReady(settings, localStatuses)
    : false;
  const hotkeyBound = settings.hotkey_bindings.some(
    (b) => b.action.type === "Ptt",
  );

  const subtitle =
    permissions === null
      ? " "
      : lifecycle === "pending"
        ? "Grant permissions below to get started."
        : lifecycle === "loading"
          ? " "
          : lifecycle === "activating"
            ? "Finish setting up."
            : "Your voice-to-text is ready.";

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
          <p
            className="text-[13px] text-muted-foreground"
            style={{ minHeight: "1.45em" }}
          >
            {subtitle}
          </p>
        </div>

        {lifecycle === "pending" && (
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
        )}

        {lifecycle === "activating" && (
          <>
            <div className="flex items-center gap-2 text-[13px] text-muted-foreground/60">
              <CheckFatIcon
                size={12}
                weight="fill"
                className="shrink-0"
                aria-hidden="true"
              />
              <span>Permissions granted</span>
            </div>

            {!guideDismissed && (
              <section className="flex flex-col gap-0">
                <SectionHeader
                  title="Set up dictation"
                  control={
                    <button
                      type="button"
                      onClick={() => {
                        writeGuideDismissed();
                        setGuideDismissed(true);
                      }}
                      aria-label="Dismiss setup guide"
                      className="text-muted-foreground/50 hover:text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded"
                    >
                      <XIcon size={13} />
                    </button>
                  }
                />
                <ul className="flex flex-col mt-1">
                  <SetupRow
                    label="Choose a speech model"
                    done={speechModelReady}
                    actionLabel="Set up"
                    onAction={() => navigate("/speech-models")}
                  />
                  <SetupRow
                    label="Bind a push-to-talk hotkey"
                    done={hotkeyBound}
                    actionLabel="Bind"
                    onAction={() => navigate("/hotkeys")}
                  />
                </ul>
                <p className="text-[13px] text-muted-foreground/50 py-2.5 border-t border-border/60">
                  Then hold your hotkey and speak.
                </p>
              </section>
            )}
          </>
        )}
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
      {granted === undefined ? (
        <span className="text-[12px] text-muted-foreground/40">—</span>
      ) : granted ? (
        <span className="text-[12px] font-medium text-muted-foreground">
          Granted
        </span>
      ) : (
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[12px]"
          onClick={onGrant}
        >
          Grant
        </Button>
      )}
    </li>
  );
}

interface SetupRowProps {
  label: string;
  done: boolean;
  actionLabel: string;
  onAction: () => void;
}

function SetupRow({ label, done, actionLabel, onAction }: SetupRowProps) {
  return (
    <li className="flex items-center gap-3 py-2.5 border-t border-border/60">
      {done ? (
        <CheckFatIcon
          size={12}
          weight="fill"
          className="text-muted-foreground/40 shrink-0"
          aria-hidden="true"
        />
      ) : (
        <DiamondIcon
          size={12}
          className="text-muted-foreground/40 shrink-0"
          aria-hidden="true"
        />
      )}
      <span
        className={`flex-1 text-[13px] ${done ? "text-muted-foreground/60" : "text-foreground"}`}
      >
        {label}
      </span>
      {!done && (
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-[12px] text-primary"
          onClick={onAction}
        >
          {actionLabel} →
        </Button>
      )}
    </li>
  );
}
