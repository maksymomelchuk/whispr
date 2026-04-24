import { useEffect, useState } from 'react'
import { setDeepgramSettings as persistDeepgramSettings } from '../lib/api'
import type { DeepgramSettings } from '../lib/types'
import { CollapsibleCard } from './CollapsibleCard'

interface Props {
  initial: DeepgramSettings
  onSaved: (deepgram: DeepgramSettings) => void
  defaultOpen?: boolean
}

type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

type BoolKey = {
  [K in keyof DeepgramSettings]: DeepgramSettings[K] extends boolean ? K : never
}[keyof DeepgramSettings]

interface BoolOption {
  key: BoolKey
  label: string
  param: string
  description: string
}

const BOOL_OPTIONS: BoolOption[] = [
  {
    key: 'smart_format',
    label: 'Smart Format',
    param: 'smart_format',
    description:
      'Improves readability by applying additional formatting. When enabled, punctuation and paragraph breaks will be applied as well as formatting of other entities, such as dates, times, and numbers.',
  },
  {
    key: 'dictation',
    label: 'Dictation',
    param: 'dictation',
    description:
      'Automatically formats spoken commands for punctuation into their respective punctuation marks.',
  },
  {
    key: 'numerals',
    label: 'Numerals',
    param: 'numerals',
    description:
      'Converts numbers from written format to numerical format (e.g., "nine hundred" becomes "900").',
  },
]

const same = (a: DeepgramSettings, b: DeepgramSettings) =>
  a.language === b.language &&
  a.smart_format === b.smart_format &&
  a.dictation === b.dictation &&
  a.numerals === b.numerals &&
  a.keyterms.length === b.keyterms.length &&
  a.keyterms.every((k, i) => k === b.keyterms[i])

export function TranscriptionField({ initial, onSaved, defaultOpen = false }: Props) {
  const [state, setState] = useState<DeepgramSettings>(initial)
  const [status, setStatus] = useState<SaveStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setState(initial)
  }, [initial])

  useEffect(() => {
    if (status !== 'saved') return
    const t = setTimeout(() => setStatus('idle'), 1500)
    return () => clearTimeout(t)
  }, [status])

  const toggle = (key: BoolKey) =>
    setState((s) => ({ ...s, [key]: !s[key] }))

  const setLanguage = (language: string) =>
    setState((s) => ({ ...s, language }))

  const updateKeyterm = (index: number, value: string) =>
    setState((s) => ({
      ...s,
      keyterms: s.keyterms.map((k, i) => (i === index ? value : k)),
    }))

  const removeKeyterm = (index: number) =>
    setState((s) => ({
      ...s,
      keyterms: s.keyterms.filter((_, i) => i !== index),
    }))

  const addKeyterm = () =>
    setState((s) => ({ ...s, keyterms: [...s.keyterms, ''] }))

  const handleSave = async () => {
    const cleaned: DeepgramSettings = {
      ...state,
      language: state.language.trim() || 'en',
      keyterms: state.keyterms.map((k) => k.trim()).filter((k) => k.length > 0),
    }
    setStatus('saving')
    setError(null)
    try {
      await persistDeepgramSettings(cleaned)
      setState(cleaned)
      onSaved(cleaned)
      setStatus('saved')
    } catch (e) {
      setStatus('error')
      setError(String(e))
    }
  }

  const dirty = !same(state, initial)

  return (
    <CollapsibleCard title="Deepgram" defaultOpen={defaultOpen} dirty={dirty}>
      <p className="hint">
        Deepgram nova-3 options. Defaults are off; enable what you need.
      </p>

      <div className="field-group">
        <label className="field-label">Language</label>
        <input
          type="text"
          value={state.language}
          placeholder="en"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => setLanguage(e.target.value)}
        />
        <p className="hint-sm left">
          Language code (e.g. <span className="mono">en</span>,{' '}
          <span className="mono">multi</span>, <span className="mono">es</span>
          , <span className="mono">de</span>).
        </p>
      </div>

      <div className="options-list">
        {BOOL_OPTIONS.map((opt) => (
          <label key={opt.key} className="option-row">
            <input
              type="checkbox"
              checked={state[opt.key]}
              onChange={() => toggle(opt.key)}
            />
            <div className="option-text">
              <div className="option-label">{opt.label}</div>
              <div className="option-param mono">
                {opt.param}={String(state[opt.key])}
              </div>
              <div className="option-description">{opt.description}</div>
            </div>
          </label>
        ))}

        <div className="option-row keyterms-option">
          <div className="option-text">
            <div className="option-label">Keyterm Prompting</div>
            <div className="option-param mono">keyterm=TERM_OR_PHRASE</div>
            <div className="option-description">
              Boosts recognition of important words or phrases, like names,
              product terms, or jargon. The model pays extra attention to
              these; you can include up to 100 keyterms per request.
            </div>
            <div className="keyterms-list">
              {state.keyterms.map((kt, i) => (
                <div key={i} className="replacement-row">
                  <input
                    type="text"
                    value={kt}
                    placeholder="e.g. Deepgram"
                    spellCheck={false}
                    autoComplete="off"
                    onChange={(e) => updateKeyterm(i, e.target.value)}
                  />
                  <button
                    className="icon-button"
                    aria-label="Remove"
                    onClick={() => removeKeyterm(i)}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
            <div className="row">
              <button onClick={addKeyterm}>+ Add keyterm</button>
            </div>
          </div>
        </div>
      </div>

      <div className="row replacements-actions save-row">
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
