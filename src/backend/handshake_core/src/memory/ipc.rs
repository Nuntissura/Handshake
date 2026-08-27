use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use super::{
    CapsuleFlightRecorderEvent, CapsuleRecord, CapsuleSuppressedEvent, FemsFlightRecorder,
    FemsFlightRecorderError, KernelActionRejection, KernelActionSubmission, KernelActionSubmitter,
    RecordReceipt, TaskType, WriteBoxV1Envelope, FR_EVT_CAPSULE_SUPPRESSED,
    KERNEL_ACTION_REQUEST_SCHEMA_ID, MEMORY_WRITE_BOX_SCHEMA_ID, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
};
use crate::kernel::{
    action_envelope::{
        validate_kernel_action_request, ApprovalPosture, AuthorityEffect, ExpectedWriteBoxRef,
        KernelActionRequestV1, KernelActorRef, KernelSessionRef, KernelTargetRef,
        ValidationRequirement,
    },
    context_bundle::{canonical_json_bytes, sha256_hex},
    write_boxes::{
        validate_write_box_common, MemoryBox, WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState,
        WriteBoxOwnerRef, WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef,
        WriteBoxValidationState, WriteBoxValidationStatus,
    },
};

pub const MEMORY_CAPSULE_LIST_RECENT_COMMAND: &str = "kernel.memory_capsule.list_recent";
pub const MEMORY_CAPSULE_GET_COMMAND: &str = "kernel.memory_capsule.get";
pub const MEMORY_CAPSULE_SUPPRESS_ITEM_COMMAND: &str = "kernel.memory_capsule.suppress_item";
pub const MEMORY_CAPSULE_SUPPRESS_CAPSULE_COMMAND: &str = "kernel.memory_capsule.suppress_capsule";
pub const MEMORY_CAPSULE_SUPPRESS_ACTION_ID: &str = "kernel.memory_capsule.suppress";
pub const MEMORY_CAPSULE_SUPPRESS_INPUT_SCHEMA_ID: &str =
    "hsk.kernel.memory_capsule_suppression_input@1";
pub const MEMORY_CAPSULE_SUPPRESS_PAYLOAD_SCHEMA_ID: &str =
    "hsk.memory_capsule.suppression_payload@1";

pub trait MemoryCapsuleIpcStore {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError>;
    fn get_capsule_record(&self, capsule_id: Uuid)
        -> Result<Option<CapsuleRecord>, MemoryIpcError>;
    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError>;
}

pub trait CapsuleRecordStore: MemoryCapsuleIpcStore {}

impl<T> CapsuleRecordStore for T where T: MemoryCapsuleIpcStore + ?Sized {}

pub struct MemoryIpcService<'a> {
    store: &'a dyn MemoryCapsuleIpcStore,
    action_catalog: &'a dyn KernelActionSubmitter,
    fems_flight_recorder: &'a dyn FemsFlightRecorder,
}

pub type CapsuleIpcService<'a> = MemoryIpcService<'a>;

impl<'a> MemoryIpcService<'a> {
    pub fn new(
        store: &'a dyn MemoryCapsuleIpcStore,
        action_catalog: &'a dyn KernelActionSubmitter,
        fems_flight_recorder: &'a dyn FemsFlightRecorder,
    ) -> Self {
        Self {
            store,
            action_catalog,
            fems_flight_recorder,
        }
    }

    pub fn list_recent(
        &self,
        request: ListRecentCapsulesRequest,
    ) -> Result<ListRecentCapsulesResponse, MemoryIpcError> {
        let mut records = self.store.all_capsule_records()?;
        records.sort_by(|left, right| {
            right
                .built_at_utc
                .cmp(&left.built_at_utc)
                .then_with(|| right.capsule_id.cmp(&left.capsule_id))
        });

        Ok(ListRecentCapsulesResponse {
            capsules: records
                .into_iter()
                .take(request.limit as usize)
                .map(CapsuleSummary::from)
                .collect(),
        })
    }

    pub fn get(&self, request: GetCapsuleRequest) -> Result<GetCapsuleResponse, MemoryIpcError> {
        let record = self.store.get_capsule_record(request.capsule_id)?.ok_or(
            MemoryIpcError::CapsuleNotFound {
                capsule_id: request.capsule_id,
            },
        )?;

        Ok(GetCapsuleResponse { record })
    }

    pub fn suppress_item(
        &self,
        request: SuppressItemRequest,
    ) -> Result<SuppressionReceipt, MemoryIpcError> {
        let reason = normalized_reason(request.reason)?;
        let mut record = self.store.get_capsule_record(request.capsule_id)?.ok_or(
            MemoryIpcError::CapsuleNotFound {
                capsule_id: request.capsule_id,
            },
        )?;
        let mut found = false;

        for entry in &mut record.audit_log.entries {
            if entry.item_id == request.item_id {
                entry.included = false;
                entry.suppression_reason = Some(reason.clone());
                found = true;
            }
        }

        if !found {
            return Err(MemoryIpcError::ItemNotFound {
                capsule_id: request.capsule_id,
                item_id: request.item_id,
            });
        }

        let actor = SuppressionActor {
            actor_id: request.actor_id,
            session_id: request.session_id,
        };
        self.persist_suppression(record, vec![request.item_id], reason, actor)
    }

    pub fn suppress_capsule(
        &self,
        request: SuppressCapsuleRequest,
    ) -> Result<SuppressionReceipt, MemoryIpcError> {
        let reason = normalized_reason(request.reason)?;
        let mut record = self.store.get_capsule_record(request.capsule_id)?.ok_or(
            MemoryIpcError::CapsuleNotFound {
                capsule_id: request.capsule_id,
            },
        )?;
        let suppressed_item_ids = record
            .audit_log
            .entries
            .iter()
            .map(|entry| entry.item_id.clone())
            .collect::<Vec<_>>();

        for entry in &mut record.audit_log.entries {
            entry.included = false;
            entry.suppression_reason = Some(reason.clone());
        }

        let actor = SuppressionActor {
            actor_id: request.actor_id,
            session_id: request.session_id,
        };
        self.persist_suppression(record, suppressed_item_ids, reason, actor)
    }

    fn persist_suppression(
        &self,
        record: CapsuleRecord,
        suppressed_item_ids: Vec<String>,
        reason: String,
        actor: SuppressionActor,
    ) -> Result<SuppressionReceipt, MemoryIpcError> {
        let receipt = SuppressionReceipt {
            suppression_id: Uuid::now_v7(),
            write_box_envelope_id: Uuid::now_v7(),
            capsule_id: record.capsule_id,
            suppressed_item_count: suppressed_item_ids.len(),
            suppressed_item_ids,
            reason,
            suppressed_at_utc: Utc::now(),
            flight_recorder_event_id: FR_EVT_CAPSULE_SUPPRESSED.to_string(),
        };

        let submission = suppression_submission(&record, &receipt, &actor)?;
        self.action_catalog.submit(submission)?;
        self.fems_flight_recorder
            .record_event(CapsuleFlightRecorderEvent::CapsuleSuppressed(
                CapsuleSuppressedEvent {
                    capsule_id: record.capsule_id,
                    reason: receipt.reason.clone(),
                },
            ))?;
        self.store.save_capsule_record(record)?;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListRecentCapsulesRequest {
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListRecentCapsulesResponse {
    pub capsules: Vec<CapsuleSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetCapsuleRequest {
    pub capsule_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetCapsuleResponse {
    pub record: CapsuleRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppressItemRequest {
    pub capsule_id: Uuid,
    pub item_id: String,
    pub reason: String,
    pub actor_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppressCapsuleRequest {
    pub capsule_id: Uuid,
    pub reason: String,
    pub actor_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapsuleSummary {
    pub capsule_id: Uuid,
    pub task_type: TaskType,
    pub built_at_utc: DateTime<Utc>,
    pub included_count: usize,
    pub suppressed_count: usize,
    pub has_outcome: bool,
}

impl From<CapsuleRecord> for CapsuleSummary {
    fn from(record: CapsuleRecord) -> Self {
        let included_count = record
            .audit_log
            .entries
            .iter()
            .filter(|entry| entry.included)
            .count();
        let suppressed_count = record
            .audit_log
            .entries
            .len()
            .saturating_sub(included_count);

        Self {
            capsule_id: record.capsule_id,
            task_type: record.task_type,
            built_at_utc: record.built_at_utc,
            included_count,
            suppressed_count,
            has_outcome: record.outcome.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuppressionReceipt {
    pub suppression_id: Uuid,
    pub write_box_envelope_id: Uuid,
    pub capsule_id: Uuid,
    pub suppressed_item_count: usize,
    pub suppressed_item_ids: Vec<String>,
    pub reason: String,
    pub suppressed_at_utc: DateTime<Utc>,
    pub flight_recorder_event_id: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MemoryIpcError {
    #[error("unknown memory capsule {capsule_id}")]
    CapsuleNotFound { capsule_id: Uuid },
    #[error("unknown memory capsule item {item_id} for capsule {capsule_id}")]
    ItemNotFound { capsule_id: Uuid, item_id: String },
    #[error("capsule suppression reason cannot be empty")]
    EmptySuppressionReason,
    #[error("memory capsule store failed: {message}")]
    Store { message: String },
    #[error("action catalog/write-box rejected memory capsule suppression: {0}")]
    Rejected(#[from] KernelActionRejection),
    #[error("{0}")]
    FemsFlightRecorder(#[from] FemsFlightRecorderError),
    #[error("memory capsule suppression serialization failed: {0}")]
    Serialization(String),
    #[error("invalid memory capsule suppression {field}: {message}")]
    InvalidSuppressionShape {
        field: &'static str,
        message: String,
    },
}

pub type CapsuleIpcError = MemoryIpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuppressionActor {
    actor_id: String,
    session_id: String,
}

fn normalized_reason(reason: String) -> Result<String, MemoryIpcError> {
    let reason = reason.trim().to_string();
    if reason.is_empty() {
        return Err(MemoryIpcError::EmptySuppressionReason);
    }
    Ok(reason)
}

fn suppression_submission(
    record: &CapsuleRecord,
    receipt: &SuppressionReceipt,
    actor: &SuppressionActor,
) -> Result<KernelActionSubmission, MemoryIpcError> {
    let payload = suppression_payload(record, receipt)?;
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
    let write_box = suppression_write_box(record, receipt, actor, &payload_sha256);
    validate_write_box_common(&write_box.common).map_err(|errors| {
        MemoryIpcError::InvalidSuppressionShape {
            field: "write_box",
            message: format!("{errors:?}"),
        }
    })?;

    let request = suppression_action_request(record, receipt, actor);
    validate_kernel_action_request(&request).map_err(|errors| {
        MemoryIpcError::InvalidSuppressionShape {
            field: "kernel_action_request",
            message: format!("{errors:?}"),
        }
    })?;

    Ok(KernelActionSubmission {
        request,
        write_box_envelope: WriteBoxV1Envelope {
            schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
            envelope_id: receipt.write_box_envelope_id,
            payload_schema_id: MEMORY_CAPSULE_SUPPRESS_PAYLOAD_SCHEMA_ID.to_string(),
            payload,
            payload_sha256,
            write_box,
        },
        proposed_receipt: RecordReceipt {
            record_id: receipt.suppression_id,
            write_box_envelope_id: receipt.write_box_envelope_id,
            persisted_at_utc: receipt.suppressed_at_utc,
        },
    })
}

fn suppression_payload(
    record: &CapsuleRecord,
    receipt: &SuppressionReceipt,
) -> Result<Value, MemoryIpcError> {
    serde_json::to_value(json!({
        "schema_id": MEMORY_CAPSULE_SUPPRESS_PAYLOAD_SCHEMA_ID,
        "suppression_id": receipt.suppression_id,
        "capsule_id": receipt.capsule_id,
        "suppressed_item_ids": receipt.suppressed_item_ids,
        "reason": receipt.reason,
        "suppressed_at_utc": receipt.suppressed_at_utc,
        "audit_log": record.audit_log,
        "flight_recorder_event_id": receipt.flight_recorder_event_id,
    }))
    .map_err(|error| MemoryIpcError::Serialization(error.to_string()))
}

fn suppression_action_request(
    record: &CapsuleRecord,
    receipt: &SuppressionReceipt,
    actor: &SuppressionActor,
) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: MEMORY_CAPSULE_SUPPRESS_ACTION_ID.to_string(),
        actor: KernelActorRef {
            actor_id: actor.actor_id.clone(),
            actor_kind: "role".to_string(),
            role_id: actor.actor_id.clone(),
        },
        session: KernelSessionRef {
            session_id: actor.session_id.clone(),
            work_profile_id: "memory-capsule-ipc".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: record.capsule_id.to_string(),
            target_kind: "memory_capsule".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        }],
        input_schema_id: MEMORY_CAPSULE_SUPPRESS_INPUT_SCHEMA_ID.to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "MemoryBox".to_string(),
            write_box_schema_id: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            target_id: "memory_capsule_suppression".to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: suppression_validation_requirements(),
        trace_id: format!("memory-capsule-suppress:{}", receipt.suppression_id),
        idempotency_key: format!(
            "memory_capsule_suppress:{}:{}:{}",
            record.capsule_id,
            receipt.suppressed_item_ids.join(","),
            receipt.suppressed_at_utc.to_rfc3339()
        ),
    }
}

fn suppression_write_box(
    record: &CapsuleRecord,
    receipt: &SuppressionReceipt,
    actor: &SuppressionActor,
    payload_sha256: &str,
) -> MemoryBox {
    let memory_extract_ref = format!("memory-capsule-suppression://{}", receipt.suppression_id);
    MemoryBox {
        common: WriteBoxCommon {
            write_box_id: receipt.write_box_envelope_id.to_string(),
            kind: WriteBoxKind::Memory,
            schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            workspace_id: actor.session_id.clone(),
            owner: WriteBoxOwnerRef {
                actor_id: actor.actor_id.clone(),
                actor_kind: "role".to_string(),
                role_id: actor.actor_id.clone(),
            },
            crdt_site_id: "memory-capsule-ipc".to_string(),
            target_refs: vec![WriteBoxTargetRef {
                target_id: record.capsule_id.to_string(),
                target_kind: "memory_capsule".to_string(),
                authority_class: "pre_promotion_memory".to_string(),
            }],
            base_snapshot_refs: vec![format!(
                "memory-capsule-source-hash://{}",
                record.capsule_source_hash
            )],
            intent_summary: "Suppress MemoryCapsule audit entries through MemoryBox evidence"
                .to_string(),
            operation_payload_refs: vec![WriteBoxPayloadRef {
                payload_id: receipt.suppression_id.to_string(),
                payload_kind: "memory_capsule_suppression_v1".to_string(),
                payload_ref: memory_extract_ref.clone(),
                payload_sha256: payload_sha256.to_string(),
            }],
            lifecycle_state: WriteBoxLifecycleState::Open,
            allowed_transitions: vec![
                WriteBoxLifecycleState::ReadyForValidation,
                WriteBoxLifecycleState::Denied,
            ],
            authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
            evidence_refs: vec![format!("memory-capsule://{}", record.capsule_id)],
            receipt_refs: vec![format!(
                "receipt://memory-capsule-suppression/{}",
                receipt.suppression_id
            )],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: suppression_validation_check_ids(),
            },
            projection_rules: vec!["dcc.memory_capsule_review".to_string()],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("memory-capsule-suppression://{}", record.capsule_id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    actor.session_id,
                    receipt.suppressed_at_utc.to_rfc3339(),
                    receipt.suppression_id
                ),
                idempotency_key: format!(
                    "memory_capsule_suppress:{}:{}",
                    record.capsule_id, receipt.suppression_id
                ),
                source_event_refs: vec![format!(
                    "{}://{}",
                    FR_EVT_CAPSULE_SUPPRESSED, record.capsule_id
                )],
            },
        },
        memory_extract_ref,
    }
}

fn suppression_validation_requirements() -> Vec<ValidationRequirement> {
    suppression_validation_check_ids()
        .into_iter()
        .map(|check_id| ValidationRequirement {
            check_id,
            required: true,
        })
        .collect()
}

fn suppression_validation_check_ids() -> Vec<String> {
    [
        "schema_validity",
        "capsule_suppression_reason",
        "write_box_review_gate",
        "flight_recorder_event",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
