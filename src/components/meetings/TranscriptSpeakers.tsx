import { PencilSimpleIcon } from "@phosphor-icons/react";
import { useState, type KeyboardEvent } from "react";
import { toast } from "sonner";

import { Input } from "@/components/ui/input";
import { renameMeetingSpeaker } from "@/lib/api";

import { speakerName } from "./transcriptLines";

interface TranscriptSpeakersProps {
  meetingId: string;
  labels: string[];
  speakerNames: Record<string, string>;
}

export function TranscriptSpeakers({
  meetingId,
  labels,
  speakerNames,
}: TranscriptSpeakersProps) {
  if (labels.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5 text-xs">
      <span className="text-muted-foreground">Speakers:</span>
      {labels.map((label) => (
        <SpeakerChip
          key={label}
          meetingId={meetingId}
          label={label}
          name={speakerName(label, speakerNames)}
        />
      ))}
    </div>
  );
}

function SpeakerChip({
  meetingId,
  label,
  name,
}: {
  meetingId: string;
  label: string;
  name: string;
}) {
  const [editing, setEditing] = useState(false);

  if (editing) {
    const save = (value: string) => {
      setEditing(false);
      if (value.trim() === name) return;
      renameMeetingSpeaker(meetingId, label, value).catch((err) =>
        toast.error(`Rename failed: ${String(err)}`),
      );
    };
    const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.key === "Enter") save(event.currentTarget.value);
      if (event.key === "Escape") setEditing(false);
    };
    return (
      <Input
        autoFocus
        defaultValue={name === label ? "" : name}
        placeholder={label}
        aria-label={`Name for ${label}`}
        onKeyDown={onKeyDown}
        onBlur={() => setEditing(false)}
        className="h-6 w-36 text-xs"
      />
    );
  }

  return (
    <button
      type="button"
      onClick={() => setEditing(true)}
      title={`Rename ${label}`}
      className="inline-flex items-center gap-1 rounded-md border px-2 py-0.5 hover:bg-muted"
    >
      <span className="font-medium">{name}</span>
      {name !== label && (
        <span className="text-muted-foreground">({label})</span>
      )}
      <PencilSimpleIcon className="size-3 text-muted-foreground" />
    </button>
  );
}
