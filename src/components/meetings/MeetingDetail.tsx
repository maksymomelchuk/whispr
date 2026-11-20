import { useCallback, useState } from "react";
import { toast } from "sonner";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useMeetingsSubscription } from "@/hooks/useMeetings";
import { getMeeting, retryMeeting } from "@/lib/api";
import type { Meeting, MeetingStatus } from "@/lib/types";

import { CopyNotesButton, DeleteMeetingButton } from "./MeetingActions";
import { formatMeetingStart, meetingTitle } from "./meetingFormat";
import { MeetingNotes } from "./MeetingNotes";
import { ProcessingProgress, RecordingPlaceholder } from "./MeetingProgress";
import { MeetingTranscript } from "./MeetingTranscript";

type View = "notes" | "transcript";

function useMeeting(id: string) {
  const [meeting, setMeeting] = useState<Meeting | null>(null);
  const refresh = useCallback(() => {
    getMeeting(id)
      .then(setMeeting)
      .catch((err) => toast.error(`Failed to load meeting: ${String(err)}`));
  }, [id]);
  useMeetingsSubscription(refresh);
  return meeting;
}

function FailedMessage({ meeting }: { meeting: Meeting }) {
  const { status } = meeting.meta;
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

function DetailHeading({ meeting }: { meeting: Meeting }) {
  const { started_at } = meeting.meta;
  return (
    <div className="flex min-w-0 flex-col">
      <h2 className="text-lg font-semibold">
        {meetingTitle(meeting.title, started_at)}
      </h2>
      {meeting.title && (
        <span className="text-xs text-muted-foreground">
          {formatMeetingStart(started_at)}
        </span>
      )}
    </div>
  );
}

function InProgressBody({ meeting }: { meeting: Meeting }) {
  if (meeting.meta.status.kind === "recording") {
    return <RecordingPlaceholder meeting={meeting.meta} />;
  }
  return <ProcessingProgress meeting={meeting.meta} />;
}

function FinishedDetail({
  meeting,
  onDeleted,
}: {
  meeting: Meeting;
  onDeleted: () => void;
}) {
  const [view, setView] = useState<View>("notes");
  return (
    <>
      <header className="flex flex-wrap items-start justify-between gap-2">
        <DetailHeading meeting={meeting} />
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
          <CopyNotesButton notes={meeting.notes} />
          <DeleteMeetingButton
            id={meeting.meta.id}
            title={meetingTitle(meeting.title, meeting.meta.started_at)}
            onDeleted={onDeleted}
          />
        </div>
      </header>
      <FailedMessage meeting={meeting} />
      {view === "notes" && meeting.notes && (
        <MeetingNotes notes={meeting.notes} hasTitle={meeting.title !== null} />
      )}
      {view === "transcript" && <MeetingTranscript meeting={meeting} />}
    </>
  );
}

const IN_PROGRESS_STATUSES = new Set<MeetingStatus["kind"]>([
  "recording",
  "transcribing",
  "summarizing",
]);

export function MeetingDetail({
  id,
  onDeleted,
}: {
  id: string;
  onDeleted: () => void;
}) {
  const meeting = useMeeting(id);
  if (!meeting) return null;

  return (
    <section className="flex min-w-0 flex-col gap-4">
      {IN_PROGRESS_STATUSES.has(meeting.meta.status.kind) ? (
        <>
          <header>
            <DetailHeading meeting={meeting} />
          </header>
          <InProgressBody meeting={meeting} />
        </>
      ) : (
        <FinishedDetail meeting={meeting} onDeleted={onDeleted} />
      )}
    </section>
  );
}
