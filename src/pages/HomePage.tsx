import { KeyboardIcon, MicrophoneIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";
import { WaveField } from "@/components/WaveField";
import {
  checkPermissions,
  getStats,
  openAccessibilitySettings,
  openMicrophoneSettings,
  type PermissionsStatus,
} from "../lib/api";
import type { StatsRow } from "../lib/types";

function todayISO(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function timeGreeting(): string {
  const h = new Date().getHours();
  if (h < 12) return "Good morning.";
  if (h < 17) return "Good afternoon.";
  return "Good evening.";
}

export function HomePage() {
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(null);
  const [todayWords, setTodayWords] = useState(0);
  const [todayDictations, setTodayDictations] = useState(0);

  useEffect(() => {
    checkPermissions().then(setPermissions).catch(() => {});
    getStats()
      .then((rows: StatsRow[]) => {
        const today = todayISO();
        const row = rows.find((r) => r.date === today);
        if (row) {
          setTodayWords(row.words);
          setTodayDictations(row.dictations);
        }
      })
      .catch(() => {});
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
    <div className="flex flex-col">
      <WaveField ready={allReady} />

      <div className="px-6 pt-4 pb-6 flex flex-col gap-6">
        <div className="flex flex-col gap-1">
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

        {todayWords > 0 && (
          <section className="flex flex-col gap-3">
            <SectionHeader
              title="Today"
              control={
                <Link
                  to="/stats"
                  className="text-[11px] text-muted-foreground hover:text-foreground transition-colors"
                >
                  View stats →
                </Link>
              }
            />
            <div className="flex gap-8 pt-0.5">
              <TodayStat value={todayWords.toLocaleString()} label="words" />
              <TodayStat value={String(todayDictations)} label="dictations" />
            </div>
          </section>
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

function TodayStat({ value, label }: { value: string; label: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-2xl font-semibold tabular-nums leading-none text-foreground">
        {value}
      </span>
      <span className="text-[11px] text-muted-foreground">{label}</span>
    </div>
  );
}
