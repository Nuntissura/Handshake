//! MT-147 end-to-end Postgres integration test for the capsule build+inject chain.
//!
//! Composes the full MT-143 -> MT-144 -> MT-145 -> MT-146 spine over real
//! Postgres:
//!   CapsuleBuilder (MT-143 real adapter, fixture-backed FEMS)
//!     -> CapsuleInjector (MT-144 wired into the model-call boundary)
//!     -> CapsuleRecorder -> PostgresKernelActionSubmitter (MT-145 real catalog
//!         + kernel_event_ledger)
//!     -> MemoryIpcService over PostgresMemoryCapsuleStore (MT-146 durable
//!         list/get/suppress)
//!
//! Spec-Realism Gate compliance:
//!  - Sub-rule 1: no LiveXxxUnavailable / todo / unimplemented paths.
//!  - Sub-rule 2: real resource = Postgres via `postgres_backend_from_env`,
//!    matching the convention in `tests/kernel_postgres_event_ledger_tests.rs`.
//!    `#[ignore]`-gated; run with `cargo test -- --ignored` after setting
//!    `POSTGRES_TEST_URL`.
//!  - Sub-rule 3: a separate validator session signs off on the behaviour.

use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::Utc;
use handshake_core::{
    memory::{
        CapsuleBuilder, CapsuleFlightRecorderEvent, CapsulePolicyTable, CapsuleRecord,
        CapsuleRecorder, FemsError, FemsFlightRecorder, FemsFlightRecorderError, FemsRetriever,
        GetCapsuleRequest, InjectionDecision, ListRecentCapsulesRequest, MemoryCapsuleIpcStore,
        ModelCallContext, PostgresKernelActionSubmitter, PostgresMemoryCapsuleStore,
        RetrievedItem, SuppressItemRequest, TaskType, MEMORY_CAPSULE_AGGREGATE_TYPE,
        MEMORY_CAPSULE_RECORD_ACTION_ID,
    },
    storage::{tests::postgres_backend_from_env, StorageError},
};
use serde::Deserialize;

const QUERY: &str = "how do I add a new HBR rule applicability tag";
const ROLE_ID: &str = "KERNEL_BUILDER";
const SESSION_ID: &str = "KERNEL_BUILDER-MT-147-POSTGRES";
const FIXTURE_RELATIVE_PATH: &str = "tests/fixtures/memory_capsule_e2e/sample_fems_items.json";

async fn postgres_or_environment_blocked() -> Arc<dyn handshake_core::storage::Database> {
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: MT-147 memory capsule E2E tests require POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn capsule_builder_injector_recorder_and_ipc_compose_over_real_postgres() {
    let db = postgres_or_environment_blocked().await;

    let fixture_items = load_fixture_items();
    let fems = TestFemsAdapter::new(fixture_items);
    let policy_table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &policy_table);

    // MT-143: build the capsule from the FEMS fixture adapter.
    let _built_capsule = builder
        .build(handshake_core::memory::BuildContext {
            task_type: TaskType::KernelBuilderMtImplementation,
            query: QUERY.to_string(),
            role_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
            override_policy: None,
        })
        .expect("CapsuleBuilder must succeed against the MT-147 fixture");

    // MT-144: inject the capsule into the model-call context boundary. The
    // recording flight-recorder lets us observe the injection event.
    let flight_recorder = RecordingFemsFlightRecorder::default();
    let injector = handshake_core::memory::CapsuleInjector::new(&builder, &flight_recorder);
    let model_call_context = ModelCallContext::eligible(
        TaskType::KernelBuilderMtImplementation,
        QUERY,
        ROLE_ID,
        SESSION_ID,
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

    // MT-145: record the capsule through the real Postgres-backed kernel
    // action catalog dispatcher.
    let submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&db));
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = CapsuleRecord::from_capsule(&injected_capsule, Utc::now(), SESSION_ID, ROLE_ID);
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

    // MT-146: list/get/suppress over the durable Postgres-backed IPC store.
    let store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    store
        .save_capsule_record(record.clone())
        .expect("store.save_capsule_record");

    let service =
        handshake_core::memory::MemoryIpcService::new(&store, &submitter, &flight_recorder);
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
            reason: "MT-147 postgres E2E suppression".to_string(),
            actor_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
        })
        .expect("suppress_item");

    let restarted_store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    let restarted_service = handshake_core::memory::MemoryIpcService::new(
        &restarted_store,
        &submitter,
        &flight_recorder,
    );
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

#[derive(Default)]
struct RecordingFemsFlightRecorder {
    events: RefCell<Vec<CapsuleFlightRecorderEvent>>,
}

impl FemsFlightRecorder for RecordingFemsFlightRecorder {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}

fn load_fixture_items() -> Vec<RetrievedItem> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Path::new(FIXTURE_RELATIVE_PATH));
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
    assert_eq!(fixture.fixture_id, "mt-147-memory-capsule-e2e-sample-fems-items");
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
