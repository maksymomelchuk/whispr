import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { Settings } from "@/lib/types";

import { MeetingRecorderCard } from "./MeetingRecorderCard";

vi.mock("@/lib/api", () => ({
  startMeeting: vi.fn(),
  stopMeeting: vi.fn(),
  setMeetingSummary: vi.fn(),
  formatShortcut: vi.fn((shortcut) => String(shortcut.key)),
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
  meeting_summary: { provider: "anthropic", model: "claude-sonnet-5-5" },
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
};

const READY_SETTINGS: Settings = {
  ...BASE_SETTINGS,
  soniox_api_key_configured: true,
  ai_cleanup_key_configured: true,
};

function renderCard(settings: Settings) {
  return render(
    <MemoryRouter>
      <TooltipProvider>
        <SettingsContext.Provider
          value={{
            settings,
            setSettings: vi.fn(),
            setSetting: vi.fn(),
            themePreference: "system",
            setThemePreference: vi.fn(),
            accent: "indigo",
            setAccent: vi.fn(),
          }}
        >
          <MeetingRecorderCard recording={null} />
        </SettingsContext.Provider>
      </TooltipProvider>
    </MemoryRouter>,
  );
}

describe("MeetingRecorderCard", () => {
  it("disables Start while the notes provider isn't connected", () => {
    renderCard({
      ...READY_SETTINGS,
      meeting_summary: { provider: "openai", model: "gpt-5.4-mini" },
    });

    expect(
      screen.getByRole("button", { name: /start meeting/i }),
    ).toBeDisabled();
  });

  it("names the missing provider by display name and links to Cleanup", () => {
    renderCard({
      ...READY_SETTINGS,
      meeting_summary: { provider: "openai", model: "gpt-5.4-mini" },
    });

    expect(
      screen.getByText("Connect OpenAI to write the notes."),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Set up" })).toHaveAttribute(
      "href",
      "/ai-providers",
    );
  });

  it("enables Start without a warning once setup is complete", () => {
    renderCard(READY_SETTINGS);

    expect(
      screen.getByRole("button", { name: /start meeting/i }),
    ).toBeEnabled();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows the notes model by name instead of always-open dropdowns", () => {
    renderCard(READY_SETTINGS);

    expect(screen.getByText("Claude Sonnet 5.5")).toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });
});
