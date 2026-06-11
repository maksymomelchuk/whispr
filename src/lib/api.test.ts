import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  addCorrectionSet,
  addMode,
  clearHistory,
  clearStats,
  createTermSet,
  deleteCorrectionSet,
  deleteMode,
  deleteTermSet,
  duplicateMode,
  getCleanupStats,
  getHistory,
  getSettings,
  getStats,
  listInputDevices,
  renameTermSet,
  setAnthropicApiKey,
  setAnthropicOauthToken,
  setAssemblyAiApiKey,
  setCleanupAuthMode,
  setCleanupThresholds,
  setDeepgramApiKey,
  setGroqApiKey,
  setHistoryLimit,
  setHotkeyBindings,
  setInputDevice,
  setPauseMediaOnRecord,
  setShortcutCapturePaused,
  setShowInDock,
  setShowLivePreview,
  setSnippets,
  updateCorrectionSet,
  updateMode,
  updateTermSetEntries,
  validateAssemblyAiApiKey,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "./api";

// A rename in Rust or a payload shape change will cause the matching test to fail.
const COMMANDS: Array<{
  call: () => void;
  cmd: string;
  args?: Record<string, unknown>;
}> = [
  { call: () => getSettings(), cmd: "get_settings" },
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
    call: () => setAssemblyAiApiKey("ak"),
    cmd: "set_assemblyai_api_key",
    args: { apiKey: "ak" },
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
    call: () => validateAssemblyAiApiKey("ak"),
    cmd: "validate_assemblyai_api_key",
    args: { apiKey: "ak" },
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
  {
    call: () => createTermSet("My Set"),
    cmd: "create_term_set",
    args: { name: "My Set" },
  },
  {
    call: () => renameTermSet("ts-1", "Renamed"),
    cmd: "rename_term_set",
    args: { id: "ts-1", name: "Renamed" },
  },
  {
    call: () => updateTermSetEntries("ts-1", ["MongoDB"]),
    cmd: "update_term_set_entries",
    args: { id: "ts-1", entries: ["MongoDB"] },
  },
  {
    call: () => deleteTermSet("ts-1"),
    cmd: "delete_term_set",
    args: { id: "ts-1" },
  },
  {
    call: () =>
      addCorrectionSet({
        id: "cs1",
        name: "My Set",
        entries: [{ from: "x", to: "y" }],
      }),
    cmd: "add_correction_set",
    args: {
      set: { id: "cs1", name: "My Set", entries: [{ from: "x", to: "y" }] },
    },
  },
  {
    call: () =>
      updateCorrectionSet({ id: "cs1", name: "Updated", entries: [] }),
    cmd: "update_correction_set",
    args: { set: { id: "cs1", name: "Updated", entries: [] } },
  },
  {
    call: () => deleteCorrectionSet("cs1"),
    cmd: "delete_correction_set",
    args: { setId: "cs1" },
  },
  {
    call: () => setSnippets([]),
    cmd: "set_snippets",
    args: { snippets: [] },
  },
  {
    call: () =>
      addMode({
        id: "m1",
        name: "Test",
        icon: null,
        language: { kind: "auto" },

        ai_cleanup: {
          enabled: false,
          prompt_override: null,
          provider: "anthropic",
          model: "claude-haiku-4-5",
          paste_raw_on_failure: true,
          clipboard_context_enabled: false,
          selected_text_context_enabled: false,
          focused_field_context_enabled: false,
        },
        term_set_ids: [],
        correction_set_ids: [],
        use_snippets: true,
        provider_model: { provider: "deepgram" },
      }),
    cmd: "add_mode",
    args: {
      mode: {
        id: "m1",
        name: "Test",
        icon: null,
        language: { kind: "auto" },

        ai_cleanup: {
          enabled: false,
          prompt_override: null,
          provider: "anthropic",
          model: "claude-haiku-4-5",
          paste_raw_on_failure: true,
          clipboard_context_enabled: false,
          selected_text_context_enabled: false,
          focused_field_context_enabled: false,
        },
        term_set_ids: [],
        correction_set_ids: [],
        use_snippets: true,
        provider_model: { provider: "deepgram" },
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

        ai_cleanup: {
          enabled: false,
          prompt_override: null,
          provider: "anthropic",
          model: "claude-haiku-4-5",
          paste_raw_on_failure: true,
          clipboard_context_enabled: false,
          selected_text_context_enabled: false,
          focused_field_context_enabled: false,
        },
        term_set_ids: [],
        correction_set_ids: [],
        use_snippets: true,
        provider_model: { provider: "groq", model: "whisper_large_v3_turbo" },
      }),
    cmd: "update_mode",
    args: {
      mode: {
        id: "m1",
        name: "Updated",
        icon: null,
        language: { kind: "auto" },

        ai_cleanup: {
          enabled: false,
          prompt_override: null,
          provider: "anthropic",
          model: "claude-haiku-4-5",
          paste_raw_on_failure: true,
          clipboard_context_enabled: false,
          selected_text_context_enabled: false,
          focused_field_context_enabled: false,
        },
        term_set_ids: [],
        correction_set_ids: [],
        use_snippets: true,
        provider_model: { provider: "groq", model: "whisper_large_v3_turbo" },
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
