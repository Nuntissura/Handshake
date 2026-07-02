use handshake_core::kernel::{
    KernelActor, KernelEventType, KernelTaskRun, NewKernelEvent, SessionRun, SessionRunState,
};
use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn kernel_event_ledger_migration() {
    let db = postgres_or_environment_blocked().await;

    assert!(
        db.migration_version().await.expect("migration version") >= 19,
        "Kernel V1 migrations must include event ledger authority fields and session queue leases"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn kernel_event_ledger_replays_by_aggregate_id_in_sequence_order() {
    let db = postgres_or_environment_blocked().await;

    let suffix = Uuid::now_v7();
    let task_id = format!("KTR-AGGREGATE-{suffix}");
    let session_id = format!("SR-AGGREGATE-{suffix}");
    let aggregate_type = "session_run";
    let aggregate_id = format!("aggregate-session-{suffix}");

    let first = db
        .append_kernel_event(
            NewKernelEvent::builder(
                task_id.clone(),
                session_id.clone(),
                KernelEventType::SessionQueued,
                KernelActor::SessionBroker("broker-aggregate-test".to_string()),
            )
            .aggregate(aggregate_type, aggregate_id.clone())
            .idempotency_key(format!("idem-aggregate-one-{suffix}"))
            .payload(json!({"ordinal": 1}))
            .build()
            .expect("first event"),
        )
        .await
        .expect("append first aggregate event");
    let second = db
        .append_kernel_event(
            NewKernelEvent::builder(
                task_id,
                session_id,
                KernelEventType::SessionClaimed,
                KernelActor::SessionBroker("broker-aggregate-test".to_string()),
            )
            .aggregate(aggregate_type, aggregate_id.clone())
            .idempotency_key(format!("idem-aggregate-two-{suffix}"))
            .causation_id(first.event_id.clone())
            .payload(json!({"ordinal": 2}))
            .build()
            .expect("second event"),
        )
        .await
        .expect("append second aggregate event");

    let events = db
        .list_kernel_events_for_aggregate(aggregate_type, &aggregate_id)
        .await
        .expect("aggregate replay");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id, first.event_id);
    assert_eq!(events[1].event_id, second.event_id);
    assert!(events[0].event_sequence < events[1].event_sequence);
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn kernel_event_ledger_idempotency_rejects_divergent_duplicate() {
    let db = postgres_or_environment_blocked().await;

    let suffix = Uuid::now_v7();
    let task_id = format!("KTR-IDEMPOTENCY-{suffix}");
    let session_id = format!("SR-IDEMPOTENCY-{suffix}");
    let idempotency_key = format!("idem-divergent-{suffix}");

    db.append_kernel_event(
        NewKernelEvent::builder(
            task_id.clone(),
            session_id.clone(),
            KernelEventType::SessionQueued,
            KernelActor::SessionBroker("broker-idempotency-test".to_string()),
        )
        .idempotency_key(idempotency_key.clone())
        .payload(json!({"ordinal": 1}))
        .build()
        .expect("original event"),
    )
    .await
    .expect("append original event");

    let err = db
        .append_kernel_event(
            NewKernelEvent::builder(
                task_id,
                session_id,
                KernelEventType::SessionClaimed,
                KernelActor::SessionBroker("broker-idempotency-test".to_string()),
            )
            .idempotency_key(idempotency_key)
            .payload(json!({"ordinal": 2}))
            .build()
            .expect("divergent duplicate event"),
        )
        .await
        .expect_err("same idempotency key with different semantics must fail");

    assert!(
        err.to_string().contains("idempotency") && err.to_string().contains("conflict"),
        "unexpected divergent idempotency error: {err}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn kernel_event_ledger_contract_metadata_idempotency_and_sequence() {
    let db = postgres_or_environment_blocked().await;

    let suffix = Uuid::now_v7();
    let task_id = format!("KTR-POSTGRES-CONTRACT-{suffix}");
    let session_id = format!("SR-POSTGRES-CONTRACT-{suffix}");
    let idempotency_key = format!("idem-postgres-contract-{suffix}");

    let first = NewKernelEvent::builder(
        task_id.clone(),
        session_id.clone(),
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-postgres-contract-test".to_string()),
    )
    .aggregate("session_run", session_id.clone())
    .idempotency_key(idempotency_key.clone())
    .event_version("kernel_event_v1")
    .source_component("session_broker")
    .correlation_id(format!("corr-postgres-contract-{suffix}"))
    .payload(json!({"queue": "primary", "ordinal": 1}))
    .build()
    .expect("valid first event");

    let stored_first = db
        .append_kernel_event(first.clone())
        .await
        .expect("append first event");
    let duplicate_first = db
        .append_kernel_event(first)
        .await
        .expect("idempotent duplicate append");

    assert_eq!(stored_first.event_id, duplicate_first.event_id);
    assert_eq!(stored_first.event_sequence, duplicate_first.event_sequence);
    assert_eq!(stored_first.aggregate_type, "session_run");
    assert_eq!(stored_first.aggregate_id, session_id);
    assert_eq!(stored_first.idempotency_key, idempotency_key);
    assert_eq!(stored_first.event_version, "kernel_event_v1");
    assert_eq!(stored_first.source_component, "session_broker");
    assert_eq!(stored_first.payload_hash.len(), 64);
    assert!(stored_first
        .payload_hash
        .chars()
        .all(|character| character.is_ascii_hexdigit()));

    let second = NewKernelEvent::builder(
        task_id,
        session_id.clone(),
        KernelEventType::SessionClaimed,
        KernelActor::ModelAdapter("adapter-postgres-contract-test".to_string()),
    )
    .aggregate("session_run", session_id.clone())
    .idempotency_key(format!("idem-postgres-contract-second-{suffix}"))
    .event_version("kernel_event_v1")
    .source_component("model_adapter")
    .causation_id(stored_first.event_id.clone())
    .payload(json!({"claim": {"worker": "contract-test"}, "ordinal": 2}))
    .build()
    .expect("valid second event");
    let stored_second = db
        .append_kernel_event(second)
        .await
        .expect("append second event");

    let events = db
        .list_kernel_events_for_session(&session_id)
        .await
        .expect("list contract events");

    assert_eq!(events.len(), 2, "duplicate idempotency key must not append");
    assert_eq!(events[0].event_id, stored_first.event_id);
    assert_eq!(events[1].event_id, stored_second.event_id);
    assert!(events[0].event_sequence < events[1].event_sequence);
    assert_eq!(events[0].payload_hash, stored_first.payload_hash);
    assert_eq!(
        events[1].causation_id.as_deref(),
        Some(events[0].event_id.as_str())
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn kernel_event_ledger_api_appends_and_lists_kernel_events_for_session() {
    let db = postgres_or_environment_blocked().await;

    let first = NewKernelEvent::builder(
        "KTR-POSTGRES-LEDGER",
        "SR-POSTGRES-LEDGER",
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-postgres-test".to_string()),
    )
    .correlation_id("corr-postgres-ledger")
    .payload(json!({"queue": "primary"}))
    .build()
    .expect("valid first event");

    let stored_first = db
        .append_kernel_event(first)
        .await
        .expect("append first event");

    let second = NewKernelEvent::builder(
        "KTR-POSTGRES-LEDGER",
        "SR-POSTGRES-LEDGER",
        KernelEventType::SessionClaimed,
        KernelActor::ModelAdapter("adapter-postgres-test".to_string()),
    )
    .causation_id(stored_first.event_id.clone())
    .correlation_id("corr-postgres-ledger")
    .payload(json!({"claim": {"lane": "codex"}}))
    .build()
    .expect("valid second event");

    let stored_second = db
        .append_kernel_event(second)
        .await
        .expect("append second event");

    let other_session = NewKernelEvent::builder(
        "KTR-POSTGRES-LEDGER",
        "SR-OTHER",
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-postgres-test".to_string()),
    )
    .payload(json!({}))
    .build()
    .expect("valid other-session event");
    db.append_kernel_event(other_session)
        .await
        .expect("append other-session event");

    let events = db
        .list_kernel_events_for_session("SR-POSTGRES-LEDGER")
        .await
        .expect("list events for session");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id, stored_first.event_id);
    assert_eq!(events[1].event_id, stored_second.event_id);
    assert_eq!(events[0].event_type, KernelEventType::SessionQueued);
    assert_eq!(events[1].event_type, KernelEventType::SessionClaimed);
    assert_eq!(
        events[0].actor,
        KernelActor::SessionBroker("broker-postgres-test".to_string())
    );
    assert_eq!(
        events[1].actor,
        KernelActor::ModelAdapter("adapter-postgres-test".to_string())
    );
    assert_eq!(
        events[1].causation_id.as_deref(),
        Some(events[0].event_id.as_str())
    );
    assert_eq!(
        events[0].correlation_id.as_deref(),
        Some("corr-postgres-ledger")
    );
    assert_eq!(events[1].payload["claim"]["lane"], "codex");
    assert!(events[0].created_at <= events[1].created_at);
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn durable_claim_and_lease() {
    let db = postgres_or_environment_blocked().await;

    let task = KernelTaskRun::new("claim-lease-test", json!({"intent": "claim once"}));
    let session = SessionRun::queued(&task.kernel_task_run_id, "dummy-echo-claim");
    let queued = db
        .enqueue_kernel_session_run(session.clone())
        .await
        .expect("enqueue kernel session run");
    assert_eq!(queued.state, SessionRunState::Queued);

    let first_claim = db
        .claim_kernel_session_run(&session.session_run_id, "worker-a", 1)
        .await
        .expect("first claim")
        .expect("first worker should claim queued session");
    assert_eq!(first_claim.state, SessionRunState::Claimed);
    assert_eq!(first_claim.claimed_by.as_deref(), Some("worker-a"));
    assert_eq!(first_claim.attempt_count, 1);
    assert!(first_claim.lease_expires_at.is_some());

    let blocked_claim = db
        .claim_kernel_session_run(&session.session_run_id, "worker-b", 30)
        .await
        .expect("second claim should be a clean miss");
    assert!(
        blocked_claim.is_none(),
        "an unexpired lease must block duplicate claims"
    );

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let reclaimed = db
        .claim_kernel_session_run(&session.session_run_id, "worker-b", 30)
        .await
        .expect("reclaim after lease expiry")
        .expect("expired lease should be reclaimable");
    assert_eq!(reclaimed.claimed_by.as_deref(), Some("worker-b"));
    assert_eq!(reclaimed.attempt_count, 2);

    let running = db
        .update_kernel_session_run_state(&session.session_run_id, SessionRunState::Running)
        .await
        .expect("transition claimed session to running");
    assert_eq!(running.state, SessionRunState::Running);
    assert_eq!(running.claimed_by.as_deref(), Some("worker-b"));

    let completed = db
        .update_kernel_session_run_state(&session.session_run_id, SessionRunState::Completed)
        .await
        .expect("transition running session to completed");
    assert_eq!(completed.state, SessionRunState::Completed);
    assert!(completed.claimed_by.is_none());
    assert!(completed.lease_expires_at.is_none());

    let invalid = db
        .update_kernel_session_run_state(&session.session_run_id, SessionRunState::Running)
        .await
        .expect_err("completed sessions must not move back to running");
    assert!(
        invalid
            .to_string()
            .contains("invalid kernel session transition"),
        "unexpected invalid-transition error: {invalid}"
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn durable_claim_matches_retry_backpressure_and_deadletter_state_table() {
    let db = postgres_or_environment_blocked().await;

    let retry_task = KernelTaskRun::new("retry-claim-test", json!({"intent": "retry claim"}));
    let retry_session = SessionRun::queued(&retry_task.kernel_task_run_id, "dummy-echo-retry");
    db.enqueue_kernel_session_run(retry_session.clone())
        .await
        .expect("enqueue retry session");
    db.claim_kernel_session_run(&retry_session.session_run_id, "worker-a", 30)
        .await
        .expect("claim retry session")
        .expect("retry session claim");
    db.update_kernel_session_run_state(&retry_session.session_run_id, SessionRunState::Running)
        .await
        .expect("retry session running");
    db.update_kernel_session_run_state(&retry_session.session_run_id, SessionRunState::Failed)
        .await
        .expect("retry session failed");
    db.update_kernel_session_run_state(
        &retry_session.session_run_id,
        SessionRunState::RetryScheduled,
    )
    .await
    .expect("retry scheduled");
    let retried = db
        .claim_kernel_session_run(&retry_session.session_run_id, "worker-b", 30)
        .await
        .expect("claim retry scheduled")
        .expect("retry scheduled session should be directly claimable");
    assert_eq!(retried.state, SessionRunState::Claimed);
    assert_eq!(retried.claimed_by.as_deref(), Some("worker-b"));
    assert_eq!(retried.attempt_count, 2);

    let backpressure_task =
        KernelTaskRun::new("backpressure-test", json!({"intent": "backpressure"}));
    let backpressure_session = SessionRun::queued(
        &backpressure_task.kernel_task_run_id,
        "dummy-echo-backpressure",
    );
    db.enqueue_kernel_session_run(backpressure_session.clone())
        .await
        .expect("enqueue backpressure session");
    db.update_kernel_session_run_state(
        &backpressure_session.session_run_id,
        SessionRunState::BackpressureDelayed,
    )
    .await
    .expect("backpressure delay");
    let blocked = db
        .claim_kernel_session_run(&backpressure_session.session_run_id, "worker-c", 30)
        .await
        .expect("backpressure claim miss");
    assert!(
        blocked.is_none(),
        "backpressure-delayed work must not claim directly"
    );
    db.update_kernel_session_run_state(
        &backpressure_session.session_run_id,
        SessionRunState::Queued,
    )
    .await
    .expect("release backpressure to queue");
    let released = db
        .claim_kernel_session_run(&backpressure_session.session_run_id, "worker-c", 30)
        .await
        .expect("claim released backpressure")
        .expect("released backpressure session should claim");
    assert_eq!(released.state, SessionRunState::Claimed);

    let dead_task = KernelTaskRun::new("deadletter-test", json!({"intent": "deadletter"}));
    let dead_session = SessionRun::queued(&dead_task.kernel_task_run_id, "dummy-echo-deadletter");
    db.enqueue_kernel_session_run(dead_session.clone())
        .await
        .expect("enqueue deadletter session");
    db.claim_kernel_session_run(&dead_session.session_run_id, "worker-d", 30)
        .await
        .expect("claim deadletter session")
        .expect("deadletter session claim");
    db.update_kernel_session_run_state(&dead_session.session_run_id, SessionRunState::Running)
        .await
        .expect("deadletter session running");
    db.update_kernel_session_run_state(&dead_session.session_run_id, SessionRunState::Failed)
        .await
        .expect("deadletter session failed");
    db.update_kernel_session_run_state(&dead_session.session_run_id, SessionRunState::DeadLettered)
        .await
        .expect("deadletter terminal state");
    let dead_claim = db
        .claim_kernel_session_run(&dead_session.session_run_id, "worker-e", 30)
        .await
        .expect("deadletter claim miss");
    assert!(dead_claim.is_none(), "dead-lettered work must be terminal");
}
