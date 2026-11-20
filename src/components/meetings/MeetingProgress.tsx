import { CheckIcon } from "@phosphor-icons/react";

import type { MeetingMeta } from "@/lib/types";
import { cn } from "@/lib/utils";

import {
  formatMeetingDuration,
  meetingStoppedAtMs,
  MS_PER_SECOND,
} from "./meetingFormat";
import { useElapsedMs } from "./useElapsedMs";

const PANEL_CLASS =
  "flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border/80 bg-card/30 px-6 py-14 text-center";

export function RecordingPlaceholder({ meeting }: { meeting: MeetingMeta }) {
  const elapsedMs = useElapsedMs(meeting.started_at * MS_PER_SECOND);
  return (
    <div className={PANEL_CLASS}>
      <span className="relative flex size-3">
        <span className="absolute inline-flex size-full animate-ping rounded-full bg-destructive/60" />
        <span className="relative inline-flex size-3 rounded-full bg-destructive" />
      </span>
      <p className="text-sm">
        <span className="font-semibold">Recording</span>
        <span className="text-muted-foreground"> · </span>
        <span className="font-mono tabular-nums">
          {formatMeetingDuration(elapsedMs)}
        </span>
        <span className="text-muted-foreground">
          {" "}
          — notes appear after you stop
        </span>
      </p>
    </div>
  );
}

// A bordered square spins exactly about its centre; the icon glyph we tried
// first visibly wobbled up and down under the same rotation.
function Spinner({ className }: { className: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "block shrink-0 animate-spin rounded-full border-muted-foreground/25 border-t-muted-foreground",
        className,
      )}
    />
  );
}

type StepState = "done" | "active" | "pending";

function ProgressStep({ label, state }: { label: string; state: StepState }) {
  return (
    <span
      className={cn(
        "flex items-center gap-1.5 text-sm",
        state === "pending" ? "text-muted-foreground" : "text-foreground",
        state === "active" && "font-medium",
      )}
    >
      {state === "done" && (
        <CheckIcon weight="bold" className="size-3.5 text-primary" />
      )}
      {state === "active" && <Spinner className="size-3.5 border-2" />}
      {state === "pending" && (
        <span className="size-1.5 rounded-full bg-muted-foreground/40" />
      )}
      {label}
    </span>
  );
}

export function ProcessingProgress({ meeting }: { meeting: MeetingMeta }) {
  const elapsedMs = useElapsedMs(meetingStoppedAtMs(meeting));
  const transcribing = meeting.status.kind === "transcribing";
  return (
    <div className={PANEL_CLASS}>
      <Spinner className="size-7 border-[3px]" />
      <div role="status" className="flex items-center gap-2">
        <ProgressStep
          label="Transcribe"
          state={transcribing ? "active" : "done"}
        />
        <span className="text-muted-foreground" aria-hidden="true">
          →
        </span>
        <ProgressStep
          label="Notes"
          state={transcribing ? "pending" : "active"}
        />
      </div>
      <span className="font-mono text-xs tabular-nums text-muted-foreground">
        {formatMeetingDuration(elapsedMs)}
      </span>
    </div>
  );
}
