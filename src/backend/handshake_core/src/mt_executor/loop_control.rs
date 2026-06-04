//! WP-KERNEL-004 cluster X.2 MT-185 — sibling re-export of the canonical
//! `crate::process_ledger::mt_loop_control` module.
//!
//! The MT-185 contract pins ownership at
//! `src/backend/handshake_core/src/process_ledger/mt_loop_control.rs`. The
//! cluster-X.2 executor (MT-189, `mt_executor::executor`) and any
//! adjacent integration code imports loop-control types via
//! `super::loop_control::*` for ergonomic locality. This file therefore
//! re-exports the canonical public surface so:
//!   * `crate::process_ledger::mt_loop_control` is the single source of
//!     truth (contract authority).
//!   * Existing `use super::loop_control::{MtLoopControl, MtLoopState,
//!     MtLoopControlBudget}` call sites keep working without source churn.
//!
//! Adding a new loop-control type? Add it in
//! `process_ledger::mt_loop_control` and extend the `pub use` glob below.

pub use crate::process_ledger::mt_loop_control::{
    CheckpointRepoError, EvidencePointer, LoopControlError, MtLoopCheckpoint, MtLoopCheckpointRepo,
    MtLoopControl, MtLoopControlBudget, MtLoopState, ResumeContext, VerifierFeedbackRef,
    COMPACT_SUMMARY_MAX_BYTES,
};
