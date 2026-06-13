import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { Mode, NamedTermSet, Settings } from "@/lib/types";

import {
  createTermSet as mockCreateTermSet,
  deleteTermSet as mockDeleteTermSet,
  renameTermSet as mockRenameTermSet,
  updateTermSetEntries as mockUpdateTermSetEntries,
} from "../lib/api";
import { TermsPage } from "./TermsPage";

vi.mock("../lib/api", () => ({
  createTermSet: vi.fn(),
  renameTermSet: vi.fn(),
  updateTermSetEntries: vi.fn(),
  deleteTermSet: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
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
    post_paste_observation_enabled: false,
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
        <TermsPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockRenameTermSet).mockResolvedValue(BASE);
  vi.mocked(mockUpdateTermSetEntries).mockResolvedValue(BASE);
  vi.mocked(mockDeleteTermSet).mockResolvedValue(BASE);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("TermsPage – vocabulary-specific", () => {
  it("renders the Vocabulary title", () => {
    render(<Wrapper />);
    expect(screen.getByText("Vocabulary")).toBeInTheDocument();
  });

  it("empty state shows 'No term sets yet'", () => {
    render(<Wrapper />);
    expect(screen.getByText(/no term sets yet/i)).toBeInTheDocument();
  });

  it("calls createTermSet with the entered name", async () => {
    vi.mocked(mockCreateTermSet).mockResolvedValue({
      ...BASE,
      term_sets: [{ id: "ts-new", name: "Medical", entries: [] }],
    });
    render(<Wrapper />);
    await userEvent.type(
      screen.getByPlaceholderText("New set name"),
      "Medical",
    );
    await userEvent.click(screen.getByRole("button", { name: /create set/i }));
    await waitFor(() =>
      expect(mockCreateTermSet).toHaveBeenCalledWith("Medical"),
    );
  });

  it("shows affected profile names from term_set_ids", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "Tech Terms", entries: [] };
    const modeWithSet: Mode = { ...BASE_MODE, term_set_ids: ["ts-1"] };
    render(
      <Wrapper initial={{ ...BASE, term_sets: [set], modes: [modeWithSet] }} />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Delete set" }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/Default/)).toBeInTheDocument();
  });

  it("shows TermChipInput when a set is expanded", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "My Set", entries: [] };
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);
    await userEvent.click(screen.getByText("My Set"));
    expect(screen.getByPlaceholderText(/type a term/i)).toBeInTheDocument();
  });

  it("saves entries when a term is committed", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "My Set", entries: [] };
    vi.mocked(mockUpdateTermSetEntries).mockResolvedValue({
      ...BASE,
      term_sets: [{ id: "ts-1", name: "My Set", entries: ["MongoDB"] }],
    });
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);
    await userEvent.click(screen.getByText("My Set"));
    await userEvent.type(
      screen.getByPlaceholderText(/type a term/i),
      "MongoDB{Enter}",
    );
    await waitFor(() =>
      expect(mockUpdateTermSetEntries).toHaveBeenCalledWith("ts-1", [
        "MongoDB",
      ]),
    );
    expect(screen.getByText("MongoDB")).toBeInTheDocument();
  });
});
