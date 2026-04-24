import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'

interface UsePttOptions {
  onPressed?: () => void
  onReleased?: () => void
}

/**
 * Subscribes to global PTT events emitted by the Rust keyboard listener.
 * Exposes a simple held/not-held state for the UI and forwards callbacks
 * for components that need to start/stop side effects on edges.
 *
 * Callbacks are held in refs so passing fresh (un-memoized) functions
 * doesn't reinstall the underlying Tauri listeners on every render.
 */
export function usePtt({ onPressed, onReleased }: UsePttOptions = {}) {
  const onPressedRef = useRef(onPressed)
  const onReleasedRef = useRef(onReleased)
  onPressedRef.current = onPressed
  onReleasedRef.current = onReleased

  const [isHeld, setIsHeld] = useState(false)

  useEffect(() => {
    let cancelled = false
    const unlisteners: Array<() => void> = []

    const attach = async () => {
      const unP = await listen('ptt-pressed', () => {
        setIsHeld(true)
        onPressedRef.current?.()
      })
      const unR = await listen('ptt-released', () => {
        setIsHeld(false)
        onReleasedRef.current?.()
      })
      // If the component unmounted before subscriptions resolved, tear them
      // down immediately instead of leaking.
      if (cancelled) {
        unP()
        unR()
        return
      }
      unlisteners.push(unP, unR)
    }
    attach()

    return () => {
      cancelled = true
      unlisteners.forEach((un) => un())
    }
  }, [])

  return { isHeld }
}
