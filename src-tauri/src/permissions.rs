#[cfg(target_os = "macos")]
pub fn ensure_accessibility_trust() {
    use macos_accessibility_client::accessibility;

    // Triggers the macOS Accessibility prompt if the current binary isn't
    // already trusted. Safe to call on every launch — it returns the current
    // trust state and only prompts once per (binary, decision) pair.
    if !accessibility::application_is_trusted_with_prompt() {
        eprintln!(
            "Accessibility: NOT granted. The PTT listener will run but receive no keys."
        );
        eprintln!(
            "Go to System Settings → Privacy & Security → Accessibility and enable this binary."
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_accessibility_trust() {}

/// Opens the Accessibility pane of System Settings. Useful as a fallback
/// action in the UI if the user dismissed the initial prompt.
#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() {}
