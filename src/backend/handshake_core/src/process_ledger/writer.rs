use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{self, error::TrySendError, OwnedPermit, Receiver, Sender},
        oneshot, Mutex, Notify, OnceCell,
    },
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};

use super::reclaim::{
    assert_process_ledger_authority_relation, force_all_constraints_immediate,
    lock_process_ledger_authority_relation, pin_transaction_search_path,
    require_postgres_crash_durability, require_synchronous_commit,
    resolve_process_ledger_authority_relation, ProcessLedgerAuthorityLockMode,
    ProcessLedgerAuthorityRelation,
};
use super::table::{
    LedgerEvent, LedgerEventKind, ProcessStart, ProcessStop, PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
    PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS,
    PROCESS_LEDGER_MIGRATION_SQL, PROCESS_START_INSERT_SQL, PROCESS_STOP_UPSERT_SQL,
};

pub const FR_EVT_LEDGER_OVERFLOW: &str = "FR_EVT_LEDGER_OVERFLOW";
const PROCESS_LEDGER_SOURCE_COMPONENT: &str = "process_ledger_writer";
const PROCESS_LEDGER_AUTHORITY_LOCK_TIMEOUT: &str = "2000ms";

/// Upper bound on the writer's in-flight START identity index.
///
/// One entry exists per process whose START row was accepted by the writer and
/// whose STOP row has not been accepted yet, so the steady-state size is the
/// number of live Handshake-owned processes. The cap only protects against an
/// unbounded leak of never-stopped lifecycles; reaching it is logged, never
/// silent.
const PROCESS_LEDGER_START_INDEX_CAPACITY: usize = 65_536;

static GLOBAL_DEGRADED_WRITERS: AtomicUsize = AtomicUsize::new(0);

/// Process-wide cumulative row-attempt volume across failed store writes.
///
/// The writer retains a failed batch and retries it, so the same row is counted
/// again on each failed attempt and may later persist successfully. This metric
/// exposes write instability; it is not a unique-row or durable-loss counter.
/// `FR-EVT-LEDGER-OVERFLOW` remains the separate enqueue-loss signal.
static GLOBAL_LEDGER_FLUSH_FAILED_ROWS: AtomicU64 = AtomicU64::new(0);

pub fn is_degraded() -> bool {
    GLOBAL_DEGRADED_WRITERS.load(Ordering::SeqCst) > 0
}

/// Cumulative row-attempt volume across failed store writes process-wide.
///
/// Non-zero means at least one write attempt failed. Rows are retained for
/// retry, so use the per-attempt error logs and eventual drain outcome to judge
/// current durability rather than interpreting this as permanent row loss.
pub fn flush_failed_row_count() -> u64 {
    GLOBAL_LEDGER_FLUSH_FAILED_ROWS.load(Ordering::SeqCst)
}

#[derive(Debug, Error)]
pub enum ProcessLedgerError {
    #[error("PROCESS_LEDGER_INVALID_CONFIG: {0}")]
    InvalidConfig(String),
    #[error("PROCESS_LEDGER_ENQUEUE_DROPPED: {0}")]
    EnqueueDropped(String),
    #[error("PROCESS_LEDGER_OVERFLOW_EMIT: {0}")]
    OverflowEmit(String),
    #[error("PROCESS_LEDGER_STORE: {0}")]
    Store(String),
    #[error("PROCESS_LEDGER_POSTGRES: {source}")]
    Postgres { source: sqlx::Error },
    #[error("PROCESS_LEDGER_EVENT: {0}")]
    Event(String),
    #[error("PROCESS_LEDGER_START_IDENTITY_CONFLICT: process_uuid {process_uuid} already belongs to a different lifecycle")]
    StartIdentityConflict {
        process_uuid: Uuid,
        conflicting_start: Box<ProcessStart>,
    },
    #[error("PROCESS_LEDGER_STOP_IDENTITY_CONFLICT: process_uuid {process_uuid} STOP does not match the authoritative lifecycle or current reclaim claim")]
    StopIdentityConflict {
        process_uuid: Uuid,
        conflicting_stop: Box<ProcessStop>,
    },
    #[error("PROCESS_LEDGER_DURABILITY_ACK_LOST: {event_kind} row for process_uuid {process_uuid} lost its store acknowledgement because the ledger writer terminated")]
    DurabilityAckLost {
        event_kind: String,
        process_uuid: Uuid,
    },
    #[error("PROCESS_LEDGER_DURABILITY_ACK_TIMEOUT: {event_kind} row for process_uuid {process_uuid} was not durably acknowledged within {timeout_ms} ms")]
    DurabilityAckTimeout {
        event_kind: String,
        process_uuid: Uuid,
        timeout_ms: u128,
    },
    #[error("PROCESS_LEDGER_DURABILITY_REJECTED: {event_kind} row for process_uuid {process_uuid} was rejected by the authoritative store: {reason}")]
    DurabilityRejected {
        event_kind: String,
        process_uuid: Uuid,
        reason: String,
    },
}

impl From<sqlx::Error> for ProcessLedgerError {
    fn from(source: sqlx::Error) -> Self {
        Self::Postgres { source }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriterConfig {
    pub capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
}

impl WriterConfig {
    pub fn for_work_profile(capacity: Option<usize>) -> Self {
        Self {
            capacity: capacity.unwrap_or(PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY),
            ..Self::default()
        }
    }

    fn validate(self) -> Result<Self, ProcessLedgerError> {
        if self.capacity == 0 {
            return Err(ProcessLedgerError::InvalidConfig(
                "capacity must be greater than zero".to_string(),
            ));
        }
        if self.batch_size == 0 {
            return Err(ProcessLedgerError::InvalidConfig(
                "batch_size must be greater than zero".to_string(),
            ));
        }
        if self.batch_size > self.capacity {
            return Err(ProcessLedgerError::InvalidConfig(format!(
                "batch_size {} must not exceed capacity {}",
                self.batch_size, self.capacity
            )));
        }
        if self.flush_interval.is_zero() {
            return Err(ProcessLedgerError::InvalidConfig(
                "flush_interval must be greater than zero".to_string(),
            ));
        }
        Ok(self)
    }
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            capacity: PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY,
            batch_size: PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
            flush_interval: Duration::from_millis(PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS),
        }
    }
}

#[async_trait]
pub trait ProcessLedgerStore: Send + Sync + 'static {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError>;
}

pub trait ProcessLedgerOverflowSink: Send + Sync + 'static {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerOverflowEvent {
    pub event_type: String,
    pub overflow_uuid: Uuid,
    pub overflow_count: u64,
    pub capacity: usize,
    pub dropped_event_kind: LedgerEventKind,
    pub sampled_event_payload: Value,
    pub emitted_at_utc: DateTime<Utc>,
}

impl LedgerOverflowEvent {
    pub fn new(overflow_count: u64, capacity: usize, dropped_event: LedgerEvent) -> Self {
        Self {
            event_type: FR_EVT_LEDGER_OVERFLOW.to_string(),
            overflow_uuid: Uuid::now_v7(),
            overflow_count,
            capacity,
            dropped_event_kind: dropped_event.kind(),
            sampled_event_payload: dropped_event.sampled_payload(),
            emitted_at_utc: Utc::now(),
        }
    }

    pub fn to_kernel_event(&self) -> Result<NewKernelEvent, ProcessLedgerError> {
        let process_uuid = self
            .sampled_event_payload
            .get("process_uuid")
            .and_then(Value::as_str)
            .unwrap_or("unknown-process");
        let session_run_id = self
            .sampled_event_payload
            .get("parent_session_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("SR-PROCESS-LEDGER-{}", self.overflow_uuid));
        let payload = json!({
            "event_type": FR_EVT_LEDGER_OVERFLOW,
            "overflow_uuid": self.overflow_uuid.to_string(),
            "overflow_count": self.overflow_count,
            "capacity": self.capacity,
            "dropped_event_kind": self.dropped_event_kind.as_str(),
            "sampled_event_payload": self.sampled_event_payload,
            "emitted_at_utc": self.emitted_at_utc,
        });

        NewKernelEvent::builder(
            format!("KTR-PROCESS-LEDGER-{}", self.overflow_uuid),
            session_run_id,
            KernelEventType::FrEvtLedgerOverflow,
            KernelActor::System(PROCESS_LEDGER_SOURCE_COMPONENT.to_string()),
        )
        .aggregate("process_lifecycle", process_uuid.to_string())
        .idempotency_key(format!(
            "{FR_EVT_LEDGER_OVERFLOW}:{}:{}",
            process_uuid, self.overflow_uuid
        ))
        .correlation_id(self.overflow_uuid.to_string())
        .source_component(PROCESS_LEDGER_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|error| ProcessLedgerError::Event(error.to_string()))
    }
}

/// The identity bookkeeping a queued row triggers once the writer has actually
/// accepted it: a START publishes its identity for the eventual terminal owner,
/// a STOP releases it.
enum AcceptedEventIdentity {
    Start(Box<ProcessStart>),
    Stop(Uuid),
}

impl AcceptedEventIdentity {
    fn of(event: &LedgerEvent) -> Self {
        match event {
            LedgerEvent::Start(start) => Self::Start(Box::new(start.clone())),
            LedgerEvent::Stop(stop) => Self::Stop(stop.process_uuid),
        }
    }
}

/// One accepted writer row plus an optional acknowledgement that resolves only
/// after the row's complete store batch has committed successfully.
struct LedgerWriteRequest {
    event: LedgerEvent,
    durable_ack: Option<oneshot::Sender<Result<(), String>>>,
    stop_authorized: Option<Arc<AtomicBool>>,
}

impl LedgerWriteRequest {
    fn unacknowledged(event: LedgerEvent) -> Self {
        Self {
            event,
            durable_ack: None,
            stop_authorized: None,
        }
    }

    fn acknowledged(
        event: LedgerEvent,
        durable_ack: oneshot::Sender<Result<(), String>>,
        stop_authorized: Arc<AtomicBool>,
    ) -> Self {
        Self {
            event,
            durable_ack: Some(durable_ack),
            stop_authorized: Some(stop_authorized),
        }
    }

    fn lifecycle_start(event: LedgerEvent, stop_authorized: Arc<AtomicBool>) -> Self {
        Self {
            event,
            durable_ack: None,
            stop_authorized: Some(stop_authorized),
        }
    }

    fn lifecycle_stop(event: LedgerEvent, stop_authorized: Arc<AtomicBool>) -> Self {
        Self {
            event,
            durable_ack: None,
            stop_authorized: Some(stop_authorized),
        }
    }
}

/// Awaitable proof that one accepted ledger row reached the authoritative
/// [`ProcessLedgerStore`]. Queue acceptance alone is not durability.
pub struct ProcessLedgerDurabilityAck {
    receiver: oneshot::Receiver<Result<(), String>>,
    process_uuid: Uuid,
    event_kind: LedgerEventKind,
}

impl ProcessLedgerDurabilityAck {
    pub async fn wait_unbounded(self) -> Result<(), ProcessLedgerError> {
        match self.receiver.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(reason)) => Err(ProcessLedgerError::DurabilityRejected {
                event_kind: self.event_kind.as_str().to_string(),
                process_uuid: self.process_uuid,
                reason,
            }),
            Err(_closed) => Err(ProcessLedgerError::DurabilityAckLost {
                event_kind: self.event_kind.as_str().to_string(),
                process_uuid: self.process_uuid,
            }),
        }
    }

    pub async fn wait(self, timeout: Duration) -> Result<(), ProcessLedgerError> {
        match time::timeout(timeout, self.receiver).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(reason))) => Err(ProcessLedgerError::DurabilityRejected {
                event_kind: self.event_kind.as_str().to_string(),
                process_uuid: self.process_uuid,
                reason,
            }),
            Ok(Err(_closed)) => Err(ProcessLedgerError::DurabilityAckLost {
                event_kind: self.event_kind.as_str().to_string(),
                process_uuid: self.process_uuid,
            }),
            Err(_elapsed) => Err(ProcessLedgerError::DurabilityAckTimeout {
                event_kind: self.event_kind.as_str().to_string(),
                process_uuid: self.process_uuid,
                timeout_ms: timeout.as_millis(),
            }),
        }
    }
}

pub struct ProcessLedgerWriter {
    sender: Sender<LedgerWriteRequest>,
    overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    degraded: Arc<AtomicBool>,
    overflow_count: Arc<AtomicU64>,
    flush_failed_rows: Arc<AtomicU64>,
    capacity: usize,
    /// WP-1 MT-013 (F1 graceful shutdown): fired by [`Self::begin_close`] to tell
    /// the spawned `run_writer` loop to stop accepting new rows, flush everything
    /// already queued (including a just-enqueued embedded-model STOP row), and
    /// terminate so its `JoinHandle` resolves. Signalling via `Notify` (rather
    /// than dropping every `Sender` clone) lets shutdown drain deterministically
    /// even though a `LedgerBatcher` clone is still held alive inside
    /// `AppState.llm_client`.
    close_notify: Arc<Notify>,
    /// START rows this writer accepted whose matching STOP row has not been
    /// accepted yet, keyed by `process_uuid`.
    ///
    /// The authoritative STOP upsert only updates a lifecycle row when every
    /// immutable identity column of the STOP is byte-identical to the persisted
    /// START (see `PROCESS_STOP_UPSERT_SQL`). A terminal owner that did not
    /// author the START therefore cannot synthesize a valid STOP from its own
    /// defaults: `started_at`, `wp_id`/`mt_id` lineage, and `metadata_jsonb`
    /// diverge and PostgreSQL rejects the row as a STOP identity conflict. This
    /// index makes the accepted START identity retrievable so the terminal path
    /// can derive a symmetric STOP instead of inventing a conflicting one.
    recorded_starts: Arc<std::sync::Mutex<HashMap<Uuid, ProcessStart>>>,
}

/// Capacity reserved for one complete resource lifecycle before that resource
/// is opened. Neither permit has emitted a row yet.
///
/// The START and STOP permits are owned (not borrowed from a `LedgerBatcher`),
/// so the STOP guarantee can travel with a long-lived runtime or subprocess.
/// Dropping an unused reservation returns both slots without fabricating rows.
pub struct ReservedProcessLifecycle {
    start_permit: Option<OwnedPermit<LedgerWriteRequest>>,
    stop_permit: Option<OwnedPermit<LedgerWriteRequest>>,
    runtime_owner: Option<super::ProcessRuntimeOwner>,
}

/// Queue authority reserved before a reclaimer is allowed to terminate an
/// owned process. Committing the reservation returns store-level durability
/// acknowledgement; merely consuming the permit is not reported as STOP.
pub struct ReservedProcessStop {
    permit: Option<OwnedPermit<LedgerWriteRequest>>,
}

impl ReservedProcessStop {
    pub fn commit_with_durable_ack(
        mut self,
        stop: ProcessStop,
    ) -> Result<ProcessLedgerDurabilityAck, ProcessLedgerError> {
        let permit = self.permit.take().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "reserved process STOP is missing its queue permit".to_string(),
            )
        })?;
        let process_uuid = stop.process_uuid;
        let (durable_ack, receiver) = oneshot::channel();
        permit.send(LedgerWriteRequest::acknowledged(
            LedgerEvent::Stop(stop),
            durable_ack,
            Arc::new(AtomicBool::new(true)),
        ));
        Ok(ProcessLedgerDurabilityAck {
            receiver,
            process_uuid,
            event_kind: LedgerEventKind::Stop,
        })
    }
}

impl ReservedProcessLifecycle {
    /// Emit the real START after the resource has a real identity. Both permits
    /// were accepted before resource access, so this transition cannot observe
    /// a later full/closed queue.
    pub fn begin(
        mut self,
        mut start: ProcessStart,
    ) -> Result<ActiveProcessLifecycle, ProcessLedgerError> {
        if start.runtime_owner.is_none() {
            start.runtime_owner = self.runtime_owner.clone();
        }
        let start_permit = self.start_permit.take().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "reserved lifecycle is missing its START permit".to_string(),
            )
        })?;
        let stop_permit = self.stop_permit.take().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "reserved lifecycle is missing its STOP permit".to_string(),
            )
        })?;
        let stop_authorized = Arc::new(AtomicBool::new(true));
        start_permit.send(LedgerWriteRequest::lifecycle_start(
            LedgerEvent::Start(start.clone()),
            Arc::clone(&stop_authorized),
        ));
        Ok(ActiveProcessLifecycle {
            start,
            stop_permit: std::sync::Mutex::new(Some(stop_permit)),
            left_open_for_reconciliation: AtomicBool::new(false),
            stop_durability_unconfirmed: AtomicBool::new(false),
            auto_stop_on_drop: true,
            stop_authorized,
        })
    }

    /// Begin the lifecycle while returning a store-level START acknowledgement.
    /// STOP authority remains disabled until the store confirms the START. A
    /// timeout, lost acknowledgement, or identity rejection therefore leaves
    /// the START open for reconciliation instead of risking a false STOP.
    pub fn begin_with_durable_ack(
        mut self,
        mut start: ProcessStart,
    ) -> Result<(ActiveProcessLifecycle, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        if start.runtime_owner.is_none() {
            start.runtime_owner = self.runtime_owner.clone();
        }
        let start_permit = self.start_permit.take().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "reserved lifecycle is missing its START permit".to_string(),
            )
        })?;
        let stop_permit = self.stop_permit.take().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "reserved lifecycle is missing its STOP permit".to_string(),
            )
        })?;
        let (durable_ack, receiver) = oneshot::channel();
        let process_uuid = start.process_uuid;
        let stop_authorized = Arc::new(AtomicBool::new(false));
        start_permit.send(LedgerWriteRequest::acknowledged(
            LedgerEvent::Start(start.clone()),
            durable_ack,
            Arc::clone(&stop_authorized),
        ));
        Ok((
            ActiveProcessLifecycle {
                start,
                stop_permit: std::sync::Mutex::new(Some(stop_permit)),
                left_open_for_reconciliation: AtomicBool::new(false),
                stop_durability_unconfirmed: AtomicBool::new(false),
                // Queue acceptance is not authority. Until the caller observes
                // the durable ACK, Drop must conservatively leave the lifecycle
                // open instead of fabricating a STOP for a START that may have
                // been rejected as an identity conflict.
                auto_stop_on_drop: false,
                stop_authorized,
            },
            ProcessLedgerDurabilityAck {
                receiver,
                process_uuid,
                event_kind: LedgerEventKind::Start,
            },
        ))
    }

    /// Integration-test access to the production durable-START transition.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn begin_with_durable_ack_for_test(
        self,
        start: ProcessStart,
    ) -> Result<(ActiveProcessLifecycle, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        self.begin_with_durable_ack(start)
    }

    pub(crate) fn with_runtime_owner(mut self, owner: Option<super::ProcessRuntimeOwner>) -> Self {
        self.runtime_owner = owner;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopRecordOutcome {
    Recorded,
    AlreadyStopped,
    LeftOpenForReconciliation,
    /// The STOP was queued, but the caller lost or timed out waiting for the
    /// store verdict. It may still commit later; this is never a graceful-
    /// shutdown proof and is distinct from a STOP deliberately not published.
    DurabilityUnconfirmed,
}

/// A STARTed lifecycle with capacity for exactly one matching STOP held for its
/// complete lifetime.
///
/// The mutex serializes concurrent shutdown callers. The first caller consumes
/// the reserved STOP permit synchronously; later callers distinguish
/// `AlreadyStopped` from `LeftOpenForReconciliation`, never reporting a
/// graceful success after another path deliberately abandoned STOP authority.
pub struct ActiveProcessLifecycle {
    start: ProcessStart,
    stop_permit: std::sync::Mutex<Option<OwnedPermit<LedgerWriteRequest>>>,
    left_open_for_reconciliation: AtomicBool,
    stop_durability_unconfirmed: AtomicBool,
    auto_stop_on_drop: bool,
    stop_authorized: Arc<AtomicBool>,
}

impl ActiveProcessLifecycle {
    pub fn process_uuid(&self) -> Uuid {
        self.start.process_uuid
    }

    pub fn start(&self) -> &ProcessStart {
        &self.start
    }

    pub fn stop(
        &self,
        exit_code: Option<i32>,
        reason: &str,
    ) -> Result<StopRecordOutcome, ProcessLedgerError> {
        let mut permit = match self.stop_permit.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(permit) = permit.take() else {
            return Ok(
                if self.left_open_for_reconciliation.load(Ordering::SeqCst) {
                    StopRecordOutcome::LeftOpenForReconciliation
                } else {
                    StopRecordOutcome::AlreadyStopped
                },
            );
        };
        if !self.stop_authorized.load(Ordering::SeqCst) {
            self.left_open_for_reconciliation
                .store(true, Ordering::SeqCst);
            drop(permit);
            return Ok(StopRecordOutcome::LeftOpenForReconciliation);
        }
        let stop = ProcessStop::from_start(&self.start, exit_code).with_stop_reason(reason);
        permit.send(LedgerWriteRequest::lifecycle_stop(
            LedgerEvent::Stop(stop),
            Arc::clone(&self.stop_authorized),
        ));
        Ok(StopRecordOutcome::Recorded)
    }

    /// Publish the matching STOP and wait until the authoritative store has
    /// durably accepted it.
    ///
    /// Queue acceptance is not a graceful-shutdown proof. Runtime and process
    /// owners that are about to release their last liveness handle must use
    /// this transition and retain ownership until it returns `Recorded`.
    /// Store rejection marks the lifecycle open for reconciliation. Timeout or
    /// writer-ack loss is recorded separately as durability-unconfirmed because
    /// the already-queued STOP may still commit later. Neither state is a
    /// graceful-shutdown proof.
    pub async fn stop_with_durable_ack(
        &self,
        exit_code: Option<i32>,
        reason: &str,
        timeout: Duration,
    ) -> Result<StopRecordOutcome, ProcessLedgerError> {
        let durable_ack = {
            let mut permit = match self.stop_permit.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let Some(permit) = permit.take() else {
                return Ok(if self.stop_durability_unconfirmed.load(Ordering::SeqCst) {
                    StopRecordOutcome::DurabilityUnconfirmed
                } else if self.left_open_for_reconciliation.load(Ordering::SeqCst) {
                    StopRecordOutcome::LeftOpenForReconciliation
                } else {
                    StopRecordOutcome::AlreadyStopped
                });
            };
            if !self.stop_authorized.load(Ordering::SeqCst) {
                self.left_open_for_reconciliation
                    .store(true, Ordering::SeqCst);
                drop(permit);
                return Ok(StopRecordOutcome::LeftOpenForReconciliation);
            }

            let stop = ProcessStop::from_start(&self.start, exit_code).with_stop_reason(reason);
            let process_uuid = stop.process_uuid;
            let (durable_ack, receiver) = oneshot::channel();
            permit.send(LedgerWriteRequest::acknowledged(
                LedgerEvent::Stop(stop),
                durable_ack,
                Arc::clone(&self.stop_authorized),
            ));
            ProcessLedgerDurabilityAck {
                receiver,
                process_uuid,
                event_kind: LedgerEventKind::Stop,
            }
        };

        match durable_ack.wait(timeout).await {
            Ok(()) => Ok(StopRecordOutcome::Recorded),
            Err(error) => {
                if matches!(error, ProcessLedgerError::DurabilityRejected { .. }) {
                    self.left_open_for_reconciliation
                        .store(true, Ordering::SeqCst);
                } else {
                    self.stop_durability_unconfirmed
                        .store(true, Ordering::SeqCst);
                }
                Err(error)
            }
        }
    }

    /// Relinquish the reserved STOP permit without recording a STOP row.
    ///
    /// This is intentionally narrow: callers use it when they cannot prove the
    /// owned resource has stopped. Leaving the START row open lets the
    /// authoritative reconciliation path classify it after the real liveness
    /// signal disappears instead of publishing a false clean shutdown. The
    /// mutex serializes this transition with [`Self::stop`]; exactly one path
    /// consumes the permit.
    pub fn leave_open_for_reconciliation(&self) -> bool {
        let mut permit = match self.stop_permit.lock() {
            Ok(permit) => permit,
            Err(poisoned) => poisoned.into_inner(),
        };
        let left_open = permit.take().is_some();
        if left_open {
            self.left_open_for_reconciliation
                .store(true, Ordering::SeqCst);
        }
        left_open
    }
}

impl Drop for ActiveProcessLifecycle {
    fn drop(&mut self) {
        let permit = match self.stop_permit.get_mut() {
            Ok(permit) => permit,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(permit) = permit.take() {
            if !self.auto_stop_on_drop || !self.stop_authorized.load(Ordering::SeqCst) {
                return;
            }
            let stop = ProcessStop::from_start(&self.start, None)
                .with_stop_reason("reserved-lifecycle-holder-dropped");
            permit.send(LedgerWriteRequest::lifecycle_stop(
                LedgerEvent::Stop(stop),
                Arc::clone(&self.stop_authorized),
            ));
        }
    }
}

impl ProcessLedgerWriter {
    pub fn spawn(
        store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
        config: WriterConfig,
    ) -> (Self, JoinHandle<Result<(), ProcessLedgerError>>) {
        let config = config
            .validate()
            .expect("ProcessLedgerWriter::spawn received invalid WriterConfig");
        let (sender, receiver) = mpsc::channel(config.capacity);
        let degraded = Arc::new(AtomicBool::new(false));
        let overflow_count = Arc::new(AtomicU64::new(0));
        let flush_failed_rows = Arc::new(AtomicU64::new(0));
        let flush_failure_attempts = Arc::new(AtomicU64::new(0));
        let close_notify = Arc::new(Notify::new());
        let writer = Self {
            sender,
            overflow_sink: Arc::clone(&overflow_sink),
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::clone(&overflow_count),
            flush_failed_rows: Arc::clone(&flush_failed_rows),
            capacity: config.capacity,
            close_notify: Arc::clone(&close_notify),
            recorded_starts: Arc::new(std::sync::Mutex::new(HashMap::new())),
        };
        let join = tokio::spawn(run_writer(
            receiver,
            store,
            config,
            degraded,
            flush_failed_rows,
            flush_failure_attempts,
            close_notify,
        ));
        (writer, join)
    }

    pub fn new_manual(
        capacity: usize,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    ) -> Result<(Self, ProcessLedgerDrain), ProcessLedgerError> {
        let config = WriterConfig {
            capacity,
            // This convenience constructor has no batch-size argument. Keep it
            // valid for deliberately small manual rings while preserving the
            // default batch size whenever the ring can hold it.
            batch_size: PROCESS_LEDGER_DEFAULT_BATCH_SIZE.min(capacity),
            ..WriterConfig::default()
        }
        .validate()?;
        Self::new_manual_with_config(config, overflow_sink)
    }

    pub fn new_manual_with_config(
        config: WriterConfig,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    ) -> Result<(Self, ProcessLedgerDrain), ProcessLedgerError> {
        let config = config.validate()?;
        let (sender, receiver) = mpsc::channel(config.capacity);
        let degraded = Arc::new(AtomicBool::new(false));
        let flush_failed_rows = Arc::new(AtomicU64::new(0));
        let flush_failure_attempts = Arc::new(AtomicU64::new(0));
        let writer = Self {
            sender,
            overflow_sink,
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::new(AtomicU64::new(0)),
            flush_failed_rows: Arc::clone(&flush_failed_rows),
            capacity: config.capacity,
            // The manual drain path runs no `run_writer` task, so this signal is
            // never awaited; it exists only to satisfy the struct shape.
            close_notify: Arc::new(Notify::new()),
            recorded_starts: Arc::new(std::sync::Mutex::new(HashMap::new())),
        };
        let drain = ProcessLedgerDrain {
            receiver: Mutex::new(receiver),
            degraded,
            flush_failed_rows,
            flush_failure_attempts,
            batch_size: config.batch_size,
        };
        Ok((writer, drain))
    }

    pub fn append_start(&self, event: ProcessStart) -> Result<(), ProcessLedgerError> {
        self.enqueue(LedgerEvent::Start(event))
    }

    pub fn append_stop(&self, event: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.enqueue(LedgerEvent::Stop(event))
    }

    pub fn append_start_lossless(&self, event: ProcessStart) -> Result<(), ProcessLedgerError> {
        self.enqueue_lossless(LedgerEvent::Start(event))
    }

    pub fn append_stop_lossless(&self, event: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.enqueue_lossless(LedgerEvent::Stop(event))
    }

    /// Reserve one lossless STOP slot before a reclaim kill begins.
    ///
    /// Full and closed queues fail synchronously, so the caller can release its
    /// fenced claim without touching the process. The permit remains owned
    /// across a slow kill and cannot be displaced by unrelated writer traffic.
    pub fn try_reserve_reclaim_stop(&self) -> Result<ReservedProcessStop, ProcessLedgerError> {
        match self.sender.clone().try_reserve_owned() {
            Ok(permit) => Ok(ReservedProcessStop {
                permit: Some(permit),
            }),
            Err(TrySendError::Full(_sender)) => {
                mark_degraded(&self.degraded);
                Err(ProcessLedgerError::EnqueueDropped(format!(
                    "could not reserve reclaim STOP in ledger writer capacity {}; writer is full",
                    self.capacity
                )))
            }
            Err(TrySendError::Closed(_sender)) => {
                mark_degraded(&self.degraded);
                Err(ProcessLedgerError::EnqueueDropped(format!(
                    "could not reserve reclaim STOP in ledger writer capacity {}; writer channel is closed",
                    self.capacity
                )))
            }
        }
    }

    /// Atomically acquire START+STOP queue authority for `count` resources.
    ///
    /// Tokio exposes owned permits one at a time. This method makes the set
    /// all-or-none by retaining every acquired permit locally and dropping the
    /// complete partial set on the first failure. No event has been emitted at
    /// that point, so a failed preflight has zero lifecycle side effects.
    pub fn try_reserve_lifecycles(
        &self,
        count: usize,
    ) -> Result<Vec<ReservedProcessLifecycle>, ProcessLedgerError> {
        if count == 0 {
            return Err(ProcessLedgerError::InvalidConfig(
                "lifecycle reservation count must be greater than zero".to_string(),
            ));
        }
        let permit_count = count.checked_mul(2).ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "lifecycle reservation count overflowed usize".to_string(),
            )
        })?;
        if permit_count > self.capacity {
            mark_degraded(&self.degraded);
            return Err(ProcessLedgerError::EnqueueDropped(format!(
                "could not atomically reserve {permit_count} lifecycle rows for {count} resources in ledger writer capacity {}; writer is undersized",
                self.capacity
            )));
        }
        let mut permits = Vec::with_capacity(permit_count);
        for _ in 0..permit_count {
            match self.sender.clone().try_reserve_owned() {
                Ok(permit) => permits.push(permit),
                Err(TrySendError::Full(_sender)) => {
                    mark_degraded(&self.degraded);
                    return Err(ProcessLedgerError::EnqueueDropped(format!(
                        "could not atomically reserve {permit_count} lifecycle rows for {count} resources in ledger writer capacity {}; writer is full or undersized",
                        self.capacity
                    )));
                }
                Err(TrySendError::Closed(_sender)) => {
                    mark_degraded(&self.degraded);
                    return Err(ProcessLedgerError::EnqueueDropped(format!(
                        "could not atomically reserve {permit_count} lifecycle rows for {count} resources in ledger writer capacity {}; writer channel is closed",
                        self.capacity
                    )));
                }
            }
        }

        let mut permits = permits.into_iter();
        let mut lifecycles = Vec::with_capacity(count);
        for _ in 0..count {
            let start_permit = permits.next().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "complete lifecycle reservation lost a START permit".to_string(),
                )
            })?;
            let stop_permit = permits.next().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "complete lifecycle reservation lost a STOP permit".to_string(),
                )
            })?;
            lifecycles.push(ReservedProcessLifecycle {
                start_permit: Some(start_permit),
                stop_permit: Some(stop_permit),
                runtime_owner: None,
            });
        }
        Ok(lifecycles)
    }

    /// Bounded backpressure-aware STOP enqueue for graceful shutdown.
    ///
    /// Ordinary spawn-path appends remain non-blocking. Shutdown is different:
    /// the background writer is still alive and may free capacity, so wait for
    /// one permit up to `timeout` before declaring a typed, observable loss.
    /// The caller must invoke this before [`Self::begin_close`].
    pub async fn append_stop_lossless_bounded(
        &self,
        event: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.enqueue_lossless_bounded(LedgerEvent::Stop(event), timeout)
            .await
    }

    /// WP-1 MT-013 (F1 graceful shutdown): signal the spawned writer loop to
    /// close. The loop closes its receiving half (no more rows accepted), drains
    /// everything already buffered to the store, then returns so its `JoinHandle`
    /// resolves. Idempotent and safe to call from any `LedgerBatcher` clone; a
    /// no-op for the manual-drain writer (which runs no loop).
    pub fn begin_close(&self) {
        self.close_notify.notify_one();
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::SeqCst)
    }

    /// Cumulative row-attempt volume across this writer's failed store calls.
    ///
    /// A retained row can contribute more than once and can later persist. The
    /// loud per-attempt error carries row identities; the drain result supplies
    /// the terminal durability signal.
    pub fn flush_failed_rows(&self) -> u64 {
        self.flush_failed_rows.load(Ordering::SeqCst)
    }

    /// The START identity the writer accepted for `process_uuid`, if its STOP
    /// row has not been accepted yet.
    ///
    /// Terminal owners that did not author the START use this to derive a STOP
    /// whose immutable identity columns match the persisted lifecycle row,
    /// instead of synthesizing one the authoritative upsert must reject.
    pub fn recorded_start(&self, process_uuid: Uuid) -> Option<ProcessStart> {
        self.recorded_starts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&process_uuid)
            .cloned()
    }

    /// Track the accepted START identity, or release it once its STOP row has
    /// been accepted. Called only after the row is actually in the queue, so a
    /// dropped row never leaves a phantom identity behind.
    fn index_accepted_event(&self, indexed: AcceptedEventIdentity) {
        let mut recorded = self
            .recorded_starts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match indexed {
            AcceptedEventIdentity::Start(start) => {
                let process_uuid = start.process_uuid;
                if recorded.len() >= PROCESS_LEDGER_START_INDEX_CAPACITY
                    && !recorded.contains_key(&process_uuid)
                {
                    tracing::warn!(
                        target: PROCESS_LEDGER_SOURCE_COMPONENT,
                        event = "ledger_start_index_saturated",
                        process_uuid = %process_uuid,
                        capacity = PROCESS_LEDGER_START_INDEX_CAPACITY,
                        "in-flight START identity index is saturated; this lifecycle's terminal owner must supply its own START identity"
                    );
                    return;
                }
                recorded.insert(process_uuid, *start);
            }
            AcceptedEventIdentity::Stop(process_uuid) => {
                recorded.remove(&process_uuid);
            }
        }
    }

    fn enqueue(&self, event: LedgerEvent) -> Result<(), ProcessLedgerError> {
        let indexed = AcceptedEventIdentity::of(&event);
        match self
            .sender
            .try_send(LedgerWriteRequest::unacknowledged(event))
        {
            Ok(()) => {
                self.index_accepted_event(indexed);
                Ok(())
            }
            Err(TrySendError::Full(request)) | Err(TrySendError::Closed(request)) => {
                mark_degraded(&self.degraded);
                emit_overflow(
                    self.overflow_sink.as_ref(),
                    &self.overflow_count,
                    self.capacity,
                    request.event,
                )?;
                Ok(())
            }
        }
    }

    fn enqueue_lossless(&self, event: LedgerEvent) -> Result<(), ProcessLedgerError> {
        let event_kind = event.kind();
        let process_uuid = event.process_uuid();
        let indexed = AcceptedEventIdentity::of(&event);
        match self
            .sender
            .try_send(LedgerWriteRequest::unacknowledged(event))
        {
            Ok(()) => {
                self.index_accepted_event(indexed);
                Ok(())
            }
            Err(error) => {
                // A full queue and a closed queue are different operational
                // faults: the first is backpressure against a live writer, the
                // second means no ledger consumer exists at all. Reporting both
                // as "capacity N" sends operators after phantom backpressure.
                let (request, cause) = match error {
                    TrySendError::Full(request) => (request, "ledger writer queue is full"),
                    TrySendError::Closed(request) => (
                        request,
                        "ledger writer channel is closed; no ledger consumer is running",
                    ),
                };
                mark_degraded(&self.degraded);
                emit_overflow(
                    self.overflow_sink.as_ref(),
                    &self.overflow_count,
                    self.capacity,
                    request.event,
                )?;
                Err(ProcessLedgerError::EnqueueDropped(format!(
                    "{} row for process_uuid {process_uuid} was not accepted by ledger writer capacity {}: {cause}",
                    event_kind.as_str(),
                    self.capacity
                )))
            }
        }
    }

    async fn enqueue_lossless_bounded(
        &self,
        event: LedgerEvent,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        let event_kind = event.kind();
        let process_uuid = event.process_uuid();
        let indexed = AcceptedEventIdentity::of(&event);
        match time::timeout(timeout, self.sender.reserve()).await {
            Ok(Ok(permit)) => {
                permit.send(LedgerWriteRequest::unacknowledged(event));
                self.index_accepted_event(indexed);
                Ok(())
            }
            outcome => {
                mark_degraded(&self.degraded);
                emit_overflow(
                    self.overflow_sink.as_ref(),
                    &self.overflow_count,
                    self.capacity,
                    event,
                )?;
                let cause = match outcome {
                    Ok(Err(_)) => "ledger writer channel closed",
                    Err(_) => "timed out waiting for ledger writer capacity",
                    Ok(Ok(_)) => unreachable!("successful permit handled above"),
                };
                Err(ProcessLedgerError::EnqueueDropped(format!(
                    "{} row for process_uuid {process_uuid} was not accepted by ledger writer capacity {} after {} ms: {cause}",
                    event_kind.as_str(),
                    self.capacity,
                    timeout.as_millis()
                )))
            }
        }
    }
}

impl Drop for ProcessLedgerWriter {
    fn drop(&mut self) {
        clear_degraded(&self.degraded);
    }
}

pub struct ProcessLedgerDrain {
    receiver: Mutex<Receiver<LedgerWriteRequest>>,
    degraded: Arc<AtomicBool>,
    flush_failed_rows: Arc<AtomicU64>,
    flush_failure_attempts: Arc<AtomicU64>,
    batch_size: usize,
}

impl ProcessLedgerDrain {
    pub async fn drain_available_to<S>(&self, store: Arc<S>) -> Result<(), ProcessLedgerError>
    where
        S: ProcessLedgerStore,
    {
        let mut receiver = self.receiver.lock().await;
        let mut batch = Vec::with_capacity(self.batch_size);
        while let Ok(event) = receiver.try_recv() {
            if rejected_lifecycle_stop(&event) {
                continue;
            }
            batch.push(event);
            if batch.len() >= self.batch_size {
                self.flush_batch_observed(&store, &mut batch).await?;
            }
        }
        if !batch.is_empty() {
            self.flush_batch_observed(&store, &mut batch).await?;
        }
        Ok(())
    }

    /// Number of ledger rows this drain failed to flush to the store.
    pub fn flush_failed_rows(&self) -> u64 {
        self.flush_failed_rows.load(Ordering::SeqCst)
    }

    /// Flush helper that propagates store errors (preserving the manual-drain
    /// contract) but records the loss observably before returning the error, so
    /// even the propagating path is never silent.
    async fn flush_batch_observed<S>(
        &self,
        store: &Arc<S>,
        batch: &mut Vec<LedgerWriteRequest>,
    ) -> Result<(), ProcessLedgerError>
    where
        S: ProcessLedgerStore,
    {
        match flush_batch(store, batch, &self.degraded).await {
            Ok(()) => Ok(()),
            Err(error) => {
                record_flush_failure(
                    &self.flush_failed_rows,
                    &self.flush_failure_attempts,
                    batch,
                    &error,
                );
                Err(error)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_writer(
    mut receiver: Receiver<LedgerWriteRequest>,
    store: Arc<dyn ProcessLedgerStore>,
    config: WriterConfig,
    degraded: Arc<AtomicBool>,
    flush_failed_rows: Arc<AtomicU64>,
    flush_failure_attempts: Arc<AtomicU64>,
    close_notify: Arc<Notify>,
) -> Result<(), ProcessLedgerError> {
    let mut ticker = time::interval_at(
        time::Instant::now() + config.flush_interval,
        config.flush_interval,
    );
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // `batch` owns rows that this writer has already accepted. A transient
    // store failure must never turn those accepted rows into an internal
    // overflow. Bound the retained set at `capacity` and stop receiving while
    // it is full; the mpsc channel then provides backpressure while ticks (or a
    // close request) retry the exact retained rows.
    let mut batch = Vec::with_capacity(config.capacity);
    let mut receiver_drained = false;

    loop {
        if receiver_drained && batch.is_empty() {
            break;
        }

        tokio::select! {
            maybe_event = receiver.recv(), if !receiver_drained && batch.len() < config.capacity => {
                let Some(event) = maybe_event else {
                    receiver_drained = true;
                    if !batch.is_empty() {
                        if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                            record_flush_failure(
                                &flush_failed_rows,
                                &flush_failure_attempts,
                                &batch,
                                &error,
                            );
                        }
                    }
                    continue;
                };
                if rejected_lifecycle_stop(&event) {
                    continue;
                }
                let requires_durable_ack = event.durable_ack.is_some();
                batch.push(event);
                if requires_durable_ack || batch.len() >= config.batch_size {
                    // The background writer must keep running across transient
                    // store failures. The failed batch stays retained and the
                    // receive branch is disabled once it reaches `capacity`.
                    if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                        record_flush_failure(
                            &flush_failed_rows,
                            &flush_failure_attempts,
                            &batch,
                            &error,
                        );
                    }
                }
            }
            _ = ticker.tick() => {
                if !batch.is_empty() {
                    if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                        record_flush_failure(
                            &flush_failed_rows,
                            &flush_failure_attempts,
                            &batch,
                            &error,
                        );
                    }
                }
            }
            _ = close_notify.notified() => {
                // WP-1 MT-013 (F1 graceful shutdown): close the receiving half so
                // no new send/reserve calls are accepted. Outstanding owned
                // permits remain valid, so a reserved STOP can still enter the
                // queue. Retry the retained failed batch immediately; after it
                // succeeds, the receive branch drains that queued STOP before
                // `recv()` can report the channel fully drained.
                receiver.close();
                if !batch.is_empty() {
                    if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                        record_flush_failure(
                            &flush_failed_rows,
                            &flush_failure_attempts,
                            &batch,
                            &error,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Make each ledger flush/store failure attempt observable.
///
/// On `flush_batch` error the batch is retained (not cleared) for retry, but the
/// error itself was previously dropped via `let _ = ...`. This:
///   * increments the per-writer and process-wide failed row-attempt counters
///     (surfaceable via `ProcessLedgerWriter::flush_failed_rows` /
///     `flush_failed_row_count`), and
///   * logs a loud `tracing::error!` carrying every affected row's identity
///     (process_uuid, kind, parent_session_id) plus the store error.
fn record_flush_failure(
    flush_failed_rows: &AtomicU64,
    flush_failure_attempts: &AtomicU64,
    batch: &[LedgerWriteRequest],
    error: &ProcessLedgerError,
) {
    let row_count = batch.len() as u64;
    flush_failed_rows.fetch_add(row_count, Ordering::SeqCst);
    GLOBAL_LEDGER_FLUSH_FAILED_ROWS.fetch_add(row_count, Ordering::SeqCst);
    let failure_attempt = flush_failure_attempts.fetch_add(1, Ordering::SeqCst) + 1;

    // A retained 10k-row batch can retry several times per second. Emit one
    // bounded aggregate on the first and power-of-two attempts instead of one
    // repeated error per row per tick; counters above remain exact.
    if failure_attempt == 1 || failure_attempt.is_power_of_two() {
        let sample: Vec<String> = batch
            .iter()
            .take(8)
            .map(|request| {
                format!(
                    "{}:{}",
                    request.event.kind().as_str(),
                    request.event.process_uuid()
                )
            })
            .collect();
        tracing::error!(
            target: PROCESS_LEDGER_SOURCE_COMPONENT,
            event = "ledger_flush_store_failed",
            failure_attempt,
            row_count,
            sampled_rows = ?sample,
            suppressed_row_count = row_count.saturating_sub(sample.len() as u64),
            error = %error,
            "process ledger flush/store failed; retained batch will retry"
        );
    }
}

async fn flush_batch<S>(
    store: &Arc<S>,
    batch: &mut Vec<LedgerWriteRequest>,
    degraded: &Arc<AtomicBool>,
) -> Result<(), ProcessLedgerError>
where
    S: ProcessLedgerStore + ?Sized,
{
    let events = batch.iter().map(|request| request.event.clone()).collect();
    match store.write_batch(events).await {
        Ok(()) => {
            for mut request in batch.drain(..) {
                if matches!(request.event, LedgerEvent::Start(_)) {
                    if let Some(stop_authorized) = request.stop_authorized.take() {
                        stop_authorized.store(true, Ordering::SeqCst);
                    }
                }
                if let Some(durable_ack) = request.durable_ack.take() {
                    let _ = durable_ack.send(Ok(()));
                }
            }
            clear_degraded(degraded);
            Ok(())
        }
        Err(ProcessLedgerError::StartIdentityConflict {
            process_uuid,
            conflicting_start,
        }) => {
            mark_degraded(degraded);
            let reason = format!(
                "PROCESS_LEDGER_START_IDENTITY_CONFLICT: process_uuid {process_uuid} already belongs to a different lifecycle"
            );
            let mut rejected_stop_authorities = Vec::new();
            let mut index = 0;
            while index < batch.len() {
                let rejected = matches!(
                    &batch[index].event,
                    LedgerEvent::Start(start) if start == conflicting_start.as_ref()
                );
                if rejected {
                    let mut request = batch.remove(index);
                    if let Some(stop_authorized) = request.stop_authorized.take() {
                        stop_authorized.store(false, Ordering::SeqCst);
                        rejected_stop_authorities.push(stop_authorized);
                    }
                    if let Some(durable_ack) = request.durable_ack.take() {
                        let _ = durable_ack.send(Err(reason.clone()));
                    }
                } else {
                    index += 1;
                }
            }
            batch.retain(|request| {
                let matching_rejected_stop =
                    matches!(
                        &request.event,
                        LedgerEvent::Stop(stop) if stop.process_uuid == process_uuid
                    ) && request.stop_authorized.as_ref().is_some_and(|authority| {
                        rejected_stop_authorities
                            .iter()
                            .any(|rejected| Arc::ptr_eq(rejected, authority))
                    });
                !matching_rejected_stop
            });
            tracing::error!(
                target: PROCESS_LEDGER_SOURCE_COMPONENT,
                event = "ledger_start_identity_conflict",
                process_uuid = %process_uuid,
                error = %reason,
                "rejected conflicting START was removed from the retained writer batch"
            );
            Err(ProcessLedgerError::StartIdentityConflict {
                process_uuid,
                conflicting_start,
            })
        }
        Err(ProcessLedgerError::StopIdentityConflict {
            process_uuid,
            conflicting_stop,
        }) => {
            mark_degraded(degraded);
            let reason = format!(
                "PROCESS_LEDGER_STOP_IDENTITY_CONFLICT: process_uuid {process_uuid} STOP does not match the authoritative lifecycle or current reclaim claim"
            );
            let mut removed = false;
            let mut index = 0;
            while index < batch.len() {
                let rejected = matches!(
                    &batch[index].event,
                    LedgerEvent::Stop(stop) if stop == conflicting_stop.as_ref()
                );
                if rejected {
                    let mut request = batch.remove(index);
                    if let Some(durable_ack) = request.durable_ack.take() {
                        let _ = durable_ack.send(Err(reason.clone()));
                    }
                    removed = true;
                } else {
                    index += 1;
                }
            }
            tracing::error!(
                target: PROCESS_LEDGER_SOURCE_COMPONENT,
                event = "ledger_stop_identity_conflict",
                process_uuid = %process_uuid,
                removed,
                retained_row_count = batch.len(),
                error = %reason,
                "rejected permanent STOP conflict was removed; remaining writer rows stay eligible for retry"
            );
            Err(ProcessLedgerError::StopIdentityConflict {
                process_uuid,
                conflicting_stop,
            })
        }
        Err(error) => {
            mark_degraded(degraded);
            Err(error)
        }
    }
}

fn rejected_lifecycle_stop(request: &LedgerWriteRequest) -> bool {
    matches!(request.event, LedgerEvent::Stop(_))
        && request
            .stop_authorized
            .as_ref()
            .is_some_and(|authorized| !authorized.load(Ordering::SeqCst))
}

fn emit_overflow(
    sink: &dyn ProcessLedgerOverflowSink,
    overflow_count: &AtomicU64,
    capacity: usize,
    event: LedgerEvent,
) -> Result<(), ProcessLedgerError> {
    let overflow_count = overflow_count.fetch_add(1, Ordering::SeqCst) + 1;
    let overflow = LedgerOverflowEvent::new(overflow_count, capacity, event);
    sink.emit_overflow(overflow)
        .map_err(|error| ProcessLedgerError::OverflowEmit(error.to_string()))
}

fn mark_degraded(degraded: &AtomicBool) {
    if !degraded.swap(true, Ordering::SeqCst) {
        GLOBAL_DEGRADED_WRITERS.fetch_add(1, Ordering::SeqCst);
    }
}

fn clear_degraded(degraded: &AtomicBool) {
    if degraded.swap(false, Ordering::SeqCst) {
        GLOBAL_DEGRADED_WRITERS.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct PostgresProcessLedgerStore {
    pool: PgPool,
    authority: OnceCell<ProcessLedgerAuthorityRelation>,
}

impl PostgresProcessLedgerStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            authority: OnceCell::new(),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn authority(&self) -> Result<&ProcessLedgerAuthorityRelation, ProcessLedgerError> {
        self.authority
            .get_or_try_init(|| resolve_process_ledger_authority_relation(&self.pool))
            .await
    }

    pub async fn apply_migration(&self) -> Result<(), ProcessLedgerError> {
        for statement in PROCESS_LEDGER_MIGRATION_SQL
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        // Resolve and cache the authoritative relation as part of preflight.
        // Leaving this catalog lookup lazy charges its latency to the first
        // process START durability deadline, which can reject a valid launch
        // before the writer reaches the insert on a catalog-heavy database.
        self.authority().await?;
        Ok(())
    }
}

#[async_trait]
impl ProcessLedgerStore for PostgresProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        if events.is_empty() {
            return Ok(());
        }
        let authority = self.authority().await?.clone();
        let mut tx = self.pool.begin().await?;
        let configured_lock_timeout: String =
            sqlx::query_scalar("SELECT pg_catalog.set_config('lock_timeout', $1, true)")
                .bind(PROCESS_LEDGER_AUTHORITY_LOCK_TIMEOUT)
                .fetch_one(&mut *tx)
                .await?;
        if configured_lock_timeout.trim().is_empty() {
            return Err(ProcessLedgerError::Store(
                "failed to bound PostgreSQL process-ledger authority lock waits".to_string(),
            ));
        }
        pin_transaction_search_path(&mut tx, &authority.schema).await?;
        lock_process_ledger_authority_relation(
            &mut tx,
            &authority,
            ProcessLedgerAuthorityLockMode::RowExclusive,
        )
        .await?;
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        require_postgres_crash_durability(&mut tx, "process-ledger mutation").await?;
        require_synchronous_commit(&mut tx, "process-ledger mutation").await?;
        for event in &events {
            match event {
                LedgerEvent::Start(start) => insert_start(&mut tx, start).await?,
                LedgerEvent::Stop(stop) => upsert_stop(&mut tx, stop).await?,
            }
        }
        force_all_constraints_immediate(&mut tx).await?;
        assert_process_ledger_authority_relation(&mut tx, &authority).await?;
        verify_final_event_rows(&mut tx, &authority, &events).await?;
        require_synchronous_commit(&mut tx, "process-ledger mutation commit").await?;
        tx.commit().await?;
        Ok(())
    }
}

async fn verify_final_event_rows(
    tx: &mut Transaction<'_, Postgres>,
    authority: &ProcessLedgerAuthorityRelation,
    events: &[LedgerEvent],
) -> Result<(), ProcessLedgerError> {
    let readback_sql = format!(
        r#"
        SELECT os_pid, parent_session_id, parent_process_id, sandbox_adapter_id,
               sandbox_internal_id, engine_kind, started_at, stopped_at, exit_code,
               stop_reason, model_artifact_sha256, work_profile_id, owner_role,
               owner_wp, role_id, wp_id, mt_id, sandbox_capabilities_snapshot,
               metadata_jsonb
        FROM ONLY {}
        WHERE process_uuid = $1
        "#,
        authority.qualified_table
    );
    let mut verified = HashSet::with_capacity(events.len());
    for event in events.iter().rev() {
        let process_uuid = event.process_uuid();
        if !verified.insert(process_uuid) {
            continue;
        }
        let row = sqlx::query(&readback_sql)
            .bind(process_uuid)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| {
                ProcessLedgerError::Store(format!(
                    "process-ledger final readback found no row for {process_uuid}"
                ))
            })?;
        let os_pid: Option<i64> = row.try_get("os_pid")?;
        let parent_session_id: Option<String> = row.try_get("parent_session_id")?;
        let parent_process_id: Option<Uuid> = row.try_get("parent_process_id")?;
        let sandbox_adapter_id: Option<String> = row.try_get("sandbox_adapter_id")?;
        let sandbox_internal_id: Option<String> = row.try_get("sandbox_internal_id")?;
        let engine_kind: String = row.try_get("engine_kind")?;
        let started_at: DateTime<Utc> = row.try_get("started_at")?;
        let stopped_at: Option<DateTime<Utc>> = row.try_get("stopped_at")?;
        let exit_code: Option<i32> = row.try_get("exit_code")?;
        let stop_reason: Option<String> = row.try_get("stop_reason")?;
        let model_artifact_sha256: Option<String> = row.try_get("model_artifact_sha256")?;
        let work_profile_id: Option<String> = row.try_get("work_profile_id")?;
        let owner_role: String = row.try_get("owner_role")?;
        let owner_wp: Option<String> = row.try_get("owner_wp")?;
        let role_id: Option<String> = row.try_get("role_id")?;
        let wp_id: Option<String> = row.try_get("wp_id")?;
        let mt_id: Option<String> = row.try_get("mt_id")?;
        let sandbox_capabilities_snapshot: Value = row.try_get("sandbox_capabilities_snapshot")?;
        let metadata_jsonb: Value = row.try_get("metadata_jsonb")?;
        let valid = match event {
            LedgerEvent::Start(start) => {
                os_pid == start.os_pid.map(i64::from)
                    && parent_session_id == start.parent_session_id
                    && parent_process_id == start.parent_process_id
                    && sandbox_adapter_id == start.sandbox_adapter_id
                    && sandbox_internal_id == start.sandbox_internal_id
                    && engine_kind == start.engine_kind.as_str()
                    && started_at.timestamp_micros() == start.started_at.timestamp_micros()
                    && stopped_at.is_none()
                    && exit_code.is_none()
                    && stop_reason.is_none()
                    && model_artifact_sha256 == start.model_artifact_sha256
                    && work_profile_id == start.work_profile_id
                    && owner_role == start.owner_role
                    && owner_wp == start.owner_wp
                    && role_id == start.role_id
                    && wp_id == start.wp_id
                    && mt_id == start.mt_id
                    && sandbox_capabilities_snapshot == start.sandbox_capabilities_snapshot
                    && metadata_jsonb == start.metadata_jsonb
            }
            LedgerEvent::Stop(stop) => {
                os_pid == stop.os_pid.map(i64::from)
                    && parent_session_id == stop.parent_session_id
                    && parent_process_id == stop.parent_process_id
                    && sandbox_adapter_id == stop.sandbox_adapter_id
                    && sandbox_internal_id == stop.sandbox_internal_id
                    && engine_kind == stop.engine_kind.as_str()
                    && started_at.timestamp_micros() == stop.started_at.timestamp_micros()
                    && stopped_at.as_ref().map(DateTime::timestamp_micros)
                        == Some(stop.stopped_at.timestamp_micros())
                    && exit_code == stop.exit_code
                    && stop_reason == stop.stop_reason
                    && model_artifact_sha256 == stop.model_artifact_sha256
                    && work_profile_id == stop.work_profile_id
                    && owner_role == stop.owner_role
                    && owner_wp == stop.owner_wp
                    && role_id == stop.role_id
                    && wp_id == stop.wp_id
                    && mt_id == stop.mt_id
                    && sandbox_capabilities_snapshot == stop.sandbox_capabilities_snapshot
                    && metadata_jsonb == stop.metadata_jsonb
            }
        };
        if !valid {
            return Err(ProcessLedgerError::Store(format!(
                "process-ledger final readback did not match the last {:?} event for {process_uuid}",
                event.kind()
            )));
        }
    }
    Ok(())
}

async fn insert_start(
    tx: &mut Transaction<'_, Postgres>,
    start: &ProcessStart,
) -> Result<(), ProcessLedgerError> {
    let result = sqlx::query(PROCESS_START_INSERT_SQL)
        .bind(start.process_uuid.to_string())
        .bind(start.os_pid.map(i64::from))
        .bind(start.parent_session_id.clone())
        .bind(start.parent_process_id.map(|id| id.to_string()))
        .bind(start.sandbox_adapter_id.clone())
        .bind(start.sandbox_internal_id.clone())
        .bind(start.engine_kind.as_str())
        .bind(start.started_at)
        .bind(start.model_artifact_sha256.clone())
        .bind(start.work_profile_id.clone())
        .bind(start.owner_role.clone())
        .bind(start.owner_wp.clone())
        .bind(start.role_id.clone())
        .bind(start.wp_id.clone())
        .bind(start.mt_id.clone())
        .bind(start.runtime_owner.as_ref().map(|owner| owner.runtime_instance_id.to_string()))
        .bind(start.runtime_owner.as_ref().map(|owner| owner.host_scope_id.clone()))
        .bind(start.runtime_owner.as_ref().map(|owner| owner.lease_schema_id.clone()))
        .bind(start.runtime_owner.as_ref().map(|owner| owner.lease_protocol.clone()))
        .bind(start.runtime_owner.as_ref().map(|owner| owner.lease_address.clone()))
        .bind(start.runtime_owner.as_ref().map(|owner| i32::from(owner.lease_port)))
        .bind(start.sandbox_capabilities_snapshot.to_string())
        .bind(start.metadata_jsonb.to_string())
        .execute(&mut **tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(ProcessLedgerError::StartIdentityConflict {
            process_uuid: start.process_uuid,
            conflicting_start: Box::new(start.clone()),
        });
    }
    Ok(())
}

async fn upsert_stop(
    tx: &mut Transaction<'_, Postgres>,
    stop: &ProcessStop,
) -> Result<(), ProcessLedgerError> {
    let result = sqlx::query(PROCESS_STOP_UPSERT_SQL)
        .bind(stop.process_uuid.to_string())
        .bind(stop.os_pid.map(i64::from))
        .bind(stop.parent_session_id.clone())
        .bind(stop.parent_process_id.map(|id| id.to_string()))
        .bind(stop.sandbox_adapter_id.clone())
        .bind(stop.sandbox_internal_id.clone())
        .bind(stop.engine_kind.as_str())
        .bind(stop.started_at)
        .bind(stop.stopped_at)
        .bind(stop.exit_code)
        .bind(stop.stop_reason.clone())
        .bind(stop.model_artifact_sha256.clone())
        .bind(stop.work_profile_id.clone())
        .bind(stop.owner_role.clone())
        .bind(stop.owner_wp.clone())
        .bind(stop.role_id.clone())
        .bind(stop.wp_id.clone())
        .bind(stop.mt_id.clone())
        .bind(stop.runtime_owner.as_ref().map(|owner| owner.runtime_instance_id.to_string()))
        .bind(stop.runtime_owner.as_ref().map(|owner| owner.host_scope_id.clone()))
        .bind(stop.runtime_owner.as_ref().map(|owner| owner.lease_schema_id.clone()))
        .bind(stop.runtime_owner.as_ref().map(|owner| owner.lease_protocol.clone()))
        .bind(stop.runtime_owner.as_ref().map(|owner| owner.lease_address.clone()))
        .bind(stop.runtime_owner.as_ref().map(|owner| i32::from(owner.lease_port)))
        .bind(stop.sandbox_capabilities_snapshot.to_string())
        .bind(stop.metadata_jsonb.to_string())
        .execute(&mut **tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(ProcessLedgerError::StopIdentityConflict {
            process_uuid: stop.process_uuid,
            conflicting_stop: Box::new(stop.clone()),
        });
    }
    Ok(())
}
