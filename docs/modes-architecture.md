# Modes architecture

This document defines the target shape of settings, the transcription pipeline, and the migration story for the Modes rework. It is the canonical reference for issues #30-#39. If anything in this doc contradicts an individual issue, the issue wins for *its* slice; this doc is the integration contract across slices.

## Why this rework exists

Today, language is duplicated across `DeepgramSettings` and `GroqSettings`. AI cleanup is a single global toggle. Switching provider loses your language choice and gives no answer for "use Ukrainian here, English there." The four scenarios the user actually wants —

1. Regular English transcription.
2. English transcription with AI cleanup.
3. Ukrainian dictation, automatically translated to English.
4. Pure Ukrainian dictation.

— are each a combination of (`spoken language`, `translate target`, `ai cleanup`), but the current shape forces those combinations to be implicit in flat toggles spread across two screens. The rework promotes that combination to a first-class object called a `Mode`.

## Domain glossary

- **Mode** — a named bundle of recording behavior: language, translate target, cleanup, dictionary/snippets opt-ins. Replaces the implicit "current settings combination." A user can have many; one is the default. Each mode can bind one or more hotkeys.
- **Engine** — the global transcription provider configuration: which provider (Deepgram or Groq), which model, API key. Engine is *model-agnostic from the Mode's point of view*: Modes carry no provider-specific fields.
- **Dictionary** — global list of `from → to` substitutions. Used twice: as a prompt hint to the engine (so the engine recognizes the term in the first place) and as a final post-substitution (so manual corrections always win).
- **Snippets** — global list of `trigger → expansion` substitutions, with placeholders like `{{DATE}}`, `{{TIME}}`, `{{CLIPBOARD}}`. Applied during post-processing.
- **Hotkey binding** — a pair `(shortcut, mode_id)`. A Mode can have multiple bindings (future-proofs tap/double-tap variants).

Important naming: we use **Mode**, not Workflow. "Workflow" is reserved for a future higher-level concept (per-app/per-URL rules that pick a Mode). Mode is what the recording *does*; Workflow would be a rule that picks a Mode. The existing "Workflows" sidebar entry is a stub and gets renamed to "Modes" in slice #30.

## Target schema

Shapes are shown in Rust; TypeScript types mirror them exactly.

```rust
// ─── Mode ────────────────────────────────────────────────────────────

struct Mode {
    id: ModeId,                    // stable uuid; seeded modes use constant ids
    name: String,
    icon: Option<String>,          // optional icon slug; ignored if absent
    language: ModeLanguage,
    translate: TranslateTarget,
    ai_cleanup: ModeCleanup,
    use_dictionary: bool,          // default true
    use_snippets:   bool,          // default true
}

enum ModeLanguage {
    Auto,                          // no language hint at all
    Exact(String),                 // single ISO code, e.g. "en"
    Hints(Vec<String>),            // 2+ codes; provider auto-detects with the list as UI state
}

enum TranslateTarget {
    Off,
    Apple { target: String },      // ISO code of target language
    // Llm { target: String },     // reserved for a future slice
}

struct ModeCleanup {
    enabled: bool,
    prompt_override: Option<String>,  // None = use cleanup::DEFAULT_SYSTEM_PROMPT
}

// ─── Top-level Settings ──────────────────────────────────────────────

struct Settings {
    engine: EngineConfig,           // provider, per-provider model, etc.
    deepgram_api_key: Option<String>,
    groq_api_key: Option<String>,

    modes: Vec<Mode>,
    default_mode_id: ModeId,

    dictionary: Vec<DictionaryEntry>,    // renamed from `replacements`
    snippets:   Vec<Snippet>,            // new

    ai_cleanup_config: AiCleanupConfig,  // auth, model, thresholds, default prompt

    hotkey_bindings: Vec<HotkeyBinding>, // replaces single `recording_shortcut`

    // Unchanged: audio, indicator, appearance, history retention, etc.
}

struct EngineConfig {
    provider: TranscriptionProvider,    // Deepgram | Groq
    deepgram: DeepgramEngineSettings,   // { model: "nova-3" }  — after slice #32
    groq:     GroqEngineSettings,       // { model: WhisperLargeV3 | WhisperLargeV3Turbo }
}

struct DictionaryEntry { from: String, to: String }
struct Snippet         { id: SnippetId, trigger: String, expansion: String }
struct HotkeyBinding   { shortcut: Shortcut, mode_id: ModeId }
```

### What got removed

- `DeepgramSettings.language` — moved to `Mode.language`.
- `DeepgramSettings.smart_format` / `dictation` / `numerals` / `keyterms` — dropped entirely; hardcoded sensible defaults in `deepgram_session.rs`.
- `GroqSettings.language` — moved to `Mode.language`.
- `Settings.recording_shortcut` — replaced by `Settings.hotkey_bindings`.
- The flat global `ai_cleanup_enabled` toggle — moved to `Mode.ai_cleanup.enabled`.

## Pipeline

The full pipeline after all 10 slices land:

```
audio capture
  │
  ▼
provider.transcribe(audio, mode.language, dictionary)
  │                                          ▲
  │                                          │ dictionary "from" terms passed
  │                                          │ as Deepgram keyterm[] (URL budget)
  │                                          │ or Groq prompt (char budget)
  ▼
if mode.translate != Off:    translate(text, mode.translate)
  │
  ▼
if mode.ai_cleanup.enabled:  cleanup(text, prompt_override ?? default_prompt)
  │
  ▼
if mode.use_snippets:        expand_snippets(text)
  │
  ▼
if mode.use_dictionary:      apply_dictionary(text)
  │
  ▼
paste(text + " ")
```

### Why the order is this way

The post-processing order is load-bearing. It mirrors TypeWhisper's priority-ordered pipeline (`PostProcessingPipeline.swift`).

- **Translate runs first** (before cleanup). The default cleanup system prompt is English-only (it has English-specific rules: contraction preservation, camelCase identifiers, etc.). Translating Ukrainian → English first lets cleanup operate on its expected input.
- **Cleanup runs before snippets** so the LLM cannot rewrite a literal `[date]` trigger into "the date" or similar before snippets get a chance to expand it.
- **Cleanup runs before dictionary** so the user's manual corrections (e.g., `Mongo → MongoDB`) are the final word and survive cleanup's "do not expand brand names" rule. Today's order is the reverse (dictionary first, then cleanup) — slice #31 flips it.
- **Dictionary runs after snippets** so any text a snippet expanded into also receives dictionary corrections.

### Dictionary's dual role

Dictionary entries are used twice:

1. As a **prompt hint** to the provider before transcription, biasing the engine toward recognizing those terms in the first place. (Deepgram: `keyterm[]` query params. Groq: a `prompt` multipart field built from the term list.)
2. As a **post-substitution** at the end of the pipeline, guaranteeing that even if the engine misrecognized, the corrected term lands in the output.

Punctuation-cue entries (e.g., the default `"dot" → "."`) are filtered out of the prompt-hint pass; sending them as engine context biases on noise. Filter rule: skip entries whose `to` is < 3 chars and consists entirely of punctuation.

## Provider mapping

How Mode fields map to each provider's request:

| Mode field | Deepgram | Groq |
|---|---|---|
| `language: Auto` | omit `language` param | omit `language` form field |
| `language: Exact("xx")` | `language=xx` | `language=xx` |
| `language: Hints([...])` | `language=multi` (codes informational) | omit `language` (Whisper auto-detects) |
| `translate` | n/a — handled by pipeline | n/a — handled by pipeline |
| dictionary terms (filtered) | `keyterm=<from>` per entry; URL budget ~4096 bytes | `prompt=Vocabulary: t1, t2, ...`; char budget ~800 |
| smart_format / numerals | hardcoded `smart_format=true&numerals=true` | not applicable (Whisper handles natively) |
| dictation | not sent | not applicable |

Budget constants live in `dictionary.rs` (or wherever the helper sits):

```rust
const DEEPGRAM_KEYTERM_BUDGET_BYTES: usize = 4096;
const GROQ_PROMPT_BUDGET_CHARS: usize = 800;
```

## Seed modes

These four modes are seeded on first launch and on upgrade (idempotent — match by id constant, not name). User-edited copies are not overwritten; missing seeds are recreated.

| Constant id | Name | language | translate | ai_cleanup |
|---|---|---|---|---|
| `mode-default-en` | Default English | `Exact("en")` | `Off` | `{ enabled: false }` |
| `mode-cleaned-en` | Cleaned English | `Exact("en")` | `Off` | `{ enabled: true }` |
| `mode-ukrainian` | Ukrainian | `Exact("uk")` | `Off` | `{ enabled: false }` |
| `mode-ua-en` | UA → EN | `Exact("uk")` | `Apple { target: "en" }` *(disabled in UI until slice #37 lands; field present, picker reads "coming soon")* | `{ enabled: true }` |

`default_mode_id = mode-default-en` after a fresh install. On upgrade, do not override the user's existing default; seed the others alongside whatever default they already have.

## Migration

Triggered on first launch after each upgrade. Idempotent on subsequent launches.

1. **Read** the legacy `settings.json`. Fields of interest: `replacements`, `recording_shortcut`, `ai_cleanup_enabled`, `deepgram` (with its `language`/`smart_format`/etc.), `groq` (with its `language`/`model`).
2. **Rename** `replacements` → `dictionary` (preserves the existing default punctuation seed entries).
3. **Create** the seeded modes if they don't exist. For users coming from before slice #30, none will exist yet, so all four are created. For users coming from after slice #30 (which seeded only `mode-default-en`), the remaining three are created.
4. **`default_mode_id`** is set to `mode-default-en` on fresh install. On upgrade, if a legacy mode named "Default" exists from slice #30's migration, keep it as default; otherwise pick `mode-default-en`.
5. **Convert** the legacy `recording_shortcut` (if present) into a single `HotkeyBinding { shortcut, mode_id: default_mode_id }`. Strip the legacy field.
6. **Drop** the legacy `deepgram.language`, `deepgram.smart_format`, `deepgram.dictation`, `deepgram.numerals`, `deepgram.keyterms`, `groq.language`, and global `ai_cleanup_enabled` fields. Use serde's `#[serde(default)]` + permissive unknown-field handling to read old shapes safely.
7. **Persist** the new shape on next save.

Migration must be idempotent: launching the app a second time on a migrated `settings.json` must not duplicate modes, hotkey bindings, or dictionary entries.

## Slice → file ownership

A rough guide to which files each slice touches most heavily. Use this to spot merge conflicts early.

| Slice | Primary files |
|---|---|
| #30 Modes foundation | `config.rs`, `mode.rs` (new), `ptt.rs`, `lib/types.ts`, `pages/Modes*.tsx`, sidebar |
| #31 Dictionary rename + order flip | `replacements.rs` → `dictionary.rs`, `ptt.rs`, `lib/types.ts`, `pages/Replacements*` → `pages/Dictionary*` |
| #32 Drop Deepgram knobs | `config.rs`, `deepgram_session.rs`, `components/TranscriptionField.tsx`, `lib/types.ts` |
| #33 Mode CRUD + seed | `pages/Modes*.tsx`, `mode.rs`, `config.rs` (seed helper) |
| #34 Dictionary as engine hint | `dictionary.rs`, `deepgram_session.rs`, `groq_session.rs` |
| #35 Snippets | `snippets.rs` (new), `ptt.rs`, `pages/Snippets*.tsx`, sidebar |
| #36 Multi-language picker | `mode.rs` (enum), `deepgram_session.rs`, `groq_session.rs`, mode editor UI |
| #37 Translation (HITL) | `translation.rs` (new) + Swift bridge, `ptt.rs`, mode editor UI |
| #38 Hotkey-per-mode | `hotkeys.rs`, `config.rs`, `pages/Hotkeys*.tsx`, mode editor UI |
| #39 Per-mode prompt override | `cleanup.rs`, `ptt.rs`, mode editor UI |

## Explicitly out of scope

These ideas appeared during design but are intentionally deferred:

- **Per-app/per-URL Workflows** (TypeWhisper-style rules that auto-pick a Mode based on active app or browser URL). The Mode abstraction reserves space for this above it; do not add it inside Mode.
- **Plugin system / multiple engines simultaneously loaded.** One provider is active at a time.
- **Auto-learned dictionary entries** (TypeWhisper learns from manual edits).
- **Snippet format strings** (`{{DATE:yyyy-MM-dd}}`). v1 only supports bare placeholders.
- **Translate via LLM** as an alternative to Apple Translate. Schema reserves `TranslateTarget::Llm { target }` but no implementation yet.
- **Per-mode provider override.** Provider is global in v1.
- **Output format / app formatter** (TypeWhisper-style per-app formatting rules).

If a slice's implementation surfaces a clean place to add one of these later without changing the Mode/Settings shape, that's fine. If it would require changing the shape, defer it explicitly in the PR description.

## Reference: source pipeline being modeled on

Where useful: TypeWhisper macOS source at `/Users/maksym/Developer/typewhisper-mac` contains a mature implementation of this pattern. Key files an implementer can consult for inspiration (not for direct porting):

- `TypeWhisper/Services/PostProcessingPipeline.swift` — priority-ordered post-processing.
- `TypeWhisper/ViewModels/SettingsViewModel.swift` — the `LanguageSelection` enum (Auto / Exact / Hints / inheritGlobal) we're modeling `ModeLanguage` on.
- `TypeWhisper/Models/Workflow.swift` — the broader Workflow concept we're explicitly *not* adopting (reserved for future).
