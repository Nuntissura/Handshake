//! Bounded exponential backoff with full jitter for replay-safe embedded
//! SurrealDB operations (MT-142 Lane A1, AC-142-5).
//!
//! Research basis (machine-readable, authoritative for this module):
//! `Handshake_Artifacts/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-142/kb01/research/research_basis.json`
//! (`selected_design.retry_policy`, `selected_design.error_classifier`,
//! `error_classes`). Recon inventory:
//! `Handshake_Artifacts/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-142/kb01/recon/lock_inventory.json`
//! (`retry_helpers`, `error_types`).
//!
//! Engine facts this module depends on (pinned crates, read 2026-09-10):
//!
//! * The only retryable engine error is `kvs::err::Error::TransactionConflict`
//!   (`surrealdb-core-3.2.0/src/kvs/err.rs:85-87`, `is_retryable`). It is
//!   produced from RocksDB `ErrorKind::Busy | TryAgain` (`kvs/err.rs:124-135`)
//!   and rendered as `Transaction conflict: {0}. This transaction can be
//!   retried` (`kvs/err.rs:47-49`).
//! * In 3.2.0 that variant does not reach SDK callers typed on any commit
//!   path: implicit per-statement commits become `QueryError::NotExecuted`
//!   (`surrealdb-core-3.2.0/src/dbs/executor.rs:1062-1066`), an explicit
//!   `COMMIT` row becomes `Cannot COMMIT: ...` with `NotExecuted`
//!   (`executor.rs:1485-1492`), and client-side `commit()` collapses to
//!   `Error::internal` (`surrealdb-3.2.0/src/engine/local/mod.rs:774-782`,
//!   `surrealdb-3.2.0/src/lib.rs:399-401`). [`classify_surreal_error`] is
//!   therefore typed-first with a message fallback.
//! * The engine never retries user transactions: `executor.rs` has no retry
//!   loop and `Datastore::retry` (`kvs/ds.rs:1632-1681`) only wraps bootstrap.
//! * Backoff shape is AWS full jitter, `sleep = random(0, min(cap, base * 2^n))`
//!   (research basis `selected_design.retry_policy.algorithm`).
//!
//! Attempts are never abandoned mid-flight: an in-flight query future is not
//! raced against the deadline or the cancellation token, because dropping it
//! after the engine acknowledged a commit would report a durable write as a
//! failure (research basis
//! `embedded_single_owner_constraints.shutdown_close_semantics`). Cancellation
//! and deadlines are observed before every attempt and during every sleep;
//! callers that need a per-attempt bound wrap `op` in `tokio::time::timeout`
//! themselves.
//!
//! The ad-hoc loops in `atelier/intake.rs:730-762`, `atelier/mod.rs:643-648`
//! and `process_ledger/reclaim.rs:30-65` are untouched by this lane; they can
//! migrate to [`retry`] because the classifier accepts every SDK-visible shape
//! they match today.
//!
//! Visibility: `pub` because the MT-142 `tests/` swarm target (validation plan
//! `retry_unit_tests`, `error_shape_proof`) drives this module from outside the
//! crate.

use std::future::Future;

use surrealdb::types::{ErrorDetails, QueryError};

use super::SurrealStorageError;
use crate::storage::StorageError;

/// Marker prefix of the retryable engine error
/// (`surrealdb-core-3.2.0/src/kvs/err.rs:47-49`).
pub const TRANSACTION_CONFLICT_MARKER: &str = "Transaction conflict:";
/// Marker suffix of the retryable engine error
/// (`surrealdb-core-3.2.0/src/kvs/err.rs:47-49`).
pub const TRANSACTION_RETRYABLE_MARKER: &str = "This transaction can be retried";
pub const RETRY_EXHAUSTED_CONFLICT_CODE: &str = "HSK-STORAGE-RETRY-EXHAUSTED";
pub const NO_RETRY_WINDOW_CONFLICT_CODE: &str = "HSK-STORAGE-NO-RETRY-WINDOW";
/// Raw RocksDB status prefix parsed to `ErrorKind::Busy`
/// (`surrealdb-rocksdb-0.24.0-surreal.5/src/lib.rs:231`).
const ROCKSDB_BUSY_STATUS: &str = "Resource busy";
/// Raw RocksDB status prefix parsed to `ErrorKind::TryAgain`
/// (`surrealdb-rocksdb-0.24.0-surreal.5/src/lib.rs:233`).
const ROCKSDB_TRY_AGAIN_STATUS: &str = "Operation failed. Try again.";
/// Unique-index violation rendering (`surrealdb-core-3.2.0/src/err/mod.rs:541`).
const UNIQUE_INDEX_PREFIX: &str = "Database index `";
const UNIQUE_INDEX_SUFFIX: &str = "` already contains";

pub use handshake_storage_support::retry::*;

/// Canonical product-visible conversion for every guarded SurrealDB mutation.
/// Keeping this at the retry boundary prevents store implementations and integration tests from
/// independently recreating the externally observable exhaustion classification.
pub fn retry_error_to_storage(error: RetryError<StorageError>) -> StorageError {
    match error {
        RetryError::Terminal { error, .. } => error,
        RetryError::Exhausted {
            attempts,
            elapsed,
            last,
            bound,
        } => StorageError::ConflictDetails {
            code: if bound.is_no_retry_window() {
                NO_RETRY_WINDOW_CONFLICT_CODE
            } else {
                RETRY_EXHAUSTED_CONFLICT_CODE
            },
            detail: format!(
                "attempts={attempts} elapsed_ms={} bound={} last={last}",
                elapsed.as_millis(),
                bound.as_str()
            ),
        },
        RetryError::Cancelled { .. } => {
            StorageError::Database(SurrealStorageError::Closed.to_string())
        }
    }
}

/// Process-wide jitter source for [`retry_transaction_conflicts`].
static STORAGE_RETRY_JITTER: std::sync::LazyLock<SystemJitter> =
    std::sync::LazyLock::new(SystemJitter::new);

/// Bounded MT-142 conflict retry for `SurrealStorage`-level mutations that
/// have no wrapper-level keyed lock (`SurrealDatabase::guarded_mutation` is the
/// keyed form). Only engine commit conflicts are replayed, under
/// [`RetryPolicy::CONTRACT`], observing the store's shutdown cancellation.
/// `op` must contain every pre-read the decision depends on so a retried
/// attempt observes the state the winning writer committed; the conflicting
/// transaction wrote nothing, which is what makes the replay safe. The last
/// conflict is returned unchanged when the budget is exhausted so callers keep
/// their existing error mapping (MT-141 V2-R3).
pub(crate) async fn retry_transaction_conflicts<T, F, Fut>(
    storage: &super::SurrealStorage,
    replay_key: impl Into<String>,
    mut op: F,
) -> Result<T, SurrealStorageError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, SurrealStorageError>>,
{
    let context = RetryContext::unbounded().with_cancel(storage.cancellation_token());
    retry(
        &RetryPolicy::CONTRACT,
        &context,
        Replay::idempotent(replay_key),
        &TokioClock,
        &*STORAGE_RETRY_JITTER,
        classify_surreal_storage_error,
        |_attempt| op(),
    )
    .await
    .map_err(|error| match error {
        RetryError::Terminal { error, .. } | RetryError::Exhausted { last: error, .. } => error,
        RetryError::Cancelled { .. } => SurrealStorageError::Closed,
    })
}

/// Typed-first classification of an SDK error.
///
/// Verified against the pinned SDK on 2026-09-10:
/// * typed: `ErrorDetails::Query(Some(QueryError::TransactionConflict))`
///   (`surrealdb-types-3.2.0/src/error.rs:869-884`, wire code `-32009` at
///   `error.rs:191`; mapped from `KvsError::TransactionConflict` by
///   `surrealdb-core-3.2.0/src/err/to_types.rs:294-297`);
/// * message fallback for the 3.2.0 shapes where the kind is erased
///   (`ErrorDetails::Query(Some(NotExecuted))` from `executor.rs:1062-1066`
///   and `1485-1492`; `ErrorDetails::Internal` from `engine/local/mod.rs:774-782`):
///   the message must contain BOTH [`TRANSACTION_CONFLICT_MARKER`] and
///   [`TRANSACTION_RETRYABLE_MARKER`] (`kvs/err.rs:47-49`);
/// * the raw RocksDB status forms `Resource busy` / `Operation failed. Try
///   again.` (`surrealdb-rocksdb-0.24.0-surreal.5/src/lib.rs:229-233`) only
///   when they arrive unwrapped as the whole message;
/// * the `cause()` chain is inspected with the same rules;
/// * everything else is [`RetryClass::Terminal`], including `Thrown`,
///   `AlreadyExists`, `TimedOut`, `Cancelled` and `NotExecuted` without the
///   conflict markers.
pub fn classify_surreal_error(error: &surrealdb::Error) -> RetryClass {
    let mut current = Some(error);
    while let Some(err) = current {
        if is_typed_transaction_conflict(err) || carries_conflict_message(err) {
            return RetryClass::RetryableTransient;
        }
        current = err.cause();
    }
    RetryClass::Terminal
}

fn is_typed_transaction_conflict(error: &surrealdb::Error) -> bool {
    matches!(error.query_details(), Some(QueryError::TransactionConflict))
}

fn carries_conflict_message(error: &surrealdb::Error) -> bool {
    let kind_erased = matches!(
        error.details(),
        ErrorDetails::Query(Some(QueryError::NotExecuted)) | ErrorDetails::Internal
    );
    kind_erased
        && (message_marks_transaction_conflict(error.message())
            || is_unwrapped_rocksdb_conflict_status(error.message()))
}

/// True when `message` carries both engine markers of a retryable conflict
/// (`surrealdb-core-3.2.0/src/kvs/err.rs:47-49`).
pub fn message_marks_transaction_conflict(message: &str) -> bool {
    message.contains(TRANSACTION_CONFLICT_MARKER) && message.contains(TRANSACTION_RETRYABLE_MARKER)
}

fn is_unwrapped_rocksdb_conflict_status(message: &str) -> bool {
    let status = message
        .trim()
        .trim_end_matches(|c: char| c == ':' || c.is_whitespace());
    [ROCKSDB_BUSY_STATUS, ROCKSDB_TRY_AGAIN_STATUS]
        .iter()
        .any(|prefix| {
            status == *prefix
                || status
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with(": "))
        })
}

/// Classification of the wrapper error returned by
/// `SurrealStorage::with_data_operation`: only the variants that carry an SDK
/// error can be transient.
pub fn classify_surreal_storage_error(error: &SurrealStorageError) -> RetryClass {
    match error {
        SurrealStorageError::Database(source) | SurrealStorageError::TransactionCommit(source) => {
            classify_surreal_error(source)
        }
        _ => RetryClass::Terminal,
    }
}

/// Classification of the opaque [`StorageError`]: the recon documents that
/// every SDK error reaches store callers as `StorageError::Database(String)`
/// via `From<SurrealStorageError>` (`storage/mod.rs:300-304`), so only the
/// rendered message can be inspected. The raw unwrapped RocksDB forms cannot
/// occur here because the wrapper prefixes are always present.
pub fn classify_storage_error(error: &StorageError) -> RetryClass {
    match error {
        StorageError::Database(message) if message_marks_transaction_conflict(message) => {
            RetryClass::RetryableTransient
        }
        _ => RetryClass::Terminal,
    }
}

/// Extracts the index name from a unique-index violation rendered as
/// ``Database index `{index}` already contains {value}, with record `{record}` ``
/// (`surrealdb-core-3.2.0/src/err/mod.rs:541`). This is NOT a transient
/// signal by itself (`ErrorDetails::AlreadyExists`, research basis
/// `error_classes`); the integrating store decides whether the surrounding
/// idempotent upsert may be replayed as [`RetryClass::RetryableSnapshotChange`].
pub fn is_unique_index_violation(message: &str) -> Option<String> {
    let start = message.find(UNIQUE_INDEX_PREFIX)? + UNIQUE_INDEX_PREFIX.len();
    let rest = message.get(start..)?;
    let end = rest.find('`')?;
    let index = rest.get(..end)?;
    if index.is_empty() || !rest.get(end..)?.starts_with(UNIQUE_INDEX_SUFFIX) {
        return None;
    }
    Some(index.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::Ordering};
    use std::time::{Duration, Instant};
    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }
    fn conflict_not_executed() -> surrealdb::Error {
        surrealdb::Error::query(
            "The query was not executed due to a failed transaction. Transaction conflict: Resource busy. This transaction can be retried".to_string(),
            Some(QueryError::NotExecuted),
        )
    }

    #[test]
    fn canonical_storage_mapper_covers_terminal_cancelled_and_all_exhaustion_bounds() {
        let terminal = retry_error_to_storage(RetryError::Terminal {
            attempts: 1,
            elapsed: ms(3),
            error: StorageError::Conflict("terminal-marker"),
        });
        assert!(matches!(
            terminal,
            StorageError::Conflict("terminal-marker")
        ));

        let cancelled = retry_error_to_storage(RetryError::Cancelled {
            attempts: 2,
            elapsed: ms(7),
        });
        assert!(
            matches!(cancelled, StorageError::Database(ref detail) if detail.to_ascii_lowercase().contains("closed")),
            "cancelled guarded mutations expose the closed-store contract: {cancelled}"
        );

        for bound in [ExhaustionBound::MaxAttempts, ExhaustionBound::MaxElapsed] {
            let mapped = retry_error_to_storage(RetryError::Exhausted {
                attempts: 3,
                elapsed: ms(11),
                last: StorageError::Conflict("transient-marker"),
                bound,
            });
            assert!(
                matches!(
                    mapped,
                    StorageError::ConflictDetails { code: RETRY_EXHAUSTED_CONFLICT_CODE, ref detail }
                        if detail.contains("attempts=3") && detail.contains(bound.as_str())
                ),
                "{bound:?} must use the canonical retry-exhausted product shape: {mapped}"
            );
        }

        let no_window = retry_error_to_storage(RetryError::Exhausted {
            attempts: 1,
            elapsed: ms(19),
            last: StorageError::Conflict("transient-marker"),
            bound: ExhaustionBound::NoRetryWindow,
        });
        assert!(
            matches!(
                no_window,
                StorageError::ConflictDetails { code: NO_RETRY_WINDOW_CONFLICT_CODE, ref detail }
                    if detail.contains("attempts=1") && detail.contains("bound=no_retry_window")
            ),
            "NoRetryWindow must remain distinct through the production mapper: {no_window}"
        );
    }

    #[test]
    fn classify_surreal_error_accepts_typed_transaction_conflict() {
        let error = surrealdb::Error::query(
            "anything".to_string(),
            Some(QueryError::TransactionConflict),
        );
        assert_eq!(
            classify_surreal_error(&error),
            RetryClass::RetryableTransient
        );
    }

    #[test]
    fn classify_surreal_error_accepts_erased_shapes_with_both_markers() {
        assert_eq!(
            classify_surreal_error(&conflict_not_executed()),
            RetryClass::RetryableTransient
        );
        let commit_row = surrealdb::Error::query(
            "Cannot COMMIT: Transaction conflict: Resource busy. This transaction can be retried"
                .to_string(),
            Some(QueryError::NotExecuted),
        );
        assert_eq!(
            classify_surreal_error(&commit_row),
            RetryClass::RetryableTransient
        );
        let client_side = surrealdb::Error::internal(
            "Transaction conflict: Operation failed. Try again.: insufficient history. This transaction can be retried".to_string(),
        );
        assert_eq!(
            classify_surreal_error(&client_side),
            RetryClass::RetryableTransient
        );
    }

    #[test]
    fn classify_surreal_error_accepts_unwrapped_rocksdb_status_only() {
        let busy = surrealdb::Error::internal("Resource busy: ".to_string());
        assert_eq!(
            classify_surreal_error(&busy),
            RetryClass::RetryableTransient
        );
        let try_again = surrealdb::Error::internal("Operation failed. Try again.".to_string());
        assert_eq!(
            classify_surreal_error(&try_again),
            RetryClass::RetryableTransient
        );
        let wrapped = surrealdb::Error::internal(
            "IO error: Failed to create lock file: store/LOCK (Resource busy)".to_string(),
        );
        assert_eq!(classify_surreal_error(&wrapped), RetryClass::Terminal);
    }

    #[test]
    fn classify_surreal_error_inspects_the_cause_chain() {
        let outer = surrealdb::Error::internal("router relay".to_string())
            .with_cause(conflict_not_executed());
        assert_eq!(
            classify_surreal_error(&outer),
            RetryClass::RetryableTransient
        );
    }

    #[test]
    fn classify_surreal_error_is_terminal_for_everything_else() {
        let cases = vec![
            surrealdb::Error::query(
                "The query was not executed due to a failed transaction. An error occurred: HSK-KRD-SAVE-STALE".to_string(),
                Some(QueryError::NotExecuted),
            ),
            surrealdb::Error::query(
                "Transaction conflict: Resource busy".to_string(),
                Some(QueryError::NotExecuted),
            ),
            surrealdb::Error::thrown(
                "Transaction conflict: spoof. This transaction can be retried".to_string(),
            ),
            surrealdb::Error::already_exists(
                "Database index `uq_x` already contains 'a', with record `t:1`".to_string(),
                None,
            ),
            surrealdb::Error::query("timed out".to_string(), Some(QueryError::Cancelled)),
            surrealdb::Error::internal("Failed to send command".to_string()),
        ];
        for error in cases {
            assert_eq!(
                classify_surreal_error(&error),
                RetryClass::Terminal,
                "{error:?}"
            );
        }
    }

    #[test]
    fn classify_wrapper_errors_follow_the_sdk_rules() {
        let transient = SurrealStorageError::Database(conflict_not_executed());
        assert_eq!(
            classify_surreal_storage_error(&transient),
            RetryClass::RetryableTransient
        );
        let commit = SurrealStorageError::TransactionCommit(conflict_not_executed());
        assert_eq!(
            classify_surreal_storage_error(&commit),
            RetryClass::RetryableTransient
        );
        assert_eq!(
            classify_surreal_storage_error(&SurrealStorageError::Closed),
            RetryClass::Terminal
        );

        let rendered = StorageError::from(transient);
        assert_eq!(
            classify_storage_error(&rendered),
            RetryClass::RetryableTransient
        );
        assert_eq!(
            classify_storage_error(&StorageError::Conflict("stale")),
            RetryClass::Terminal
        );
        assert_eq!(
            classify_storage_error(&StorageError::Database(
                "database error: Database index `uq_x` already contains 'a', with record `t:1`"
                    .to_string()
            )),
            RetryClass::Terminal
        );
    }

    /// Review finding R1-1-4: an engine commit conflict only ever reaches a
    /// caller as rendered text. The executor rewrites every prior result slot
    /// to the generic not-executed message and pushes the real cause on a
    /// trailing COMMIT row (`surrealdb-core-3.2.0/src/dbs/executor.rs:1476-1492`),
    /// which `meaningful_check` in `knowledge.rs` selects and `map_err` renders
    /// into `StorageError::Database`. These are the exact captured renderings;
    /// if an SDK bump changes either, this unit test fails before any load run.
    #[test]
    fn classify_storage_error_pins_the_pinned_sdk_commit_renderings() {
        // Trailing COMMIT row of a conflicted explicit transaction.
        let commit_row = StorageError::Database(
            "database error: embedded database error: Cannot COMMIT: Transaction conflict: \
             Resource busy. This transaction can be retried"
                .to_string(),
        );
        assert_eq!(
            classify_storage_error(&commit_row),
            RetryClass::RetryableTransient,
            "the COMMIT row rendering must stay retryable"
        );

        // Implicit per-statement transaction (no BEGIN in the query string).
        let implicit = StorageError::Database(
            "database error: embedded database error: The query was not executed due to a failed \
             transaction. Transaction conflict: Resource busy. This transaction can be retried"
                .to_string(),
        );
        assert_eq!(
            classify_storage_error(&implicit),
            RetryClass::RetryableTransient,
            "the implicit-path rendering must stay retryable"
        );

        // The generic first-slot text every prior statement is rewritten to.
        // Selecting it instead of the COMMIT row is the regression R1-1-4
        // guards: on its own it carries no conflict marker and is terminal.
        let generic = StorageError::Database(
            "database error: embedded database error: The query was not executed due to a failed \
             transaction"
                .to_string(),
        );
        assert_eq!(
            classify_storage_error(&generic),
            RetryClass::Terminal,
            "the generic not-executed slot must never be treated as retryable"
        );

        // Both markers are required; half a marker is terminal.
        for half in [
            "database error: embedded database error: Cannot COMMIT: Transaction conflict: \
             Resource busy",
            "database error: embedded database error: This transaction can be retried",
        ] {
            assert_eq!(
                classify_storage_error(&StorageError::Database(half.to_string())),
                RetryClass::Terminal,
                "a single marker must not be enough: {half}"
            );
        }
        assert!(TRANSACTION_CONFLICT_MARKER.starts_with("Transaction conflict:"));
        assert_eq!(
            TRANSACTION_RETRYABLE_MARKER,
            "This transaction can be retried"
        );
    }

    #[test]
    fn unique_index_violation_extracts_the_index_name() {
        assert_eq!(
            is_unique_index_violation(
                "embedded database error: Database index `uq_knowledge_entities_identity` already contains ['ws', 'k'], with record `knowledge_entities:x`"
            ),
            Some("uq_knowledge_entities_identity".to_string())
        );
        assert_eq!(
            is_unique_index_violation("Database index `` already contains"),
            None
        );
        assert_eq!(
            is_unique_index_violation("Database index `x` is missing"),
            None
        );
        assert_eq!(
            is_unique_index_violation("Transaction conflict: Resource busy"),
            None
        );
    }

    /// MT-151 I-151-6: the retry budget's zero-remaining branch. When the
    /// enclosing `with_operation_deadline` scope has already passed,
    /// `SurrealStorage::with_data_operation` must refuse to dispatch and return
    /// the typed `StatementBudgetExhausted { budget_ms }` (the configured
    /// statement timeout, `surreal.rs` `with_data_operation`) WITHOUT taking a
    /// lease or running the operation: a `timeout(0, ..)` would instead report
    /// `StatementTimeout { waited_ms: 0 }`, which claims the engine may have
    /// applied a statement it never received. A bare engine open (no schema
    /// bootstrap) is enough because no statement is ever issued.
    #[tokio::test]
    async fn expired_operation_deadline_refuses_to_dispatch_with_the_typed_budget_error() {
        use std::sync::atomic::AtomicBool;

        use crate::storage::surreal::{SurrealStorage, SurrealStorageConfig};

        let temp = tempfile::tempdir().expect("create temporary root");
        let config = SurrealStorageConfig::with_path(temp.path().join("store"))
            .expect("configure store")
            .with_statement_timeout(ms(1_500))
            .expect("non-zero statement timeout");
        let storage = SurrealStorage::open(config).await.expect("open bare store");
        storage.reset_lease_high_water();

        let issued = Arc::new(AtomicBool::new(false));
        let expired = Instant::now()
            .checked_sub(ms(10))
            .expect("an instant 10 ms in the past exists");
        let outcome = SurrealStorage::with_operation_deadline(expired, {
            let issued = Arc::clone(&issued);
            let storage = storage.clone();
            async move {
                storage
                    .with_data_operation(move |_database| {
                        Box::pin(async move {
                            issued.store(true, Ordering::SeqCst);
                            Ok(())
                        })
                    })
                    .await
            }
        })
        .await;

        match outcome {
            Err(SurrealStorageError::StatementBudgetExhausted { budget_ms }) => {
                assert_eq!(
                    budget_ms, 1_500,
                    "budget_ms is the configured statement timeout"
                );
            }
            other => panic!("expected StatementBudgetExhausted, got {other:?}"),
        }
        assert!(
            !issued.load(Ordering::SeqCst),
            "the operation must never run once the budget is spent"
        );
        assert_eq!(
            storage.lease_high_water(),
            0,
            "no lifecycle lease may be taken for a statement that is never dispatched"
        );
        assert_eq!(storage.leases_in_flight(), 0);

        // The same store still dispatches normally outside the expired scope,
        // so the refusal came from the budget, not from the store's state.
        let ran = storage
            .with_data_operation(|_database| Box::pin(async move { Ok(7u8) }))
            .await
            .expect("an unscoped operation dispatches");
        assert_eq!(ran, 7);
        assert_eq!(storage.lease_high_water(), 1);
        storage.shutdown().await.expect("close bare store");
    }
}
