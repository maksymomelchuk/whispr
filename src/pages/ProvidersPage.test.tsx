import { render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "../context/SettingsContext";
import type { Settings } from "../lib/types";
import { ProvidersPage } from "./ProvidersPage";

vi.mock("../lib/api", () => ({
  setDeepgramApiKey: vi.fn(),
  setGroqApiKey: vi.fn(),
  setAssemblyAiApiKey: vi.fn(),
  validateDeepgramApiKey: vi.fn(),
  validateGroqApiKey: vi.fn(),
  validateAssemblyAiApiKey: vi.fn(),
  setCleanupAuthMode: vi.fn(),
  setAnthropicApiKey: vi.fn(),
  setAnthropicOauthToken: vi.fn(),
  setCleanupThresholds: vi.fn(),
  getLocalModelStatuses: vi.fn().mockResolvedValue([]),
  startModelDownload: vi.fn(),
  cancelModelDownload: vi.fn(),
  deleteLocalModel: vi.fn(),
  setLocalWhisperIdleTimeout: vi.fn().mockResolvedValue(undefined),
}));

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
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
};

function Wrapper({ settings = BASE_SETTINGS }: { settings?: Settings }) {
  const [s, setRawSettings] = useState<Settings>(settings);
  return (
    <TooltipProvider>
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
        <ProvidersPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

describe("ProvidersPage", () => {
  it("renders a card for each of the three providers", () => {
    render(<Wrapper />);
    expect(screen.getByText("Deepgram")).toBeInTheDocument();
    expect(screen.getByText("Groq")).toBeInTheDocument();
    expect(screen.getByText("AssemblyAI")).toBeInTheDocument();
  });

  it("renders an API key field for each provider", () => {
    render(<Wrapper />);
    const apiKeyLabels = screen.getAllByText("API key");
    expect(apiKeyLabels.length).toBeGreaterThanOrEqual(3);
  });
});

describe("LocalModelsField idle timeout", () => {
  it("shows idle timeout label when local models are present", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      { model: "large_v3_turbo", downloaded: true, downloading: false, size_bytes: 809_000_000 },
    ]);
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByText("Idle timeout")).toBeInTheDocument());
  });

  it("does not show idle timeout when no local models are present", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([]);
    render(<Wrapper />);
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.queryByText("Idle timeout")).not.toBeInTheDocument();
  });
});
