import { describe, expect, it } from "vitest";

import type { Settings } from "./types";
import { SPEECH_MODEL_CATALOG } from "./speechModelCatalog";

const BASE_SETTINGS: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  hotkey_bindings: [],
  term_sets: [],
  correction_sets: [],
  snippets: [],
  modes: [],
  default_mode_id: "",
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 9,
  ai_cleanup_min_duration_ms: 3000,
  input_device: null,
  pause_media_on_record: true,
  history_limit: 5,
  show_in_dock: false,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
};

describe("speechModelCatalog", () => {
  it("contains exactly three engines", () => {
    expect(SPEECH_MODEL_CATALOG).toHaveLength(3);
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
      deepgram.selectConfigured({ ...BASE_SETTINGS, deepgram_api_key_configured: true }),
    ).toBe(true);
  });

  it("groq selector returns false when not configured", () => {
    const groq = SPEECH_MODEL_CATALOG.find((d) => d.id === "groq")!;
    expect(groq.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("groq selector returns true when configured", () => {
    const groq = SPEECH_MODEL_CATALOG.find((d) => d.id === "groq")!;
    expect(
      groq.selectConfigured({ ...BASE_SETTINGS, groq_api_key_configured: true }),
    ).toBe(true);
  });

  it("assemblyai selector returns false when not configured", () => {
    const assemblyai = SPEECH_MODEL_CATALOG.find((d) => d.id === "assemblyai")!;
    expect(assemblyai.selectConfigured(BASE_SETTINGS)).toBe(false);
  });

  it("assemblyai selector returns true when configured", () => {
    const assemblyai = SPEECH_MODEL_CATALOG.find((d) => d.id === "assemblyai")!;
    expect(
      assemblyai.selectConfigured({ ...BASE_SETTINGS, assemblyai_api_key_configured: true }),
    ).toBe(true);
  });
});
