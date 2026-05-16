import { formatShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";
import { SectionCard } from "./SectionCard";
import { Button } from "./ui/button";

interface Props {
  shortcut: Shortcut;
  onStartRecord: () => void;
}

export function ShortcutField({ shortcut, onStartRecord }: Props) {
  return (
    <SectionCard
      title="Dictation Shortcut"
      info="Hold this key to record. Release to transcribe and paste."
    >
      <div className="flex items-center gap-2">
        <div className="flex-1 px-2.5 py-1.5 text-sm font-mono border border-input rounded-md bg-secondary text-foreground">
          {formatShortcut(shortcut)}
        </div>
        <Button variant="outline" size="sm" onClick={onStartRecord}>
          Record new
        </Button>
      </div>
    </SectionCard>
  );
}
