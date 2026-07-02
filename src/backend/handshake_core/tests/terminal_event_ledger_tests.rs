use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::{
    event_ledger::EventLedgerFlightRecorderMirror, EventFilter, FlightRecorder,
    FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};
use handshake_core::terminal::runtime::{SessionBinding, TerminalRuntime};

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

struct NoopFlightRecorder;

#[async_trait]
impl FlightRecorder for NoopFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test --test terminal_event_ledger_tests -- --ignored`"]
async fn terminal_capture_session_receipts_land_in_postgres_event_ledger() {
    let db = postgres_or_environment_blocked().await;
    let recorder: Arc<dyn FlightRecorder> = Arc::new(EventLedgerFlightRecorderMirror::new(
        Arc::new(NoopFlightRecorder),
        db.clone(),
    ));
    let runtime = TerminalRuntime::new(Arc::new(CapabilityRegistry::new()), recorder);

    let binding = SessionBinding {
        swarm_id: Some("swarm-terminal-ledger".to_string()),
        worktree_id: Some("wt-terminal-ledger".to_string()),
        instance_id: Some("agent-terminal-ledger".to_string()),
    };
    let (info, sink) = runtime
        .create_capture_session(binding, Some("terminal ledger receipt".to_string()))
        .await;

    sink.feed(b"terminal-ledger-proof\n").await;
    sink.close(0).await;

    let events = db
        .list_kernel_events_for_aggregate("terminal_session", &info.session_id)
        .await
        .expect("replay terminal EventLedger receipts");

    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|event| event.event_type == KernelEventType::FlightRecorderMirrorRecorded));
    assert!(events
        .iter()
        .all(|event| event.aggregate_type == "terminal_session"));
    assert!(events
        .iter()
        .all(|event| event.aggregate_id == info.session_id));
    assert!(events
        .iter()
        .all(|event| event.source_component == "terminal_event_ledger_mirror"));

    let fr_events = events
        .iter()
        .map(|event| {
            event
                .payload
                .get("fr_event")
                .and_then(|value| value.as_str())
                .expect("fr_event payload")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fr_events,
        vec![
            "FR-EVT-TERMINAL-SESSION-OPEN",
            "FR-EVT-TERMINAL-COMMAND-EXEC",
            "FR-EVT-TERMINAL-SESSION-CLOSE"
        ]
    );
    assert_eq!(events[1].payload["payload"]["command"], "<captured-output>");
    assert_eq!(events[1].payload["terminal_session_id"], info.session_id);
}
