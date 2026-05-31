export interface Shortcut {
  key: string;
  modifiers: string[];
  is_double_tap?: boolean;
}

export type HotkeyAction =
  | { type: "Ptt"; mode_id: string }
  | { type: "PasteLatest" };

export interface HotkeyBinding {
  shortcut: Shortcut;
  action: HotkeyAction;
}

export function pttBinding(shortcut: Shortcut, modeId: string): HotkeyBinding {
  return { shortcut, action: { type: "Ptt", mode_id: modeId } };
}

export function pasteLatestBinding(shortcut: Shortcut): HotkeyBinding {
  return { shortcut, action: { type: "PasteLatest" } };
}

export function isPasteLatestBinding(b: HotkeyBinding): boolean {
  return b.action.type === "PasteLatest";
}

export function pttModeId(b: HotkeyBinding): string | null {
  return b.action.type === "Ptt" ? b.action.mode_id : null;
}

export interface CorrectionEntry {
  from: string;
  to: string;
}

export interface NamedCorrectionSet {
  id: string;
  name: string;
  entries: CorrectionEntry[];
}

export interface Snippet {
  id: string;
  trigger: string;
  expansion: string;
}

export type TranscriptionProvider =
  | "deepgram"
  | "groq"
  | "assembly_ai"
  | "local";

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

export type LocalWhisperModel = "large_v3" | "large_v3_turbo" | "parakeet";

export type LocalWhisperIdleTimeout =
  | "five_minutes"
  | "fifteen_minutes"
  | "thirty_minutes"
  | "one_hour"
  | "never";

export type ProviderModel =
  | { provider: "deepgram" }
  | { provider: "groq"; model: GroqModel }
  | { provider: "assembly_ai"; model: AssemblyAiModel }
  | { provider: "local"; model: LocalWhisperModel };

export function providerModelLanguageCodes(pm: ProviderModel): string[] | null {
  if (pm.provider !== "assembly_ai") return null;
  return ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES[pm.model];
}

const GROQ_MODEL_LABELS: Record<GroqModel, string> = {
  whisper_large_v3: "Whisper Large v3",
  whisper_large_v3_turbo: "Whisper Large v3-turbo",
};

const ASSEMBLYAI_MODEL_LABELS: Record<AssemblyAiModel, string> = {
  universal_pro_streaming: "Universal-3 Pro",
  universal_streaming_english: "Universal English",
  universal_streaming_multilingual: "Universal Multilingual",
  whisper_streaming: "Whisper Streaming",
};

const LOCAL_MODEL_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "Large v3",
  large_v3_turbo: "Large v3 Turbo",
  parakeet: "Parakeet TDT",
};

export function providerModelLabel(pm: ProviderModel): string {
  switch (pm.provider) {
    case "deepgram":
      return "Deepgram";
    case "groq":
      return `Groq · ${GROQ_MODEL_LABELS[pm.model]}`;
    case "assembly_ai":
      return `AssemblyAI · ${ASSEMBLYAI_MODEL_LABELS[pm.model]}`;
    case "local": {
      const prefix = pm.model === "parakeet" ? "Local" : "Local Whisper";
      return `${prefix} · ${LOCAL_MODEL_LABELS[pm.model]}`;
    }
  }
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

export interface ModeCleanup {
  enabled: boolean;
  prompt_override: string | null;
}

export interface NamedTermSet {
  id: string;
  name: string;
  entries: string[];
}

export interface Mode {
  id: string;
  name: string;
  icon: string | null;
  language: ModeLanguage;
  ai_cleanup: ModeCleanup;
  term_set_ids: string[];
  correction_set_ids: string[];
  use_snippets: boolean;
  provider_model: ProviderModel;
}

// ── Settings ──────────────────────────────────────────────────────────────────

export interface Settings {
  deepgram_api_key_configured: boolean;
  groq_api_key_configured: boolean;
  assemblyai_api_key_configured: boolean;
  hotkey_bindings: HotkeyBinding[];
  term_sets: NamedTermSet[];
  correction_sets: NamedCorrectionSet[];
  snippets: Snippet[];
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
  start_at_login: boolean;
  show_live_preview: boolean;
  local_whisper_idle_timeout: LocalWhisperIdleTimeout;
}

// ── Local model download types ─────────────────────────────────────────────

export interface LocalModelStatus {
  model: LocalWhisperModel;
  downloaded: boolean;
  downloading: boolean;
  size_bytes: number;
}

export interface ModelDownloadProgress {
  model: LocalWhisperModel;
  bytes_downloaded: number;
  total_bytes: number;
  percentage: number;
}

export interface ModelDownloadError {
  model: LocalWhisperModel;
  message: string;
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
  provider_model?: ProviderModel | null;
  app_name?: string | null;
  bundle_id?: string | null;
}

export interface AppUsage {
  name: string;
  count: number;
}

export interface StatsRow {
  date: string;
  words: number;
  dictations: number;
  total_seconds: number;
  app_counts?: Record<string, AppUsage>;
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
  week: PeriodCounter;
  month: PeriodCounter;
  overall: TotalCounter;
}
