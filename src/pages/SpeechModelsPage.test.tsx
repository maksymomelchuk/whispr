import { render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";

import { SettingsContext } from "../context/SettingsContext";
import type { Settings } from "../lib/types";
import { SpeechModelsPage } from "./SpeechModelsPage";

vi.mock("../lib/api", () => ({
  setDeepgramApiKey: vi.fn(),
  setGroqApiKey: vi.fn(),
  setAssemblyAiApiKey: vi.fn(),
  setOpenaiApiKey: vi.fn(),
  setElevenLabsApiKey: vi.fn(),
  validateDeepgramApiKey: vi.fn(),
  validateGroqApiKey: vi.fn(),
  validateAssemblyAiApiKey: vi.fn(),
  validateOpenaiApiKey: vi.fn(),
  validateElevenLabsApiKey: vi.fn(),
  getLocalModelStatuses: vi.fn().mockResolvedValue([]),
  startModelDownload: vi.fn(),
  cancelModelDownload: vi.fn(),
  deleteLocalModel: vi.fn(),
  getLocalModelPath: vi.fn(),
  setLocalWhisperIdleTimeout: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div role="dialog">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h2>{children}</h2>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
  DialogClose: ({ children, onClick, ...props }: any) => (
    <button type="button" onClick={onClick} {...props}>
      {children}
    </button>
  ),
}));

const BASE_SETTINGS: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  openai_api_key_configured: false,
  elevenlabs_api_key_configured: false,
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
        <SpeechModelsPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

describe("SpeechModelsPage", () => {
  it("renders a card for each engine in the catalog", () => {
    render(<Wrapper />);
    expect(screen.getByText("Deepgram")).toBeInTheDocument();
    expect(screen.getByText("Groq")).toBeInTheDocument();
    expect(screen.getByText("AssemblyAI")).toBeInTheDocument();
    expect(screen.getByText("OpenAI")).toBeInTheDocument();
    expect(screen.getByText("ElevenLabs")).toBeInTheDocument();
  });

  it("marks every engine as needing setup when none are configured", () => {
    render(<Wrapper />);
    expect(screen.getAllByRole("img", { name: "Set up" })).toHaveLength(5);
  });

  it("marks an engine as configured when it has a key", () => {
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, deepgram_api_key_configured: true }}
      />,
    );
    expect(screen.getByRole("img", { name: "Configured" })).toBeInTheDocument();
  });

  it("renders the Cloud section heading", () => {
    render(<Wrapper />);
    expect(screen.getByText("Cloud")).toBeInTheDocument();
  });
});

describe("SpeechModelsPage LOCAL section", () => {
  it("hides LOCAL section when no local models exist", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([]);
    render(<Wrapper />);
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.queryByText("Local")).not.toBeInTheDocument();
  });

  it("shows LOCAL section heading when local models exist", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 1_624_555_275,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() => expect(screen.getByText("Local")).toBeInTheDocument());
  });

  it("renders a card for each local model", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3",
        downloaded: false,
        downloading: false,
        size_bytes: 3_115_853_312,
      },
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 1_624_555_275,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() =>
      expect(screen.getByText("Large v3")).toBeInTheDocument(),
    );
    expect(screen.getByText("Large v3 Turbo")).toBeInTheDocument();
  });

  it("shows idle timeout control when local models exist", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 1_624_555_275,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() =>
      expect(screen.getByText("Idle timeout")).toBeInTheDocument(),
    );
  });

  it("shows download button for a not-downloaded model", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3",
        downloaded: false,
        downloading: false,
        size_bytes: 3_115_853_312,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Download model" }),
      ).toBeInTheDocument(),
    );
  });

  it("shows action buttons for a downloaded model", async () => {
    const { getLocalModelStatuses } = await import("../lib/api");
    vi.mocked(getLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 1_624_555_275,
      },
    ]);
    render(<Wrapper />);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Show in Finder" }),
      ).toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", { name: "Delete model" }),
    ).toBeInTheDocument();
  });
});
