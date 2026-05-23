import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";

import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";

import { useSettings } from "../context/SettingsContext";
import { ACCENTS, type Accent } from "../hooks/useAccent";
import { usePersistedToggle } from "../hooks/usePersistedToggle";
import type { ThemePreference } from "../hooks/useTheme";
import {
  setShowInDock as persistShowInDock,
  setShowLivePreview as persistShowLivePreview,
  setStartAtLogin as persistStartAtLogin,
} from "../lib/api";
import { SectionCard } from "./SectionCard";
import { ToggleRow } from "./ToggleRow";

const themeSchema = z.object({
  theme: z.enum(["system", "light", "dark"]),
});

type ThemeFormValues = z.infer<typeof themeSchema>;

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const ACCENT_SWATCH: Record<
  Accent,
  { label: string; light: string; dark: string }
> = {
  indigo: {
    label: "Indigo",
    light: "hsl(224 76% 56%)",
    dark: "hsl(224 88% 66%)",
  },
  violet: {
    label: "Violet",
    light: "hsl(262 70% 58%)",
    dark: "hsl(262 80% 68%)",
  },
  coral: { label: "Coral", light: "hsl(15 80% 55%)", dark: "hsl(15 85% 65%)" },
  emerald: {
    label: "Emerald",
    light: "hsl(155 55% 38%)",
    dark: "hsl(155 48% 54%)",
  },
  graphite: {
    label: "Graphite",
    light: "hsl(220 16% 18%)",
    dark: "hsl(220 10% 88%)",
  },
};

export function AppearanceField() {
  const {
    settings,
    setSettings,
    themePreference,
    setThemePreference,
    accent,
    setAccent,
  } = useSettings();

  const isDark =
    themePreference === "dark" ||
    (themePreference === "system" &&
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);

  const form = useForm<ThemeFormValues>({
    resolver: zodResolver(themeSchema),
    values: { theme: themePreference },
  });

  const dock = usePersistedToggle(
    settings.show_in_dock,
    persistShowInDock,
    (next) => setSettings((s) => ({ ...s, show_in_dock: next })),
  );
  const preview = usePersistedToggle(
    settings.show_live_preview,
    persistShowLivePreview,
    (next) => setSettings((s) => ({ ...s, show_live_preview: next })),
  );
  const startAtLogin = usePersistedToggle(
    settings.start_at_login,
    persistStartAtLogin,
    (next) => setSettings((s) => ({ ...s, start_at_login: next })),
  );
  const saveError = dock.error ?? preview.error ?? startAtLogin.error;

  return (
    <SectionCard title="Appearance">
      <Form {...form}>
        <FormField
          control={form.control}
          name="theme"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel>Theme</FormLabel>
              <FormControl>
                <ToggleGroup
                  type="single"
                  variant="outline"
                  value={field.value}
                  onValueChange={(val) => {
                    if (val) setThemePreference(val as ThemePreference);
                  }}
                  className="w-full"
                >
                  {THEME_OPTIONS.map(({ value, label }) => (
                    <ToggleGroupItem
                      key={value}
                      value={value}
                      className="flex-1 text-xs"
                    >
                      {label}
                    </ToggleGroupItem>
                  ))}
                </ToggleGroup>
              </FormControl>
            </FormItem>
          )}
        />
      </Form>

      <div data-slot="form-item" className="mt-3.5 flex flex-col gap-[6px]">
        <span className="text-form-label text-muted-foreground">Accent</span>
        <div
          role="radiogroup"
          aria-label="Accent color"
          className="flex items-center gap-2.5"
        >
          {ACCENTS.map((name) => {
            const selected = accent === name;
            const swatch = ACCENT_SWATCH[name];
            return (
              <button
                key={name}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={swatch.label}
                title={swatch.label}
                onClick={() => setAccent(name)}
                className={cn(
                  "relative size-[22px] rounded-full outline-none transition-[box-shadow,transform] duration-150",
                  "focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:ring-offset-2 focus-visible:ring-offset-card",
                  selected
                    ? "ring-2 ring-foreground/70 ring-offset-2 ring-offset-card"
                    : "hover:scale-[1.08]",
                )}
                style={{ backgroundColor: isDark ? swatch.dark : swatch.light }}
              />
            );
          })}
        </div>
      </div>

      <ToggleRow
        id="show-in-dock"
        label="Show in Dock & Cmd-Tab"
        checked={dock.enabled}
        onCheckedChange={dock.toggle}
      />

      <ToggleRow
        id="show-live-preview"
        label="Show live preview while dictating"
        checked={preview.enabled}
        onCheckedChange={preview.toggle}
      />

      <ToggleRow
        id="start-at-login"
        label="Start at login"
        checked={startAtLogin.enabled}
        onCheckedChange={startAtLogin.toggle}
      />

      {saveError && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
