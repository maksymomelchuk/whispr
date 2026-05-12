import {
  setShowInDock as persistShowInDock,
  setShowLivePreview as persistShowLivePreview,
} from "../lib/api";
import { usePersistedToggle } from "../hooks/usePersistedToggle";
import type { ThemePreference } from "../hooks/useTheme";

interface Props {
  preference: ThemePreference;
  onChange: (next: ThemePreference) => void;
  showInDock: boolean;
  onShowInDockChange: (next: boolean) => void;
  showLivePreview: boolean;
  onShowLivePreviewChange: (next: boolean) => void;
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
  showLivePreview,
  onShowLivePreviewChange,
}: Props) {
  const dock = usePersistedToggle(
    showInDock,
    persistShowInDock,
    onShowInDockChange,
  );
  const preview = usePersistedToggle(
    showLivePreview,
    persistShowLivePreview,
    onShowLivePreviewChange,
  );
  const saveError = dock.error ?? preview.error;

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
          checked={dock.enabled}
          onChange={dock.toggle}
        />
      </label>

      <label className="toggle-row">
        <span className="toggle-row-label">Show live preview while dictating</span>
        <input
          type="checkbox"
          role="switch"
          checked={preview.enabled}
          onChange={preview.toggle}
        />
      </label>

      {saveError && <div className="status err">{saveError}</div>}
    </section>
  );
}
