import { useCallback, useEffect, useRef } from 'react'

export interface CapturedAudio {
  bytes: Uint8Array
  blob: Blob
  mimeType: string
}

interface UseAudioCaptureOptions {
  onCaptured?: (audio: CapturedAudio) => void
  onError?: (err: unknown) => void
}

const MIME_CANDIDATES = ['audio/webm', 'audio/ogg'] as const

const pickMimeType = (): string => {
  for (const candidate of MIME_CANDIDATES) {
    if (MediaRecorder.isTypeSupported(candidate)) return candidate
  }
  return ''
}

/**
 * Push-to-talk audio capture. `start()` acquires the mic and begins recording;
 * `stop()` finalizes the recording and hands a WebM blob + bytes to `onCaptured`.
 *
 * The stream is acquired fresh per recording so device changes (e.g. plugging in
 * a headset) take effect on the next press without an app restart.
 */
export function useAudioCapture(options: UseAudioCaptureOptions = {}) {
  const optsRef = useRef(options)
  optsRef.current = options

  const recorderRef = useRef<MediaRecorder | null>(null)
  const streamRef = useRef<MediaStream | null>(null)
  const chunksRef = useRef<Blob[]>([])
  const startingRef = useRef(false)
  const cancelledRef = useRef(false)

  const teardownStream = () => {
    streamRef.current?.getTracks().forEach((t) => t.stop())
    streamRef.current = null
  }

  const start = useCallback(async () => {
    if (startingRef.current || recorderRef.current) return
    startingRef.current = true
    cancelledRef.current = false

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })

      // A release event may have arrived while getUserMedia was pending;
      // honor it and drop the stream without ever starting the recorder.
      if (cancelledRef.current) {
        stream.getTracks().forEach((t) => t.stop())
        return
      }

      const mimeType = pickMimeType()
      const recorder = new MediaRecorder(
        stream,
        mimeType ? { mimeType } : undefined,
      )
      chunksRef.current = []

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data)
      }

      recorder.onstop = async () => {
        const finalType = recorder.mimeType || mimeType || 'audio/webm'
        const blob = new Blob(chunksRef.current, { type: finalType })
        chunksRef.current = []
        teardownStream()
        recorderRef.current = null

        if (cancelledRef.current || blob.size === 0) return

        const buffer = await blob.arrayBuffer()
        optsRef.current.onCaptured?.({
          bytes: new Uint8Array(buffer),
          blob,
          mimeType: finalType,
        })
      }

      recorder.onerror = (e) => optsRef.current.onError?.(e)

      streamRef.current = stream
      recorderRef.current = recorder
      recorder.start(100)
    } catch (err) {
      teardownStream()
      recorderRef.current = null
      optsRef.current.onError?.(err)
    } finally {
      startingRef.current = false
    }
  }, [])

  const stop = useCallback(() => {
    // Release arrived before the recorder was even constructed (still awaiting
    // getUserMedia). Flag the pending start() to bail out when it resumes.
    if (startingRef.current && !recorderRef.current) {
      cancelledRef.current = true
      return
    }
    const recorder = recorderRef.current
    if (!recorder) return
    if (recorder.state !== 'inactive') {
      recorder.stop()
    } else {
      teardownStream()
      recorderRef.current = null
    }
  }, [])

  useEffect(() => {
    return () => {
      cancelledRef.current = true
      if (recorderRef.current && recorderRef.current.state !== 'inactive') {
        recorderRef.current.stop()
      }
      teardownStream()
      recorderRef.current = null
      chunksRef.current = []
    }
  }, [])

  return { start, stop }
}
