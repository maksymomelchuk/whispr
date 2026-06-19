import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { LearnedEntry, Mode, Settings } from "@/lib/types";

import {
  deleteLearnedEntry as mockDeleteLearnedEntry,
  getLearnedEntries as mockGetLearnedEntries,
  promoteLearnedEntry as mockPromoteLearnedEntry,
} from "../lib/api";
import { LearnedEntriesPage } from "./LearnedEntriesPage";

vi.mock("../lib/api", () => ({
  getLearnedEntries: vi.fn(),
  deleteLearnedEntry: vi.fn(),
  promoteLearnedEntry: vi.fn(),
  setLearnFromCorrections: vi.fn(),
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

const CANDIDATE_CORRECTION: LearnedEntry = {
  id: "learned-1",
  word: "Tauri",
  kind: "correction",
  from: "tory",
  status: "candidate",
  total_observations: 1,
  last_observed_ms: 1000,
};

const PROMOTED_TERM: LearnedEntry = {
  id: "learned-2",
  word: "GitHub",
  kind: "term",
  status: "promoted",
  total_observations: 2,
  last_observed_ms: 2000,
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
        <LearnedEntriesPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockGetLearnedEntries).mockResolvedValue([]);
  vi.mocked(mockDeleteLearnedEntry).mockResolvedValue(undefined);
  vi.mocked(mockPromoteLearnedEntry).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("LearnedEntriesPage – toggle gating", () => {
  it("does not show entry list when learning is disabled", async () => {
    render(<Wrapper />);
    await waitFor(() => expect(mockGetLearnedEntries).toHaveBeenCalledOnce());
    expect(screen.queryByText("Candidates")).not.toBeInTheDocument();
    expect(screen.queryByText("Ready to use")).not.toBeInTheDocument();
  });

  it("shows empty state when learning is enabled but no entries exist", async () => {
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);
    await waitFor(() =>
      expect(screen.getByText("No learned entries yet")).toBeInTheDocument(),
    );
  });
});

describe("LearnedEntriesPage – entry display", () => {
  beforeEach(() => {
    vi.mocked(mockGetLearnedEntries).mockResolvedValue([
      CANDIDATE_CORRECTION,
      PROMOTED_TERM,
    ]);
  });

  it("renders candidate correction with from→to display and observation count", async () => {
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);
    await waitFor(() => expect(screen.getByText("tory")).toBeInTheDocument());
    expect(screen.getByText("Tauri")).toBeInTheDocument();
    expect(screen.getByText("1×")).toBeInTheDocument();
  });

  it("renders promoted term with a delete control", async () => {
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);
    await waitFor(() => expect(screen.getByText("GitHub")).toBeInTheDocument());
    expect(
      screen.getByRole("button", { name: "Delete GitHub" }),
    ).toBeInTheDocument();
  });

  it("separates candidates and promoted into distinct sections", async () => {
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);
    await waitFor(() =>
      expect(screen.getByText("Candidates")).toBeInTheDocument(),
    );
    expect(screen.getByText("Ready to use")).toBeInTheDocument();
  });
});

describe("LearnedEntriesPage – delete", () => {
  it("optimistically removes the row and shows undo toast on delete", async () => {
    const { toast } = await import("sonner");
    vi.mocked(mockGetLearnedEntries).mockResolvedValue([CANDIDATE_CORRECTION]);
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);

    await waitFor(() => expect(screen.getByText("Tauri")).toBeInTheDocument());

    await userEvent.click(screen.getByRole("button", { name: "Delete Tauri" }));

    expect(screen.queryByText("Tauri")).not.toBeInTheDocument();
    expect(mockDeleteLearnedEntry).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("Tauri"),
      expect.objectContaining({
        action: expect.objectContaining({ label: "Undo" }),
      }),
    );
  });

  it("restores the row when undo is pressed", async () => {
    const { toast } = await import("sonner");
    vi.mocked(mockGetLearnedEntries).mockResolvedValue([CANDIDATE_CORRECTION]);
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);

    await waitFor(() => expect(screen.getByText("Tauri")).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Delete Tauri" }));

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    const action = opts.action as { onClick: () => void };
    action.onClick();

    await waitFor(() => expect(screen.getByText("Tauri")).toBeInTheDocument());
    expect(mockDeleteLearnedEntry).not.toHaveBeenCalled();
  });

  it("commits the delete to the backend when the toast closes without undo", async () => {
    const { toast } = await import("sonner");
    vi.mocked(mockGetLearnedEntries).mockResolvedValue([CANDIDATE_CORRECTION]);
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);

    await waitFor(() => expect(screen.getByText("Tauri")).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Delete Tauri" }));

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onAutoClose as (t: unknown) => void)({});

    await waitFor(() =>
      expect(mockDeleteLearnedEntry).toHaveBeenCalledWith("learned-1"),
    );
  });
});

describe("LearnedEntriesPage – promote", () => {
  it("moves a promoted candidate into the Ready to use section and shows success toast", async () => {
    const { toast } = await import("sonner");
    vi.mocked(mockGetLearnedEntries).mockResolvedValue([CANDIDATE_CORRECTION]);
    render(<Wrapper initial={{ ...BASE, learn_from_corrections: true }} />);

    await waitFor(() =>
      expect(screen.getByText("Candidates")).toBeInTheDocument(),
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Promote to permanent entry" }),
    );

    await waitFor(() =>
      expect(mockPromoteLearnedEntry).toHaveBeenCalledWith("learned-1"),
    );
    expect(screen.getByText("Ready to use")).toBeInTheDocument();
    expect(screen.queryByText("Candidates")).not.toBeInTheDocument();
    expect(toast.success).toHaveBeenCalledWith("Entry activated");
  });
});
