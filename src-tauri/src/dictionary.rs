use crate::config::DictionaryEntry;

/// Punctuation whose replacement should glue to both neighbors with no spaces.
/// Example: "test dot ts" → "test.ts".
const COMPACT: &[char] = &['.', '/', '-', '_', '@'];
/// Punctuation whose replacement should lose the leading space but keep a
/// trailing one. Example: "hello comma world" → "hello, world".
const CLING_LEFT: &[char] = &[',', ';', ':', '?', '!'];

/// Case-insensitive whole-word replacement with a small spacing policy. We
/// pad the whole transcript with spaces on both ends, then search for `from`
/// flanked by non-word characters — so Deepgram's terminal punctuation
/// ("Design skill.", "design skill,") doesn't kill the right-hand boundary
/// the way a literal " from " search would. The replacement is spliced in
/// with surrounding spaces and collapsed by the phase-2 punctuation passes
/// below. The outer loop re-runs replacements until stable so chains like
/// "dash dash help" fully resolve to "--help".
pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
    if entries.is_empty() {
        return text.to_string();
    }

    let mut padded = format!(" {} ", text);
    let froms_lc: Vec<String> = entries
        .iter()
        .map(|e| e.from.to_lowercase())
        .collect();

    loop {
        let lower = padded.to_lowercase();
        let mut changed = false;
        for (e, from_lc) in entries.iter().zip(froms_lc.iter()) {
            if from_lc.is_empty() {
                continue;
            }
            if let Some((start, end)) = find_word_match(&lower, from_lc) {
                let replacement = format!(" {} ", e.to);
                padded.replace_range(start..end, &replacement);
                changed = true;
                // Restart the scan from the top — replacement may have
                // exposed a new match earlier in the string.
                break;
            }
        }
        if !changed {
            break;
        }
    }

    // Phase 2: compact / cling-left spacing for punctuation.
    for &c in COMPACT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        padded = padded.replace(&middle, &c.to_string());
        padded = padded.replace(&tail, &c.to_string());
    }
    for &c in CLING_LEFT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        padded = padded.replace(&middle, &format!("{c} "));
        padded = padded.replace(&tail, &c.to_string());
    }

    // Collapse any runs of spaces that survived the passes above.
    padded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Chars that count as part of a word for replacement boundary purposes.
/// Hyphen/underscore/apostrophe are included so a rule like "well" doesn't
/// match inside "well-being" or "don't".
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-' || c == '_'
}

/// Locate `needle` inside `haystack` as a whole word — flanked by non-word
/// characters or the string boundary. Both inputs are expected to already be
/// lowercased; byte offsets are returned for direct use with
/// `replace_range` on a same-length original-case string (ASCII-safe).
fn find_word_match(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let left_ok = haystack[..start]
            .chars()
            .next_back()
            .map_or(true, |c| !is_word_char(c));
        let right_ok = haystack[end..]
            .chars()
            .next()
            .map_or(true, |c| !is_word_char(c));
        if left_ok && right_ok {
            return Some((start, end));
        }
        from = start + needle.chars().next().map_or(1, |c| c.len_utf8());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(from: &str, to: &str) -> DictionaryEntry {
        DictionaryEntry {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn matches_with_trailing_period() {
        let out = apply_dictionary(
            "Let me improve my design skill.",
            &[entry("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Let me improve my /emil-design-engineering.");
    }

    #[test]
    fn matches_with_trailing_comma() {
        let out = apply_dictionary(
            "I rely on my design skill, every day.",
            &[entry("design skill", "/emil-design-engineering")],
        );
        assert_eq!(
            out,
            "I rely on my /emil-design-engineering, every day."
        );
    }

    #[test]
    fn matches_with_trailing_question_mark() {
        let out = apply_dictionary(
            "Want to use my design skill?",
            &[entry("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Want to use my /emil-design-engineering?");
    }

    #[test]
    fn does_not_match_inside_word() {
        let out = apply_dictionary("I love design skills", &[entry("design skill", "X")]);
        assert_eq!(out, "I love design skills");
    }

    #[test]
    fn does_not_match_with_hyphen_boundary() {
        let out = apply_dictionary("well-being matters", &[entry("well", "good")]);
        assert_eq!(out, "well-being matters");
    }

    #[test]
    fn case_insensitive_match() {
        let out = apply_dictionary("Design Skill rules.", &[entry("design skill", "X")]);
        assert_eq!(out, "X rules.");
    }

    #[test]
    fn pipeline_applies_dictionary_after_cleanup_stub() {
        // Simulated cleanup pass: identity function returns input unchanged.
        let after_cleanup = "I prefer Mongo";
        let entries = [entry("Mongo", "MongoDB")];
        let final_text = apply_dictionary(after_cleanup, &entries);
        assert_eq!(final_text, "I prefer MongoDB");
    }
}
