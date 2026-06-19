import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { Mode, NamedCorrectionSet, Settings } from "@/lib/types";

import {
  createCorrectionSet as mockCreateCorrectionSet,
  deleteCorrectionSet as mockDeleteCorrectionSet,
  renameCorrectionSet as mockRenameCorrectionSet,
  updateCorrectionSetEntries as mockUpdateCorrectionSetEntries,
} from "../lib/api";
import { CorrectionsPage } from "./CorrectionsPage";

vi.mock("../lib/api", () => ({
  createCorrectionSet: vi.fn(),
  renameCorrectionSet: vi.fn(),
  updateCorrectionSetEntries: vi.fn(),
  deleteCorrectionSet: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn().mockReturnValue("toast-id"), dismiss: vi.fn() },
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

const BASE: Settings = {
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
  modes: [BASE_MODE],
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 3,
  ai_cleanup_min_duration_ms: 1000,
  ai_cleanup_tone_overlay_enabled: false,
  tone_app_overrides: {},
  tone_app_custom_prompts: {},
  learn_from_corrections: false,
  input_device: null,
  pause_media_on_record: false,
  history_limit: null,
  save_audio_recordings: false,
  hands_free_max_minutes: 30,
  show_in_dock: true,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
  configured_providers: [],
  custom_provider_configured: false,
  custom_provider_base_url: null,
  custom_provider_model: "",
};

function Wrapper({ initial = BASE }: { initial?: Settings }) {
  const [settings, setRawSettings] = useState<Settings>(initial);
  const setSettings = (updater: (prev: Settings) => Settings) =>
    setRawSettings(updater);
  return (
    <TooltipProvider>
      <SettingsContext.Provider
        value={{
          settings,
          setSettings,
          setSetting: vi.fn(),
          themePreference: "system",
          setThemePreference: vi.fn(),
          accent: "indigo",
          setAccent: vi.fn(),
        }}
      >
        <CorrectionsPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockCreateCorrectionSet).mockResolvedValue({
    ...BASE,
    correction_sets: [{ id: "correction-set-123", name: "", entries: [] }],
  });
  vi.mocked(mockRenameCorrectionSet).mockResolvedValue(BASE);
  vi.mocked(mockUpdateCorrectionSetEntries).mockResolvedValue(BASE);
  vi.mocked(mockDeleteCorrectionSet).mockResolvedValue(BASE);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("CorrectionsPage – corrections-specific", () => {
  it("renders the Corrections title", () => {
    render(<Wrapper />);
    expect(screen.getByText("Corrections")).toBeInTheDocument();
  });

  it("empty state shows 'spoken → text' preview", () => {
    render(<Wrapper />);
    expect(screen.getByText(/spoken → text/)).toBeInTheDocument();
  });

  it("calls createCorrectionSet with the entered name", async () => {
    vi.mocked(mockCreateCorrectionSet).mockResolvedValue({
      ...BASE,
      correction_sets: [
        { id: "correction-set-123", name: "My Rules", entries: [] },
      ],
    });
    render(<Wrapper />);
    await userEvent.click(
      screen.getByRole("button", { name: /new correction set/i }),
    );
    const input = screen.getByPlaceholderText("Set name");
    await userEvent.type(input, "My Rules");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() =>
      expect(mockCreateCorrectionSet).toHaveBeenCalledWith("My Rules"),
    );
  });

  it("shows affected profile names from correction_set_ids", async () => {
    const set: NamedCorrectionSet = {
      id: "cs-1",
      name: "Punctuation",
      entries: [],
    };
    const modeWithSet: Mode = {
      ...BASE_MODE,
      id: "mode-2",
      name: "Writing",
      correction_set_ids: ["cs-1"],
    };
    render(
      <Wrapper
        initial={{
          ...BASE,
          correction_sets: [set],
          modes: [BASE_MODE, modeWithSet],
        }}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));
    expect(screen.getByText(/Writing/)).toBeInTheDocument();
  });

  it("shows EntriesEditor when a set is opened", async () => {
    const set: NamedCorrectionSet = { id: "cs-1", name: "Tech", entries: [] };
    render(<Wrapper initial={{ ...BASE, correction_sets: [set] }} />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(
      screen.getByRole("button", { name: /add rule/i }),
    ).toBeInTheDocument();
  });

  it("shows a Retry action on entries save failure", async () => {
    vi.mocked(mockUpdateCorrectionSetEntries).mockRejectedValueOnce(
      new Error("network error"),
    );
    const set: NamedCorrectionSet = { id: "cs-1", name: "Tech", entries: [] };
    render(<Wrapper initial={{ ...BASE, correction_sets: [set] }} />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    await userEvent.click(screen.getByRole("button", { name: /add rule/i }));
    await userEvent.type(screen.getByPlaceholderText("spoken"), "ty");
    await userEvent.type(screen.getByPlaceholderText("text"), "TypeScript");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    const { toast } = await import("sonner");
    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "Couldn't save entries",
        expect.objectContaining({
          action: expect.objectContaining({ label: "Retry" }),
        }),
      ),
    );
  });

  it("saves new correction entry via updateCorrectionSetEntries", async () => {
    vi.mocked(mockUpdateCorrectionSetEntries).mockResolvedValue({
      ...BASE,
      correction_sets: [
        {
          id: "cs-1",
          name: "Tech",
          entries: [{ from: "mongo", to: "MongoDB" }],
        },
      ],
    });
    const set: NamedCorrectionSet = { id: "cs-1", name: "Tech", entries: [] };
    render(<Wrapper initial={{ ...BASE, correction_sets: [set] }} />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    await userEvent.click(screen.getByRole("button", { name: /add rule/i }));
    await userEvent.type(screen.getByPlaceholderText("spoken"), "mongo");
    await userEvent.type(screen.getByPlaceholderText("text"), "MongoDB");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(mockUpdateCorrectionSetEntries).toHaveBeenCalledWith("cs-1", [
        { from: "mongo", to: "MongoDB" },
      ]),
    );
    expect(screen.getByText("mongo")).toBeInTheDocument();
  });
});
