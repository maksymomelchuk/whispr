import { RecordIcon, StopIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import {
  AiProviderModelPicker,
  configuredAiProviders,
  type AiProviderModel,
} from "@/components/AiProviderModelPicker";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useSettings } from "@/context/SettingsContext";
import { setMeetingSummary, startMeeting, stopMeeting } from "@/lib/api";
import type { MeetingMeta } from "@/lib/types";

import { formatMeetingDuration } from "./meetingFormat";

const ELAPSED_TICK_MS = 1000;
const MS_PER_SECOND = 1000;

function useElapsedMs(startedAtSeconds: number | null): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (startedAtSeconds === null) return;
    const timer = setInterval(() => setNow(Date.now()), ELAPSED_TICK_MS);
    return () => clearInterval(timer);
  }, [startedAtSeconds]);
  return startedAtSeconds === null
    ? 0
    : Math.max(0, now - startedAtSeconds * MS_PER_SECOND);
}

function MissingSetupAlert() {
  const { settings } = useSettings();
  const summaryProviderReady = configuredAiProviders(settings).includes(
    settings.meeting_summary.provider,
  );
  const missing = [
    !settings.soniox_api_key_configured && "a Soniox key (Speech models)",
    !summaryProviderReady && "the notes provider connected (Cleanup page)",
  ].filter(Boolean);
  if (missing.length === 0) return null;
  return (
    <Alert>
      <AlertDescription>
        Meeting notes need {missing.join(" and ")}.
      </AlertDescription>
    </Alert>
  );
}

function SummaryProviderPicker() {
  const { settings, setSetting } = useSettings();
  const change = (summary: AiProviderModel) =>
    setSetting(
      "meeting_summary",
      summary,
      () => setMeetingSummary(summary),
      (err) => toast.error(`Failed to save notes model: ${String(err)}`),
    );
  return (
    <AiProviderModelPicker
      selection={settings.meeting_summary}
      onChange={change}
      configuredProviders={configuredAiProviders(settings)}
      customProviderModel={settings.custom_provider_model}
    />
  );
}

export function MeetingRecorderCard({
  recording,
}: {
  recording: MeetingMeta | null;
}) {
  const [busy, setBusy] = useState(false);
  const elapsedMs = useElapsedMs(recording?.started_at ?? null);

  const toggle = async () => {
    setBusy(true);
    try {
      if (recording) await stopMeeting();
      else await startMeeting();
    } catch (err) {
      toast.error(String(err), { duration: 8000 });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border/60 bg-card/40 p-4">
      <MissingSetupAlert />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Button
            onClick={toggle}
            disabled={busy}
            variant={recording ? "destructive" : "default"}
          >
            {recording ? <StopIcon weight="fill" /> : <RecordIcon />}
            {recording ? "Stop & summarize" : "Start meeting"}
          </Button>
          {recording && (
            <span className="font-mono text-sm tabular-nums text-muted-foreground">
              {formatMeetingDuration(elapsedMs)}
            </span>
          )}
        </div>
      </div>
      <SummaryProviderPicker />
      {recording?.system_audio_error && (
        <p className="text-xs text-amber-700 dark:text-amber-400">
          Recording your mic only: {recording.system_audio_error}
        </p>
      )}
      <p className="text-xs text-muted-foreground">
        Records your microphone and everything your Mac plays (the other side of
        the call). Nobody on the call is notified, so let them know you're
        taking notes. Without headphones your mic also picks up the other side.
      </p>
    </div>
  );
}
