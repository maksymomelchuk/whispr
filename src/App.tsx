import { useEffect, useState } from 'react'
import { ApiKeyField } from './components/ApiKeyField'
import { ReplacementsField } from './components/ReplacementsField'
import { ShortcutField } from './components/ShortcutField'
import { ShortcutRecorder } from './components/ShortcutRecorder'
import { TranscriptionField } from './components/TranscriptionField'
import { getSettings, setShortcut as persistShortcut } from './lib/api'
import { usePtt } from './hooks/usePtt'
import type {
  DeepgramSettings,
  Replacement,
  Settings,
  Shortcut,
} from './lib/types'
import './App.css'

type TabId = 'general' | 'shortcut' | 'transcription' | 'replacements'

const TABS: { id: TabId; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'shortcut', label: 'Shortcut' },
  { id: 'transcription', label: 'Transcription' },
  { id: 'replacements', label: 'Replacements' },
]

function App() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [recording, setRecording] = useState(false)
  const [activeTab, setActiveTab] = useState<TabId>('general')

  const { isHeld } = usePtt()

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
          <h1>Wispr Tauri</h1>
          <div className={`ptt-indicator ${isHeld ? 'active' : ''}`}>
            {isHeld ? '● Recording' : '○ Idle'}
          </div>
        </div>
      </header>

      <nav className="tabs" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`tab ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <div className="tab-panel">
        {activeTab === 'general' && (
          <ApiKeyField initialValue={settings.api_key ?? ''} />
        )}
        {activeTab === 'shortcut' && (
          <ShortcutField
            shortcut={settings.shortcut}
            onStartRecord={() => setRecording(true)}
          />
        )}
        {activeTab === 'transcription' && (
          <TranscriptionField
            initial={settings.deepgram}
            onSaved={(deepgram: DeepgramSettings) =>
              setSettings((s) => (s ? { ...s, deepgram } : s))
            }
          />
        )}
        {activeTab === 'replacements' && (
          <ReplacementsField
            initial={settings.replacements}
            onSaved={(replacements: Replacement[]) =>
              setSettings((s) => (s ? { ...s, replacements } : s))
            }
          />
        )}
      </div>

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
