//! WP-KERNEL-005 MT-170: operator-review pause invariant proven against
//! managed PostgreSQL.
//!
//! Integration validation v2 failed MT-170 because the pause invariant
//! (`promote`/apply must not run while the review ticket is Pending or
//! Rejected) was only ever asserted against a mock gate writing to a RefCell.
//! These tests run the SAME invariant over the production
//! [`PostgresPromotionGate`]: review tickets and operator decisions are
//! durable `kernel_event_ledger` rows (aggregate
//! [`PROMOTION_TICKET_AGGREGATE_TYPE`]), and every post-decision check
//! re-reads the persisted state through a FRESH gate instance — so the pause
//! provably lives in PostgreSQL, not in the submitting gate's memory.
//!
//! Invariant instrumented exactly like the loop core uses it
//! (`LoopPromotionGate::require_approved` before any authority write):
//!  - while the persisted ticket is Pending, `promote` is never reached
//!    (mutation counter stays 0; live policy value unchanged);
//!  - a persisted Rejection keeps the block across gate instances and
//!    preserves the rationale;
//!  - a persisted Approval unblocks exactly one promotion and the live
//!    authority value advances.
//!
//! EventLedger evidence (`PROMOTION_REQUESTED` / `PROMOTION_ACCEPTED` /
//! `PROMOTION_REJECTED`) is asserted directly on the `Database` ledger
//! reader. The gate bridges sync traits over async storage, so every test
//! runs on a multi-thread runtime. Gated on
//! `atelier_pg_support::database_url()`: when no PostgreSQL is available the
//! test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::kernel::KernelEventType;
use handshake_core::memory::TaskType;
use handshake_core::self_improve::editable_surface::{
    EditableSurfaceError, EditableSurfaceProvider, EditableSurfaceSnapshot, PolicyParameter,
    RetrievalPolicySurface, SurfaceProposal,
};
use handshake_core::self_improve::{
    EvalResult, GateError, LoopPromotionGate, LoopTarget, MetricDelta, OperatorId,
    PostgresPromotionGate, PromotionApproval, PromotionDecision, PromotionRejection,
    PromotionRequest, PromotionTicket, SentinelDecision, SplitMetrics,
    PROMOTION_TICKET_AGGREGATE_TYPE,
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

fn sample_request(
    iteration_id: Uuid,
    proposed_snapshot: EditableSurfaceSnapshot,
) -> PromotionRequest {
    PromotionRequest {
        iteration_id,
        target: LoopTarget::RetrievalPolicyParams {
            task_type: TaskType::ValidatorHbrTestPacket,
            parameter: PolicyParameter::TopK,
        },
        baseline_snapshot: EditableSurfaceSnapshot::RetrievalPolicy {
            task_type: TaskType::ValidatorHbrTestPacket,
            parameter: PolicyParameter::TopK,
            before_value: 6,
            after_value: 6,
        },
        proposed_snapshot,
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

/// Live retrieval-policy authority + mutation observer: `promote_count`
/// counts authority writes, `live_value` is the live top_k.
struct LiveAuthority {
    live_value: Arc<Mutex<u64>>,
    promote_count: Arc<AtomicUsize>,
}

impl LiveAuthority {
    fn new(initial: u64) -> Self {
        Self {
            live_value: Arc::new(Mutex::new(initial)),
            promote_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Production-shape RetrievalPolicySurface over the live authority cell.
    fn surface(
        &self,
    ) -> RetrievalPolicySurface<
        impl Fn(TaskType, PolicyParameter) -> Result<u64, EditableSurfaceError>,
        impl Fn(TaskType, PolicyParameter, u64) -> Result<(), EditableSurfaceError>,
    > {
        let read_cell = Arc::clone(&self.live_value);
        let write_cell = Arc::clone(&self.live_value);
        let counter = Arc::clone(&self.promote_count);
        RetrievalPolicySurface::new(
            move |_task, _param| Ok(*read_cell.lock().unwrap()),
            move |_task, _param, value| {
                counter.fetch_add(1, Ordering::SeqCst);
                *write_cell.lock().unwrap() = value;
                Ok(())
            },
        )
    }

    fn value(&self) -> u64 {
        *self.live_value.lock().unwrap()
    }

    fn promotions(&self) -> usize {
        self.promote_count.load(Ordering::SeqCst)
    }
}

/// The loop-core progression step under test: the persisted gate verdict is
/// re-read via `require_approved` BEFORE any authority write — exactly the
/// call order `LoopPromotionGate` documents for the self-improve loop.
fn try_progress(
    adapter: &LoopPromotionGate<'_>,
    ticket: &PromotionTicket,
    surface: &dyn EditableSurfaceProvider,
    snapshot: &EditableSurfaceSnapshot,
) -> Result<PromotionApproval, GateError> {
    let approval = adapter.require_approved(ticket)?;
    surface
        .promote(snapshot)
        .map_err(|error| GateError::Io {
            message: format!("promotion write failed: {error}"),
        })?;
    Ok(approval)
}

/// Build the proposed snapshot through the production surface (snapshot +
/// sandbox-scoped apply_proposal), proving apply_proposal itself never
/// mutates the live authority either.
fn proposed_snapshot(
    surface: &dyn EditableSurfaceProvider,
    new_value: u64,
) -> EditableSurfaceSnapshot {
    let target = LoopTarget::RetrievalPolicyParams {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameter::TopK,
    };
    let baseline = surface.snapshot(&target).expect("snapshot live authority");
    surface
        .apply_proposal(&baseline, SurfaceProposal::RetrievalPolicyValue { new_value })
        .expect("sandbox-scoped apply_proposal")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt170_pending_review_persisted_in_postgres_blocks_promotion() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt170_pending_review_persisted_in_postgres_blocks_promotion: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let authority = LiveAuthority::new(6);
    let surface = authority.surface();
    let proposal = proposed_snapshot(&surface, 8);
    assert_eq!(
        authority.promotions(),
        0,
        "apply_proposal is sandbox-scoped and must not write live authority"
    );

    let gate = PostgresPromotionGate::with_db(Arc::clone(&database));
    let adapter = LoopPromotionGate::new(&gate);
    let iteration_id = Uuid::now_v7();
    let ticket = adapter
        .submit(sample_request(iteration_id, proposal.clone()))
        .expect("submit promotion request to the durable gate");

    // The pause must hold no matter which gate instance polls: re-read the
    // persisted Pending state through FRESH gates, repeatedly.
    for attempt in 0..3 {
        let fresh_gate = PostgresPromotionGate::with_db(Arc::clone(&database));
        let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
        let err = try_progress(&fresh_adapter, &ticket, &surface, &proposal).unwrap_err();
        assert!(
            matches!(err, GateError::ReviewPending),
            "attempt {attempt}: persisted Pending review must block with typed \
             ReviewPending; got {err:?}"
        );
    }
    assert_eq!(
        authority.promotions(),
        0,
        "promote must never be reached while the persisted review is Pending"
    );
    assert_eq!(authority.value(), 6, "live top_k must stay at the baseline");

    // EventLedger proof: the pending ticket is a durable PROMOTION_REQUESTED
    // row carrying the reviewer evidence bundle.
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
    assert_eq!(requested.payload["status"], json!("pending"));
    assert_eq!(
        requested.correlation_id.as_deref(),
        Some(iteration_id.to_string().as_str())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt170_persisted_rejection_keeps_promotion_blocked_across_gate_instances() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt170_persisted_rejection_keeps_promotion_blocked_across_gate_instances: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let authority = LiveAuthority::new(6);
    let surface = authority.surface();
    let proposal = proposed_snapshot(&surface, 8);

    let gate = PostgresPromotionGate::with_db(Arc::clone(&database));
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(sample_request(Uuid::now_v7(), proposal.clone()))
        .expect("submit");

    let rationale = "latency regression unacceptable for MT-170";
    gate.record_rejection(
        &ticket,
        PromotionRejection {
            rejected_by: OperatorId::new("operator-strict"),
            rejected_at_utc: Utc::now(),
            rejection_reason: rationale.to_string(),
        },
    )
    .expect("record operator rejection");

    // FRESH gate: the rejection is re-read from PostgreSQL with the
    // rationale preserved, and progression stays blocked.
    let fresh_gate = PostgresPromotionGate::with_db(Arc::clone(&database));
    let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
    match try_progress(&fresh_adapter, &ticket, &surface, &proposal).unwrap_err() {
        GateError::ReviewRejected { rationale: r } => assert_eq!(r, rationale),
        other => panic!("expected ReviewRejected from PostgreSQL re-read; got {other:?}"),
    }
    assert_eq!(
        authority.promotions(),
        0,
        "promote must never be reached for a persisted Rejected ticket"
    );
    assert_eq!(authority.value(), 6);

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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt170_persisted_approval_unblocks_exactly_one_promotion() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt170_persisted_approval_unblocks_exactly_one_promotion: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let authority = LiveAuthority::new(6);
    let surface = authority.surface();
    let proposal = proposed_snapshot(&surface, 8);

    let gate = PostgresPromotionGate::with_db(Arc::clone(&database));
    let adapter = LoopPromotionGate::new(&gate);
    let ticket = adapter
        .submit(sample_request(Uuid::now_v7(), proposal.clone()))
        .expect("submit");

    // Before the operator decides, progression is blocked (same invariant).
    let err = try_progress(&adapter, &ticket, &surface, &proposal).unwrap_err();
    assert!(matches!(err, GateError::ReviewPending));
    assert_eq!(authority.promotions(), 0);

    let approval = PromotionApproval {
        approved_by: OperatorId::new("operator-prime"),
        approved_at_utc: Utc::now(),
        signoff_evidence_id: Uuid::now_v7(),
    };
    gate.record_approval(&ticket, approval.clone())
        .expect("record operator approval");

    // FRESH gate: the persisted approval unblocks promotion exactly once and
    // the live authority value advances.
    let fresh_gate = PostgresPromotionGate::with_db(Arc::clone(&database));
    let fresh_adapter = LoopPromotionGate::new(&fresh_gate);
    let reloaded = try_progress(&fresh_adapter, &ticket, &surface, &proposal)
        .expect("persisted approval must unblock promotion");
    assert_eq!(
        reloaded, approval,
        "approval audit metadata (who/when/signoff) must round-trip via PostgreSQL"
    );
    assert_eq!(
        authority.promotions(),
        1,
        "exactly one authority write after approval (no double-mutation)"
    );
    assert_eq!(authority.value(), 8, "live top_k must carry the approved value");

    // EventLedger proof: PROMOTION_ACCEPTED persisted alongside the request
    // (append-only audit history).
    let events = database
        .list_kernel_events_for_aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            &ticket.ticket_id.to_string(),
        )
        .await
        .expect("list promotion ticket ledger events");
    assert!(events
        .iter()
        .any(|event| event.event_type == KernelEventType::PromotionRequested));
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
