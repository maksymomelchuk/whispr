import { useEffect, useState } from 'react'
import { setApiKey as persistApiKey } from '../lib/api'

interface Props {
  initialValue: string
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export function ApiKeyField({ initialValue }: Props) {
  const [value, setValue] = useState(initialValue)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setValue(initialValue)
  }, [initialValue])

  useEffect(() => {
    if (status !== 'saved') return
    const t = setTimeout(() => setStatus('idle'), 1500)
    return () => clearTimeout(t)
  }, [status])

  const handleSave = async () => {
    setStatus('saving')
    setError(null)
    try {
      await persistApiKey(value.trim())
      setStatus('saved')
    } catch (e) {
      setStatus('error')
      setError(String(e))
    }
  }

  const dirty = value.trim() !== initialValue.trim()

  return (
    <section className="card">
      <h2>Deepgram API Key</h2>
      <p className="hint">
        Required to transcribe audio. Paste your key from{' '}
        <span className="mono">console.deepgram.com</span>.
      </p>
      <div className="row">
        <input
          type="password"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="dg_..."
          spellCheck={false}
          autoComplete="off"
        />
        <button
          onClick={handleSave}
          disabled={!dirty || status === 'saving'}
        >
          {status === 'saving' ? 'Saving…' : 'Save'}
        </button>
      </div>
      {status === 'saved' && <div className="status ok">Saved</div>}
      {status === 'error' && <div className="status err">{error}</div>}
    </section>
  )
}
