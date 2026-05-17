# Manual smoke checklist

Run this before cutting a release. Copy the whole file into the PR body as the release log — checkboxes survive the copy.

Each item is a binary pass/fail. If anything fails, open a bug before merging.

---

## 1. First launch + permissions

- [ ] Fresh install: app launches without crash on macOS 14+.
- [ ] Accessibility prompt appears on first launch; granting it does not require a restart.
- [ ] Microphone prompt appears on first record attempt; granting it lets audio flow.
- [ ] Revoking Accessibility in System Settings → relaunching → re-granting restores full functionality.
- [ ] Settings file is created at `~/Library/Application Support/com.whispr.app/settings.json` with mode 0600.

---

## 2. Hotkey — single-press / double-tap / modifier release

- [ ] Single press-and-hold: overlay appears, spinner shows, audio is captured, text is pasted on release.
- [ ] Short tap (< 300 ms): session is discarded, nothing is pasted, overlay dismisses cleanly.
- [ ] Double-tap within the configured interval: second mode activates (or fallback if none is bound), not first.
- [ ] Modifier-only release (no speech): no crash, overlay dismisses within 500 ms.
- [ ] Holding key while switching app focus: paste lands in the focused app at release.
- [ ] Rebinding the hotkey in Settings takes effect without restarting the app.

---

## 3. Provider sweep

### Deepgram

- [ ] Valid API key: transcription succeeds and text is pasted.
- [ ] Invalid / missing API key: error notice shown in overlay, nothing pasted, no crash.
- [ ] Deepgram model dropdown (Nova-2, etc.) switches without requiring a restart.

### Groq

- [ ] Switch provider to Groq in General settings: subsequent dictation uses Groq, text pasted correctly.
- [ ] Valid Groq key: transcription succeeds.
- [ ] Invalid / missing Groq key: error notice shown, nothing pasted, no crash.
- [ ] Groq live-preview appears during recording and converges to final transcript on release.

---

## 4. Mode sweep

- [ ] **Default EN** — plain English dictation: transcript pasted verbatim, no cleanup, no translation.
- [ ] **Cleaned EN** — English with AI cleanup: transcript pasted with LLM-cleaned punctuation and casing.
- [ ] **Ukrainian** — Ukrainian speech: transcript pasted in Ukrainian.
- [ ] **UA → EN** — Ukrainian speech translated to English: English text pasted, no Ukrainian leaks through.
- [ ] Switching modes mid-session (between recordings) takes effect on the next press without restart.
- [ ] Mode name is visible in the tray tooltip or overlay so the active mode is unambiguous.

---

## 5. Per-toggle dictation

Test with a mode that has all toggles enabled, then disable each one individually and confirm the pipeline behaves correctly.

- [ ] **use_terms ON**: custom terms appear in Deepgram keyterms / Groq prompt hint; recognizing a term works.
- [ ] **use_terms OFF**: terms list is not sent to the provider; no visible change to transcription accuracy for normal words.
- [ ] **use_corrections ON**: a known correction rule (e.g. "teh" → "the") is applied to the pasted text.
- [ ] **use_corrections OFF**: the same correction rule is not applied; raw transcript is pasted.
- [ ] **use_snippets ON**: a configured snippet trigger expands to its full text on paste.
- [ ] **use_snippets OFF**: snippet trigger is pasted literally.
- [ ] **AI cleanup ON**: short utterances below `min_words` / `min_duration` bypass the LLM (no extra latency).
- [ ] **AI cleanup OFF**: no LLM call is made; pasted text is the raw transcript regardless of length.

---

## 6. Paste targets

Dictate a short sentence into each target and verify the full text arrives correctly.

- [ ] **iTerm2** — terminal emulator: no garbled characters, paste arrives in the correct buffer.
- [ ] **Slack** (browser or native app): text arrives in the message box, not in another field.
- [ ] **Browser address bar** (Safari / Chrome): text replaces selection, does not trigger navigation.
- [ ] **Browser text field** (e.g. GitHub comment box): full text pasted, no characters dropped.
- [ ] **Password field**: characters are injected without being visible, field accepts them.
- [ ] **Cyrillic input source active** (Ukrainian keyboard selected in macOS): Latin text still pastes correctly; switching back to Latin input source works seamlessly.

---

## 7. Microphone hot-swap

- [ ] Dictate with the default mic, then change the input device in System Settings mid-session (between recordings): next dictation uses the new device without a restart.
- [ ] Unplugging a USB mic between recordings: app falls back to built-in mic, no crash or hang.
- [ ] Reconnecting the USB mic: next recording uses it again without a restart.

---

## 8. Network drop

- [ ] Disconnect network (Wi-Fi off or airplane mode) mid-recording: overlay shows an error notice, nothing is pasted, no crash.
- [ ] Reconnect network and record again: transcription resumes successfully without restarting the app.
- [ ] Transient drop (toggle off/on quickly during recording): result is either a clean error or a successful paste — no silent hang.

---

## 9. Sleep / wake

- [ ] Put machine to sleep mid-recording (lid close): session terminates cleanly, no crash on wake.
- [ ] Wake machine and record immediately: first recording after wake succeeds.
- [ ] Rapid sleep / wake cycle (three times): app remains responsive after each wake.

---

## 10. Tray + quit

- [ ] Tray icon is visible in the menu bar at launch.
- [ ] Tray menu opens on click and shows the correct active mode and settings shortcut.
- [ ] **Quit** from the tray terminates the process cleanly (no orphaned helpers in Activity Monitor).
- [ ] Relaunch after quit: app starts without requiring a settings re-entry.
- [ ] History persists across quit/relaunch: entries visible in the History tab after restart.
