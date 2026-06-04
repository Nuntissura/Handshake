//! WP-KERNEL-004 cluster X.2 (MT-184..MT-189) MicroTask Executor.
//!
//! Submodules:
//!  - `job`: MicroTaskJob + EscalationTier + JobState primitives (MT-184)
//!  - `queue`: Postgres-backed MicroTaskQueue with SKIP LOCKED claims (MT-184)
//!  - `loop_control`: MtLoopCheckpoint + MtLoopControlBudget (MT-185)
//!  - `cancellation`: cooperative + forced cancellation (MT-186)
//!  - `scheduler`: FairScheduler with age-based priority (MT-187)
//!  - `outcome`: MT outcome recording + 6-level escalation routing + distillation candidates (MT-188)
//!  - `executor`: MicroTaskExecutor orchestrator (MT-189)

pub mod cancellation;
pub mod executor;
pub mod job;
pub mod loop_control;
pub mod outcome;
pub mod queue;
pub mod scheduler;

pub use cancellation::{
    MtCancellationCleanupHook, MtCancellationReason, MtCancellationToken, MtCanceller,
};
pub use executor::{
    MicroTaskExecutor, MtCoderError, MtCoderHandle, MtExecutionContext, MtIterationOutcome,
    MtIterationResult,
};
pub use job::{
    CompletionSignal, EscalationStep, EscalationTier, MicroTaskJob, MicroTaskJobId,
    MicroTaskJobState, RunLedgerPointer,
};
pub use loop_control::{
    LoopControlError, MtLoopCheckpoint, MtLoopControl, MtLoopControlBudget, MtLoopState,
    VerifierFeedbackRef,
};
pub use outcome::{
    enact_decision, insert_distillation_candidate, DistillationCandidate,
    DistillationCandidateStatus, DistillationThreshold, EscalationDecision,
    EscalationMailboxPoster, EscalationRouter, MtOutcome, MtOutcomeError, MtOutcomeKind,
    MtOutcomeRecord, MtOutcomeRecorder, RoleMailboxEscalationPoster,
};
pub use queue::{MicroTaskQueue, QueueError};
pub use scheduler::{
    FairScheduler, SchedulerError, StarvationConfig, StarvationGuard, StarvationSignal,
};
