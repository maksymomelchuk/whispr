import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsContext } from "@/context/SettingsContext";
import type { Settings } from "@/lib/types";

import { setCorrections as mockSetCorrections } from "../lib/api";
import { DictionaryPage } from "./DictionaryPage";

vi.mock("../lib/api", () => ({
  setCorrections: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

const BASE: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  hotkey_bindings: [],
  term_sets: [],
  corrections: [],
  snippets: [],
  modes: [],
  default_mode_id: "",
  ai_cleanup_enabled: true,
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 3,
  ai_cleanup_min_duration_ms: 1000,
  input_device: null,
  pause_media_on_record: false,
  history_limit: null,
  show_in_dock: true,
  show_live_preview: true,
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
        <DictionaryPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockSetCorrections).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("DictionaryPage – corrections", () => {
  it("add: saves new entry", async () => {
    render(<Wrapper />);
    await userEvent.click(
      screen.getByRole("button", { name: /add correction/i }),
    );
    await userEvent.type(screen.getByPlaceholderText("spoken"), "hello");
    await userEvent.type(screen.getByPlaceholderText("text"), "world");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(mockSetCorrections).toHaveBeenCalledWith([
        { from: "hello", to: "world" },
      ]),
    );
    expect(screen.getByText("hello")).toBeInTheDocument();
  });

  it("edit: updates existing entry", async () => {
    render(
      <Wrapper
        initial={{ ...BASE, corrections: [{ from: "hey", to: "hello" }] }}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: "Edit" }));
    const fromInput = screen.getByPlaceholderText("spoken");
    await userEvent.clear(fromInput);
    await userEvent.type(fromInput, "hi");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(mockSetCorrections).toHaveBeenCalledWith([
        { from: "hi", to: "hello" },
      ]),
    );
  });

  it("delete: removes entry", async () => {
    render(
      <Wrapper
        initial={{ ...BASE, corrections: [{ from: "foo", to: "bar" }] }}
      />,
    );
    await userEvent.click(
      screen.getByRole("button", { name: "Delete correction" }),
    );

    await waitFor(() => expect(mockSetCorrections).toHaveBeenCalledWith([]));
  });

  it("cancel: closes editor without persisting", async () => {
    render(<Wrapper />);
    await userEvent.click(
      screen.getByRole("button", { name: /add correction/i }),
    );
    await userEvent.type(screen.getByPlaceholderText("spoken"), "oops");
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(mockSetCorrections).not.toHaveBeenCalled();
    expect(screen.queryByPlaceholderText("spoken")).not.toBeInTheDocument();
  });

  it("empty-from validation: shows error, blocks persist", async () => {
    render(<Wrapper />);
    await userEvent.click(
      screen.getByRole("button", { name: /add correction/i }),
    );
    await userEvent.keyboard("{Control>}{Enter}{/Control}");

    expect(
      screen.getByText("Spoken form cannot be empty."),
    ).toBeInTheDocument();
    expect(mockSetCorrections).not.toHaveBeenCalled();
  });
});
