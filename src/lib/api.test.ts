import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  addMode,
  clearHistory,
  clearStats,
  deleteMode,
  duplicateMode,
  getCleanupStats,
  getHistory,
  getSettings,
  getStats,
  listInputDevices,
  openTranslationSettings,
  setAnthropicApiKey,
  setAnthropicOauthToken,
  setCleanupAuthMode,
  setCleanupThresholds,
  setCorrections,
  setDefaultMode,
  setCleanupEnabled,
  setDeepgramApiKey,
  setGroqApiKey,
  setGroqSettings,
  setHistoryLimit,
  setHotkeyBindings,
  setInputDevice,
  setPauseMediaOnRecord,
  setShortcutCapturePaused,
  setShowInDock,
  setShowLivePreview,
  setSnippets,
  setTerms,
  setTranscriptionProvider,
  updateMode,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "./api";

// Fixture: maps each Tauri command to its expected argument keys.
// A rename in Rust or a payload shape change will cause the matching test to fail.
const COMMANDS: Array<{ call: () => void; cmd: string; args?: Record<string, unknown> }> = [
  { call: () => getSettings(), cmd: "get_settings" },
  {
    call: () => setTranscriptionProvider("deepgram"),
    cmd: "set_transcription_provider",
    args: { provider: "deepgram" },
  },
  {
    call: () => setDeepgramApiKey("dk"),
    cmd: "set_deepgram_api_key",
    args: { apiKey: "dk" },
  },
  {
    call: () => setGroqApiKey("gk"),
    cmd: "set_groq_api_key",
    args: { apiKey: "gk" },
  },
  {
    call: () => setGroqSettings({ model: "whisper_large_v3" }),
    cmd: "set_groq_settings",
    args: { groq: { model: "whisper_large_v3" } },
  },
  {
    call: () => validateDeepgramApiKey("dk"),
    cmd: "validate_deepgram_api_key",
    args: { apiKey: "dk" },
  },
  {
    call: () => validateGroqApiKey("gk"),
    cmd: "validate_groq_api_key",
    args: { apiKey: "gk" },
  },
  {
    call: () => setHotkeyBindings([]),
    cmd: "set_hotkey_bindings",
    args: { bindings: [] },
  },
  {
    call: () => setShortcutCapturePaused(true),
    cmd: "set_shortcut_capture_paused",
    args: { paused: true },
  },
  { call: () => openTranslationSettings(), cmd: "open_translation_settings" },
  {
    call: () => setTerms(["a"]),
    cmd: "set_terms",
    args: { terms: ["a"] },
  },
  {
    call: () => setCorrections([{ from: "x", to: "y" }]),
    cmd: "set_corrections",
    args: { corrections: [{ from: "x", to: "y" }] },
  },
  {
    call: () => setSnippets([]),
    cmd: "set_snippets",
    args: { snippets: [] },
  },
  {
    call: () => setCleanupEnabled(true),
    cmd: "set_cleanup_enabled",
    args: { enabled: true },
  },
  {
    call: () =>
      addMode({
        id: "m1",
        name: "Test",
        icon: null,
        language: { kind: "auto" },
        translate: { kind: "off" },
        ai_cleanup: { enabled: false, prompt_override: null },
        use_terms: true,
        use_corrections: true,
        use_snippets: true,
      }),
    cmd: "add_mode",
    args: {
      mode: {
        id: "m1",
        name: "Test",
        icon: null,
        language: { kind: "auto" },
        translate: { kind: "off" },
        ai_cleanup: { enabled: false, prompt_override: null },
        use_terms: true,
        use_corrections: true,
        use_snippets: true,
      },
    },
  },
  {
    call: () =>
      updateMode({
        id: "m1",
        name: "Updated",
        icon: null,
        language: { kind: "auto" },
        translate: { kind: "off" },
        ai_cleanup: { enabled: false, prompt_override: null },
        use_terms: true,
        use_corrections: true,
        use_snippets: true,
      }),
    cmd: "update_mode",
    args: {
      mode: {
        id: "m1",
        name: "Updated",
        icon: null,
        language: { kind: "auto" },
        translate: { kind: "off" },
        ai_cleanup: { enabled: false, prompt_override: null },
        use_terms: true,
        use_corrections: true,
        use_snippets: true,
      },
    },
  },
  {
    call: () => deleteMode("m1"),
    cmd: "delete_mode",
    args: { id: "m1" },
  },
  {
    call: () => duplicateMode("m1"),
    cmd: "duplicate_mode",
    args: { id: "m1" },
  },
  {
    call: () => setDefaultMode("m1"),
    cmd: "set_default_mode",
    args: { id: "m1" },
  },
  {
    call: () => setAnthropicApiKey("ak"),
    cmd: "set_anthropic_api_key",
    args: { apiKey: "ak" },
  },
  {
    call: () => setAnthropicOauthToken("tok"),
    cmd: "set_anthropic_oauth_token",
    args: { token: "tok" },
  },
  {
    call: () => setCleanupAuthMode("api_key"),
    cmd: "set_cleanup_auth_mode",
    args: { mode: "api_key" },
  },
  {
    call: () => setCleanupThresholds(10, 500),
    cmd: "set_cleanup_thresholds",
    args: { minWords: 10, minDurationMs: 500 },
  },
  { call: () => listInputDevices(), cmd: "list_input_devices" },
  {
    call: () => setInputDevice("mic"),
    cmd: "set_input_device",
    args: { device: "mic" },
  },
  {
    call: () => setPauseMediaOnRecord(true),
    cmd: "set_pause_media_on_record",
    args: { enabled: true },
  },
  {
    call: () => setShowInDock(false),
    cmd: "set_show_in_dock",
    args: { enabled: false },
  },
  {
    call: () => setShowLivePreview(true),
    cmd: "set_show_live_preview",
    args: { enabled: true },
  },
  { call: () => getHistory(), cmd: "get_history" },
  { call: () => clearHistory(), cmd: "clear_history" },
  {
    call: () => setHistoryLimit(100),
    cmd: "set_history_limit",
    args: { limit: 100 },
  },
  { call: () => getStats(), cmd: "get_stats" },
  { call: () => clearStats(), cmd: "clear_stats" },
  { call: () => getCleanupStats(), cmd: "get_cleanup_stats" },
];

describe("lib/api — Tauri command contracts", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  for (const { call, cmd, args } of COMMANDS) {
    it(cmd, () => {
      call();
      if (args !== undefined) {
        expect(invoke).toHaveBeenCalledWith(cmd, args);
      } else {
        expect(invoke).toHaveBeenCalledWith(cmd);
      }
    });
  }
});
