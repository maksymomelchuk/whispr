import { useEffect, useState } from 'react'
import { ApiKeyField } from './components/ApiKeyField'
import { ShortcutField } from './components/ShortcutField'
import { ShortcutRecorder } from './components/ShortcutRecorder'
import {
  getSettings,
  setShortcut as persistShortcut,
  transcribe,
  pasteText,
} from './lib/api'
import { usePtt } from './hooks/usePtt'
import { useAudioCapture } from './hooks/useAudioCapture'
import type { Settings, Shortcut } from './lib/types'
import './App.css'

function App() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [recording, setRecording] = useState(false)

  const { start: startCapture, stop: stopCapture } = useAudioCapture({
    onCaptured: async ({ blob, bytes, mimeType }) => {
      console.log('[audio] captured', {
        mimeType,
        blobSize: blob.size,
        bytes: bytes.length,
      })
      try {
        const transcript = await transcribe(bytes)
        console.log('[transcribe] result', JSON.stringify(transcript))
        if (transcript) {
          await pasteText(transcript)
        }
      } catch (err) {
        console.error('[transcribe] failed', err)
      }
    },
    onError: (err) => console.error('[audio] capture error', err),
  })
  const { isHeld } = usePtt({
    onPressed: startCapture,
    onReleased: stopCapture,
  })

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e) => setLoadError(String(e)))
  }, [])

  const handleShortcutSave = async (shortcut: Shortcut) => {
    try {
      await persistShortcut(shortcut)
      setSettings((s) => (s ? { ...s, shortcut } : s))
      setRecording(false)
    } catch (e) {
      console.error('Failed to save shortcut', e)
    }
  }

  if (loadError) {
    return (
      <main className="app">
        <div className="card err-card">Failed to load settings: {loadError}</div>
      </main>
    )
  }

  if (!settings) {
    return (
      <main className="app">
        <div className="loading">Loading…</div>
      </main>
    )
  }

  return (
    <main className="app">
      <header className="app-header">
        <div className="header-row">
          <div>
            <h1>Wispr Tauri</h1>
            <p className="subtitle">Push-to-talk speech-to-text</p>
          </div>
          <div className={`ptt-indicator ${isHeld ? 'active' : ''}`}>
            {isHeld ? '● Recording' : '○ Idle'}
          </div>
        </div>
      </header>
      <ApiKeyField initialValue={settings.api_key ?? ''} />
      <ShortcutField
        shortcut={settings.shortcut}
        onStartRecord={() => setRecording(true)}
      />
      {recording && (
        <ShortcutRecorder
          initial={settings.shortcut}
          onSave={handleShortcutSave}
          onCancel={() => setRecording(false)}
        />
      )}
    </main>
  )
}

export default App
