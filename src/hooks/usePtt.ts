import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'

interface UsePttOptions {
  onPressed?: () => void
  onReleased?: () => void
}

/**
 * Subscribes to global PTT events emitted by the Rust keyboard listener.
 * Exposes a simple held/not-held state for the UI and forwards callbacks
 * for components that need to start/stop side effects on edges.
 */
export function usePtt({ onPressed, onReleased }: UsePttOptions = {}) {
  const [isHeld, setIsHeld] = useState(false)

  useEffect(() => {
    const unlisteners: Array<() => void> = []

    listen('ptt-pressed', () => {
      setIsHeld(true)
      onPressed?.()
    }).then((un) => unlisteners.push(un))

    listen('ptt-released', () => {
      setIsHeld(false)
      onReleased?.()
    }).then((un) => unlisteners.push(un))

    return () => {
      unlisteners.forEach((un) => un())
    }
  }, [onPressed, onReleased])

  return { isHeld }
}
