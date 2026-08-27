use std::{
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
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{self, error::TrySendError, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

use super::table::{
    LedgerEvent, LedgerEventKind, ProcessStart, ProcessStop, PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
    PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS,
    PROCESS_LEDGER_TABLE_NAME,
};

pub const FR_EVT_LEDGER_OVERFLOW: &str = "FR_EVT_LEDGER_OVERFLOW";
const PROCESS_LEDGER_SOURCE_COMPONENT: &str = "process_ledger_writer";

static GLOBAL_DEGRADED_WRITERS: AtomicUsize = AtomicUsize::new(0);

/// Process-wide count of ledger rows whose flush/store write failed.
///
/// A flush failure means one or more `ProcessStart` / `ProcessStop` rows could
/// not be persisted to the ledger store. Previously the in-loop flush result
/// was discarded with `let _ = ...`, so a dropped row was completely invisible.
/// This counter makes the loss observable and surfaceable to operators and
/// monitoring without inventing a new spec event (spec 5.7.3 mandates only
/// `FR-EVT-LEDGER-OVERFLOW`, which is emitted separately by `emit_overflow`).
static GLOBAL_LEDGER_FLUSH_FAILED_ROWS: AtomicU64 = AtomicU64::new(0);

pub fn is_degraded() -> bool {
    GLOBAL_DEGRADED_WRITERS.load(Ordering::SeqCst) > 0
}

/// Total number of ledger rows that failed to flush to the store process-wide.
///
/// Non-zero means at least one `ProcessStart`/`ProcessStop` row was not durably
/// recorded; pair with the loud `tracing::error!` emitted at the failure site to
/// recover the affected row identities.
pub fn flush_failed_row_count() -> u64 {
    GLOBAL_LEDGER_FLUSH_FAILED_ROWS.load(Ordering::SeqCst)
}

#[derive(Debug, Error)]
pub enum ProcessLedgerError {
    #[error("PROCESS_LEDGER_INVALID_CONFIG: {0}")]
    InvalidConfig(String),
    #[error("PROCESS_LEDGER_OVERFLOW_EMIT: {0}")]
    OverflowEmit(String),
    #[error("PROCESS_LEDGER_STORE: {0}")]
    Store(String),
    #[error("PROCESS_LEDGER_SURREAL: {source}")]
    Surreal {
        #[source]
        source: SurrealStorageError,
    },
    #[error("PROCESS_LEDGER_EVENT: {0}")]
    Event(String),
}

impl From<SurrealStorageError> for ProcessLedgerError {
    fn from(source: SurrealStorageError) -> Self {
        Self::Surreal { source }
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

pub struct ProcessLedgerWriter {
    sender: Sender<LedgerEvent>,
    overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    degraded: Arc<AtomicBool>,
    overflow_count: Arc<AtomicU64>,
    flush_failed_rows: Arc<AtomicU64>,
    capacity: usize,
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
        let writer = Self {
            sender,
            overflow_sink: Arc::clone(&overflow_sink),
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::clone(&overflow_count),
            flush_failed_rows: Arc::clone(&flush_failed_rows),
            capacity: config.capacity,
        };
        let join = tokio::spawn(run_writer(
            receiver,
            store,
            overflow_sink,
            config,
            degraded,
            overflow_count,
            flush_failed_rows,
        ));
        (writer, join)
    }

    pub fn new_manual(
        capacity: usize,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    ) -> Result<(Self, ProcessLedgerDrain), ProcessLedgerError> {
        let config = WriterConfig {
            capacity,
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
        let writer = Self {
            sender,
            overflow_sink,
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::new(AtomicU64::new(0)),
            flush_failed_rows: Arc::clone(&flush_failed_rows),
            capacity: config.capacity,
        };
        let drain = ProcessLedgerDrain {
            receiver: Mutex::new(receiver),
            degraded,
            flush_failed_rows,
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

    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::SeqCst)
    }

    /// Number of ledger rows this writer failed to flush to the store.
    ///
    /// Non-zero means a `write_batch` call returned an error and the affected
    /// rows were not durably persisted; the loud `tracing::error!` at the
    /// failure site carries the per-row identities.
    pub fn flush_failed_rows(&self) -> u64 {
        self.flush_failed_rows.load(Ordering::SeqCst)
    }

    fn enqueue(&self, event: LedgerEvent) -> Result<(), ProcessLedgerError> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(event)) | Err(TrySendError::Closed(event)) => {
                mark_degraded(&self.degraded);
                emit_overflow(
                    self.overflow_sink.as_ref(),
                    &self.overflow_count,
                    self.capacity,
                    event,
                )?;
                Ok(())
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
    receiver: Mutex<Receiver<LedgerEvent>>,
    degraded: Arc<AtomicBool>,
    flush_failed_rows: Arc<AtomicU64>,
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
        batch: &mut Vec<LedgerEvent>,
    ) -> Result<(), ProcessLedgerError>
    where
        S: ProcessLedgerStore,
    {
        match flush_batch(store, batch, &self.degraded).await {
            Ok(()) => Ok(()),
            Err(error) => {
                record_flush_failure(&self.flush_failed_rows, batch, &error);
                Err(error)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_writer(
    mut receiver: Receiver<LedgerEvent>,
    store: Arc<dyn ProcessLedgerStore>,
    overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    config: WriterConfig,
    degraded: Arc<AtomicBool>,
    overflow_count: Arc<AtomicU64>,
    flush_failed_rows: Arc<AtomicU64>,
) -> Result<(), ProcessLedgerError> {
    let mut ticker = time::interval_at(
        time::Instant::now() + config.flush_interval,
        config.flush_interval,
    );
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut batch = Vec::with_capacity(config.batch_size);

    loop {
        tokio::select! {
            maybe_event = receiver.recv() => {
                let Some(event) = maybe_event else {
                    break;
                };
                if batch.len() >= config.capacity {
                    emit_overflow(
                        overflow_sink.as_ref(),
                        &overflow_count,
                        config.capacity,
                        event,
                    )?;
                    mark_degraded(&degraded);
                    continue;
                }
                batch.push(event);
                if batch.len() >= config.batch_size {
                    // The background writer must keep running across transient
                    // store failures, so we do not propagate the error here.
                    // It must NOT be silent, however: record the loss loudly and
                    // count it before continuing (was previously `let _ = ...`).
                    if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                        record_flush_failure(&flush_failed_rows, &batch, &error);
                    }
                }
            }
            _ = ticker.tick() => {
                if !batch.is_empty() {
                    if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
                        record_flush_failure(&flush_failed_rows, &batch, &error);
                    }
                }
            }
        }
    }

    if !batch.is_empty() {
        if let Err(error) = flush_batch(&store, &mut batch, &degraded).await {
            record_flush_failure(&flush_failed_rows, &batch, &error);
            return Err(error);
        }
    }
    Ok(())
}

/// Make a ledger flush/store failure observable instead of silently discarding it.
///
/// On `flush_batch` error the batch is retained (not cleared) for retry, but the
/// error itself was previously dropped via `let _ = ...`, so a dropped row was
/// invisible. This:
///   * increments the per-writer and process-wide flush-failure row counters
///     (surfaceable via `ProcessLedgerWriter::flush_failed_rows` /
///     `flush_failed_row_count`), and
///   * logs a loud `tracing::error!` carrying every affected row's identity
///     (process_uuid, kind, parent_session_id) plus the store error.
fn record_flush_failure(
    flush_failed_rows: &AtomicU64,
    batch: &[LedgerEvent],
    error: &ProcessLedgerError,
) {
    let row_count = batch.len() as u64;
    flush_failed_rows.fetch_add(row_count, Ordering::SeqCst);
    GLOBAL_LEDGER_FLUSH_FAILED_ROWS.fetch_add(row_count, Ordering::SeqCst);

    for event in batch {
        tracing::error!(
            target: PROCESS_LEDGER_SOURCE_COMPONENT,
            event = "ledger_flush_store_failed",
            process_uuid = %event.process_uuid(),
            event_kind = event.kind().as_str(),
            parent_session_id = event.parent_session_id().unwrap_or("unknown-session"),
            error = %error,
            "process ledger flush/store failed; row not durably persisted"
        );
    }
}

async fn flush_batch<S>(
    store: &Arc<S>,
    batch: &mut Vec<LedgerEvent>,
    degraded: &Arc<AtomicBool>,
) -> Result<(), ProcessLedgerError>
where
    S: ProcessLedgerStore + ?Sized,
{
    let events = batch.clone();
    match store.write_batch(events).await {
        Ok(()) => {
            batch.clear();
            clear_degraded(degraded);
            Ok(())
        }
        Err(error) => {
            mark_degraded(degraded);
            Err(error)
        }
    }
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

/// One `kernel_process_lifecycle` record.
///
/// The field set mirrors the SurrealDB `SCHEMAFULL` table definition in
/// `storage/surreal/schema.surql`. The schema's derived mirrors (`process_id`,
/// `spawned_at_utc`, `adapter_id`, `stopped_at_utc`) carry `VALUE` clauses and
/// are computed by the store, so they are deliberately absent here; reads
/// tolerate them because `SurrealValue` ignores unmodelled record fields.
#[derive(Debug, Clone, SurrealValue)]
struct ProcessLifecycleRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
}

impl ProcessLifecycleRow {
    fn from_start(start: &ProcessStart) -> Self {
        Self {
            process_uuid: start.process_uuid,
            os_pid: start.os_pid.map(i64::from),
            parent_session_id: start.parent_session_id.clone(),
            parent_process_id: start.parent_process_id,
            sandbox_adapter_id: start.sandbox_adapter_id.clone(),
            sandbox_internal_id: start.sandbox_internal_id.clone(),
            engine_kind: start.engine_kind.as_str().to_string(),
            started_at: start.started_at,
            stopped_at: None,
            exit_code: None,
            stop_reason: None,
            model_artifact_sha256: start.model_artifact_sha256.clone(),
            work_profile_id: start.work_profile_id.clone(),
            owner_role: start.owner_role.clone(),
            owner_wp: start.owner_wp.clone(),
            role_id: start.role_id.clone(),
            wp_id: start.wp_id.clone(),
            mt_id: start.mt_id.clone(),
            sandbox_capabilities_snapshot: start.sandbox_capabilities_snapshot.clone(),
            metadata: start.metadata_jsonb.clone(),
        }
    }

    fn from_stop(stop: &ProcessStop) -> Self {
        Self {
            process_uuid: stop.process_uuid,
            os_pid: stop.os_pid.map(i64::from),
            parent_session_id: stop.parent_session_id.clone(),
            parent_process_id: stop.parent_process_id,
            sandbox_adapter_id: stop.sandbox_adapter_id.clone(),
            sandbox_internal_id: stop.sandbox_internal_id.clone(),
            engine_kind: stop.engine_kind.as_str().to_string(),
            started_at: stop.started_at,
            stopped_at: Some(stop.stopped_at),
            exit_code: stop.exit_code.map(i64::from),
            stop_reason: stop.stop_reason.clone(),
            model_artifact_sha256: stop.model_artifact_sha256.clone(),
            work_profile_id: stop.work_profile_id.clone(),
            owner_role: stop.owner_role.clone(),
            owner_wp: stop.owner_wp.clone(),
            role_id: stop.role_id.clone(),
            wp_id: stop.wp_id.clone(),
            mt_id: stop.mt_id.clone(),
            sandbox_capabilities_snapshot: stop.sandbox_capabilities_snapshot.clone(),
            metadata: stop.metadata_jsonb.clone(),
        }
    }
}

#[derive(SurrealValue)]
struct ProcessApplyBindings {
    record: RecordId,
    incoming: ProcessLifecycleRow,
    is_start: bool,
    reclaim_claimed_at: Option<DateTime<Utc>>,
    reclaim_expected_reason: Option<String>,
    reclaim_expected_killed_reason: Option<String>,
}

// One statement means the read, merge decision, and write share the same
// SurrealDB transaction snapshot. The update lists intentionally reproduce the
// former PostgreSQL ON CONFLICT clauses field-for-field. START coalesces every
// optional identity column, takes the earliest started_at, replaces the
// incoming-owned fields, and never touches the terminal triple. STOP
// unconditionally replaces the terminal triple, coalesces its optional fields,
// and preserves the START-owned fields. The only additional guard protects an
// in-flight reclaim sentinel from an ordinary or stale STOP.
const APPLY_PROCESS_EVENT_ATOMIC: &str = r#"
RETURN {
    LET $exact_reclaim_stop = $reclaim_claimed_at != NONE;
    LET $existing = SELECT process_uuid, os_pid, parent_session_id,
        parent_process_id, sandbox_adapter_id, sandbox_internal_id,
        engine_kind, started_at, stopped_at, exit_code, stop_reason,
        model_artifact_sha256, work_profile_id, owner_role, owner_wp,
        role_id, wp_id, mt_id, sandbox_capabilities_snapshot, metadata
        FROM ONLY $record;
    IF $existing = NONE {
        IF $exact_reclaim_stop {
            RETURN 'ignored_conflict';
        };
        CREATE $record CONTENT $incoming RETURN NONE;
        RETURN 'inserted';
    };
    IF $is_start {
        UPDATE $record SET
            os_pid = $incoming.os_pid ?? $existing.os_pid,
            parent_session_id = $incoming.parent_session_id ?? $existing.parent_session_id,
            parent_process_id = $incoming.parent_process_id ?? $existing.parent_process_id,
            sandbox_adapter_id = $incoming.sandbox_adapter_id ?? $existing.sandbox_adapter_id,
            sandbox_internal_id = $incoming.sandbox_internal_id ?? $existing.sandbox_internal_id,
            engine_kind = $incoming.engine_kind,
            started_at = IF $incoming.started_at < $existing.started_at {
                $incoming.started_at
            } ELSE {
                $existing.started_at
            },
            model_artifact_sha256 = $incoming.model_artifact_sha256 ?? $existing.model_artifact_sha256,
            work_profile_id = $incoming.work_profile_id ?? $existing.work_profile_id,
            owner_role = $incoming.owner_role,
            owner_wp = $incoming.owner_wp ?? $existing.owner_wp,
            role_id = $incoming.role_id ?? $existing.role_id,
            wp_id = $incoming.wp_id ?? $existing.wp_id,
            mt_id = $incoming.mt_id ?? $existing.mt_id,
            sandbox_capabilities_snapshot = $incoming.sandbox_capabilities_snapshot,
            metadata = $incoming.metadata
            RETURN NONE;
        RETURN 'started';
    };
    LET $reclaim_sentinel =
        $existing.stopped_at != NONE
        AND $existing.exit_code = NONE
        AND $existing.stop_reason != NONE
        AND (
            string::starts_with($existing.stop_reason, 'reclaim_claimed:')
            OR string::starts_with($existing.stop_reason, 'reclaim_killed:')
        );
    IF $exact_reclaim_stop AND $reclaim_sentinel = false {
        IF $existing.stopped_at != NONE
            AND $existing.exit_code = $incoming.exit_code
            AND $existing.stop_reason = $incoming.stop_reason
        {
            RETURN 'stopped_idempotent';
        };
        RETURN 'ignored_conflict';
    };
    IF $reclaim_sentinel AND (
        $exact_reclaim_stop = false
        OR $existing.stopped_at != $reclaim_claimed_at
        OR $reclaim_expected_reason = NONE
        OR $reclaim_expected_killed_reason = NONE
        OR (
            $existing.stop_reason != $reclaim_expected_reason
            AND $existing.stop_reason != $reclaim_expected_killed_reason
        )
    ) {
        RETURN 'ignored_conflict';
    };
    UPDATE $record SET
        os_pid = $incoming.os_pid ?? $existing.os_pid,
        parent_process_id = $incoming.parent_process_id ?? $existing.parent_process_id,
        sandbox_internal_id = $incoming.sandbox_internal_id ?? $existing.sandbox_internal_id,
        stopped_at = $incoming.stopped_at,
        exit_code = $incoming.exit_code,
        stop_reason = $incoming.stop_reason,
        model_artifact_sha256 = $incoming.model_artifact_sha256 ?? $existing.model_artifact_sha256,
        work_profile_id = $incoming.work_profile_id ?? $existing.work_profile_id,
        owner_role = $incoming.owner_role,
        owner_wp = $incoming.owner_wp ?? $existing.owner_wp,
        role_id = $incoming.role_id ?? $existing.role_id,
        wp_id = $incoming.wp_id ?? $existing.wp_id,
        mt_id = $incoming.mt_id ?? $existing.mt_id,
        sandbox_capabilities_snapshot = $incoming.sandbox_capabilities_snapshot,
        metadata = $incoming.metadata
        RETURN NONE;
    RETURN 'stopped';
};
"#;

/// `ProcessLedgerStore` backed by the Handshake-managed embedded SurrealDB
/// store.
///
/// Every row is keyed by `process_uuid` (the record id), so a replayed batch
/// re-applies the same merge and converges on the same record. That keeps the
/// retry loop in [`flush_batch`] safe: on a store error the batch is retained
/// and re-sent whole, and re-sending an already-applied event is a no-op.
///
/// The complete merge is one SurrealDB `UPSERT` statement. This matters because
/// reclaim and direct store callers can update the same row concurrently even
/// though the RocksDB directory itself has only one owning process.
pub struct SurrealProcessLedgerStore {
    storage: SurrealStorage,
}

impl SurrealProcessLedgerStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub(crate) fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    async fn apply(
        &self,
        record_id: String,
        incoming: ProcessLifecycleRow,
        kind: LedgerEventKind,
        reclaim_claimed_at: Option<DateTime<Utc>>,
        reclaim_expected_reason: Option<String>,
        reclaim_expected_killed_reason: Option<String>,
    ) -> Result<(), ProcessLedgerError> {
        let exact_reclaim_stop = kind == LedgerEventKind::Stop && reclaim_claimed_at.is_some();
        let bindings = ProcessApplyBindings {
            record: RecordId::new(PROCESS_LEDGER_TABLE_NAME, record_id),
            incoming,
            is_start: kind == LedgerEventKind::Start,
            reclaim_claimed_at,
            reclaim_expected_reason,
            reclaim_expected_killed_reason,
        };
        let outcome: Option<String> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first(APPLY_PROCESS_EVENT_ATOMIC, bindings)
                        .await
                })
            })
            .await
            .map_err(ProcessLedgerError::from)?;
        match outcome.as_deref() {
            Some("inserted" | "started" | "stopped" | "stopped_idempotent") => Ok(()),
            Some("ignored_conflict") if !exact_reclaim_stop => Ok(()),
            Some("ignored_conflict") => Err(ProcessLedgerError::Event(
                "exact reclaim STOP did not own the current durable sentinel".to_owned(),
            )),
            _ => Err(ProcessLedgerError::Event(format!(
                "process lifecycle apply returned an invalid outcome: {outcome:?}"
            ))),
        }
    }
}

#[async_trait]
impl ProcessLedgerStore for SurrealProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        if events.is_empty() {
            return Ok(());
        }
        for event in events {
            match event {
                LedgerEvent::Start(start) => {
                    self.apply(
                        start.process_uuid.to_string(),
                        ProcessLifecycleRow::from_start(&start),
                        LedgerEventKind::Start,
                        None,
                        None,
                        None,
                    )
                    .await?
                }
                LedgerEvent::Stop(stop) => {
                    let reclaim_claimed_at = stop.reclaim_claimed_at;
                    let reclaim_expected_reason = stop.reclaim_expected_reason.clone();
                    let reclaim_expected_killed_reason =
                        stop.reclaim_expected_killed_reason.clone();
                    self.apply(
                        stop.process_uuid.to_string(),
                        ProcessLifecycleRow::from_stop(&stop),
                        LedgerEventKind::Stop,
                        reclaim_claimed_at,
                        reclaim_expected_reason,
                        reclaim_expected_killed_reason,
                    )
                    .await?
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod surreal_restart_tests {
    use super::*;
    use crate::process_ledger::{ProcessEngineKind, ReclaimProcessStore};
    #[cfg(feature = "surreal-test-support")]
    use crate::storage::surreal::bootstrap_mt137_process_ledger_test_schema;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};
    #[cfg(feature = "surreal-test-support")]
    use surrealdb::types::Value as SurrealDataValue;

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid process-ledger test path"),
        )
        .await
        .expect("open embedded process-ledger store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap process-ledger schema");
        storage
    }

    #[cfg(feature = "surreal-test-support")]
    async fn open_mt137_process_ledger_slice(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid focused process-ledger test path"),
        )
        .await
        .expect("open focused embedded process-ledger store");
        bootstrap_mt137_process_ledger_test_schema(&storage)
            .await
            .expect("bootstrap focused process-ledger schema");
        storage
    }

    async fn read_process_row(storage: &SurrealStorage, process_uuid: Uuid) -> ProcessLifecycleRow {
        let record_id = process_uuid.to_string();
        storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one(PROCESS_LEDGER_TABLE_NAME, &record_id)
                        .await
                })
            })
            .await
            .expect("query process lifecycle")
            .expect("process lifecycle exists")
    }

    #[tokio::test]
    async fn mt137_process_start_and_stop_survive_store_reopen() {
        let directory = tempfile::tempdir().expect("temporary process-ledger root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let start = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id("mt137-session")
        .with_os_pid(137)
        .with_mt_id("MT-137");
        let process_uuid = start.process_uuid;
        let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("proof-complete");

        store
            .write_batch(vec![LedgerEvent::Start(start), LedgerEvent::Stop(stop)])
            .await
            .expect("persist process lifecycle");
        storage
            .shutdown()
            .await
            .expect("close process-ledger store");
        drop(store);
        drop(storage);

        let reopened = open(&path).await;
        let record_id = process_uuid.to_string();
        let row: ProcessLifecycleRow = reopened
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one(PROCESS_LEDGER_TABLE_NAME, &record_id)
                        .await
                })
            })
            .await
            .expect("query reopened process lifecycle")
            .expect("read reopened process lifecycle");
        assert_eq!(row.process_uuid, process_uuid);
        assert_eq!(row.parent_session_id.as_deref(), Some("mt137-session"));
        assert_eq!(row.exit_code, Some(0));
        assert_eq!(row.stop_reason.as_deref(), Some("proof-complete"));
        assert!(row.stopped_at.is_some());
        reopened.shutdown().await.expect("close reopened store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_replayed_start_cannot_erase_concurrent_reclaim() {
        use crate::process_ledger::ReclaimProcessStore;

        let directory = tempfile::tempdir().expect("temporary process-reclaim root");
        let path = directory.path().join("store");
        let storage = open_mt137_process_ledger_slice(&path).await;
        let store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let start = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id("mt137-reclaim-session")
        .with_os_pid(138)
        .with_mt_id("MT-137");
        let process_uuid = start.process_uuid;
        store
            .write_batch(vec![LedgerEvent::Start(start.clone())])
            .await
            .expect("persist initial process start");

        let replay_store = Arc::clone(&store);
        let reclaim_store = Arc::clone(&store);
        let (replay, reclaim) = tokio::join!(
            replay_store.write_batch(vec![LedgerEvent::Start(start.clone())]),
            reclaim_store.active_processes_for_session("mt137-reclaim-session"),
        );
        replay.expect("replay process start");
        let mut reclaimed = reclaim.expect("claim active process");
        assert_eq!(reclaimed.len(), 1);
        let claimed_process = reclaimed.pop().expect("one claimed process");
        let contention = store
            .active_processes_for_session("mt137-reclaim-session")
            .await
            .expect_err("an outstanding same-boot claim must not look like zero work");
        assert!(contention
            .to_string()
            .contains("same-boot reclaim claims have not converged"));

        let ordinary_stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("ordinary");
        store
            .write_batch(vec![LedgerEvent::Stop(ordinary_stop)])
            .await
            .expect("ordinary STOP conflict is handled");
        let still_claimed = read_process_row(&storage, process_uuid).await;
        assert_eq!(still_claimed.exit_code, None);
        assert!(
            still_claimed
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("reclaim_claimed:")),
            "a STOP without the exact claim timestamp must not consume the sentinel"
        );

        store
            .write_batch(vec![LedgerEvent::Stop(claimed_process.reclaim_stop(-1))])
            .await
            .expect("exact reclaim STOP finalizes sentinel");

        drop(replay_store);
        drop(reclaim_store);
        drop(store);
        storage.shutdown().await.expect("close process store");
        drop(storage);

        let reopened = open_mt137_process_ledger_slice(&path).await;
        let record_id = process_uuid.to_string();
        let row: ProcessLifecycleRow = reopened
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one(PROCESS_LEDGER_TABLE_NAME, &record_id)
                        .await
                })
            })
            .await
            .expect("query reclaimed process")
            .expect("reclaimed process exists");
        assert!(row.stopped_at.is_some());
        assert_eq!(row.exit_code, Some(-1));
        assert_eq!(row.stop_reason.as_deref(), Some("reclaim"));
        reopened.shutdown().await.expect("close reopened store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_abandoned_reclaim_claim_is_reacquired_after_reopen() {
        let directory = tempfile::tempdir().expect("temporary stale-reclaim root");
        let path = directory.path().join("store");
        let storage = open_mt137_process_ledger_slice(&path).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let session_id = "mt137-stale-reclaim-session";
        let first = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(139)
        .with_mt_id("MT-137");
        let second = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(140)
        .with_mt_id("MT-137");
        store
            .write_batch(vec![LedgerEvent::Start(first), LedgerEvent::Start(second)])
            .await
            .expect("persist stale-reclaim fixtures");

        let initial_claim_at = Utc::now();
        let first_boot_owner = Uuid::nil();
        let mut claimed = store
            .active_processes_for_session_at_with_owner(
                session_id,
                initial_claim_at,
                first_boot_owner,
            )
            .await
            .expect("claim two active processes");
        claimed.sort_by_key(|process| process.process_uuid);
        assert_eq!(claimed.len(), 2);
        let completed = claimed.remove(0);
        let abandoned = claimed.remove(0);
        store
            .write_batch(vec![LedgerEvent::Stop(completed.reclaim_stop(-1))])
            .await
            .expect("finalize one process before simulated crash");

        drop(store);
        storage
            .shutdown()
            .await
            .expect("close after partial reclaim");
        drop(storage);

        let reopened = open_mt137_process_ledger_slice(&path).await;
        let reopened_store = SurrealProcessLedgerStore::new(reopened.clone());
        let same_live_owner_error = reopened_store
            .active_processes_for_session_at_with_owner(
                session_id,
                initial_claim_at + chrono::Duration::hours(1),
                first_boot_owner,
            )
            .await
            .expect_err("same live boot owner must surface its outstanding claim");
        assert!(
            same_live_owner_error
                .to_string()
                .contains("same-boot reclaim claims have not converged"),
            "same-owner claim ownership must not expire or look like zero work"
        );

        let restarted_boot_owner = Uuid::now_v7();
        let mut recovered = reopened_store
            .active_processes_for_session_at_with_owner(
                session_id,
                initial_claim_at + chrono::Duration::hours(1),
                restarted_boot_owner,
            )
            .await
            .expect("a different process boot re-acquires the abandoned sentinel");
        assert_eq!(recovered.len(), 1);
        let recovered = recovered.pop().expect("one abandoned process");
        assert_eq!(recovered.process_uuid, abandoned.process_uuid);
        reopened_store
            .write_batch(vec![LedgerEvent::Stop(recovered.reclaim_stop(-1))])
            .await
            .expect("finalize re-acquired process");

        let completed_row = read_process_row(&reopened, completed.process_uuid).await;
        let recovered_row = read_process_row(&reopened, abandoned.process_uuid).await;
        for row in [completed_row, recovered_row] {
            assert_eq!(row.exit_code, Some(-1));
            assert_eq!(row.stop_reason.as_deref(), Some("reclaim"));
        }
        drop(reopened_store);
        reopened.shutdown().await.expect("close recovered store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[derive(SurrealValue)]
    struct OwnerRoleOverrideBindings {
        record: RecordId,
        owner_role: SurrealDataValue,
    }

    #[cfg(feature = "surreal-test-support")]
    async fn override_owner_role(
        storage: &SurrealStorage,
        process_uuid: Uuid,
        owner_role: SurrealDataValue,
    ) {
        let bindings = OwnerRoleOverrideBindings {
            record: RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
            owner_role,
        };
        let updated: Vec<SurrealDataValue> = storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values_at(
                            "DEFINE FIELD OVERWRITE owner_role ON TABLE kernel_process_lifecycle TYPE any; \
                             UPDATE $record SET owner_role = $owner_role RETURN AFTER;",
                            bindings,
                            1,
                        )
                        .await
                })
            })
            .await
            .expect("override process owner_role fixture");
        assert_eq!(updated.len(), 1, "owner_role fixture must target one row");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_reclaim_structural_decode_failure_releases_every_raw_claim_receipt() {
        let directory = tempfile::tempdir().expect("temporary reclaim compensation root");
        let storage = open_mt137_process_ledger_slice(&directory.path().join("store")).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let session_id = "mt137-reclaim-decode-compensation";
        let valid = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(137)
        .with_mt_id("MT-137");
        let malformed = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(138)
        .with_mt_id("MT-137");
        let valid_uuid = valid.process_uuid;
        let malformed_uuid = malformed.process_uuid;
        store
            .write_batch(vec![
                LedgerEvent::Start(valid),
                LedgerEvent::Start(malformed),
            ])
            .await
            .expect("persist reclaim decode fixtures");
        override_owner_role(&storage, malformed_uuid, 137_i64.into_value()).await;

        let owner_id = Uuid::now_v7();
        let first_claimed_at = Utc::now();
        let error = store
            .active_processes_for_session_at_with_owner(session_id, first_claimed_at, owner_id)
            .await
            .expect_err("wrong durable owner_role type must fail structural decode");
        assert!(error.to_string().contains("failed typed decode"));

        override_owner_role(
            &storage,
            malformed_uuid,
            "mt137-proof".to_string().into_value(),
        )
        .await;

        for process_uuid in [valid_uuid, malformed_uuid] {
            let row = read_process_row(&storage, process_uuid).await;
            assert_eq!(
                row.stopped_at, None,
                "every row claimed by the failed statement must be released"
            );
            assert_eq!(row.stop_reason, None);
        }

        let recovered = store
            .active_processes_for_session_at_with_owner(
                session_id,
                first_claimed_at + chrono::Duration::seconds(1),
                owner_id,
            )
            .await
            .expect("same boot can immediately reclaim all compensated rows");
        assert_eq!(recovered.len(), 2);
        for process in recovered {
            store
                .write_batch(vec![LedgerEvent::Stop(process.reclaim_stop(-1))])
                .await
                .expect("finalize successful proof claim");
        }
        storage.shutdown().await.expect("close compensation store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_same_boot_cleanup_marker_is_preserved_for_retry_finalization() {
        let directory = tempfile::tempdir().expect("temporary same-boot marker root");
        let storage = open_mt137_process_ledger_slice(&directory.path().join("store")).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let session_id = "mt137-same-boot-killed-retry";
        let start = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(13_710)
        .with_mt_id("MT-137");
        let process_uuid = start.process_uuid;
        store
            .write_batch(vec![LedgerEvent::Start(start)])
            .await
            .expect("persist same-boot marker fixture");

        let first = store
            .active_processes_for_session(session_id)
            .await
            .expect("claim same-boot marker fixture")
            .pop()
            .expect("one same-boot marker fixture");
        let marker_timestamp = first.reclaim_claimed_at;
        store
            .mark_reclaim_cleanup_completed(&first)
            .await
            .expect("persist cleanup-completed marker");
        let marked = read_process_row(&storage, process_uuid).await;
        let marker_reason = marked
            .stop_reason
            .clone()
            .expect("cleanup marker has a reason");
        assert_eq!(marked.stopped_at, Some(marker_timestamp));
        assert!(marker_reason.starts_with("reclaim_killed:"));

        let retry = store
            .active_processes_for_session(session_id)
            .await
            .expect("same boot re-acquires cleanup-completed work")
            .pop()
            .expect("one cleanup-completed retry");
        assert!(retry.reclaim_cleanup_completed);
        assert_eq!(retry.reclaim_claimed_at, marker_timestamp);
        let preserved = read_process_row(&storage, process_uuid).await;
        assert_eq!(preserved.stopped_at, Some(marker_timestamp));
        assert_eq!(
            preserved.stop_reason.as_deref(),
            Some(marker_reason.as_str())
        );

        store
            .write_batch(vec![LedgerEvent::Stop(retry.reclaim_stop(-1))])
            .await
            .expect("retry finalizes the preserved cleanup marker");
        let terminal_before_stale_finalizer = read_process_row(&storage, process_uuid).await;
        store
            .write_batch(vec![LedgerEvent::Stop(first.reclaim_stop(-1))])
            .await
            .expect("concurrent original finalizer is an idempotent no-op");
        assert!(store
            .active_processes_for_session(session_id)
            .await
            .expect("terminal row has no remaining reclaim work")
            .is_empty());
        let terminal = read_process_row(&storage, process_uuid).await;
        assert_eq!(terminal.exit_code, Some(-1));
        assert_eq!(terminal.stop_reason.as_deref(), Some("reclaim"));
        assert_eq!(
            terminal.stopped_at, terminal_before_stale_finalizer.stopped_at,
            "a stale exact finalizer must not rewrite terminal chronology"
        );
        assert_eq!(
            terminal.metadata, terminal_before_stale_finalizer.metadata,
            "a stale exact finalizer must not rewrite terminal metadata"
        );

        drop(store);
        storage
            .shutdown()
            .await
            .expect("close same-boot marker store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_stale_exact_reclaim_stop_surfaces_claim_conflict() {
        let directory = tempfile::tempdir().expect("temporary stale exact STOP root");
        let storage = open_mt137_process_ledger_slice(&directory.path().join("store")).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let session_id = "mt137-stale-exact-stop";
        let start = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(13_711)
        .with_mt_id("MT-137");
        let process_uuid = start.process_uuid;
        store
            .write_batch(vec![LedgerEvent::Start(start)])
            .await
            .expect("persist stale exact STOP fixture");

        let owner_a = Uuid::now_v7();
        let owner_b = Uuid::now_v7();
        let claimed_at_a = Utc::now();
        let claimed_at_b = claimed_at_a;
        let claim_a = store
            .active_processes_for_session_at_with_owner(session_id, claimed_at_a, owner_a)
            .await
            .expect("first owner claims fixture")
            .pop()
            .expect("first owner receives fixture");
        let claim_b = store
            .active_processes_for_session_at_with_owner(session_id, claimed_at_b, owner_b)
            .await
            .expect("replacement boot owner reclaims fixture")
            .pop()
            .expect("replacement owner receives fixture");
        assert_eq!(claim_a.reclaim_claimed_at, claim_b.reclaim_claimed_at);
        assert_ne!(
            claim_a.reclaim_expected_reason, claim_b.reclaim_expected_reason,
            "owner-qualified sentinel reason, not timestamp, separates equal-time claims"
        );

        let stale_error = store
            .write_batch(vec![LedgerEvent::Stop(claim_a.reclaim_stop(-1))])
            .await
            .expect_err("stale exact STOP must not report durable success");
        assert!(stale_error
            .to_string()
            .contains("exact reclaim STOP did not own the current durable sentinel"));
        let still_owned_by_b = read_process_row(&storage, process_uuid).await;
        assert_eq!(still_owned_by_b.stopped_at, Some(claimed_at_b));
        let owner_b_reason = format!("reclaim_claimed:{owner_b}");
        assert_eq!(
            still_owned_by_b.stop_reason.as_deref(),
            Some(owner_b_reason.as_str())
        );

        store
            .write_batch(vec![LedgerEvent::Stop(claim_b.reclaim_stop(-1))])
            .await
            .expect("current exact owner finalizes fixture");
        drop(store);
        storage
            .shutdown()
            .await
            .expect("close stale exact STOP store");
    }

    #[cfg(feature = "surreal-test-support")]
    #[tokio::test]
    async fn mt137_raw_receipt_failure_restores_prior_cleanup_state_without_second_cleanup() {
        use crate::process_ledger::restart_resume::{
            RestartOrphanReclaimer, StartupProcessCleanup, SurrealRestartOrphanReclaimer,
        };
        use crate::process_ledger::ReclaimableProcess;

        struct RecordingCleanup {
            cleaned: Mutex<Vec<Uuid>>,
        }

        #[async_trait]
        impl StartupProcessCleanup for RecordingCleanup {
            async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String> {
                self.cleaned.lock().await.push(process.process_uuid);
                Ok(())
            }
        }

        let directory = tempfile::tempdir().expect("temporary raw receipt failure root");
        let storage = open_mt137_process_ledger_slice(&directory.path().join("store")).await;
        let store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
        let session_id = "mt137-raw-receipt-exact-compensation";
        let active = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(13_708)
        .with_mt_id("MT-137");
        let marked = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-proof",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id(session_id)
        .with_os_pid(13_709)
        .with_mt_id("MT-137");
        let active_uuid = active.process_uuid;
        let marked_uuid = marked.process_uuid;
        store
            .write_batch(vec![
                LedgerEvent::Start(active),
                LedgerEvent::Start(marked.clone()),
            ])
            .await
            .expect("persist mixed raw-receipt fixtures");

        let prior_owner = Uuid::now_v7();
        let prior_marker = format!("reclaim_killed:{prior_owner}");
        let prior_stopped_at = Utc::now() - chrono::Duration::seconds(7);
        let mut prior_cleanup =
            ProcessStop::from_start(&marked, None).with_stop_reason(prior_marker.clone());
        prior_cleanup.stopped_at = prior_stopped_at;
        store
            .write_batch(vec![LedgerEvent::Stop(prior_cleanup)])
            .await
            .expect("persist prior cleanup marker");

        let error = store
            .active_processes_for_session_with_raw_receipt_failure(
                session_id,
                Utc::now(),
                Uuid::now_v7(),
                active_uuid,
            )
            .await
            .expect_err("injected non-object claim result must fail closed after compensation");
        assert!(error.to_string().contains("non-object row"));

        let active_after = read_process_row(&storage, active_uuid).await;
        assert_eq!(active_after.stopped_at, None);
        assert_eq!(active_after.stop_reason, None);
        let marked_after = read_process_row(&storage, marked_uuid).await;
        assert_eq!(marked_after.stopped_at, Some(prior_stopped_at));
        assert_eq!(
            marked_after.stop_reason.as_deref(),
            Some(prior_marker.as_str())
        );
        assert!(
            !marked_after
                .metadata
                .as_object()
                .is_some_and(|metadata| metadata
                    .contains_key("__handshake_internal_reclaim_durable_receipt_v1")),
            "successful exact compensation must not leak its durable receipt"
        );

        let cleanup = Arc::new(RecordingCleanup {
            cleaned: Mutex::new(Vec::new()),
        });
        let reclaimer = SurrealRestartOrphanReclaimer::new(
            Arc::clone(&store),
            Arc::clone(&cleanup) as Arc<dyn StartupProcessCleanup>,
        );
        assert_eq!(
            reclaimer
                .reclaim_session(session_id)
                .await
                .expect("reclaim compensated mixed rows"),
            2
        );
        assert_eq!(
            cleanup.cleaned.lock().await.as_slice(),
            &[active_uuid],
            "the restored reclaim_killed row must bypass external cleanup"
        );

        drop(reclaimer);
        drop(cleanup);
        drop(store);
        storage
            .shutdown()
            .await
            .expect("close raw receipt compensation store");
    }

    #[tokio::test]
    async fn mt137_events_reproduce_process_ledger_conflict_merge_lists() {
        let directory = tempfile::tempdir().expect("temporary process guard root");
        let storage = open(&directory.path().join("store")).await;
        let store = SurrealProcessLedgerStore::new(storage.clone());
        let original = ProcessStart::new(
            ProcessEngineKind::HelperSubprocess,
            "mt137-owner",
            Some("WP-KERNEL-012".to_owned()),
        )
        .with_parent_session_id("mt137-guard-session")
        .with_os_pid(9137)
        .with_sandbox_adapter_id("windows_native_jail")
        .with_mt_id("MT-137")
        .with_metadata_jsonb(serde_json::json!({"owner":"original"}));
        let process_uuid = original.process_uuid;
        store
            .write_batch(vec![LedgerEvent::Start(original.clone())])
            .await
            .expect("persist guarded process start");

        let mut enriched = original.clone();
        enriched.os_pid = None;
        enriched.parent_session_id = None;
        enriched.sandbox_adapter_id = None;
        enriched.parent_process_id = Some(Uuid::now_v7());
        enriched.sandbox_internal_id = Some("hsk-mt137-enriched".to_owned());
        enriched.started_at = original.started_at - chrono::Duration::seconds(5);
        enriched.owner_role = "mt137-enriched-owner".to_owned();
        enriched.metadata_jsonb = serde_json::json!({"owner":"enriched"});
        store
            .write_batch(vec![LedgerEvent::Start(enriched.clone())])
            .await
            .expect("START enrichment merges onto existing process");
        let after_start = read_process_row(&storage, process_uuid).await;
        assert_eq!(after_start.os_pid, Some(9137));
        assert_eq!(
            after_start.parent_session_id.as_deref(),
            Some("mt137-guard-session")
        );
        assert_eq!(
            after_start.sandbox_adapter_id.as_deref(),
            Some("windows_native_jail")
        );
        assert_eq!(after_start.parent_process_id, enriched.parent_process_id);
        assert_eq!(
            after_start.sandbox_internal_id.as_deref(),
            Some("hsk-mt137-enriched")
        );
        assert_eq!(after_start.started_at, enriched.started_at);
        assert_eq!(after_start.owner_role, "mt137-enriched-owner");
        assert_eq!(
            after_start.metadata,
            serde_json::json!({"owner":"enriched"})
        );
        assert!(after_start.stopped_at.is_none());

        let mut stop = ProcessStop::from_start(&enriched, Some(77)).with_stop_reason("merged-stop");
        stop.owner_role = "mt137-stop-owner".to_owned();
        stop.metadata_jsonb = serde_json::json!({"owner":"stop"});
        store
            .write_batch(vec![LedgerEvent::Stop(stop)])
            .await
            .expect("STOP applies its conflict update list");
        let stopped = read_process_row(&storage, process_uuid).await;
        assert_eq!(stopped.os_pid, Some(9137));
        assert_eq!(
            stopped.parent_session_id.as_deref(),
            Some("mt137-guard-session")
        );
        assert_eq!(
            stopped.sandbox_adapter_id.as_deref(),
            Some("windows_native_jail")
        );
        assert_eq!(stopped.started_at, enriched.started_at);
        assert_eq!(stopped.owner_role, "mt137-stop-owner");
        assert_eq!(stopped.metadata, serde_json::json!({"owner":"stop"}));
        assert_eq!(stopped.exit_code, Some(77));
        assert_eq!(stopped.stop_reason.as_deref(), Some("merged-stop"));
        assert!(stopped.stopped_at.is_some());

        enriched.owner_role = "mt137-post-stop-start".to_owned();
        store
            .write_batch(vec![LedgerEvent::Start(enriched)])
            .await
            .expect("post-STOP START merges without erasing terminal fields");
        let final_row = read_process_row(&storage, process_uuid).await;
        assert_eq!(final_row.os_pid, Some(9137));
        assert_eq!(final_row.owner_role, "mt137-post-stop-start");
        assert_eq!(final_row.exit_code, Some(77));
        assert_eq!(final_row.stop_reason.as_deref(), Some("merged-stop"));
        assert!(final_row.stopped_at.is_some());

        storage.shutdown().await.expect("close process guard store");
    }
}
