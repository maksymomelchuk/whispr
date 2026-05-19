export interface Shortcut {
  key: string;
  modifiers: string[];
  is_double_tap?: boolean;
}

export interface HotkeyBinding {
  shortcut: Shortcut;
  mode_id: string;
}

export interface CorrectionEntry {
  from: string;
  to: string;
}

export interface Snippet {
  id: string;
  trigger: string;
  expansion: string;
}

export type TranscriptionProvider = "deepgram" | "groq" | "assembly_ai";

export type GroqModel = "whisper_large_v3" | "whisper_large_v3_turbo";

export type AssemblyAiModel =
  | "universal_pro_streaming"
  | "universal_streaming_english"
  | "universal_streaming_multilingual"
  | "whisper_streaming";

export const ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES: Record<
  AssemblyAiModel,
  string[] | null
> = {
  universal_pro_streaming: ["en", "es", "de", "fr", "pt", "it"],
  universal_streaming_english: ["en"],
  universal_streaming_multilingual: ["en", "es", "de", "fr", "pt", "it"],
  whisper_streaming: null,
};

export type ProviderModel =
  | { provider: "deepgram" }
  | { provider: "groq"; model: GroqModel }
  | { provider: "assembly_ai"; model: AssemblyAiModel };

export function providerModelLanguageCodes(pm: ProviderModel): string[] | null {
  if (pm.provider !== "assembly_ai") return null;
  return ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES[pm.model];
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
  | { kind: "exact"; code: string }
  | { kind: "hints"; codes: string[] };

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
  use_terms: boolean;
  use_corrections: boolean;
  use_snippets: boolean;
  provider_model: ProviderModel;
}

// ── Settings ──────────────────────────────────────────────────────────────────

export interface Settings {
  deepgram_api_key_configured: boolean;
  groq_api_key_configured: boolean;
  assemblyai_api_key_configured: boolean;
  hotkey_bindings: HotkeyBinding[];
  terms: string[];
  corrections: CorrectionEntry[];
  snippets: Snippet[];
  modes: Mode[];
  default_mode_id: string;
  ai_cleanup_enabled: boolean;
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

// ── Capability matrix (from Tauri backend) ────────────────────────────────────

export interface ModelEntry {
  id: ProviderModel;
  supported_language_codes: string[] | null;
}

export interface ProviderEntry {
  id: TranscriptionProvider;
  models: ModelEntry[];
}

export interface CapabilityMatrix {
  providers: ProviderEntry[];
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
