import { invoke } from "@tauri-apps/api/core";
import { act, renderHook } from "@testing-library/react";
import { useCallback, useEffect, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Settings } from "../lib/types";
import { SettingsContext, useSettings } from "./SettingsContext";

const MOCK_SETTINGS: Settings = {
  transcription_provider: "deepgram",
  deepgram_api_key_configured: false,
  groq_api_key_configured: false,
  hotkey_bindings: [],
  terms: [],
  corrections: [],
  snippets: [],
  groq: { model: "whisper_large_v3" },
  modes: [],
  default_mode_id: "default",
  ai_cleanup_enabled: true,
  ai_cleanup_auth_mode: "api_key",
  ai_cleanup_key_configured: false,
  ai_cleanup_oauth_token_configured: false,
  ai_cleanup_min_words: 5,
  ai_cleanup_min_duration_ms: 1000,
  input_device: null,
  pause_media_on_record: false,
  history_limit: null,
  show_in_dock: true,
  show_live_preview: true,
};

function TestWrapper({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    // mirrors App.tsx hydration pattern
    invoke<Settings>("get_settings").then(setSettings);
  }, []);

  const setSetting = useCallback(
    async <K extends keyof Settings>(
      key: K,
      value: Settings[K],
      persist: () => Promise<void>,
    ) => {
      const prev = settings;
      if (!prev) return;
      setSettings({ ...prev, [key]: value });
      try {
        await persist();
      } catch {
        setSettings(prev);
      }
    },
    [settings],
  );

  return (
    <SettingsContext.Provider
      value={{
        settings,
        setSettings,
        setSetting,
        themePreference: "system",
        setThemePreference: vi.fn(),
        accent: "indigo",
        setAccent: vi.fn(),
      }}
    >
      {children}
    </SettingsContext.Provider>
  );
}

describe("useSettings", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue(MOCK_SETTINGS);
  });

  it("hydration from getSettings populates settings state", async () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });

    expect(result.current.settings).toBeNull();

    await act(async () => {});

    expect(result.current.settings).toEqual(MOCK_SETTINGS);
  });

  it("setSetting applies optimistic update before persist resolves", async () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });
    await act(async () => {});

    const persist = vi.fn().mockReturnValue(new Promise<void>(() => {}));

    act(() => {
      void result.current.setSetting("show_in_dock", false, persist);
    });

    expect(result.current.settings?.show_in_dock).toBe(false);
  });

  it("setSetting rolls back to prior value when persist rejects", async () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });
    await act(async () => {});

    const initial = result.current.settings!.show_in_dock;

    await act(async () => {
      await result.current
        .setSetting("show_in_dock", !initial, () =>
          Promise.reject(new Error("persist failed")),
        )
        .catch(() => {});
    });

    expect(result.current.settings?.show_in_dock).toBe(initial);
  });
});
