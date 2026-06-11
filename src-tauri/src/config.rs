use crate::mode::{Mode, ModeId, SetId, SEED_MODE_UA_EN};
pub use crate::provider::{AssemblyAiModel, GroqModel, ProviderModel, TranscriptionProvider};
pub use crate::tone::TonePreset;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const SETTINGS_FILE: &str = "settings.json";

pub const SEED_TERM_SET_DEFAULT_ID: &str = "term-set-default";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedTermSet {
    pub id: SetId,
    pub name: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<String>,
    #[serde(default)]
    pub is_double_tap: bool,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: "AltRight".to_string(),
            modifiers: vec![],
            is_double_tap: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HotkeyAction {
    Ptt { mode_id: ModeId },
    PasteLatest,
    RecoverLatest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HotkeyBinding {
    pub shortcut: Shortcut,
    pub action: HotkeyAction,
}

impl HotkeyBinding {
    pub fn ptt(shortcut: Shortcut, mode_id: ModeId) -> Self {
        Self {
            shortcut,
            action: HotkeyAction::Ptt { mode_id },
        }
    }

    pub fn paste_latest(shortcut: Shortcut) -> Self {
        Self {
            shortcut,
            action: HotkeyAction::PasteLatest,
        }
    }

    pub fn recover_latest(shortcut: Shortcut) -> Self {
        Self {
            shortcut,
            action: HotkeyAction::RecoverLatest,
        }
    }
}

/// Accepts both the new `{shortcut, action}` shape and the legacy
/// `{shortcut, mode_id}` shape. Legacy entries deserialize as
/// `HotkeyAction::Ptt { mode_id }` so existing settings.json files keep
/// working across this upgrade.
impl<'de> Deserialize<'de> for HotkeyBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            shortcut: Shortcut,
            #[serde(default)]
            action: Option<HotkeyAction>,
            #[serde(default)]
            mode_id: Option<ModeId>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let action = match (raw.action, raw.mode_id) {
            (Some(action), _) => action,
            (None, Some(mode_id)) => HotkeyAction::Ptt { mode_id },
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "hotkey binding must have either `action` or legacy `mode_id`",
                ))
            }
        };
        Ok(HotkeyBinding {
            shortcut: raw.shortcut,
            action,
        })
    }
}

/// Same key+modifiers with different is_double_tap are distinct shortcuts and allowed.
pub fn check_hotkey_conflicts(bindings: &[HotkeyBinding]) -> Result<(), String> {
    let mut seen: HashSet<(&str, Vec<&str>, bool)> = HashSet::new();
    for b in bindings {
        let mods: Vec<&str> = b.shortcut.modifiers.iter().map(String::as_str).collect();
        if !seen.insert((b.shortcut.key.as_str(), mods, b.shortcut.is_double_tap)) {
            return Err(
                "Shortcut conflict: the same key combination is used by more than one binding."
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// At most one binding per PTT mode_id, at most one PasteLatest binding, and at
/// most one RecoverLatest binding total. Two bindings for the same action would
/// fire identically on different gestures — redundant and confusing in the UI.
pub fn check_action_constraints(bindings: &[HotkeyBinding]) -> Result<(), String> {
    let mut seen_modes: HashSet<&str> = HashSet::new();
    let mut paste_latest_count = 0;
    let mut recover_latest_count = 0;
    for b in bindings {
        match &b.action {
            HotkeyAction::Ptt { mode_id } => {
                if !seen_modes.insert(mode_id.as_str()) {
                    return Err(format!(
                        "Mode '{mode_id}' already has a hotkey. Each mode supports a single binding."
                    ));
                }
            }
            HotkeyAction::PasteLatest => {
                paste_latest_count += 1;
                if paste_latest_count > 1 {
                    return Err(
                        "Paste Latest already has a hotkey. Only one Paste Latest binding is allowed."
                            .to_string(),
                    );
                }
            }
            HotkeyAction::RecoverLatest => {
                recover_latest_count += 1;
                if recover_latest_count > 1 {
                    return Err(
                        "Recover Latest already has a hotkey. Only one Recover Latest binding is allowed."
                            .to_string(),
                    );
                }
            }
        }
    }
    Ok(())
}

/// Legacy two-field entry; only used for reading old `dictionary` JSON during migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectionEntry {
    pub from: String,
    pub to: String,
}

pub const DEFAULT_CORRECTION_SET_ID: &str = "correction-set-default";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NamedCorrectionSet {
    pub id: SetId,
    pub name: String,
    pub entries: Vec<CorrectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LearnedKind {
    Correction { from: String },
    Term,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LearnedEntryStatus {
    Candidate,
    Promoted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedEntry {
    pub id: String,
    pub word: String,
    #[serde(flatten)]
    pub kind: LearnedKind,
    pub status: LearnedEntryStatus,
    pub total_observations: u32,
    pub last_observed_ms: i64,
    #[serde(default)]
    pub per_app_observations: std::collections::BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetEntry {
    pub id: String,
    pub trigger: String,
    pub expansion: String,
}

/// Language is now owned by Mode; this field is read from legacy JSON during
/// migration and never written back (skip_serializing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeepgramSettings {
    #[serde(default, skip_serializing)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssemblyAiSettings {
    #[serde(default, skip_serializing)]
    pub model: AssemblyAiModel,
}

/// Language and model are now owned by Mode; these fields are read from legacy
/// JSON during migration and never written back (skip_serializing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqSettings {
    #[serde(default, skip_serializing)]
    pub model: GroqModel,
    #[serde(default, skip_serializing)]
    pub language: Option<String>,
}

impl Default for GroqSettings {
    fn default() -> Self {
        Self {
            model: GroqModel::default(),
            language: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupAuthMode {
    #[default]
    ApiKey,
    Oauth,
}

pub const DEFAULT_CLEANUP_MIN_WORDS: usize = 9;
pub const DEFAULT_CLEANUP_MIN_DURATION_MS: u64 = 3000;

fn default_cleanup_min_words() -> usize {
    DEFAULT_CLEANUP_MIN_WORDS
}

fn default_cleanup_min_duration_ms() -> u64 {
    DEFAULT_CLEANUP_MIN_DURATION_MS
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomProvider {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCleanupSettings {
    /// Legacy global master switch; per-mode `ai_cleanup.enabled` is now the
    /// single source of truth. Read during migration: if explicitly `false`,
    /// every mode's cleanup toggle is forced off (preserving the user's
    /// previous "all-off" intent). Then dropped from subsequent saves.
    #[serde(rename = "enabled", default, skip_serializing)]
    pub legacy_enabled: Option<bool>,
    #[serde(default)]
    pub auth_mode: CleanupAuthMode,
    /// Legacy Anthropic API key; migrated to `provider_keys["anthropic"]` on first load.
    #[serde(default, skip_serializing)]
    pub anthropic_api_key: Option<String>,
    /// Long-lived `sk-ant-oat…` token from `claude setup-token`. Sent as
    /// `Authorization: Bearer` against the OAuth-gated Messages endpoint —
    /// outside Anthropic's recommended path; see README for caveats.
    #[serde(default)]
    pub anthropic_oauth_token: Option<String>,
    /// Per-provider API keys. Keys are provider ID strings (`anthropic`, `openai`, …).
    #[serde(default)]
    pub provider_keys: BTreeMap<String, String>,
    #[serde(default)]
    pub custom_provider: Option<CustomProvider>,
    /// Minimum word count at which cleanup runs. Below this, dictations paste
    /// raw to preserve snappiness for short utterances.
    #[serde(default = "default_cleanup_min_words")]
    pub min_words: usize,
    #[serde(default = "default_cleanup_min_duration_ms")]
    pub min_duration_ms: u64,
    // Defaults to false so existing behaviour is unchanged after upgrade.
    #[serde(default)]
    pub tone_overlay_enabled: bool,
    #[serde(default)]
    pub tone_app_overrides: BTreeMap<String, TonePreset>,
}

impl Default for AiCleanupSettings {
    fn default() -> Self {
        Self {
            legacy_enabled: None,
            auth_mode: CleanupAuthMode::default(),
            anthropic_api_key: None,
            anthropic_oauth_token: None,
            provider_keys: BTreeMap::new(),
            custom_provider: None,
            min_words: DEFAULT_CLEANUP_MIN_WORDS,
            min_duration_ms: DEFAULT_CLEANUP_MIN_DURATION_MS,
            tone_overlay_enabled: false,
            tone_app_overrides: BTreeMap::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// `None` = unlimited, `Some(0)` = off (no history kept), `Some(n)` = keep n.
fn default_history_limit() -> Option<usize> {
    Some(5)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalWhisperIdleTimeout {
    FiveMinutes,
    #[default]
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    Never,
}

impl LocalWhisperIdleTimeout {
    pub fn as_duration(self) -> Option<std::time::Duration> {
        match self {
            Self::FiveMinutes => Some(std::time::Duration::from_secs(300)),
            Self::FifteenMinutes => Some(std::time::Duration::from_secs(900)),
            Self::ThirtyMinutes => Some(std::time::Duration::from_secs(1800)),
            Self::OneHour => Some(std::time::Duration::from_secs(3600)),
            Self::Never => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LocalWhisperSettings {
    #[serde(default)]
    pub idle_timeout: LocalWhisperIdleTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Legacy single-provider field. Read on load for migration into
    /// `deepgram_api_key`, then cleared so it drops out of subsequent saves.
    /// New code should not write to this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Legacy flat cleanup toggle; read during migration to seed the default
    /// mode's `ai_cleanup.enabled`, then cleared.
    #[serde(default, skip_serializing)]
    pub ai_cleanup_enabled: Option<bool>,
    /// Legacy field; renamed to `dictionary`. Read during migration, never written back.
    #[serde(rename = "replacements", default, skip_serializing)]
    pub legacy_replacements: Option<Vec<DictionaryEntry>>,
    #[serde(default, skip_serializing)]
    pub transcription_provider: TranscriptionProvider,
    #[serde(default)]
    pub deepgram_api_key: Option<String>,
    #[serde(default)]
    pub groq_api_key: Option<String>,
    #[serde(default)]
    pub assemblyai_api_key: Option<String>,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default)]
    pub elevenlabs_api_key: Option<String>,
    /// Legacy single-shortcut field; converted to a HotkeyBinding on first load.
    #[serde(rename = "shortcut", default, skip_serializing)]
    pub legacy_shortcut: Shortcut,
    #[serde(default)]
    pub hotkey_bindings: Vec<HotkeyBinding>,
    /// Legacy unified dictionary field; split into terms + corrections during migration.
    #[serde(rename = "dictionary", default, skip_serializing)]
    pub legacy_dictionary: Vec<DictionaryEntry>,
    /// Legacy flat term list; read during migration to seed a NamedTermSet, never written back.
    #[serde(default, skip_serializing)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub term_sets: Vec<NamedTermSet>,
    /// Legacy flat corrections list; seeded into the "Default Corrections" set on
    /// first migration, then dropped from subsequent saves.
    #[serde(rename = "corrections", default, skip_serializing)]
    pub legacy_corrections: Vec<CorrectionEntry>,
    #[serde(default)]
    pub correction_sets: Vec<NamedCorrectionSet>,
    #[serde(default)]
    pub snippets: Vec<SnippetEntry>,
    #[serde(default)]
    pub deepgram: DeepgramSettings,
    #[serde(default)]
    pub groq: GroqSettings,
    #[serde(default)]
    pub assemblyai: AssemblyAiSettings,
    #[serde(default)]
    pub ai_cleanup: AiCleanupSettings,
    #[serde(default)]
    pub modes: Vec<Mode>,
    #[serde(default)]
    pub input_device: Option<String>,
    #[serde(default = "default_true")]
    pub pause_media_on_record: bool,
    #[serde(default = "default_history_limit")]
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub show_in_dock: bool,
    #[serde(default)]
    pub start_at_login: bool,
    #[serde(default = "default_true")]
    pub show_live_preview: bool,
    #[serde(default)]
    pub local_whisper: LocalWhisperSettings,
    #[serde(default)]
    pub learn_from_corrections: bool,
    #[serde(default)]
    pub learned_entries: Vec<LearnedEntry>,
    /// from-words that have experienced an inconsistent mapping replacement.
    /// Once a from-word is here, new observations for it never create a Correction
    /// (only Terms), preventing replacement cycles.
    #[serde(default)]
    pub learned_inconsistent_from: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: None,
            ai_cleanup_enabled: None,
            legacy_replacements: None,
            transcription_provider: TranscriptionProvider::default(),
            deepgram_api_key: None,
            groq_api_key: None,
            assemblyai_api_key: None,
            openai_api_key: None,
            elevenlabs_api_key: None,
            legacy_shortcut: Shortcut::default(),
            hotkey_bindings: vec![],
            legacy_dictionary: vec![],
            terms: vec![],
            term_sets: vec![],
            legacy_corrections: vec![],
            correction_sets: vec![],
            snippets: vec![],
            deepgram: DeepgramSettings::default(),
            groq: GroqSettings::default(),
            assemblyai: AssemblyAiSettings::default(),
            ai_cleanup: AiCleanupSettings::default(),
            modes: vec![],
            input_device: None,
            pause_media_on_record: true,
            history_limit: default_history_limit(),
            show_in_dock: false,
            start_at_login: false,
            show_live_preview: true,
            local_whisper: LocalWhisperSettings::default(),
            learn_from_corrections: false,
            learned_entries: vec![],
            learned_inconsistent_from: vec![],
        }
    }
}

/// Migrates legacy settings into the new shape. Returns `true` if any change
/// was made so the caller can re-save.
fn migrate(s: &mut Settings) -> bool {
    let mut changed = false;

    if let Some(legacy) = s.api_key.take() {
        let deepgram_already_set = s.deepgram_api_key.as_deref().is_some_and(|k| !k.is_empty());
        if !legacy.is_empty() && !deepgram_already_set {
            s.deepgram_api_key = Some(legacy);
        }
        changed = true;
    }

    if let Some(key) = s.ai_cleanup.anthropic_api_key.take() {
        changed = true;
        if !key.is_empty() && !s.ai_cleanup.provider_keys.contains_key("anthropic") {
            s.ai_cleanup
                .provider_keys
                .insert("anthropic".to_string(), key);
        }
    }

    // Profiles are never auto-created or backfilled: a fresh install starts with
    // an empty list and the user builds their own. Deleted profiles stay deleted.

    if s.ai_cleanup_enabled.take().is_some() {
        changed = true;
    }

    // If the user had explicitly turned it off, preserve that intent by forcing
    // every mode's per-mode cleanup toggle off — otherwise modes that defaulted
    // to enabled would silently start running cleanup after the migration.
    match s.ai_cleanup.legacy_enabled.take() {
        Some(false) => {
            for mode in s.modes.iter_mut() {
                mode.ai_cleanup.enabled = false;
            }
            changed = true;
        }
        Some(true) => changed = true,
        None => {}
    }

    // Configs saved before issue #90 carried translation via a dedicated Apple
    // Translate stage (now removed). Those modes serialised as
    // `ai_cleanup:{enabled:true, prompt_override:null}`. On upgrade the unknown
    // `translate` field is silently ignored, leaving the mode using the default
    // English cleanup rules on Ukrainian text instead of translating. Set the
    // translation prompt when the slot is still empty so existing users keep
    // the behaviour they had. A user-supplied prompt_override is left untouched.
    if let Some(ua_en) = s.modes.iter_mut().find(|m| m.id == SEED_MODE_UA_EN) {
        if ua_en.ai_cleanup.enabled && ua_en.ai_cleanup.prompt_override.is_none() {
            ua_en.ai_cleanup.prompt_override = Mode::seed_ua_en().ai_cleanup.prompt_override;
            changed = true;
        }
    }

    if let Some(legacy) = s.legacy_replacements.take() {
        s.legacy_dictionary = legacy;
        changed = true;
    }

    if !s.legacy_dictionary.is_empty() {
        let mut terms = Vec::new();
        let mut corrections = Vec::new();
        for entry in s.legacy_dictionary.drain(..) {
            if entry.from == entry.to {
                terms.push(entry.from);
            } else {
                corrections.push(CorrectionEntry {
                    from: entry.from,
                    to: entry.to,
                });
            }
        }
        s.terms = terms;
        s.legacy_corrections = corrections;
        changed = true;
    }

    for mode in s.modes.iter_mut() {
        if let Some(use_dict) = mode.legacy_use_dictionary {
            if !use_dict {
                mode.use_terms = false;
                mode.use_corrections = false;
            }
            mode.legacy_use_dictionary = None;
            changed = true;
        }
    }

    // Only runs when the global provider was explicitly non-default (Groq or
    // AssemblyAI) AND a mode still has the default Deepgram — which means it
    // was loaded from old JSON that predates per-mode providers.
    if s.transcription_provider != TranscriptionProvider::Deepgram {
        let target =
            ProviderModel::from_legacy(s.transcription_provider, s.groq.model, s.assemblyai.model);
        for mode in s.modes.iter_mut() {
            if mode.provider_model == ProviderModel::default() {
                mode.provider_model = target.clone();
                changed = true;
            }
        }
    }

    if !s.terms.is_empty() {
        let entries: Vec<String> = std::mem::take(&mut s.terms);
        if !s
            .term_sets
            .iter()
            .any(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
        {
            s.term_sets.push(NamedTermSet {
                id: SEED_TERM_SET_DEFAULT_ID.to_string(),
                name: "Default Terms".to_string(),
                entries,
            });
        }
        for mode in s.modes.iter_mut() {
            if mode.use_terms
                && !mode
                    .term_set_ids
                    .contains(&SEED_TERM_SET_DEFAULT_ID.to_string())
            {
                mode.term_set_ids.push(SEED_TERM_SET_DEFAULT_ID.to_string());
            }
        }
        changed = true;
    }

    if s.correction_sets.is_empty() && !s.legacy_corrections.is_empty() {
        let default_set = NamedCorrectionSet {
            id: DEFAULT_CORRECTION_SET_ID.to_string(),
            name: "Default Corrections".to_string(),
            entries: s.legacy_corrections.clone(),
        };
        s.correction_sets.push(default_set);
        let set_id = DEFAULT_CORRECTION_SET_ID.to_string();
        for mode in s.modes.iter_mut() {
            if mode.use_corrections && !mode.correction_set_ids.contains(&set_id) {
                mode.correction_set_ids.push(set_id.clone());
            }
        }
        changed = true;
    }

    // On first load after this upgrade the bindings list will be empty
    // (the old JSON only has "shortcut"). Seed it from the legacy field, bound
    // to the first mode (the seeding above guarantees at least one exists).
    if s.hotkey_bindings.is_empty() {
        if let Some(first_mode_id) = s.modes.first().map(|m| m.id.clone()) {
            s.hotkey_bindings
                .push(HotkeyBinding::ptt(s.legacy_shortcut.clone(), first_mode_id));
            changed = true;
        }
    }

    // PasteLatest bindings are independent of modes and always retained.
    {
        let mode_ids: HashSet<&str> = s.modes.iter().map(|m| m.id.as_str()).collect();
        let before = s.hotkey_bindings.len();
        s.hotkey_bindings.retain(|b| match &b.action {
            HotkeyAction::Ptt { mode_id } => mode_ids.contains(mode_id.as_str()),
            HotkeyAction::PasteLatest | HotkeyAction::RecoverLatest => true,
        });
        if s.hotkey_bindings.len() != before {
            changed = true;
        }
    }

    // Older settings (pre one-binding-per-action) could carry duplicates.
    {
        let before = s.hotkey_bindings.len();
        let mut seen_modes: HashSet<String> = HashSet::new();
        let mut seen_paste_latest = false;
        let mut seen_recover_latest = false;
        s.hotkey_bindings.retain(|b| match &b.action {
            HotkeyAction::Ptt { mode_id } => seen_modes.insert(mode_id.clone()),
            HotkeyAction::PasteLatest => {
                if seen_paste_latest {
                    false
                } else {
                    seen_paste_latest = true;
                    true
                }
            }
            HotkeyAction::RecoverLatest => {
                if seen_recover_latest {
                    false
                } else {
                    seen_recover_latest = true;
                    true
                }
            }
        });
        if s.hotkey_bindings.len() != before {
            changed = true;
        }
    }

    changed
}

/// Parse `json` into a `Settings` value and apply any pending migrations.
///
/// Useful for testing migration behaviour without a live Tauri `AppHandle`.
pub fn from_json(json: &str) -> Result<Settings, serde_json::Error> {
    let mut s: Settings = serde_json::from_str(json)?;
    migrate(&mut s);
    Ok(s)
}

/// Like [`update`] but the closure may return an error to abort the save.
pub fn update_fallible<F>(app: &tauri::AppHandle, f: F) -> Result<(), String>
where
    F: FnOnce(&mut Settings) -> Result<(), String>,
{
    let mut settings = load(app);
    f(&mut settings)?;
    save(app, &settings)
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(SETTINGS_FILE))
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Settings path error: {e}");
            return Settings::default();
        }
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    let mut settings: Settings = serde_json::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("Failed to parse {path:?}, using defaults: {e}");
        Settings::default()
    });
    if migrate(&mut settings) {
        if let Err(e) = save(app, &settings) {
            eprintln!("Failed to re-save migrated settings: {e}");
        }
    }
    settings
}

pub fn update<F: FnOnce(&mut Settings)>(app: &tauri::AppHandle, f: F) -> Result<(), String> {
    let mut settings = load(app);
    f(&mut settings);
    save(app, &settings)
}

/// Empty strings come from cleared inputs and should be persisted as `None`.
pub fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn save(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write {path:?}: {e}"))?;
    // Defense-in-depth: API keys live in this file. Tighten to user-only
    // read/write even though the parent dir is already 0700 on macOS.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::SEED_MODE_DEFAULT_EN;

    #[test]
    fn tone_app_overrides_default_is_empty() {
        let s = AiCleanupSettings::default();
        assert!(s.tone_app_overrides.is_empty());
    }

    #[test]
    fn tone_app_overrides_round_trips_through_json() {
        let json = r#"{"ai_cleanup": {"tone_app_overrides": {"com.apple.mail": "casual"}}}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(
            s.ai_cleanup.tone_app_overrides.get("com.apple.mail").copied(),
            Some(TonePreset::Casual)
        );
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(
            v["ai_cleanup"]["tone_app_overrides"]["com.apple.mail"],
            serde_json::json!("casual")
        );
    }

    #[test]
    fn settings_without_tone_app_overrides_defaults_to_empty_map() {
        let json = r#"{}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.ai_cleanup.tone_app_overrides.is_empty());
    }

    #[test]
    fn default_settings_have_expected_provider_and_groq_defaults() {
        let s = Settings::default();
        assert_eq!(s.transcription_provider, TranscriptionProvider::Deepgram);
        assert_eq!(s.groq.model, GroqModel::WhisperLargeV3Turbo);
        assert!(s.deepgram_api_key.is_none());
        assert!(s.groq_api_key.is_none());
    }

    #[test]
    fn settings_without_elevenlabs_key_defaults_to_absent() {
        let json = r#"{}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.elevenlabs_api_key.is_none());
    }

    #[test]
    fn default_settings_start_with_no_profiles_or_bindings() {
        let s = Settings::default();
        assert!(s.modes.is_empty());
        assert!(s.hotkey_bindings.is_empty());
    }

    #[test]
    fn provider_enum_serializes_as_snake_case() {
        let s = serde_json::to_string(&TranscriptionProvider::Deepgram).unwrap();
        assert_eq!(s, "\"deepgram\"");
        let s = serde_json::to_string(&TranscriptionProvider::Groq).unwrap();
        assert_eq!(s, "\"groq\"");
    }

    #[test]
    fn groq_model_serializes_as_snake_case() {
        let s = serde_json::to_string(&GroqModel::WhisperLargeV3).unwrap();
        assert_eq!(s, "\"whisper_large_v3\"");
        let s = serde_json::to_string(&GroqModel::WhisperLargeV3Turbo).unwrap();
        assert_eq!(s, "\"whisper_large_v3_turbo\"");
    }

    #[test]
    fn migration_copies_legacy_api_key_to_deepgram_and_drops_legacy_field() {
        let legacy = r#"{"api_key": "dg-legacy-key"}"#;
        let mut s: Settings = serde_json::from_str(legacy).unwrap();
        assert_eq!(s.api_key.as_deref(), Some("dg-legacy-key"));
        assert!(s.deepgram_api_key.is_none());

        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.deepgram_api_key.as_deref(), Some("dg-legacy-key"));
        assert!(s.api_key.is_none());

        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(
            v.get("api_key").is_none(),
            "legacy api_key must drop out of subsequent saves, got: {reserialized}"
        );
        assert_eq!(
            v.get("deepgram_api_key").and_then(|x| x.as_str()),
            Some("dg-legacy-key")
        );
    }

    #[test]
    fn migration_preserves_existing_deepgram_api_key() {
        let json = r#"{"api_key": "old-key", "deepgram_api_key": "new-key"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();

        let changed = migrate(&mut s);
        assert!(changed, "legacy api_key should be cleared on save");
        assert_eq!(
            s.deepgram_api_key.as_deref(),
            Some("new-key"),
            "deepgram_api_key must be left untouched when already set"
        );
        assert!(s.api_key.is_none());

        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(v.get("api_key").is_none());
        assert_eq!(
            v.get("deepgram_api_key").and_then(|x| x.as_str()),
            Some("new-key")
        );
    }

    #[test]
    fn migration_global_ai_cleanup_off_forces_every_mode_off() {
        // Legacy flat `ai_cleanup.enabled = false` must force every existing
        // mode's per-mode cleanup off so nothing silently starts cleaning.
        let json = r#"{
            "ai_cleanup": {"enabled": false},
            "modes": [
                {"id":"a","name":"A","language":{"kind":"auto"},"ai_cleanup":{"enabled":true,"prompt_override":null},"use_snippets":true},
                {"id":"b","name":"B","language":{"kind":"auto"},"ai_cleanup":{"enabled":true,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        for mode in &s.modes {
            assert!(
                !mode.ai_cleanup.enabled,
                "mode {} must inherit the previous global-off intent",
                mode.id
            );
        }
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(
            v["ai_cleanup"].get("enabled").is_none(),
            "ai_cleanup.enabled must drop out of subsequent saves"
        );
    }

    #[test]
    fn migration_global_ai_cleanup_on_leaves_modes_alone() {
        // Legacy flat `ai_cleanup.enabled = true` must not touch per-mode toggles.
        let json = r#"{
            "ai_cleanup": {"enabled": true},
            "modes": [
                {"id":"on","name":"On","language":{"kind":"auto"},"ai_cleanup":{"enabled":true,"prompt_override":null},"use_snippets":true},
                {"id":"off","name":"Off","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let on = s.modes.iter().find(|m| m.id == "on").unwrap();
        assert!(on.ai_cleanup.enabled, "an enabled mode stays enabled");
        let off = s.modes.iter().find(|m| m.id == "off").unwrap();
        assert!(!off.ai_cleanup.enabled, "a disabled mode stays disabled");
    }

    #[test]
    fn migration_renames_legacy_replacements_then_splits_to_corrections() {
        let json = r#"{"replacements": [{"from": "dot", "to": "."}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.legacy_replacements.as_ref().map(|v| v.len()), Some(1));
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.legacy_corrections.len(), 1);
        assert_eq!(s.legacy_corrections[0].from, "dot");
        assert_eq!(s.legacy_corrections[0].to, ".");
        assert!(s.terms.is_empty());
        assert!(s.legacy_replacements.is_none());
        assert_eq!(s.correction_sets.len(), 1);
        assert_eq!(s.correction_sets[0].entries.len(), 1);
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(v.get("replacements").is_none());
        assert!(
            v.get("dictionary").is_none(),
            "dictionary must not be written back"
        );
        assert!(v.get("terms").is_none(), "terms is skip_serializing");
        assert!(
            v.get("corrections").is_none(),
            "legacy corrections must not be written back"
        );
        assert!(v.get("correction_sets").is_some());
    }

    #[test]
    fn migration_splits_dictionary_from_eq_to_as_term() {
        let json = r#"{"dictionary": [{"from": "MongoDB", "to": "MongoDB"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must be created");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        // No corrections in the dictionary → no Default Corrections set is seeded.
        assert!(s.legacy_corrections.is_empty());
        assert!(s.correction_sets.is_empty());
    }

    #[test]
    fn migration_splits_dictionary_from_ne_to_as_correction() {
        let json = r#"{"dictionary": [{"from": "anthropik", "to": "Anthropic"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        assert!(s.terms.is_empty());
        assert!(
            s.term_sets.is_empty(),
            "no terms to seed a Default Terms set"
        );
        assert_eq!(s.legacy_corrections.len(), 1);
        assert_eq!(s.legacy_corrections[0].from, "anthropik");
        assert_eq!(s.legacy_corrections[0].to, "Anthropic");
        assert_eq!(s.correction_sets[0].entries.len(), 1);
        assert_eq!(s.correction_sets[0].entries[0].from, "anthropik");
    }

    #[test]
    fn migration_splits_mixed_dictionary() {
        let json = r#"{"dictionary": [
            {"from": "MongoDB", "to": "MongoDB"},
            {"from": "dot", "to": "."},
            {"from": "anthropik", "to": "Anthropic"}
        ]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must exist");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        assert_eq!(s.legacy_corrections.len(), 2);
        assert_eq!(s.legacy_corrections[0].from, "dot");
        assert_eq!(s.legacy_corrections[1].from, "anthropik");
        assert_eq!(s.correction_sets[0].entries.len(), 2);
    }

    #[test]
    fn migration_dictionary_is_idempotent() {
        let json = r#"{"dictionary": [{"from": "MongoDB", "to": "MongoDB"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let changed2 = migrate(&mut s);
        assert!(!changed2, "second migrate must be a no-op");
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must exist");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        assert!(s.correction_sets.is_empty());
    }

    #[test]
    fn migration_mode_use_dictionary_false_sets_corrections_false() {
        let json = r#"{"modes": [
            {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
             "ai_cleanup":{"enabled":false,"prompt_override":null},
             "use_dictionary":false,"use_snippets":true}
        ], "default_mode_id": "mode-default-en"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s
            .modes
            .iter()
            .find(|m| m.id == SEED_MODE_DEFAULT_EN)
            .unwrap();
        assert!(!mode.use_corrections, "use_corrections must be false");
        assert!(
            mode.term_set_ids.is_empty(),
            "term_set_ids must be empty when use_dictionary was false"
        );
    }

    #[test]
    fn migration_mode_use_dictionary_true_leaves_corrections_true() {
        let json = r#"{"modes": [
            {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
             "ai_cleanup":{"enabled":false,"prompt_override":null},
             "use_dictionary":true,"use_snippets":true}
        ], "default_mode_id": "mode-default-en"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s
            .modes
            .iter()
            .find(|m| m.id == SEED_MODE_DEFAULT_EN)
            .unwrap();
        assert!(mode.use_corrections, "use_corrections must remain true");
        assert!(mode.term_set_ids.is_empty(), "no legacy terms to migrate");
    }

    #[test]
    fn migration_empty_legacy_terms_creates_no_term_set() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        assert!(
            s.term_sets.is_empty(),
            "no legacy terms → no Default Terms set"
        );
    }

    #[test]
    fn migration_nonempty_legacy_terms_seeds_default_term_set() {
        let json = r#"{"terms": ["MongoDB", "TypeScript"]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must exist");
        assert_eq!(set.name, "Default Terms");
        assert_eq!(set.entries, vec!["MongoDB", "TypeScript"]);
        assert!(s.terms.is_empty(), "legacy terms must be drained");
    }

    #[test]
    fn migration_modes_with_use_terms_true_reference_default_set() {
        let json = r#"{"terms": ["MongoDB"]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        for mode in &s.modes {
            assert!(
                mode.term_set_ids
                    .contains(&SEED_TERM_SET_DEFAULT_ID.to_string()),
                "mode '{}' must reference Default Terms set",
                mode.id
            );
        }
    }

    #[test]
    fn migration_mode_with_use_terms_false_does_not_reference_default_set() {
        let json = r#"{
            "terms": ["MongoDB"],
            "modes": [
                {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
                 "ai_cleanup":{"enabled":false,"prompt_override":null},
                 "use_terms":false,"use_corrections":true,"use_snippets":true,"term_set_ids":[]}
            ],
            "default_mode_id": "mode-default-en"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s
            .modes
            .iter()
            .find(|m| m.id == SEED_MODE_DEFAULT_EN)
            .unwrap();
        assert!(
            !mode
                .term_set_ids
                .contains(&SEED_TERM_SET_DEFAULT_ID.to_string()),
            "mode with use_terms=false must not reference the Default Terms set"
        );
    }

    #[test]
    fn migration_term_sets_is_idempotent() {
        let json = r#"{"terms": ["MongoDB"]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let set_count = s.term_sets.len();
        let changed2 = migrate(&mut s);
        assert!(!changed2, "second migrate must be a no-op");
        assert_eq!(s.term_sets.len(), set_count, "no duplicate term sets");
    }

    #[test]
    fn migration_modes_already_in_new_shape_are_unchanged() {
        let json = r#"{
            "term_sets": [{"id":"ts-1","name":"My Set","entries":["Rust"]}],
            "modes": [
                {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
                 "ai_cleanup":{"enabled":false,"prompt_override":null},
                 "use_corrections":true,"use_snippets":true,"term_set_ids":["ts-1"]}
            ],
            "default_mode_id": "mode-default-en"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s
            .modes
            .iter()
            .find(|m| m.id == SEED_MODE_DEFAULT_EN)
            .unwrap();
        assert_eq!(
            mode.term_set_ids,
            vec!["ts-1"],
            "term_set_ids must be unchanged"
        );
        assert_eq!(s.term_sets.len(), 1, "no extra sets added");
    }

    #[test]
    fn serialized_settings_has_no_terms_field_and_no_use_terms_on_modes() {
        let mut s = Settings::default();
        s.terms = vec!["MongoDB".to_string()];
        migrate(&mut s);
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.get("terms").is_none(), "terms is skip_serializing");
        for mode in v["modes"].as_array().unwrap() {
            assert!(
                mode.get("use_terms").is_none(),
                "use_terms is skip_serializing on Mode"
            );
        }
    }

    #[test]
    fn migrate_does_not_seed_profiles_into_an_empty_config() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.modes.is_empty());

        migrate(&mut s);

        assert!(s.modes.is_empty(), "a fresh config must stay profile-free");
    }

    #[test]
    fn migrate_leaves_an_explicit_profile_list_unchanged() {
        let json = r#"{
            "modes": [
                {"id":"a","name":"A","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true},
                {"id":"b","name":"B","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);

        let ids: Vec<&str> = s.modes.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"], "neither dropped nor backfilled");
    }

    #[test]
    fn migrate_does_not_backfill_seeds_into_a_non_empty_mode_list() {
        // A user who deleted every predefined profile but one keeps exactly that
        // list — migration must not resurrect the deleted seeds.
        let json = r#"{
            "modes": [{"id":"mode-default-en","name":"My Custom Name","language":{"kind":"exact","code":"en"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_terms":true,"use_corrections":true,"use_snippets":true}]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.modes.len(), 1);

        migrate(&mut s);

        assert_eq!(s.modes.len(), 1, "seeds must not be backfilled");
        assert_eq!(s.modes[0].id, SEED_MODE_DEFAULT_EN);
        assert_eq!(s.modes[0].name, "My Custom Name");
    }

    #[test]
    fn deepgram_language_does_not_appear_in_serialized_output() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            v["deepgram"].get("language").is_none(),
            "deepgram.language must not appear in serialized settings"
        );
    }

    #[test]
    fn groq_language_does_not_appear_in_serialized_output() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            v["groq"].get("language").is_none(),
            "groq.language must not appear in serialized settings"
        );
    }

    #[test]
    fn legacy_settings_round_trip_loses_language_fields() {
        let legacy = r#"{
            "api_key": "dg-key",
            "shortcut": {"key": "AltRight", "modifiers": []},
            "deepgram": {
                "language": "fr",
                "smart_format": true,
                "dictation": false,
                "numerals": false,
                "keyterms": []
            }
        }"#;
        let mut s: Settings = serde_json::from_str(legacy).unwrap();
        migrate(&mut s);
        assert_eq!(s.deepgram_api_key.as_deref(), Some("dg-key"));
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["deepgram"].get("language").is_none());
    }

    #[test]
    fn migration_converts_legacy_shortcut_to_hotkey_binding() {
        // The legacy single shortcut binds to the first profile in the list.
        let json = r#"{
            "shortcut": {"key": "MetaRight", "modifiers": []},
            "modes": [{"id":"first","name":"First","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.hotkey_bindings.is_empty());
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(s.hotkey_bindings[0].shortcut.key, "MetaRight");
        assert_eq!(
            s.hotkey_bindings[0].action,
            HotkeyAction::Ptt {
                mode_id: "first".to_string()
            }
        );
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(
            v.get("shortcut").is_none(),
            "legacy shortcut must drop out of subsequent saves"
        );
        assert!(v.get("hotkey_bindings").is_some());
    }

    #[test]
    fn migration_hotkey_bindings_is_idempotent() {
        let json = r#"{
            "modes": [{"id":"m1","name":"M","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}],
            "hotkey_bindings": [{"shortcut":{"key":"AltRight","modifiers":[]},"action":{"type":"Ptt","mode_id":"m1"}}]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        assert_eq!(s.hotkey_bindings.len(), 1);

        let changed = migrate(&mut s);
        assert!(
            !changed,
            "migrate on already-migrated settings must return false"
        );
        assert_eq!(s.hotkey_bindings.len(), 1);
    }

    #[test]
    fn migration_drops_orphaned_bindings_for_deleted_modes() {
        let mut s = Settings {
            modes: vec![Mode::seed_default_en(false)],
            ..Settings::default()
        };
        s.hotkey_bindings.push(HotkeyBinding::ptt(
            Shortcut {
                key: "AltRight".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            },
            SEED_MODE_DEFAULT_EN.to_string(),
        ));
        s.hotkey_bindings.push(HotkeyBinding::ptt(
            Shortcut {
                key: "MetaRight".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            },
            "mode-nonexistent".to_string(),
        ));
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(
            s.hotkey_bindings[0].action,
            HotkeyAction::Ptt {
                mode_id: SEED_MODE_DEFAULT_EN.to_string()
            }
        );
    }

    #[test]
    fn check_hotkey_conflicts_allows_distinct_shortcuts() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "MetaRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-cleaned-en".to_string(),
            ),
        ];
        assert!(check_hotkey_conflicts(&bindings).is_ok());
    }

    #[test]
    fn check_hotkey_conflicts_rejects_duplicate_shortcuts() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-cleaned-en".to_string(),
            ),
        ];
        assert!(check_hotkey_conflicts(&bindings).is_err());
    }

    #[test]
    fn shortcut_deserializes_without_is_double_tap_defaults_to_false() {
        let json = r#"{"key": "AltRight", "modifiers": []}"#;
        let s: Shortcut = serde_json::from_str(json).unwrap();
        assert_eq!(s.key, "AltRight");
        assert!(!s.is_double_tap);
    }

    #[test]
    fn check_hotkey_conflicts_allows_single_press_and_double_tap_same_key() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: true,
                },
                "mode-cleaned-en".to_string(),
            ),
        ];
        assert!(check_hotkey_conflicts(&bindings).is_ok());
    }

    #[test]
    fn check_action_constraints_allows_distinct_modes() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "MetaRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-cleaned-en".to_string(),
            ),
        ];
        assert!(check_action_constraints(&bindings).is_ok());
    }

    #[test]
    fn check_action_constraints_rejects_duplicate_mode() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: true,
                },
                "mode-default-en".to_string(),
            ),
        ];
        assert!(check_action_constraints(&bindings).is_err());
    }

    #[test]
    fn check_action_constraints_rejects_two_paste_latest_bindings() {
        let bindings = vec![
            HotkeyBinding::paste_latest(Shortcut {
                key: "F1".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
            HotkeyBinding::paste_latest(Shortcut {
                key: "F2".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
        ];
        assert!(check_action_constraints(&bindings).is_err());
    }

    #[test]
    fn check_action_constraints_allows_ptt_and_paste_latest_together() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::paste_latest(Shortcut {
                key: "F1".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
        ];
        assert!(check_action_constraints(&bindings).is_ok());
    }

    #[test]
    fn check_action_constraints_rejects_two_recover_latest_bindings() {
        let bindings = vec![
            HotkeyBinding::recover_latest(Shortcut {
                key: "F3".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
            HotkeyBinding::recover_latest(Shortcut {
                key: "F4".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
        ];
        assert!(check_action_constraints(&bindings).is_err());
    }

    #[test]
    fn check_action_constraints_allows_recover_latest_with_other_actions() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::paste_latest(Shortcut {
                key: "F1".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
            HotkeyBinding::recover_latest(Shortcut {
                key: "F2".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
        ];
        assert!(check_action_constraints(&bindings).is_ok());
    }

    #[test]
    fn migration_collapses_duplicate_mode_bindings_to_first() {
        let mut s = Settings {
            modes: vec![Mode::seed_default_en(false)],
            ..Settings::default()
        };
        s.hotkey_bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                SEED_MODE_DEFAULT_EN.to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: true,
                },
                SEED_MODE_DEFAULT_EN.to_string(),
            ),
        ];
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert!(!s.hotkey_bindings[0].shortcut.is_double_tap);
    }

    #[test]
    fn migration_collapses_duplicate_paste_latest_bindings_to_first() {
        let mut s = Settings::default();
        s.hotkey_bindings
            .push(HotkeyBinding::paste_latest(Shortcut {
                key: "F1".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }));
        s.hotkey_bindings
            .push(HotkeyBinding::paste_latest(Shortcut {
                key: "F2".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }));
        let changed = migrate(&mut s);
        assert!(changed);
        let paste_latest_count = s
            .hotkey_bindings
            .iter()
            .filter(|b| matches!(b.action, HotkeyAction::PasteLatest))
            .count();
        assert_eq!(paste_latest_count, 1);
    }

    #[test]
    fn legacy_hotkey_binding_with_mode_id_deserializes_as_ptt_action() {
        let json =
            r#"{"shortcut": {"key": "AltRight", "modifiers": []}, "mode_id": "mode-default-en"}"#;
        let binding: HotkeyBinding = serde_json::from_str(json).unwrap();
        assert_eq!(
            binding.action,
            HotkeyAction::Ptt {
                mode_id: "mode-default-en".to_string()
            }
        );
    }

    #[test]
    fn legacy_settings_with_mode_id_bindings_migrate_to_ptt_actions() {
        let json = r#"{
            "modes": [{"id":"mode-default-en","name":"M","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}],
            "hotkey_bindings": [
                {"shortcut": {"key": "AltRight", "modifiers": []}, "mode_id": "mode-default-en"}
            ]
        }"#;
        let s = from_json(json).unwrap();
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(
            s.hotkey_bindings[0].action,
            HotkeyAction::Ptt {
                mode_id: "mode-default-en".to_string()
            }
        );
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        let bindings = v["hotkey_bindings"].as_array().unwrap();
        assert!(
            bindings[0].get("mode_id").is_none(),
            "legacy mode_id must drop out of subsequent saves"
        );
        assert!(bindings[0].get("action").is_some());
    }

    #[test]
    fn paste_latest_binding_round_trips_through_json() {
        let binding = HotkeyBinding::paste_latest(Shortcut {
            key: "F1".to_string(),
            modifiers: vec![],
            is_double_tap: false,
        });
        let json = serde_json::to_string(&binding).unwrap();
        let decoded: HotkeyBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, binding);
    }

    #[test]
    fn check_hotkey_conflicts_rejects_two_double_tap_same_key() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: true,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::ptt(
                Shortcut {
                    key: "AltRight".to_string(),
                    modifiers: vec![],
                    is_double_tap: true,
                },
                "mode-cleaned-en".to_string(),
            ),
        ];
        assert!(check_hotkey_conflicts(&bindings).is_err());
    }

    #[test]
    fn check_hotkey_conflicts_rejects_ptt_and_paste_latest_sharing_shortcut() {
        let bindings = vec![
            HotkeyBinding::ptt(
                Shortcut {
                    key: "F1".to_string(),
                    modifiers: vec![],
                    is_double_tap: false,
                },
                "mode-default-en".to_string(),
            ),
            HotkeyBinding::paste_latest(Shortcut {
                key: "F1".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            }),
        ];
        assert!(check_hotkey_conflicts(&bindings).is_err());
    }

    #[test]
    fn migration_stamps_groq_provider_model_onto_default_provider_modes() {
        // A pre-per-mode-provider config: global Groq, modes still on the
        // default (Deepgram) provider_model get stamped with Groq.
        let json = r#"{
            "transcription_provider": "groq",
            "groq": {"model": "whisper_large_v3"},
            "modes": [
                {"id":"a","name":"A","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true},
                {"id":"b","name":"B","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        let expected = ProviderModel::Groq {
            model: GroqModel::WhisperLargeV3,
        };
        for mode in &s.modes {
            assert_eq!(
                mode.provider_model, expected,
                "mode {} should have Groq provider_model",
                mode.id
            );
        }
    }

    #[test]
    fn migration_stamps_assemblyai_provider_model_onto_default_provider_modes() {
        let json = r#"{
            "transcription_provider": "assembly_ai",
            "assemblyai": {"model": "whisper_streaming"},
            "modes": [
                {"id":"a","name":"A","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        let expected = ProviderModel::AssemblyAi {
            model: AssemblyAiModel::WhisperStreaming,
        };
        for mode in &s.modes {
            assert_eq!(
                mode.provider_model, expected,
                "mode {} should have AssemblyAI provider_model",
                mode.id
            );
        }
    }

    #[test]
    fn migration_provider_model_skips_already_customised_modes() {
        let json = r#"{
            "transcription_provider": "groq",
            "modes": [
                {"id":"a","name":"A","language":{"kind":"auto"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_snippets":true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.modes[0].provider_model = ProviderModel::AssemblyAi {
            model: AssemblyAiModel::default(),
        };
        migrate(&mut s);
        assert_eq!(
            s.modes[0].provider_model,
            ProviderModel::AssemblyAi {
                model: AssemblyAiModel::default()
            },
            "already-customised mode must not be overwritten"
        );
    }

    #[test]
    fn migration_deepgram_provider_does_not_stamp_modes() {
        let mut s = Settings::default();
        s.transcription_provider = TranscriptionProvider::Deepgram;
        let changed = migrate(&mut s);
        assert!(!changed, "deepgram is the default; no stamping needed");
        for mode in &s.modes {
            assert_eq!(mode.provider_model, ProviderModel::Deepgram);
        }
    }

    #[test]
    fn migration_provider_model_does_not_appear_in_reserialized_json() {
        let json = r#"{"transcription_provider": "groq"}"#;
        let s = from_json(json).unwrap();
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(
            v.get("transcription_provider").is_none(),
            "transcription_provider must be skipped on serialization"
        );
        assert!(
            v.get("groq").and_then(|g| g.get("model")).is_none(),
            "groq.model must be skipped on serialization"
        );
    }

    #[test]
    fn local_whisper_idle_timeout_default_is_fifteen_minutes() {
        assert_eq!(
            LocalWhisperIdleTimeout::default(),
            LocalWhisperIdleTimeout::FifteenMinutes
        );
        assert_eq!(
            LocalWhisperSettings::default().idle_timeout,
            LocalWhisperIdleTimeout::FifteenMinutes
        );
    }

    #[test]
    fn local_whisper_idle_timeout_as_duration_returns_correct_values() {
        use std::time::Duration;
        assert_eq!(
            LocalWhisperIdleTimeout::FiveMinutes.as_duration(),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            LocalWhisperIdleTimeout::FifteenMinutes.as_duration(),
            Some(Duration::from_secs(900))
        );
        assert_eq!(
            LocalWhisperIdleTimeout::ThirtyMinutes.as_duration(),
            Some(Duration::from_secs(1800))
        );
        assert_eq!(
            LocalWhisperIdleTimeout::OneHour.as_duration(),
            Some(Duration::from_secs(3600))
        );
        assert_eq!(LocalWhisperIdleTimeout::Never.as_duration(), None);
    }

    #[test]
    fn local_whisper_settings_round_trips_through_json() {
        let settings = LocalWhisperSettings {
            idle_timeout: LocalWhisperIdleTimeout::ThirtyMinutes,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let decoded: LocalWhisperSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn local_whisper_idle_timeout_serializes_as_snake_case() {
        let v = serde_json::to_value(LocalWhisperIdleTimeout::FifteenMinutes).unwrap();
        assert_eq!(v, serde_json::json!("fifteen_minutes"));
        let v = serde_json::to_value(LocalWhisperIdleTimeout::OneHour).unwrap();
        assert_eq!(v, serde_json::json!("one_hour"));
        let v = serde_json::to_value(LocalWhisperIdleTimeout::Never).unwrap();
        assert_eq!(v, serde_json::json!("never"));
    }

    #[test]
    fn settings_default_has_fifteen_minute_local_whisper_idle_timeout() {
        let s = Settings::default();
        assert_eq!(
            s.local_whisper.idle_timeout,
            LocalWhisperIdleTimeout::FifteenMinutes
        );
    }

    #[test]
    fn settings_preserves_local_whisper_idle_timeout_through_json() {
        let mut s = Settings::default();
        s.local_whisper.idle_timeout = LocalWhisperIdleTimeout::OneHour;
        let json = serde_json::to_string(&s).unwrap();
        let decoded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(
            decoded.local_whisper.idle_timeout,
            LocalWhisperIdleTimeout::OneHour
        );
    }

    #[test]
    fn settings_without_local_whisper_field_defaults_to_fifteen_minutes() {
        let json = r#"{}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(
            s.local_whisper.idle_timeout,
            LocalWhisperIdleTimeout::FifteenMinutes
        );
    }
}
