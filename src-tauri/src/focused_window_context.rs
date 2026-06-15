use tauri::Manager;

const CAPTURE_TIMEOUT_SECS: f32 = 0.2;
const MAX_NODES: usize = 4000;
const MAX_DEPTH: u32 = 80;
const MAX_CHARS: usize = 4000;

pub fn capture(app: tauri::AppHandle) {
    use crate::state::AppState;
    let (tx, rx) = tokio::sync::oneshot::channel();
    *app.state::<AppState>()
        .pending_focused_window_rx
        .lock()
        .unwrap() = Some(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = tx.send(platform_read_focused_window());
    });
}

/// Joins text fragments into one block, dropping blanks and consecutive
/// duplicates, capped at `max_chars`. Pure so cap/dedup is testable without an
/// accessibility tree. Used by the native (non-web) traversal fallback.
fn assemble(fragments: Vec<String>, max_chars: usize) -> Option<String> {
    let mut out = String::new();
    let mut char_count = 0;
    let mut last: Option<&str> = None;
    for fragment in &fragments {
        let trimmed = fragment.trim();
        if trimmed.is_empty() || last == Some(trimmed) {
            continue;
        }
        let separator = usize::from(!out.is_empty());
        if char_count + trimmed.chars().count() + separator > max_chars {
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(trimmed);
        char_count += trimmed.chars().count() + separator;
        last = Some(trimmed);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Trims to `max_chars` on a char boundary so a long document doesn't blow the
/// token budget. Pure.
fn cap_text(text: String, max_chars: usize) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= max_chars {
        return Some(trimmed.to_string());
    }
    Some(trimmed.chars().take(max_chars).collect())
}

/// Slices a `max_chars`-wide window from `text` centered on `center_fraction`
/// (0.0 = start, 1.0 = end), clamped so the window stays within bounds. Returns
/// the whole text when it already fits. Pure so the viewport→character mapping
/// is testable without an accessibility tree.
fn char_window(text: &str, center_fraction: f64, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if total <= max_chars {
        return text.to_string();
    }
    let center = (center_fraction.clamp(0.0, 1.0) * total as f64) as usize;
    let half = max_chars / 2;
    let lo = center.saturating_sub(half);
    let hi = (lo + max_chars).min(total);
    let lo = hi.saturating_sub(max_chars);
    chars[lo..hi].iter().collect()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn platform_read_focused_window() -> Option<String> {
    use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
    use core_foundation::base::{CFGetTypeID, CFRelease, CFRetain, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};
    use std::os::raw::c_void;
    use std::ptr;

    type AXUIElementRef = CFTypeRef;
    type AXError = i32;
    const AX_ERROR_SUCCESS: AXError = 0;
    const AX_SECURE_TEXT_FIELD_ROLE: &str = "AXSecureTextField";
    const AX_WEB_AREA_ROLE: &str = "AXWebArea";
    const AX_VALUE_CG_POINT: u32 = 1;
    const AX_VALUE_CG_SIZE: u32 = 2;
    const AX_VALUE_CG_RECT: u32 = 3;
    const WEB_AREA_SEARCH_BUDGET: usize = 3000;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementCopyParameterizedAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            parameter: CFTypeRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, seconds: f32) -> AXError;
        fn AXValueGetValue(value: CFTypeRef, the_type: u32, value_ptr: *mut c_void) -> u8;
        fn AXTextMarkerRangeCreate(
            allocator: CFTypeRef,
            start: CFTypeRef,
            end: CFTypeRef,
        ) -> CFTypeRef;
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct CgPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct CgSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct CgRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    #[derive(Clone, Copy)]
    struct Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    impl Rect {
        fn overlaps(&self, other: &Rect) -> bool {
            self.x < other.x + other.w
                && self.x + self.w > other.x
                && self.y < other.y + other.h
                && self.y + self.h > other.y
        }
    }

    unsafe fn copy_attr(element: CFTypeRef, attr: &CFString) -> Option<CFTypeRef> {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value);
        if err != AX_ERROR_SUCCESS || value.is_null() {
            None
        } else {
            Some(value)
        }
    }

    unsafe fn copy_param(
        element: CFTypeRef,
        attr: &CFString,
        param: CFTypeRef,
    ) -> Option<CFTypeRef> {
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyParameterizedAttributeValue(
            element,
            attr.as_concrete_TypeRef(),
            param,
            &mut value,
        );
        if err != AX_ERROR_SUCCESS || value.is_null() {
            None
        } else {
            Some(value)
        }
    }

    // Consumes `value` (a create-rule ref). Returns the string or releases and
    // returns None when it isn't a CFString.
    unsafe fn take_string(value: CFTypeRef) -> Option<String> {
        if CFGetTypeID(value) != CFStringGetTypeID() {
            CFRelease(value);
            return None;
        }
        Some(CFString::wrap_under_create_rule(value as CFStringRef).to_string())
    }

    unsafe fn read_rect(element: CFTypeRef, pos: &CFString, size: &CFString) -> Option<Rect> {
        let pv = copy_attr(element, pos)?;
        let mut point = CgPoint::default();
        let ok_p = AXValueGetValue(pv, AX_VALUE_CG_POINT, &mut point as *mut _ as *mut c_void);
        CFRelease(pv);
        if ok_p == 0 {
            return None;
        }
        let sv = copy_attr(element, size)?;
        let mut sz = CgSize::default();
        let ok_s = AXValueGetValue(sv, AX_VALUE_CG_SIZE, &mut sz as *mut _ as *mut c_void);
        CFRelease(sv);
        if ok_s == 0 {
            return None;
        }
        Some(Rect {
            x: point.x,
            y: point.y,
            w: sz.width,
            h: sz.height,
        })
    }

    // Breadth-first hunt for the first AXWebArea. Returns a retained ref the
    // caller must release. Leaves `root` untouched.
    unsafe fn find_web_area(
        root: CFTypeRef,
        role: &CFString,
        children: &CFString,
    ) -> Option<CFTypeRef> {
        use std::collections::VecDeque;
        CFRetain(root);
        let mut queue: VecDeque<CFTypeRef> = VecDeque::from([root]);
        let mut found: Option<CFTypeRef> = None;
        let mut seen = 0usize;
        while let Some(element) = queue.pop_front() {
            seen += 1;
            if found.is_none() && seen <= WEB_AREA_SEARCH_BUDGET {
                let is_web = copy_attr(element, role)
                    .and_then(|r| take_string(r))
                    .as_deref()
                    == Some(AX_WEB_AREA_ROLE);
                if is_web {
                    CFRetain(element);
                    found = Some(element);
                } else if let Some(kids) = copy_attr(element, children) {
                    let array = kids as CFArrayRef;
                    let count = CFArrayGetCount(array);
                    for i in 0..count {
                        let child = CFArrayGetValueAtIndex(array, i) as CFTypeRef;
                        if !child.is_null() {
                            CFRetain(child);
                            queue.push_back(child);
                        }
                    }
                    CFRelease(kids);
                }
            }
            CFRelease(element);
        }
        found
    }

    struct WebAttrs {
        start: CFString,
        end: CFString,
        string_for_marker_range: CFString,
        bounds_for_range: CFString,
        position: CFString,
        size: CFString,
    }

    impl WebAttrs {
        fn new() -> Self {
            WebAttrs {
                start: CFString::from_static_string("AXStartTextMarker"),
                end: CFString::from_static_string("AXEndTextMarker"),
                string_for_marker_range: CFString::from_static_string("AXStringForTextMarkerRange"),
                bounds_for_range: CFString::from_static_string("AXBoundsForTextMarkerRange"),
                position: CFString::from_static_string("AXPosition"),
                size: CFString::from_static_string("AXSize"),
            }
        }
    }

    // Pulls text from a web area: tries the viewport-scoped range first, then
    // falls back to the whole document. Both return text in reading order and
    // include inline content — which node-by-node AXStaticText walking drops.
    unsafe fn web_text(web: CFTypeRef, attrs: &WebAttrs) -> Option<String> {
        visible_range_text(web, attrs).or_else(|| full_document_text(web, attrs))
    }

    unsafe fn full_document_text(web: CFTypeRef, attrs: &WebAttrs) -> Option<String> {
        let start = copy_attr(web, &attrs.start)?;
        let end = copy_attr(web, &attrs.end)?;
        let range = AXTextMarkerRangeCreate(ptr::null(), start, end);
        CFRelease(start);
        CFRelease(end);
        if range.is_null() {
            return None;
        }
        let text =
            copy_param(web, &attrs.string_for_marker_range, range).and_then(|v| take_string(v));
        CFRelease(range);
        text.filter(|t| !t.trim().is_empty())
    }

    unsafe fn range_bounds(web: CFTypeRef, attrs: &WebAttrs, range: CFTypeRef) -> Option<Rect> {
        let bounds = copy_param(web, &attrs.bounds_for_range, range)?;
        let mut rect = CgRect::default();
        let ok = AXValueGetValue(bounds, AX_VALUE_CG_RECT, &mut rect as *mut _ as *mut c_void);
        CFRelease(bounds);
        if ok == 0 {
            None
        } else {
            Some(Rect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            })
        }
    }

    // Chromium gives real geometry only for the whole-document marker range;
    // any range built from index markers collapses to the viewport rect, and
    // bounds→marker / point→marker are stubbed. But text-by-global-index works,
    // so the visible slice is found arithmetically: the document's full rect and
    // the web area's viewport rect pin down the vertical fraction on screen, and
    // mapping that fraction onto the character count centers a budget-sized
    // window on the visible region.
    unsafe fn visible_range_text(web: CFTypeRef, attrs: &WebAttrs) -> Option<String> {
        let viewport = read_rect(web, &attrs.position, &attrs.size)?;
        let start = copy_attr(web, &attrs.start)?;
        let end = copy_attr(web, &attrs.end)?;
        let doc_range = AXTextMarkerRangeCreate(ptr::null(), start, end);
        CFRelease(start);
        CFRelease(end);
        if doc_range.is_null() {
            return None;
        }
        let doc_rect = range_bounds(web, attrs, doc_range);
        let full =
            copy_param(web, &attrs.string_for_marker_range, doc_range).and_then(|v| take_string(v));
        CFRelease(doc_range);
        let full = full?;
        let doc_rect = doc_rect?;

        let center_fraction = if doc_rect.h <= viewport.h {
            0.5
        } else {
            ((viewport.y + viewport.h * 0.5 - doc_rect.y) / doc_rect.h).clamp(0.0, 1.0)
        };
        let window = char_window(&full, center_fraction, MAX_CHARS);
        if window.trim().is_empty() {
            None
        } else {
            Some(window)
        }
    }

    // Non-web fallback; web content goes through web_text instead.
    unsafe fn native_window_text(
        root: CFTypeRef,
        role: &CFString,
        value_attr: &CFString,
        title: &CFString,
        children: &CFString,
        pos: &CFString,
        size: &CFString,
    ) -> Option<String> {
        let viewport = read_rect(root, pos, size);
        let root_y = viewport.map(|v| v.y).unwrap_or(0.0);
        CFRetain(root);
        let mut stack: Vec<(CFTypeRef, u32, f64)> = vec![(root, 0, root_y)];
        let mut fragments: Vec<(f64, String)> = Vec::new();
        let mut nodes = 0usize;

        while let Some((element, depth, inherited_y)) = stack.pop() {
            nodes += 1;
            if nodes > MAX_NODES {
                CFRelease(element);
                break;
            }

            let role_name = copy_attr(element, role).and_then(|r| take_string(r));
            if role_name.as_deref() == Some(AX_SECURE_TEXT_FIELD_ROLE) {
                CFRelease(element);
                continue;
            }

            let rect = read_rect(element, pos, size);
            let own_y = rect.map(|r| r.y).unwrap_or(inherited_y);

            let text = copy_attr(element, value_attr)
                .and_then(|v| take_string(v))
                .or_else(|| {
                    if role_name.as_deref() == Some("AXStaticText") {
                        copy_attr(element, title).and_then(|v| take_string(v))
                    } else {
                        None
                    }
                });
            if let Some(text) = text {
                let on_screen = match (viewport, rect) {
                    (Some(win), Some(node)) => node.overlaps(&win),
                    (Some(win), None) => own_y >= win.y && own_y <= win.y + win.h,
                    (None, _) => true,
                };
                if on_screen {
                    fragments.push((own_y, text));
                }
            }

            if depth < MAX_DEPTH {
                if let Some(kids) = copy_attr(element, children) {
                    let array = kids as CFArrayRef;
                    let count = CFArrayGetCount(array);
                    for i in 0..count {
                        let child = CFArrayGetValueAtIndex(array, i) as CFTypeRef;
                        if !child.is_null() {
                            CFRetain(child);
                            stack.push((child, depth + 1, own_y));
                        }
                    }
                    CFRelease(kids);
                }
            }
            CFRelease(element);
        }
        for (element, _, _) in stack {
            CFRelease(element);
        }

        fragments.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        assemble(fragments.into_iter().map(|(_, t)| t).collect(), MAX_CHARS)
    }

    if crate::platform::is_secure_input_active() {
        return None;
    }

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(system_wide, CAPTURE_TIMEOUT_SECS);

        let focused_attr = CFString::from_static_string("AXFocusedUIElement");
        let Some(focused_element) = copy_attr(system_wide, &focused_attr) else {
            CFRelease(system_wide);
            return None;
        };
        CFRelease(system_wide);

        let window_attr = CFString::from_static_string("AXWindow");
        let root = match copy_attr(focused_element, &window_attr) {
            Some(window) => {
                CFRelease(focused_element);
                window
            }
            None => focused_element,
        };

        let role_attr = CFString::from_static_string("AXRole");
        let value_attr = CFString::from_static_string("AXValue");
        let title_attr = CFString::from_static_string("AXTitle");
        let children_attr = CFString::from_static_string("AXChildren");
        let position_attr = CFString::from_static_string("AXPosition");
        let size_attr = CFString::from_static_string("AXSize");

        let result = match find_web_area(root, &role_attr, &children_attr) {
            Some(web) => {
                let text = web_text(web, &WebAttrs::new());
                CFRelease(web);
                text
            }
            None => native_window_text(
                root,
                &role_attr,
                &value_attr,
                &title_attr,
                &children_attr,
                &position_attr,
                &size_attr,
            ),
        };

        CFRelease(root);
        result.and_then(|t| cap_text(t, MAX_CHARS))
    }
}

// ── Windows / Linux (not yet implemented) ───────────────────────────────────────

#[cfg(not(target_os = "macos"))]
fn platform_read_focused_window() -> Option<String> {
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_drops_blanks_and_consecutive_duplicates() {
        let fragments = vec![
            "  Naxulith  ".to_string(),
            "".to_string(),
            "Naxulith".to_string(),
            "scheduler".to_string(),
        ];
        assert_eq!(
            assemble(fragments, 4000),
            Some("Naxulith\nscheduler".to_string())
        );
    }

    #[test]
    fn assemble_empty_returns_none() {
        assert_eq!(assemble(vec!["  ".to_string(), "".to_string()], 4000), None);
    }

    #[test]
    fn assemble_stops_at_char_cap() {
        let fragments = vec!["aaaa".to_string(), "bbbb".to_string(), "cccc".to_string()];
        assert_eq!(assemble(fragments, 9), Some("aaaa\nbbbb".to_string()));
    }

    #[test]
    fn cap_text_returns_none_for_blank() {
        assert_eq!(cap_text("   ".to_string(), 100), None);
    }

    #[test]
    fn cap_text_passes_short_text_through_trimmed() {
        assert_eq!(
            cap_text("  hello  ".to_string(), 100),
            Some("hello".to_string())
        );
    }

    #[test]
    fn cap_text_caps_by_characters_not_bytes() {
        // "aéb" is 3 chars / 4 bytes. A char cap of 2 keeps the first two chars;
        // a byte cap would have stopped after "a".
        assert_eq!(cap_text("aéb".to_string(), 2), Some("aé".to_string()));
    }

    #[test]
    fn assemble_counts_multibyte_chars_not_bytes() {
        // Each Cyrillic char is 2 bytes; a byte budget would drop the second
        // fragment that a 9-char budget comfortably fits.
        let fragments = vec!["привіт".to_string(), "світ".to_string()];
        assert_eq!(assemble(fragments, 11), Some("привіт\nсвіт".to_string()));
    }

    #[test]
    fn char_window_returns_whole_text_when_it_fits() {
        assert_eq!(char_window("short document", 0.5, 100), "short document");
    }

    #[test]
    fn char_window_centers_on_fraction() {
        let text: String = ('a'..='z').collect(); // 26 chars
                                                  // fraction 0.5 → center 13, half 2 → chars [11, 15) = "lmno"
        assert_eq!(char_window(&text, 0.5, 4), "lmno");
    }

    #[test]
    fn char_window_clamps_window_to_end() {
        let text: String = ('a'..='z').collect();
        // fraction 1.0 → center 26, clamped so the window is the last 4 chars
        assert_eq!(char_window(&text, 1.0, 4), "wxyz");
    }

    #[test]
    fn char_window_clamps_window_to_start() {
        let text: String = ('a'..='z').collect();
        assert_eq!(char_window(&text, 0.0, 4), "abcd");
    }
}
