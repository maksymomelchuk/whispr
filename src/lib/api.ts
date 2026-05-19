import { invoke } from "@tauri-apps/api/core";

import type {
  ApiKeyValidation,
  CleanupAuthMode,
  CleanupStats,
  CorrectionEntry,
  HistoryEntry,
  HistoryLimit,
  HotkeyBinding,
  Mode,
  NamedTermSet,
  Settings,
  Snippet,
  StatsRow,
} from "./types";

export { formatShortcut } from "./shortcut";

export const getSettings = () => invoke<Settings>("get_settings");

export const setDeepgramApiKey = (apiKey: string) =>
  invoke<void>("set_deepgram_api_key", { apiKey });

export const setGroqApiKey = (apiKey: string) =>
  invoke<void>("set_groq_api_key", { apiKey });

export const setAssemblyAiApiKey = (apiKey: string) =>
  invoke<void>("set_assemblyai_api_key", { apiKey });

export const validateAssemblyAiApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_assemblyai_api_key", { apiKey });

export const validateDeepgramApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_deepgram_api_key", { apiKey });

export const validateGroqApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_groq_api_key", { apiKey });

export const setHotkeyBindings = (bindings: HotkeyBinding[]) =>
  invoke<void>("set_hotkey_bindings", { bindings });

export const setShortcutCapturePaused = (paused: boolean) =>
  invoke<void>("set_shortcut_capture_paused", { paused });

export const openTranslationSettings = () =>
  invoke<void>("open_translation_settings");

export const createTermSet = (name: string) =>
  invoke<NamedTermSet>("create_term_set", { name });

export const renameTermSet = (id: string, name: string) =>
  invoke<void>("rename_term_set", { id, name });

export const updateTermSetEntries = (id: string, entries: string[]) =>
  invoke<void>("update_term_set_entries", { id, entries });

export const deleteTermSet = (id: string) =>
  invoke<void>("delete_term_set", { id });

export const setCorrections = (corrections: CorrectionEntry[]) =>
  invoke<void>("set_corrections", { corrections });

export const setSnippets = (snippets: Snippet[]) =>
  invoke<void>("set_snippets", { snippets });

export const setCleanupEnabled = (enabled: boolean) =>
  invoke<void>("set_cleanup_enabled", { enabled });

export const addMode = (mode: Mode) => invoke<void>("add_mode", { mode });

export const updateMode = (mode: Mode) => invoke<void>("update_mode", { mode });

export const deleteMode = (id: string) => invoke<void>("delete_mode", { id });

export const duplicateMode = (id: string) =>
  invoke<void>("duplicate_mode", { id });

export const setDefaultMode = (id: string) =>
  invoke<void>("set_default_mode", { id });

export const setAnthropicApiKey = (apiKey: string) =>
  invoke<void>("set_anthropic_api_key", { apiKey });

export const setAnthropicOauthToken = (token: string) =>
  invoke<void>("set_anthropic_oauth_token", { token });

export const setCleanupAuthMode = (mode: CleanupAuthMode) =>
  invoke<void>("set_cleanup_auth_mode", { mode });

export const setCleanupThresholds = (minWords: number, minDurationMs: number) =>
  invoke<void>("set_cleanup_thresholds", {
    minWords,
    minDurationMs,
  });

export const listInputDevices = () => invoke<string[]>("list_input_devices");

export const setInputDevice = (device: string | null) =>
  invoke<void>("set_input_device", { device });

export const setPauseMediaOnRecord = (enabled: boolean) =>
  invoke<void>("set_pause_media_on_record", { enabled });

export const setShowInDock = (enabled: boolean) =>
  invoke<void>("set_show_in_dock", { enabled });

export const setShowLivePreview = (enabled: boolean) =>
  invoke<void>("set_show_live_preview", { enabled });

export const getHistory = () => invoke<HistoryEntry[]>("get_history");

export const clearHistory = () => invoke<void>("clear_history");

export const setHistoryLimit = (limit: HistoryLimit) =>
  invoke<void>("set_history_limit", { limit });

export const getStats = () => invoke<StatsRow[]>("get_stats");

export const clearStats = () => invoke<void>("clear_stats");

export const getCleanupStats = () => invoke<CleanupStats>("get_cleanup_stats");

export interface PermissionsStatus {
  microphone: boolean;
  accessibility: boolean;
}

export const checkPermissions = () =>
  invoke<PermissionsStatus>("check_permissions");
export const openMicrophoneSettings = () =>
  invoke<void>("open_microphone_settings");
export const openAccessibilitySettings = () =>
  invoke<void>("open_accessibility_settings");
