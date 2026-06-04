//! MT-148: LoopCore — the engine that walks one [`LoopIteration`] through
//! every stage.
//!
//! The driver is engine-only: it does not call an LLM. Each stage executor
//! is injected as a trait so test runs are deterministic.
//!
//! Also exports the [`SelfImprovementLoop`] holder type which owns the
//! durable per-instance loop state across iterations:
//! - current in-flight [`LoopIteration`]
//! - bounded history of completed iterations
//! - loop iteration counter (slot — bound enforced by MT-156 scheduler)
//! - pending review queue (slot — populated by MT-154 promotion gate)
//! - per-target metric snapshot
//! - pause_reason (slot — populated by MT-153 Goodhart sentinel)
//! - last review token (slot — populated by MT-154 promotion gate)
//!
//! The slots above are declared and gated here so downstream MTs can wire
//! their concrete logic without re-shaping the holder.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::corpus::{CorpusSplit, KeyProvider};
use super::editable_surface::{EditableSurfaceProvider, EditableSurfaceSnapshot, SurfaceProposal};
use super::evaluator::EvalResult;
use super::iteration::{
    AcceptRejectDecision, LoopIteration, LoopIterationError, LoopStage, LoopTarget,
};

/// Sandbox interface the loop core consumes. Different implementations may
/// wrap Cluster B `SandboxAdapter` for real isolation, or use an in-process
/// stub for unit tests.
pub trait LoopSandbox {
    /// Run the trial inside a sandbox using `snapshot.after` as the
    /// override on top of the baseline world. Returns a sandbox run id so
    /// the iteration can cite the sandbox in audits.
    fn run(&self, snapshot: &EditableSurfaceSnapshot)
        -> Result<SandboxRunResult, LoopSandboxError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRunResult {
    pub sandbox_run_id: Uuid,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("loop sandbox error: {message}")]
pub struct LoopSandboxError {
    pub message: String,
}

impl LoopSandboxError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Evaluator interface — runs the validator first-pass eval against the
/// HBR test-packet corpus split given a candidate snapshot.
pub trait Evaluator {
    fn evaluate(
        &self,
        split: &CorpusSplit,
        key_provider: &dyn KeyProvider,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<EvalResult, super::evaluator::EvalError>;
}

/// Operator gate decides whether a proposal is accepted, rejected, or held
/// for review. Wired in MT-154 to the KERNEL-001 PromotionGate adapter.
pub trait OperatorGate {
    fn decide(
        &self,
        iteration: &LoopIteration,
        eval: &EvalResult,
    ) -> Result<AcceptRejectDecision, OperatorGateError>;
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("operator gate error: {message}")]
pub struct OperatorGateError {
    pub message: String,
}

impl OperatorGateError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// IterationRecorder persists the completed iteration so future iterations
/// can read prior decisions (rejected targets are skipped, accepted
/// targets become baseline). Wires to FEMS via CapsuleRecorder in
/// production.
pub trait IterationRecorder {
    fn record(&self, iteration: &LoopIteration) -> Result<Uuid, IterationRecorderError>;
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("iteration recorder error: {message}")]
pub struct IterationRecorderError {
    pub message: String,
}

impl IterationRecorderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// TargetPicker selects the next [`LoopTarget`] from the project state +
/// prior iterations. Picker is deterministic given fixed inputs so the
/// loop is replayable.
pub trait TargetPicker {
    fn pick(&self, prior_iterations: &[LoopIteration]) -> Result<LoopTarget, TargetPickerError>;

    /// Propose the concrete change for the picked target. Separate hook
    /// so the picker can stay engine-only while the proposer wires to a
    /// proposal generator (possibly LLM-backed) elsewhere.
    fn propose(&self, target: &LoopTarget) -> Result<SurfaceProposal, TargetPickerError>;
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("target picker error: {message}")]
pub struct TargetPickerError {
    pub message: String,
}

impl TargetPickerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// One-iteration stage executor split, exposed so each stage is
/// independently testable + swappable.
pub trait LoopStageExecutor {
    fn execute_choose_target(
        &self,
        iteration: &mut LoopIteration,
        prior: &[LoopIteration],
        picker: &dyn TargetPicker,
    ) -> Result<(), LoopCoreError>;

    fn execute_isolate_surface(
        &self,
        iteration: &mut LoopIteration,
        surface: &dyn EditableSurfaceProvider,
        picker: &dyn TargetPicker,
    ) -> Result<(), LoopCoreError>;

    fn execute_run_in_sandbox(
        &self,
        iteration: &mut LoopIteration,
        sandbox: &dyn LoopSandbox,
    ) -> Result<(), LoopCoreError>;

    fn execute_eval(
        &self,
        iteration: &mut LoopIteration,
        split: &CorpusSplit,
        key_provider: &dyn KeyProvider,
        evaluator: &dyn Evaluator,
    ) -> Result<(), LoopCoreError>;

    fn execute_accept_reject(
        &self,
        iteration: &mut LoopIteration,
        gate: &dyn OperatorGate,
    ) -> Result<(), LoopCoreError>;

    /// Promote the proposed editable-surface snapshot to the live authority
    /// surface IFF the AcceptReject stage yielded `Accept`. On `Reject` (or a
    /// missing decision) the live authority surface is left untouched. This is
    /// the only step in the loop that writes live authority — `apply_proposal`
    /// at the IsolateEditableSurface stage is sandbox-scoped and does not write.
    fn execute_promote_if_accepted(
        &self,
        iteration: &LoopIteration,
        surface: &dyn EditableSurfaceProvider,
    ) -> Result<(), LoopCoreError>;

    fn execute_record(
        &self,
        iteration: &mut LoopIteration,
        recorder: &dyn IterationRecorder,
    ) -> Result<(), LoopCoreError>;
}

/// Default LoopStageExecutor — pure engine code, no LLM, no I/O of its own.
#[derive(Debug, Clone, Default)]
pub struct LoopCore;

impl LoopCore {
    pub fn new() -> Self {
        Self
    }

    /// Drive one full iteration through every stage. Idempotent given a
    /// fresh `LoopIteration` (constructed via [`LoopIteration::new`]).
    /// Caller passes the corpus split + the key provider used by the
    /// evaluator to read the encrypted holdout.
    #[allow(clippy::too_many_arguments)]
    pub fn run_one_iteration(
        &self,
        prior_iterations: &[LoopIteration],
        iteration_number: u32,
        target_picker: &dyn TargetPicker,
        surface: &dyn EditableSurfaceProvider,
        sandbox: &dyn LoopSandbox,
        evaluator: &dyn Evaluator,
        split: &CorpusSplit,
        key_provider: &dyn KeyProvider,
        gate: &dyn OperatorGate,
        recorder: &dyn IterationRecorder,
    ) -> Result<LoopIteration, LoopCoreError> {
        let mut iteration = LoopIteration::new(iteration_number);

        // Stage 1: ChooseTarget
        self.execute_choose_target(&mut iteration, prior_iterations, target_picker)?;
        iteration.advance()?;

        // Stage 2: IsolateEditableSurface
        self.execute_isolate_surface(&mut iteration, surface, target_picker)?;
        iteration.advance()?;

        // Stage 3: RunInSandbox
        self.execute_run_in_sandbox(&mut iteration, sandbox)?;
        iteration.advance()?;

        // Stage 4: ExecuteEval
        self.execute_eval(&mut iteration, split, key_provider, evaluator)?;
        iteration.advance()?;

        // Stage 5: AcceptReject
        self.execute_accept_reject(&mut iteration, gate)?;

        // Promotion: write the proposed snapshot to the LIVE authority surface
        // ONLY when the gate accepted. On Reject the live authority store is
        // never mutated (the IsolateEditableSurface stage was sandbox-scoped).
        self.execute_promote_if_accepted(&iteration, surface)?;

        // Stage 6: RecordAsMemory (always run — Rejects also recorded for
        // future-iteration learning) then Complete.
        iteration.advance()?;
        self.execute_record(&mut iteration, recorder)?;
        iteration.advance()?;

        Ok(iteration)
    }
}

impl LoopStageExecutor for LoopCore {
    fn execute_choose_target(
        &self,
        iteration: &mut LoopIteration,
        prior: &[LoopIteration],
        picker: &dyn TargetPicker,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::ChooseTarget)?;
        let target = picker.pick(prior).map_err(LoopCoreError::TargetPicker)?;
        iteration.target = Some(target);
        Ok(())
    }

    fn execute_isolate_surface(
        &self,
        iteration: &mut LoopIteration,
        surface: &dyn EditableSurfaceProvider,
        picker: &dyn TargetPicker,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::IsolateEditableSurface)?;
        let target = iteration.target.as_ref().ok_or(LoopCoreError::Iteration(
            LoopIterationError::MissingTarget {
                stage: LoopStage::IsolateEditableSurface,
            },
        ))?;
        let baseline = surface.snapshot(target).map_err(LoopCoreError::Surface)?;
        let proposal = picker
            .propose(target)
            .map_err(LoopCoreError::TargetPicker)?;
        let proposed = surface
            .apply_proposal(&baseline, proposal)
            .map_err(LoopCoreError::Surface)?;
        iteration.editable_surface_snapshot = Some(proposed);
        Ok(())
    }

    fn execute_run_in_sandbox(
        &self,
        iteration: &mut LoopIteration,
        sandbox: &dyn LoopSandbox,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::RunInSandbox)?;
        let snapshot =
            iteration
                .editable_surface_snapshot
                .as_ref()
                .ok_or(LoopCoreError::Iteration(
                    LoopIterationError::MissingSnapshot {
                        stage: LoopStage::RunInSandbox,
                    },
                ))?;
        let run = sandbox.run(snapshot).map_err(LoopCoreError::Sandbox)?;
        iteration.sandbox_run_id = Some(run.sandbox_run_id);
        Ok(())
    }

    fn execute_eval(
        &self,
        iteration: &mut LoopIteration,
        split: &CorpusSplit,
        key_provider: &dyn KeyProvider,
        evaluator: &dyn Evaluator,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::ExecuteEval)?;
        let snapshot =
            iteration
                .editable_surface_snapshot
                .as_ref()
                .ok_or(LoopCoreError::Iteration(
                    LoopIterationError::MissingSnapshot {
                        stage: LoopStage::ExecuteEval,
                    },
                ))?;
        let result = evaluator
            .evaluate(split, key_provider, snapshot)
            .map_err(LoopCoreError::Eval)?;
        iteration.eval_result = Some(result);
        Ok(())
    }

    fn execute_accept_reject(
        &self,
        iteration: &mut LoopIteration,
        gate: &dyn OperatorGate,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::AcceptReject)?;
        let eval = iteration
            .eval_result
            .as_ref()
            .ok_or(LoopCoreError::Iteration(LoopIterationError::MissingEval {
                stage: LoopStage::AcceptReject,
            }))?;
        let decision = gate.decide(iteration, eval).map_err(LoopCoreError::Gate)?;
        iteration.decision = Some(decision);
        Ok(())
    }

    fn execute_promote_if_accepted(
        &self,
        iteration: &LoopIteration,
        surface: &dyn EditableSurfaceProvider,
    ) -> Result<(), LoopCoreError> {
        // Only an explicit Accept may touch the live authority surface. Reject
        // and a missing/unresolved decision both leave it untouched.
        if !matches!(
            iteration.decision,
            Some(AcceptRejectDecision::Accept { .. })
        ) {
            return Ok(());
        }
        let snapshot =
            iteration
                .editable_surface_snapshot
                .as_ref()
                .ok_or(LoopCoreError::Iteration(
                    LoopIterationError::MissingSnapshot {
                        stage: LoopStage::AcceptReject,
                    },
                ))?;
        surface.promote(snapshot).map_err(LoopCoreError::Surface)?;
        Ok(())
    }

    fn execute_record(
        &self,
        iteration: &mut LoopIteration,
        recorder: &dyn IterationRecorder,
    ) -> Result<(), LoopCoreError> {
        iteration.assert_stage(LoopStage::RecordAsMemory)?;
        let memory_id = recorder
            .record(iteration)
            .map_err(LoopCoreError::Recorder)?;
        // For Reject paths we still record but tag the iteration with the
        // memory id holding the rejection rationale so future picks can
        // skip it.
        if matches!(
            iteration.decision,
            Some(AcceptRejectDecision::Reject { .. })
        ) {
            iteration.rejection_memory_id = Some(memory_id);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoopCoreError {
    #[error(transparent)]
    Iteration(#[from] LoopIterationError),
    #[error(transparent)]
    Surface(#[from] super::editable_surface::EditableSurfaceError),
    #[error(transparent)]
    Sandbox(#[from] LoopSandboxError),
    #[error(transparent)]
    Eval(#[from] super::evaluator::EvalError),
    #[error(transparent)]
    Gate(#[from] OperatorGateError),
    #[error(transparent)]
    Recorder(#[from] IterationRecorderError),
    #[error(transparent)]
    TargetPicker(#[from] TargetPickerError),
}

// ---------------------------------------------------------------------------
// SelfImprovementLoop: the durable per-instance holder declared by MT-148.
// ---------------------------------------------------------------------------

/// Default bounded history capacity for [`SelfImprovementLoop`].
///
/// Bounded so a long-running loop instance cannot accumulate unbounded
/// memory. Downstream MTs (MT-156 LoopScheduler) own the per-window
/// iteration cap which is independent of this in-memory cap.
pub const SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY: usize = 100;

/// Reason the [`SelfImprovementLoop`] is paused. MT-148 declares the slot;
/// MT-153 GoodhartSentinel populates it. Other reasons (operator manual
/// pause) are reserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PauseReason {
    /// Populated by the MT-153 Goodhart sentinel when the dev/holdout gap
    /// widens monotonically for the configured number of iterations.
    GoodhartGapWidening {
        detected_at_iteration: u32,
        gap: f64,
    },
    /// Operator manually paused the loop (e.g. via the MT-155 IPC).
    OperatorPause {
        rationale: String,
        paused_at_utc: DateTime<Utc>,
    },
    /// Scheduler exhausted the per-window iteration budget (MT-156). The
    /// loop pauses until the rolling window admits a new iteration.
    SchedulerBudgetExhausted { cap: u32, window_seconds: u32 },
}

/// Token issued by the MT-154 PromotionGate when an iteration's
/// accept/reject decision is approved. MT-148 declares the slot; MT-154
/// owns the wiring. The token is opaque to MT-148 — the gate adapter is
/// responsible for ticket lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewToken {
    pub ticket_id: Uuid,
    pub iteration_id: Uuid,
    pub issued_at_utc: DateTime<Utc>,
}

impl ReviewToken {
    pub fn new(ticket_id: Uuid, iteration_id: Uuid) -> Self {
        Self {
            ticket_id,
            iteration_id,
            issued_at_utc: Utc::now(),
        }
    }
}

/// Per-target metric snapshot held by the loop instance. MT-148 declares
/// the typed shape; MT-151 (evaluator) and MT-156 (scheduler) populate it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopMetricSnapshot {
    pub last_eval_dev_pass_rate: f64,
    pub last_eval_holdout_pass_rate: f64,
    pub last_eval_at_utc: DateTime<Utc>,
}

/// SelfImprovementLoop is the durable per-instance state holder declared
/// by MT-148. It composes the [`LoopIteration`] state machine across many
/// iterations and exposes the slots downstream MTs wire into. The struct
/// is intentionally non-atomic: callers that want concurrent access wrap
/// it in `Arc<Mutex<SelfImprovementLoop>>` — see MT-148 concurrency tests
/// in `tests/self_improve_loop_core_tests.rs`.
#[derive(Debug)]
pub struct SelfImprovementLoop {
    /// Currently in-flight iteration, if any.
    current: Option<LoopIteration>,

    /// Bounded ring buffer of completed iterations.
    history: VecDeque<LoopIteration>,

    /// In-memory cap on `history`. Iterations beyond this drop the oldest.
    history_capacity: usize,

    /// Monotonic iteration counter. Increments on each `begin_iteration`.
    /// MT-156 scheduler reads this to enforce per-window caps.
    iteration_counter: u32,

    /// Per-target metric snapshot. MT-151 evaluator populates this with
    /// the latest eval result; MT-153 Goodhart sentinel reads it.
    metric_snapshot_by_target: BTreeMap<String, LoopMetricSnapshot>,

    /// Pause slot. While `Some`, every call to `begin_iteration` returns
    /// [`LoopPausedError`] until [`SelfImprovementLoop::unpause`] is called.
    /// MT-148 declares this slot; MT-153 wires GoodhartSentinel.
    pause_reason: Option<PauseReason>,

    /// Pending review queue slot. MT-148 declares this; MT-154
    /// PromotionGate adapter populates and consumes it.
    pending_review_queue: VecDeque<ReviewToken>,

    /// Last issued review token. MT-148 declares the slot; MT-154 sets
    /// this after a successful PromotionGate submit.
    last_review_token: Option<ReviewToken>,
}

impl SelfImprovementLoop {
    /// Build a loop with the default bounded history capacity
    /// ([`SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY`]).
    pub fn new() -> Self {
        Self::with_capacity(SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY)
    }

    /// Build a loop with an explicit bounded history capacity. The
    /// capacity must be at least 1.
    pub fn with_capacity(history_capacity: usize) -> Self {
        let capacity = history_capacity.max(1);
        Self {
            current: None,
            history: VecDeque::with_capacity(capacity),
            history_capacity: capacity,
            iteration_counter: 0,
            metric_snapshot_by_target: BTreeMap::new(),
            pause_reason: None,
            pending_review_queue: VecDeque::new(),
            last_review_token: None,
        }
    }

    /// Begin a fresh iteration in stage `ChooseTarget`. Returns
    /// [`LoopPausedError`] when the loop is paused. The iteration counter
    /// is incremented even if a prior in-flight iteration exists, so the
    /// counter is monotonic across the loop's lifetime regardless of
    /// short-circuit paths.
    pub fn begin_iteration(&mut self) -> Result<&mut LoopIteration, LoopPausedError> {
        if let Some(reason) = self.pause_reason.clone() {
            return Err(LoopPausedError { reason });
        }
        self.iteration_counter = self.iteration_counter.saturating_add(1);
        let iteration = LoopIteration::new(self.iteration_counter);
        self.current = Some(iteration);
        Ok(self.current.as_mut().expect("just inserted"))
    }

    /// Borrow the in-flight iteration, if any.
    pub fn current_iteration(&self) -> Option<&LoopIteration> {
        self.current.as_ref()
    }

    /// Mutable borrow of the in-flight iteration.
    pub fn current_iteration_mut(&mut self) -> Option<&mut LoopIteration> {
        self.current.as_mut()
    }

    /// Finalize the in-flight iteration: move it to history and clear the
    /// current slot. Returns the finalized iteration. The bounded history
    /// drops the oldest entry once `history_capacity` is exceeded.
    pub fn finalize_current(&mut self) -> Option<LoopIteration> {
        let iter = self.current.take()?;
        self.history.push_back(iter.clone());
        while self.history.len() > self.history_capacity {
            self.history.pop_front();
        }
        Some(iter)
    }

    /// The bounded history of completed iterations.
    pub fn history(&self) -> &VecDeque<LoopIteration> {
        &self.history
    }

    /// The history capacity declared at construction time.
    pub fn history_capacity(&self) -> usize {
        self.history_capacity
    }

    /// Monotonic loop-instance iteration counter. MT-156 scheduler reads
    /// this; MT-148 only declares the slot + bound.
    pub fn iteration_counter(&self) -> u32 {
        self.iteration_counter
    }

    /// Pause slot read. While `Some`, [`SelfImprovementLoop::begin_iteration`]
    /// returns [`LoopPausedError`].
    pub fn pause_reason(&self) -> Option<&PauseReason> {
        self.pause_reason.as_ref()
    }

    /// Pause the loop with a typed reason. Idempotent on the latest reason;
    /// callers should not chain pauses unless they want to record the
    /// latest cause.
    pub fn pause(&mut self, reason: PauseReason) {
        self.pause_reason = Some(reason);
    }

    /// Clear the pause slot. Returns the prior reason, if any.
    pub fn unpause(&mut self) -> Option<PauseReason> {
        self.pause_reason.take()
    }

    /// Read the per-target metric snapshot. MT-148 declares the slot;
    /// MT-151 evaluator populates it.
    pub fn metric_snapshot(&self, target_key: &str) -> Option<&LoopMetricSnapshot> {
        self.metric_snapshot_by_target.get(target_key)
    }

    /// Update the per-target metric snapshot. Wired by MT-151 evaluator.
    pub fn record_metric_snapshot(
        &mut self,
        target_key: impl Into<String>,
        snapshot: LoopMetricSnapshot,
    ) {
        self.metric_snapshot_by_target
            .insert(target_key.into(), snapshot);
    }

    /// Push a review token onto the pending queue. Wired by MT-154
    /// PromotionGate adapter after a successful submit. The latest token
    /// is also retained as `last_review_token`.
    pub fn enqueue_review_token(&mut self, token: ReviewToken) {
        self.last_review_token = Some(token.clone());
        self.pending_review_queue.push_back(token);
    }

    /// Pop the head of the pending review queue. Wired by MT-154 when
    /// the gate adapter resolves a ticket.
    pub fn dequeue_review_token(&mut self) -> Option<ReviewToken> {
        self.pending_review_queue.pop_front()
    }

    /// Length of the pending review queue.
    pub fn pending_review_queue_len(&self) -> usize {
        self.pending_review_queue.len()
    }

    /// Last review token issued. MT-148 only declares this slot; MT-154
    /// wires it.
    pub fn last_review_token(&self) -> Option<&ReviewToken> {
        self.last_review_token.as_ref()
    }
}

impl Default for SelfImprovementLoop {
    fn default() -> Self {
        Self::new()
    }
}

/// Returned by [`SelfImprovementLoop::begin_iteration`] when the loop is
/// paused. The carried reason mirrors the slot the sentinel populated so
/// callers can distinguish Goodhart-pause from operator-pause.
#[derive(Debug, Clone, thiserror::Error)]
#[error("self-improvement loop is paused: {reason:?}")]
pub struct LoopPausedError {
    pub reason: PauseReason,
}

/// Build a thread-safe handle around a [`SelfImprovementLoop`]. The
/// concurrency contract is "one writer at a time" — the `Mutex` ensures
/// transitions are serialized; concurrent readers may observe a
/// snapshot-consistent view by holding the lock briefly.
pub fn shared_self_improvement_loop(capacity: usize) -> std::sync::Arc<Mutex<SelfImprovementLoop>> {
    std::sync::Arc::new(Mutex::new(SelfImprovementLoop::with_capacity(capacity)))
}
