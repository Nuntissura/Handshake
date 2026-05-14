use serde::{Deserialize, Serialize};

const FOLDED_OVERLAY_COORDINATION_STUB: &str =
    "WP-1-Software-Delivery-Overlay-Coordination-Records-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimMode {
    Claim,
    Lease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeasePosture {
    Active,
    Expired,
    Released,
    RenewalRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TakeoverLegality {
    Allowed,
    BlockedActiveLease,
    RequiresApproval,
    ActorIneligible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstructionKind {
    QueuedSteering,
    FollowUp,
    DeferredEscalation,
    Renewal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstructionStatus {
    Pending,
    Applied,
    Superseded,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordinationSourceKind {
    ProductCoordinationRecord,
    WorkflowStateRecord,
    RoleMailboxStableRecord,
    DccActionReceipt,
    TransitionRegistryRecord,
    MailboxChronology,
    AdvisoryComment,
    TranscriptOrder,
}

impl CoordinationSourceKind {
    pub fn is_authoritative(self) -> bool {
        matches!(
            self,
            CoordinationSourceKind::ProductCoordinationRecord
                | CoordinationSourceKind::WorkflowStateRecord
                | CoordinationSourceKind::RoleMailboxStableRecord
                | CoordinationSourceKind::DccActionReceipt
                | CoordinationSourceKind::TransitionRegistryRecord
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationActorRefV1 {
    pub actor_id: String,
    pub actor_kind: String,
    pub role_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorEligibilityV1 {
    pub actor_kind: String,
    pub eligible: bool,
    pub reason: String,
    pub transition_rule_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCoordinationInstructionV1 {
    pub instruction_id: String,
    pub kind: InstructionKind,
    pub status: InstructionStatus,
    pub target_actor_kind: String,
    pub governed_action_id: String,
    pub stable_source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayCoordinationJoinsV1 {
    pub work_packet_id: String,
    pub task_board_item_id: String,
    pub role_mailbox_thread_id: String,
    pub dcc_projection_id: String,
    pub workflow_state_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayCoordinationRecordV1 {
    pub schema_id: String,
    pub coordination_id: String,
    pub work_item_id: String,
    pub record_seq: u64,
    pub claimant: CoordinationActorRefV1,
    pub claim_mode: ClaimMode,
    pub lease_id: String,
    pub lease_posture: LeasePosture,
    pub takeover_legality: TakeoverLegality,
    pub next_actor_kinds: Vec<String>,
    pub actor_eligibility: Vec<ActorEligibilityV1>,
    pub queued_instructions: Vec<QueuedCoordinationInstructionV1>,
    pub source_kind: CoordinationSourceKind,
    pub joins: OverlayCoordinationJoinsV1,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayCoordinationPostureProjectionV1 {
    pub schema_id: String,
    pub work_item_id: String,
    pub current_record: OverlayCoordinationRecordV1,
    pub pending_instruction_ids: Vec<String>,
    pub eligible_next_actor_kinds: Vec<String>,
    pub ignored_non_authority_source_kinds: Vec<CoordinationSourceKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayCoordinationValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_overlay_coordination_record(
    record: &OverlayCoordinationRecordV1,
) -> Result<(), Vec<OverlayCoordinationValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &record.schema_id);
    require_non_empty(&mut errors, "coordination_id", &record.coordination_id);
    require_non_empty(&mut errors, "work_item_id", &record.work_item_id);
    require_non_empty(&mut errors, "claimant.actor_id", &record.claimant.actor_id);
    require_non_empty(
        &mut errors,
        "claimant.actor_kind",
        &record.claimant.actor_kind,
    );
    require_non_empty(&mut errors, "claimant.role_id", &record.claimant.role_id);
    require_non_empty(&mut errors, "lease_id", &record.lease_id);
    require_non_empty(
        &mut errors,
        "joins.work_packet_id",
        &record.joins.work_packet_id,
    );
    require_non_empty(
        &mut errors,
        "joins.task_board_item_id",
        &record.joins.task_board_item_id,
    );
    require_non_empty(
        &mut errors,
        "joins.role_mailbox_thread_id",
        &record.joins.role_mailbox_thread_id,
    );
    require_non_empty(
        &mut errors,
        "joins.dcc_projection_id",
        &record.joins.dcc_projection_id,
    );
    require_non_empty(
        &mut errors,
        "joins.workflow_state_id",
        &record.joins.workflow_state_id,
    );

    if record.record_seq == 0 {
        errors.push(OverlayCoordinationValidationError {
            field: "record_seq",
            message: "record sequence must be greater than zero",
        });
    }

    if !record.source_kind.is_authoritative() {
        errors.push(OverlayCoordinationValidationError {
            field: "source_kind",
            message: "coordination source is not authoritative",
        });
    }

    require_vec(&mut errors, "next_actor_kinds", &record.next_actor_kinds);
    require_vec(&mut errors, "actor_eligibility", &record.actor_eligibility);
    require_vec(
        &mut errors,
        "queued_instructions",
        &record.queued_instructions,
    );

    for eligibility in &record.actor_eligibility {
        require_non_empty(
            &mut errors,
            "actor_eligibility.actor_kind",
            &eligibility.actor_kind,
        );
        require_non_empty(&mut errors, "actor_eligibility.reason", &eligibility.reason);
        require_vec(
            &mut errors,
            "actor_eligibility.transition_rule_ids",
            &eligibility.transition_rule_ids,
        );
    }

    for instruction in &record.queued_instructions {
        require_non_empty(
            &mut errors,
            "queued_instructions.instruction_id",
            &instruction.instruction_id,
        );
        require_non_empty(
            &mut errors,
            "queued_instructions.target_actor_kind",
            &instruction.target_actor_kind,
        );
        require_non_empty(
            &mut errors,
            "queued_instructions.governed_action_id",
            &instruction.governed_action_id,
        );
        require_non_empty(
            &mut errors,
            "queued_instructions.stable_source_id",
            &instruction.stable_source_id,
        );
    }

    if !record
        .folded_source_refs
        .iter()
        .any(|source| source.contains(FOLDED_OVERLAY_COORDINATION_STUB))
    {
        errors.push(OverlayCoordinationValidationError {
            field: "folded_source_refs",
            message: "folded overlay coordination source must be preserved",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn query_overlay_coordination_posture(
    records: &[OverlayCoordinationRecordV1],
    work_item_id: &str,
) -> Result<OverlayCoordinationPostureProjectionV1, Vec<OverlayCoordinationValidationError>> {
    let mut valid_records = Vec::new();
    let mut ignored_non_authority_source_kinds = Vec::new();
    let mut errors = Vec::new();

    for record in records
        .iter()
        .filter(|record| record.work_item_id == work_item_id)
    {
        match validate_overlay_coordination_record(record) {
            Ok(()) => valid_records.push(record.clone()),
            Err(record_errors) if is_only_non_authority_source_error(&record_errors) => {
                if !ignored_non_authority_source_kinds.contains(&record.source_kind) {
                    ignored_non_authority_source_kinds.push(record.source_kind);
                }
            }
            Err(record_errors) => errors.extend(record_errors),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let Some(current_record) = valid_records
        .into_iter()
        .max_by_key(|record| record.record_seq)
    else {
        return Err(vec![OverlayCoordinationValidationError {
            field: "records",
            message: "no authoritative overlay coordination record exists for work item",
        }]);
    };

    let pending_instruction_ids = current_record
        .queued_instructions
        .iter()
        .filter(|instruction| instruction.status == InstructionStatus::Pending)
        .map(|instruction| instruction.instruction_id.clone())
        .collect();

    let eligible_next_actor_kinds = current_record
        .actor_eligibility
        .iter()
        .filter(|eligibility| eligibility.eligible)
        .map(|eligibility| eligibility.actor_kind.clone())
        .collect();

    Ok(OverlayCoordinationPostureProjectionV1 {
        schema_id: "hsk.kernel.overlay_coordination_posture_projection@1".to_string(),
        work_item_id: work_item_id.to_string(),
        current_record,
        pending_instruction_ids,
        eligible_next_actor_kinds,
        ignored_non_authority_source_kinds,
    })
}

fn require_non_empty(
    errors: &mut Vec<OverlayCoordinationValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(OverlayCoordinationValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<OverlayCoordinationValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(OverlayCoordinationValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn is_only_non_authority_source_error(errors: &[OverlayCoordinationValidationError]) -> bool {
    errors.len() == 1 && errors[0].field == "source_kind"
}
