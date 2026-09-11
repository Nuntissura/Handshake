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

use std::collections::hash_map::RandomState;
use std::fmt;
use std::future::Future;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surrealdb::types::{ErrorDetails, QueryError};
use tokio_util::sync::CancellationToken;

use super::SurrealStorageError;
use crate::storage::StorageError;

const TARGET: &str = "handshake_core::storage::surreal::retry";

/// Marker prefix of the retryable engine error
/// (`surrealdb-core-3.2.0/src/kvs/err.rs:47-49`).
pub const TRANSACTION_CONFLICT_MARKER: &str = "Transaction conflict:";
/// Marker suffix of the retryable engine error
/// (`surrealdb-core-3.2.0/src/kvs/err.rs:47-49`).
pub const TRANSACTION_RETRYABLE_MARKER: &str = "This transaction can be retried";
/// Raw RocksDB status prefix parsed to `ErrorKind::Busy`
/// (`surrealdb-rocksdb-0.24.0-surreal.5/src/lib.rs:231`).
const ROCKSDB_BUSY_STATUS: &str = "Resource busy";
/// Raw RocksDB status prefix parsed to `ErrorKind::TryAgain`
/// (`surrealdb-rocksdb-0.24.0-surreal.5/src/lib.rs:233`).
const ROCKSDB_TRY_AGAIN_STATUS: &str = "Operation failed. Try again.";
/// Unique-index violation rendering (`surrealdb-core-3.2.0/src/err/mod.rs:541`).
const UNIQUE_INDEX_PREFIX: &str = "Database index `";
const UNIQUE_INDEX_SUFFIX: &str = "` already contains";

/// Retry budget. Contract values come from research basis
/// `selected_design.retry_policy` (`base_delay_ms` 5, `maximum_delay_ms` 250,
/// `maximum_attempts` 8, `maximum_elapsed_ms` 2000; worst-case sleep sum
/// 815 ms per `numeric_budgets.retry_budget_check`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Upper bound of the first sleep; doubles per retry.
    pub base_delay: Duration,
    /// Cap on any single sleep.
    pub maximum_delay: Duration,
    /// Total attempts including the first; at least one attempt always runs.
    pub maximum_attempts: u32,
    /// No sleep may end later than `start + maximum_elapsed`.
    pub maximum_elapsed: Duration,
}

impl RetryPolicy {
    /// The MT-142 contract policy.
    pub const CONTRACT: RetryPolicy = RetryPolicy {
        base_delay: Duration::from_millis(5),
        maximum_delay: Duration::from_millis(250),
        maximum_attempts: 8,
        maximum_elapsed: Duration::from_millis(2000),
    };

    /// Explicit policy for tests and callers with a different budget.
    pub const fn new(
        base_delay: Duration,
        maximum_delay: Duration,
        maximum_attempts: u32,
        maximum_elapsed: Duration,
    ) -> Self {
        Self {
            base_delay,
            maximum_delay,
            maximum_attempts,
            maximum_elapsed,
        }
    }

    pub const fn with_base_delay(self, base_delay: Duration) -> Self {
        Self { base_delay, ..self }
    }

    pub const fn with_maximum_delay(self, maximum_delay: Duration) -> Self {
        Self {
            maximum_delay,
            ..self
        }
    }

    pub const fn with_maximum_attempts(self, maximum_attempts: u32) -> Self {
        Self {
            maximum_attempts,
            ..self
        }
    }

    pub const fn with_maximum_elapsed(self, maximum_elapsed: Duration) -> Self {
        Self {
            maximum_elapsed,
            ..self
        }
    }

    /// `maximum_attempts` clamped so the operation always runs once.
    pub fn effective_maximum_attempts(&self) -> u32 {
        self.maximum_attempts.max(1)
    }

    /// `min(maximum_delay, base_delay * 2^retry_index)`; saturates at
    /// `maximum_delay` when the multiplication would overflow.
    pub fn backoff_upper_bound(&self, retry_index: u32) -> Duration {
        let factor = 1u32.checked_shl(retry_index).filter(|_| retry_index < 32);
        match factor.and_then(|factor| self.base_delay.checked_mul(factor)) {
            Some(delay) => delay.min(self.maximum_delay),
            None => self.maximum_delay,
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::CONTRACT
    }
}

/// Injectable time source so tests are deterministic without weakening
/// production behaviour (research basis
/// `selected_design.retry_policy.injectable_clock_and_jitter`; tokio
/// `test-util` is intentionally not enabled, `Cargo.toml` tokio features).
pub trait RetryClock: Send + Sync {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

/// Production clock over `tokio::time::sleep`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioClock;

impl RetryClock for TokioClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(duration)
    }
}

/// Injectable randomness for the full-jitter sleep.
pub trait JitterSource: Send + Sync {
    /// Uniform sample in `[0, upper]` (inclusive). [`retry`] clamps the result
    /// to `upper` defensively, so a misbehaving source can only shorten sleeps.
    fn full_jitter(&self, upper: Duration) -> Duration;
}

/// Production jitter: SplitMix64 seeded from the operating-system CSPRNG
/// (`getrandom`, a direct dependency) with a `RandomState` fallback. No `rand`
/// crate is used because it is not a direct dependency (`Cargo.toml`
/// `[dependencies]`); the generator only needs to decorrelate sleeps, not to
/// be cryptographic.
#[derive(Debug)]
pub struct SystemJitter {
    state: AtomicU64,
}

impl SystemJitter {
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(seed_from_os()),
        }
    }

    fn next_u64(&self) -> u64 {
        let mut z = self
            .state
            .fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed)
            .wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

impl Default for SystemJitter {
    fn default() -> Self {
        Self::new()
    }
}

impl JitterSource for SystemJitter {
    fn full_jitter(&self, upper: Duration) -> Duration {
        let upper_nanos = u64::try_from(upper.as_nanos()).unwrap_or(u64::MAX);
        if upper_nanos == 0 {
            return Duration::ZERO;
        }
        let span = u128::from(upper_nanos) + 1;
        let sample = u128::from(self.next_u64()) % span;
        Duration::from_nanos(u64::try_from(sample).unwrap_or(upper_nanos))
    }
}

fn seed_from_os() -> u64 {
    let mut bytes = [0u8; 8];
    if getrandom::getrandom(&mut bytes).is_ok() {
        let seed = u64::from_le_bytes(bytes);
        if seed != 0 {
            return seed;
        }
    }
    let mut hasher = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    hasher.write_u128(nanos);
    hasher.finish() | 1
}

/// Caller-owned bounds. `deadline` is folded into the policy's
/// `maximum_elapsed` bound (effective deadline = the earlier of the two,
/// research basis `selected_design.retry_policy.deadline_and_cancellation`),
/// so exceeding it reports [`ExhaustionBound::MaxElapsed`].
#[derive(Clone, Debug, Default)]
pub struct RetryContext {
    pub deadline: Option<Instant>,
    pub cancel: Option<CancellationToken>,
}

impl RetryContext {
    pub const fn unbounded() -> Self {
        Self {
            deadline: None,
            cancel: None,
        }
    }

    pub fn with_deadline(mut self, deadline: Instant) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = Some(cancel);
        self
    }

    fn is_cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
    }
}

/// Replay safety declared by the caller. Only `Idempotent` operations are ever
/// retried (contract `retryable_only[1]`, research basis
/// `selected_design.retry_policy.never_retry`: "any operation lacking an
/// idempotency key or proven replay safety").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Replay {
    /// Re-running the same statement converges; `key` names the idempotency
    /// key or the replay-safety proof for diagnostics.
    Idempotent { key: String },
    /// Never retried on any class; the first error is returned as
    /// [`RetryError::Terminal`].
    NotIdempotent,
}

impl Replay {
    pub fn idempotent(key: impl Into<String>) -> Self {
        Self::Idempotent { key: key.into() }
    }

    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Idempotent { key } => Some(key.as_str()),
            Self::NotIdempotent => None,
        }
    }

    fn permits(&self, class: RetryClass) -> bool {
        match (self, class) {
            (_, RetryClass::Terminal) | (Self::NotIdempotent, _) => false,
            (Self::Idempotent { .. }, _) => true,
        }
    }
}

/// Outcome class assigned by the caller's classifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryClass {
    /// Engine commit conflict (`KvsError::TransactionConflict` in any
    /// SDK-visible shape); the transaction wrote nothing.
    RetryableTransient,
    /// A unique-index or IF-EXISTS guard raced on an idempotent upsert;
    /// re-running the same statement converges. Honoured only under
    /// [`Replay::Idempotent`]; decided by the integrating store, never by
    /// [`is_unique_index_violation`] alone.
    RetryableSnapshotChange,
    /// Everything else (research basis `selected_design.retry_policy.never_retry`).
    Terminal,
}

/// Passed to the operation on every attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryAttempt {
    /// 1-based attempt number.
    pub number: u32,
    /// Retries before this attempt (`number - 1`).
    pub retries: u32,
    /// Elapsed since the retry loop started.
    pub elapsed: Duration,
}

/// Which bound stopped an otherwise retryable operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExhaustionBound {
    MaxAttempts,
    /// The wall-clock bound (the caller deadline, else `maximum_elapsed`) or
    /// the accumulated-backoff budget ended AFTER at least one replay ran.
    MaxElapsed,
    /// The budget ended BEFORE any replay could be scheduled: the first
    /// attempt failed retryably and there was no room even for the shortest
    /// backoff. Distinct from [`Self::MaxElapsed`] because "no retry window"
    /// and "retried until the budget ran out" are different operational
    /// states - the first says the caller budget was too small for the
    /// observed attempt latency (a load-budget signal), the second says
    /// replays genuinely did not converge.
    NoRetryWindow,
}

impl ExhaustionBound {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MaxAttempts => "max_attempts",
            Self::MaxElapsed => "max_elapsed",
            Self::NoRetryWindow => "no_retry_window",
        }
    }

    /// True when the operation stopped without ever scheduling a replay.
    pub const fn is_no_retry_window(&self) -> bool {
        matches!(self, Self::NoRetryWindow)
    }
}

/// Failure of a retried operation.
#[derive(Debug)]
pub enum RetryError<E> {
    /// The classifier or [`Replay::NotIdempotent`] stopped the loop.
    Terminal {
        attempts: u32,
        elapsed: Duration,
        error: E,
    },
    /// Every attempt failed retryably and a bound was reached.
    Exhausted {
        attempts: u32,
        elapsed: Duration,
        last: E,
        bound: ExhaustionBound,
    },
    /// The cancellation token fired before an attempt or during a sleep.
    Cancelled { attempts: u32, elapsed: Duration },
}

impl<E> RetryError<E> {
    pub fn attempts(&self) -> u32 {
        match self {
            Self::Terminal { attempts, .. }
            | Self::Exhausted { attempts, .. }
            | Self::Cancelled { attempts, .. } => *attempts,
        }
    }

    pub fn elapsed(&self) -> Duration {
        match self {
            Self::Terminal { elapsed, .. }
            | Self::Exhausted { elapsed, .. }
            | Self::Cancelled { elapsed, .. } => *elapsed,
        }
    }

    /// The underlying error, if any attempt ran.
    pub fn error(&self) -> Option<&E> {
        match self {
            Self::Terminal { error, .. } | Self::Exhausted { last: error, .. } => Some(error),
            Self::Cancelled { .. } => None,
        }
    }

    pub fn into_error(self) -> Option<E> {
        match self {
            Self::Terminal { error, .. } | Self::Exhausted { last: error, .. } => Some(error),
            Self::Cancelled { .. } => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminal {
                attempts, error, ..
            } => write!(f, "terminal after {attempts} attempt(s): {error}"),
            Self::Exhausted {
                attempts,
                elapsed,
                last,
                bound,
            } => write!(
                f,
                "retry exhausted ({}) after {attempts} attempt(s) in {} ms: {last}",
                bound.as_str(),
                elapsed.as_millis()
            ),
            Self::Cancelled { attempts, elapsed } => write!(
                f,
                "retry cancelled after {attempts} attempt(s) in {} ms",
                elapsed.as_millis()
            ),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RetryError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error().map(|error| error as &(dyn std::error::Error + 'static))
    }
}

/// Runs `op` until it succeeds, a terminal error occurs, a bound is reached
/// or the context is cancelled.
///
/// Guarantees (each proven by a unit test in this module):
/// * the attempt count never exceeds `policy.effective_maximum_attempts()`;
/// * no sleep starts if it would end after the effective deadline
///   (`Exhausted { bound: MaxElapsed }` is returned instead);
/// * cancellation is observed before every attempt and during every sleep;
/// * the exhaustion diagnostic is emitted exactly once per exhausted retry at
///   `warn` level; per-attempt scheduling is logged at `debug` only;
/// * no panic on any path.
pub async fn retry<T, E, C, J, K, F, Fut>(
    policy: &RetryPolicy,
    ctx: &RetryContext,
    replay: Replay,
    clock: &C,
    jitter: &J,
    classify: K,
    mut op: F,
) -> Result<T, RetryError<E>>
where
    E: fmt::Display,
    C: RetryClock,
    J: JitterSource,
    K: Fn(&E) -> RetryClass,
    F: FnMut(RetryAttempt) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let started = clock.now();
    let deadline = effective_deadline(policy, ctx, started);
    let maximum_attempts = policy.effective_maximum_attempts();
    // Accumulated backoff, bounded separately from wall clock (see
    // [`effective_deadline`]).
    let mut slept = Duration::ZERO;
    let mut attempts = 0u32;
    loop {
        if ctx.is_cancelled() {
            return Err(RetryError::Cancelled {
                attempts,
                elapsed: elapsed_since(clock, started),
            });
        }
        let attempt = RetryAttempt {
            number: attempts.saturating_add(1),
            retries: attempts,
            elapsed: elapsed_since(clock, started),
        };
        attempts = attempts.saturating_add(1);
        let error = match op(attempt).await {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        let elapsed = elapsed_since(clock, started);
        let class = classify(&error);
        if !replay.permits(class) {
            return Err(RetryError::Terminal {
                attempts,
                elapsed,
                error,
            });
        }
        if attempts >= maximum_attempts {
            return Err(exhausted(
                &replay,
                attempts,
                elapsed,
                error,
                ExhaustionBound::MaxAttempts,
            ));
        }
        let upper = policy.backoff_upper_bound(attempts - 1);
        let sleep = jitter.full_jitter(upper).min(upper);
        // Two independent bounds: the wall clock (caller deadline, else
        // `maximum_elapsed`) and the accumulated backoff budget. Stopping
        // before the FIRST replay is reported as its own bound so a caller
        // budget too small for the observed attempt latency is never filed as
        // "retried until exhausted".
        let backoff_exhausted = slept
            .checked_add(sleep)
            .is_none_or(|total| total > policy.maximum_elapsed);
        if backoff_exhausted || !sleep_fits(clock.now(), sleep, deadline) {
            let bound = if attempts <= 1 {
                ExhaustionBound::NoRetryWindow
            } else {
                ExhaustionBound::MaxElapsed
            };
            return Err(exhausted(&replay, attempts, elapsed, error, bound));
        }
        slept = slept.saturating_add(sleep);
        tracing::debug!(
            target: TARGET,
            attempt = attempts,
            class = ?class,
            sleep_ms = u64::try_from(sleep.as_millis()).unwrap_or(u64::MAX),
            replay_key = replay.key().unwrap_or(""),
            "surreal retry scheduled"
        );
        tokio::select! {
            biased;
            _ = wait_for_cancel(ctx.cancel.as_ref()) => {
                return Err(RetryError::Cancelled {
                    attempts,
                    elapsed: elapsed_since(clock, started),
                });
            }
            _ = clock.sleep(sleep) => {}
        }
        // Never START another attempt once the effective deadline has passed.
        // A running attempt is deliberately not raced against the deadline (an
        // acknowledged commit must never be dropped), so refusing the next
        // attempt is what bounds the whole loop; the caller's per-statement
        // bound, clamped to the remaining budget, bounds the attempt itself.
        if deadline.is_some_and(|deadline| clock.now() >= deadline) {
            let bound = if attempts <= 1 {
                ExhaustionBound::NoRetryWindow
            } else {
                ExhaustionBound::MaxElapsed
            };
            return Err(exhausted(
                &replay,
                attempts,
                elapsed_since(clock, started),
                error,
                bound,
            ));
        }
    }
}

/// Wall-clock bound for the whole loop, DERIVED (never a silent edit of
/// [`RetryPolicy::CONTRACT`], whose documented 5 ms / 250 ms / 8 / 2000 ms stay
/// exactly as the contract pins them):
///
/// * with a caller deadline (the whole-operation budget, e.g. one
///   `statement_timeout`), THAT is the wall-clock bound and
///   `maximum_elapsed` governs only the accumulated backoff. Taking the
///   earlier of the two instead would switch retrying OFF exactly under
///   saturation: one attempt may take a full `statement_timeout` (300 s
///   default) while `maximum_elapsed` is 2000 ms, i.e. 0.67% of a single
///   attempt's allowance, so the first conflict would end the loop with
///   `attempts = 1` and no replay ever scheduled.
/// * with no caller deadline, `maximum_elapsed` remains the wall-clock bound,
///   so an unbounded caller can never run `maximum_attempts` slow attempts
///   back to back.
fn effective_deadline(policy: &RetryPolicy, ctx: &RetryContext, started: Instant) -> Option<Instant> {
    ctx.deadline
        .or_else(|| started.checked_add(policy.maximum_elapsed))
}

fn elapsed_since<C: RetryClock>(clock: &C, started: Instant) -> Duration {
    clock.now().saturating_duration_since(started)
}

fn sleep_fits(now: Instant, sleep: Duration, deadline: Option<Instant>) -> bool {
    match deadline {
        None => true,
        Some(deadline) => now
            .checked_add(sleep)
            .is_some_and(|wake_at| wake_at <= deadline),
    }
}

async fn wait_for_cancel(cancel: Option<&CancellationToken>) {
    match cancel {
        Some(token) => token.cancelled().await,
        None => std::future::pending::<()>().await,
    }
}

fn exhausted<E: fmt::Display>(
    replay: &Replay,
    attempts: u32,
    elapsed: Duration,
    last: E,
    bound: ExhaustionBound,
) -> RetryError<E> {
    tracing::warn!(
        target: TARGET,
        attempts,
        elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        bound = bound.as_str(),
        replay_key = replay.key().unwrap_or(""),
        last_error = %last,
        "surreal retry exhausted"
    );
    RetryError::Exhausted {
        attempts,
        elapsed,
        last,
        bound,
    }
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
    matches!(
        error.query_details(),
        Some(QueryError::TransactionConflict)
    )
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

/// Deterministic clock for tests: `sleep` advances virtual time instantly and
/// records the requested duration; [`VirtualClock::advance`] simulates attempt
/// latency. Used by the in-crate tests and the MT-142 `tests/` swarm target.
#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(Debug)]
pub struct VirtualClock {
    origin: Instant,
    offset: std::sync::Mutex<Duration>,
    sleeps: std::sync::Mutex<Vec<Duration>>,
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl VirtualClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            offset: std::sync::Mutex::new(Duration::ZERO),
            sleeps: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn advance(&self, delta: Duration) {
        let mut offset = self
            .offset
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *offset = offset.saturating_add(delta);
    }

    pub fn elapsed(&self) -> Duration {
        *self
            .offset
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn recorded_sleeps(&self) -> Vec<Duration> {
        self.sleeps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl Default for VirtualClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl RetryClock for VirtualClock {
    fn now(&self) -> Instant {
        self.origin
            .checked_add(self.elapsed())
            .unwrap_or(self.origin)
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        self.advance(duration);
        self.sleeps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(duration);
        std::future::ready(())
    }
}

/// Test clock whose sleeps never complete, proving cancellation is observed
/// during a sleep rather than only before an attempt.
#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(Debug)]
pub struct HangingClock {
    origin: Instant,
    sleeps_started: AtomicU64,
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl HangingClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            sleeps_started: AtomicU64::new(0),
        }
    }

    pub fn sleeps_started(&self) -> u64 {
        self.sleeps_started.load(Ordering::SeqCst)
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl Default for HangingClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl RetryClock for HangingClock {
    fn now(&self) -> Instant {
        self.origin
    }

    fn sleep(&self, _duration: Duration) -> impl Future<Output = ()> + Send {
        self.sleeps_started.fetch_add(1, Ordering::SeqCst);
        std::future::pending::<()>()
    }
}

/// Scripted jitter for tests: returns the scripted durations in order and
/// then a fixed fallback. Values are returned unclamped so tests can prove
/// that [`retry`] clamps to the schedule bound.
#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(Debug)]
pub struct ScriptedJitter {
    script: std::sync::Mutex<std::collections::VecDeque<Duration>>,
    fallback: JitterFallback,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JitterFallback {
    /// Return the schedule upper bound (worst-case sleep).
    UpperBound,
    /// Return zero (fastest schedule).
    Zero,
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl ScriptedJitter {
    pub fn upper_bound() -> Self {
        Self::sequence(Vec::new(), JitterFallback::UpperBound)
    }

    pub fn zero() -> Self {
        Self::sequence(Vec::new(), JitterFallback::Zero)
    }

    pub fn sequence(script: Vec<Duration>, fallback: JitterFallback) -> Self {
        Self {
            script: std::sync::Mutex::new(script.into_iter().collect()),
            fallback,
        }
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
impl JitterSource for ScriptedJitter {
    fn full_jitter(&self, upper: Duration) -> Duration {
        let scripted = self
            .script
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop_front();
        match (scripted, self.fallback) {
            (Some(value), _) => value,
            (None, JitterFallback::UpperBound) => upper,
            (None, JitterFallback::Zero) => Duration::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{ready, Ready};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum FakeError {
        Transient,
        SnapshotChange,
        Terminal,
    }

    impl fmt::Display for FakeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let text = match self {
                Self::Transient => "transient conflict",
                Self::SnapshotChange => "snapshot change",
                Self::Terminal => "terminal failure",
            };
            f.write_str(text)
        }
    }

    fn classify_fake(error: &FakeError) -> RetryClass {
        match error {
            FakeError::Transient => RetryClass::RetryableTransient,
            FakeError::SnapshotChange => RetryClass::RetryableSnapshotChange,
            FakeError::Terminal => RetryClass::Terminal,
        }
    }

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn ms_list(values: &[u64]) -> Vec<Duration> {
        values.iter().copied().map(ms).collect()
    }

    /// Returns the scripted outcomes in order, then `Ok(attempt.number)`.
    fn scripted(
        outcomes: Vec<Result<u32, FakeError>>,
        seen: &mut Vec<RetryAttempt>,
    ) -> impl FnMut(RetryAttempt) -> Ready<Result<u32, FakeError>> + '_ {
        let mut outcomes: VecDeque<_> = outcomes.into_iter().collect();
        move |attempt| {
            seen.push(attempt);
            ready(outcomes.pop_front().unwrap_or(Ok(attempt.number)))
        }
    }

    #[derive(Clone, Default)]
    struct CapturedLog(Arc<Mutex<Vec<u8>>>);

    impl CapturedLog {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().expect("log mutex")).into_owned()
        }
    }

    impl Write for CapturedLog {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("log mutex").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
        type Writer = CapturedLog;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn conflict_not_executed() -> surrealdb::Error {
        surrealdb::Error::query(
            "The query was not executed due to a failed transaction. Transaction conflict: Resource busy. This transaction can be retried".to_string(),
            Some(QueryError::NotExecuted),
        )
    }

    #[test]
    fn contract_policy_matches_research_basis() {
        let policy = RetryPolicy::default();
        assert_eq!(policy, RetryPolicy::CONTRACT);
        assert_eq!(policy.base_delay, ms(5));
        assert_eq!(policy.maximum_delay, ms(250));
        assert_eq!(policy.maximum_attempts, 8);
        assert_eq!(policy.maximum_elapsed, ms(2000));
        let schedule: Vec<Duration> = (0..8).map(|n| policy.backoff_upper_bound(n)).collect();
        assert_eq!(schedule, ms_list(&[5, 10, 20, 40, 80, 160, 250, 250]));
        assert_eq!(policy.backoff_upper_bound(40), ms(250));
        assert_eq!(
            policy.with_maximum_attempts(0).effective_maximum_attempts(),
            1
        );
    }

    #[tokio::test]
    async fn first_attempt_success_makes_exactly_one_attempt() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            scripted(vec![Ok(7)], &mut seen),
        )
        .await;
        assert!(matches!(result, Ok(7)));
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].number, 1);
        assert_eq!(seen[0].retries, 0);
        assert!(clock.recorded_sleeps().is_empty());
    }

    #[tokio::test]
    async fn eventual_success_after_retryable_conflicts_sleeps_per_schedule() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            scripted(
                vec![
                    Err(FakeError::Transient),
                    Err(FakeError::Transient),
                    Err(FakeError::Transient),
                    Ok(9),
                ],
                &mut seen,
            ),
        )
        .await;
        assert!(matches!(result, Ok(9)));
        assert_eq!(seen.len(), 4);
        assert_eq!(seen.iter().map(|a| a.number).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert_eq!(seen.iter().map(|a| a.retries).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        assert_eq!(clock.recorded_sleeps(), ms_list(&[5, 10, 20]));
    }

    #[tokio::test]
    async fn terminal_error_is_not_retried() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            scripted(vec![Err(FakeError::Terminal), Ok(1)], &mut seen),
        )
        .await;
        match result {
            Err(RetryError::Terminal {
                attempts, error, ..
            }) => {
                assert_eq!(attempts, 1);
                assert_eq!(error, FakeError::Terminal);
            }
            other => panic!("expected Terminal, got {other:?}"),
        }
        assert_eq!(seen.len(), 1);
        assert!(clock.recorded_sleeps().is_empty());
    }

    #[tokio::test]
    async fn not_idempotent_is_never_retried_on_any_class() {
        for first in [FakeError::Transient, FakeError::SnapshotChange] {
            let clock = VirtualClock::new();
            let jitter = ScriptedJitter::upper_bound();
            let mut seen = Vec::new();
            let result = retry(
                &RetryPolicy::CONTRACT,
                &RetryContext::unbounded(),
                Replay::NotIdempotent,
                &clock,
                &jitter,
                classify_fake,
                scripted(vec![Err(first), Ok(1)], &mut seen),
            )
            .await;
            assert!(
                matches!(result, Err(RetryError::Terminal { attempts: 1, .. })),
                "{result:?}"
            );
            assert_eq!(seen.len(), 1);
            assert!(clock.recorded_sleeps().is_empty());
        }
    }

    #[tokio::test]
    async fn snapshot_change_is_retried_only_under_idempotent_replay() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::zero();
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("upsert:ws:key"),
            &clock,
            &jitter,
            classify_fake,
            scripted(vec![Err(FakeError::SnapshotChange), Ok(3)], &mut seen),
        )
        .await;
        assert!(matches!(result, Ok(3)));
        assert_eq!(seen.len(), 2);
        assert_eq!(clock.recorded_sleeps(), ms_list(&[0]));
    }

    #[tokio::test]
    async fn maximum_attempts_exhaustion_runs_eight_attempts_and_seven_sleeps() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            |attempt| {
                seen.push(attempt);
                ready(Err::<u32, _>(FakeError::Transient))
            },
        )
        .await;
        match result {
            Err(RetryError::Exhausted {
                attempts,
                bound,
                last,
                ..
            }) => {
                assert_eq!(attempts, 8);
                assert_eq!(bound, ExhaustionBound::MaxAttempts);
                assert_eq!(last, FakeError::Transient);
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
        assert_eq!(seen.len(), 8);
        assert_eq!(
            clock.recorded_sleeps(),
            ms_list(&[5, 10, 20, 40, 80, 160, 250])
        );
        assert_eq!(clock.elapsed(), ms(565));
    }

    #[tokio::test]
    async fn maximum_elapsed_exhaustion_never_sleeps_past_the_bound() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| {
                clock.advance(ms(600));
                ready(Err::<u32, _>(FakeError::Transient))
            },
        )
        .await;
        match result {
            Err(RetryError::Exhausted {
                attempts,
                bound,
                elapsed,
                ..
            }) => {
                assert_eq!(attempts, 4);
                assert_eq!(bound, ExhaustionBound::MaxElapsed);
                assert_eq!(elapsed, ms(2435));
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
        assert_eq!(clock.recorded_sleeps(), ms_list(&[5, 10, 20]));
        let mut wake_at = Duration::ZERO;
        for sleep in clock.recorded_sleeps() {
            wake_at += ms(600) + sleep;
            assert!(wake_at <= ms(2000), "sleep ended at {wake_at:?}");
        }
    }

    #[tokio::test]
    async fn caller_deadline_exhausts_as_max_elapsed() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(30));
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Transient)),
        )
        .await;
        match result {
            Err(RetryError::Exhausted {
                attempts, bound, ..
            }) => {
                assert_eq!(attempts, 3);
                assert_eq!(bound, ExhaustionBound::MaxElapsed);
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
        assert_eq!(clock.recorded_sleeps(), ms_list(&[5, 10]));
        assert_eq!(clock.elapsed(), ms(15));
    }

    /// The whole retried operation stays inside ONE caller budget: no attempt
    /// may START after the deadline, so a repeatedly-conflicting idempotent
    /// operation cannot multiply its per-attempt bound by `maximum_attempts`
    /// (the 8 x statement_timeout stall class). The outcome stays typed.
    #[tokio::test]
    async fn slow_repeated_conflicts_stay_inside_one_caller_budget() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        // Each attempt consumes most of the budget, as a slow engine statement
        // would; the budget is the caller's, not the policy's sleep bound.
        let budget = ms(1_000);
        let deadline = clock.now() + budget;
        let ctx = RetryContext::unbounded().with_deadline(deadline);
        let mut starts = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("slow"),
            &clock,
            &jitter,
            classify_fake,
            |attempt| {
                starts.push(attempt.elapsed);
                clock.advance(ms(300));
                ready(Err::<u32, _>(FakeError::Transient))
            },
        )
        .await;
        match result {
            Err(RetryError::Exhausted { bound, attempts, .. }) => {
                assert_eq!(bound, ExhaustionBound::MaxElapsed);
                assert!(
                    (1..=RetryPolicy::CONTRACT.maximum_attempts).contains(&attempts),
                    "attempts {attempts} must stay within the policy"
                );
            }
            other => panic!("expected a typed MaxElapsed exhaustion, got {other:?}"),
        }
        assert!(
            starts.iter().all(|start| *start <= budget),
            "no attempt may start after the caller deadline: {starts:?}"
        );
        // Four 300 ms attempts would overshoot a 1 s budget; the loop stops at
        // three plus their sleeps rather than running all eight.
        assert!(
            starts.len() <= 4,
            "the budget must cap the attempt count, ran {}",
            starts.len()
        );
    }

    /// A deadline reached exactly at a sleep's wake-up must stop the loop
    /// before the next attempt starts.
    #[tokio::test]
    async fn an_attempt_never_starts_once_the_deadline_is_reached() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::sequence(vec![ms(5)], JitterFallback::Zero);
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(100));
        let mut starts = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("edge"),
            &clock,
            &jitter,
            classify_fake,
            |attempt| {
                starts.push(attempt.elapsed);
                clock.advance(ms(95));
                ready(Err::<u32, _>(FakeError::Transient))
            },
        )
        .await;
        // One attempt ran and no replay ever did, so this is the distinct
        // no-retry-window bound, not retry exhaustion.
        assert!(
            matches!(
                result,
                Err(RetryError::Exhausted {
                    bound: ExhaustionBound::NoRetryWindow,
                    attempts: 1,
                    ..
                })
            ),
            "{result:?}"
        );
        assert_eq!(starts, vec![Duration::ZERO]);
        assert_eq!(clock.elapsed(), ms(100), "the loop stops exactly at the deadline");
    }

    /// Regression guard for the review's open MAJOR: an attempt may legitimately
    /// take far longer than `maximum_elapsed` (one `statement_timeout` is 300 s
    /// against a 2000 ms backoff budget). The replay must still be scheduled,
    /// because the caller deadline - not the backoff budget - is the wall-clock
    /// bound. Under the previous `min(start + maximum_elapsed, caller)`
    /// derivation this returned `attempts = 1` with no replay at all.
    #[tokio::test]
    async fn a_slow_attempt_still_gets_its_replay_under_a_caller_budget() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(10_000));
        let mut attempts_seen = 0u32;
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("slow-but-retryable"),
            &clock,
            &jitter,
            classify_fake,
            |attempt| {
                attempts_seen = attempt.number;
                // 5 s per attempt: 2500x the 2000 ms backoff budget.
                clock.advance(ms(5_000));
                ready(if attempt.number >= 2 {
                    Ok(7)
                } else {
                    Err(FakeError::Transient)
                })
            },
        )
        .await;
        assert!(matches!(result, Ok(7)), "{result:?}");
        assert_eq!(attempts_seen, 2, "the conflict must get its replay");
        assert_eq!(clock.recorded_sleeps(), ms_list(&[5]));
    }

    /// The two budget outcomes are distinguishable in the typed error and in
    /// the diagnostic field, so a no-context model can tell "the budget was
    /// too small to retry at all" from "replays ran and did not converge".
    #[tokio::test]
    async fn no_retry_window_is_reported_separately_from_exhaustion() {
        // Budget smaller than the shortest backoff: no replay is possible.
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(1));
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("tiny-budget"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Transient)),
        )
        .await;
        match result {
            Err(RetryError::Exhausted {
                bound, attempts, ..
            }) => {
                assert_eq!(bound, ExhaustionBound::NoRetryWindow);
                assert!(bound.is_no_retry_window());
                assert_eq!(bound.as_str(), "no_retry_window");
                assert_eq!(attempts, 1);
            }
            other => panic!("expected NoRetryWindow, got {other:?}"),
        }
        assert!(clock.recorded_sleeps().is_empty(), "no replay was scheduled");

        // A budget that admits replays reports genuine exhaustion instead.
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(30));
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("small-budget"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Transient)),
        )
        .await;
        match result {
            Err(RetryError::Exhausted {
                bound, attempts, ..
            }) => {
                assert_eq!(bound, ExhaustionBound::MaxElapsed);
                assert!(!bound.is_no_retry_window());
                assert!(attempts > 1, "replays ran before the budget ended");
            }
            other => panic!("expected MaxElapsed, got {other:?}"),
        }
    }

    /// `maximum_elapsed` still caps the ACCUMULATED backoff even when the
    /// caller budget is large, so the schedule cannot grow without bound.
    #[tokio::test]
    async fn accumulated_backoff_stays_within_maximum_elapsed() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let policy = RetryPolicy::CONTRACT
            .with_maximum_attempts(64)
            .with_maximum_elapsed(ms(100));
        let ctx = RetryContext::unbounded().with_deadline(clock.now() + ms(600_000));
        let result = retry(
            &policy,
            &ctx,
            Replay::idempotent("backoff-budget"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Transient)),
        )
        .await;
        assert!(
            matches!(
                result,
                Err(RetryError::Exhausted {
                    bound: ExhaustionBound::MaxElapsed,
                    ..
                })
            ),
            "{result:?}"
        );
        let total: Duration = clock.recorded_sleeps().into_iter().sum();
        assert!(
            total <= ms(100),
            "accumulated backoff {total:?} must stay within maximum_elapsed"
        );
    }

    #[tokio::test]
    async fn cancellation_before_the_first_attempt_runs_nothing() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let token = CancellationToken::new();
        token.cancel();
        let ctx = RetryContext::unbounded().with_cancel(token);
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            scripted(vec![Ok(1)], &mut seen),
        )
        .await;
        assert!(matches!(result, Err(RetryError::Cancelled { attempts: 0, .. })));
        assert!(seen.is_empty());
    }

    #[tokio::test]
    async fn cancellation_during_a_sleep_is_observed() {
        let clock = HangingClock::new();
        let jitter = ScriptedJitter::upper_bound();
        let token = CancellationToken::new();
        let ctx = RetryContext::unbounded().with_cancel(token.clone());
        let result = retry(
            &RetryPolicy::CONTRACT,
            &ctx,
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| {
                let token = token.clone();
                tokio::spawn(async move { token.cancel() });
                ready(Err::<u32, _>(FakeError::Transient))
            },
        )
        .await;
        assert!(matches!(result, Err(RetryError::Cancelled { attempts: 1, .. })));
        assert_eq!(clock.sleeps_started(), 1);
    }

    #[tokio::test]
    async fn every_sleep_is_clamped_to_the_schedule_bound() {
        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::sequence(
            vec![Duration::from_secs(10), ms(3)],
            JitterFallback::UpperBound,
        );
        let mut seen = Vec::new();
        let result = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("k"),
            &clock,
            &jitter,
            classify_fake,
            scripted(
                vec![
                    Err(FakeError::Transient),
                    Err(FakeError::Transient),
                    Err(FakeError::Transient),
                    Ok(1),
                ],
                &mut seen,
            ),
        )
        .await;
        assert!(matches!(result, Ok(1)));
        assert_eq!(clock.recorded_sleeps(), ms_list(&[5, 3, 20]));
    }

    #[test]
    fn system_jitter_stays_within_the_inclusive_upper_bound() {
        let jitter = SystemJitter::new();
        for upper in [Duration::ZERO, Duration::from_nanos(1), ms(5), ms(250)] {
            for _ in 0..1000 {
                assert!(jitter.full_jitter(upper) <= upper);
            }
        }
        let distinct: std::collections::HashSet<Duration> =
            (0..64).map(|_| jitter.full_jitter(ms(250))).collect();
        assert!(distinct.len() > 1, "jitter must vary between samples");
    }

    #[tokio::test]
    async fn exhaustion_diagnostic_fires_exactly_once() {
        let log = CapturedLog::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(log.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let clock = VirtualClock::new();
        let jitter = ScriptedJitter::zero();
        let exhausted = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("replay-key-1"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Transient)),
        )
        .await;
        assert!(matches!(exhausted, Err(RetryError::Exhausted { .. })));
        let text = log.text();
        assert_eq!(text.matches("surreal retry exhausted").count(), 1, "{text}");
        assert!(text.contains("attempts=8"), "{text}");
        assert!(text.contains("bound=\"max_attempts\""), "{text}");
        assert!(text.contains("replay_key=\"replay-key-1\""), "{text}");
        assert!(text.contains("last_error=transient conflict"), "{text}");
        assert!(text.contains("elapsed_ms="), "{text}");

        let terminal = retry(
            &RetryPolicy::CONTRACT,
            &RetryContext::unbounded(),
            Replay::idempotent("replay-key-2"),
            &clock,
            &jitter,
            classify_fake,
            |_attempt| ready(Err::<u32, _>(FakeError::Terminal)),
        )
        .await;
        assert!(matches!(terminal, Err(RetryError::Terminal { .. })));
        assert_eq!(log.text().matches("surreal retry exhausted").count(), 1);
    }

    #[test]
    fn retry_error_display_and_accessors() {
        let error: RetryError<FakeError> = RetryError::Exhausted {
            attempts: 8,
            elapsed: ms(565),
            last: FakeError::Transient,
            bound: ExhaustionBound::MaxAttempts,
        };
        assert_eq!(error.attempts(), 8);
        assert_eq!(error.elapsed(), ms(565));
        assert_eq!(error.error(), Some(&FakeError::Transient));
        assert_eq!(
            error.to_string(),
            "retry exhausted (max_attempts) after 8 attempt(s) in 565 ms: transient conflict"
        );
        let cancelled: RetryError<FakeError> = RetryError::Cancelled {
            attempts: 2,
            elapsed: ms(7),
        };
        assert!(cancelled.error().is_none());
        assert!(cancelled.into_error().is_none());
    }

    #[test]
    fn classify_surreal_error_accepts_typed_transaction_conflict() {
        let error = surrealdb::Error::query(
            "anything".to_string(),
            Some(QueryError::TransactionConflict),
        );
        assert_eq!(classify_surreal_error(&error), RetryClass::RetryableTransient);
    }

    #[test]
    fn classify_surreal_error_accepts_erased_shapes_with_both_markers() {
        assert_eq!(
            classify_surreal_error(&conflict_not_executed()),
            RetryClass::RetryableTransient
        );
        let commit_row = surrealdb::Error::query(
            "Cannot COMMIT: Transaction conflict: Resource busy. This transaction can be retried".to_string(),
            Some(QueryError::NotExecuted),
        );
        assert_eq!(classify_surreal_error(&commit_row), RetryClass::RetryableTransient);
        let client_side = surrealdb::Error::internal(
            "Transaction conflict: Operation failed. Try again.: insufficient history. This transaction can be retried".to_string(),
        );
        assert_eq!(classify_surreal_error(&client_side), RetryClass::RetryableTransient);
    }

    #[test]
    fn classify_surreal_error_accepts_unwrapped_rocksdb_status_only() {
        let busy = surrealdb::Error::internal("Resource busy: ".to_string());
        assert_eq!(classify_surreal_error(&busy), RetryClass::RetryableTransient);
        let try_again = surrealdb::Error::internal("Operation failed. Try again.".to_string());
        assert_eq!(classify_surreal_error(&try_again), RetryClass::RetryableTransient);
        let wrapped = surrealdb::Error::internal(
            "IO error: Failed to create lock file: store/LOCK (Resource busy)".to_string(),
        );
        assert_eq!(classify_surreal_error(&wrapped), RetryClass::Terminal);
    }

    #[test]
    fn classify_surreal_error_inspects_the_cause_chain() {
        let outer = surrealdb::Error::internal("router relay".to_string())
            .with_cause(conflict_not_executed());
        assert_eq!(classify_surreal_error(&outer), RetryClass::RetryableTransient);
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
            assert_eq!(classify_surreal_error(&error), RetryClass::Terminal, "{error:?}");
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
        assert_eq!(classify_surreal_storage_error(&commit), RetryClass::RetryableTransient);
        assert_eq!(
            classify_surreal_storage_error(&SurrealStorageError::Closed),
            RetryClass::Terminal
        );

        let rendered = StorageError::from(transient);
        assert_eq!(classify_storage_error(&rendered), RetryClass::RetryableTransient);
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
        assert_eq!(TRANSACTION_RETRYABLE_MARKER, "This transaction can be retried");
    }

    #[test]
    fn unique_index_violation_extracts_the_index_name() {
        assert_eq!(
            is_unique_index_violation(
                "embedded database error: Database index `uq_knowledge_entities_identity` already contains ['ws', 'k'], with record `knowledge_entities:x`"
            ),
            Some("uq_knowledge_entities_identity".to_string())
        );
        assert_eq!(is_unique_index_violation("Database index `` already contains"), None);
        assert_eq!(is_unique_index_violation("Database index `x` is missing"), None);
        assert_eq!(is_unique_index_violation("Transaction conflict: Resource busy"), None);
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
                assert_eq!(budget_ms, 1_500, "budget_ms is the configured statement timeout");
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
