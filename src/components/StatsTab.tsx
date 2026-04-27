import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";

import { useConfirmAction } from "../hooks/useConfirmAction";
import { clearStats as persistClearStats, getStats } from "../lib/api";
import type { StatsRow } from "../lib/types";

type LoadState = "loading" | "ready" | "error";

interface PeriodSpec {
  label: string;
  /// Number of trailing calendar days to include (1 = today only). `null` = all time.
  days: number | null;
}

const PERIODS: PeriodSpec[] = [
  { label: "Today", days: 1 },
  { label: "Last 7 days", days: 7 },
  { label: "Last 30 days", days: 30 },
  { label: "All time", days: null },
];

function localDateISO(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function daysAgoISO(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() - n);
  return localDateISO(d);
}

interface Aggregate {
  words: number;
  dictations: number;
  seconds: number;
}

function aggregateFor(rows: StatsRow[], spec: PeriodSpec): Aggregate {
  // Include today plus the previous (days - 1) calendar days; days = 1 → today only.
  const cutoff = spec.days === null ? null : daysAgoISO(spec.days - 1);
  const a: Aggregate = { words: 0, dictations: 0, seconds: 0 };
  for (const r of rows) {
    if (cutoff !== null && r.date < cutoff) continue;
    a.words += r.words;
    a.dictations += r.dictations;
    a.seconds += r.total_seconds;
  }
  return a;
}

function formatDuration(seconds: number): string {
  if (seconds <= 0) return "0s";
  if (seconds < 60) return `${seconds}s`;
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins < 60) return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
  const hrs = Math.floor(mins / 60);
  const remMins = mins % 60;
  return remMins > 0 ? `${hrs}h ${remMins}m` : `${hrs}h`;
}

function formatCount(n: number): string {
  return n.toLocaleString();
}

// Below ~5 seconds of dictation per period, WPM is too noisy to be meaningful
// (a single 2-word press skews it absurdly). Show "—" instead.
const MIN_SECONDS_FOR_WPM = 5;

function formatWpm(words: number, seconds: number): string {
  if (seconds < MIN_SECONDS_FOR_WPM) return "—";
  return String(Math.round((words / seconds) * 60));
}

export function StatsTab() {
  const [rows, setRows] = useState<StatsRow[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);

  const aggregates = useMemo(
    () => PERIODS.map((p) => ({ spec: p, agg: aggregateFor(rows, p) })),
    [rows],
  );

  const { confirming: confirmingClear, trigger: handleClear } = useConfirmAction(
    async () => {
      try {
        await persistClearStats();
        setRows([]);
      } catch (e) {
        console.error("clear stats failed", e);
      }
    },
  );

  const refresh = () => {
    getStats()
      .then((list) => {
        setRows(list);
        setLoadState("ready");
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
    listen("stats-updated", () => refresh())
      .then((u) => {
        unlisten = u;
      })
      .catch((e) => console.error("stats-updated listen failed", e));
    return () => {
      unlisten?.();
    };
  }, []);

  if (loadState === "loading") {
    return <div className="loading">Loading…</div>;
  }

  if (loadState === "error") {
    return (
      <section className="card err-card">Failed to load stats: {loadError}</section>
    );
  }

  const hasAny = rows.length > 0;

  return (
    <section className="stats">
      <div className="stats-header">
        <div>
          <h2 className="stats-title">Dictation stats</h2>
          <p className="hint stats-hint">
            Words dictated and your effective words per minute.
          </p>
        </div>
        {hasAny && (
          <button
            type="button"
            className={`stats-clear ${confirmingClear ? "confirm" : ""}`}
            onClick={handleClear}
          >
            {confirmingClear ? "Click to confirm" : "Clear stats"}
          </button>
        )}
      </div>

      {!hasAny && <EmptyState />}
      {hasAny && (
        <ul className="stats-list">
          {aggregates.map(({ spec, agg }) => (
            <li key={spec.label} className="stats-row">
              <div className="stats-row-head">
                <span className="stats-row-label">{spec.label}</span>
                <span className="stats-wpm">
                  <span className="stats-wpm-num">
                    {formatWpm(agg.words, agg.seconds)}
                  </span>
                  <span className="stats-wpm-unit">WPM</span>
                </span>
              </div>
              <div className="stats-row-detail">
                <span className="stats-metric">
                  {formatCount(agg.words)} words
                </span>
                <span className="stats-dot" aria-hidden="true">
                  ·
                </span>
                <span className="stats-metric">
                  {formatCount(agg.dictations)}{" "}
                  {agg.dictations === 1 ? "dictation" : "dictations"}
                </span>
                <span className="stats-dot" aria-hidden="true">
                  ·
                </span>
                <span className="stats-metric">
                  {formatDuration(agg.seconds)}
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function EmptyState() {
  return (
    <div className="stats-empty">
      <svg
        width="32"
        height="32"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M3 20h18" />
        <path d="M7 16v-5" />
        <path d="M12 16V8" />
        <path d="M17 16v-3" />
      </svg>
      <div className="stats-empty-title">No dictations yet</div>
      <div className="stats-empty-hint">
        Hold your shortcut and speak — your stats will start showing up here.
      </div>
    </div>
  );
}
