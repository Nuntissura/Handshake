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

use std::sync::Mutex;

use serde_json::json;

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use uuid::Uuid;

use super::breaker::FailureFingerprint;
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

/// Typed lifecycle event emitted by the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwarmEvent {
    SessionSpawned {
        instance_id: ModelInstanceId,
        parent_session_id: String,
        process_uuid: Uuid,
    },
    SessionReady {
        instance_id: ModelInstanceId,
    },
    SessionStateChanged {
        instance_id: ModelInstanceId,
        from: ModelSessionState,
        to: ModelSessionState,
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

/// Sink the coordinator emits lifecycle events through. Synchronous + infallible
/// at the trait boundary on purpose: emitting telemetry must never be able to
/// stall or fail a teardown path. Implementations that can fail (a DB sink)
/// must absorb their own errors and surface them out-of-band.
pub trait SwarmEventSink: Send + Sync + 'static {
    fn emit(&self, event: SwarmEvent);
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
    fn emit(&self, event: SwarmEvent) {
        self.events
            .lock()
            .expect("recording sink poisoned")
            .push(event);
    }
}

/// Production sink: maps swarm events to flight-recorder envelopes. The
/// FR-EVT-SWARM-* id is stamped into `payload.fr_event_id` (see registry note
/// at the top of this file). Uses the generic [`FlightRecorderEventType::System`]
/// carrier type until the dedicated swarm variants are folded into the locked
/// `.GOV/` enum, keeping the event structurally valid today.
pub struct FlightRecorderSwarmSink<F>
where
    F: Fn(FlightRecorderEvent) + Send + Sync + 'static,
{
    trace_id: Uuid,
    emit_fn: F,
}

impl<F> FlightRecorderSwarmSink<F>
where
    F: Fn(FlightRecorderEvent) + Send + Sync + 'static,
{
    pub fn new(trace_id: Uuid, emit_fn: F) -> Self {
        Self { trace_id, emit_fn }
    }

    fn build(&self, event: &SwarmEvent) -> FlightRecorderEvent {
        let fr_id = event.fr_event_id().as_str();
        let (payload, model_id) = match event {
            SwarmEvent::SessionSpawned {
                instance_id,
                parent_session_id,
                process_uuid,
            } => (
                json!({
                    "fr_event_id": fr_id,
                    "instance_id": instance_id.to_string(),
                    "instance": instance_id.instance,
                    "parent_session_id": parent_session_id,
                    "process_uuid": process_uuid.to_string(),
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
            self.trace_id,
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
    F: Fn(FlightRecorderEvent) + Send + Sync + 'static,
{
    fn emit(&self, event: SwarmEvent) {
        let fr_event = self.build(&event);
        (self.emit_fn)(fr_event);
    }
}

/// rank-3: durable persistence bridge for swarm Flight-Recorder events.
///
/// `SwarmEventSink::emit` (and the `FlightRecorderSwarmSink` closure) is
/// SYNCHRONOUS and contractually infallible, but `FlightRecorder::record_event`
/// is ASYNC and fallible. This bridges the two without blocking the coordinator:
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
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl DurableSwarmFrBridge {
    /// Spawn the drain task against `recorder` and return the bridge plus the
    /// drain `JoinHandle` (hold it for the bridge's lifetime; it ends when every
    /// bridge clone is dropped, closing the channel). `capacity` bounds the
    /// in-flight queue; overflow is counted, never blocking the emitter.
    pub fn spawn(
        recorder: std::sync::Arc<dyn crate::flight_recorder::FlightRecorder>,
        capacity: usize,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<FlightRecorderEvent>(capacity.max(1));
        let task = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // record_event errors are absorbed out-of-band: the sink trait is
                // infallible, so a recorder hiccup must not propagate or block.
                let _ = recorder.record_event(event).await;
            }
        });
        (
            Self {
                tx,
                dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            task,
        )
    }

    /// Sync, infallible emit for the `FlightRecorderSwarmSink` closure: enqueue
    /// the event for durable recording; on a full queue increment `dropped`.
    pub fn emit(&self, event: FlightRecorderEvent) {
        if self.tx.try_send(event).is_err() {
            self.dropped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Number of FR events dropped because the durable queue was full
    /// (observability — a non-zero value means the recorder cannot keep up).
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
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
            assert!(ids.contains(expected), "missing FR-EVT-SWARM id: {expected}");
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
        });
        let model_id = crate::model_runtime::ModelId::new_v7();
        let iid = ModelInstanceId::new(model_id, 0);
        sink.emit(SwarmEvent::SessionReady { instance_id: iid });
        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        events[0].validate().expect("emitted FR event must validate");
        assert_eq!(
            events[0].payload["fr_event_id"],
            "FR-EVT-SWARM-SESSION-READY"
        );
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

    #[tokio::test]
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
            events.iter().any(|e| e
                .payload
                .get("fr_event_id")
                .and_then(|v| v.as_str())
                == Some("FR-EVT-SWARM-SESSION-READY")),
            "the swarm SessionReady event must be durably recorded; got {} events",
            events.len()
        );
    }
}
