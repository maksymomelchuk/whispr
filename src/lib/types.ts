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

export interface Settings {
  api_key_configured: boolean;
  shortcut: Shortcut;
  replacements: Replacement[];
  deepgram: DeepgramSettings;
  input_device: string | null;
  pause_media_on_record: boolean;
}

export interface HistoryEntry {
  text: string;
  timestamp: number;
}
