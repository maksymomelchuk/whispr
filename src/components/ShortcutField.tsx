import { formatShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";

interface Props {
  shortcut: Shortcut;
  onStartRecord: () => void;
}

export function ShortcutField({ shortcut, onStartRecord }: Props) {
  return (
    <section className="card">
      <h2>Dictation Shortcut</h2>
      <p className="hint">
        Hold this key to record. Release to transcribe and paste.
      </p>
      <div className="row">
        <div className="shortcut-display">{formatShortcut(shortcut)}</div>
        <button onClick={onStartRecord}>Record new</button>
      </div>
    </section>
  );
}
