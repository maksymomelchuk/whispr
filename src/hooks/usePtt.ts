import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import type { Shortcut } from "../lib/types";

interface UsePttOptions {
  onPressed?: () => void;
  onReleased?: () => void;
}

// Callbacks are held in refs so passing fresh (un-memoized) functions
// doesn't reinstall the underlying Tauri listeners on every render.
export function usePtt({ onPressed, onReleased }: UsePttOptions = {}) {
  const onPressedRef = useRef(onPressed);
  const onReleasedRef = useRef(onReleased);
  onPressedRef.current = onPressed;
  onReleasedRef.current = onReleased;

  const [isHeld, setIsHeld] = useState(false);
  const [activeShortcut, setActiveShortcut] = useState<Shortcut | null>(null);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    const attach = async () => {
      const unP = await listen<Shortcut | null>("ptt-pressed", (e) => {
        setIsHeld(true);
        setActiveShortcut(e.payload ?? null);
        onPressedRef.current?.();
      });
      const unR = await listen("ptt-released", () => {
        setIsHeld(false);
        setActiveShortcut(null);
        onReleasedRef.current?.();
      });
      // If the component unmounted before subscriptions resolved, tear them
      // down immediately instead of leaking.
      if (cancelled) {
        unP();
        unR();
        return;
      }
      unlisteners.push(unP, unR);
    };
    attach();

    return () => {
      cancelled = true;
      unlisteners.forEach((un) => un());
    };
  }, []);

  return { isHeld, activeShortcut };
}
