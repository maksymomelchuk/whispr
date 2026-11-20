import { useEffect, useState } from "react";

const ELAPSED_TICK_MS = 1000;

export function useElapsedMs(startedAtMs: number | null): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (startedAtMs === null) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), ELAPSED_TICK_MS);
    return () => clearInterval(timer);
  }, [startedAtMs]);
  return startedAtMs === null ? 0 : Math.max(0, now - startedAtMs);
}
