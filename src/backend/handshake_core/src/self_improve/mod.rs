//! Self-improvement loop module per WP-KERNEL-004 cluster D.2.
//!
//! Implements the Karpathy autoresearch one-iteration pattern
//! (target -> eval -> propose -> review -> accept/reject) bounded by
//! HBR-SWARM-002 loop caps and protected by a multi-metric promotion floor
//! plus a Goodhart sentinel.
//!
//! Surfaces:
//! - [`iteration`] - typed LoopIteration state machine (MT-148)
//! - [`loop_core`] - LoopCore + LoopStageExecutor + run_one_iteration driver (MT-148)
//! - [`editable_surface`] - V0 allow-list + forbid-list (MT-149)
//! - [`corpus`] - HBR test-packet corpus + 60/20/20 split + encrypted holdout (MT-150)
//! - [`evaluator`] - ValidatorFirstPassEvaluator (MT-151)
//! - [`promotion_floor`] - MultiMetricPromotionFloor (MT-152)
//! - [`goodhart_sentinel`] - dev/holdout gap monotonic widening detector (MT-153)
//! - [`promotion_gate_adapter`] - KERNEL-001 PromotionGate wiring (MT-154)
//! - [`ipc`] - Tauri IPC commands (MT-155)
//! - [`scheduler`] - LoopScheduler with HBR-SWARM-002 cap (MT-156)

pub mod corpus;
pub mod editable_surface;
pub mod evaluator;
pub mod goodhart_sentinel;
pub mod ipc;
pub mod iteration;
pub mod loop_core;
pub mod promotion_floor;
pub mod promotion_gate_adapter;
pub mod scheduler;

pub use corpus::{
    CorpusError, CorpusItem, CorpusSplit, EncryptedHoldout, HbrTestPacketCorpus, KeyError,
    KeyProvider, StaticKeyProvider, ValidatorVerdict,
};
pub use editable_surface::{
    EditableSurfaceError, EditableSurfaceProvider, EditableSurfaceSnapshot, ForbidReason,
    ForbiddenSurfaceGuard, ModelManualSurface, PolicyParameter, RetrievalPolicySurface,
    SurfaceProposal,
};
pub use evaluator::{
    EvalError, EvalResult, PerItemResult, SplitMetrics, ValidatorFirstPassEvaluator,
    ValidatorRunner,
};
pub use goodhart_sentinel::{
    GoodhartSentinel, PauseReason, SentinelDecision, SentinelEntry, SentinelHistory,
    SentinelReceipt, FR_EVT_GOODHART_PAUSE,
};
pub use ipc::{
    LoopIpcError, LoopIpcState, LoopStatusSnapshot, PauseReceipt, UnpauseReceipt,
    FR_EVT_LOOP_PAUSE, FR_EVT_LOOP_UNPAUSE, FR_EVT_PROMOTION_DECISION,
};
pub use iteration::{
    AcceptRejectDecision, LoopIteration, LoopIterationError, LoopStage, LoopTarget, OperatorId,
};
pub use loop_core::{
    shared_self_improvement_loop, Evaluator, IterationRecorder, LoopCore, LoopMetricSnapshot,
    LoopPausedError, LoopSandbox, LoopStageExecutor, OperatorGate, PauseReason as LoopPauseReason,
    ReviewToken, SandboxRunResult, SelfImprovementLoop, TargetPicker,
    SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY,
};
pub use promotion_floor::{
    FloorReason, MetricDelta, MultiMetricPromotionFloor, PromotionDecision, PromotionTolerances,
};
pub use promotion_gate_adapter::{
    GateError, LoopPromotionGate, PromotionApproval, PromotionGateSubmitter, PromotionRejection,
    PromotionRequest, PromotionStatus, PromotionTicket,
};
pub use scheduler::{
    IterationBudget, LoopScheduler, ScheduleDecision, SchedulerHistory, SchedulerHistoryEntry,
    SkipReason, FR_EVT_DISTILL_LOOP_CAP,
};
