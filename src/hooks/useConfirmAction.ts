import { useEffect, useRef, useState } from "react";

/// Two-step destructive action: first click arms the button (`confirming = true`),
/// second click runs `action`. Auto-disarms after `timeoutMs` if no second click.
export function useConfirmAction(
  action: () => Promise<void> | void,
  timeoutMs = 3000,
) {
  const [confirming, setConfirming] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  const trigger = async () => {
    if (!confirming) {
      setConfirming(true);
      timeoutRef.current = window.setTimeout(() => {
        setConfirming(false);
      }, timeoutMs);
      return;
    }
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    setConfirming(false);
    await action();
  };

  return { confirming, trigger };
}
