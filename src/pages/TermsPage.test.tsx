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
  ai_cleanup: { enabled: false, prompt_override: null, provider: "anthropic", model: "claude-haiku-4-5" },
  term_set_ids: [],
  correction_set_ids: [],
  use_snippets: true,
  provider_model: { provider: "deepgram" },
};

const BASE: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  hotkey_bindings: [],
  term_sets: [],
  correction_sets: [],
  snippets: [],
  modes: [BASE_MODE],
  default_mode_id: "mode-1",
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
  vi.mocked(mockRenameTermSet).mockResolvedValue(undefined);
  vi.mocked(mockUpdateTermSetEntries).mockResolvedValue(undefined);
  vi.mocked(mockDeleteTermSet).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("TermsPage – empty state", () => {
  it("shows empty card when no term sets exist", () => {
    render(<Wrapper />);
    expect(screen.getByText(/no term sets yet/i)).toBeInTheDocument();
  });
});

describe("TermsPage – create", () => {
  it("creates a term set and shows it in the list", async () => {
    const created: NamedTermSet = {
      id: "ts-new",
      name: "Medical",
      entries: [],
    };
    vi.mocked(mockCreateTermSet).mockResolvedValue(created);

    render(<Wrapper />);
    await userEvent.type(
      screen.getByPlaceholderText("New set name"),
      "Medical",
    );
    await userEvent.click(screen.getByRole("button", { name: /create set/i }));

    await waitFor(() =>
      expect(mockCreateTermSet).toHaveBeenCalledWith("Medical"),
    );
    expect(screen.getByText("Medical")).toBeInTheDocument();
  });

  it("clears the name input after creation", async () => {
    const created: NamedTermSet = { id: "ts-1", name: "Legal", entries: [] };
    vi.mocked(mockCreateTermSet).mockResolvedValue(created);

    render(<Wrapper />);
    const input = screen.getByPlaceholderText("New set name");
    await userEvent.type(input, "Legal");
    await userEvent.click(screen.getByRole("button", { name: /create set/i }));

    await waitFor(() => expect(input).toHaveValue(""));
  });

  it("create button is disabled when name is empty", () => {
    render(<Wrapper />);
    expect(screen.getByRole("button", { name: /create set/i })).toBeDisabled();
  });
});

describe("TermsPage – rename", () => {
  it("renames a set on Enter", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "Originals", entries: [] };
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);

    await userEvent.click(screen.getByRole("button", { name: /rename set/i }));
    const input = screen.getByDisplayValue("Originals");
    await userEvent.clear(input);
    await userEvent.type(input, "Renamed{Enter}");

    await waitFor(() =>
      expect(mockRenameTermSet).toHaveBeenCalledWith("ts-1", "Renamed"),
    );
    expect(screen.getByText("Renamed")).toBeInTheDocument();
  });
});

describe("TermsPage – delete", () => {
  it("shows confirmation dialog and deletes on confirm", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "Old Set", entries: [] };
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);

    await userEvent.click(screen.getByRole("button", { name: /delete set/i }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: /delete/i,
      }),
    );

    await waitFor(() => expect(mockDeleteTermSet).toHaveBeenCalledWith("ts-1"));
    expect(screen.queryByText("Old Set")).not.toBeInTheDocument();
  });

  it("cancel on dialog keeps the set", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "Keep Me", entries: [] };
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);

    await userEvent.click(screen.getByRole("button", { name: /delete set/i }));
    await userEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: /cancel/i,
      }),
    );

    expect(mockDeleteTermSet).not.toHaveBeenCalled();
    expect(screen.getByText("Keep Me")).toBeInTheDocument();
  });

  it("shows affected mode names in the delete dialog", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "Tech Terms", entries: [] };
    const modeWithSet: Mode = { ...BASE_MODE, term_set_ids: ["ts-1"] };
    render(
      <Wrapper
        initial={{
          ...BASE,
          term_sets: [set],
          modes: [modeWithSet],
        }}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: /delete set/i }));

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/Default/)).toBeInTheDocument();
  });
});

describe("TermsPage – expand and edit entries", () => {
  it("expands row to show entry input on click", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "My Set", entries: [] };
    render(<Wrapper initial={{ ...BASE, term_sets: [set] }} />);

    await userEvent.click(screen.getByText("My Set"));

    expect(screen.getByPlaceholderText(/type a term/i)).toBeInTheDocument();
  });

  it("saves entries when a term is committed", async () => {
    const set: NamedTermSet = { id: "ts-1", name: "My Set", entries: [] };
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
  });
});
