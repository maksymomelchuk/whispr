use crate::mode::{
    Mode, ModeId, ModeLanguage, SetId, SEED_MODE_CLEANED_EN, SEED_MODE_DEFAULT_EN, SEED_MODE_UA_EN,
    SEED_MODE_UKRAINIAN,
};
pub use crate::provider::{AssemblyAiModel, GroqModel, ProviderModel, TranscriptionProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
pub struct HotkeyBinding {
    pub shortcut: Shortcut,
    pub mode_id: ModeId,
}

pub fn default_hotkey_bindings() -> Vec<HotkeyBinding> {
    vec![HotkeyBinding {
        shortcut: Shortcut::default(),
        mode_id: SEED_MODE_DEFAULT_EN.to_string(),
    }]
}

/// Returns `Err` if any two bindings share the same (key, modifiers, is_double_tap) triple.
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

/// Each mode supports at most one hotkey binding. Two bindings for the same
/// mode would fire identical actions on different gestures — redundant and
/// confusing in the UI.
pub fn check_one_binding_per_mode(bindings: &[HotkeyBinding]) -> Result<(), String> {
    let mut seen: HashSet<&str> = HashSet::new();
    for b in bindings {
        if !seen.insert(b.mode_id.as_str()) {
            return Err(format!(
                "Mode '{}' already has a hotkey. Each mode supports a single binding.",
                b.mode_id
            ));
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEntry {
    pub from: String,
    pub to: String,
}

pub const DEFAULT_CORRECTION_SET_ID: &str = "correction-set-default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedCorrectionSet {
    pub id: SetId,
    pub name: String,
    pub entries: Vec<CorrectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetEntry {
    pub id: String,
    pub trigger: String,
    pub expansion: String,
}

pub fn default_corrections() -> Vec<CorrectionEntry> {
    [
        ("dot", "."),
        ("slash", "/"),
        ("dash", "-"),
        ("underscore", "_"),
        ("at sign", "@"),
        ("comma", ","),
        ("colon", ":"),
        ("semicolon", ";"),
        ("question mark", "?"),
        ("exclamation mark", "!"),
    ]
    .into_iter()
    .map(|(from, to)| CorrectionEntry {
        from: from.to_string(),
        to: to.to_string(),
    })
    .collect()
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
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    /// Long-lived `sk-ant-oat…` token from `claude setup-token`. Sent as
    /// `Authorization: Bearer` against the OAuth-gated Messages endpoint —
    /// outside Anthropic's recommended path; see README for caveats.
    #[serde(default)]
    pub anthropic_oauth_token: Option<String>,
    /// Minimum word count at which cleanup runs. Below this, dictations paste
    /// raw to preserve snappiness for short utterances.
    #[serde(default = "default_cleanup_min_words")]
    pub min_words: usize,
    /// Minimum spoken duration (ms) at which cleanup runs.
    #[serde(default = "default_cleanup_min_duration_ms")]
    pub min_duration_ms: u64,
}

impl Default for AiCleanupSettings {
    fn default() -> Self {
        Self {
            legacy_enabled: None,
            auth_mode: CleanupAuthMode::default(),
            anthropic_api_key: None,
            anthropic_oauth_token: None,
            min_words: DEFAULT_CLEANUP_MIN_WORDS,
            min_duration_ms: DEFAULT_CLEANUP_MIN_DURATION_MS,
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

fn default_mode_id() -> ModeId {
    SEED_MODE_DEFAULT_EN.to_string()
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
    #[serde(rename = "corrections", default = "default_corrections", skip_serializing)]
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
    #[serde(default = "default_mode_id")]
    pub default_mode_id: ModeId,
    #[serde(default)]
    pub input_device: Option<String>,
    #[serde(default = "default_true")]
    pub pause_media_on_record: bool,
    #[serde(default = "default_history_limit")]
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub show_in_dock: bool,
    #[serde(default = "default_true")]
    pub show_live_preview: bool,
    #[serde(default)]
    pub local_whisper: LocalWhisperSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let default_set = NamedCorrectionSet {
            id: DEFAULT_CORRECTION_SET_ID.to_string(),
            name: "Default Corrections".to_string(),
            entries: default_corrections(),
        };
        let default_set_id = DEFAULT_CORRECTION_SET_ID.to_string();
        let mut mode_default_en = Mode::seed_default_en(false);
        let mut mode_cleaned_en = Mode::seed_cleaned_en();
        let mut mode_ukrainian = Mode::seed_ukrainian();
        let mut mode_ua_en = Mode::seed_ua_en();
        mode_default_en.correction_set_ids = vec![default_set_id.clone()];
        mode_cleaned_en.correction_set_ids = vec![default_set_id.clone()];
        mode_ukrainian.correction_set_ids = vec![default_set_id.clone()];
        mode_ua_en.correction_set_ids = vec![default_set_id.clone()];
        Self {
            api_key: None,
            ai_cleanup_enabled: None,
            legacy_replacements: None,
            transcription_provider: TranscriptionProvider::default(),
            deepgram_api_key: None,
            groq_api_key: None,
            assemblyai_api_key: None,
            legacy_shortcut: Shortcut::default(),
            hotkey_bindings: default_hotkey_bindings(),
            legacy_dictionary: vec![],
            terms: vec![],
            term_sets: vec![],
            legacy_corrections: default_corrections(),
            correction_sets: vec![default_set],
            snippets: vec![],
            deepgram: DeepgramSettings::default(),
            groq: GroqSettings::default(),
            assemblyai: AssemblyAiSettings::default(),
            ai_cleanup: AiCleanupSettings::default(),
            modes: vec![mode_default_en, mode_cleaned_en, mode_ukrainian, mode_ua_en],
            default_mode_id: default_mode_id(),
            input_device: None,
            pause_media_on_record: true,
            history_limit: default_history_limit(),
            show_in_dock: false,
            show_live_preview: true,
            local_whisper: LocalWhisperSettings::default(),
        }
    }
}

/// Returns the default mode from settings, falling back to the first mode if
/// `default_mode_id` doesn't match any entry, or creating a fallback mode if
/// `modes` is somehow empty.
pub fn get_default_mode(settings: &Settings) -> &Mode {
    settings
        .modes
        .iter()
        .find(|m| m.id == settings.default_mode_id)
        .or_else(|| settings.modes.first())
        .unwrap_or_else(|| {
            // Statically allocated fallback so we can return a reference.
            // Unreachable in practice: migration always seeds at least one mode.
            static FALLBACK: std::sync::OnceLock<Mode> = std::sync::OnceLock::new();
            FALLBACK.get_or_init(|| Mode::seed_default_en(false))
        })
}

/// Migrates legacy settings into the new shape. Returns `true` if any change
/// was made so the caller can re-save.
fn migrate(s: &mut Settings) -> bool {
    let mut changed = false;

    // ── Legacy api_key → deepgram_api_key ────────────────────────────────
    if let Some(legacy) = s.api_key.take() {
        let deepgram_already_set = s
            .deepgram_api_key
            .as_deref()
            .is_some_and(|k| !k.is_empty());
        if !legacy.is_empty() && !deepgram_already_set {
            s.deepgram_api_key = Some(legacy);
        }
        changed = true;
    }

    // ── Seed predefined modes ─────────────────────────────────────────────
    if s.modes.is_empty() {
        // Prefer the active provider's language, then the other provider's.
        // An empty/whitespace value is treated as missing.
        let non_empty = |opt: &Option<String>| -> Option<String> {
            opt.clone().filter(|l| !l.trim().is_empty())
        };
        let (primary, secondary) = match s.transcription_provider {
            TranscriptionProvider::Groq => (&s.groq.language, &s.deepgram.language),
            _ => (&s.deepgram.language, &s.groq.language),
        };
        let legacy_language = non_empty(primary).or_else(|| non_empty(secondary));

        let mut mode = Mode::seed_default_en(s.ai_cleanup_enabled.unwrap_or(false));
        if let Some(code) = legacy_language {
            mode.language = ModeLanguage::exact(code);
        }
        s.modes.push(mode);
        s.modes.push(Mode::seed_cleaned_en());
        s.modes.push(Mode::seed_ukrainian());
        s.modes.push(Mode::seed_ua_en());
        s.default_mode_id = SEED_MODE_DEFAULT_EN.to_string();
        changed = true;
    } else {
        // For any predefined mode that's absent, add it — idempotent across upgrades.
        for (id, seed) in [
            (SEED_MODE_DEFAULT_EN, Mode::seed_default_en(false)),
            (SEED_MODE_CLEANED_EN, Mode::seed_cleaned_en()),
            (SEED_MODE_UKRAINIAN, Mode::seed_ukrainian()),
            (SEED_MODE_UA_EN, Mode::seed_ua_en()),
        ] {
            if !s.modes.iter().any(|m| m.id == id) {
                s.modes.push(seed);
                changed = true;
            }
        }
    }

    // Drop the legacy flat cleanup toggle; it's now in the mode.
    if s.ai_cleanup_enabled.take().is_some() {
        changed = true;
    }

    // Drop the legacy global ai_cleanup.enabled switch. If the user had
    // explicitly turned it off, preserve that intent by forcing every mode's
    // per-mode cleanup toggle off — otherwise modes that defaulted to enabled
    // would silently start running cleanup after the migration.
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

    // ── Legacy replacements → dictionary (now terms + corrections) ───────
    if let Some(legacy) = s.legacy_replacements.take() {
        s.legacy_dictionary = legacy;
        changed = true;
    }

    // ── Legacy dictionary → terms + corrections ───────────────────────────
    // Each entry where from == to becomes a Term; all others become Corrections.
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

    // ── Mode migration: use_dictionary → use_terms + use_corrections ──────
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

    // ── Term sets migration: flat terms → NamedTermSet ───────────────────
    // If legacy flat terms remain, fold them into the Default Terms set
    // and wire each mode that had use_terms=true to reference it.
    if !s.terms.is_empty() {
        let entries: Vec<String> = std::mem::take(&mut s.terms);
        if !s.term_sets.iter().any(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID) {
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

    // ── Correction sets: seed "Default Corrections" from legacy flat list ──
    // Runs exactly once: when correction_sets is empty, the legacy corrections
    // (user's existing list or the seeded defaults) become a named set. Every
    // mode whose use_corrections flag was true gets the set ID appended to
    // correction_set_ids. Idempotent: skipped on all subsequent loads.
    if s.correction_sets.is_empty() {
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

    // ── Legacy shortcut → hotkey_bindings ────────────────────────────────
    // On first load after this upgrade the bindings list will be empty
    // (the old JSON only has "shortcut"). Seed it from the legacy field.
    if s.hotkey_bindings.is_empty() {
        s.hotkey_bindings.push(HotkeyBinding {
            shortcut: s.legacy_shortcut.clone(),
            mode_id: s.default_mode_id.clone(),
        });
        changed = true;
    }

    // Drop bindings that reference a mode that no longer exists.
    {
        let mode_ids: HashSet<&str> = s.modes.iter().map(|m| m.id.as_str()).collect();
        let before = s.hotkey_bindings.len();
        s.hotkey_bindings
            .retain(|b| mode_ids.contains(b.mode_id.as_str()));
        if s.hotkey_bindings.len() != before {
            changed = true;
        }
    }

    // Collapse to one binding per mode. Older settings (pre one-binding-per-mode)
    // could carry duplicates; keep the first occurrence and drop the rest.
    {
        let before = s.hotkey_bindings.len();
        let mut seen: HashSet<String> = HashSet::new();
        s.hotkey_bindings.retain(|b| seen.insert(b.mode_id.clone()));
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

/// Returns `Err` if deleting `id` would violate an invariant.
pub fn check_delete_mode(s: &Settings, id: &str) -> Result<(), String> {
    if s.modes.len() <= 1 {
        return Err("Cannot delete the last mode.".to_string());
    }
    if s.default_mode_id == id {
        return Err("Set a different default mode before deleting this one.".to_string());
    }
    Ok(())
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

/// Convenience for the many `load → mutate → save` setter commands.
pub fn update<F: FnOnce(&mut Settings)>(
    app: &tauri::AppHandle,
    f: F,
) -> Result<(), String> {
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
    use crate::mode::ModeLanguage;

    #[test]
    fn default_settings_have_expected_provider_and_groq_defaults() {
        let s = Settings::default();
        assert_eq!(s.transcription_provider, TranscriptionProvider::Deepgram);
        assert_eq!(s.groq.model, GroqModel::WhisperLargeV3Turbo);
        assert!(s.deepgram_api_key.is_none());
        assert!(s.groq_api_key.is_none());
    }

    #[test]
    fn default_settings_seed_four_predefined_modes() {
        let s = Settings::default();
        assert_eq!(s.modes.len(), 4);
        assert_eq!(s.default_mode_id, SEED_MODE_DEFAULT_EN);
        let ids: Vec<&str> = s.modes.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&SEED_MODE_DEFAULT_EN));
        assert!(ids.contains(&crate::mode::SEED_MODE_CLEANED_EN));
        assert!(ids.contains(&crate::mode::SEED_MODE_UKRAINIAN));
        assert!(ids.contains(&crate::mode::SEED_MODE_UA_EN));
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
    fn migration_creates_mode_from_deepgram_language() {
        let json = r#"{
            "transcription_provider": "deepgram",
            "deepgram": { "language": "fr" }
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.modes.is_empty());

        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.modes.len(), 4);
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.language, ModeLanguage::exact("fr"));
    }

    #[test]
    fn migration_creates_mode_from_groq_language_when_groq_is_active() {
        let json = r#"{
            "transcription_provider": "groq",
            "groq": { "model": "whisper_large_v3_turbo", "language": "uk" }
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.modes.len(), 4);
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.language, ModeLanguage::exact("uk"));
    }

    #[test]
    fn migration_defaults_language_to_en_when_none_present() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.language, ModeLanguage::exact("en"));
    }

    #[test]
    fn migration_reads_legacy_ai_cleanup_enabled_into_mode() {
        let json = r#"{"ai_cleanup_enabled": true}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(default.ai_cleanup.enabled);
        // The flat field must be gone from subsequent saves.
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(v.get("ai_cleanup_enabled").is_none());
    }

    #[test]
    fn migration_global_ai_cleanup_off_forces_every_mode_off() {
        let json = r#"{"ai_cleanup": {"enabled": false}}"#;
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
        let json = r#"{"ai_cleanup": {"enabled": true}}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let cleaned = s.modes.iter().find(|m| m.id == SEED_MODE_CLEANED_EN).unwrap();
        assert!(
            cleaned.ai_cleanup.enabled,
            "global-on must not stomp seed-mode defaults"
        );
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(
            !default.ai_cleanup.enabled,
            "global-on must not enable modes that were off by default"
        );
    }

    #[test]
    fn migration_renames_legacy_replacements_then_splits_to_corrections() {
        // "dot → ." is from != to, so it becomes a Correction entry in the default set.
        let json = r#"{"replacements": [{"from": "dot", "to": "."}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.legacy_replacements.as_ref().map(|v| v.len()), Some(1));
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.legacy_corrections.len(), 1);
        assert_eq!(s.legacy_corrections[0].from, "dot");
        assert_eq!(s.legacy_corrections[0].to, ".");
        // terms is drained into term_sets; nothing terms-side here.
        assert!(s.terms.is_empty());
        assert!(s.legacy_replacements.is_none());
        // Entries end up in the seeded correction set.
        assert_eq!(s.correction_sets.len(), 1);
        assert_eq!(s.correction_sets[0].entries.len(), 1);
        let reserialized = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
        assert!(v.get("replacements").is_none());
        assert!(v.get("dictionary").is_none(), "dictionary must not be written back");
        assert!(v.get("terms").is_none(), "terms is skip_serializing");
        assert!(v.get("corrections").is_none(), "legacy corrections must not be written back");
        assert!(v.get("correction_sets").is_some());
    }

    #[test]
    fn migration_splits_dictionary_from_eq_to_as_term() {
        // "MongoDB → MongoDB" (from == to) becomes a term entry in the Default Terms set.
        let json = r#"{"dictionary": [{"from": "MongoDB", "to": "MongoDB"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        // terms is drained into the Default Terms set.
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must be created");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        // No corrections — but the empty Default Corrections set is still seeded.
        assert!(s.legacy_corrections.is_empty());
        assert_eq!(s.correction_sets.len(), 1);
        assert!(s.correction_sets[0].entries.is_empty());
    }

    #[test]
    fn migration_splits_dictionary_from_ne_to_as_correction() {
        // "anthropik → Anthropic" becomes a Correction entry in the default set; no term set is created.
        let json = r#"{"dictionary": [{"from": "anthropik", "to": "Anthropic"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        assert!(s.terms.is_empty());
        assert!(s.term_sets.is_empty(), "no terms to seed a Default Terms set");
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
        // MongoDB ends up in the Default Terms set.
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must exist");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        // The two corrections end up in the seeded Default Corrections set.
        assert_eq!(s.legacy_corrections.len(), 2);
        assert_eq!(s.legacy_corrections[0].from, "dot");
        assert_eq!(s.legacy_corrections[1].from, "anthropik");
        assert_eq!(s.correction_sets[0].entries.len(), 2);
    }

    #[test]
    fn migration_dictionary_is_idempotent() {
        // After migration, re-running migrate() must not change anything.
        let json = r#"{"dictionary": [{"from": "MongoDB", "to": "MongoDB"}]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        // Second run: legacy_dictionary and terms are drained; term_sets and correction_sets already seeded.
        let changed2 = migrate(&mut s);
        assert!(!changed2, "second migrate must be a no-op");
        assert!(s.terms.is_empty());
        let default_set = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            .expect("Default Terms set must exist");
        assert_eq!(default_set.entries, vec!["MongoDB"]);
        assert_eq!(s.correction_sets.len(), 1);
    }

    #[test]
    fn migration_mode_use_dictionary_false_sets_corrections_false() {
        let json = r#"{"modes": [
            {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
             "translate":{"kind":"off"},"ai_cleanup":{"enabled":false,"prompt_override":null},
             "use_dictionary":false,"use_snippets":true}
        ], "default_mode_id": "mode-default-en"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        // Absorb the seed-mode migrations too.
        migrate(&mut s);
        let mode = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(!mode.use_corrections, "use_corrections must be false");
        // use_terms=false means term_set_ids is empty (no Default Terms set created — no legacy terms)
        assert!(mode.term_set_ids.is_empty(), "term_set_ids must be empty when use_dictionary was false");
    }

    #[test]
    fn migration_mode_use_dictionary_true_leaves_corrections_true() {
        let json = r#"{"modes": [
            {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
             "translate":{"kind":"off"},"ai_cleanup":{"enabled":false,"prompt_override":null},
             "use_dictionary":true,"use_snippets":true}
        ], "default_mode_id": "mode-default-en"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(mode.use_corrections, "use_corrections must remain true");
        // No legacy terms in this JSON so no Default Terms set is created
        assert!(mode.term_set_ids.is_empty(), "no legacy terms to migrate");
    }

    // ── Term sets migration tests ─────────────────────────────────────────

    #[test]
    fn migration_empty_legacy_terms_creates_no_term_set() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        assert!(s.term_sets.is_empty(), "no legacy terms → no Default Terms set");
        let default_mode = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(default_mode.term_set_ids.is_empty());
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
        // All seed modes default use_terms=true; with legacy terms present they should
        // all reference the Default Terms set.
        let json = r#"{"terms": ["MongoDB"]}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        for mode in &s.modes {
            assert!(
                mode.term_set_ids.contains(&SEED_TERM_SET_DEFAULT_ID.to_string()),
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
                 "translate":{"kind":"off"},"ai_cleanup":{"enabled":false,"prompt_override":null},
                 "use_terms":false,"use_corrections":true,"use_snippets":true,"term_set_ids":[]}
            ],
            "default_mode_id": "mode-default-en"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert!(
            !mode.term_set_ids.contains(&SEED_TERM_SET_DEFAULT_ID.to_string()),
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
        // Modes with term_set_ids already set and no legacy terms → idempotent.
        let json = r#"{
            "term_sets": [{"id":"ts-1","name":"My Set","entries":["Rust"]}],
            "modes": [
                {"id":"mode-default-en","name":"D","language":{"kind":"exact","code":"en"},
                 "translate":{"kind":"off"},"ai_cleanup":{"enabled":false,"prompt_override":null},
                 "use_corrections":true,"use_snippets":true,"term_set_ids":["ts-1"]}
            ],
            "default_mode_id": "mode-default-en"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        let mode = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(mode.term_set_ids, vec!["ts-1"], "term_set_ids must be unchanged");
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
    fn migration_is_idempotent_when_all_four_modes_already_present() {
        let mut s = Settings::default();
        assert_eq!(s.modes.len(), 4);
        // Running migration on already-seeded settings must be a no-op.
        migrate(&mut s);
        assert_eq!(s.modes.len(), 4, "second migration must not duplicate modes");
    }

    #[test]
    fn migrate_seeds_all_four_modes_when_modes_empty() {
        let json = r#"{}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.modes.is_empty());

        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.modes.len(), 4);

        let ids: Vec<&str> = s.modes.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&SEED_MODE_DEFAULT_EN));
        assert!(ids.contains(&crate::mode::SEED_MODE_CLEANED_EN));
        assert!(ids.contains(&crate::mode::SEED_MODE_UKRAINIAN));
        assert!(ids.contains(&crate::mode::SEED_MODE_UA_EN));
    }

    #[test]
    fn migrate_adds_missing_seeds_to_existing_mode_default_en() {
        let json = r#"{
            "modes": [{"id":"mode-default-en","name":"My Custom Name","language":{"kind":"exact","code":"en"},"translate":{"kind":"off"},"ai_cleanup":{"enabled":false,"prompt_override":null},"use_terms":true,"use_corrections":true,"use_snippets":true}],
            "default_mode_id": "mode-default-en"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.modes.len(), 1);

        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.modes.len(), 4);

        // User-edited name is preserved, not overwritten.
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.name, "My Custom Name");
    }

    #[test]
    fn migrate_is_fully_idempotent_with_all_four_seeds() {
        let mut s = Settings::default();
        assert_eq!(s.modes.len(), 4);

        let changed = migrate(&mut s);
        assert!(!changed, "migrate on fully-seeded settings must return false");
        assert_eq!(s.modes.len(), 4);
    }

    #[test]
    fn check_delete_mode_rejects_last_mode() {
        let s = Settings {
            modes: vec![Mode::seed_default_en(false)],
            default_mode_id: SEED_MODE_DEFAULT_EN.to_string(),
            ..Settings::default()
        };
        assert!(check_delete_mode(&s, SEED_MODE_DEFAULT_EN).is_err());
    }

    #[test]
    fn check_delete_mode_rejects_default_mode() {
        let s = Settings::default();
        assert!(check_delete_mode(&s, SEED_MODE_DEFAULT_EN).is_err());
    }

    #[test]
    fn check_delete_mode_allows_non_default_non_last_mode() {
        let s = Settings::default();
        assert!(check_delete_mode(&s, crate::mode::SEED_MODE_CLEANED_EN).is_ok());
    }

    #[test]
    fn get_default_mode_finds_mode_by_id() {
        let s = Settings::default();
        let mode = get_default_mode(&s);
        assert_eq!(mode.id, SEED_MODE_DEFAULT_EN);
    }

    #[test]
    fn get_default_mode_falls_back_to_first_when_id_missing() {
        let mut s = Settings::default();
        s.default_mode_id = "nonexistent".to_string();
        let mode = get_default_mode(&s);
        // Falls back to the first (and only) mode.
        assert_eq!(mode.id, SEED_MODE_DEFAULT_EN);
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
        // Language moved to the default mode; old option knobs silently ignored.
        let default = s.modes.iter().find(|m| m.id == SEED_MODE_DEFAULT_EN).unwrap();
        assert_eq!(default.language, ModeLanguage::exact("fr"));
        assert_eq!(s.deepgram_api_key.as_deref(), Some("dg-key"));
        // Serialized form must not contain deepgram.language.
        let json = serde_json::to_string(&s).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["deepgram"].get("language").is_none());
    }

    #[test]
    fn migration_converts_legacy_shortcut_to_hotkey_binding() {
        let json = r#"{"shortcut": {"key": "MetaRight", "modifiers": []}}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.hotkey_bindings.is_empty());
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(s.hotkey_bindings[0].shortcut.key, "MetaRight");
        assert_eq!(s.hotkey_bindings[0].mode_id, SEED_MODE_DEFAULT_EN);
        // Serialized form must not contain legacy "shortcut" key.
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
        let mut s = Settings::default();
        assert_eq!(s.hotkey_bindings.len(), 1);
        let changed = migrate(&mut s);
        assert!(!changed, "migrate on already-migrated settings must return false");
        assert_eq!(s.hotkey_bindings.len(), 1);
    }

    #[test]
    fn migration_drops_orphaned_bindings_for_deleted_modes() {
        let mut s = Settings::default();
        s.hotkey_bindings.push(HotkeyBinding {
            shortcut: Shortcut {
                key: "MetaRight".to_string(),
                modifiers: vec![],
                is_double_tap: false,
            },
            mode_id: "mode-nonexistent".to_string(),
        });
        assert_eq!(s.hotkey_bindings.len(), 2);
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(s.hotkey_bindings[0].mode_id, SEED_MODE_DEFAULT_EN);
    }

    #[test]
    fn default_settings_have_one_hotkey_binding_for_default_mode() {
        let s = Settings::default();
        assert_eq!(s.hotkey_bindings.len(), 1);
        assert_eq!(s.hotkey_bindings[0].shortcut.key, "AltRight");
        assert_eq!(s.hotkey_bindings[0].mode_id, SEED_MODE_DEFAULT_EN);
    }

    #[test]
    fn check_hotkey_conflicts_allows_distinct_shortcuts() {
        let bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "MetaRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-cleaned-en".to_string(),
            },
        ];
        assert!(check_hotkey_conflicts(&bindings).is_ok());
    }

    #[test]
    fn check_hotkey_conflicts_rejects_duplicate_shortcuts() {
        let bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-cleaned-en".to_string(),
            },
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
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: true },
                mode_id: "mode-cleaned-en".to_string(),
            },
        ];
        assert!(check_hotkey_conflicts(&bindings).is_ok());
    }

    #[test]
    fn check_one_binding_per_mode_allows_distinct_modes() {
        let bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "MetaRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-cleaned-en".to_string(),
            },
        ];
        assert!(check_one_binding_per_mode(&bindings).is_ok());
    }

    #[test]
    fn check_one_binding_per_mode_rejects_duplicate_mode() {
        let bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: true },
                mode_id: "mode-default-en".to_string(),
            },
        ];
        assert!(check_one_binding_per_mode(&bindings).is_err());
    }

    #[test]
    fn migration_collapses_duplicate_mode_bindings_to_first() {
        let mut s = Settings::default();
        s.hotkey_bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: false },
                mode_id: SEED_MODE_DEFAULT_EN.to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: true },
                mode_id: SEED_MODE_DEFAULT_EN.to_string(),
            },
        ];
        let changed = migrate(&mut s);
        assert!(changed);
        assert_eq!(s.hotkey_bindings.len(), 1);
        // First binding (the single-press variant) is preserved.
        assert!(!s.hotkey_bindings[0].shortcut.is_double_tap);
    }

    #[test]
    fn check_hotkey_conflicts_rejects_two_double_tap_same_key() {
        let bindings = vec![
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: true },
                mode_id: "mode-default-en".to_string(),
            },
            HotkeyBinding {
                shortcut: Shortcut { key: "AltRight".to_string(), modifiers: vec![], is_double_tap: true },
                mode_id: "mode-cleaned-en".to_string(),
            },
        ];
        assert!(check_hotkey_conflicts(&bindings).is_err());
    }

    #[test]
    fn migration_stamps_groq_provider_model_onto_all_seeded_modes() {
        let json = r#"{"transcription_provider": "groq", "groq": {"model": "whisper_large_v3"}}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        let expected = ProviderModel::Groq { model: GroqModel::WhisperLargeV3 };
        for mode in &s.modes {
            assert_eq!(
                mode.provider_model, expected,
                "mode {} should have Groq provider_model",
                mode.id
            );
        }
    }

    #[test]
    fn migration_stamps_assemblyai_provider_model_onto_all_seeded_modes() {
        let json = r#"{"transcription_provider": "assembly_ai", "assemblyai": {"model": "whisper_streaming"}}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        let changed = migrate(&mut s);
        assert!(changed);
        let expected = ProviderModel::AssemblyAi { model: AssemblyAiModel::WhisperStreaming };
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
        let json = r#"{"transcription_provider": "groq"}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        migrate(&mut s);
        // Customise one mode to AssemblyAI after first migration.
        s.modes[0].provider_model = ProviderModel::AssemblyAi { model: AssemblyAiModel::default() };
        // A second migrate (e.g. transcription_provider still Groq after reload)
        // must not overwrite the already-customised mode.
        migrate(&mut s);
        assert_eq!(
            s.modes[0].provider_model,
            ProviderModel::AssemblyAi { model: AssemblyAiModel::default() },
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
        assert_eq!(LocalWhisperIdleTimeout::default(), LocalWhisperIdleTimeout::FifteenMinutes);
        assert_eq!(LocalWhisperSettings::default().idle_timeout, LocalWhisperIdleTimeout::FifteenMinutes);
    }

    #[test]
    fn local_whisper_idle_timeout_as_duration_returns_correct_values() {
        use std::time::Duration;
        assert_eq!(LocalWhisperIdleTimeout::FiveMinutes.as_duration(), Some(Duration::from_secs(300)));
        assert_eq!(LocalWhisperIdleTimeout::FifteenMinutes.as_duration(), Some(Duration::from_secs(900)));
        assert_eq!(LocalWhisperIdleTimeout::ThirtyMinutes.as_duration(), Some(Duration::from_secs(1800)));
        assert_eq!(LocalWhisperIdleTimeout::OneHour.as_duration(), Some(Duration::from_secs(3600)));
        assert_eq!(LocalWhisperIdleTimeout::Never.as_duration(), None);
    }

    #[test]
    fn local_whisper_settings_round_trips_through_json() {
        let settings = LocalWhisperSettings { idle_timeout: LocalWhisperIdleTimeout::ThirtyMinutes };
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
        assert_eq!(s.local_whisper.idle_timeout, LocalWhisperIdleTimeout::FifteenMinutes);
    }

    #[test]
    fn settings_preserves_local_whisper_idle_timeout_through_json() {
        let mut s = Settings::default();
        s.local_whisper.idle_timeout = LocalWhisperIdleTimeout::OneHour;
        let json = serde_json::to_string(&s).unwrap();
        let decoded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.local_whisper.idle_timeout, LocalWhisperIdleTimeout::OneHour);
    }

    #[test]
    fn settings_without_local_whisper_field_defaults_to_fifteen_minutes() {
        let json = r#"{}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.local_whisper.idle_timeout, LocalWhisperIdleTimeout::FifteenMinutes);
    }
}
