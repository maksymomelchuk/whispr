import { ChartBarIcon } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useMemo, useState, useEffect } from "react";
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import type { ChartConfig } from "@/components/ui/chart";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import { useConfirmAction } from "../hooks/useConfirmAction";
import {
  getCleanupStats,
  getHistory,
  getStats,
  clearStats as persistClearStats,
} from "../lib/api";
import type { CleanupStats, HistoryEntry, StatsRow } from "../lib/types";
import { EmptyPanel } from "./EmptyPanel";
import { InfoTip } from "./InfoTip";
import { SectionHeader } from "./SectionHeader";

type LoadState = "loading" | "ready" | "error";
type Period = "week" | "month" | "all";

interface PeriodSpec {
  id: Period;
  label: string;
  days: number | null;
}

const PERIOD_SPECS: PeriodSpec[] = [
  { id: "week", label: "Week", days: 7 },
  { id: "month", label: "Month", days: 30 },
  { id: "all", label: "All Time", days: null },
];

const TYPING_WPM_BASELINE = 45;

const HAIKU_INPUT_PER_MTOK_USD = 1;
const HAIKU_OUTPUT_PER_MTOK_USD = 5;

const chartConfig = {
  words: { label: "Words", color: "var(--color-primary)" },
} satisfies ChartConfig;

interface ChartPoint {
  date: string;
  words: number;
}

interface Aggregate {
  words: number;
  dictations: number;
  seconds: number;
}

interface CleanupRowSpec {
  label: string;
  input: number;
  output: number;
}

function localDateISO(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function dateCutoff(period: Period): string | null {
  if (period === "all") return null;
  const days = period === "week" ? 7 : 30;
  const d = new Date();
  d.setDate(d.getDate() - (days - 1));
  return localDateISO(d);
}

function timestampCutoff(period: Period): number | null {
  if (period === "all") return null;
  const days = period === "week" ? 7 : 30;
  return Math.floor(Date.now() / 1000) - days * 86400;
}

function aggregateRows(rows: StatsRow[], period: Period): Aggregate {
  const cutoff = dateCutoff(period);
  const agg: Aggregate = { words: 0, dictations: 0, seconds: 0 };
  for (const r of rows) {
    if (cutoff !== null && r.date < cutoff) continue;
    agg.words += r.words;
    agg.dictations += r.dictations;
    agg.seconds += r.total_seconds;
  }
  return agg;
}

function countUniqueApps(entries: HistoryEntry[], period: Period): number {
  const cutoff = timestampCutoff(period);
  const ids = new Set<string>();
  for (const e of entries) {
    if (cutoff !== null && e.timestamp < cutoff) continue;
    if (e.bundle_id) ids.add(e.bundle_id);
  }
  return ids.size;
}

function buildChartData(rows: StatsRow[], period: Period): ChartPoint[] {
  if (period === "all") {
    return rows.map((r) => ({ date: r.date, words: r.words }));
  }
  const days = period === "week" ? 7 : 30;
  const rowMap = new Map(rows.map((r) => [r.date, r.words]));
  const result: ChartPoint[] = [];
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    const date = localDateISO(d);
    result.push({ date, words: rowMap.get(date) ?? 0 });
  }
  return result;
}

function formatXTick(dateStr: string, period: Period): string {
  const d = new Date(dateStr + "T00:00:00");
  if (period === "week") {
    return d.toLocaleDateString("en-US", { weekday: "short" });
  }
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

function xTickInterval(dataLength: number, period: Period): number | "preserveStartEnd" {
  if (period === "week") return 0;
  if (period === "month") return 4;
  if (dataLength <= 60) return 6;
  return Math.floor(dataLength / 8);
}

function formatTimeSaved(words: number, seconds: number): string {
  const savedMinutes = words / TYPING_WPM_BASELINE - seconds / 60;
  if (savedMinutes < 1) return "0m";
  const h = Math.floor(savedMinutes / 60);
  const m = Math.round(savedMinutes % 60);
  if (h === 0) return `${m}m`;
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

function formatWpm(words: number, seconds: number): string {
  if (seconds < 5) return "—";
  return String(Math.round((words / seconds) * 60));
}

function formatCount(n: number): string {
  return n.toLocaleString();
}

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

export function StatsTab() {
  const [period, setPeriod] = useState<Period>("week");
  const [rows, setRows] = useState<StatsRow[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [cleanup, setCleanup] = useState<CleanupStats | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);

  const agg = useMemo(() => aggregateRows(rows, period), [rows, period]);
  const appsUsed = useMemo(
    () => countUniqueApps(history, period),
    [history, period],
  );
  const chartData = useMemo(() => buildChartData(rows, period), [rows, period]);

  const cleanupRows: CleanupRowSpec[] = useMemo(() => {
    if (!cleanup) return [];
    return [
      { label: "Today", input: cleanup.today.input_tokens, output: cleanup.today.output_tokens },
      { label: "This month", input: cleanup.month.input_tokens, output: cleanup.month.output_tokens },
      { label: "Overall", input: cleanup.overall.input_tokens, output: cleanup.overall.output_tokens },
    ];
  }, [cleanup]);

  const { confirming: confirmingClear, trigger: handleClear } =
    useConfirmAction(async () => {
      try {
        await persistClearStats();
        setRows([]);
        setHistory([]);
        setCleanup(null);
      } catch (e) {
        console.error("clear stats failed", e);
      }
    });

  const refresh = () => {
    Promise.all([getStats(), getHistory(), getCleanupStats()])
      .then(([statRows, historyEntries, cs]) => {
        setRows(statRows);
        setHistory(historyEntries);
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
    attach("history-updated");
    return () => {
      cancelled = true;
      unsubs.forEach((u) => u());
    };
  }, []);

  if (loadState === "loading") {
    return <div className="py-10 text-center text-muted-foreground">Loading…</div>;
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
    <section className="flex flex-col gap-5">
      <SectionHeader
        title="Dictation stats"
        badge={<InfoTip text="Words dictated and your effective words per minute." />}
        control={
          <div className="flex items-center gap-2">
            {hasAny && (
              <Button
                variant={confirmingClear ? "destructive" : "ghost"}
                size="xs"
                onClick={handleClear}
              >
                {confirmingClear ? "Click to confirm" : "Clear stats"}
              </Button>
            )}
            <PeriodToggle value={period} onChange={setPeriod} />
          </div>
        }
      />

      {!hasAny && <EmptyState />}

      {hasAny && (
        <>
          <KpiGrid
            words={agg.words}
            wpm={formatWpm(agg.words, agg.seconds)}
            appsUsed={appsUsed}
            timeSaved={formatTimeSaved(agg.words, agg.seconds)}
          />

          <div className="rounded-lg border border-border bg-card p-4">
            <p className="mb-3 text-[13px] font-semibold text-foreground">Activity</p>
            <ActivityChart data={chartData} period={period} />
          </div>
        </>
      )}

      {hasCleanup && (
        <>
          <SectionHeader
            title="AI Cleanup"
            badge={<InfoTip text="Anthropic Claude Haiku 4.5 token usage and estimated cost." />}
          />
          <ul className="m-0 list-none overflow-hidden rounded-lg border border-border bg-card p-0">
            {cleanupRows.map((row) => (
              <CleanupRow key={row.label} spec={row} />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

function PeriodToggle({ value, onChange }: { value: Period; onChange: (p: Period) => void }) {
  return (
    <div className="flex overflow-hidden rounded-md border border-border bg-card">
      {PERIOD_SPECS.map((spec) => (
        <button
          key={spec.id}
          onClick={() => onChange(spec.id)}
          className={cn(
            "px-3 py-1 text-xs font-medium transition-colors",
            value === spec.id
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground",
          )}
        >
          {spec.label}
        </button>
      ))}
    </div>
  );
}

interface KpiGridProps {
  words: number;
  wpm: string;
  appsUsed: number;
  timeSaved: string;
}

function KpiGrid({ words, wpm, appsUsed, timeSaved }: KpiGridProps) {
  return (
    <div className="grid grid-cols-4 overflow-hidden rounded-lg border border-border bg-card">
      <KpiCard value={formatCount(words)} label="Words" />
      <KpiCard value={wpm} label="Avg WPM" />
      <KpiCard value={String(appsUsed)} label="Apps Used" />
      <KpiCard value={timeSaved} label="Time Saved" />
    </div>
  );
}

function KpiCard({ value, label }: { value: string; label: string }) {
  return (
    <div className="flex flex-col items-center gap-1 px-4 py-4 [&+div]:border-l [&+div]:border-border">
      <span className="text-2xl font-bold tabular-nums leading-none text-foreground">
        {value}
      </span>
      <span className="text-[11px] text-muted-foreground">{label}</span>
    </div>
  );
}

function ActivityChart({ data, period }: { data: ChartPoint[]; period: Period }) {
  const interval = xTickInterval(data.length, period);
  return (
    <ChartContainer config={chartConfig} className="h-[180px] w-full">
      <BarChart data={data} barCategoryGap="30%">
        <CartesianGrid vertical={false} strokeDasharray="3 3" className="stroke-border" />
        <XAxis
          dataKey="date"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          interval={interval}
          tick={{ fontSize: 11 }}
          tickFormatter={(d) => formatXTick(d, period)}
        />
        <YAxis
          tickLine={false}
          axisLine={false}
          tickMargin={4}
          tick={{ fontSize: 11 }}
          width={40}
          allowDecimals={false}
        />
        <ChartTooltip
          content={
            <ChartTooltipContent
              formatter={(value) => [formatCount(value as number), "words"]}
              labelFormatter={(label) =>
                new Date(label + "T00:00:00").toLocaleDateString("en-US", {
                  month: "long",
                  day: "numeric",
                })
              }
            />
          }
        />
        <Bar dataKey="words" fill="var(--color-words)" radius={[3, 3, 0, 0]} />
      </BarChart>
    </ChartContainer>
  );
}

function CleanupRow({ spec }: { spec: CleanupRowSpec }) {
  const cost = estimateCostUsd(spec.input, spec.output);
  return (
    <li className="flex items-baseline justify-between gap-3 px-4 py-3.5 [&+li]:border-t [&+li]:border-border">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
        <span className="text-[13px] font-medium text-foreground">{spec.label}</span>
        <span className="flex items-baseline gap-1.5 text-xs tabular-nums text-muted-foreground">
          <span className="whitespace-nowrap">{formatCount(spec.input)} input</span>
          <span aria-hidden="true" className="select-none text-muted-foreground/70">·</span>
          <span className="whitespace-nowrap">{formatCount(spec.output)} output</span>
        </span>
      </div>
      <span className="inline-flex shrink-0 items-baseline gap-1 tabular-nums">
        <span className="text-lg font-semibold leading-none text-foreground">
          {formatCost(cost)}
        </span>
        <span className="text-eyebrow uppercase text-muted-foreground/70">est.</span>
      </span>
    </li>
  );
}

function EmptyState() {
  return (
    <EmptyPanel
      icon={<ChartBarIcon size={32} />}
      title="No dictations yet"
      hint="Hold your shortcut and speak — your stats will start showing up here."
    />
  );
}
