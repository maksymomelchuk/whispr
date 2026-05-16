export interface Shortcut {
  key: string;
  modifiers: string[];
}

export interface HotkeyBinding {
  shortcut: Shortcut;
  mode_id: string;
}

export interface DictionaryEntry {
  from: string;
  to: string;
}

export type TranscriptionProvider = "deepgram" | "groq";

export type GroqModel = "whisper_large_v3" | "whisper_large_v3_turbo";

export interface GroqSettings {
  model: GroqModel;
}

/// `null` = unlimited, `0` = off, `n` = keep last n.
export type HistoryLimit = number | null;

export type CleanupAuthMode = "api_key" | "oauth";

export type ApiKeyValidation =
  | { kind: "valid" }
  | { kind: "invalid" }
  | { kind: "error"; message: string };

// ── Mode types ────────────────────────────────────────────────────────────────

export type ModeLanguage =
  | { kind: "auto" }
  | { kind: "exact"; code: string };

export type TranslateTarget =
  | { kind: "off" }
  | { kind: "apple"; target: string };

export interface ModeCleanup {
  enabled: boolean;
  prompt_override: string | null;
}

export interface Mode {
  id: string;
  name: string;
  icon: string | null;
  language: ModeLanguage;
  translate: TranslateTarget;
  ai_cleanup: ModeCleanup;
  use_dictionary: boolean;
  use_snippets: boolean;
}

// ── Settings ──────────────────────────────────────────────────────────────────

export interface Settings {
  transcription_provider: TranscriptionProvider;
  deepgram_api_key_configured: boolean;
  groq_api_key_configured: boolean;
  hotkey_bindings: HotkeyBinding[];
  dictionary: DictionaryEntry[];
  groq: GroqSettings;
  modes: Mode[];
  default_mode_id: string;
  ai_cleanup_auth_mode: CleanupAuthMode;
  ai_cleanup_key_configured: boolean;
  ai_cleanup_oauth_token_configured: boolean;
  ai_cleanup_min_words: number;
  ai_cleanup_min_duration_ms: number;
  input_device: string | null;
  pause_media_on_record: boolean;
  history_limit: HistoryLimit;
  show_in_dock: boolean;
  show_live_preview: boolean;
}

export type CleanupStatus =
  | { kind: "disabled" }
  | { kind: "skipped_below_min_words" }
  | { kind: "skipped_below_min_duration" }
  | { kind: "no_credential" }
  | { kind: "ran" }
  | { kind: "failed_timeout" }
  | { kind: "failed_transient"; message: string }
  | { kind: "failed_credential"; message: string };

export interface HistoryEntry {
  timestamp: number;
  speak_duration_ms: number;
  raw_text: string;
  replaced_text: string;
  final_text: string;
  cleanup_status: CleanupStatus;
}

export interface StatsRow {
  date: string;
  words: number;
  dictations: number;
  total_seconds: number;
}

export interface PeriodCounter {
  /// "YYYY-MM-DD" for today, "YYYY-MM" for the month counter.
  period: string;
  input_tokens: number;
  output_tokens: number;
}

export interface TotalCounter {
  input_tokens: number;
  output_tokens: number;
}

export interface CleanupStats {
  today: PeriodCounter;
  month: PeriodCounter;
  overall: TotalCounter;
}
