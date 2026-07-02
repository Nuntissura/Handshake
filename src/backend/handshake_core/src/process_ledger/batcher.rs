use std::{sync::Arc, time::Duration};

use tokio::task::JoinHandle;

use super::{
    LedgerOverflowEvent, ProcessLedgerDrain, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessLedgerStore, ProcessLedgerWriter, ProcessStart, ProcessStop, WriterConfig,
    PROCESS_LEDGER_BATCH_SIZE, PROCESS_LEDGER_FLUSH_INTERVAL_MS, PROCESS_LEDGER_RING_CAPACITY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerBatcherConfig {
    pub capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
}

impl Default for LedgerBatcherConfig {
    fn default() -> Self {
        Self {
            capacity: PROCESS_LEDGER_RING_CAPACITY,
            batch_size: PROCESS_LEDGER_BATCH_SIZE,
            flush_interval: Duration::from_millis(PROCESS_LEDGER_FLUSH_INTERVAL_MS),
        }
    }
}

impl From<LedgerBatcherConfig> for WriterConfig {
    fn from(value: LedgerBatcherConfig) -> Self {
        Self {
            capacity: value.capacity,
            batch_size: value.batch_size,
            flush_interval: value.flush_interval,
        }
    }
}

#[derive(Clone)]
pub struct LedgerBatcher {
    writer: Arc<ProcessLedgerWriter>,
}

impl LedgerBatcher {
    pub fn spawn(
        store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
        config: LedgerBatcherConfig,
    ) -> (Self, JoinHandle<Result<(), ProcessLedgerError>>) {
        let (writer, join) = ProcessLedgerWriter::spawn(store, overflow_sink, config.into());
        (
            Self {
                writer: Arc::new(writer),
            },
            join,
        )
    }

    pub fn manual_for_tests(
        config: LedgerBatcherConfig,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
    ) -> Result<(Self, ProcessLedgerDrain), ProcessLedgerError> {
        let (writer, drain) =
            ProcessLedgerWriter::new_manual_with_config(config.into(), overflow_sink)?;
        Ok((
            Self {
                writer: Arc::new(writer),
            },
            drain,
        ))
    }

    pub fn record_start(&self, event: ProcessStart) -> Result<(), ProcessLedgerError> {
        self.writer.append_start(event)
    }

    pub fn record_stop(&self, event: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.writer.append_stop(event)
    }

    /// WP-1 MT-013 (F1 graceful shutdown): signal the spawned background writer
    /// to stop accepting new rows, flush everything already queued, and
    /// terminate so its `JoinHandle` resolves. Callable from any clone. Pair with
    /// [`drain_and_join_ledger_writer`] (or await the `JoinHandle` directly) to
    /// bound the flush.
    pub fn begin_close(&self) {
        self.writer.begin_close();
    }
}

/// Outcome of the bounded graceful-shutdown drain-and-join for the process
/// ledger's background writer (WP-1 MT-013 F1).
#[derive(Debug)]
pub enum LedgerDrainJoinOutcome {
    /// The writer flushed all queued rows and terminated cleanly within the
    /// timeout.
    Flushed,
    /// The writer terminated but its final flush surfaced a store error (rows may
    /// be recovered by the boot-time reclaim sweep on next start).
    WriterError(ProcessLedgerError),
    /// The writer task panicked or was cancelled.
    JoinError,
    /// The writer did not terminate within the timeout (rows may be recovered by
    /// the boot-time reclaim sweep on next start).
    TimedOut,
}

/// WP-1 MT-013 (F1 graceful shutdown): close `ledger`'s writer channel and await
/// the spawned writer's `JoinHandle` under a bounded `timeout`, so a
/// just-enqueued embedded-model STOP row is durably flushed to PostgreSQL BEFORE
/// the managed cluster is stopped. Returns the terminal outcome; never blocks
/// past `timeout`.
pub async fn drain_and_join_ledger_writer(
    ledger: &LedgerBatcher,
    writer_join: JoinHandle<Result<(), ProcessLedgerError>>,
    timeout: Duration,
) -> LedgerDrainJoinOutcome {
    ledger.begin_close();
    match tokio::time::timeout(timeout, writer_join).await {
        Ok(Ok(Ok(()))) => LedgerDrainJoinOutcome::Flushed,
        Ok(Ok(Err(error))) => LedgerDrainJoinOutcome::WriterError(error),
        Ok(Err(_join_error)) => LedgerDrainJoinOutcome::JoinError,
        Err(_elapsed) => LedgerDrainJoinOutcome::TimedOut,
    }
}

#[derive(Clone, Default)]
pub struct NoopOverflowSink;

impl ProcessLedgerOverflowSink for NoopOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}
