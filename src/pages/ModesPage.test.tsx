import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { MemoryRouter } from "react-router-dom";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { Mode, NamedCorrectionSet, Settings } from "@/lib/types";
import { pttBinding } from "@/lib/types";

import { SettingsContext } from "../context/SettingsContext";
import {
  addMode as mockAddMode,
  deleteMode as mockDeleteMode,
  getLocalModelStatuses as mockGetLocalModelStatuses,
  getSettings as mockGetSettings,
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
  formatShortcut: vi.fn(),
  getLocalModelStatuses: vi.fn().mockResolvedValue([]),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

const MODE: Mode = {
  id: "mode-1",
  name: "Original",
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

function EditorWrapper({
  mode = MODE,
  isNew = false,
  onClose = vi.fn(),
  onPersist = vi.fn(),
  configuredProviders,
  customProviderModel,
}: {
  mode?: Mode;
  isNew?: boolean;
  onClose?: () => void;
  onPersist?: (m: Mode, wasNew: boolean) => void;
  configuredProviders?: import("@/lib/types").AiProviderId[];
  customProviderModel?: string;
}) {
  return (
    <MemoryRouter>
      <TooltipProvider>
        <ModeEditor
          mode={mode}
          isNew={isNew}
          onClose={onClose}
          onPersist={onPersist}
          configuredProviders={configuredProviders}
          customProviderModel={customProviderModel}
        />
      </TooltipProvider>
    </MemoryRouter>
  );
}

beforeEach(() => {
  vi.mocked(mockUpdateMode).mockResolvedValue(undefined);
  vi.mocked(mockAddMode).mockResolvedValue(undefined);
  vi.mocked(mockGetLocalModelStatuses).mockResolvedValue([]);
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
      "Couldn't save profile",
      expect.anything(),
    );
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

    expect(
      screen.getByRole("button", { name: "Create profile" }),
    ).toBeDisabled();

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
      screen.getByRole("button", { name: "Create profile" }),
    ).not.toBeDisabled();
    await userEvent.click(
      screen.getByRole("button", { name: "Create profile" }),
    );

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
  ai_cleanup_min_words: 9,
  ai_cleanup_min_duration_ms: 3000,
  ai_cleanup_tone_overlay_enabled: false,
  learn_from_corrections: false,
  input_device: null,
  pause_media_on_record: true,
  history_limit: 5,
  show_in_dock: false,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
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

function StatefulModesPageWrapper({ initial }: { initial: Settings }) {
  const [settings, setSettings] = useState(initial);
  return (
    <MemoryRouter>
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
          <ModesPage />
        </SettingsContext.Provider>
      </TooltipProvider>
    </MemoryRouter>
  );
}

describe("ModesPage – delete", () => {
  const firstMode: Mode = { ...MODE, id: "mode-1", name: "Original" };
  const secondMode: Mode = { ...MODE, id: "mode-2", name: "Second" };

  it("re-fetches settings after delete so the removed mode's binding does not linger", async () => {
    const initial: Settings = {
      ...BASE_SETTINGS,
      modes: [firstMode, secondMode],
      hotkey_bindings: [
        pttBinding(
          { key: "KeyK", modifiers: ["AltLeft", "MetaLeft"] },
          "mode-1",
        ),
      ],
    };
    // Backend drops the deleted mode and its binding; the page must adopt this
    // server truth rather than optimistically filtering only `modes`.
    const afterDelete: Settings = {
      ...initial,
      modes: [secondMode],
      hotkey_bindings: [],
    };
    vi.mocked(mockDeleteMode).mockResolvedValue(undefined);
    vi.mocked(mockGetSettings).mockResolvedValue(afterDelete);

    render(<StatefulModesPageWrapper initial={initial} />);

    fireEvent.click(screen.getAllByLabelText("Delete")[0]);

    await waitFor(() =>
      expect(vi.mocked(mockDeleteMode)).toHaveBeenCalledWith("mode-1"),
    );
    expect(vi.mocked(mockGetSettings)).toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.queryByText("Original")).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("allows deleting the only remaining profile (down to zero)", async () => {
    const initial: Settings = { ...BASE_SETTINGS, modes: [firstMode] };
    vi.mocked(mockDeleteMode).mockResolvedValue(undefined);
    vi.mocked(mockGetSettings).mockResolvedValue({
      ...BASE_SETTINGS,
      modes: [],
    });

    render(<StatefulModesPageWrapper initial={initial} />);

    const deleteButton = screen.getByLabelText("Delete");
    expect(deleteButton).not.toBeDisabled();
    fireEvent.click(deleteButton);

    await waitFor(() =>
      expect(vi.mocked(mockDeleteMode)).toHaveBeenCalledWith("mode-1"),
    );
    await waitFor(() =>
      expect(
        screen.getByText("No profiles yet. Add one to start dictating."),
      ).toBeInTheDocument(),
    );
  });

  it("shows an empty state when there are no profiles", () => {
    render(
      <StatefulModesPageWrapper initial={{ ...BASE_SETTINGS, modes: [] }} />,
    );
    expect(
      screen.getByText("No profiles yet. Add one to start dictating."),
    ).toBeInTheDocument();
  });
});

describe("ModeEditor – correction sets", () => {
  const SETS: NamedCorrectionSet[] = [
    { id: "cs-1", name: "Punctuation", entries: [] },
    { id: "cs-2", name: "Tech Terms", entries: [] },
  ];

  it("renders a chip for each selected correction set", () => {
    render(
      <TooltipProvider>
        <ModeEditor
          mode={{ ...MODE, correction_set_ids: ["cs-1"] }}
          isNew={false}
          onClose={vi.fn()}
          onPersist={vi.fn()}
          correctionSets={SETS}
        />
      </TooltipProvider>,
    );
    expect(
      screen.getByRole("button", { name: "Remove Punctuation" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Remove Tech Terms" }),
    ).not.toBeInTheDocument();
  });

  it("shows the add picker when not every set is selected", () => {
    render(
      <TooltipProvider>
        <ModeEditor
          mode={{ ...MODE, correction_set_ids: ["cs-1"] }}
          isNew={false}
          onClose={vi.fn()}
          onPersist={vi.fn()}
          correctionSets={SETS}
        />
      </TooltipProvider>,
    );
    expect(screen.getByText("+ Add correction set")).toBeInTheDocument();
  });

  it("removing a chip autosaves without that set in correction_set_ids", async () => {
    render(
      <TooltipProvider>
        <ModeEditor
          mode={{ ...MODE, correction_set_ids: ["cs-1"] }}
          isNew={false}
          onClose={vi.fn()}
          onPersist={vi.fn()}
          correctionSets={SETS}
        />
      </TooltipProvider>,
    );

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.click(screen.getByRole("button", { name: "Remove Punctuation" }));

    await act(async () => {
      vi.advanceTimersByTime(450);
    });

    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ correction_set_ids: [] }),
    );
  });
});

describe("ModeEditor – local model picker", () => {
  const LOCAL_MODE: Mode = {
    ...MODE,
    provider_model: { provider: "local", model: "large_v3_turbo" },
  };

  it("shows Model picker when provider is Local", () => {
    render(<EditorWrapper mode={LOCAL_MODE} />);
    expect(screen.getByText("Model")).toBeInTheDocument();
  });

  it("shows download hint when some local models are not downloaded", async () => {
    vi.mocked(mockGetLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: false,
        downloading: false,
        size_bytes: 0,
      },
      {
        model: "large_v3",
        downloaded: false,
        downloading: false,
        size_bytes: 0,
      },
    ]);
    render(<EditorWrapper mode={LOCAL_MODE} />);
    await waitFor(() =>
      expect(screen.getByText(/Speech models/)).toBeInTheDocument(),
    );
  });

  it("does not show download hint when all local models are downloaded", async () => {
    vi.mocked(mockGetLocalModelStatuses).mockResolvedValue([
      {
        model: "large_v3_turbo",
        downloaded: true,
        downloading: false,
        size_bytes: 0,
      },
      {
        model: "large_v3",
        downloaded: true,
        downloading: false,
        size_bytes: 0,
      },
    ]);
    render(<EditorWrapper mode={LOCAL_MODE} />);
    await waitFor(() =>
      expect(screen.queryByText(/Speech models/)).not.toBeInTheDocument(),
    );
  });

  it("does not show Model picker when provider is Deepgram", () => {
    render(<EditorWrapper mode={MODE} />);
    expect(screen.queryByText("Model")).not.toBeInTheDocument();
  });
});

describe("ModeEditor – cleanup provider/model pickers", () => {
  const cleanupEnabledMode: Mode = {
    ...MODE,
    ai_cleanup: {
      enabled: true,
      prompt_override: null,
      provider: "anthropic",
      model: "claude-haiku-4-5",
      paste_raw_on_failure: true,
    },
  };

  it("shows cleanup model value when AI cleanup is enabled with Anthropic", () => {
    render(
      <EditorWrapper
        mode={cleanupEnabledMode}
        configuredProviders={["anthropic"]}
      />,
    );
    expect(screen.getByText("Claude Haiku 4.5")).toBeInTheDocument();
  });

  it("shows selected model value when AI cleanup is enabled with OpenAI", () => {
    const openaiMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "openai",
        model: "gpt-5.4-mini",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper mode={openaiMode} configuredProviders={["openai"]} />,
    );
    expect(screen.getByText("GPT-5.4 mini")).toBeInTheDocument();
  });

  it("disables the cleanup toggle when no cleanup provider is configured", () => {
    render(<EditorWrapper mode={MODE} configuredProviders={[]} />);
    const toggle = screen.getByRole("switch", { name: /ai cleanup/i });
    expect(toggle).toBeDisabled();
  });

  it("enables the cleanup toggle when a cleanup provider is configured", () => {
    render(<EditorWrapper mode={MODE} configuredProviders={["anthropic"]} />);
    const toggle = screen.getByRole("switch", { name: /ai cleanup/i });
    expect(toggle).not.toBeDisabled();
  });

  it("shows selected model value when AI cleanup is enabled with Google", () => {
    const googleMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "google",
        model: "gemini-2.5-flash",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper mode={googleMode} configuredProviders={["google"]} />,
    );
    expect(screen.getByText("Gemini 2.5 Flash")).toBeInTheDocument();
  });

  it("shows selected model value when AI cleanup is enabled with Groq", () => {
    const groqMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "groq",
        model: "llama-3.1-8b-instant",
        paste_raw_on_failure: true,
      },
    };
    render(<EditorWrapper mode={groqMode} configuredProviders={["groq"]} />);
    expect(screen.getByText("Llama 3.1 8B")).toBeInTheDocument();
  });

  it("shows selected model value when AI cleanup is enabled with DeepSeek", () => {
    const deepseekMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "deepseek",
        model: "deepseek-v4-flash",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper mode={deepseekMode} configuredProviders={["deepseek"]} />,
    );
    expect(screen.getByText("DeepSeek V4 Flash")).toBeInTheDocument();
  });

  it("shows selected model value when AI cleanup is enabled with Cerebras", () => {
    const cerebrasMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "cerebras",
        model: "llama-3.3-70b",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper mode={cerebrasMode} configuredProviders={["cerebras"]} />,
    );
    expect(screen.getByText("Llama 3.3 70B")).toBeInTheDocument();
  });

  it("shows selected model value when AI cleanup is enabled with OpenRouter", () => {
    const openrouterMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "openrouter",
        model: "anthropic/claude-haiku-4.5",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper
        mode={openrouterMode}
        configuredProviders={["openrouter"]}
      />,
    );
    expect(screen.getByText("Claude Haiku 4.5")).toBeInTheDocument();
  });

  it("shows global model text for Custom provider instead of a model dropdown", () => {
    const customMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "custom",
        model: "",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper
        mode={customMode}
        configuredProviders={["custom"]}
        customProviderModel="llama3.2"
      />,
    );
    expect(screen.getByText("llama3.2")).toBeInTheDocument();
  });

  it("shows blank model hint when Custom provider has no model configured", () => {
    const customMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "custom",
        model: "",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper
        mode={customMode}
        configuredProviders={["custom"]}
        customProviderModel=""
      />,
    );
    expect(
      screen.getByText("(blank — single-model server)"),
    ).toBeInTheDocument();
  });

  it("shows Custom as selected cleanup provider label when provider is custom", () => {
    const customMode: Mode = {
      ...MODE,
      ai_cleanup: {
        enabled: true,
        prompt_override: null,
        provider: "custom",
        model: "",
        paste_raw_on_failure: true,
      },
    };
    render(
      <EditorWrapper
        mode={customMode}
        configuredProviders={["custom"]}
        customProviderModel="llama3.2"
      />,
    );
    expect(screen.getByText("Custom")).toBeInTheDocument();
  });
});
