//! WP-KERNEL-004 cluster X.3 (MT-190..MT-195) Session Checkpoint + Restart Recovery.
//!
//! Submodules:
//!  - `checkpoint`: SessionCheckpoint primitive (MT-190)
//!  - `writer`: bounded-channel writer (MT-191)
//!  - `replay`: EventLedger replay (MT-192)
//!  - `restart`: restart resume orchestrator (MT-193)
//!  - `idempotency`: idempotent-recovery ledger (MT-194)
//!  - `crash_recovery`: integration test harness module (MT-195)

pub mod checkpoint;
pub mod crash_recovery;
pub mod idempotency;
pub mod replay;
pub mod restart;
pub mod writer;

pub use checkpoint::{
    CheckpointStateKind, SessionCheckpoint, SessionCheckpointId, CHECKPOINT_MAX_BYTES,
};
pub use crash_recovery::{CrashRecoveryHarness, CrashRecoveryScenario, RecoveryEvidence};
pub use idempotency::{
    ApplyOutcome, IdempotencyKey, IdempotencyLedger, IdempotencyLedgerError, IdempotentApply,
    SideEffectKind,
};
pub use replay::{
    EventLedgerRow, ReplayError, ReplayPlan, ReplayProgress, ReplayResult, StateReplayer,
};
pub use restart::{
    OperatorDecisionRequest, OrphanReclaimInfo, RestartResumeOrchestrator, ResumableSession,
    ResumeError, ResumeReport, ResumedSessionInfo,
};
pub use writer::{
    CheckpointHandle, CheckpointSink, CheckpointWriter, CheckpointWriterConfig,
    CheckpointWriterError, InMemoryCheckpointSink, PostgresCheckpointSink, StateSnapshotter,
};
