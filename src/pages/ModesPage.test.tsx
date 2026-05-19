import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { Mode, NamedCorrectionSet, Settings } from "@/lib/types";

import { SettingsContext } from "../context/SettingsContext";
import {
  addMode as mockAddMode,
  updateMode as mockUpdateMode,
} from "../lib/api";
import { ModeEditor, ModesPage } from "./ModesPage";

vi.mock("@/components/ui/sheet", () => ({
  Sheet: ({ children }: any) => <div>{children}</div>,
  SheetContent: ({ children, ...p }: any) => <div {...p}>{children}</div>,
  SheetHeader: ({ children, ...p }: any) => <div {...p}>{children}</div>,
  SheetTitle: ({ children, ...p }: any) => <h2 {...p}>{children}</h2>,
  SheetFooter: ({ children, ...p }: any) => <div {...p}>{children}</div>,
}));

vi.mock("../lib/api", () => ({
  updateMode: vi.fn(),
  addMode: vi.fn(),
  deleteMode: vi.fn(),
  duplicateMode: vi.fn(),
  getSettings: vi.fn(),
  setDefaultMode: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

const MODE: Mode = {
  id: "mode-1",
  name: "Original",
  icon: null,
  language: { kind: "auto" },
  translate: { kind: "off" },
  ai_cleanup: { enabled: false, prompt_override: null },
  term_set_ids: [],
  correction_set_ids: [],
  use_snippets: true,
  provider_model: { provider: "deepgram" },
};

function EditorWrapper({
  mode = MODE,
  isNew = false,
  onClose = vi.fn(),
  onPersist = vi.fn(),
}: {
  mode?: Mode;
  isNew?: boolean;
  onClose?: () => void;
  onPersist?: (m: Mode, wasNew: boolean) => void;
}) {
  return (
    <ModeEditor
      mode={mode}
      isNew={isNew}
      onClose={onClose}
      onPersist={onPersist}
    />
  );
}

beforeEach(() => {
  vi.mocked(mockUpdateMode).mockResolvedValue(undefined);
  vi.mocked(mockAddMode).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("ModeEditor – autosave", () => {
  it("coalesces rapid edits into a single persist call after 450 ms", async () => {
    render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    // Enable fake timers after render so React's initial setup isn't affected.
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    // Two rapid changes — each cancels the previous debounce and schedules a new one.
    fireEvent.change(nameInput, { target: { value: "First" } });
    fireEvent.change(nameInput, { target: { value: "Renamed" } });

    act(() => vi.advanceTimersByTime(449));
    expect(vi.mocked(mockUpdateMode)).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ name: "Renamed" }),
    );
  });

  it("unmount flushes pending debounced write before teardown", () => {
    const { unmount } = render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(nameInput, { target: { value: "Flushed" } });
    // 450 ms timer is pending — do NOT advance it.

    unmount();

    // Cleanup effect must have called updateMode synchronously on unmount.
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ name: "Flushed" }),
    );
  });

  it("persist failure shows toast without reverting local state", async () => {
    vi.mocked(mockUpdateMode).mockRejectedValueOnce(new Error("server error"));

    render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(nameInput, { target: { value: "Broken" } });

    await act(async () => {
      vi.advanceTimersByTime(450);
    });

    expect(vi.mocked(toast.error)).toHaveBeenCalledWith(
      "Couldn't save mode",
      expect.anything(),
    );
    // Local state must not revert.
    expect(nameInput).toHaveValue("Broken");
  });

  it("new-mode: CTA visible; changes do not autosave; creation calls addMode", async () => {
    const onClose = vi.fn();
    const onPersist = vi.fn();
    const newMode: Mode = { ...MODE, id: "mode-new", name: "" };

    render(
      <EditorWrapper
        mode={newMode}
        isNew={true}
        onClose={onClose}
        onPersist={onPersist}
      />,
    );

    expect(screen.getByRole("button", { name: "Create mode" })).toBeDisabled();

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "My Mode" },
    });

    // Advance past debounce window — must NOT autosave for new modes.
    act(() => vi.advanceTimersByTime(450));
    expect(vi.mocked(mockUpdateMode)).not.toHaveBeenCalled();

    // Restore real timers before the async click/create flow.
    vi.useRealTimers();

    expect(
      screen.getByRole("button", { name: "Create mode" }),
    ).not.toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Create mode" }));

    await waitFor(() =>
      expect(vi.mocked(mockAddMode)).toHaveBeenCalledTimes(1),
    );
    expect(onClose).toHaveBeenCalled();
  });
});

const BASE_SETTINGS: Settings = {
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  assemblyai_api_key_configured: false,
  hotkey_bindings: [],
  term_sets: [],
  correction_sets: [],
  snippets: [],
  modes: [],
  default_mode_id: "mode-1",
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 9,
  ai_cleanup_min_duration_ms: 3000,
  input_device: null,
  pause_media_on_record: true,
  history_limit: 5,
  show_in_dock: false,
  show_live_preview: true,
};

function ModesPageWrapper({ settings }: { settings: Settings }) {
  return (
    <MemoryRouter>
      <TooltipProvider>
        <SettingsContext.Provider
          value={{
            settings,
            setSettings: vi.fn(),
            setSetting: vi.fn(),
            themePreference: "system",
            setThemePreference: vi.fn(),
            accent: "indigo",
            setAccent: vi.fn(),
          }}
        >
          <ModesPage />
        </SettingsContext.Provider>
      </TooltipProvider>
    </MemoryRouter>
  );
}

describe("ModeEditor – provider/model pickers", () => {
  it("shows Model picker when provider is Groq", () => {
    const groqMode: Mode = {
      ...MODE,
      provider_model: { provider: "groq", model: "whisper_large_v3_turbo" },
    };
    render(<EditorWrapper mode={groqMode} />);
    expect(screen.getByText("Model")).toBeInTheDocument();
  });

  it("shows Model picker when provider is AssemblyAI", () => {
    const assemblyMode: Mode = {
      ...MODE,
      provider_model: {
        provider: "assembly_ai",
        model: "universal_pro_streaming",
      },
    };
    render(<EditorWrapper mode={assemblyMode} />);
    expect(screen.getByText("Model")).toBeInTheDocument();
  });

  it("does not show Model picker when provider is Deepgram", () => {
    render(<EditorWrapper mode={MODE} />);
    expect(screen.queryByText("Model")).not.toBeInTheDocument();
  });
});

describe("ModesPage – warning badge", () => {
  const modeWithGroq: Mode = {
    ...MODE,
    id: "mode-1",
    name: "Groq Mode",
    provider_model: { provider: "groq", model: "whisper_large_v3_turbo" },
  };

  it("shows warning badge when provider key is missing", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      groq_api_key_configured: false,
      modes: [modeWithGroq],
    };
    const { container } = render(<ModesPageWrapper settings={settings} />);
    expect(container.querySelector(".text-amber-500")).toBeInTheDocument();
  });

  it("does not show warning badge when provider key is configured", () => {
    const settings: Settings = {
      ...BASE_SETTINGS,
      groq_api_key_configured: true,
      modes: [modeWithGroq],
    };
    const { container } = render(<ModesPageWrapper settings={settings} />);
    expect(container.querySelector(".text-amber-500")).not.toBeInTheDocument();
  });
});

describe("ModeEditor – correction sets", () => {
  const SETS: NamedCorrectionSet[] = [
    { id: "cs-1", name: "Punctuation", entries: [] },
    { id: "cs-2", name: "Tech Terms", entries: [] },
  ];

  it("shows a toggle for each correction set", () => {
    render(
      <ModeEditor
        mode={MODE}
        isNew={false}
        onClose={vi.fn()}
        onPersist={vi.fn()}
        correctionSets={SETS}
      />,
    );
    expect(screen.getByLabelText("Punctuation")).toBeInTheDocument();
    expect(screen.getByLabelText("Tech Terms")).toBeInTheDocument();
  });

  it("toggling a set on autosaves with it in correction_set_ids", async () => {
    render(
      <ModeEditor
        mode={{ ...MODE, correction_set_ids: [] }}
        isNew={false}
        onClose={vi.fn()}
        onPersist={vi.fn()}
        correctionSets={SETS}
      />,
    );

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.click(screen.getByLabelText("Punctuation"));

    await act(async () => {
      vi.advanceTimersByTime(450);
    });

    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ correction_set_ids: ["cs-1"] }),
    );
  });

  it("toggling a set off autosaves without it in correction_set_ids", async () => {
    render(
      <ModeEditor
        mode={{ ...MODE, correction_set_ids: ["cs-1"] }}
        isNew={false}
        onClose={vi.fn()}
        onPersist={vi.fn()}
        correctionSets={SETS}
      />,
    );

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.click(screen.getByLabelText("Punctuation"));

    await act(async () => {
      vi.advanceTimersByTime(450);
    });

    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ correction_set_ids: [] }),
    );
  });
});
