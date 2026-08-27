//! MT-136 proof for the synchronous memory kernel-action adapter.
//!
//! The proof captures a production-valid capsule submission, persists it to a
//! real embedded RocksDB-backed Surreal store, attacks replay and catalog/gate
//! boundaries, exercises the adapter from a current-thread Tokio runtime, and
//! reopens the store before the final EventLedger assertions.

use std::{collections::BTreeMap, sync::Mutex};

use chrono::{TimeZone, Utc};
use serde_json::json;
use uuid::Uuid;

use super::mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend};
use crate::{
    kernel::{action_catalog::kernel002_action_catalog, action_envelope::ApprovalPosture},
    memory::{
        CapsuleAuditEntry, CapsuleAuditLog, CapsuleRecord, CapsuleRecorder, DegradationTier,
        KernelActionRejection, KernelActionSubmission, KernelActionSubmitter, RetrievalPolicy,
        SurrealKernelActionSubmitter, TaskType,
    },
    storage::{Database, StorageError, StorageResult},
};

#[derive(Default)]
struct CapturingSubmitter {
    submission: Mutex<Option<KernelActionSubmission>>,
}

impl KernelActionSubmitter for CapturingSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        *self.submission.lock().expect("capture lock") = Some(submission);
        Ok(())
    }
}

fn proof_record() -> CapsuleRecord {
    let task_type = TaskType::KernelBuilderMtImplementation;
    CapsuleRecord {
        capsule_id: Uuid::now_v7(),
        capsule_source_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        task_type,
        policy: RetrievalPolicy {
            top_k: 12,
            capsule_budget_bytes: 65_536,
            task_type,
            scoring_formula_version: "retrieval_scoring_formula_v0".to_owned(),
            graceful_degradation_tier: DegradationTier::Tiered,
        },
        audit_log: CapsuleAuditLog {
            entries: vec![CapsuleAuditEntry {
                item_id: "mt136-submit-item".to_owned(),
                source_uri: "fems://source/artifact/mt136-submit-proof#item".to_owned(),
                included: true,
                suppression_reason: None,
                score: 0.92,
                score_breakdown: BTreeMap::from([("similarity".to_owned(), 0.92)]),
                pinned: false,
            }],
        },
        built_at_utc: Utc
            .with_ymd_and_hms(2026, 8, 22, 10, 0, 0)
            .single()
            .expect("valid proof time"),
        recorded_at_utc: Utc
            .with_ymd_and_hms(2026, 8, 22, 10, 5, 0)
            .single()
            .expect("valid proof time"),
        session_id: "mt136-submit-session".to_owned(),
        role_id: "KERNEL_BUILDER".to_owned(),
        outcome: None,
    }
}

fn production_submission() -> KernelActionSubmission {
    let capture = CapturingSubmitter::default();
    CapsuleRecorder {
        action_catalog: &capture,
    }
    .record(proof_record())
    .expect("production capsule recorder must build a valid submission");
    let submission = capture
        .submission
        .lock()
        .expect("capture lock")
        .take()
        .expect("captured production submission");
    submission
}

fn rejection(error: KernelActionRejection) -> StorageError {
    StorageError::Database(format!("unexpected kernel-action rejection: {error}"))
}

async fn reopen(backend: EmbeddedProofBackend) -> StorageResult<EmbeddedProofBackend> {
    backend.reopen().await
}

async fn submitter_contract_survives_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let submission = production_submission();
    let aggregate_id = submission.request.target_ids[0].target_id.clone();
    let first_key = submission.request.idempotency_key.clone();
    let submitter = SurrealKernelActionSubmitter::with_db(database.clone());

    submitter.submit(submission.clone()).map_err(rejection)?;
    let stored = database
        .list_kernel_events_for_aggregate("memory_capsule", &aggregate_id)
        .await?;
    assert_eq!(stored.len(), 1, "first submit must append one event");
    assert_eq!(stored[0].idempotency_key, first_key);
    assert_eq!(
        stored[0]
            .payload
            .get("catalog_action_id")
            .and_then(serde_json::Value::as_str),
        Some("kernel.memory_capsule.record")
    );

    let mut replay = submission.clone();
    let replay_record_id = Uuid::now_v7();
    replay.proposed_receipt.record_id = replay_record_id;
    replay.proposed_receipt.write_box_envelope_id = Uuid::now_v7();
    replay.proposed_receipt.persisted_at_utc = Utc::now();
    replay.write_box_envelope.envelope_id = replay.proposed_receipt.write_box_envelope_id;
    replay.write_box_envelope.payload["record_id"] = json!(replay_record_id);
    submitter.submit(replay.clone()).map_err(rejection)?;
    assert_eq!(
        database
            .list_kernel_events_for_aggregate("memory_capsule", &aggregate_id)
            .await?
            .len(),
        1,
        "semantic replay must return success without appending a duplicate"
    );

    let mut conflicting = replay;
    conflicting.write_box_envelope.payload["record"]["outcome"] =
        json!({"status": "failed", "reason": "changed semantic payload"});
    let conflict = submitter
        .submit(conflicting)
        .expect_err("same idempotency key with changed record semantics must fail");
    assert_eq!(conflict.code, "kernel_event_ledger_append_failed");

    let mut unknown = submission.clone();
    unknown.request.action_id = "kernel.unknown.mt136".to_owned();
    let unknown_error = submitter
        .submit(unknown)
        .expect_err("unknown catalog action must fail before EventLedger append");
    assert_eq!(unknown_error.code, "kernel_action_unknown");

    let mut non_gated_catalog = kernel002_action_catalog();
    non_gated_catalog
        .actions
        .iter_mut()
        .find(|action| action.action_id == submission.request.action_id)
        .expect("record action in catalog")
        .approval_posture = ApprovalPosture::NoApprovalRequired;
    let non_gated_submitter =
        SurrealKernelActionSubmitter::with_catalog(database.clone(), non_gated_catalog);
    let mut non_gated = submission.clone();
    non_gated.request.approval_posture = ApprovalPosture::NoApprovalRequired;
    let gate_error = non_gated_submitter
        .submit(non_gated)
        .expect_err("non-promotion-gated action must fail closed");
    assert_eq!(gate_error.code, "kernel_action_unsupported_posture");

    let mut current_thread_submission = submission;
    current_thread_submission.request.idempotency_key =
        format!("mt136-current-thread:{}", Uuid::now_v7());
    current_thread_submission.request.trace_id =
        format!("mt136-current-thread-trace:{}", Uuid::now_v7());
    current_thread_submission
        .write_box_envelope
        .write_box
        .common
        .replay_metadata
        .idempotency_key = current_thread_submission.request.idempotency_key.clone();
    let current_thread_db = database.clone();
    tokio::task::spawn_blocking(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current-thread proof runtime")
            .block_on(async move {
                SurrealKernelActionSubmitter::with_db(current_thread_db)
                    .submit(current_thread_submission)
            })
    })
    .await
    .map_err(|error| StorageError::Database(format!("current-thread proof join failed: {error}")))?
    .map_err(rejection)?;

    assert_eq!(
        database
            .list_kernel_events_for_aggregate("memory_capsule", &aggregate_id)
            .await?
            .len(),
        2,
        "current-thread submission must append exactly one additional event"
    );

    let reopened = reopen(backend).await?;
    let durable = reopened
        .database
        .list_kernel_events_for_aggregate("memory_capsule", &aggregate_id)
        .await?;
    assert_eq!(durable.len(), 2, "both accepted events must survive reopen");
    assert!(durable
        .iter()
        .any(|event| event.idempotency_key == first_key));
    reopened.close_and_remove().await?;
    Ok(())
}

pub(super) async fn run_all() -> StorageResult<()> {
    submitter_contract_survives_reopen().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn submitter_contract() -> StorageResult<()> {
        run_all().await
    }
}
