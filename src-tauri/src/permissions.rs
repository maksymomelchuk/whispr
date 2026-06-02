#[cfg(target_os = "macos")]
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[cfg(target_os = "macos")]
pub fn ensure_accessibility_trust() {
    use macos_accessibility_client::accessibility;

    // Triggers the macOS Accessibility prompt if the current binary isn't
    // already trusted. Safe to call on every launch — it returns the current
    // trust state and only prompts once per (binary, decision) pair.
    if !accessibility::application_is_trusted_with_prompt() {
        eprintln!("Accessibility: NOT granted. The PTT listener will run but receive no keys.");
        eprintln!(
            "Go to System Settings → Privacy & Security → Accessibility and enable this binary."
        );
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_accessibility_trust() {}

#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use macos_accessibility_client::accessibility;
    accessibility::application_is_trusted()
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    use objc2_foundation::ns_string;

    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return false;
    };
    // AVMediaTypeAudio = "soun"; AVAuthorizationStatusAuthorized = 3
    let status: isize =
        unsafe { msg_send![cls, authorizationStatusForMediaType: ns_string!("soun")] };
    status == 3
}

/// Routes the mic prompt through AVFoundation so AVCaptureDevice's cached
/// status reflects the grant. If the first mic access comes from cpal's
/// CoreAudio path instead, `authorizationStatusForMediaType:` stays stuck at
/// NotDetermined for the rest of the session. Idempotent: once status is
/// determined, requestAccess returns immediately without re-prompting.
#[cfg(target_os = "macos")]
pub fn ensure_microphone_trust() {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, Bool};
    use objc2_foundation::ns_string;

    let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
        return;
    };
    type CompletionBlock = Option<extern "C" fn(Bool)>;
    let completion: CompletionBlock = None;
    unsafe {
        let _: () = msg_send![
            cls,
            requestAccessForMediaType: ns_string!("soun"),
            completionHandler: completion
        ];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_microphone_trust() {}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() {}

#[cfg(target_os = "macos")]
pub fn open_microphone_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn open_microphone_settings() {}
