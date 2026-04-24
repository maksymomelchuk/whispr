import { useEffect, useState } from 'react'
import { setReplacements as persistReplacements } from '../lib/api'
import type { Replacement } from '../lib/types'
import { CollapsibleCard } from './CollapsibleCard'

interface Props {
  initial: Replacement[]
  onSaved: (replacements: Replacement[]) => void
  defaultOpen?: boolean
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

const same = (a: Replacement[], b: Replacement[]) =>
  a.length === b.length &&
  a.every((r, i) => r.from === b[i].from && r.to === b[i].to)

export function ReplacementsField({ initial, onSaved, defaultOpen = true }: Props) {
  const [rows, setRows] = useState<Replacement[]>(initial)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setRows(initial)
  }, [initial])

  useEffect(() => {
    if (status !== 'saved') return
    const t = setTimeout(() => setStatus('idle'), 1500)
    return () => clearTimeout(t)
  }, [status])

  const updateRow = (index: number, patch: Partial<Replacement>) =>
    setRows((prev) =>
      prev.map((r, i) => (i === index ? { ...r, ...patch } : r)),
    )

  const removeRow = (index: number) =>
    setRows((prev) => prev.filter((_, i) => i !== index))

  const addRow = () => setRows((prev) => [...prev, { from: '', to: '' }])

  const handleSave = async () => {
    const cleaned = rows
      .map((r) => ({ from: r.from.trim(), to: r.to }))
      .filter((r) => r.from.length > 0)
    setStatus('saving')
    setError(null)
    try {
      await persistReplacements(cleaned)
      setRows(cleaned)
      onSaved(cleaned)
      setStatus('saved')
    } catch (e) {
      setStatus('error')
      setError(String(e))
    }
  }

  const dirty = !same(rows, initial)

  return (
    <CollapsibleCard title="Voice Replacements" defaultOpen={defaultOpen} dirty={dirty}>
      <p className="hint">
        Spoken words on the left become the text on the right. Punctuation
        like <span className="mono">.</span> <span className="mono">/</span>{' '}
        <span className="mono">-</span> is spaced intelligently — saying
        &ldquo;test dot ts&rdquo; produces <span className="mono">test.ts</span>.
      </p>
      <div className="replacements-list">
        {rows.map((row, i) => (
          <div key={i} className="replacement-row">
            <input
              type="text"
              value={row.from}
              placeholder="spoken"
              spellCheck={false}
              autoComplete="off"
              onChange={(e) => updateRow(i, { from: e.target.value })}
            />
            <span className="replacement-arrow">→</span>
            <input
              type="text"
              value={row.to}
              placeholder="text"
              spellCheck={false}
              autoComplete="off"
              onChange={(e) => updateRow(i, { to: e.target.value })}
            />
            <button
              className="icon-button"
              aria-label="Remove"
              onClick={() => removeRow(i)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <div className="row replacements-actions">
        <button onClick={addRow}>+ Add</button>
        <div className="spacer" />
        <button
          className="primary"
          onClick={handleSave}
          disabled={!dirty || status === 'saving'}
        >
          {status === 'saving' ? 'Saving…' : 'Save'}
        </button>
      </div>
      {status === 'saved' && <div className="status ok">Saved</div>}
      {status === 'error' && <div className="status err">{error}</div>}
    </CollapsibleCard>
  )
}
