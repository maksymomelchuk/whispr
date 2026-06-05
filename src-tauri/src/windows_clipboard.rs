use clipboard_win::{raw, register_format, Clipboard, EnumFormats};

// Windows mirror of the nspasteboard transient marker: an update carrying this
// registered format is skipped by Clipboard History (Win+V), Cloud Clipboard,
// and well-behaved managers (Ditto, …) while the real data stays pasteable.
// Tagging both our writes — the transcription and the restore of the user's
// content — stops every dictation from re-archiving the clipboard.
const EXCLUDE_FORMAT_NAME: &str = "ExcludeClipboardContentFromMonitorProcessing";

// Opening can briefly fail while another process holds the clipboard; retry a
// few times before giving up.
const OPEN_ATTEMPTS: usize = 10;

// GDI-handle formats: their clipboard data is a handle, not a global-memory
// byte buffer, so snapshotting their raw bytes is meaningless. Windows
// re-synthesizes CF_BITMAP/CF_PALETTE from CF_DIB on restore anyway.
const HANDLE_FORMATS: &[u32] = &[
    2,    // CF_BITMAP
    3,    // CF_METAFILEPICT
    9,    // CF_PALETTE
    14,   // CF_ENHMETAFILE
    0x80, // CF_OWNERDISPLAY
    0x82, // CF_DSPBITMAP
    0x83, // CF_DSPMETAFILEPICT
    0x8E, // CF_DSPENHMETAFILE
];

/// Every memory-backed (format, raw bytes) pair on the clipboard, owned so it
/// survives the paste delay before we write it back.
pub struct ClipboardSnapshot {
    items: Vec<(u32, Vec<u8>)>,
}

pub fn snapshot() -> ClipboardSnapshot {
    let mut items = Vec::new();
    if let Ok(_clipboard) = Clipboard::new_attempts(OPEN_ATTEMPTS) {
        for format in EnumFormats::new() {
            if HANDLE_FORMATS.contains(&format) {
                continue;
            }
            let mut bytes = Vec::new();
            if raw::get_vec(format, &mut bytes).is_ok() {
                items.push((format, bytes));
            }
        }
    }
    ClipboardSnapshot { items }
}

pub fn write_transient_text(text: &str) -> Result<(), String> {
    let _clipboard =
        Clipboard::new_attempts(OPEN_ATTEMPTS).map_err(|e| format!("clipboard open: {e}"))?;
    raw::set_string(text).map_err(|e| format!("clipboard set text: {e}"))?;
    set_exclusion_marker();
    Ok(())
}

pub fn restore(snapshot: ClipboardSnapshot) {
    let Ok(_clipboard) = Clipboard::new_attempts(OPEN_ATTEMPTS) else {
        return;
    };
    let _ = raw::empty();
    if snapshot.items.is_empty() {
        return;
    }
    for (format, bytes) in &snapshot.items {
        let _ = raw::set_without_clear(*format, bytes);
    }
    set_exclusion_marker();
}

fn set_exclusion_marker() {
    if let Some(format) = register_format(EXCLUDE_FORMAT_NAME) {
        // Presence of the format is the signal; a one-byte payload is enough.
        let _ = raw::set_without_clear(format.get(), &[0]);
    }
}
