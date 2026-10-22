use handshake_account_activation::{
    parse_workspace_activation_observation, WorkspaceActivationExpected,
    WorkspaceActivationObservation, WorkspaceActivationObservationError, WorkspaceActivationOutcome,
    WorkspaceActivationReadiness,
};
use serde_json::{json, Value};
use uuid::Uuid;

const OPERATION: &str = "018f1000-0000-7000-8000-000000000001";
const OTHER_OPERATION: &str = "018f1000-0000-7000-8000-000000000002";

fn expected() -> WorkspaceActivationExpected<'static> {
    WorkspaceActivationExpected {
        operation_id: Uuid::parse_str(OPERATION).unwrap(),
        owner_account_id: "owner",
        actor_principal_id: "principal",
        authenticated_session_id: "session",
        access_space_id: "space",
        workspace_id: "workspace",
    }
}

fn unresolved() -> Value {
    json!({
        "schema_id": "hsk.workspace_activation.observation@1",
        "operation_id": OPERATION,
        "owner_account_id": "owner",
        "actor_principal_id": "principal",
        "authenticated_session_id": "session",
        "access_space_id": "space",
        "workspace_id": "workspace",
        "readiness": { "status": "unavailable" },
        "outcome": { "status": "unresolved" }
    })
}

fn runtime() -> Value {
    json!({
        "runtime_instance_id": "018f1000-0000-7000-8000-000000000003",
        "runtime_host_scope_id": "host",
        "runtime_lease_protocol": "hsk.embedded_runtime.loopback_udp_exclusive@1",
        "runtime_lease_address": "127.0.0.1",
        "runtime_lease_port": 37502
    })
}

fn applied() -> Value {
    let mut value = unresolved();
    value["readiness"] = json!({ "status": "current", "descriptor": runtime() });
    value["outcome"] = json!({
        "status": "applied",
        "result": {
            "schema_version": "hsk.workspace_activation@1",
            "operation_id": OPERATION,
            "owner_account_id": "owner",
            "actor_principal_id": "principal",
            "authenticated_session_id": "session",
            "access_space_id": "space",
            "workspace_id": "workspace"
        },
        "publication_witness": {
            "operation_id": OPERATION,
            "original_envelope": {
                "operation_id": OPERATION,
                "owner_account_id": "owner",
                "actor_principal_id": "principal",
                "authenticated_session_id": "session",
                "access_space_id": "space",
                "workspace_id": "workspace",
                "resource_id": "resource",
                "grant_id": "grant",
                "grant_version": 1,
                "policy_version": 1,
                "policy_versions": [1, 1, 1, 1, 1, 1],
                "revocation_epochs": [0, 0, 0, 0, 0, 0],
                "actor_kind": "operator",
                "actor_id": "principal",
                "actions": ["read"],
                "capability_ids": ["model_lane.navigation.read"],
                "delegation_chain": [],
                "session_capabilities": ["model_lane.navigation.read"],
                "principal_capabilities": ["model_lane.navigation.read"],
                "capability_profile_id": "Operator",
                "statuses": ["active", "active", "enabled", "enabled", "active"],
                "resource_kind": "workspace",
                "capability_id": "model_lane.navigation.read",
                "action": "read",
                "purpose": "navigation_read",
                "grant_expires_at": null,
                "session_expires_at": "2030-01-01T00:00:00Z"
            },
            "actual_runtime": runtime()
        }
    });
    value
}

fn decode(value: Value) -> Result<WorkspaceActivationObservation<Value>, WorkspaceActivationObservationError> {
    parse_workspace_activation_observation(value, &expected())
}

#[test]
fn rejects_missing_null_or_wrong_schema() {
    for schema in [None, Some(Value::Null), Some(json!("other@1"))] {
        let mut value = unresolved();
        value.as_object_mut().unwrap().remove("schema_id");
        if let Some(schema) = schema {
            value["schema_id"] = schema;
        }
        assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidSchema)));
    }
}

#[test]
fn rejects_missing_null_required_fields_and_unknown_tags() {
    for field in [
        "operation_id", "owner_account_id", "actor_principal_id", "authenticated_session_id",
        "access_space_id", "workspace_id", "readiness", "outcome",
    ] {
        for remove in [false, true] {
            let mut value = unresolved();
            if remove {
                value.as_object_mut().unwrap().remove(field);
            } else {
                value[field] = Value::Null;
            }
            assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)), "{field}, remove={remove}");
        }
    }
    for field in ["outcome", "readiness"] {
        for tag in [Value::Null, json!("unknown"), json!(3)] {
            let mut value = unresolved();
            value[field]["status"] = tag;
            assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)), "{field}");
        }
        let mut value = unresolved();
        value[field].as_object_mut().unwrap().remove("status");
        assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)));
    }
}

#[test]
fn rejects_original_identity_mismatches_in_observation_ack_and_witness() {
    for prefix in ["", "/outcome/result", "/outcome/publication_witness/original_envelope"] {
        for field in [
            "operation_id", "owner_account_id", "actor_principal_id",
            "authenticated_session_id", "access_space_id", "workspace_id",
        ] {
            let mut value = applied();
            let pointer = format!("{prefix}/{field}");
            *value.pointer_mut(&pointer).unwrap() = if field == "operation_id" {
                json!(OTHER_OPERATION)
            } else {
                json!("foreign")
            };
            assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::Mismatch)), "{pointer}");
        }
    }
    let mut value = applied();
    value["outcome"]["publication_witness"]["operation_id"] = json!(OTHER_OPERATION);
    assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::Mismatch)));
}

#[test]
fn rejects_applied_without_required_result_or_witness_fields() {
    for pointer in [
        "/outcome/result", "/outcome/publication_witness",
        "/outcome/publication_witness/operation_id",
        "/outcome/publication_witness/original_envelope",
        "/outcome/publication_witness/actual_runtime", "/readiness/descriptor",
    ] {
        for remove in [false, true] {
            let mut value = applied();
            if remove {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                value.pointer_mut(parent).unwrap().as_object_mut().unwrap().remove(key);
            } else {
                *value.pointer_mut(pointer).unwrap() = Value::Null;
            }
            assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)), "{pointer}, remove={remove}");
        }
    }
    let mut value = applied();
    value["outcome"]["result"]["schema_version"] = json!("other@1");
    assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)));
}

#[test]
fn rejects_payload_fields_in_unresolved_or_unavailable_variants() {
    for field in ["result", "publication_witness"] {
        let mut value = unresolved();
        value["outcome"][field] = applied()["outcome"][field].clone();
        assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)), "{field}");
    }
    let mut value = unresolved();
    value["readiness"]["descriptor"] = runtime();
    assert!(matches!(decode(value), Err(WorkspaceActivationObservationError::InvalidObservation)));
}

#[test]
fn unresolved_roundtrip_preserves_absence_of_result_and_witness() {
    for readiness in [json!({ "status": "unavailable" }), json!({ "status": "current", "descriptor": runtime() })] {
        let mut value = unresolved();
        value["readiness"] = readiness;
        let observation = decode(value.clone()).unwrap();
        assert!(matches!(&observation.outcome, WorkspaceActivationOutcome::Unresolved));
        let encoded = observation.to_value().unwrap();
        assert_eq!(encoded, value);
        assert!(!encoded["outcome"].as_object().unwrap().contains_key("result"));
        assert!(!encoded["outcome"].as_object().unwrap().contains_key("publication_witness"));
        assert!(matches!(decode(encoded).unwrap().outcome, WorkspaceActivationOutcome::Unresolved));
    }
}

#[test]
fn applied_roundtrip_preserves_nullable_expiry_and_separate_readiness() {
    for expiry in [Value::Null, json!("2029-01-01T00:00:00Z")] {
        for readiness in [json!({ "status": "unavailable" }), json!({ "status": "current", "descriptor": runtime() })] {
            let mut value = applied();
            value["outcome"]["publication_witness"]["original_envelope"]["grant_expires_at"] = expiry.clone();
            value["readiness"] = readiness;
            let observation = decode(value.clone()).unwrap();
            assert!(matches!(&observation.outcome, WorkspaceActivationOutcome::Applied { .. }));
            if value["readiness"]["status"] == "unavailable" {
                assert!(matches!(&observation.readiness, WorkspaceActivationReadiness::Unavailable));
            }
            let encoded = observation.to_value().unwrap();
            assert_eq!(encoded, value);
            assert!(matches!(decode(encoded).unwrap().outcome, WorkspaceActivationOutcome::Applied { .. }));
        }
    }
}
