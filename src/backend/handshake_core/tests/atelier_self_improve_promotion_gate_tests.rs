//! WP-KERNEL-005 MT-154 live PostgreSQL proof for the production
//! [`PromotionGateSubmitter`]: [`PostgresPromotionGate`].
//!
//! Integration validation v2 failed MT-154 because every prior proof ran the
//! [`LoopPromotionGate`] adapter against in-memory mocks
//! (MockGate/MisbindingGate/StubGate) and the KERNEL-001 PromotionGate
//! boundary never touched PostgreSQL. This file exercises the real gate:
//! tickets and operator decisions are appended to `kernel_event_ledger` rows
//! (aggregate `self_improve_promotion_ticket`), then RE-READ from PostgreSQL
//! through FRESH gate instances so no in-memory state can satisfy the
//! assertions. EventLedger evidence (`PROMOTION_REQUESTED` /
//! `PROMOTION_ACCEPTED` / `PROMOTION_REJECTED`) is asserted directly on the
//! `Database` ledger reader.
//!
//! The gate trait is sync over async storage (shared Tokio bridge), so every
//! test runs on a multi-thread runtime.

mod atelier_pg_support;

use std::sync::Arc;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::kernel::KernelEventType;
use handshake_core::memory::TaskType;
use handshake_core::self_improve::{
    EditableSurfaceSnapshot, EvalResult, GateError, LoopPromotionGate, LoopTarget, MetricDelta,
    OperatorId, PolicyParameter, PostgresPromotionGate, PromotionApproval, PromotionDecision,
    PromotionGateSubmitter, PromotionRejection, PromotionRequest, PromotionStatus,
    PromotionTicket, SentinelDecision, SplitMetrics, PROMOTION_GATE_SOURCE_COMPONENT,
    PROMOTION_TICKET_AGGREGATE_TYPE, PROMOTION_TICKET_PAYLOAD_SCHEMA_ID,
};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn connected_database(url: &str) -> Arc<dyn Database> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool);
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    database.into_arc()
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
        justification_text: "raise top_k from 6 to 8 to improve recall".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_postgres_gate_submit_persists_pending_ticket_and_ledger_event() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt154_postgres_gate_submit_persists_pending_ticket_and_ledger_event: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let iteration_id = Uuid::now_v7();
    let request = sample_request(iteration_id);

    // Submit through the loop adapter wired to the REAL gate (no mock).
    let gate = PostgresPromotionGate::with_db(database.clone());
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(request.clone())
        .expect("submit through PostgresPromotionGate");
    assert_eq!(
        ticket.iteration_id, iteration_id,
        "production gate must bind the ticket to the request's iteration_id"
    );

    // RE-READ through a FRESH gate instance: the review state lives in
    // PostgreSQL, not in the submitting gate's memory.
    let fresh_gate = PostgresPromotionGate::with_db(database.clone());
    let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
    let status = fresh_adapter.poll(&ticket).expect("poll persisted ticket");
    assert!(
        status.is_pending(),
        "fresh gate instance must re-read Pending from PostgreSQL; got {status:?}"
    );
    let err = fresh_adapter.require_approved(&ticket).unwrap_err();
    assert!(
        matches!(err, GateError::ReviewPending),
        "Pending review must block apply with typed ReviewPending; got {err:?}"
    );

    // EventLedger proof: the submit appended a PROMOTION_REQUESTED event on
    // the ticket aggregate with the full evidence bundle.
    let events = database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("list promotion ticket ledger events");
    let requested = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionRequested)
        .expect("PROMOTION_REQUESTED event must persist in kernel_event_ledger");
    assert_eq!(
        requested.payload["schema_id"],
        json!(PROMOTION_TICKET_PAYLOAD_SCHEMA_ID)
    );
    assert_eq!(requested.payload["status"], json!("pending"));
    assert_eq!(
        requested.payload["request"]["justification_text"],
        json!(request.justification_text),
        "the reviewer-facing evidence bundle must round-trip through the ledger"
    );
    assert_eq!(requested.source_component, PROMOTION_GATE_SOURCE_COMPONENT);
    assert_eq!(
        requested.correlation_id.as_deref(),
        Some(iteration_id.to_string().as_str()),
        "ledger correlation must link the ticket back to the loop iteration"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_operator_approval_persists_across_gate_instances_and_blocks_double_decision() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt154_operator_approval_persists_across_gate_instances_and_blocks_double_decision: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let gate = PostgresPromotionGate::with_db(database.clone());
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(sample_request(Uuid::now_v7()))
        .expect("submit");

    let approval = PromotionApproval {
        approved_by: OperatorId::new("operator-prime"),
        approved_at_utc: Utc::now(),
        signoff_evidence_id: Uuid::now_v7(),
    };
    gate.record_approval(&ticket, approval.clone())
        .expect("record operator approval");

    // FRESH gate: approval must be re-read from PostgreSQL, byte-equal.
    let fresh_gate = PostgresPromotionGate::with_db(database.clone());
    let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
    let reloaded = fresh_adapter
        .require_approved(&ticket)
        .expect("approved ticket unblocks apply");
    assert_eq!(
        reloaded, approval,
        "approval audit metadata (who/when/signoff) must round-trip via PostgreSQL"
    );
    match fresh_adapter.poll(&ticket).expect("poll approved ticket") {
        PromotionStatus::Approved { approval: polled } => assert_eq!(polled, approval),
        other => panic!("expected persisted Approved; got {other:?}"),
    }

    // Double-decision defense: a later rejection on the decided ticket fails
    // typed instead of silently overwriting the persisted approval.
    let rejection = PromotionRejection {
        rejected_by: OperatorId::new("operator-strict"),
        rejected_at_utc: Utc::now(),
        rejection_reason: "stale double decision".to_string(),
    };
    let err = fresh_gate.record_rejection(&ticket, rejection).unwrap_err();
    match err {
        GateError::Io { message } => assert!(
            message.contains("already decided"),
            "double decision must be rejected loudly; got: {message}"
        ),
        other => panic!("expected GateError::Io for double decision; got {other:?}"),
    }

    // EventLedger proof: PROMOTION_ACCEPTED persisted alongside the request.
    let events = database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("list promotion ticket ledger events");
    assert!(
        events
            .iter()
            .any(|event| event.event_type == KernelEventType::PromotionRequested),
        "submit event must remain in the ledger (append-only audit history)"
    );
    let accepted = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionAccepted)
        .expect("PROMOTION_ACCEPTED event must persist in kernel_event_ledger");
    assert_eq!(accepted.payload["status"], json!("approved"));
    assert_eq!(
        accepted.payload["approval"]["approved_by"],
        json!("operator-prime")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_rejection_persists_rationale_and_forged_tickets_stay_unknown() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt154_rejection_persists_rationale_and_forged_tickets_stay_unknown: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let gate = PostgresPromotionGate::with_db(database.clone());
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(sample_request(Uuid::now_v7()))
        .expect("submit");

    let rationale = "latency regression unacceptable";
    gate.record_rejection(
        &ticket,
        PromotionRejection {
            rejected_by: OperatorId::new("operator-strict"),
            rejected_at_utc: Utc::now(),
            rejection_reason: rationale.to_string(),
        },
    )
    .expect("record operator rejection");

    // FRESH gate re-reads the rejection with the rationale preserved.
    let fresh_gate = PostgresPromotionGate::with_db(database.clone());
    let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
    match fresh_adapter.require_approved(&ticket).unwrap_err() {
        GateError::ReviewRejected { rationale: r } => assert_eq!(r, rationale),
        other => panic!("expected ReviewRejected from PostgreSQL re-read; got {other:?}"),
    }

    // EventLedger proof: PROMOTION_REJECTED with the rationale.
    let events = database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("list promotion ticket ledger events");
    let rejected = events
        .iter()
        .find(|event| event.event_type == KernelEventType::PromotionRejected)
        .expect("PROMOTION_REJECTED event must persist in kernel_event_ledger");
    assert_eq!(rejected.payload["status"], json!("rejected"));
    assert_eq!(
        rejected.payload["rejection"]["rejection_reason"],
        json!(rationale)
    );

    // Ticket-forging defense holds against the durable store: a ticket the
    // ledger has never seen yields typed UnknownTicket, and deciding it
    // fails the same way (no phantom decision rows).
    let forged = PromotionTicket {
        ticket_id: Uuid::now_v7(),
        iteration_id: Uuid::now_v7(),
        submitted_at_utc: Utc::now(),
    };
    let err = fresh_adapter.poll(&forged).unwrap_err();
    assert!(matches!(err, GateError::UnknownTicket));
    let err = fresh_gate
        .record_approval(
            &forged,
            PromotionApproval {
                approved_by: OperatorId::new("operator-prime"),
                approved_at_utc: Utc::now(),
                signoff_evidence_id: Uuid::now_v7(),
            },
        )
        .unwrap_err();
    assert!(
        matches!(err, GateError::UnknownTicket),
        "deciding a forged ticket must fail typed; got {err:?}"
    );
}
