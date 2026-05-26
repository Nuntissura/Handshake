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
use sqlx::{postgres::PgPool, Postgres, Transaction};
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

use super::table::{
    LedgerEvent, LedgerEventKind, ProcessStart, ProcessStop, PROCESS_LEDGER_DEFAULT_BATCH_SIZE,
    PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, PROCESS_LEDGER_DEFAULT_FLUSH_INTERVAL_MS,
    PROCESS_LEDGER_MIGRATION_SQL, PROCESS_START_INSERT_SQL, PROCESS_STOP_UPSERT_SQL,
};

pub const FR_EVT_LEDGER_OVERFLOW: &str = "FR_EVT_LEDGER_OVERFLOW";
const PROCESS_LEDGER_SOURCE_COMPONENT: &str = "process_ledger_writer";

static GLOBAL_DEGRADED_WRITERS: AtomicUsize = AtomicUsize::new(0);

pub fn is_degraded() -> bool {
    GLOBAL_DEGRADED_WRITERS.load(Ordering::SeqCst) > 0
}

#[derive(Debug, Error)]
pub enum ProcessLedgerError {
    #[error("PROCESS_LEDGER_INVALID_CONFIG: {0}")]
    InvalidConfig(String),
    #[error("PROCESS_LEDGER_OVERFLOW_EMIT: {0}")]
    OverflowEmit(String),
    #[error("PROCESS_LEDGER_STORE: {0}")]
    Store(String),
    #[error("PROCESS_LEDGER_POSTGRES: {source}")]
    Postgres { source: sqlx::Error },
    #[error("PROCESS_LEDGER_EVENT: {0}")]
    Event(String),
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
        let writer = Self {
            sender,
            overflow_sink: Arc::clone(&overflow_sink),
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::clone(&overflow_count),
            capacity: config.capacity,
        };
        let join = tokio::spawn(run_writer(
            receiver,
            store,
            overflow_sink,
            config,
            degraded,
            overflow_count,
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
        let writer = Self {
            sender,
            overflow_sink,
            degraded: Arc::clone(&degraded),
            overflow_count: Arc::new(AtomicU64::new(0)),
            capacity: config.capacity,
        };
        let drain = ProcessLedgerDrain {
            receiver: Mutex::new(receiver),
            degraded,
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
                flush_batch(&store, &mut batch, &self.degraded).await?;
            }
        }
        if !batch.is_empty() {
            flush_batch(&store, &mut batch, &self.degraded).await?;
        }
        Ok(())
    }
}

async fn run_writer(
    mut receiver: Receiver<LedgerEvent>,
    store: Arc<dyn ProcessLedgerStore>,
    overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    config: WriterConfig,
    degraded: Arc<AtomicBool>,
    overflow_count: Arc<AtomicU64>,
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
                    let _ = flush_batch(&store, &mut batch, &degraded).await;
                }
            }
            _ = ticker.tick() => {
                if !batch.is_empty() {
                    let _ = flush_batch(&store, &mut batch, &degraded).await;
                }
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(&store, &mut batch, &degraded).await?;
    }
    Ok(())
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

pub struct PostgresProcessLedgerStore {
    pool: PgPool,
}

impl PostgresProcessLedgerStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn apply_migration(&self) -> Result<(), ProcessLedgerError> {
        for statement in PROCESS_LEDGER_MIGRATION_SQL
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ProcessLedgerStore for PostgresProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for event in events {
            match event {
                LedgerEvent::Start(start) => insert_start(&mut tx, &start).await?,
                LedgerEvent::Stop(stop) => upsert_stop(&mut tx, &stop).await?,
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

async fn insert_start(
    tx: &mut Transaction<'_, Postgres>,
    start: &ProcessStart,
) -> Result<(), ProcessLedgerError> {
    sqlx::query(PROCESS_START_INSERT_SQL)
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
        .bind(start.sandbox_capabilities_snapshot.to_string())
        .bind(start.metadata_jsonb.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn upsert_stop(
    tx: &mut Transaction<'_, Postgres>,
    stop: &ProcessStop,
) -> Result<(), ProcessLedgerError> {
    sqlx::query(PROCESS_STOP_UPSERT_SQL)
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
        .bind(stop.sandbox_capabilities_snapshot.to_string())
        .bind(stop.metadata_jsonb.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
