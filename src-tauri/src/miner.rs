use crate::config::{
    CorrectionEntry, LearnedEntry, LearnedEntryStatus, LearnedKind, NamedCorrectionSet,
    NamedTermSet, Settings, DEFAULT_CORRECTION_SET_ID, SEED_TERM_SET_DEFAULT_ID,
};
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
        .filter(|s| {
            s.from.split_whitespace().count() <= 3
                && s.to.split_whitespace().count() <= 3
        })
        .filter(|s| {
            is_phonetically_similar(&s.from, &s.to)
                || looks_like_proper_noun(&s.to, s.to_word_pos == 0)
        })
        .map(|s| MinedCandidate {
            from: s.from,
            to: s.to,
        })
        .collect()
}

/// Inconsistent mappings (same `from`, different `to`) demote existing
/// Correction entries to Terms.
pub fn observe_candidates(candidates: &[MinedCandidate], settings: &mut Settings, now_ms: i64) {
    for candidate in candidates {
        observe_one(&candidate.from, &candidate.to, &mut settings.learned_entries, now_ms);
    }
    evict_stale(&mut settings.learned_entries, now_ms);
}

/// Default correction/term set is created if absent. The learned entry is
/// removed after promotion regardless of its current status.
pub fn promote_entry(settings: &mut Settings, id: &str) {
    let entry = match settings.learned_entries.iter().find(|e| e.id == id).cloned() {
        Some(e) => e,
        None => return,
    };

    match &entry.kind {
        LearnedKind::Correction { from } => {
            let correction = CorrectionEntry {
                from: from.clone(),
                to: entry.word.clone(),
            };
            if let Some(cs) = settings
                .correction_sets
                .iter_mut()
                .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
            {
                let already_exact = cs
                    .entries
                    .iter()
                    .any(|ce| ce.from == correction.from && ce.to == correction.to);
                let has_conflict = !already_exact
                    && cs.entries.iter().any(|ce| ce.from == correction.from);
                if has_conflict {
                    // A different mapping for the same `from` exists in the permanent set.
                    // Leave the learned entry in place so the user can resolve it manually.
                    return;
                }
                if !already_exact {
                    cs.entries.push(correction);
                }
            } else {
                settings.correction_sets.push(NamedCorrectionSet {
                    id: DEFAULT_CORRECTION_SET_ID.to_string(),
                    name: "Default Corrections".to_string(),
                    entries: vec![correction],
                });
            }
        }
        LearnedKind::Term => {
            if let Some(ts) = settings
                .term_sets
                .iter_mut()
                .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
            {
                if !ts.entries.contains(&entry.word) {
                    ts.entries.push(entry.word.clone());
                }
            } else {
                settings.term_sets.push(NamedTermSet {
                    id: SEED_TERM_SET_DEFAULT_ID.to_string(),
                    name: "Default Terms".to_string(),
                    entries: vec![entry.word.clone()],
                });
            }
        }
    }

    settings.learned_entries.retain(|e| e.id != id);
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
    Equal(&'a str),
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
            ops.push(DiffOp::Equal(before[i - 1]));
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
        if matches!(diff[i], DiffOp::Equal(_)) {
            after_pos += 1;
            i += 1;
            continue;
        }

        let block_after_start = after_pos;
        let mut deletes = vec![];
        let mut inserts = vec![];

        while i < diff.len() && !matches!(diff[i], DiffOp::Equal(_)) {
            match &diff[i] {
                DiffOp::Delete(w) => deletes.push(*w),
                DiffOp::Insert(w) => {
                    inserts.push(*w);
                    after_pos += 1;
                }
                DiffOp::Equal(_) => unreachable!(),
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

fn observe_one(from: &str, to: &str, entries: &mut Vec<LearnedEntry>, now_ms: i64) {
    // If `from` already maps inconsistently to different targets, demote all
    // existing Correction entries for `from` to Terms and add `to` as a Term.
    let has_different_target = entries.iter().any(|e| {
        matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) && e.word != to
    });

    if has_different_target {
        for e in entries.iter_mut() {
            if matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) {
                e.kind = LearnedKind::Term;
            }
        }
        observe_term(to, entries, now_ms);
        return;
    }

    if let Some(e) = entries.iter_mut().find(|e| {
        matches!(&e.kind, LearnedKind::Correction { from: f } if f == from) && e.word == to
    }) {
        e.total_observations += 1;
        e.last_observed_ms = now_ms;
        if e.total_observations >= PROMOTE_THRESHOLD {
            e.status = LearnedEntryStatus::Promoted;
        }
        return;
    }

    let id = format!("learned-{now_ms}-{}", entries.len());
    entries.push(LearnedEntry {
        id,
        word: to.to_string(),
        kind: LearnedKind::Correction {
            from: from.to_string(),
        },
        status: LearnedEntryStatus::Candidate,
        total_observations: 1,
        last_observed_ms: now_ms,
    });

    if entries.len() > MAX_ENTRIES {
        evict_lru(entries);
    }
}

fn observe_term(to: &str, entries: &mut Vec<LearnedEntry>, now_ms: i64) {
    if let Some(e) = entries
        .iter_mut()
        .find(|e| matches!(&e.kind, LearnedKind::Term) && e.word == to)
    {
        e.total_observations += 1;
        e.last_observed_ms = now_ms;
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
    });
}

fn evict_stale(entries: &mut Vec<LearnedEntry>, now_ms: i64) {
    entries.retain(|e| now_ms - e.last_observed_ms < STALENESS_MS);
}

fn evict_lru(entries: &mut Vec<LearnedEntry>) {
    entries.sort_by_key(|e| e.last_observed_ms);
    entries.truncate(MAX_ENTRIES);
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(candidates: Vec<MinedCandidate>) -> Vec<(String, String)> {
        candidates
            .into_iter()
            .map(|c| (c.from, c.to))
            .collect()
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

    // ── lifecycle / store tests ────────────────────────────────────────────

    fn make_settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn one_observation_stays_candidate() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
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
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        observe_one("tory", "Tauri", &mut s.learned_entries, 2000);
        assert_eq!(s.learned_entries.len(), 1);
        assert_eq!(s.learned_entries[0].status, LearnedEntryStatus::Promoted);
        assert_eq!(s.learned_entries[0].total_observations, 2);
    }

    #[test]
    fn consistent_fix_is_correction_kind() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        observe_one("tory", "Tauri", &mut s.learned_entries, 2000);
        assert!(matches!(
            &s.learned_entries[0].kind,
            LearnedKind::Correction { from } if from == "tory"
        ));
    }

    #[test]
    fn inconsistent_mapping_demotes_to_term() {
        let mut s = make_settings();
        // First: "tory" → "Tauri" (Correction candidate).
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        // Second: "tory" → "Toronto" (different target — inconsistency).
        observe_one("tory", "Toronto", &mut s.learned_entries, 2000);

        // The original Correction entry for "Tauri" must be demoted to Term.
        let tauri_entry = s.learned_entries.iter().find(|e| e.word == "Tauri");
        assert!(
            tauri_entry.is_some(),
            "Tauri entry should still exist as a Term"
        );
        assert!(
            matches!(tauri_entry.unwrap().kind, LearnedKind::Term),
            "Tauri entry should be demoted to Term"
        );

        // "Toronto" should be added as a Term candidate.
        let toronto_entry = s.learned_entries.iter().find(|e| e.word == "Toronto");
        assert!(toronto_entry.is_some());
        assert!(matches!(toronto_entry.unwrap().kind, LearnedKind::Term));
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
        }];
        let now = STALENESS_MS + 1;
        evict_stale(&mut entries, now);
        assert!(entries.is_empty());
    }

    // ── promote_entry tests ────────────────────────────────────────────────

    #[test]
    fn promote_correction_adds_to_default_set() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        observe_one("tory", "Tauri", &mut s.learned_entries, 2000);
        let id = s.learned_entries[0].id.clone();
        promote_entry(&mut s, &id);

        assert!(s.learned_entries.is_empty(), "entry removed after promote");
        let default_cs = s
            .correction_sets
            .iter()
            .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID);
        assert!(default_cs.is_some());
        assert!(default_cs
            .unwrap()
            .entries
            .iter()
            .any(|e| e.from == "tory" && e.to == "Tauri"));
    }

    #[test]
    fn promote_correction_skipped_when_conflicting_from_exists() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        observe_one("tory", "Tauri", &mut s.learned_entries, 2000);
        let id = s.learned_entries[0].id.clone();

        // Manually plant a conflicting correction in the permanent set.
        s.correction_sets.push(NamedCorrectionSet {
            id: DEFAULT_CORRECTION_SET_ID.to_string(),
            name: "Default Corrections".to_string(),
            entries: vec![CorrectionEntry {
                from: "tory".to_string(),
                to: "Toronto".to_string(),
            }],
        });

        promote_entry(&mut s, &id);

        // Learned entry must survive — it was not promoted.
        assert_eq!(s.learned_entries.len(), 1, "learned entry kept on conflict");
        // Permanent set must be unchanged.
        let cs = s
            .correction_sets
            .iter()
            .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
            .unwrap();
        assert_eq!(cs.entries.len(), 1);
        assert_eq!(cs.entries[0].to, "Toronto");
    }

    #[test]
    fn promote_correction_noop_when_exact_duplicate_exists() {
        let mut s = make_settings();
        observe_one("tory", "Tauri", &mut s.learned_entries, 1000);
        observe_one("tory", "Tauri", &mut s.learned_entries, 2000);
        let id = s.learned_entries[0].id.clone();

        // Exact duplicate already in permanent set.
        s.correction_sets.push(NamedCorrectionSet {
            id: DEFAULT_CORRECTION_SET_ID.to_string(),
            name: "Default Corrections".to_string(),
            entries: vec![CorrectionEntry {
                from: "tory".to_string(),
                to: "Tauri".to_string(),
            }],
        });

        promote_entry(&mut s, &id);

        // Learned entry is removed — it was effectively already promoted.
        assert!(s.learned_entries.is_empty(), "learned entry removed for duplicate");
        // Permanent set unchanged — no duplicate pushed.
        let cs = s
            .correction_sets
            .iter()
            .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
            .unwrap();
        assert_eq!(cs.entries.len(), 1);
    }

    #[test]
    fn promote_term_adds_to_default_set() {
        let mut s = make_settings();
        observe_term("Tauri", &mut s.learned_entries, 1000);
        observe_term("Tauri", &mut s.learned_entries, 2000);
        let id = s.learned_entries[0].id.clone();
        promote_entry(&mut s, &id);

        assert!(s.learned_entries.is_empty());
        let default_ts = s
            .term_sets
            .iter()
            .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID);
        assert!(default_ts.is_some());
        assert!(default_ts.unwrap().entries.contains(&"Tauri".to_string()));
    }
}
