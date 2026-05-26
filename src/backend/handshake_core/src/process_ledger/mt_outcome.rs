//! Contract-path re-export for MT-188 outcome recording.
//!
//! MT-188 originally names this module as the durable authority surface. The
//! cluster X.2 implementation remains colocated under `mt_executor::outcome`
//! so executor-local call sites share one implementation.

pub use crate::mt_executor::outcome::{
    DistillationThreshold, MtOutcome, MtOutcomeError, MtOutcomeKind, MtOutcomeRecord,
    MtOutcomeRecorder,
};
