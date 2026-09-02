//! WP-KERNEL-005 MT-154 durable embedded-Surreal/EventLedger proof for the
//! production self-improve promotion gate.
//!
//! Every test closes the RocksDB-backed embedded store and reopens the same
//! path before asserting the gate state. No in-memory substitute can satisfy
//! these proofs.

use std::sync::Arc;

use chrono::Utc;
use handshake_core::kernel::KernelEventType;
use handshake_core::memory::TaskType;
use handshake_core::self_improve::{
    EditableSurfaceSnapshot, EvalResult, EventLedgerPromotionGate, GateError, LoopPromotionGate,
    LoopTarget, MetricDelta, OperatorId, PolicyParameter, PromotionApproval, PromotionDecision,
    PromotionGateSubmitter, PromotionRejection, PromotionRequest, PromotionStatus, PromotionTicket,
    SentinelDecision, SplitMetrics, PROMOTION_GATE_SOURCE_COMPONENT,
    PROMOTION_TICKET_AGGREGATE_TYPE, PROMOTION_TICKET_PAYLOAD_SCHEMA_ID,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::Database;
use serde_json::json;
use uuid::Uuid;

async fn open_store(config: SurrealStorageConfig) -> (SurrealStorage, Arc<dyn Database>) {
    let storage = SurrealStorage::open(config)
        .await
        .expect("open embedded Surreal promotion-gate store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap embedded Surreal promotion-gate schema");
    let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
    (storage, database)
}

async fn close_store(storage: SurrealStorage) {
    storage
        .shutdown()
        .await
        .expect("close embedded Surreal promotion-gate store");
}

fn split(pass_rate: f64) -> SplitMetrics {
    SplitMetrics {
        pass_rate,
        pass_count: (pass_rate * 100.0).round() as u32,
        total_count: 100,
        latency_p95_ms: 100,
        capsule_bytes_p95: 10_000,
        per_item_results: Vec::new(),
    }
}

fn snapshot(before: u64, after: u64) -> EditableSurfaceSnapshot {
    EditableSurfaceSnapshot::RetrievalPolicy {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameter::TopK,
        before_value: before,
        after_value: after,
    }
}

fn sample_request(iteration_id: Uuid) -> PromotionRequest {
    PromotionRequest {
        iteration_id,
        target: LoopTarget::RetrievalPolicyParams {
            task_type: TaskType::ValidatorHbrTestPacket,
            parameter: PolicyParameter::TopK,
        },
        baseline_snapshot: snapshot(6, 6),
        proposed_snapshot: snapshot(6, 8),
        eval_result: EvalResult {
            train: SplitMetrics::empty(),
            dev: split(0.70),
            holdout: split(0.60),
            evaluated_at_utc: Utc::now(),
            snapshot_hash: "0".repeat(64),
        },
        floor_decision: PromotionDecision::Approved {
            delta: MetricDelta {
                dev_pass_delta_pp: 0.10,
                latency_p95_delta_ms: 0,
                capsule_bytes_p95_delta_bytes: 0,
                holdout_pass_delta_pp: 0.0,
            },
        },
        sentinel_decision: SentinelDecision::Continue,
        justification_text: "raise top_k from 6 to 8 to improve recall".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_surreal_gate_submit_survives_close_reopen_as_pending_with_ledger_evidence() {
    let directory = tempfile::tempdir().expect("create promotion-gate test directory");
    let config = SurrealStorageConfig::for_data_dir(directory.path())
        .expect("configure embedded promotion-gate store");
    let (storage, database) = open_store(config.clone()).await;
    let iteration_id = Uuid::now_v7();
    let request = sample_request(iteration_id);

    let gate = EventLedgerPromotionGate::with_db(database.clone());
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(request.clone())
        .expect("submit through durable EventLedger promotion gate");
    assert_eq!(ticket.iteration_id, iteration_id);

    drop(adapter);
    drop(gate);
    drop(database);
    close_store(storage).await;

    let (reopened_storage, reopened_database) = open_store(config).await;
    let reopened_gate = EventLedgerPromotionGate::with_db(reopened_database.clone());
    let reopened_adapter = LoopPromotionGate::new(&reopened_gate);
    assert!(reopened_adapter
        .poll(&ticket)
        .expect("poll reopened ticket")
        .is_pending());
    assert!(matches!(
        reopened_adapter.require_approved(&ticket).unwrap_err(),
        GateError::ReviewPending
    ));

    let events = reopened_database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("read reopened promotion EventLedger aggregate");
    let requested = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionRequested)
        .expect("persisted PROMOTION_REQUESTED evidence");
    assert_eq!(
        requested.payload["schema_id"],
        json!(PROMOTION_TICKET_PAYLOAD_SCHEMA_ID)
    );
    assert_eq!(requested.payload["status"], json!("pending"));
    assert_eq!(
        requested.payload["request"]["justification_text"],
        json!(request.justification_text)
    );
    assert_eq!(requested.source_component, PROMOTION_GATE_SOURCE_COMPONENT);
    assert_eq!(
        requested.correlation_id.as_deref(),
        Some(iteration_id.to_string().as_str())
    );

    drop(reopened_adapter);
    drop(reopened_gate);
    drop(reopened_database);
    close_store(reopened_storage).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_surreal_approval_survives_close_reopen_and_rejects_double_decision() {
    let directory = tempfile::tempdir().expect("create promotion-gate test directory");
    let config = SurrealStorageConfig::for_data_dir(directory.path())
        .expect("configure embedded promotion-gate store");
    let (storage, database) = open_store(config.clone()).await;
    let gate = EventLedgerPromotionGate::with_db(database.clone());
    let ticket = LoopPromotionGate::new(&gate)
        .submit(sample_request(Uuid::now_v7()))
        .expect("submit promotion request");
    let approval = PromotionApproval {
        approved_by: OperatorId::new("operator-prime"),
        approved_at_utc: Utc::now(),
        signoff_evidence_id: Uuid::now_v7(),
    };
    gate.record_approval(&ticket, approval.clone())
        .expect("persist operator approval");

    drop(gate);
    drop(database);
    close_store(storage).await;

    let (reopened_storage, reopened_database) = open_store(config).await;
    let reopened_gate = EventLedgerPromotionGate::with_db(reopened_database.clone());
    let reopened_adapter = LoopPromotionGate::new(&reopened_gate);
    assert_eq!(
        reopened_adapter
            .require_approved(&ticket)
            .expect("reopen approved ticket"),
        approval
    );
    assert_eq!(
        reopened_adapter
            .poll(&ticket)
            .expect("poll approved ticket"),
        PromotionStatus::Approved {
            approval: approval.clone()
        }
    );

    let rejection = PromotionRejection {
        rejected_by: OperatorId::new("operator-strict"),
        rejected_at_utc: Utc::now(),
        rejection_reason: "stale double decision".to_owned(),
    };
    match reopened_gate
        .record_rejection(&ticket, rejection)
        .unwrap_err()
    {
        GateError::Io { message } => assert!(message.contains("already decided")),
        other => panic!("expected typed double-decision failure; got {other:?}"),
    }

    let events = reopened_database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("read approved promotion EventLedger aggregate");
    assert!(events
        .iter()
        .any(|event| event.event_type == KernelEventType::PromotionRequested));
    let accepted = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionAccepted)
        .expect("persisted PROMOTION_ACCEPTED evidence");
    assert_eq!(accepted.payload["status"], json!("approved"));
    assert_eq!(
        accepted.payload["approval"]["approved_by"],
        json!("operator-prime")
    );
    assert!(accepted.causation_id.is_some());

    drop(reopened_adapter);
    drop(reopened_gate);
    drop(reopened_database);
    close_store(reopened_storage).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_surreal_rejection_survives_close_reopen_and_forged_tickets_stay_unknown() {
    let directory = tempfile::tempdir().expect("create promotion-gate test directory");
    let config = SurrealStorageConfig::for_data_dir(directory.path())
        .expect("configure embedded promotion-gate store");
    let (storage, database) = open_store(config.clone()).await;
    let gate = EventLedgerPromotionGate::with_db(database.clone());
    let ticket = LoopPromotionGate::new(&gate)
        .submit(sample_request(Uuid::now_v7()))
        .expect("submit promotion request");
    let rationale = "latency regression unacceptable";
    let rejection = PromotionRejection {
        rejected_by: OperatorId::new("operator-strict"),
        rejected_at_utc: Utc::now(),
        rejection_reason: rationale.to_owned(),
    };
    gate.record_rejection(&ticket, rejection.clone())
        .expect("persist operator rejection");

    drop(gate);
    drop(database);
    close_store(storage).await;

    let (reopened_storage, reopened_database) = open_store(config).await;
    let reopened_gate = EventLedgerPromotionGate::with_db(reopened_database.clone());
    let reopened_adapter = LoopPromotionGate::new(&reopened_gate);
    assert_eq!(
        reopened_adapter
            .poll(&ticket)
            .expect("poll rejected ticket"),
        PromotionStatus::Rejected {
            rejection: rejection.clone()
        }
    );
    match reopened_adapter.require_approved(&ticket).unwrap_err() {
        GateError::ReviewRejected { rationale: stored } => assert_eq!(stored, rationale),
        other => panic!("expected reopened rejection; got {other:?}"),
    }

    let forged = PromotionTicket {
        ticket_id: Uuid::now_v7(),
        iteration_id: Uuid::now_v7(),
        submitted_at_utc: Utc::now(),
    };
    assert!(matches!(
        reopened_adapter.poll(&forged).unwrap_err(),
        GateError::UnknownTicket
    ));
    assert!(matches!(
        reopened_gate
            .record_approval(
                &forged,
                PromotionApproval {
                    approved_by: OperatorId::new("operator-prime"),
                    approved_at_utc: Utc::now(),
                    signoff_evidence_id: Uuid::now_v7(),
                },
            )
            .unwrap_err(),
        GateError::UnknownTicket
    ));

    let borrowed_id_forgery = PromotionTicket {
        ticket_id: ticket.ticket_id,
        iteration_id: Uuid::now_v7(),
        submitted_at_utc: ticket.submitted_at_utc,
    };
    assert!(matches!(
        reopened_adapter.poll(&borrowed_id_forgery).unwrap_err(),
        GateError::UnknownTicket
    ));

    let events = reopened_database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("read rejected promotion EventLedger aggregate");
    let rejected = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionRejected)
        .expect("persisted PROMOTION_REJECTED evidence");
    assert_eq!(rejected.payload["status"], json!("rejected"));
    assert_eq!(
        rejected.payload["rejection"]["rejection_reason"],
        json!(rationale)
    );

    drop(reopened_adapter);
    drop(reopened_gate);
    drop(reopened_database);
    close_store(reopened_storage).await;
}
