import { listen } from "@tauri-apps/api/event";
import { ChartBar } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useConfirmAction } from "../hooks/useConfirmAction";
import {
  clearStats as persistClearStats,
  getCleanupStats,
  getStats,
} from "../lib/api";
import type { CleanupStats, StatsRow } from "../lib/types";
import { EmptyPanel } from "./EmptyPanel";
import { InfoTip } from "./InfoTip";
import { SectionHeader } from "./SectionHeader";

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
    return (
      <div className="py-10 text-center text-muted-foreground">Loading…</div>
    );
  }

  if (loadState === "error") {
    return (
      <Alert variant="destructive">
        <AlertDescription>Failed to load stats: {loadError}</AlertDescription>
      </Alert>
    );
  }

  const hasAny = rows.length > 0;
  const hasCleanup =
    cleanup !== null &&
    (cleanup.overall.input_tokens > 0 || cleanup.overall.output_tokens > 0);

  return (
    <section className="flex flex-col gap-3.5">
      <SectionHeader
        title="Dictation stats"
        badge={<InfoTip text="Words dictated and your effective words per minute." />}
        control={hasAny ? (
          <Button
            variant={confirmingClear ? "destructive" : "ghost"}
            size="sm"
            onClick={handleClear}
          >
            {confirmingClear ? "Click to confirm" : "Clear stats"}
          </Button>
        ) : undefined}
      />

      {!hasAny && <EmptyState />}
      {hasAny && (
        <ul className="m-0 list-none overflow-hidden rounded-lg border border-border bg-card p-0">
          {aggregates.map(({ spec, agg }) => (
            <StatRow
              key={spec.label}
              label={spec.label}
              metric={
                <>
                  <span className="text-lg font-semibold leading-none text-foreground">
                    {formatWpm(agg.words, agg.seconds)}
                  </span>
                  <span className="text-eyebrow uppercase text-muted-foreground/70">
                    WPM
                  </span>
                </>
              }
              detail={
                <>
                  <Metric>{formatCount(agg.words)} words</Metric>
                  <Dot />
                  <Metric>
                    {formatCount(agg.dictations)}{" "}
                    {agg.dictations === 1 ? "dictation" : "dictations"}
                  </Metric>
                  <Dot />
                  <Metric>{formatDuration(agg.seconds)}</Metric>
                </>
              }
            />
          ))}
        </ul>
      )}

      {hasCleanup && (
        <>
          <SectionHeader
            title="AI Cleanup"
            badge={<InfoTip text="Anthropic Claude Haiku 4.5 token usage and estimated cost." />}
          />
          <ul className="m-0 list-none overflow-hidden rounded-lg border border-border bg-card p-0">
            {cleanupRows.map((row) => (
              <StatRow
                key={row.label}
                label={row.label}
                metric={
                  <>
                    <span className="text-lg font-semibold leading-none text-foreground">
                      {formatCost(estimateCostUsd(row.input, row.output))}
                    </span>
                    <span className="text-eyebrow uppercase text-muted-foreground/70">
                      est.
                    </span>
                  </>
                }
                detail={
                  <>
                    <Metric>{formatCount(row.input)} input</Metric>
                    <Dot />
                    <Metric>{formatCount(row.output)} output</Metric>
                  </>
                }
              />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

interface StatRowProps {
  label: string;
  metric: ReactNode;
  detail: ReactNode;
}

function StatRow({ label, metric, detail }: StatRowProps) {
  return (
    <li className="flex flex-col gap-1 px-4 py-3.5 [&+li]:border-t [&+li]:border-border">
      <div className="flex items-baseline justify-between gap-3">
        <span className="text-[13px] font-medium text-foreground">{label}</span>
        <span className="inline-flex items-baseline gap-1 tabular-nums">
          {metric}
        </span>
      </div>
      <div className="flex flex-wrap items-baseline gap-1.5 text-xs tabular-nums text-muted-foreground">
        {detail}
      </div>
    </li>
  );
}

function Metric({ children }: { children: ReactNode }) {
  return <span className="whitespace-nowrap">{children}</span>;
}

function Dot() {
  return (
    <span aria-hidden="true" className="select-none text-muted-foreground/70">
      ·
    </span>
  );
}

function EmptyState() {
  return (
    <EmptyPanel
      icon={<ChartBar size={32} />}
      title="No dictations yet"
      hint="Hold your shortcut and speak — your stats will start showing up here."
    />
  );
}
