//! MT-156 — LoopScheduler integration tests.
//!
//! Per the MT-156 contract proof_command:
//!   `cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target loop_scheduler_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/scheduler.rs
//! already covers basic schedule/skip discriminants (7 inline tests:
//! empty-history schedules, at-cap skips, rolling window drops old,
//! operator pause blocks, goodhart pause blocks, pending review blocks,
//! cap_event builds from BudgetExhausted). This integration test file
//! satisfies the contract owned_files entry and proves the cross-cutting
//! invariants required by the MT-156 red_team minimum_controls + the
//! HBR-SWARM-002 loop counter cap contract:
//!
//!   - HBR-SWARM-002: at most 25 iterations / rolling 24h window per
//!     loop instance; 26th call returns Skip BudgetExhausted typed.
//!   - Rolling window is a TRUE 24h slide, not a fixed daily reset; the
//!     "burst + wait + burst" attack on a fixed-window scheduler does not
//!     work here (each iteration falls out exactly 24h after it landed).
//!   - Pending review for a prior iteration blocks scheduling — proven
//!     end-to-end through MT-154 LoopPromotionGate (submit returns
//!     ticket; scheduler observes Pending and refuses to schedule
//!     another iteration; once the gate Approves, scheduler unblocks).
//!   - Goodhart pause (MT-153) auto-pauses scheduling; once the operator
//!     unpauses via the IPC state (MT-155), scheduling resumes.
//!   - Operator pause (MT-155) blocks scheduling; unpause resumes.
//!   - Skip-reason priority order is deterministic when multiple block
//!     conditions co-fire (operator pause > goodhart pause > pending
//!     review > budget exhausted) — proven by feeding all four conditions
//!     and inspecting which typed reason surfaces.
//!   - FR-EVT-DISTILL-LOOP-CAP receipt is built exactly once per
//!     cap-skip via `LoopScheduler::cap_event`; non-budget reasons return
//!     None so audit replay does not double-count cap rejections.
//!   - SchedulerHistory entries are bounded so long-running loops don't
//!     leak memory (push past the internal cap drops the oldest).
//!   - Determinism: given fixed clock + history + flag inputs, the
//!     scheduler returns identical decisions across repeated calls.
//!   - Concurrency: multiple parallel schedulers reading the same history
//!     observe consistent counts (sched is Copy + history is &-borrowed).
//!   - SchedulerHistory + SchedulerHistoryEntry + ScheduleDecision +
//!     SkipReason + LoopCapEvent + IterationBudget all serde round-trip
//!     so EventLedger replay can reconstruct the schedule trace
//!     byte-for-byte.
//!   - Custom budget overrides honor HBR-SWARM-002 default but allow
//!     operator tuning (smaller cap for prod or larger for synthetic
//!     stress tests).
//!   - Cluster-D cross-cutting: scheduler is the only caller of
//!     `LoopCore::run_one_iteration`. The integration tests don't drive
//!     LoopCore directly (that lives in MT-148 territory) but verify
//!     the scheduler returns Schedule/Skip on every public surface so a
//!     caller can wire the dispatch loop without guessing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use handshake_core::memory::TaskType;
use handshake_core::self_improve::ipc::{LoopIpcError, LoopIpcState};
use handshake_core::self_improve::{
    EditableSurfaceSnapshot, EvalResult, GateError, GoodhartSentinel, IterationBudget,
    LoopPromotionGate, LoopScheduler, LoopTarget, MetricDelta, OperatorId, PauseReason,
    PolicyParameter, PromotionApproval, PromotionDecision, PromotionGateSubmitter,
    PromotionRejection, PromotionRequest, PromotionStatus, PromotionTicket, ScheduleDecision,
    SchedulerHistory, SchedulerHistoryEntry, SentinelDecision, SentinelEntry, SentinelHistory,
    SentinelReceipt, SkipReason, SplitMetrics, FR_EVT_DISTILL_LOOP_CAP, FR_EVT_GOODHART_PAUSE,
};

// ----------------------------------------------------------------------------
// Mock gate: drives the LoopPromotionGate so the scheduler's pending-review
// gating path can be exercised end-to-end with typed PromotionTicket /
// PromotionStatus transitions.
// ----------------------------------------------------------------------------

enum TicketState {
    Pending,
    Approved(PromotionApproval),
    Rejected(PromotionRejection),
}

struct MockGate {
    tickets: Mutex<HashMap<Uuid, TicketState>>,
}

impl MockGate {
    fn new() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
        }
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
            .insert(ticket_id, TicketState::Approved(approval));
    }
}

impl PromotionGateSubmitter for MockGate {
    fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
        let ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: request.iteration_id,
            submitted_at_utc: Utc::now(),
        };
        self.tickets
            .lock()
            .unwrap()
            .insert(ticket.ticket_id, TicketState::Pending);
        Ok(ticket)
    }

    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        let guard = self.tickets.lock().unwrap();
        match guard.get(&ticket.ticket_id) {
            Some(TicketState::Pending) => Ok(PromotionStatus::Pending {
                submitted_at_utc: ticket.submitted_at_utc,
            }),
            Some(TicketState::Approved(a)) => Ok(PromotionStatus::Approved {
                approval: a.clone(),
            }),
            Some(TicketState::Rejected(r)) => Ok(PromotionStatus::Rejected {
                rejection: r.clone(),
            }),
            None => Err(GateError::UnknownTicket),
        }
    }
}

// ----------------------------------------------------------------------------
// Fixture helpers — keep test bodies short and focused on the invariant.
// ----------------------------------------------------------------------------

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

fn eval_result(dev: f64, holdout: f64) -> EvalResult {
    EvalResult {
        train: SplitMetrics::empty(),
        dev: split(dev),
        holdout: split(holdout),
        evaluated_at_utc: Utc::now(),
        snapshot_hash: "0".repeat(64),
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

fn target() -> LoopTarget {
    LoopTarget::RetrievalPolicyParams {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameter::TopK,
    }
}

fn sample_request(iteration_id: Uuid) -> PromotionRequest {
    PromotionRequest {
        iteration_id,
        target: target(),
        baseline_snapshot: snapshot(6, 6),
        proposed_snapshot: snapshot(6, 8),
        eval_result: eval_result(0.70, 0.60),
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

/// Build a SchedulerHistory containing `count` entries all stamped at
/// `seconds_ago` before `now`.
fn history_at(now: DateTime<Utc>, count: u32, seconds_ago: i64) -> SchedulerHistory {
    let mut h = SchedulerHistory::new();
    for _ in 0..count {
        h.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: now - Duration::seconds(seconds_ago),
        });
    }
    h
}

/// Build a SentinelReceipt the scheduler can consume for goodhart_pause.
fn goodhart_receipt() -> SentinelReceipt {
    SentinelReceipt {
        receipt_id: Uuid::now_v7(),
        paused_at_utc: Utc::now(),
        fr_event_kind: FR_EVT_GOODHART_PAUSE.to_string(),
        history_snapshot: SentinelHistory::default(),
    }
}

/// Build a PromotionTicket the scheduler can consume for pending_review.
fn pending_ticket(iteration_id: Uuid) -> PromotionTicket {
    PromotionTicket {
        ticket_id: Uuid::now_v7(),
        iteration_id,
        submitted_at_utc: Utc::now(),
    }
}

// ============================================================================
// HBR-SWARM-002: 25 iterations / 24h cap enforcement
// ============================================================================

#[test]
fn mt156_default_budget_is_25_iterations_per_24h() {
    let budget = IterationBudget::default();
    assert_eq!(budget.max_iterations_per_24h, 25, "HBR-SWARM-002 cap = 25");
    assert_eq!(
        budget.rolling_window_seconds, 86400,
        "HBR-SWARM-002 window = 24h"
    );
}

#[test]
fn mt156_empty_history_schedules() {
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let decision = scheduler.try_schedule_next(&history, Utc::now(), None, None, None);
    assert!(
        decision.is_schedule(),
        "empty history must schedule; got {:?}",
        decision
    );
}

#[test]
fn mt156_twenty_fifth_iteration_in_window_still_schedules() {
    // 24 entries already in the window; 25th call should succeed (cap is 25,
    // so we can schedule up to AND INCLUDING the 25th).
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 24, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "24 entries in window => 25th schedule must succeed (cap is 25, not 24)"
    );
}

#[test]
fn mt156_twenty_sixth_iteration_in_window_rejects_with_typed_budget_exhausted() {
    // 25 entries already in the window; 26th call must Skip BudgetExhausted.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    match decision {
        ScheduleDecision::Skip { reason } => match reason {
            SkipReason::BudgetExhausted {
                count_in_window,
                cap,
                window_started_at_utc,
                next_eligible_at_utc,
            } => {
                assert_eq!(count_in_window, 25, "26th call sees 25 entries in window");
                assert_eq!(cap, 25, "HBR-SWARM-002 cap surfaces in typed reason");
                // window_started_at_utc must be the OLDEST entry in window.
                assert!(
                    window_started_at_utc <= now,
                    "window_started_at_utc must be <= now"
                );
                // next_eligible_at_utc = window_started_at_utc + 24h.
                let delta = next_eligible_at_utc - window_started_at_utc;
                assert_eq!(delta.num_seconds(), 86400, "next_eligible = oldest + 24h");
            }
            other => panic!("expected BudgetExhausted; got {:?}", other),
        },
        ScheduleDecision::Schedule => panic!("expected Skip; got Schedule"),
    }
}

#[test]
fn mt156_count_in_window_strictly_excludes_entries_outside_24h() {
    // 25 entries all stamped > 24h ago; the rolling window must drop them
    // ALL and a new iteration must schedule.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 86_400 + 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "entries > 24h must fall out of window"
    );
}

// ============================================================================
// Rolling window is a TRUE 24h slide, not a fixed-daily reset
// ============================================================================

#[test]
fn mt156_rolling_window_burst_wait_burst_does_not_evade_cap() {
    // Adversarial scenario: a fixed-daily-reset scheduler would let an
    // attacker run 25 iterations late in day N, then 25 more early in day
    // N+1 (within 24h of the first batch) — total 50 iterations in ~26h.
    // A TRUE rolling window prevents this: iterations from the first burst
    // remain in the window until they each independently age past 24h.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();

    // Burst 1: 13 iterations stamped 12h ago.
    let mut history = SchedulerHistory::new();
    for _ in 0..13 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: now - Duration::hours(12),
        });
    }
    // Burst 2: 12 iterations stamped 30 minutes ago.
    for _ in 0..12 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: now - Duration::minutes(30),
        });
    }

    // Total 25 entries inside the 24h window — cap met, 26th rejected.
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        !decision.is_schedule(),
        "true rolling window must count BOTH bursts; 26th must reject"
    );
    match decision.skip_reason() {
        Some(SkipReason::BudgetExhausted { count_in_window, .. }) => {
            assert_eq!(*count_in_window, 25, "all 25 bursted entries still in window");
        }
        other => panic!("expected BudgetExhausted; got {:?}", other),
    }
}

#[test]
fn mt156_rolling_window_oldest_entry_aging_out_unblocks_one_slot() {
    // 25 entries; one of them is stamped just BARELY older than 24h. Window
    // count should drop to 24 -> 25th call schedules.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let mut history = SchedulerHistory::new();
    // 24 entries stamped within 24h.
    for _ in 0..24 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: now - Duration::hours(12),
        });
    }
    // 1 entry stamped > 24h.
    history.push(SchedulerHistoryEntry {
        iteration_id: Uuid::now_v7(),
        scheduled_at_utc: now - Duration::seconds(86_500),
    });
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "aged-out entry must free a slot; 24 in-window count <= 25 cap"
    );
}

#[test]
fn mt156_count_at_exact_24h_boundary_is_excluded_from_window() {
    // Entry stamped exactly 24h ago: per the implementation cutoff =
    // now - 24h and the filter keeps entries STRICTLY greater than the
    // cutoff. So an entry at exactly cutoff falls out of window.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let mut history = SchedulerHistory::new();
    history.push(SchedulerHistoryEntry {
        iteration_id: Uuid::now_v7(),
        scheduled_at_utc: now - Duration::seconds(86_400), // exactly cutoff
    });
    // Cap is 25 but we have only the one entry at-or-past-cutoff (filtered
    // out). Should still schedule.
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "entry at exact 24h boundary falls out of strict-greater-than window"
    );
}

// ============================================================================
// Pending review for prior iteration blocks scheduling
// ============================================================================

#[test]
fn mt156_pending_review_blocks_with_typed_reason() {
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let iteration_id = Uuid::now_v7();
    let ticket = pending_ticket(iteration_id);
    let decision = scheduler.try_schedule_next(
        &history,
        Utc::now(),
        None,
        None,
        Some((iteration_id, ticket.clone())),
    );
    match decision {
        ScheduleDecision::Skip { reason } => match reason {
            SkipReason::PendingPromotionReview {
                iteration_id: rid,
                ticket: rticket,
            } => {
                assert_eq!(rid, iteration_id);
                assert_eq!(rticket.ticket_id, ticket.ticket_id);
                assert_eq!(rticket.iteration_id, ticket.iteration_id);
            }
            other => panic!("expected PendingPromotionReview; got {:?}", other),
        },
        ScheduleDecision::Schedule => {
            panic!("pending review must block scheduling for the prior iteration")
        }
    }
}

#[test]
fn mt156_end_to_end_pending_review_blocks_then_approve_unblocks() {
    // Full MT-154 + MT-156 integration: submit -> ticket Pending ->
    // scheduler blocks; gate approves -> scheduler reads None for
    // pending_review -> scheduler schedules next iteration.
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let gate = MockGate::new();
    let adapter = LoopPromotionGate::new(&gate);
    let iteration_id = Uuid::now_v7();

    // Submit the prior iteration to the gate.
    let request = sample_request(iteration_id);
    let ticket = adapter.submit(request).expect("submit ok");

    // While ticket is Pending, the loop driver consults the gate, sees
    // Pending, and feeds Some((iter, ticket)) to the scheduler.
    let status = adapter.poll(&ticket).expect("poll ok");
    assert!(status.is_pending(), "ticket starts Pending");
    let decision_while_pending = scheduler.try_schedule_next(
        &history,
        Utc::now(),
        None,
        None,
        Some((iteration_id, ticket.clone())),
    );
    assert!(matches!(
        decision_while_pending.skip_reason(),
        Some(SkipReason::PendingPromotionReview { .. })
    ));

    // Operator approves via the gate.
    gate.approve(ticket.ticket_id, "operator-1");
    let status_after = adapter.poll(&ticket).expect("poll ok");
    assert!(
        status_after.is_approved(),
        "approve must make poll return Approved"
    );

    // Driver now reads pending_review = None (loop core would have
    // cleared the slot on approval). Scheduler unblocks.
    let decision_after_approve =
        scheduler.try_schedule_next(&history, Utc::now(), None, None, None);
    assert!(decision_after_approve.is_schedule());
}

// ============================================================================
// Goodhart pause (MT-153) blocks scheduling
// ============================================================================

#[test]
fn mt156_goodhart_pause_blocks_with_typed_reason() {
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let receipt = goodhart_receipt();
    let decision =
        scheduler.try_schedule_next(&history, Utc::now(), None, Some(&receipt), None);
    match decision.skip_reason() {
        Some(SkipReason::GoodhartPause { receipt: r }) => {
            assert_eq!(r.fr_event_kind, FR_EVT_GOODHART_PAUSE);
            assert_eq!(
                r.receipt_id.get_version_num(),
                7,
                "sentinel receipt id must be UUIDv7"
            );
        }
        other => panic!("expected GoodhartPause; got {:?}", other),
    }
}

#[test]
fn mt156_end_to_end_goodhart_detection_then_unpause_resumes_scheduling() {
    // Drive MT-153 -> MT-155 -> MT-156: build a sentinel history with 3
    // consecutive widening transitions; GoodhartSentinel::evaluate fires
    // Pause; route the PauseReason into LoopIpcState; scheduler sees the
    // pause and refuses; operator unpauses via IPC; scheduler resumes.
    let scheduler = LoopScheduler::with_defaults();

    // Build a sentinel history with widening gaps.
    let mut sentinel_history = SentinelHistory::default();
    sentinel_history.push(SentinelEntry {
        iteration_number: 1,
        dev_pass_rate: 0.55,
        holdout_pass_rate: 0.50,
        gap: 0.05,
        accepted_at_utc: Utc::now(),
    });
    sentinel_history.push(SentinelEntry {
        iteration_number: 2,
        dev_pass_rate: 0.60,
        holdout_pass_rate: 0.50,
        gap: 0.10,
        accepted_at_utc: Utc::now(),
    });
    sentinel_history.push(SentinelEntry {
        iteration_number: 3,
        dev_pass_rate: 0.65,
        holdout_pass_rate: 0.50,
        gap: 0.15,
        accepted_at_utc: Utc::now(),
    });
    // Latest eval widens again (gap 0.20).
    let latest = eval_result(0.70, 0.50);
    let decision = GoodhartSentinel::evaluate(&sentinel_history, &latest);
    let (pause_reason, sentinel_receipt) = match decision {
        SentinelDecision::Pause { reason, receipt } => (reason, receipt),
        SentinelDecision::Continue => {
            panic!("expected sentinel Pause on 3 widening transitions")
        }
    };

    // Route through LoopIpcState (MT-155): pause_with_reason should succeed.
    let ipc = LoopIpcState::new(25);
    ipc.pause_with_reason(pause_reason.clone())
        .expect("pause_with_reason");
    assert!(ipc.status().paused);

    // Scheduler observes goodhart pause (from sentinel receipt).
    let scheduler_history = SchedulerHistory::new();
    let blocked = scheduler.try_schedule_next(
        &scheduler_history,
        Utc::now(),
        None,
        Some(&sentinel_receipt),
        None,
    );
    assert!(matches!(
        blocked.skip_reason(),
        Some(SkipReason::GoodhartPause { .. })
    ));

    // Operator unpauses with rationale.
    ipc.unpause("reviewed Goodhart drift; metrics stabilized".to_string())
        .expect("unpause ok");
    assert!(!ipc.status().paused);

    // Driver clears goodhart_pause input. Scheduler resumes.
    let resumed = scheduler.try_schedule_next(&scheduler_history, Utc::now(), None, None, None);
    assert!(resumed.is_schedule());
}

// ============================================================================
// Operator pause (MT-155) blocks scheduling
// ============================================================================

#[test]
fn mt156_operator_pause_blocks_with_typed_reason() {
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let decision = scheduler.try_schedule_next(
        &history,
        Utc::now(),
        Some("manual safety check"),
        None,
        None,
    );
    match decision.skip_reason() {
        Some(SkipReason::OperatorPause { rationale }) => {
            assert_eq!(rationale, "manual safety check");
        }
        other => panic!("expected OperatorPause; got {:?}", other),
    }
}

#[test]
fn mt156_operator_pause_then_unpause_via_ipc_state_resumes_scheduling() {
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let ipc = LoopIpcState::new(25);

    // Operator pauses.
    ipc.pause("manual review".to_string()).expect("pause ok");
    let snap = ipc.status();
    assert!(snap.paused);
    let rationale = match snap.pause_reason {
        Some(PauseReason::Operator { rationale }) => rationale,
        other => panic!("expected Operator pause reason; got {:?}", other),
    };

    // Driver sees IPC pause; routes it as operator_paused into scheduler.
    let blocked = scheduler.try_schedule_next(
        &history,
        Utc::now(),
        Some(rationale.as_str()),
        None,
        None,
    );
    assert!(matches!(
        blocked.skip_reason(),
        Some(SkipReason::OperatorPause { .. })
    ));

    // Unpause.
    ipc.unpause("done".to_string()).expect("unpause ok");
    assert!(!ipc.status().paused);
    let resumed = scheduler.try_schedule_next(&history, Utc::now(), None, None, None);
    assert!(resumed.is_schedule());
}

// ============================================================================
// Skip-reason priority ordering when multiple block conditions co-fire
// ============================================================================

#[test]
fn mt156_skip_reason_priority_operator_pause_wins_over_all_other_blocks() {
    // All four block conditions fire simultaneously. The scheduler's
    // documented priority is: operator > goodhart > pending-review > budget.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60); // cap exhausted
    let receipt = goodhart_receipt();
    let iter = Uuid::now_v7();
    let ticket = pending_ticket(iter);

    let decision = scheduler.try_schedule_next(
        &history,
        now,
        Some("operator stop"),
        Some(&receipt),
        Some((iter, ticket)),
    );
    assert!(matches!(
        decision.skip_reason(),
        Some(SkipReason::OperatorPause { .. })
    ));
}

#[test]
fn mt156_skip_reason_priority_goodhart_wins_over_pending_and_budget() {
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let receipt = goodhart_receipt();
    let iter = Uuid::now_v7();
    let ticket = pending_ticket(iter);
    let decision = scheduler.try_schedule_next(
        &history,
        now,
        None,
        Some(&receipt),
        Some((iter, ticket)),
    );
    assert!(matches!(
        decision.skip_reason(),
        Some(SkipReason::GoodhartPause { .. })
    ));
}

#[test]
fn mt156_skip_reason_priority_pending_review_wins_over_budget() {
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let iter = Uuid::now_v7();
    let ticket = pending_ticket(iter);
    let decision = scheduler.try_schedule_next(&history, now, None, None, Some((iter, ticket)));
    assert!(matches!(
        decision.skip_reason(),
        Some(SkipReason::PendingPromotionReview { .. })
    ));
}

#[test]
fn mt156_skip_reason_priority_budget_fires_when_no_other_block_active() {
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(matches!(
        decision.skip_reason(),
        Some(SkipReason::BudgetExhausted { .. })
    ));
}

// ============================================================================
// FR-EVT-DISTILL-LOOP-CAP receipt invariants
// ============================================================================

#[test]
fn mt156_cap_event_built_from_budget_exhausted_only() {
    let now = Utc::now();
    let scheduler = LoopScheduler::with_defaults();
    let history = history_at(now, 25, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    let reason = decision.skip_reason().expect("must Skip");
    let event = LoopScheduler::cap_event(reason).expect("cap event for BudgetExhausted");
    assert_eq!(event.kind, FR_EVT_DISTILL_LOOP_CAP);
    assert_eq!(event.count_in_window, 25);
    assert_eq!(event.cap, 25);
    // next_eligible_at_utc - window_started_at_utc == 24h
    let delta = event.next_eligible_at_utc - event.window_started_at_utc;
    assert_eq!(delta.num_seconds(), 86_400);
}

#[test]
fn mt156_cap_event_returns_none_for_non_budget_skips() {
    // Per the contract, FR-EVT-DISTILL-LOOP-CAP is emitted ONLY on
    // BudgetExhausted so audit reconstruction does not double-count
    // operator / goodhart / pending-review pauses.
    let operator_reason = SkipReason::OperatorPause {
        rationale: "x".to_string(),
    };
    let goodhart_reason = SkipReason::GoodhartPause {
        receipt: goodhart_receipt(),
    };
    let pending_reason = SkipReason::PendingPromotionReview {
        iteration_id: Uuid::now_v7(),
        ticket: pending_ticket(Uuid::now_v7()),
    };
    assert!(LoopScheduler::cap_event(&operator_reason).is_none());
    assert!(LoopScheduler::cap_event(&goodhart_reason).is_none());
    assert!(LoopScheduler::cap_event(&pending_reason).is_none());
}

#[test]
fn mt156_cap_event_serde_round_trip_preserves_fr_event_kind() {
    // EventLedger replay must be able to reconstruct FR-EVT-DISTILL-LOOP-CAP
    // byte-for-byte so audit tooling can index by kind.
    let now = Utc::now();
    let scheduler = LoopScheduler::with_defaults();
    let history = history_at(now, 25, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    let reason = decision.skip_reason().expect("must Skip");
    let event = LoopScheduler::cap_event(reason).expect("cap event");
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(
        json.contains(FR_EVT_DISTILL_LOOP_CAP),
        "fr_event kind must surface in JSON"
    );
    let parsed: handshake_core::self_improve::scheduler::LoopCapEvent =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.kind, event.kind);
    assert_eq!(parsed.count_in_window, event.count_in_window);
    assert_eq!(parsed.cap, event.cap);
    assert_eq!(parsed.window_started_at_utc, event.window_started_at_utc);
    assert_eq!(parsed.next_eligible_at_utc, event.next_eligible_at_utc);
}

// ============================================================================
// SchedulerHistory bounded ring buffer
// ============================================================================

#[test]
fn mt156_scheduler_history_is_bounded_under_long_runs() {
    // Push 256 entries; internal cap is 128.
    let mut history = SchedulerHistory::new();
    for i in 0..256 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: Utc::now() - Duration::seconds(i as i64),
        });
    }
    assert_eq!(
        history.entries.len(),
        128,
        "history must be bounded so long-running loops don't leak memory"
    );
}

#[test]
fn mt156_scheduler_history_ring_drops_oldest_entries() {
    let mut history = SchedulerHistory::new();
    // Push 200; track oldest then assert it's no longer in the live buffer.
    let oldest_id = Uuid::now_v7();
    history.push(SchedulerHistoryEntry {
        iteration_id: oldest_id,
        scheduled_at_utc: Utc::now() - Duration::seconds(1_000),
    });
    for i in 1..200 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: Utc::now() - Duration::seconds(1_000 - i as i64),
        });
    }
    let still_has_oldest = history
        .entries
        .iter()
        .any(|e| e.iteration_id == oldest_id);
    assert!(!still_has_oldest, "ring buffer must drop the oldest entry");
}

// ============================================================================
// SchedulerHistoryEntry / ScheduleDecision / SkipReason / IterationBudget serde
// ============================================================================

#[test]
fn mt156_schedule_decision_serde_round_trip_schedule_variant() {
    let decision = ScheduleDecision::Schedule;
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("\"decision\":\"schedule\""));
    let parsed: ScheduleDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}

#[test]
fn mt156_schedule_decision_serde_round_trip_skip_budget_exhausted() {
    let reason = SkipReason::BudgetExhausted {
        count_in_window: 25,
        cap: 25,
        window_started_at_utc: Utc::now() - Duration::hours(12),
        next_eligible_at_utc: Utc::now() + Duration::hours(12),
    };
    let decision = ScheduleDecision::Skip { reason };
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("\"decision\":\"skip\""));
    assert!(json.contains("budget_exhausted"));
    let parsed: ScheduleDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}

#[test]
fn mt156_schedule_decision_serde_round_trip_skip_goodhart_pause() {
    let decision = ScheduleDecision::Skip {
        reason: SkipReason::GoodhartPause {
            receipt: goodhart_receipt(),
        },
    };
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("goodhart_pause"));
    let parsed: ScheduleDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}

#[test]
fn mt156_schedule_decision_serde_round_trip_skip_operator_pause() {
    let decision = ScheduleDecision::Skip {
        reason: SkipReason::OperatorPause {
            rationale: "operator manual stop".to_string(),
        },
    };
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("operator_pause"));
    let parsed: ScheduleDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}

#[test]
fn mt156_schedule_decision_serde_round_trip_skip_pending_promotion_review() {
    let iter = Uuid::now_v7();
    let decision = ScheduleDecision::Skip {
        reason: SkipReason::PendingPromotionReview {
            iteration_id: iter,
            ticket: pending_ticket(iter),
        },
    };
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("pending_promotion_review"));
    let parsed: ScheduleDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}

#[test]
fn mt156_iteration_budget_serde_round_trip() {
    let budget = IterationBudget {
        max_iterations_per_24h: 10,
        rolling_window_seconds: 3600,
    };
    let json = serde_json::to_string(&budget).expect("serialize");
    let parsed: IterationBudget = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(budget, parsed);
}

#[test]
fn mt156_scheduler_history_serde_round_trip() {
    let mut history = SchedulerHistory::new();
    for _ in 0..3 {
        history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: Utc::now(),
        });
    }
    let json = serde_json::to_string(&history).expect("serialize");
    let parsed: SchedulerHistory = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.entries.len(), 3);
}

// ============================================================================
// Determinism + idempotency
// ============================================================================

#[test]
fn mt156_repeated_calls_with_same_inputs_yield_same_decision_discriminant() {
    // Determinism: pure function over (history, now, paused, sentinel,
    // pending) inputs.
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let now = Utc::now();
    let a = scheduler.try_schedule_next(&history, now, None, None, None);
    let b = scheduler.try_schedule_next(&history, now, None, None, None);
    assert_eq!(a, b);
}

#[test]
fn mt156_repeated_skip_calls_with_same_inputs_yield_same_skip_reason() {
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let a = scheduler.try_schedule_next(&history, now, None, None, None);
    let b = scheduler.try_schedule_next(&history, now, None, None, None);
    // BudgetExhausted reasons should be structurally identical given fixed
    // `now` (window_started_at_utc is derived from history not wall time).
    assert_eq!(a, b);
}

#[test]
fn mt156_try_schedule_next_does_not_mutate_history() {
    // History is &-borrowed; ensure call does not increment its length.
    let scheduler = LoopScheduler::with_defaults();
    let now = Utc::now();
    let history = history_at(now, 10, 60);
    let pre_len = history.entries.len();
    let _ = scheduler.try_schedule_next(&history, now, None, None, None);
    let _ = scheduler.try_schedule_next(&history, now, None, None, None);
    assert_eq!(
        history.entries.len(),
        pre_len,
        "scheduler must not mutate history (driver records the entry post-Schedule)"
    );
}

// ============================================================================
// Concurrency — multiple readers under Arc-shared history
// ============================================================================

#[test]
fn mt156_concurrent_schedulers_observe_consistent_counts_under_arc_history() {
    // Two threads reading the same Arc<history> see the same decision
    // discriminant (scheduler is Copy + history is &-borrowed; no
    // interior mutation).
    let now = Utc::now();
    let history = Arc::new(history_at(now, 25, 60));
    let scheduler = LoopScheduler::with_defaults();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let h = Arc::clone(&history);
        handles.push(thread::spawn(move || {
            scheduler.try_schedule_next(&h, now, None, None, None)
        }));
    }
    let mut decisions = Vec::new();
    for h in handles {
        decisions.push(h.join().expect("join"));
    }
    // All eight must be Skip BudgetExhausted with count_in_window == 25.
    for d in &decisions {
        match d.skip_reason() {
            Some(SkipReason::BudgetExhausted { count_in_window, .. }) => {
                assert_eq!(*count_in_window, 25);
            }
            other => panic!("expected BudgetExhausted; got {:?}", other),
        }
    }
}

// ============================================================================
// Custom budget overrides
// ============================================================================

#[test]
fn mt156_custom_budget_smaller_cap_rejects_earlier() {
    let scheduler = LoopScheduler::new(IterationBudget {
        max_iterations_per_24h: 3,
        rolling_window_seconds: 86_400,
    });
    let now = Utc::now();
    let history = history_at(now, 3, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    match decision.skip_reason() {
        Some(SkipReason::BudgetExhausted { cap, count_in_window, .. }) => {
            assert_eq!(*cap, 3);
            assert_eq!(*count_in_window, 3);
        }
        other => panic!("expected BudgetExhausted at cap 3; got {:?}", other),
    }
}

#[test]
fn mt156_custom_budget_smaller_window_falls_out_faster() {
    // 1h rolling window; entries 2h old should fall out.
    let scheduler = LoopScheduler::new(IterationBudget {
        max_iterations_per_24h: 1,
        rolling_window_seconds: 3600,
    });
    let now = Utc::now();
    let history = history_at(now, 1, 7_200);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "1h window with 2h-old entry must fall out"
    );
}

#[test]
fn mt156_custom_budget_larger_cap_allows_more_iterations() {
    let scheduler = LoopScheduler::new(IterationBudget {
        max_iterations_per_24h: 100,
        rolling_window_seconds: 86_400,
    });
    let now = Utc::now();
    let history = history_at(now, 25, 60);
    let decision = scheduler.try_schedule_next(&history, now, None, None, None);
    assert!(
        decision.is_schedule(),
        "25 entries at cap 100 must allow schedule"
    );
}

// ============================================================================
// LoopIpcState integration: scheduler driver consults IPC state
// ============================================================================

#[test]
fn mt156_ipc_pause_state_drives_scheduler_via_pause_reason_string() {
    // Simulates the driver wiring: read ipc.status().pause_reason, if
    // Operator pass rationale to scheduler.
    let scheduler = LoopScheduler::with_defaults();
    let history = SchedulerHistory::new();
    let ipc = LoopIpcState::new(25);
    ipc.pause("ad-hoc safety hold".to_string()).expect("pause");
    let snap = ipc.status();
    let rationale = match snap.pause_reason {
        Some(PauseReason::Operator { ref rationale }) => rationale.as_str(),
        other => panic!("expected Operator pause; got {:?}", other),
    };
    let decision = scheduler.try_schedule_next(&history, Utc::now(), Some(rationale), None, None);
    match decision.skip_reason() {
        Some(SkipReason::OperatorPause { rationale: r }) => {
            assert_eq!(r, "ad-hoc safety hold");
        }
        other => panic!("expected OperatorPause; got {:?}", other),
    }
}

#[test]
fn mt156_already_paused_ipc_error_does_not_change_scheduler_outcome() {
    // Defense-in-depth: if the driver mishandles a double-pause and
    // surfaces an error, the scheduler still has the correct state (the
    // first pause stuck so subsequent driver poll sees paused == true).
    let ipc = LoopIpcState::new(25);
    ipc.pause("first".to_string()).expect("first pause ok");
    let err = ipc.pause("second".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::AlreadyPaused));
    let snap = ipc.status();
    assert!(snap.paused, "AlreadyPaused error must NOT clear paused flag");
}

// ============================================================================
// Sanity: skip_reason() / is_schedule() helpers
// ============================================================================

#[test]
fn mt156_schedule_decision_helpers_classify_correctly() {
    let s = ScheduleDecision::Schedule;
    assert!(s.is_schedule());
    assert!(s.skip_reason().is_none());

    let k = ScheduleDecision::Skip {
        reason: SkipReason::OperatorPause {
            rationale: "x".to_string(),
        },
    };
    assert!(!k.is_schedule());
    assert!(k.skip_reason().is_some());
}
