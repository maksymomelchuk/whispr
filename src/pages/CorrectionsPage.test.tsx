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

describe("CorrectionsPage – set creation", () => {
  it("empty state shows create action", () => {
    render(<Wrapper />);
    expect(
      screen.getByRole("button", { name: /new correction set/i }),
    ).toBeInTheDocument();
  });

  it("create: saves set and expands it", async () => {
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
    expect(screen.getByText("My Rules")).toBeInTheDocument();
  });

  it("create: cancel clears the inline form", async () => {
    render(<Wrapper />);
    await userEvent.click(
      screen.getByRole("button", { name: /new correction set/i }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByPlaceholderText("Set name")).not.toBeInTheDocument();
  });
});

describe("CorrectionsPage – set deletion", () => {
  const SET: NamedCorrectionSet = {
    id: "cs-1",
    name: "Punctuation",
    entries: [{ from: "dot", to: "." }],
  };

  it("delete: shows confirm dialog and removes on confirm", async () => {
    vi.mocked(mockDeleteCorrectionSet).mockResolvedValue({
      ...BASE,
      correction_sets: [],
    });
    render(<Wrapper initial={{ ...BASE, correction_sets: [SET] }} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete correction set" }),
    );
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockDeleteCorrectionSet).toHaveBeenCalledWith("cs-1"),
    );
    expect(screen.queryByText("Punctuation")).not.toBeInTheDocument();
  });

  it("delete with affected modes: dialog names the modes", async () => {
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
          correction_sets: [SET],
          modes: [BASE_MODE, modeWithSet],
        }}
      />,
    );

    await userEvent.click(
      screen.getByRole("button", { name: "Delete correction set" }),
    );

    expect(screen.getByText(/Writing/)).toBeInTheDocument();
  });

  it("delete: cancel leaves set intact", async () => {
    render(<Wrapper initial={{ ...BASE, correction_sets: [SET] }} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Delete correction set" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(mockDeleteCorrectionSet).not.toHaveBeenCalled();
    expect(screen.getByText("Punctuation")).toBeInTheDocument();
  });

  it("delete: applies returned Settings verbatim, including cascade removals", async () => {
    const setA: NamedCorrectionSet = {
      id: "cs-1",
      name: "Rules A",
      entries: [],
    };
    const setB: NamedCorrectionSet = {
      id: "cs-2",
      name: "Rules B",
      entries: [],
    };
    vi.mocked(mockDeleteCorrectionSet).mockResolvedValue({
      ...BASE,
      correction_sets: [],
    });

    render(<Wrapper initial={{ ...BASE, correction_sets: [setA, setB] }} />);

    await userEvent.click(
      screen.getAllByRole("button", { name: "Delete correction set" })[0],
    );
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockDeleteCorrectionSet).toHaveBeenCalledWith("cs-1"),
    );
    expect(screen.queryByText("Rules A")).not.toBeInTheDocument();
    expect(screen.queryByText("Rules B")).not.toBeInTheDocument();
  });
});

describe("CorrectionsPage – entry management", () => {
  const SET: NamedCorrectionSet = {
    id: "cs-1",
    name: "Tech",
    entries: [],
  };

  it("add rule: saves new entry on Save", async () => {
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
    render(<Wrapper initial={{ ...BASE, correction_sets: [SET] }} />);

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
