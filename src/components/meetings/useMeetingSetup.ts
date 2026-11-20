import {
  configuredAiProviders,
  providerDisplayName,
} from "@/components/AiProviderModelPicker";
import { useSettings } from "@/context/SettingsContext";

export interface MissingSetupStep {
  key: "soniox" | "notes-provider";
  message: string;
  path: string;
}

const SPEECH_MODELS_PATH = "/speech-models";
const CLEANUP_PATH = "/ai-providers";

export function useMissingMeetingSetup(): MissingSetupStep[] {
  const { settings } = useSettings();
  const notesProvider = settings.meeting_summary.provider;
  const steps: MissingSetupStep[] = [];
  if (!settings.soniox_api_key_configured) {
    steps.push({
      key: "soniox",
      message: "Add a Soniox API key to transcribe meetings.",
      path: SPEECH_MODELS_PATH,
    });
  }
  if (!configuredAiProviders(settings).includes(notesProvider)) {
    steps.push({
      key: "notes-provider",
      message: `Connect ${providerDisplayName(notesProvider)} to write the notes.`,
      path: CLEANUP_PATH,
    });
  }
  return steps;
}
