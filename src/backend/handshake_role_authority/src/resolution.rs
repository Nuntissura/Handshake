//! Deterministic evidence integrity only; no authentication or live authority.

use crate::canonical::{
    body_content_hash, canonical_event_payload_hash, canonical_receipt_bytes, validate_body,
    validate_event_key,
};
use crate::domain::{
    CanonicalEventPayload, PurePreparedRoleIntent,
    PureVerifiedRoleEvidence, Purpose, ResolutionExpectation, RetryDisposition, RoleError,
    RoleEvidenceRef, RoleReceipt, StageAckKey,
};

pub fn verify_receipt(
    receipt: &RoleReceipt,
    payload: &CanonicalEventPayload,
    expected: &ResolutionExpectation,
) -> Result<PureVerifiedRoleEvidence, RoleError> {
    validate_body(&receipt.body)?;
    validate_event_key(&receipt.event)?;
    validate_event_key(&payload.key)?;
    if body_content_hash(&receipt.body)? != receipt.content_hash {
        return Err(RoleError::ContentHashMismatch);
    }
    if receipt.event != payload.key {
        return Err(RoleError::EventIdentityMismatch);
    }
    if payload.payload_json.as_bytes() != canonical_receipt_bytes(receipt)?.as_slice() {
        return Err(RoleError::PayloadMismatch);
    }
    if canonical_event_payload_hash(&payload.payload_json)? != payload.payload_hash {
        return Err(RoleError::EventHashMismatch);
    }
    let body = &receipt.body;
    for (field, matches) in [
        ("receipt_scope", body.scope == expected.receipt_scope),
        ("role", body.role == expected.role),
        ("run", body.run == expected.run),
        ("subject", body.subject == expected.subject),
        ("purpose", body.purpose == expected.purpose),
        ("expected", body.expected == expected.expected),
    ] {
        if !matches {
            return Err(RoleError::BindingMismatch { field });
        }
    }
    let reference = RoleEvidenceRef::parse(&format!(
        "eventledger://role-authority/{}",
        receipt.event.event_id
    ))?;
    PureVerifiedRoleEvidence::new_checked(receipt.clone(), reference)
}

pub fn compare_retry(
    existing: &RoleReceipt,
    incoming: &PurePreparedRoleIntent,
) -> Result<RetryDisposition, RoleError> {
    validate_body(&existing.body)?;
    if body_content_hash(&existing.body)? != existing.content_hash {
        return Err(RoleError::ContentHashMismatch);
    }
    if &existing.body != incoming.body() {
        return Err(RoleError::RetryConflict);
    }
    Ok(RetryDisposition::ExactExisting)
}

pub fn verify_ack(
    prior: &PureVerifiedRoleEvidence,
    acknowledgement: &PureVerifiedRoleEvidence,
    requested: &StageAckKey,
) -> Result<(), RoleError> {
    let prior_body = &prior.receipt().body;
    let ack_body = &acknowledgement.receipt().body;
    if prior_body.purpose != Purpose::PriorSelection {
        return Err(RoleError::AckMismatch { field: "prior_purpose" });
    }
    if ack_body.purpose != Purpose::StageAcknowledgement(requested.clone()) {
        return Err(RoleError::AckMismatch { field: "stage" });
    }
    if prior.reference() != &requested.prior_receipt_ref {
        return Err(RoleError::AckMismatch { field: "prior_receipt_ref" });
    }
    // Each session/version was checked against its own receipt expectation.
    // Their equality or ordering is not a live authorization rule here.
    for (field, matches) in [
        ("role", prior_body.role == ack_body.role),
        ("issuer", prior_body.issuer == ack_body.issuer),
        ("subject", prior_body.subject == ack_body.subject),
        ("run", prior_body.run == ack_body.run),
        ("owner_account_id", prior_body.scope.owner_account_id == ack_body.scope.owner_account_id),
        ("access_space_id", prior_body.scope.access_space_id == ack_body.scope.access_space_id),
        ("workspace_id", prior_body.scope.workspace_id == ack_body.scope.workspace_id),
        ("aggregate_id", prior_body.expected.aggregate_id == ack_body.expected.aggregate_id),
        ("base_ref", prior_body.expected.base_ref == ack_body.expected.base_ref),
        ("base_vector_ref", prior_body.expected.base_vector_ref == ack_body.expected.base_vector_ref),
        ("schema_id", prior_body.expected.schema_id == ack_body.expected.schema_id),
        ("designation", prior_body.designation == ack_body.designation),
        ("validator_execution", prior_body.validator_execution == ack_body.validator_execution),
    ] {
        if !matches {
            return Err(RoleError::AckMismatch { field });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::prepare_operator;
    use crate::domain::{ApproveOperatorArtifact, ContentSha256, RoleEvidenceBody};
    use serde_json::{json, Value};

    const BODY: &str = r#"{"action":{"action_id":"fixture-action","idempotency_key":"fixture-key","request_id":"fixture-request"},"designation":null,"expected":{"aggregate_id":"fixture-aggregate","aggregate_version":1,"base_ref":null,"base_vector_ref":null,"schema_id":"fixture-role@1"},"issuer":"00000000-0000-0000-0000-000000000002","predecessor_refs":[],"purpose":"prior_selection","retention":{"artifact_retention_ttl_days":null,"authority_class":"fixture-class","policy_ref":"fixture-policy","replay_policy_ref":"fixture-replay"},"role":"operator","run":{"coordinator_session_id":"fixture-coordinator","launch_plan_hash":"0000000000000000000000000000000000000000000000000000000000000000","locus_binding_ref":"fixture-locus","micro_task_id":"fixture-mt","owner_session":"fixture-owner","run_id":"fixture-run","task_board_id":"fixture-board","trace_id":"fixture-trace","work_packet_id":"fixture-wp"},"scope":{"access_space_id":"00000000-0000-0000-0000-000000000004","actor_principal_id":"00000000-0000-0000-0000-000000000002","authenticated_session_id":"00000000-0000-0000-0000-000000000003","owner_account_id":"00000000-0000-0000-0000-000000000001","workspace_id":"fixture-workspace"},"subject":{"artifact_ref":"fixture-artifact","manifest_ref":"fixture-manifest","sha256":"0000000000000000000000000000000000000000000000000000000000000000","version":"1"},"validator_execution":null}"#;
    const BODY_HASH: &str = "6fa4261398ac698405c9defbcd6b2cb4897da8554488631e95606a054206b397";
    const PAYLOAD_HASH: &str = "09e165447c8501807c3bdec1e712d3ab454457da0ffeb3d60a61f47d1164a840";

    fn fixture() -> (RoleReceipt, CanonicalEventPayload) {
        let payload_json = format!(
            "{{\"body\":{BODY},\"content_hash\":\"{BODY_HASH}\",\"event\":{{\"event_id\":\"00000000-0000-0000-0000-000000000009\",\"event_seq\":1,\"stream_id\":\"fixture-stream\",\"transaction_seq\":1}}}}"
        );
        let receipt: RoleReceipt = serde_json::from_str(&payload_json).unwrap();
        let payload = CanonicalEventPayload {
            key: receipt.event.clone(),
            payload_json,
            payload_hash: ContentSha256::from_hex(PAYLOAD_HASH).unwrap(),
        };
        (receipt, payload)
    }

    fn expectation(receipt: &RoleReceipt) -> ResolutionExpectation {
        ResolutionExpectation {
            receipt_scope: receipt.body.scope.clone(),
            role: receipt.body.role.clone(),
            run: receipt.body.run.clone(),
            subject: receipt.body.subject.clone(),
            purpose: receipt.body.purpose.clone(),
            expected: receipt.body.expected.clone(),
        }
    }

    fn seal(body: RoleEvidenceBody, event_id: &str) -> (RoleReceipt, CanonicalEventPayload) {
        let mut receipt = fixture().0;
        receipt.content_hash = body_content_hash(&body).unwrap();
        receipt.body = body;
        receipt.event.event_id = event_id.into();
        let payload_json = String::from_utf8(canonical_receipt_bytes(&receipt).unwrap()).unwrap();
        let payload = CanonicalEventPayload {
            key: receipt.event.clone(),
            payload_hash: canonical_event_payload_hash(&payload_json).unwrap(),
            payload_json,
        };
        (receipt, payload)
    }

    fn verified(body: RoleEvidenceBody, event_id: &str) -> PureVerifiedRoleEvidence {
        let (receipt, payload) = seal(body, event_id);
        verify_receipt(&receipt, &payload, &expectation(&receipt)).unwrap()
    }

    fn stage(prior: &PureVerifiedRoleEvidence) -> StageAckKey {
        StageAckKey {
            execution_id: "fixture-execution".into(),
            stage_id: "fixture-stage".into(),
            lane_id: "fixture-lane".into(),
            authority_request_message_ref: "fixture-request-message".into(),
            prior_receipt_ref: prior.reference().clone(),
            selecting_decision_id: Some("fixture-selected-decision".into()),
        }
    }

    fn ack(prior: &PureVerifiedRoleEvidence, key: &StageAckKey) -> RoleEvidenceBody {
        let mut body = prior.receipt().body.clone();
        body.purpose = Purpose::StageAcknowledgement(key.clone());
        body.action.action_id = "fixture-ack-action".into();
        body.action.request_id = "fixture-ack-request".into();
        body.action.idempotency_key = "fixture-ack-key".into();
        body
    }

    fn incoming(body: &RoleEvidenceBody) -> PurePreparedRoleIntent {
        prepare_operator(&ApproveOperatorArtifact {
            action: body.action.clone(), scope: body.scope.clone(), run: body.run.clone(),
            subject: body.subject.clone(), expected: body.expected.clone(), retention: body.retention.clone(),
        }).unwrap()
    }

    #[test]
    fn fixed_receipt_bytes_and_hashes_verify_as_integrity_only() {
        let (receipt, payload) = fixture();
        assert_eq!(BODY.len(), 1360);
        assert_eq!(payload.payload_json.len(), 1574);
        let result = verify_receipt(&receipt, &payload, &expectation(&receipt)).unwrap();
        assert_eq!(result.receipt(), &receipt);
        assert_eq!(result.content_hash().to_hex(), BODY_HASH);
        assert_eq!(result.reference(), &RoleEvidenceRef::parse("eventledger://role-authority/00000000-0000-0000-0000-000000000009").unwrap());
    }

    #[test]
    fn tampered_body_hash_fails() {
        let (mut receipt, payload) = fixture();
        receipt.body.subject.version = "2".into();
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::ContentHashMismatch)));
    }

    #[test]
    fn event_identity_and_raw_payload_hash_fail_independently() {
        let (receipt, mut payload) = fixture();
        payload.key.event_seq += 1;
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::EventIdentityMismatch)));
        payload.key = receipt.event.clone();
        payload.payload_hash = ContentSha256::from_hex(&"f".repeat(64)).unwrap();
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::EventHashMismatch)));
    }

    #[test]
    fn equivalent_noncanonical_payload_bytes_fail() {
        let (receipt, mut payload) = fixture();
        payload.payload_json.push(' ');
        payload.payload_hash = canonical_event_payload_hash(&payload.payload_json).unwrap();
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::PayloadMismatch)));
    }

    #[test]
    fn every_own_scope_dimension_is_bound() {
        let (receipt, payload) = fixture();
        for field in ["owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"] {
            let mut expected = expectation(&receipt);
            let mut scope = serde_json::to_value(&expected.receipt_scope).unwrap();
            scope[field] = json!(if field == "workspace_id" { "another-workspace" } else { "00000000-0000-0000-0000-000000000099" });
            expected.receipt_scope = serde_json::from_value(scope).unwrap();
            assert!(matches!(verify_receipt(&receipt, &payload, &expected), Err(RoleError::BindingMismatch { field: "receipt_scope" })), "{field}");
        }
    }

    #[test]
    fn every_resolution_expectation_is_checked() {
        let (receipt, payload) = fixture();
        let mut values = Vec::new();
        let mut expected = expectation(&receipt);
        expected.role = crate::domain::Role::Validator;
        values.push(expected);
        let mut expected = expectation(&receipt);
        expected.run.trace_id = "another-trace".into();
        values.push(expected);
        let mut expected = expectation(&receipt);
        expected.subject.version = "2".into();
        values.push(expected);
        let mut expected = expectation(&receipt);
        expected.purpose = Purpose::StageAcknowledgement(stage(&verify_receipt(&receipt, &payload, &expectation(&receipt)).unwrap()));
        values.push(expected);
        let mut expected = expectation(&receipt);
        expected.expected.aggregate_version += 1;
        values.push(expected);
        for expected in values {
            assert!(matches!(verify_receipt(&receipt, &payload, &expected), Err(RoleError::BindingMismatch { .. })));
        }
    }

    #[test]
    fn decoded_issuer_scope_mismatch_is_rejected_before_hash_checks() {
        let (mut receipt, payload) = fixture();
        receipt.body.issuer = serde_json::from_value(json!("00000000-0000-0000-0000-000000000099")).unwrap();
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::BindingMismatch { field: "issuer" })));
    }

    #[test]
    fn exact_retry_ignores_event_metadata_but_not_hashed_body() {
        let (mut receipt, _) = fixture();
        let prepared = incoming(&receipt.body);
        receipt.event.event_seq = 50;
        assert_eq!(compare_retry(&receipt, &prepared).unwrap(), RetryDisposition::ExactExisting);
        let mut changed = receipt.body.clone();
        changed.retention.policy_ref = "another-declared-policy".into();
        assert!(matches!(compare_retry(&receipt, &incoming(&changed)), Err(RoleError::RetryConflict)));
        receipt.body.action.request_id = "tampered-request".into();
        assert!(matches!(compare_retry(&receipt, &prepared), Err(RoleError::ContentHashMismatch)));
    }

    #[test]
    fn ack_allows_own_version_and_session_progression_without_freshness_claims() {
        let prior = verified(fixture().0.body, "prior-event");
        let key = stage(&prior);
        let mut body = ack(&prior, &key);
        body.expected.aggregate_version = 7;
        let mut scope = serde_json::to_value(&body.scope).unwrap();
        scope["authenticated_session_id"] = json!("00000000-0000-0000-0000-000000000098");
        body.scope = serde_json::from_value(scope).unwrap();
        let acknowledgement = verified(body, "ack-event");
        verify_ack(&prior, &acknowledgement, &key).unwrap();
    }

    #[test]
    fn ack_requires_prior_selection_and_exact_prior_reference() {
        let prior = verified(fixture().0.body, "prior-event");
        let mut key = stage(&prior);
        key.prior_receipt_ref = RoleEvidenceRef::parse("eventledger://role-authority/another-event").unwrap();
        let acknowledgement = verified(ack(&prior, &key), "ack-event");
        assert!(matches!(verify_ack(&prior, &acknowledgement, &key), Err(RoleError::AckMismatch { field: "prior_receipt_ref" })));
        assert!(matches!(verify_ack(&acknowledgement, &acknowledgement, &key), Err(RoleError::AckMismatch { field: "prior_purpose" })));
    }

    #[test]
    fn ack_binds_every_causal_stage_key_field() {
        let prior = verified(fixture().0.body, "prior-event");
        let key = stage(&prior);
        let acknowledgement = verified(ack(&prior, &key), "ack-event");
        for field in ["execution_id", "stage_id", "lane_id", "authority_request_message_ref", "selecting_decision_id"] {
            let mut changed = serde_json::to_value(&key).unwrap();
            changed[field] = json!("another-value");
            let changed = serde_json::from_value(changed).unwrap();
            assert!(matches!(verify_ack(&prior, &acknowledgement, &changed), Err(RoleError::AckMismatch { field: "stage" })), "{field}");
        }
    }

    #[test]
    fn ack_retains_base_and_schema_binding_without_cross_version_equality() {
        let prior = verified(fixture().0.body, "prior-event");
        let key = stage(&prior);
        for field in ["aggregate_id", "base_ref", "base_vector_ref", "schema_id"] {
            let mut body = ack(&prior, &key);
            let mut expected = serde_json::to_value(&body.expected).unwrap();
            expected[field] = json!("another-value");
            body.expected = serde_json::from_value(expected).unwrap();
            let acknowledgement = verified(body, "ack-event");
            assert!(matches!(verify_ack(&prior, &acknowledgement, &key), Err(RoleError::AckMismatch { .. })), "{field}");
        }
    }

    #[test]
    fn ack_cannot_change_hashed_validator_execution_declarations() {
        let mut value = serde_json::to_value(fixture().0.body).unwrap();
        value["role"] = json!("validator");
        value["designation"] = json!("eventledger://validator-designation/fixture-designation");
        let issuer = value["issuer"].clone();
        let subject = value["subject"].clone();
        value["validator_execution"] = json!({
            "runner_principal": issuer, "runner_lane_id":"validator-lane", "invocation_ref":"runner-invocation",
            "request_message_ref":"validator-request", "execution_event":{"event_id":"runner-event","stream_id":"runner-stream","event_seq":1,"transaction_seq":1},
            "verdict_artifact":subject,
        });
        let prior = verified(serde_json::from_value(value).unwrap(), "prior-event");
        let key = stage(&prior);
        let mut body = ack(&prior, &key);
        body.validator_execution.as_mut().unwrap().invocation_ref = "another-invocation".into();
        let acknowledgement = verified(body, "ack-event");
        assert!(matches!(verify_ack(&prior, &acknowledgement, &key), Err(RoleError::AckMismatch { field: "validator_execution" })));
    }

    #[test]
    fn validator_facts_and_unsupported_controls_are_not_accepted_as_shape() {
        let (mut receipt, payload) = fixture();
        let mut value: Value = serde_json::to_value(&receipt.body).unwrap();
        value["role"] = json!("validator");
        receipt.body = serde_json::from_value(value).unwrap();
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::InvalidField { .. })));
        let (mut receipt, payload) = fixture();
        receipt.body.action.request_id.push('\u{0001}');
        assert!(matches!(verify_receipt(&receipt, &payload, &expectation(&receipt)), Err(RoleError::InvalidField { .. })));
    }
}
