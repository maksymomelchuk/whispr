import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  checkSpeechProviderKey,
  getLocalModelStatuses,
  listInputDevices,
} from "../lib/api";
import type { Mode, Settings } from "../lib/types";
import { SettingsContext } from "./SettingsContext";
import { SystemStatusProvider, useSystemStatus } from "./SystemStatusContext";

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({
    onFocusChanged: vi.fn().mockResolvedValue(() => {}),
  })),
}));

vi.mock("../lib/api", () => ({
  listInputDevices: vi.fn().mockResolvedValue([]),
  getLocalModelStatuses: vi.fn().mockResolvedValue([]),
  checkSpeechProviderKey: vi.fn().mockResolvedValue({ kind: "valid" }),
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

const DEEPGRAM_MODE: Mode = {
  id: "mode-1",
  name: "Default",
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

function StatusReader() {
  const { micMissing, loadFailedModels, speechProviderStatuses } =
    useSystemStatus();
  return (
    <div>
      <span data-testid="mic-missing">{String(micMissing)}</span>
      <span data-testid="load-failed-count">{loadFailedModels.size}</span>
      <span data-testid="deepgram-status">
        {speechProviderStatuses.get("deepgram") ?? "none"}
      </span>
    </div>
  );
}

function Wrapper({ settings = BASE_SETTINGS }: { settings?: Settings }) {
  return (
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
      <SystemStatusProvider>
        <StatusReader />
      </SystemStatusProvider>
    </SettingsContext.Provider>
  );
}

beforeEach(() => {
  vi.mocked(listInputDevices).mockResolvedValue([]);
  vi.mocked(getLocalModelStatuses).mockResolvedValue([]);
  vi.mocked(checkSpeechProviderKey).mockResolvedValue({ kind: "valid" });
});

describe("SystemStatusContext", () => {
  it("micMissing is false when no input device is configured", async () => {
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByTestId("mic-missing").textContent).toBe("false");
    });
  });

  it("micMissing is true when configured device is not in device list", async () => {
    vi.mocked(listInputDevices).mockResolvedValue(["Built-in Microphone"]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, input_device: "Sony Microphone" }}
      />,
    );
    await waitFor(() => {
      expect(screen.getByTestId("mic-missing").textContent).toBe("true");
    });
  });

  it("micMissing is false when configured device is found in device list", async () => {
    vi.mocked(listInputDevices).mockResolvedValue(["Sony Microphone"]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, input_device: "Sony Microphone" }}
      />,
    );
    await waitFor(() => {
      expect(screen.getByTestId("mic-missing").textContent).toBe("false");
    });
  });

  it("loadFailedModels contains a model whose load_failed is true", async () => {
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        load_failed: true,
        downloading: false,
        size_bytes: 1_624_555_275,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() => {
      expect(screen.getByTestId("load-failed-count").textContent).toBe("1");
    });
  });

  it("maps 'invalid' API key response to 'rejected' provider status", async () => {
    vi.mocked(checkSpeechProviderKey).mockResolvedValue({ kind: "invalid" });
    render(<Wrapper settings={{ ...BASE_SETTINGS, modes: [DEEPGRAM_MODE] }} />);
    await waitFor(() => {
      expect(screen.getByTestId("deepgram-status").textContent).toBe(
        "rejected",
      );
    });
  });

  it("maps 'error' API key response to 'unreachable' provider status", async () => {
    vi.mocked(checkSpeechProviderKey).mockResolvedValue({
      kind: "error",
      message: "Network error",
    });
    render(<Wrapper settings={{ ...BASE_SETTINGS, modes: [DEEPGRAM_MODE] }} />);
    await waitFor(() => {
      expect(screen.getByTestId("deepgram-status").textContent).toBe(
        "unreachable",
      );
    });
  });
});
