import { useCallback, useEffect, useRef, useState } from "react";

export function useFlash(durationMs = 700) {
  const [flashId, setFlashId] = useState<string | null>(null);
  const timeoutRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  const flash = useCallback(
    (id: string) => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
      setFlashId(id);
      timeoutRef.current = window.setTimeout(() => {
        setFlashId(null);
        timeoutRef.current = null;
      }, durationMs);
    },
    [durationMs],
  );

  const isFlashing = useCallback((id: string) => flashId === id, [flashId]);

  return { flash, isFlashing };
}
