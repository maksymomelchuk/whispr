export interface Shortcut {
  key: string;
  modifiers: string[];
  is_double_tap?: boolean;
}

export type HotkeyAction =
  | { type: "Ptt"; mode_id: string }
  | { type: "PasteLatest" }
  | { type: "RecoverLatest" };

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

export function recoverLatestBinding(shortcut: Shortcut): HotkeyBinding {
  return { shortcut, action: { type: "RecoverLatest" } };
}

export function isPasteLatestBinding(b: HotkeyBinding): boolean {
  return b.action.type === "PasteLatest";
}

export function isRecoverLatestBinding(b: HotkeyBinding): boolean {
  return b.action.type === "RecoverLatest";
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
  | "local"
  | "open_ai"
  | "eleven_labs"
  | "soniox";

export type GroqModel = "whisper_large_v3" | "whisper_large_v3_turbo";

export type OpenAiTranscribeModel = "gpt_transcribe";

export type ElevenLabsModel = "scribe_v3" | "scribe_v3_live";

export type AssemblyAiModel =
  | "stt_3_7_realtime"
  | "universal_3_6_pro"
  | "whisper_streaming"
  | "universal_2";

const ASSEMBLYAI_STREAMING_LANGUAGES = [
  "af",
  "ar",
  "ca",
  "da",
  "de",
  "en",
  "es",
  "et",
  "fa",
  "fi",
  "fr",
  "gl",
  "he",
  "hi",
  "it",
  "ja",
  "ko",
  "mr",
  "nb",
  "nl",
  "no",
  "pt",
  "ro",
  "ru",
  "sv",
  "tr",
  "ur",
  "vi",
  "xh",
  "yue",
  "zh",
  "zu",
];

export const ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES: Record<
  AssemblyAiModel,
  string[] | null
> = {
  stt_3_7_realtime: ASSEMBLYAI_STREAMING_LANGUAGES,
  universal_3_6_pro: ASSEMBLYAI_STREAMING_LANGUAGES,
  whisper_streaming: null,
  universal_2: null,
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
  | { provider: "local"; model: LocalWhisperModel }
  | { provider: "open_ai"; model: OpenAiTranscribeModel }
  | { provider: "eleven_labs"; model: ElevenLabsModel }
  | { provider: "soniox"; translate_to: string | null };

export function providerModelLanguageCodes(pm: ProviderModel): string[] | null {
  if (pm.provider !== "assembly_ai") return null;
  return ASSEMBLYAI_MODEL_SUPPORTED_LANGUAGES[pm.model];
}

const GROQ_MODEL_LABELS: Record<GroqModel, string> = {
  whisper_large_v3: "Whisper Large v3",
  whisper_large_v3_turbo: "Whisper Large v3-turbo",
};

const OPENAI_TRANSCRIBE_MODEL_LABELS: Record<OpenAiTranscribeModel, string> = {
  gpt_transcribe: "GPT Transcribe",
};

const ASSEMBLYAI_MODEL_LABELS: Record<AssemblyAiModel, string> = {
  stt_3_7_realtime: "STT-3.7 Realtime",
  universal_3_6_pro: "Universal-3.6 Pro",
  whisper_streaming: "Whisper Streaming",
  universal_2: "Universal-2 (multilingual)",
};

const LOCAL_MODEL_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "Large v3",
  large_v3_turbo: "Large v3 Turbo",
  parakeet: "Parakeet TDT",
};

const ELEVENLABS_MODEL_LABELS: Record<ElevenLabsModel, string> = {
  scribe_v3: "Scribe v3",
  scribe_v3_live: "Scribe v3 Live",
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
    case "open_ai":
      return `OpenAI · ${OPENAI_TRANSCRIBE_MODEL_LABELS[pm.model]}`;
    case "eleven_labs":
      return `ElevenLabs · ${ELEVENLABS_MODEL_LABELS[pm.model]}`;
    case "soniox":
      return pm.translate_to
        ? `Soniox · → ${pm.translate_to.toUpperCase()}`
        : "Soniox";
  }
}

/// `null` = unlimited, `0` = off, `n` = keep last n.
export type HistoryLimit = number | null;

export type TonePreset = "casual" | "formal" | "technical_casing" | "neutral";

export interface AppToneInfo {
  bundle_id: string;
  app_name: string;
  tone_preset: TonePreset;
  tone_override: TonePreset | null;
  custom_prompt: string | null;
  icon_data_url: string | null;
}

export type CleanupAuthMode = "api_key" | "oauth";

export type AiProviderId =
  | "anthropic"
  | "openai"
  | "google"
  | "groq"
  | "deepseek"
  | "cerebras"
  | "openrouter"
  | "custom";

export type ApiKeyValidation =
  | { kind: "valid" }
  | { kind: "invalid" }
  | { kind: "error"; message: string };

export type ModeLanguage =
  | { kind: "auto" }
  | { kind: "exact"; code: string }
  | { kind: "hints"; codes: string[] };

export interface ModeCleanup {
  enabled: boolean;
  prompt_override: string | null;
  provider: AiProviderId;
  model: string;
  paste_raw_on_failure: boolean;
  context_capture_enabled: boolean;
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

export interface Settings {
  deepgram_api_key_configured: boolean;
  groq_api_key_configured: boolean;
  assemblyai_api_key_configured: boolean;
  openai_api_key_configured: boolean;
  elevenlabs_api_key_configured: boolean;
  soniox_api_key_configured: boolean;
  hotkey_bindings: HotkeyBinding[];
  term_sets: NamedTermSet[];
  correction_sets: NamedCorrectionSet[];
  snippets: Snippet[];
  modes: Mode[];
  ai_cleanup_auth_mode: CleanupAuthMode;
  ai_cleanup_key_configured: boolean;
  ai_cleanup_oauth_token_configured: boolean;
  /** Provider IDs that have a saved API key. */
  configured_providers: AiProviderId[];
  custom_provider_configured: boolean;
  custom_provider_base_url: string | null;
  custom_provider_model: string;
  ai_cleanup_min_words: number;
  ai_cleanup_min_duration_ms: number;
  ai_cleanup_tone_overlay_enabled: boolean;
  tone_app_overrides: Record<string, TonePreset>;
  tone_app_custom_prompts: Record<string, string>;
  learn_from_corrections: boolean;
  input_device: string | null;
  pause_media_on_record: boolean;
  history_limit: HistoryLimit;
  save_audio_recordings: boolean;
  hands_free_max_minutes: number;
  show_in_dock: boolean;
  start_at_login: boolean;
  show_live_preview: boolean;
  local_whisper_idle_timeout: LocalWhisperIdleTimeout;
  meeting_summary: MeetingSummarySettings;
}

export interface LocalModelStatus {
  model: LocalWhisperModel;
  downloaded: boolean;
  load_failed: boolean;
  downloading: boolean;
  size_bytes: number;
}

export type ProviderHealthStatus = "valid" | "rejected" | "unreachable";

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
  | { kind: "recovered_manually" }
  | { kind: "failed_timeout" }
  | { kind: "failed_transient"; message: string }
  | { kind: "failed_credential"; message: string };

export interface ProfileSnapshot {
  cleanup_provider: AiProviderId;
  cleanup_model: string;
  cleanup_prompt_override: string | null;
  use_snippets: boolean;
  correction_set_ids: string[];
}

export interface HistoryEntry {
  /** Stable unique id generated at dictation time. Empty string on pre-upgrade entries. */
  id: string;
  timestamp: number;
  speak_duration_ms: number;
  raw_text: string;
  replaced_text: string;
  final_text: string;
  cleanup_status: CleanupStatus;
  profile_snapshot?: ProfileSnapshot | null;
  provider_model?: ProviderModel | null;
  app_name?: string | null;
  bundle_id?: string | null;
  context_channels?: string[];
  /** A saved mic recording exists at `recordings/{id}.flac`. */
  has_audio?: boolean;
  /** Pinned by the user: exempt from automatic retention. */
  favorite?: boolean;
}

export type LearnedKind =
  | { kind: "correction"; from: string }
  | { kind: "term" };

export type LearnedEntryStatus = "candidate" | "promoted";

export type LearnedEntry = {
  id: string;
  word: string;
  status: LearnedEntryStatus;
  total_observations: number;
  last_observed_ms: number;
} & LearnedKind;

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

export interface MeetingSummarySettings {
  provider: AiProviderId;
  model: string;
}

export type MeetingStatus =
  | { kind: "recording" }
  | { kind: "transcribing" }
  | { kind: "summarizing" }
  | { kind: "ready" }
  | { kind: "failed"; message: string };

export interface MeetingMeta {
  id: string;
  started_at: number;
  duration_ms: number;
  status: MeetingStatus;
  summary_model: string | null;
  system_audio_error: string | null;
}

export interface MeetingSegment {
  start_ms: number;
  end_ms: number;
  speaker: string;
  text: string;
}

export interface Meeting {
  meta: MeetingMeta;
  segments: MeetingSegment[];
  notes: string | null;
}
