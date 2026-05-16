import { invoke } from "@tauri-apps/api/core";

import type {
  ApiKeyValidation,
  CleanupAuthMode,
  CleanupStats,
  DictionaryEntry,
  GroqSettings,
  HistoryEntry,
  HistoryLimit,
  HotkeyBinding,
  Mode,
  Settings,
  Shortcut,
  Snippet,
  StatsRow,
  TranscriptionProvider,
} from "./types";

export const getSettings = () => invoke<Settings>("get_settings");

export const setTranscriptionProvider = (provider: TranscriptionProvider) =>
  invoke<void>("set_transcription_provider", { provider });

export const setDeepgramApiKey = (apiKey: string) =>
  invoke<void>("set_deepgram_api_key", { apiKey });

export const setGroqApiKey = (apiKey: string) =>
  invoke<void>("set_groq_api_key", { apiKey });

export const setGroqSettings = (groq: GroqSettings) =>
  invoke<void>("set_groq_settings", { groq });

export const validateDeepgramApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_deepgram_api_key", { apiKey });

export const validateGroqApiKey = (apiKey: string) =>
  invoke<ApiKeyValidation>("validate_groq_api_key", { apiKey });

export const setHotkeyBindings = (bindings: HotkeyBinding[]) =>
  invoke<void>("set_hotkey_bindings", { bindings });

export const setShortcutCapturePaused = (paused: boolean) =>
  invoke<void>("set_shortcut_capture_paused", { paused });

export const setDictionary = (dictionary: DictionaryEntry[]) =>
  invoke<void>("set_dictionary", { dictionary });

export const setSnippets = (snippets: Snippet[]) =>
  invoke<void>("set_snippets", { snippets });

export const setDefaultModeCleanupEnabled = (enabled: boolean) =>
  invoke<void>("set_default_mode_cleanup_enabled", { enabled });

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

const MOD_MAP: Record<string, string> = {
  Meta: "⌘",
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
};

const KEY_MAP: Record<string, string> = {
  AltRight: "Right ⌥",
  AltLeft: "Left ⌥",
  MetaRight: "Right ⌘",
  MetaLeft: "Left ⌘",
  ControlRight: "Right ⌃",
  ControlLeft: "Left ⌃",
  ShiftRight: "Right ⇧",
  ShiftLeft: "Left ⇧",
  Space: "Space",
  Escape: "Esc",
  Tab: "Tab",
  Enter: "Return",
  Backspace: "Del",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

const displayKey = (code: string): string => {
  if (KEY_MAP[code]) return KEY_MAP[code];
  // KeyA → A, KeyZ → Z
  const keyMatch = code.match(/^Key([A-Z])$/);
  if (keyMatch) return keyMatch[1];
  // Digit0 → 0, Digit9 → 9
  const digitMatch = code.match(/^Digit(\d)$/);
  if (digitMatch) return digitMatch[1];
  // F1..F20 → F1..F20
  if (/^F\d{1,2}$/.test(code)) return code;
  return code;
};

export const formatShortcut = (s: Shortcut): string => {
  const mods = s.modifiers.map((m) => MOD_MAP[m] ?? m).join(" + ");
  const key = displayKey(s.key);
  const base = mods ? `${mods} + ${key}` : key;
  return s.is_double_tap ? `${base} (double-tap)` : base;
};
