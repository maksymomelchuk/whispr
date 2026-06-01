# PRD: Multi-platform port — macOS + Windows + Linux

## Problem Statement

Whispr runs only on macOS. Every push-to-talk dictation — capture, the keyboard
listener, text injection, the overlay, frontmost-app detection, media muting, and
the on-device [[Local Engine]] — is wired to macOS-only system APIs (`CGEventTap`,
`CGEvent`, `NSWorkspace`, `osascript`, whisper.cpp's Metal backend). A Windows or
Linux user can launch the app and see the settings UI, but dictation is disabled:
there is no way to record, transcribe, or paste. This caps Whispr's audience to a
single platform and forecloses being a credible cross-platform open-source
dictation tool.

## Solution

Whispr runs natively on macOS, Windows, and Linux. On every platform the core job
works: press the hotkey, speak, and the transcript is injected into the focused
application. Cloud Engines (Deepgram, Groq, AssemblyAI) and the [[Local Engine]]
both work everywhere; [[Mode]]s, [[Correction]]s, [[Term]]s, [[Snippet]]s, history,
and stats behave identically because they are already platform-neutral.

Platform-specific behavior lives behind a small set of capability modules, each
with a platform-neutral public API and per-OS internals — the core pipeline never
branches on the operating system. Features the OS genuinely forbids (notably on
Linux/Wayland: click-through overlay, frontmost-app icon) degrade silently and
never block a recording. The on-device engine migrates from `whisper-rs` to
`transcribe-rs` so the same code path serves Whisper on every platform's GPU and
opens the door to the Parakeet model in the same refactor.

Where a platform mechanism is generic and app-agnostic (keyboard capture, text
injection, GPU backend selection), we reuse the proven approach from the
MIT-licensed [Handy](https://github.com/cjpais/Handy) project rather than inventing
our own, keeping its copyright notice on any lifted code.

## Scope summary

- **Targets:** macOS (existing) + Windows + Linux/X11 at full feature-parity;
  Linux/Wayland best-effort. Windows and Linux ship in the same release.
- **Day-one gate:** record → transcribe → paste works on every platform. Extras
  (target-app icon, click-through overlay) degrade silently where the OS forbids
  them; they never block a release.
- **Out of the box:** Apple Translate is already removed end-to-end (see
  `docs/adr/0001-remove-apple-translate.md`); it is not part of this work.

## User Stories

1. As a Windows user, I want to install Whispr and have it launch, so that I can use dictation on my primary OS.
2. As a Linux user on an X11 session, I want full dictation parity with macOS, so that I am not a second-class user.
3. As a Linux user on a Wayland session, I want core dictation (record → transcribe → paste) to work, so that the app is usable even where some extras are unavailable.
4. As any user, I want to hold my configured push-to-talk hotkey and have recording start on key-down and stop on key-up, so that push-to-talk feels identical to macOS.
5. As any user, I want my double-tap and single-press [[Hotkey binding]]s to behave the same as on macOS, so that my muscle memory transfers across platforms.
6. As any user, I want the transcript injected into whatever app is focused, so that dictation lands where I am typing.
7. As a Windows user, I want injected text to arrive intact in Electron apps, browsers, and native apps, so that no characters are dropped or reordered.
8. As a Linux user, I want text injection to use my session's native tool (wtype/dotool/ydotool on Wayland, xdotool on X11) and fall back gracefully, so that injection works across compositors and desktop environments.
9. As any user, I want my held PTT modifier keys to be released before text is typed, so that injected text is not misinterpreted as a shortcut.
10. As any user, I want to choose my input microphone from the devices my OS exposes, so that I can record from the right source on any platform.
11. As any user, I want a Cloud [[Engine]] (Deepgram, Groq, AssemblyAI) to work on my platform, so that I can dictate without downloading a local model.
12. As any user, I want the [[Local Engine]] to run on-device on my platform, so that I can dictate fully offline.
13. As a macOS user, I want the local engine to keep using Metal, so that on-device transcription stays fast on my hardware.
14. As a Windows or Linux user with a GPU, I want the local engine to use Vulkan acceleration, so that on-device transcription is fast.
15. As any user without a supported GPU, I want a CPU fallback for the local engine, so that on-device transcription still works.
16. As any user, I want to download, see disk usage for, and delete local models from the [[Model Catalog]] on my platform, so that I can manage on-device storage.
17. As any user, I want an interrupted model download to resume rather than restart, so that I do not waste bandwidth — including for multi-file models.
18. As any user, I want to select Parakeet as a local model where available, so that I get fast CPU transcription without a GPU.
19. As any user, I want the [[Model Idle Timeout]] to unload models the same way on every platform, so that memory behavior is predictable.
20. As any user, I want the recording overlay pill to appear above my other windows during a [[Session]], so that I get visual feedback while dictating.
21. As a Linux/Wayland user whose compositor blocks click-through overlays, I want the overlay to disable cleanly rather than steal focus or error, so that dictation still works.
22. As a macOS or Windows user, I want the overlay to show the icon of the app the text will land in, so that I can confirm the target before speaking.
23. As a Linux/Wayland user where frontmost-app detection is unavailable, I want the overlay to omit the icon silently, so that the absence of an extra does not break the flow.
24. As any user, I want media to mute (or not) during recording per my setting, so that recording behavior matches my preference on my platform.
25. As any user, I want my history to attribute each dictation to the app it was sent to where the platform can detect it, so that I can review where text went.
26. As a Windows user, I want to grant whatever permission my OS requires (or none, if none is required), so that I am not blocked by macOS-style permission prompts that do not apply.
27. As a macOS user, I want the existing Accessibility and Microphone permission prompts to keep working, so that the port does not regress my platform.
28. As any user, I want a [[Cancelled Session]] (Escape while recording) to behave identically on every platform, so that aborting a dictation is reliable.
29. As any user, I want [[Correction]]s, [[Term]]s, and [[Snippet]]s applied identically on every platform, so that my post-processing is consistent.
30. As any user, I want my [[Mode]]s and per-mode model selection to work on every platform, including choosing between Whisper variants and Parakeet, so that my workflows transfer.
31. As a maintainer, I want platform-specific code isolated behind capability modules with platform-neutral APIs, so that the core pipeline carries no `cfg` branches and stays easy to reason about.
32. As a maintainer, I want the platform-agnostic cores (hotkey state machine, keysym mapping, paste chunking, model-catalog spec, platform detection) unit-tested without an OS, so that I can refactor per-OS glue without fear.
33. As a maintainer, I want a CI matrix that builds and releases macOS, Windows, and Linux artifacts, so that I can ship all three from one tag.
34. As any user, I want an installable artifact for my platform (signed where feasible), so that installation is not blocked by OS security warnings I cannot bypass.
35. As any user, I want auto-updates to work on my platform, so that I receive future releases without reinstalling.
36. As a maintainer, I want a documented decision on Windows code-signing and Linux packaging formats, so that distribution is not an afterthought at release time.

## Implementation Decisions

### Architecture: capability modules (module-facade + internal `cfg`)

- Each platform-coupled capability is a single module exposing a platform-neutral
  public API. Per-OS implementations and runtime fallback chains live **inside** the
  module behind `#[cfg(...)]`. The core pipeline depends only on the neutral API and
  contains no `cfg` branches. This matches the existing intent recorded at
  `lib.rs:57` and Handy's structure.
- Formal capability traits are explicitly **not** used: a Tauri build compiles exactly
  one impl per target, so compile-time `cfg` selection is sufficient and trait
  ceremony would buy nothing. The discipline that matters is keeping each module's
  _public_ surface platform-neutral.
- Modules currently gated only because they were wired into the macOS pipeline —
  `recorder` (already `cpal`), and the Cloud [[Engine]] sessions
  (`deepgram_session`, `groq_session`, `assemblyai_session`, pure
  `tokio-tungstenite`/`reqwest`) — are un-gated and compiled on all platforms.

### Deep modules (platform-neutral cores, extracted from per-OS facades)

- **`hotkey` core** — the double-tap state machine (`advance_tap_state`) and
  [[Hotkey binding]] resolution, already platform-agnostic in `ptt.rs`. Extracted to
  consume _abstract_ key events (`KeyDown`/`KeyUp` + keysym + modifier state) and emit
  the existing dispatch decisions, independent of the event source.
- **`keysym` mapping** — pure translation between the keyboard library's key
  representation and Whispr's `Shortcut`, replacing the hardcoded macOS keycodes
  (e.g. `0x3A`). Bidirectional where the UI needs to display a binding.
- **`paste` chunking + injector selection** — the existing `next_chunk_end`
  word-boundary chunker stays pure; a new pure decision picks the Linux injector given
  the detected display server and which tools are present on `PATH`.
- **`model_catalog` spec + download planner** — a `ModelSpec` describing a model's
  files (single-file Whisper GGUF _vs._ multi-file Parakeet ONNX set + tokenizer) and a
  planner computing which files to fetch and where to resume. Pure logic over a spec.
- **`platform` detection** — pure detection of OS and Linux display server
  (`XDG_SESSION_TYPE` / `WAYLAND_DISPLAY`), feeding injector choice and overlay
  degradation.

### Per-OS facade modules

- **`ptt` event source** — replace the macOS `CGEventTap` + `CFRunLoop` + hardcoded
  keycodes with `rdev` (rustdesk fork) off-macOS, feeding the same `hotkey` core. The
  source must deliver reliable key-**down** and key-**up** globally so push-to-talk
  hold semantics hold; macOS may retain its existing `CGEventTap` source behind the
  same neutral API.
- **`paste` injectors** — macOS keeps `CGEvent` Unicode injection; Windows and macOS
  use `enigo`; Linux tries native tools (`wtype`/`dotool`/`ydotool` on Wayland,
  `xdotool` on X11) and falls back to `enigo`. The held-modifier-release wait is
  preserved on platforms where it applies.
- **`local_session` adapter** — rewritten against `transcribe-rs`'s `SpeechModel`
  trait, replacing direct `whisper-rs` use. Keeps the [[Local Engine]]'s batch-only,
  lazy-load, idle-unload behavior described in the domain glossary.
- **`target_app`** — macOS keeps `osascript` + `NSWorkspace` icon rendering; Windows
  uses Win32 (`GetForegroundWindow`, icon extraction); Linux/Wayland omits the icon
  (degrade). The neutral API returns an optional frontmost-app descriptor + optional
  icon.
- **`media`** — macOS keeps `osascript` output-mute; Windows uses Core Audio; Linux
  uses `pactl` (or no-op if unavailable). Fire-and-forget on every platform.
- **`overlay`** — Tauri transparent always-on-top window on macOS/Windows; Linux uses
  `gtk-layer-shell` where the compositor supports it and disables cleanly otherwise.
  The `macOSPrivateApi` / `visible_on_all_workspaces` settings stay macOS-only.
- **`permissions`** — existing non-macOS stubs (return granted) stay; Windows requires
  nothing; Wayland-specific handling is added only if a probe shows it is needed.

### Local engine migration: `whisper-rs` → `transcribe-rs`

- Decided to do the engine swap **during** the port (not after) because Parakeet is a
  near-term requirement; doing the `local_session` + [[Model Catalog]] refactor once
  avoids tearing it down twice.
- GPU/feature matrix mirrors Handy: Whisper via whisper.cpp with `whisper-metal`
  (macOS), `whisper-vulkan` (Windows/Linux), CPU fallback everywhere; ONNX via `ort`
  with `ort-directml` (Windows), `ort-coreml` (macOS), CPU/Vulkan (Linux) for Parakeet.
- The [[Model Catalog]] gains multi-file model support: download-resume (HTTP Range),
  disk-usage accounting, deletion, and idle eviction all operate over a `ModelSpec`
  that may enumerate several files (Parakeet = encoder/decoder/joiner + tokenizer)
  rather than a single GGUF.
- The [[Mode]] / Profile model-picker spans Whisper variants and Parakeet. Existing
  Whisper Large v3 / Large v3 Turbo variants and their behavior are preserved.

### Early validation (first work-items, not release blockers)

Handy demonstrates the stack works on all three platforms, so these are validation
tasks to run first rather than gating spikes:

1. `rdev` (rustdesk fork) delivers reliable hold-to-talk key-down **and** key-up
   globally, including under Wayland.
2. `transcribe-rs` builds with the GPU feature matrix and loads/runs Parakeet on at
   least one off-macOS target (Windows).

### Distribution & CI (decisions to be made within this work)

- Extend the release workflow (currently macOS-only, `runs-on: macos-latest`,
  `--target universal-apple-darwin`) to a macOS + Windows + Linux matrix.
- **Open decision — Windows signing:** Authenticode/EV cert vs. shipping unsigned and
  accepting SmartScreen warnings for v1. The current macOS build is already unsigned
  (Gatekeeper right-click-to-open), so unsigned-for-v1 is consistent with precedent.
- **Open decision — Linux packaging:** which of AppImage / `.deb` / Flatpak to ship.
- **Open decision — auto-updater:** today only the macOS `latest.json` endpoint exists
  in `tauri.conf.json`; per-platform updater artifacts and endpoints must be added so
  story #35 holds on Windows and Linux.

## Testing Decisions

A good test here asserts **external behavior of pure logic**, never implementation
detail or OS calls. Per `AGENTS.md`: test behavior not implementation, one logical
concept per test, no shared mutable state, and **do not mock what we own** — only the
genuine external boundary (the OS) is excluded, by virtue of the pure cores having no
OS dependency. The user's directive: test as much as possible _without overhead_ —
i.e. cover every pure core, but do not write OS-mocking or integration-glue tests that
would be brittle and low-value.

Modules to test (all pure, no OS):

- **`hotkey` core** — double-tap vs single-press dispatch over abstract key-event
  sequences; binding resolution when a shortcut has both a single-press and double-tap
  sibling. Extends the existing `ptt.rs` tap-state tests, which are the prior art.
- **`keysym` mapping** — round-trips and known-key assertions between the keyboard
  library's key representation and `Shortcut`.
- **`model_catalog` spec + download planner** — single-file vs multi-file specs;
  which files a resume fetches given partial state; disk-usage and delete accounting.
  New; highest-value coverage for the engine migration.
- **`paste` chunking + injector selection** — word-boundary chunking (existing
  coverage retained) plus the pure Linux injector-selection decision given a display
  server and a set of available tools.
- **`platform` detection** — OS + Linux display-server resolution from environment
  variable combinations.

Prior art in the repo: `ptt.rs` already unit-tests the tap state machine;
`corrections`, `terms`, and `snippets` carry pure-logic tests in the same style.

Explicitly **not** unit-tested (integration glue / OS-bound, would require mocking the
OS): the per-OS `ptt` event sources, the concrete `paste` injectors, `target_app`,
`media`, `overlay`, and the `local_session`/`transcribe-rs` adapter. These are
exercised via the manual smoke checklist (`docs/manual-smoke.md`) per platform.

## Out of Scope

- **Apple Translate / any translation stage** — already removed
  (`docs/adr/0001-remove-apple-translate.md`); translation is achieved via a [[Mode]]'s
  AI cleanup prompt and is unaffected by this port.
- **Mobile (iOS/Android).** Desktop only.
- **New product features beyond Parakeet.** The engine migration enables Parakeet; no
  other new models, no per-app/per-URL Workflows, no plugin system.
- **Wayland feature-parity with X11.** Wayland is best-effort; OS-forbidden extras
  degrade rather than being re-engineered to match.
- **Reaching the same on-device transcription latency on every GPU vendor.** Vulkan +
  CPU fallback is the bar; per-vendor tuning (e.g. CUDA, ROCm) is not.

## Further Notes

- **Reference implementation:** [Handy](https://github.com/cjpais/Handy)
  (`cjpais/Handy`) — a same-stack (Tauri v2 + React/TS + Rust) cross-platform
  dictation app. It is the proof that this stack runs on macOS, Windows, and Linux,
  and the source of the platform-glue approach reused throughout this PRD.
- Lifted Handy code is MIT-licensed; retain its copyright notice on any copied glue.
- Handy is local-only and has no [[Mode]]s, [[Correction]]s, [[Term]]s, [[Snippet]]s,
  or Cloud [[Engine]]s — so those parts of Whispr have no Handy code to copy and are
  the substantive porting work: threading the lifted platform layer through Whispr's
  richer pipeline.
- The `recorder` (`cpal`) and Cloud [[Engine]] modules are expected to need only
  un-gating, making them the lowest-risk first slices and a good way to prove the
  cross-platform build before tackling `ptt`.
- Suggested slice ordering for follow-up issues: (1) early validation; (2) un-gate
  recorder + cloud engines + CI matrix skeleton; (3) `hotkey`/`keysym`/`ptt` over
  `rdev`; (4) `paste` over `enigo` + Linux tools; (5) `transcribe-rs` migration +
  `model_catalog` multi-file; (6) `overlay`/`target_app`/`media` per-OS + Wayland
  degradation; (7) distribution/signing/updater.
