use std::sync::Arc;

use handshake_core::ace::validators::role_registry_append_only::{
    canonical_json_sha256, record_role_registry_violation_diagnostic, DepartmentId,
    RoleContractKind, RoleContractSurface, RoleId, RoleRegistryAppendOnlyValidator,
    RoleRegistryDiagnosticContext, RoleRegistrySnapshot, RoleRegistryViolation, RoleSpecEntry,
};
use handshake_core::diagnostics::{DiagFilter, DiagnosticsStore};
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEventType};
use serde_json::json;
use uuid::Uuid;

fn role(role_id: &str) -> RoleSpecEntry {
    RoleSpecEntry {
        role_id: RoleId(role_id.to_string()),
        department_id: DepartmentId("dept".to_string()),
        display_name: role_id.to_string(),
        aliases: Vec::new(),
    }
}

fn contract(
    contract_id: &str,
    role_id: &str,
    kind: RoleContractKind,
    schema_json: serde_json::Value,
) -> RoleContractSurface {
    let version = contract_id
        .rsplit_once(':')
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "1".to_string());
    RoleContractSurface {
        contract_id: contract_id.to_string(),
        role_id: RoleId(role_id.to_string()),
        kind,
        version,
        schema_hash: canonical_json_sha256(&schema_json),
    }
}

#[test]
fn removed_role_id_is_detected() {
    let baseline = RoleRegistrySnapshot {
        roles: vec![role("a"), role("b")],
        contracts: Vec::new(),
    };
    let current = RoleRegistrySnapshot {
        roles: vec![role("b")],
        contracts: Vec::new(),
    };

    let violations = RoleRegistryAppendOnlyValidator::validate_all(&current, &baseline);
    assert!(
        violations.iter().any(|v| matches!(v, RoleRegistryViolation::RoleIdRemoved { role_id } if role_id.as_str() == "a")),
        "expected RoleIdRemoved violation for role_id 'a', got: {violations:?}"
    );
}

#[test]
fn removed_contract_id_is_detected() {
    let baseline = RoleRegistrySnapshot {
        roles: vec![role("a")],
        contracts: vec![contract(
            "ROLE:a:X:1",
            "a",
            RoleContractKind::Extract,
            json!({"type":"role_extract_contract","contract_version":1}),
        )],
    };
    let current = RoleRegistrySnapshot {
        roles: vec![role("a")],
        contracts: Vec::new(),
    };

    let violations = RoleRegistryAppendOnlyValidator::validate_all(&current, &baseline);
    assert!(
        violations.iter().any(|v| matches!(v, RoleRegistryViolation::ContractIdRemoved { contract_id } if contract_id == "ROLE:a:X:1")),
        "expected ContractIdRemoved violation for ROLE:a:X:1, got: {violations:?}"
    );
}

#[test]
fn contract_surface_drift_is_detected() {
    let baseline = RoleRegistrySnapshot {
        roles: vec![role("a")],
        contracts: vec![contract(
            "ROLE:a:X:1",
            "a",
            RoleContractKind::Extract,
            json!({"type":"role_extract_contract","contract_version":1,"fields":{"a":1}}),
        )],
    };
    let current = RoleRegistrySnapshot {
        roles: vec![role("a")],
        contracts: vec![contract(
            "ROLE:a:X:1",
            "a",
            RoleContractKind::Extract,
            json!({"type":"role_extract_contract","contract_version":1,"fields":{"a":2}}),
        )],
    };

    let violations = RoleRegistryAppendOnlyValidator::validate_all(&current, &baseline);
    let drift = violations.iter().find_map(|v| match v {
        RoleRegistryViolation::ContractSurfaceDrift {
            contract_id,
            expected_hash,
            got_hash,
        } if contract_id == "ROLE:a:X:1" => Some((*expected_hash, *got_hash)),
        _ => None,
    });
    assert!(
        drift.is_some(),
        "expected ContractSurfaceDrift, got: {violations:?}"
    );
    if let Some((expected_hash, got_hash)) = drift {
        assert_ne!(
            expected_hash, got_hash,
            "expected different schema hashes on drift"
        );
    }
}

#[test]
fn additions_are_allowed() {
    let baseline = RoleRegistrySnapshot {
        roles: vec![role("a")],
        contracts: vec![contract(
            "ROLE:a:X:1",
            "a",
            RoleContractKind::Extract,
            json!({"type":"role_extract_contract","contract_version":1}),
        )],
    };
    let current = RoleRegistrySnapshot {
        roles: vec![role("a"), role("b")],
        contracts: vec![
            contract(
                "ROLE:a:X:1",
                "a",
                RoleContractKind::Extract,
                json!({"type":"role_extract_contract","contract_version":1}),
            ),
            contract(
                "ROLE:b:X:1",
                "b",
                RoleContractKind::Extract,
                json!({"type":"role_extract_contract","contract_version":1}),
            ),
        ],
    };

    let violations = RoleRegistryAppendOnlyValidator::validate_all(&current, &baseline);
    assert!(
        violations.is_empty(),
        "expected no violations for pure additions, got: {violations:?}"
    );
}

#[test]
fn canonical_hashing_is_stable_across_key_order() {
    let a = json!({
        "b": 1,
        "a": {"y": true, "x": [3,2,1]},
    });
    let b = json!({
        "a": {"x": [3,2,1], "y": true},
        "b": 1,
    });

    let ha = canonical_json_sha256(&a);
    let hb = canonical_json_sha256(&b);
    assert_eq!(ha, hb);
}

#[tokio::test]
async fn recording_violation_creates_diagnostic_and_fr_evt_003() {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7).expect("recorder should init"));
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();

    let job_id = Uuid::new_v4().to_string();
    let violation = RoleRegistryViolation::RoleIdRemoved {
        role_id: RoleId("missing_role".to_string()),
    };

    let diag_id = record_role_registry_violation_diagnostic(
        diagnostics.as_ref(),
        &violation,
        RoleRegistryDiagnosticContext {
            wsid: Some("ws-1".to_string()),
            job_id: Some(job_id.clone()),
        },
    )
    .await
    .expect("diagnostic record should succeed");

    let diags = recorder
        .list_diagnostics(DiagFilter::default())
        .await
        .expect("diagnostics should be queryable");
    assert!(
        diags.iter().any(|d| d.id == diag_id),
        "expected recorded diagnostic to exist"
    );

    let events = flight_recorder
        .list_events(EventFilter {
            job_id: Some(job_id.clone()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::Diagnostic),
        "expected FR-EVT-003 Diagnostic event"
    );
}
