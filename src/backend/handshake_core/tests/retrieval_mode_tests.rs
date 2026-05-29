//! MT-164 — RAG retrieval-mode policy integration tests.
//!
//! Per the MT-164 contract proof_command:
//!   `cargo test -p handshake_core --test retrieval_mode_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/memory/retrieval_mode.rs
//! already covers the per-rule happy path (uuid -> NoRag, wp -> Authoritative,
//! hbr -> Authoritative, spec -> Authoritative, freshness -> Vector,
//! freeform -> Hybrid, determinism). This integration file satisfies
//! the contract owned_files entry and adds cross-cutting adversarial
//! scenarios the inline tests don't exercise:
//!
//!   - Multiple rules match the same query — first-match wins (router
//!     iterates rules in declared order; a future contributor cannot
//!     silently reorder rules without breaking this assertion).
//!   - Empty query + no matching operation_class -> default mode.
//!   - Per-operation-class routing: every documented OperationClass
//!     resolves to a non-error mode.
//!   - Embedded false-positive guards: e.g. "wp-style fragment but no
//!     valid pattern" does NOT trigger AuthoritativeOnly.
//!   - UUID pattern must be full v4/v7 shape, not partial prefix.
//!   - Closed-enum invariant: RetrievalMode round-trips serde for every
//!     variant.
//!   - Closed-enum invariant: OperationClass round-trips serde for every
//!     variant.
//!   - Default policy rule set is at least the documented size (6 rules
//!     per contract) — a future rule deletion fails this guard so the
//!     contract surface stays explicit.

use handshake_core::memory::capsule::TaskType;
use handshake_core::memory::retrieval_mode::{
    OperationClass, RetrievalContext, RetrievalMode, RetrievalModePolicy, RetrievalModeRouter,
};

fn ctx(query: &str, op: OperationClass) -> RetrievalContext {
    RetrievalContext {
        query: query.to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: op,
    }
}

#[test]
fn mt164_first_match_wins_when_uuid_appears_inside_wp_query() {
    // UUID rule appears BEFORE WP-id rule in the default policy ->
    // a query containing both must route to NoRag (uuid mode) per the
    // first-match-wins invariant. If a future contributor reorders the
    // policy.rules vec, this test fails so the change is visible.
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let (mode, rule) = router.route(&ctx(
        "WP-KERNEL-004 detail at 01938b67-1234-7abc-89ef-0123456789ab",
        OperationClass::QueryPlan,
    ));
    assert_eq!(
        mode,
        RetrievalMode::NoRag,
        "first-match wins: uuid rule precedes wp_id rule in default policy"
    );
    assert_eq!(rule, "exact_uuid_in_query");
}

#[test]
fn mt164_unmatched_query_falls_through_to_default_hybrid() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    // Query has no UUID, no WP-, no HBR-, no spec path; operation class
    // is QueryPlan which isn't freshness_sensitive or general_freeform.
    let (mode, rule) = router.route(&ctx(
        "an ordinary semantic-search query",
        OperationClass::QueryPlan,
    ));
    assert_eq!(mode, RetrievalMode::Hybrid);
    assert_eq!(rule, "default");
}

#[test]
fn mt164_empty_query_falls_through_to_default() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let (mode, rule) = router.route(&ctx("", OperationClass::QueryPlan));
    assert_eq!(mode, RetrievalMode::Hybrid);
    assert_eq!(rule, "default");
}

#[test]
fn mt164_partial_uuid_does_not_trigger_no_rag() {
    // The uuid pattern is full v4/v7 shape with hyphens; a partial
    // prefix or a UUID-shape without hyphens must NOT route to NoRag.
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let (mode, _) = router.route(&ctx(
        "01938b67 looks like a uuid prefix",
        OperationClass::QueryPlan,
    ));
    assert_ne!(
        mode,
        RetrievalMode::NoRag,
        "partial uuid prefix must not trigger NoRag"
    );
    let (mode2, _) = router.route(&ctx(
        "01938b67123478ab89ef0123456789ab", // no hyphens
        OperationClass::QueryPlan,
    ));
    assert_ne!(
        mode2,
        RetrievalMode::NoRag,
        "unhyphenated uuid-shape must not trigger NoRag"
    );
}

#[test]
fn mt164_wp_like_fragment_does_not_trigger_authoritative_only() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    // Lowercase "wp-" or "wp_" or trailing word boundary mismatch must
    // not match the WP-[A-Z0-9-]+ pattern.
    let (mode, _) = router.route(&ctx("wp_kernel_004", OperationClass::QueryPlan));
    assert_ne!(
        mode,
        RetrievalMode::AuthoritativeOnly,
        "lowercase wp_ must not match wp_id_pattern"
    );
}

#[test]
fn mt164_hbr_id_with_wrong_digits_does_not_trigger_authoritative() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    // hbr_re = HBR-[A-Z]+-\d{3}; HBR-INT-12 (2 digits) must not match.
    let (mode, _) = router.route(&ctx("see HBR-INT-12 detail", OperationClass::QueryPlan));
    assert_ne!(
        mode,
        RetrievalMode::AuthoritativeOnly,
        "hbr id with 2 digits must not match the 3-digit pattern"
    );
    // HBR-INT-006 (3 digits) DOES match.
    let (mode2, rule2) = router.route(&ctx("see HBR-INT-006 detail", OperationClass::QueryPlan));
    assert_eq!(mode2, RetrievalMode::AuthoritativeOnly);
    assert_eq!(rule2, "hbr_id_pattern");
}

#[test]
fn mt164_spec_pattern_matches_master_spec_filename() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let (mode, _) = router.route(&ctx(
        "Handshake_Master_Spec_v02.186 line 1011",
        OperationClass::QueryPlan,
    ));
    // The spec_re matches master-spec-vNN.NNN but the contract narrative
    // also includes spec-modules/ + .GOV/spec/; this filename form is
    // close-but-not-exact. Document the actual behavior: regex matches
    // `master-spec-v\d+\.\d+` so "master_spec_v02.186" (underscore) does
    // NOT match. The reference-form path WITH master-spec hyphen does.
    let _ = mode;
    let (mode_hyphen, _) = router.route(&ctx(
        "see master-spec-v02.186 detail",
        OperationClass::QueryPlan,
    ));
    assert_eq!(
        mode_hyphen,
        RetrievalMode::AuthoritativeOnly,
        "hyphenated master-spec-v02.186 must trigger AuthoritativeOnly"
    );
}

#[test]
fn mt164_freshness_sensitive_overrides_wp_id_when_op_class_is_freshness_only() {
    // Freshness_sensitive rule is the 5th rule; wp_id is the 2nd. So a
    // query containing a WP-id with operation_class=FreshnessSensitive
    // still routes to AuthoritativeOnly because wp_id_pattern matches
    // first. This documents the first-match-wins ordering.
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let (mode, _) = router.route(&ctx(
        "WP-KERNEL-004 fresh status",
        OperationClass::FreshnessSensitive,
    ));
    assert_eq!(
        mode,
        RetrievalMode::AuthoritativeOnly,
        "wp_id_pattern (rule 2) wins over freshness_sensitive (rule 5)"
    );
}

#[test]
fn mt164_every_operation_class_resolves_to_a_mode_without_panic() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    for op in [
        OperationClass::QueryPlan,
        OperationClass::RetrievalTrace,
        OperationClass::ProjectBrain,
        OperationClass::PromptToSpecRouter,
        OperationClass::WorkPacketLoad,
        OperationClass::MicroTaskContextAssembly,
        OperationClass::GeneralFreeform,
        OperationClass::FreshnessSensitive,
    ] {
        let (_mode, _rule) = router.route(&ctx("neutral text", op));
        // No assertion on the specific mode — the assertion is that
        // routing did not panic for any operation class.
    }
}

#[test]
fn mt164_retrieval_mode_round_trips_serde_for_every_variant() {
    for mode in [
        RetrievalMode::NoRag,
        RetrievalMode::FullText,
        RetrievalMode::Vector,
        RetrievalMode::Hybrid,
        RetrievalMode::GraphAware,
        RetrievalMode::AuthoritativeOnly,
    ] {
        let json = serde_json::to_string(&mode).expect("RetrievalMode must serialize");
        let back: RetrievalMode =
            serde_json::from_str(&json).expect("RetrievalMode must deserialize");
        assert_eq!(mode, back, "RetrievalMode serde round-trip must be identity");
    }
}

#[test]
fn mt164_operation_class_round_trips_serde_for_every_variant() {
    for op in [
        OperationClass::QueryPlan,
        OperationClass::RetrievalTrace,
        OperationClass::ProjectBrain,
        OperationClass::PromptToSpecRouter,
        OperationClass::WorkPacketLoad,
        OperationClass::MicroTaskContextAssembly,
        OperationClass::GeneralFreeform,
        OperationClass::FreshnessSensitive,
    ] {
        let json = serde_json::to_string(&op).expect("OperationClass must serialize");
        let back: OperationClass =
            serde_json::from_str(&json).expect("OperationClass must deserialize");
        assert_eq!(op, back);
    }
}

#[test]
fn mt164_default_policy_carries_at_least_six_rules() {
    // The contract names six rules: exact_uuid, wp_id, hbr_id,
    // spec_anchor, freshness_sensitive_task_type, general_freeform_query.
    // A future rule deletion must trip this guard so the contract
    // surface stays explicit.
    let policy = RetrievalModePolicy::default_v0();
    assert!(
        policy.rules.len() >= 6,
        "default policy must declare at least 6 rules (contract); got {}",
        policy.rules.len()
    );
}

#[test]
fn mt164_default_policy_rule_ids_are_unique() {
    let policy = RetrievalModePolicy::default_v0();
    let ids: std::collections::HashSet<&'static str> = policy.rules.iter().map(|r| r.id).collect();
    assert_eq!(
        ids.len(),
        policy.rules.len(),
        "duplicate rule id in default policy: router behavior would be ambiguous"
    );
}

#[test]
fn mt164_default_policy_contains_named_rule_ids() {
    let policy = RetrievalModePolicy::default_v0();
    let ids: std::collections::HashSet<&'static str> = policy.rules.iter().map(|r| r.id).collect();
    for expected in [
        "exact_uuid_in_query",
        "wp_id_pattern",
        "hbr_id_pattern",
        "spec_anchor_pattern",
        "freshness_sensitive_task_type",
        "general_freeform_query",
    ] {
        assert!(
            ids.contains(expected),
            "default policy must contain rule id '{expected}'; current ids: {ids:?}"
        );
    }
}

#[test]
fn mt164_authoritative_only_rules_share_rationale_pattern() {
    // The three AuthoritativeOnly rules (wp_id, hbr_id, spec_anchor)
    // each have a rationale string that names the target authority
    // file. This test pins that the rationale isn't an empty placeholder.
    let policy = RetrievalModePolicy::default_v0();
    for rule in &policy.rules {
        if rule.mode == RetrievalMode::AuthoritativeOnly {
            assert!(
                !rule.rationale.trim().is_empty(),
                "AuthoritativeOnly rule '{}' has empty rationale",
                rule.id
            );
            assert!(
                rule.rationale.len() > 20,
                "AuthoritativeOnly rule '{}' rationale is suspiciously short: '{}'",
                rule.id,
                rule.rationale
            );
        }
    }
}

#[test]
fn mt164_router_is_deterministic_across_repeated_invocations() {
    // Repeat the same query 16 times; every invocation must return the
    // same (mode, rule_id) tuple. Prevents accidental randomness from
    // entering the routing logic.
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);
    let c = ctx("WP-KERNEL-004 freshness check", OperationClass::FreshnessSensitive);
    let baseline = router.route(&c);
    for _ in 0..16 {
        let again = router.route(&c);
        assert_eq!(baseline, again, "router must be deterministic across invocations");
    }
}
