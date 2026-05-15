import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";

import {
  setShowInDock as persistShowInDock,
  setShowLivePreview as persistShowLivePreview,
} from "../lib/api";
import { usePersistedToggle } from "../hooks/usePersistedToggle";
import type { ThemePreference } from "../hooks/useTheme";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Switch } from "@/components/ui/switch";

const themeSchema = z.object({
  theme: z.enum(["system", "light", "dark"]),
});

type ThemeFormValues = z.infer<typeof themeSchema>;

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
    defaultValues: { theme: preference },
  });

  useEffect(() => {
    form.setValue("theme", preference);
  }, [preference, form]);

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
                    if (!val) return;
                    const theme = val as ThemePreference;
                    field.onChange(theme);
                    onChange(theme);
                  }}
                  className="w-full"
                >
                  <ToggleGroupItem value="system" className="flex-1 text-xs">
                    System
                  </ToggleGroupItem>
                  <ToggleGroupItem value="light" className="flex-1 text-xs">
                    Light
                  </ToggleGroupItem>
                  <ToggleGroupItem value="dark" className="flex-1 text-xs">
                    Dark
                  </ToggleGroupItem>
                </ToggleGroup>
              </FormControl>
            </FormItem>
          )}
        />
      </Form>

      <label className="toggle-row">
        <span className="toggle-row-label">Show in Dock &amp; Cmd-Tab</span>
        <Switch
          checked={dock.enabled}
          onCheckedChange={() => dock.toggle()}
        />
      </label>

      <label className="toggle-row">
        <span className="toggle-row-label">Show live preview while dictating</span>
        <Switch
          checked={preview.enabled}
          onCheckedChange={() => preview.toggle()}
        />
      </label>

      {saveError && <div className="status err">{saveError}</div>}
    </section>
  );
}
