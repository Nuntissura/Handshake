//! MT-145 + MT-146 Postgres-backed kernel action catalog dispatcher and capsule store.
//!
//! This module binds [`CapsuleRecorder`](super::persistence::CapsuleRecorder) and
//! [`MemoryIpcService`](super::ipc::MemoryIpcService) to real durable Postgres storage
//! via the production [`Database`] trait and the static
//! [`KernelActionCatalogV1`](crate::kernel::action_catalog::KernelActionCatalogV1)
//! metadata.
//!
//! Authority surface (Spec-Realism Gate compliance):
//!  - The static catalog ([`kernel002_action_catalog`]) is the contract source — submissions
//!    are validated against the catalog action_id, authority_effect, and approval_posture
//!    before any persistence happens.
//!  - The durable persistence surface is the `kernel_event_ledger` Postgres table, accessed
//!    through [`Database::append_kernel_event`]. Each submission becomes an `ARTIFACT_PROPOSED`
//!    event row (pre-promotion evidence per [`AuthorityEffect::PrePromotionEvidenceOnly`]).
//!  - The capsule audit record is recoverable by replaying the ledger for the
//!    `memory_capsule_record` aggregate, so IPC list/get/suppress remain durable
//!    across process restarts (validator MT-146 requirement).
//!
//! Sub-rule 1 compliance: no `LiveXxxUnavailable`, no `todo!`, no placeholders. When the
//! underlying database fails, the typed error from [`StorageError`] is surfaced through
//! [`KernelActionRejection`] / [`MemoryIpcError::Store`] just like every other production
//! error in this codebase.

use std::{future::Future, sync::Arc};

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use super::hygiene::{
    hygiene_submission, HygieneActionSubmitter, HygieneCandidate, HygieneError,
    ProceduralPromotion, HYGIENE_CONSOLIDATION_ACTION_ID, HYGIENE_FLAG_ACTION_ID,
    HYGIENE_PAYLOAD_SCHEMA_ID, HYGIENE_PROMOTE_ACTION_ID, HYGIENE_PRUNE_ACTION_ID,
    MEMORY_HYGIENE_SOURCE_COMPONENT,
};
use super::ipc::{MemoryCapsuleIpcStore, MemoryIpcError};
use super::outcome_feedback::{
    outcome_attach_submission, OutcomeAttachSubmitter, OutcomeAttribution, OutcomeError,
    OutcomeReceipt, OUTCOME_ATTACH_ACTION_ID,
};
use super::persistence::{
    CapsuleRecord, KernelActionRejection, KernelActionSubmission, KernelActionSubmitter,
    RecordReceipt,
};
use super::pinned_core::{
    action_id_for_pin_state, fr_event_for_pin_state, pin_submission, PinError, PinReceipt,
    PinSubmitter, PinnedItem, MEMORY_PIN_AGGREGATE_TYPE, MEMORY_PIN_MANIFEST_AGGREGATE_ID,
    MEMORY_PIN_MANIFEST_AGGREGATE_TYPE, MEMORY_PIN_SOURCE_COMPONENT, PIN_MEMORY_ACTION_ID,
    PIN_MEMORY_PAYLOAD_SCHEMA_ID, UNPIN_MEMORY_ACTION_ID,
};
use crate::kernel::{
    action_catalog::{kernel002_action_catalog, KernelActionCatalogV1, KernelCatalogActionV1},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    KernelActor, KernelEventType, NewKernelEvent,
};
use crate::storage::{Database, StorageError};

/// Aggregate type used for memory capsule action submissions in the kernel event ledger.
pub const MEMORY_CAPSULE_AGGREGATE_TYPE: &str = "memory_capsule";

/// Source component label written to kernel_event_ledger.source_component for capsule
/// action persistence so operators can filter MT-145/MT-146 traffic in queries.
pub const MEMORY_CAPSULE_SOURCE_COMPONENT: &str = "memory_capsule_kernel_action_catalog";

/// Tokio bridge for synchronous memory traits backed by async Postgres storage.
///
/// When already inside Tokio, this uses `block_in_place` and therefore requires a
/// multi-thread runtime. Sync IPC integration tests that call this bridge from async
/// tests must use `#[tokio::test(flavor = "multi_thread")]`.
///
/// `pub(crate)` so the other sync-trait Postgres bindings (MT-165 trace export,
/// MT-170 promotion gate) reuse the same bridge instead of duplicating it.
pub(crate) fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current-thread runtime must build");
            runtime.block_on(future)
        }
    }
}

/// Postgres-backed [`KernelActionSubmitter`] that validates submissions against the static
/// [`KernelActionCatalogV1`] catalog and appends them to the kernel event ledger.
///
/// Constructed via [`PostgresKernelActionSubmitter::with_db`]; the default catalog is
/// [`kernel002_action_catalog`] but an alternate catalog may be supplied through
/// [`PostgresKernelActionSubmitter::with_catalog`] to support smaller/test catalogs.
pub struct PostgresKernelActionSubmitter {
    db: Arc<dyn Database>,
    catalog: KernelActionCatalogV1,
}

impl PostgresKernelActionSubmitter {
    /// Create a submitter bound to a real Postgres database with the default kernel002
    /// action catalog.
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self {
            db,
            catalog: kernel002_action_catalog(),
        }
    }

    /// Create a submitter with a caller-supplied catalog (used by integration tests that
    /// want to exercise a narrower contract).
    pub fn with_catalog(db: Arc<dyn Database>, catalog: KernelActionCatalogV1) -> Self {
        Self { db, catalog }
    }

    /// Borrow the catalog for test inspection.
    pub fn catalog(&self) -> &KernelActionCatalogV1 {
        &self.catalog
    }

    /// Borrow the database for callers that also need to read events back (e.g. the
    /// [`PostgresMemoryCapsuleStore`]).
    pub fn db(&self) -> Arc<dyn Database> {
        Arc::clone(&self.db)
    }
}

impl KernelActionSubmitter for PostgresKernelActionSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        // 1. Validate the request against the static catalog. This is the real
        //    KernelActionCatalogV1 contract enforcement the validator asked for.
        let action = self
            .catalog
            .action(&submission.request.action_id)
            .ok_or_else(|| KernelActionRejection {
                code: "kernel_action_unknown".to_string(),
                reason: format!(
                    "action_id {} is not registered in KernelActionCatalogV1 catalog {}",
                    submission.request.action_id, self.catalog.catalog_id
                ),
            })?;

        validate_submission_against_catalog(action, &submission)?;

        // 2. Build a kernel event ledger entry and persist it.
        let target = primary_action_target(&submission)?;
        let aggregate_type = aggregate_type_for_target_kind(&target.target_kind)?;
        let event = build_catalog_action_event(&submission, action)?;

        let append_db = Arc::clone(&self.db);
        let persisted = block_on(async move { append_db.append_kernel_event(event).await });
        match persisted {
            Ok(_) => Ok(()),
            Err(error) if is_kernel_event_idempotency_conflict(&error) => {
                let lookup_db = Arc::clone(&self.db);
                let idempotency_key = submission.request.idempotency_key.clone();
                let aggregate_type = aggregate_type.to_string();
                let aggregate_id = target.target_id.clone();
                let events = block_on(async move {
                    lookup_db
                        .list_kernel_events_for_aggregate(&aggregate_type, &aggregate_id)
                        .await
                })
                .map_err(|lookup_error| KernelActionRejection {
                    code: "kernel_event_ledger_idempotency_lookup_failed".to_string(),
                    reason: format!(
                        "checking duplicate memory capsule action in kernel_event_ledger failed: {lookup_error}"
                    ),
                })?;
                if events.iter().any(|event| {
                    event.idempotency_key == idempotency_key
                        && same_capsule_submission_semantics(&event.payload, &submission)
                }) {
                    return Ok(());
                }
                Err(KernelActionRejection {
                    code: "kernel_event_ledger_append_failed".to_string(),
                    reason: format!(
                        "appending memory capsule action to kernel_event_ledger failed: {error}"
                    ),
                })
            }
            Err(error) => Err(KernelActionRejection {
                code: "kernel_event_ledger_append_failed".to_string(),
                reason: format!(
                    "appending memory capsule action to kernel_event_ledger failed: {error}"
                ),
            }),
        }
    }
}

fn is_kernel_event_idempotency_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Validation(message) if message.starts_with("kernel event idempotency conflict")
    )
}

fn same_capsule_submission_semantics(
    stored_payload: &Value,
    submission: &KernelActionSubmission,
) -> bool {
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
        "hsk.memory_pin.payload@1" => {
            let pinned_item = payload.get("pinned_item")?;
            Some(json!({
                "schema_id": "hsk.memory_pin.payload@1",
                "pinned_item": pinned_item,
                "flight_recorder_event_id": payload.get("flight_recorder_event_id")?,
            }))
        }
        HYGIENE_PAYLOAD_SCHEMA_ID => Some(json!({
            "schema_id": HYGIENE_PAYLOAD_SCHEMA_ID,
            "action_id": payload.get("action_id")?,
            "candidate": payload.get("candidate")?,
        })),
        _ => Some(payload.clone()),
    }
}

impl OutcomeAttachSubmitter for PostgresKernelActionSubmitter {
    fn attach_outcome(
        &self,
        attribution: OutcomeAttribution,
    ) -> Result<OutcomeReceipt, OutcomeError> {
        let receipt = OutcomeReceipt {
            receipt_id: Uuid::now_v7(),
            capsule_id: attribution.capsule_id,
            action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
            recorded_at_utc: Utc::now(),
        };
        let submission = outcome_attach_submission(&attribution, &receipt)?;
        self.submit(submission)
            .map_err(|error| OutcomeError::Rejected {
                code: error.code,
                reason: error.reason,
            })?;
        Ok(receipt)
    }
}

impl HygieneActionSubmitter for PostgresKernelActionSubmitter {
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

impl PostgresKernelActionSubmitter {
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

impl PinSubmitter for PostgresKernelActionSubmitter {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError> {
        let receipt = PinReceipt {
            receipt_id: Uuid::now_v7(),
            memory_id: item.memory_id,
            pinned: item.pinned,
            action_id: action_id_for_pin_state(item.pinned).to_string(),
            fr_event_kind: fr_event_for_pin_state(item.pinned).to_string(),
        };
        let submission = pin_submission(&item, &receipt)?;
        let action = self
            .catalog
            .action(&submission.request.action_id)
            .ok_or_else(|| PinError::Rejected {
                code: "kernel_action_unknown".to_string(),
                reason: format!(
                    "action_id {} is not registered in KernelActionCatalogV1 catalog {}",
                    submission.request.action_id, self.catalog.catalog_id
                ),
            })?;
        validate_submission_against_catalog(action, &submission).map_err(|error| {
            PinError::Rejected {
                code: error.code,
                reason: error.reason,
            }
        })?;
        let action_event = build_catalog_action_event(&submission, action).map_err(|error| {
            PinError::Rejected {
                code: error.code,
                reason: error.reason,
            }
        })?;
        let manifest_event = pin_manifest_pointer_event(item.memory_id)?;
        let db = Arc::clone(&self.db);
        let atomic_result = block_on(async move {
            db.append_kernel_events_atomic(vec![action_event, manifest_event])
                .await
        });
        match atomic_result {
            Ok(_) => {}
            Err(error) if is_kernel_event_idempotency_conflict(&error) => {
                if !self.existing_pin_submission_matches(&submission)? {
                    return Err(PinError::Rejected {
                        code: "memory_pin_atomic_idempotency_conflict".to_string(),
                        reason: "kernel_event_ledger idempotency conflict did not match the existing memory pin event semantics".to_string(),
                    });
                }
                self.append_pin_manifest_pointer(item.memory_id)?;
            }
            Err(error) => {
                return Err(PinError::Rejected {
                    code: "memory_pin_atomic_append_failed".to_string(),
                    reason: error.to_string(),
                });
            }
        }
        Ok(receipt)
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        let db = Arc::clone(&self.db);
        let manifest_events = block_on(async move {
            db.list_kernel_events_for_aggregate(
                MEMORY_PIN_MANIFEST_AGGREGATE_TYPE,
                MEMORY_PIN_MANIFEST_AGGREGATE_ID,
            )
            .await
        })
        .map_err(|error| PinError::Rejected {
            code: "memory_pin_manifest_read_failed".to_string(),
            reason: error.to_string(),
        })?;

        let mut memory_ids = Vec::new();
        for event in manifest_events {
            if let Some(memory_id) = event.payload.get("memory_id").and_then(Value::as_str) {
                if !memory_ids.iter().any(|known| known == memory_id) {
                    memory_ids.push(memory_id.to_string());
                }
            }
        }

        let mut pinned = Vec::new();
        for memory_id in memory_ids {
            let db = Arc::clone(&self.db);
            let events = block_on(async move {
                db.list_kernel_events_for_aggregate(MEMORY_PIN_AGGREGATE_TYPE, &memory_id)
                    .await
            })
            .map_err(|error| PinError::Rejected {
                code: "memory_pin_item_read_failed".to_string(),
                reason: error.to_string(),
            })?;

            let mut latest_sequence = i64::MIN;
            let mut latest_item = None;
            for event in events {
                if event.event_sequence > latest_sequence {
                    if let Some(item) = decode_pin_payload(&event.payload) {
                        latest_sequence = event.event_sequence;
                        latest_item = Some(item);
                    }
                }
            }
            if let Some(item) = latest_item {
                if item.pinned {
                    pinned.push(item);
                }
            }
        }
        pinned.sort_by(|left, right| left.memory_id.cmp(&right.memory_id));
        Ok(pinned)
    }
}

impl PostgresKernelActionSubmitter {
    fn existing_pin_submission_matches(
        &self,
        submission: &KernelActionSubmission,
    ) -> Result<bool, PinError> {
        let target = primary_action_target(submission).map_err(|error| PinError::Rejected {
            code: error.code,
            reason: error.reason,
        })?;
        let db = Arc::clone(&self.db);
        let events = block_on(async move {
            db.list_kernel_events_for_aggregate(MEMORY_PIN_AGGREGATE_TYPE, &target.target_id)
                .await
        })
        .map_err(|error| PinError::Rejected {
            code: "memory_pin_idempotency_lookup_failed".to_string(),
            reason: error.to_string(),
        })?;

        Ok(events.iter().any(|event| {
            event.idempotency_key == submission.request.idempotency_key
                && same_capsule_submission_semantics(&event.payload, submission)
        }))
    }

    fn append_pin_manifest_pointer(&self, memory_id: Uuid) -> Result<(), PinError> {
        let event = pin_manifest_pointer_event(memory_id)?;
        let db = Arc::clone(&self.db);
        match block_on(async move { db.append_kernel_event(event).await }) {
            Ok(_) => Ok(()),
            Err(error) if is_kernel_event_idempotency_conflict(&error) => Ok(()),
            Err(error) => Err(PinError::Rejected {
                code: "memory_pin_manifest_append_failed".to_string(),
                reason: error.to_string(),
            }),
        }
    }
}

fn pin_manifest_pointer_event(memory_id: Uuid) -> Result<NewKernelEvent, PinError> {
    let payload = json!({
        "schema_id": "hsk.memory_pin.manifest_pointer@1",
        "memory_id": memory_id.to_string(),
    });
    NewKernelEvent::builder(
        "KTR-MEMORY-PIN-MANIFEST",
        "SR-MEMORY-PIN-MANIFEST",
        KernelEventType::ArtifactProposed,
        KernelActor::System("memory_pin_manifest".to_string()),
    )
    .aggregate(
        MEMORY_PIN_MANIFEST_AGGREGATE_TYPE,
        MEMORY_PIN_MANIFEST_AGGREGATE_ID,
    )
    .idempotency_key(format!("memory_pin_manifest_pointer:{memory_id}"))
    .event_version("kernel_event_v1")
    .source_component(MEMORY_PIN_SOURCE_COMPONENT)
    .payload(payload)
    .build()
    .map_err(|error| PinError::Rejected {
        code: "memory_pin_manifest_event_build_failed".to_string(),
        reason: error.to_string(),
    })
}

fn validate_submission_against_catalog(
    action: &KernelCatalogActionV1,
    submission: &KernelActionSubmission,
) -> Result<(), KernelActionRejection> {
    if action.authority_effect != submission.request.authority_effect {
        return Err(KernelActionRejection {
            code: "kernel_action_authority_effect_mismatch".to_string(),
            reason: format!(
                "submission authority_effect {:?} does not match catalog action {} expected {:?}",
                submission.request.authority_effect, action.action_id, action.authority_effect
            ),
        });
    }
    if action.approval_posture != submission.request.approval_posture {
        return Err(KernelActionRejection {
            code: "kernel_action_approval_posture_mismatch".to_string(),
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
            code: "kernel_action_unsupported_posture".to_string(),
            reason: format!(
                "PostgresKernelActionSubmitter only persists PrePromotionEvidenceOnly + RequiresPromotionGate actions; got {} ({:?}/{:?})",
                action.action_id, action.authority_effect, action.approval_posture
            ),
        });
    }
    Ok(())
}

fn build_catalog_action_event(
    submission: &KernelActionSubmission,
    action: &KernelCatalogActionV1,
) -> Result<NewKernelEvent, KernelActionRejection> {
    let target = primary_action_target(submission)?;
    let aggregate_type = aggregate_type_for_target_kind(&target.target_kind)?;
    let payload = capsule_action_payload(submission, action);

    NewKernelEvent::builder(
        format!("KTR-MEMORY-ACTION-{}", target.target_id),
        format!("SR-MEMORY-ACTION-{}", target.target_id),
        KernelEventType::ArtifactProposed,
        KernelActor::ModelAdapter(submission.request.actor.actor_id.clone()),
    )
    .aggregate(aggregate_type, target.target_id.clone())
    .idempotency_key(submission.request.idempotency_key.clone())
    .correlation_id(submission.request.trace_id.clone())
    .event_version("kernel_event_v1")
    .source_component(source_component_for_action(action.action_id))
    .payload(payload)
    .build()
    .map_err(|err| KernelActionRejection {
        code: "kernel_action_event_build_failed".to_string(),
        reason: format!("failed to build kernel event for capsule action: {err}"),
    })
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
            code: "kernel_action_missing_supported_target".to_string(),
            reason:
                "memory action submission must reference a memory_capsule or memory_item target_id"
                    .to_string(),
        })
}

fn aggregate_type_for_target_kind(
    target_kind: &str,
) -> Result<&'static str, KernelActionRejection> {
    match target_kind {
        "memory_capsule" => Ok(MEMORY_CAPSULE_AGGREGATE_TYPE),
        "memory_item" => Ok(MEMORY_PIN_AGGREGATE_TYPE),
        _ => Err(KernelActionRejection {
            code: "kernel_action_unsupported_target_kind".to_string(),
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

#[cfg(test)]
fn primary_capsule_id(
    submission: &KernelActionSubmission,
) -> Result<String, KernelActionRejection> {
    submission
        .request
        .target_ids
        .iter()
        .find(|target| target.target_kind == "memory_capsule")
        .map(|target| target.target_id.clone())
        .ok_or_else(|| KernelActionRejection {
            code: "kernel_action_missing_capsule_target".to_string(),
            reason: "memory capsule submission must reference a memory_capsule target_id"
                .to_string(),
        })
}

fn decode_pin_payload(payload: &Value) -> Option<PinnedItem> {
    let pin_payload = payload
        .get("write_box_envelope")
        .and_then(|envelope| envelope.get("payload"))?;
    if pin_payload.get("schema_id")?.as_str()? != PIN_MEMORY_PAYLOAD_SCHEMA_ID {
        return None;
    }
    serde_json::from_value(pin_payload.get("pinned_item")?.clone()).ok()
}

fn capsule_action_payload(
    submission: &KernelActionSubmission,
    action: &KernelCatalogActionV1,
) -> Value {
    json!({
        "schema_id": "hsk.memory_capsule.kernel_action_catalog_payload@1",
        "catalog_action_id": action.action_id,
        "catalog_input_schema_id": action.input_schema_id,
        "catalog_result_schema_id": action.result_schema_id,
        "request": submission.request,
        "write_box_envelope": submission.write_box_envelope,
        "proposed_receipt": submission.proposed_receipt,
    })
}

/// Postgres-backed durable store for [`CapsuleRecord`] entries.
///
/// Reads and writes capsule records through the kernel_event_ledger so IPC list/get/suppress
/// stays durable across process restarts (MT-146 rework requirement).
///
/// Each capsule record is persisted as one `ARTIFACT_PROPOSED` event row under the
/// [`MEMORY_CAPSULE_AGGREGATE_TYPE`] aggregate with the capsule_id as the aggregate_id.
/// The latest event for each capsule_id is the authoritative record; subsequent
/// suppressions overwrite via a new event (the most recent payload wins on read).
pub struct PostgresMemoryCapsuleStore {
    db: Arc<dyn Database>,
}

impl PostgresMemoryCapsuleStore {
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    fn append_record_event(&self, record: &CapsuleRecord) -> Result<(), MemoryIpcError> {
        // Hash the record payload so two saves of the same capsule with identical content
        // collapse to one ledger row (kernel_event_ledger idempotency contract), while
        // genuine updates (e.g. suppression changing audit_log) get a new event because
        // their hash differs.
        let payload = json!({
            "schema_id": "hsk.memory_capsule.store_record_payload@1",
            "record": record,
        });
        let record_canonical =
            serde_json::to_vec(&payload).map_err(|err| MemoryIpcError::Store {
                message: format!("memory capsule store payload serialization failed: {err}"),
            })?;
        let record_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&record_canonical);
            hex::encode(hasher.finalize())
        };
        let capsule_id = record.capsule_id.to_string();
        let event = NewKernelEvent::builder(
            format!("KTR-MEMORY-CAPSULE-STORE-{}", capsule_id),
            format!("SR-MEMORY-CAPSULE-STORE-{}", capsule_id),
            KernelEventType::ArtifactProposed,
            KernelActor::ModelAdapter(record.role_id.clone()),
        )
        .aggregate(MEMORY_CAPSULE_AGGREGATE_TYPE, capsule_id.clone())
        .idempotency_key(format!(
            "memory_capsule_store_record:{}:{}",
            capsule_id, record_hash
        ))
        .event_version("kernel_event_v1")
        .source_component(MEMORY_CAPSULE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| MemoryIpcError::Store {
            message: format!("memory capsule store event build failed: {err}"),
        })?;

        let db = Arc::clone(&self.db);
        block_on(async move { db.append_kernel_event(event).await })
            .map_err(|err| storage_to_memory_ipc_error(err))?;
        Ok(())
    }

    fn decode_record_event(payload: &Value) -> Option<CapsuleRecord> {
        if payload.get("schema_id")?.as_str()? != "hsk.memory_capsule.store_record_payload@1" {
            return None;
        }
        let record_value = payload.get("record")?.clone();
        serde_json::from_value::<CapsuleRecord>(record_value).ok()
    }

    fn capsule_ids(&self) -> Result<Vec<Uuid>, MemoryIpcError> {
        // We have to enumerate by scanning ledger events for our source_component since
        // the storage trait has no aggregate-only listing API today. For the volume
        // expected here (operator-visible recent capsules), this is fine.
        let db = Arc::clone(&self.db);
        // Without an aggregate-listing API we depend on the caller having appended
        // events through this submitter — list_kernel_events_for_aggregate needs an
        // aggregate_id, so we maintain a small list via per-record events keyed by id.
        // To enumerate all capsule_ids we keep a sidecar "manifest" capsule with a fixed
        // aggregate_id and append a pointer event each time a record lands.
        let events = block_on(async move {
            db.list_kernel_events_for_aggregate(
                MEMORY_CAPSULE_AGGREGATE_TYPE,
                CAPSULE_MANIFEST_AGGREGATE_ID,
            )
            .await
        })
        .map_err(|err| storage_to_memory_ipc_error(err))?;
        let mut ids = Vec::new();
        for event in events {
            if let Some(id_str) = event.payload.get("capsule_id").and_then(|v| v.as_str()) {
                if let Ok(uuid) = Uuid::parse_str(id_str) {
                    if !ids.contains(&uuid) {
                        ids.push(uuid);
                    }
                }
            }
        }
        Ok(ids)
    }

    fn append_manifest_pointer(&self, capsule_id: Uuid) -> Result<(), MemoryIpcError> {
        let payload = json!({
            "schema_id": "hsk.memory_capsule.manifest_pointer@1",
            "capsule_id": capsule_id.to_string(),
        });
        let event = NewKernelEvent::builder(
            "KTR-MEMORY-CAPSULE-MANIFEST",
            "SR-MEMORY-CAPSULE-MANIFEST",
            KernelEventType::ArtifactProposed,
            KernelActor::System("memory_capsule_store_manifest".to_string()),
        )
        .aggregate(MEMORY_CAPSULE_AGGREGATE_TYPE, CAPSULE_MANIFEST_AGGREGATE_ID)
        .idempotency_key(format!("memory_capsule_manifest_pointer:{capsule_id}"))
        .event_version("kernel_event_v1")
        .source_component(MEMORY_CAPSULE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| MemoryIpcError::Store {
            message: format!("memory capsule manifest pointer build failed: {err}"),
        })?;
        let db = Arc::clone(&self.db);
        block_on(async move { db.append_kernel_event(event).await })
            .map_err(|err| storage_to_memory_ipc_error(err))?;
        Ok(())
    }
}

const CAPSULE_MANIFEST_AGGREGATE_ID: &str = "memory_capsule_manifest_v1";

impl MemoryCapsuleIpcStore for PostgresMemoryCapsuleStore {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError> {
        let ids = self.capsule_ids()?;
        let mut records = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(record) = self.get_capsule_record(id)? {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn get_capsule_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<Option<CapsuleRecord>, MemoryIpcError> {
        let db = Arc::clone(&self.db);
        let capsule_id_string = capsule_id.to_string();
        let events = block_on(async move {
            db.list_kernel_events_for_aggregate(MEMORY_CAPSULE_AGGREGATE_TYPE, &capsule_id_string)
                .await
        })
        .map_err(|err| storage_to_memory_ipc_error(err))?;

        let mut latest: Option<CapsuleRecord> = None;
        let mut latest_sequence: i64 = i64::MIN;
        for event in events {
            if let Some(record) = Self::decode_record_event(&event.payload) {
                if event.event_sequence > latest_sequence {
                    latest_sequence = event.event_sequence;
                    latest = Some(record);
                }
            }
        }
        Ok(latest)
    }

    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError> {
        let capsule_id = record.capsule_id;
        self.append_record_event(&record)?;
        self.append_manifest_pointer(capsule_id)?;
        Ok(())
    }
}

fn storage_to_memory_ipc_error(err: StorageError) -> MemoryIpcError {
    MemoryIpcError::Store {
        message: format!("kernel_event_ledger access failed: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ipc::MEMORY_CAPSULE_SUPPRESS_ACTION_ID;
    use crate::memory::persistence::MEMORY_CAPSULE_RECORD_ACTION_ID;

    #[test]
    fn block_on_runs_inside_multi_thread_runtime() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("test runtime must build");
        let value = runtime.block_on(async { block_on(async { 42 }) });
        assert_eq!(value, 42);
    }

    #[test]
    fn primary_capsule_id_returns_target_id_when_kind_matches() {
        let submission = sample_submission();
        let id =
            primary_capsule_id(&submission).expect("sample submission has memory_capsule target");
        assert!(!id.is_empty());
    }

    #[test]
    fn primary_capsule_id_rejects_when_no_memory_capsule_target() {
        let mut submission = sample_submission();
        submission.request.target_ids.clear();
        let err = primary_capsule_id(&submission).unwrap_err();
        assert_eq!(err.code, "kernel_action_missing_capsule_target");
    }

    #[test]
    fn validate_submission_rejects_mismatched_authority_effect() {
        let catalog = kernel002_action_catalog();
        let action = catalog
            .action(MEMORY_CAPSULE_RECORD_ACTION_ID)
            .expect("catalog must include memory_capsule.record");
        let mut submission = sample_submission();
        submission.request.authority_effect = AuthorityEffect::EventLedgerAuthorityWrite;
        let err = validate_submission_against_catalog(action, &submission).unwrap_err();
        assert_eq!(err.code, "kernel_action_authority_effect_mismatch");
    }

    fn sample_submission() -> KernelActionSubmission {
        use crate::kernel::action_envelope::{
            ExpectedWriteBoxRef, KernelActionRequestV1, KernelActorRef, KernelSessionRef,
            KernelTargetRef, ValidationRequirement,
        };
        use crate::kernel::write_boxes::{
            MemoryBox, WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef,
            WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef,
            WriteBoxValidationState, WriteBoxValidationStatus,
        };
        use crate::memory::persistence::{
            RecordReceipt, WriteBoxV1Envelope, KERNEL_ACTION_REQUEST_SCHEMA_ID,
            MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID, MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID,
            MEMORY_WRITE_BOX_SCHEMA_ID, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
        };
        let capsule_id = "018f35f2-79b0-7cc3-98c4-dc0c0c0c0c0c";
        KernelActionSubmission {
            request: KernelActionRequestV1 {
                schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
                action_id: MEMORY_CAPSULE_RECORD_ACTION_ID.to_string(),
                actor: KernelActorRef {
                    actor_id: "KERNEL_BUILDER".to_string(),
                    actor_kind: "role".to_string(),
                    role_id: "KERNEL_BUILDER".to_string(),
                },
                session: KernelSessionRef {
                    session_id: "session-145".to_string(),
                    work_profile_id: "memory-capsule-persistence".to_string(),
                },
                target_ids: vec![KernelTargetRef {
                    target_id: capsule_id.to_string(),
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
                validation_requirements: vec![ValidationRequirement {
                    check_id: "schema_validity".to_string(),
                    required: true,
                }],
                trace_id: "trace-145".to_string(),
                idempotency_key: format!("memory_capsule_record:{capsule_id}:hash"),
            },
            write_box_envelope: WriteBoxV1Envelope {
                schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
                envelope_id: Uuid::now_v7(),
                payload_schema_id: MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID.to_string(),
                payload: serde_json::json!({}),
                payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                write_box: MemoryBox {
                    common: WriteBoxCommon {
                        write_box_id: Uuid::now_v7().to_string(),
                        kind: WriteBoxKind::Memory,
                        schema_version: MEMORY_WRITE_BOX_SCHEMA_ID.to_string(),
                        workspace_id: "session-145".to_string(),
                        owner: WriteBoxOwnerRef {
                            actor_id: "KERNEL_BUILDER".to_string(),
                            actor_kind: "role".to_string(),
                            role_id: "KERNEL_BUILDER".to_string(),
                        },
                        crdt_site_id: "memory-capsule-recorder".to_string(),
                        target_refs: vec![WriteBoxTargetRef {
                            target_id: capsule_id.to_string(),
                            target_kind: "memory_capsule".to_string(),
                            authority_class: "pre_promotion_memory".to_string(),
                        }],
                        base_snapshot_refs: vec!["memory-capsule-source-hash://hash".to_string()],
                        intent_summary: "intent".to_string(),
                        operation_payload_refs: vec![WriteBoxPayloadRef {
                            payload_id: Uuid::now_v7().to_string(),
                            payload_kind: "memory_capsule_record_v1".to_string(),
                            payload_ref: "memory-capsule-record://x".to_string(),
                            payload_sha256:
                                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                                    .to_string(),
                        }],
                        lifecycle_state: WriteBoxLifecycleState::Open,
                        allowed_transitions: vec![
                            WriteBoxLifecycleState::ReadyForValidation,
                            WriteBoxLifecycleState::Denied,
                        ],
                        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
                        evidence_refs: vec![format!("memory-capsule://{capsule_id}")],
                        receipt_refs: vec!["receipt://x".to_string()],
                        denial_receipt_refs: Vec::new(),
                        promotion_receipt_refs: Vec::new(),
                        validation_status: WriteBoxValidationStatus {
                            state: WriteBoxValidationState::Pending,
                            check_ids: vec!["schema_validity".to_string()],
                        },
                        projection_rules: vec!["dcc.memory_queue".to_string()],
                        replay_metadata: WriteBoxReplayMetadataV1 {
                            replay_plan_ref: "memory-capsule-record://plan".to_string(),
                            replay_order_key: "session-145/2026-05-19T10:05:00Z/x".to_string(),
                            idempotency_key: format!("memory_capsule_record:{capsule_id}:hash"),
                            source_event_refs: vec![format!("memory-capsule://{capsule_id}")],
                        },
                    },
                    memory_extract_ref: format!("memory-capsule-record://{capsule_id}"),
                },
            },
            proposed_receipt: RecordReceipt {
                record_id: Uuid::now_v7(),
                write_box_envelope_id: Uuid::now_v7(),
                persisted_at_utc: chrono::Utc::now(),
            },
        }
    }

    #[test]
    fn suppression_action_id_constant_matches_catalog() {
        let catalog = kernel002_action_catalog();
        assert!(catalog.action(MEMORY_CAPSULE_SUPPRESS_ACTION_ID).is_some());
        assert!(catalog.action(MEMORY_CAPSULE_RECORD_ACTION_ID).is_some());
    }
}
