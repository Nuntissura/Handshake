//! MT-148: typed LoopIteration state machine.
//!
//! `LoopStage` is a closed enum so future contributors cannot add stages
//! without a typed code change. Stage advancement is total — every legal
//! transition is enumerated in [`LoopStage::next`]. Illegal transitions
//! return [`LoopIterationError::IllegalTransition`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::corpus::ValidatorVerdict;
use super::editable_surface::EditableSurfaceSnapshot;
use super::evaluator::EvalResult;

/// Opaque operator identifier used for AcceptRejectDecision signoff.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(pub String);

impl OperatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// LoopStage is the closed enum of every legal stage of one iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopStage {
    ChooseTarget,
    IsolateEditableSurface,
    RunInSandbox,
    ExecuteEval,
    AcceptReject,
    RecordAsMemory,
    Complete,
}

impl LoopStage {
    /// Return the next legal stage in the happy path. Used by
    /// [`LoopIteration::advance`] to enforce strict ordering.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::ChooseTarget => Some(Self::IsolateEditableSurface),
            Self::IsolateEditableSurface => Some(Self::RunInSandbox),
            Self::RunInSandbox => Some(Self::ExecuteEval),
            Self::ExecuteEval => Some(Self::AcceptReject),
            Self::AcceptReject => Some(Self::RecordAsMemory),
            Self::RecordAsMemory => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    /// Return the previous stage on the happy-path chain. Inverse of
    /// [`LoopStage::next`]. The reverse direction is used by replay
    /// inspection tooling (validators reviewing a persisted iteration that
    /// short-circuited may need to walk back one stage to confirm the
    /// surface snapshot they are inspecting). The state machine itself is
    /// forward-only: `previous` returns the prior stage as a value, it does
    /// not mutate the iteration.
    pub fn previous(self) -> Option<Self> {
        match self {
            Self::ChooseTarget => None,
            Self::IsolateEditableSurface => Some(Self::ChooseTarget),
            Self::RunInSandbox => Some(Self::IsolateEditableSurface),
            Self::ExecuteEval => Some(Self::RunInSandbox),
            Self::AcceptReject => Some(Self::ExecuteEval),
            Self::RecordAsMemory => Some(Self::AcceptReject),
            Self::Complete => Some(Self::RecordAsMemory),
        }
    }

    /// Test whether `target` is a legal transition from `self`. Used by
    /// the MT-148 transition-matrix fuzzing test to confirm every
    /// (from, to) pair is classified correctly. The only legal forward
    /// edge is `self.next() == Some(target)`. Self-edges and back-edges
    /// are illegal at runtime even though `previous` returns them for
    /// inspection.
    pub fn is_valid_transition(self, target: LoopStage) -> bool {
        self.next() == Some(target)
    }

    /// Total list of every stage. Used by exhaustive transition-matrix
    /// tests so a future contributor who adds a stage cannot skip the
    /// audit.
    pub const ALL: [LoopStage; 7] = [
        LoopStage::ChooseTarget,
        LoopStage::IsolateEditableSurface,
        LoopStage::RunInSandbox,
        LoopStage::ExecuteEval,
        LoopStage::AcceptReject,
        LoopStage::RecordAsMemory,
        LoopStage::Complete,
    ];

    /// Stable string identifier for logs and audits.
    pub fn slug(self) -> &'static str {
        match self {
            Self::ChooseTarget => "choose_target",
            Self::IsolateEditableSurface => "isolate_editable_surface",
            Self::RunInSandbox => "run_in_sandbox",
            Self::ExecuteEval => "execute_eval",
            Self::AcceptReject => "accept_reject",
            Self::RecordAsMemory => "record_as_memory",
            Self::Complete => "complete",
        }
    }

    /// True for the terminal stage; the iteration is done.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// LoopTarget defines what one iteration is allowed to modify. The enum is
/// closed — widening requires a typed code change reviewed by the operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LoopTarget {
    ModelManualCapsuleText {
        manual_section_id: String,
    },
    RetrievalPolicyParams {
        task_type: crate::memory::TaskType,
        parameter: PolicyParameterRef,
    },
}

/// Subset of editable retrieval policy parameters that the self-improvement
/// loop may propose changes for. Closed enum so the editable surface is
/// provably finite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyParameterRef {
    TopK,
    CapsuleBudgetBytes,
}

/// Operator decision recorded at the AcceptReject stage. Once recorded the
/// iteration may only advance to RecordAsMemory (on Accept) or short-circuit
/// to Complete (on Reject — surface mutation is not applied).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum AcceptRejectDecision {
    Accept {
        rationale: String,
        signed_off_by: OperatorId,
    },
    Reject {
        rationale: String,
    },
}

impl AcceptRejectDecision {
    pub fn is_accept(&self) -> bool {
        matches!(self, Self::Accept { .. })
    }
}

/// One pass through the self-improvement loop. Persisted; replayable from
/// its serialized state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopIteration {
    pub iteration_id: Uuid,
    pub iteration_number: u32,
    pub stage: LoopStage,
    pub target: Option<LoopTarget>,
    pub editable_surface_snapshot: Option<EditableSurfaceSnapshot>,
    pub sandbox_run_id: Option<Uuid>,
    pub eval_result: Option<EvalResult>,
    pub decision: Option<AcceptRejectDecision>,
    pub started_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
    pub completed_at_utc: Option<DateTime<Utc>>,
    /// For Reject paths: a memory entry id where the rejection rationale is
    /// recorded so future iterations do not re-propose the same change.
    pub rejection_memory_id: Option<Uuid>,
}

impl LoopIteration {
    /// Build a fresh iteration in stage `ChooseTarget`. iteration_id is v7
    /// (time-sortable) and `iteration_number` must be monotonically
    /// increasing per loop instance — the caller (LoopCore + scheduler)
    /// owns that invariant.
    pub fn new(iteration_number: u32) -> Self {
        let now = Utc::now();
        Self {
            iteration_id: Uuid::now_v7(),
            iteration_number,
            stage: LoopStage::ChooseTarget,
            target: None,
            editable_surface_snapshot: None,
            sandbox_run_id: None,
            eval_result: None,
            decision: None,
            started_at_utc: now,
            updated_at_utc: now,
            completed_at_utc: None,
            rejection_memory_id: None,
        }
    }

    /// Force-advance the stage. Returns the previous stage. Caller is
    /// responsible for filling in the per-stage outputs (target, snapshot,
    /// eval_result, decision) before calling advance.
    pub fn advance(&mut self) -> Result<LoopStage, LoopIterationError> {
        let prev = self.stage;
        let next = prev.next().ok_or(LoopIterationError::AlreadyComplete)?;
        self.stage = next;
        self.updated_at_utc = Utc::now();
        if next.is_terminal() {
            self.completed_at_utc = Some(self.updated_at_utc);
        }
        Ok(prev)
    }

    /// Attempt to transition to `target`. Returns
    /// [`LoopIterationError::IllegalTransition`] if `target` is not the
    /// immediate next stage; this is the typed-error path tests should use
    /// when they want to assert that out-of-order calls are rejected
    /// without panic.
    pub fn transition_to(&mut self, target: LoopStage) -> Result<LoopStage, LoopIterationError> {
        if !self.stage.is_valid_transition(target) {
            return Err(LoopIterationError::IllegalTransition {
                from: self.stage,
                to: target,
            });
        }
        self.advance()
    }

    /// Read the current stage. Idempotent: calling `current` twice does
    /// not mutate state and returns the same value both times. The
    /// MT-148 test contract calls this out explicitly so future
    /// refactors cannot accidentally couple reads to advancement.
    pub fn current(&self) -> LoopStage {
        self.stage
    }

    /// Short-circuit from AcceptReject directly to Complete on a Reject
    /// decision. The RecordAsMemory step still runs but is bounded: only
    /// the rejection rationale is recorded, not a surface mutation.
    pub fn short_circuit_to_complete(&mut self) -> Result<(), LoopIterationError> {
        if !matches!(
            self.stage,
            LoopStage::AcceptReject | LoopStage::RecordAsMemory
        ) {
            return Err(LoopIterationError::IllegalTransition {
                from: self.stage,
                to: LoopStage::Complete,
            });
        }
        self.stage = LoopStage::Complete;
        self.updated_at_utc = Utc::now();
        self.completed_at_utc = Some(self.updated_at_utc);
        Ok(())
    }

    /// Enforce that the iteration is currently at the expected stage; used
    /// by stage executors to reject out-of-order calls.
    pub fn assert_stage(&self, expected: LoopStage) -> Result<(), LoopIterationError> {
        if self.stage != expected {
            Err(LoopIterationError::IllegalTransition {
                from: self.stage,
                to: expected,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LoopIterationError {
    #[error("loop iteration cannot transition from {from:?} to {to:?}")]
    IllegalTransition { from: LoopStage, to: LoopStage },
    #[error("loop iteration already complete; no further transitions allowed")]
    AlreadyComplete,
    #[error("loop target is required at stage {stage:?} but is missing")]
    MissingTarget { stage: LoopStage },
    #[error("editable surface snapshot is required at stage {stage:?} but is missing")]
    MissingSnapshot { stage: LoopStage },
    #[error("eval result is required at stage {stage:?} but is missing")]
    MissingEval { stage: LoopStage },
    #[error("accept/reject decision is required at stage {stage:?} but is missing")]
    MissingDecision { stage: LoopStage },
    #[error("rejected proposals must not mutate the editable surface; apply skipped")]
    RejectedProposal,
    #[error("unknown verdict in iteration: {verdict:?}")]
    UnknownVerdict { verdict: ValidatorVerdict },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_stage_next_enumerates_full_chain() {
        let mut current = LoopStage::ChooseTarget;
        let chain = [
            LoopStage::IsolateEditableSurface,
            LoopStage::RunInSandbox,
            LoopStage::ExecuteEval,
            LoopStage::AcceptReject,
            LoopStage::RecordAsMemory,
            LoopStage::Complete,
        ];
        for expected in chain {
            let next = current.next().unwrap();
            assert_eq!(next, expected);
            current = next;
        }
        assert!(current.next().is_none());
        assert!(current.is_terminal());
    }

    #[test]
    fn loop_iteration_advances_in_order() {
        let mut it = LoopIteration::new(0);
        assert_eq!(it.stage, LoopStage::ChooseTarget);
        let stages = [
            LoopStage::IsolateEditableSurface,
            LoopStage::RunInSandbox,
            LoopStage::ExecuteEval,
            LoopStage::AcceptReject,
            LoopStage::RecordAsMemory,
            LoopStage::Complete,
        ];
        for expected in stages {
            it.advance().unwrap();
            assert_eq!(it.stage, expected);
        }
        assert!(it.completed_at_utc.is_some());
        assert!(it.advance().is_err());
    }

    #[test]
    fn loop_iteration_id_is_v7() {
        let it = LoopIteration::new(0);
        assert_eq!(it.iteration_id.get_version_num(), 7);
    }

    #[test]
    fn short_circuit_only_from_accept_reject_or_record() {
        let mut it = LoopIteration::new(0);
        // Move to ChooseTarget -> short-circuit fails
        assert!(it.short_circuit_to_complete().is_err());
        // Advance up to AcceptReject
        for _ in 0..4 {
            it.advance().unwrap();
        }
        assert_eq!(it.stage, LoopStage::AcceptReject);
        it.short_circuit_to_complete().unwrap();
        assert_eq!(it.stage, LoopStage::Complete);
    }
}
