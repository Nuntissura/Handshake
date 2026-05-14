use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelActorRef {
    pub actor_id: String,
    pub actor_kind: String,
    pub role_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSessionRef {
    pub session_id: String,
    pub work_profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelTargetRef {
    pub target_id: String,
    pub target_kind: String,
    pub authority_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedWriteBoxRef {
    pub write_box_kind: String,
    pub write_box_schema_id: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorityEffect {
    None,
    PrePromotionEvidenceOnly,
    EventLedgerAuthorityWrite,
    ProjectionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPosture {
    NoApprovalRequired,
    RequiresPromotionGate,
    RequiresHumanApproval,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationRequirement {
    pub check_id: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelActionRequestV1 {
    pub schema_id: String,
    pub action_id: String,
    pub actor: KernelActorRef,
    pub session: KernelSessionRef,
    pub target_ids: Vec<KernelTargetRef>,
    pub input_schema_id: String,
    pub expected_write_boxes: Vec<ExpectedWriteBoxRef>,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub validation_requirements: Vec<ValidationRequirement>,
    pub trace_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelReceiptMapping {
    pub receipt_kind: String,
    pub receipt_schema_id: String,
    pub correlation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLedgerMapping {
    pub event_kind: String,
    pub event_schema_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelActionResultStatus {
    Accepted,
    WriteBoxesCreated,
    PromotionQueued,
    Promoted,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelActionResultV1 {
    pub schema_id: String,
    pub result_id: String,
    pub request_trace_id: String,
    pub status: KernelActionResultStatus,
    pub write_box_ids: Vec<String>,
    pub receipt_mappings: Vec<KernelReceiptMapping>,
    pub event_mappings: Vec<EventLedgerMapping>,
    pub denial: Option<KernelActionDenialV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelActionDenialV1 {
    pub schema_id: String,
    pub denial_id: String,
    pub request_trace_id: String,
    pub denial_code: String,
    pub reason: String,
    pub lawful_replacement_action_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub receipt_mappings: Vec<KernelReceiptMapping>,
    pub event_mappings: Vec<EventLedgerMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelActionRequestValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_kernel_action_request(
    request: &KernelActionRequestV1,
) -> Result<(), Vec<KernelActionRequestValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &request.schema_id);
    require_non_empty(&mut errors, "action_id", &request.action_id);
    require_non_empty(&mut errors, "actor.actor_id", &request.actor.actor_id);
    require_non_empty(&mut errors, "actor.actor_kind", &request.actor.actor_kind);
    require_non_empty(&mut errors, "actor.role_id", &request.actor.role_id);
    require_non_empty(
        &mut errors,
        "session.session_id",
        &request.session.session_id,
    );
    require_non_empty(
        &mut errors,
        "session.work_profile_id",
        &request.session.work_profile_id,
    );
    require_non_empty(&mut errors, "input_schema_id", &request.input_schema_id);
    require_non_empty(&mut errors, "trace_id", &request.trace_id);
    require_non_empty(&mut errors, "idempotency_key", &request.idempotency_key);

    if request.target_ids.is_empty() {
        errors.push(KernelActionRequestValidationError {
            field: "target_ids",
            message: "at least one target id is required",
        });
    }

    if request.expected_write_boxes.is_empty() {
        errors.push(KernelActionRequestValidationError {
            field: "expected_write_boxes",
            message: "at least one expected write box is required",
        });
    }

    if request.validation_requirements.is_empty() {
        errors.push(KernelActionRequestValidationError {
            field: "validation_requirements",
            message: "at least one validation requirement is required",
        });
    }

    for target in &request.target_ids {
        require_non_empty(&mut errors, "target_ids.target_id", &target.target_id);
        require_non_empty(&mut errors, "target_ids.target_kind", &target.target_kind);
        require_non_empty(
            &mut errors,
            "target_ids.authority_class",
            &target.authority_class,
        );
    }

    for write_box in &request.expected_write_boxes {
        require_non_empty(
            &mut errors,
            "expected_write_boxes.write_box_kind",
            &write_box.write_box_kind,
        );
        require_non_empty(
            &mut errors,
            "expected_write_boxes.write_box_schema_id",
            &write_box.write_box_schema_id,
        );
        require_non_empty(
            &mut errors,
            "expected_write_boxes.target_id",
            &write_box.target_id,
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn require_non_empty(
    errors: &mut Vec<KernelActionRequestValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(KernelActionRequestValidationError {
            field,
            message: "value must not be empty",
        });
    }
}
