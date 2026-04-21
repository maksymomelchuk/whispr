import type { Shortcut } from '../lib/types'
import { formatShortcut } from '../lib/api'

interface Props {
  shortcut: Shortcut
  onStartRecord: () => void
}

export function ShortcutField({ shortcut, onStartRecord }: Props) {
  return (
    <section className="card">
      <h2>Dictation Shortcut</h2>
      <p className="hint">Hold this key to record. Release to transcribe and paste.</p>
      <div className="row">
        <div className="shortcut-display">{formatShortcut(shortcut)}</div>
        <button onClick={onStartRecord}>Record new</button>
      </div>
    </section>
  )
}
