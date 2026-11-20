import { UsersThreeIcon } from "@phosphor-icons/react";
import { useState } from "react";

import { EmptyPanel } from "@/components/EmptyPanel";
import { MeetingDetail } from "@/components/meetings/MeetingDetail";
import { MeetingList } from "@/components/meetings/MeetingList";
import { MeetingRecorderCard } from "@/components/meetings/MeetingRecorderCard";
import { PageShell } from "@/components/PageShell";
import { useMeetings } from "@/hooks/useMeetings";

export function MeetingsPage() {
  const { meetings, refresh } = useMeetings();
  const [selectedId, setSelectedId] = useState<string | null>(null);
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
              onDeleted={() => {
                setSelectedId(null);
                refresh();
              }}
            />
          )}
        </div>
      )}
    </PageShell>
  );
}
