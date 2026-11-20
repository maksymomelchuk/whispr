import { CopyIcon, TrashIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useMeetingsUpdated } from "@/hooks/useMeetings";
import { deleteMeeting, getMeeting, retryMeeting } from "@/lib/api";
import type { Meeting, MeetingSegment } from "@/lib/types";

import { formatMeetingStart, formatSegmentTime } from "./meetingFormat";

type View = "notes" | "transcript";

function useMeeting(id: string) {
  const [meeting, setMeeting] = useState<Meeting | null>(null);
  const refresh = useCallback(() => {
    getMeeting(id)
      .then(setMeeting)
      .catch((err) => toast.error(`Failed to load meeting: ${String(err)}`));
  }, [id]);
  useEffect(refresh, [refresh]);
  useMeetingsUpdated(refresh);
  return meeting;
}

function Transcript({ segments }: { segments: MeetingSegment[] }) {
  return (
    <ol className="flex flex-col gap-2 text-sm">
      {segments.map((segment) => (
        <li
          key={`${segment.start_ms}-${segment.speaker}`}
          className="flex gap-3"
        >
          <span className="w-12 shrink-0 font-mono text-xs text-muted-foreground tabular-nums">
            {formatSegmentTime(segment.start_ms)}
          </span>
          <span>
            <span className="font-semibold">{segment.speaker}: </span>
            {segment.text}
          </span>
        </li>
      ))}
    </ol>
  );
}

function StatusMessage({ meeting }: { meeting: Meeting }) {
  const { status } = meeting.meta;
  if (status.kind === "transcribing" || status.kind === "summarizing") {
    return (
      <p className="text-sm text-muted-foreground">
        {status.kind === "transcribing" ? "Transcribing…" : "Writing notes…"}
      </p>
    );
  }
  if (status.kind !== "failed") return null;
  const retry = () =>
    retryMeeting(meeting.meta.id).catch((err) => toast.error(String(err)));
  return (
    <Alert variant="destructive">
      <AlertDescription className="flex flex-col gap-2">
        {status.message}
        <Button size="sm" variant="outline" onClick={retry} className="w-fit">
          Retry
        </Button>
      </AlertDescription>
    </Alert>
  );
}

export function MeetingDetail({
  id,
  onDeleted,
}: {
  id: string;
  onDeleted: () => void;
}) {
  const meeting = useMeeting(id);
  const [view, setView] = useState<View>("notes");
  if (!meeting) return null;

  const copyNotes = () =>
    navigator.clipboard
      .writeText(meeting.notes ?? "")
      .then(() => toast.success("Notes copied"))
      .catch((err) => toast.error(`Copy failed: ${String(err)}`));
  const remove = () =>
    deleteMeeting(id)
      .then(onDeleted)
      .catch((err) => toast.error(String(err)));

  return (
    <section className="flex min-w-0 flex-col gap-4">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-lg font-semibold">
          {formatMeetingStart(meeting.meta.started_at)}
        </h2>
        <div className="flex gap-1">
          {(["notes", "transcript"] as const).map((option) => (
            <Button
              key={option}
              size="sm"
              variant={view === option ? "secondary" : "ghost"}
              onClick={() => setView(option)}
            >
              {option === "notes" ? "Notes" : "Transcript"}
            </Button>
          ))}
          <Button
            size="icon-sm"
            variant="ghost"
            onClick={copyNotes}
            disabled={!meeting.notes}
            aria-label="Copy notes"
          >
            <CopyIcon />
          </Button>
          <Button
            size="icon-sm"
            variant="ghost"
            onClick={remove}
            aria-label="Delete meeting"
          >
            <TrashIcon />
          </Button>
        </div>
      </header>
      <StatusMessage meeting={meeting} />
      {view === "notes" && meeting.notes && (
        <div className="whitespace-pre-wrap text-sm leading-relaxed">
          {meeting.notes}
        </div>
      )}
      {view === "transcript" && <Transcript segments={meeting.segments} />}
    </section>
  );
}
