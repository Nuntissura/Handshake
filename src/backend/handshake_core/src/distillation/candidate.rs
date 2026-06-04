//! Contract-path re-export for MT-188 logging-only distillation candidates.
//!
//! This is distinct from the cluster C training/promotion surfaces in this
//! directory; Phase 1 candidates are emitted by `mt_executor::outcome`.

pub use crate::mt_executor::outcome::{
    DistillationCandidate, DistillationCandidateStatus, DistillationThreshold,
};
