//! MT-194 process-ledger-facing idempotency exports.
//!
//! The folded cluster X.3 implementation lives in `session_checkpoint` so
//! checkpoint replay and idempotent side-effect tracking share one type
//! boundary. This module preserves the MT-194 process-ledger import path.

pub use crate::session_checkpoint::{
    ApplyOutcome, IdempotencyKey, IdempotencyLedger, IdempotencyLedgerError, IdempotentApply,
    SideEffectKind,
};
