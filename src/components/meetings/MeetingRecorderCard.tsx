import { RecordIcon, StopIcon, WarningIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { toast } from "sonner";

import { ShortcutKeycaps } from "@/components/Keycap";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSettings } from "@/context/SettingsContext";
import { startMeeting, stopMeeting } from "@/lib/api";
import { isMeetingToggleBinding, type MeetingMeta } from "@/lib/types";

import { formatMeetingDuration, MS_PER_SECOND } from "./meetingFormat";
import { NotesModelPicker } from "./NotesModelPicker";
import { useElapsedMs } from "./useElapsedMs";
import {
  useMissingMeetingSetup,
  type MissingSetupStep,
} from "./useMeetingSetup";

const START_ERROR_TOAST_MS = 8000;

function MissingSetupAlert({ steps }: { steps: MissingSetupStep[] }) {
  return (
    <Alert variant="warning">
      <WarningIcon weight="fill" />
      <AlertDescription className="flex flex-col gap-1">
        {steps.map((step) => (
          <span key={step.key} className="flex flex-wrap items-center gap-x-2">
            {step.message}
            <Link
              to={step.path}
              className="font-medium underline underline-offset-2 hover:no-underline"
            >
              Set up
            </Link>
          </span>
        ))}
      </AlertDescription>
    </Alert>
  );
}

function MeetingShortcutHint() {
  const { settings } = useSettings();
  const binding = settings.hotkey_bindings.find(isMeetingToggleBinding);
  if (!binding) return null;
  return (
    <span className="flex items-center gap-2 text-xs text-muted-foreground">
      or press <ShortcutKeycaps shortcut={binding.shortcut} /> anywhere
    </span>
  );
}

function StartButton({
  missingSteps,
  busy,
  onStart,
}: {
  missingSteps: MissingSetupStep[];
  busy: boolean;
  onStart: () => void;
}) {
  const button = (
    <Button onClick={onStart} disabled={busy || missingSteps.length > 0}>
      <RecordIcon />
      Start meeting
    </Button>
  );
  if (missingSteps.length === 0) return button;
  // Disabled buttons swallow pointer events, so the tooltip hangs off a
  // focusable wrapper instead.
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} className="rounded-md">
          {button}
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-[260px] leading-snug">
        {missingSteps.map((step) => step.message).join(" ")}
      </TooltipContent>
    </Tooltip>
  );
}

function RecordingControls({
  recording,
  busy,
  onStop,
}: {
  recording: MeetingMeta;
  busy: boolean;
  onStop: () => void;
}) {
  const elapsedMs = useElapsedMs(recording.started_at * MS_PER_SECOND);
  return (
    <>
      <Button onClick={onStop} disabled={busy} variant="destructive">
        <StopIcon weight="fill" />
        Stop & summarize
      </Button>
      <span className="font-mono text-sm tabular-nums text-muted-foreground">
        {formatMeetingDuration(elapsedMs)}
      </span>
    </>
  );
}

function useMeetingToggle() {
  const [busy, setBusy] = useState(false);
  const run = (action: () => Promise<unknown>) => async () => {
    setBusy(true);
    try {
      await action();
    } catch (err) {
      toast.error(String(err), { duration: START_ERROR_TOAST_MS });
    } finally {
      setBusy(false);
    }
  };
  return { busy, start: run(startMeeting), stop: run(stopMeeting) };
}

export function MeetingRecorderCard({
  recording,
}: {
  recording: MeetingMeta | null;
}) {
  const { busy, start, stop } = useMeetingToggle();
  const missingSteps = useMissingMeetingSetup();
  const showSetupAlert = !recording && missingSteps.length > 0;

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-border/60 bg-card/40 p-4">
      {showSetupAlert && <MissingSetupAlert steps={missingSteps} />}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          {recording ? (
            <RecordingControls
              recording={recording}
              busy={busy}
              onStop={stop}
            />
          ) : (
            <StartButton
              missingSteps={missingSteps}
              busy={busy}
              onStart={start}
            />
          )}
          <MeetingShortcutHint />
        </div>
        <NotesModelPicker />
      </div>
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
