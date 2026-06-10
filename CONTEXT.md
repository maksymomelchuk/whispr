# Domain glossary

Terms meaningful to anyone reasoning about the transcription pipeline. Implementation details live in the code; this file describes _what_ the concepts are, not _how_ they are stored.

## Term

A word or phrase the user wants the speech recognizer to know exists. Terms are injected into the STT engine as recognition hints (Deepgram `keyterm`, Groq Whisper `prompt`) _before_ audio is transcribed. They have no replacement — their only job is to bias the recognizer toward producing the right word in the first place.

Example: `Anthropic`, `Tauri`, `kubectl`.

Not a Term: anything with a `from → to` shape. That's a [[Correction]].

## Correction

A post-STT find-and-replace rule. After the recognizer produces text, every Correction is applied: `from` text is replaced by `to` text. Corrections fix things the recognizer got wrong (or things the user _wants_ phrased differently, e.g. verbal punctuation cues like `dot → .`).

Corrections do **not** bias the recognizer — their `from` is by definition the _wrong_ word and biasing toward it would make transcription worse.

## Snippet

A user-defined shorthand the user _deliberately_ uses in speech. Triggers are matched in the post-STT text and expanded into longer content, optionally with placeholders (`{{DATE}}`, `{{TIME}}`, `{{CLIPBOARD}}`).

Distinction from [[Correction]]: a Correction patches over a recognizer mistake (involuntary); a Snippet expands a chosen shortcut (voluntary). A Correction's replacement is always static text; a Snippet's expansion can contain placeholders resolved at injection time.

## Set

A named, reusable collection of [[Term]]s or [[Correction]]s. Modes reference Sets by id; a Mode can apply any combination, and one Set can be shared across Modes. Deleting a Set unlinks it from every Mode that references it — the **delete cascade** — atomically with the deletion itself. The backend owns the cascade; UI state only reflects what the backend returns, never re-derives it.

Two kinds exist: Term Sets and Correction Sets. A [[Snippet]] is not a Set — Snippets are individual entries, global rather than assigned to Modes, and have no cascade.

## Engine

Provider-specific plumbing that turns audio into text — Deepgram's streaming WebSocket, Groq's polling REST. An Engine sees raw audio chunks and emits raw text-so-far updates plus a final raw transcript. It knows its protocol and nothing else: not [[Correction]]s, not the UI overlay, not how previews are throttled. Swapping providers means writing a new Engine; nothing else in the pipeline needs to change.

## Session

One push-to-talk dictation, from PTT-down to paste — or to [[Cancelled Session]], the alternate terminal state. The Session owns an [[Engine]] and everything around it: computing audio levels for the overlay, applying [[Correction]]s to the raw partials the Engine emits, throttling the preview event stream, and translating soft engine failures (e.g. Groq's final-POST fallback) into user-visible flashes. Engines are pluggable; the Session is the same regardless of which Engine is in use.

The Session's output is the raw transcript plus the speak duration. The post-transcript stages (AI cleanup, [[Snippet]] expansion, [[Correction]]s on the final text, history, paste) form a separate pipeline, and mic teardown, media resume, and [[Cancelled Session]] handling belong to the PTT orchestration around the Session — not the Session itself. So "from PTT-down to paste" describes the user-facing dictation journey, not the Session module's boundary: the Session hands off a transcript and the orchestration carries it the rest of the way.

## Cancelled Session

A [[Session]] the user aborted by pressing Escape while still recording. Cancellation tears down the mic and resumes any paused media, but skips every downstream stage — AI cleanup, history append, stats, and paste — so the focused app receives no text and the dictation leaves no trace beyond a brief "Cancelled" flash in the overlay pill. Only valid during the recording phase: once the user releases PTT, the Session is committed and Escape is inert.

## Local Engine

An [[Engine]] that runs Whisper inference on-device via whisper.cpp, rather than sending audio to a cloud provider. Unlike cloud Engines it is **batch-only**: it collects all audio until PTT-up, then returns a single transcript (no streaming partials). Supports two model variants: **Large v3** (~1.5 GB, highest accuracy) and **Large v3 Turbo** (~809 MB, faster). Each variant is loaded lazily on the first PTT that uses it and kept in memory until the [[Model Idle Timeout]] for that variant expires. Both can be loaded simultaneously if different Profiles use different variants.

## Model Catalog

A dedicated Settings section where locally downloadable models are listed with their download state, disk usage, and controls to start/cancel a download or delete a model. The Catalog manages what is on disk; Profiles reference what is already downloaded. Download uses HTTP Range requests so an interrupted transfer resumes from where it left off rather than restarting. It surfaces as the **Local** section of the Speech models settings page.

Not a Profile setting: the Catalog is global, shared across all Profiles that use the [[Local Engine]].

## AI Provider

The LLM backend used for the post-STT cleanup step (Anthropic Claude, OpenAI, Google Gemini, …), distinct from an [[Engine]] / speech model that turns audio into text. An AI Provider never sees audio — it only rewrites already-transcribed text.

Each AI Provider has **one global credential** (entered once on the AI Providers settings page — having a key saved makes the provider "configured"). But which provider + which model a cleanup run uses is chosen **per-Profile**: a Profile that enables cleanup selects its own provider and a model from that provider. So two Profiles can clean up with different providers/models while sharing the same saved keys.

Anthropic is the only provider reached through its native API (and the only one supporting [[OAuth credential]]); every other provider is reached through an OpenAI-compatible HTTP path. This split is an implementation detail invisible to domain reasoning — see Flagged ambiguities for why we keep Anthropic native.

Distinction from [[Engine]]: an Engine is a _speech model_ (Deepgram, Groq, AssemblyAI, local Whisper); an AI Provider is a _language model_ used only for cleanup. Both are cloud "providers", which is why the word is ambiguous — see Flagged ambiguities.

## Custom Provider

A user-supplied [[AI Provider]] reachable at any OpenAI-compatible `/chat/completions` endpoint — typically a self-hosted local model server (Ollama, LM Studio, llama.cpp, vLLM). Unlike the built-in providers (whose base URL is fixed and whose models come from a curated list), the Custom Provider stores its own `base_url` (required), `model` name (free-text, may be blank for single-model servers), and `api_key` (optional — omitted from the request when blank, since local servers usually need no auth). Exactly one Custom Provider exists.

Named "Custom", deliberately **not** "Local", to avoid colliding with [[Local Engine]] — that is on-device Whisper _speech_ recognition, a completely separate feature. See Flagged ambiguities.

## OAuth credential

An alternate way to authenticate the Anthropic [[AI Provider]]: a Claude Pro/Max subscription token (`sk-ant-oat…`) used instead of a pay-per-token API key. It is **Anthropic-only** and **global** — a single app-wide toggle chooses whether Anthropic authenticates via OAuth token or API key; no other provider supports it, and Profiles never see it (a Profile picks the provider "Anthropic", not how Anthropic authenticates). The OAuth path additionally asserts the "Claude Code" identity and sends Anthropic's OAuth beta header.

## Flagged ambiguities

- "Profile" vs "Mode" are the same concept: the UI consistently says **Profile** (sidebar, editor, toasts); the code, Tauri commands, and ADRs say **Mode** (`Mode`, `update_mode`, `mode.term_set_ids`). Glossary entries use Mode for the code-facing concept; anything user-facing renders it as Profile.
- "Provider" was overloaded: it meant both an STT [[Engine]] (speech model) and the cleanup LLM. Resolved by splitting the settings surface into **Speech models** (cloud Engines + the local [[Model Catalog]]) and **AI Providers** (the [[AI Provider]] for cleanup). "Speech model" or "Engine" = STT; "AI Provider" = cleanup LLM.
- "Local" is overloaded: [[Local Engine]] = on-device Whisper STT, whereas a self-hosted cleanup LLM is a [[Custom Provider]] — never called "local". The two are unrelated.
- Groq appears on **both** sides: it is a speech [[Engine]] (Whisper STT) _and_ can be a cleanup [[AI Provider]] (Llama LLMs). Same vendor, two different jobs in two different settings sections. The Speech models / AI Providers split keeps them apart in the UI.
- OpenAI also appears on **both** sides: it is a speech [[Engine]] (gpt-4o-transcribe) _and_ can be a cleanup [[AI Provider]] (GPT-4o, etc.). The speech key (`openai_api_key` in settings) is independent of the cleanup key (`ai_cleanup.provider_keys["openai"]`). A user who uses OpenAI for both STT and cleanup enters the key twice — same vendor, two jobs, two independent keys. This matches the Groq precedent.
- "Native vs OpenAI-compatible" is an implementation detail, not a domain concept: Anthropic is reached through its own API (the only path supporting an [[OAuth credential]] and prompt caching via `cache_control`); every other [[AI Provider]] is reached through the shared OpenAI-compatible HTTP path. The cleanup _rules_ are identical across providers — only the request envelope differs.

## Model Idle Timeout

A global setting that controls how long after the last PTT session a loaded local model stays in memory before being unloaded. Configurable: 5 min / 15 min / 30 min / 1 hour / Never. Default: 15 minutes. The timer resets on every successful transcription regardless of which Profile triggered it.
