import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";

import { useConfirmAction } from "../hooks/useConfirmAction";
import {
  clearStats as persistClearStats,
  getCleanupStats,
  getStats,
} from "../lib/api";
import type { CleanupStats, StatsRow } from "../lib/types";

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

// Anthropic Claude Haiku 4.5 standard rates as of 2026-01.
const HAIKU_INPUT_PER_MTOK_USD = 1;
const HAIKU_OUTPUT_PER_MTOK_USD = 5;

function estimateCostUsd(input: number, output: number): number {
  return (
    (input * HAIKU_INPUT_PER_MTOK_USD + output * HAIKU_OUTPUT_PER_MTOK_USD) /
    1_000_000
  );
}

function formatCost(cost: number): string {
  if (cost <= 0) return "$0";
  if (cost < 0.01) return "<$0.01";
  return `$${cost.toFixed(2)}`;
}

interface CleanupRowSpec {
  label: string;
  input: number;
  output: number;
}

export function StatsTab() {
  const [rows, setRows] = useState<StatsRow[]>([]);
  const [cleanup, setCleanup] = useState<CleanupStats | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);

  const aggregates = useMemo(
    () => PERIODS.map((p) => ({ spec: p, agg: aggregateFor(rows, p) })),
    [rows],
  );

  const cleanupRows: CleanupRowSpec[] = useMemo(() => {
    if (!cleanup) return [];
    return [
      {
        label: "Today",
        input: cleanup.today.input_tokens,
        output: cleanup.today.output_tokens,
      },
      {
        label: "This month",
        input: cleanup.month.input_tokens,
        output: cleanup.month.output_tokens,
      },
      {
        label: "Overall",
        input: cleanup.overall.input_tokens,
        output: cleanup.overall.output_tokens,
      },
    ];
  }, [cleanup]);

  const { confirming: confirmingClear, trigger: handleClear } = useConfirmAction(
    async () => {
      try {
        await persistClearStats();
        setRows([]);
        setCleanup(null);
      } catch (e) {
        console.error("clear stats failed", e);
      }
    },
  );

  const refresh = () => {
    Promise.all([getStats(), getCleanupStats()])
      .then(([list, cs]) => {
        setRows(list);
        setCleanup(cs);
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
    const unsubs: (() => void)[] = [];
    let cancelled = false;
    const attach = (event: string) =>
      listen(event, () => refresh())
        .then((u) => (cancelled ? u() : unsubs.push(u)))
        .catch((e) => console.error(`${event} listen failed`, e));
    attach("stats-updated");
    attach("cleanup-stats-updated");
    return () => {
      cancelled = true;
      unsubs.forEach((u) => u());
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

      {cleanup &&
        (cleanup.overall.input_tokens > 0 ||
          cleanup.overall.output_tokens > 0) && (
        <>
          <div className="stats-header">
            <div>
              <h2 className="stats-title">AI Cleanup</h2>
              <p className="hint stats-hint">
                Anthropic Claude Haiku 4.5 token usage and estimated cost.
              </p>
            </div>
          </div>
          <ul className="stats-list">
            {cleanupRows.map((row) => (
              <li key={row.label} className="stats-row">
                <div className="stats-row-head">
                  <span className="stats-row-label">{row.label}</span>
                  <span className="stats-wpm">
                    <span className="stats-wpm-num">
                      {formatCost(estimateCostUsd(row.input, row.output))}
                    </span>
                    <span className="stats-wpm-unit">est.</span>
                  </span>
                </div>
                <div className="stats-row-detail">
                  <span className="stats-metric">
                    {formatCount(row.input)} input
                  </span>
                  <span className="stats-dot" aria-hidden="true">
                    ·
                  </span>
                  <span className="stats-metric">
                    {formatCount(row.output)} output
                  </span>
                </div>
              </li>
            ))}
          </ul>
        </>
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
