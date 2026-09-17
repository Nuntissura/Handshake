use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use handshake_core::hbr::applicability::{
    Applicability, HbrApplicability, HbrNaOverride, PacketContext,
};
use handshake_core::hbr::registry::{HbrPillar, HbrRegistry};
use tempfile::NamedTempFile;

/// Fixture copy of `HANDSHAKE_BUILD_RULES.json` v1.11.0 (content-as-data, AC-141-1
/// exemption b). The product test tree MUST NOT read the live governance kernel at
/// runtime ([CX-211]): the registry copy under `tests/fixtures/hbr/` binds this proof
/// to the committed tree instead of to whatever the governance junction currently points at.
/// When the live registry gains rules, refresh the fixture and the counts below together.
fn hbr_registry_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("hbr")
        .join("HANDSHAKE_BUILD_RULES.json");
    assert!(
        path.exists(),
        "missing HBR registry fixture at {}",
        path.display()
    );
    path
}

#[test]
fn hbr_registry_tests_live_registry_loads_active_rules_and_distribution() {
    let registry =
        HbrRegistry::load_from_path(&hbr_registry_path()).expect("HBR registry fixture loads");
    let active_rules = registry.active_rules().collect::<Vec<_>>();

    // Registry v1.11.0: 41 ACTIVE rules across the seven pillars, including the
    // PRIV pillar (account-bound resource privacy, Codex [CX-132]).
    assert_eq!(registry.version, "1.11.0");
    assert_eq!(active_rules.len(), 41);

    let mut distribution: HashMap<HbrPillar, usize> = HashMap::new();
    for rule in active_rules {
        *distribution.entry(rule.pillar).or_default() += 1;
    }

    assert_eq!(distribution.get(&HbrPillar::Int), Some(&9));
    assert_eq!(distribution.get(&HbrPillar::Swarm), Some(&6));
    assert_eq!(distribution.get(&HbrPillar::Vis), Some(&5));
    assert_eq!(distribution.get(&HbrPillar::Quiet), Some(&4));
    assert_eq!(distribution.get(&HbrPillar::Man), Some(&4));
    assert_eq!(distribution.get(&HbrPillar::Stop), Some(&5));
    assert_eq!(distribution.get(&HbrPillar::Priv), Some(&8));
}

#[test]
fn hbr_registry_tests_applicability_matches_declared_tags_and_touched_path_globs() {
    let registry =
        HbrRegistry::load_from_path(&hbr_registry_path()).expect("HBR registry fixture loads");

    let observable_rule = registry.rule("HBR-INT-001").expect("HBR-INT-001 exists");
    let declared_tag_context = PacketContext {
        wp_id: "WP-TEST-HBR".to_string(),
        touched_paths: Vec::new(),
        tags_declared: vec!["observable_behavior".to_string()],
        not_applicable_overrides: Vec::new(),
    };
    assert_eq!(
        HbrApplicability::evaluate(observable_rule, &declared_tag_context),
        Applicability::Applicable,
    );

    let crdt_rule = registry.rule("HBR-INT-004").expect("HBR-INT-004 exists");
    let touched_path_context = PacketContext {
        wp_id: "WP-TEST-HBR".to_string(),
        touched_paths: vec![PathBuf::from(
            "src/backend/handshake_core/src/kernel/crdt/identity.rs",
        )],
        tags_declared: Vec::new(),
        not_applicable_overrides: Vec::new(),
    };
    assert_eq!(
        HbrApplicability::evaluate(crdt_rule, &touched_path_context),
        Applicability::Applicable,
    );
}

#[test]
fn hbr_registry_tests_not_applicable_override_returns_non_empty_reason() {
    let registry =
        HbrRegistry::load_from_path(&hbr_registry_path()).expect("HBR registry fixture loads");
    let rule = registry.rule("HBR-INT-001").expect("HBR-INT-001 exists");
    let context = PacketContext {
        wp_id: "WP-TEST-HBR".to_string(),
        touched_paths: Vec::new(),
        tags_declared: vec!["observable_behavior".to_string()],
        not_applicable_overrides: vec![HbrNaOverride {
            hbr_id: "HBR-INT-001".to_string(),
            reason: "Existing covered evidence applies without new product behavior.".to_string(),
        }],
    };

    match HbrApplicability::evaluate(rule, &context) {
        Applicability::NotApplicable { reason } => assert!(!reason.trim().is_empty()),
        Applicability::Applicable => panic!("matching override must return NotApplicable"),
    }
}

#[test]
fn hbr_registry_tests_foreground_required_tag_makes_quiet_004_applicable() {
    let registry =
        HbrRegistry::load_from_path(&hbr_registry_path()).expect("HBR registry fixture loads");
    let rule = registry
        .rule("HBR-QUIET-004")
        .expect("HBR-QUIET-004 exists");
    let context = PacketContext {
        wp_id: "WP-TEST-HBR-FOREGROUND".to_string(),
        touched_paths: Vec::new(),
        tags_declared: vec!["foreground_required".to_string()],
        not_applicable_overrides: Vec::new(),
    };

    assert_eq!(
        HbrApplicability::evaluate(rule, &context),
        Applicability::Applicable,
    );
}

#[test]
fn hbr_registry_tests_registry_rejects_tampered_schema() {
    let mut registry_json =
        fs::read_to_string(hbr_registry_path()).expect("registry fixture readable");
    registry_json = registry_json.replace("handshake.build_rules@1", "handshake.build_rules@0");
    let temp = NamedTempFile::new().expect("temp registry file");
    fs::write(temp.path(), registry_json).expect("tampered registry written");

    let error = HbrRegistry::load_from_path(temp.path()).expect_err("tampered schema is rejected");
    assert!(error.to_string().contains("schema"));
}
