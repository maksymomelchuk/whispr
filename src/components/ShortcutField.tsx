import { formatShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";
import { InfoTip } from "./InfoTip";
import { Button } from "./ui/button";

interface Props {
  shortcut: Shortcut;
  onStartRecord: () => void;
}

export function ShortcutField({ shortcut, onStartRecord }: Props) {
  return (
    <section className="card">
      <div className="card-title-row">
        <h2 style={{ margin: 0 }}>Dictation Shortcut</h2>
        <InfoTip text="Hold this key to record. Release to transcribe and paste." />
      </div>
      <div className="row">
        <div className="flex-1 px-2.5 py-1.5 text-sm font-mono border border-input rounded-md bg-secondary text-foreground">
          {formatShortcut(shortcut)}
        </div>
        <Button variant="outline" size="sm" onClick={onStartRecord}>
          Record new
        </Button>
      </div>
    </section>
  );
}
