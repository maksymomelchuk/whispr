fn main() {
    tauri_build::build();

    #[cfg(target_os = "macos")]
    compile_apple_translate_sidecar();
}

/// Compile the Swift sidecar that wraps Apple's Translation.framework.
///
/// The output binary is placed in `src-tauri/binaries/` with the Tauri
/// sidecar naming convention (`apple-translate-{target_triple}`). Tauri
/// strips the triple suffix when it bundles the binary into the app's
/// MacOS/ directory so the runtime lookup via `current_exe().parent()` finds
/// it as plain `apple-translate` in both dev and production.
///
/// If `swiftc` is not available (CI without Xcode), the build succeeds with a
/// warning. The sidecar just won't work until swiftc is present.
#[cfg(target_os = "macos")]
fn compile_apple_translate_sidecar() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let swift_src = format!("{manifest_dir}/swift/apple-translate.swift");
    let target = std::env::var("TARGET").unwrap_or_else(|_| "aarch64-apple-darwin".to_string());

    // Tauri sidecar naming convention: binary-name-{target_triple}.
    let binaries_dir = format!("{manifest_dir}/binaries");
    std::fs::create_dir_all(&binaries_dir).ok();
    let output = format!("{binaries_dir}/apple-translate-{target}");

    // Map Rust target triple to Swift/Clang arch-os-version strings.
    let swift_target = if target.starts_with("aarch64") {
        "arm64-apple-macosx14.0"
    } else {
        "x86_64-apple-macosx14.0"
    };

    let status = std::process::Command::new("swiftc")
        .args([
            &swift_src,
            "-target",
            swift_target,
            "-o",
            &output,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!(
            "cargo:warning=swiftc exited with {s}; apple-translate sidecar not compiled"
        ),
        Err(e) => eprintln!(
            "cargo:warning=swiftc not found ({e}); apple-translate sidecar not compiled. \
             Install Xcode to enable on-device translation."
        ),
    }

    println!("cargo:rerun-if-changed={swift_src}");
}
