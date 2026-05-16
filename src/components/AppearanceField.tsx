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

import { usePersistedToggle } from "../hooks/usePersistedToggle";
import type { ThemePreference } from "../hooks/useTheme";
import { SectionCard } from "./SectionCard";
import { ToggleRow } from "./ToggleRow";
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
    <SectionCard title="Appearance">
      <Form {...form}>
        <FormField
          control={form.control}
          name="theme"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel className="text-[11px] font-medium tracking-[0.2px] text-muted-foreground">
                Theme
              </FormLabel>
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

      {saveError && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
