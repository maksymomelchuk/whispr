import { formatClock } from "@/lib/formatDuration";
import type { MeetingStatus } from "@/lib/types";

const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

export function formatMeetingStart(startedAtSeconds: number): string {
  return new Date(startedAtSeconds * MS_PER_SECOND).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatMeetingDuration(durationMs: number): string {
  return formatClock(durationMs / MS_PER_SECOND);
}

export function formatSegmentTime(startMs: number): string {
  const totalSeconds = Math.floor(startMs / MS_PER_SECOND);
  const hours = Math.floor(totalSeconds / SECONDS_PER_HOUR);
  if (hours === 0) return formatClock(totalSeconds);
  const minutes = Math.floor(
    (totalSeconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
  );
  const seconds = totalSeconds % SECONDS_PER_MINUTE;
  return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

export const STATUS_LABELS: Record<MeetingStatus["kind"], string> = {
  recording: "Recording",
  transcribing: "Transcribing",
  summarizing: "Summarizing",
  ready: "Ready",
  failed: "Failed",
};

export const STATUS_BADGE_VARIANTS: Record<
  MeetingStatus["kind"],
  "neutral" | "accent" | "warn" | "error"
> = {
  recording: "error",
  transcribing: "warn",
  summarizing: "warn",
  ready: "accent",
  failed: "error",
};
