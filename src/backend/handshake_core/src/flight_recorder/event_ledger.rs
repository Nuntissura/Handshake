use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use duckdb::Connection as DuckDbConnection;
use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{Database, StorageResult};

use super::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};

const TERMINAL_AGGREGATE_TYPE: &str = "terminal_session";
const TERMINAL_MIRROR_SOURCE: &str = "terminal_event_ledger_mirror";

/// Mirrors canonical terminal Flight Recorder events into the kernel
/// EventLedger while delegating the normal FlightRecorder API to an inner
/// recorder.
///
/// MT-252 needs real-Postgres session receipts for the integrated terminal.
/// The terminal runtime already emits canonical `terminal_command` FR events,
/// so this adapter preserves the existing FR sink and adds durable kernel
/// ledger receipts keyed by terminal session id.
pub struct EventLedgerFlightRecorderMirror {
    inner: Arc<dyn FlightRecorder>,
    event_sink: Arc<dyn KernelEventSink>,
}

impl EventLedgerFlightRecorderMirror {
    pub fn new(inner: Arc<dyn FlightRecorder>, database: Arc<dyn Database>) -> Self {
        Self::new_with_event_sink(inner, Arc::new(DatabaseKernelEventSink { database }))
    }

    fn new_with_event_sink(
        inner: Arc<dyn FlightRecorder>,
        event_sink: Arc<dyn KernelEventSink>,
    ) -> Self {
        Self { inner, event_sink }
    }

    async fn mirror_terminal_event(
        &self,
        event: &FlightRecorderEvent,
    ) -> Result<(), RecorderError> {
        if event.event_type != FlightRecorderEventType::TerminalCommand {
            return Ok(());
        }

        let session_id = terminal_session_id(event).ok_or_else(|| {
            RecorderError::InvalidEvent(
                "terminal EventLedger mirror requires session_id".to_string(),
            )
        })?;
        let kernel_task_run_id = terminal_task_run_id(event, &session_id);
        let fr_event = payload_string(&event.payload, "fr_event")
            .unwrap_or_else(|| event.event_type.to_string());
        let payload = json!({
            "event_type": KernelEventType::FlightRecorderMirrorRecorded.as_str(),
            "receipt_kind": "terminal_flight_recorder_mirror",
            "fr_event": fr_event,
            "fr_event_id": event.event_id.to_string(),
            "fr_event_type": event.event_type.to_string(),
            "trace_id": event.trace_id.to_string(),
            "terminal_session_id": session_id,
            "actor_kind": event.actor.to_string(),
            "actor_id": event.actor_id,
            "source_component": TERMINAL_MIRROR_SOURCE,
            "payload": event.payload,
        });

        let kernel_event = NewKernelEvent::builder(
            kernel_task_run_id,
            session_id.clone(),
            KernelEventType::FlightRecorderMirrorRecorded,
            kernel_actor(event),
        )
        .aggregate(TERMINAL_AGGREGATE_TYPE, session_id)
        .idempotency_key(format!("terminal-fr-mirror:{}", event.event_id))
        .source_component(TERMINAL_MIRROR_SOURCE)
        .correlation_id(event.trace_id.to_string())
        .payload(payload)
        .build()
        .map_err(|err| RecorderError::InvalidEvent(err.to_string()))?;

        self.event_sink
            .append_kernel_event(kernel_event)
            .await
            .map_err(|err| {
                RecorderError::SinkError(format!(
                    "terminal EventLedger mirror append failed: {err}"
                ))
            })?;

        Ok(())
    }
}

#[async_trait]
trait KernelEventSink: Send + Sync {
    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent>;
}

struct DatabaseKernelEventSink {
    database: Arc<dyn Database>,
}

#[async_trait]
impl KernelEventSink for DatabaseKernelEventSink {
    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent> {
        self.database.append_kernel_event(event).await
    }
}

#[async_trait]
impl FlightRecorder for EventLedgerFlightRecorderMirror {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.inner.record_event(event.clone()).await?;
        self.mirror_terminal_event(&event).await
    }

    fn duckdb_connection(&self) -> Option<Arc<Mutex<DuckDbConnection>>> {
        self.inner.duckdb_connection()
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        self.inner.enforce_retention().await
    }

    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        self.inner.list_events(filter).await
    }

    async fn list_session_scoped_events(
        &self,
        session_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        self.inner
            .list_session_scoped_events(session_id, from, to)
            .await
    }
}

fn terminal_session_id(event: &FlightRecorderEvent) -> Option<String> {
    event
        .session_span_id
        .clone()
        .or_else(|| payload_string(&event.payload, "session_id"))
        .filter(|value| !value.trim().is_empty())
}

fn terminal_task_run_id(event: &FlightRecorderEvent, session_id: &str) -> String {
    event
        .job_id
        .clone()
        .or_else(|| payload_string(&event.payload, "job_id"))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("KTR-TERMINAL-{session_id}"))
}

fn payload_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn kernel_actor(event: &FlightRecorderEvent) -> KernelActor {
    match event.actor {
        FlightRecorderActor::Human => KernelActor::Operator(event.actor_id.clone()),
        FlightRecorderActor::Agent | FlightRecorderActor::System => {
            KernelActor::System(event.actor_id.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::KernelEvent;
    use crate::storage::{StorageError, StorageResult};
    use uuid::Uuid;

    #[derive(Default)]
    struct RecordingFlightRecorder {
        events: Mutex<Vec<FlightRecorderEvent>>,
    }

    impl RecordingFlightRecorder {
        fn events(&self) -> Vec<FlightRecorderEvent> {
            self.events.lock().expect("recorder events lock").clone()
        }
    }

    #[async_trait]
    impl FlightRecorder for RecordingFlightRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            event.validate()?;
            self.events
                .lock()
                .expect("recorder events lock")
                .push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self.events())
        }
    }

    #[derive(Default)]
    struct RecordingKernelEventSink {
        events: Mutex<Vec<NewKernelEvent>>,
    }

    impl RecordingKernelEventSink {
        fn events(&self) -> Vec<NewKernelEvent> {
            self.events.lock().expect("kernel events lock").clone()
        }
    }

    #[async_trait]
    impl KernelEventSink for RecordingKernelEventSink {
        async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent> {
            self.events
                .lock()
                .expect("kernel events lock")
                .push(event.clone());
            Ok(KernelEvent::from_new(event))
        }
    }

    struct FailingKernelEventSink;

    #[async_trait]
    impl KernelEventSink for FailingKernelEventSink {
        async fn append_kernel_event(&self, _event: NewKernelEvent) -> StorageResult<KernelEvent> {
            Err(StorageError::Database("forced append failure".to_string()))
        }
    }

    fn terminal_command_event() -> FlightRecorderEvent {
        FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            json!({
                "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
                "session_id": "terminal-session-1",
                "command": "cargo test terminal",
                "cwd": ".",
                "exit_code": 0,
                "duration_ms": 42,
                "timed_out": false,
                "cancelled": false,
                "truncated_bytes": 0
            }),
        )
        .with_actor_id("agent-terminal-1")
        .with_job_id("terminal-job-1")
        .with_session_span("terminal-session-1")
    }

    #[tokio::test]
    async fn terminal_events_are_mirrored_to_event_ledger_sink() {
        let inner = Arc::new(RecordingFlightRecorder::default());
        let sink = Arc::new(RecordingKernelEventSink::default());
        let recorder =
            EventLedgerFlightRecorderMirror::new_with_event_sink(inner.clone(), sink.clone());
        let event = terminal_command_event();
        let trace_id = event.trace_id;

        recorder
            .record_event(event.clone())
            .await
            .expect("terminal event should mirror");

        let recorded_events = inner.events();
        assert_eq!(recorded_events.len(), 1);
        assert_eq!(recorded_events[0].event_id, event.event_id);
        assert_eq!(
            recorded_events[0].event_type,
            FlightRecorderEventType::TerminalCommand
        );
        assert_eq!(
            recorded_events[0].payload["session_id"],
            "terminal-session-1"
        );
        let events = sink.events();
        assert_eq!(events.len(), 1);
        let mirrored = &events[0];
        assert_eq!(mirrored.kernel_task_run_id, "terminal-job-1");
        assert_eq!(mirrored.session_run_id, "terminal-session-1");
        assert_eq!(mirrored.aggregate_type, TERMINAL_AGGREGATE_TYPE);
        assert_eq!(mirrored.aggregate_id, "terminal-session-1");
        assert_eq!(
            mirrored.event_type,
            KernelEventType::FlightRecorderMirrorRecorded
        );
        assert_eq!(mirrored.source_component, TERMINAL_MIRROR_SOURCE);
        assert_eq!(
            mirrored.idempotency_key,
            format!("terminal-fr-mirror:{}", event.event_id)
        );
        assert_eq!(mirrored.correlation_id, Some(trace_id.to_string()));
        assert_eq!(
            mirrored.payload["receipt_kind"],
            "terminal_flight_recorder_mirror"
        );
        assert_eq!(mirrored.payload["fr_event"], "FR-EVT-TERMINAL-COMMAND-EXEC");
        assert_eq!(
            mirrored.payload["terminal_session_id"],
            "terminal-session-1"
        );
        assert_eq!(
            mirrored.payload["payload"]["command"],
            "cargo test terminal"
        );
    }

    #[tokio::test]
    async fn terminal_event_ledger_sink_errors_are_not_masked() {
        let inner = Arc::new(RecordingFlightRecorder::default());
        let recorder = EventLedgerFlightRecorderMirror::new_with_event_sink(
            inner.clone(),
            Arc::new(FailingKernelEventSink),
        );

        let error = recorder
            .record_event(terminal_command_event())
            .await
            .expect_err("EventLedger append failure must propagate");

        match error {
            RecorderError::SinkError(message) => {
                assert!(message.contains("terminal EventLedger mirror append failed"));
                assert!(message.contains("forced append failure"));
            }
            other => panic!("expected RecorderError::SinkError, got {other:?}"),
        }
        assert_eq!(
            inner.events().len(),
            1,
            "inner Flight Recorder write should happen before mirror append failure"
        );
    }
}
