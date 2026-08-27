//! MT-191 Checkpoint write path: periodic + event-triggered + pre-shutdown.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::Duration;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, Notify};
use uuid::Uuid;

use super::checkpoint::{CheckpointStateKind, SessionCheckpoint};
use crate::flight_recorder::{
    fr_event_registry::FrEventId, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType, RecorderError,
};
use crate::storage::surreal::SurrealStorage;

pub const CHANNEL_CAPACITY: usize = 256;
const SINK_WRITE_MAX_ATTEMPTS: usize = 4;
const SINK_WRITE_RETRY_BASE_DELAY: Duration = Duration::from_millis(10);
const SINK_WRITE_RETRY_CAP: Duration = Duration::from_millis(100);
const SHUTDOWN_ABORT_JOIN_GRACE: Duration = Duration::from_millis(250);
const WRITER_OPEN: u8 = 0;
const WRITER_CLOSING: u8 = 1;
const WRITER_CLOSED: u8 = 2;

#[derive(Debug, Clone)]
pub struct CheckpointWriterConfig {
    pub period: Duration,
    pub channel_capacity: usize,
    pub batch_size: usize,
    pub shutdown_grace: Duration,
}

impl Default for CheckpointWriterConfig {
    fn default() -> Self {
        Self {
            period: Duration::from_secs(15),
            channel_capacity: CHANNEL_CAPACITY,
            batch_size: 32,
            shutdown_grace: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Error)]
pub enum CheckpointWriterError {
    #[error("channel is full (saturation)")]
    ChannelFull,
    #[error("send error")]
    Send,
    #[error("shutdown grace expired; checkpoint writer task was aborted")]
    ShutdownForced,
    #[error("shutdown grace expired and abort/drain failed: {0}")]
    ShutdownForcedWithError(String),
    #[error("flight recorder error: {0}")]
    Recorder(#[from] RecorderError),
    #[error("checkpoint sink error: {0}")]
    Sink(String),
    #[error("checkpoint writer task failed: {0}")]
    Task(String),
}

impl Clone for CheckpointWriterError {
    fn clone(&self) -> Self {
        match self {
            Self::ChannelFull => Self::ChannelFull,
            Self::Send => Self::Send,
            Self::ShutdownForced => Self::ShutdownForced,
            Self::ShutdownForcedWithError(error) => Self::ShutdownForcedWithError(error.clone()),
            Self::Recorder(error) => Self::Recorder(match error {
                RecorderError::InvalidEvent(reason) => RecorderError::InvalidEvent(reason.clone()),
                RecorderError::SinkError(reason) => RecorderError::SinkError(reason.clone()),
                RecorderError::LockError => RecorderError::LockError,
            }),
            Self::Sink(error) => Self::Sink(error.clone()),
            Self::Task(error) => Self::Task(error.clone()),
        }
    }
}

struct CheckpointLifecycle {
    state: AtomicU8,
    submission_gate: StdMutex<()>,
    terminal: StdMutex<Option<Result<(), CheckpointWriterError>>>,
    changed: Notify,
}

impl CheckpointLifecycle {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(WRITER_OPEN),
            submission_gate: StdMutex::new(()),
            terminal: StdMutex::new(None),
            changed: Notify::new(),
        }
    }

    fn begin_shutdown(&self) -> bool {
        let _gate = self
            .submission_gate
            .lock()
            .expect("checkpoint submission gate");
        if self.state.load(Ordering::Acquire) != WRITER_OPEN {
            return false;
        }
        self.state.store(WRITER_CLOSING, Ordering::Release);
        self.changed.notify_waiters();
        true
    }

    fn complete(&self, result: Result<(), CheckpointWriterError>) {
        *self.terminal.lock().expect("checkpoint terminal result") = Some(result);
        self.state.store(WRITER_CLOSED, Ordering::Release);
        self.changed.notify_waiters();
    }

    async fn terminal_result(&self) -> Result<(), CheckpointWriterError> {
        loop {
            let notified = self.changed.notified();
            if let Some(result) = self
                .terminal
                .lock()
                .expect("checkpoint terminal result")
                .clone()
            {
                return result;
            }
            notified.await;
        }
    }

    #[cfg(test)]
    async fn wait_until_closing(&self) {
        loop {
            let notified = self.changed.notified();
            if self.state.load(Ordering::Acquire) != WRITER_OPEN {
                return;
            }
            notified.await;
        }
    }
}

#[async_trait::async_trait]
pub trait StateSnapshotter: Send + Sync {
    async fn snapshot(&self) -> Option<SessionCheckpoint>;
}

#[async_trait::async_trait]
pub trait CheckpointSink: Send + Sync {
    async fn write_batch(
        &self,
        batch: Vec<SessionCheckpoint>,
    ) -> Result<u64, CheckpointWriterError>;
}

/// In-memory `CheckpointSink` for tests. Production wires
/// [`SurrealCheckpointSink`] (below), which writes checkpoint rows into the
/// embedded SurrealDB `kernel_session_checkpoint` table.
pub struct InMemoryCheckpointSink {
    pub written: Mutex<Vec<SessionCheckpoint>>,
}

impl InMemoryCheckpointSink {
    pub fn new() -> Self {
        Self {
            written: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryCheckpointSink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CheckpointSink for InMemoryCheckpointSink {
    async fn write_batch(
        &self,
        batch: Vec<SessionCheckpoint>,
    ) -> Result<u64, CheckpointWriterError> {
        let mut buf = self.written.lock().await;
        let n = batch.len() as u64;
        buf.extend(batch);
        Ok(n)
    }
}

const SESSION_CHECKPOINT_TABLE: &str = "kernel_session_checkpoint";

/// One `kernel_session_checkpoint` record.
///
/// Mirrors the `SCHEMAFULL` table definition in
/// `storage/surreal/schema.surql`. `checkpoint_id` is also the record id, which
/// is what makes a duplicate write detectable without a secondary lookup path.
#[derive(Debug, Clone, SurrealValue)]
struct SessionCheckpointRow {
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: Value,
    state_kind: String,
    pending_artifacts: Vec<String>,
    created_at_utc: DateTime<Utc>,
    created_by_process: i64,
    schema_version: i64,
}

#[derive(Debug, Clone, SurrealValue)]
struct CheckpointWriteBindings {
    record_id: RecordId,
    row: SessionCheckpointRow,
}

const WRITE_CHECKPOINT_ATOMIC: &str = r#"
RETURN {
    LET $existing = SELECT VALUE id FROM ONLY $record_id;
    IF $existing = NONE {
        CREATE $record_id CONTENT $row RETURN NONE;
        RETURN 'inserted';
    };
    RETURN 'duplicate';
};
"#;

impl From<&SessionCheckpoint> for SessionCheckpointRow {
    fn from(cp: &SessionCheckpoint) -> Self {
        Self {
            checkpoint_id: cp.checkpoint_id.as_uuid(),
            session_id: cp.session_id,
            model_session_id: cp.model_session_id,
            last_event_ledger_seq: cp.last_event_ledger_seq,
            compact_state: cp.compact_state.clone(),
            state_kind: cp.state_kind.as_str().to_string(),
            pending_artifacts: cp.pending_artifacts.clone(),
            created_at_utc: cp.created_at_utc,
            created_by_process: i64::from(cp.created_by_process),
            schema_version: i64::from(cp.schema_version),
        }
    }
}

/// Production `CheckpointSink` backed by the Handshake-managed embedded
/// SurrealDB store.
///
/// Each `write_batch` writes the drained rows into `kernel_session_checkpoint`,
/// keyed by the UUID-v7 `checkpoint_id` record id. The field names, the
/// `state_kind` text encoding and the append-only convention match the ones
/// MT-193's restart-resume path uses, so cadence-driven checkpoints and
/// recovery-time checkpoints land in the same table with the same shape.
///
/// Rows are append-only and never updated in place, which preserves the
/// `ORDER BY created_at_utc DESC` latest-wins read pattern. A checkpoint id that
/// is already present is skipped rather than rewritten and is not counted in the
/// returned row count, which is the behaviour the previous
/// `ON CONFLICT (checkpoint_id) DO NOTHING` clause provided: an idempotent retry
/// of an already-persisted batch neither errors nor double-counts.
#[derive(Clone)]
pub struct SurrealCheckpointSink {
    storage: SurrealStorage,
}

impl SurrealCheckpointSink {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    /// Writes one checkpoint with a single compare-or-create statement.
    /// Returns `true` for a new row and `false` for every duplicate record id.
    /// This deliberately preserves `ON CONFLICT (checkpoint_id) DO NOTHING`:
    /// the first append is immutable, and a conflicting duplicate cannot block
    /// later independent rows in the retained batch.
    async fn write_one(&self, row: SessionCheckpointRow) -> Result<bool, CheckpointWriterError> {
        let record_id = RecordId::new(SESSION_CHECKPOINT_TABLE, row.checkpoint_id.to_string());
        let outcome: Option<String> = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first::<String, _>(
                            WRITE_CHECKPOINT_ATOMIC,
                            CheckpointWriteBindings { record_id, row },
                        )
                        .await
                })
            })
            .await
            .map_err(|error| CheckpointWriterError::Sink(error.to_string()))?;
        match outcome.as_deref() {
            Some("inserted") => Ok(true),
            Some("duplicate") => Ok(false),
            other => Err(CheckpointWriterError::Sink(format!(
                "HSK-SESSION-CHECKPOINT-WRITE-OUTCOME-INVALID: {other:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl CheckpointSink for SurrealCheckpointSink {
    async fn write_batch(
        &self,
        batch: Vec<SessionCheckpoint>,
    ) -> Result<u64, CheckpointWriterError> {
        if batch.is_empty() {
            return Ok(0);
        }

        let mut written = 0u64;
        for checkpoint in batch.iter() {
            let inserted = self
                .write_one(SessionCheckpointRow::from(checkpoint))
                .await?;
            if inserted {
                written += 1;
            }
        }
        Ok(written)
    }
}

pub struct CheckpointWriter {
    cfg: CheckpointWriterConfig,
    sink: Arc<dyn CheckpointSink>,
}

async fn flush_batch_with_retry(
    sink: &Arc<dyn CheckpointSink>,
    batch: &mut Vec<SessionCheckpoint>,
) -> Result<(), CheckpointWriterError> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut attempt = 1usize;
    loop {
        match sink.write_batch(batch.clone()).await {
            Ok(_) => {
                batch.clear();
                return Ok(());
            }
            Err(_error) if attempt < SINK_WRITE_MAX_ATTEMPTS => {
                let multiplier = 1u32 << (attempt - 1).min(6);
                let delay = SINK_WRITE_RETRY_BASE_DELAY
                    .saturating_mul(multiplier)
                    .min(SINK_WRITE_RETRY_CAP);
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

impl CheckpointWriter {
    pub fn new(cfg: CheckpointWriterConfig, sink: Arc<dyn CheckpointSink>) -> Self {
        Self { cfg, sink }
    }

    /// Spawn the background drain task and return a handle for submission +
    /// shutdown.
    pub fn start(self) -> CheckpointHandle {
        let (tx, mut rx) = mpsc::channel::<SessionCheckpoint>(self.cfg.channel_capacity);
        let sink = Arc::clone(&self.sink);
        let batch_size = self.cfg.batch_size;
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let join = tokio::spawn(async move {
            let mut buffer: Vec<SessionCheckpoint> = Vec::with_capacity(batch_size);
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown_rx.recv() => {
                        // Reject every sender before draining. Lifecycle state
                        // already blocks well-behaved clones, while close also
                        // seals the channel against a sender that raced before
                        // observing Closing. `recv` then drains to exhaustion.
                        rx.close();
                        while let Some(cp) = rx.recv().await {
                            buffer.push(cp);
                            if buffer.len() >= batch_size {
                                flush_batch_with_retry(&sink, &mut buffer).await?;
                            }
                        }
                        flush_batch_with_retry(&sink, &mut buffer).await?;
                        return Ok(());
                    }
                    Some(cp) = rx.recv() => {
                        buffer.push(cp);
                        if buffer.len() >= batch_size {
                            flush_batch_with_retry(&sink, &mut buffer).await?;
                        }
                    }
                    else => {
                        flush_batch_with_retry(&sink, &mut buffer).await?;
                        return Ok(());
                    }
                }
            }
        });
        CheckpointHandle {
            tx,
            shutdown_tx,
            join: Arc::new(Mutex::new(Some(join))),
            shutdown_grace: self.cfg.shutdown_grace,
            last_checkpoint: Arc::new(StdMutex::new(None)),
            lifecycle: Arc::new(CheckpointLifecycle::new()),
        }
    }
}

#[derive(Clone)]
pub struct CheckpointHandle {
    tx: mpsc::Sender<SessionCheckpoint>,
    shutdown_tx: mpsc::Sender<()>,
    join: Arc<Mutex<Option<tokio::task::JoinHandle<Result<(), CheckpointWriterError>>>>>,
    shutdown_grace: Duration,
    last_checkpoint: Arc<StdMutex<Option<(Uuid, Uuid)>>>,
    lifecycle: Arc<CheckpointLifecycle>,
}

impl CheckpointHandle {
    /// Non-blocking submit. Returns ChannelFull on saturation; caller should
    /// emit FR-EVT-CHECKPOINT-OVERFLOW.
    pub fn submit(&self, cp: SessionCheckpoint) -> Result<(), CheckpointWriterError> {
        let session_id = cp.session_id;
        let checkpoint_id = cp.checkpoint_id.as_uuid();
        let _gate = self
            .lifecycle
            .submission_gate
            .lock()
            .expect("checkpoint submission gate");
        if self.lifecycle.state.load(Ordering::Acquire) != WRITER_OPEN {
            return Err(CheckpointWriterError::Send);
        }
        match self.tx.try_send(cp) {
            Ok(()) => {
                self.remember_checkpoint(session_id, checkpoint_id);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => Err(CheckpointWriterError::ChannelFull),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(CheckpointWriterError::Send),
        }
    }

    pub async fn submit_with_flight_recorder(
        &self,
        cp: SessionCheckpoint,
        recorder: &dyn FlightRecorder,
    ) -> Result<(), CheckpointWriterError> {
        let session_id = cp.session_id;
        let checkpoint_id = cp.checkpoint_id.as_uuid();
        match self.submit(cp) {
            Ok(()) => Ok(()),
            Err(CheckpointWriterError::ChannelFull) => {
                let _ = record_checkpoint_event(
                    recorder,
                    FrEventId::CheckpointOverflow,
                    session_id,
                    checkpoint_id,
                    "session_checkpoint_writer",
                )
                .await;
                Err(CheckpointWriterError::ChannelFull)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn submit_event_triggered(
        &self,
        mut cp: SessionCheckpoint,
    ) -> Result<(), CheckpointWriterError> {
        cp.state_kind = CheckpointStateKind::EventTriggered;
        self.submit(cp)
    }

    pub async fn shutdown(self) -> Result<(), CheckpointWriterError> {
        self.shutdown_inner(None).await
    }

    pub async fn shutdown_with_flight_recorder(
        self,
        recorder: &dyn FlightRecorder,
    ) -> Result<(), CheckpointWriterError> {
        self.shutdown_inner(Some(recorder)).await
    }

    async fn shutdown_inner(
        self,
        recorder: Option<&dyn FlightRecorder>,
    ) -> Result<(), CheckpointWriterError> {
        if self.lifecycle.begin_shutdown() {
            // Shutdown ownership lives in a detached coordinator, not in the
            // first caller's future. Cancelling that caller therefore cannot
            // strand every clone in Closing or detach the writer task.
            let coordinator = self.clone();
            let lifecycle = Arc::clone(&self.lifecycle);
            tokio::spawn(async move {
                let result = coordinator.finish_shutdown().await;
                lifecycle.complete(result);
            });
        }

        let result = self.lifecycle.terminal_result().await;
        if matches!(
            &result,
            Err(CheckpointWriterError::ShutdownForced)
                | Err(CheckpointWriterError::ShutdownForcedWithError(_))
        ) {
            if let Some(recorder) = recorder {
                let (session_id, checkpoint_id) = self.latest_checkpoint();
                let _ = record_checkpoint_event(
                    recorder,
                    FrEventId::CheckpointShutdownForced,
                    session_id,
                    checkpoint_id,
                    "session_checkpoint_writer",
                )
                .await;
            }
        }
        result
    }

    async fn finish_shutdown(&self) -> Result<(), CheckpointWriterError> {
        let _ = self.shutdown_tx.send(()).await;
        let join_opt = self.join.lock().await.take();
        if let Some(join) = join_opt {
            let mut join = join;
            match tokio::time::timeout(self.shutdown_grace, &mut join).await {
                Ok(Ok(result)) => result?,
                Ok(Err(error)) => return Err(CheckpointWriterError::Task(error.to_string())),
                Err(_) => {
                    join.abort();
                    let abort_error =
                        match tokio::time::timeout(SHUTDOWN_ABORT_JOIN_GRACE, &mut join).await {
                            Ok(Err(error)) if error.is_cancelled() => None,
                            Ok(Ok(Ok(()))) => None,
                            Ok(Ok(Err(error))) => Some(format!(
                                "writer returned after timeout with terminal error: {error}"
                            )),
                            Ok(Err(error)) => Some(format!(
                                "aborted writer join returned a non-cancellation error: {error}"
                            )),
                            Err(_) => Some(format!(
                                "aborted writer did not terminate within {} ms",
                                SHUTDOWN_ABORT_JOIN_GRACE.as_millis()
                            )),
                        };
                    return match abort_error {
                        Some(error) => Err(CheckpointWriterError::ShutdownForcedWithError(error)),
                        None => Err(CheckpointWriterError::ShutdownForced),
                    };
                }
            }
        }
        Ok(())
    }

    fn remember_checkpoint(&self, session_id: Uuid, checkpoint_id: Uuid) {
        *self.last_checkpoint.lock().expect("last checkpoint lock") =
            Some((session_id, checkpoint_id));
    }

    fn latest_checkpoint(&self) -> (Uuid, Uuid) {
        self.last_checkpoint
            .lock()
            .expect("last checkpoint lock")
            .unwrap_or_else(|| (Uuid::now_v7(), Uuid::now_v7()))
    }
}

async fn record_checkpoint_event(
    recorder: &dyn FlightRecorder,
    event_id: FrEventId,
    session_id: Uuid,
    checkpoint_id: Uuid,
    actor_id: &str,
) -> Result<(), RecorderError> {
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        session_id,
        serde_json::json!({
            "schema_version": "hsk.fr.session_checkpoint@1",
            "event_id": event_id.as_str(),
            "session_id": session_id.to_string(),
            "checkpoint_id": checkpoint_id.to_string(),
        }),
    )
    .with_actor_id(actor_id);
    recorder.record_event(event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_checkpoint::checkpoint::SessionCheckpoint;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    struct ScriptedSink {
        failures_remaining: AtomicUsize,
        calls: AtomicUsize,
        written: Mutex<Vec<SessionCheckpoint>>,
    }

    struct BarrierSink {
        entered: Arc<tokio::sync::Barrier>,
        release: Arc<tokio::sync::Barrier>,
        written: Mutex<Vec<SessionCheckpoint>>,
    }

    impl BarrierSink {
        fn new() -> Self {
            Self {
                entered: Arc::new(tokio::sync::Barrier::new(2)),
                release: Arc::new(tokio::sync::Barrier::new(2)),
                written: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl CheckpointSink for BarrierSink {
        async fn write_batch(
            &self,
            batch: Vec<SessionCheckpoint>,
        ) -> Result<u64, CheckpointWriterError> {
            self.entered.wait().await;
            self.release.wait().await;
            let count = batch.len() as u64;
            self.written.lock().await.extend(batch);
            Ok(count)
        }
    }

    fn checkpoint(sequence: i64) -> SessionCheckpoint {
        SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            sequence,
            serde_json::json!({"sequence": sequence}),
            CheckpointStateKind::Periodic,
        )
        .expect("build checkpoint")
    }

    impl ScriptedSink {
        fn new(failures: usize) -> Self {
            Self {
                failures_remaining: AtomicUsize::new(failures),
                calls: AtomicUsize::new(0),
                written: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl CheckpointSink for ScriptedSink {
        async fn write_batch(
            &self,
            batch: Vec<SessionCheckpoint>,
        ) -> Result<u64, CheckpointWriterError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self
                .failures_remaining
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                    remaining.checked_sub(1)
                })
                .is_ok()
            {
                return Err(CheckpointWriterError::Sink("scripted failure".to_owned()));
            }
            let count = batch.len() as u64;
            self.written.lock().await.extend(batch);
            Ok(count)
        }
    }

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid checkpoint test path"),
        )
        .await
        .expect("open embedded checkpoint store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap checkpoint schema");
        storage
    }

    #[tokio::test]
    async fn event_triggered_write_observable() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let cp = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({"k": "v"}),
            CheckpointStateKind::EventTriggered,
        )
        .unwrap();
        handle.submit_event_triggered(cp).await.unwrap();
        // Give the drainer a moment to consume.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let written = sink.written.lock().await;
        assert_eq!(written.len(), 1);
    }

    #[tokio::test]
    async fn channel_full_returns_error() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        // Tiny channel + slow consumer (no spawn) — submit returns ChannelFull.
        let (tx, _rx) = mpsc::channel::<SessionCheckpoint>(1);
        let (shutdown_tx, _shutdown_rx) = mpsc::channel::<()>(1);
        let handle = CheckpointHandle {
            tx,
            shutdown_tx,
            join: Arc::new(Mutex::new(None)),
            shutdown_grace: Duration::from_secs(1),
            last_checkpoint: Arc::new(StdMutex::new(None)),
            lifecycle: Arc::new(CheckpointLifecycle::new()),
        };
        let cp1 = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let cp2 = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            serde_json::json!({}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        handle.submit(cp1).unwrap();
        let r = handle.submit(cp2);
        assert!(matches!(r, Err(CheckpointWriterError::ChannelFull)));
        drop(sink);
    }

    #[tokio::test]
    async fn shutdown_flushes_pending() {
        let sink = Arc::new(InMemoryCheckpointSink::new());
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 16,
                batch_size: 8,
                shutdown_grace: Duration::from_secs(1),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        for _ in 0..5 {
            let cp = SessionCheckpoint::new(
                Uuid::now_v7(),
                Uuid::now_v7(),
                0,
                serde_json::json!({}),
                CheckpointStateKind::Periodic,
            )
            .unwrap();
            handle.submit(cp).unwrap();
        }
        handle.shutdown().await.unwrap();
        let written = sink.written.lock().await;
        assert_eq!(written.len(), 5);
    }

    #[tokio::test]
    async fn clone_submission_is_rejected_after_shutdown_enters_closing() {
        let sink = Arc::new(BarrierSink::new());
        let handle = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        )
        .start();
        let submitter = handle.clone();
        let lifecycle = Arc::clone(&handle.lifecycle);
        handle.submit(checkpoint(1)).expect("queue initial write");
        sink.entered.wait().await;

        let shutdown = tokio::spawn(handle.shutdown());
        lifecycle.wait_until_closing().await;
        assert!(matches!(
            submitter.submit(checkpoint(2)),
            Err(CheckpointWriterError::Send)
        ));
        sink.release.wait().await;
        shutdown
            .await
            .expect("shutdown task joins")
            .expect("shutdown succeeds");
        assert_eq!(sink.written.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_shutdown_callers_wait_for_the_same_terminal_result() {
        let sink = Arc::new(BarrierSink::new());
        let handle = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        )
        .start();
        let lifecycle = Arc::clone(&handle.lifecycle);
        handle.submit(checkpoint(1)).expect("queue blocked write");
        sink.entered.wait().await;

        let start = Arc::new(tokio::sync::Barrier::new(3));
        let first_handle = handle.clone();
        let first_start = Arc::clone(&start);
        let first = tokio::spawn(async move {
            first_start.wait().await;
            first_handle.shutdown().await
        });
        let second_start = Arc::clone(&start);
        let second = tokio::spawn(async move {
            second_start.wait().await;
            handle.shutdown().await
        });
        start.wait().await;
        lifecycle.wait_until_closing().await;
        tokio::task::yield_now().await;
        assert!(!first.is_finished());
        assert!(!second.is_finished());
        sink.release.wait().await;

        let first_result = first.await.expect("first shutdown task joins");
        let second_result = second.await.expect("second shutdown task joins");
        assert!(first_result.is_ok());
        assert!(second_result.is_ok());
        assert_eq!(sink.written.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn mt137_cancelled_first_shutdown_caller_cannot_strand_followers() {
        let sink = Arc::new(BarrierSink::new());
        let handle = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 1,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        )
        .start();
        let follower = handle.clone();
        let lifecycle = Arc::clone(&handle.lifecycle);
        handle.submit(checkpoint(1)).expect("queue blocked write");
        sink.entered.wait().await;

        let leader = tokio::spawn(handle.shutdown());
        lifecycle.wait_until_closing().await;
        leader.abort();
        assert!(leader
            .await
            .expect_err("leader caller is cancelled")
            .is_cancelled());

        let follower_shutdown = tokio::spawn(follower.shutdown());
        tokio::task::yield_now().await;
        assert!(
            !follower_shutdown.is_finished(),
            "follower must await the shared writer drain"
        );
        sink.release.wait().await;
        tokio::time::timeout(Duration::from_secs(3), follower_shutdown)
            .await
            .expect("follower shutdown remains bounded")
            .expect("follower task joins")
            .expect("shared shutdown succeeds after leader cancellation");
        assert_eq!(sink.written.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn failed_batch_is_retained_and_retried_before_shutdown_succeeds() {
        let sink = Arc::new(ScriptedSink::new(2));
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 4,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        handle
            .submit(
                SessionCheckpoint::new(
                    Uuid::now_v7(),
                    Uuid::now_v7(),
                    1,
                    serde_json::json!({"retry": true}),
                    CheckpointStateKind::Periodic,
                )
                .unwrap(),
            )
            .unwrap();

        handle.shutdown().await.expect("retry eventually persists");
        assert_eq!(sink.calls.load(Ordering::SeqCst), 3);
        assert_eq!(sink.written.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn terminal_sink_failure_is_returned_by_shutdown() {
        let sink = Arc::new(ScriptedSink::new(SINK_WRITE_MAX_ATTEMPTS));
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 4,
                shutdown_grace: Duration::from_secs(2),
            },
            Arc::clone(&sink) as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        handle
            .submit(
                SessionCheckpoint::new(
                    Uuid::now_v7(),
                    Uuid::now_v7(),
                    2,
                    serde_json::json!({"persist": "must-not-be-discarded"}),
                    CheckpointStateKind::PreShutdown,
                )
                .unwrap(),
            )
            .unwrap();

        let error = handle
            .shutdown()
            .await
            .expect_err("shutdown must expose terminal sink failure");
        assert!(matches!(error, CheckpointWriterError::Sink(_)));
        assert_eq!(sink.calls.load(Ordering::SeqCst), SINK_WRITE_MAX_ATTEMPTS);
        assert!(sink.written.lock().await.is_empty());
    }

    #[tokio::test]
    async fn surreal_sink_skips_all_duplicates_without_stranding_later_rows() {
        let directory = tempfile::tempdir().expect("temporary checkpoint root");
        let storage = open(&directory.path().join("store")).await;
        let sink = SurrealCheckpointSink::new(storage.clone());
        let checkpoint = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            3,
            serde_json::json!({"payload": "original"}),
            CheckpointStateKind::EventTriggered,
        )
        .unwrap();

        assert_eq!(sink.write_batch(vec![checkpoint.clone()]).await.unwrap(), 1);
        assert_eq!(sink.write_batch(vec![checkpoint.clone()]).await.unwrap(), 0);

        let checkpoint_id = checkpoint.checkpoint_id;
        let original_state = checkpoint.compact_state.clone();
        let mut conflicting = checkpoint;
        conflicting.compact_state = serde_json::json!({"payload": "conflicting"});
        let trailing = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            4,
            serde_json::json!({"payload": "trailing"}),
            CheckpointStateKind::Periodic,
        )
        .unwrap();
        let trailing_id = trailing.checkpoint_id;
        assert_eq!(
            sink.write_batch(vec![conflicting, trailing]).await.unwrap(),
            1,
            "conflicting duplicate must be skipped so the valid tail persists"
        );

        let original_record_id = checkpoint_id.as_uuid().to_string();
        let trailing_record_id = trailing_id.as_uuid().to_string();
        let (original, trailing): (Option<SessionCheckpointRow>, Option<SessionCheckpointRow>) =
            storage
                .with_data_operation(move |database| {
                    Box::pin(async move {
                        Ok((
                            database
                                .select_one(SESSION_CHECKPOINT_TABLE, &original_record_id)
                                .await?,
                            database
                                .select_one(SESSION_CHECKPOINT_TABLE, &trailing_record_id)
                                .await?,
                        ))
                    })
                })
                .await
                .expect("read duplicate and trailing checkpoints");
        assert_eq!(
            original.expect("original checkpoint remains").compact_state,
            original_state
        );
        assert_eq!(
            trailing
                .expect("valid checkpoint after conflict persists")
                .compact_state,
            serde_json::json!({"payload": "trailing"})
        );
        storage.shutdown().await.expect("close checkpoint store");
    }

    #[tokio::test]
    async fn mt137_checkpoint_writer_state_survives_store_reopen() {
        let directory = tempfile::tempdir().expect("temporary checkpoint root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let sink = Arc::new(SurrealCheckpointSink::new(storage.clone()));
        let writer = CheckpointWriter::new(
            CheckpointWriterConfig {
                period: Duration::from_secs(60),
                channel_capacity: 4,
                batch_size: 4,
                shutdown_grace: Duration::from_secs(5),
            },
            sink as Arc<dyn CheckpointSink>,
        );
        let handle = writer.start();
        let session_id = Uuid::now_v7();
        let model_session_id = Uuid::now_v7();
        let checkpoint = SessionCheckpoint::new(
            session_id,
            model_session_id,
            137,
            serde_json::json!({"phase":"restart-proof"}),
            CheckpointStateKind::PreShutdown,
        )
        .expect("build checkpoint");
        let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
        handle.submit(checkpoint).expect("queue checkpoint");
        handle.shutdown().await.expect("flush checkpoint writer");
        storage.shutdown().await.expect("close checkpoint store");
        drop(storage);

        let reopened = open(&path).await;
        let record_id = checkpoint_id.to_string();
        let row: SessionCheckpointRow = reopened
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one(SESSION_CHECKPOINT_TABLE, &record_id)
                        .await
                })
            })
            .await
            .expect("query reopened checkpoint")
            .expect("read reopened checkpoint");
        assert_eq!(row.checkpoint_id, checkpoint_id);
        assert_eq!(row.session_id, session_id);
        assert_eq!(row.model_session_id, model_session_id);
        assert_eq!(row.last_event_ledger_seq, 137);
        assert_eq!(
            row.compact_state,
            serde_json::json!({"phase":"restart-proof"})
        );
        assert_eq!(row.state_kind, "pre_shutdown");
        reopened.shutdown().await.expect("close reopened store");
    }
}
