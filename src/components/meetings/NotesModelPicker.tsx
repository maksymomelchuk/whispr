import { toast } from "sonner";

import {
  AiProviderModelPicker,
  configuredAiProviders,
  modelDisplayName,
  type AiProviderModel,
} from "@/components/AiProviderModelPicker";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useSettings } from "@/context/SettingsContext";
import { setMeetingSummary } from "@/lib/api";

export function NotesModelPicker() {
  const { settings, setSetting } = useSettings();
  const change = (summary: AiProviderModel) =>
    setSetting(
      "meeting_summary",
      summary,
      () => setMeetingSummary(summary),
      (err) => toast.error(`Failed to save notes model: ${String(err)}`),
    );
  const modelName = modelDisplayName(
    settings.meeting_summary,
    settings.custom_provider_model,
  );

  return (
    <span className="flex items-center gap-1 text-xs text-muted-foreground">
      Notes by <span className="font-medium text-foreground">{modelName}</span>
      <span aria-hidden="true">·</span>
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="link" size="xs" className="h-auto px-0 text-xs">
            Change
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-[420px]">
          <AiProviderModelPicker
            selection={settings.meeting_summary}
            onChange={change}
            configuredProviders={configuredAiProviders(settings)}
            customProviderModel={settings.custom_provider_model}
          />
        </PopoverContent>
      </Popover>
    </span>
  );
}
