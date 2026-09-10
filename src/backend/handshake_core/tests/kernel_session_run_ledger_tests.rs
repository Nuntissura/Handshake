#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: coverage restored from the suite deleted by 4f92cc25.
//!
//! Recovered from (pre-deletion):
//! `git show 4f92cc25^:src/backend/handshake_core/tests/kernel_postgres_event_ledger_tests.rs`
//!
//! That file's PostgreSQL-only tests (`kernel_event_ledger_migration`,
//! `kernel_event_ledger_replays_by_aggregate_id_in_sequence_order`,
//! `kernel_event_ledger_idempotency_rejects_divergent_duplicate`) are not
//! restored here; they are out of this microtask's scope.
//!
//! Four tests are restored onto a real embedded SurrealDB store
//! (`storage::tests::embedded_test_backend` — scoped per-run directory,
//! `SurrealDatabase::new(storage)` under the hood, same harness
//! `kernel_end_to_end_tests.rs` already uses for kernel proofs):
//!
//!   * `durable_claim_and_lease` — GENUINELY UNCOVERED. A prior lane found
//!     the kernel session-run claim/lease/retry/backpressure/deadletter
//!     state machine has no real-store coverage anywhere; all five
//!     `Database` methods it exercises
//!     (`enqueue_kernel_session_run`/`claim_kernel_session_run`/
//!     `update_kernel_session_run_state`) are live with Surreal impls
//!     (`src/storage/surreal/kernel_queue_store.rs`). Ported in full.
//!   * `kernel_event_ledger_contract_metadata_idempotency_and_sequence` —
//!     ported in full, including the duplicate-append idempotency re-check.
//!     tests/knowledge_code_index_tests.rs:91
//!     `event_ledger_idempotency_uses_payload_hash_not_jsonb_shape` also
//!     covers idempotent-duplicate-append dedup, but that test opens its own
//!     store with `open_embedded_store()` and `eprintln!`s `"SKIP ...:
//!     embedded store unavailable"` then returns early if that open fails —
//!     it is not a hard failure, so it can silently not execute. Per MT-150
//!     AC-150-1 a citation with a skip branch is not equivalent-strength
//!     coverage, so the dedup assertion (append the identical event twice,
//!     assert the second append returns the identical event_id/event_sequence
//!     as the first) is kept here rather than relied upon elsewhere, on top
//!     of the contract-metadata-shape and causation-linked-sequencing
//!     assertions the citation never covers regardless.
//!   * `kernel_event_ledger_api_appends_and_lists_kernel_events_for_session` —
//!     PARTIAL. tests/kernel_end_to_end_tests.rs:298
//!     `restart_reconstruction_proof` covers `list_kernel_events_for_session`
//!     generically through `KernelProofRunner`, but not direct multi-actor
//!     causation/correlation/payload assertions or cross-session isolation
//!     (a second session's events must not leak into the first session's
//!     list). Both are restored here.
//!   * `durable_claim_matches_retry_backpressure_and_deadletter_state_table` —
//!     PARTIAL. tests/kernel_runtime_tests.rs:49
//!     `broker_cancellation_backpressure_deadletter_states` only proves the
//!     abstract `SessionBroker::can_transition` legality graph in memory; it
//!     never calls `claim_kernel_session_run` /
//!     `update_kernel_session_run_state` against a real store. The durable
//!     retry/backpressure/deadletter claim behaviour is restored in full.

use handshake_core::kernel::{
    KernelActor, KernelEventType, KernelTaskRun, NewKernelEvent, SessionRun, SessionRunState,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

async fn embedded_or_environment_blocked() -> EmbeddedTestBackend {
    embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend")
}

#[tokio::test]
async fn durable_claim_and_lease() {
    let backend = embedded_or_environment_blocked().await;
    let db = backend.database.clone();

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

/// Full restore — see the module-level doc comment. Kept the original
/// duplicate-append idempotency re-check (tests/knowledge_code_index_tests.rs
/// :91 covers the same dedup behaviour but has its own skip branch, so it is
/// not equivalent-strength coverage under MT-150 AC-150-1) alongside the
/// contract-metadata shape and causation-linked sequencing that citation
/// never asserts regardless.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kernel_event_ledger_contract_metadata_idempotency_and_sequence() {
    let backend = embedded_or_environment_blocked().await;
    let db = backend.database.clone();

    let suffix = Uuid::now_v7();
    let task_id = format!("KTR-CONTRACT-{suffix}");
    let session_id = format!("SR-CONTRACT-{suffix}");
    let idempotency_key = format!("idem-contract-{suffix}");

    let first = NewKernelEvent::builder(
        task_id.clone(),
        session_id.clone(),
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-contract-test".to_string()),
    )
    .aggregate("session_run", session_id.clone())
    .idempotency_key(idempotency_key.clone())
    .event_version("kernel_event_v1")
    .source_component("session_broker")
    .correlation_id(format!("corr-contract-{suffix}"))
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
        KernelActor::ModelAdapter("adapter-contract-test".to_string()),
    )
    .aggregate("session_run", session_id.clone())
    .idempotency_key(format!("idem-contract-second-{suffix}"))
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

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id, stored_first.event_id);
    assert_eq!(events[1].event_id, stored_second.event_id);
    assert!(events[0].event_sequence < events[1].event_sequence);
    assert_eq!(events[0].payload_hash, stored_first.payload_hash);
    assert_eq!(
        events[1].causation_id.as_deref(),
        Some(events[0].event_id.as_str())
    );
}

/// PARTIAL restore — see the module-level doc comment. Adds the
/// cross-session isolation and direct multi-actor causation/correlation
/// assertions that `restart_reconstruction_proof` does not make.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kernel_event_ledger_api_appends_and_lists_kernel_events_for_session() {
    let backend = embedded_or_environment_blocked().await;
    let db = backend.database.clone();

    let suffix = Uuid::now_v7();
    let task_id = format!("KTR-KERNEL-LEDGER-{suffix}");
    let target_session = format!("SR-KERNEL-LEDGER-{suffix}");
    let other_session_id = format!("SR-OTHER-{suffix}");
    let correlation_id = format!("corr-kernel-ledger-{suffix}");

    let first = NewKernelEvent::builder(
        task_id.clone(),
        target_session.clone(),
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-ledger-test".to_string()),
    )
    .correlation_id(correlation_id.clone())
    .payload(json!({"queue": "primary"}))
    .build()
    .expect("valid first event");

    let stored_first = db
        .append_kernel_event(first)
        .await
        .expect("append first event");

    let second = NewKernelEvent::builder(
        task_id.clone(),
        target_session.clone(),
        KernelEventType::SessionClaimed,
        KernelActor::ModelAdapter("adapter-ledger-test".to_string()),
    )
    .causation_id(stored_first.event_id.clone())
    .correlation_id(correlation_id.clone())
    .payload(json!({"claim": {"lane": "codex"}}))
    .build()
    .expect("valid second event");

    let stored_second = db
        .append_kernel_event(second)
        .await
        .expect("append second event");

    // Cross-session isolation: an event for a DIFFERENT session under the
    // same task must not leak into the target session's list.
    let other_session = NewKernelEvent::builder(
        task_id,
        other_session_id,
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-ledger-test".to_string()),
    )
    .payload(json!({}))
    .build()
    .expect("valid other-session event");
    db.append_kernel_event(other_session)
        .await
        .expect("append other-session event");

    let events = db
        .list_kernel_events_for_session(&target_session)
        .await
        .expect("list events for session");

    assert_eq!(
        events.len(),
        2,
        "the other session's event must not appear in this session's list"
    );
    assert_eq!(events[0].event_id, stored_first.event_id);
    assert_eq!(events[1].event_id, stored_second.event_id);
    assert_eq!(events[0].event_type, KernelEventType::SessionQueued);
    assert_eq!(events[1].event_type, KernelEventType::SessionClaimed);
    assert_eq!(
        events[0].actor,
        KernelActor::SessionBroker("broker-ledger-test".to_string())
    );
    assert_eq!(
        events[1].actor,
        KernelActor::ModelAdapter("adapter-ledger-test".to_string())
    );
    assert_eq!(
        events[1].causation_id.as_deref(),
        Some(events[0].event_id.as_str())
    );
    assert_eq!(events[0].correlation_id.as_deref(), Some(correlation_id.as_str()));
    assert_eq!(events[1].payload["claim"]["lane"], "codex");
    assert!(events[0].created_at <= events[1].created_at);
}

/// PARTIAL restore — see the module-level doc comment. The cited coverage
/// only proves the abstract transition-legality graph in memory; this
/// restores the durable retry/backpressure/deadletter claim behaviour
/// against a real store in full.
#[tokio::test]
async fn durable_claim_matches_retry_backpressure_and_deadletter_state_table() {
    let backend = embedded_or_environment_blocked().await;
    let db = backend.database.clone();

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
