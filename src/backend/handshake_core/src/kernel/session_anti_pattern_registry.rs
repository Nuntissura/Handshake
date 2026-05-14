use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_SESSION_ANTI_PATTERN_REGISTRY_STUB_ID: &str =
    "WP-1-Session-Anti-Pattern-Registry-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionAntiPatternDomain {
    Scheduler,
    TrustBoundary,
    CapabilityGate,
    SessionOrchestration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetectionSourceKind {
    SchedulerCheck,
    TrustBoundaryValidation,
    CapabilityGateOutcome,
    SessionLifecycleRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyOutcome {
    Deny,
    Downgrade,
    RequireConsent,
    ForceStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionAntiPatternCoverage {
    SpawnAbuse,
    TrustBoundaryViolation,
    UnauthorizedEscalation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAntiPatternDetectionV1 {
    pub source_id: String,
    pub source_kind: DetectionSourceKind,
    pub signal_ref: String,
    pub deterministic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAntiPatternEntryV1 {
    pub entry_id: String,
    pub title: String,
    pub domain: SessionAntiPatternDomain,
    pub severity: String,
    pub trigger_condition: String,
    pub detections: Vec<SessionAntiPatternDetectionV1>,
    pub policy_outcome: PolicyOutcome,
    pub coverage_tags: Vec<SessionAntiPatternCoverage>,
    pub required_flight_recorder_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAntiPatternRegistryV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub version: u32,
    pub folded_stub_ids: Vec<String>,
    pub entries: Vec<SessionAntiPatternEntryV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAntiPatternRegistryProjectionV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub version: u32,
    pub covered_domains: Vec<SessionAntiPatternDomain>,
    pub detection_source_kinds: Vec<DetectionSourceKind>,
    pub covered_outcomes: Vec<PolicyOutcome>,
    pub covered_risks: Vec<SessionAntiPatternCoverage>,
    pub deny_entry_ids: Vec<String>,
    pub downgrade_entry_ids: Vec<String>,
    pub consent_entry_ids: Vec<String>,
    pub force_stop_entry_ids: Vec<String>,
    pub required_flight_recorder_refs: Vec<String>,
    pub machine_readable_detections: bool,
    pub mutates_session: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAntiPatternRegistryValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_session_anti_pattern_registry(
    registry: &SessionAntiPatternRegistryV1,
) -> Result<(), Vec<SessionAntiPatternRegistryValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &registry.schema_id);
    require_non_empty(&mut errors, "registry_id", &registry.registry_id);
    if registry.version == 0 {
        errors.push(SessionAntiPatternRegistryValidationError {
            field: "version",
            message: "registry version must be positive",
        });
    }
    require_vec(&mut errors, "folded_stub_ids", &registry.folded_stub_ids);
    require_vec(&mut errors, "entries", &registry.entries);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &registry.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &registry.folded_source_refs,
    );

    if !contains_exact(
        &registry.folded_stub_ids,
        FOLDED_SESSION_ANTI_PATTERN_REGISTRY_STUB_ID,
    ) {
        errors.push(SessionAntiPatternRegistryValidationError {
            field: "folded_stub_ids",
            message: "registry must preserve the folded session anti-pattern stub id",
        });
    }
    if !contains_text(
        &registry.folded_source_refs,
        FOLDED_SESSION_ANTI_PATTERN_REGISTRY_STUB_ID,
    ) {
        errors.push(SessionAntiPatternRegistryValidationError {
            field: "folded_source_refs",
            message: "registry must preserve the folded source reference",
        });
    }

    validate_authority_refs(&mut errors, registry);
    validate_entries(&mut errors, registry);
    validate_required_coverage(&mut errors, registry);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_session_anti_pattern_registry(
    registry: &SessionAntiPatternRegistryV1,
) -> Result<SessionAntiPatternRegistryProjectionV1, Vec<SessionAntiPatternRegistryValidationError>>
{
    validate_session_anti_pattern_registry(registry)?;

    Ok(SessionAntiPatternRegistryProjectionV1 {
        schema_id: "hsk.kernel.session_anti_pattern_registry_projection@1".to_string(),
        registry_id: registry.registry_id.clone(),
        version: registry.version,
        covered_domains: ordered_domains(registry),
        detection_source_kinds: ordered_detection_sources(registry),
        covered_outcomes: ordered_outcomes(registry),
        covered_risks: ordered_coverage(registry),
        deny_entry_ids: entry_ids_for_outcome(registry, PolicyOutcome::Deny),
        downgrade_entry_ids: entry_ids_for_outcome(registry, PolicyOutcome::Downgrade),
        consent_entry_ids: entry_ids_for_outcome(registry, PolicyOutcome::RequireConsent),
        force_stop_entry_ids: entry_ids_for_outcome(registry, PolicyOutcome::ForceStop),
        required_flight_recorder_refs: ordered_flight_recorder_refs(registry),
        machine_readable_detections: registry.entries.iter().all(|entry| {
            !entry.detections.is_empty()
                && entry
                    .detections
                    .iter()
                    .all(|detection| detection.deterministic)
        }),
        mutates_session: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    registry: &SessionAntiPatternRegistryV1,
) {
    for required_ref in [
        "kernel.role_turn_isolation",
        "kernel.work_profiles",
        "kernel.local_first_mcp_posture",
        "flight_recorder.session_orchestration",
        "capability_gate.session",
    ] {
        if !contains_exact(&registry.product_authority_refs, required_ref) {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "product_authority_refs",
                message: "session anti-pattern registry must cite role isolation, work profiles, local-first posture, session Flight Recorder, and capability gate authorities",
            });
        }
    }
}

fn validate_entries(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    registry: &SessionAntiPatternRegistryV1,
) {
    let mut entry_ids = HashSet::new();

    for entry in &registry.entries {
        if !entry_ids.insert(entry.entry_id.as_str()) {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.entry_id",
                message: "session anti-pattern entry ids must be unique",
            });
        }

        require_non_empty(errors, "entries.entry_id", &entry.entry_id);
        if !entry.entry_id.starts_with("sap.") {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.entry_id",
                message: "session anti-pattern entry ids must use the stable sap.* namespace",
            });
        }
        require_non_empty(errors, "entries.title", &entry.title);
        require_non_empty(errors, "entries.severity", &entry.severity);
        require_non_empty(
            errors,
            "entries.trigger_condition",
            &entry.trigger_condition,
        );
        require_vec(errors, "entries.detections", &entry.detections);
        require_vec(errors, "entries.coverage_tags", &entry.coverage_tags);
        require_vec(
            errors,
            "entries.required_flight_recorder_refs",
            &entry.required_flight_recorder_refs,
        );

        validate_detection_sources(errors, entry);
        validate_flight_recorder_refs(errors, entry);
    }
}

fn validate_detection_sources(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    entry: &SessionAntiPatternEntryV1,
) {
    let mut source_ids = HashSet::new();

    for detection in &entry.detections {
        if !source_ids.insert(detection.source_id.as_str()) {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.detections.source_id",
                message: "detection source ids must be unique inside an anti-pattern entry",
            });
        }
        require_non_empty(errors, "entries.detections.source_id", &detection.source_id);
        require_non_empty(
            errors,
            "entries.detections.signal_ref",
            &detection.signal_ref,
        );
        if !detection.deterministic {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.detections.deterministic",
                message: "anti-pattern detections must be deterministic machine-readable signals",
            });
        }
    }
}

fn validate_flight_recorder_refs(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    entry: &SessionAntiPatternEntryV1,
) {
    for event_ref in &entry.required_flight_recorder_refs {
        require_non_empty(errors, "entries.required_flight_recorder_refs", event_ref);
        if !event_ref.starts_with("FR-EVT-SESSION-") {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.required_flight_recorder_refs",
                message: "session anti-pattern evidence must cite FR-EVT-SESSION events",
            });
        }
    }
}

fn validate_required_coverage(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    registry: &SessionAntiPatternRegistryV1,
) {
    for domain in [
        SessionAntiPatternDomain::Scheduler,
        SessionAntiPatternDomain::TrustBoundary,
        SessionAntiPatternDomain::CapabilityGate,
        SessionAntiPatternDomain::SessionOrchestration,
    ] {
        if !registry.entries.iter().any(|entry| entry.domain == domain) {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.domain",
                message: "registry must cover scheduler, trust, capability, and session orchestration domains",
            });
        }
    }

    for source_kind in [
        DetectionSourceKind::SchedulerCheck,
        DetectionSourceKind::TrustBoundaryValidation,
        DetectionSourceKind::CapabilityGateOutcome,
        DetectionSourceKind::SessionLifecycleRecord,
    ] {
        if !registry
            .entries
            .iter()
            .flat_map(|entry| entry.detections.iter())
            .any(|detection| detection.source_kind == source_kind)
        {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.detections.source_kind",
                message: "registry must include scheduler, trust-boundary, capability-gate, and session-lifecycle detection sources",
            });
        }
    }

    for outcome in [
        PolicyOutcome::Deny,
        PolicyOutcome::Downgrade,
        PolicyOutcome::RequireConsent,
        PolicyOutcome::ForceStop,
    ] {
        if !registry
            .entries
            .iter()
            .any(|entry| entry.policy_outcome == outcome)
        {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.policy_outcome",
                message:
                    "registry must cover deny, downgrade, require-consent, and force-stop outcomes",
            });
        }
    }

    for coverage in [
        SessionAntiPatternCoverage::SpawnAbuse,
        SessionAntiPatternCoverage::TrustBoundaryViolation,
        SessionAntiPatternCoverage::UnauthorizedEscalation,
    ] {
        if !registry
            .entries
            .iter()
            .flat_map(|entry| entry.coverage_tags.iter())
            .any(|tag| *tag == coverage)
        {
            errors.push(SessionAntiPatternRegistryValidationError {
                field: "entries.coverage_tags",
                message: "registry must cover spawn abuse, trust-boundary violations, and unauthorized escalation",
            });
        }
    }
}

fn ordered_domains(registry: &SessionAntiPatternRegistryV1) -> Vec<SessionAntiPatternDomain> {
    [
        SessionAntiPatternDomain::Scheduler,
        SessionAntiPatternDomain::TrustBoundary,
        SessionAntiPatternDomain::CapabilityGate,
        SessionAntiPatternDomain::SessionOrchestration,
    ]
    .into_iter()
    .filter(|domain| registry.entries.iter().any(|entry| entry.domain == *domain))
    .collect()
}

fn ordered_detection_sources(registry: &SessionAntiPatternRegistryV1) -> Vec<DetectionSourceKind> {
    [
        DetectionSourceKind::SchedulerCheck,
        DetectionSourceKind::TrustBoundaryValidation,
        DetectionSourceKind::CapabilityGateOutcome,
        DetectionSourceKind::SessionLifecycleRecord,
    ]
    .into_iter()
    .filter(|source_kind| {
        registry
            .entries
            .iter()
            .flat_map(|entry| entry.detections.iter())
            .any(|detection| detection.source_kind == *source_kind)
    })
    .collect()
}

fn ordered_outcomes(registry: &SessionAntiPatternRegistryV1) -> Vec<PolicyOutcome> {
    [
        PolicyOutcome::Deny,
        PolicyOutcome::Downgrade,
        PolicyOutcome::RequireConsent,
        PolicyOutcome::ForceStop,
    ]
    .into_iter()
    .filter(|outcome| {
        registry
            .entries
            .iter()
            .any(|entry| entry.policy_outcome == *outcome)
    })
    .collect()
}

fn ordered_coverage(registry: &SessionAntiPatternRegistryV1) -> Vec<SessionAntiPatternCoverage> {
    [
        SessionAntiPatternCoverage::SpawnAbuse,
        SessionAntiPatternCoverage::TrustBoundaryViolation,
        SessionAntiPatternCoverage::UnauthorizedEscalation,
    ]
    .into_iter()
    .filter(|coverage| {
        registry
            .entries
            .iter()
            .flat_map(|entry| entry.coverage_tags.iter())
            .any(|tag| tag == coverage)
    })
    .collect()
}

fn entry_ids_for_outcome(
    registry: &SessionAntiPatternRegistryV1,
    outcome: PolicyOutcome,
) -> Vec<String> {
    registry
        .entries
        .iter()
        .filter(|entry| entry.policy_outcome == outcome)
        .map(|entry| entry.entry_id.clone())
        .collect()
}

fn ordered_flight_recorder_refs(registry: &SessionAntiPatternRegistryV1) -> Vec<String> {
    let mut refs = Vec::new();
    let mut seen = HashSet::new();

    for event_ref in registry
        .entries
        .iter()
        .flat_map(|entry| entry.required_flight_recorder_refs.iter())
    {
        if seen.insert(event_ref.as_str()) {
            refs.push(event_ref.clone());
        }
    }

    refs
}

fn require_non_empty(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(SessionAntiPatternRegistryValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<SessionAntiPatternRegistryValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(SessionAntiPatternRegistryValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
