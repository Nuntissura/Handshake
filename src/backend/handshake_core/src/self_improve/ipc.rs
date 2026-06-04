//! MT-155: Tauri IPC state + commands for the self-improvement loop.
//!
//! The IPC surface exposes:
//! - `status` — read-only projection
//! - `pause` / `unpause`
//! - `list_pending_reviews`
//! - `approve_promotion` / `reject_promotion`
//!
//! All mutating commands route through `KernelActionCatalogV1` action ids
//! and emit FR-EVT-* flight recorder events.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::goodhart_sentinel::PauseReason;
use super::iteration::OperatorId;
use super::promotion_gate_adapter::{
    GateError, PromotionApproval, PromotionGateSubmitter, PromotionRejection, PromotionRequest,
    PromotionStatus, PromotionTicket,
};

pub const FR_EVT_LOOP_PAUSE: &str = "FR-EVT-LOOP-PAUSE";
pub const FR_EVT_LOOP_UNPAUSE: &str = "FR-EVT-LOOP-UNPAUSE";
pub const FR_EVT_PROMOTION_DECISION: &str = "FR-EVT-PROMOTION-DECISION";

pub const LOOP_PAUSE_ACTION_ID: &str = "kernel.self_improve.pause";
pub const LOOP_UNPAUSE_ACTION_ID: &str = "kernel.self_improve.unpause";
pub const LOOP_APPROVE_PROMOTION_ACTION_ID: &str = "kernel.self_improve.approve_promotion";
pub const LOOP_REJECT_PROMOTION_ACTION_ID: &str = "kernel.self_improve.reject_promotion";

/// Snapshot returned to the operator panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopStatusSnapshot {
    pub loop_state: LoopState,
    pub last_iteration_at_utc: Option<DateTime<Utc>>,
    pub last_iteration_id: Option<Uuid>,
    pub paused: bool,
    pub pause_reason: Option<PauseReason>,
    pub pending_review_count: u32,
    pub iteration_budget_remaining_24h: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopState {
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauseReceipt {
    pub receipt_id: Uuid,
    pub paused_at_utc: DateTime<Utc>,
    pub reason: PauseReason,
    pub fr_event_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnpauseReceipt {
    pub receipt_id: Uuid,
    pub unpaused_at_utc: DateTime<Utc>,
    pub rationale: String,
    pub fr_event_kind: String,
}

/// In-memory Tauri-managed state for the loop. Production wires to the
/// kernel event ledger via KernelActionCatalogV1; tests use this in-memory
/// state directly.
#[derive(Debug, Default)]
pub struct LoopIpcState {
    inner: RwLock<LoopIpcInner>,
}

#[derive(Debug, Default)]
struct LoopIpcInner {
    state: Option<LoopState>,
    last_iteration_at_utc: Option<DateTime<Utc>>,
    last_iteration_id: Option<Uuid>,
    paused: bool,
    pause_reason: Option<PauseReason>,
    pending_reviews: HashMap<Uuid, PromotionRequest>,
    pending_tickets: HashMap<Uuid, PromotionTicket>,
    iteration_budget_remaining_24h: u32,
}

impl LoopIpcState {
    pub fn new(initial_budget: u32) -> Self {
        Self {
            inner: RwLock::new(LoopIpcInner {
                state: Some(LoopState::Idle),
                iteration_budget_remaining_24h: initial_budget,
                ..LoopIpcInner::default()
            }),
        }
    }

    /// status() — pure projection.
    pub fn status(&self) -> LoopStatusSnapshot {
        let guard = self.inner.read().unwrap();
        LoopStatusSnapshot {
            loop_state: guard.state.unwrap_or(LoopState::Idle),
            last_iteration_at_utc: guard.last_iteration_at_utc,
            last_iteration_id: guard.last_iteration_id,
            paused: guard.paused,
            pause_reason: guard.pause_reason.clone(),
            pending_review_count: guard.pending_reviews.len() as u32,
            iteration_budget_remaining_24h: guard.iteration_budget_remaining_24h,
        }
    }

    /// pause() — manual pause by operator.
    pub fn pause(&self, rationale: String) -> Result<PauseReceipt, LoopIpcError> {
        let mut guard = self.inner.write().unwrap();
        if guard.paused {
            return Err(LoopIpcError::AlreadyPaused);
        }
        guard.paused = true;
        guard.pause_reason = Some(PauseReason::Operator {
            rationale: rationale.clone(),
        });
        guard.state = Some(LoopState::Paused);
        Ok(PauseReceipt {
            receipt_id: Uuid::now_v7(),
            paused_at_utc: Utc::now(),
            reason: PauseReason::Operator { rationale },
            fr_event_kind: FR_EVT_LOOP_PAUSE.to_string(),
        })
    }

    /// Goodhart-triggered pause. Used by the scheduler.
    pub fn pause_with_reason(&self, reason: PauseReason) -> Result<PauseReceipt, LoopIpcError> {
        let mut guard = self.inner.write().unwrap();
        if guard.paused {
            return Err(LoopIpcError::AlreadyPaused);
        }
        guard.paused = true;
        guard.pause_reason = Some(reason.clone());
        guard.state = Some(LoopState::Paused);
        Ok(PauseReceipt {
            receipt_id: Uuid::now_v7(),
            paused_at_utc: Utc::now(),
            reason,
            fr_event_kind: FR_EVT_LOOP_PAUSE.to_string(),
        })
    }

    /// unpause() — operator clears the pause.
    pub fn unpause(&self, rationale: String) -> Result<UnpauseReceipt, LoopIpcError> {
        if rationale.trim().is_empty() {
            return Err(LoopIpcError::EmptyRationale);
        }
        let mut guard = self.inner.write().unwrap();
        if !guard.paused {
            return Err(LoopIpcError::NotPaused);
        }
        guard.paused = false;
        guard.pause_reason = None;
        guard.state = Some(LoopState::Idle);
        Ok(UnpauseReceipt {
            receipt_id: Uuid::now_v7(),
            unpaused_at_utc: Utc::now(),
            rationale,
            fr_event_kind: FR_EVT_LOOP_UNPAUSE.to_string(),
        })
    }

    pub fn list_pending_reviews(&self, limit: u32) -> Vec<PromotionRequest> {
        let guard = self.inner.read().unwrap();
        guard
            .pending_reviews
            .values()
            .take(limit as usize)
            .cloned()
            .collect()
    }

    /// Register a request as pending review. Used by LoopCore + scheduler.
    pub fn submit_for_review(
        &self,
        request: PromotionRequest,
        ticket: PromotionTicket,
    ) -> Result<(), LoopIpcError> {
        // MT-155 finding #2: pending_reviews and pending_tickets MUST be
        // keyed on the same iteration_id. The previous code keyed the request
        // on request.iteration_id and the ticket on ticket.iteration_id; a
        // mismatched ticket desynchronized the maps so
        // ticket_for_iteration(request_iter) returned None while
        // pending_reviews still held the entry. Reject the mismatch and key
        // both maps on the canonical request iteration_id.
        if request.iteration_id != ticket.iteration_id {
            return Err(LoopIpcError::TicketIterationMismatch {
                request_iteration_id: request.iteration_id,
                ticket_iteration_id: ticket.iteration_id,
            });
        }
        let iteration_id = request.iteration_id;
        let mut guard = self.inner.write().unwrap();
        guard.pending_reviews.insert(iteration_id, request);
        guard.pending_tickets.insert(iteration_id, ticket);
        Ok(())
    }

    pub fn ticket_for_iteration(&self, iteration_id: Uuid) -> Option<PromotionTicket> {
        let guard = self.inner.read().unwrap();
        guard.pending_tickets.get(&iteration_id).cloned()
    }

    /// approve_promotion: route through the gate. Caller passes the gate
    /// submitter so this stays a thin orchestration layer.
    pub fn approve_promotion(
        &self,
        gate: Arc<dyn PromotionGateSubmitter>,
        iteration_id: Uuid,
        operator: OperatorId,
        rationale: String,
    ) -> Result<PromotionApproval, LoopIpcError> {
        if rationale.trim().is_empty() {
            return Err(LoopIpcError::EmptyRationale);
        }
        let ticket = self
            .ticket_for_iteration(iteration_id)
            .ok_or(LoopIpcError::UnknownIteration { iteration_id })?;
        // Mark approved in the gate. For mock testing, this is a no-op
        // routing; the gate itself records approval. We synthesize the
        // approval shape locally so the caller can render it.
        match gate.poll(&ticket).map_err(LoopIpcError::Gate)? {
            PromotionStatus::Approved { approval } => {
                // MT-155 finding #3: record the operator's approve-time
                // identity + rationale rather than discarding it. The gate's
                // approval is the authority; this audit line preserves the
                // operator-side justification for the trace/EventLedger.
                tracing::info!(
                    target: "self_improve.promotion",
                    %iteration_id,
                    operator = operator.as_str(),
                    rationale = rationale.as_str(),
                    "operator confirmed gate-approved promotion via IPC"
                );
                let mut guard = self.inner.write().unwrap();
                guard.pending_reviews.remove(&iteration_id);
                guard.pending_tickets.remove(&iteration_id);
                Ok(approval)
            }
            PromotionStatus::Pending { .. } => {
                // MT-155 finding #1 (FAIL): the gate has NOT approved yet.
                // The previous implementation synthesized a local approval
                // bearing this operator's id and a fresh signoff_evidence_id,
                // then cleared pending state — minting an approval the
                // downstream consumer could not distinguish from a real gate
                // decision and removing the iteration from review. That
                // defeated the operator-review-required invariant. Fail
                // closed and leave pending_reviews / pending_tickets intact so
                // the iteration stays under review until the gate decides.
                Err(LoopIpcError::Gate(GateError::ReviewPending))
            }
            PromotionStatus::Rejected { rejection } => {
                Err(LoopIpcError::Gate(GateError::ReviewRejected {
                    rationale: rejection.rejection_reason,
                }))
            }
        }
    }

    pub fn reject_promotion(
        &self,
        iteration_id: Uuid,
        operator: OperatorId,
        rationale: String,
    ) -> Result<PromotionRejection, LoopIpcError> {
        if rationale.trim().is_empty() {
            return Err(LoopIpcError::EmptyRationale);
        }
        let _ticket = self
            .ticket_for_iteration(iteration_id)
            .ok_or(LoopIpcError::UnknownIteration { iteration_id })?;
        let rejection = PromotionRejection {
            rejected_by: operator,
            rejected_at_utc: Utc::now(),
            rejection_reason: rationale,
        };
        let mut guard = self.inner.write().unwrap();
        guard.pending_reviews.remove(&iteration_id);
        guard.pending_tickets.remove(&iteration_id);
        Ok(rejection)
    }

    /// Record an iteration completion for the status projection.
    pub fn record_iteration_complete(&self, iteration_id: Uuid) {
        let mut guard = self.inner.write().unwrap();
        guard.last_iteration_at_utc = Some(Utc::now());
        guard.last_iteration_id = Some(iteration_id);
        guard.state = Some(LoopState::Running);
        guard.iteration_budget_remaining_24h =
            guard.iteration_budget_remaining_24h.saturating_sub(1);
    }

    pub fn restore_budget(&self, budget: u32) {
        let mut guard = self.inner.write().unwrap();
        guard.iteration_budget_remaining_24h = budget;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LoopIpcError {
    #[error("loop is already paused")]
    AlreadyPaused,
    #[error("loop is not paused; nothing to unpause")]
    NotPaused,
    #[error("operator rationale must not be empty")]
    EmptyRationale,
    #[error("unknown iteration id: {iteration_id}")]
    UnknownIteration { iteration_id: Uuid },
    #[error(
        "promotion ticket iteration_id {ticket_iteration_id} does not match request iteration_id {request_iteration_id}"
    )]
    TicketIterationMismatch {
        request_iteration_id: Uuid,
        ticket_iteration_id: Uuid,
    },
    #[error(transparent)]
    Gate(#[from] GateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_projects_idle_at_construction() {
        let state = LoopIpcState::new(25);
        let snap = state.status();
        assert_eq!(snap.loop_state, LoopState::Idle);
        assert!(!snap.paused);
        assert_eq!(snap.iteration_budget_remaining_24h, 25);
    }

    #[test]
    fn pause_then_already_paused_errors() {
        let state = LoopIpcState::new(25);
        let receipt = state.pause("manual".to_string()).unwrap();
        assert_eq!(receipt.fr_event_kind, FR_EVT_LOOP_PAUSE);
        assert!(state.pause("again".to_string()).is_err());
    }

    #[test]
    fn unpause_when_not_paused_errors() {
        let state = LoopIpcState::new(25);
        let err = state.unpause("ok".to_string()).unwrap_err();
        assert!(matches!(err, LoopIpcError::NotPaused));
    }

    #[test]
    fn unpause_clears_pause() {
        let state = LoopIpcState::new(25);
        state.pause("m".to_string()).unwrap();
        assert!(state.status().paused);
        let r = state.unpause("done reviewing".to_string()).unwrap();
        assert_eq!(r.fr_event_kind, FR_EVT_LOOP_UNPAUSE);
        assert!(!state.status().paused);
    }

    #[test]
    fn unpause_empty_rationale_errors() {
        let state = LoopIpcState::new(25);
        state.pause("m".to_string()).unwrap();
        let err = state.unpause("   ".to_string()).unwrap_err();
        assert!(matches!(err, LoopIpcError::EmptyRationale));
    }

    #[test]
    fn unknown_iteration_for_approve_returns_typed() {
        // For this test, we don't even build a gate — the lookup fails
        // before the gate is touched.
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
        let result = state.approve_promotion(
            Arc::new(StubGate),
            Uuid::now_v7(),
            OperatorId::new("op-1"),
            "ok".to_string(),
        );
        assert!(matches!(
            result.unwrap_err(),
            LoopIpcError::UnknownIteration { .. }
        ));
    }

    #[test]
    fn record_iteration_complete_decrements_budget() {
        let state = LoopIpcState::new(3);
        assert_eq!(state.status().iteration_budget_remaining_24h, 3);
        state.record_iteration_complete(Uuid::now_v7());
        assert_eq!(state.status().iteration_budget_remaining_24h, 2);
        state.record_iteration_complete(Uuid::now_v7());
        state.record_iteration_complete(Uuid::now_v7());
        state.record_iteration_complete(Uuid::now_v7()); // saturating
        assert_eq!(state.status().iteration_budget_remaining_24h, 0);
    }
}
