//! Shared storage error, keyed lock and bounded retry contracts.
mod error;
pub use error::{StorageError, StorageResult};
pub mod canonical_json;
pub mod keyed_lock;
pub mod retry;

pub mod diagnostics;
