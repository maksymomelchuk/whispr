use crate::config::DictionaryEntry;

/// Punctuation whose replacement should glue to both neighbors with no spaces.
/// Example: "test dot ts" → "test.ts".
const COMPACT: &[char] = &['.', '/', '-', '_', '@'];
/// Punctuation whose replacement should lose the leading space but keep a
/// trailing one. Example: "hello comma world" → "hello, world".
const CLING_LEFT: &[char] = &[',', ';', ':', '?', '!'];

/// 4 KB ceiling — Deepgram's documented maximum is ~8 KB; halving gives
/// comfortable headroom for the base URL and engine parameters that precede
/// any keyterm pairs.
pub const DEEPGRAM_KEYTERM_BUDGET_BYTES: usize = 4096;

/// Whisper's prompt parameter is documented as up to 224 tokens; 800 chars
/// stays well within that window for typical English vocabulary lists.
pub const GROQ_PROMPT_BUDGET_CHARS: usize = 800;

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
/// `from` is the byte offset to start searching from (pass 0 to search the full string).
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

/// Returns true if `entry` is a punctuation-cue that should be excluded
/// from engine prompt hints. Sending these biases the engine on noise rather
/// than real vocabulary words.
pub fn is_punctuation_cue(entry: &DictionaryEntry) -> bool {
    let to = entry.to.trim();
    to.len() < 3 && to.chars().all(|c| c.is_ascii_punctuation())
}

/// Yields trimmed `from` terms that are eligible to send as engine hints —
/// drops punctuation-cue and blank-`from` entries. Preserves insertion order.
fn eligible_terms(entries: &[DictionaryEntry]) -> impl Iterator<Item = &str> {
    entries.iter().filter_map(|entry| {
        if is_punctuation_cue(entry) {
            return None;
        }
        let term = entry.from.trim();
        (!term.is_empty()).then_some(term)
    })
}

/// Returns the `from` terms suitable for Deepgram `keyterm` query params.
/// Filters out punctuation cues and blank entries; truncates in insertion
/// order once the next term would push the consumed bytes past
/// `remaining_budget`. Pass `DEEPGRAM_KEYTERM_BUDGET_BYTES - url_base_len`
/// as the budget so the final URL stays within the 4 KB ceiling.
pub fn deepgram_keyterms(entries: &[DictionaryEntry], remaining_budget: usize) -> Vec<String> {
    // Size each encoded value the way `url::Url::query_pairs_mut().append_pair`
    // will, so the running total matches the bytes the final URL actually gains.
    const KEY_PREFIX_BYTES: usize = "&keyterm=".len();
    let mut terms = Vec::new();
    let mut used = 0usize;
    for term in eligible_terms(entries) {
        let encoded: String = url::form_urlencoded::byte_serialize(term.as_bytes()).collect();
        let needed = KEY_PREFIX_BYTES + encoded.len();
        if used + needed > remaining_budget {
            break;
        }
        terms.push(term.to_string());
        used += needed;
    }
    terms
}

/// Builds the Groq prompt hint (`"Vocabulary: t1, t2, t3"`) from dictionary
/// entries. Filters punctuation cues; truncates at a comma boundary so the
/// result stays within `GROQ_PROMPT_BUDGET_CHARS`. Returns `None` when no
/// eligible entries exist.
pub fn groq_prompt_hint(entries: &[DictionaryEntry]) -> Option<String> {
    const PREFIX: &str = "Vocabulary: ";
    let mut result = PREFIX.to_string();
    for term in eligible_terms(entries) {
        let sep = if result.len() == PREFIX.len() { "" } else { ", " };
        if result.len() + sep.len() + term.len() > GROQ_PROMPT_BUDGET_CHARS {
            break;
        }
        result.push_str(sep);
        result.push_str(term);
    }
    (result.len() > PREFIX.len()).then_some(result)
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

    // ── is_punctuation_cue ──────────────────────────────────────────────

    #[test]
    fn punctuation_cue_single_char_punct() {
        assert!(is_punctuation_cue(&entry("dot", ".")));
        assert!(is_punctuation_cue(&entry("slash", "/")));
        assert!(is_punctuation_cue(&entry("comma", ",")));
        assert!(is_punctuation_cue(&entry("question mark", "?")));
        assert!(is_punctuation_cue(&entry("exclamation mark", "!")));
    }

    #[test]
    fn punctuation_cue_two_char_punct() {
        assert!(is_punctuation_cue(&entry("double dash", "--")));
    }

    #[test]
    fn not_punctuation_cue_real_word() {
        assert!(!is_punctuation_cue(&entry("MongoDB", "MongoDB")));
        assert!(!is_punctuation_cue(&entry("TypeScript", "TypeScript")));
    }

    #[test]
    fn not_punctuation_cue_three_char_punct_to() {
        // to.len() == 3 → does not meet < 3 condition
        assert!(!is_punctuation_cue(&entry("ellipsis", "...")));
    }

    #[test]
    fn not_punctuation_cue_mixed_to() {
        // 'y' is not ascii_punctuation
        assert!(!is_punctuation_cue(&entry("something", "y")));
    }

    // ── deepgram_keyterms ───────────────────────────────────────────────

    #[test]
    fn keyterms_filters_punctuation_cues() {
        let entries = vec![
            entry("dot", "."),
            entry("MongoDB", "MongoDB"),
            entry("slash", "/"),
            entry("TypeScript", "TypeScript"),
        ];
        let terms = deepgram_keyterms(&entries, 4096);
        assert_eq!(terms, vec!["MongoDB", "TypeScript"]);
    }

    #[test]
    fn keyterms_empty_entries() {
        assert!(deepgram_keyterms(&[], 4096).is_empty());
    }

    #[test]
    fn keyterms_all_punctuation_returns_empty() {
        let entries = vec![entry("dot", "."), entry("slash", "/"), entry("comma", ",")];
        assert!(deepgram_keyterms(&entries, 4096).is_empty());
    }

    #[test]
    fn keyterms_truncates_at_budget() {
        // "alpha" → encoded "alpha" (5 bytes); needed = 9+5 = 14.
        // "beta"  → encoded "beta"  (4 bytes); needed = 9+4 = 13.
        // Budget 25: alpha (14) fits, beta (14+13=27 > 25) truncated.
        let entries = vec![entry("alpha", "alpha"), entry("beta", "beta")];
        let terms = deepgram_keyterms(&entries, 25);
        assert_eq!(terms, vec!["alpha"]);
    }

    #[test]
    fn keyterms_zero_budget_returns_empty() {
        let entries = vec![entry("MongoDB", "MongoDB")];
        assert!(deepgram_keyterms(&entries, 0).is_empty());
    }

    #[test]
    fn keyterms_exact_budget_fit() {
        // "hi" → 2 bytes; needed = 9+2 = 11.
        // Budget 11 → exactly fits one term.
        let entries = vec![entry("hi", "hi"), entry("bye", "bye")];
        let terms = deepgram_keyterms(&entries, 11);
        assert_eq!(terms, vec!["hi"]);
    }

    #[test]
    fn keyterms_skips_blank_from() {
        let entries = vec![entry("  ", "something"), entry("MongoDB", "MongoDB")];
        let terms = deepgram_keyterms(&entries, 4096);
        assert_eq!(terms, vec!["MongoDB"]);
    }

    // ── groq_prompt_hint ────────────────────────────────────────────────

    #[test]
    fn prompt_hint_formats_correctly() {
        let entries = vec![
            entry("MongoDB", "MongoDB"),
            entry("TypeScript", "TypeScript"),
            entry("Kubernetes", "Kubernetes"),
        ];
        assert_eq!(
            groq_prompt_hint(&entries).unwrap(),
            "Vocabulary: MongoDB, TypeScript, Kubernetes"
        );
    }

    #[test]
    fn prompt_hint_filters_punctuation_cues() {
        let entries = vec![entry("dot", "."), entry("MongoDB", "MongoDB"), entry("slash", "/")];
        assert_eq!(groq_prompt_hint(&entries).unwrap(), "Vocabulary: MongoDB");
    }

    #[test]
    fn prompt_hint_returns_none_for_empty() {
        assert!(groq_prompt_hint(&[]).is_none());
    }

    #[test]
    fn prompt_hint_returns_none_for_all_punctuation() {
        let entries = vec![entry("dot", "."), entry("comma", ",")];
        assert!(groq_prompt_hint(&entries).is_none());
    }

    #[test]
    fn prompt_hint_truncates_at_budget() {
        // "Vocabulary: " = 12 chars. Budget = 800.
        // filler (785 chars): result becomes 797 chars.
        // ", yy" would add 4 chars → 801 > 800 → truncated.
        let filler = "x".repeat(785);
        let entries = vec![entry(&filler, &filler), entry("yy", "yy")];
        let hint = groq_prompt_hint(&entries).unwrap();
        assert_eq!(hint.len(), 797);
        assert!(!hint.contains("yy"));
    }

    #[test]
    fn prompt_hint_includes_term_that_exactly_fits() {
        // "Vocabulary: " = 12 chars. 785-char filler → 797 chars.
        // ", y" = 3 chars → 800 exactly ≤ 800 → included.
        let filler = "x".repeat(785);
        let entries = vec![entry(&filler, &filler), entry("y", "y")];
        let hint = groq_prompt_hint(&entries).unwrap();
        assert_eq!(hint.len(), 800);
        assert!(hint.ends_with(", y"));
    }

    #[test]
    fn prompt_hint_single_term_no_comma() {
        let entries = vec![entry("MongoDB", "MongoDB")];
        assert_eq!(groq_prompt_hint(&entries).unwrap(), "Vocabulary: MongoDB");
    }
}
