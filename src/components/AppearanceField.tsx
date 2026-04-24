import type { ThemePreference } from "../hooks/useTheme";

interface Props {
  preference: ThemePreference;
  onChange: (next: ThemePreference) => void;
}

const OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function AppearanceField({ preference, onChange }: Props) {
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
    </section>
  );
}
