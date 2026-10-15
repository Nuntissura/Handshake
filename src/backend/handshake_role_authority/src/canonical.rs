//! Deterministic declared-evidence bytes. These functions neither issue nor authenticate authority.

use crate::domain::*;
use handshake_storage_support::canonical_json::{canonical_json_bytes, sha256_hex};
use serde::Serialize;
use serde_json::Value;

pub(crate) fn validate_string(value: &str, field: &'static str) -> Result<(), RoleError> {
    if value.is_empty() || value.chars().count() > 512 || unsupported_control(value) {
        return Err(RoleError::InvalidField { field });
    }
    Ok(())
}

fn unsupported_control(value: &str) -> bool {
    value.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

fn validate_json(value: &Value) -> Result<(), RoleError> {
    match value {
        Value::String(value) if unsupported_control(value) => return Err(RoleError::InvalidField { field: "json_string" }),
        Value::Array(items) => for item in items { validate_json(item)?; },
        Value::Object(items) => for (key, value) in items {
            if unsupported_control(key) { return Err(RoleError::InvalidField { field: "json_key" }); }
            validate_json(value)?;
        },
        _ => {}
    }
    Ok(())
}

fn encoded(value: &impl Serialize) -> Result<Vec<u8>, RoleError> {
    let value = serde_json::to_value(value).map_err(|e| RoleError::Json(e.to_string()))?;
    validate_json(&value)?;
    Ok(canonical_json_bytes(&value))
}

pub(crate) fn validate_event_key(event: &CanonicalEventKey) -> Result<(), RoleError> {
    validate_string(&event.event_id, "event_id")?;
    validate_string(&event.stream_id, "stream_id")
}

fn validate_subject(subject: &ArtifactSubject) -> Result<(), RoleError> {
    validate_string(&subject.artifact_ref, "artifact_ref")?;
    validate_string(&subject.manifest_ref, "manifest_ref")?;
    validate_string(&subject.version, "artifact_version")
}

fn validate_stage(stage: &StageAckKey) -> Result<(), RoleError> {
    for (value, field) in [(&stage.execution_id, "execution_id"), (&stage.stage_id, "stage_id"),
        (&stage.lane_id, "lane_id"), (&stage.authority_request_message_ref, "authority_request_message_ref")] {
        validate_string(value, field)?;
    }
    if let Some(value) = &stage.selecting_decision_id { validate_string(value, "selecting_decision_id")?; }
    Ok(())
}

pub(crate) fn validate_body(body: &RoleEvidenceBody) -> Result<(), RoleError> {
    if body.issuer != body.scope.actor_principal_id { return Err(RoleError::BindingMismatch { field: "issuer" }); }
    validate_string(body.scope.workspace_id.as_str(), "workspace_id")?;
    for (value, field) in [(&body.action.action_id, "action_id"), (&body.action.request_id, "request_id"),
        (&body.action.idempotency_key, "idempotency_key"), (&body.run.run_id, "run_id"),
        (&body.run.trace_id, "trace_id"), (&body.run.coordinator_session_id, "coordinator_session_id"),
        (&body.run.owner_session, "owner_session"), (&body.run.work_packet_id, "work_packet_id"),
        (&body.run.micro_task_id, "micro_task_id"), (&body.run.task_board_id, "task_board_id"),
        (&body.run.locus_binding_ref, "locus_binding_ref"), (&body.expected.aggregate_id, "aggregate_id"),
        (&body.expected.schema_id, "schema_id"), (&body.retention.policy_ref, "policy_ref"),
        (&body.retention.authority_class, "authority_class"), (&body.retention.replay_policy_ref, "replay_policy_ref")] {
        validate_string(value, field)?;
    }
    for value in [&body.expected.base_ref, &body.expected.base_vector_ref].into_iter().flatten() {
        validate_string(value, "expected_base_ref")?;
    }
    for value in &body.predecessor_refs {
        if value.starts_with("eventledger://role-authority/") {
            RoleEvidenceRef::parse(value)?;
        } else {
            validate_string(value, "predecessor_ref")?;
        }
    }
    validate_subject(&body.subject)?;
    if let Purpose::StageAcknowledgement(stage) = &body.purpose { validate_stage(stage)?; }
    match (&body.role, &body.designation, &body.validator_execution) {
        (Role::Operator, None, None) => {}
        (Role::Validator, Some(_), Some(execution)) => {
            if execution.runner_principal != body.issuer { return Err(RoleError::BindingMismatch { field: "runner_principal" }); }
            validate_string(&execution.runner_lane_id, "runner_lane_id")?;
            validate_string(&execution.invocation_ref, "invocation_ref")?;
            validate_string(&execution.request_message_ref, "request_message_ref")?;
            validate_event_key(&execution.execution_event)?;
            validate_subject(&execution.verdict_artifact)?;
        }
        _ => return Err(RoleError::InvalidField { field: "role_evidence_shape" }),
    }
    validate_json(&serde_json::to_value(body).map_err(|e| RoleError::Json(e.to_string()))?)
}

fn predecessors(purpose: &Purpose) -> Vec<String> {
    match purpose {
        Purpose::PriorSelection => Vec::new(),
        Purpose::StageAcknowledgement(stage) => vec![stage.prior_receipt_ref.to_string()],
    }
}

pub fn prepare_operator(command: &ApproveOperatorArtifact) -> Result<PurePreparedRoleIntent, RoleError> {
    PurePreparedRoleIntent::new_checked(RoleEvidenceBody {
        role: Role::Operator, issuer: command.scope.actor_principal_id, scope: command.scope.clone(),
        action: command.action.clone(), run: command.run.clone(), subject: command.subject.clone(),
        purpose: Purpose::PriorSelection, designation: None, validator_execution: None,
        expected: command.expected.clone(), retention: command.retention.clone(), predecessor_refs: Vec::new(),
    })
}

pub fn prepare_validator_verdict_declared(command: &RecordValidatorVerdict) -> Result<PurePreparedRoleIntent, RoleError> {
    PurePreparedRoleIntent::new_checked(RoleEvidenceBody {
        role: Role::Validator, issuer: command.scope.actor_principal_id, scope: command.scope.clone(),
        action: command.action.clone(), run: command.run.clone(), subject: command.subject.clone(),
        purpose: command.purpose.clone(), designation: Some(command.designation_ref.clone()),
        validator_execution: Some(command.execution.clone()), expected: command.expected.clone(),
        retention: command.retention.clone(), predecessor_refs: predecessors(&command.purpose),
    })
}

pub fn prepare_operator_ack(command: &AcknowledgeOperatorStage) -> Result<PurePreparedRoleIntent, RoleError> {
    if command.approval != command.stage.prior_receipt_ref { return Err(RoleError::AckMismatch { field: "prior_receipt_ref" }); }
    let mut body = prepare_operator(&ApproveOperatorArtifact {
        action: command.action.clone(), scope: command.scope.clone(), run: command.run.clone(),
        subject: command.subject.clone(), expected: command.expected.clone(), retention: command.retention.clone(),
    })?.body().clone();
    body.purpose = Purpose::StageAcknowledgement(command.stage.clone());
    body.predecessor_refs = predecessors(&body.purpose);
    PurePreparedRoleIntent::new_checked(body)
}

pub fn prepare_validator_ack(command: &AcknowledgeValidatorStage) -> Result<PurePreparedRoleIntent, RoleError> {
    if command.verdict != command.stage.prior_receipt_ref { return Err(RoleError::AckMismatch { field: "prior_receipt_ref" }); }
    prepare_validator_verdict_declared(&RecordValidatorVerdict {
        action: command.action.clone(), scope: command.scope.clone(), designation_ref: command.designation_ref.clone(),
        execution: command.execution.clone(), run: command.run.clone(), subject: command.subject.clone(),
        purpose: Purpose::StageAcknowledgement(command.stage.clone()), expected: command.expected.clone(), retention: command.retention.clone(),
    })
}

pub fn canonical_body_bytes(body: &RoleEvidenceBody) -> Result<Vec<u8>, RoleError> {
    validate_body(body)?;
    encoded(body)
}
pub fn canonical_intent_bytes(intent: &PurePreparedRoleIntent) -> Result<Vec<u8>, RoleError> { canonical_body_bytes(intent.body()) }
pub fn body_content_hash(body: &RoleEvidenceBody) -> Result<ContentSha256, RoleError> {
    ContentSha256::from_hex(&sha256_hex(&canonical_body_bytes(body)?))
}
pub fn canonical_receipt_bytes(receipt: &RoleReceipt) -> Result<Vec<u8>, RoleError> {
    validate_body(&receipt.body)?;
    validate_event_key(&receipt.event)?;
    if body_content_hash(&receipt.body)? != receipt.content_hash { return Err(RoleError::ContentHashMismatch); }
    encoded(receipt)
}
pub fn canonical_event_payload_hash(payload_json: &str) -> Result<ContentSha256, RoleError> {
    let value: Value = serde_json::from_str(payload_json).map_err(|e| RoleError::Json(e.to_string()))?;
    validate_json(&value)?;
    ContentSha256::from_hex(&sha256_hex(payload_json.as_bytes()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) const BODY: &str = r#"{"action":{"action_id":"fixture-action","idempotency_key":"fixture-key","request_id":"fixture-request"},"designation":null,"expected":{"aggregate_id":"fixture-aggregate","aggregate_version":1,"base_ref":null,"base_vector_ref":null,"schema_id":"fixture-role@1"},"issuer":"00000000-0000-0000-0000-000000000002","predecessor_refs":[],"purpose":"prior_selection","retention":{"artifact_retention_ttl_days":null,"authority_class":"fixture-class","policy_ref":"fixture-policy","replay_policy_ref":"fixture-replay"},"role":"operator","run":{"coordinator_session_id":"fixture-coordinator","launch_plan_hash":"0000000000000000000000000000000000000000000000000000000000000000","locus_binding_ref":"fixture-locus","micro_task_id":"fixture-mt","owner_session":"fixture-owner","run_id":"fixture-run","task_board_id":"fixture-board","trace_id":"fixture-trace","work_packet_id":"fixture-wp"},"scope":{"access_space_id":"00000000-0000-0000-0000-000000000004","actor_principal_id":"00000000-0000-0000-0000-000000000002","authenticated_session_id":"00000000-0000-0000-0000-000000000003","owner_account_id":"00000000-0000-0000-0000-000000000001","workspace_id":"fixture-workspace"},"subject":{"artifact_ref":"fixture-artifact","manifest_ref":"fixture-manifest","sha256":"0000000000000000000000000000000000000000000000000000000000000000","version":"1"},"validator_execution":null}"#;
    pub(crate) fn body() -> RoleEvidenceBody { serde_json::from_str(BODY).unwrap() }
    pub(crate) fn receipt() -> RoleReceipt {
        RoleReceipt { body: body(), content_hash: ContentSha256::from_hex("6fa4261398ac698405c9defbcd6b2cb4897da8554488631e95606a054206b397").unwrap(),
            event: CanonicalEventKey { event_id: "00000000-0000-0000-0000-000000000009".into(), stream_id: "fixture-stream".into(), event_seq: 1, transaction_seq: 1 } }
    }
    #[test]
    fn fixed_body_and_receipt_bytes_and_hashes() {
        let body = body();
        assert_eq!(BODY.len(), 1360);
        assert_eq!(canonical_body_bytes(&body).unwrap(), BODY.as_bytes());
        assert_eq!(body_content_hash(&body).unwrap().to_hex(), "6fa4261398ac698405c9defbcd6b2cb4897da8554488631e95606a054206b397");
        let expected = format!("{{\"body\":{BODY},\"content_hash\":\"6fa4261398ac698405c9defbcd6b2cb4897da8554488631e95606a054206b397\",\"event\":{{\"event_id\":\"00000000-0000-0000-0000-000000000009\",\"event_seq\":1,\"stream_id\":\"fixture-stream\",\"transaction_seq\":1}}}}");
        assert_eq!(expected.len(), 1574);
        assert_eq!(canonical_receipt_bytes(&receipt()).unwrap(), expected.as_bytes());
        assert_eq!(canonical_event_payload_hash(&expected).unwrap().to_hex(), "09e165447c8501807c3bdec1e712d3ab454457da0ffeb3d60a61f47d1164a840");
    }
    #[test]
    fn decoded_issuer_and_role_shapes_are_checked() {
        let mut body = body();
        body.issuer = serde_json::from_str("\"00000000-0000-0000-0000-000000000008\"").unwrap();
        assert_eq!(canonical_body_bytes(&body), Err(RoleError::BindingMismatch { field: "issuer" }));
        body.issuer = body.scope.actor_principal_id;
        body.role = Role::Validator;
        assert_eq!(canonical_body_bytes(&body), Err(RoleError::InvalidField { field: "role_evidence_shape" }));
    }
    #[test]
    fn controls_and_predecessor_order_preserve_canonical_domain() {
        let mut body = body();
        body.action.action_id = "line\nreturn\rtab\t".into();
        let bytes = canonical_body_bytes(&body).unwrap();
        assert!(String::from_utf8(bytes).unwrap().contains("line\\nreturn\\rtab\\t"));
        body.action.action_id.push('\u{0008}');
        assert!(canonical_body_bytes(&body).is_err());
        assert!(canonical_event_payload_hash(r#"{"\u0000":"value"}"#).is_err());
        let mut body = self::body();
        body.predecessor_refs = vec!["first".into(), "second".into()];
        let first = body_content_hash(&body).unwrap();
        body.predecessor_refs.reverse();
        assert_ne!(body_content_hash(&body).unwrap(), first);
    }
    #[test]
    fn preparations_retain_full_validator_facts_and_ack_causality() {
        let body = body();
        let operator = ApproveOperatorArtifact { action: body.action.clone(), scope: body.scope.clone(), run: body.run.clone(), subject: body.subject.clone(), expected: body.expected.clone(), retention: body.retention.clone() };
        assert_eq!(prepare_operator(&operator).unwrap().body(), &body);
        let execution = ValidatorExecutionEvidence { runner_principal: body.issuer, runner_lane_id: "runner".into(), invocation_ref: "invocation".into(), request_message_ref: "request".into(), execution_event: receipt().event, verdict_artifact: body.subject.clone() };
        let verdict = RecordValidatorVerdict { action: body.action.clone(), scope: body.scope.clone(), designation_ref: DesignationRef::parse("eventledger://validator-designation/declared").unwrap(), execution: execution.clone(), run: body.run.clone(), subject: body.subject.clone(), purpose: Purpose::PriorSelection, expected: body.expected.clone(), retention: body.retention.clone() };
        let prepared = prepare_validator_verdict_declared(&verdict).unwrap();
        assert_eq!(prepared.body().validator_execution.as_ref(), Some(&execution));
        let mut changed = prepared.body().clone();
        changed.validator_execution.as_mut().unwrap().invocation_ref.push_str("-changed");
        assert_ne!(body_content_hash(&changed).unwrap(), body_content_hash(prepared.body()).unwrap());
        changed.validator_execution.as_mut().unwrap().runner_principal = serde_json::from_str("\"00000000-0000-0000-0000-000000000008\"").unwrap();
        assert!(canonical_body_bytes(&changed).is_err());
        let prior = RoleEvidenceRef::parse("eventledger://role-authority/prior").unwrap();
        let stage = StageAckKey { execution_id: "execution".into(), stage_id: "stage".into(), lane_id: "lane".into(), authority_request_message_ref: "request".into(), prior_receipt_ref: prior.clone(), selecting_decision_id: None };
        let mut ack = AcknowledgeOperatorStage { approval: prior.clone(), action: body.action, scope: body.scope, run: body.run, subject: body.subject, stage: stage.clone(), expected: body.expected, retention: body.retention };
        let prepared = prepare_operator_ack(&ack).unwrap();
        assert_eq!(prepared.body().purpose, Purpose::StageAcknowledgement(stage));
        assert_eq!(prepared.body().predecessor_refs, vec![prior.to_string()]);
        ack.approval = RoleEvidenceRef::parse("eventledger://role-authority/wrong").unwrap();
        assert_eq!(prepare_operator_ack(&ack), Err(RoleError::AckMismatch { field: "prior_receipt_ref" }));
        let maximum = RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", "x".repeat(512))).unwrap();
        ack.approval = maximum.clone();
        ack.stage.prior_receipt_ref = maximum.clone();
        assert_eq!(prepare_operator_ack(&ack).unwrap().body().predecessor_refs, vec![maximum.to_string()]);
        assert!(RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", "x".repeat(513))).is_err());
    }
}
