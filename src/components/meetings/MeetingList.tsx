import { Badge } from "@/components/ui/badge";
import type { MeetingListItem } from "@/lib/types";
import { cn } from "@/lib/utils";

import {
  formatMeetingDuration,
  formatMeetingStart,
  meetingTitle,
  STATUS_BADGE_VARIANTS,
  STATUS_LABELS,
} from "./meetingFormat";

interface MeetingListProps {
  meetings: MeetingListItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function MeetingList({
  meetings,
  selectedId,
  onSelect,
}: MeetingListProps) {
  return (
    <ul className="relative flex min-h-0 flex-col gap-1 overflow-y-auto pr-1">
      {meetings.map((meeting) => (
        <li key={meeting.id}>
          <button
            type="button"
            onClick={() => onSelect(meeting.id)}
            aria-pressed={meeting.id === selectedId}
            className={cn(
              "flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-accent",
              meeting.id === selectedId && "bg-accent",
            )}
          >
            <span className="flex min-w-0 flex-col">
              <span
                className="truncate font-medium"
                title={meeting.title ?? undefined}
              >
                {meetingTitle(meeting.title, meeting.started_at)}
              </span>
              <span className="truncate text-xs text-muted-foreground tabular-nums">
                {meeting.title &&
                  `${formatMeetingStart(meeting.started_at)} · `}
                {meeting.status.kind === "recording"
                  ? "In progress"
                  : formatMeetingDuration(meeting.duration_ms)}
              </span>
            </span>
            <Badge
              variant={STATUS_BADGE_VARIANTS[meeting.status.kind]}
              className="shrink-0"
            >
              {STATUS_LABELS[meeting.status.kind]}
            </Badge>
          </button>
        </li>
      ))}
    </ul>
  );
}
