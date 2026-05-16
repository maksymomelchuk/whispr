import { Keyboard, Microphone } from "@phosphor-icons/react";

import { Card, CardContent } from "../components/ui/card";
import { useSettings } from "../context/SettingsContext";
import { formatShortcut } from "../lib/api";

export function HomePage() {
  const { settings } = useSettings();

  if (!settings) return null;

  const missingApiKey =
    settings.transcription_provider === "deepgram"
      ? !settings.deepgram_api_key_configured
      : !settings.groq_api_key_configured;
  const missingMic = !settings.input_device;

  return (
    <div className="p-6 flex flex-col gap-4 max-w-lg">
      <Card>
        <CardContent className="pt-6">
          <div className="flex items-center gap-3 mb-2">
            <Keyboard className="text-muted-foreground" size={20} />
            <span className="text-sm text-muted-foreground font-medium">
              Push to Talk
            </span>
          </div>
          <p className="text-2xl font-semibold text-foreground font-mono tracking-wide">
            {settings.hotkey_bindings.length > 0
              ? settings.hotkey_bindings
                  .map((b) => formatShortcut(b.shortcut))
                  .join("  ·  ")
              : "No hotkeys set"}
          </p>
          <p className="text-sm text-muted-foreground mt-1">
            Hold to dictate, release to transcribe and paste
          </p>
        </CardContent>
      </Card>

      {missingApiKey && (
        <Card className="border-amber-300 dark:border-amber-600 bg-amber-50 dark:bg-amber-950/30">
          <CardContent className="pt-6">
            <p className="text-sm font-semibold text-amber-800 dark:text-amber-300 mb-1">
              Set up your transcription API key
            </p>
            <p className="text-sm text-amber-700 dark:text-amber-400">
              Go to Transcription to configure your{" "}
              {settings.transcription_provider === "deepgram"
                ? "Deepgram"
                : "Groq"}{" "}
              API key before your first dictation.
            </p>
          </CardContent>
        </Card>
      )}

      {missingMic && (
        <Card className="border-amber-300 dark:border-amber-600 bg-amber-50 dark:bg-amber-950/30">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 mb-1">
              <Microphone size={15} className="text-amber-700 dark:text-amber-400" />
              <p className="text-sm font-semibold text-amber-800 dark:text-amber-300">
                Select a microphone
              </p>
            </div>
            <p className="text-sm text-amber-700 dark:text-amber-400">
              Go to General to choose your input device.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
