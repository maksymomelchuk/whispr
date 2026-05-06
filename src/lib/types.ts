export interface Shortcut {
  key: string;
  modifiers: string[];
}

export interface Replacement {
  from: string;
  to: string;
}

export interface DeepgramSettings {
  language: string;
  smart_format: boolean;
  dictation: boolean;
  numerals: boolean;
  keyterms: string[];
}

/// `null` = unlimited, `0` = off, `n` = keep last n.
export type HistoryLimit = number | null;

export interface Settings {
  api_key_configured: boolean;
  shortcut: Shortcut;
  replacements: Replacement[];
  deepgram: DeepgramSettings;
  ai_cleanup_enabled: boolean;
  ai_cleanup_key_configured: boolean;
  input_device: string | null;
  pause_media_on_record: boolean;
  history_limit: HistoryLimit;
}

export interface HistoryEntry {
  text: string;
  timestamp: number;
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
