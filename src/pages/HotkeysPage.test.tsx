import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { Mode, Settings } from "@/lib/types";

import { SettingsContext } from "../context/SettingsContext";
import { HotkeysPage } from "./HotkeysPage";

vi.mock("../hooks/usePtt", () => ({
  usePtt: () => ({ activeShortcut: null, isHeld: false }),
}));

vi.mock("../lib/api", () => ({
  setHotkeyBindings: vi.fn().mockResolvedValue(undefined),
  setShortcutCapturePaused: vi.fn().mockResolvedValue(undefined),
  formatShortcut: vi.fn((s) => String(s.key)),
}));

const BASE_MODE: Mode = {
  id: "mode-1",
  name: "Default",
  icon: null,
  language: { kind: "auto" },
  ai_cleanup: {
    enabled: false,
    prompt_override: null,
    provider: "anthropic",
    model: "claude-haiku-4-5",
    paste_raw_on_failure: true,
    context_capture_enabled: false,
  },
  term_set_ids: [],
  correction_set_ids: [],
  use_snippets: true,
  provider_model: { provider: "deepgram" },
};

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
};

function HotkeysPageWrapper({ settings }: { settings: Settings }) {
  return (
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
          <HotkeysPage />
        </SettingsContext.Provider>
      </TooltipProvider>
    </MemoryRouter>
  );
}

const BINDING_SETTINGS: Settings = {
  ...BASE_SETTINGS,
  modes: [BASE_MODE],
  hotkey_bindings: [
    {
      shortcut: { key: "AltRight", modifiers: [] },
      action: { type: "Ptt", mode_id: "mode-1" },
    },
  ],
};

describe("HotkeysPage – binding row accessibility", () => {
  it("renders Re-record and Remove binding buttons when a binding exists", () => {
    render(<HotkeysPageWrapper settings={BINDING_SETTINGS} />);
    expect(
      screen.getByRole("button", { name: "Re-record" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Remove binding" }),
    ).toBeInTheDocument();
  });

  it("Re-record and Remove buttons are not disabled", () => {
    render(<HotkeysPageWrapper settings={BINDING_SETTINGS} />);
    expect(
      screen.getByRole("button", { name: "Re-record" }),
    ).not.toBeDisabled();
    expect(
      screen.getByRole("button", { name: "Remove binding" }),
    ).not.toBeDisabled();
  });
});

describe("HotkeysPage – empty states", () => {
  it("shows an EmptyPanel with teaching copy when no profiles exist", () => {
    render(<HotkeysPageWrapper settings={BASE_SETTINGS} />);
    expect(screen.getByText("No profiles yet")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Create a profile first, then come back to bind a push-to-talk hotkey.",
      ),
    ).toBeInTheDocument();
  });

  it("does not show the no-profiles panel when profiles exist", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      modes: [BASE_MODE],
    };
    render(<HotkeysPageWrapper settings={settings} />);
    expect(screen.queryByText("No profiles yet")).not.toBeInTheDocument();
  });

  it("shows EmptyRowCard with add action when a profile has no hotkeys", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      modes: [BASE_MODE],
      hotkey_bindings: [],
    };
    render(<HotkeysPageWrapper settings={settings} />);
    const actions = screen.getAllByText("Add hotkey");
    expect(actions.length).toBeGreaterThan(0);
  });
});
