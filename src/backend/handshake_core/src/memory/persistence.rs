use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::capsule::{CapsuleAuditLog, MemoryCapsule, RetrievalPolicy, TaskType};
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

pub const MEMORY_CAPSULE_RECORD_ACTION_ID: &str = "kernel.memory_capsule.record";
pub const MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID: &str = "hsk.kernel.memory_capsule_record_input@1";
pub const MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_capsule.record_payload@1";
pub const KERNEL_ACTION_REQUEST_SCHEMA_ID: &str = "hsk.kernel_action_request@1";
pub const WRITE_BOX_V1_ENVELOPE_SCHEMA_ID: &str = "hsk.write_box_v1_envelope@1";
pub const MEMORY_WRITE_BOX_SCHEMA_ID: &str = "hsk.write_box.memory@1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum CapsuleOutcome {
    Accepted,
    Skipped { reason: String },
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapsuleRecord {
    pub capsule_id: Uuid,
    pub capsule_source_hash: String,
    pub task_type: TaskType,
    pub policy: RetrievalPolicy,
    pub audit_log: CapsuleAuditLog,
    pub built_at_utc: DateTime<Utc>,
    pub recorded_at_utc: DateTime<Utc>,
    pub session_id: String,
    pub role_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<CapsuleOutcome>,
}

impl CapsuleRecord {
    pub fn from_capsule(
        capsule: &MemoryCapsule,
        recorded_at_utc: DateTime<Utc>,
        session_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> Self {
        Self {
            capsule_id: capsule.id,
            capsule_source_hash: capsule.source_hash.clone(),
            task_type: capsule.task_type,
            policy: capsule.policy.clone(),
            audit_log: capsule.audit.clone(),
            built_at_utc: capsule.built_at_utc,
            recorded_at_utc,
            session_id: session_id.into(),
            role_id: role_id.into(),
            outcome: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordReceipt {
    pub record_id: Uuid,
    pub write_box_envelope_id: Uuid,
    pub persisted_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KernelActionSubmission {
    pub request: KernelActionRequestV1,
    pub write_box_envelope: WriteBoxV1Envelope,
    pub proposed_receipt: RecordReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WriteBoxV1Envelope {
    pub schema_id: String,
    pub envelope_id: Uuid,
    pub payload_schema_id: String,
    pub payload: Value,
    pub payload_sha256: String,
    pub write_box: MemoryBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelActionRejection {
    pub code: String,
    pub reason: String,
}

impl std::fmt::Display for KernelActionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.reason)
    }
}

impl std::error::Error for KernelActionRejection {}

pub trait KernelActionSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection>;
}

pub struct CapsuleRecorder<'a> {
    pub action_catalog: &'a dyn KernelActionSubmitter,
}

impl<'a> CapsuleRecorder<'a> {
    pub fn record(&self, record: CapsuleRecord) -> Result<RecordReceipt, RecorderError> {
        validate_record(&record)?;

        let receipt = RecordReceipt {
            record_id: Uuid::now_v7(),
            write_box_envelope_id: Uuid::now_v7(),
            persisted_at_utc: Utc::now(),
        };
        let payload = payload_value(&record, receipt.record_id)?;
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
        let write_box = memory_write_box(&record, &receipt, &payload_sha256);
        validate_write_box_common(&write_box.common).map_err(|errors| {
            RecorderError::InvalidRecordShape {
                field: "write_box",
                message: format!("{errors:?}"),
            }
        })?;

        let request = action_request(&record, &receipt);
        validate_kernel_action_request(&request).map_err(|errors| {
            RecorderError::InvalidRecordShape {
                field: "kernel_action_request",
                message: format!("{errors:?}"),
            }
        })?;

        let submission = KernelActionSubmission {
            request,
            write_box_envelope: WriteBoxV1Envelope {
                schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
                envelope_id: receipt.write_box_envelope_id,
                payload_schema_id: MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID.to_string(),
                payload,
                payload_sha256,
                write_box,
            },
            proposed_receipt: receipt.clone(),
        };

        self.action_catalog.submit(submission)?;
        Ok(receipt)
    }
}

fn validate_record(record: &CapsuleRecord) -> Result<(), RecorderError> {
    if record.capsule_id.is_nil() {
        return invalid_record("capsule_id", "capsule id must not be nil");
    }
    if !is_sha256_hex(&record.capsule_source_hash) {
        return invalid_record(
            "capsule_source_hash",
            "capsule source hash must be a sha256 hex digest",
        );
    }
    if record.policy.task_type != record.task_type {
        return invalid_record(
            "policy.task_type",
            "record task type must match retrieval policy task type",
        );
    }
    if record.built_at_utc > record.recorded_at_utc {
        return invalid_record(
            "recorded_at_utc",
            "recorded timestamp must not precede built timestamp",
        );
    }
    if record.session_id.trim().is_empty() {
        return invalid_record("session_id", "session id must not be empty");
    }
    if record.role_id.trim().is_empty() {
        return invalid_record("role_id", "role id must not be empty");
    }
    for entry in &record.audit_log.entries {
        if entry.item_id.trim().is_empty() {
            return invalid_record("audit_log.entries.item_id", "item id must not be empty");
        }
        if entry.source_uri.trim().is_empty() {
            return invalid_record(
                "audit_log.entries.source_uri",
                "source uri must not be empty",
            );
        }
        if !entry.score.is_finite() {
            return invalid_record("audit_log.entries.score", "score must be finite");
        }
        for value in entry.score_breakdown.values() {
            if !value.is_finite() {
                return invalid_record(
                    "audit_log.entries.score_breakdown",
                    "score breakdown values must be finite",
                );
            }
        }
    }
    Ok(())
}

fn invalid_record<T>(field: &'static str, message: impl Into<String>) -> Result<T, RecorderError> {
    Err(RecorderError::InvalidRecordShape {
        field,
        message: message.into(),
    })
}

fn payload_value(record: &CapsuleRecord, record_id: Uuid) -> Result<Value, RecorderError> {
    serde_json::to_value(CapsuleRecordPayload {
        schema_id: MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID,
        record_id,
        record,
    })
    .map_err(|error| RecorderError::Serialization(error.to_string()))
}

#[derive(Serialize)]
struct CapsuleRecordPayload<'a> {
    schema_id: &'static str,
    record_id: Uuid,
    record: &'a CapsuleRecord,
}

fn action_request(record: &CapsuleRecord, receipt: &RecordReceipt) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: MEMORY_CAPSULE_RECORD_ACTION_ID.to_string(),
        actor: KernelActorRef {
            actor_id: record.role_id.clone(),
            actor_kind: "role".to_string(),
            role_id: record.role_id.clone(),
        },
        session: KernelSessionRef {
            session_id: record.session_id.clone(),
            work_profile_id: "memory-capsule-persistence".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: record.capsule_id.to_string(),
            target_kind: "memory_capsule".to_string(),
            authority_class: "pre_promotion_memory".to_string(),
        }],
        input_schema_id: MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID.to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "MemoryBox".to_string(),
            write_box_schema_id: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            target_id: "memory_capsule_record".to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: memory_validation_requirements(),
        trace_id: format!("memory-capsule-record:{}", receipt.record_id),
        idempotency_key: idempotency_key(record),
    }
}

fn memory_write_box(
    record: &CapsuleRecord,
    receipt: &RecordReceipt,
    payload_sha256: &str,
) -> MemoryBox {
    let memory_extract_ref = format!("memory-capsule-record://{}", receipt.record_id);
    MemoryBox {
        common: WriteBoxCommon {
            write_box_id: receipt.write_box_envelope_id.to_string(),
            kind: WriteBoxKind::Memory,
            schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
            workspace_id: record.session_id.clone(),
            owner: WriteBoxOwnerRef {
                actor_id: record.role_id.clone(),
                actor_kind: "role".to_string(),
                role_id: record.role_id.clone(),
            },
            crdt_site_id: "memory-capsule-recorder".to_string(),
            target_refs: vec![WriteBoxTargetRef {
                target_id: record.capsule_id.to_string(),
                target_kind: "memory_capsule".to_string(),
                authority_class: "pre_promotion_memory".to_string(),
            }],
            base_snapshot_refs: vec![format!(
                "memory-capsule-source-hash://{}",
                record.capsule_source_hash
            )],
            intent_summary: "Record MemoryCapsule metadata through MemoryBox evidence".to_string(),
            operation_payload_refs: vec![WriteBoxPayloadRef {
                payload_id: receipt.record_id.to_string(),
                payload_kind: "memory_capsule_record_v1".to_string(),
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
                "receipt://memory-capsule-record/{}",
                receipt.record_id
            )],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: memory_validation_check_ids(),
            },
            projection_rules: vec!["dcc.memory_queue".to_string()],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("memory-capsule-record://{}", record.capsule_id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    record.session_id,
                    record.recorded_at_utc.to_rfc3339(),
                    receipt.record_id
                ),
                idempotency_key: idempotency_key(record),
                source_event_refs: vec![format!("memory-capsule://{}", record.capsule_id)],
            },
        },
        memory_extract_ref,
    }
}

fn memory_validation_requirements() -> Vec<ValidationRequirement> {
    memory_validation_check_ids()
        .into_iter()
        .map(|check_id| ValidationRequirement {
            check_id,
            required: true,
        })
        .collect()
}

fn memory_validation_check_ids() -> Vec<String> {
    ["schema_validity", "novelty", "contradiction", "dedup"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn idempotency_key(record: &CapsuleRecord) -> String {
    format!(
        "memory_capsule_record:{}:{}",
        record.capsule_id, record.capsule_source_hash
    )
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecorderError {
    #[error("action catalog/write-box rejected memory capsule record: {0}")]
    Rejected(#[from] KernelActionRejection),
    #[error("memory capsule record serialization failed: {0}")]
    Serialization(String),
    #[error("invalid memory capsule record {field}: {message}")]
    InvalidRecordShape {
        field: &'static str,
        message: String,
    },
}
