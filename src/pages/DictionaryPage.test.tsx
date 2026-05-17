import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsContext, useSettings } from "@/context/SettingsContext";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { Settings } from "@/lib/types";

import { DictionaryPage } from "./DictionaryPage";

vi.mock("../lib/api", () => ({
  setTerms: vi.fn(),
  setCorrections: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

import {
  setTerms as mockSetTerms,
  setCorrections as mockSetCorrections,
} from "../lib/api";

const BASE: Settings = {
  transcription_provider: "deepgram",
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  hotkey_bindings: [],
  terms: [],
  corrections: [],
  snippets: [],
  groq: { model: "whisper_large_v3" },
  modes: [],
  default_mode_id: "",
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

function TermsObserver() {
  const { settings } = useSettings();
  return (
    <output data-testid="terms-count">{settings?.terms?.length ?? 0}</output>
  );
}

function Wrapper({ initial = BASE }: { initial?: Settings }) {
  const [settings, setSettings] = useState<Settings | null>(initial);
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
        <TermsObserver />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

beforeEach(() => {
  vi.mocked(mockSetTerms).mockResolvedValue(undefined);
  vi.mocked(mockSetCorrections).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("Terms tab", () => {
  it("out-of-order persist: later edit wins", async () => {
    let resolveFirst!: () => void;
    const firstDeferred = new Promise<void>((r) => {
      resolveFirst = r;
    });
    vi.mocked(mockSetTerms)
      .mockReturnValueOnce(firstDeferred)
      .mockResolvedValueOnce(undefined);

    render(<Wrapper />);
    const input = screen.getByRole("textbox");

    await userEvent.type(input, "alpha{Enter}");
    await userEvent.type(input, "beta{Enter}");

    // Second request resolves immediately; settings must reflect both terms.
    await waitFor(() =>
      expect(screen.getByTestId("terms-count")).toHaveTextContent("2"),
    );

    // Resolve the stale first request — guard must not revert to 1 term.
    resolveFirst();
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.getByTestId("terms-count")).toHaveTextContent("2");
  });
});

describe("Corrections tab", () => {
  async function switchToCorrections() {
    await userEvent.click(screen.getByText("Corrections"));
  }

  it("add: saves new entry", async () => {
    render(<Wrapper />);
    await switchToCorrections();
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
    await switchToCorrections();
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
    await switchToCorrections();
    await userEvent.click(
      screen.getByRole("button", { name: "Delete correction" }),
    );

    await waitFor(() =>
      expect(mockSetCorrections).toHaveBeenCalledWith([]),
    );
  });

  it("cancel: closes editor without persisting", async () => {
    render(<Wrapper />);
    await switchToCorrections();
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
    await switchToCorrections();
    await userEvent.click(
      screen.getByRole("button", { name: /add correction/i }),
    );
    // EditorRow handles Ctrl+Enter → calls onSave with empty from
    await userEvent.keyboard("{Control>}{Enter}{/Control}");

    expect(
      screen.getByText("Spoken form cannot be empty."),
    ).toBeInTheDocument();
    expect(mockSetCorrections).not.toHaveBeenCalled();
  });
});

describe("Tab switching", () => {
  it("saved corrections survive a round-trip through the Terms tab", async () => {
    render(<Wrapper />);
    await userEvent.click(screen.getByText("Corrections"));

    await userEvent.click(
      screen.getByRole("button", { name: /add correction/i }),
    );
    await userEvent.type(screen.getByPlaceholderText("spoken"), "hi");
    await userEvent.type(screen.getByPlaceholderText("text"), "hello");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByText("hi")).toBeInTheDocument());

    await userEvent.click(screen.getByText("Terms"));
    await userEvent.click(screen.getByText("Corrections"));

    expect(screen.getByText("hi")).toBeInTheDocument();
  });
});
