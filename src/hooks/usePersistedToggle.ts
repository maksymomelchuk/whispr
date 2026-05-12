import { useEffect, useState } from "react";

export interface PersistedToggle {
  enabled: boolean;
  toggle: () => Promise<void>;
  error: string | null;
}

export function usePersistedToggle(
  value: boolean,
  persist: (next: boolean) => Promise<void>,
  onPersisted: (next: boolean) => void,
): PersistedToggle {
  const [enabled, setEnabled] = useState(value);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setEnabled(value);
  }, [value]);

  const toggle = async () => {
    const next = !enabled;
    setEnabled(next);
    setError(null);
    try {
      await persist(next);
      onPersisted(next);
    } catch (e) {
      setEnabled(!next);
      setError(String(e));
    }
  };

  return { enabled, toggle, error };
}
