use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::{
    event_ledger::EventLedgerFlightRecorderMirror, EventFilter, FlightRecorder,
    FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::Database;
use handshake_core::terminal::runtime::{SessionBinding, TerminalRuntime};

async fn reopen_embedded_store(
    backend: &EmbeddedTestBackend,
) -> (SurrealStorage, Arc<dyn Database>) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded terminal EventLedger store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded terminal EventLedger store"),
    )
    .await
    .expect("reopen embedded terminal EventLedger store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened terminal EventLedger schema");
    let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(reopened.clone()));
    (reopened, database)
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
async fn terminal_capture_session_receipts_land_in_surreal_event_ledger() {
    let backend = embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend");
    let db = backend.database.clone();
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

    drop(sink);
    drop(runtime);
    drop(recorder);
    drop(db);
    let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
    let events = reopened_db
        .list_kernel_events_for_aggregate("terminal_session", &info.session_id)
        .await
        .expect("replay terminal EventLedger receipts after reopen");

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
    drop(reopened_db);
    reopened
        .shutdown()
        .await
        .expect("close reopened terminal EventLedger store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("embedded SurrealDB storage cleanup");
}
