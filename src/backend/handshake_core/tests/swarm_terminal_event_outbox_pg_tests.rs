//! Real-PostgreSQL proof for the migration-0361 terminal swarm event outbox.

#[allow(dead_code)]
mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use async_trait::async_trait;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};
use handshake_core::swarm_orchestration::DurableSwarmFrBridge;
use sqlx::PgPool;
use uuid::Uuid;

struct GatedRecorder {
    allow_success: AtomicBool,
    recorded: Mutex<Vec<Uuid>>,
}

#[async_trait]
impl FlightRecorder for GatedRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        if !self.allow_success.load(Ordering::Acquire) {
            return Err(RecorderError::SinkError(
                "injected recorder outage".to_string(),
            ));
        }
        self.recorded
            .lock()
            .expect("recorded events poisoned")
            .push(event.event_id);
        Ok(())
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_event_is_committed_before_ack_and_recovered_after_recorder_failure() {
    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("terminal outbox proof requires real PostgreSQL");
    let pool = PgPool::connect(&pg.schema_url)
        .await
        .expect("connect terminal outbox proof pool");
    let recorder = Arc::new(GatedRecorder {
        allow_success: AtomicBool::new(false),
        recorded: Mutex::new(Vec::new()),
    });
    let recorder_trait: Arc<dyn FlightRecorder> = recorder.clone();
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_postgres_outbox(recorder_trait, pool.clone(), 2);
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        serde_json::json!({"invocation_event": "finished", "outcome": "failed"}),
    );
    let event_id = event.event_id;
    let producer = bridge.clone();
    tokio::task::spawn_blocking(move || producer.emit(event))
        .await
        .expect("join terminal producer")
        .expect("terminal producer acknowledges committed outbox row");

    let durable_before_recorder_recovery: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM swarm_terminal_event_outbox WHERE event_id = $1 AND attempts >= 1)",
    )
    .bind(event_id)
    .fetch_one(&pool)
    .await
    .expect("read committed terminal outbox row");
    assert!(
        durable_before_recorder_recovery,
        "producer acknowledgement must leave a committed recoverable row while recorder delivery fails"
    );

    recorder.allow_success.store(true, Ordering::Release);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let pending: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM swarm_terminal_event_outbox WHERE event_id = $1)",
            )
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .expect("poll terminal outbox recovery");
            if !pending {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("terminal outbox row is retried and deleted after recorder recovery");
    assert_eq!(
        recorder
            .recorded
            .lock()
            .expect("recorded events poisoned")
            .as_slice(),
        &[event_id]
    );

    bridge.begin_shutdown();
    tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .expect("terminal outbox drain stops")
        .expect("terminal outbox drain joins");
}
