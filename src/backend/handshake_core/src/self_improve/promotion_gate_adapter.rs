//! MT-154: PromotionGate integration for the self-improvement loop.
//!
//! Wires the loop's AcceptRejectDecision through the KERNEL-001
//! PromotionGate contract. The adapter is asynchronous (submit + poll)
//! because operator review may take hours/days.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::editable_surface::EditableSurfaceSnapshot;
use super::evaluator::EvalResult;
use super::goodhart_sentinel::SentinelDecision;
use super::iteration::{LoopTarget, OperatorId};
use super::promotion_floor::PromotionDecision;

/// Evidence bundle submitted to the gate. The reviewer sees the baseline
/// + proposed surface, the eval, the floor decision, and the sentinel
/// decision in one place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionRequest {
    pub iteration_id: Uuid,
    pub target: LoopTarget,
    pub baseline_snapshot: EditableSurfaceSnapshot,
    pub proposed_snapshot: EditableSurfaceSnapshot,
    pub eval_result: EvalResult,
    pub floor_decision: PromotionDecision,
    pub sentinel_decision: SentinelDecision,
    pub justification_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionTicket {
    pub ticket_id: Uuid,
    pub iteration_id: Uuid,
    pub submitted_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionApproval {
    pub approved_by: OperatorId,
    pub approved_at_utc: DateTime<Utc>,
    pub signoff_evidence_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionRejection {
    pub rejected_by: OperatorId,
    pub rejected_at_utc: DateTime<Utc>,
    pub rejection_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum PromotionStatus {
    Pending { submitted_at_utc: DateTime<Utc> },
    Approved { approval: PromotionApproval },
    Rejected { rejection: PromotionRejection },
}

impl PromotionStatus {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

/// Async gate trait the loop adapter consumes. Production wires this to
/// the KERNEL-001 PromotionGate; tests use an in-memory mock.
pub trait PromotionGateSubmitter {
    fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError>;
    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError>;
}

/// Adapter from LoopCore -> PromotionGate.
pub struct LoopPromotionGate<'a> {
    pub gate: &'a dyn PromotionGateSubmitter,
}

impl<'a> LoopPromotionGate<'a> {
    pub fn new(gate: &'a dyn PromotionGateSubmitter) -> Self {
        Self { gate }
    }

    pub fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
        // MT-154 remediation: the gate's ticket MUST stay bound to the
        // iteration it was submitted for. A gate that returns a ticket
        // carrying a different iteration_id (buggy or malicious) would let
        // one iteration's approval evidence be inherited by another, breaking
        // the audit chain operator-review depends on. Reject the mismatch
        // loudly instead of trusting the returned ticket verbatim.
        let expected_iteration_id = request.iteration_id;
        let ticket = self.gate.submit(request)?;
        if ticket.iteration_id != expected_iteration_id {
            return Err(GateError::Io {
                message: format!(
                    "gate returned mismatched iteration_id (expected {expected_iteration_id}, got {})",
                    ticket.iteration_id
                ),
            });
        }
        Ok(ticket)
    }

    pub fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        self.gate.poll(ticket)
    }

    /// Helper: callers (the loop) use this before invoking
    /// `apply_proposal` on the editable surface so a Pending or Rejected
    /// status returns a typed error.
    pub fn require_approved(
        &self,
        ticket: &PromotionTicket,
    ) -> Result<PromotionApproval, GateError> {
        match self.poll(ticket)? {
            PromotionStatus::Approved { approval } => Ok(approval),
            PromotionStatus::Pending { .. } => Err(GateError::ReviewPending),
            PromotionStatus::Rejected { rejection } => Err(GateError::ReviewRejected {
                rationale: rejection.rejection_reason,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GateError {
    #[error("promotion gate ticket not found")]
    UnknownTicket,
    #[error("promotion review pending — apply blocked")]
    ReviewPending,
    #[error("promotion review rejected: {rationale}")]
    ReviewRejected { rationale: String },
    #[error("promotion gate I/O error: {message}")]
    Io { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_improve::evaluator::SplitMetrics;
    use crate::self_improve::promotion_floor::MetricDelta;
    use std::collections::HashMap;
    use std::sync::Mutex;

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

        fn reject(&self, ticket_id: Uuid, op: &str, reason: &str) {
            let rej = PromotionRejection {
                rejected_by: OperatorId::new(op),
                rejected_at_utc: Utc::now(),
                rejection_reason: reason.to_string(),
            };
            self.tickets
                .lock()
                .unwrap()
                .insert(ticket_id, TicketState::Rejected(rej));
        }
    }

    impl PromotionGateSubmitter for MockGate {
        fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
            // A correct gate binds the issued ticket to the request's
            // iteration_id (MT-154 finding #2).
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

    fn sample_request() -> PromotionRequest {
        let snapshot = EditableSurfaceSnapshot::RetrievalPolicy {
            task_type: crate::memory::TaskType::ValidatorHbrTestPacket,
            parameter: super::super::editable_surface::PolicyParameter::TopK,
            before_value: 6,
            after_value: 8,
        };
        PromotionRequest {
            iteration_id: Uuid::now_v7(),
            target: LoopTarget::RetrievalPolicyParams {
                task_type: crate::memory::TaskType::ValidatorHbrTestPacket,
                parameter: super::super::editable_surface::PolicyParameter::TopK,
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
            justification_text: "test".to_string(),
        }
    }

    #[test]
    fn submit_returns_typed_ticket_then_pending() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        let status = adapter.poll(&ticket).unwrap();
        assert!(status.is_pending());
    }

    #[test]
    fn require_approved_returns_err_pending_then_ok_after_approval() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        let err = adapter.require_approved(&ticket).unwrap_err();
        assert!(matches!(err, GateError::ReviewPending));

        gate.approve(ticket.ticket_id, "operator-1");
        let approval = adapter.require_approved(&ticket).unwrap();
        assert_eq!(approval.approved_by.as_str(), "operator-1");
        assert_eq!(approval.signoff_evidence_id.get_version_num(), 7);
    }

    #[test]
    fn require_approved_returns_rejected_typed() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        gate.reject(ticket.ticket_id, "operator-1", "bad change");
        let err = adapter.require_approved(&ticket).unwrap_err();
        match err {
            GateError::ReviewRejected { rationale } => assert_eq!(rationale, "bad change"),
            _ => panic!("expected rejected variant"),
        }
    }

    #[test]
    fn unknown_ticket_returns_typed_error() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let fake_ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: Uuid::now_v7(),
            submitted_at_utc: Utc::now(),
        };
        let err = adapter.require_approved(&fake_ticket).unwrap_err();
        assert!(matches!(err, GateError::UnknownTicket));
    }

    /// A buggy/malicious gate that detaches the issued ticket from the
    /// request's iteration_id. MT-154 finding #1: `LoopPromotionGate::submit`
    /// must reject this rather than propagate a ticket whose approval
    /// evidence could be inherited by a different iteration.
    struct MisbindingGate;
    impl PromotionGateSubmitter for MisbindingGate {
        fn submit(&self, _request: PromotionRequest) -> Result<PromotionTicket, GateError> {
            Ok(PromotionTicket {
                ticket_id: Uuid::now_v7(),
                iteration_id: Uuid::now_v7(), // detached from request.iteration_id
                submitted_at_utc: Utc::now(),
            })
        }
        fn poll(&self, _ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
            unreachable!("submit must fail before poll is reached")
        }
    }

    #[test]
    fn submit_rejects_gate_returned_mismatched_iteration_id() {
        let gate = MisbindingGate;
        let adapter = LoopPromotionGate::new(&gate);
        let err = adapter.submit(sample_request()).unwrap_err();
        match err {
            GateError::Io { message } => {
                assert!(
                    message.contains("mismatched iteration_id"),
                    "expected mismatch message; got: {message}"
                );
            }
            other => panic!("expected GateError::Io for mismatched iteration_id; got {other:?}"),
        }
    }
}
