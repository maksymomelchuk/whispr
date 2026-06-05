# Handoff — Benchmarking feature: planning & implementation

**Repo:** `/Users/maksym/Developer/wispr-tauri` (branch `main`)
**Date:** 2026-06-05
**Focus for next session:** Finish designing, then implement, a user-facing **Profile Comparison Playground** — born out of a transcription-engine benchmark that's already built and working.

---

## 1. Where we are in two sentences

A working offline benchmark harness exists and has produced a valid cross-engine accuracy/latency/cost report. We are now mid-way through a `/grill-with-docs` design interview to turn that capability into a shipped feature where users dictate once and compare transcription **Profiles** side by side.

---

## 2. What's already done (do NOT redo)

### The benchmark harness — complete, compiles clean, tests pass
Lives behind a Cargo feature so it never ships in the app. Reference the code, don't re-read it into your head unless editing:
- `src-tauri/src/bench/` — `score.rs` (WER/CER + normalization, 6 passing unit tests), `wav.rs`, `clips.rs`, `engines.rs`, `report.rs`, `run.rs`, `mod.rs`
- `src-tauri/src/bench_main.rs` — bin entrypoint (`#[tokio::main(flavor = "current_thread")]`)
- `src-tauri/Cargo.toml` — `[features] bench = []`, `[[bin]] name="bench" required-features=["bench"]`, `default-run = "whispr"`
- `src-tauri/src/lib.rs` — `#[cfg(feature = "bench")] pub mod bench;`
- `benchmark/passages.md` (6 ground-truth passages A–F), `benchmark/README.md` (run guide), `benchmark/results.md` (latest valid results)

Run command (requires API keys the agent cannot access — see constraints):
```sh
cd src-tauri
cargo run --features bench --bin bench -- ~/Downloads/benchmark_recordings > ../benchmark/results.md
```

**Key methodology fix already applied:** streaming engines (Deepgram, AssemblyAI) are now fed at **real time** via a concurrent feeder (`engines.rs::drive`, `is_streaming()` in `clips.rs`). An earlier run fed all audio in one burst and truncated streaming transcripts → bogus 87–100% WER. The current `benchmark/results.md` is the valid re-run.

### The analysis — already delivered to the user
Full per-engine analysis is in the conversation history; the raw data is in `benchmark/results.md`. Headlines (so you don't re-derive):
- All 4 seed Profiles default to **Deepgram** (`src-tauri/src/mode.rs`).
- Deepgram is fast + cheap-ish and fine on clean single-language speech, but **worst on technical jargon** (23% WER) and **fails completely on code-switched/mixed UK+EN with Auto language** (100%+ WER). AssemblyAI also fails mixed and **has no Ukrainian support at all**.
- **gpt-4o-transcribe** is the best all-rounder (perfect on mixed, cleanest numbers, supports `uk`). **ElevenLabs scribe_v2** is best on technical English but slowest. **Groq v3-turbo** is ~10× cheapest.

---

## 3. What we're doing now — the feature design interview

The user wants to productize the benchmark as a **Profile Comparison Playground**: select existing Profiles and/or build draft Profiles, dictate **one** utterance, and compare each Profile's output side by side. Running via the `/grill-with-docs` skill (one question at a time, recommended answer each, explore code before asking, update `CONTEXT.md` inline, offer ADRs sparingly).

### Domain naming — RESOLVED by code exploration (not a question)
- User-facing term is **Profile** (UI: sidebar "Profiles", "New Profile", "Add profile" in `src/pages/ModesPage.tsx`, `src/components/AppShell.tsx`).
- Code internals still say **Mode** (`src-tauri/src/mode.rs`, `ModesPage.tsx`, `/modes` route); `docs/modes-architecture.md` predates the rename.
- **Use "Profile" in all user-facing design; treat "Mode" as the legacy internal alias.** This resolution is not yet written into `CONTEXT.md` — do it when the feature/terms are named.

### A Profile's axes (confirmed against `src-tauri/src/mode.rs`)
Each Profile owns, and can differ on: speech model (`provider_model: ProviderModel`), `language` (`ModeLanguage`: Auto/Exact/Hints), `ai_cleanup` (enabled + provider + model + `prompt_override`), `term_set_ids`, `correction_set_ids`, `use_snippets`. The pipeline order (see `docs/modes-architecture.md` + `CONTEXT.md`): STT (terms as recognition hints) → AI cleanup → snippet expansion → corrections → paste.

### Decisions resolved in the interview
- **Q1 — Capture model: RESOLVED.** Single dictation captured once and fanned out **live** to all selected Profiles simultaneously (streaming engines transcribe live; batch engines fire on PTT release; each Profile then runs its own full pipeline). Reuses the bench fan-out pattern. Rationale: only same-utterance comparison is honest.

### Open question on the table right now
- **Q2 — What a Draft Profile IS (asked, awaiting answer).** Recommended: a Profile-shaped config living only in the playground (not in saved Profiles); primary path = "duplicate an existing Profile → tweak", blank also allowed; exposes the full Profile editor (all axes); **references** existing term/correction sets by ID but does **not** author new sets inline. Two sub-questions posed to the user: (1) full editor vs a reduced "swap speech model + AI model only" editor; (2) confirm drafts only reference existing sets, no inline set creation.

### Remaining design-tree nodes still to grill (suggested order)
1. **Draft Profile lifecycle & promotion** — ephemeral vs persisted across navigation/restart; promotion (draft → real Profile: assign id, name, append to `modes`, persist, hotkey?); does a promoted column become the real Profile or stay a draft.
2. **Selection / comparison model** — how many columns max (cost cap), mixing real Profiles + multiple drafts in one comparison.
3. **What's displayed & how quality is judged** — user confirmed *qualitative eyeball* eval: show transcript + processing time, no auto-WER (no ground truth). Decide: show raw STT transcript vs final post-pipeline output? per-stage timing (STT vs cleanup)? estimated cost per column?
4. **AI Provider gating** — user said "need at least one AI provider connected." Clarify: a provider is required only for columns whose Profile enables cleanup; behavior when the chosen provider has no saved key (block/warn/skip cleanup). (See `CONTEXT.md` "AI Provider": one global credential, chosen per-Profile.)
5. **UI placement & feature name** — new surface vs settings section; needs a canonical name (e.g. "Playground" / "Compare").
6. **Guardrails** — one dictation = N STT calls + up to N cleanup LLM calls at once; cost warning + column cap; possible STT de-dup when columns share speech model + language + term sets.

### Architecture reuse note (already identified)
The bench's direct `Engine::run` drive + fan-out (`src-tauri/src/bench/engines.rs`) is the playground backend. Plan to refactor it into a shared "fan one audio source to many engines" core so the offline bench and live playground can't drift.

---

## 4. Hard constraints (carry forward — these bit us already)
- **`.env` and `**/.env*` are protected** by a deny rule. Do NOT read, grep, cat, or source them. The agent cannot run the benchmark itself; the user runs it with their own keys. Redact any keys if seen.
- **`cat` is aliased to `bat` (not installed)** in the user's shell. Never use `cat`/heredocs in Bash; for `gh` use `--body-file`/`-F`. Use the Read tool to read files.
- **`AGENTS.md` is strict and enforced in review** — no WHAT comments, no commented-out code, functions ≤40 lines, files ≤400 lines, no single-caller abstractions, no magic numbers, return early. Match these or review auto-rejects.

---

## 5. Suggested skills for the next session
- **`grill-with-docs`** (currently in flight) — resume the interview at Q2's answer and continue down the design tree above. Keep updating `CONTEXT.md` inline; the **Profile/Mode** alias resolution and new terms (**Draft Profile**, the feature name) are pending entries.
- **`to-prd`** — once the design tree is resolved, turn it into a PRD in the repo's established style (cf. `docs/prd-multi-platform-port.md`, `docs/modes-architecture.md`).
- **`to-issues`** — break the PRD into tracer-bullet vertical-slice issues (the repo already works this way; cf. the #30–#39 Modes slices).
- **`staff-engineer`** — senior review of the implementation plan before coding.
- During implementation: **`emil-design-engineering`** / **`impeccable`** for the comparison UI (multi-column side-by-side, live transcript streaming, latency/cost chips), and **`pr`** / **`code-review`** / **`coderabbit:code-review`** for landing changes.

---

## 6. First moves for the next agent
1. Read this doc, `CONTEXT.md`, and `docs/modes-architecture.md`.
2. Resume `grill-with-docs`: the user is answering **Q2** (Draft Profile definition). Process their answer, record resolved terms in `CONTEXT.md`, then proceed to "Draft Profile lifecycle & promotion".
3. Do not re-run or re-analyze the benchmark — that work is done and valid in `benchmark/results.md`.
