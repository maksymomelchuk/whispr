import { formatClock } from "@/lib/formatDuration";
import type { MeetingStatus } from "@/lib/types";

const MS_PER_SECOND = 1000;

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
  return formatClock(startMs / MS_PER_SECOND);
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
