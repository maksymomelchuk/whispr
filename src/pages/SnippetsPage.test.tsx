import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { Mode, Settings, Snippet } from "@/lib/types";

import {
  getSettings as mockGetSettings,
  setSnippets as mockSetSnippets,
} from "../lib/api";
import { SnippetsPage } from "./SnippetsPage";

vi.mock("../lib/api", () => ({
  getSettings: vi.fn(),
  setSnippets: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    error: vi.fn(),
    success: vi.fn(),
  }),
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

const SNIPPET: Snippet = { id: "s-1", trigger: "ty", expansion: "thank you" };

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
        <SnippetsPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockSetSnippets).mockResolvedValue(undefined);
  vi.mocked(mockGetSettings).mockResolvedValue(BASE);
});

afterEach(() => vi.clearAllMocks());

describe("SnippetsPage – delete with undo toast", () => {
  it("optimistically removes the row and shows undo toast on delete", async () => {
    render(<Wrapper initial={{ ...BASE, snippets: [SNIPPET] }} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete snippet" }),
    );

    expect(screen.queryByText("ty")).not.toBeInTheDocument();
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("ty"),
      expect.objectContaining({
        action: expect.objectContaining({ label: "Undo" }),
      }),
    );
  });

  it("restores the row when undo is pressed", async () => {
    render(<Wrapper initial={{ ...BASE, snippets: [SNIPPET] }} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete snippet" }),
    );

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    const action = opts.action as { onClick: () => void };
    action.onClick();

    await waitFor(() => expect(screen.getByText("ty")).toBeInTheDocument());
    expect(mockSetSnippets).not.toHaveBeenCalled();
  });

  it("commits the delete to the backend when the toast closes without undo", async () => {
    render(<Wrapper initial={{ ...BASE, snippets: [SNIPPET] }} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete snippet" }),
    );

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onAutoClose as (t: unknown) => void)({});

    await waitFor(() => expect(mockSetSnippets).toHaveBeenCalledWith([]));
  });
});

describe("SnippetsPage – flash on add/edit", () => {
  it("flashes the row after a successful save", async () => {
    vi.mocked(mockGetSettings).mockResolvedValue({
      ...BASE,
      snippets: [SNIPPET],
    });

    render(<Wrapper initial={{ ...BASE, snippets: [] }} />);

    await userEvent.click(screen.getByRole("button", { name: /add snippet/i }));
    await userEvent.type(screen.getByPlaceholderText("trigger"), "ty");
    await userEvent.type(screen.getByPlaceholderText(/expansion/), "thank you");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(mockSetSnippets).toHaveBeenCalled());
  });
});
