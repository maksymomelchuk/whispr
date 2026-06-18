import {
  ArrowRightIcon,
  BrainIcon,
  CheckFatIcon,
  DiamondIcon,
  KeyboardIcon,
  MicrophoneIcon,
  TextTIcon,
  XIcon,
} from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { AbstractLoops } from "@/components/AbstractLoops";
import { AppAvatar } from "@/components/AppAvatar";
import { SectionHeader } from "@/components/SectionHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

import { useSettings } from "../context/SettingsContext";
import {
  checkPermissions,
  ensurePttStarted,
  formatShortcut,
  getHistory,
  getLocalModelStatuses,
  getStats,
  openAccessibilitySettings,
  openMicrophoneSettings,
  type PermissionsStatus,
} from "../lib/api";
import type {
  CleanupStatus,
  HistoryEntry,
  LocalModelStatus,
  Settings,
  StatsRow,
} from "../lib/types";

const GUIDE_DISMISSED_KEY = "whispr.setup-guide-dismissed";
const TYPING_WPM_BASELINE = 40;
const RECENT_LIMIT = 4;
const RECENT_SNIPPET_CHARS = 90;
const ACTIVITY_DAYS = 14;

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

function localDateISO(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function formatCount(n: number): string {
  return n.toLocaleString();
}

function formatTimeSaved(words: number, seconds: number): string {
  const savedMinutes = words / TYPING_WPM_BASELINE - seconds / 60;
  if (savedMinutes < 1) return "0m";
  const h = Math.floor(savedMinutes / 60);
  const m = Math.round(savedMinutes % 60);
  if (h === 0) return `${m}m`;
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

function relativeTime(unixSeconds: number): string {
  const minutes = Math.floor((Date.now() - unixSeconds * 1000) / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

// streak walks back from today; if today has no activity yet it starts from
// yesterday so a day still in progress doesn't read as a broken streak.
function computeStreak(rows: StatsRow[]): number {
  const active = new Set(rows.filter((r) => r.words > 0).map((r) => r.date));
  if (active.size === 0) return 0;
  const cursor = new Date();
  if (!active.has(localDateISO(cursor))) cursor.setDate(cursor.getDate() - 1);
  let streak = 0;
  while (active.has(localDateISO(cursor))) {
    streak += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return streak;
}

interface TodayStats {
  words: number;
  seconds: number;
  streak: number;
}

function summarizeToday(rows: StatsRow[]): TodayStats {
  const todayISO = localDateISO(new Date());
  const today = rows.find((r) => r.date === todayISO);
  return {
    words: today?.words ?? 0,
    seconds: today?.total_seconds ?? 0,
    streak: computeStreak(rows),
  };
}

interface DayPoint {
  words: number;
  seconds: number;
}

function lastNDays(rows: StatsRow[], days: number): DayPoint[] {
  const byDate = new Map(rows.map((r) => [r.date, r]));
  const out: DayPoint[] = [];
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    const row = byDate.get(localDateISO(d));
    out.push({ words: row?.words ?? 0, seconds: row?.total_seconds ?? 0 });
  }
  return out;
}

function sumWords(days: DayPoint[]): number {
  return days.reduce((total, day) => total + day.words, 0);
}

// percentage change of the most recent week vs the one before it; null when
// there's no prior baseline to compare against.
function weekOverWeekDelta(days: DayPoint[]): number | null {
  if (days.length < 14) return null;
  const prior = sumWords(days.slice(0, 7));
  if (prior === 0) return null;
  const recent = sumWords(days.slice(7, 14));
  return Math.round(((recent - prior) / prior) * 100);
}

function entrySnippet(entry: HistoryEntry): string {
  const text = entry.final_text || entry.replaced_text || entry.raw_text;
  const trimmed = text.trim();
  if (trimmed.length <= RECENT_SNIPPET_CHARS) return trimmed;
  return `${trimmed.slice(0, RECENT_SNIPPET_CHARS).trimEnd()}…`;
}

function formatDictationDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const seconds = ms / 1000;
  return seconds < 10 ? `${seconds.toFixed(1)}s` : `${Math.round(seconds)}s`;
}

interface CleanupChip {
  label: string;
  tone: "warn" | "error";
}

// only the states worth flagging at a glance earn a chip; the common
// ran/disabled/skipped cases stay quiet so the strip reads calm.
function cleanupChip(status: CleanupStatus): CleanupChip | null {
  switch (status.kind) {
    case "failed_timeout":
    case "failed_transient":
      return { label: "cleanup failed", tone: "error" };
    case "failed_credential":
      return { label: "auth error", tone: "error" };
    case "no_credential":
      return { label: "cleanup off", tone: "warn" };
    case "recovered_manually":
      return { label: "recovered", tone: "warn" };
    default:
      return null;
  }
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
  const [recent, setRecent] = useState<HistoryEntry[]>([]);
  const [statsRows, setStatsRows] = useState<StatsRow[]>([]);
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
                if (cancelled) return;
                setHasDictated(entries.length > 0);
                setRecent(
                  [...entries]
                    .sort((a, b) => b.timestamp - a.timestamp)
                    .slice(0, RECENT_LIMIT),
                );
              })
              .catch(() => {});
            getStats()
              .then((rows) => {
                if (!cancelled) setStatsRows(rows);
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
  const hotkeyBinding = settings.hotkey_bindings.find(
    (b) => b.action.type === "Ptt",
  );
  const hotkeyLabel = hotkeyBinding
    ? formatShortcut(hotkeyBinding.shortcut)
    : null;

  if (lifecycle === "activated") {
    return (
      <Dashboard
        recent={recent}
        statsRows={statsRows}
        hotkeyLabel={hotkeyLabel}
        settings={settings}
        onNavigate={navigate}
      />
    );
  }

  return (
    <SetupHero
      lifecycle={lifecycle}
      permissions={permissions}
      allReady={allReady}
      speechModelReady={speechModelReady}
      hotkeyBound={Boolean(hotkeyBinding)}
      guideDismissed={guideDismissed}
      onDismissGuide={() => {
        writeGuideDismissed();
        setGuideDismissed(true);
      }}
      onNavigate={navigate}
    />
  );
}

interface DashboardProps {
  recent: HistoryEntry[];
  statsRows: StatsRow[];
  hotkeyLabel: string | null;
  settings: Settings;
  onNavigate: (path: string) => void;
}

function Dashboard({
  recent,
  statsRows,
  hotkeyLabel,
  settings,
  onNavigate,
}: DashboardProps) {
  const today = useMemo(() => summarizeToday(statsRows), [statsRows]);
  const activity = useMemo(
    () => lastNDays(statsRows, ACTIVITY_DAYS),
    [statsRows],
  );
  const activityWords = sumWords(activity);
  const activitySeconds = activity.reduce((total, d) => total + d.seconds, 0);
  const delta = weekOverWeekDelta(activity);

  const termCount = settings.term_sets.reduce(
    (n, set) => n + set.entries.length,
    0,
  );
  const suggestVocabulary = termCount === 0;
  const suggestAutoLearn = !settings.learn_from_corrections;

  return (
    <div className="flex w-full flex-col gap-6 p-6">
      <header className="flex flex-col gap-1.5">
        <h1 className="text-page-title text-foreground text-balance">
          {timeGreeting()}
        </h1>
        <p className="text-[13px] text-muted-foreground">
          {hotkeyLabel ? (
            <>
              Hold{" "}
              <kbd className="rounded bg-muted px-1.5 py-0.5 font-mono text-[12px] text-foreground">
                {hotkeyLabel}
              </kbd>{" "}
              and speak.
            </>
          ) : (
            "Your voice-to-text is ready."
          )}
        </p>
      </header>

      <section className="grid grid-cols-3 gap-3">
        <StatTile value={formatCount(today.words)} label="words today" />
        <StatTile
          value={formatTimeSaved(today.words, today.seconds)}
          label="saved today"
        />
        <StatTile
          value={today.streak > 0 ? String(today.streak) : "—"}
          label="day streak"
        />
      </section>

      {activityWords > 0 && (
        <section className="flex flex-col gap-2.5">
          <SectionHeader
            title="Activity"
            control={
              <button
                type="button"
                onClick={() => onNavigate("/stats")}
                className="inline-flex items-center gap-1 rounded text-[12px] text-primary transition-colors hover:text-primary/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                Stats <ArrowRightIcon size={12} />
              </button>
            }
          />
          <ActivityBars data={activity.map((d) => d.words)} />
          <p className="text-[12px] text-muted-foreground">
            Past 14 days{" · "}
            <span className="font-medium tabular-nums text-foreground">
              {formatCount(activityWords)}
            </span>{" "}
            words{" · "}
            <span className="font-medium tabular-nums text-foreground">
              {formatTimeSaved(activityWords, activitySeconds)}
            </span>{" "}
            saved
            {delta !== null && (
              <span className="text-muted-foreground/80">
                {" · "}
                {delta >= 0 ? "↑" : "↓"} {Math.abs(delta)}% vs last week
              </span>
            )}
          </p>
        </section>
      )}

      {recent.length > 0 && (
        <section className="flex flex-col gap-2">
          <SectionHeader
            title="Recent"
            control={
              <button
                type="button"
                onClick={() => onNavigate("/history")}
                className="inline-flex items-center gap-1 text-[12px] text-primary transition-colors hover:text-primary/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded"
              >
                View all <ArrowRightIcon size={12} />
              </button>
            }
          />
          <ul className="flex flex-col">
            {recent.map((entry) => (
              <RecentRow
                key={entry.id || entry.timestamp}
                entry={entry}
                onOpen={() => onNavigate("/history")}
              />
            ))}
          </ul>
        </section>
      )}

      {(suggestVocabulary || suggestAutoLearn) && (
        <section className="flex flex-col gap-2">
          <SectionHeader title="Sharpen your dictation" />
          <ul className="flex flex-col">
            {suggestVocabulary && (
              <NudgeRow
                icon={TextTIcon}
                label="Add vocabulary for names and jargon"
                onAction={() => onNavigate("/terms")}
              />
            )}
            {suggestAutoLearn && (
              <NudgeRow
                icon={BrainIcon}
                label="Turn on Auto-Learn from your edits"
                onAction={() => onNavigate("/learned")}
              />
            )}
          </ul>
        </section>
      )}
    </div>
  );
}

// mostly one muted hue; today is the single accent — color encodes "now",
// not decoration.
function ActivityBars({ data }: { data: number[] }) {
  const max = Math.max(...data, 1);
  const lastIndex = data.length - 1;
  return (
    <div className="flex h-12 items-end gap-1" aria-hidden="true">
      {data.map((words, index) => {
        const isToday = index === lastIndex;
        const heightPct = words === 0 ? 0 : Math.max((words / max) * 100, 8);
        return (
          <div
            key={index}
            className="relative h-full flex-1 overflow-hidden rounded-[3px] bg-muted/60"
          >
            <div
              className="absolute inset-x-0 bottom-0 rounded-[3px]"
              style={{
                height: `${heightPct}%`,
                background: isToday
                  ? "var(--loops-warm)"
                  : "var(--color-primary)",
                opacity: isToday ? 1 : 0.55,
              }}
            />
          </div>
        );
      })}
    </div>
  );
}

function StatTile({ value, label }: { value: string; label: string }) {
  return (
    <div className="flex flex-col gap-1 rounded-lg bg-card px-4 py-3.5 shadow-xs">
      <span className="text-[26px] font-semibold leading-none tabular-nums text-foreground">
        {value}
      </span>
      <span className="text-[12px] text-muted-foreground">{label}</span>
    </div>
  );
}

function RecentRow({
  entry,
  onOpen,
}: {
  entry: HistoryEntry;
  onOpen: () => void;
}) {
  const chip = cleanupChip(entry.cleanup_status);

  return (
    <li className="border-t border-border/60 last:border-b">
      <button
        type="button"
        onClick={onOpen}
        className="flex w-full items-center gap-3 rounded py-2.5 text-left transition-colors hover:bg-accent/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <AppAvatar
          name={entry.app_name ?? "?"}
          bundleId={entry.bundle_id}
          size={28}
        />
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="truncate text-[13px] text-foreground">
            {entrySnippet(entry)}
          </span>
          <span className="flex items-center gap-1.5 truncate text-[11px] text-muted-foreground/70">
            <span className="tabular-nums">
              {formatDictationDuration(entry.speak_duration_ms)}
            </span>
            <Dot />
            <span className="whitespace-nowrap tabular-nums">
              {relativeTime(entry.timestamp)}
            </span>
          </span>
        </span>
        {chip && (
          <Badge variant={chip.tone} className="shrink-0 text-[9px]">
            {chip.label}
          </Badge>
        )}
      </button>
    </li>
  );
}

function Dot() {
  return (
    <span aria-hidden="true" className="text-muted-foreground/40">
      ·
    </span>
  );
}

interface NudgeRowProps {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  onAction: () => void;
}

function NudgeRow({ icon: Icon, label, onAction }: NudgeRowProps) {
  return (
    <li className="flex items-center gap-3 border-t border-border/60 py-2.5 last:border-b">
      <Icon size={15} className="shrink-0 text-muted-foreground" />
      <span className="flex-1 text-[13px] text-foreground">{label}</span>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 text-[12px] text-primary"
        onClick={onAction}
      >
        Set up →
      </Button>
    </li>
  );
}

interface SetupHeroProps {
  lifecycle: "pending" | "loading" | "activating";
  permissions: PermissionsStatus | null;
  allReady: boolean;
  speechModelReady: boolean;
  hotkeyBound: boolean;
  guideDismissed: boolean;
  onDismissGuide: () => void;
  onNavigate: (path: string) => void;
}

function SetupHero({
  lifecycle,
  permissions,
  allReady,
  speechModelReady,
  hotkeyBound,
  guideDismissed,
  onDismissGuide,
  onNavigate,
}: SetupHeroProps) {
  const subtitle =
    permissions === null
      ? " "
      : lifecycle === "pending"
        ? "Grant permissions below to get started."
        : lifecycle === "loading"
          ? " "
          : "Finish setting up.";

  return (
    <div className="relative flex min-h-full items-center justify-center overflow-hidden px-10 py-10">
      <div
        className="pointer-events-none absolute inset-0"
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
          className="absolute inset-0 h-full w-full opacity-70"
        />
      </div>

      <div className="relative flex w-full max-w-sm flex-col gap-7">
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
                      onClick={onDismissGuide}
                      aria-label="Dismiss setup guide"
                      className="rounded text-muted-foreground/50 transition-colors hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      <XIcon size={13} />
                    </button>
                  }
                />
                <ul className="mt-1 flex flex-col">
                  <SetupRow
                    label="Choose a speech model"
                    done={speechModelReady}
                    actionLabel="Set up"
                    onAction={() => onNavigate("/speech-models")}
                  />
                  <SetupRow
                    label="Bind a push-to-talk hotkey"
                    done={hotkeyBound}
                    actionLabel="Bind"
                    onAction={() => onNavigate("/hotkeys")}
                  />
                </ul>
                <p className="border-t border-border/60 py-2.5 text-[13px] text-muted-foreground/50">
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
    <li className="flex items-center gap-3 border-t border-border/60 py-2.5 last:border-b">
      <Icon size={15} className="shrink-0 text-muted-foreground" />
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
    <li className="flex items-center gap-3 border-t border-border/60 py-2.5">
      {done ? (
        <CheckFatIcon
          size={12}
          weight="fill"
          className="shrink-0 text-muted-foreground/40"
          aria-hidden="true"
        />
      ) : (
        <DiamondIcon
          size={12}
          className="shrink-0 text-muted-foreground/40"
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
