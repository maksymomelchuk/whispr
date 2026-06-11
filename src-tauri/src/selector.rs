use crate::config::{
    LearnedEntry, LearnedEntryStatus, LearnedKind, NamedCorrectionSet, NamedTermSet,
};

/// Maximum terms forwarded to STT Engine recognition hint slots (keyterms,
/// prompt vocabulary). Industry data shows ASR prompt biasing saturates around
/// 70–100 terms; 40 leaves comfortable headroom while still spending the budget
/// on the most relevant words for the current app.
pub const ENGINE_TERM_BUDGET: usize = 40;

pub const GLOSSARY_BUDGET: usize = 200;

/// Half-life for recency decay: an entry last observed this many days ago
/// retains ~37% of its original relevance weight.
const RECENCY_HALF_LIFE_DAYS: f64 = 30.0;

const MS_PER_DAY: f64 = 1000.0 * 60.0 * 60.0 * 24.0;

fn recency_score(last_observed_ms: i64, now_ms: i64) -> f64 {
    let elapsed_ms = (now_ms - last_observed_ms).max(0) as f64;
    let elapsed_days = elapsed_ms / MS_PER_DAY;
    (-elapsed_days / RECENCY_HALF_LIFE_DAYS).exp()
}

fn app_frequency_ratio(entry: &LearnedEntry, bundle_id: &str) -> f64 {
    if entry.total_observations == 0 {
        return 0.0;
    }
    let app_obs = *entry.per_app_observations.get(bundle_id).unwrap_or(&0);
    app_obs as f64 / entry.total_observations as f64
}

fn score(entry: &LearnedEntry, bundle_id: Option<&str>, now_ms: i64) -> f64 {
    let recency = recency_score(entry.last_observed_ms, now_ms);
    let app_freq = bundle_id
        .map(|id| app_frequency_ratio(entry, id))
        .unwrap_or(0.0);
    (1.0 + app_freq) * recency
}

/// Manual entries (from named term sets) always fill the budget before any
/// learned entries — they win all budget ties. Among learned entries, those
/// observed most frequently in `bundle_id`'s app and most recently rank highest.
pub fn select_terms(
    term_sets: &[NamedTermSet],
    set_ids: &[String],
    learned: &[LearnedEntry],
    bundle_id: Option<&str>,
    now_ms: i64,
) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for id in set_ids {
        let Some(set) = term_sets.iter().find(|ts| &ts.id == id) else {
            continue;
        };
        for entry in &set.entries {
            let word = entry.trim().to_string();
            if !word.is_empty() && seen.insert(word.clone()) {
                result.push(word);
                if result.len() >= ENGINE_TERM_BUDGET {
                    return result;
                }
            }
        }
    }

    let mut scored: Vec<(&LearnedEntry, f64)> = learned
        .iter()
        .filter(|e| e.status == LearnedEntryStatus::Promoted && matches!(e.kind, LearnedKind::Term))
        .map(|e| (e, score(e, bundle_id, now_ms)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (entry, _) in scored {
        let word = entry.word.trim().to_string();
        if !word.is_empty() && seen.insert(word.clone()) {
            result.push(word);
            if result.len() >= ENGINE_TERM_BUDGET {
                break;
            }
        }
    }

    result
}

/// Manual entries always precede learned entries so the budget is spent on
/// user-managed vocabulary first.
pub fn select_glossary_words(
    term_sets: &[NamedTermSet],
    set_ids: &[String],
    correction_sets: &[NamedCorrectionSet],
    correction_set_ids: &[String],
    learned: &[LearnedEntry],
    bundle_id: Option<&str>,
    now_ms: i64,
) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for id in set_ids {
        let Some(set) = term_sets.iter().find(|ts| &ts.id == id) else {
            continue;
        };
        for entry in &set.entries {
            let word = entry.trim().to_string();
            if !word.is_empty() && seen.insert(word.clone()) {
                result.push(word);
                if result.len() >= GLOSSARY_BUDGET {
                    return result;
                }
            }
        }
    }

    for id in correction_set_ids {
        let Some(set) = correction_sets.iter().find(|cs| &cs.id == id) else {
            continue;
        };
        for entry in &set.entries {
            let word = entry.to.trim().to_string();
            if !word.is_empty() && seen.insert(word.clone()) {
                result.push(word);
                if result.len() >= GLOSSARY_BUDGET {
                    return result;
                }
            }
        }
    }

    let mut scored: Vec<(&LearnedEntry, f64)> = learned
        .iter()
        .filter(|e| e.status == LearnedEntryStatus::Promoted)
        .map(|e| (e, score(e, bundle_id, now_ms)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (entry, _) in scored {
        let word = entry.word.trim().to_string();
        if !word.is_empty() && seen.insert(word.clone()) {
            result.push(word);
            if result.len() >= GLOSSARY_BUDGET {
                break;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CorrectionEntry, LearnedEntry, LearnedEntryStatus, LearnedKind, NamedCorrectionSet,
        NamedTermSet,
    };

    fn term_set(id: &str, words: &[&str]) -> NamedTermSet {
        NamedTermSet {
            id: id.to_string(),
            name: id.to_string(),
            entries: words.iter().map(|w| w.to_string()).collect(),
        }
    }

    fn corr_set(id: &str, pairs: &[(&str, &str)]) -> NamedCorrectionSet {
        NamedCorrectionSet {
            id: id.to_string(),
            name: id.to_string(),
            entries: pairs
                .iter()
                .map(|(f, t)| CorrectionEntry {
                    from: f.to_string(),
                    to: t.to_string(),
                })
                .collect(),
        }
    }

    fn promoted_term(word: &str, app: Option<&str>, total: u32, last_ms: i64) -> LearnedEntry {
        let mut per_app = std::collections::BTreeMap::new();
        if let Some(id) = app {
            per_app.insert(id.to_string(), total);
        }
        LearnedEntry {
            id: format!("id-{word}"),
            word: word.to_string(),
            kind: LearnedKind::Term,
            status: LearnedEntryStatus::Promoted,
            total_observations: total,
            last_observed_ms: last_ms,
            per_app_observations: per_app,
        }
    }

    fn promoted_correction(
        word: &str,
        from: &str,
        app: Option<&str>,
        total: u32,
        last_ms: i64,
    ) -> LearnedEntry {
        let mut per_app = std::collections::BTreeMap::new();
        if let Some(id) = app {
            per_app.insert(id.to_string(), total);
        }
        LearnedEntry {
            id: format!("id-{word}"),
            word: word.to_string(),
            kind: LearnedKind::Correction {
                from: from.to_string(),
            },
            status: LearnedEntryStatus::Promoted,
            total_observations: total,
            last_observed_ms: last_ms,
            per_app_observations: per_app,
        }
    }

    fn candidate_term(word: &str) -> LearnedEntry {
        LearnedEntry {
            id: format!("id-{word}"),
            word: word.to_string(),
            kind: LearnedKind::Term,
            status: LearnedEntryStatus::Candidate,
            total_observations: 1,
            last_observed_ms: 1000,
            per_app_observations: Default::default(),
        }
    }

    const NOW: i64 = 1_000_000;

    // ── select_terms ──────────────────────────────────────────────────────────

    #[test]
    fn select_terms_empty_inputs_returns_empty() {
        assert!(select_terms(&[], &[], &[], None, NOW).is_empty());
    }

    #[test]
    fn select_terms_manual_entries_returned_first() {
        let sets = vec![term_set("s1", &["Manual"])];
        let learned = vec![promoted_term("Learned", None, 5, NOW)];
        let result = select_terms(&sets, &["s1".to_string()], &learned, None, NOW);
        assert_eq!(result[0], "Manual");
        assert_eq!(result[1], "Learned");
    }

    #[test]
    fn select_terms_candidate_entries_excluded() {
        let learned = vec![candidate_term("ShouldBeExcluded")];
        let result = select_terms(&[], &[], &learned, None, NOW);
        assert!(result.is_empty());
    }

    #[test]
    fn select_terms_engine_budget_never_exceeded() {
        let learned: Vec<LearnedEntry> = (0..60)
            .map(|i| promoted_term(&format!("term{i}"), None, i as u32 + 1, NOW))
            .collect();
        let result = select_terms(&[], &[], &learned, None, NOW);
        assert!(
            result.len() <= ENGINE_TERM_BUDGET,
            "budget exceeded: {}",
            result.len()
        );
    }

    #[test]
    fn select_terms_manual_fills_budget_before_learned() {
        let words: Vec<&str> = (0..ENGINE_TERM_BUDGET).map(|_| "ManualWord").collect();
        let sets = vec![NamedTermSet {
            id: "s1".to_string(),
            name: "s1".to_string(),
            entries: (0..ENGINE_TERM_BUDGET)
                .map(|i| format!("Manual{i}"))
                .collect(),
        }];
        let learned = vec![promoted_term("LearnedWord", None, 5, NOW)];
        let _ = words; // silence unused
        let result = select_terms(&sets, &["s1".to_string()], &learned, None, NOW);
        assert_eq!(result.len(), ENGINE_TERM_BUDGET);
        assert!(!result.contains(&"LearnedWord".to_string()));
    }

    #[test]
    fn select_terms_per_app_frequency_reorders_learned() {
        let learned = vec![
            promoted_term("OtherTerm", Some("com.otherapp"), 5, NOW),
            promoted_term("AppTerm", Some("com.myapp"), 5, NOW),
        ];
        let result = select_terms(&[], &[], &learned, Some("com.myapp"), NOW);
        assert_eq!(result[0], "AppTerm", "app-local term should rank first");
        assert_eq!(result[1], "OtherTerm");
    }

    #[test]
    fn select_terms_no_bundle_id_uses_recency_only() {
        let older = promoted_term("OlderTerm", None, 5, NOW - 1_000_000);
        let newer = promoted_term("NewerTerm", None, 5, NOW - 100);
        let learned = vec![older, newer];
        let result = select_terms(&[], &[], &learned, None, NOW);
        assert_eq!(result[0], "NewerTerm");
    }

    #[test]
    fn select_terms_deduplicates_across_manual_and_learned() {
        let sets = vec![term_set("s1", &["MongoDB"])];
        let learned = vec![promoted_term("MongoDB", None, 5, NOW)];
        let result = select_terms(&sets, &["s1".to_string()], &learned, None, NOW);
        assert_eq!(result, vec!["MongoDB"]);
    }

    // ── select_glossary_words ─────────────────────────────────────────────────

    #[test]
    fn select_glossary_includes_manual_terms() {
        let sets = vec![term_set("s1", &["Tauri"])];
        let result = select_glossary_words(&sets, &["s1".to_string()], &[], &[], &[], None, NOW);
        assert!(result.contains(&"Tauri".to_string()));
    }

    #[test]
    fn select_glossary_includes_correction_targets() {
        let corr = vec![corr_set("c1", &[("tauri", "Tauri")])];
        let result = select_glossary_words(&[], &[], &corr, &["c1".to_string()], &[], None, NOW);
        assert!(result.contains(&"Tauri".to_string()));
    }

    #[test]
    fn select_glossary_includes_promoted_learned_terms() {
        let learned = vec![promoted_term("MongoDB", None, 5, NOW)];
        let result = select_glossary_words(&[], &[], &[], &[], &learned, None, NOW);
        assert!(result.contains(&"MongoDB".to_string()));
    }

    #[test]
    fn select_glossary_includes_promoted_learned_corrections() {
        let learned = vec![promoted_correction(
            "TypeScript",
            "typescript",
            None,
            5,
            NOW,
        )];
        let result = select_glossary_words(&[], &[], &[], &[], &learned, None, NOW);
        assert!(result.contains(&"TypeScript".to_string()));
    }

    #[test]
    fn select_glossary_excludes_candidates() {
        let learned = vec![candidate_term("ShouldBeExcluded")];
        let result = select_glossary_words(&[], &[], &[], &[], &learned, None, NOW);
        assert!(result.is_empty());
    }

    #[test]
    fn select_glossary_budget_never_exceeded() {
        let learned: Vec<LearnedEntry> = (0..250)
            .map(|i| promoted_term(&format!("word{i}"), None, i as u32 + 1, NOW))
            .collect();
        let result = select_glossary_words(&[], &[], &[], &[], &learned, None, NOW);
        assert!(
            result.len() <= GLOSSARY_BUDGET,
            "budget exceeded: {}",
            result.len()
        );
    }

    #[test]
    fn select_glossary_manual_precedes_learned() {
        let sets = vec![term_set("s1", &["Manual"])];
        let learned = vec![promoted_term("Learned", None, 5, NOW)];
        let result =
            select_glossary_words(&sets, &["s1".to_string()], &[], &[], &learned, None, NOW);
        let manual_pos = result.iter().position(|w| w == "Manual").unwrap();
        let learned_pos = result.iter().position(|w| w == "Learned").unwrap();
        assert!(manual_pos < learned_pos);
    }

    #[test]
    fn select_glossary_deduplicates() {
        let sets = vec![term_set("s1", &["TypeScript"])];
        let learned = vec![promoted_term("TypeScript", None, 5, NOW)];
        let result =
            select_glossary_words(&sets, &["s1".to_string()], &[], &[], &learned, None, NOW);
        assert_eq!(result.iter().filter(|w| *w == "TypeScript").count(), 1);
    }

    #[test]
    fn select_glossary_per_app_frequency_reorders_learned() {
        let learned = vec![
            promoted_term("OtherTerm", Some("com.otherapp"), 5, NOW),
            promoted_term("AppTerm", Some("com.myapp"), 5, NOW),
        ];
        let result = select_glossary_words(&[], &[], &[], &[], &learned, Some("com.myapp"), NOW);
        let app_pos = result.iter().position(|w| w == "AppTerm").unwrap();
        let other_pos = result.iter().position(|w| w == "OtherTerm").unwrap();
        assert!(app_pos < other_pos);
    }
}
