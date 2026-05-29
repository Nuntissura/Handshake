use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    kernel::{KernelError, KernelEvent},
    process_ledger::{
        LedgerEvent, LedgerOverflowEvent, ProcessLedgerDrain, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter,
    },
};

use super::session::{SessionResult, SessionStep, SwarmSession, SwarmSessionRuntime};

pub trait SwarmScenario: Send + Sync + 'static {
    fn scenario_id(&self) -> &str;
    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep>;
}

#[derive(Clone, Debug)]
pub struct SwarmHarness<S> {
    n: usize,
    scenario: S,
}

impl<S> SwarmHarness<S>
where
    S: SwarmScenario,
{
    pub fn new(n: usize, scenario: S) -> Self {
        Self { n, scenario }
    }

    pub async fn run(self) -> Result<SwarmReport, SwarmHarnessError> {
        if self.n == 0 {
            return Err(SwarmHarnessError::InvalidConfig(
                "n must be greater than zero".to_string(),
            ));
        }

        let started = Instant::now();
        let process_store = Arc::new(InMemoryProcessLedgerStore::default());
        let overflow_sink = Arc::new(InMemoryOverflowSink::default());
        let (writer, drain) = ProcessLedgerWriter::new_manual(
            self.n.saturating_mul(8).max(16),
            overflow_sink.clone(),
        )?;
        let runtime = SwarmSessionRuntime::new(
            Arc::new(writer),
            Arc::new(Mutex::new(Vec::<KernelEvent>::new())),
        );
        let scenario_id = self.scenario.scenario_id().to_string();
        let mut join_set = JoinSet::new();

        for session_idx in 0..self.n {
            let session_id = Uuid::now_v7().to_string();
            let steps = self.scenario.session_steps(session_idx);
            let session = SwarmSession::new(session_idx, session_id, steps, runtime.clone());
            join_set.spawn(async move { session.run().await });
        }

        let mut sessions = Vec::with_capacity(self.n);
        while let Some(result) = join_set.join_next().await {
            sessions.push(result.map_err(|error| SwarmHarnessError::Join(error.to_string()))??);
        }

        drain_process_rows(&drain, process_store.clone()).await?;
        attach_process_rows(&mut sessions, &process_store.events()?);
        sessions.sort_by_key(|session| session.session_idx);

        Ok(SwarmReport {
            n: self.n,
            scenario_id,
            sessions,
            total_duration_ms: started.elapsed().as_millis().max(1),
            contention_events: Vec::new(),
            ledger_overflow_count: overflow_sink.overflow_count()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwarmReport {
    pub n: usize,
    pub scenario_id: String,
    pub sessions: Vec<SessionResult>,
    pub total_duration_ms: u128,
    pub contention_events: Vec<ContentionEvent>,
    pub ledger_overflow_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentionEvent {
    pub event_id: String,
    pub session_id: String,
    pub contention_kind: String,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum SwarmHarnessError {
    #[error("SWARM_HARNESS_INVALID_CONFIG: {0}")]
    InvalidConfig(String),
    #[error("SWARM_HARNESS_JOIN: {0}")]
    Join(String),
    #[error("SWARM_HARNESS_KERNEL_EVENT: {0}")]
    KernelEvent(String),
    #[error("SWARM_HARNESS_PROCESS_LEDGER: {0}")]
    ProcessLedger(#[from] ProcessLedgerError),
    #[error("SWARM_HARNESS_POISONED: {0}")]
    Poisoned(String),
}

impl From<KernelError> for SwarmHarnessError {
    fn from(value: KernelError) -> Self {
        Self::KernelEvent(value.to_string())
    }
}

async fn drain_process_rows(
    drain: &ProcessLedgerDrain,
    store: Arc<InMemoryProcessLedgerStore>,
) -> Result<(), SwarmHarnessError> {
    drain.drain_available_to(store).await?;
    Ok(())
}

fn attach_process_rows(sessions: &mut [SessionResult], events: &[LedgerEvent]) {
    for session in sessions {
        session.process_ledger_rows = events
            .iter()
            .filter(|event| event.parent_session_id() == Some(session.session_id.as_str()))
            .cloned()
            .collect();
    }
}

#[derive(Clone, Default)]
struct InMemoryProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Result<Vec<LedgerEvent>, SwarmHarnessError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| SwarmHarnessError::Poisoned("process ledger store".to_string()))
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .map_err(|_| ProcessLedgerError::Store("process ledger store poisoned".to_string()))?
            .extend(events);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl InMemoryOverflowSink {
    fn overflow_count(&self) -> Result<u64, SwarmHarnessError> {
        self.events
            .lock()
            .map(|events| events.last().map(|event| event.overflow_count).unwrap_or(0))
            .map_err(|_| SwarmHarnessError::Poisoned("overflow sink".to_string()))
    }
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .map_err(|_| ProcessLedgerError::Store("overflow sink poisoned".to_string()))?
            .push(event);
        Ok(())
    }
}
