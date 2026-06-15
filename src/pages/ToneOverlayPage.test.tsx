import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";

import { SettingsContext } from "../context/SettingsContext";
import type { Settings } from "../lib/types";
import { ToneOverlayPage } from "./ToneOverlayPage";

vi.mock("../lib/api", () => ({
  setToneOverlayEnabled: vi.fn(),
  getAppsSeenInHistory: vi.fn().mockResolvedValue([]),
  setToneAppOverride: vi.fn().mockResolvedValue(undefined),
  setToneAppCustomPrompt: vi.fn().mockResolvedValue(undefined),
  clearToneAppOverride: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    error: vi.fn(),
    success: vi.fn(),
  }),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div role="dialog">{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h2>{children}</h2>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({
    value,
    onValueChange,
    children,
  }: {
    value?: string;
    onValueChange?: (value: string) => void;
    children?: React.ReactNode;
  }) => (
    <select value={value} onChange={(e) => onValueChange?.(e.target.value)}>
      {children}
    </select>
  ),
  SelectTrigger: () => null,
  SelectValue: () => null,
  SelectContent: ({ children }: { children?: React.ReactNode }) => (
    <>{children}</>
  ),
  SelectItem: ({
    value,
    children,
  }: {
    value: string;
    children?: React.ReactNode;
  }) => <option value={value}>{children}</option>,
}));

const BASE_SETTINGS: Settings = {
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
  tone_app_overrides: {},
  tone_app_custom_prompts: {},
  learn_from_corrections: false,
  input_device: null,
  pause_media_on_record: true,
  history_limit: 5,
  show_in_dock: false,
  start_at_login: false,
  show_live_preview: true,
  local_whisper_idle_timeout: "fifteen_minutes",
};

function Wrapper({ settings = BASE_SETTINGS }: { settings?: Settings }) {
  const [s, setRawSettings] = useState<Settings>(settings);
  return (
    <TooltipProvider>
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
        <ToneOverlayPage />
      </SettingsContext.Provider>
    </TooltipProvider>
  );
}

describe("ToneOverlayPage", () => {
  afterEach(() => vi.clearAllMocks());

  it("hides per-app overrides section when tone overlay is disabled", () => {
    render(<Wrapper />);
    expect(screen.queryByText("Per-app overrides")).not.toBeInTheDocument();
  });

  it("lists only apps that have an explicit override", async () => {
    const { getAppsSeenInHistory } = await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: "casual",
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText("Per-app overrides")).toBeInTheDocument();
      expect(screen.getByText("Mail")).toBeInTheDocument();
    });
  });

  it("calls setToneAppOverride when the user changes an overridden app's preset", async () => {
    const { getAppsSeenInHistory, setToneAppOverride } =
      await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: "formal",
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await waitFor(() => screen.getByText("Mail"));
    await userEvent.selectOptions(screen.getByRole("combobox"), "casual");
    await waitFor(() => {
      expect(setToneAppOverride).toHaveBeenCalledWith(
        "com.apple.mail",
        "casual",
      );
    });
  });

  it("calls setToneAppOverride with the app's auto preset when added via the picker", async () => {
    const { getAppsSeenInHistory, setToneAppOverride } =
      await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: null,
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await waitFor(() => screen.getByRole("combobox"));
    await userEvent.selectOptions(
      screen.getByRole("combobox"),
      "com.apple.mail",
    );
    await waitFor(() => {
      expect(setToneAppOverride).toHaveBeenCalledWith(
        "com.apple.mail",
        "formal",
      );
    });
  });

  it("optimistically removes the row and shows undo toast when override is removed", async () => {
    const { getAppsSeenInHistory } = await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: "formal",
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await userEvent.click(
      await screen.findByRole("button", { name: /remove mail override/i }),
    );
    expect(
      screen.queryByRole("button", { name: /remove mail override/i }),
    ).not.toBeInTheDocument();
    expect(toast).toHaveBeenCalledWith(
      expect.stringContaining("Mail"),
      expect.objectContaining({
        action: expect.objectContaining({ label: "Undo" }),
      }),
    );
  });

  it("restores the row when undo is pressed", async () => {
    const { getAppsSeenInHistory, clearToneAppOverride } =
      await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: "formal",
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await userEvent.click(
      await screen.findByRole("button", { name: /remove mail override/i }),
    );
    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    const action = opts.action as { onClick: () => void };
    action.onClick();
    await waitFor(() => expect(screen.getByText("Mail")).toBeInTheDocument());
    expect(clearToneAppOverride).not.toHaveBeenCalled();
  });

  it("commits the remove to the backend when the toast closes without undo", async () => {
    const { getAppsSeenInHistory, clearToneAppOverride } =
      await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: "formal",
        custom_prompt: null,
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await userEvent.click(
      await screen.findByRole("button", { name: /remove mail override/i }),
    );
    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onAutoClose as (t: unknown) => void)({});
    await waitFor(() =>
      expect(clearToneAppOverride).toHaveBeenCalledWith("com.apple.mail"),
    );
  });

  it("lists an app with a custom prompt as an override", async () => {
    const { getAppsSeenInHistory } = await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: null,
        custom_prompt: "be terse",
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText("Mail")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /edit mail custom prompt/i }),
      ).toBeInTheDocument();
    });
  });

  it("saves a custom prompt via the editor", async () => {
    const { getAppsSeenInHistory, setToneAppCustomPrompt } =
      await import("../lib/api");
    vi.mocked(getAppsSeenInHistory).mockResolvedValueOnce([
      {
        bundle_id: "com.apple.mail",
        app_name: "Mail",
        tone_preset: "formal",
        tone_override: null,
        custom_prompt: "old",
        icon_data_url: null,
      },
    ]);
    render(
      <Wrapper
        settings={{ ...BASE_SETTINGS, ai_cleanup_tone_overlay_enabled: true }}
      />,
    );
    await userEvent.click(
      await screen.findByRole("button", { name: /edit mail custom prompt/i }),
    );
    const textarea = await screen.findByRole("textbox");
    await userEvent.clear(textarea);
    await userEvent.type(textarea, "write like a pirate");
    await userEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() => {
      expect(setToneAppCustomPrompt).toHaveBeenCalledWith(
        "com.apple.mail",
        "write like a pirate",
      );
    });
  });
});
