import { describe, expect, it } from "vitest";

import type { Settings } from "@/lib/types";

import {
  configuredAiProviders,
  defaultModelFor,
} from "./AiProviderModelPicker";

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
  save_audio_recordings: false,
  hands_free_max_minutes: 30,
  show_in_dock: false,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
  configured_providers: [],
  custom_provider_configured: false,
  custom_provider_base_url: null,
  custom_provider_model: "",
  meeting_summary: { provider: "anthropic", model: "claude-sonnet-5-5" },
};

describe("configuredAiProviders", () => {
  it("counts Anthropic as configured through the OAuth token in OAuth mode", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      ai_cleanup_auth_mode: "oauth",
      ai_cleanup_oauth_token_configured: true,
    };

    expect(configuredAiProviders(settings)).toEqual(["anthropic"]);
  });

  it("ignores the credential of the inactive Anthropic auth mode", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      ai_cleanup_auth_mode: "api_key",
      ai_cleanup_oauth_token_configured: true,
    };

    expect(configuredAiProviders(settings)).toEqual([]);
  });

  it("includes the Custom provider once it has a base URL", () => {
    const settings = {
      ...BASE_SETTINGS,
      configured_providers: ["openai" as const],
      custom_provider_configured: true,
    };

    expect(configuredAiProviders(settings)).toEqual(["openai", "custom"]);
  });
});

describe("defaultModelFor", () => {
  it("picks the provider's first listed model", () => {
    expect(defaultModelFor("anthropic")).toBe("claude-haiku-5-5");
  });

  it("leaves the model blank for Custom, which names its own", () => {
    expect(defaultModelFor("custom")).toBe("");
  });
});
