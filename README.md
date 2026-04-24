# Whispr

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

- **Settings** — `~/Library/Application Support/com.whispr.app/settings.json` (0600)
- **History** — `~/Library/Application Support/com.whispr.app/history.json` (0600)

The Deepgram API key is stored in the settings file on disk; it is never
returned to the webview over IPC.

## Project layout

- `src/` — React + TypeScript frontend (settings UI, overlay window)
- `src-tauri/src/` — Rust backend (audio capture, CGEventTap, Deepgram client, keystroke injection)

## Installing a pre-built release

Grab the latest `.dmg` from [Releases](https://github.com/maksymomelchuk/whispr/releases/latest)
and drag Whispr into `/Applications`.

The build is **unsigned** (no Apple Developer ID), so the first launch
needs a one-time Gatekeeper bypass:

1. Right-click the app → **Open** → confirm the "unidentified developer" prompt.
2. Subsequent launches work normally.

Once Whispr is running, auto-updates are handled by the built-in updater
(`tauri-plugin-updater`). No re-prompt required for later versions — the
already-trusted app writes the new bundle in place.

## Releasing a new version

Releases are cut by `.github/workflows/release.yml`: push a semver tag and
the workflow builds a universal macOS binary, publishes a GitHub Release,
and uploads a signed `latest.json` manifest that the installed app polls.

1. Bump the version in **three** files (they must match):
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `src-tauri/tauri.conf.json` → `version`
2. Commit: `git commit -am "Release v0.2.0"`.
3. Tag and push:
   ```sh
   git tag v0.2.0
   git push origin main --tags
   ```
4. Watch the run at `Actions → Release`. Takes ~10 min.

### One-time setup before the first release

The updater verifies downloads against a signing key you generate locally
and store as a GitHub Actions secret:

```sh
pnpm tauri signer generate -w ~/.tauri/whispr.key
```

- The command prints a **public** key — paste it into
  `src-tauri/tauri.conf.json → plugins.updater.pubkey`
  (replacing the `REPLACE_WITH_BASE64_UPDATER_PUBKEY` placeholder).
- The **private** key file (`~/.tauri/whispr.key`) stays on your machine —
  never commit it. Paste its full contents as the GitHub secret
  `TAURI_SIGNING_PRIVATE_KEY` (Repo → Settings → Secrets and variables →
  Actions). If you set a password for it, add
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` too.

If the private key is ever lost, updates break for existing installs —
you'd have to publish a new public key and ship a build with it, which
existing installs can't auto-update to. Back it up somewhere safe.

## License

MIT — see `LICENSE` if present, or add one before publishing a release.
