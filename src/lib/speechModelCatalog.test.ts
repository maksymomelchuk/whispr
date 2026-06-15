import { describe, expect, it } from "vitest";

import { SPEECH_MODEL_CATALOG } from "./speechModelCatalog";
import type { Settings } from "./types";

const BASE_SETTINGS: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  openai_api_key_configured: false,
  elevenlabs_api_key_configured: false,
  soniox_api_key_configured: false,
  hotkey_bindings: [],
  term_sets: [],
  correction_sets: [],
  snippets: [],
  modes: [],
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 9,
  ai_cleanup_min_duration_ms: 3000,
  ai_cleanup_tone_overlay_enabled: false,
  tone_app_overrides: {},
  tone_app_custom_prompts: {},
  learn_from_corrections: false,
  input_device: null,
  pause_media_on_record: true,
  history_limit: 5,
  show_in_dock: false,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
  configured_providers: [],
  custom_provider_configured: false,
  custom_provider_base_url: null,
  custom_provider_model: "",
};

describe("speechModelCatalog", () => {
  it("contains exactly six engines", () => {
    expect(SPEECH_MODEL_CATALOG).toHaveLength(6);
  });

  it("every descriptor has a non-empty key placeholder and a help URL", () => {
    for (const descriptor of SPEECH_MODEL_CATALOG) {
      expect(descriptor.keyPlaceholder).toBeTruthy();
      expect(descriptor.helpUrl).toBeTruthy();
    }
  });

  it("every descriptor has a unique id", () => {
    const ids = SPEECH_MODEL_CATALOG.map((d) => d.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("deepgram selector returns false when not configured", () => {
    const deepgram = SPEECH_MODEL_CATALOG.find((d) => d.id === "deepgram")!;
    expect(deepgram.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("deepgram selector returns true when configured", () => {
    const deepgram = SPEECH_MODEL_CATALOG.find((d) => d.id === "deepgram")!;
    expect(
      deepgram.selectConfigured({
        ...BASE_SETTINGS,
        deepgram_api_key_configured: true,
      }),
    ).toBe(true);
  });

  it("groq selector returns false when not configured", () => {
    const groq = SPEECH_MODEL_CATALOG.find((d) => d.id === "groq")!;
    expect(groq.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("groq selector returns true when configured", () => {
    const groq = SPEECH_MODEL_CATALOG.find((d) => d.id === "groq")!;
    expect(
      groq.selectConfigured({
        ...BASE_SETTINGS,
        groq_api_key_configured: true,
      }),
    ).toBe(true);
  });

  it("assemblyai selector returns false when not configured", () => {
    const assemblyai = SPEECH_MODEL_CATALOG.find((d) => d.id === "assemblyai")!;
    expect(assemblyai.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("assemblyai selector returns true when configured", () => {
    const assemblyai = SPEECH_MODEL_CATALOG.find((d) => d.id === "assemblyai")!;
    expect(
      assemblyai.selectConfigured({
        ...BASE_SETTINGS,
        assemblyai_api_key_configured: true,
      }),
    ).toBe(true);
  });

  it("openai selector returns false when not configured", () => {
    const openai = SPEECH_MODEL_CATALOG.find((d) => d.id === "openai")!;
    expect(openai.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("openai selector returns true when configured", () => {
    const openai = SPEECH_MODEL_CATALOG.find((d) => d.id === "openai")!;
    expect(
      openai.selectConfigured({
        ...BASE_SETTINGS,
        openai_api_key_configured: true,
      }),
    ).toBe(true);
  });

  it("elevenlabs selector returns false when not configured", () => {
    const elevenlabs = SPEECH_MODEL_CATALOG.find((d) => d.id === "elevenlabs")!;
    expect(elevenlabs.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("elevenlabs selector returns true when configured", () => {
    const elevenlabs = SPEECH_MODEL_CATALOG.find((d) => d.id === "elevenlabs")!;
    expect(
      elevenlabs.selectConfigured({
        ...BASE_SETTINGS,
        elevenlabs_api_key_configured: true,
      }),
    ).toBe(true);
  });
});
