# Wispr Tauri — Implementation Plan

Minimal Tauri-based push-to-talk speech-to-text app. Port of an existing Electron app living at `/Users/maksym/Developer/wispr-clone` — reference that code when looking for behavior parity, but do not share code or dependencies; this is a clean rewrite.

## Product scope

The user explicitly wants only two visible features in the UI:

1. **Deepgram API key field** — password input, persisted.
2. **Shortcut recorder** — click a button, press the desired key(s), save. Used as push-to-talk trigger (hold to record, release to transcribe + paste).

Everything else is implementation detail behind the scenes:

- Hold shortcut → MediaRecorder captures mic
- Release → audio sent to Deepgram → transcript pasted into whichever app is focused
- Small overlay pills: "recording" pill and "⚠ no input focused" warning pill
- Mute system audio while dictating, restore on release

Strip anything the current Electron app has that doesn't serve the above: Dashboard, History, Templates, manual "Start Recording" button, transcript display panel, etc. The Tauri port is deliberately tiny.

## Target stack

- **Framework:** Tauri 2.x
- **Frontend:** React + TypeScript + Vite (scaffold defaults)
- **Styling:** Plain CSS (no Tailwind) — keeping it dependency-light. macOS system font stack.
- **Storage:** Direct JSON file at `~/Library/Application Support/com.wisprclone.tauri/settings.json` (no plugin)
- **Global keyboard:** Hand-rolled `CGEventTap` via `core-graphics` + `core-foundation` crates — NOT `rdev`, NOT `device_query` (see Gotchas below)
- **HTTP:** `reqwest`
- **Audio capture:** Browser `MediaRecorder` via WKWebView
- **Clipboard:** `arboard` crate (or native via `objc2`) — not decided yet
- **macOS automation:** Shell-out to `osascript` for paste / focus detection / mute — same pragmatic approach as the Electron app

## Bundle identifier

`com.wisprclone.tauri` (deliberately distinct from the Electron app's `com.wisprclone.app` so macOS treats the two as separate binaries with separate TCC entries during the port).

---

## Current state

### ✅ Phase 0 — Scaffold (done)
- `pnpm create tauri-app@latest wispr-tauri` with React+TS+pnpm
- Window: 520×420, non-resizable, title "Wispr Tauri"
- Initial Tauri demo code removed

### ✅ Phase 1 — Settings persistence (done)
- **`src-tauri/src/config.rs`** — `Settings { api_key: Option<String>, shortcut: Shortcut }` and `Shortcut { key: String, modifiers: Vec<String> }` structs, load/save JSON at `~/Library/Application Support/com.wisprclone.tauri/settings.json`
- **`src-tauri/src/commands.rs`** — `get_settings`, `set_api_key`, `set_shortcut`, `open_accessibility_settings`
- **Frontend:**
  - `src/lib/types.ts` — TS mirrors of Rust types
  - `src/lib/api.ts` — typed `invoke` wrappers + `formatShortcut()` for display (`AltRight` → `Right ⌥`)
  - `src/components/ApiKeyField.tsx` — password input with Save button, transient "Saved" chip
  - `src/components/ShortcutField.tsx` — displays current shortcut + "Record new" button
  - `src/App.tsx`, `src/App.css` — layout, dark-mode-aware styling

### ✅ Phase 2 — Shortcut recorder (done)
- **`src/components/ShortcutRecorder.tsx`** — modal backdrop + live keyboard capture
- Uses `KeyboardEvent.code` (layout-independent) so captured values like `KeyA`, `Slash`, `AltRight` match what the Rust listener sees
- Modifier-only shortcuts (e.g., Right Option alone) stored as `{ key: "AltRight", modifiers: [] }`
- Modifier+key combos stored as `{ key: "Space", modifiers: ["Meta", "Shift"] }`
- Esc or backdrop-click cancels, Save persists via `set_shortcut` command

### ✅ Phase 3 — Global keyboard listener (done — via CGEventTap)
- **`src-tauri/src/ptt.rs`** — spawned thread creates a `CGEventTap` listening for KeyDown / KeyUp / FlagsChanged, attaches to a fresh `CFRunLoop` on the same thread, enables the tap, blocks on `CFRunLoop::run_current()`
- **`src-tauri/src/state.rs`** — `AppState` with `Arc<Mutex<Shortcut>>`, `Arc<Mutex<ModifierState>>`, `Arc<Mutex<bool>>` (PTT active). All Arcs → cheap `Clone` for sharing between tap thread and Tauri command handlers.
- **`src-tauri/src/permissions.rs`** — `ensure_accessibility_trust()` triggers the macOS Accessibility prompt at first run via `macos-accessibility-client`
- **`src/hooks/usePtt.ts`** — subscribes to Tauri events `ptt-pressed` / `ptt-released`, exposes `isHeld` + optional onPressed/onReleased callbacks
- Header chip in main window flips red when held (visual confirmation)
- Live updates — changing the shortcut in the UI takes effect immediately, no restart

**What the Rust listener does:**

- KeyDown/KeyUp: maps macOS virtual keycode → `KeyboardEvent.code` string (letters, digits, punctuation, arrows, F1–F12, Space/Enter/Tab/Esc/Backspace)
- FlagsChanged: macOS fires one per modifier toggle, so per-side state (L/R separately) is tracked by flipping a bit per event. This is needed because the CGEventFlags bitmask can't distinguish L vs R Option, Cmd, etc.
- Emits `ptt-pressed` / `ptt-released` Tauri events when the configured shortcut's key+modifiers match

**Dependencies so far (`src-tauri/Cargo.toml`):**

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[target.'cfg(target_os = "macos")'.dependencies]
macos-accessibility-client = "0.0.1"
core-foundation = "0.10"
core-graphics = "0.24"
```

---

## Remaining phases

### ⏸ Phase 4 — Audio capture

**Frontend work only.** Reuse the pattern from the Electron app at `/Users/maksym/Developer/wispr-clone/src/renderer/hooks/useAudioTranscription.ts`.

Steps:
1. New hook `src/hooks/useAudioCapture.ts` using `navigator.mediaDevices.getUserMedia({ audio: true })` + `MediaRecorder(stream, { mimeType: 'audio/webm' })`.
2. On first PTT press: create the stream, start the MediaRecorder with a 100ms timeslice.
3. Collect chunks in `ondataavailable`.
4. On PTT release: call `stop()`, assemble `Blob` from chunks, convert to `Uint8Array` bytes.
5. Wire into `App.tsx` via `usePtt({ onPressed, onReleased })`.
6. Send the audio bytes to Rust via a new command (see Phase 5) as `Array.from(new Uint8Array(buffer))`.

**Info.plist:** Phase 10 will add `NSMicrophoneUsageDescription`. During `pnpm tauri dev`, the WKWebView will prompt for mic on first getUserMedia call — that's fine.

**Deliverable:** hold shortcut → console logs a Blob of reasonable size (tens of KB for a 2-second utterance) on release.

### ⏸ Phase 5 — Deepgram transcription

**Rust work.** New module `src-tauri/src/transcription.rs`.

```rust
#[tauri::command]
pub async fn transcribe(
    audio: Vec<u8>,
    app: AppHandle,
) -> Result<String, String> {
    let settings = config::load(&app);
    let key = settings.api_key.ok_or("API key not configured")?;
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.deepgram.com/v1/listen?model=nova-3&smart_format=true&language=multi")
        .header("Authorization", format!("Token {key}"))
        .header("Content-Type", "audio/webm")
        .body(audio)
        .send()
        .await
        .map_err(|e| format!("Deepgram request failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Deepgram API error: {}", res.status()));
    }

    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;
    let transcript = json["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(if transcript.is_empty() {
        String::new()
    } else {
        // Trailing space so back-to-back dictations concatenate cleanly.
        format!("{} ", transcript.trim_end())
    })
}
```

Add to `Cargo.toml`:

```toml
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

And update `main.rs` / `lib.rs` if async runtime needs configuration.

Register in invoke_handler. Frontend calls via `invoke<string>('transcribe', { audio: Array.from(bytes) })`.

**Deliverable:** hold shortcut → talk → release → Rust returns the transcript string.

### ⏸ Phase 6 — Paste + clipboard restore

**Rust work.** New module `src-tauri/src/paste.rs`.

Strategy (matches Electron app behavior):
1. Save current clipboard text.
2. Write transcript to clipboard (use `arboard` crate or native `NSPasteboard`).
3. Simulate Cmd+V via AppleScript: `tell application "System Events" to keystroke "v" using command down`.
4. After ~200ms, restore the previous clipboard content.

```rust
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn paste_with_restore(text: &str) {
    // save current clipboard
    let previous = read_clipboard();

    write_clipboard(text);

    let previous_clone = previous.clone();
    thread::spawn(move || {
        let _ = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to keystroke \"v\" using command down"])
            .output();
        thread::sleep(Duration::from_millis(200));
        if let Some(prev) = previous_clone {
            write_clipboard(&prev);
        }
    });
}
```

Call from the transcription flow: after getting the transcript back, frontend invokes a `paste_text` command that does the above.

**Info.plist:** Phase 10 adds `NSAppleEventsUsageDescription`. First paste attempt triggers the macOS Automation prompt for System Events.

**Nice-to-have (v2):** replace osascript with `CGEventPost` via `core-graphics` crate for faster, no-prompt paste. Skip for MVP.

### ⏸ Phase 7 — Focus detection

**Rust work.** New module `src-tauri/src/focus.rs`.

Port the focus-check logic from `/Users/maksym/Developer/wispr-clone/src/main/index.ts` (functions `getFocusedElementInfo`, `isTextInputFocused`).

Blacklist approach, NOT whitelist — terminals and many apps don't expose text input via standard AX roles, so we allow by default and block only clearly-non-input cases:

```rust
const NON_INPUT_ROLES: &[&str] = &[
    "AXButton", "AXMenuItem", "AXMenu", "AXMenuBar", "AXMenuBarItem",
    "AXCheckBox", "AXRadioButton", "AXSlider", "AXPopUpButton",
    "AXImage", "AXLink", "AXTab",
];
```

Rules:
- If script fails (Automation denied, etc.) → **fail open** (allow dictation)
- If role is in NON_INPUT_ROLES → block, show warning overlay
- If app is Finder with no text-input role → block (likely desktop or file list)
- Otherwise → allow

Shell out to `osascript`:

```applescript
tell application "System Events"
  try
    set frontApp to first process whose frontmost is true
    set appName to name of frontApp
    try
      set focusedElement to value of attribute "AXFocusedUIElement" of frontApp
      return appName & "|" & (value of attribute "AXRole" of focusedElement)
    on error
      return appName & "|"
    end try
  on error
    return "|"
  end try
end tell
```

Call from `ptt-pressed` handler before emitting the recording event; if the check fails, emit a different event `ptt-blocked-no-input` that the warning overlay listens for.

**Skip the check** when the Wispr Tauri main window itself is focused — transcript should go somewhere visible.

### ⏸ Phase 8 — Overlays

Two additional Tauri windows declared in `tauri.conf.json`:

```json
{
  "windows": [
    { "label": "main", "title": "Wispr Tauri", "width": 520, "height": 420, ... },
    {
      "label": "recording-overlay",
      "url": "overlay.html",
      "decorations": false,
      "transparent": true,
      "alwaysOnTop": true,
      "skipTaskbar": true,
      "resizable": false,
      "focus": false,
      "visible": false,
      "width": 80,
      "height": 26
    },
    {
      "label": "warning-overlay",
      "url": "warning-overlay.html",
      "decorations": false,
      "transparent": true,
      "alwaysOnTop": true,
      "skipTaskbar": true,
      "resizable": false,
      "focus": false,
      "visible": false,
      "width": 200,
      "height": 30
    }
  ]
}
```

Static HTML files (like the Electron app's `public/overlay.html` and `public/warning-overlay.html`) — waveform pill for recording, red pill with "⚠ Focus a text input" for warning.

Rust commands to show/hide: `app.get_webview_window("recording-overlay").map(|w| w.show())` etc.

Positions: center bottom of primary display. Use `app.monitor()` to get display bounds.

### ⏸ Phase 9 — System audio mute

**Rust work.** New module `src-tauri/src/audio_control.rs`.

```rust
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

static WE_MUTED: AtomicBool = AtomicBool::new(false);

pub fn mute_system_audio() {
    let script = "set wasMuted to output muted of (get volume settings)\nif not wasMuted then set volume output muted true\nreturn wasMuted as string";
    let output = Command::new("osascript")
        .args(["-e", script])
        .output();
    if let Ok(out) = output {
        let was_muted = String::from_utf8_lossy(&out.stdout).trim() == "true";
        WE_MUTED.store(!was_muted, Ordering::SeqCst);
    }
}

pub fn restore_system_audio() {
    if !WE_MUTED.swap(false, Ordering::SeqCst) {
        return;
    }
    let _ = Command::new("osascript")
        .args(["-e", "set volume output muted false"])
        .output();
}
```

Called from:
- `startGlobalRecording` path (after focus check passes)
- `stopGlobalRecording` path
- App shutdown (`RunEvent::ExitRequested` or similar) — critical, so user isn't left muted after quitting

### ⏸ Phase 10 — macOS packaging

**`tauri.conf.json` bundle config:**

```json
{
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "identifier": "com.wisprclone.tauri",
    "macOS": {
      "category": "public.app-category.productivity",
      "minimumSystemVersion": "11.0",
      "entitlements": "entitlements.plist",
      "extendInfo": {
        "NSMicrophoneUsageDescription": "Wispr Tauri needs microphone access to transcribe speech.",
        "NSAppleEventsUsageDescription": "Wispr Tauri uses System Events to paste dictations and detect text inputs."
      }
    }
  }
}
```

**`src-tauri/entitlements.plist`:**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key>
  <true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
  <true/>
  <key>com.apple.security.cs.allow-dyld-environment-variables</key>
  <true/>
  <key>com.apple.security.cs.disable-library-validation</key>
  <true/>
  <key>com.apple.security.device.audio-input</key>
  <true/>
  <key>com.apple.security.automation.apple-events</key>
  <true/>
</dict>
</plist>
```

Run: `pnpm tauri build`. Output: `src-tauri/target/release/bundle/dmg/Wispr Tauri_0.1.0_aarch64.dmg`. Target size: ~15 MB (vs 282 MB for the Electron app).

### ⏸ Phase 11 — Polish

- First-run flow: if API key empty, highlight that field with a helpful message
- Error states: "Invalid key" (Deepgram 401), "Out of credits" (402), "No network"
- Status chip in main window: "Ready" / "Recording…" / "Transcribing…"
- App icon (replace Tauri defaults in `src-tauri/icons/`)
- Optional: small Deepgram console link via `tauri-plugin-opener`

---

## Gotchas (lessons from Phase 3)

**These cost real iteration — don't retry them.**

### `rdev` crashes on macOS when called from a spawned thread

On macOS 13+, `rdev::listen()` on a thread other than the main thread segfaults the whole process on the first key event. Symptom: "wispr-tauri quit unexpectedly" dialog. **Don't use rdev.**

### `device_query` returns silent empty keys on macOS despite permissions

Even with Accessibility AND Input Monitoring both granted for the dev binary, `device_query` returns `vec![]` — no errors, no prompts, just silent failure. Likely a combination of macOS TCC strictness on unsigned ad-hoc-signed dev binaries and the specific API it uses. **Don't use device_query.**

### `macos-accessibility-client::application_is_trusted_with_prompt()`

Can return `true` for dev binaries that aren't actually in the Accessibility list — stale TCC grant state. Trust its return value for logging only; test actual PTT behavior to confirm permissions.

The prompt it fires is also flaky for unsigned dev binaries — sometimes it never appears even after `tccutil reset Accessibility`. Document manual steps (add the binary via `+` in System Settings) as the fallback.

### What actually works: `CGEventTap` + `CFRunLoop` on a dedicated thread

The working pattern:
1. `std::thread::spawn(move || { ... })`
2. Inside: create `CGEventTap` with the closure (closure must be `Fn + Send + Sync + 'static`)
3. Get the tap's `mach_port` **field** (not a method — accessed as `tap.mach_port` without parens in core-graphics 0.24)
4. Create a runloop source: `tap.mach_port.create_runloop_source(0)`
5. `CFRunLoop::get_current().add_source(&loop_source, kCFRunLoopCommonModes)`
6. `tap.enable()`
7. `CFRunLoop::run_current()` — blocks forever

All steps on the same spawned thread. rdev gets this wrong somewhere internally.

### Modifier keys and CGEventFlags

The flag bitmask can't distinguish L from R Option (both map to `kCGEventFlagMaskAlternate`). To track individual modifier keys:
- Maintain a `ModKeyState` with 8 bools (l/r × alt/meta/control/shift)
- On every FlagsChanged event with a modifier keycode: **toggle** the corresponding bool
- macOS guarantees one FlagsChanged per press/release, so toggle-on-event is correct

### Keycode mapping

Be exhaustive with the `macos_keycode_to_code` function. Missing entries cause silent dropped events, confusingly making only *some* shortcuts work. Current map includes A-Z, 0-9, F1-F12, Space, Enter, Tab, Escape, Backspace, Arrows, Slash, Comma, Period, Semicolon, Quote, Backquote, Backslash, Minus, Equal, BracketLeft, BracketRight, and all modifier keys. F13-F20 are NOT currently mapped; add if needed.

### Frontend vs Rust key-code naming

The frontend ShortcutRecorder stores `KeyboardEvent.code` strings (`KeyA`, `Slash`, `AltRight`, etc.). The Rust listener maps macOS virtual keycodes to the *same* strings. **Do not diverge these two naming schemes** — they must match exactly.

### Permissions the packaged app will need

- **Microphone** — `NSMicrophoneUsageDescription` + `com.apple.security.device.audio-input` entitlement. Triggers WKWebView's getUserMedia prompt.
- **Accessibility** — needed for CGEventTap. No entitlement needed, just Info.plist and user grant.
- **Automation (System Events)** — `NSAppleEventsUsageDescription` + `com.apple.security.automation.apple-events` entitlement. Triggers on first osascript + System Events call.

Dev builds will show the generic "exec" icon in Settings lists because they're ad-hoc signed. That's cosmetic — permissions still work.

---

## Design decisions worth preserving

- **JSON settings file, not tauri-plugin-store.** One less plugin dependency, and we control the schema fully.
- **No Tailwind / no shadcn.** Plain CSS is sufficient for the tiny UI; keeps bundle small and avoids a dep tree.
- **`formatShortcut()` is the display layer** — the stored shape `{ key, modifiers }` never changes, only how it's rendered to the user.
- **Fail-open focus detection.** We'd rather let a dictation through than silently block the user for a detection edge case.
- **System-wide mute, not per-app pause.** One mechanism handles Spotify / YouTube / Apple Music / anything else. Tracks pre-mute state so we never unmute a user who was already muted.

---

## File layout

```
wispr-tauri/
├── PLAN.md                               # this file
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── src/                                  # React frontend
│   ├── App.tsx
│   ├── App.css
│   ├── main.tsx
│   ├── components/
│   │   ├── ApiKeyField.tsx
│   │   ├── ShortcutField.tsx
│   │   └── ShortcutRecorder.tsx
│   ├── hooks/
│   │   └── usePtt.ts                     # + useAudioCapture.ts in Phase 4
│   └── lib/
│       ├── types.ts
│       └── api.ts
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── capabilities/
    │   └── default.json
    ├── icons/                            # default Tauri icons, replace in Phase 11
    └── src/
        ├── main.rs
        ├── lib.rs                        # Tauri::Builder setup
        ├── commands.rs                   # all #[tauri::command] entry points
        ├── config.rs                     # Settings + Shortcut structs, file I/O
        ├── state.rs                      # AppState w/ shared Arc<Mutex<_>> fields
        ├── ptt.rs                        # CGEventTap + CFRunLoop
        ├── permissions.rs                # Accessibility prompt + settings opener
        # Phase 4+: new modules land here
        # ├── transcription.rs            # Phase 5
        # ├── paste.rs                    # Phase 6
        # ├── focus.rs                    # Phase 7
        # ├── overlays.rs                 # Phase 8
        # └── audio_control.rs            # Phase 9
```

---

## Running the project

```bash
cd /Users/maksym/Developer/wispr-tauri
pnpm install              # only needed on first clone / after dep changes
pnpm tauri dev            # starts Vite dev server + Tauri shell
```

First run on macOS will:
1. Compile Rust dependencies (~2–3 min)
2. Trigger the Accessibility permission prompt (may be flaky — fallback: manually add `/Users/maksym/Developer/wispr-tauri/src-tauri/target/debug/wispr-tauri` to System Settings → Privacy & Security → Accessibility)
3. Launch the Tauri window

## Reference

Existing working Electron app at `/Users/maksym/Developer/wispr-clone`. Check its source for behavior parity when porting:

- `src/main/index.ts` — uiohook, AppleScript paste, focus detection, system mute, tray, overlays
- `src/renderer/services/transcription.ts` — Deepgram endpoint + response parsing
- `src/renderer/hooks/useAudioTranscription.ts` — MediaRecorder flow
- `public/overlay.html`, `public/warning-overlay.html` — overlay HTML/CSS to copy over
- `build/entitlements.mac.plist` — entitlements for reference
- `CLAUDE.md` — project overview (some sections stale, treat with suspicion)
