//! Integration tests for the config migration path.
//!
//! Each test parses a representative legacy JSON fixture through
//! `config::from_json` (which deserialises then calls `migrate`) and asserts
//! the resulting `Settings` has the expected shape.  These complement the
//! fine-grained unit tests in `config.rs` by exercising realistic multi-field
//! blobs rather than surgically minimal JSON strings.
use whispr_lib::config::{self, Settings};

// ── Legacy dictionary → terms + corrections ─────────────────────────────────
// Entries where from == to become Terms; all others become Corrections.

#[test]
fn legacy_dictionary_splits_into_terms_and_corrections() {
    let json = r#"{
        "dictionary": [
            {"from": "MongoDB",  "to": "MongoDB"},
            {"from": "Kubernetes", "to": "Kubernetes"},
            {"from": "dot",      "to": "."},
            {"from": "anthropik","to": "Anthropic"}
        ]
    }"#;

    let s = config::from_json(json).unwrap();

    assert!(s.terms.contains(&"MongoDB".to_string()));
    assert!(s.terms.contains(&"Kubernetes".to_string()));
    assert_eq!(s.terms.len(), 2, "only from==to entries become terms");

    assert_eq!(s.corrections.len(), 2, "only from!=to entries become corrections");
    let dot = s.corrections.iter().find(|c| c.from == "dot").expect("dot correction");
    assert_eq!(dot.to, ".");
    let fix = s
        .corrections
        .iter()
        .find(|c| c.from == "anthropik")
        .expect("anthropik correction");
    assert_eq!(fix.to, "Anthropic");
}

#[test]
fn legacy_dictionary_fields_absent_from_reserialized_output() {
    let json = r#"{"dictionary": [{"from": "MongoDB", "to": "MongoDB"}]}"#;
    let s = config::from_json(json).unwrap();
    let reserialized = serde_json::to_string(&s).unwrap();
    let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();

    assert!(v.get("dictionary").is_none(), "legacy field must not appear after migration");
    assert!(v.get("replacements").is_none(), "legacy field must not appear after migration");
    assert!(v.get("terms").is_some(), "migrated terms must appear in output");
}

// ── Legacy use_dictionary → use_terms + use_corrections ────────────────────

#[test]
fn legacy_use_dictionary_false_disables_both_term_flags() {
    // A pre-split settings file has use_dictionary (no use_terms / use_corrections).
    // Setting it to false must disable both successor flags.
    let json = r#"{
        "modes": [{
            "id":       "mode-default-en",
            "name":     "Default",
            "language": {"kind": "exact", "code": "en"},
            "translate":{"kind": "off"},
            "ai_cleanup":{"enabled": false, "prompt_override": null},
            "use_dictionary": false,
            "use_snippets":   true
        }],
        "default_mode_id": "mode-default-en"
    }"#;

    let s = config::from_json(json).unwrap();
    let mode = s
        .modes
        .iter()
        .find(|m| m.id == "mode-default-en")
        .expect("mode must be present after migration");

    assert!(!mode.use_terms, "use_terms must be false when use_dictionary was false");
    assert!(!mode.use_corrections, "use_corrections must be false when use_dictionary was false");
}

#[test]
fn legacy_use_dictionary_true_preserves_both_term_flags() {
    let json = r#"{
        "modes": [{
            "id":       "mode-default-en",
            "name":     "Default",
            "language": {"kind": "exact", "code": "en"},
            "translate":{"kind": "off"},
            "ai_cleanup":{"enabled": false, "prompt_override": null},
            "use_dictionary": true,
            "use_snippets":   true
        }],
        "default_mode_id": "mode-default-en"
    }"#;

    let s = config::from_json(json).unwrap();
    let mode = s
        .modes
        .iter()
        .find(|m| m.id == "mode-default-en")
        .expect("mode must be present after migration");

    assert!(mode.use_terms, "use_terms must remain true when use_dictionary was true");
    assert!(mode.use_corrections, "use_corrections must remain true when use_dictionary was true");
}

// ── Current-shape config round-trips without modification ───────────────────

#[test]
fn current_shape_config_round_trips_without_modification() {
    // Serialise a default Settings (already in current shape — post migration),
    // parse it back, and verify that a second migration pass is a no-op and
    // the re-serialised JSON is byte-for-byte identical.
    let original = Settings::default();
    let json = serde_json::to_string(&original).unwrap();

    let round_tripped = config::from_json(&json).unwrap();
    let reserialized = serde_json::to_string(&round_tripped).unwrap();

    assert_eq!(
        json, reserialized,
        "round-trip through from_json must not modify a current-shape config"
    );
}
