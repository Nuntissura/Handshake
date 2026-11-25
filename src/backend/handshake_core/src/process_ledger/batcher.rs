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

    /// The START identity this ledger accepted for `process_uuid`, while its
    /// STOP row is still outstanding.
    ///
    /// A terminal owner that did not author the START (for example the
    /// `SwarmCoordinator`, when the session factory recorded the START itself)
    /// must derive its STOP from this record. The authoritative STOP upsert
    /// only updates a lifecycle row whose immutable identity columns match
    /// exactly, so a STOP synthesized from the terminal owner's own defaults is
    /// rejected as a STOP identity conflict and the lifecycle never closes.
    pub fn recorded_start(&self, process_uuid: uuid::Uuid) -> Option<ProcessStart> {
        self.writer.recorded_start(process_uuid)
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

    pub(crate) fn try_reserve_fresh_reclaim_stop(&self) -> Result<ReservedProcessStop, ProcessLedgerError> {
        self.writer.try_reserve_fresh_reclaim_stop()
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
    pub(crate) fn has_pending_start_admissions(&self) -> bool {
        self.writer.has_pending_start_admissions()
    }

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
    writer_join: Arc<tokio::sync::Mutex<RetainedWriterJoin>>,
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
                writer_join: Arc::new(tokio::sync::Mutex::new(RetainedWriterJoin::new(writer_join))),
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
                writer_join: Arc::new(tokio::sync::Mutex::new(RetainedWriterJoin::new(writer_join))),
            }),
        }
    }

    pub fn ledger(&self) -> LedgerBatcher {
        self.inner.ledger.clone()
    }

    pub async fn drain_and_join(&self, timeout: Duration) -> LedgerDrainJoinOutcome {
        let caller_deadline = tokio::time::Instant::now() + timeout;
        let mut owns_grace = false;
        self.inner.ledger.begin_close();
        loop {
            let Ok(mut state) = tokio::time::timeout_at(caller_deadline, self.inner.writer_join.lock()).await else {
                return LedgerDrainJoinOutcome::Pending;
            };
            state.recover_failed_handoff();
            let grace_deadline = if state.abort_requested {
                state.deadline.unwrap_or(caller_deadline)
            } else {
                if state.deadline.is_none() {
                    state.deadline = Some(caller_deadline);
                    owns_grace = true;
                }
                state.deadline.expect("original graceful deadline")
            };
            if let Some(outcome) = &state.terminal {
                return match outcome {
                    RetainedWriterOutcome::Flushed => LedgerDrainJoinOutcome::AlreadyDrained,
                    other => other.public_outcome(),
                };
            }
            if state.handle.is_none() {
                let changed = Arc::clone(&state.changed);
                let notified = changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                // Failure is published outside this mutex. Recheck AFTER the
                // waiter is enabled so a failure before registration cannot
                // consume the only wake and strand a recoverable handle.
                state.recover_failed_handoff();
                if state.handle.is_some() || state.terminal.is_some() { continue; }
                drop(state);
                if tokio::time::timeout_at(caller_deadline, notified).await.is_err() {
                    return LedgerDrainJoinOutcome::Pending;
                }
                continue;
            }
            let observation_deadline = if state.abort_requested {
                caller_deadline
            } else {
                caller_deadline.min(grace_deadline)
            };
            let join = state.handle.as_mut().expect("retained writer handle");
            let result = tokio::time::timeout_at(observation_deadline, join).await;
            let outcome = match result {
                Ok(result) => writer_join_outcome(result, state.timed_out),
                Err(_) => {
                    let mut settlement_deadline = caller_deadline;
                    if !state.abort_requested {
                        if tokio::time::Instant::now() < grace_deadline {
                            // This observer's shorter budget expired without
                            // exhausting the original graceful phase.
                            return LedgerDrainJoinOutcome::Pending;
                        }
                        state.timed_out = true;
                        state.handle.as_ref().expect("retained writer handle").abort();
                        state.abort_requested = true;
                        if owns_grace {
                            // Only the original grace observer gets its one
                            // bounded post-abort terminal settlement allowance.
                            settlement_deadline = tokio::time::Instant::now() + timeout;
                        }
                    }
                    let join = state.handle.as_mut().expect("retained writer handle");
                    match tokio::time::timeout_at(settlement_deadline, join).await {
                        Ok(result) => writer_join_outcome(result, state.timed_out),
                        Err(_) => return LedgerDrainJoinOutcome::Pending,
                    }
                }
            };
            state.handle.take();
            state.terminal = Some(outcome.clone());
            state.changed.notify_waiters();
            return outcome.public_outcome();
        }
    }

    pub fn abort_and_join_blocking(&self, timeout: Duration) -> bool {
        self.inner.ledger.begin_close();
        abort_retained_writer(Arc::clone(&self.inner.writer_join), timeout)
    }

}

impl Drop for RetainedLedgerBatcherInner {
    fn drop(&mut self) {
        self.ledger.begin_close();
        if !abort_retained_writer(Arc::clone(&self.writer_join), Duration::from_secs(2)) {
            tracing::error!("process-ledger writer shutdown remains pending or failed");
            if self.writer_join.try_lock().map_or(true, |state| state.handle.is_some() || state.handoff_failed.load(std::sync::atomic::Ordering::Acquire)) {
                std::mem::forget(Arc::clone(&self.writer_join));
            }
        }
    }
}

#[derive(Clone)]
enum RetainedWriterOutcome {
    Flushed,
    WriterError(Arc<ProcessLedgerError>),
    JoinError,
    TimedOut,
}

impl RetainedWriterOutcome {
    fn public_outcome(&self) -> LedgerDrainJoinOutcome {
        match self {
            Self::Flushed => LedgerDrainJoinOutcome::Flushed,
            Self::WriterError(error) => LedgerDrainJoinOutcome::RetainedWriterError(Arc::clone(error)),
            Self::JoinError => LedgerDrainJoinOutcome::JoinError,
            Self::TimedOut => LedgerDrainJoinOutcome::TimedOut,
        }
    }
}

struct RetainedWriterJoin {
    handle: Option<JoinHandle<Result<(), ProcessLedgerError>>>,
    terminal: Option<RetainedWriterOutcome>,
    timed_out: bool,
    abort_requested: bool,
    deadline: Option<tokio::time::Instant>,
    changed: Arc<tokio::sync::Notify>,
    handoff_failed: Arc<std::sync::atomic::AtomicBool>,
    handoff: Option<Arc<Mutex<Option<JoinHandle<Result<(), ProcessLedgerError>>>>>>,
}

impl RetainedWriterJoin {
    fn new(handle: JoinHandle<Result<(), ProcessLedgerError>>) -> Self {
        Self { handle: Some(handle), terminal: None, timed_out: false, abort_requested: false, deadline: None, handoff: None,
            changed: Arc::new(tokio::sync::Notify::new()),
            handoff_failed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    fn recover_failed_handoff(&mut self) {
        if self.handoff_failed.swap(false, std::sync::atomic::Ordering::AcqRel) {
            if let Some(packet) = self.handoff.take() {
                self.handle = packet.lock().unwrap_or_else(|error| error.into_inner()).take();
            }
        }
    }

}

fn writer_join_outcome(result: Result<Result<(), ProcessLedgerError>, tokio::task::JoinError>, timed_out: bool) -> RetainedWriterOutcome {
    match result {
        Ok(Ok(())) if timed_out => RetainedWriterOutcome::TimedOut,
        Ok(Ok(())) => RetainedWriterOutcome::Flushed,
        Ok(Err(error)) => RetainedWriterOutcome::WriterError(Arc::new(error)),
        Err(error) if timed_out && error.is_cancelled() => RetainedWriterOutcome::TimedOut,
        Err(_) => RetainedWriterOutcome::JoinError,
    }
}

fn abort_retained_writer(owner: Arc<tokio::sync::Mutex<RetainedWriterJoin>>, timeout: Duration) -> bool {
    let Ok(mut state) = owner.try_lock() else { return false; };
    state.recover_failed_handoff();
    if state.terminal.is_some() {
        return true;
    }
    let Some(handle) = state.handle.take() else { return false; };
    if !state.abort_requested {
        handle.abort();
        state.abort_requested = true;
    }
    let packet = Arc::new(Mutex::new(Some(handle)));
    state.handoff = Some(Arc::clone(&packet));
    let timed_out = state.timed_out;
    let failed = Arc::clone(&state.handoff_failed);
    let handoff_changed = Arc::clone(&state.changed);
    drop(state);
    let worker_owner = Arc::clone(&owner);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let worker = move || {
        let handle = packet.lock().unwrap_or_else(|error| error.into_inner()).take();
        let Some(handle) = handle else { return; };
        let result = futures::executor::block_on(handle);
        let outcome = writer_join_outcome(result, timed_out);
        let mut state = worker_owner.blocking_lock();
        state.terminal = Some(outcome);
        state.handoff = None;
        state.changed.notify_waiters();
        drop(state);
        let _ = tx.send(true);
    };
    let helper = std::thread::Builder::new().name("handshake-ledger-retained-join".into()).spawn(worker);
    // A failed helper spawn leaves the aborted handle packet in retained state.
    if helper.is_err() {
        failed.store(true, std::sync::atomic::Ordering::Release);
        handoff_changed.notify_waiters();
        return false;
    }
    rx.recv_timeout(timeout).unwrap_or(false)
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
    /// The original typed failure retained by a shared writer owner.
    RetainedWriterError(Arc<ProcessLedgerError>),
    /// Terminal observation is incomplete; the retained owner still holds custody.
    Pending,
    /// The writer task panicked or was cancelled.
    JoinError,
    /// The graceful-drain deadline elapsed. The writer was then aborted and its
    /// terminal join result observed, so no detached writer survives shutdown.
    TimedOut,
    /// An earlier retained drain observed a successful flush and terminal join.
    AlreadyDrained,
}

/// WP-1 MT-013 (F1 graceful shutdown): close `ledger`'s writer channel and await
/// the spawned writer's `JoinHandle` under a bounded `timeout`, so a
/// just-enqueued embedded-model STOP row is durably flushed to the injected store BEFORE
/// embedded storage shutdown begins. `timeout` bounds the graceful wait; after the
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
            // writer must not outlive shutdown and later write through a store
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
