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
    duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorder, FlightRecorderActor,
    FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use handshake_core::swarm_orchestration::DurableSwarmFrBridge;
use sqlx::PgPool;
use uuid::Uuid;

struct GatedRecorder {
    allow_success: AtomicBool,
    recorded: Mutex<Vec<Uuid>>,
}

struct GatedDuckDbRecorder {
    allow_success: AtomicBool,
    inner: DuckDbFlightRecorder,
}

#[async_trait]
impl FlightRecorder for GatedDuckDbRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        if !self.allow_success.load(Ordering::Acquire) {
            return Err(RecorderError::SinkError(
                "injected recorder outage".to_string(),
            ));
        }
        self.inner.record_event(event).await
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

fn terminal_llm_event(trace_id: Uuid, outcome: &str) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::System,
        trace_id,
        serde_json::json!({
            "type": "llm_inference",
            "trace_id": trace_id,
            "model_id": "outbox-proof-model",
            "token_usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2
            },
            "invocation_event": "finished",
            "outcome": outcome
        }),
    )
    .with_model_id("outbox-proof-model")
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delivered_event_reemit_is_idempotent_and_does_not_starve_later_outbox_event() {
    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("idempotent terminal outbox proof requires real PostgreSQL");
    let pool = PgPool::connect(&pg.schema_url)
        .await
        .expect("connect idempotent terminal outbox proof pool");
    let recorder = Arc::new(GatedDuckDbRecorder {
        allow_success: AtomicBool::new(true),
        inner: DuckDbFlightRecorder::new_in_memory(7).expect("create real DuckDB flight recorder"),
    });
    let recorder_trait: Arc<dyn FlightRecorder> = recorder.clone();
    let (bridge, drain) =
        DurableSwarmFrBridge::spawn_with_postgres_outbox(recorder_trait, pool.clone(), 4);

    let trace_id = Uuid::now_v7();
    let first_event = terminal_llm_event(trace_id, "failed");
    let first_event_id = first_event.event_id;
    let reemitted_event = first_event.clone();
    let first_delivery = bridge.clone();
    tokio::task::spawn_blocking(move || first_delivery.emit(first_event))
        .await
        .expect("join first terminal event producer")
        .expect("first terminal event is committed and delivered");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let pending: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM swarm_terminal_event_outbox WHERE event_id = $1)",
            )
            .bind(first_event_id)
            .fetch_one(&pool)
            .await
            .expect("poll first terminal outbox delivery");
            if !pending {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("first terminal event is delivered and deleted from the outbox");

    recorder.allow_success.store(false, Ordering::Release);
    let duplicate_producer = bridge.clone();
    tokio::task::spawn_blocking(move || duplicate_producer.emit(reemitted_event))
        .await
        .expect("join duplicate terminal event producer")
        .expect("duplicate terminal event is committed during recorder outage");

    let later_event = terminal_llm_event(trace_id, "completed");
    let later_event_id = later_event.event_id;
    let later_producer = bridge.clone();
    tokio::task::spawn_blocking(move || later_producer.emit(later_event))
        .await
        .expect("join later terminal event producer")
        .expect("later terminal event is committed during recorder outage");

    let queued_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM swarm_terminal_event_outbox WHERE event_id = ANY($1)",
    )
    .bind(vec![first_event_id, later_event_id])
    .fetch_one(&pool)
    .await
    .expect("read queued duplicate and later terminal events");
    assert_eq!(
        queued_count, 2,
        "both events must be queued before recorder recovery exercises ordered retry"
    );

    recorder.allow_success.store(true, Ordering::Release);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let pending: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM swarm_terminal_event_outbox WHERE event_id = ANY($1)",
            )
            .bind(vec![first_event_id, later_event_id])
            .fetch_one(&pool)
            .await
            .expect("poll duplicate and later terminal outbox recovery");
            if pending == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("idempotent duplicate is deleted and later event is not head-of-line starved");

    let stored = recorder
        .list_events(EventFilter {
            trace_id: Some(trace_id),
            ..EventFilter::default()
        })
        .await
        .expect("list recovered DuckDB events");
    assert_eq!(
        stored
            .iter()
            .filter(|event| event.event_id == first_event_id)
            .count(),
        1,
        "re-emission must not duplicate the first recorded event"
    );
    assert!(
        stored.iter().any(|event| event.event_id == later_event_id),
        "the later event must be delivered after the duplicate retry"
    );

    bridge.begin_shutdown();
    tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .expect("idempotent terminal outbox drain stops")
        .expect("idempotent terminal outbox drain joins");
}
