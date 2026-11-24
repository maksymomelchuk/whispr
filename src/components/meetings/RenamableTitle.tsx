import { PencilSimpleIcon } from "@phosphor-icons/react";
import { useState, type KeyboardEvent } from "react";
import { toast } from "sonner";

import { Input } from "@/components/ui/input";
import { renameMeeting } from "@/lib/api";

interface RenamableTitleProps {
  meetingId: string;
  title: string;
}

export function RenamableTitle({ meetingId, title }: RenamableTitleProps) {
  const [editing, setEditing] = useState(false);

  if (editing) {
    const save = (value: string) => {
      setEditing(false);
      const trimmed = value.trim();
      if (trimmed === "" || trimmed === title) return;
      renameMeeting(meetingId, trimmed).catch((err) =>
        toast.error(`Rename failed: ${String(err)}`),
      );
    };
    const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.nativeEvent.isComposing) return;
      if (event.key === "Enter") save(event.currentTarget.value);
      if (event.key === "Escape") setEditing(false);
    };
    return (
      <Input
        autoFocus
        defaultValue={title}
        aria-label="Meeting title"
        onKeyDown={onKeyDown}
        onBlur={() => setEditing(false)}
        onFocus={(event) => event.currentTarget.select()}
        className="h-8 text-lg font-semibold"
      />
    );
  }

  return (
    <h2 className="text-lg font-semibold">
      <button
        type="button"
        onClick={() => setEditing(true)}
        title="Rename meeting"
        className="group inline-flex items-center gap-2 rounded-md text-left hover:text-foreground/80"
      >
        {title}
        <PencilSimpleIcon className="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" />
      </button>
    </h2>
  );
}
