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
    kernel::{sandbox::CancellationToken, KernelError, KernelEvent},
    process_ledger::{
        LedgerEvent, LedgerOverflowEvent, ProcessLedgerDrain, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter,
    },
};

use super::crdt_workspace::{SharedCrdtWorkspace, SharedLeaseRegistry};
use super::session::{SessionResult, SessionStep, SwarmSession, SwarmSessionRuntime};

/// Canonical shared exclusive-lease resource id used by lease-contention
/// scenarios. The lease summary on `SwarmReport` reports occupancy for this id.
pub const SHARED_LEASE_RESOURCE: &str = "shared-lease-resource";

pub trait SwarmScenario: Send + Sync + 'static {
    fn scenario_id(&self) -> &str;
    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep>;

    /// When set, the harness spawns a watcher that flips the real platform
    /// cancellation token after this delay, so cancellable `MutateCrdtField`
    /// steps in flight observe a real mid-run cancellation. `None` (default)
    /// means no cancellation is injected.
    fn cancel_after_ms(&self) -> Option<u64> {
        None
    }
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

        // Real shared CRDT workspace, lease registry, and platform cancellation
        // token, shared by every session so contention is *measured* from
        // concurrent execution rather than fabricated arithmetically.
        let workspace = Arc::new(SharedCrdtWorkspace::new());
        let leases = Arc::new(SharedLeaseRegistry::new());
        let cancellation = CancellationToken::new();
        let runtime = SwarmSessionRuntime::with_crdt_workspace(
            Arc::new(writer),
            Arc::new(Mutex::new(Vec::<KernelEvent>::new())),
            workspace.clone(),
            leases,
            cancellation.clone(),
        );
        let scenario_id = self.scenario.scenario_id().to_string();
        let mut join_set = JoinSet::new();

        // Optional real mid-run cancellation: flip the platform token after a
        // real delay so cancellable steps in flight observe it.
        let cancel_watcher = self.scenario.cancel_after_ms().map(|delay_ms| {
            let cancellation = cancellation.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                cancellation.cancel();
            })
        });

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

        if let Some(watcher) = cancel_watcher {
            watcher.abort();
            let _ = watcher.await;
        }

        drain_process_rows(&drain, process_store.clone()).await?;
        attach_process_rows(&mut sessions, &process_store.events()?);
        sessions.sort_by_key(|session| session.session_idx);

        let contention_events = workspace
            .contention_summary()
            .into_iter()
            .enumerate()
            .map(|(idx, (event_id, contention_kind, detail))| ContentionEvent {
                event_id,
                session_id: format!("{scenario_id}-contention-{idx}"),
                contention_kind,
                detail,
            })
            .collect();

        let crdt_workspace = SwarmCrdtWorkspaceSummary {
            silent_overwrites: workspace.silent_overwrites(),
            conflict_report_count: workspace.conflict_count(),
            revision_rejection_count: workspace.revision_rejection_count(),
            max_lease_wait_ms: workspace.max_lease_wait_ms(),
            cancellation_count: workspace.cancellation_count(),
            distinct_cancelled_sessions: workspace.distinct_cancelled_sessions(),
            conflict_signature: workspace.conflict_signature(),
            // The kernel CRDT conflict-presence projection is the authority for
            // the conflict / rejection counts; building it here proves the
            // harness evidence passes the real kernel validation path.
            projection_conflict_count: workspace
                .build_conflict_presence_projection()
                .map(|projection| projection.pending_conflicts.len())
                .map_err(SwarmHarnessError::KernelEvent)?,
            lease_grants_completed: workspace.lease_grants_completed(SHARED_LEASE_RESOURCE),
            max_simultaneous_lease_holders: workspace
                .max_simultaneous_lease_holders(SHARED_LEASE_RESOURCE),
        };

        Ok(SwarmReport {
            n: self.n,
            scenario_id,
            sessions,
            total_duration_ms: started.elapsed().as_millis().max(1),
            contention_events,
            ledger_overflow_count: overflow_sink.overflow_count()?,
            crdt_workspace,
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
    /// Real measured CRDT contention from the shared workspace. Empty / zero for
    /// scenarios that do not issue `MutateCrdtField` steps.
    pub crdt_workspace: SwarmCrdtWorkspaceSummary,
}

/// Measured (never arithmetic) CRDT contention evidence produced by the shared
/// workspace runtime over the course of a swarm run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmCrdtWorkspaceSummary {
    pub silent_overwrites: usize,
    pub conflict_report_count: usize,
    pub revision_rejection_count: usize,
    pub max_lease_wait_ms: u64,
    pub cancellation_count: usize,
    /// Distinct sessions that observed a real mid-mutation cancellation.
    pub distinct_cancelled_sessions: usize,
    pub conflict_signature: String,
    /// Conflict count read back off the real kernel
    /// `build_crdt_conflict_presence_projection`, proving the harness evidence
    /// passes the real kernel CRDT validation path.
    pub projection_conflict_count: usize,
    /// Real exclusive-lease grants completed on the shared lease resource.
    pub lease_grants_completed: usize,
    /// Measured high-water mark of simultaneous lease holders on the shared
    /// resource (1 for a correct exclusive lease).
    pub max_simultaneous_lease_holders: usize,
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
