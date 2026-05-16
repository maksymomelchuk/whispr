use crate::config::Replacement;

// 4 KB ceiling — Deepgram's documented maximum is ~8 KB; halving gives
// comfortable headroom for the base URL and engine parameters that precede
// any keyterm pairs.
pub const DEEPGRAM_KEYTERM_BUDGET_BYTES: usize = 4096;

// Whisper's prompt parameter is documented as up to 224 tokens; 800 chars
// stays well within that window for typical English vocabulary lists.
pub const GROQ_PROMPT_BUDGET_CHARS: usize = 800;

/// Returns true if `entry` is a punctuation-cue that should be excluded
/// from engine prompt hints. Sending these biases the engine on noise rather
/// than real vocabulary words.
pub fn is_punctuation_cue(entry: &Replacement) -> bool {
    let to = entry.to.trim();
    to.len() < 3 && to.chars().all(|c| c.is_ascii_punctuation())
}

/// Returns the `from` terms suitable for Deepgram `keyterm` query params.
/// Filters out punctuation cues and blank entries; truncates in insertion
/// order once the next term would push the consumed bytes past
/// `remaining_budget`. Pass `DEEPGRAM_KEYTERM_BUDGET_BYTES - url_base_len`
/// as the budget so the final URL stays within the 4 KB ceiling.
pub fn deepgram_keyterms(replacements: &[Replacement], remaining_budget: usize) -> Vec<String> {
    let mut terms = Vec::new();
    let mut used = 0usize;
    for entry in replacements {
        if is_punctuation_cue(entry) {
            continue;
        }
        let term = entry.from.trim();
        if term.is_empty() {
            continue;
        }
        // "&keyterm=" = 9 bytes; use form_urlencoded to size the encoded value
        // the same way the url crate will when we call append_pair.
        let encoded: String =
            url::form_urlencoded::byte_serialize(term.as_bytes()).collect();
        let needed = 9 + encoded.len();
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
pub fn groq_prompt_hint(replacements: &[Replacement]) -> Option<String> {
    let prefix = "Vocabulary: ";
    let mut result = prefix.to_string();
    let mut first = true;
    for entry in replacements {
        if is_punctuation_cue(entry) {
            continue;
        }
        let term = entry.from.trim();
        if term.is_empty() {
            continue;
        }
        let addition = if first {
            term.to_string()
        } else {
            format!(", {}", term)
        };
        if result.len() + addition.len() > GROQ_PROMPT_BUDGET_CHARS {
            break;
        }
        result.push_str(&addition);
        first = false;
    }
    if first {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(from: &str, to: &str) -> Replacement {
        Replacement {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    // ── is_punctuation_cue ──────────────────────────────────────────────

    #[test]
    fn punctuation_cue_single_char_punct() {
        assert!(is_punctuation_cue(&r("dot", ".")));
        assert!(is_punctuation_cue(&r("slash", "/")));
        assert!(is_punctuation_cue(&r("comma", ",")));
        assert!(is_punctuation_cue(&r("question mark", "?")));
        assert!(is_punctuation_cue(&r("exclamation mark", "!")));
    }

    #[test]
    fn punctuation_cue_two_char_punct() {
        assert!(is_punctuation_cue(&r("double dash", "--")));
    }

    #[test]
    fn not_punctuation_cue_real_word() {
        assert!(!is_punctuation_cue(&r("MongoDB", "MongoDB")));
        assert!(!is_punctuation_cue(&r("TypeScript", "TypeScript")));
    }

    #[test]
    fn not_punctuation_cue_three_char_punct_to() {
        // to.len() == 3 → does not meet < 3 condition
        assert!(!is_punctuation_cue(&r("ellipsis", "...")));
    }

    #[test]
    fn not_punctuation_cue_mixed_to() {
        // 'y' is not ascii_punctuation
        assert!(!is_punctuation_cue(&r("something", "y")));
    }

    // ── deepgram_keyterms ───────────────────────────────────────────────

    #[test]
    fn keyterms_filters_punctuation_cues() {
        let reps = vec![
            r("dot", "."),
            r("MongoDB", "MongoDB"),
            r("slash", "/"),
            r("TypeScript", "TypeScript"),
        ];
        let terms = deepgram_keyterms(&reps, 4096);
        assert_eq!(terms, vec!["MongoDB", "TypeScript"]);
    }

    #[test]
    fn keyterms_empty_replacements() {
        assert!(deepgram_keyterms(&[], 4096).is_empty());
    }

    #[test]
    fn keyterms_all_punctuation_returns_empty() {
        let reps = vec![r("dot", "."), r("slash", "/"), r("comma", ",")];
        assert!(deepgram_keyterms(&reps, 4096).is_empty());
    }

    #[test]
    fn keyterms_truncates_at_budget() {
        // "alpha" → encoded "alpha" (5 bytes); needed = 9+5 = 14.
        // "beta"  → encoded "beta"  (4 bytes); needed = 9+4 = 13.
        // Budget 25: alpha (14) fits, beta (14+13=27 > 25) truncated.
        let reps = vec![r("alpha", "alpha"), r("beta", "beta")];
        let terms = deepgram_keyterms(&reps, 25);
        assert_eq!(terms, vec!["alpha"]);
    }

    #[test]
    fn keyterms_zero_budget_returns_empty() {
        let reps = vec![r("MongoDB", "MongoDB")];
        assert!(deepgram_keyterms(&reps, 0).is_empty());
    }

    #[test]
    fn keyterms_exact_budget_fit() {
        // "hi" → 2 bytes; needed = 9+2 = 11.
        // Budget 11 → exactly fits one term.
        let reps = vec![r("hi", "hi"), r("bye", "bye")];
        let terms = deepgram_keyterms(&reps, 11);
        assert_eq!(terms, vec!["hi"]);
    }

    #[test]
    fn keyterms_skips_blank_from() {
        let reps = vec![r("  ", "something"), r("MongoDB", "MongoDB")];
        let terms = deepgram_keyterms(&reps, 4096);
        assert_eq!(terms, vec!["MongoDB"]);
    }

    // ── groq_prompt_hint ────────────────────────────────────────────────

    #[test]
    fn prompt_hint_formats_correctly() {
        let reps = vec![
            r("MongoDB", "MongoDB"),
            r("TypeScript", "TypeScript"),
            r("Kubernetes", "Kubernetes"),
        ];
        assert_eq!(
            groq_prompt_hint(&reps).unwrap(),
            "Vocabulary: MongoDB, TypeScript, Kubernetes"
        );
    }

    #[test]
    fn prompt_hint_filters_punctuation_cues() {
        let reps = vec![r("dot", "."), r("MongoDB", "MongoDB"), r("slash", "/")];
        assert_eq!(groq_prompt_hint(&reps).unwrap(), "Vocabulary: MongoDB");
    }

    #[test]
    fn prompt_hint_returns_none_for_empty() {
        assert!(groq_prompt_hint(&[]).is_none());
    }

    #[test]
    fn prompt_hint_returns_none_for_all_punctuation() {
        let reps = vec![r("dot", "."), r("comma", ",")];
        assert!(groq_prompt_hint(&reps).is_none());
    }

    #[test]
    fn prompt_hint_truncates_at_budget() {
        // "Vocabulary: " = 12 chars. Budget = 800.
        // filler (785 chars): result becomes 797 chars.
        // ", yy" would add 4 chars → 801 > 800 → truncated.
        let filler = "x".repeat(785);
        let reps = vec![r(&filler, &filler), r("yy", "yy")];
        let hint = groq_prompt_hint(&reps).unwrap();
        assert_eq!(hint.len(), 797);
        assert!(!hint.contains("yy"));
    }

    #[test]
    fn prompt_hint_includes_term_that_exactly_fits() {
        // "Vocabulary: " = 12 chars. 785-char filler → 797 chars.
        // ", y" = 3 chars → 800 exactly ≤ 800 → included.
        let filler = "x".repeat(785);
        let reps = vec![r(&filler, &filler), r("y", "y")];
        let hint = groq_prompt_hint(&reps).unwrap();
        assert_eq!(hint.len(), 800);
        assert!(hint.ends_with(", y"));
    }

    #[test]
    fn prompt_hint_single_term_no_comma() {
        let reps = vec![r("MongoDB", "MongoDB")];
        assert_eq!(groq_prompt_hint(&reps).unwrap(), "Vocabulary: MongoDB");
    }
}
