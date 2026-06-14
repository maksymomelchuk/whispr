use crate::config::{LearnedEntry, LearnedEntryStatus, LearnedKind, Settings};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinedCandidate {
    pub from: String,
    pub to: String,
}

/// Filter chain (in order):
/// 1. Whole-text word-edit ratio < 0.30 — rewrites discard everything.
/// 2. Each substitution span ≤ 3 words on both sides.
/// 3. Phonetic similarity (same Soundex code, confirmed by edit distance) OR
///    proper-noun shape (mixed case, or title-case mid-sentence).
pub fn mine(before: &str, after: &str) -> Vec<MinedCandidate> {
    if !passes_ratio_filter(before, after) {
        return vec![];
    }

    let before_words: Vec<&str> = before.split_whitespace().collect();
    let after_words: Vec<&str> = after.split_whitespace().collect();

    extract_substitutions(&before_words, &after_words)
        .into_iter()
        .filter(|s| s.from.split_whitespace().count() <= 3 && s.to.split_whitespace().count() <= 3)
        .filter_map(|s| {
            let from = trim_to_word_bounds(&s.from).to_string();
            let to = trim_to_word_bounds(&s.to).to_string();
            if from.is_empty() || to.is_empty() || from == to {
                return None;
            }
            let accept = is_phonetically_similar(&from, &to)
                || looks_like_proper_noun(&to, s.to_word_pos == 0);
            accept.then_some(MinedCandidate { from, to })
        })
        .collect()
}

// Strip boundary punctuation so a sentence-final mishearing ("Postgres.") maps
// to the same `from` the whole-word matcher uses — keeping word-internal `'-_`.
fn trim_to_word_bounds(token: &str) -> &str {
    token.trim_matches(|c: char| !crate::corrections::is_word_char(c))
}

/// When a `from` word has seen conflicting targets, the old rule is replaced
/// by the new one and the `from` word is marked inconsistent — preventing
/// replacement cycles on future observations.
pub fn observe_candidates(
    candidates: &[MinedCandidate],
    settings: &mut Settings,
    bundle_id: Option<&str>,
    now_ms: i64,
) {
    for candidate in candidates {
        observe_one(&candidate.from, &candidate.to, settings, bundle_id, now_ms);
    }
    evict_stale(&mut settings.learned_entries, now_ms);
}

/// Promotes a candidate in place. Promoted entries stay in `learned_entries`
/// (subject to staleness/LRU) and are applied directly by the correction/term
/// selectors — the same state auto-promotion produces at PROMOTE_THRESHOLD.
pub fn promote_entry(settings: &mut Settings, id: &str) {
    if let Some(entry) = settings.learned_entries.iter_mut().find(|e| e.id == id) {
        entry.status = LearnedEntryStatus::Promoted;
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── internal types ─────────────────────────────────────────────────────────

struct Substitution {
    from: String,
    to: String,
    to_word_pos: usize,
}

// ── word diff ──────────────────────────────────────────────────────────────

#[derive(Clone)]
enum DiffOp<'a> {
    Equal,
    Delete(&'a str),
    Insert(&'a str),
}

fn word_diff<'a>(before: &[&'a str], after: &[&'a str]) -> Vec<DiffOp<'a>> {
    let n = before.len();
    let m = after.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if before[i - 1] == after[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }

    let mut ops = vec![];
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && before[i - 1] == after[j - 1] {
            ops.push(DiffOp::Equal);
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(DiffOp::Insert(after[j - 1]));
            j -= 1;
        } else {
            ops.push(DiffOp::Delete(before[i - 1]));
            i -= 1;
        }
    }
    ops.reverse();
    ops
}

fn extract_substitutions(before: &[&str], after: &[&str]) -> Vec<Substitution> {
    let diff = word_diff(before, after);
    let mut result = vec![];
    let mut i = 0;
    let mut after_pos: usize = 0;

    while i < diff.len() {
        if matches!(diff[i], DiffOp::Equal) {
            after_pos += 1;
            i += 1;
            continue;
        }

        let block_after_start = after_pos;
        let mut deletes = vec![];
        let mut inserts = vec![];

        while i < diff.len() && !matches!(diff[i], DiffOp::Equal) {
            match &diff[i] {
                DiffOp::Delete(w) => deletes.push(*w),
                DiffOp::Insert(w) => {
                    inserts.push(*w);
                    after_pos += 1;
                }
                DiffOp::Equal => unreachable!(),
            }
            i += 1;
        }

        if !deletes.is_empty() && !inserts.is_empty() {
            result.push(Substitution {
                from: deletes.join(" "),
                to: inserts.join(" "),
                to_word_pos: block_after_start,
            });
        }
    }

    result
}

// ── edit ratio filter ──────────────────────────────────────────────────────

fn passes_ratio_filter(before: &str, after: &str) -> bool {
    let bw: Vec<&str> = before.split_whitespace().collect();
    let aw: Vec<&str> = after.split_whitespace().collect();
    let total = bw.len().max(aw.len());
    if total == 0 {
        return true;
    }
    let dist = word_edit_distance(&bw, &aw);
    (dist as f64 / total as f64) < 0.30
}

fn word_edit_distance(a: &[&str], b: &[&str]) -> usize {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[n][m]
}

// ── phonetic filter ────────────────────────────────────────────────────────

// Pure case changes are excluded — those are proper-noun candidates, not phonetic ones.
fn is_phonetically_similar(from: &str, to: &str) -> bool {
    if from.contains(' ') || to.contains(' ') {
        return false;
    }
    let fl = from.to_lowercase();
    let tl = to.to_lowercase();
    // Pure case change: handled by looks_like_proper_noun, not phonetic path.
    if fl == tl {
        return false;
    }
    if soundex(&fl) != soundex(&tl) {
        return false;
    }
    let dist = char_edit_distance(&fl, &tl);
    let max_len = fl.len().max(tl.len());
    max_len > 0 && (dist as f64 / max_len as f64) < 0.5
}

fn soundex(s: &str) -> String {
    #[rustfmt::skip]
    const TABLE: [u8; 26] = [
    //  A  B  C  D  E  F  G  H  I  J  K  L  M
        0, 1, 2, 3, 0, 1, 2, 0, 0, 2, 2, 4, 5,
    //  N  O  P  Q  R  S  T  U  V  W  X  Y  Z
        5, 0, 1, 2, 6, 2, 3, 0, 1, 0, 2, 0, 2,
    ];

    let chars: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return "0000".to_string();
    }

    let first = chars[0].to_ascii_uppercase();
    let mut code = first.to_string();
    let mut last = TABLE[(chars[0] as u8 - b'a') as usize];

    for &c in &chars[1..] {
        let idx = (c as u8 - b'a') as usize;
        let digit = TABLE[idx];
        if digit != 0 && digit != last {
            code.push((b'0' + digit) as char);
            if code.len() >= 4 {
                break;
            }
        }
        if digit != 0 {
            last = digit;
        }
    }

    while code.len() < 4 {
        code.push('0');
    }
    code
}

fn char_edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[n][m]
}

// ── proper-noun filter ─────────────────────────────────────────────────────

// Mixed case anywhere (camelCase, PascalCase, etc.) is always a signal.
// Simple title case is only a signal mid-sentence (`!at_sentence_start`).
fn looks_like_proper_noun(to: &str, at_sentence_start: bool) -> bool {
    let has_upper = to.chars().any(|c| c.is_uppercase());
    let has_lower = to.chars().any(|c| c.is_lowercase());

    if has_upper && has_lower {
        let first_upper = to.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let rest_lower = to
            .chars()
            .skip(1)
            .all(|c| c.is_lowercase() || !c.is_alphabetic());
        if first_upper && rest_lower {
            // Simple title case: "Hello", "John", "Tauri"
            return !at_sentence_start;
        }
        // Real mixed case: "camelCase", "McLaughlin", "iPhone"
        return true;
    }

    // All-caps acronym (at least 2 characters)
    if to.len() >= 2 && to.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        return true;
    }

    false
}

// ── lifecycle store ────────────────────────────────────────────────────────

const STALENESS_MS: i64 = 90 * 24 * 60 * 60 * 1000;
const MAX_ENTRIES: usize = 1000;
const PROMOTE_THRESHOLD: u32 = 2;

fn observe_one(
    from: &str,
    to: &str,
    settings: &mut Settings,
    bundle_id: Option<&str>,
    now_ms: i64,
) {
    let known_inconsistent = settings.learned_inconsistent_from.iter().any(|f| f == from);

    if known_inconsistent {
        // Reinforce the current rule if it still matches; otherwise fall through to
        // Term — re-creating a Correction here would enable replacement cycles.
        if let Some(e) = settings.learned_entries.iter_mut().find(|e| {
            matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) && e.word == to
        }) {
            e.total_observations += 1;
            e.last_observed_ms = now_ms;
            increment_app_obs(e, bundle_id);
            if e.total_observations >= PROMOTE_THRESHOLD {
                e.status = LearnedEntryStatus::Promoted;
            }
            return;
        }
        observe_term(to, &mut settings.learned_entries, bundle_id, now_ms);
        return;
    }

    // Inverse contradiction: an existing `to → from` rule makes this a 2-cycle
    // that oscillates under apply_corrections. Drop the existing rule, skip the
    // new one, and mark both from-words inconsistent so neither side recreates.
    let from_lc = from.to_lowercase();
    let to_lc = to.to_lowercase();
    let inverse_from = settings.learned_entries.iter().find_map(|e| match &e.kind {
        LearnedKind::Correction { from: f }
            if f.to_lowercase() == to_lc && e.word.to_lowercase() == from_lc =>
        {
            Some(f.clone())
        }
        _ => None,
    });
    if let Some(inverse_from) = inverse_from {
        settings.learned_entries.retain(
            |e| !matches!(&e.kind, LearnedKind::Correction { from: f } if *f == inverse_from),
        );
        for word in [from.to_string(), inverse_from] {
            if !settings.learned_inconsistent_from.contains(&word) {
                settings.learned_inconsistent_from.push(word);
            }
        }
        return;
    }

    let has_different_target = settings.learned_entries.iter().any(|e| {
        matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) && e.word != to
    });

    if has_different_target {
        // Replace: remove the old Correction entirely (one rule per from-word).
        settings
            .learned_entries
            .retain(|e| !matches!(&e.kind, LearnedKind::Correction { from: f } if f == from));
        // Record the inconsistency so future observations never create a new Correction.
        settings.learned_inconsistent_from.push(from.to_string());
        // The new target starts fresh as a Correction candidate.
        let id = format!("learned-{now_ms}-{}", settings.learned_entries.len());
        settings.learned_entries.push(LearnedEntry {
            id,
            word: to.to_string(),
            kind: LearnedKind::Correction {
                from: from.to_string(),
            },
            status: LearnedEntryStatus::Candidate,
            total_observations: 1,
            last_observed_ms: now_ms,
            per_app_observations: initial_app_obs(bundle_id),
        });
        if settings.learned_entries.len() > MAX_ENTRIES {
            evict_lru(&mut settings.learned_entries);
        }
        return;
    }

    if let Some(e) = settings.learned_entries.iter_mut().find(|e| {
        matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) && e.word == to
    }) {
        e.total_observations += 1;
        e.last_observed_ms = now_ms;
        increment_app_obs(e, bundle_id);
        if e.total_observations >= PROMOTE_THRESHOLD {
            e.status = LearnedEntryStatus::Promoted;
        }
        return;
    }

    let id = format!("learned-{now_ms}-{}", settings.learned_entries.len());
    settings.learned_entries.push(LearnedEntry {
        id,
        word: to.to_string(),
        kind: LearnedKind::Correction {
            from: from.to_string(),
        },
        status: LearnedEntryStatus::Candidate,
        total_observations: 1,
        last_observed_ms: now_ms,
        per_app_observations: initial_app_obs(bundle_id),
    });

    if settings.learned_entries.len() > MAX_ENTRIES {
        evict_lru(&mut settings.learned_entries);
    }
}

fn observe_term(to: &str, entries: &mut Vec<LearnedEntry>, bundle_id: Option<&str>, now_ms: i64) {
    if let Some(e) = entries
        .iter_mut()
        .find(|e| matches!(&e.kind, LearnedKind::Term) && e.word == to)
    {
        e.total_observations += 1;
        e.last_observed_ms = now_ms;
        increment_app_obs(e, bundle_id);
        if e.total_observations >= PROMOTE_THRESHOLD {
            e.status = LearnedEntryStatus::Promoted;
        }
        return;
    }

    let id = format!("learned-{now_ms}-t-{}", entries.len());
    entries.push(LearnedEntry {
        id,
        word: to.to_string(),
        kind: LearnedKind::Term,
        status: LearnedEntryStatus::Candidate,
        total_observations: 1,
        last_observed_ms: now_ms,
        per_app_observations: initial_app_obs(bundle_id),
    });
}

fn initial_app_obs(bundle_id: Option<&str>) -> std::collections::BTreeMap<String, u32> {
    let mut map = std::collections::BTreeMap::new();
    if let Some(id) = bundle_id {
        map.insert(id.to_string(), 1);
    }
    map
}

fn increment_app_obs(entry: &mut LearnedEntry, bundle_id: Option<&str>) {
    if let Some(id) = bundle_id {
        *entry
            .per_app_observations
            .entry(id.to_string())
            .or_insert(0) += 1;
    }
}

fn evict_stale(entries: &mut Vec<LearnedEntry>, now_ms: i64) {
    entries.retain(|e| now_ms - e.last_observed_ms < STALENESS_MS);
}

fn evict_lru(entries: &mut Vec<LearnedEntry>) {
    entries.sort_by(|a, b| b.last_observed_ms.cmp(&a.last_observed_ms));
    entries.truncate(MAX_ENTRIES);
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(candidates: Vec<MinedCandidate>) -> Vec<(String, String)> {
        candidates.into_iter().map(|c| (c.from, c.to)).collect()
    }

    // ── miner tests ────────────────────────────────────────────────────────

    #[test]
    fn proper_noun_fix_is_extracted() {
        // "tori" is a phonetically similar mishearing of "Tauri"; Tauri is also
        // a proper noun (title-case mid-sentence).
        let candidates = mine(
            "i use tori for the desktop app",
            "i use Tauri for the desktop app",
        );
        assert_eq!(pairs(candidates), vec![("tori".into(), "Tauri".into())]);
    }

    #[test]
    fn phonetically_similar_mishear_is_extracted() {
        // "wether" is a phonetic mishear of "weather" (same Soundex W360).
        let candidates = mine(
            "i checked the wether forecast",
            "i checked the weather forecast",
        );
        assert_eq!(pairs(candidates), vec![("wether".into(), "weather".into())]);
    }

    #[test]
    fn content_rewrite_yields_no_candidates() {
        // Over 30% word edit ratio → discard everything.
        let candidates = mine(
            "one two three four five six seven eight nine ten",
            "alpha beta gamma delta epsilon zeta eta theta iota kappa",
        );
        assert!(candidates.is_empty());
    }

    #[test]
    fn substitution_span_over_three_words_yields_nothing() {
        // The 4-word substitution span exceeds the limit even though ratio is low.
        let before = "w1 w2 w3 w4 w5 w6 w7 w8 w9 w10 w11 w12 w13 w14 w15 w16 w17 w18 w19 w20";
        let after = "w1 w2 w3 w4 w5 w6 w7 w8 w9 w10 Foo Bar Baz Qux w15 w16 w17 w18 w19 w20";
        let candidates = mine(before, after);
        assert!(candidates.is_empty());
    }

    #[test]
    fn mixed_case_technical_term_is_extracted() {
        // "github" → "GitHub" has mixed case → proper noun, not sentence-start.
        let candidates = mine(
            "push the code to github today",
            "push the code to GitHub today",
        );
        assert_eq!(pairs(candidates), vec![("github".into(), "GitHub".into())]);
    }

    #[test]
    fn first_word_title_case_is_not_extracted() {
        // Title-case at position 0 is normal sentence capitalisation, not a
        // proper noun correction.
        let candidates = mine("hello world", "Hello world");
        assert!(candidates.is_empty());
    }

    #[test]
    fn pure_deletion_without_insertion_yields_nothing() {
        let candidates = mine("hello beautiful world", "hello world");
        assert!(candidates.is_empty());
    }

    #[test]
    fn trailing_punctuation_is_trimmed_from_candidates() {
        // Sentence-final period must not glue onto the `from` word.
        let candidates = mine("i really like postgres.", "i really like PostgreSQL.");
        assert_eq!(
            pairs(candidates),
            vec![("postgres".into(), "PostgreSQL".into())]
        );
    }

    #[test]
    fn punctuation_only_change_yields_no_candidate() {
        let candidates = mine(
            "i really love using postgres",
            "i really love using postgres.",
        );
        assert!(candidates.is_empty());
    }

    // ── lifecycle / store tests ────────────────────────────────────────────

    fn make_settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn one_observation_stays_candidate() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        assert_eq!(s.learned_entries.len(), 1);
        assert_eq!(s.learned_entries[0].status, LearnedEntryStatus::Candidate);
        assert_eq!(s.learned_entries[0].total_observations, 1);
        assert!(matches!(
            &s.learned_entries[0].kind,
            LearnedKind::Correction { from } if from == "tory"
        ));
    }

    #[test]
    fn second_observation_promotes() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        observe_one("tory", "Tauri", &mut s, None, 2000);
        assert_eq!(s.learned_entries.len(), 1);
        assert_eq!(s.learned_entries[0].status, LearnedEntryStatus::Promoted);
        assert_eq!(s.learned_entries[0].total_observations, 2);
    }

    #[test]
    fn consistent_fix_is_correction_kind() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        observe_one("tory", "Tauri", &mut s, None, 2000);
        assert!(matches!(
            &s.learned_entries[0].kind,
            LearnedKind::Correction { from } if from == "tory"
        ));
    }

    #[test]
    fn inconsistent_mapping_replaces_existing_rule() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        observe_one("tory", "Toronto", &mut s, None, 2000);

        // Old Correction for Tauri is removed (not kept as a Term).
        assert!(
            s.learned_entries
                .iter()
                .find(|e| e.word == "Tauri")
                .is_none(),
            "replaced entry must be removed"
        );

        // Toronto becomes the new Correction for the same from-word.
        let toronto = s.learned_entries.iter().find(|e| e.word == "Toronto");
        assert!(toronto.is_some(), "replacement entry must exist");
        assert!(
            matches!(&toronto.unwrap().kind, LearnedKind::Correction { from } if from == "tory"),
            "replacement entry must be a Correction for the same from-word"
        );

        // from-word is marked inconsistent.
        assert!(
            s.learned_inconsistent_from.iter().any(|f| f == "tory"),
            "from-word must be recorded as inconsistent"
        );
    }

    #[test]
    fn replacement_cycle_is_impossible() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        // First inconsistency: Toronto replaces Tauri.
        observe_one("tory", "Toronto", &mut s, None, 2000);
        // Second observation re-introduces Tauri — must become a Term, not a new Correction.
        observe_one("tory", "Tauri", &mut s, None, 3000);

        let tauri = s.learned_entries.iter().find(|e| e.word == "Tauri");
        assert!(tauri.is_some(), "Tauri entry must exist as a Term");
        assert!(
            matches!(tauri.unwrap().kind, LearnedKind::Term),
            "re-introduced target must be a Term, not a Correction"
        );

        // Only one Correction for tory must remain (Toronto).
        let corrections_for_tory: Vec<_> = s
            .learned_entries
            .iter()
            .filter(|e| matches!(&e.kind, LearnedKind::Correction { from } if from == "tory"))
            .collect();
        assert_eq!(
            corrections_for_tory.len(),
            1,
            "at most one Correction per from-word"
        );
        assert_eq!(corrections_for_tory[0].word, "Toronto");
    }

    #[test]
    fn inverse_correction_drops_both_and_marks_inconsistent() {
        let mut s = make_settings();
        observe_one("postgres", "PostgreSQL", &mut s, None, 1000);
        observe_one("PostgreSQL", "Postgres", &mut s, None, 2000);

        let corrections: Vec<_> = s
            .learned_entries
            .iter()
            .filter(|e| matches!(&e.kind, LearnedKind::Correction { .. }))
            .collect();
        assert!(
            corrections.is_empty(),
            "both directions of an inverse pair are dropped"
        );
        assert!(s.learned_inconsistent_from.iter().any(|f| f == "postgres"));
        assert!(s
            .learned_inconsistent_from
            .iter()
            .any(|f| f == "PostgreSQL"));
    }

    #[test]
    fn at_most_one_correction_per_from_word() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        observe_one("tory", "Toronto", &mut s, None, 2000);

        let corrections: Vec<_> = s
            .learned_entries
            .iter()
            .filter(|e| matches!(&e.kind, LearnedKind::Correction { from } if from == "tory"))
            .collect();
        assert_eq!(
            corrections.len(),
            1,
            "exactly one Correction must exist after replacement"
        );
    }

    #[test]
    fn stale_entries_are_evicted() {
        let mut entries = vec![LearnedEntry {
            id: "old".into(),
            word: "old".into(),
            kind: LearnedKind::Term,
            status: LearnedEntryStatus::Candidate,
            total_observations: 1,
            last_observed_ms: 0,
            per_app_observations: Default::default(),
        }];
        let now = STALENESS_MS + 1;
        evict_stale(&mut entries, now);
        assert!(entries.is_empty());
    }

    #[test]
    fn evict_lru_keeps_newest_entries() {
        let mut entries: Vec<LearnedEntry> = (0..=(MAX_ENTRIES as i64))
            .map(|i| LearnedEntry {
                id: format!("e{i}"),
                word: format!("w{i}"),
                kind: LearnedKind::Term,
                status: LearnedEntryStatus::Candidate,
                total_observations: 1,
                last_observed_ms: i,
                per_app_observations: Default::default(),
            })
            .collect();
        evict_lru(&mut entries);
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert!(entries.iter().all(|e| e.last_observed_ms > 0));
        assert!(entries
            .iter()
            .any(|e| e.last_observed_ms == MAX_ENTRIES as i64));
    }

    // ── promote_entry tests ────────────────────────────────────────────────

    #[test]
    fn promote_marks_candidate_promoted_in_place() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        let id = s.learned_entries[0].id.clone();
        promote_entry(&mut s, &id);

        assert_eq!(s.learned_entries.len(), 1, "entry stays in learned_entries");
        assert_eq!(s.learned_entries[0].status, LearnedEntryStatus::Promoted);
        assert!(
            s.correction_sets.is_empty(),
            "promote no longer ejects into a named set"
        );
    }

    #[test]
    fn promote_unknown_id_is_noop() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s, None, 1000);
        promote_entry(&mut s, "no-such-id");

        assert_eq!(s.learned_entries[0].status, LearnedEntryStatus::Candidate);
    }
}
