//! Observable lifecycle events + the [`SwarmEventSink`] abstraction.
//!
//! The coordinator never reaches into the flight recorder directly; it emits
//! typed [`SwarmEvent`]s through a [`SwarmEventSink`]. Production wires a
//! [`FlightRecorderSwarmSink`] that maps each event to a
//! [`crate::flight_recorder::FlightRecorderEvent`]; tests wire a
//! [`RecordingSwarmSink`] that captures events for assertions.
//!
//! ## FR-EVT-SWARM-* registry note
//!
//! The canonical `FrEventId` enum + its JSON manifest live under `.GOV/` and
//! are locked by an alignment test (`tests/fr_event_registry_tests.rs`) that
//! fails CI if the Rust enum and the on-disk manifest drift. This backend wave
//! is constrained to product code only and must not edit `.GOV/`. So the
//! FR-EVT-SWARM-* identifiers are defined here as their own self-contained,
//! round-trippable constant table ([`SwarmFrEventId`]) — the same shape and
//! discipline as `FrEventId` — ready to be folded into the master `FrEventId`
//! enum + `.GOV/` manifest by the governance-owning wave. Until then the
//! production sink stamps the FR-EVT-SWARM-* id into the event payload's
//! `fr_event_id` field so downstream filtering still works.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use uuid::Uuid;

use super::ids::ModelInstanceId;
use super::state::ModelSessionState;

/// Self-contained FR-EVT-SWARM-* identifier table. Canonical case is
/// UPPER-KEBAB-CASE after the `FR-EVT-` prefix, matching the governance
/// registry convention exactly so a future fold into `FrEventId` is mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwarmFrEventId {
    SessionSpawned,
    SessionReady,
    SessionGenerating,
    SessionCancelled,
    SessionCompleted,
    SessionFailed,
    ResourceAllocated,
    ResourceEvicted,
    BreakerTripped,
    LeaseExpired,
    SpawnRejected,
    // rank-3: VM/sandbox worktree lifecycle (each emits one FR event so the
    // Flight Recorder can replay/audit per-worktree state and the board can
    // drill down by worktree).
    WorktreeCreated,
    WorktreeMounted,
    WorktreeReclaimed,
    WorktreeCleanupFailed,
    // rank-7 groundwork: calendar-scheduled spin-up / teardown fires.
    ScheduledSpinupFired,
    ScheduledTeardownFired,
}

impl SwarmFrEventId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionSpawned => "FR-EVT-SWARM-SESSION-SPAWNED",
            Self::SessionReady => "FR-EVT-SWARM-SESSION-READY",
            Self::SessionGenerating => "FR-EVT-SWARM-SESSION-GENERATING",
            Self::SessionCancelled => "FR-EVT-SWARM-SESSION-CANCELLED",
            Self::SessionCompleted => "FR-EVT-SWARM-SESSION-COMPLETED",
            Self::SessionFailed => "FR-EVT-SWARM-SESSION-FAILED",
            Self::ResourceAllocated => "FR-EVT-SWARM-RESOURCE-ALLOCATED",
            Self::ResourceEvicted => "FR-EVT-SWARM-RESOURCE-EVICTED",
            Self::BreakerTripped => "FR-EVT-SWARM-BREAKER-TRIPPED",
            Self::LeaseExpired => "FR-EVT-SWARM-LEASE-EXPIRED",
            Self::SpawnRejected => "FR-EVT-SWARM-SPAWN-REJECTED",
            Self::WorktreeCreated => "FR-EVT-SWARM-WORKTREE-CREATED",
            Self::WorktreeMounted => "FR-EVT-SWARM-WORKTREE-MOUNTED",
            Self::WorktreeReclaimed => "FR-EVT-SWARM-WORKTREE-RECLAIMED",
            Self::WorktreeCleanupFailed => "FR-EVT-SWARM-WORKTREE-CLEANUP-FAILED",
            Self::ScheduledSpinupFired => "FR-EVT-SWARM-SCHED-SPINUP-FIRED",
            Self::ScheduledTeardownFired => "FR-EVT-SWARM-SCHED-TEARDOWN-FIRED",
        }
    }

    pub fn all() -> &'static [SwarmFrEventId] {
        &[
            Self::SessionSpawned,
            Self::SessionReady,
            Self::SessionGenerating,
            Self::SessionCancelled,
            Self::SessionCompleted,
            Self::SessionFailed,
            Self::ResourceAllocated,
            Self::ResourceEvicted,
            Self::BreakerTripped,
            Self::LeaseExpired,
            Self::SpawnRejected,
            Self::WorktreeCreated,
            Self::WorktreeMounted,
            Self::WorktreeReclaimed,
            Self::WorktreeCleanupFailed,
            Self::ScheduledSpinupFired,
            Self::ScheduledTeardownFired,
        ]
    }

    pub fn from_str_id(s: &str) -> Option<Self> {
        Self::all().iter().copied().find(|id| id.as_str() == s)
    }
}

/// Typed lifecycle event emitted by the coordinator. `Serialize`/`Deserialize`
/// so the rank-4 board forwarder can `app.emit` it to the React operator board as
/// a typed delta (externally-tagged JSON), and tests can round-trip it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmEvent {
    SessionSpawned {
        instance_id: ModelInstanceId,
        parent_session_id: String,
        process_uuid: Uuid,
        swarm_id: Option<String>,
        worktree_id: Option<String>,
    },
    SessionReady {
        instance_id: ModelInstanceId,
    },
    SessionStateChanged {
        instance_id: ModelInstanceId,
        from: ModelSessionState,
        to: ModelSessionState,
    },
    ModelInvocationStarted {
        instance_id: ModelInstanceId,
        trace_id: Uuid,
        run_id: String,
        session_id: String,
        max_tokens: u32,
    },
    ModelInvocationFinished {
        instance_id: ModelInstanceId,
        trace_id: Uuid,
        run_id: String,
        session_id: String,
        outcome: String,
        generated_tokens: u64,
        error: Option<String>,
    },
    SessionCancelled {
        instance_id: ModelInstanceId,
        reason: String,
    },
    SessionCompleted {
        instance_id: ModelInstanceId,
    },
    SessionFailed {
        instance_id: ModelInstanceId,
        error: String,
    },
    ResourceAllocated {
        instance_id: ModelInstanceId,
        permits_in_use: usize,
        permits_cap: usize,
    },
    ResourceEvicted {
        instance_id: ModelInstanceId,
        terminal_state: ModelSessionState,
    },
    BreakerTripped {
        signature: String,
        consecutive_failures: u32,
    },
    LeaseExpired {
        instance_id: ModelInstanceId,
        owner: String,
    },
    SpawnRejected {
        instance_id: ModelInstanceId,
        reason: String,
    },
}

impl SwarmEvent {
    pub fn fr_event_id(&self) -> SwarmFrEventId {
        match self {
            Self::SessionSpawned { .. } => SwarmFrEventId::SessionSpawned,
            Self::SessionReady { .. } => SwarmFrEventId::SessionReady,
            Self::SessionStateChanged { to, .. } => match to {
                ModelSessionState::Generating => SwarmFrEventId::SessionGenerating,
                ModelSessionState::Ready => SwarmFrEventId::SessionReady,
                ModelSessionState::Completed => SwarmFrEventId::SessionCompleted,
                ModelSessionState::Failed => SwarmFrEventId::SessionFailed,
                ModelSessionState::Cancelled => SwarmFrEventId::SessionCancelled,
                _ => SwarmFrEventId::SessionGenerating,
            },
            Self::ModelInvocationStarted { .. } => SwarmFrEventId::SessionGenerating,
            Self::ModelInvocationFinished { outcome, .. } => match outcome.as_str() {
                "failed" => SwarmFrEventId::SessionFailed,
                "cancelled" | "dropped" => SwarmFrEventId::SessionCancelled,
                _ => SwarmFrEventId::SessionReady,
            },
            Self::SessionCancelled { .. } => SwarmFrEventId::SessionCancelled,
            Self::SessionCompleted { .. } => SwarmFrEventId::SessionCompleted,
            Self::SessionFailed { .. } => SwarmFrEventId::SessionFailed,
            Self::ResourceAllocated { .. } => SwarmFrEventId::ResourceAllocated,
            Self::ResourceEvicted { .. } => SwarmFrEventId::ResourceEvicted,
            Self::BreakerTripped { .. } => SwarmFrEventId::BreakerTripped,
            Self::LeaseExpired { .. } => SwarmFrEventId::LeaseExpired,
            Self::SpawnRejected { .. } => SwarmFrEventId::SpawnRejected,
        }
    }
}

/// Sink the coordinator emits lifecycle events through. Persistence rejection
/// is part of the producer contract: terminal lifecycle producers must observe
/// and propagate a durable-outbox failure instead of logging and continuing as
/// though the terminal record were accepted.
pub trait SwarmEventSink: Send + Sync + 'static {
    fn emit(&self, event: SwarmEvent) -> Result<(), String>;
}

/// Test/diagnostic sink that records every event in order for assertions.
#[derive(Default)]
pub struct RecordingSwarmSink {
    events: Mutex<Vec<SwarmEvent>>,
}

impl RecordingSwarmSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<SwarmEvent> {
        self.events.lock().expect("recording sink poisoned").clone()
    }

    pub fn count_of(&self, id: SwarmFrEventId) -> usize {
        self.events()
            .iter()
            .filter(|e| e.fr_event_id() == id)
            .count()
    }

    pub fn contains(&self, id: SwarmFrEventId) -> bool {
        self.count_of(id) > 0
    }
}

impl SwarmEventSink for RecordingSwarmSink {
    fn emit(&self, event: SwarmEvent) -> Result<(), String> {
        self.events
            .lock()
            .expect("recording sink poisoned")
            .push(event);
        Ok(())
    }
}

/// Production sink: maps swarm events to flight-recorder envelopes. The
/// FR-EVT-SWARM-* id is stamped into `payload.fr_event_id` (see registry note
/// at the top of this file). Uses the generic [`FlightRecorderEventType::System`]
/// carrier type until the dedicated swarm variants are folded into the locked
/// `.GOV/` enum, keeping the event structurally valid today.
pub struct FlightRecorderSwarmSink<F>
where
    F: Fn(FlightRecorderEvent) -> Result<(), String> + Send + Sync + 'static,
{
    trace_id: Uuid,
    emit_fn: F,
}

impl<F> FlightRecorderSwarmSink<F>
where
    F: Fn(FlightRecorderEvent) -> Result<(), String> + Send + Sync + 'static,
{
    pub fn new(trace_id: Uuid, emit_fn: F) -> Self {
        Self { trace_id, emit_fn }
    }

    fn build(&self, event: &SwarmEvent) -> FlightRecorderEvent {
        let fr_id = event.fr_event_id().as_str();
        let trace_id = match event {
            SwarmEvent::ModelInvocationStarted { trace_id, .. }
            | SwarmEvent::ModelInvocationFinished { trace_id, .. } => *trace_id,
            _ => self.trace_id,
        };
        let (payload, model_id) = match event {
            SwarmEvent::SessionSpawned {
                instance_id,
                parent_session_id,
                process_uuid,
                swarm_id,
                worktree_id,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "instance": instance_id.instance,
                    "parent_session_id": parent_session_id,
                    "process_uuid": process_uuid.to_string(),
                    "swarm_id": swarm_id,
                    "worktree_id": worktree_id,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::SessionReady { instance_id }
            | SwarmEvent::SessionCompleted { instance_id } => (
                json!({ "fr_event_id": fr_id, "instance_id": instance_id.to_string() }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::SessionStateChanged {
                instance_id,
                from,
                to,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "from": from.as_str(),
                    "to": to.as_str(),
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::ModelInvocationStarted {
                instance_id,
                trace_id,
                run_id,
                session_id,
                max_tokens,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "invocation_event": "started",
                    "instance_id": instance_id.to_string(),
                    "trace_id": trace_id.to_string(),
                    "run_id": run_id,
                    "session_id": session_id,
                    "max_tokens": max_tokens,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::ModelInvocationFinished {
                instance_id,
                trace_id,
                run_id,
                session_id,
                outcome,
                generated_tokens,
                error,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "invocation_event": "finished",
                    "instance_id": instance_id.to_string(),
                    "trace_id": trace_id.to_string(),
                    "run_id": run_id,
                    "session_id": session_id,
                    "outcome": outcome,
                    "generated_tokens": generated_tokens,
                    "error": error,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::SessionCancelled {
                instance_id,
                reason,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "reason": reason,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::SessionFailed { instance_id, error } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "error": error,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::ResourceAllocated {
                instance_id,
                permits_in_use,
                permits_cap,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "permits_in_use": permits_in_use,
                    "permits_cap": permits_cap,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::ResourceEvicted {
                instance_id,
                terminal_state,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "terminal_state": terminal_state.as_str(),
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::BreakerTripped {
                signature,
                consecutive_failures,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "signature": signature,
                    "consecutive_failures": consecutive_failures,
                }),
                None,
            ),
            SwarmEvent::LeaseExpired { instance_id, owner } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "owner": owner,
                }),
                Some(instance_id.model_id.to_string()),
            ),
            SwarmEvent::SpawnRejected {
                instance_id,
                reason,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "reason": reason,
                }),
                Some(instance_id.model_id.to_string()),
            ),
        };

        let mut fr_event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            trace_id,
            payload,
        );
        if let Some(model_id) = model_id {
            fr_event = fr_event.with_model_id(model_id);
        }
        fr_event
    }
}

impl<F> SwarmEventSink for FlightRecorderSwarmSink<F>
where
    F: Fn(FlightRecorderEvent) -> Result<(), String> + Send + Sync + 'static,
{
    fn emit(&self, event: SwarmEvent) -> Result<(), String> {
        let fr_event = self.build(&event);
        (self.emit_fn)(fr_event)
    }
}

/// rank-3: durable persistence bridge for swarm Flight-Recorder events.
///
/// `SwarmEventSink::emit` (and the `FlightRecorderSwarmSink` closure) is
/// synchronous and fallible, while `FlightRecorder::record_event` is async and
/// fallible. This bridge converts an async persistence rejection into the
/// synchronous producer result after terminal outbox commit acknowledgement:
/// `emit` does a non-blocking `try_send` into a bounded channel, and a spawned
/// drain task records each event into the async recorder (e.g. the DuckDB store).
/// A full channel increments a `dropped` counter so event loss is OBSERVABLE
/// (mirrors the process-ledger overflow counter) rather than silently swallowed.
///
/// Wire it by capturing a clone in the `FlightRecorderSwarmSink` closure:
/// `FlightRecorderSwarmSink::new(trace, move |ev| bridge.emit(ev))`.
#[derive(Clone)]
pub struct DurableSwarmFrBridge {
    tx: tokio::sync::mpsc::Sender<FlightRecorderEvent>,
    terminal_tx: tokio::sync::mpsc::Sender<DurableTerminalCommand>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    accepting: Arc<AtomicBool>,
    terminal_fenced: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    terminal_retry_count: Arc<AtomicU64>,
}

struct DurableTerminalCommand {
    event: FlightRecorderEvent,
    ack: std::sync::mpsc::SyncSender<Result<(), String>>,
}

impl DurableSwarmFrBridge {
    /// Spawn the drain task against `recorder` and return the bridge plus the
    /// drain `JoinHandle` (hold it for the bridge's lifetime; it ends when every
    /// bridge clone is dropped, closing the channel). `capacity` bounds the
    /// in-flight queue; overflow is counted, never blocking the emitter.
    #[cfg(test)]
    pub fn spawn(
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
        capacity: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        Self::spawn_inner(recorder, None, capacity)
    }

    /// Production bridge backed by the migration-0361 PostgreSQL outbox.
    /// Terminal emit returns success only after its outbox transaction commits.
    pub fn spawn_with_postgres_outbox(
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
        pool: sqlx::PgPool,
        capacity: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        Self::spawn_inner(recorder, Some(pool), capacity)
    }

    /// Degraded production mode when PostgreSQL is available but the primary
    /// Flight Recorder is not. Terminal events are still acknowledged only
    /// after an outbox commit and deliberately remain there for a later healthy
    /// startup to deliver; they are never downgraded to stderr-only success.
    pub fn spawn_outbox_only(
        pool: sqlx::PgPool,
        capacity: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        Self::spawn_inner(Arc::new(UnavailableFlightRecorder), Some(pool), capacity)
    }

    fn spawn_inner(
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
        outbox_pool: Option<sqlx::PgPool>,
        capacity: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let capacity = capacity.max(1);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<FlightRecorderEvent>(capacity.max(1));
        let (terminal_tx, mut terminal_rx) =
            tokio::sync::mpsc::channel::<DurableTerminalCommand>(capacity);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let accepting = Arc::new(AtomicBool::new(true));
        let terminal_fenced = Arc::new(AtomicBool::new(false));
        let task_terminal_fenced = Arc::clone(&terminal_fenced);
        let terminal_retry_count = Arc::new(AtomicU64::new(0));
        let task_retry_count = Arc::clone(&terminal_retry_count);
        let task = tokio::spawn(async move {
            let mut retry_tick = tokio::time::interval(Duration::from_millis(100));
            loop {
                tokio::select! {
                    biased;
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            terminal_rx.close();
                            rx.close();
                            while let Some(command) = terminal_rx.recv().await {
                                persist_and_attempt_terminal(
                                    &recorder,
                                    outbox_pool.as_ref(),
                                    capacity,
                                    command,
                                    &task_retry_count,
                                ).await;
                            }
                            while let Some(event) = rx.recv().await {
                                if let Err(error) = recorder.record_event(event).await {
                                    tracing::warn!(
                                        target: "handshake_core::swarm_orchestration",
                                        %error,
                                        "non-terminal swarm Flight Recorder event failed during shutdown drain"
                                    );
                                }
                            }
                            // One bounded recovery attempt is enough at shutdown:
                            // failed rows remain committed for the next startup.
                            if let Some(pool) = outbox_pool.as_ref() {
                                if retry_one_terminal_outbox(pool, &recorder, &task_retry_count).await.is_ok() {
                                    task_terminal_fenced.store(false, Ordering::Release);
                                }
                            }
                            break;
                        }
                    },
                    command = terminal_rx.recv() => if let Some(command) = command {
                        persist_and_attempt_terminal(
                            &recorder,
                            outbox_pool.as_ref(),
                            capacity,
                            command,
                            &task_retry_count,
                        ).await;
                    },
                    event = rx.recv() => match event {
                        Some(event) => {
                            if let Err(error) = recorder.record_event(event).await {
                                tracing::warn!(
                                    target: "handshake_core::swarm_orchestration",
                                    %error,
                                    "non-terminal swarm Flight Recorder event persistence failed"
                                );
                            }
                        }
                        None => {
                            if terminal_rx.is_closed() { break; }
                        }
                    },
                    _ = retry_tick.tick(), if outbox_pool.is_some() => {
                        if let Some(pool) = outbox_pool.as_ref() {
                            if retry_one_terminal_outbox(pool, &recorder, &task_retry_count).await.is_ok() {
                                task_terminal_fenced.store(false, Ordering::Release);
                            }
                        }
                    },
                }
            }
        });
        (
            Self {
                tx,
                terminal_tx,
                shutdown_tx,
                accepting,
                terminal_fenced,
                dropped: Arc::new(AtomicU64::new(0)),
                terminal_retry_count,
            },
            task,
        )
    }

    /// Synchronous emit for the `FlightRecorderSwarmSink` closure. Terminal
    /// success means the PostgreSQL outbox commit was acknowledged; rejection
    /// is returned to the lifecycle producer and fences later terminal writes.
    pub fn emit(&self, event: FlightRecorderEvent) -> Result<(), String> {
        if !self.accepting.load(Ordering::Acquire) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return Err("swarm Flight Recorder bridge is not accepting events".to_string());
        }
        if is_terminal_flight_recorder_event(&event) {
            if self.terminal_fenced.load(Ordering::Acquire) {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                return Err("terminal swarm Flight Recorder producer is fenced".to_string());
            }
            let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel(1);
            let command = DurableTerminalCommand { event, ack: ack_tx };
            if let Err(error) = self.terminal_tx.try_send(command) {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                self.terminal_fenced.store(true, Ordering::Release);
                return Err(format!(
                    "terminal swarm Flight Recorder spool capacity exhausted; producer fenced: {error}"
                ));
            }
            let acknowledgement = if tokio::runtime::Handle::try_current().is_ok_and(|handle| {
                handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread
            }) {
                tokio::task::block_in_place(|| ack_rx.recv_timeout(Duration::from_secs(5)))
            } else {
                ack_rx.recv_timeout(Duration::from_secs(5))
            };
            match acknowledgement {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => {
                    self.terminal_fenced.store(true, Ordering::Release);
                    Err(error)
                }
                Err(error) => {
                    self.terminal_fenced.store(true, Ordering::Release);
                    Err(format!(
                        "terminal swarm Flight Recorder durable acknowledgement failed; producer fenced: {error}"
                    ))
                }
            }
        } else if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            Err("non-terminal swarm Flight Recorder queue is full".to_string())
        } else {
            Ok(())
        }
    }

    /// Stop new acceptance and ask the owner-held drain task to flush every
    /// queued terminal event before it exits. The app must then await the join
    /// handle under its shutdown deadline.
    pub fn begin_shutdown(&self) {
        if self.accepting.swap(false, Ordering::AcqRel) {
            let _ = self.shutdown_tx.send(true);
        }
    }

    /// Number of FR events dropped because the durable queue was full
    /// (observability — a non-zero value means the recorder cannot keep up).
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn terminal_retry_count(&self) -> u64 {
        self.terminal_retry_count.load(Ordering::Relaxed)
    }

    pub fn terminal_is_fenced(&self) -> bool {
        self.terminal_fenced.load(Ordering::Acquire)
    }
}

struct UnavailableFlightRecorder;

#[async_trait::async_trait]
impl crate::flight_recorder::FlightRecorder for UnavailableFlightRecorder {
    async fn record_event(
        &self,
        _event: FlightRecorderEvent,
    ) -> Result<(), crate::flight_recorder::RecorderError> {
        Err(crate::flight_recorder::RecorderError::SinkError(
            "primary Flight Recorder unavailable; retain PostgreSQL outbox row".to_string(),
        ))
    }

    async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: crate::flight_recorder::EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
        Ok(Vec::new())
    }
}

async fn persist_and_attempt_terminal(
    recorder: &Arc<dyn crate::flight_recorder::FlightRecorder>,
    outbox_pool: Option<&sqlx::PgPool>,
    capacity: usize,
    command: DurableTerminalCommand,
    retry_count: &AtomicU64,
) {
    let DurableTerminalCommand { event, ack } = command;
    if let Some(pool) = outbox_pool {
        match persist_terminal_outbox(pool, &event, capacity).await {
            Ok(()) => {
                let _ = ack.send(Ok(()));
                if let Err(error) = deliver_terminal_outbox_event(pool, recorder, &event).await {
                    retry_count.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(
                        target: "handshake_core::swarm_orchestration",
                        %error,
                        event_id = %event.event_id,
                        "terminal swarm event remains in durable PostgreSQL outbox"
                    );
                }
            }
            Err(error) => {
                let _ = ack.send(Err(error));
            }
        }
    } else {
        // Test/legacy seam: the recorder itself is the durable acknowledgement.
        match recorder.record_event(event).await {
            Ok(()) => {
                let _ = ack.send(Ok(()));
            }
            Err(error) => {
                retry_count.fetch_add(1, Ordering::Relaxed);
                let _ = ack.send(Err(format!(
                    "terminal Flight Recorder persistence failed: {error}"
                )));
            }
        }
    }
}

async fn persist_terminal_outbox(
    pool: &sqlx::PgPool,
    event: &FlightRecorderEvent,
    capacity: usize,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended('swarm_terminal_event_outbox', 361))")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    let already_present: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM swarm_terminal_event_outbox WHERE event_id = $1)",
    )
    .bind(event.event_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM swarm_terminal_event_outbox")
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    if !already_present && row_count >= i64::try_from(capacity).unwrap_or(i64::MAX) {
        return Err(format!(
            "terminal swarm PostgreSQL outbox reached bounded capacity {capacity}"
        ));
    }
    let event_json = serde_json::to_value(event).map_err(|error| error.to_string())?;
    sqlx::query(
        r#"
        INSERT INTO swarm_terminal_event_outbox (event_id, event_jsonb)
        VALUES ($1, $2)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(event.event_id)
    .bind(event_json)
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())
}

async fn deliver_terminal_outbox_event(
    pool: &sqlx::PgPool,
    recorder: &Arc<dyn crate::flight_recorder::FlightRecorder>,
    event: &FlightRecorderEvent,
) -> Result<(), String> {
    match recorder.record_event(event.clone()).await {
        Ok(()) => {
            sqlx::query("DELETE FROM swarm_terminal_event_outbox WHERE event_id = $1")
                .bind(event.event_id)
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => {
            sqlx::query(
                r#"
                UPDATE swarm_terminal_event_outbox
                SET attempts = attempts + 1,
                    last_error = $2,
                    last_attempt_at_utc = NOW()
                WHERE event_id = $1
                "#,
            )
            .bind(event.event_id)
            .bind(error.to_string())
            .execute(pool)
            .await
            .map_err(|update_error| update_error.to_string())?;
            Err(error.to_string())
        }
    }
}

async fn retry_one_terminal_outbox(
    pool: &sqlx::PgPool,
    recorder: &Arc<dyn crate::flight_recorder::FlightRecorder>,
    retry_count: &AtomicU64,
) -> Result<(), String> {
    let row = sqlx::query(
        r#"
        SELECT event_jsonb
        FROM swarm_terminal_event_outbox
        ORDER BY created_at_utc, event_id
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    let Some(row) = row else {
        return Ok(());
    };
    let event_json: serde_json::Value = row
        .try_get("event_jsonb")
        .map_err(|error| error.to_string())?;
    let event: FlightRecorderEvent =
        serde_json::from_value(event_json).map_err(|error| error.to_string())?;
    if let Err(error) = deliver_terminal_outbox_event(pool, recorder, &event).await {
        retry_count.fetch_add(1, Ordering::Relaxed);
        return Err(error);
    }
    Ok(())
}

fn is_terminal_flight_recorder_event(event: &FlightRecorderEvent) -> bool {
    event.event_type == FlightRecorderEventType::LlmInference
        || event
            .payload
            .get("invocation_event")
            .and_then(|value| value.as_str())
            == Some("finished")
        || event
            .payload
            .get("fr_event_id")
            .and_then(|value| value.as_str())
            .is_some_and(|id| {
                matches!(
                    id,
                    "FR-EVT-SWARM-SESSION-CANCELLED"
                        | "FR-EVT-SWARM-SESSION-COMPLETED"
                        | "FR-EVT-SWARM-SESSION-FAILED"
                        | "FR-EVT-SWARM-RESOURCE-EVICTED"
                )
            })
}

/// rank-4: live-update broadcast source for the operator board. Implements
/// `SwarmEventSink` by re-publishing each `SwarmEvent` into a `tokio::broadcast`
/// channel; the Tauri layer subscribes and forwards to `app.emit("swarm://event")`
/// so the React board updates IN PLACE (replacing the 1500ms poll). A slow
/// subscriber observes `RecvError::Lagged` and full-snapshot reconciles, which
/// guards against silent board drift (the Vibe-Kanban snapshot+live-deltas shape).
pub struct BroadcastSwarmSink {
    tx: tokio::sync::broadcast::Sender<SwarmEvent>,
}

impl BroadcastSwarmSink {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Subscribe to the live `SwarmEvent` stream (the Tauri forwarder / tests).
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<SwarmEvent> {
        self.tx.subscribe()
    }

    /// Current live subscriber count (0 = no board open; emit is a cheap no-op).
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl SwarmEventSink for BroadcastSwarmSink {
    fn emit(&self, event: SwarmEvent) -> Result<(), String> {
        // `send` errors only when there are no receivers (no board open) — fine.
        // A full ring drops the OLDEST for slow receivers, who see Lagged and
        // reconcile. Never blocks the coordinator.
        let _ = self.tx.send(event);
        Ok(())
    }
}

/// Composes multiple `SwarmEventSink`s so one coordinator drives BOTH durable
/// Flight-Recorder persistence AND the live board broadcast (and any future sink)
/// from a single emit. Each child's emit is infallible per the trait, so one
/// cannot block another.
pub struct FanoutSwarmSink {
    sinks: Vec<std::sync::Arc<dyn SwarmEventSink>>,
}

impl FanoutSwarmSink {
    pub fn new(sinks: Vec<std::sync::Arc<dyn SwarmEventSink>>) -> Self {
        Self { sinks }
    }
}

impl SwarmEventSink for FanoutSwarmSink {
    fn emit(&self, event: SwarmEvent) -> Result<(), String> {
        for sink in &self.sinks {
            sink.emit(event.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swarm_event_ids_round_trip() {
        for id in SwarmFrEventId::all() {
            assert_eq!(SwarmFrEventId::from_str_id(id.as_str()), Some(*id));
        }
    }

    #[test]
    fn swarm_event_ids_are_canonical_kebab() {
        for id in SwarmFrEventId::all() {
            let s = id.as_str();
            assert!(s.starts_with("FR-EVT-SWARM-"), "bad prefix: {s}");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'),
                "non-canonical char in {s}"
            );
        }
    }

    #[test]
    fn swarm_event_ids_include_worktree_and_scheduled_lifecycle() {
        // rank-3: the VM/worktree + calendar-scheduled lifecycle ids are wired
        // into the table (so coordinator/worktree/scheduler code can stamp them).
        let ids: std::collections::HashSet<&str> =
            SwarmFrEventId::all().iter().map(|i| i.as_str()).collect();
        for expected in [
            "FR-EVT-SWARM-WORKTREE-CREATED",
            "FR-EVT-SWARM-WORKTREE-MOUNTED",
            "FR-EVT-SWARM-WORKTREE-RECLAIMED",
            "FR-EVT-SWARM-WORKTREE-CLEANUP-FAILED",
            "FR-EVT-SWARM-SCHED-SPINUP-FIRED",
            "FR-EVT-SWARM-SCHED-TEARDOWN-FIRED",
        ] {
            assert!(
                ids.contains(expected),
                "missing FR-EVT-SWARM id: {expected}"
            );
        }
        // Every canonical string is unique (no two variants collide).
        assert_eq!(
            ids.len(),
            SwarmFrEventId::all().len(),
            "duplicate canonical FR-EVT-SWARM id string"
        );
    }

    #[test]
    fn flight_recorder_sink_produces_valid_events() {
        let captured: std::sync::Arc<Mutex<Vec<FlightRecorderEvent>>> =
            std::sync::Arc::new(Mutex::new(Vec::new()));
        let cap2 = captured.clone();
        let sink = FlightRecorderSwarmSink::new(Uuid::now_v7(), move |e| {
            cap2.lock().unwrap().push(e);
            Ok(())
        });
        let model_id = crate::model_runtime::ModelId::new_v7();
        let iid = ModelInstanceId::new(model_id, 0);
        sink.emit(SwarmEvent::SessionReady { instance_id: iid });
        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        events[0]
            .validate()
            .expect("emitted FR event must validate");
        assert_eq!(
            events[0].payload["fr_event_id"],
            "FR-EVT-SWARM-SESSION-READY"
        );
    }

    #[test]
    fn flight_recorder_spawn_payload_carries_grouping_for_replay_search() {
        let captured: std::sync::Arc<Mutex<Vec<FlightRecorderEvent>>> =
            std::sync::Arc::new(Mutex::new(Vec::new()));
        let cap2 = captured.clone();
        let sink = FlightRecorderSwarmSink::new(Uuid::now_v7(), move |e| {
            cap2.lock().unwrap().push(e);
            Ok(())
        });
        let model_id = crate::model_runtime::ModelId::new_v7();
        let iid = ModelInstanceId::new(model_id, 4);
        sink.emit(SwarmEvent::SessionSpawned {
            instance_id: iid,
            parent_session_id: "parent-1".to_string(),
            process_uuid: Uuid::now_v7(),
            swarm_id: Some("swarm-alpha".to_string()),
            worktree_id: Some("wt-recovery-1".to_string()),
        });

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        events[0]
            .validate()
            .expect("emitted FR event must validate");
        assert_eq!(
            events[0].payload["fr_event_id"],
            "FR-EVT-SWARM-SESSION-SPAWNED"
        );
        assert_eq!(events[0].payload["swarm_id"], "swarm-alpha");
        assert_eq!(events[0].payload["worktree_id"], "wt-recovery-1");
    }

    #[test]
    fn flight_recorder_invocation_uses_call_trace_and_identity_payload() {
        let captured: std::sync::Arc<Mutex<Vec<FlightRecorderEvent>>> =
            std::sync::Arc::new(Mutex::new(Vec::new()));
        let cap2 = captured.clone();
        let sink = FlightRecorderSwarmSink::new(Uuid::now_v7(), move |event| {
            cap2.lock().unwrap().push(event);
            Ok(())
        });
        let invocation_trace = Uuid::now_v7();
        let iid = ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 5);
        sink.emit(SwarmEvent::ModelInvocationFinished {
            instance_id: iid,
            trace_id: invocation_trace,
            run_id: "run-1".to_string(),
            session_id: "session-1".to_string(),
            outcome: "failed".to_string(),
            generated_tokens: 2,
            error: Some("provider failed".to_string()),
        });

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        events[0]
            .validate()
            .expect("invocation FR event must validate");
        assert_eq!(events[0].trace_id, invocation_trace);
        assert_eq!(events[0].payload["trace_id"], invocation_trace.to_string());
        assert_eq!(events[0].payload["run_id"], "run-1");
        assert_eq!(events[0].payload["session_id"], "session-1");
        assert_eq!(events[0].payload["outcome"], "failed");
    }

    /// Deterministic in-process recorder so the bridge test runs in default CI
    /// without the `duckdb-flight-recorder` feature (the real production recorder
    /// is DuckDB; the bridge contract is recorder-agnostic).
    struct CollectingRecorder {
        events: std::sync::Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    #[async_trait::async_trait]
    impl crate::flight_recorder::FlightRecorder for CollectingRecorder {
        async fn record_event(
            &self,
            event: FlightRecorderEvent,
        ) -> Result<(), crate::flight_recorder::RecorderError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
            Ok(self.events.lock().unwrap().clone())
        }
    }

    struct RecoveringRecorder {
        failures_remaining: AtomicU64,
        events: std::sync::Arc<Mutex<Vec<FlightRecorderEvent>>>,
    }

    #[async_trait::async_trait]
    impl crate::flight_recorder::FlightRecorder for RecoveringRecorder {
        async fn record_event(
            &self,
            event: FlightRecorderEvent,
        ) -> Result<(), crate::flight_recorder::RecorderError> {
            if self
                .failures_remaining
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                    if remaining > 0 {
                        Some(remaining - 1)
                    } else {
                        None
                    }
                })
                .is_ok()
            {
                return Err(crate::flight_recorder::RecorderError::SinkError(
                    "injected transient recorder failure".to_string(),
                ));
            }
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, crate::flight_recorder::RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: crate::flight_recorder::EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, crate::flight_recorder::RecorderError> {
            Ok(self.events.lock().unwrap().clone())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn durable_swarm_fr_bridge_records_events_to_recorder() {
        // rank-3: the bridge persists swarm events into the async FlightRecorder
        // from the SYNC sink emit, with an observable dropped counter.
        let collected = std::sync::Arc::new(Mutex::new(Vec::new()));
        let recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder> =
            std::sync::Arc::new(CollectingRecorder {
                events: collected.clone(),
            });
        let (bridge, drain) = DurableSwarmFrBridge::spawn(recorder, 64);

        // Wire the bridge into a swarm sink (the production shape) and emit.
        let sink = {
            let b = bridge.clone();
            FlightRecorderSwarmSink::new(Uuid::now_v7(), move |ev| b.emit(ev))
        };
        let iid = ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 1);
        sink.emit(SwarmEvent::SessionReady { instance_id: iid });
        assert_eq!(bridge.dropped_count(), 0, "no drops on a healthy queue");

        // Close every sender so the drain task finishes, then join it.
        drop(sink);
        drop(bridge);
        let _ = drain.await;

        // The swarm event was durably recorded into the recorder.
        let events = collected.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| e.payload.get("fr_event_id").and_then(|v| v.as_str())
                    == Some("FR-EVT-SWARM-SESSION-READY")),
            "the swarm SessionReady event must be durably recorded; got {} events",
            events.len()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn durable_swarm_fr_bridge_fences_terminal_producer_on_persistence_failure() {
        let collected = std::sync::Arc::new(Mutex::new(Vec::new()));
        let recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder> =
            std::sync::Arc::new(RecoveringRecorder {
                failures_remaining: AtomicU64::new(3),
                events: collected.clone(),
            });
        let (bridge, drain) = DurableSwarmFrBridge::spawn(recorder, 1);
        let sink = {
            let bridge = bridge.clone();
            FlightRecorderSwarmSink::new(Uuid::now_v7(), move |event| bridge.emit(event))
        };
        let iid = ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 1);
        let producer_error = sink
            .emit(SwarmEvent::ModelInvocationFinished {
                instance_id: iid,
                trace_id: Uuid::now_v7(),
                run_id: "run-terminal-fence".to_string(),
                session_id: "session-terminal-fence".to_string(),
                outcome: "failed".to_string(),
                generated_tokens: 0,
                error: Some("injected provider failure".to_string()),
            })
            .expect_err("terminal persistence rejection must reach the producer");
        assert!(producer_error.contains("persistence failed"));
        assert!(bridge.terminal_is_fenced());
        bridge.begin_shutdown();
        drop(sink);
        tokio::time::timeout(Duration::from_secs(5), drain)
            .await
            .expect("terminal bridge drain completes after recorder failure")
            .expect("terminal bridge task joins");

        assert!(bridge.terminal_is_fenced());
        assert_eq!(bridge.terminal_retry_count(), 1);
        assert!(collected.lock().unwrap().is_empty());
    }

    struct TestRecordingSink {
        events: std::sync::Arc<Mutex<Vec<SwarmEvent>>>,
    }

    impl SwarmEventSink for TestRecordingSink {
        fn emit(&self, event: SwarmEvent) -> Result<(), String> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn broadcast_swarm_sink_delivers_to_subscribers() {
        // rank-4: the live board source re-publishes each event to subscribers.
        let sink = BroadcastSwarmSink::new(16);
        let mut rx = sink.subscribe();
        assert_eq!(sink.receiver_count(), 1);
        let iid = ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 1);
        sink.emit(SwarmEvent::SessionReady { instance_id: iid });
        assert_eq!(
            rx.recv().await.expect("subscriber receives the event"),
            SwarmEvent::SessionReady { instance_id: iid }
        );
    }

    #[tokio::test]
    async fn fanout_swarm_sink_emits_to_every_child() {
        // rank-4: one coordinator drives BOTH the live broadcast AND a durable
        // sink from a single emit.
        let broadcast = std::sync::Arc::new(BroadcastSwarmSink::new(16));
        let mut rx = broadcast.subscribe();
        let collected = std::sync::Arc::new(Mutex::new(Vec::new()));
        let recording = std::sync::Arc::new(TestRecordingSink {
            events: collected.clone(),
        });
        let fanout = FanoutSwarmSink::new(vec![broadcast.clone(), recording.clone()]);

        let iid = ModelInstanceId::new(crate::model_runtime::ModelId::new_v7(), 2);
        fanout.emit(SwarmEvent::SessionReady { instance_id: iid });

        assert_eq!(
            rx.recv().await.expect("broadcast child delivered"),
            SwarmEvent::SessionReady { instance_id: iid }
        );
        assert_eq!(
            collected.lock().unwrap().len(),
            1,
            "the recording child also received the event"
        );
    }
}
