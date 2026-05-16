import { createContext, useContext } from "react";

import type { Accent } from "../hooks/useAccent";
import type { ThemePreference } from "../hooks/useTheme";
import type { Settings } from "../lib/types";

interface SettingsContextValue {
  settings: Settings | null;
  setSettings: React.Dispatch<React.SetStateAction<Settings | null>>;
  themePreference: ThemePreference;
  setThemePreference: (next: ThemePreference) => void;
  accent: Accent;
  setAccent: (next: Accent) => void;
}

export const SettingsContext = createContext<SettingsContextValue | null>(null);

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx)
    throw new Error("useSettings must be used inside SettingsContext.Provider");
  return ctx;
}
