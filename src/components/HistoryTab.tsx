import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { clearHistory as persistClearHistory, getHistory } from '../lib/api'
import type { HistoryEntry } from '../lib/types'

type LoadState = 'loading' | 'ready' | 'error'

function formatRelative(timestamp: number, now: number): string {
  const diff = Math.max(0, now - timestamp)
  if (diff < 45) return 'just now'
  if (diff < 60 * 60) {
    const m = Math.round(diff / 60)
    return `${m}m ago`
  }
  if (diff < 60 * 60 * 24) {
    const h = Math.round(diff / 3600)
    return `${h}h ago`
  }

  const date = new Date(timestamp * 1000)
  const today = new Date(now * 1000)
  const startOfToday = new Date(
    today.getFullYear(),
    today.getMonth(),
    today.getDate(),
  ).getTime() / 1000
  const entryDay = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  ).getTime() / 1000
  const dayDiff = Math.round((startOfToday - entryDay) / (60 * 60 * 24))

  const time = date.toLocaleTimeString([], {
    hour: 'numeric',
    minute: '2-digit',
  })
  if (dayDiff === 1) return `Yesterday, ${time}`
  if (dayDiff < 7) {
    const weekday = date.toLocaleDateString([], { weekday: 'short' })
    return `${weekday}, ${time}`
  }
  return date.toLocaleDateString([], {
    month: 'short',
    day: 'numeric',
    year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
  })
}

function useClock(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000))
  useEffect(() => {
    const id = window.setInterval(() => {
      setNow(Math.floor(Date.now() / 1000))
    }, intervalMs)
    return () => window.clearInterval(id)
  }, [intervalMs])
  return now
}

export function HistoryTab() {
  const [entries, setEntries] = useState<HistoryEntry[]>([])
  const [loadState, setLoadState] = useState<LoadState>('loading')
  const [loadError, setLoadError] = useState<string | null>(null)
  const [confirmingClear, setConfirmingClear] = useState(false)
  const confirmTimeout = useRef<number | null>(null)
  const now = useClock()

  const refresh = () => {
    getHistory()
      .then((list) => {
        setEntries(list)
        setLoadState('ready')
      })
      .catch((e) => {
        setLoadState('error')
        setLoadError(String(e))
      })
  }

  useEffect(() => {
    refresh()
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | null = null
    listen('history-updated', () => refresh())
      .then((u) => {
        unlisten = u
      })
      .catch((e) => console.error('history-updated listen failed', e))
    return () => {
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    return () => {
      if (confirmTimeout.current) window.clearTimeout(confirmTimeout.current)
    }
  }, [])

  const handleClear = async () => {
    if (!confirmingClear) {
      setConfirmingClear(true)
      confirmTimeout.current = window.setTimeout(() => {
        setConfirmingClear(false)
      }, 3000)
      return
    }
    if (confirmTimeout.current) window.clearTimeout(confirmTimeout.current)
    setConfirmingClear(false)
    try {
      await persistClearHistory()
      setEntries([])
    } catch (e) {
      console.error('clear history failed', e)
    }
  }

  if (loadState === 'loading') {
    return <div className="loading">Loading…</div>
  }

  if (loadState === 'error') {
    return (
      <section className="card err-card">
        Failed to load history: {loadError}
      </section>
    )
  }

  return (
    <section className="history">
      <div className="history-header">
        <div>
          <h2 className="history-title">Recent transcriptions</h2>
          <p className="hint history-hint">
            The last {entries.length} {entries.length === 1 ? 'entry' : 'entries'} stored locally. Up to 50 are kept.
          </p>
        </div>
        {entries.length > 0 && (
          <button
            type="button"
            className={`history-clear ${confirmingClear ? 'confirm' : ''}`}
            onClick={handleClear}
          >
            {confirmingClear ? 'Click to confirm' : 'Clear all'}
          </button>
        )}
      </div>

      {entries.length === 0 ? (
        <EmptyState />
      ) : (
        <ul className="history-list">
          {entries.map((entry, i) => (
            <HistoryItem
              key={`${entry.timestamp}-${i}`}
              entry={entry}
              now={now}
            />
          ))}
        </ul>
      )}
    </section>
  )
}

function EmptyState() {
  return (
    <div className="history-empty">
      <svg
        width="32"
        height="32"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M12 8v4l3 2" />
        <circle cx="12" cy="12" r="9" />
      </svg>
      <div className="history-empty-title">No transcriptions yet</div>
      <div className="history-empty-hint">
        Hold your shortcut and speak — transcripts will appear here.
      </div>
    </div>
  )
}

interface ItemProps {
  entry: HistoryEntry
  now: number
}

function HistoryItem({ entry, now }: ItemProps) {
  const [copied, setCopied] = useState(false)
  const copyTimeout = useRef<number | null>(null)

  useEffect(() => {
    return () => {
      if (copyTimeout.current) window.clearTimeout(copyTimeout.current)
    }
  }, [])

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(entry.text)
      setCopied(true)
      if (copyTimeout.current) window.clearTimeout(copyTimeout.current)
      copyTimeout.current = window.setTimeout(() => setCopied(false), 1200)
    } catch (e) {
      console.error('copy failed', e)
    }
  }

  return (
    <li className="history-item">
      <div className="history-item-head">
        <time className="history-time" dateTime={new Date(entry.timestamp * 1000).toISOString()}>
          {formatRelative(entry.timestamp, now)}
        </time>
        <button
          type="button"
          className={`history-copy ${copied ? 'copied' : ''}`}
          aria-label="Copy transcript"
          aria-live="polite"
          onClick={handleCopy}
        >
          <span className="history-copy-label" aria-hidden="true">
            {copied ? 'Copied' : 'Copy'}
          </span>
        </button>
      </div>
      <div className="history-text">{entry.text}</div>
    </li>
  )
}
