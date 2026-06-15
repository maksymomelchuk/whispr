import { ChartBarIcon } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import type { ChartConfig } from "@/components/ui/chart";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import { useConfirmAction } from "../hooks/useConfirmAction";
import {
  getAppIcon,
  getCleanupStats,
  getStats,
  clearStats as persistClearStats,
} from "../lib/api";
import type { CleanupStats, StatsRow } from "../lib/types";
import { EmptyPanel } from "./EmptyPanel";
import { InfoTip } from "./InfoTip";
import { SectionHeader } from "./SectionHeader";

type LoadState = "loading" | "ready" | "error";
export type Period = "week" | "month" | "all";

interface PeriodSpec {
  id: Period;
  label: string;
}

const PERIOD_SPECS: PeriodSpec[] = [
  { id: "week", label: "Week" },
  { id: "month", label: "Month" },
  { id: "all", label: "All Time" },
];

const TYPING_WPM_BASELINE = 40;

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

interface AppEntry {
  bundleId: string;
  name: string;
  count: number;
}

interface CleanupTokens {
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

function collectAppStats(rows: StatsRow[], period: Period): AppEntry[] {
  const cutoff = dateCutoff(period);
  const map = new Map<string, { name: string; count: number }>();
  for (const r of rows) {
    if (cutoff !== null && r.date < cutoff) continue;
    for (const [bundleId, usage] of Object.entries(r.app_counts ?? {})) {
      const existing = map.get(bundleId);
      if (existing) {
        existing.count += usage.count;
      } else {
        map.set(bundleId, { name: usage.name, count: usage.count });
      }
    }
  }
  return Array.from(map.entries())
    .map(([bundleId, { name, count }]) => ({ bundleId, name, count }))
    .sort((a, b) => b.count - a.count);
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

function xTickInterval(
  dataLength: number,
  period: Period,
): number | "preserveStartEnd" {
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

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const minutes = Math.round(seconds / 60);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
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

interface StatsTabProps {
  period: Period;
}

export function StatsTab({ period }: StatsTabProps) {
  const [rows, setRows] = useState<StatsRow[]>([]);
  const [cleanup, setCleanup] = useState<CleanupStats | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);

  const agg = useMemo(() => aggregateRows(rows, period), [rows, period]);
  const appStats = useMemo(() => collectAppStats(rows, period), [rows, period]);
  const chartData = useMemo(() => buildChartData(rows, period), [rows, period]);

  const cleanupTokens: CleanupTokens | null = useMemo(() => {
    if (!cleanup) return null;
    if (period === "week")
      return {
        input: cleanup.week.input_tokens,
        output: cleanup.week.output_tokens,
      };
    if (period === "month")
      return {
        input: cleanup.month.input_tokens,
        output: cleanup.month.output_tokens,
      };
    return {
      input: cleanup.overall.input_tokens,
      output: cleanup.overall.output_tokens,
    };
  }, [cleanup, period]);

  const { confirming: confirmingClear, trigger: handleClear } =
    useConfirmAction(async () => {
      try {
        await persistClearStats();
        setRows([]);
        setCleanup(null);
      } catch (e) {
        console.error("clear stats failed", e);
      }
    });

  const refresh = () => {
    Promise.all([getStats(), getCleanupStats()])
      .then(([statRows, cs]) => {
        setRows(statRows);
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
    <section className="flex flex-col gap-5">
      {hasAny && (
        <div className="flex items-center justify-end">
          <Button
            variant={confirmingClear ? "destructive" : "ghost"}
            size="xs"
            onClick={handleClear}
          >
            {confirmingClear ? "Click to confirm" : "Clear stats"}
          </Button>
        </div>
      )}

      {!hasAny && <EmptyState />}

      {hasAny && (
        <>
          <StatSummary
            words={formatCount(agg.words)}
            timeSaved={formatTimeSaved(agg.words, agg.seconds)}
            dictationTime={formatDuration(agg.seconds)}
            wpm={formatWpm(agg.words, agg.seconds)}
            dictations={agg.dictations}
          />

          <div>
            <p className="mb-3 text-[13px] font-semibold text-foreground">
              Activity
            </p>
            <ActivityChart data={chartData} period={period} />
          </div>
        </>
      )}

      {appStats.length > 0 && (
        <>
          <SectionHeader title="Apps used" />
          <AppsUsed apps={appStats} />
        </>
      )}

      {hasCleanup && (
        <>
          <SectionHeader
            title="AI Cleanup"
            badge={
              <InfoTip text="Tokens sent to and received from your AI cleanup provider." />
            }
          />
          {cleanupTokens && <CleanupRow tokens={cleanupTokens} />}
        </>
      )}
    </section>
  );
}

export function PeriodToggle({
  value,
  onChange,
}: {
  value: Period;
  onChange: (p: Period) => void;
}) {
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

interface StatSummaryProps {
  words: string;
  timeSaved: string;
  dictationTime: string;
  wpm: string;
  dictations: number;
}

function StatSummary({
  words,
  timeSaved,
  dictationTime,
  wpm,
  dictations,
}: StatSummaryProps) {
  return (
    <div className="flex flex-wrap items-baseline gap-x-6 gap-y-3 px-1 py-1">
      <div className="flex items-baseline gap-1.5">
        <span className="whitespace-nowrap text-[28px] font-semibold tabular-nums leading-none text-foreground">
          {words}
        </span>
        <span className="text-[12px] text-muted-foreground">words</span>
      </div>

      <span className="text-border select-none text-base">·</span>

      <div className="flex items-baseline gap-1.5">
        <span className="whitespace-nowrap text-[28px] font-semibold tabular-nums leading-none text-foreground">
          {timeSaved}
        </span>
        <span className="text-[12px] text-muted-foreground">saved</span>
      </div>

      <div className="ml-auto flex items-center gap-4 text-[12px] text-muted-foreground">
        <span className="whitespace-nowrap">
          <span className="font-medium tabular-nums text-foreground">
            {dictationTime}
          </span>{" "}
          spoken
        </span>
        <span className="whitespace-nowrap">
          <span className="font-medium tabular-nums text-foreground">
            {wpm}
          </span>{" "}
          WPM avg
        </span>
        <span className="whitespace-nowrap">
          <span className="font-medium tabular-nums text-foreground">
            {formatCount(dictations)}
          </span>{" "}
          {dictations === 1 ? "dictation" : "dictations"}
        </span>
      </div>
    </div>
  );
}

function ActivityChart({
  data,
  period,
}: {
  data: ChartPoint[];
  period: Period;
}) {
  const interval = xTickInterval(data.length, period);
  return (
    <ChartContainer config={chartConfig} className="h-[180px] w-full">
      <BarChart data={data} barCategoryGap="30%">
        <CartesianGrid
          vertical={false}
          strokeDasharray="3 3"
          className="stroke-border"
        />
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
          isAnimationActive={false}
          content={
            <ChartTooltipContent
              formatter={(value) => [formatCount(value as number), " words"]}
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

function AppsUsed({ apps }: { apps: AppEntry[] }) {
  const fetchedRef = useRef(new Set<string>());
  const [icons, setIcons] = useState<Record<string, string | null>>({});

  useEffect(() => {
    for (const { bundleId } of apps) {
      if (fetchedRef.current.has(bundleId)) continue;
      fetchedRef.current.add(bundleId);
      getAppIcon(bundleId)
        .then((url) =>
          setIcons((prev) => ({ ...prev, [bundleId]: url ?? null })),
        )
        .catch(() => setIcons((prev) => ({ ...prev, [bundleId]: null })));
    }
  }, [apps]);

  return (
    <ul className="m-0 flex list-none flex-wrap items-center gap-2 p-0">
      {apps.map((app) => (
        <AppBadge key={app.bundleId} app={app} icon={icons[app.bundleId]} />
      ))}
    </ul>
  );
}

function AppBadge({
  app,
  icon,
}: {
  app: AppEntry;
  icon: string | null | undefined;
}) {
  const noun = app.count === 1 ? "dictation" : "dictations";
  const label = `${app.name} · ${formatCount(app.count)} ${noun}`;
  return (
    <li>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            tabIndex={0}
            aria-label={label}
            className="inline-flex rounded-[8px] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
          >
            <AppIcon name={app.name} src={icon} />
          </span>
        </TooltipTrigger>
        <TooltipContent>
          {app.name}
          <span aria-hidden="true" className="mx-1.5 opacity-50">
            ·
          </span>
          <span className="tabular-nums opacity-75">
            {formatCount(app.count)} {noun}
          </span>
        </TooltipContent>
      </Tooltip>
    </li>
  );
}

function AppIcon({
  name,
  src,
}: {
  name: string;
  src: string | null | undefined;
}) {
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    setLoaded(false);
  }, [src]);

  return (
    <div className="relative size-7 shrink-0">
      <div
        className={cn(
          "absolute inset-0 flex items-center justify-center rounded-[6px] bg-muted transition-opacity duration-150",
          loaded && src ? "opacity-0" : "opacity-100",
        )}
      >
        <span className="select-none text-[11px] font-medium text-muted-foreground">
          {name[0]?.toUpperCase() ?? "?"}
        </span>
      </div>
      {src && (
        <img
          src={src}
          alt={name}
          draggable={false}
          onLoad={() => setLoaded(true)}
          className={cn(
            "absolute inset-0 size-7 select-none object-contain transition-opacity duration-150",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      )}
    </div>
  );
}

function CleanupRow({ tokens }: { tokens: CleanupTokens }) {
  const total = tokens.input + tokens.output;
  return (
    <div className="flex items-baseline justify-between gap-3 overflow-hidden rounded-lg border border-border bg-card px-4 py-3.5">
      <span className="flex items-baseline gap-1.5 text-xs tabular-nums text-muted-foreground">
        <span className="whitespace-nowrap">
          {formatCount(tokens.input)} input
        </span>
        <span
          aria-hidden="true"
          className="select-none text-muted-foreground/70"
        >
          ·
        </span>
        <span className="whitespace-nowrap">
          {formatCount(tokens.output)} output
        </span>
      </span>
      <span className="inline-flex shrink-0 items-baseline gap-1 tabular-nums">
        <span className="text-lg font-semibold leading-none text-foreground">
          {formatCount(total)}
        </span>
        <span className="text-eyebrow uppercase text-muted-foreground/70">
          tokens
        </span>
      </span>
    </div>
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
