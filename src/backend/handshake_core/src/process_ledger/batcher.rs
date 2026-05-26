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
}

#[derive(Clone, Default)]
pub struct NoopOverflowSink;

impl ProcessLedgerOverflowSink for NoopOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}
