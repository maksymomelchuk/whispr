# Audio processing pipeline

This document traces a single dictation from hotkey-down to paste, in execution order, and pins down **when each user-facing transform applies** — Vocabulary (Terms), Tone, AI cleanup, Snippets, and Corrections. It is a reference map of the real code; file/line anchors point at the orchestration so the doc stays verifiable. If a line moves, trust the function name over the number.

The orchestrator is `run_session` in `src-tauri/src/ptt.rs`; the post-cleanup tail is `run_stages` in `src-tauri/src/pipeline.rs`; the cleanup prompt is assembled by `effective_prompt` in `src-tauri/src/cleanup.rs`.

## Phase 0 — Hotkey down (capture kickoff)

The instant the dictation key is pressed, before audio matters:

- **App identity** is captured (`target_app::capture`) — frontmost app's bundle id, name, and icon (osascript on macOS). Always on, requires no extra permission. This is what the Tone page's app list and per-app term/glossary scoring key off.
- **Context channels** are kicked off on background workers, each only if the active Profile opts in: `clipboard_context::capture`, `selected_text_context::capture`, `focused_field_context::capture`. They run concurrently so results are ready by the time cleanup needs them.
- The **recorder starts** streaming audio; system audio is muted if `pause_media_on_record`.

## Phase 1 — Recording

`recorder.start(...)` streams audio chunks while the key is held. On release the recorder stops and the buffer is finalized. Format negotiation happens up front; failure aborts with `pipeline::recorder_failed_error`.

## Phase 2 — Pre-transcription setup

`run_session`, `ptt.rs:318`–`372`.

1. Load settings; resolve the **active Profile/Mode** (carries language, cleanup config, context-channel toggles, prompt override).
2. Resolve the session app (await pending app-identity, 500 ms timeout) → `session_bundle_id`. Everything downstream keys off this one snapshot, so a window switch mid-processing cannot change which app applies.
3. **① Vocabulary / Terms selection** — `selector::select_terms(...)`. Merges the Profile's Term sets **+ learned Terms** (Auto-Learn), scores by per-app frequency / recency / manual boost for `session_bundle_id`, fills the **top-K (~40) budget** → `session_terms`.
4. Verify the provider's API key (else abort with a setup notice).

## Phase 3 — Transcription (STT) — ① Vocabulary applies here

Provider dispatch, `ptt.rs:~404`–`571`. The chosen engine (Deepgram / Groq / AssemblyAI / OpenAI / ElevenLabs / Local Whisper) is configured with `terms: session_terms` and the Profile language.

**Vocabulary applies as recognition biasing on the input side** (provider-specific: Deepgram keyterms, Whisper-family initial prompt). The engine returns **`raw_text`**.

## Phase 4 — Post-transcription, pre-cleanup

`ptt.rs:600`–`650`.

1. **Await context captures** (clipboard / selected / focused field) concurrently under one deadline → `cleanup::ContextBlocks` (plus `system_date`, `system_user`).
2. **Glossary selection** — `selector::select_glossary_words(...)`. Merges Term sets **+ Correction sets + learned entries**, scores per-app, fills the **top-N (~200) budget**. This is a _spell-exactly_ list for the cleanup prompt — distinct from the Term hints in Phase 3.

## Phase 5 — AI Cleanup — ② Tone applies here

`maybe_cleanup`, `ptt.rs:740`; prompt assembly `cleanup::effective_prompt`, `cleanup.rs:310`.

**Gating** (`ptt.rs:755`–`781`) — cleanup is skipped and `raw_text` passes through unchanged when:

- the Profile has cleanup disabled → `Disabled`
- word count < `min_words` → `SkippedBelowMinWords`
- speak duration < `min_duration_ms` → `SkippedBelowMinDuration`

If it runs:

1. **② Tone resolution** (`ptt.rs:783`): only if `tone_overlay_enabled`, via `tone::resolve_tone(bundle_id, overrides)` — per-app override → taxonomy category → preset directive (`None` for Neutral).
2. **Prompt assembly** (`effective_prompt`), in this exact order:

   ```
   SAFETY_PREAMBLE
   + cleanup rules            (DEFAULT_SYSTEM_PROMPT, or the Profile's prompt override)
   + tone directive           ← ② Tone
   + glossary block           (the ~200-word spell-exactly list from Phase 4)
   + CONTEXT_HARDENING_RULES   (only when a context block is present)
   + context blocks            (system info → selected text → focused field → clipboard)
   ```

3. **③ Cleanup invoke** (`cleanup_invoke::invoke`) sends `raw_text` + the assembled system prompt to the provider — Anthropic-native or OpenAI-compatible, same prompt either way (ADR 0003). Returns cleaned text → **`replaced_text`**, status `Ran` (or a `Failed*` variant).

## Phase 6 — Post-cleanup transforms — ④ Snippets, then ⑤ Corrections

`pipeline::run_stages`, `pipeline.rs:93`. Starting from `replaced_text` (cleaned, or raw if cleanup was skipped):

1. **④ Snippets** (`pipeline.rs:108`): if `mode.use_snippets`, `expand_snippets(...)` — text-expansion of triggers / placeholders like `{{DATE}}`.
2. **⑤ Corrections** (`pipeline.rs:111`–`118`): if the Profile has Correction sets **or** there are learned entries, `compose_corrections(...)` merges the Profile's Correction sets **+ learned Corrections**, then `apply_corrections(...)` runs a **deterministic find-and-replace**. This is **after** cleanup and **after** snippets, so it is the last word — and it works even when AI cleanup is off.
3. `final_text` is sealed; `pasted_text = final_text + " "`. A `HistoryEntry` records `raw_text`, `replaced_text`, `final_text`, status, profile snapshot, app, and the context-channel **flags only** (never the captured content).

## Phase 7 — Paste & after

`ptt.rs:689`–`725`.

1. **Paste policy** (`resolve_paste_policy`): if cleanup failed and `paste_raw_on_failure` is off → write `raw_text` to the clipboard and suppress the paste; otherwise paste `pasted_text`.
2. `paste::paste_text(...)` (or `write_to_clipboard`).
3. `stats::record(...)` — words, duration, app (the 365-day aggregate behind the Tone page's app list).
4. Media resumes.
5. **Post-paste observation**: if enabled and the paste went out, `post_paste_observer::start(...)` watches the field for the user typing over the result → feeds the miner → new learned entries.

## The five transforms, in order

| #   | Transform          | Where it applies                 | Side                        | Source                                     |
| --- | ------------------ | -------------------------------- | --------------------------- | ------------------------------------------ |
| ①   | Vocabulary / Terms | During STT, as recognition hints | Input (biases what's heard) | `selector::select_terms` → engine `terms:` |
| ②   | Tone               | Inside the cleanup prompt        | Cleanup prompt              | `tone::resolve_tone` → `effective_prompt`  |
| ③   | AI Cleanup         | The cleanup model call           | Output rewrite              | `cleanup_invoke::invoke`                   |
| ④   | Snippets           | After cleanup                    | Output                      | `expand_snippets`                          |
| ⑤   | Corrections        | After snippets (last)            | Output (deterministic)      | `apply_corrections`                        |

## Two invariants worth internalizing

- **Terms and Corrections are opposite ends of the pipeline.** A _Term_ biases the recognizer _before_ any text exists (input side); a _Correction_ is a deterministic replace _after_ everything else (output side). A Correction's `from` is the wrong word by definition and must never be fed to the engine as a hint.
- **Auto-Learn feeds three places at once.** A promoted learned entry joins (1) the Term hints for the engine, (2) the cleanup glossary, and (3) the deterministic Corrections. The post-paste and History-edit miners create those learned entries, closing the loop.
