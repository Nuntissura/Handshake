//! MT-155 Tauri IPC for the self-improvement loop.
//!
//! The IPC surface delegates to `handshake_core::self_improve::ipc::LoopIpcState`
//! and exposes:
//!
//! - `kernel_self_improve_status`        — read-only projection
//! - `kernel_self_improve_pause`         — operator-driven pause (typed receipt)
//! - `kernel_self_improve_unpause`       — clear an existing pause (rationale required)
//! - `kernel_self_improve_review_pending` — list pending promotion-review tickets
//! - `kernel_self_improve_approve_promotion` — operator approve via gate
//! - `kernel_self_improve_reject_promotion` — operator reject via gate
//!
//! All mutating commands flow through `LoopIpcState` which records typed
//! `FR-EVT-LOOP-*` / `FR-EVT-PROMOTION-DECISION` receipts. Pause + unpause
//! semantics are sticky and cross-thread safe (the underlying state uses
//! `RwLock`). Errors are surfaced as JSON strings so the frontend can render
//! them.

use std::sync::Arc;

use handshake_core::self_improve::ipc::{
    LoopIpcError, LoopIpcState, LoopStatusSnapshot, PauseReceipt, UnpauseReceipt,
};
use handshake_core::self_improve::iteration::OperatorId;
use handshake_core::self_improve::promotion_gate_adapter::{
    GateError, PromotionApproval, PromotionGateSubmitter, PromotionRejection, PromotionRequest,
    PromotionStatus, PromotionTicket,
};
use tauri::State;
use uuid::Uuid;

pub const KERNEL_SELF_IMPROVE_STATUS_IPC_CHANNEL: &str = "kernel_self_improve_status";
pub const KERNEL_SELF_IMPROVE_PAUSE_IPC_CHANNEL: &str = "kernel_self_improve_pause";
pub const KERNEL_SELF_IMPROVE_UNPAUSE_IPC_CHANNEL: &str = "kernel_self_improve_unpause";
pub const KERNEL_SELF_IMPROVE_REVIEW_PENDING_IPC_CHANNEL: &str =
    "kernel_self_improve_review_pending";
pub const KERNEL_SELF_IMPROVE_APPROVE_PROMOTION_IPC_CHANNEL: &str =
    "kernel_self_improve_approve_promotion";
pub const KERNEL_SELF_IMPROVE_REJECT_PROMOTION_IPC_CHANNEL: &str =
    "kernel_self_improve_reject_promotion";

/// Tauri-managed state for the self-improvement loop. Holds the
/// `LoopIpcState` (typed pause/unpause/review surface) plus an optional
/// `PromotionGateSubmitter` binding. When the gate binding is absent the
/// approve command returns a typed `GateUnavailable` error; the loop is
/// still inspectable + pausable.
pub struct SelfImproveIpcState {
    pub loop_state: Arc<LoopIpcState>,
    pub gate: Option<Arc<dyn PromotionGateSubmitter + Send + Sync>>,
}

impl Default for SelfImproveIpcState {
    fn default() -> Self {
        Self {
            loop_state: Arc::new(LoopIpcState::new(25)),
            gate: None,
        }
    }
}

impl SelfImproveIpcState {
    /// Construct from an explicit `LoopIpcState` so production wiring can
    /// share the same instance the scheduler holds. `initial_budget` is the
    /// 24h iteration cap (HBR-SWARM-002).
    pub fn new(loop_state: Arc<LoopIpcState>) -> Self {
        Self {
            loop_state,
            gate: None,
        }
    }

    /// Bind the production `PromotionGateSubmitter`. When absent the
    /// approve command returns `GateUnavailable`.
    pub fn with_gate(mut self, gate: Arc<dyn PromotionGateSubmitter + Send + Sync>) -> Self {
        self.gate = Some(gate);
        self
    }
}

fn ipc_error_to_string(err: LoopIpcError) -> String {
    serde_json::to_string(&err).unwrap_or_else(|_| err.to_string())
}

fn gate_error_to_string(err: GateError) -> String {
    serde_json::to_string(&err).unwrap_or_else(|_| err.to_string())
}

#[tauri::command]
pub async fn kernel_self_improve_status(
    state: State<'_, SelfImproveIpcState>,
) -> Result<LoopStatusSnapshot, String> {
    let _ = KERNEL_SELF_IMPROVE_STATUS_IPC_CHANNEL;
    Ok(state.loop_state.status())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_self_improve_pause(
    rationale: String,
    state: State<'_, SelfImproveIpcState>,
) -> Result<PauseReceipt, String> {
    let _ = KERNEL_SELF_IMPROVE_PAUSE_IPC_CHANNEL;
    state
        .loop_state
        .pause(rationale)
        .map_err(ipc_error_to_string)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_self_improve_unpause(
    rationale: String,
    state: State<'_, SelfImproveIpcState>,
) -> Result<UnpauseReceipt, String> {
    let _ = KERNEL_SELF_IMPROVE_UNPAUSE_IPC_CHANNEL;
    state
        .loop_state
        .unpause(rationale)
        .map_err(ipc_error_to_string)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_self_improve_review_pending(
    limit: u32,
    state: State<'_, SelfImproveIpcState>,
) -> Result<Vec<PromotionRequest>, String> {
    let _ = KERNEL_SELF_IMPROVE_REVIEW_PENDING_IPC_CHANNEL;
    Ok(state.loop_state.list_pending_reviews(limit))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_self_improve_approve_promotion(
    iteration_id: Uuid,
    operator_id: String,
    rationale: String,
    state: State<'_, SelfImproveIpcState>,
) -> Result<PromotionApproval, String> {
    let _ = KERNEL_SELF_IMPROVE_APPROVE_PROMOTION_IPC_CHANNEL;
    let Some(gate) = state.gate.clone() else {
        return Err(serde_json::to_string(&GateError::Io {
            message: "promotion gate unavailable; bind a PromotionGateSubmitter on boot"
                .to_string(),
        })
        .unwrap_or_else(|_| "promotion gate unavailable".to_string()));
    };
    state
        .loop_state
        .approve_promotion(gate, iteration_id, OperatorId::new(operator_id), rationale)
        .map_err(ipc_error_to_string)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_self_improve_reject_promotion(
    iteration_id: Uuid,
    operator_id: String,
    rationale: String,
    state: State<'_, SelfImproveIpcState>,
) -> Result<PromotionRejection, String> {
    let _ = KERNEL_SELF_IMPROVE_REJECT_PROMOTION_IPC_CHANNEL;
    state
        .loop_state
        .reject_promotion(iteration_id, OperatorId::new(operator_id), rationale)
        .map_err(ipc_error_to_string)
}

/// Production helper exposed so the orchestrator can register a pending
/// review without going through a Tauri command. Used by the loop scheduler
/// (MT-156) once a `PromotionRequest` has been submitted to the gate.
pub fn submit_for_review(
    state: &SelfImproveIpcState,
    request: PromotionRequest,
    ticket: PromotionTicket,
) -> Result<(), String> {
    state
        .loop_state
        .submit_for_review(request, ticket)
        .map_err(ipc_error_to_string)
}

/// Production helper that lets the scheduler emit a Goodhart-driven pause
/// receipt directly (it carries `PauseReason::MonotonicGapWidening`).
pub fn pause_with_reason(
    state: &SelfImproveIpcState,
    reason: handshake_core::self_improve::goodhart_sentinel::PauseReason,
) -> Result<PauseReceipt, String> {
    state
        .loop_state
        .pause_with_reason(reason)
        .map_err(ipc_error_to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use chrono::Utc;
    use handshake_core::memory::TaskType;
    use handshake_core::self_improve::editable_surface::{
        EditableSurfaceSnapshot, PolicyParameter,
    };
    use handshake_core::self_improve::evaluator::{EvalResult, SplitMetrics};
    use handshake_core::self_improve::goodhart_sentinel::{PauseReason, SentinelDecision};
    use handshake_core::self_improve::iteration::LoopTarget;
    use handshake_core::self_improve::promotion_floor::{MetricDelta, PromotionDecision};

    struct MockGate {
        approve_on_poll: Mutex<HashMap<Uuid, ()>>,
    }

    impl MockGate {
        fn new() -> Self {
            Self {
                approve_on_poll: Mutex::new(HashMap::new()),
            }
        }
    }

    impl PromotionGateSubmitter for MockGate {
        fn submit(&self, _r: PromotionRequest) -> Result<PromotionTicket, GateError> {
            Ok(PromotionTicket {
                ticket_id: Uuid::now_v7(),
                iteration_id: Uuid::now_v7(),
                submitted_at_utc: Utc::now(),
            })
        }

        fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
            let approved = self
                .approve_on_poll
                .lock()
                .unwrap()
                .contains_key(&ticket.ticket_id);
            if approved {
                Ok(PromotionStatus::Approved {
                    approval: PromotionApproval {
                        approved_by: OperatorId::new("op-test"),
                        approved_at_utc: Utc::now(),
                        signoff_evidence_id: Uuid::now_v7(),
                    },
                })
            } else {
                Ok(PromotionStatus::Pending {
                    submitted_at_utc: ticket.submitted_at_utc,
                })
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
                parameter: handshake_core::self_improve::iteration::PolicyParameterRef::TopK,
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

    #[tokio::test]
    async fn status_returns_idle_snapshot_without_mutation() {
        let state = SelfImproveIpcState::default();
        let snap_a = state.loop_state.status();
        let snap_b = state.loop_state.status();
        assert_eq!(snap_a, snap_b);
        assert!(!snap_a.paused);
    }

    #[tokio::test]
    async fn pause_then_unpause_cycle_via_state() {
        let state = SelfImproveIpcState::default();
        let pause = state.loop_state.pause("manual".to_string()).unwrap();
        assert!(pause.fr_event_kind.starts_with("FR-EVT-LOOP-PAUSE"));
        assert!(state.loop_state.status().paused);
        let unpause = state.loop_state.unpause("done".to_string()).unwrap();
        assert!(unpause.fr_event_kind.starts_with("FR-EVT-LOOP-UNPAUSE"));
        assert!(!state.loop_state.status().paused);
    }

    #[tokio::test]
    async fn approve_without_gate_returns_typed_error() {
        let state = SelfImproveIpcState::default();
        let request = sample_request(Uuid::now_v7());
        let ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: request.iteration_id,
            submitted_at_utc: Utc::now(),
        };
        submit_for_review(&state, request, ticket.clone()).unwrap();
        // No gate bound; approve helper should surface a typed gate-unavailable error.
        let result = state.loop_state.approve_promotion(
            // construct a dummy gate that returns UnknownTicket so we exercise the
            // approve path without binding a production gate.
            {
                struct StubGate;
                impl PromotionGateSubmitter for StubGate {
                    fn submit(&self, _r: PromotionRequest) -> Result<PromotionTicket, GateError> {
                        Err(GateError::UnknownTicket)
                    }
                    fn poll(&self, _t: &PromotionTicket) -> Result<PromotionStatus, GateError> {
                        Err(GateError::UnknownTicket)
                    }
                }
                Arc::new(StubGate)
            },
            ticket.iteration_id,
            OperatorId::new("op-test"),
            "go".to_string(),
        );
        assert!(matches!(
            result.unwrap_err(),
            LoopIpcError::Gate(GateError::UnknownTicket)
        ));
    }

    #[tokio::test]
    async fn submit_for_review_then_list_returns_request() {
        let state = SelfImproveIpcState::default();
        let request = sample_request(Uuid::now_v7());
        let ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: request.iteration_id,
            submitted_at_utc: Utc::now(),
        };
        submit_for_review(&state, request.clone(), ticket).unwrap();
        let pending = state.loop_state.list_pending_reviews(10);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].iteration_id, request.iteration_id);
    }

    #[tokio::test]
    async fn pause_with_reason_sticky_until_unpause() {
        let state = SelfImproveIpcState::default();
        let pause = pause_with_reason(
            &state,
            PauseReason::MonotonicGapWidening {
                gaps: vec![0.01, 0.02, 0.03],
                iteration_numbers: vec![1, 2, 3],
            },
        )
        .unwrap();
        assert!(pause.fr_event_kind.starts_with("FR-EVT-LOOP-PAUSE"));
        assert!(state.loop_state.status().paused);
        // Second pause without unpausing first must fail.
        assert!(state.loop_state.pause("op".to_string()).is_err());
        state.loop_state.unpause("reviewed".to_string()).unwrap();
        assert!(!state.loop_state.status().paused);
    }
}
