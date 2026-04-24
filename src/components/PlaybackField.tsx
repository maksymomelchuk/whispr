import { useState } from "react";

import { setPauseMediaOnRecord } from "../lib/api";

interface Props {
  initial: boolean;
  onSaved: (enabled: boolean) => void;
}

export function PlaybackField({ initial, onSaved }: Props) {
  const [enabled, setEnabled] = useState(initial);
  const [error, setError] = useState<string | null>(null);

  const toggle = async () => {
    const next = !enabled;
    setEnabled(next);
    setError(null);
    try {
      await setPauseMediaOnRecord(next);
      onSaved(next);
    } catch (e) {
      setEnabled(!next);
      setError(String(e));
    }
  };

  return (
    <section className="card">
      <h2>Playback</h2>
      <p className="hint">
        Control what happens to audio playing in other apps while you dictate.
      </p>
      <div className="options-list">
        <label className="option-row">
          <input type="checkbox" checked={enabled} onChange={toggle} />
          <div className="option-text">
            <div className="option-label">Pause media while recording</div>
            <div className="option-description">
              Pauses the Now Playing app (Spotify, Apple Music, YouTube, etc.)
              for the duration of a dictation, then resumes it when you release
              the shortcut.
            </div>
          </div>
        </label>
      </div>
      {error && <div className="status err">{error}</div>}
    </section>
  );
}
