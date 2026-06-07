# Distribution: Windows signing, Linux packaging, and per-platform auto-updater

Three decisions needed before the release matrix (issue #101) can ship.

## Windows code-signing

**Decision: unsigned for v1.**

The existing macOS build ships unsigned (Gatekeeper requires right-click → Open on first
launch), so unsigned-for-v1 is a consistent precedent. SmartScreen bypass: click
**More info → Run anyway** in the Windows Defender SmartScreen dialog. Installation
instructions in every release body document this step.

Two distinct problems must both be solved to ship a warning-free Windows build, and a
certificate alone does not solve the second:

- An **"Unknown publisher" prompt**, removed by any valid code-signing certificate.
- The **SmartScreen reputation screen** ("Windows protected your PC"), removed only by an
  EV certificate or a cloud signing service that carries reputation — an OV certificate
  does not clear it until enough installs accrue.

Since June 2023, all OV and EV certificates must be stored on a FIPS-validated hardware
token or cloud HSM; file-based certificates are no longer issued. Every real option below
also requires a registered legal entity, except the individual tier of Azure Trusted
Signing. This overhead is not warranted for an early-access open-source tool.

### Considered options

- **EV Authenticode certificate.** Immediate SmartScreen trust with no bypass needed.
  ~$300–600+ USD/year. Rejected: cost, registered-legal-entity validation, mandatory
  hardware token, and certificate lifecycle management are disproportionate for an
  early-access project.
- **OV Authenticode certificate.** ~$200–400 USD/year; clears "Unknown publisher" but
  **not** SmartScreen until reputation builds over many installs. Rejected: pays the cost
  and hardware-token overhead without removing the warning users actually see first.
- **Azure Trusted Signing.** Microsoft's cloud signing service, ~$120 USD/year (~$10/mo);
  carries SmartScreen reputation, no physical token. Cheapest path that removes the
  warning, and its individual tier is the only no-company option. Deferred: revisit
  post-v1 if Windows install friction proves material.
- **Self-signed certificate.** Still triggers SmartScreen; no practical benefit over
  unsigned for end-users without pre-installing the cert. Rejected.
- **Unsigned.** Consistent with macOS precedent; documented bypass is one extra click.
  **Accepted.**

For contrast, macOS removes its warning fully for a flat $99 USD/year Apple Developer
membership (Developer ID signing + notarization + stapling), with no company required —
the cheaper and cleaner first buy if signing is revisited.

## Windows packaging and runtime dependency

**Decision: NSIS only; bundle the Visual C++ runtime and install it on first run.**

The native dependencies (`whisper.cpp` compiled into `whispr.exe`, and the bundled
`onnxruntime.dll`) link the dynamic VC++ 2015–2022 runtime. A clean Windows install lacks
it, so the app fails to launch with `MSVCP140.dll was not found` — confirmed on a fresh
Windows VM. Dev machines and most consumer PCs only have the runtime because some other
installer pulled it in incidentally; the installer cannot rely on that.

- **Windows targets are restricted to `nsis`** (overridden in `tauri.windows.conf.json`).
  The runtime is installed via an NSIS `installerHooks` script, which only runs for the
  NSIS installer — an MSI build would ship the same broken-on-clean-install binary. NSIS
  is also the only Windows artifact the updater consumes (see the updater table below), so
  MSI carried no benefit.
- **`vc_redist.x64.exe` is bundled as an installer resource** and run silently from
  `NSIS_HOOK_POSTINSTALL` (`/install /quiet /norestart`). A registry check
  (`SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64`, read via `SetRegView 64`) skips
  the sub-install when the runtime is already present, so silent auto-updates don't trigger
  a UAC prompt on every run.
- The redistributable is **downloaded at build time** by a Windows-only CI step (from
  `https://aka.ms/vs/17/release/vc_redist.x64.exe`) rather than committed to the repo.

### Considered options

- **Static-link the CRT (`+crt-static`).** Fixes `whispr.exe` but not the separately
  shipped `onnxruntime.dll`, a prebuilt Microsoft binary that dynamically links the same
  runtime — the app would launch and then crash when ONNX transcription runs. Rejected as
  insufficient on its own.
- **Bundle the loose CRT DLLs app-locally.** Lighter and needs no UAC, but the DLL set must
  be kept complete by hand and located in CI from a versioned VS path. Rejected as more
  fragile than the redistributable installer, which is Microsoft's supported mechanism.
- **Document a manual VC++ install in the release notes.** Pushes a hard launch-time
  dependency onto every first-time user. Rejected.

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

| Platform | Updater artifact   | Format        |
| -------- | ------------------ | ------------- |
| macOS    | `.app.tar.gz`      | Tauri default |
| Windows  | NSIS `.zip`        | Tauri default |
| Linux    | `.AppImage.tar.gz` | Tauri default |

Updater artifacts are signed with the existing `TAURI_SIGNING_PRIVATE_KEY` secret (Tauri's
own minisign scheme, not OS code-signing) on all three platforms.

### Consequences

- Windows users see a SmartScreen dialog on first install; documented in every release's
  installation notes.
- Linux users on RPM-based distributions must use the AppImage.
- Auto-updates on Linux work only via AppImage installs; `.deb` installs do not receive
  in-app updates.
