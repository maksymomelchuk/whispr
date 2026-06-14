use crate::config::{
    CorrectionEntry, LearnedEntry, LearnedEntryStatus, LearnedKind, NamedCorrectionSet,
};

/// Punctuation whose replacement should glue to both neighbors with no spaces.
/// Example: "test dot ts" → "test.ts".
const COMPACT: &[char] = &['.', '/', '-', '_', '@'];
/// Punctuation whose replacement should lose the leading space but keep a
/// trailing one. Example: "hello comma world" → "hello, world".
const CLING_LEFT: &[char] = &[',', ';', ':', '?', '!'];

/// Merges correction entries from the named sets identified by `set_ids`, in
/// order. On `from` collision (case-insensitive), later-set entries win.
/// Promoted learned corrections are appended after manual sets; manual entries
/// always win on `from` collision.
/// The first-occurrence position is preserved for each key so the output order
/// is deterministic and matches the order entries appear across all sets.
pub fn compose_corrections(
    set_ids: &[String],
    correction_sets: &[NamedCorrectionSet],
    learned: &[LearnedEntry],
) -> Vec<CorrectionEntry> {
    let mut keys: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, CorrectionEntry> =
        std::collections::HashMap::new();
    for id in set_ids {
        if let Some(set) = correction_sets.iter().find(|s| &s.id == id) {
            for entry in &set.entries {
                let key = entry.from.to_lowercase();
                if !map.contains_key(&key) {
                    keys.push(key.clone());
                }
                map.insert(key, entry.clone());
            }
        }
    }
    let manual_froms: std::collections::HashSet<String> = map.keys().cloned().collect();
    for entry in learned {
        if entry.status == LearnedEntryStatus::Promoted {
            if let LearnedKind::Correction { from } = &entry.kind {
                let key = from.to_lowercase();
                if !map.contains_key(&key) {
                    keys.push(key.clone());
                    map.insert(
                        key,
                        CorrectionEntry {
                            from: from.clone(),
                            to: entry.word.clone(),
                        },
                    );
                }
            }
        }
    }
    let composed: Vec<CorrectionEntry> = keys.into_iter().filter_map(|k| map.remove(&k)).collect();
    drop_inverse_pairs(composed, &manual_froms)
}

/// Drops inverse pairs (`A→B` alongside `B→A`, case-insensitive) that would
/// oscillate under `apply_corrections` until the pass cap. A manual rule (its
/// from-word is in `manual_froms`) outranks a learned one: when only one side
/// is manual, the learned side is dropped and the manual rule survives.
/// Otherwise both are dropped — neither direction is trustworthy. A case-only
/// rule (`from == to`) is never its own inverse.
fn drop_inverse_pairs(
    entries: Vec<CorrectionEntry>,
    manual_froms: &std::collections::HashSet<String>,
) -> Vec<CorrectionEntry> {
    let by_from: std::collections::HashMap<String, String> = entries
        .iter()
        .map(|e| (e.from.to_lowercase(), e.to.to_lowercase()))
        .collect();
    let mut drop: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in &entries {
        let from_lc = entry.from.to_lowercase();
        let to_lc = entry.to.to_lowercase();
        if from_lc == to_lc {
            continue;
        }
        if by_from.get(&to_lc) != Some(&from_lc) {
            continue;
        }
        // Keep this entry only when it is the manual side of a manual/learned
        // split; otherwise drop it.
        let self_manual = manual_froms.contains(&from_lc);
        let partner_manual = manual_froms.contains(&to_lc);
        if !self_manual || partner_manual {
            drop.insert(from_lc);
        }
    }
    entries
        .into_iter()
        .filter(|e| !drop.contains(&e.from.to_lowercase()))
        .collect()
}

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

    // Split rules into two groups: case-only ("ukraine" → "Ukraine") and the
    // rest ("dash" → "-", "mongo" → "MongoDB"). Case-only rules need a single
    // sweep with cursor advancement — the lowercased replacement is identical
    // to the lowercased pattern, so the main re-scan loop would either pin on
    // the first occurrence (advancing nothing) or burn passes wrapping it in
    // whitespace that later trips up phase-2 punctuation cleanup. The rest go
    // through the cascade loop so chains like "dash dash help" → "--help"
    // still resolve.
    let mut case_only: Vec<(&CorrectionEntry, String)> = Vec::new();
    let mut cascade: Vec<(&CorrectionEntry, String)> = Vec::new();
    for e in entries {
        let from_lc = e.from.to_lowercase();
        if from_lc.is_empty() {
            continue;
        }
        if e.to.to_lowercase() == from_lc {
            case_only.push((e, from_lc));
        } else {
            cascade.push((e, from_lc));
        }
    }

    for (e, from_lc) in &case_only {
        let replacement = format!(" {} ", e.to);
        let mut search_from = 0;
        loop {
            let lower = padded.to_lowercase();
            // find_word_match indexes `lower`; its byte offsets only map onto
            // `padded` when case-folding preserves byte length. A length-changing
            // fold (e.g. learned-rule text containing İ → i̇) would shift them and
            // make replace_range split a codepoint or hit the wrong slice.
            if lower.len() != padded.len() {
                break;
            }
            match find_word_match(&lower, from_lc, search_from) {
                Some((start, end)) => {
                    padded.replace_range(start..end, &replacement);
                    search_from = start + replacement.len();
                }
                None => break,
            }
        }
    }

    const MAX_PASSES: usize = 32;
    for _ in 0..MAX_PASSES {
        let lower = padded.to_lowercase();
        // same byte-length guard as the case-only sweep above.
        if lower.len() != padded.len() {
            break;
        }
        let mut changed = false;
        for (e, from_lc) in &cascade {
            if let Some((start, end)) = find_word_match(&lower, from_lc, 0) {
                let replacement = format!(" {} ", e.to);
                padded.replace_range(start..end, &replacement);
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }

    for &c in COMPACT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        if c == '.' {
            // Run middle in a loop to collapse double-spaces left by cascade
            // replacements ("  .  " → " . " → ".") without touching the
            // original dot. middle only matches space-flanked dots, so it
            // never compacts sentence-boundary dots (those have a letter
            // directly before them). head ". " is intentionally skipped —
            // applying it would strip the space in "size. Check".
            while padded.contains(&middle) {
                padded = padded.replace(&middle, &c.to_string());
            }
            padded = padded.replace(&tail, &c.to_string());
        } else {
            padded = padded.replace(&middle, &c.to_string());
            padded = padded.replace(&tail, &c.to_string());
            // strips trailing space so chained replacements glue to the
            // right neighbor: "  -   -  help " → "--help ".
            let head = format!("{} ", c);
            padded = padded.replace(&head, &c.to_string());
        }
    }
    for &c in CLING_LEFT {
        let middle = format!(" {} ", c);
        let tail = format!(" {}", c);
        padded = padded.replace(&middle, &format!("{c} "));
        padded = padded.replace(&tail, &c.to_string());
    }

    padded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Chars that count as part of a word for replacement boundary purposes.
/// Hyphen/underscore/apostrophe are included so a rule like "well" doesn't
/// match inside "well-being" or "don't".
pub(crate) fn is_word_char(c: char) -> bool {
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
    fn transcript_with_length_changing_fold_does_not_panic() {
        // İ (U+0130) lowercases to a 2-char sequence, so the lowercased copy
        // diverges in byte length from the original — the guard must bail
        // rather than feed shifted offsets into replace_range.
        let out = apply_corrections("İ test", &[entry("test", "best")]);
        assert!(out.contains("İ"));
    }

    #[test]
    fn learned_rule_emitting_length_changing_fold_does_not_panic() {
        let out = apply_corrections("hi i there", &[entry("i", "İ")]);
        assert!(out.contains("İ"));
    }

    #[test]
    fn space_after_sentence_period_preserved() {
        let out = apply_corrections("They should be same in size. Check and fix this.", &[]);
        assert_eq!(out, "They should be same in size. Check and fix this.");
    }

    #[test]
    fn dot_correction_still_compacts_compound_tokens() {
        let out = apply_corrections("test dot ts", &[entry("dot", ".")]);
        assert_eq!(out, "test.ts");
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
        assert_eq!(out, "I rely on my /emil-design-engineering, every day.");
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
    fn case_only_rule_applies_and_terminates() {
        let out = apply_corrections("I love Getmany.", &[entry("getmany", "Getmany")]);
        assert_eq!(out, "I love Getmany.");
    }

    #[test]
    fn case_only_rule_capitalizes_lowercase_input() {
        let out = apply_corrections("hello from ukraine", &[entry("ukraine", "Ukraine")]);
        assert_eq!(out, "hello from Ukraine");
    }

    #[test]
    fn case_only_rule_applies_to_every_occurrence() {
        let out = apply_corrections("from ukraine to ukraine", &[entry("ukraine", "Ukraine")]);
        assert_eq!(out, "from Ukraine to Ukraine");
    }

    #[test]
    fn rule_whose_replacement_contains_pattern_terminates() {
        // "abc" → "abc def" — the replacement re-contains the pattern as a
        // whole word, so naive re-scan loops forever. MAX_PASSES caps it;
        // here we just need termination with a sensible result.
        let out = apply_corrections("hello abc world", &[entry("abc", "abc def")]);
        assert!(
            out.contains("def"),
            "replacement should apply at least once: {out}"
        );
    }

    #[test]
    fn chained_corrections_resolve() {
        let out = apply_corrections("dash dash help", &[entry("dash", "-")]);
        assert_eq!(out, "--help");
    }

    fn named_set(id: &str, rules: &[(&str, &str)]) -> NamedCorrectionSet {
        NamedCorrectionSet {
            id: id.to_string(),
            name: id.to_string(),
            entries: rules.iter().map(|(f, t)| entry(f, t)).collect(),
        }
    }

    #[test]
    fn compose_empty_set_ids_returns_empty() {
        let sets = vec![named_set("a", &[("foo", "bar")])];
        assert!(compose_corrections(&[], &sets, &[]).is_empty());
    }

    #[test]
    fn compose_unknown_set_id_silently_skipped() {
        let sets = vec![named_set("a", &[("foo", "bar")])];
        let ids = ["a".to_string(), "nonexistent".to_string()];
        let result = compose_corrections(&ids, &sets, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].from, "foo");
    }

    #[test]
    fn compose_multiple_sets_all_entries_merged() {
        let sets = vec![
            named_set("a", &[("mongo", "MongoDB"), ("js", "JavaScript")]),
            named_set("b", &[("ts", "TypeScript")]),
        ];
        let ids = ["a".to_string(), "b".to_string()];
        let result = compose_corrections(&ids, &sets, &[]);
        assert_eq!(result.len(), 3);
        let froms: Vec<&str> = result.iter().map(|e| e.from.as_str()).collect();
        assert!(froms.contains(&"mongo"));
        assert!(froms.contains(&"js"));
        assert!(froms.contains(&"ts"));
    }

    #[test]
    fn compose_later_set_wins_on_from_collision() {
        let sets = vec![
            named_set("a", &[("foo", "bar-a")]),
            named_set("b", &[("foo", "bar-b")]),
        ];
        let ids = ["a".to_string(), "b".to_string()];
        let result = compose_corrections(&ids, &sets, &[]);
        assert_eq!(result.len(), 1, "collision deduplicates to one entry");
        assert_eq!(result[0].to, "bar-b", "later set wins");
    }

    #[test]
    fn compose_collision_preserves_first_occurrence_position() {
        let sets = vec![
            named_set("a", &[("aaa", "AAA"), ("foo", "foo-a")]),
            named_set("b", &[("foo", "foo-b"), ("zzz", "ZZZ")]),
        ];
        let ids = ["a".to_string(), "b".to_string()];
        let result = compose_corrections(&ids, &sets, &[]);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].from, "aaa");
        assert_eq!(result[1].from, "foo");
        assert_eq!(result[1].to, "foo-b");
        assert_eq!(result[2].from, "zzz");
    }

    #[test]
    fn compose_drops_inverse_pair_both_directions() {
        let sets = vec![named_set(
            "a",
            &[("postgres", "PostgreSQL"), ("PostgreSQL", "Postgres")],
        )];
        let result = compose_corrections(&["a".to_string()], &sets, &[]);
        assert!(result.is_empty(), "inverse pair drops both rules");
    }

    #[test]
    fn compose_inverse_drop_leaves_unrelated_rules() {
        let sets = vec![named_set(
            "a",
            &[
                ("postgres", "PostgreSQL"),
                ("PostgreSQL", "Postgres"),
                ("mongo", "MongoDB"),
            ],
        )];
        let result = compose_corrections(&["a".to_string()], &sets, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].from, "mongo");
    }

    fn learned_correction(from: &str, to: &str) -> LearnedEntry {
        LearnedEntry {
            id: format!("l-{from}"),
            word: to.to_string(),
            kind: LearnedKind::Correction {
                from: from.to_string(),
            },
            status: LearnedEntryStatus::Promoted,
            total_observations: 2,
            last_observed_ms: 0,
            per_app_observations: Default::default(),
        }
    }

    #[test]
    fn compose_manual_rule_survives_learned_inverse() {
        let sets = vec![named_set("a", &[("postgres", "PostgreSQL")])];
        let learned = vec![learned_correction("PostgreSQL", "Postgres")];
        let result = compose_corrections(&["a".to_string()], &sets, &learned);
        assert_eq!(
            result.len(),
            1,
            "manual rule survives, learned inverse dropped"
        );
        assert_eq!(result[0].from, "postgres");
        assert_eq!(result[0].to, "PostgreSQL");
    }

    #[test]
    fn compose_keeps_case_only_rule() {
        // from == to must not be mistaken for its own inverse and dropped.
        let sets = vec![named_set("a", &[("ukraine", "Ukraine")])];
        let result = compose_corrections(&["a".to_string()], &sets, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].to, "Ukraine");
    }

    #[test]
    fn inverse_pair_no_longer_rewrites_text() {
        let sets = vec![named_set(
            "a",
            &[("postgres", "PostgreSQL"), ("PostgreSQL", "Postgres")],
        )];
        let entries = compose_corrections(&["a".to_string()], &sets, &[]);
        let out = apply_corrections("i use postgres daily", &entries);
        assert_eq!(out, "i use postgres daily", "neither direction applied");
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
