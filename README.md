# Whispr

A push-to-talk dictation app. Hold a shortcut, speak, release — the
transcription is typed into whatever app has focus. Speech is transcribed by
the cloud provider (or on-device model) you choose, optionally cleaned up by an
LLM, then injected as keystrokes. Nothing is persisted to disk beyond a local
transcript history and your settings.

Runs on **macOS, Windows, and Linux**.

## Features

- **Push-to-talk dictation** — hold the shortcut to record, release to paste
  into the focused app; double-tap and "paste latest" actions are configurable.
- **Multiple speech providers** — cloud streaming via
  [Deepgram](https://console.deepgram.com),
  [Groq](https://console.groq.com) (Whisper Large v3 / v3 Turbo), and
  [AssemblyAI](https://www.assemblyai.com) (Universal streaming + Whisper), or
  fully **on-device** with local Whisper (Large v3, Large v3 Turbo) and Parakeet.
- **Local model catalog** — download, resume, verify, and delete on-device
  models from Settings. GPU-accelerated where available (Metal on macOS,
  DirectML on Windows, Vulkan on Linux).
- **AI cleanup** — optionally pass the raw transcript through an LLM to strip
  fillers, fix self-corrections, and apply casing/punctuation. Providers:
  Anthropic (native API, with Claude Pro/Max OAuth support), OpenAI, Google
  Gemini, Groq, DeepSeek, Cerebras, OpenRouter, and any OpenAI-compatible
  endpoint (Ollama, LM Studio, vLLM, …) via the Custom provider.
- **Profiles** — each profile picks its own speech provider/model, language,
  cleanup provider/model, terms, corrections, and snippet behavior.
- **Terms** — recognition hints biasing the speech model toward known words.
- **Corrections** — post-transcription find-and-replace rules.
- **Snippets** — spoken shorthands expanded into longer text, with placeholders
  (`{{DATE}}`, `{{TIME}}`, `{{CLIPBOARD}}`).
- **Transcript history** — stored locally, never sent anywhere.

## Prerequisites

- [Rust](https://rustup.rs/) stable
- [pnpm](https://pnpm.io/installation) and Node.js 20+
- [cmake](https://cmake.org/download/) — required to compile whisper.cpp for the
  local Whisper engine

Platform-specific:

- **macOS** — Xcode Command Line Tools (`xcode-select --install`); `brew install cmake`.
- **Windows** — the MSVC toolchain (Visual Studio Build Tools) and the WebView2
  runtime (preinstalled on Windows 11).
- **Linux** — WebKitGTK and supporting libraries. On Debian/Ubuntu:

  ```sh
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev \
    patchelf libasound2-dev libxdo-dev libvulkan-dev glslc cmake
  ```

## Setup

```sh
pnpm install
pnpm tauri dev
```

On first launch the OS prompts for the permissions Whispr needs to tap global
key events (for the push-to-talk shortcut) and inject transcribed text:

- **macOS** — grant **Accessibility** under _System Settings → Privacy &
  Security → Accessibility_.
- **Windows** — no extra permission required.
- **Linux** — keystroke injection uses `wtype`/`ydotool`/`dotool` on Wayland or
  `xdotool` on X11; install the tool matching your session.

To use a cloud speech provider or AI cleanup, add the relevant API key in the
app's **Speech models** / **AI Providers** settings pages. Keys are stored in
the local settings file and are never returned to the webview over IPC.

## Build

```sh
pnpm tauri build
```

Bundles are produced under `src-tauri/target/release/bundle/`:

- **macOS** — `.dmg`
- **Windows** — `.msi` and `_en-US.setup.exe`
- **Linux** — `.AppImage` and `.deb`

## Where things live

Settings and history are written to Tauri's per-platform app data directory for
`com.whispr.app`:

- **macOS** — `~/Library/Application Support/com.whispr.app/`
- **Windows** — `%APPDATA%\com.whispr.app\`
- **Linux** — `$XDG_DATA_HOME/com.whispr.app/` (usually `~/.local/share/com.whispr.app/`)

That directory holds `settings.json` and `history.json` (mode `0600` on Unix),
plus downloaded local models under `models/`. API keys live in the settings
file on disk and are never returned to the webview over IPC.

## Project layout

- `src/` — React + TypeScript frontend (settings UI, overlay window)
- `src-tauri/src/` — Rust backend (audio capture, global key listener,
  speech engines, AI cleanup, keystroke injection); platform-specific paths are
  gated with `#[cfg(target_os = ...)]`

## Installing a pre-built release

Grab the latest build from
[Releases](https://github.com/maksymomelchuk/whispr/releases/latest):

- **macOS** — open the `.dmg` and drag Whispr into `/Applications`. The build is
  **unsigned** (no Apple Developer ID), so the first launch needs a one-time
  Gatekeeper bypass: right-click the app → **Open** → confirm the "unidentified
  developer" prompt. Subsequent launches work normally.
- **Windows** — run the `.msi` or `_en-US.setup.exe`. The build is unsigned; if
  SmartScreen warns, click **More info → Run anyway**.
- **Linux** — either `chmod +x Whispr_*.AppImage` and run the AppImage, or
  install the `.deb` with `sudo dpkg -i whispr_*.deb`.

Once running, auto-updates are handled by the built-in updater
(`tauri-plugin-updater`) — no reinstall required for later versions.

## Releasing a new version

Before tagging, run through the [manual smoke checklist](docs/manual-smoke.md)
to verify the parts of the app that can't be covered by automated tests.

Releases are cut by `.github/workflows/release.yml`: push a semver tag and the
workflow builds installers for macOS, Windows, and Linux in parallel, publishes
a single GitHub Release, and uploads the signed `latest.json` manifest that
installed apps poll.

1. Bump the version in **three** files (they must match):
   - `package.json` → `version`
   - `src-tauri/Cargo.toml` → `[package] version`
   - `src-tauri/tauri.conf.json` → `version`
2. Commit: `git commit -am "chore(release): v1.5.0"`.
3. Tag and push:
   ```sh
   git tag v1.5.0
   git push origin main --tags
   ```
4. Watch the run at `Actions → Release`.

### One-time setup before the first release

The updater verifies downloads against a signing key you generate locally and
store as a GitHub Actions secret:

```sh
pnpm tauri signer generate -w ~/.tauri/whispr.key
```

- The command prints a **public** key — paste it into
  `src-tauri/tauri.conf.json → plugins.updater.pubkey`.
- The **private** key file (`~/.tauri/whispr.key`) stays on your machine —
  never commit it. Paste its full contents as the GitHub secret
  `TAURI_SIGNING_PRIVATE_KEY` (Repo → Settings → Secrets and variables →
  Actions). If you set a password for it, add
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` too.

If the private key is ever lost, updates break for existing installs — you'd
have to publish a new public key and ship a build with it, which existing
installs can't auto-update to. Back it up somewhere safe.

## License

MIT — see `LICENSE` if present, or add one before publishing a release.
