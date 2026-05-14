use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::{ApprovalPosture, AuthorityEffect};

pub const FOLDED_ROLE_MAILBOX_CONTRACT_STUB_ID: &str =
    "WP-1-Role-Mailbox-Message-Thread-Contract-v1";

const REQUIRED_RESPONSES: [RoleMailboxAllowedResponseKind; 7] = [
    RoleMailboxAllowedResponseKind::Acknowledge,
    RoleMailboxAllowedResponseKind::Snooze,
    RoleMailboxAllowedResponseKind::Reply,
    RoleMailboxAllowedResponseKind::Escalate,
    RoleMailboxAllowedResponseKind::Delegate,
    RoleMailboxAllowedResponseKind::Resolve,
    RoleMailboxAllowedResponseKind::RequestTranscription,
];

const REQUIRED_MESSAGE_FAMILIES: [RoleMailboxMessageFamily; 5] = [
    RoleMailboxMessageFamily::Request,
    RoleMailboxMessageFamily::Feedback,
    RoleMailboxMessageFamily::Verification,
    RoleMailboxMessageFamily::Escalation,
    RoleMailboxMessageFamily::CompletionReport,
];

const REQUIRED_DCC_TRIAGE_FIELDS: [&str; 6] = [
    "thread_lifecycle_state",
    "message_delivery_state",
    "allowed_responses",
    "due_posture",
    "dead_letter_posture",
    "action_request_boundary",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxThreadLifecycleState {
    Open,
    AwaitingResponse,
    Escalated,
    Resolved,
    Expired,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxDeliveryState {
    Queued,
    Delivered,
    Acknowledged,
    Replied,
    Ignored,
    Failed,
    DeadLettered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxAllowedResponseKind {
    Acknowledge = 1,
    Snooze = 2,
    Reply = 3,
    Escalate = 4,
    Delegate = 5,
    Resolve = 6,
    RequestTranscription = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxAuthorityBoundary {
    MailboxLocal,
    GovernedActionRequired,
    TranscriptionRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DuePosture {
    NotDue,
    DueSoon,
    Overdue,
    Snoozed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeadLetterPosture {
    None,
    Retryable,
    RequiresRemediation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleMailboxMessageFamily {
    Request,
    Feedback,
    Verification,
    Escalation,
    CompletionReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedRecordRefV1 {
    pub record_id: String,
    pub record_kind: String,
    pub authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxAllowedResponseV1 {
    pub response_kind: RoleMailboxAllowedResponseKind,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub requires_action_request: bool,
    pub display_order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxActionRequestV1 {
    pub request_id: String,
    pub response_kind: RoleMailboxAllowedResponseKind,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub target_record_ids: Vec<String>,
    pub kernel_action_id: Option<String>,
    pub approval_posture: ApprovalPosture,
    pub evidence_refs: Vec<String>,
    pub transcription_target_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxThreadContractV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub folded_stub_id: String,
    pub thread_id: String,
    pub lifecycle_state: RoleMailboxThreadLifecycleState,
    pub latest_delivery_state: RoleMailboxDeliveryState,
    pub due_posture: DuePosture,
    pub dead_letter_posture: DeadLetterPosture,
    pub linked_records: Vec<LinkedRecordRefV1>,
    pub allowed_responses: Vec<RoleMailboxAllowedResponseV1>,
    pub action_requests: Vec<RoleMailboxActionRequestV1>,
    pub message_families: Vec<RoleMailboxMessageFamily>,
    pub dcc_triage_fields: Vec<String>,
    pub mailbox_local_actions_mutate_linked_authority: bool,
    pub transcript_order_authority_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxTriageProjectionV1 {
    pub schema_id: String,
    pub thread_id: String,
    pub lifecycle_state: RoleMailboxThreadLifecycleState,
    pub latest_delivery_state: RoleMailboxDeliveryState,
    pub due_posture: DuePosture,
    pub dead_letter_posture: DeadLetterPosture,
    pub mailbox_local_actions: Vec<RoleMailboxAllowedResponseKind>,
    pub governed_actions: Vec<RoleMailboxAllowedResponseKind>,
    pub transcription_actions: Vec<RoleMailboxAllowedResponseKind>,
    pub linked_record_ids: Vec<String>,
    pub dead_letter_visible: bool,
    pub mutates_linked_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxResponsePreviewV1 {
    pub schema_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub response_kind: RoleMailboxAllowedResponseKind,
    pub boundary: RoleMailboxAuthorityBoundary,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub kernel_action_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub transcription_target_required: bool,
    pub mutates_linked_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxContractValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_thread_contract(
    thread: &RoleMailboxThreadContractV1,
) -> Result<(), Vec<RoleMailboxContractValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &thread.schema_id);
    require_non_empty(&mut errors, "contract_id", &thread.contract_id);
    require_non_empty(&mut errors, "folded_stub_id", &thread.folded_stub_id);
    require_non_empty(&mut errors, "thread_id", &thread.thread_id);
    require_vec(&mut errors, "linked_records", &thread.linked_records);
    require_vec(&mut errors, "allowed_responses", &thread.allowed_responses);
    require_vec(&mut errors, "action_requests", &thread.action_requests);
    require_vec(&mut errors, "message_families", &thread.message_families);
    require_vec(&mut errors, "dcc_triage_fields", &thread.dcc_triage_fields);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &thread.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &thread.folded_source_refs,
    );

    if thread.folded_stub_id != FOLDED_ROLE_MAILBOX_CONTRACT_STUB_ID {
        errors.push(RoleMailboxContractValidationError {
            field: "folded_stub_id",
            message: "thread contract must bind the folded Role Mailbox message/thread stub",
        });
    }

    if !contains_text(
        &thread.folded_source_refs,
        FOLDED_ROLE_MAILBOX_CONTRACT_STUB_ID,
    ) {
        errors.push(RoleMailboxContractValidationError {
            field: "folded_source_refs",
            message: "folded Role Mailbox message/thread source must be preserved",
        });
    }

    if thread.mailbox_local_actions_mutate_linked_authority {
        errors.push(RoleMailboxContractValidationError {
            field: "mailbox_local_actions_mutate_linked_authority",
            message: "mailbox-local actions must not mutate linked authoritative records",
        });
    }

    if thread.transcript_order_authority_allowed {
        errors.push(RoleMailboxContractValidationError {
            field: "transcript_order_authority_allowed",
            message: "transcript or mailbox chronology must not become authority",
        });
    }

    validate_posture(&mut errors, thread);
    validate_refs(&mut errors, thread);
    validate_allowed_responses(&mut errors, thread);
    validate_action_requests(&mut errors, thread);
    validate_message_families(&mut errors, &thread.message_families);
    validate_dcc_triage_fields(&mut errors, &thread.dcc_triage_fields);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_role_mailbox_triage(
    thread: &RoleMailboxThreadContractV1,
) -> Result<RoleMailboxTriageProjectionV1, Vec<RoleMailboxContractValidationError>> {
    validate_role_mailbox_thread_contract(thread)?;

    Ok(RoleMailboxTriageProjectionV1 {
        schema_id: "hsk.kernel.role_mailbox_triage_projection@1".to_string(),
        thread_id: thread.thread_id.clone(),
        lifecycle_state: thread.lifecycle_state,
        latest_delivery_state: thread.latest_delivery_state,
        due_posture: thread.due_posture,
        dead_letter_posture: thread.dead_letter_posture,
        mailbox_local_actions: actions_for_boundary(
            &thread.allowed_responses,
            RoleMailboxAuthorityBoundary::MailboxLocal,
        ),
        governed_actions: actions_for_boundary(
            &thread.allowed_responses,
            RoleMailboxAuthorityBoundary::GovernedActionRequired,
        ),
        transcription_actions: actions_for_boundary(
            &thread.allowed_responses,
            RoleMailboxAuthorityBoundary::TranscriptionRequired,
        ),
        linked_record_ids: thread
            .linked_records
            .iter()
            .map(|record| record.record_id.clone())
            .collect(),
        dead_letter_visible: thread.dead_letter_posture != DeadLetterPosture::None,
        mutates_linked_authority: false,
    })
}

pub fn preview_role_mailbox_response(
    thread: &RoleMailboxThreadContractV1,
    response_kind: RoleMailboxAllowedResponseKind,
    action_request_id: &str,
) -> Result<RoleMailboxResponsePreviewV1, Vec<RoleMailboxContractValidationError>> {
    validate_role_mailbox_thread_contract(thread)?;

    let Some(action_request) = thread.action_requests.iter().find(|request| {
        request.request_id == action_request_id && request.response_kind == response_kind
    }) else {
        return Err(vec![RoleMailboxContractValidationError {
            field: "action_request_id",
            message: "requested mailbox action request is not registered",
        }]);
    };

    Ok(RoleMailboxResponsePreviewV1 {
        schema_id: "hsk.kernel.role_mailbox_response_preview@1".to_string(),
        thread_id: thread.thread_id.clone(),
        request_id: action_request.request_id.clone(),
        response_kind,
        boundary: action_request.boundary,
        authority_effect: authority_effect_for_boundary(action_request.boundary),
        approval_posture: action_request.approval_posture,
        kernel_action_id: action_request.kernel_action_id.clone(),
        evidence_refs: action_request.evidence_refs.clone(),
        transcription_target_required: action_request.transcription_target_required,
        mutates_linked_authority: false,
    })
}

fn validate_posture(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    thread: &RoleMailboxThreadContractV1,
) {
    if thread.latest_delivery_state == RoleMailboxDeliveryState::DeadLettered
        && thread.dead_letter_posture == DeadLetterPosture::None
    {
        errors.push(RoleMailboxContractValidationError {
            field: "dead_letter_posture",
            message: "dead-lettered messages require visible dead-letter posture",
        });
    }

    if thread.due_posture == DuePosture::Expired
        && !matches!(
            thread.lifecycle_state,
            RoleMailboxThreadLifecycleState::Expired
                | RoleMailboxThreadLifecycleState::Escalated
                | RoleMailboxThreadLifecycleState::AwaitingResponse
        )
    {
        errors.push(RoleMailboxContractValidationError {
            field: "due_posture",
            message: "expired due posture must align with an active, escalated, or expired thread",
        });
    }
}

fn validate_refs(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    thread: &RoleMailboxThreadContractV1,
) {
    for required_ref in [
        "kernel.action_catalog",
        "kernel.workflow_transition_registry",
        "kernel.locus_work_tracking",
        "kernel.dcc_layout_projection_registry",
    ] {
        if !contains_exact(&thread.product_authority_refs, required_ref) {
            errors.push(RoleMailboxContractValidationError {
                field: "product_authority_refs",
                message: "Role Mailbox contract must cite catalog, workflow, Locus, and DCC triage authority refs",
            });
        }
    }

    let mut record_ids = HashSet::new();
    for record in &thread.linked_records {
        if !record_ids.insert(record.record_id.as_str()) {
            errors.push(RoleMailboxContractValidationError {
                field: "linked_records.record_id",
                message: "linked record ids must be unique",
            });
        }
        require_non_empty(errors, "linked_records.record_id", &record.record_id);
        require_non_empty(errors, "linked_records.record_kind", &record.record_kind);
        require_non_empty(
            errors,
            "linked_records.authority_ref",
            &record.authority_ref,
        );
    }
}

fn validate_allowed_responses(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    thread: &RoleMailboxThreadContractV1,
) {
    let mut response_kinds = HashSet::new();
    let mut display_orders = HashSet::new();
    for response in &thread.allowed_responses {
        if !response_kinds.insert(response.response_kind) {
            errors.push(RoleMailboxContractValidationError {
                field: "allowed_responses.response_kind",
                message: "allowed response kinds must be unique",
            });
        }
        if !display_orders.insert(response.display_order) {
            errors.push(RoleMailboxContractValidationError {
                field: "allowed_responses.display_order",
                message: "allowed response display order must be unique",
            });
        }
        if !response.requires_action_request {
            errors.push(RoleMailboxContractValidationError {
                field: "allowed_responses.requires_action_request",
                message: "allowed responses must expose action-request metadata before use",
            });
        }
        if expected_boundary(response.response_kind) != response.boundary {
            errors.push(RoleMailboxContractValidationError {
                field: "allowed_responses.boundary",
                message: "allowed response boundary does not match mailbox action law",
            });
        }
    }

    for required_response in REQUIRED_RESPONSES {
        if !response_kinds.contains(&required_response) {
            errors.push(RoleMailboxContractValidationError {
                field: "allowed_responses.response_kind",
                message: "acknowledge, snooze, reply, escalate, delegate, resolve, and transcription responses are required",
            });
        }
    }
}

fn validate_action_requests(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    thread: &RoleMailboxThreadContractV1,
) {
    let linked_record_ids: HashSet<&str> = thread
        .linked_records
        .iter()
        .map(|record| record.record_id.as_str())
        .collect();
    let allowed_response_kinds: HashSet<RoleMailboxAllowedResponseKind> = thread
        .allowed_responses
        .iter()
        .map(|response| response.response_kind)
        .collect();

    let mut request_ids = HashSet::new();
    let mut request_response_kinds = HashSet::new();
    for request in &thread.action_requests {
        if !request_ids.insert(request.request_id.as_str()) {
            errors.push(RoleMailboxContractValidationError {
                field: "action_requests.request_id",
                message: "action request ids must be unique",
            });
        }
        require_non_empty(errors, "action_requests.request_id", &request.request_id);
        require_vec(
            errors,
            "action_requests.target_record_ids",
            &request.target_record_ids,
        );
        require_vec(
            errors,
            "action_requests.evidence_refs",
            &request.evidence_refs,
        );

        if !allowed_response_kinds.contains(&request.response_kind) {
            errors.push(RoleMailboxContractValidationError {
                field: "action_requests.response_kind",
                message: "action request response kind must be allowed",
            });
        }
        request_response_kinds.insert(request.response_kind);
        if expected_boundary(request.response_kind) != request.boundary {
            errors.push(RoleMailboxContractValidationError {
                field: "action_requests.boundary",
                message: "action request boundary does not match mailbox response law",
            });
        }
        for target_id in &request.target_record_ids {
            if !linked_record_ids.contains(target_id.as_str()) {
                errors.push(RoleMailboxContractValidationError {
                    field: "action_requests.target_record_ids",
                    message: "action requests must target linked records by stable id",
                });
            }
        }

        match request.boundary {
            RoleMailboxAuthorityBoundary::MailboxLocal => {
                if request.kernel_action_id.is_some() {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.kernel_action_id",
                        message: "mailbox-local actions must not carry linked authority action ids",
                    });
                }
                if request.transcription_target_required {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.transcription_target_required",
                        message: "mailbox-local actions do not require transcription targets",
                    });
                }
            }
            RoleMailboxAuthorityBoundary::GovernedActionRequired => {
                if request
                    .kernel_action_id
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.kernel_action_id",
                        message: "governed mailbox actions require a catalog action id",
                    });
                }
                if request.transcription_target_required {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.transcription_target_required",
                        message: "governed actions use catalog actions, not transcription targets",
                    });
                }
            }
            RoleMailboxAuthorityBoundary::TranscriptionRequired => {
                if request.kernel_action_id.is_some() {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.kernel_action_id",
                        message:
                            "transcription actions should not impersonate linked authority actions",
                    });
                }
                if !request.transcription_target_required {
                    errors.push(RoleMailboxContractValidationError {
                        field: "action_requests.transcription_target_required",
                        message: "transcription actions require an explicit transcription target",
                    });
                }
            }
        }
    }

    for response in &thread.allowed_responses {
        if response.requires_action_request
            && !request_response_kinds.contains(&response.response_kind)
        {
            errors.push(RoleMailboxContractValidationError {
                field: "action_requests.response_kind",
                message: "each allowed response must expose matching action-request metadata",
            });
        }
    }
}

fn validate_message_families(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    message_families: &[RoleMailboxMessageFamily],
) {
    let observed: HashSet<RoleMailboxMessageFamily> = message_families.iter().copied().collect();
    for required_family in REQUIRED_MESSAGE_FAMILIES {
        if !observed.contains(&required_family) {
            errors.push(RoleMailboxContractValidationError {
                field: "message_families",
                message: "request, feedback, verification, escalation, and completion-report message families are required",
            });
        }
    }
}

fn validate_dcc_triage_fields(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    dcc_triage_fields: &[String],
) {
    for required_field in REQUIRED_DCC_TRIAGE_FIELDS {
        if !contains_exact(dcc_triage_fields, required_field) {
            errors.push(RoleMailboxContractValidationError {
                field: "dcc_triage_fields",
                message: "DCC triage must expose lifecycle, delivery, allowed responses, due, dead-letter, and action boundary fields",
            });
        }
    }
}

fn actions_for_boundary(
    responses: &[RoleMailboxAllowedResponseV1],
    boundary: RoleMailboxAuthorityBoundary,
) -> Vec<RoleMailboxAllowedResponseKind> {
    responses
        .iter()
        .filter(|response| response.boundary == boundary)
        .map(|response| response.response_kind)
        .collect()
}

fn expected_boundary(
    response_kind: RoleMailboxAllowedResponseKind,
) -> RoleMailboxAuthorityBoundary {
    match response_kind {
        RoleMailboxAllowedResponseKind::Acknowledge
        | RoleMailboxAllowedResponseKind::Snooze
        | RoleMailboxAllowedResponseKind::Reply => RoleMailboxAuthorityBoundary::MailboxLocal,
        RoleMailboxAllowedResponseKind::Escalate
        | RoleMailboxAllowedResponseKind::Delegate
        | RoleMailboxAllowedResponseKind::Resolve => {
            RoleMailboxAuthorityBoundary::GovernedActionRequired
        }
        RoleMailboxAllowedResponseKind::RequestTranscription => {
            RoleMailboxAuthorityBoundary::TranscriptionRequired
        }
    }
}

fn authority_effect_for_boundary(boundary: RoleMailboxAuthorityBoundary) -> AuthorityEffect {
    match boundary {
        RoleMailboxAuthorityBoundary::MailboxLocal => AuthorityEffect::None,
        RoleMailboxAuthorityBoundary::GovernedActionRequired => {
            AuthorityEffect::PrePromotionEvidenceOnly
        }
        RoleMailboxAuthorityBoundary::TranscriptionRequired => {
            AuthorityEffect::PrePromotionEvidenceOnly
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleMailboxContractValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleMailboxContractValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleMailboxContractValidationError {
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
