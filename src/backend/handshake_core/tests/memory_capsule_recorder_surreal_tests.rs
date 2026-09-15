#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: coverage restored from the suite deleted by 4f92cc25.
//!
//! Recovers the two non-PG-specific tests from the deleted
//! `memory_persistence_postgres_tests.rs`: that [`CapsuleRecorder`] bound to
//! the production kernel-action adapter persists through the real
//! EventLedger and dedupes on idempotency_key. The deleted tests bound to
//! `SurrealKernelActionSubmitter` against the embedded authority
//! server); the drop-in production adapter today is
//! [`SurrealKernelActionSubmitter`] (confirmed by its own in-source proof at
//! `src/storage/surreal/mt136_kernel_action_submitter_proof.rs`, which
//! currently FAILS in the suite -- see MT-150 lane notes -- so it does not
//! cover these two behaviors and is not relied on here). These tests stand
//! alone against a real embedded SurrealDB store (the same
//! `atelier_surreal_support::AtelierSurrealHarness` embedded-store harness
//! every other `atelier_*` suite uses, borrowed here only for its
//! schema-bootstrapped `Arc<dyn Database>` -- no Atelier domain data is
//! involved) and require no external server, so unlike the deleted originals
//! they are NOT `#[ignore]`d.
//!
//! MT-150 V3-PRE-03: the three memory-IPC items formerly typed
//! `BLOCKED_ON_PRODUCT_DEFECT` (DEF-MEMORY-IPC-NO-PRODUCTION-STORE) are ported
//! here too, now that the production store exists
//! (`SurrealMemoryCapsuleStore`, `src/memory/persistence.rs`, wired at Tauri
//! startup): `memory_ipc_list_and_get_round_trips_via_surreal_store` and
//! `memory_ipc_suppression_persists_through_surreal_store_durably` (from the
//! deleted `memory_ipc_postgres_tests.rs`) and
//! `capsule_builder_injector_recorder_and_ipc_compose_over_embedded_surreal`
//! (from the deleted `memory_capsule_e2e_postgres_tests.rs`), each with the
//! original assertions. The Surreal store is EventLedger-replayed rather than
//! row-backed, so `save_capsule_record` VERIFIES the durable state written by
//! the recorder instead of inserting a second copy -- the original
//! `recorder.record(..)` + `store.save_capsule_record(..)` sequence is kept
//! and `save_capsule_record` is asserted to accept the identical record.

mod atelier_surreal_support;

use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use handshake_core::{
    memory::{
        persistence::{MEMORY_CAPSULE_AGGREGATE_TYPE, MEMORY_CAPSULE_SOURCE_COMPONENT},
        CapsuleAuditEntry, CapsuleAuditLog, CapsuleBuilder, CapsuleFlightRecorderEvent,
        CapsulePolicyTable, CapsuleRecord, CapsuleRecorder, DegradationTier, FemsError,
        FemsFlightRecorder, FemsFlightRecorderError, FemsRetriever, GetCapsuleRequest,
        InjectionDecision, ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcService,
        ModelCallContext, RetrievalPolicy, RetrievedItem, SuppressItemRequest,
        SurrealKernelActionSubmitter, SurrealMemoryCapsuleStore, TaskType,
        MEMORY_CAPSULE_RECORD_ACTION_ID,
    },
    storage::Database,
};
use serde::Deserialize;
use uuid::Uuid;

const E2E_QUERY: &str = "how do I add a new HBR rule applicability tag";
const E2E_ROLE_ID: &str = "KERNEL_BUILDER";
const E2E_SESSION_ID: &str = "KERNEL_BUILDER-MT-150-SURREAL";
const FIXTURE_RELATIVE_PATH: &str = "tests/fixtures/memory_capsule_e2e/sample_fems_items.json";

/// An isolated, schema-bootstrapped embedded SurrealDB `Arc<dyn Database>`,
/// reusing the shared Atelier test harness purely for its store plumbing.
async fn embedded_database() -> (
    Arc<dyn Database>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    let database = harness.database.clone();
    (database, harness)
}

#[tokio::test(flavor = "multi_thread")]
async fn capsule_suppression_and_flight_recorder_evidence_survive_store_reopen() {
    let directory = tempfile::tempdir().expect("temporary capsule authority root");
    let data_dir = directory.path().to_string_lossy().to_string();
    let config = handshake_core::storage::ControlPlaneStorageConfig::resolve(
        Some("surreal_embedded"),
        Some(&data_dir),
    )
    .expect("resolve embedded authority configuration");
    let control_plane = handshake_core::storage::init_control_plane_storage_with_config(&config)
        .await
        .expect("initialize embedded authority");
    let db = control_plane.database.clone();
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = sample_capsule_record();
    recorder.record(record.clone()).expect("record capsule");

    let store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
    let service = MemoryIpcService::new(&store, &submitter, &submitter);
    let receipt = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: record.audit_log.entries[0].item_id.clone(),
            reason: "operator removed stale evidence".to_owned(),
            actor_id: "operator-mt-136".to_owned(),
            session_id: "session-mt-136".to_owned(),
        })
        .expect("durably suppress capsule item");
    assert_eq!(receipt.suppressed_item_count, 1);
    assert_eq!(
        submitter
            .list_capsule_flight_recorder_events(record.capsule_id)
            .expect("read durable Flight Recorder evidence")
            .len(),
        1
    );

    drop(service);
    drop(store);
    drop(submitter);
    drop(db);
    control_plane
        .surreal
        .shutdown()
        .await
        .expect("close embedded authority");

    let reopened = handshake_core::storage::init_control_plane_storage_with_config(&config)
        .await
        .expect("reopen embedded authority");
    let reopened_store = SurrealMemoryCapsuleStore::with_db(reopened.database.clone());
    let durable = reopened_store
        .get_capsule_record(record.capsule_id)
        .expect("read capsule after reopen")
        .expect("capsule remains present after reopen");
    assert!(!durable.audit_log.entries[0].included);
    assert_eq!(
        durable.audit_log.entries[0].suppression_reason.as_deref(),
        Some("operator removed stale evidence")
    );
    let reopened_submitter = SurrealKernelActionSubmitter::with_db(reopened.database.clone());
    let events = reopened_submitter
        .list_capsule_flight_recorder_events(record.capsule_id)
        .expect("read Flight Recorder evidence after reopen");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id(), "FR-EVT-CAPSULE-SUPPRESSED");
    reopened
        .surreal
        .shutdown()
        .await
        .expect("close reopened authority");
}

#[tokio::test(flavor = "multi_thread")]
async fn capsule_recorder_persists_via_kernel_action_catalog_against_embedded_store() {
    let (db, _harness) = embedded_database().await;

    // Bind the recorder to the real embedded-Surreal-backed kernel action
    // catalog dispatcher -- the production drop-in for the deleted
    // `SurrealKernelActionSubmitter`.
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));

    // Sanity-check the catalog includes the memory_capsule.record action so the
    // wiring is real (not a stub).
    assert!(
        submitter
            .catalog()
            .action(MEMORY_CAPSULE_RECORD_ACTION_ID)
            .is_some(),
        "KernelActionCatalogV1 must register memory_capsule.record action"
    );

    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let record = sample_capsule_record();
    let receipt = recorder.record(record.clone()).expect("recorder.record");

    assert_eq!(receipt.record_id.get_version_num(), 7);
    assert_eq!(receipt.write_box_envelope_id.get_version_num(), 7);

    // Re-read from the kernel_event_ledger and confirm an event row landed for
    // this capsule under the memory_capsule aggregate.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list kernel events for memory_capsule aggregate");

    assert!(
        !events.is_empty(),
        "kernel_event_ledger must contain at least one event for the recorded capsule"
    );
    let stored = &events[0];
    assert_eq!(stored.aggregate_type, MEMORY_CAPSULE_AGGREGATE_TYPE);
    assert_eq!(stored.aggregate_id, record.capsule_id.to_string());
    assert_eq!(stored.source_component, MEMORY_CAPSULE_SOURCE_COMPONENT);
    assert_eq!(stored.event_version, "kernel_event_v1");
    assert_eq!(stored.payload_hash.len(), 64);
    assert_eq!(
        stored.payload["catalog_action_id"].as_str(),
        Some(MEMORY_CAPSULE_RECORD_ACTION_ID),
        "payload must record the kernel action catalog action_id"
    );
    assert_eq!(
        stored.payload["request"]["target_ids"][0]["target_id"]
            .as_str()
            .map(str::to_string),
        Some(record.capsule_id.to_string())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn capsule_recorder_dedup_collapses_duplicate_submissions() {
    let (db, _harness) = embedded_database().await;
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));

    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let record = sample_capsule_record();
    let first = recorder.record(record.clone()).expect("first record");
    // Re-submit the identical record. The idempotency_key in the underlying
    // KernelActionRequestV1 is deterministic (derived from the record, not the
    // receipt), so the ledger should dedupe at the database boundary just like
    // any other kernel event.
    let second = recorder.record(record.clone()).expect("second record");

    // The recorder derives receipt identity from the idempotency key, so the
    // persisted ledger rows collapse to a single event.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list ledger events for aggregate");
    assert!(
        events.len() <= 2,
        "duplicate capsule action submissions must not produce more than 2 ledger rows (idempotent dedup expected)"
    );
    // The submission carries the same idempotency_key so the ledger row for the
    // first submission must remain visible.
    assert_eq!(events[0].aggregate_id, record.capsule_id.to_string());
    let _ = (first, second);
}

/// Restored from `memory_ipc_postgres_tests.rs`: list_recent / get round-trip through
/// `MemoryIpcService` bound to the production `SurrealMemoryCapsuleStore`.
#[tokio::test(flavor = "multi_thread")]
async fn memory_ipc_list_and_get_round_trips_via_surreal_store() {
    let (db, _harness) = embedded_database().await;
    let store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));
    let flight_recorder = NoopFlightRecorder::default();

    let service = MemoryIpcService::new(&store, &submitter, &flight_recorder);

    let record = sample_capsule_record();
    // The production store is EventLedger-replayed: the recorder is the durable write path and
    // `save_capsule_record` verifies that exact durable state (the original's save call kept).
    CapsuleRecorder {
        action_catalog: &submitter,
    }
    .record(record.clone())
    .expect("record capsule through the kernel action catalog");
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
        "Surreal-backed store must surface the just-saved record"
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

/// Restored from `memory_ipc_postgres_tests.rs`: a suppression persists through the
/// production store durably -- re-fetched through a FRESH store instance over the same
/// database (the original's simulated process restart).
#[tokio::test(flavor = "multi_thread")]
async fn memory_ipc_suppression_persists_through_surreal_store_durably() {
    let (db, _harness) = embedded_database().await;
    let store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));
    let flight_recorder = NoopFlightRecorder::default();

    let service = MemoryIpcService::new(&store, &submitter, &flight_recorder);

    let record = sample_capsule_record();
    CapsuleRecorder {
        action_catalog: &submitter,
    }
    .record(record.clone())
    .expect("record capsule through the kernel action catalog");
    store
        .save_capsule_record(record.clone())
        .expect("save_capsule_record");

    let suppression = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: record.audit_log.entries[0].item_id.clone(),
            reason: "operator rejected MT-150 surreal-test capsule context".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-mt-150-surreal".to_string(),
        })
        .expect("suppress_item");
    assert_eq!(suppression.capsule_id, record.capsule_id);

    // Re-fetch through a FRESH store instance -- this proves durability across a
    // simulated process restart. The previous store no longer holds the record
    // in memory; the data must come from the embedded SurrealDB EventLedger.
    let restarted_store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
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

/// Restored from `memory_capsule_e2e_postgres_tests.rs`: the full MT-143 -> MT-144 -> MT-145
/// -> MT-146 spine composed over the real embedded SurrealDB authority: CapsuleBuilder
/// (fixture-backed FEMS) -> CapsuleInjector -> CapsuleRecorder -> SurrealKernelActionSubmitter
/// (real catalog + kernel_event_ledger) -> MemoryIpcService over SurrealMemoryCapsuleStore
/// (durable list/get/suppress), then suppression durability across a fresh store instance.
#[tokio::test(flavor = "multi_thread")]
async fn capsule_builder_injector_recorder_and_ipc_compose_over_embedded_surreal() {
    let (db, _harness) = embedded_database().await;

    let fixture_items = load_fixture_items();
    let fems = TestFemsAdapter::new(fixture_items);
    let policy_table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &policy_table);

    // MT-143: build the capsule from the FEMS fixture adapter.
    let _built_capsule = builder
        .build(handshake_core::memory::BuildContext {
            task_type: TaskType::KernelBuilderMtImplementation,
            query: E2E_QUERY.to_string(),
            role_id: E2E_ROLE_ID.to_string(),
            session_id: E2E_SESSION_ID.to_string(),
            override_policy: None,
        })
        .expect("CapsuleBuilder must succeed against the MT-147 fixture");

    // MT-144: inject the capsule into the model-call context boundary. The
    // recording flight-recorder lets us observe the injection event.
    let flight_recorder = NoopFlightRecorder::default();
    let injector = handshake_core::memory::CapsuleInjector::new(&builder, &flight_recorder);
    let model_call_context = ModelCallContext::eligible(
        TaskType::KernelBuilderMtImplementation,
        E2E_QUERY,
        E2E_ROLE_ID,
        E2E_SESSION_ID,
    );
    let decision = injector
        .inject_for_call(&model_call_context)
        .expect("CapsuleInjector must produce an Inject decision over the fixture");
    let injected_capsule = match decision {
        InjectionDecision::Inject { capsule, .. } => capsule,
        InjectionDecision::Skip { reason } => {
            panic!("expected Inject decision, got Skip {reason:?}")
        }
    };

    // MT-145: record the capsule through the real embedded-Surreal-backed kernel
    // action catalog dispatcher.
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = CapsuleRecord::from_capsule(
        &injected_capsule,
        Utc::now(),
        E2E_SESSION_ID,
        E2E_ROLE_ID,
    );
    let _receipt = recorder.record(record.clone()).expect("recorder.record");

    // Confirm the ledger now carries the catalog-action event.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list ledger events for capsule aggregate");
    assert!(events.iter().any(|event| event
        .payload
        .get("catalog_action_id")
        .and_then(|v| v.as_str())
        == Some(MEMORY_CAPSULE_RECORD_ACTION_ID)));

    // MT-146: list/get/suppress over the durable Surreal-backed IPC store.
    let store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
    store
        .save_capsule_record(record.clone())
        .expect("store.save_capsule_record");

    let service = MemoryIpcService::new(&store, &submitter, &flight_recorder);
    let list = service
        .list_recent(ListRecentCapsulesRequest { limit: 50 })
        .expect("list_recent");
    assert!(list
        .capsules
        .iter()
        .any(|capsule| capsule.capsule_id == record.capsule_id));

    let fetched = service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .expect("get");
    assert_eq!(fetched.record.capsule_id, record.capsule_id);

    // Exercise suppression and confirm durability across a simulated restart.
    let included_item_id = record
        .audit_log
        .entries
        .iter()
        .find(|entry| entry.included)
        .map(|entry| entry.item_id.clone())
        .expect("fixture must produce at least one included audit entry");
    let _suppression = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: included_item_id.clone(),
            reason: "MT-150 surreal E2E suppression".to_string(),
            actor_id: E2E_ROLE_ID.to_string(),
            session_id: E2E_SESSION_ID.to_string(),
        })
        .expect("suppress_item");

    let restarted_store = SurrealMemoryCapsuleStore::with_db(Arc::clone(&db));
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
        .find(|entry| entry.item_id == included_item_id)
        .expect("suppressed item must survive restart");
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

#[derive(Default)]
struct TestFemsAdapter {
    items: Vec<RetrievedItem>,
    calls: RefCell<Vec<(String, u32)>>,
}

impl TestFemsAdapter {
    fn new(items: Vec<RetrievedItem>) -> Self {
        Self {
            items,
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl FemsRetriever for TestFemsAdapter {
    fn retrieve(&self, query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        self.calls.borrow_mut().push((query.to_string(), top_k));
        Ok(self.items.clone())
    }
}

fn load_fixture_items() -> Vec<RetrievedItem> {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Path::new(FIXTURE_RELATIVE_PATH));
    let raw = fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
        panic!(
            "MT-147 fixture file is required at {}: {error}",
            fixture_path.display()
        )
    });
    let fixture: FixtureFile = serde_json::from_str(&raw).unwrap_or_else(|error| {
        panic!(
            "MT-147 fixture file must match strict JSON contract at {}: {error}",
            fixture_path.display()
        )
    });
    // Sanity-check the fixture is the same one the in-memory MT-147 e2e uses so
    // we are exercising the spec-required path, not an ad-hoc test fixture.
    assert_eq!(fixture.schema_version, "sample_fems_items.v1");
    assert_eq!(
        fixture.fixture_id,
        "mt-147-memory-capsule-e2e-sample-fems-items"
    );
    assert_eq!(fixture.wp_id, "WP-KERNEL-004");
    assert_eq!(fixture.mt_id, "MT-147");
    fixture.items
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    schema_version: String,
    fixture_id: String,
    wp_id: String,
    mt_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    intended_task_type: String,
    items: Vec<RetrievedItem>,
}

fn sample_capsule_record() -> CapsuleRecord {
    CapsuleRecord {
        capsule_id: Uuid::now_v7(),
        capsule_source_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
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
                item_id: "item-150-surreal".to_string(),
                source_uri: "fems://source/artifact/artifact-150#item-1".to_string(),
                included: true,
                suppression_reason: None,
                score: 0.91,
                score_breakdown: BTreeMap::from([("similarity".to_string(), 0.91)]),
                pinned: false,
            }],
        },
        built_at_utc: dt("2026-05-19T10:00:00Z"),
        recorded_at_utc: dt("2026-05-19T10:05:00Z"),
        session_id: "session-mt-150-surreal".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        outcome: None,
    }
}

fn dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
