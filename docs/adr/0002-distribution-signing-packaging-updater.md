# Distribution: Windows signing, Linux packaging, and per-platform auto-updater

Three decisions needed before the release matrix (issue #101) can ship.

## Windows code-signing

**Decision: unsigned for v1.**

The existing macOS build ships unsigned (Gatekeeper requires right-click → Open on first
launch). Authenticode/EV certificates cost roughly $200–400 USD/year and require legal
entity verification; this overhead is not warranted for an early-access open-source tool.
SmartScreen bypass: click **More info → Run anyway** in the Windows Defender SmartScreen
dialog. Installation instructions in every release body document this step.

### Considered options

- **EV Authenticode certificate.** Immediate SmartScreen trust with no bypass needed.
  Rejected: annual cost, legal entity verification, and certificate lifecycle management
  are disproportionate for an early-access project.
- **Self-signed certificate.** Still triggers SmartScreen; no practical benefit over
  unsigned for end-users without pre-installing the cert. Rejected.
- **Unsigned.** Consistent with macOS precedent; documented bypass is one extra click.
  **Accepted.**

## Linux packaging

**Decision: AppImage + `.deb`.**

- **AppImage** ships as a single self-contained executable — no install required, runs on
  any modern Linux distribution without root access. This is the format `tauri-action`
  produces by default.
- **`.deb`** covers Debian, Ubuntu, and Mint users who prefer a package-manager-managed
  install with system-level application registration.

Both formats are produced by `tauri-action` without extra configuration when
`targets: "all"` is set in `tauri.conf.json`.

### Considered options

- **Flatpak.** Universal format with Flathub distribution. Rejected: Flathub submission
  takes weeks; sandbox constraints conflict with the Accessibility APIs required for PTT
  keyboard capture on X11.
- **Snap.** Canonical-controlled distribution; snap confinement incompatible with PTT
  global event capture without elevated privileges. Rejected.
- **RPM.** Covers Fedora/RHEL; `tauri-action` does not produce RPMs by default. Deferred
  to a future slice if demand exists.

## Auto-updater

**Decision: single `latest.json` endpoint; AppImage-only updater on Linux.**

- A single endpoint at
  `https://github.com/maksymomelchuk/whispr/releases/latest/download/latest.json`
  serves all platforms.
- The JSON's `platforms` field contains per-OS/arch entries; Tauri's updater plugin
  selects the correct entry at runtime using the host's target triple.
- Each matrix build job (macOS, Windows, Linux) uploads its artifacts and updates
  `latest.json` via `tauri-action` with `includeUpdaterJson: true`. The action merges
  entries so the final file covers all three platforms.

| Platform | Updater artifact | Format |
|---|---|---|
| macOS | `.app.tar.gz` | Tauri default |
| Windows | NSIS `.zip` | Tauri default |
| Linux | `.AppImage.tar.gz` | Tauri default |

Updater artifacts are signed with the existing `TAURI_SIGNING_PRIVATE_KEY` secret (Tauri's
own minisign scheme, not OS code-signing) on all three platforms.

### Consequences

- Windows users see a SmartScreen dialog on first install; documented in every release's
  installation notes.
- Linux users on RPM-based distributions must use the AppImage.
- Auto-updates on Linux work only via AppImage installs; `.deb` installs do not receive
  in-app updates.
