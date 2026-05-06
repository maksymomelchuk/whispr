import { useEffect, useState } from "react";

import { setShowInDock as persistShowInDock } from "../lib/api";
import type { ThemePreference } from "../hooks/useTheme";

interface Props {
  preference: ThemePreference;
  onChange: (next: ThemePreference) => void;
  showInDock: boolean;
  onShowInDockChange: (next: boolean) => void;
}

const OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function AppearanceField({
  preference,
  onChange,
  showInDock,
  onShowInDockChange,
}: Props) {
  const [dockEnabled, setDockEnabled] = useState(showInDock);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    setDockEnabled(showInDock);
  }, [showInDock]);

  const toggleDock = async () => {
    const next = !dockEnabled;
    setDockEnabled(next);
    setSaveError(null);
    try {
      await persistShowInDock(next);
      onShowInDockChange(next);
    } catch (e) {
      setDockEnabled(!next);
      setSaveError(String(e));
    }
  };

  return (
    <section className="card">
      <h2>Appearance</h2>
      <div className="field">
        <span className="field-label" id="theme-label">
          Theme
        </span>
        <div
          className="segmented"
          role="radiogroup"
          aria-labelledby="theme-label"
        >
          {OPTIONS.map((opt) => {
            const active = preference === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                role="radio"
                aria-checked={active}
                className={`segmented-option ${active ? "active" : ""}`}
                onClick={() => onChange(opt.value)}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </div>

      <label className="toggle-row">
        <span className="toggle-row-label">Show in Dock & Cmd-Tab</span>
        <input
          type="checkbox"
          role="switch"
          checked={dockEnabled}
          onChange={toggleDock}
        />
      </label>

      {saveError && <div className="status err">{saveError}</div>}
    </section>
  );
}
