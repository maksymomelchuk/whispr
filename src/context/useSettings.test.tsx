import { act, renderHook } from "@testing-library/react";
import { useCallback, useState } from "react";
import { describe, expect, it, vi } from "vitest";

import type { Settings } from "../lib/types";
import { SettingsContext, useSettings } from "./SettingsContext";

const MOCK_SETTINGS: Settings = {
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
  ai_cleanup_min_words: 5,
  ai_cleanup_min_duration_ms: 1000,
  ai_cleanup_tone_overlay_enabled: false,
  tone_app_overrides: {},
  tone_app_custom_prompts: {},
  learn_from_corrections: false,
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

function TestWrapper({ children }: { children: React.ReactNode }) {
  const [settings, setRawSettings] = useState<Settings>(MOCK_SETTINGS);

  const setSettings = useCallback((updater: (prev: Settings) => Settings) => {
    setRawSettings(updater);
  }, []);

  const setSetting = useCallback(
    async <K extends keyof Settings>(
      key: K,
      value: Settings[K],
      persist: () => Promise<void>,
      onError?: (err: unknown) => void,
    ) => {
      const snapshot = settings;
      setSettings(() => ({ ...snapshot, [key]: value }));
      try {
        await persist();
      } catch (e) {
        setSettings(() => snapshot);
        onError?.(e);
      }
    },
    [settings, setSettings],
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
  it("exposes initial settings", () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });

    expect(result.current.settings).toEqual(MOCK_SETTINGS);
  });

  it("setSetting applies optimistic update before persist resolves", async () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });

    const persist = vi.fn().mockReturnValue(new Promise<void>(() => {}));

    act(() => {
      void result.current.setSetting("show_in_dock", false, persist);
    });

    expect(result.current.settings.show_in_dock).toBe(false);
  });

  it("setSetting rolls back to prior value when persist rejects", async () => {
    const { result } = renderHook(() => useSettings(), {
      wrapper: TestWrapper,
    });

    const initial = result.current.settings.show_in_dock;

    await act(async () => {
      await result.current
        .setSetting("show_in_dock", !initial, () =>
          Promise.reject(new Error("persist failed")),
        )
        .catch(() => {});
    });

    expect(result.current.settings.show_in_dock).toBe(initial);
  });
});
