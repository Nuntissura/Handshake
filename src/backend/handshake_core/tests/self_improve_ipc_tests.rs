//! MT-155: integration tests for the self-improvement loop Tauri IPC
//! surface.
//!
//! Contract covers the six commands exposed in
//! [`handshake_core::self_improve::ipc`] plus their cross-thread sticky
//! semantics:
//!
//! - `status()` returns the current loop iteration state without
//!   mutating it.
//! - `pause(rationale)` sets the sticky pause flag and stamps an
//!   operator rationale onto the receipt.
//! - `unpause(rationale)` clears the flag. Calling `unpause` when the
//!   loop is not paused returns `LoopIpcError::NotPaused`. Calling it
//!   with an empty rationale returns `LoopIpcError::EmptyRationale`
//!   and leaves the pause sticky.
//! - `list_pending_reviews(limit)` returns the
//!   [`PromotionRequest`]s awaiting an MT-154 [`PromotionGate`]
//!   decision.
//! - `approve_promotion` + `reject_promotion` round-trip through the
//!   gate trait.
//!
//! Sticky cross-thread semantics: every mutation goes through
//! `LoopIpcState`'s `RwLock`, so a pause from one thread is visible to
//! `status()` reads from any other thread. The test
//! `pause_visible_across_threads` exercises this with `std::thread`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use handshake_core::memory::TaskType;
use handshake_core::self_improve::editable_surface::{EditableSurfaceSnapshot, PolicyParameter};
use handshake_core::self_improve::evaluator::{EvalResult, SplitMetrics};
use handshake_core::self_improve::goodhart_sentinel::{PauseReason, SentinelDecision};
use handshake_core::self_improve::ipc::{
    LoopIpcError, LoopIpcState, LoopState, FR_EVT_LOOP_PAUSE, FR_EVT_LOOP_UNPAUSE,
};
use handshake_core::self_improve::iteration::{LoopTarget, OperatorId, PolicyParameterRef};
use handshake_core::self_improve::promotion_floor::{MetricDelta, PromotionDecision};
use handshake_core::self_improve::promotion_gate_adapter::{
    GateError, PromotionApproval, PromotionGateSubmitter, PromotionRejection, PromotionRequest,
    PromotionStatus, PromotionTicket,
};
use uuid::Uuid;

// ---------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------

/// Mock gate: production-shape `PromotionGateSubmitter` driven by an
/// explicit state map. Tests call `approve(ticket_id)` /
/// `reject(ticket_id, reason)` to flip the next `poll` result.
enum MockTicketState {
    Pending,
    Approved(PromotionApproval),
    Rejected(PromotionRejection),
}

struct MockGate {
    tickets: Mutex<HashMap<Uuid, MockTicketState>>,
}

impl MockGate {
    fn new() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
        }
    }

    fn register_pending(&self, ticket_id: Uuid) {
        self.tickets
            .lock()
            .unwrap()
            .insert(ticket_id, MockTicketState::Pending);
    }

    fn approve(&self, ticket_id: Uuid, op: &str) {
        let approval = PromotionApproval {
            approved_by: OperatorId::new(op),
            approved_at_utc: Utc::now(),
            signoff_evidence_id: Uuid::now_v7(),
        };
        self.tickets
            .lock()
            .unwrap()
            .insert(ticket_id, MockTicketState::Approved(approval));
    }

    fn reject(&self, ticket_id: Uuid, op: &str, reason: &str) {
        let rej = PromotionRejection {
            rejected_by: OperatorId::new(op),
            rejected_at_utc: Utc::now(),
            rejection_reason: reason.to_string(),
        };
        self.tickets
            .lock()
            .unwrap()
            .insert(ticket_id, MockTicketState::Rejected(rej));
    }
}

impl PromotionGateSubmitter for MockGate {
    fn submit(&self, _request: PromotionRequest) -> Result<PromotionTicket, GateError> {
        let ticket_id = Uuid::now_v7();
        self.tickets
            .lock()
            .unwrap()
            .insert(ticket_id, MockTicketState::Pending);
        Ok(PromotionTicket {
            ticket_id,
            iteration_id: Uuid::now_v7(),
            submitted_at_utc: Utc::now(),
        })
    }

    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        let guard = self.tickets.lock().unwrap();
        match guard.get(&ticket.ticket_id) {
            Some(MockTicketState::Pending) => Ok(PromotionStatus::Pending {
                submitted_at_utc: ticket.submitted_at_utc,
            }),
            Some(MockTicketState::Approved(a)) => Ok(PromotionStatus::Approved {
                approval: a.clone(),
            }),
            Some(MockTicketState::Rejected(r)) => Ok(PromotionStatus::Rejected {
                rejection: r.clone(),
            }),
            None => Err(GateError::UnknownTicket),
        }
    }
}

fn sample_request(iteration_id: Uuid) -> PromotionRequest {
    let snapshot = EditableSurfaceSnapshot::RetrievalPolicy {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameter::TopK,
        before_value: 6,
        after_value: 8,
    };
    PromotionRequest {
        iteration_id,
        target: LoopTarget::RetrievalPolicyParams {
            task_type: TaskType::ValidatorHbrTestPacket,
            parameter: PolicyParameterRef::TopK,
        },
        baseline_snapshot: snapshot.clone(),
        proposed_snapshot: snapshot,
        eval_result: EvalResult {
            train: SplitMetrics::empty(),
            dev: SplitMetrics::empty(),
            holdout: SplitMetrics::empty(),
            evaluated_at_utc: Utc::now(),
            snapshot_hash: "0".repeat(64),
        },
        floor_decision: PromotionDecision::Approved {
            delta: MetricDelta {
                dev_pass_delta_pp: 0.1,
                latency_p95_delta_ms: 0,
                capsule_bytes_p95_delta_bytes: 0,
                holdout_pass_delta_pp: 0.0,
            },
        },
        sentinel_decision: SentinelDecision::Continue,
        justification_text: "ipc test".to_string(),
    }
}

fn ticket_for(iteration_id: Uuid) -> PromotionTicket {
    PromotionTicket {
        ticket_id: Uuid::now_v7(),
        iteration_id,
        submitted_at_utc: Utc::now(),
    }
}

// ---------------------------------------------------------------
// status() — read-only projection
// ---------------------------------------------------------------

#[test]
fn status_returns_idle_at_construction_with_full_budget() {
    let state = LoopIpcState::new(25);
    let snap = state.status();
    assert_eq!(snap.loop_state, LoopState::Idle);
    assert!(!snap.paused);
    assert!(snap.pause_reason.is_none());
    assert_eq!(snap.pending_review_count, 0);
    assert_eq!(snap.iteration_budget_remaining_24h, 25);
    assert!(snap.last_iteration_id.is_none());
    assert!(snap.last_iteration_at_utc.is_none());
}

#[test]
fn status_is_idempotent_and_does_not_mutate_state() {
    let state = LoopIpcState::new(10);
    let a = state.status();
    let b = state.status();
    let c = state.status();
    assert_eq!(a, b);
    assert_eq!(b, c);
    // budget unchanged
    assert_eq!(c.iteration_budget_remaining_24h, 10);
}

#[test]
fn status_reflects_running_after_iteration_complete() {
    let state = LoopIpcState::new(3);
    let id = Uuid::now_v7();
    state.record_iteration_complete(id);
    let snap = state.status();
    assert_eq!(snap.loop_state, LoopState::Running);
    assert_eq!(snap.last_iteration_id, Some(id));
    assert!(snap.last_iteration_at_utc.is_some());
    assert_eq!(snap.iteration_budget_remaining_24h, 2);
}

#[test]
fn status_pending_review_count_matches_submit_for_review_count() {
    let state = LoopIpcState::new(25);
    assert_eq!(state.status().pending_review_count, 0);
    let r1 = sample_request(Uuid::now_v7());
    let r2 = sample_request(Uuid::now_v7());
    state
        .submit_for_review(r1.clone(), ticket_for(r1.iteration_id))
        .unwrap();
    state
        .submit_for_review(r2.clone(), ticket_for(r2.iteration_id))
        .unwrap();
    assert_eq!(state.status().pending_review_count, 2);
}

// ---------------------------------------------------------------
// pause() — sticky pause flag carrying operator rationale
// ---------------------------------------------------------------

#[test]
fn pause_sets_sticky_flag_with_operator_rationale() {
    let state = LoopIpcState::new(25);
    let receipt = state.pause("manual review required".to_string()).unwrap();
    assert_eq!(receipt.fr_event_kind, FR_EVT_LOOP_PAUSE);
    assert_eq!(receipt.receipt_id.get_version_num(), 7);
    match receipt.reason {
        PauseReason::Operator { rationale } => {
            assert_eq!(rationale, "manual review required");
        }
        _ => panic!("expected PauseReason::Operator"),
    }
    let snap = state.status();
    assert!(snap.paused);
    assert_eq!(snap.loop_state, LoopState::Paused);
    assert!(snap.pause_reason.is_some());
}

#[test]
fn pause_when_already_paused_returns_typed_already_paused_error() {
    let state = LoopIpcState::new(25);
    state.pause("first".to_string()).unwrap();
    let err = state.pause("second".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::AlreadyPaused));
    // Pause remains sticky.
    assert!(state.status().paused);
}

#[test]
fn pause_with_reason_carries_goodhart_monotonic_gap_widening() {
    let state = LoopIpcState::new(25);
    let reason = PauseReason::MonotonicGapWidening {
        gaps: vec![0.01, 0.02, 0.03],
        iteration_numbers: vec![1, 2, 3],
    };
    let receipt = state.pause_with_reason(reason.clone()).unwrap();
    assert_eq!(receipt.fr_event_kind, FR_EVT_LOOP_PAUSE);
    assert_eq!(receipt.reason, reason);
    let snap = state.status();
    assert!(snap.paused);
    match snap.pause_reason {
        Some(PauseReason::MonotonicGapWidening {
            gaps,
            iteration_numbers,
        }) => {
            assert_eq!(gaps, vec![0.01, 0.02, 0.03]);
            assert_eq!(iteration_numbers, vec![1, 2, 3]);
        }
        other => panic!("unexpected pause reason: {:?}", other),
    }
}

// ---------------------------------------------------------------
// unpause() — clears the flag (idempotent if already unpaused -> typed error)
// ---------------------------------------------------------------

#[test]
fn unpause_when_not_paused_returns_typed_not_paused_error() {
    let state = LoopIpcState::new(25);
    let err = state.unpause("nothing to clear".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::NotPaused));
}

#[test]
fn unpause_clears_pause_and_emits_unpause_event() {
    let state = LoopIpcState::new(25);
    state.pause("manual".to_string()).unwrap();
    let receipt = state.unpause("reviewed".to_string()).unwrap();
    assert_eq!(receipt.fr_event_kind, FR_EVT_LOOP_UNPAUSE);
    assert_eq!(receipt.rationale, "reviewed");
    let snap = state.status();
    assert!(!snap.paused);
    assert!(snap.pause_reason.is_none());
    assert_eq!(snap.loop_state, LoopState::Idle);
}

#[test]
fn unpause_with_empty_rationale_rejected_and_pause_remains_sticky() {
    let state = LoopIpcState::new(25);
    state.pause("manual".to_string()).unwrap();
    let err = state.unpause("   ".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::EmptyRationale));
    // Pause must remain sticky after rejected unpause attempt.
    assert!(state.status().paused);
}

#[test]
fn unpause_then_unpause_again_returns_typed_not_paused() {
    let state = LoopIpcState::new(25);
    state.pause("manual".to_string()).unwrap();
    state.unpause("done".to_string()).unwrap();
    let err = state.unpause("again".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::NotPaused));
}

#[test]
fn pause_then_unpause_then_pause_again_works() {
    let state = LoopIpcState::new(25);
    state.pause("first".to_string()).unwrap();
    state.unpause("clear".to_string()).unwrap();
    state.pause("second".to_string()).unwrap();
    assert!(state.status().paused);
}

// ---------------------------------------------------------------
// review_pending — lists tickets awaiting PromotionGate approval
// ---------------------------------------------------------------

#[test]
fn list_pending_reviews_empty_at_construction() {
    let state = LoopIpcState::new(25);
    let pending = state.list_pending_reviews(10);
    assert!(pending.is_empty());
}

#[test]
fn list_pending_reviews_returns_submitted_requests() {
    let state = LoopIpcState::new(25);
    let r1 = sample_request(Uuid::now_v7());
    let r2 = sample_request(Uuid::now_v7());
    state
        .submit_for_review(r1.clone(), ticket_for(r1.iteration_id))
        .unwrap();
    state
        .submit_for_review(r2.clone(), ticket_for(r2.iteration_id))
        .unwrap();
    let pending = state.list_pending_reviews(10);
    assert_eq!(pending.len(), 2);
    let ids: Vec<Uuid> = pending.iter().map(|r| r.iteration_id).collect();
    assert!(ids.contains(&r1.iteration_id));
    assert!(ids.contains(&r2.iteration_id));
}

#[test]
fn list_pending_reviews_respects_limit() {
    let state = LoopIpcState::new(25);
    for _ in 0..5 {
        let r = sample_request(Uuid::now_v7());
        state
            .submit_for_review(r.clone(), ticket_for(r.iteration_id))
            .unwrap();
    }
    let three = state.list_pending_reviews(3);
    assert_eq!(three.len(), 3);
    let all_five = state.list_pending_reviews(10);
    assert_eq!(all_five.len(), 5);
}

#[test]
fn approve_promotion_with_approved_status_clears_pending() {
    let gate = Arc::new(MockGate::new());
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    let ticket = ticket_for(r.iteration_id);
    gate.register_pending(ticket.ticket_id);
    state.submit_for_review(r.clone(), ticket.clone()).unwrap();
    gate.approve(ticket.ticket_id, "operator-1");
    let approval = state
        .approve_promotion(
            gate.clone(),
            r.iteration_id,
            OperatorId::new("operator-1"),
            "looks good".to_string(),
        )
        .unwrap();
    assert_eq!(approval.approved_by.as_str(), "operator-1");
    assert_eq!(approval.signoff_evidence_id.get_version_num(), 7);
    assert_eq!(state.status().pending_review_count, 0);
}

#[test]
fn approve_promotion_for_unknown_iteration_returns_typed_error() {
    struct StubGate;
    impl PromotionGateSubmitter for StubGate {
        fn submit(&self, _r: PromotionRequest) -> Result<PromotionTicket, GateError> {
            unreachable!()
        }
        fn poll(&self, _t: &PromotionTicket) -> Result<PromotionStatus, GateError> {
            unreachable!()
        }
    }
    let state = LoopIpcState::new(25);
    let err = state
        .approve_promotion(
            Arc::new(StubGate),
            Uuid::now_v7(),
            OperatorId::new("op-1"),
            "rationale".to_string(),
        )
        .unwrap_err();
    assert!(matches!(err, LoopIpcError::UnknownIteration { .. }));
}

#[test]
fn approve_promotion_with_empty_rationale_returns_typed_error() {
    let gate = Arc::new(MockGate::new());
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    let ticket = ticket_for(r.iteration_id);
    state.submit_for_review(r.clone(), ticket).unwrap();
    let err = state
        .approve_promotion(
            gate,
            r.iteration_id,
            OperatorId::new("op-1"),
            "  ".to_string(),
        )
        .unwrap_err();
    assert!(matches!(err, LoopIpcError::EmptyRationale));
}

#[test]
fn reject_promotion_records_rejection_and_clears_pending() {
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    state
        .submit_for_review(r.clone(), ticket_for(r.iteration_id))
        .unwrap();
    assert_eq!(state.status().pending_review_count, 1);
    let rejection = state
        .reject_promotion(
            r.iteration_id,
            OperatorId::new("op-1"),
            "too risky".to_string(),
        )
        .unwrap();
    assert_eq!(rejection.rejected_by.as_str(), "op-1");
    assert_eq!(rejection.rejection_reason, "too risky");
    assert_eq!(state.status().pending_review_count, 0);
}

#[test]
fn reject_promotion_for_unknown_iteration_returns_typed_error() {
    let state = LoopIpcState::new(25);
    let err = state
        .reject_promotion(
            Uuid::now_v7(),
            OperatorId::new("op-1"),
            "rationale".to_string(),
        )
        .unwrap_err();
    assert!(matches!(err, LoopIpcError::UnknownIteration { .. }));
}

#[test]
fn reject_promotion_with_empty_rationale_returns_typed_error() {
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    state
        .submit_for_review(r.clone(), ticket_for(r.iteration_id))
        .unwrap();
    let err = state
        .reject_promotion(r.iteration_id, OperatorId::new("op-1"), "".to_string())
        .unwrap_err();
    assert!(matches!(err, LoopIpcError::EmptyRationale));
    // Pending entry remains because rejection was rejected.
    assert_eq!(state.status().pending_review_count, 1);
}

#[test]
fn approve_promotion_propagates_gate_rejected_as_typed_gate_error() {
    let gate = Arc::new(MockGate::new());
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    let ticket = ticket_for(r.iteration_id);
    gate.register_pending(ticket.ticket_id);
    state.submit_for_review(r.clone(), ticket.clone()).unwrap();
    gate.reject(ticket.ticket_id, "op-1", "bad change");
    let err = state
        .approve_promotion(
            gate,
            r.iteration_id,
            OperatorId::new("op-1"),
            "trying anyway".to_string(),
        )
        .unwrap_err();
    match err {
        LoopIpcError::Gate(GateError::ReviewRejected { rationale }) => {
            assert_eq!(rationale, "bad change");
        }
        other => panic!("expected Gate(ReviewRejected), got: {:?}", other),
    }
}

#[test]
fn approve_promotion_while_gate_pending_fails_closed_and_keeps_review() {
    // MT-155 finding #1 (FAIL) regression: approving while the gate is still
    // Pending must NOT mint a synthesized approval. It must return a typed
    // ReviewPending error AND leave the iteration in pending_reviews so the
    // operator-review-required invariant holds.
    let gate = Arc::new(MockGate::new());
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    let ticket = ticket_for(r.iteration_id);
    gate.register_pending(ticket.ticket_id); // gate stays Pending — never approved
    state.submit_for_review(r.clone(), ticket.clone()).unwrap();

    let err = state
        .approve_promotion(
            gate.clone(),
            r.iteration_id,
            OperatorId::new("operator-eager"),
            "approve before gate responded".to_string(),
        )
        .unwrap_err();
    assert!(
        matches!(err, LoopIpcError::Gate(GateError::ReviewPending)),
        "pending gate must yield typed ReviewPending, not a synthesized approval; got {err:?}"
    );
    // The review must remain pending — fail-closed leaves state intact.
    assert_eq!(
        state.status().pending_review_count,
        1,
        "iteration must stay in review until the gate actually decides"
    );
    assert!(
        state.ticket_for_iteration(r.iteration_id).is_some(),
        "pending ticket must not be cleared on a failed-closed approve"
    );
}

#[test]
fn submit_for_review_rejects_iteration_id_mismatch() {
    // MT-155 finding #2 regression: a ticket whose iteration_id does not
    // match the request must be rejected so the two maps cannot desync.
    let state = LoopIpcState::new(25);
    let r = sample_request(Uuid::now_v7());
    let mismatched_ticket = ticket_for(Uuid::now_v7()); // different iteration_id
    let err = state
        .submit_for_review(r.clone(), mismatched_ticket)
        .unwrap_err();
    match err {
        LoopIpcError::TicketIterationMismatch {
            request_iteration_id,
            ticket_iteration_id,
        } => {
            assert_eq!(request_iteration_id, r.iteration_id);
            assert_ne!(ticket_iteration_id, r.iteration_id);
        }
        other => panic!("expected TicketIterationMismatch; got {other:?}"),
    }
    // Nothing was registered.
    assert_eq!(state.status().pending_review_count, 0);
}

// ---------------------------------------------------------------
// Cross-thread sticky-pause semantics
// ---------------------------------------------------------------

#[test]
fn pause_visible_across_threads() {
    let state = Arc::new(LoopIpcState::new(25));
    state.pause("global pause".to_string()).unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let s = Arc::clone(&state);
        handles.push(thread::spawn(move || {
            // Every observer thread must see the sticky pause.
            assert!(s.status().paused);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn concurrent_pause_attempts_only_one_succeeds() {
    let state = Arc::new(LoopIpcState::new(25));
    let success_counter = Arc::new(Mutex::new(0u32));

    let mut handles = Vec::new();
    for i in 0..16 {
        let s = Arc::clone(&state);
        let c = Arc::clone(&success_counter);
        handles.push(thread::spawn(move || {
            if s.pause(format!("thread-{i}")).is_ok() {
                *c.lock().unwrap() += 1;
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let successes = *success_counter.lock().unwrap();
    assert_eq!(successes, 1, "exactly one pause should succeed");
    assert!(state.status().paused);
}

#[test]
fn cross_thread_unpause_visible() {
    let state = Arc::new(LoopIpcState::new(25));
    state.pause("on-main".to_string()).unwrap();
    let worker = {
        let s = Arc::clone(&state);
        thread::spawn(move || {
            s.unpause("from-worker".to_string()).unwrap();
        })
    };
    worker.join().unwrap();
    assert!(!state.status().paused);
}

#[test]
fn concurrent_status_polls_are_consistent_with_pause() {
    let state = Arc::new(LoopIpcState::new(25));
    state.pause("global".to_string()).unwrap();
    let mut handles = Vec::new();
    for _ in 0..32 {
        let s = Arc::clone(&state);
        handles.push(thread::spawn(move || {
            let snap = s.status();
            assert!(snap.paused);
            assert_eq!(snap.loop_state, LoopState::Paused);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

// ---------------------------------------------------------------
// Serde round-trip for receipts
// ---------------------------------------------------------------

#[test]
fn pause_receipt_round_trips_via_serde() {
    let state = LoopIpcState::new(25);
    let receipt = state.pause("manual".to_string()).unwrap();
    let json = serde_json::to_string(&receipt).unwrap();
    let recovered: handshake_core::self_improve::ipc::PauseReceipt =
        serde_json::from_str(&json).unwrap();
    assert_eq!(recovered, receipt);
}

#[test]
fn unpause_receipt_round_trips_via_serde() {
    let state = LoopIpcState::new(25);
    state.pause("m".to_string()).unwrap();
    let receipt = state.unpause("done".to_string()).unwrap();
    let json = serde_json::to_string(&receipt).unwrap();
    let recovered: handshake_core::self_improve::ipc::UnpauseReceipt =
        serde_json::from_str(&json).unwrap();
    assert_eq!(recovered, receipt);
}

#[test]
fn status_snapshot_round_trips_via_serde() {
    let state = LoopIpcState::new(7);
    state.pause("inspect".to_string()).unwrap();
    let snap = state.status();
    let json = serde_json::to_string(&snap).unwrap();
    let recovered: handshake_core::self_improve::ipc::LoopStatusSnapshot =
        serde_json::from_str(&json).unwrap();
    assert_eq!(recovered, snap);
    assert!(recovered.paused);
    assert_eq!(recovered.iteration_budget_remaining_24h, 7);
}

#[test]
fn loop_ipc_error_round_trips_via_serde() {
    let err = LoopIpcError::AlreadyPaused;
    let json = serde_json::to_string(&err).unwrap();
    let recovered: LoopIpcError = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered, err);
    let err2 = LoopIpcError::UnknownIteration {
        iteration_id: Uuid::now_v7(),
    };
    let json2 = serde_json::to_string(&err2).unwrap();
    let recovered2: LoopIpcError = serde_json::from_str(&json2).unwrap();
    assert_eq!(recovered2, err2);
}
