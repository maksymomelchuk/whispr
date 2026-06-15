import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import {
  getHistory,
  clearHistory as persistClearHistory,
  setHistoryLimit as persistHistoryLimit,
  recoverCleanup,
  updateHistoryEntry,
} from "../lib/api";
import type { CleanupStatus, HistoryEntry, HistoryLimit } from "../lib/types";
import { providerModelLabel } from "../lib/types";

const LIMIT_OPTIONS: { label: string; value: string }[] = [
  { label: "Off", value: "0" },
  { label: "5", value: "5" },
  { label: "10", value: "10" },
  { label: "25", value: "25" },
  { label: "50", value: "50" },
  { label: "100", value: "100" },
  { label: "Unlimited", value: "unlimited" },
];

const limitToOptionValue = (l: HistoryLimit): string =>
  l === null ? "unlimited" : String(l);

const optionValueToLimit = (v: string): HistoryLimit =>
  v === "unlimited" ? null : Number(v);

const limitHint = (l: HistoryLimit, count: number): string => {
  if (l === 0) return "History is off. Recent dictations are not saved.";
  if (l === null)
    return `${count} ${count === 1 ? "entry" : "entries"} stored locally. No limit.`;
  return `${count} of up to ${l} ${l === 1 ? "entry" : "entries"} stored locally.`;
};

type LoadState = "loading" | "ready" | "error";

function entryId(entry: HistoryEntry): string {
  return entry.id || `${entry.timestamp}|${entry.final_text.length}`;
}

function dayKey(timestamp: number): string {
  const d = new Date(timestamp * 1000);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

function dayLabel(timestamp: number, now: number): string {
  const date = new Date(timestamp * 1000);
  const today = new Date(now * 1000);
  const startOfToday = new Date(
    today.getFullYear(),
    today.getMonth(),
    today.getDate(),
  ).getTime();
  const entryDay = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  ).getTime();
  const dayDiff = Math.round((startOfToday - entryDay) / (1000 * 60 * 60 * 24));
  if (dayDiff === 0) return "Today";
  if (dayDiff === 1) return "Yesterday";
  if (dayDiff < 7) return date.toLocaleDateString([], { weekday: "long" });
  return date.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== today.getFullYear() ? "numeric" : undefined,
  });
}

function formatTimeOfDay(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function formatHeldDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  return s < 10 ? `${s.toFixed(1)}s` : `${Math.round(s)}s`;
}

function useClock(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    const id = window.setInterval(() => {
      setNow(Math.floor(Date.now() / 1000));
    }, intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);
  return now;
}

export function HistoryTab() {
  const { settings, setSettings } = useSettings();
  const historyLimit = settings.history_limit;
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const now = useClock();
  const { flash, isFlashing } = useFlash();
  const seenIds = useRef<Set<string>>(new Set());
  const initialized = useRef(false);

  const [clearDialogOpen, setClearDialogOpen] = useState(false);

  const handleClearConfirm = async () => {
    try {
      await persistClearHistory();
      setEntries([]);
      seenIds.current.clear();
    } catch (e) {
      console.error("clear history failed", e);
    }
    setClearDialogOpen(false);
  };

  const handleLimitChange = async (value: string) => {
    const next = optionValueToLimit(value);
    try {
      await persistHistoryLimit(next);
      setSettings((s) => ({ ...s, history_limit: next }));
    } catch (err) {
      console.error("set history limit failed", err);
    }
  };

  const refresh = () => {
    getHistory()
      .then((list) => {
        setEntries(list);
        setLoadState("ready");
        if (!initialized.current) {
          seenIds.current = new Set(list.map(entryId));
          initialized.current = true;
          return;
        }
        for (const e of list) {
          const id = entryId(e);
          if (!seenIds.current.has(id)) {
            seenIds.current.add(id);
            flash(id);
            break;
          }
        }
      })
      .catch((e) => {
        setLoadState("error");
        setLoadError(String(e));
      });
  };

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen("history-updated", () => refresh())
      .then((u) => {
        unlisten = u;
      })
      .catch((e) => console.error("history-updated listen failed", e));
    return () => {
      unlisten?.();
    };
  }, []);

  const groups = useMemo(() => {
    const buckets: { key: string; label: string; entries: HistoryEntry[] }[] =
      [];
    for (const e of entries) {
      const key = dayKey(e.timestamp);
      const existing = buckets.find((b) => b.key === key);
      if (existing) {
        existing.entries.push(e);
      } else {
        buckets.push({ key, label: dayLabel(e.timestamp, now), entries: [e] });
      }
    }
    return buckets;
  }, [entries, now]);

  if (loadState === "loading") {
    return (
      <div className="py-10 text-center text-muted-foreground">Loading…</div>
    );
  }

  if (loadState === "error") {
    return (
      <Alert variant="destructive">
        <AlertDescription>Failed to load history: {loadError}</AlertDescription>
      </Alert>
    );
  }

  const isOff = historyLimit === 0;

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-start justify-between gap-3 pb-1">
        <p className="text-xs text-muted-foreground">
          {limitHint(historyLimit, entries.length)}
        </p>
        <div className="flex shrink-0 items-center gap-2.5">
          <div className="inline-flex items-center gap-1.5">
            <span className="whitespace-nowrap text-form-label text-muted-foreground">
              Keep last
            </span>
            <Select
              value={limitToOptionValue(historyLimit)}
              onValueChange={handleLimitChange}
            >
              <SelectTrigger size="sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LIMIT_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {entries.length > 0 && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setClearDialogOpen(true)}
            >
              Clear all
            </Button>
          )}
        </div>
      </div>

      {entries.length === 0 && (isOff ? <DisabledState /> : <EmptyState />)}

      {groups.map((group) => (
        <section key={group.key} className="flex flex-col gap-2.5">
          <SectionHeader
            title={group.label}
            trailing={`${group.entries.length} ${
              group.entries.length === 1 ? "entry" : "entries"
            }`}
          />
          <div className="flex flex-col gap-2">
            {group.entries.map((entry) => (
              <HistoryRow
                key={entryId(entry)}
                entry={entry}
                flashing={isFlashing(entryId(entry))}
              />
            ))}
          </div>
        </section>
      ))}

      <ClearHistoryConfirmDialog
        entryCount={entries.length}
        open={clearDialogOpen}
        onConfirm={handleClearConfirm}
        onCancel={() => setClearDialogOpen(false)}
      />
    </div>
  );
}

function ClearHistoryConfirmDialog({
  entryCount,
  open,
  onConfirm,
  onCancel,
}: {
  entryCount: number;
  open: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Clear all history?</DialogTitle>
          <DialogDescription>
            Delete all {entryCount} {entryCount === 1 ? "entry" : "entries"}?
            This can&rsquo;t be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            Clear all
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function EmptyState() {
  return (
    <EmptyPanel
      title="No transcriptions yet"
      hint="Hold your shortcut and speak — transcripts will appear here."
    />
  );
}

function DisabledState() {
  return (
    <EmptyPanel
      title="History is disabled"
      hint="Pick a Keep last value above to start saving transcripts again."
    />
  );
}

type Tone = "neutral" | "warn" | "error";
interface CleanupBadge {
  label: string;
  tone: Tone;
}
interface StageNote {
  text: string;
  tone: "info" | "error";
}
interface CleanupView {
  badge: CleanupBadge | null;
  note: (textChanged: boolean) => StageNote | null;
}

function cleanupView(status: CleanupStatus): CleanupView {
  switch (status.kind) {
    case "disabled":
      return {
        badge: null,
        note: () => ({ text: "Cleanup is disabled.", tone: "info" }),
      };
    case "ran":
      return {
        badge: null,
        note: (changed) =>
          changed
            ? null
            : { text: "Cleanup ran — no edits needed.", tone: "info" },
      };
    case "recovered_manually":
      return {
        badge: { label: "recovered", tone: "neutral" },
        note: () => ({ text: "Recovered via manual retry.", tone: "info" }),
      };
    case "skipped_below_min_words":
      return {
        badge: { label: "skipped: too short", tone: "neutral" },
        note: () => ({
          text: "Skipped: below minimum word count.",
          tone: "info",
        }),
      };
    case "skipped_below_min_duration":
      return {
        badge: { label: "skipped: too brief", tone: "neutral" },
        note: () => ({
          text: "Skipped: below minimum duration.",
          tone: "info",
        }),
      };
    case "no_credential":
      return {
        badge: { label: "cleanup unconfigured", tone: "warn" },
        note: () => ({
          text: "Skipped: no credential configured.",
          tone: "error",
        }),
      };
    case "failed_timeout":
      return {
        badge: { label: "cleanup failed: timeout", tone: "error" },
        note: () => ({ text: "Failed: request timed out.", tone: "error" }),
      };
    case "failed_transient":
      return {
        badge: { label: "cleanup failed", tone: "error" },
        note: () => ({ text: `Failed: ${status.message}`, tone: "error" }),
      };
    case "failed_credential":
      return {
        badge: { label: "cleanup auth error", tone: "error" },
        note: () => ({ text: `Auth error: ${status.message}`, tone: "error" }),
      };
  }
}

function useCopyFlash(): { copied: boolean; flash: (text: string) => void } {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  const flash = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
      timeoutRef.current = window.setTimeout(() => setCopied(false), 1200);
    } catch (e) {
      console.error("copy failed", e);
    }
  };

  return { copied, flash };
}

function isRecoverable(entry: HistoryEntry): boolean {
  if (!entry.id || !entry.profile_snapshot) return false;
  const { kind } = entry.cleanup_status;
  return (
    kind === "failed_timeout" ||
    kind === "failed_transient" ||
    kind === "failed_credential" ||
    kind === "no_credential"
  );
}

function HistoryRow({
  entry,
  flashing,
}: {
  entry: HistoryEntry;
  flashing: boolean;
}) {
  const [traceOpen, setTraceOpen] = useState(false);
  const { copied, flash } = useCopyFlash();
  const [recovering, setRecovering] = useState(false);
  const [recoverError, setRecoverError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(entry.final_text);
  const [savedText, setSavedText] = useState(entry.final_text);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const view = cleanupView(entry.cleanup_status);
  const recoverable = isRecoverable(entry);
  const editable = Boolean(entry.id);

  const handleRecover = async () => {
    setRecovering(true);
    setRecoverError(null);
    try {
      const text = await recoverCleanup(entry.id);
      flash(text);
    } catch (e) {
      setRecoverError(String(e));
    } finally {
      setRecovering(false);
    }
  };

  const handleEditStart = () => {
    setEditText(savedText);
    setSaveError(null);
    setEditing(true);
  };

  const handleEditCancel = () => {
    setEditing(false);
    setSaveError(null);
  };

  const handleEditSave = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await updateHistoryEntry(entry.id, savedText, editText);
      setSavedText(editText);
      setEditing(false);
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <RowCard
      flashing={flashing}
      interactive={!traceOpen && !editing}
      className="items-stretch flex-col gap-2 py-3 pr-3"
    >
      <div className="flex items-start gap-3.5">
        <div
          className="flex flex-col items-end shrink-0 w-[68px] pt-0.5"
          aria-label="time and duration"
        >
          <time
            dateTime={new Date(entry.timestamp * 1000).toISOString()}
            className="font-mono text-[13px] font-semibold tabular-nums text-foreground leading-none"
          >
            {formatTimeOfDay(entry.timestamp)}
          </time>
          <span className="mt-1 font-mono text-kbd tabular-nums text-muted-foreground/70">
            {formatHeldDuration(entry.speak_duration_ms)}
          </span>
        </div>

        <div className="flex flex-1 min-w-0 flex-col gap-1.5">
          {editing ? (
            <>
              <Textarea
                value={editText}
                onChange={(e) => setEditText(e.target.value)}
                className="text-[13px] leading-[1.5] resize-none min-h-[60px]"
                autoFocus
              />
              {saveError && (
                <p className="text-xs text-destructive">{saveError}</p>
              )}
              <div className="flex items-center gap-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  className="text-muted-foreground"
                  disabled={saving}
                  onClick={handleEditSave}
                >
                  {saving ? "Saving…" : "Save"}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  className="text-muted-foreground"
                  disabled={saving}
                  onClick={handleEditCancel}
                >
                  Cancel
                </Button>
              </div>
            </>
          ) : (
            <>
              <div className="whitespace-pre-wrap break-words text-[13px] leading-[1.5] text-foreground select-text">
                {savedText}
              </div>
              {view.badge && (
                <Badge
                  variant={view.badge.tone}
                  className="self-start text-[10px]"
                >
                  {view.badge.label}
                </Badge>
              )}
              <div className="flex items-center gap-1 pt-0.5 opacity-65 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity">
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  className="text-muted-foreground"
                  aria-expanded={traceOpen}
                  onClick={() => setTraceOpen((o) => !o)}
                >
                  {traceOpen ? "Hide trace" : "Show trace"}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="xs"
                  className="text-muted-foreground"
                  aria-label="Copy transcript"
                  aria-live="polite"
                  onClick={() => flash(savedText)}
                >
                  {copied ? "Copied" : "Copy"}
                </Button>
                {editable && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="xs"
                    className="text-muted-foreground"
                    onClick={handleEditStart}
                  >
                    Edit
                  </Button>
                )}
                {recoverable && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="xs"
                    className="text-muted-foreground"
                    disabled={recovering}
                    onClick={handleRecover}
                  >
                    {recovering ? "Recovering…" : "Recover"}
                  </Button>
                )}
              </div>
              {recoverError && (
                <p className="text-xs text-destructive">{recoverError}</p>
              )}
            </>
          )}
        </div>
      </div>

      {!editing && traceOpen && (
        <div className="ml-[84px]">
          <HistoryTrace entry={entry} cleanupNote={view.note} />
        </div>
      )}
    </RowCard>
  );
}

interface TraceProps {
  entry: HistoryEntry;
  cleanupNote: (textChanged: boolean) => StageNote | null;
}

function HistoryTrace({ entry, cleanupNote }: TraceProps) {
  const cleanupTextChanged = entry.replaced_text !== entry.raw_text;
  return (
    <div className="flex flex-col gap-2 rounded-lg bg-muted/60 px-3 py-2.5">
      <Stage
        label={
          entry.provider_model
            ? providerModelLabel(entry.provider_model)
            : "Transcription"
        }
        text={entry.raw_text}
      />
      <Stage
        label="AI cleanup"
        text={entry.replaced_text}
        previousText={entry.raw_text}
        note={cleanupNote(cleanupTextChanged)}
      />
      <Stage
        label="Dictionary"
        text={entry.final_text}
        previousText={entry.replaced_text}
      />
    </div>
  );
}

interface StageProps {
  label: string;
  text: string;
  previousText?: string;
  note?: StageNote | null;
}

function Stage({ label, text, previousText, note }: StageProps) {
  const { copied, flash } = useCopyFlash();
  const unchanged = previousText !== undefined && previousText === text;
  return (
    <div
      data-history-stage
      className="flex flex-col gap-1 [&+[data-history-stage]]:mt-2 [&+[data-history-stage]]:border-t [&+[data-history-stage]]:border-dashed [&+[data-history-stage]]:border-border [&+[data-history-stage]]:pt-2"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-eyebrow uppercase text-muted-foreground/70">
          {label}
        </span>
        {!unchanged && text.length > 0 && (
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="text-muted-foreground"
            aria-label={`Copy ${label} output`}
            onClick={() => flash(text)}
          >
            {copied ? "Copied" : "Copy"}
          </Button>
        )}
      </div>
      {unchanged ? (
        <div className="text-help italic text-muted-foreground/70">
          (no change)
        </div>
      ) : (
        <div className="whitespace-pre-wrap break-words text-xs leading-[1.5] text-foreground select-text">
          {text.length === 0 ? (
            <span className="text-help italic text-muted-foreground/70">
              (empty)
            </span>
          ) : (
            text
          )}
        </div>
      )}
      {note && (
        <div
          className={cn(
            "text-help break-words",
            note.tone === "info"
              ? "italic text-muted-foreground/70"
              : "font-mono text-destructive",
          )}
        >
          {note.text}
        </div>
      )}
    </div>
  );
}
