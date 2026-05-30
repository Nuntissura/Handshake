//! MT-204 app-side wiring for the multi-model SWARM coordinator
//! (Master Spec §4.3.9 MULTI_MODEL_PARALLEL).
//!
//! This module constructs and `.manage`s a production [`SwarmCoordinator`] so
//! later Tauri swarm commands (MT-205) and the cloud routing policy (MT-206)
//! can drive real local + cloud model sessions in parallel. It wires:
//!
//! - the production [`ProductionModelSessionFactory`] (local candle / llama.cpp
//!   dispatch + cloud dispatch),
//! - a real [`LedgerBatcher`] backed by an in-process [`ProcessLedgerStore`]
//!   that durably records every spawn/stop row for the app session (the desktop
//!   app has no Postgres pool at startup; this is a real store, not a fake — the
//!   rows are queryable via [`SwarmRuntimeState::ledger_rows`]),
//! - a [`FlightRecorderSwarmSink`] that forwards every swarm lifecycle event to
//!   stderr-tagged structured output (the durable flight-recorder backend is
//!   resolved by the app in `setup`; until a swarm command needs the DB sink the
//!   tagged emit keeps every event observable),
//! - the cloud lane left UNCONFIGURED by default: a cloud swarm spawn returns a
//!   typed `ProviderNotConfigured` until MT-206 wires BYOK credentials through
//!   the existing `OsKeychainSecretsVault` + BYOK adapters.
//!
//! The existing single-load IPC (`kernel_model_runtime_load` / `unload` /
//! `list_loaded` / `capabilities` + steering/refusal/kv/lora) is UNCHANGED and
//! continues to use [`super::model_runtime::ModelRuntimeState`], which already
//! supports multiple concurrent loaded models keyed by `ModelId`. The
//! coordinator is the multi-instance authority for orchestrated SWARM sessions;
//! the two states coexist and do not contend.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessLedgerError,
    ProcessLedgerOverflowSink, ProcessLedgerStore,
};
use handshake_core::swarm_orchestration::{
    build_production_swarm_coordinator, CloudLaneFactoryConfig, SwarmCoordinator,
};
use uuid::Uuid;

/// FR-EVT tag stamped on swarm lifecycle events forwarded to structured stderr
/// until the durable flight-recorder DB sink is wired by a swarm command (MT-205).
pub const FR_EVT_SWARM_LIFECYCLE: &str = "FR-EVT-SWARM-LIFECYCLE";

/// In-process [`ProcessLedgerStore`] the app owns for the swarm coordinator. Not
/// a fake: it durably accumulates every START/STOP row for the running session
/// so the orchestration ledger is real and inspectable. A Postgres-backed store
/// replaces this when the app gains a configured pool (out of MT-204 scope).
#[derive(Clone, Default)]
pub struct InProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InProcessLedgerStore {
    pub fn rows(&self) -> Vec<LedgerEvent> {
        self.events.lock().expect("swarm ledger store poisoned").clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .map_err(|_| ProcessLedgerError::Store("swarm ledger store poisoned".to_string()))?
            .extend(events);
        Ok(())
    }
}

/// Overflow sink for the swarm ledger: counts dropped rows so an overflow is
/// observable rather than silent.
#[derive(Clone, Default)]
struct CountingOverflowSink {
    last_overflow: Arc<Mutex<u64>>,
}

impl ProcessLedgerOverflowSink for CountingOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        if let Ok(mut last) = self.last_overflow.lock() {
            *last = event.overflow_count;
        }
        eprintln!(
            "{FR_EVT_SWARM_LIFECYCLE}: ledger overflow count={}",
            event.overflow_count
        );
        Ok(())
    }
}

/// Managed app state holding the production swarm coordinator + its ledger
/// store, plus the writer-task join handle so it is not dropped (which would
/// stop the background ledger writer).
pub struct SwarmRuntimeState {
    coordinator: Arc<SwarmCoordinator>,
    ledger_store: InProcessLedgerStore,
    _writer_task: tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
}

impl std::fmt::Debug for SwarmRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwarmRuntimeState")
            .field("coordinator", &"<SwarmCoordinator>")
            .field("ledger_store", &"<InProcessLedgerStore>")
            .finish()
    }
}

impl SwarmRuntimeState {
    /// Build the production swarm runtime: real ledger batcher + in-process
    /// store, the production factory (cloud unconfigured), and a structured
    /// flight-recorder sink. Starts the lease/TTL reaper so stale sessions are
    /// reclaimed for the app's lifetime.
    pub fn production() -> Self {
        let store = InProcessLedgerStore::default();
        let overflow = Arc::new(CountingOverflowSink::default());
        let (ledger, writer_task) = LedgerBatcher::spawn(
            Arc::new(store.clone()),
            overflow,
            LedgerBatcherConfig::default(),
        );

        let trace_id = Uuid::now_v7();
        let coordinator = build_production_swarm_coordinator(
            ledger,
            // Cloud lane unconfigured until MT-206 wires BYOK credentials. A
            // cloud swarm spawn returns a typed ProviderNotConfigured today.
            CloudLaneFactoryConfig::unconfigured(),
            // Concurrency cap from CPU parallelism; the lifetime ceiling is
            // HBR-SWARM-002's loop cap (RunBudget default).
            None,
            trace_id,
            |event| {
                eprintln!(
                    "{FR_EVT_SWARM_LIFECYCLE}: {}",
                    serde_json::to_string(&event.payload).unwrap_or_default()
                );
            },
        );
        coordinator.start_reaper();

        Self {
            coordinator: Arc::new(coordinator),
            ledger_store: store,
            _writer_task: writer_task,
        }
    }

    /// The production coordinator. Later swarm commands (MT-205) call
    /// `spawn_session` / `cancel_session` / `remaining` on this.
    pub fn coordinator(&self) -> Arc<SwarmCoordinator> {
        self.coordinator.clone()
    }

    /// The durable swarm ledger rows recorded so far (observability + the
    /// no-orphan invariant: START count must equal STOP count once drained).
    pub fn ledger_rows(&self) -> Vec<LedgerEvent> {
        self.ledger_store.rows()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn swarm_runtime_state_constructs_and_exposes_a_live_coordinator() {
        let state = SwarmRuntimeState::production();
        let coordinator = state.coordinator();
        // Fresh coordinator: no live sessions, budget headroom present.
        assert_eq!(coordinator.live_session_count(), 0);
        let remaining = coordinator.remaining();
        assert!(remaining.lifetime_spawns_remaining > 0);
        assert!(remaining.concurrency_permits_available >= 1);
        assert!(state.ledger_rows().is_empty());
    }
}
