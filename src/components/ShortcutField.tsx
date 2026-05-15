import { formatShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";
import { InfoTip } from "./InfoTip";

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
        <div className="shortcut-display">{formatShortcut(shortcut)}</div>
        <button onClick={onStartRecord}>Record new</button>
      </div>
    </section>
  );
}
