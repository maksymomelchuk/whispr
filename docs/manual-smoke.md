# Manual smoke checklist

Run this **once per environment** before cutting a release. Copy the file into the PR body as
the release log — checkboxes survive the copy. Record the environment at the top of each run.

Each item is a binary pass/fail. If anything fails, open a bug before merging.

> **Environment under test:** `____________________`
> (e.g. `macOS 14 / Apple Silicon`, `Windows 11`, `Ubuntu 24.04 / X11`, `Fedora 40 / Wayland`)

Automated coverage (run by CI on all three OSes, do **not** re-test by hand): the pure cores in
`hotkey`, `keysym`, `paste` (chunking + injector selection), `model_catalog`, `platform`, the
cloud-session logic, and the React layer. This checklist covers only what those tests cannot — the
per-OS facades and real end-to-end behavior.

---

## Environment matrix

| Environment                         | Parity bar                                      |
| ----------------------------------- | ----------------------------------------------- |
| macOS (Apple Silicon **and** Intel) | full parity — regression guard                  |
| Windows 10 / 11                     | full parity                                     |
| Linux — **X11** session             | full parity                                     |
| Linux — **Wayland** session         | core only; OS-forbidden extras degrade silently |

Legend for per-item scope tags: **(all)** every environment · **(mac)** · **(win)** ·
**(x11)** · **(wayland)** · **(linux)** both Linux sessions.

---

## 0. Day-one parity gate (all)

The single check that must pass on every environment. If this fails, stop — nothing else matters.

- [ ] **(all)** Hold the push-to-talk hotkey, speak one sentence, release: the transcript is
      injected into the currently focused application.

---

## 1. Install + first launch + permissions

- [ ] **(mac)** `.dmg`: right-click → Open bypasses Gatekeeper; app launches without crash.
- [ ] **(win)** NSIS `.setup.exe` installs (incl. on a clean Windows with no VC++ runtime); SmartScreen → **More info → Run anyway** launches it; no extra permission prompts.
- [ ] **(linux)** `.AppImage` runs after `chmod +x`; `.deb` installs via `sudo dpkg -i` and launches.
- [ ] **(mac)** Accessibility prompt appears on first launch; granting it needs no restart.
- [ ] **(mac)** Microphone prompt appears on first record; granting it lets audio flow.
- [ ] **(mac)** Revoke Accessibility → relaunch → re-grant restores full functionality.
- [ ] **(all)** Settings file is created and recording works on a clean profile:
  - macOS: `~/Library/Application Support/com.whispr.app/settings.json` (mode 0600)
  - Windows: `%APPDATA%\com.whispr.app\settings.json`
  - Linux: `~/.config/com.whispr.app/settings.json`

---

## 2. Hotkey — `ptt` event source (all)

- [ ] **(all)** Single press-and-hold: overlay appears, audio is captured, text pasted on release.
- [ ] **(all)** Short tap (< 300 ms): session discarded, nothing pasted, overlay dismisses cleanly.
- [ ] **(all)** Double-tap within the configured interval activates the second-mode binding (or fallback), not the first.
- [ ] **(all)** Modifier-only release (no speech): no crash, overlay dismisses within 500 ms.
- [ ] **(all)** Escape while recording cancels the session: nothing pasted, overlay dismisses, no crash.
- [ ] **(all)** Holding the key while switching app focus: paste lands in the newly focused app at release.
- [ ] **(all)** Rebinding the hotkey in Settings takes effect without restarting the app.
- [ ] **(wayland)** Key-down **and** key-up are both detected globally (hold semantics survive under the compositor).

---

## 3. Paste targets + injector — `paste` (all)

Dictate a short sentence into each target and verify the full text arrives intact.

- [ ] **(all)** Terminal emulator (iTerm2 / Windows Terminal / GNOME Terminal): no garbled characters.
- [ ] **(all)** Electron app (e.g. Slack / VS Code): text lands in the message/editor field, none dropped.
- [ ] **(all)** Browser text field (e.g. GitHub comment box): full text pasted, nothing reordered.
- [ ] **(all)** Browser address bar: text replaces selection, does not trigger navigation.
- [ ] **(all)** Password field: characters injected and accepted.
- [ ] **(all)** Held PTT modifiers are released **before** injection — text is not eaten as a shortcut.
- [ ] **(all)** Non-Latin keyboard layout active (e.g. Cyrillic): Latin transcript still pastes correctly; switching layout back works.
- [ ] **(x11)** Injector uses `xdotool` (verify; `enigo` is the fallback if absent).
- [ ] **(wayland)** Injector uses `wtype` / `dotool` / `ydotool` per availability; `enigo` fallback when none present.

---

## 4. Local engine — `local_session` / transcribe-rs (all)

Skip cloud for these; force a local model in the active mode.

- [ ] **(mac)** Whisper transcribes on Metal.
- [ ] **(win)** **(linux)** Whisper transcribes on Vulkan where a supported GPU exists.
- [ ] **(all)** CPU fallback: with no usable GPU, Whisper still transcribes (slower, but correct).
- [ ] **(all)** Parakeet model transcribes end-to-end where available.
- [ ] **(all)** Idle timeout: after the configured idle period, the loaded model is unloaded and memory drops.
- [ ] **(all)** First record after an idle-unload lazy-loads the model and succeeds.

---

## 5. Model catalog — `model_catalog` (all)

- [ ] **(all)** Download a single-file Whisper model: progress shown, completes, model usable.
- [ ] **(all)** Download a multi-file Parakeet model (encoder/decoder/joiner + tokenizer): all files fetched.
- [ ] **(all)** Interrupt a multi-file download mid-way → restart: it **resumes** rather than re-downloading completed files.
- [ ] **(all)** Disk-usage figure shown per model matches actual on-disk size.
- [ ] **(all)** Delete a model: files removed, disk-usage updates, model no longer selectable.

---

## 6. Overlay + target-app icon — `overlay` / `target_app`

- [ ] **(all)** Recording overlay pill appears above other windows during a session.
- [ ] **(mac)** **(win)** Overlay shows the icon of the app the text will land in.
- [ ] **(x11)** Overlay shows the target-app icon where the WM exposes it.
- [ ] **(wayland)** Target-app icon is **omitted silently** (no error, no broken layout) when the compositor blocks detection.
- [ ] **(wayland)** On a compositor that blocks click-through overlays, the overlay disables cleanly — no focus theft, no crash; recording still works.

---

## 7. Media mute — `media` (all)

- [ ] **(all)** With "mute during recording" ON: system output mutes while recording, restores on release.
  - macOS: `osascript` · Windows: Core Audio · Linux: `pactl`
- [ ] **(all)** With the setting OFF: output is untouched during recording.
- [ ] **(linux)** Where no audio control tool is present, recording proceeds as a no-op (no crash).

---

## 8. Provider sweep — cloud engines (all)

### Deepgram

- [ ] **(all)** Valid API key: transcription succeeds and text is pasted.
- [ ] **(all)** Invalid / missing key: error notice in overlay, nothing pasted, no crash.
- [ ] **(all)** Model dropdown (Nova-2, etc.) switches without restart.

### Groq

- [ ] **(all)** Switch provider to Groq: subsequent dictation uses Groq, text pasted correctly.
- [ ] **(all)** Valid key: transcription succeeds; live-preview appears and converges to the final transcript on release.
- [ ] **(all)** Invalid / missing key: error notice shown, nothing pasted, no crash.

### AssemblyAI

- [ ] **(all)** Valid key: transcription succeeds and text is pasted.
- [ ] **(all)** Invalid / missing key: error notice shown, nothing pasted, no crash.

---

## 9. Mode sweep (all)

- [ ] **(all)** Plain dictation mode: transcript pasted verbatim, no cleanup.
- [ ] **(all)** AI-cleanup mode: transcript pasted with LLM-cleaned punctuation and casing.
- [ ] **(all)** Non-English mode: speech in the configured language pastes correctly.
- [ ] **(all)** Translate-via-cleanup mode (cleanup prompt that translates): output language is correct, source language does not leak through.
- [ ] **(all)** Per-mode model selection works, spanning Whisper variants **and** Parakeet.
- [ ] **(all)** Switching modes between recordings takes effect on the next press without restart.
- [ ] **(all)** Active mode is unambiguous from the tray tooltip or overlay.

---

## 10. Per-toggle dictation (all)

Use a mode with every toggle ON, then flip each individually.

- [ ] **(all)** `use_terms` ON: custom terms reach the provider; recognizing a term works. OFF: terms not sent.
- [ ] **(all)** `use_corrections` ON: a known rule (e.g. "teh" → "the") is applied. OFF: raw transcript pasted.
- [ ] **(all)** `use_snippets` ON: a configured trigger expands. OFF: trigger pasted literally.
- [ ] **(all)** AI cleanup ON: utterances below `min_words` / `min_duration` bypass the LLM. OFF: no LLM call regardless of length.

---

## 11. Microphone hot-swap (all)

- [ ] **(all)** Change input device between recordings: next dictation uses the new device, no restart.
- [ ] **(all)** Unplug a USB mic between recordings: falls back to a default device, no crash or hang.
- [ ] **(all)** Reconnect the USB mic: next recording uses it again, no restart.

---

## 12. Network drop (all)

- [ ] **(all)** Disconnect network mid-recording (cloud engine): overlay shows an error, nothing pasted, no crash.
- [ ] **(all)** Reconnect and record again: transcription resumes without restart.
- [ ] **(all)** Transient drop (toggle off/on quickly): result is a clean error or a successful paste — no silent hang.

---

## 13. Sleep / wake (all)

- [ ] **(all)** Sleep mid-recording (lid close / suspend): session terminates cleanly, no crash on wake.
- [ ] **(all)** Record immediately after wake: first recording succeeds.
- [ ] **(all)** Three rapid sleep/wake cycles: app stays responsive after each wake.

---

## 14. Tray + quit + persistence (all)

- [ ] **(all)** Tray / indicator icon visible at launch.
- [ ] **(all)** Tray menu opens and shows the correct active mode and a settings shortcut.
- [ ] **(all)** Quit from the tray terminates the process cleanly (no orphaned helper processes).
- [ ] **(all)** Relaunch after quit starts without re-entering settings.
- [ ] **(all)** History persists across quit/relaunch; entries visible in the History tab.
- [ ] **(all)** History attributes each dictation to its target app where the platform can detect it (omitted silently where it cannot, e.g. Wayland).

---

## 15. Artifact + auto-updater (per release tag)

Verify against `docs/adr/0002-distribution-signing-packaging-updater.md`.

- [ ] **(all)** `latest.json` is published on the release and contains a `platforms` entry for this environment's target triple.
- [ ] **(all)** Installing the previous release, then launching with this release published, triggers an in-app update that installs and relaunches.
  - macOS: `.app.tar.gz` · Windows: NSIS `.zip` · Linux: `.AppImage.tar.gz`
- [ ] **(all)** Updater signature verifies (minisign / `TAURI_SIGNING_PRIVATE_KEY`) — no signature-mismatch error.
- [ ] **(linux)** A `.deb` install does **not** receive in-app updates (expected per ADR); the AppImage does.
