import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsContext } from "../context/SettingsContext";
import {
  checkPermissions,
  getHistory,
  getLocalModelStatuses,
} from "../lib/api";
import type { Mode, Settings } from "../lib/types";
import { HomePage } from "./HomePage";

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    onFocusChanged: vi.fn().mockResolvedValue(() => {}),
  })),
}));

vi.mock("../lib/api", () => ({
  checkPermissions: vi.fn(),
  ensurePttStarted: vi.fn().mockResolvedValue(undefined),
  openMicrophoneSettings: vi.fn(),
  openAccessibilitySettings: vi.fn(),
  getHistory: vi.fn().mockResolvedValue([]),
  getLocalModelStatuses: vi.fn().mockResolvedValue([]),
}));

const DEFAULT_MODE: Mode = {
  id: "mode-default-en",
  name: "Default English",
  icon: null,
  language: { kind: "exact", code: "en" },
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
  modes: [DEFAULT_MODE],
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

function Wrapper({ settings = BASE_SETTINGS }: { settings?: Settings }) {
  const [s] = useState<Settings>(settings);
  return (
    <MemoryRouter>
      <SettingsContext.Provider
        value={{
          settings: s,
          setSettings: vi.fn(),
          setSetting: vi.fn(),
          themePreference: "system",
          setThemePreference: vi.fn(),
          accent: "indigo",
          setAccent: vi.fn(),
        }}
      >
        <HomePage />
      </SettingsContext.Provider>
    </MemoryRouter>
  );
}

beforeEach(() => {
  localStorage.clear();
});

describe("HomePage — pending state (permissions not granted)", () => {
  it("shows 'Grant permissions below' subtitle when permissions denied", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: false,
      accessibility: false,
    });
    render(<Wrapper />);
    await waitFor(() => {
      expect(
        screen.getByText("Grant permissions below to get started."),
      ).toBeInTheDocument();
    });
  });

  it("shows the Permissions section when permissions not granted", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: false,
      accessibility: false,
    });
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByText("Microphone")).toBeInTheDocument();
      expect(screen.getByText("Accessibility")).toBeInTheDocument();
    });
  });

  it("does not show the Set up dictation guide when permissions denied", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: false,
      accessibility: false,
    });
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.queryByText("Set up dictation")).not.toBeInTheDocument();
    });
  });
});

describe("HomePage — activating state (permissions granted, no history)", () => {
  it("shows 'Finish setting up.' subtitle when permissions granted and no history", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByText("Finish setting up.")).toBeInTheDocument();
    });
  });

  it("shows 'Permissions granted' collapsed line", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByText("Permissions granted")).toBeInTheDocument();
    });
  });

  it("shows 'Set up dictation' section header", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByText(/set up dictation/i)).toBeInTheDocument();
    });
  });

  it("shows speech model step with 'Set up' action when not configured", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByText("Choose a speech model")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /set up/i }),
      ).toBeInTheDocument();
    });
  });

  it("shows hotkey step with 'Bind' action when no Ptt binding", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(
        screen.getByText("Bind a push-to-talk hotkey"),
      ).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /bind/i })).toBeInTheDocument();
    });
  });

  it("shows the closing instruction", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(
        screen.getByText("Then hold your hotkey and speak."),
      ).toBeInTheDocument();
    });
  });

  it("hides speech model button when Deepgram API key is configured", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, deepgram_api_key_configured: true }}
      />,
    );
    await waitFor(() => {
      expect(
        screen.queryByRole("button", { name: /set up/i }),
      ).not.toBeInTheDocument();
    });
  });

  it("hides hotkey button when a Ptt binding exists", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    render(
      <Wrapper
        settings={{
          ...BASE_SETTINGS,
          hotkey_bindings: [
            {
              shortcut: { key: "AltRight", modifiers: [] },
              action: { type: "Ptt", mode_id: "mode-default-en" },
            },
          ],
        }}
      />,
    );
    await waitFor(() => {
      expect(
        screen.queryByRole("button", { name: /bind/i }),
      ).not.toBeInTheDocument();
    });
  });

  it("hides the guide after dismiss and keeps subtitle", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => {
      expect(screen.getByText(/set up dictation/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /dismiss/i }));

    await waitFor(() => {
      expect(screen.queryByText(/set up dictation/i)).not.toBeInTheDocument();
      expect(screen.getByText("Finish setting up.")).toBeInTheDocument();
    });
  });

  it("guide stays dismissed after remount (navigating away and back)", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    const user = userEvent.setup();
    const { unmount } = render(<Wrapper />);

    await waitFor(() => {
      expect(screen.getByText(/set up dictation/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: /dismiss/i }));
    unmount();

    render(<Wrapper />);

    await waitFor(() => {
      expect(screen.queryByText(/set up dictation/i)).not.toBeInTheDocument();
      expect(screen.getByText("Finish setting up.")).toBeInTheDocument();
    });
  });

  it("shows local model step as done when local model is downloaded", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([]);
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 100,
      },
    ]);
    render(
      <Wrapper
        settings={{
          ...BASE_SETTINGS,
          modes: [
            {
              ...DEFAULT_MODE,
              provider_model: { provider: "local", model: "large_v3_turbo" },
            },
          ],
        }}
      />,
    );
    await waitFor(() => {
      expect(
        screen.queryByRole("button", { name: /set up/i }),
      ).not.toBeInTheDocument();
    });
  });
});

describe("HomePage — activated state (history present)", () => {
  it("shows 'Your voice-to-text is ready.' when history exists", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([
      {
        id: "h1",
        timestamp: 1000,
        speak_duration_ms: 500,
        raw_text: "hello",
        replaced_text: "hello",
        final_text: "hello",
        cleanup_status: { kind: "disabled" },
      },
    ]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(
        screen.getByText("Your voice-to-text is ready."),
      ).toBeInTheDocument();
    });
  });

  it("does not show the Set up guide when history exists", async () => {
    vi.mocked(checkPermissions).mockResolvedValue({
      microphone: true,
      accessibility: true,
    });
    vi.mocked(getHistory).mockResolvedValue([
      {
        id: "h1",
        timestamp: 1000,
        speak_duration_ms: 500,
        raw_text: "hello",
        replaced_text: "hello",
        final_text: "hello",
        cleanup_status: { kind: "disabled" },
      },
    ]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.queryByText(/set up dictation/i)).not.toBeInTheDocument();
    });
  });
});
