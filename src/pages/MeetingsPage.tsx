import { UsersThreeIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { toast } from "sonner";

import { EmptyPanel } from "@/components/EmptyPanel";
import { MeetingDetail } from "@/components/meetings/MeetingDetail";
import { MeetingList } from "@/components/meetings/MeetingList";
import { MeetingRecorderCard } from "@/components/meetings/MeetingRecorderCard";
import { PageShell } from "@/components/PageShell";
import { useMeetings } from "@/hooks/useMeetings";
import { deleteMeeting } from "@/lib/api";

export function MeetingsPage() {
  const { meetings: allMeetings, refresh } = useMeetings();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // Hidden before the delete call: the backend's update event lands while the
  // detail is still mounted and would re-fetch a meeting that no longer exists.
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const meetings = allMeetings.filter((m) => m.id !== deletingId);
  const recording = meetings.find((m) => m.status.kind === "recording") ?? null;
  const shownId = selectedId ?? meetings[0]?.id ?? null;

  return (
    <PageShell
      title="Meetings"
      description="Record a call, then get a transcript and notes with decisions and action items."
    >
      <MeetingRecorderCard recording={recording} />
      {meetings.length === 0 ? (
        <EmptyPanel
          icon={<UsersThreeIcon size={28} />}
          title="No meetings yet"
          hint="Start a meeting when your call begins and stop it when it ends."
        />
      ) : (
        <div className="grid gap-6 md:grid-cols-[220px_1fr]">
          <MeetingList
            meetings={meetings}
            selectedId={shownId}
            onSelect={setSelectedId}
          />
          {shownId && (
            <MeetingDetail
              key={shownId}
              id={shownId}
              onDelete={(id) => {
                setDeletingId(id);
                setSelectedId(null);
                deleteMeeting(id)
                  .then(refresh)
                  .catch((err) => {
                    setDeletingId(null);
                    toast.error(`Failed to delete meeting: ${String(err)}`);
                  });
              }}
            />
          )}
        </div>
      )}
    </PageShell>
  );
}
