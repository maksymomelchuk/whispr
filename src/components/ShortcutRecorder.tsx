import { useCallback, useEffect, useState } from 'react'
import type { Shortcut } from '../lib/types'
import { formatShortcut } from '../lib/api'

interface Props {
  initial: Shortcut
  onSave: (shortcut: Shortcut) => void
  onCancel: () => void
}

const MODIFIER_CODES = new Set([
  'MetaLeft',
  'MetaRight',
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
])

function isModifierCode(code: string): boolean {
  return MODIFIER_CODES.has(code)
}

// Translate KeyboardEvent.key modifier names to our stored modifier names.
function collectModifiers(e: KeyboardEvent): string[] {
  const mods: string[] = []
  if (e.metaKey) mods.push('Meta')
  if (e.ctrlKey) mods.push('Control')
  if (e.altKey) mods.push('Alt')
  if (e.shiftKey) mods.push('Shift')
  return mods
}

export function ShortcutRecorder({ initial, onSave, onCancel }: Props) {
  const [captured, setCaptured] = useState<Shortcut | null>(null)

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      onCancel()
      return
    }
    e.preventDefault()
    e.stopPropagation()

    const modifiers = collectModifiers(e)
    const code = e.code

    // Modifier-only shortcut (like "Right Option" alone): store the modifier
    // as the key with no additional modifiers.
    if (isModifierCode(code)) {
      setCaptured({ key: code, modifiers: [] })
      return
    }

    setCaptured({ key: code, modifiers })
  }, [onCancel])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown, { capture: true })
    return () =>
      window.removeEventListener('keydown', handleKeyDown, { capture: true })
  }, [handleKeyDown])

  const hasChanges = captured !== null

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>Record shortcut</h2>
        <p className="hint">
          Press the key combination you want to hold for dictation.
          <br />
          Esc to cancel.
        </p>
        <div className="shortcut-preview">
          {captured ? formatShortcut(captured) : 'Listening…'}
        </div>
        <p className="hint-sm">
          Current: <span className="mono">{formatShortcut(initial)}</span>
        </p>
        <div className="modal-actions">
          <button onClick={onCancel}>Cancel</button>
          <button
            disabled={!hasChanges || !captured}
            onClick={() => captured && onSave(captured)}
            className="primary"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  )
}
