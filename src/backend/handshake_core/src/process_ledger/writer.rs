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
use surrealdb::types::SurrealValue;
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

    /// START conflict merge.
    ///
    /// Field-for-field equivalent of the previous
    /// `ON CONFLICT (process_uuid) DO UPDATE` clause: incoming values win for
    /// `engine_kind`, `owner_role` and both JSON columns, `COALESCE(new, old)`
    /// applies everywhere the incoming value is optional, `started_at` keeps the
    /// earliest of the two, and `stopped_at` / `exit_code` / `stop_reason` are
    /// NOT in the update list, so a replayed START can never erase a STOP that
    /// already landed.
    fn merge_start_onto(self, previous: Self) -> Self {
        Self {
            process_uuid: previous.process_uuid,
            os_pid: self.os_pid.or(previous.os_pid),
            parent_session_id: self.parent_session_id.or(previous.parent_session_id),
            parent_process_id: self.parent_process_id.or(previous.parent_process_id),
            sandbox_adapter_id: self.sandbox_adapter_id.or(previous.sandbox_adapter_id),
            sandbox_internal_id: self.sandbox_internal_id.or(previous.sandbox_internal_id),
            engine_kind: self.engine_kind,
            started_at: self.started_at.min(previous.started_at),
            stopped_at: previous.stopped_at,
            exit_code: previous.exit_code,
            stop_reason: previous.stop_reason,
            model_artifact_sha256: self
                .model_artifact_sha256
                .or(previous.model_artifact_sha256),
            work_profile_id: self.work_profile_id.or(previous.work_profile_id),
            owner_role: self.owner_role,
            owner_wp: self.owner_wp.or(previous.owner_wp),
            role_id: self.role_id.or(previous.role_id),
            wp_id: self.wp_id.or(previous.wp_id),
            mt_id: self.mt_id.or(previous.mt_id),
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot,
            metadata: self.metadata,
        }
    }

    /// STOP conflict merge.
    ///
    /// Mirrors the previous STOP `ON CONFLICT` update list: the stop triple is
    /// overwritten unconditionally, optional identity columns coalesce, and
    /// `parent_session_id`, `sandbox_adapter_id`, `engine_kind` and `started_at`
    /// are absent from the update list so the START row keeps ownership of them.
    fn merge_stop_onto(self, previous: Self) -> Self {
        Self {
            process_uuid: previous.process_uuid,
            os_pid: self.os_pid.or(previous.os_pid),
            parent_session_id: previous.parent_session_id,
            parent_process_id: self.parent_process_id.or(previous.parent_process_id),
            sandbox_adapter_id: previous.sandbox_adapter_id,
            sandbox_internal_id: self.sandbox_internal_id.or(previous.sandbox_internal_id),
            engine_kind: previous.engine_kind,
            started_at: previous.started_at,
            stopped_at: self.stopped_at,
            exit_code: self.exit_code,
            stop_reason: self.stop_reason,
            model_artifact_sha256: self
                .model_artifact_sha256
                .or(previous.model_artifact_sha256),
            work_profile_id: self.work_profile_id.or(previous.work_profile_id),
            owner_role: self.owner_role,
            owner_wp: self.owner_wp.or(previous.owner_wp),
            role_id: self.role_id.or(previous.role_id),
            wp_id: self.wp_id.or(previous.wp_id),
            mt_id: self.mt_id.or(previous.mt_id),
            sandbox_capabilities_snapshot: self.sandbox_capabilities_snapshot,
            metadata: self.metadata,
        }
    }
}

/// `ProcessLedgerStore` backed by the Handshake-managed embedded SurrealDB
/// store.
///
/// Every row is keyed by `process_uuid` (the record id), so a replayed batch
/// re-applies the same merge and converges on the same record. That keeps the
/// retry loop in [`flush_batch`] safe: on a store error the batch is retained
/// and re-sent whole, and re-sending an already-applied event is a no-op.
///
/// The embedded store is opened in-process and RocksDB holds an exclusive lock
/// on its directory, so the read-merge-write pair below has no cross-process
/// competitor; within the process, ledger rows are drained and written by the
/// single writer task in [`run_writer`].
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
    ) -> Result<(), ProcessLedgerError> {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    let previous: Option<ProcessLifecycleRow> = database
                        .select_one(PROCESS_LEDGER_TABLE_NAME, &record_id)
                        .await?;
                    let merged = match previous {
                        Some(previous) => match kind {
                            LedgerEventKind::Start => incoming.merge_start_onto(previous),
                            LedgerEventKind::Stop => incoming.merge_stop_onto(previous),
                        },
                        None => incoming,
                    };
                    let _: Option<ProcessLifecycleRow> = database
                        .upsert_one(PROCESS_LEDGER_TABLE_NAME, &record_id, merged)
                        .await?;
                    Ok(())
                })
            })
            .await
            .map_err(ProcessLedgerError::from)
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
                    )
                    .await?
                }
                LedgerEvent::Stop(stop) => {
                    self.apply(
                        stop.process_uuid.to_string(),
                        ProcessLifecycleRow::from_stop(&stop),
                        LedgerEventKind::Stop,
                    )
                    .await?
                }
            }
        }
        Ok(())
    }
}
