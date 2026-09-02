use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, future::Future, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

use super::capsule::{CapsuleAuditLog, MemoryCapsule, RetrievalPolicy, TaskType};
use super::hygiene::{
    hygiene_submission, HygieneActionSubmitter, HygieneCandidate, HygieneError,
    ProceduralPromotion, HYGIENE_CONSOLIDATION_ACTION_ID, HYGIENE_FLAG_ACTION_ID,
    HYGIENE_PAYLOAD_SCHEMA_ID, HYGIENE_PROMOTE_ACTION_ID, HYGIENE_PRUNE_ACTION_ID,
    MEMORY_HYGIENE_SOURCE_COMPONENT,
};
use super::pinned_core::{
    action_id_for_pin_state, fr_event_for_pin_state, pin_submission, PinError, PinReceipt,
    PinSubmitter, PinnedItem, MEMORY_PIN_AGGREGATE_TYPE, MEMORY_PIN_MANIFEST_AGGREGATE_ID,
    MEMORY_PIN_MANIFEST_AGGREGATE_TYPE, MEMORY_PIN_SOURCE_COMPONENT, PIN_MEMORY_ACTION_ID,
    PIN_MEMORY_PAYLOAD_SCHEMA_ID, UNPIN_MEMORY_ACTION_ID,
};
use crate::kernel::{
    action_catalog::{kernel002_action_catalog, KernelActionCatalogV1, KernelCatalogActionV1},
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
    KernelActor, KernelEventType, NewKernelEvent,
};
use crate::storage::{Database, StorageError};

pub const MEMORY_CAPSULE_RECORD_ACTION_ID: &str = "kernel.memory_capsule.record";
pub const MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID: &str = "hsk.kernel.memory_capsule_record_input@1";
pub const MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID: &str = "hsk.memory_capsule.record_payload@1";
pub const KERNEL_ACTION_REQUEST_SCHEMA_ID: &str = "hsk.kernel_action_request@1";
pub const WRITE_BOX_V1_ENVELOPE_SCHEMA_ID: &str = "hsk.write_box_v1_envelope@1";
pub const MEMORY_WRITE_BOX_SCHEMA_ID: &str = "hsk.write_box.memory@1";
pub const MEMORY_CAPSULE_AGGREGATE_TYPE: &str = "memory_capsule";
pub const MEMORY_CAPSULE_SOURCE_COMPONENT: &str = "memory_capsule_kernel_action_catalog";

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

fn block_on_storage<F>(future: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle)
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) =>
        {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("tokio current-thread runtime must build")
                        .block_on(future)
                })
                .join()
                .expect("dedicated storage runtime thread must not panic")
        }),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio current-thread runtime must build")
            .block_on(future),
    }
}

/// Synchronous kernel-action adapter backed by Handshake's embedded Surreal EventLedger.
pub struct SurrealKernelActionSubmitter {
    db: Arc<dyn Database>,
    catalog: KernelActionCatalogV1,
}

impl SurrealKernelActionSubmitter {
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self {
            db,
            catalog: kernel002_action_catalog(),
        }
    }

    pub fn with_catalog(db: Arc<dyn Database>, catalog: KernelActionCatalogV1) -> Self {
        Self { db, catalog }
    }

    pub fn catalog(&self) -> &KernelActionCatalogV1 {
        &self.catalog
    }

    fn submit_hygiene_candidate(&self, candidate: HygieneCandidate) -> Result<Uuid, HygieneError> {
        let receipt = RecordReceipt {
            record_id: Uuid::now_v7(),
            write_box_envelope_id: Uuid::now_v7(),
            persisted_at_utc: Utc::now(),
        };
        let submission = hygiene_submission(&candidate, &receipt)?;
        self.submit(submission)
            .map_err(|error| HygieneError::Rejected {
                code: error.code,
                reason: error.reason,
            })?;
        Ok(receipt.record_id)
    }
}

impl KernelActionSubmitter for SurrealKernelActionSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        let action = self
            .catalog
            .action(&submission.request.action_id)
            .ok_or_else(|| KernelActionRejection {
                code: "kernel_action_unknown".to_owned(),
                reason: format!(
                    "action_id {} is not registered in KernelActionCatalogV1 catalog {}",
                    submission.request.action_id, self.catalog.catalog_id
                ),
            })?;
        validate_submission_against_catalog(action, &submission)?;

        let target = primary_action_target(&submission)?;
        let aggregate_type = aggregate_type_for_target_kind(&target.target_kind)?;
        let event = build_catalog_action_event(&submission, action)?;
        let db = Arc::clone(&self.db);
        match block_on_storage(async move { db.append_kernel_event(event).await }) {
            Ok(_) => Ok(()),
            Err(error) if is_kernel_event_idempotency_conflict(&error) => {
                let db = Arc::clone(&self.db);
                let idempotency_key = submission.request.idempotency_key.clone();
                let aggregate_id = target.target_id.clone();
                let events = block_on_storage(async move {
                    db.list_kernel_events_for_aggregate(aggregate_type, &aggregate_id)
                        .await
                })
                .map_err(|lookup_error| KernelActionRejection {
                    code: "kernel_event_ledger_idempotency_lookup_failed".to_owned(),
                    reason: format!(
                        "checking duplicate memory action in EventLedger failed: {lookup_error}"
                    ),
                })?;
                if events.iter().any(|event| {
                    event.idempotency_key == idempotency_key
                        && same_submission_semantics(&event.payload, &submission)
                }) {
                    Ok(())
                } else {
                    Err(KernelActionRejection {
                        code: "kernel_event_ledger_append_failed".to_owned(),
                        reason: format!("appending memory action to EventLedger failed: {error}"),
                    })
                }
            }
            Err(error) => Err(KernelActionRejection {
                code: "kernel_event_ledger_append_failed".to_owned(),
                reason: format!("appending memory action to EventLedger failed: {error}"),
            }),
        }
    }
}

impl PinSubmitter for SurrealKernelActionSubmitter {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError> {
        let receipt = PinReceipt {
            receipt_id: Uuid::now_v7(),
            memory_id: item.memory_id,
            pinned: item.pinned,
            action_id: action_id_for_pin_state(item.pinned).to_owned(),
            fr_event_kind: fr_event_for_pin_state(item.pinned).to_owned(),
        };
        let submission = pin_submission(&item, &receipt)?;
        let action = self
            .catalog
            .action(&submission.request.action_id)
            .ok_or_else(|| PinError::Rejected {
                code: "kernel_action_unknown".to_owned(),
                reason: format!(
                    "action_id {} is not registered in KernelActionCatalogV1 catalog {}",
                    submission.request.action_id, self.catalog.catalog_id
                ),
            })?;
        validate_submission_against_catalog(action, &submission).map_err(pin_rejection)?;

        let action_event =
            build_catalog_action_event(&submission, action).map_err(pin_rejection)?;
        let manifest_event = build_pin_manifest_event(&item, &receipt, &submission)?;
        let db = Arc::clone(&self.db);
        match block_on_storage(async move {
            db.append_kernel_events_atomic(vec![action_event, manifest_event])
                .await
        }) {
            Ok(_) => Ok(receipt),
            Err(error) if is_kernel_event_idempotency_conflict(&error) => {
                if let Some(existing) = existing_pin_submission_matches(&self.db, &submission)? {
                    Ok(existing)
                } else {
                    Err(PinError::Rejected {
                        code: "memory_pin_atomic_append_failed".to_owned(),
                        reason: format!(
                            "atomic memory pin action/manifest append conflicted: {error}"
                        ),
                    })
                }
            }
            Err(error) => Err(PinError::Rejected {
                code: "memory_pin_atomic_append_failed".to_owned(),
                reason: format!("atomic memory pin action/manifest append failed: {error}"),
            }),
        }
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        let db = Arc::clone(&self.db);
        let mut events = block_on_storage(async move {
            db.list_kernel_events_for_aggregate(
                MEMORY_PIN_MANIFEST_AGGREGATE_TYPE,
                MEMORY_PIN_MANIFEST_AGGREGATE_ID,
            )
            .await
        })
        .map_err(|error| PinError::Rejected {
            code: "memory_pin_manifest_replay_failed".to_owned(),
            reason: format!("replaying the memory pin manifest failed: {error}"),
        })?;
        events.sort_by_key(|event| event.event_sequence);

        let mut latest_by_memory_id = BTreeMap::new();
        for event in events {
            let item = event
                .payload
                .get("pinned_item")
                .cloned()
                .ok_or_else(|| PinError::InvalidShape {
                    field: "memory_pin_manifest.pinned_item",
                    message: format!(
                        "manifest event {} does not contain pinned_item",
                        event.event_id
                    ),
                })
                .and_then(|value| {
                    serde_json::from_value::<PinnedItem>(value)
                        .map_err(|error| PinError::Serialization(error.to_string()))
                })?;
            latest_by_memory_id.insert(item.memory_id, item);
        }

        Ok(latest_by_memory_id
            .into_values()
            .filter(|item| item.pinned)
            .collect())
    }
}

fn pin_rejection(error: KernelActionRejection) -> PinError {
    PinError::Rejected {
        code: error.code,
        reason: error.reason,
    }
}

fn build_pin_manifest_event(
    item: &PinnedItem,
    receipt: &PinReceipt,
    submission: &KernelActionSubmission,
) -> Result<NewKernelEvent, PinError> {
    NewKernelEvent::builder(
        format!("KTR-MEMORY-PIN-MANIFEST-{}", receipt.receipt_id),
        item.session_id.clone(),
        KernelEventType::ArtifactStored,
        KernelActor::ModelAdapter(item.actor_id.clone()),
    )
    .aggregate(
        MEMORY_PIN_MANIFEST_AGGREGATE_TYPE,
        MEMORY_PIN_MANIFEST_AGGREGATE_ID,
    )
    .idempotency_key(format!(
        "memory_pin_manifest:{}",
        submission.request.idempotency_key
    ))
    .correlation_id(submission.request.trace_id.clone())
    .event_version("kernel_event_v1")
    .source_component(MEMORY_PIN_SOURCE_COMPONENT)
    .payload(json!({
        "schema_id": "hsk.memory_pin.manifest_event@1",
        "memory_id": item.memory_id,
        "pinned_item": item,
        "pin_receipt": receipt,
        "action_id": receipt.action_id,
        "flight_recorder_event_id": receipt.fr_event_kind,
        "action_idempotency_key": submission.request.idempotency_key,
    }))
    .build()
    .map_err(|error| PinError::InvalidShape {
        field: "memory_pin_manifest_event",
        message: error.to_string(),
    })
}

fn existing_pin_submission_matches(
    db: &Arc<dyn Database>,
    submission: &KernelActionSubmission,
) -> Result<Option<PinReceipt>, PinError> {
    let memory_id = submission
        .request
        .target_ids
        .iter()
        .find(|target| target.target_kind == "memory_item")
        .map(|target| target.target_id.clone())
        .ok_or_else(|| PinError::InvalidShape {
            field: "kernel_action_request.target_ids",
            message: "memory pin submission has no memory_item target".to_owned(),
        })?;
    let action_key = submission.request.idempotency_key.clone();
    let manifest_key = format!("memory_pin_manifest:{action_key}");
    let db = Arc::clone(db);
    let (action_events, manifest_events) = block_on_storage(async move {
        let action_events = db
            .list_kernel_events_for_aggregate(MEMORY_PIN_AGGREGATE_TYPE, &memory_id)
            .await?;
        let manifest_events = db
            .list_kernel_events_for_aggregate(
                MEMORY_PIN_MANIFEST_AGGREGATE_TYPE,
                MEMORY_PIN_MANIFEST_AGGREGATE_ID,
            )
            .await?;
        Ok::<_, StorageError>((action_events, manifest_events))
    })
    .map_err(|error| PinError::Rejected {
        code: "memory_pin_idempotency_lookup_failed".to_owned(),
        reason: format!("checking an existing memory pin submission failed: {error}"),
    })?;

    let action_matches = action_events.iter().any(|event| {
        event.idempotency_key == action_key && same_submission_semantics(&event.payload, submission)
    });
    if !action_matches {
        return Ok(None);
    }

    manifest_events
        .iter()
        .find(|event| event.idempotency_key == manifest_key)
        .map(|event| {
            event
                .payload
                .get("pin_receipt")
                .cloned()
                .ok_or_else(|| PinError::InvalidShape {
                    field: "memory_pin_manifest.pin_receipt",
                    message: format!(
                        "manifest event {} does not contain pin_receipt",
                        event.event_id
                    ),
                })
                .and_then(|value| {
                    serde_json::from_value::<PinReceipt>(value)
                        .map_err(|error| PinError::Serialization(error.to_string()))
                })
        })
        .transpose()
}

impl HygieneActionSubmitter for SurrealKernelActionSubmitter {
    fn submit_consolidation_candidate(
        &self,
        left: Uuid,
        right: Uuid,
    ) -> Result<Uuid, HygieneError> {
        self.submit_hygiene_candidate(HygieneCandidate::Consolidation { left, right })
    }

    fn submit_prune(
        &self,
        memory_id: Uuid,
        at: chrono::DateTime<Utc>,
    ) -> Result<Uuid, HygieneError> {
        self.submit_hygiene_candidate(HygieneCandidate::Prune {
            memory_id,
            requested_invalidated_at: at,
        })
    }

    fn submit_contradiction_flag(&self, left: Uuid, right: Uuid) -> Result<Uuid, HygieneError> {
        self.submit_hygiene_candidate(HygieneCandidate::ContradictionFlag { left, right })
    }

    fn submit_procedural_promotion(
        &self,
        candidate: ProceduralPromotion,
    ) -> Result<Uuid, HygieneError> {
        self.submit_hygiene_candidate(HygieneCandidate::ProceduralPromotion { candidate })
    }
}

fn is_kernel_event_idempotency_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Conflict(
            "kernel event idempotency key was reused with different event content"
        )
    ) || matches!(
        error,
        StorageError::Validation(message)
            if message.starts_with("kernel event idempotency conflict")
    )
}

fn same_submission_semantics(stored_payload: &Value, submission: &KernelActionSubmission) -> bool {
    stored_payload
        .get("catalog_action_id")
        .and_then(Value::as_str)
        == Some(submission.request.action_id.as_str())
        && stored_payload
            .get("request")
            .and_then(|request| request.get("idempotency_key"))
            .and_then(Value::as_str)
            == Some(submission.request.idempotency_key.as_str())
        && semantic_write_box_payload(
            stored_payload
                .get("write_box_envelope")
                .and_then(|envelope| envelope.get("payload")),
        ) == semantic_write_box_payload(Some(&submission.write_box_envelope.payload))
}

fn semantic_write_box_payload(payload: Option<&Value>) -> Option<Value> {
    let payload = payload?;
    match payload.get("schema_id").and_then(Value::as_str)? {
        "hsk.memory_capsule.record_payload@1" => Some(json!({
            "schema_id": "hsk.memory_capsule.record_payload@1",
            "record": payload.get("record")?,
        })),
        "hsk.memory_capsule.outcome_payload@1" => {
            let attribution = payload.get("attribution")?;
            Some(json!({
                "schema_id": "hsk.memory_capsule.outcome_payload@1",
                "capsule_id": attribution.get("capsule_id")?,
                "outcome": attribution.get("outcome")?,
            }))
        }
        PIN_MEMORY_PAYLOAD_SCHEMA_ID => Some(json!({
            "schema_id": PIN_MEMORY_PAYLOAD_SCHEMA_ID,
            "pinned_item": payload.get("pinned_item")?,
            "flight_recorder_event_id": payload.get("flight_recorder_event_id")?,
        })),
        HYGIENE_PAYLOAD_SCHEMA_ID => Some(json!({
            "schema_id": HYGIENE_PAYLOAD_SCHEMA_ID,
            "action_id": payload.get("action_id")?,
            "candidate": payload.get("candidate")?,
        })),
        _ => Some(payload.clone()),
    }
}

fn validate_submission_against_catalog(
    action: &KernelCatalogActionV1,
    submission: &KernelActionSubmission,
) -> Result<(), KernelActionRejection> {
    if action.authority_effect != submission.request.authority_effect {
        return Err(KernelActionRejection {
            code: "kernel_action_authority_effect_mismatch".to_owned(),
            reason: format!(
                "submission authority_effect {:?} does not match catalog action {} expected {:?}",
                submission.request.authority_effect, action.action_id, action.authority_effect
            ),
        });
    }
    if action.approval_posture != submission.request.approval_posture {
        return Err(KernelActionRejection {
            code: "kernel_action_approval_posture_mismatch".to_owned(),
            reason: format!(
                "submission approval_posture {:?} does not match catalog action {} expected {:?}",
                submission.request.approval_posture, action.action_id, action.approval_posture
            ),
        });
    }
    if !matches!(
        action.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    ) || !matches!(
        action.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    ) {
        return Err(KernelActionRejection {
            code: "kernel_action_unsupported_posture".to_owned(),
            reason: format!(
                "SurrealKernelActionSubmitter only persists PrePromotionEvidenceOnly + RequiresPromotionGate actions; got {} ({:?}/{:?})",
                action.action_id, action.authority_effect, action.approval_posture
            ),
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ActionTarget {
    target_id: String,
    target_kind: String,
}

fn primary_action_target(
    submission: &KernelActionSubmission,
) -> Result<ActionTarget, KernelActionRejection> {
    submission
        .request
        .target_ids
        .iter()
        .find(|target| {
            target.target_kind == "memory_capsule" || target.target_kind == "memory_item"
        })
        .map(|target| ActionTarget {
            target_id: target.target_id.clone(),
            target_kind: target.target_kind.clone(),
        })
        .ok_or_else(|| KernelActionRejection {
            code: "kernel_action_missing_supported_target".to_owned(),
            reason:
                "memory action submission must reference a memory_capsule or memory_item target_id"
                    .to_owned(),
        })
}

fn aggregate_type_for_target_kind(
    target_kind: &str,
) -> Result<&'static str, KernelActionRejection> {
    match target_kind {
        "memory_capsule" => Ok(MEMORY_CAPSULE_AGGREGATE_TYPE),
        "memory_item" => Ok(MEMORY_PIN_AGGREGATE_TYPE),
        _ => Err(KernelActionRejection {
            code: "kernel_action_unsupported_target_kind".to_owned(),
            reason: format!("unsupported memory action target_kind {target_kind}"),
        }),
    }
}

fn source_component_for_action(action_id: &str) -> &'static str {
    match action_id {
        PIN_MEMORY_ACTION_ID | UNPIN_MEMORY_ACTION_ID => MEMORY_PIN_SOURCE_COMPONENT,
        HYGIENE_CONSOLIDATION_ACTION_ID
        | HYGIENE_PRUNE_ACTION_ID
        | HYGIENE_FLAG_ACTION_ID
        | HYGIENE_PROMOTE_ACTION_ID => MEMORY_HYGIENE_SOURCE_COMPONENT,
        _ => MEMORY_CAPSULE_SOURCE_COMPONENT,
    }
}

fn build_catalog_action_event(
    submission: &KernelActionSubmission,
    action: &KernelCatalogActionV1,
) -> Result<NewKernelEvent, KernelActionRejection> {
    let target = primary_action_target(submission)?;
    NewKernelEvent::builder(
        format!("KTR-MEMORY-ACTION-{}", target.target_id),
        format!("SR-MEMORY-ACTION-{}", target.target_id),
        KernelEventType::ArtifactProposed,
        KernelActor::ModelAdapter(submission.request.actor.actor_id.clone()),
    )
    .aggregate(
        aggregate_type_for_target_kind(&target.target_kind)?,
        target.target_id,
    )
    .idempotency_key(submission.request.idempotency_key.clone())
    .correlation_id(submission.request.trace_id.clone())
    .event_version("kernel_event_v1")
    .source_component(source_component_for_action(action.action_id))
    .payload(json!({
        "schema_id": "hsk.memory_capsule.kernel_action_catalog_payload@1",
        "catalog_action_id": action.action_id,
        "catalog_input_schema_id": action.input_schema_id,
        "catalog_result_schema_id": action.result_schema_id,
        "request": submission.request,
        "write_box_envelope": submission.write_box_envelope,
        "proposed_receipt": submission.proposed_receipt,
    }))
    .build()
    .map_err(|error| KernelActionRejection {
        code: "kernel_action_event_build_failed".to_owned(),
        reason: format!("failed to build kernel event for memory action: {error}"),
    })
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
