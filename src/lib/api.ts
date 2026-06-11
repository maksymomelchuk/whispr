import { invoke } from "@tauri-apps/api/core";

import type {
  AiProviderId,
  ApiKeyValidation,
  CleanupAuthMode,
  CleanupStats,
  HistoryEntry,
  HistoryLimit,
  HotkeyBinding,
  LocalModelStatus,
  LocalWhisperIdleTimeout,
  LocalWhisperModel,
  Mode,
  NamedCorrectionSet,
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

export const setOpenaiApiKey = (apiKey: string) =>
  invoke<void>("set_openai_api_key", { apiKey });

export const setElevenLabsApiKey = (apiKey: string) =>
  invoke<void>("set_elevenlabs_api_key", { apiKey });

export const validateAssemblyAiApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_assemblyai_api_key", { apiKey });

export const validateOpenaiApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_openai_api_key", { apiKey });

export const validateElevenLabsApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_elevenlabs_api_key", { apiKey });

export const validateDeepgramApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_deepgram_api_key", { apiKey });

export const validateGroqApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_groq_api_key", { apiKey });

export const setHotkeyBindings = (bindings: HotkeyBinding[]) =>
  invoke<void>("set_hotkey_bindings", { bindings });

export const setShortcutCapturePaused = (paused: boolean) =>
  invoke<void>("set_shortcut_capture_paused", { paused });

export const createTermSet = (name: string) =>
  invoke<Settings>("create_term_set", { name });

export const renameTermSet = (id: string, name: string) =>
  invoke<Settings>("rename_term_set", { id, name });

export const updateTermSetEntries = (id: string, entries: string[]) =>
  invoke<Settings>("update_term_set_entries", { id, entries });

export const deleteTermSet = (id: string) =>
  invoke<Settings>("delete_term_set", { id });

export const createCorrectionSet = (name: string) =>
  invoke<Settings>("create_correction_set", { name });

export const renameCorrectionSet = (id: string, name: string) =>
  invoke<Settings>("rename_correction_set", { id, name });

export const updateCorrectionSetEntries = (
  id: string,
  entries: NamedCorrectionSet["entries"],
) => invoke<Settings>("update_correction_set_entries", { id, entries });

export const deleteCorrectionSet = (id: string) =>
  invoke<Settings>("delete_correction_set", { id });

export const setSnippets = (snippets: Snippet[]) =>
  invoke<void>("set_snippets", { snippets });

export const addMode = (mode: Mode) => invoke<void>("add_mode", { mode });

export const updateMode = (mode: Mode) => invoke<void>("update_mode", { mode });

export const deleteMode = (id: string) => invoke<void>("delete_mode", { id });

export const duplicateMode = (id: string) =>
  invoke<void>("duplicate_mode", { id });

export const reorderModes = (ids: string[]) =>
  invoke<void>("reorder_modes", { ids });

export const setAnthropicApiKey = (apiKey: string) =>
  invoke<void>("set_anthropic_api_key", { apiKey });

export const setAnthropicOauthToken = (token: string) =>
  invoke<void>("set_anthropic_oauth_token", { token });

export const setProviderKey = (providerId: AiProviderId, apiKey: string) =>
  invoke<void>("set_provider_key", { providerId, apiKey });

export const clearProviderKey = (providerId: AiProviderId) =>
  invoke<void>("clear_provider_key", { providerId });

export const setCustomProvider = (
  baseUrl: string,
  model: string,
  apiKey: string,
) => invoke<void>("set_custom_provider", { baseUrl, model, apiKey });

export const clearCustomProvider = () => invoke<void>("clear_custom_provider");

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

export const setStartAtLogin = (enabled: boolean) =>
  invoke<void>("set_start_at_login", { enabled });

export const setShowLivePreview = (enabled: boolean) =>
  invoke<void>("set_show_live_preview", { enabled });

export const getHistory = () => invoke<HistoryEntry[]>("get_history");

export const clearHistory = () => invoke<void>("clear_history");

export const setHistoryLimit = (limit: HistoryLimit) =>
  invoke<void>("set_history_limit", { limit });

export const updateHistoryEntry = (
  id: string,
  replacedText: string,
  finalText: string,
) =>
  invoke<void>("update_history_entry", {
    id,
    replaced_text: replacedText,
    final_text: finalText,
  });

export const recoverCleanup = (id: string) =>
  invoke<string>("recover_cleanup", { id });

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
export const ensurePttStarted = () => invoke<void>("ensure_ptt_started");

export const getLocalModelStatuses = () =>
  invoke<LocalModelStatus[]>("get_local_model_statuses");

export const startModelDownload = (model: LocalWhisperModel) =>
  invoke<void>("start_model_download", { model });

export const cancelModelDownload = (model: LocalWhisperModel) =>
  invoke<void>("cancel_model_download", { model });

export const deleteLocalModel = (model: LocalWhisperModel) =>
  invoke<void>("delete_local_model", { model });

export const getLocalModelPath = (model: LocalWhisperModel) =>
  invoke<string>("get_local_model_path", { model });

export const setLocalWhisperIdleTimeout = (timeout: LocalWhisperIdleTimeout) =>
  invoke<void>("set_local_whisper_idle_timeout", { timeout });

export const getAppIcon = (bundleId: string) =>
  invoke<string | null>("get_app_icon", { bundleId });
