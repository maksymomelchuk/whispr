use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};

// nspasteboard.org convention: an item that also declares this type is excluded
// from clipboard-history managers (Raycast, Maccy, Paste, …) while its real
// payload stays pasteable. Tagging both our writes — the transcription and the
// restore of the user's content — stops every dictation from re-archiving the
// clipboard as a fresh history entry.
const TRANSIENT_TYPE: &str = "org.nspasteboard.TransientType";
const UTF8_TEXT_TYPE: &str = "public.utf8-plain-text";

// Convention says the marker's value identifies the writing app; managers only
// check the type's presence, so any non-empty payload works.
const MARKER_OWNER: &[u8] = b"com.whispr.app";

/// Every (type, raw data) pair of every item on the pasteboard, owned so it
/// survives the paste delay before we write it back.
pub struct ClipboardSnapshot {
    items: Vec<Vec<(String, Vec<u8>)>>,
}

pub fn snapshot() -> ClipboardSnapshot {
    let pasteboard = NSPasteboard::generalPasteboard();
    let mut items = Vec::new();
    if let Some(pasteboard_items) = pasteboard.pasteboardItems() {
        for item in pasteboard_items.iter() {
            let pairs = item
                .types()
                .iter()
                .filter_map(|item_type| {
                    let data = item.dataForType(&item_type)?;
                    Some((item_type.to_string(), data.to_vec()))
                })
                .collect();
            items.push(pairs);
        }
    }
    ClipboardSnapshot { items }
}

pub fn write_transient_text(text: &str) -> Result<(), String> {
    let wrote = write_tagged(&[vec![(UTF8_TEXT_TYPE.to_string(), text.as_bytes().to_vec())]]);
    if wrote {
        return Ok(());
    }
    Err("NSPasteboard writeObjects failed".to_string())
}

pub fn restore(snapshot: ClipboardSnapshot) {
    if snapshot.items.is_empty() {
        NSPasteboard::generalPasteboard().clearContents();
        return;
    }
    write_tagged(&snapshot.items);
}

/// Replace the pasteboard with `items`, each tagged transient so history
/// managers skip it. `clearContents` + `writeObjects` is one `changeCount`
/// bump per call, and the transient marker makes managers ignore that bump.
fn write_tagged(items: &[Vec<(String, Vec<u8>)>]) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    let transient_type = NSString::from_str(TRANSIENT_TYPE);
    let pasteboard_items: Vec<Retained<NSPasteboardItem>> = items
        .iter()
        .map(|pairs| {
            let item = NSPasteboardItem::new();
            for (item_type, data) in pairs {
                item.setData_forType(&NSData::with_bytes(data), &NSString::from_str(item_type));
            }
            item.setData_forType(&NSData::with_bytes(MARKER_OWNER), &transient_type);
            item
        })
        .collect();
    let writable: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = pasteboard_items
        .iter()
        .map(|item| ProtocolObject::from_ref(&**item))
        .collect();
    pasteboard.writeObjects(&NSArray::from_slice(&writable))
}
