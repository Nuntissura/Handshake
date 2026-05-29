//! MT-146 Postgres integration tests for the capsule inspection + suppression IPC.
//!
//! These tests bind [`MemoryIpcService`] to a real Postgres-backed
//! [`PostgresMemoryCapsuleStore`] + [`PostgresKernelActionSubmitter`] so the
//! list/get/suppress IPC operations are durable across process restarts.
//!
//! Spec-Realism Gate compliance:
//!  - Sub-rule 1: no LiveXxxUnavailable / todo / unimplemented paths.
//!  - Sub-rule 2: real resource = Postgres via `postgres_backend_from_env`
//!    matching `tests/kernel_postgres_event_ledger_tests.rs`; `#[ignore]`-gated
//!    so they only run with `cargo test -- --ignored` after the operator sets
//!    `POSTGRES_TEST_URL`.
//!  - Sub-rule 3: a separate validator session signs off on the behaviour.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use handshake_core::{
    memory::{
        CapsuleAuditEntry, CapsuleAuditLog, CapsuleFlightRecorderEvent, CapsuleRecord,
        DegradationTier, FemsFlightRecorder, FemsFlightRecorderError, GetCapsuleRequest,
        ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcService,
        PostgresKernelActionSubmitter, PostgresMemoryCapsuleStore, RetrievalPolicy,
        SuppressItemRequest, TaskType,
    },
    storage::{tests::postgres_backend_from_env, StorageError},
};
use std::cell::RefCell;
use std::sync::Arc;
use uuid::Uuid;

async fn postgres_or_environment_blocked() -> Arc<dyn handshake_core::storage::Database> {
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: MT-146 memory capsule IPC tests require POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn memory_ipc_list_and_get_round_trips_via_postgres_store() {
    let db = postgres_or_environment_blocked().await;
    let store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    let submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&db));
    let flight_recorder = NoopFlightRecorder::default();

    let service = MemoryIpcService::new(&store, &submitter, &flight_recorder);

    let record = sample_capsule_record();
    store
        .save_capsule_record(record.clone())
        .expect("save_capsule_record");

    // list_recent should include the just-saved record.
    let list = service
        .list_recent(ListRecentCapsulesRequest { limit: 25 })
        .expect("list_recent");
    assert!(
        list.capsules
            .iter()
            .any(|capsule| capsule.capsule_id == record.capsule_id),
        "Postgres-backed store must surface the just-saved record"
    );

    // get should round-trip the same record.
    let fetched = service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .expect("get");
    assert_eq!(fetched.record.capsule_id, record.capsule_id);
    assert_eq!(fetched.record.task_type, record.task_type);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn memory_ipc_suppression_persists_through_postgres_store_durably() {
    let db = postgres_or_environment_blocked().await;
    let store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    let submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&db));
    let flight_recorder = NoopFlightRecorder::default();

    let service = MemoryIpcService::new(&store, &submitter, &flight_recorder);

    let record = sample_capsule_record();
    store
        .save_capsule_record(record.clone())
        .expect("save_capsule_record");

    let suppression = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: record.audit_log.entries[0].item_id.clone(),
            reason: "operator rejected MT-146 postgres-test capsule context".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-mt-146-postgres".to_string(),
        })
        .expect("suppress_item");
    assert_eq!(suppression.capsule_id, record.capsule_id);

    // Re-fetch through a FRESH store instance — this proves durability across a
    // simulated process restart. The previous store no longer holds the record
    // in memory; the data must come from Postgres.
    let restarted_store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    let restarted_service = MemoryIpcService::new(&restarted_store, &submitter, &flight_recorder);
    let after_restart = restarted_service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .expect("get after restart");
    let suppressed_entry = after_restart
        .record
        .audit_log
        .entries
        .iter()
        .find(|entry| entry.item_id == record.audit_log.entries[0].item_id)
        .expect("suppressed entry must remain visible");
    assert!(!suppressed_entry.included);
    assert!(suppressed_entry.suppression_reason.is_some());
}

#[derive(Default)]
struct NoopFlightRecorder {
    events: RefCell<Vec<CapsuleFlightRecorderEvent>>,
}

impl FemsFlightRecorder for NoopFlightRecorder {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}

fn sample_capsule_record() -> CapsuleRecord {
    CapsuleRecord {
        capsule_id: Uuid::now_v7(),
        capsule_source_hash: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
            .to_string(),
        task_type: TaskType::KernelBuilderMtImplementation,
        policy: RetrievalPolicy {
            top_k: 12,
            capsule_budget_bytes: 65_536,
            task_type: TaskType::KernelBuilderMtImplementation,
            scoring_formula_version: "retrieval_scoring_formula_v0".to_string(),
            graceful_degradation_tier: DegradationTier::Tiered,
        },
        audit_log: CapsuleAuditLog {
            entries: vec![CapsuleAuditEntry {
                item_id: "item-146-postgres".to_string(),
                source_uri: "fems://source/artifact/artifact-146#item-1".to_string(),
                included: true,
                suppression_reason: None,
                score: 0.88,
                score_breakdown: BTreeMap::from([("similarity".to_string(), 0.88)]),
                pinned: false,
            }],
        },
        built_at_utc: dt("2026-05-19T11:00:00Z"),
        recorded_at_utc: dt("2026-05-19T11:05:00Z"),
        session_id: "session-mt-146-postgres".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        outcome: None,
    }
}

fn dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
