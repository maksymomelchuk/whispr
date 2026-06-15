# Soniox: realtime-only Engine with STT-layer translation on the provider variant

Adding Soniox as a cloud speech [[Engine]] (wispr `CONTEXT.md` terms) introduced two
choices a future reader would question: why realtime-only when ElevenLabs has both batch
and realtime, and why the Soniox `ProviderModel` variant carries a `translate_to` field
instead of the `model` sub-enum every other variant carries.

## Decision

**Soniox is wired up as a single realtime WebSocket Engine that optionally translates in the
same stream, modeled as `ProviderModel::Soniox { translate_to: Option<String> }` with no
`model` sub-enum.**

- **Realtime only.** Soniox's async (file) API is job-based — create job, poll, fetch — which
  is structurally slower per dictation utterance than our existing _synchronous_ batch
  Engines (OpenAI, Groq, ElevenLabs Scribe v2), and slower than Soniox's own realtime stream
  where text finalizes as the user speaks. There is no per-utterance latency win to justify a
  second code path, so no batch Engine is added.
- **`translate_to` on the variant, not on `Mode`.** STT-layer translation (the Engine
  translates while transcribing — see [[Translation]]) is a Soniox-only capability. Keeping
  the field inside the variant makes illegal states unrepresentable (a Deepgram mode cannot
  carry a translation target), follows the existing `{ model }` precedent for
  provider-specific config, drops the field automatically when a Profile switches providers,
  and keeps a near-always-`null` field off every other Profile's persisted config. `None` =
  verbatim code-switching; `Some(code)` = one-way translation to that target.
- **No `model` sub-enum.** Soniox has exactly one realtime model (`stt-rt-v4`); a one-value
  enum is speculative scaffolding. The model id is a `const` in `soniox_session.rs`.
- **`language_hints` is never sent strict.** `Auto` → no hints; `Exact { code }` →
  `language_hints: [code]`; `Hints { codes }` → `language_hints: codes`. `language_hints_strict`
  is never set, because forcing a single language is exactly what breaks intra-sentence UK↔EN
  code-switching — the reason Soniox was chosen. This deliberately differs from the ElevenLabs
  realtime Engine, which sends a hard `language_code` for `Exact`.

## Considered options

- **Add Soniox async as a batch Engine too.** Rejected: poll-based job API adds round trips
  our synchronous batch Engines don't have; no latency benefit for short dictation clips.
- **`Mode.translate_to` (shared field).** Rejected today on YAGNI and illegal-states grounds.
  The refactor trigger is the day a _second_ Engine supports STT-layer translation (Google
  STT does); at that point extract a shared `TranslationConfig`. Until then the variant keeps
  the invariant in the type instead of in runtime guard code.
- **One-way and two-way translation.** Two-way (bidirectional, for live interpreters) does
  not fit a dictation-into-a-focused-field tool. One-way only.

## Consequences

- A Soniox Profile is _either_ verbatim code-switching _or_ translating — never both. The two
  [[Translation]] mechanisms (STT-layer here, cleanup-layer via [[AI Provider]]) are not
  guarded against each other; a Profile with both set translates twice (second pass a near
  no-op), and the product guidance is "pick one."
- Terms flow into Soniox's `context.terms` (JSON body, capped under the ~10k-char context
  ceiling) — no URL-length budget like the ElevenLabs realtime keyterm path.
- `enable_language_identification` and `enable_speaker_diarization` are fixed off (we never
  read per-token language or speaker). PTT key-release still drives an explicit `finalize`
  then end-of-stream, mirroring the ElevenLabs commit-then-drain.
- `enable_endpoint_detection` is **on** (`max_endpoint_delay_ms: 500`). The first cut left it
  off — PTT gives an exact end boundary, so finalize-at-release seemed sufficient. In practice
  that concentrated _all_ finalization into a post-release lump (worse the longer the
  utterance) and felt markedly slower than Soniox's Playground, which finalizes continuously.
  Turning endpoint detection on commits tokens to final as the speaker pauses, so the
  post-release paste only flushes the last short segment. The 500 ms delay (vs the 2000 ms
  default) trades a little stability for dictation snappiness and is the one exposed tunable.

## Sources

- [Soniox real-time transcription](https://soniox.com/docs/stt/rt/real-time-transcription)
- [Soniox WebSocket API](https://soniox.com/docs/api-reference/stt/websocket-api)
- [Soniox real-time translation](https://soniox.com/docs/translation/stt-translation/rt-translation)
