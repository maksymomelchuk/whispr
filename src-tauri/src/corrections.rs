use crate::config::CorrectionEntry;

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
/// "dash dash help" fully resolve to "--help". MAX_PASSES caps it so a rule
/// whose `to` (case-folded) contains its `from` can't pin the pipeline.
pub fn apply_corrections(text: &str, entries: &[CorrectionEntry]) -> String {
    if entries.is_empty() {
        return text.to_string();
    }

    let mut padded = format!(" {} ", text);
    // Drop pure case-folded identity rules ("getmany" → "Getmany"): each
    // pass would re-find the same whole-word match and re-wrap it in spaces,
    // a true infinite loop with no progress toward the cap below.
    let active: Vec<(&CorrectionEntry, String)> = entries
        .iter()
        .filter_map(|e| {
            let from_lc = e.from.to_lowercase();
            if from_lc.is_empty() || e.to.to_lowercase() == from_lc {
                None
            } else {
                Some((e, from_lc))
            }
        })
        .collect();

    const MAX_PASSES: usize = 32;
    for _ in 0..MAX_PASSES {
        let lower = padded.to_lowercase();
        let mut changed = false;
        for (e, from_lc) in &active {
            if let Some((start, end)) = find_word_match(&lower, from_lc, 0) {
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
        // head strips trailing space so chained replacements glue to the
        // right neighbor too: "  -   -  help " → "-- help " → "--help ".
        let head = format!("{} ", c);
        padded = padded.replace(&middle, &c.to_string());
        padded = padded.replace(&tail, &c.to_string());
        padded = padded.replace(&head, &c.to_string());
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
/// `from` is the byte offset to start searching from (pass 0 to search the
/// full string).
pub fn find_word_match(haystack: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let mut pos = from;
    while let Some(rel) = haystack[pos..].find(needle) {
        let start = pos + rel;
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
        pos = start + needle.chars().next().map_or(1, |c| c.len_utf8());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn entry(from: &str, to: &str) -> CorrectionEntry {
        CorrectionEntry {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn matches_with_trailing_period() {
        let out = apply_corrections(
            "Let me improve my design skill.",
            &[entry("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Let me improve my /emil-design-engineering.");
    }

    #[test]
    fn matches_with_trailing_comma() {
        let out = apply_corrections(
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
        let out = apply_corrections(
            "Want to use my design skill?",
            &[entry("design skill", "/emil-design-engineering")],
        );
        assert_eq!(out, "Want to use my /emil-design-engineering?");
    }

    #[test]
    fn does_not_match_inside_word() {
        let out = apply_corrections("I love design skills", &[entry("design skill", "X")]);
        assert_eq!(out, "I love design skills");
    }

    #[test]
    fn does_not_match_with_hyphen_boundary() {
        let out = apply_corrections("well-being matters", &[entry("well", "good")]);
        assert_eq!(out, "well-being matters");
    }

    #[test]
    fn case_insensitive_match() {
        let out = apply_corrections("Design Skill rules.", &[entry("design skill", "X")]);
        assert_eq!(out, "X rules.");
    }

    #[test]
    fn pipeline_applies_corrections_after_cleanup_stub() {
        let after_cleanup = "I prefer Mongo";
        let entries = [entry("Mongo", "MongoDB")];
        let final_text = apply_corrections(after_cleanup, &entries);
        assert_eq!(final_text, "I prefer MongoDB");
    }

    #[test]
    fn punctuation_cue_applies_as_correction() {
        // Verbal punctuation still works as a correction rule.
        let out = apply_corrections("test dot ts", &[entry("dot", ".")]);
        assert_eq!(out, "test.ts");
    }

    #[test]
    fn case_folded_identity_rule_does_not_hang() {
        // "getmany" → "Getmany" is a case-folded identity: the lowercased
        // replacement still matches the lowercased pattern, which previously
        // pinned apply_corrections in an infinite outer loop.
        let out = apply_corrections(
            "I love Getmany.",
            &[entry("getmany", "Getmany")],
        );
        assert_eq!(out, "I love Getmany.");
    }

    #[test]
    fn rule_whose_replacement_contains_pattern_terminates() {
        // "abc" → "abc def" — the replacement re-contains the pattern as a
        // whole word, so naive re-scan loops forever. MAX_PASSES caps it;
        // here we just need termination with a sensible result.
        let out = apply_corrections("hello abc world", &[entry("abc", "abc def")]);
        assert!(out.contains("def"), "replacement should apply at least once: {out}");
    }

    #[test]
    fn chained_corrections_resolve() {
        let out = apply_corrections(
            "dash dash help",
            &[entry("dash", "-")],
        );
        assert_eq!(out, "--help");
    }

    proptest! {
        #[test]
        fn apply_corrections_terminates_and_is_valid_utf8(
            entries in proptest::collection::vec(
                ("[a-z]{1,10}", "[a-z._/-]{0,15}").prop_map(|(from, to)| CorrectionEntry { from, to }),
                0..8
            ),
            text in "[a-z ]{0,100}"
        ) {
            let result = apply_corrections(&text, &entries);
            prop_assert!(std::str::from_utf8(result.as_bytes()).is_ok());
        }

        #[test]
        fn find_word_match_slice_roundtrip(
            haystack in "[a-z ]{0,50}",
            needle in "[a-z]{1,10}",
            from in 0usize..51usize
        ) {
            let from_clamped = from.min(haystack.len());
            let is_wc = |c: char| c.is_alphanumeric() || c == '\'' || c == '-' || c == '_';
            if let Some((start, end)) = find_word_match(&haystack, &needle, from_clamped) {
                prop_assert_eq!(&haystack[start..end], needle.as_str());
                let left_ok = haystack[..start].chars().next_back().map_or(true, |c| !is_wc(c));
                let right_ok = haystack[end..].chars().next().map_or(true, |c| !is_wc(c));
                prop_assert!(left_ok, "left boundary violated: {:?}", &haystack[..start]);
                prop_assert!(right_ok, "right boundary violated: {:?}", &haystack[end..]);
            }
        }
    }
}
