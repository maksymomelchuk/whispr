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

## Engine

Provider-specific plumbing that turns audio into text — Deepgram's streaming WebSocket, Groq's polling REST. An Engine sees raw audio chunks and emits raw text-so-far updates plus a final raw transcript. It knows its protocol and nothing else: not [[Correction]]s, not the UI overlay, not how previews are throttled. Swapping providers means writing a new Engine; nothing else in the pipeline needs to change.

## Session

One push-to-talk dictation, from PTT-down to paste. The Session owns an [[Engine]] and everything around it: computing audio levels for the overlay, applying [[Correction]]s to the raw partials the Engine emits, throttling the preview event stream, and translating soft engine failures (e.g. Groq's final-POST fallback) into user-visible flashes. Engines are pluggable; the Session is the same regardless of which Engine is in use.
