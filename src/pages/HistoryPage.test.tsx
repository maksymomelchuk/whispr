import { render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { SettingsContext } from "@/context/SettingsContext";
import type { Settings } from "@/lib/types";

import { HistoryPage } from "./HistoryPage";

vi.mock("../lib/api", () => ({
  getHistory: vi.fn().mockResolvedValue([]),
  clearHistory: vi.fn(),
  setHistoryLimit: vi.fn(),
  recoverCleanup: vi.fn(),
  updateHistoryEntry: vi.fn(),
}));

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
  configured_providers: [],
  custom_provider_configured: false,
  custom_provider_base_url: null,
  custom_provider_model: "",
  ai_cleanup_min_words: 3,
  ai_cleanup_min_duration_ms: 1000,
  ai_cleanup_tone_overlay_enabled: false,
  tone_app_overrides: {},
  tone_app_custom_prompts: {},
  learn_from_corrections: false,
  input_device: null,
  pause_media_on_record: false,
  history_limit: 100,
  save_audio_recordings: false,
  hands_free_max_minutes: 30,
  show_in_dock: true,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
};

function Wrapper({ children }: { children: React.ReactNode }) {
  const [s, setRawSettings] = useState<Settings>(BASE_SETTINGS);
  return (
    <SettingsContext.Provider
      value={{
        settings: s,
        setSettings: (updater) => setRawSettings(updater),
        setSetting: vi.fn(),
        themePreference: "system",
        setThemePreference: vi.fn(),
        accent: "indigo",
        setAccent: vi.fn(),
      }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

describe("HistoryPage", () => {
  it("renders a page heading 'History'", () => {
    render(
      <Wrapper>
        <HistoryPage />
      </Wrapper>,
    );
    expect(
      screen.getByRole("heading", { name: "History", level: 1 }),
    ).toBeInTheDocument();
  });
});
