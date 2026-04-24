# Wispr Tauri

A push-to-talk dictation app. Hold a shortcut, speak, release — the
transcription is typed into whatever app has focus. Audio is sent to
[Deepgram](https://console.deepgram.com) for transcription; nothing is
persisted to disk beyond a local transcript history.

> Runtime support today is **macOS only** (CGEventTap, CGEventPost, CoreAudio
> via cpal, transparent overlay via `macOSPrivateApi`). The Tauri + React
> frontend is portable; the native modules are gated with
> `#[cfg(target_os = "macos")]`, so the crate still compiles on other
> targets — ports for Windows/Linux are a welcome contribution.

## Prerequisites

- macOS (tested on recent versions of macOS 14+)
- [Rust](https://rustup.rs/) stable
- [pnpm](https://pnpm.io/installation)
- Xcode Command Line Tools (`xcode-select --install`)

## Setup

```sh
pnpm install
pnpm tauri dev
```

On first launch macOS will prompt for **Accessibility** permission — this is
required to tap global key events (for the push-to-talk shortcut) and to
inject the transcribed text as keystrokes. You can manage this later under
_System Settings → Privacy & Security → Accessibility_.

You also need a Deepgram API key. Get one from
[console.deepgram.com](https://console.deepgram.com) and paste it into the
**General** tab inside the app.

## Build

```sh
pnpm tauri build
```

The app bundle is produced under `src-tauri/target/release/bundle/`.

## Where things live

- **Settings** — `~/Library/Application Support/com.wispr-tauri.app/settings.json` (0600)
- **History** — `~/Library/Application Support/com.wispr-tauri.app/history.json` (0600)

The Deepgram API key is stored in the settings file on disk; it is never
returned to the webview over IPC.

## Project layout

- `src/` — React + TypeScript frontend (settings UI, overlay window)
- `src-tauri/src/` — Rust backend (audio capture, CGEventTap, Deepgram client, keystroke injection)

## License

MIT — see `LICENSE` if present, or add one before publishing a release.
