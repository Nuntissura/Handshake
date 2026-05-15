use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    session_anti_pattern_registry::{
        project_session_anti_pattern_registry, validate_session_anti_pattern_registry,
        DetectionSourceKind, PolicyOutcome, SessionAntiPatternCoverage,
        SessionAntiPatternDetectionV1, SessionAntiPatternDomain, SessionAntiPatternEntryV1,
        SessionAntiPatternRegistryV1,
    },
};

#[test]
fn kernel_session_anti_pattern_registry_covers_required_domains_and_outcomes() {
    let registry = sample_registry();
    validate_session_anti_pattern_registry(&registry).expect("registry validates");

    let projection = project_session_anti_pattern_registry(&registry).expect("projection builds");

    assert_eq!(projection.registry_id, "session-anti-pattern-mt041");
    assert!(projection.machine_readable_detections);
    assert!(projection
        .covered_domains
        .contains(&SessionAntiPatternDomain::Scheduler));
    assert!(projection
        .covered_domains
        .contains(&SessionAntiPatternDomain::TrustBoundary));
    assert!(projection
        .covered_domains
        .contains(&SessionAntiPatternDomain::CapabilityGate));
    assert!(projection
        .covered_domains
        .contains(&SessionAntiPatternDomain::SessionOrchestration));
    assert!(projection.covered_outcomes.contains(&PolicyOutcome::Deny));
    assert!(projection
        .covered_outcomes
        .contains(&PolicyOutcome::Downgrade));
    assert!(projection
        .covered_outcomes
        .contains(&PolicyOutcome::RequireConsent));
    assert!(projection
        .covered_outcomes
        .contains(&PolicyOutcome::ForceStop));
    assert!(projection
        .force_stop_entry_ids
        .contains(&"sap.scheduler.spawn-abuse".to_string()));
    assert!(!projection.mutates_session);
}

#[test]
fn kernel_session_anti_pattern_registry_projects_required_risk_coverage() {
    let projection =
        project_session_anti_pattern_registry(&sample_registry()).expect("projection builds");

    assert!(projection
        .covered_risks
        .contains(&SessionAntiPatternCoverage::SpawnAbuse));
    assert!(projection
        .covered_risks
        .contains(&SessionAntiPatternCoverage::TrustBoundaryViolation));
    assert!(projection
        .covered_risks
        .contains(&SessionAntiPatternCoverage::UnauthorizedEscalation));
    assert!(projection
        .required_flight_recorder_refs
        .iter()
        .all(|event_ref| event_ref.starts_with("FR-EVT-SESSION-")));
}

#[test]
fn kernel_session_anti_pattern_registry_rejects_underspecified_or_nondeterministic_rules() {
    let mut registry = sample_registry();
    registry.entries[0].entry_id = registry.entries[1].entry_id.clone();
    registry.entries[0].detections[0].deterministic = false;
    registry.entries[1].required_flight_recorder_refs.clear();
    registry.entries[2].policy_outcome = PolicyOutcome::Deny;
    registry.entries[3].coverage_tags.clear();

    let errors =
        validate_session_anti_pattern_registry(&registry).expect_err("unsafe registry must fail");

    assert!(errors.iter().any(|error| error.field == "entries.entry_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "entries.detections.deterministic"));
    assert!(errors
        .iter()
        .any(|error| error.field == "entries.required_flight_recorder_refs"));
    assert!(errors
        .iter()
        .any(|error| error.field == "entries.coverage_tags"));
    assert!(errors
        .iter()
        .any(|error| error.field == "entries.policy_outcome"));
}

#[test]
fn kernel_session_anti_pattern_registry_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.session_anti_pattern_registry.project")
        .expect("session anti-pattern registry projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "session_anti_pattern_policy_outcomes"));
}

fn sample_registry() -> SessionAntiPatternRegistryV1 {
    SessionAntiPatternRegistryV1 {
        schema_id: "hsk.kernel.session_anti_pattern_registry@1".to_string(),
        registry_id: "session-anti-pattern-mt041".to_string(),
        version: 1,
        folded_stub_ids: vec!["WP-1-Session-Anti-Pattern-Registry-v1".to_string()],
        entries: vec![
            entry(
                "sap.scheduler.spawn-abuse",
                SessionAntiPatternDomain::Scheduler,
                PolicyOutcome::ForceStop,
                SessionAntiPatternCoverage::SpawnAbuse,
                DetectionSourceKind::SchedulerCheck,
                "FR-EVT-SESSION-SPAWN-ABUSE",
            ),
            entry(
                "sap.trust.inbound-boundary-bypass",
                SessionAntiPatternDomain::TrustBoundary,
                PolicyOutcome::Deny,
                SessionAntiPatternCoverage::TrustBoundaryViolation,
                DetectionSourceKind::TrustBoundaryValidation,
                "FR-EVT-SESSION-TRUST-BOUNDARY",
            ),
            entry(
                "sap.capability.unauthorized-escalation",
                SessionAntiPatternDomain::CapabilityGate,
                PolicyOutcome::Downgrade,
                SessionAntiPatternCoverage::UnauthorizedEscalation,
                DetectionSourceKind::CapabilityGateOutcome,
                "FR-EVT-SESSION-CAPABILITY-ESCALATION",
            ),
            entry(
                "sap.session.cross-workspace-routing",
                SessionAntiPatternDomain::SessionOrchestration,
                PolicyOutcome::RequireConsent,
                SessionAntiPatternCoverage::TrustBoundaryViolation,
                DetectionSourceKind::SessionLifecycleRecord,
                "FR-EVT-SESSION-CROSS-WORKSPACE",
            ),
        ],
        product_authority_refs: vec![
            "kernel.role_turn_isolation".to_string(),
            "kernel.work_profiles".to_string(),
            "kernel.local_first_mcp_posture".to_string(),
            "flight_recorder.session_orchestration".to_string(),
            "capability_gate.session".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Session-Anti-Pattern-Registry-v1.contract.json"
                .to_string(),
        ],
    }
}

fn entry(
    entry_id: &str,
    domain: SessionAntiPatternDomain,
    policy_outcome: PolicyOutcome,
    coverage: SessionAntiPatternCoverage,
    detection_source: DetectionSourceKind,
    flight_recorder_ref: &str,
) -> SessionAntiPatternEntryV1 {
    SessionAntiPatternEntryV1 {
        entry_id: entry_id.to_string(),
        title: format!("Anti-pattern {entry_id}"),
        domain,
        severity: "HIGH".to_string(),
        trigger_condition: format!("{entry_id} trigger condition is observed"),
        detections: vec![SessionAntiPatternDetectionV1 {
            source_id: format!("detector.{entry_id}"),
            source_kind: detection_source,
            signal_ref: format!("signal.{entry_id}"),
            deterministic: true,
        }],
        policy_outcome,
        coverage_tags: vec![coverage],
        required_flight_recorder_refs: vec![flight_recorder_ref.to_string()],
    }
}
