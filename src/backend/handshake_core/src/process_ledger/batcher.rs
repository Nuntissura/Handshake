use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::task::JoinHandle;

use super::{
    LedgerOverflowEvent, ProcessLedgerDrain, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessLedgerStore, ProcessLedgerWriter, ProcessRuntimeOwner, ProcessStart, ProcessStop,
    ReservedProcessLifecycle, ReservedProcessStop, WriterConfig, PROCESS_LEDGER_BATCH_SIZE,
    PROCESS_LEDGER_FLUSH_INTERVAL_MS, PROCESS_LEDGER_RING_CAPACITY,
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
    runtime_owner: Option<ProcessRuntimeOwner>,
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
                runtime_owner: None,
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
                runtime_owner: None,
            },
            drain,
        ))
    }

    pub fn with_runtime_owner(mut self, runtime_owner: ProcessRuntimeOwner) -> Self {
        self.runtime_owner = Some(runtime_owner);
        self
    }

    fn attach_runtime_owner_to_start(&self, mut event: ProcessStart) -> ProcessStart {
        if event.runtime_owner.is_none() {
            event.runtime_owner = self.runtime_owner.clone();
        }
        event
    }

    fn attach_runtime_owner_to_stop(&self, mut event: ProcessStop) -> ProcessStop {
        if event.runtime_owner.is_none() {
            event.runtime_owner = self.runtime_owner.clone();
        }
        event
    }

    pub fn record_start(&self, event: ProcessStart) -> Result<(), ProcessLedgerError> {
        self.writer
            .append_start(self.attach_runtime_owner_to_start(event))
    }

    pub fn record_stop(&self, event: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.writer
            .append_stop(self.attach_runtime_owner_to_stop(event))
    }

    pub fn record_start_lossless(&self, event: ProcessStart) -> Result<(), ProcessLedgerError> {
        self.writer
            .append_start_lossless(self.attach_runtime_owner_to_start(event))
    }

    pub fn record_stop_lossless(&self, event: ProcessStop) -> Result<(), ProcessLedgerError> {
        self.writer
            .append_stop_lossless(self.attach_runtime_owner_to_stop(event))
    }

    /// Reserve START+STOP capacity for a complete resource set before any
    /// artifact is opened or child is spawned.
    pub fn try_reserve_lifecycles(
        &self,
        count: usize,
    ) -> Result<Vec<ReservedProcessLifecycle>, ProcessLedgerError> {
        self.writer
            .try_reserve_lifecycles(count)
            .map(|reservations| {
                reservations
                    .into_iter()
                    .map(|reservation| reservation.with_runtime_owner(self.runtime_owner.clone()))
                    .collect()
            })
    }

    pub fn try_reserve_reclaim_stop(&self) -> Result<ReservedProcessStop, ProcessLedgerError> {
        self.writer.try_reserve_reclaim_stop()
    }

    /// Graceful-shutdown STOP append that waits a bounded time for the live
    /// writer to free capacity. Call before [`Self::begin_close`].
    pub async fn record_stop_lossless_bounded(
        &self,
        event: ProcessStop,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        self.writer
            .append_stop_lossless_bounded(self.attach_runtime_owner_to_stop(event), timeout)
            .await
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

#[derive(Clone)]
pub struct RetainedLedgerBatcher {
    inner: Arc<RetainedLedgerBatcherInner>,
}

struct RetainedLedgerBatcherInner {
    ledger: LedgerBatcher,
    writer_join: Mutex<Option<JoinHandle<Result<(), ProcessLedgerError>>>>,
}

impl RetainedLedgerBatcher {
    pub fn spawn(
        store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
        config: LedgerBatcherConfig,
    ) -> Self {
        let (ledger, writer_join) = LedgerBatcher::spawn(store, overflow_sink, config);
        Self {
            inner: Arc::new(RetainedLedgerBatcherInner {
                ledger,
                writer_join: Mutex::new(Some(writer_join)),
            }),
        }
    }

    pub fn spawn_with_runtime_owner(
        store: Arc<dyn ProcessLedgerStore>,
        overflow_sink: Arc<dyn ProcessLedgerOverflowSink>,
        config: LedgerBatcherConfig,
        runtime_owner: ProcessRuntimeOwner,
    ) -> Self {
        let (ledger, writer_join) = LedgerBatcher::spawn(store, overflow_sink, config);
        let ledger = ledger.with_runtime_owner(runtime_owner);
        Self {
            inner: Arc::new(RetainedLedgerBatcherInner {
                ledger,
                writer_join: Mutex::new(Some(writer_join)),
            }),
        }
    }

    pub fn ledger(&self) -> LedgerBatcher {
        self.inner.ledger.clone()
    }

    pub async fn drain_and_join(&self, timeout: Duration) -> LedgerDrainJoinOutcome {
        let writer_join = self
            .inner
            .writer_join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(writer_join) = writer_join else {
            return LedgerDrainJoinOutcome::AlreadyDrained;
        };
        drain_and_join_ledger_writer(&self.inner.ledger, writer_join, timeout).await
    }

    /// Emergency synchronous teardown for constructor-failure and implicit-Drop
    /// paths. Tokio `JoinHandle::abort` is not completion: the cancelled task may
    /// still be running until its next yield. Move the handle to a plain helper
    /// thread, abort it, and observe its terminal state for a bounded interval.
    /// A `false` result means the helper still owns the join handle and the
    /// caller must retain any OS liveness lease that protects the writer.
    pub fn abort_and_join_blocking(&self, timeout: Duration) -> bool {
        self.inner.ledger.begin_close();
        let writer_join = self
            .inner
            .writer_join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(writer_join) = writer_join else {
            return true;
        };
        writer_join.abort();
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-ledger-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(writer_join);
                let _ = completed_tx.send(());
            });
        let Ok(_helper) = helper else {
            return false;
        };
        completed_rx.recv_timeout(timeout).is_ok()
    }
}

impl Drop for RetainedLedgerBatcherInner {
    fn drop(&mut self) {
        self.ledger.begin_close();
        let writer_join = self
            .writer_join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(writer_join) = writer_join else {
            return;
        };
        writer_join.abort();
        // This is the last retained owner. Observe cancellation before the
        // process-ledger store/pool can be torn down. The writer is an async
        // Tokio task (not spawn_blocking), so abort reaches a yield boundary.
        let (completed_tx, completed_rx) = std::sync::mpsc::sync_channel(1);
        let helper = std::thread::Builder::new()
            .name("handshake-ledger-final-drop-join".to_string())
            .spawn(move || {
                let _ = futures::executor::block_on(writer_join);
                let _ = completed_tx.send(());
            });
        if helper.is_ok() && completed_rx.recv_timeout(Duration::from_secs(2)).is_err() {
            tracing::error!(
                "process-ledger writer did not terminate within the bounded final-drop deadline"
            );
        }
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
    /// The graceful-drain deadline elapsed. The writer was then aborted and its
    /// terminal join result observed, so no detached writer survives shutdown.
    TimedOut,
    /// The retained writer handle was already consumed by an earlier bounded
    /// drain call.
    AlreadyDrained,
}

/// WP-1 MT-013 (F1 graceful shutdown): close `ledger`'s writer channel and await
/// the spawned writer's `JoinHandle` under a bounded `timeout`, so a
/// just-enqueued embedded-model STOP row is durably flushed to PostgreSQL BEFORE
/// the managed cluster is stopped. `timeout` bounds the graceful wait; after the
/// deadline the task is aborted and awaited so it cannot detach into teardown.
pub async fn drain_and_join_ledger_writer(
    ledger: &LedgerBatcher,
    mut writer_join: JoinHandle<Result<(), ProcessLedgerError>>,
    timeout: Duration,
) -> LedgerDrainJoinOutcome {
    ledger.begin_close();
    match tokio::time::timeout(timeout, &mut writer_join).await {
        Ok(Ok(Ok(()))) => LedgerDrainJoinOutcome::Flushed,
        Ok(Ok(Err(error))) => LedgerDrainJoinOutcome::WriterError(error),
        Ok(Err(_join_error)) => LedgerDrainJoinOutcome::JoinError,
        Err(_elapsed) => {
            // Dropping a Tokio JoinHandle detaches the task. A timed-out ledger
            // writer must not outlive shutdown and later write through a pool
            // the caller is already tearing down, so cancel it and observe its
            // terminal join result before returning.
            writer_join.abort();
            let _ = writer_join.await;
            LedgerDrainJoinOutcome::TimedOut
        }
    }
}

#[derive(Clone, Default)]
pub struct NoopOverflowSink;

impl ProcessLedgerOverflowSink for NoopOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}
