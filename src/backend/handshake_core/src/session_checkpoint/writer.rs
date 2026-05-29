//! MT-191 Checkpoint write path: periodic + event-triggered + pre-shutdown.

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use super::checkpoint::{CheckpointStateKind, SessionCheckpoint};
use crate::flight_recorder::{
    fr_event_registry::FrEventId, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType, RecorderError,
};

pub const CHANNEL_CAPACITY: usize = 256;

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
    #[error("flight recorder error: {0}")]
    Recorder(#[from] RecorderError),
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
/// `PostgresCheckpointSink` (small wrapper around `RoleMailboxRepository`-style
/// batch insert; left to MT-193 wiring).
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

pub struct CheckpointWriter {
    cfg: CheckpointWriterConfig,
    sink: Arc<dyn CheckpointSink>,
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
                        // Drain remaining channel content then exit.
                        while let Ok(cp) = rx.try_recv() {
                            buffer.push(cp);
                            if buffer.len() >= batch_size {
                                let _ = sink.write_batch(std::mem::take(&mut buffer)).await;
                            }
                        }
                        if !buffer.is_empty() {
                            let _ = sink.write_batch(std::mem::take(&mut buffer)).await;
                        }
                        break;
                    }
                    Some(cp) = rx.recv() => {
                        buffer.push(cp);
                        if buffer.len() >= batch_size {
                            let _ = sink.write_batch(std::mem::take(&mut buffer)).await;
                        }
                    }
                    else => {
                        break;
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
        }
    }
}

#[derive(Clone)]
pub struct CheckpointHandle {
    tx: mpsc::Sender<SessionCheckpoint>,
    shutdown_tx: mpsc::Sender<()>,
    join: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    shutdown_grace: Duration,
    last_checkpoint: Arc<StdMutex<Option<(Uuid, Uuid)>>>,
}

impl CheckpointHandle {
    /// Non-blocking submit. Returns ChannelFull on saturation; caller should
    /// emit FR-EVT-CHECKPOINT-OVERFLOW.
    pub fn submit(&self, cp: SessionCheckpoint) -> Result<(), CheckpointWriterError> {
        let session_id = cp.session_id;
        let checkpoint_id = cp.checkpoint_id.as_uuid();
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
        match self.tx.try_send(cp) {
            Ok(()) => {
                self.remember_checkpoint(session_id, checkpoint_id);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
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
            Err(mpsc::error::TrySendError::Closed(_)) => Err(CheckpointWriterError::Send),
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
        let _ = self.shutdown_tx.send(()).await;
        let join_opt = self.join.lock().await.take();
        if let Some(join) = join_opt {
            let mut join = join;
            match tokio::time::timeout(self.shutdown_grace, &mut join).await {
                Ok(_) => {}
                Err(_) => {
                    join.abort();
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
                    return Err(CheckpointWriterError::ShutdownForced);
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
    use uuid::Uuid;

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
}
