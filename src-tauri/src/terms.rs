/// 4 KB ceiling — Deepgram's documented maximum is ~8 KB; halving gives
/// comfortable headroom for the base URL and engine parameters that precede
/// any keyterm pairs.
pub const DEEPGRAM_KEYTERM_BUDGET_BYTES: usize = 4096;

/// Whisper's prompt parameter is documented as up to 224 tokens; 800 chars
/// stays well within that window for typical English vocabulary lists.
pub const GROQ_PROMPT_BUDGET_CHARS: usize = 800;

/// Returns terms suitable for Deepgram `keyterm` query params, truncating in
/// insertion order once the next term would push consumed bytes past
/// `remaining_budget`. All terms are treated as real vocabulary hints —
/// no punctuation filtering.
pub fn deepgram_keyterms(terms: &[String], remaining_budget: usize) -> Vec<String> {
    const KEY_PREFIX_BYTES: usize = "&keyterm=".len();
    let mut result = Vec::new();
    let mut used = 0usize;
    for term in terms {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            continue;
        }
        let encoded: String =
            url::form_urlencoded::byte_serialize(trimmed.as_bytes()).collect();
        let needed = KEY_PREFIX_BYTES + encoded.len();
        if used + needed > remaining_budget {
            break;
        }
        result.push(trimmed.to_string());
        used += needed;
    }
    result
}

/// Builds the Groq prompt hint (`"Vocabulary: t1, t2, t3"`) from terms.
/// Truncates at a comma boundary so the result stays within
/// `GROQ_PROMPT_BUDGET_CHARS`. Returns `None` when no eligible terms exist.
pub fn groq_prompt_hint(terms: &[String]) -> Option<String> {
    const PREFIX: &str = "Vocabulary: ";
    let mut result = PREFIX.to_string();
    for term in terms {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            continue;
        }
        let sep = if result.len() == PREFIX.len() { "" } else { ", " };
        if result.len() + sep.len() + trimmed.len() > GROQ_PROMPT_BUDGET_CHARS {
            break;
        }
        result.push_str(sep);
        result.push_str(trimmed);
    }
    (result.len() > PREFIX.len()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── deepgram_keyterms ────────────────────────────────────────────────

    #[test]
    fn keyterms_returns_all_terms() {
        let terms: Vec<String> = vec!["MongoDB".into(), "TypeScript".into()];
        let result = deepgram_keyterms(&terms, 4096);
        assert_eq!(result, vec!["MongoDB", "TypeScript"]);
    }

    #[test]
    fn keyterms_empty_input() {
        assert!(deepgram_keyterms(&[], 4096).is_empty());
    }

    #[test]
    fn keyterms_truncates_at_budget() {
        // "alpha" → 5 bytes; needed = 9+5 = 14.
        // "beta"  → 4 bytes; needed = 9+4 = 13.
        // Budget 25: alpha (14) fits, beta (14+13=27 > 25) truncated.
        let terms: Vec<String> = vec!["alpha".into(), "beta".into()];
        let result = deepgram_keyterms(&terms, 25);
        assert_eq!(result, vec!["alpha"]);
    }

    #[test]
    fn keyterms_zero_budget_returns_empty() {
        let terms: Vec<String> = vec!["MongoDB".into()];
        assert!(deepgram_keyterms(&terms, 0).is_empty());
    }

    #[test]
    fn keyterms_exact_budget_fit() {
        // "hi" → 2 bytes; needed = 9+2 = 11. Budget 11 → exactly fits one.
        let terms: Vec<String> = vec!["hi".into(), "bye".into()];
        let result = deepgram_keyterms(&terms, 11);
        assert_eq!(result, vec!["hi"]);
    }

    #[test]
    fn keyterms_skips_blank_terms() {
        let terms: Vec<String> = vec!["  ".into(), "MongoDB".into()];
        let result = deepgram_keyterms(&terms, 4096);
        assert_eq!(result, vec!["MongoDB"]);
    }

    #[test]
    fn keyterms_off_by_one_over_budget_excludes_term() {
        // "hi" URL-encodes to "hi" (2 bytes); needed = 9+2 = 11.
        // Budget 10 (one short) → term must not fit.
        let terms: Vec<String> = vec!["hi".into()];
        assert!(deepgram_keyterms(&terms, 10).is_empty());
    }

    #[test]
    fn keyterms_unicode_term_counted_by_url_encoded_bytes() {
        // "caf\u{e9}" → UTF-8: 5 bytes → URL-encoded: "caf%C3%A9" (9 bytes).
        // needed = 9 + 9 = 18. Budget 18 → fits; budget 17 → excluded.
        let terms: Vec<String> = vec!["caf\u{e9}".into()];
        assert_eq!(deepgram_keyterms(&terms, 18), vec!["café"]);
        assert!(deepgram_keyterms(&terms, 17).is_empty());
    }

    // ── groq_prompt_hint ────────────────────────────────────────────────

    #[test]
    fn prompt_hint_formats_correctly() {
        let terms: Vec<String> =
            vec!["MongoDB".into(), "TypeScript".into(), "Kubernetes".into()];
        assert_eq!(
            groq_prompt_hint(&terms).unwrap(),
            "Vocabulary: MongoDB, TypeScript, Kubernetes"
        );
    }

    #[test]
    fn prompt_hint_returns_none_for_empty() {
        assert!(groq_prompt_hint(&[]).is_none());
    }

    #[test]
    fn prompt_hint_truncates_at_budget() {
        // "Vocabulary: " = 12 chars. Budget = 800.
        // filler (785 chars): result becomes 797 chars.
        // ", yy" would add 4 chars → 801 > 800 → truncated.
        let filler = "x".repeat(785);
        let terms: Vec<String> = vec![filler, "yy".into()];
        let hint = groq_prompt_hint(&terms).unwrap();
        assert_eq!(hint.len(), 797);
        assert!(!hint.contains("yy"));
    }

    #[test]
    fn prompt_hint_includes_term_that_exactly_fits() {
        // "Vocabulary: " = 12 chars. 785-char filler → 797 chars.
        // ", y" = 3 chars → 800 exactly ≤ 800 → included.
        let filler = "x".repeat(785);
        let terms: Vec<String> = vec![filler, "y".into()];
        let hint = groq_prompt_hint(&terms).unwrap();
        assert_eq!(hint.len(), 800);
        assert!(hint.ends_with(", y"));
    }

    #[test]
    fn prompt_hint_single_term_no_comma() {
        let terms: Vec<String> = vec!["MongoDB".into()];
        assert_eq!(
            groq_prompt_hint(&terms).unwrap(),
            "Vocabulary: MongoDB"
        );
    }

    #[test]
    fn prompt_hint_skips_blank_terms() {
        let terms: Vec<String> = vec!["  ".into(), "MongoDB".into(), "\t".into()];
        assert_eq!(
            groq_prompt_hint(&terms).unwrap(),
            "Vocabulary: MongoDB"
        );
    }

    #[test]
    fn prompt_hint_returns_none_when_all_terms_are_blank() {
        let terms: Vec<String> = vec!["  ".into(), "\t".into()];
        assert!(groq_prompt_hint(&terms).is_none());
    }

    #[test]
    fn prompt_hint_unicode_term_fits_within_budget() {
        // "caf\u{e9}" is 5 UTF-8 bytes; "Vocabulary: " (12) + 5 = 17 ≤ 800.
        let terms: Vec<String> = vec!["caf\u{e9}".into()];
        assert_eq!(
            groq_prompt_hint(&terms).unwrap(),
            "Vocabulary: café"
        );
    }
}
