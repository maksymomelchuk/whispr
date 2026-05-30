# Remove the dedicated translation stage (Apple Translate)

We removed the dedicated translation pipeline stage and its only implementation, an on-device Swift sidecar wrapping macOS `Translation.framework`. Translation is now done — when needed — through the existing AI cleanup step with a custom `prompt_override` (e.g. "Translate the transcript to English"), so a `from → to` translation is just a Mode whose cleanup prompt asks for it. The seeded `UA → EN` mode was reshaped accordingly: Ukrainian spoken language plus a translate-to-English cleanup prompt, instead of `TranslateTarget::Apple { target: "en" }`.

## Considered Options

- **Keep Apple Translate.** On-device, private, free, offline. Rejected because it required macOS 26 (Tahoe), an explicitly-set source language (no Auto), user-downloaded language packs, and a `swiftc`-compiled sidecar with its own JSON protocol to build and maintain — a large surface for a single language pair in practice.
- **Keep `TranslateTarget` as an empty seam for a future provider (LLM/DeepL/etc.).** Rejected: an enum with only `Off` and a single-caller abstraction violates the repo's own "no speculative features / no single-caller abstractions" rules (AGENTS.md). Reintroduce a real stage if and when a second translation need appears.

## Consequences

- Translation now costs AI tokens and requires a cloud AI provider plus cleanup enabled — it is no longer available offline or for free.
- Translation and cleanup share one prompt slot, so a mode cannot both translate *and* apply a separate custom cleanup prompt in two distinct steps.
- No config migration ships: the orphaned `translate` field in persisted Modes is silently ignored on load (Mode has no `deny_unknown_fields`). Pre-existing translating modes degrade to plain transcription until reseeded or edited.
