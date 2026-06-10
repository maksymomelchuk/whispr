import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsContext } from "@/context/SettingsContext";
import type { HistoryEntry, Settings } from "@/lib/types";

import {
  getHistory as mockGetHistory,
  recoverCleanup as mockRecoverCleanup,
  updateHistoryEntry as mockUpdateHistoryEntry,
} from "../lib/api";
import { HistoryTab } from "./HistoryTab";

vi.mock("../lib/api", () => ({
  getHistory: vi.fn(),
  clearHistory: vi.fn(),
  setHistoryLimit: vi.fn(),
  recoverCleanup: vi.fn(),
  updateHistoryEntry: vi.fn(),
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
  configured_providers: [],
  custom_provider_configured: false,
  custom_provider_base_url: null,
  custom_provider_model: "",
  ai_cleanup_min_words: 3,
  ai_cleanup_min_duration_ms: 1000,
  input_device: null,
  pause_media_on_record: false,
  history_limit: 100,
  show_in_dock: true,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
};

const NOW_SECONDS = Math.floor(Date.now() / 1000);

const RECOVERABLE_ENTRY: HistoryEntry = {
  id: "entry-123",
  timestamp: NOW_SECONDS,
  speak_duration_ms: 5000,
  raw_text: "raw transcript",
  replaced_text: "raw transcript",
  final_text: "raw transcript",
  cleanup_status: { kind: "failed_timeout" },
  profile_snapshot: {
    cleanup_provider: "anthropic",
    cleanup_model: "claude-haiku-4-5",
    cleanup_prompt_override: null,
    use_snippets: false,
    correction_set_ids: [],
  },
  provider_model: null,
  app_name: null,
  bundle_id: null,
};

const RAN_ENTRY: HistoryEntry = {
  id: "entry-456",
  timestamp: NOW_SECONDS - 1,
  speak_duration_ms: 3000,
  raw_text: "raw text",
  replaced_text: "cleaned text",
  final_text: "cleaned text",
  cleanup_status: { kind: "ran" },
  profile_snapshot: null,
  provider_model: null,
  app_name: null,
  bundle_id: null,
};

function Wrapper({ settings = BASE_SETTINGS }: { settings?: Settings }) {
  const [s, setRawSettings] = useState<Settings>(settings);
  return (
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
      <HistoryTab />
    </SettingsContext.Provider>
  );
}

describe("HistoryTab Recover button", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows Recover button only on recoverable entries", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RECOVERABLE_ENTRY, RAN_ENTRY]);
    render(<Wrapper />);

    await waitFor(() => screen.getByText("raw transcript"));

    const recoverButtons = screen.queryAllByRole("button", { name: "Recover" });
    expect(recoverButtons).toHaveLength(1);
  });

  it("hides Recover button when entry has no id", async () => {
    const entryNoId: HistoryEntry = { ...RECOVERABLE_ENTRY, id: "" };
    vi.mocked(mockGetHistory).mockResolvedValue([entryNoId]);
    render(<Wrapper />);

    await waitFor(() => screen.getByText("raw transcript"));

    expect(
      screen.queryByRole("button", { name: "Recover" }),
    ).not.toBeInTheDocument();
  });

  it("hides Recover button when entry has no profile snapshot", async () => {
    const entryNoSnap: HistoryEntry = {
      ...RECOVERABLE_ENTRY,
      profile_snapshot: null,
    };
    vi.mocked(mockGetHistory).mockResolvedValue([entryNoSnap]);
    render(<Wrapper />);

    await waitFor(() => screen.getByText("raw transcript"));

    expect(
      screen.queryByRole("button", { name: "Recover" }),
    ).not.toBeInTheDocument();
  });

  it("shows disabled Recovering button while recovery is in flight", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RECOVERABLE_ENTRY]);
    let resolveRecover!: (value: string) => void;
    vi.mocked(mockRecoverCleanup).mockReturnValue(
      new Promise((res) => {
        resolveRecover = res;
      }),
    );

    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Recover" }));
    await user.click(screen.getByRole("button", { name: "Recover" }));

    const inFlight = screen.getByRole("button", { name: "Recovering…" });
    expect(inFlight).toBeDisabled();

    resolveRecover("recovered text");
    await waitFor(() => screen.getByRole("button", { name: "Recover" }));
  });

  it("copies recovered text to clipboard on success", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RECOVERABLE_ENTRY]);
    vi.mocked(mockRecoverCleanup).mockResolvedValue("recovered text");

    const user = userEvent.setup();
    const writeSpy = vi.spyOn(navigator.clipboard, "writeText");

    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Recover" }));
    await user.click(screen.getByRole("button", { name: "Recover" }));

    await waitFor(() => {
      expect(writeSpy).toHaveBeenCalledWith("recovered text");
    });
  });

  it("shows inline error and keeps Recover button available on failure", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RECOVERABLE_ENTRY]);
    vi.mocked(mockRecoverCleanup).mockRejectedValue(
      new Error("cleanup timed out"),
    );

    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Recover" }));
    await user.click(screen.getByRole("button", { name: "Recover" }));

    await waitFor(() => {
      expect(screen.getByText(/cleanup timed out/i)).toBeInTheDocument();
    });

    const recoverBtn = screen.getByRole("button", { name: "Recover" });
    expect(recoverBtn).toBeInTheDocument();
    expect(recoverBtn).not.toBeDisabled();
  });
});

describe("HistoryTab entry editing", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows Edit button on entries with an id", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RAN_ENTRY]);
    render(<Wrapper />);

    await waitFor(() => screen.getByText("cleaned text"));

    expect(screen.getByRole("button", { name: "Edit" })).toBeInTheDocument();
  });

  it("does not show Edit button on entries without an id", async () => {
    const entryNoId: HistoryEntry = { ...RAN_ENTRY, id: "" };
    vi.mocked(mockGetHistory).mockResolvedValue([entryNoId]);
    render(<Wrapper />);

    await waitFor(() => screen.getByText("cleaned text"));

    expect(
      screen.queryByRole("button", { name: "Edit" }),
    ).not.toBeInTheDocument();
  });

  it("shows a textarea with current text when Edit is clicked", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RAN_ENTRY]);
    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Edit" }));

    const textarea = screen.getByRole("textbox");
    expect(textarea).toBeInTheDocument();
    expect(textarea).toHaveValue("cleaned text");
  });

  it("saves the edited text and updates the list", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RAN_ENTRY]);
    vi.mocked(mockUpdateHistoryEntry).mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Edit" }));

    const textarea = screen.getByRole("textbox");
    await user.clear(textarea);
    await user.type(textarea, "edited text");

    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(mockUpdateHistoryEntry).toHaveBeenCalledWith(
        "entry-456",
        "cleaned text",
        "edited text",
      );
    });

    expect(screen.getByText("edited text")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("cancels editing and leaves the entry unchanged", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RAN_ENTRY]);
    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Edit" }));

    const textarea = screen.getByRole("textbox");
    await user.clear(textarea);
    await user.type(textarea, "should not be saved");

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.getByText("cleaned text")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(mockUpdateHistoryEntry).not.toHaveBeenCalled();
  });

  it("shows inline error and keeps edit mode on save failure", async () => {
    vi.mocked(mockGetHistory).mockResolvedValue([RAN_ENTRY]);
    vi.mocked(mockUpdateHistoryEntry).mockRejectedValue(
      new Error("save failed"),
    );
    const user = userEvent.setup();
    render(<Wrapper />);

    await waitFor(() => screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Edit" }));

    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(screen.getByText(/save failed/i)).toBeInTheDocument();
    });

    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });
});
