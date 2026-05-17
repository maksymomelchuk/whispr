# Domain glossary

Terms meaningful to anyone reasoning about the transcription pipeline. Implementation details live in the code; this file describes *what* the concepts are, not *how* they are stored.

## Term

A word or phrase the user wants the speech recognizer to know exists. Terms are injected into the STT engine as recognition hints (Deepgram `keyterm`, Groq Whisper `prompt`) *before* audio is transcribed. They have no replacement — their only job is to bias the recognizer toward producing the right word in the first place.

Example: `Anthropic`, `Tauri`, `kubectl`.

Not a Term: anything with a `from → to` shape. That's a [[Correction]].

## Correction

A post-STT find-and-replace rule. After the recognizer produces text, every Correction is applied: `from` text is replaced by `to` text. Corrections fix things the recognizer got wrong (or things the user *wants* phrased differently, e.g. verbal punctuation cues like `dot → .`).

Corrections do **not** bias the recognizer — their `from` is by definition the *wrong* word and biasing toward it would make transcription worse.

## Snippet

A user-defined shorthand the user *deliberately* uses in speech. Triggers are matched in the post-STT text and expanded into longer content, optionally with placeholders (`{{DATE}}`, `{{TIME}}`, `{{CLIPBOARD}}`).

Distinction from [[Correction]]: a Correction patches over a recognizer mistake (involuntary); a Snippet expands a chosen shortcut (voluntary). A Correction's replacement is always static text; a Snippet's expansion can contain placeholders resolved at injection time.
