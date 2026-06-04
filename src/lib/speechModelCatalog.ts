import type { ComponentType } from "react";

import { AssemblyAiLogo } from "@/assets/AssemblyAiLogo";
import { DeepgramLogo } from "@/assets/DeepgramLogo";
import { ElevenLabsLogo } from "@/assets/ElevenLabsLogo";
import { GroqLogo } from "@/assets/GroqLogo";
import { OpenAiLogo } from "@/assets/OpenAiLogo";

import {
  setAssemblyAiApiKey,
  setDeepgramApiKey,
  setElevenLabsApiKey,
  setGroqApiKey,
  setOpenaiApiKey,
  validateAssemblyAiApiKey,
  validateDeepgramApiKey,
  validateElevenLabsApiKey,
  validateGroqApiKey,
  validateOpenaiApiKey,
} from "./api";
import type { ApiKeyValidation, Settings } from "./types";

export interface EngineDescriptor {
  id: string;
  name: string;
  logo: ComponentType<{ className?: string }>;
  description: string;
  metadata: { languages: string; streaming: string; diarization: string };
  keyPlaceholder: string;
  helpUrl: string;
  selectConfigured: (settings: Settings) => boolean;
  persist: (key: string) => Promise<void>;
  validate: (key: string) => Promise<ApiKeyValidation>;
}

export const SPEECH_MODEL_CATALOG: EngineDescriptor[] = [
  {
    id: "deepgram",
    name: "Deepgram",
    logo: DeepgramLogo,
    description: "Real-time and batch transcription with Deepgram Nova.",
    metadata: {
      languages: "30+ languages",
      streaming: "Yes",
      diarization: "Yes",
    },
    keyPlaceholder: "dg_...",
    helpUrl: "https://console.deepgram.com/",
    selectConfigured: (s) => s.deepgram_api_key_configured,
    persist: setDeepgramApiKey,
    validate: validateDeepgramApiKey,
  },
  {
    id: "groq",
    name: "Groq",
    logo: GroqLogo,
    description: "Ultra-fast transcription powered by Groq LPU inference.",
    metadata: {
      languages: "100+ languages",
      streaming: "No",
      diarization: "No",
    },
    keyPlaceholder: "gsk_...",
    helpUrl: "https://console.groq.com/keys",
    selectConfigured: (s) => s.groq_api_key_configured,
    persist: setGroqApiKey,
    validate: validateGroqApiKey,
  },
  {
    id: "assemblyai",
    name: "AssemblyAI",
    logo: AssemblyAiLogo,
    description: "High-accuracy transcription with speaker diarization.",
    metadata: {
      languages: "99+ languages",
      streaming: "Yes",
      diarization: "Yes",
    },
    keyPlaceholder: "assembly_...",
    helpUrl: "https://www.assemblyai.com/app/account",
    selectConfigured: (s) => s.assemblyai_api_key_configured,
    persist: setAssemblyAiApiKey,
    validate: validateAssemblyAiApiKey,
  },
  {
    id: "openai",
    name: "OpenAI",
    logo: OpenAiLogo,
    description: "Batch transcription with gpt-4o-transcribe.",
    metadata: {
      languages: "100+ languages",
      streaming: "No",
      diarization: "No",
    },
    keyPlaceholder: "sk-...",
    helpUrl: "https://platform.openai.com/api-keys",
    selectConfigured: (s) => s.openai_api_key_configured,
    persist: setOpenaiApiKey,
    validate: validateOpenaiApiKey,
  },
  {
    id: "elevenlabs",
    name: "ElevenLabs",
    logo: ElevenLabsLogo,
    description: "Batch transcription with ElevenLabs Scribe v2.",
    metadata: {
      languages: "99+ languages",
      streaming: "No",
      diarization: "No",
    },
    keyPlaceholder: "xi_...",
    helpUrl: "https://elevenlabs.io/app/settings/api-keys",
    selectConfigured: (s) => s.elevenlabs_api_key_configured,
    persist: setElevenLabsApiKey,
    validate: validateElevenLabsApiKey,
  },
];
