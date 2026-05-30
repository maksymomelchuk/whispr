//! Integration tests for the config migration path.
//!
//! Each test parses a representative legacy JSON fixture through
//! `config::from_json` (which deserialises then calls `migrate`) and asserts
//! the resulting `Settings` has the expected shape.  These complement the
//! fine-grained unit tests in `config.rs` by exercising realistic multi-field
//! blobs rather than surgically minimal JSON strings.
use whispr_lib::config::{
    self, GroqModel, ProviderModel, Settings, DEFAULT_CORRECTION_SET_ID, SEED_TERM_SET_DEFAULT_ID,
};

// ── Legacy dictionary → terms + corrections ─────────────────────────────────
// Entries where from == to become Terms; all others become Corrections.

#[test]
fn legacy_dictionary_splits_into_term_sets_and_corrections() {
    let json = r#"{
        "dictionary": [
            {"from": "MongoDB",  "to": "MongoDB"},
            {"from": "Kubernetes", "to": "Kubernetes"},
            {"from": "dot",      "to": "."},
            {"from": "anthropik","to": "Anthropic"}
        ]
    }"#;

    let s = config::from_json(json).unwrap();

    // from==to entries become the Default Terms set
    let default_set = s
        .term_sets
        .iter()
        .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
        .expect("Default Terms set must exist");
    assert!(default_set.entries.contains(&"MongoDB".to_string()));
    assert!(default_set.entries.contains(&"Kubernetes".to_string()));
    assert_eq!(default_set.entries.len(), 2);

    let default_set = s
        .correction_sets
        .iter()
        .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
        .expect("default correction set must exist after migration");
    assert_eq!(default_set.entries.len(), 2, "only from!=to entries become corrections");
    let dot = default_set.entries.iter().find(|c| c.from == "dot").expect("dot correction");
    assert_eq!(dot.to, ".");
    let fix = default_set
        .entries
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
    assert!(v.get("terms").is_none(), "terms is skip_serializing — absorbed into term_sets");
    assert!(v.get("term_sets").is_some(), "term_sets must appear in output");
}

// ── Legacy use_dictionary → use_terms + use_corrections ────────────────────

#[test]
fn legacy_use_dictionary_false_disables_corrections_and_leaves_term_set_ids_empty() {
    // A pre-split settings file has use_dictionary (no use_corrections / term_set_ids).
    // Setting it to false must disable use_corrections; no term sets are created since
    // there are no legacy terms to migrate.
    let json = r#"{
        "modes": [{
            "id":       "mode-default-en",
            "name":     "Default",
            "language": {"kind": "exact", "code": "en"},

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

    assert!(!mode.use_corrections, "use_corrections must be false when use_dictionary was false");
    assert!(mode.term_set_ids.is_empty(), "no legacy terms → no term_set_ids assigned");
    assert!(
        mode.correction_set_ids.is_empty(),
        "correction_set_ids must be empty when use_corrections was false"
    );
}

#[test]
fn legacy_use_dictionary_true_preserves_corrections_flag() {
    let json = r#"{
        "modes": [{
            "id":       "mode-default-en",
            "name":     "Default",
            "language": {"kind": "exact", "code": "en"},

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

    assert!(mode.use_corrections, "use_corrections must remain true when use_dictionary was true");
    // No legacy terms in this JSON, so no Default Terms set is created.
    assert!(mode.term_set_ids.is_empty(), "no legacy terms → no term_set_ids assigned");
    assert!(
        mode.correction_set_ids.contains(&DEFAULT_CORRECTION_SET_ID.to_string()),
        "correction_set_ids must contain the default set when use_corrections was true"
    );
}

// ── Legacy corrections → named correction set ───────────────────────────────

#[test]
fn legacy_corrections_field_seeds_default_correction_set() {
    let json = r#"{
        "corrections": [
            {"from": "mongo", "to": "MongoDB"},
            {"from": "js",    "to": "JavaScript"}
        ]
    }"#;

    let s = config::from_json(json).unwrap();

    let default_set = s
        .correction_sets
        .iter()
        .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
        .expect("default correction set must exist after migration");
    assert_eq!(default_set.entries.len(), 2);
    assert!(default_set.entries.iter().any(|e| e.from == "mongo" && e.to == "MongoDB"));
    assert!(default_set.entries.iter().any(|e| e.from == "js" && e.to == "JavaScript"));
}

#[test]
fn corrections_field_absent_from_reserialized_output() {
    let json = r#"{"corrections": [{"from": "dot", "to": "."}]}"#;
    let s = config::from_json(json).unwrap();
    let reserialized = serde_json::to_string(&s).unwrap();
    let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();

    assert!(v.get("corrections").is_none(), "legacy corrections field must not appear after migration");
    assert!(v.get("correction_sets").is_some(), "correction_sets must appear in output");
}

// ── Combined pre-umbrella-73 migration ──────────────────────────────────────

#[test]
fn full_pre_issue_73_migration_preserves_behavior() {
    // Realistic settings.json from before all three umbrella slices (#74/#75/#76):
    // - Groq selected with whisper-large-v3, API key set
    // - Flat legacy `terms` list
    // - Flat legacy `corrections` list
    // - Mode with use_terms=true and use_corrections=true
    let json = r#"{
        "transcription_provider": "groq",
        "groq": {"model": "whisper_large_v3"},
        "groq_api_key": "sk-groq-test",
        "terms": ["MongoDB", "Kubernetes"],
        "corrections": [
            {"from": "anthropik", "to": "Anthropic"},
            {"from": "dot",       "to": "."}
        ],
        "modes": [{
            "id":       "mode-default-en",
            "name":     "Default",
            "language": {"kind": "exact", "code": "en"},

            "ai_cleanup":{"enabled": false, "prompt_override": null},
            "use_terms": true,
            "use_corrections": true,
            "use_snippets": true
        }],
        "default_mode_id": "mode-default-en"
    }"#;

    let s = config::from_json(json).unwrap();
    let mode = s.modes.iter().find(|m| m.id == "mode-default-en").expect("mode must be present");

    assert_eq!(
        mode.provider_model,
        ProviderModel::Groq { model: GroqModel::WhisperLargeV3 },
        "mode must carry groq+whisper-large-v3 after migration"
    );

    let term_set = s
        .term_sets
        .iter()
        .find(|ts| ts.id == SEED_TERM_SET_DEFAULT_ID)
        .expect("Default Terms set must exist");
    assert!(term_set.entries.contains(&"MongoDB".to_string()));
    assert!(term_set.entries.contains(&"Kubernetes".to_string()));
    assert!(
        mode.term_set_ids.contains(&SEED_TERM_SET_DEFAULT_ID.to_string()),
        "mode must reference the default term set"
    );

    let correction_set = s
        .correction_sets
        .iter()
        .find(|cs| cs.id == DEFAULT_CORRECTION_SET_ID)
        .expect("Default Corrections set must exist");
    assert!(correction_set.entries.iter().any(|e| e.from == "anthropik" && e.to == "Anthropic"));
    assert!(correction_set.entries.iter().any(|e| e.from == "dot" && e.to == "."));
    assert!(
        mode.correction_set_ids.contains(&DEFAULT_CORRECTION_SET_ID.to_string()),
        "mode must reference the default correction set"
    );

    let reserialized = serde_json::to_string(&s).unwrap();
    let v: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
    assert!(v.get("transcription_provider").is_none(), "transcription_provider must not appear");
    assert!(v.get("terms").is_none(), "legacy terms must not appear");
    assert!(v.get("corrections").is_none(), "legacy corrections must not appear");

    let mode_v = v["modes"].as_array().unwrap().iter().find(|m| m["id"] == "mode-default-en").unwrap();
    assert!(mode_v.get("use_terms").is_none(), "use_terms must not appear on mode");
    assert!(mode_v.get("use_corrections").is_none(), "use_corrections must not appear on mode");
    assert_eq!(mode_v["provider_model"]["provider"], "groq", "provider_model must serialize to groq on mode");
}

// ── UA→EN translation prompt backfill ──────────────────────────────────────

#[test]
fn old_ua_en_with_no_prompt_override_gets_translation_prompt_on_migration() {
    // Pre-issue-#90 configs carried translation via Apple Translate; the per-mode
    // ai_cleanup had enabled=true and prompt_override=null. The `translate` field
    // is silently dropped during deserialisation, so migration must backfill the
    // translation prompt to preserve the mode's original intent.
    let json = r#"{
        "modes": [{
            "id": "mode-ua-en",
            "name": "UA → EN",
            "language": {"kind": "exact", "code": "uk"},
            "translate": {"kind": "apple", "target": "en"},
            "ai_cleanup": {"enabled": true, "prompt_override": null},
            "use_snippets": true
        }],
        "default_mode_id": "mode-ua-en"
    }"#;

    let s = config::from_json(json).unwrap();
    let ua_en = s.modes.iter().find(|m| m.id == "mode-ua-en").expect("mode must survive migration");

    assert!(ua_en.ai_cleanup.enabled);
    assert!(
        ua_en.ai_cleanup.prompt_override.as_deref().unwrap_or("").contains("Ukrainian"),
        "migration must set the translation prompt on an old mode-ua-en"
    );
}

#[test]
fn old_ua_en_with_custom_prompt_is_not_overwritten_on_migration() {
    let json = r#"{
        "modes": [{
            "id": "mode-ua-en",
            "name": "UA → EN",
            "language": {"kind": "exact", "code": "uk"},
            "ai_cleanup": {"enabled": true, "prompt_override": "My custom prompt"},
            "use_snippets": true
        }],
        "default_mode_id": "mode-ua-en"
    }"#;

    let s = config::from_json(json).unwrap();
    let ua_en = s.modes.iter().find(|m| m.id == "mode-ua-en").expect("mode must survive migration");

    assert_eq!(
        ua_en.ai_cleanup.prompt_override.as_deref(),
        Some("My custom prompt"),
        "user-supplied prompt_override must not be overwritten by migration"
    );
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
