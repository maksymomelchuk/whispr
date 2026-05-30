import { render, screen } from "@testing-library/react";
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
  validateDeepgramApiKey: vi.fn(),
  validateGroqApiKey: vi.fn(),
  validateAssemblyAiApiKey: vi.fn(),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div role="dialog">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DialogFooter: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DialogTitle: ({ children }: { children: React.ReactNode }) => <h2>{children}</h2>,
  DialogDescription: ({ children }: { children: React.ReactNode }) => <p>{children}</p>,
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
  });

  it("shows Setup badge when no engines are configured", () => {
    render(<Wrapper />);
    const setupBadges = screen.getAllByText("Setup");
    expect(setupBadges).toHaveLength(3);
  });

  it("shows Configured badge when an engine is configured", () => {
    render(
      <Wrapper settings={{ ...BASE_SETTINGS, deepgram_api_key_configured: true }} />,
    );
    expect(screen.getByText("Configured")).toBeInTheDocument();
  });

  it("renders the Cloud section heading", () => {
    render(<Wrapper />);
    expect(screen.getByText("Cloud")).toBeInTheDocument();
  });
});
