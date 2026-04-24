import { useEffect, useState } from 'react'
import { setApiKey as persistApiKey } from '../lib/api'

interface Props {
  isConfigured: boolean
  onSaved: (configured: boolean) => void
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export function ApiKeyField({ isConfigured, onSaved }: Props) {
  const [value, setValue] = useState('')
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (status !== 'saved') return
    const t = setTimeout(() => setStatus('idle'), 1500)
    return () => clearTimeout(t)
  }, [status])

  const handleSave = async () => {
    setStatus('saving')
    setError(null)
    try {
      const trimmed = value.trim()
      await persistApiKey(trimmed)
      setValue('')
      onSaved(trimmed.length > 0)
      setStatus('saved')
    } catch (e) {
      setStatus('error')
      setError(String(e))
    }
  }

  const handleClear = async () => {
    setStatus('saving')
    setError(null)
    try {
      await persistApiKey('')
      setValue('')
      onSaved(false)
      setStatus('saved')
    } catch (e) {
      setStatus('error')
      setError(String(e))
    }
  }

  const dirty = value.trim().length > 0

  return (
    <section className="card">
      <h2>Deepgram API Key</h2>
      <p className="hint">
        Required to transcribe audio. Paste your key from{' '}
        <span className="mono">console.deepgram.com</span>.
      </p>
      <p className="hint-sm">
        Status:{' '}
        {isConfigured ? (
          <span className="status ok">Configured</span>
        ) : (
          <span className="status err">Not set</span>
        )}
      </p>
      <div className="row">
        <input
          type="password"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={isConfigured ? 'Enter new key to replace…' : 'dg_...'}
          spellCheck={false}
          autoComplete="off"
        />
        <button
          onClick={handleSave}
          disabled={!dirty || status === 'saving'}
        >
          {status === 'saving' ? 'Saving…' : 'Save'}
        </button>
        {isConfigured && (
          <button
            onClick={handleClear}
            disabled={status === 'saving'}
            className="secondary"
          >
            Clear
          </button>
        )}
      </div>
      {status === 'saved' && <div className="status ok">Saved</div>}
      {status === 'error' && <div className="status err">{error}</div>}
    </section>
  )
}
