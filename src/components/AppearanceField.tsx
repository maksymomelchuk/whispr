import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";

import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

import { usePersistedToggle } from "../hooks/usePersistedToggle";
import type { ThemePreference } from "../hooks/useTheme";
import {
  setShowInDock as persistShowInDock,
  setShowLivePreview as persistShowLivePreview,
} from "../lib/api";

const themeSchema = z.object({
  theme: z.enum(["system", "light", "dark"]),
});

type ThemeFormValues = z.infer<typeof themeSchema>;

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

interface Props {
  preference: ThemePreference;
  onChange: (next: ThemePreference) => void;
  showInDock: boolean;
  onShowInDockChange: (next: boolean) => void;
  showLivePreview: boolean;
  onShowLivePreviewChange: (next: boolean) => void;
}

export function AppearanceField({
  preference,
  onChange,
  showInDock,
  onShowInDockChange,
  showLivePreview,
  onShowLivePreviewChange,
}: Props) {
  const form = useForm<ThemeFormValues>({
    resolver: zodResolver(themeSchema),
    values: { theme: preference },
  });

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
      <Form {...form}>
        <FormField
          control={form.control}
          name="theme"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel className="field-label">Theme</FormLabel>
              <FormControl>
                <ToggleGroup
                  type="single"
                  variant="outline"
                  value={field.value}
                  onValueChange={(val) => {
                    if (val) onChange(val as ThemePreference);
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

      <label className="toggle-row">
        <span className="toggle-row-label">Show in Dock & Cmd-Tab</span>
        <Switch checked={dock.enabled} onCheckedChange={dock.toggle} />
      </label>

      <label className="toggle-row">
        <span className="toggle-row-label">
          Show live preview while dictating
        </span>
        <Switch checked={preview.enabled} onCheckedChange={preview.toggle} />
      </label>

      {saveError && <div className="status err">{saveError}</div>}
    </section>
  );
}
