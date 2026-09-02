use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use handshake_core::ace::{FemsSourceRef, FemsSourceRefKind};
use handshake_core::kernel::action_catalog::kernel002_action_catalog;
use handshake_core::memory::outcome_feedback::{
    CapsuleOutcome, FailureClass, MemoryPackItemRef, OutcomeScoringTuner, TuningParams,
};
use handshake_core::memory::pinned_core::{
    PinError, PinIpcService, PinReceipt, PinSubmitter, PinnedBudget, PinnedCoreSelector,
    PinnedItem, SetPinRequest, FR_EVT_MEMORY_PIN, FR_EVT_MEMORY_UNPIN, PIN_MEMORY_ACTION_ID,
    UNPIN_MEMORY_ACTION_ID,
};
use handshake_core::memory::{
    BuildContext, BuilderError, CapsuleBuilder, CapsulePolicyTable, DegradationTier, FemsError,
    FemsRetriever, RetrievalPolicy, RetrievedItem, SurrealKernelActionSubmitter, TaskType,
    RETRIEVAL_SCORING_FORMULA_V0,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::Database;
use uuid::Uuid;

struct TestFemsRetriever {
    items: Vec<RetrievedItem>,
}

impl TestFemsRetriever {
    fn new(items: Vec<RetrievedItem>) -> Self {
        Self { items }
    }
}

impl FemsRetriever for TestFemsRetriever {
    fn retrieve(&self, _query: &str, _top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        Ok(self.items.clone())
    }
}

struct RecordingPinSubmitter {
    submissions: RefCell<Vec<PinnedItem>>,
}

impl RecordingPinSubmitter {
    fn new() -> Self {
        Self {
            submissions: RefCell::new(Vec::new()),
        }
    }

    fn submitted_count(&self) -> usize {
        self.submissions.borrow().len()
    }

    fn submitted(&self) -> Vec<PinnedItem> {
        self.submissions.borrow().clone()
    }
}

impl PinSubmitter for RecordingPinSubmitter {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError> {
        self.submissions.borrow_mut().push(item.clone());
        Ok(PinReceipt {
            receipt_id: Uuid::now_v7(),
            memory_id: item.memory_id,
            pinned: item.pinned,
            action_id: if item.pinned {
                PIN_MEMORY_ACTION_ID.to_string()
            } else {
                UNPIN_MEMORY_ACTION_ID.to_string()
            },
            fr_event_kind: if item.pinned {
                FR_EVT_MEMORY_PIN.to_string()
            } else {
                FR_EVT_MEMORY_UNPIN.to_string()
            },
        })
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        Ok(self
            .submissions
            .borrow()
            .iter()
            .filter(|item| item.pinned)
            .cloned()
            .collect())
    }
}

#[test]
fn selector_rejects_pinned_item_count_overflow_before_unpinned_scoring() {
    let items = vec![
        retrieved("pinned-a", 0.01, 10, true),
        retrieved("pinned-b", 0.02, 10, true),
        retrieved("unpinned-high", 0.99, 10, false),
    ];

    let error = PinnedCoreSelector::select_pack_with_pins(
        &items,
        PinnedBudget {
            max_items: 1,
            max_bytes: 1_000,
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PinError::PinnedExceedsBudget {
            pinned_items: 2,
            budget_items: 1,
            ..
        }
    ));
}

#[test]
fn capsule_builder_rejects_pinned_byte_overflow_before_capsule_creation() {
    let fems = TestFemsRetriever::new(vec![
        retrieved("pinned-a", 0.01, 70, true),
        retrieved("pinned-b", 0.02, 40, true),
        retrieved("unpinned-high", 0.99, 5, false),
    ]);
    let builder = CapsuleBuilder::new(&fems, &CapsulePolicyTable);
    let mut context = build_context();
    context.override_policy = Some(policy(4, 100));

    let error = builder.build(context).unwrap_err();

    assert!(matches!(
        error,
        BuilderError::PinnedCore(PinError::PinnedExceedsBudget {
            pinned_bytes: 110,
            budget_bytes: 100,
            ..
        })
    ));
}

#[test]
fn capsule_builder_keeps_pinned_items_first_then_scores_remaining_budget() {
    let fems = TestFemsRetriever::new(vec![
        retrieved("unpinned-high", 0.99, 40, false),
        retrieved("pinned-low", 0.01, 50, true),
        retrieved("unpinned-medium", 0.50, 30, false),
        retrieved("unpinned-overflow", 0.49, 30, false),
    ]);
    let builder = CapsuleBuilder::new(&fems, &CapsulePolicyTable);
    let mut context = build_context();
    context.override_policy = Some(policy(4, 120));

    let capsule = builder.build(context).unwrap();

    assert_eq!(
        capsule
            .pack
            .items
            .iter()
            .map(|item| item.memory_id.as_str())
            .collect::<Vec<_>>(),
        vec!["pinned-low", "unpinned-high", "unpinned-medium"]
    );
    assert!(capsule.audit.entry("pinned-low").unwrap().pinned);
    assert!(!capsule.audit.entry("unpinned-overflow").unwrap().included);
}

#[test]
fn outcome_tuner_does_not_decay_pinned_items() {
    let pinned_id = Uuid::from_u128(1);
    let unpinned_id = Uuid::from_u128(2);
    let mut scores = HashMap::from([(pinned_id, 0.5), (unpinned_id, 0.5)]);
    let pack = vec![
        MemoryPackItemRef {
            memory_id: pinned_id,
            pinned: true,
        },
        MemoryPackItemRef {
            memory_id: unpinned_id,
            pinned: false,
        },
    ];

    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Fail {
            mt_id: "MT-159".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::Other,
        },
        &pack,
        &TuningParams::default(),
    );

    assert_eq!(scores[&pinned_id], 0.5);
    assert!(scores[&unpinned_id] < 0.5);
}

#[test]
fn pin_and_unpin_actions_are_registered_in_kernel_action_catalog() {
    let catalog = kernel002_action_catalog();
    let pin = catalog
        .action(PIN_MEMORY_ACTION_ID)
        .expect("pin memory action must be registered");
    let unpin = catalog
        .action(UNPIN_MEMORY_ACTION_ID)
        .expect("unpin memory action must be registered");

    assert_eq!(pin.expected_write_boxes[0].target_id, "memory_item_pin");
    assert_eq!(unpin.expected_write_boxes[0].target_id, "memory_item_unpin");
    assert!(pin
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "flight_recorder_event"));
    assert!(unpin
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "flight_recorder_event"));
}

#[test]
fn pin_ipc_set_routes_through_submitter_and_returns_ledger_fr_receipt() {
    let submitter = RecordingPinSubmitter::new();
    let service = PinIpcService::new(&submitter);
    let memory_id = Uuid::now_v7();

    let receipt = service
        .set(SetPinRequest {
            item_id: memory_id,
            pinned: true,
            reason: "operator core memory".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
        })
        .expect("pin request");

    assert_eq!(receipt.memory_id, memory_id);
    assert_eq!(receipt.action_id, PIN_MEMORY_ACTION_ID);
    assert_eq!(receipt.fr_event_kind, FR_EVT_MEMORY_PIN);
    assert_eq!(submitter.submitted_count(), 1);
    let submitted = submitter.submitted();
    assert_eq!(submitted[0].reason, "operator core memory");
    assert_eq!(submitted[0].actor_id, "KERNEL_BUILDER");
    assert_eq!(submitted[0].session_id, "session-159");
}

#[test]
fn pin_ipc_rejects_empty_reason_before_submitter_or_fr_side_effects() {
    let submitter = RecordingPinSubmitter::new();
    let service = PinIpcService::new(&submitter);

    let error = service
        .set(SetPinRequest {
            item_id: Uuid::now_v7(),
            pinned: false,
            reason: "   ".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
        })
        .unwrap_err();

    assert!(matches!(error, PinError::EmptyReason));
    assert_eq!(submitter.submitted_count(), 0);
}

#[test]
fn pinned_migration_source_scan_is_guarded_for_memory_item_table() {
    let migration = std::fs::read_to_string("migrations/2026_05_18_fems_pinned.sql")
        .expect("MT-159 pinned migration must exist");

    assert!(migration.contains("kernel_event_ledger"));
    assert!(migration.contains("memory_item"));
    assert!(migration.contains("hsk.memory_pin.payload@1"));
    assert!(migration.contains("WHERE aggregate_type = 'memory_item'"));
    assert!(!migration.contains("CREATE TABLE memory_item"));
    assert!(!migration.contains("ALTER TABLE memory_item"));
    assert!(!migration.to_ascii_lowercase().contains("sqlite"));
    assert!(!migration.contains("INTEGER NOT NULL DEFAULT 0"));
}

#[test]
fn pin_tauri_commands_are_registered_and_legacy_adapter_source_scan_is_explicit() {
    let repo = repo_root();
    let memory_pin_rs =
        std::fs::read_to_string(repo.join("app/src-tauri/src/commands/memory_pin.rs"))
            .expect("read memory pin Tauri command source");
    let lib_rs = std::fs::read_to_string(repo.join("app/src-tauri/src/lib.rs"))
        .expect("read Tauri lib source");

    for command in [
        "kernel_memory_pin_set",
        "kernel_memory_pin_unset",
        "kernel_memory_pin_list",
    ] {
        assert!(
            memory_pin_rs.contains(&format!("pub async fn {command}")),
            "missing Tauri command function {command}"
        );
        assert!(
            lib_rs.contains(&format!("commands::memory_pin::{command}")),
            "missing invoke_handler registration for {command}"
        );
    }
    assert!(lib_rs.contains("pub mod memory_pin"));
    assert!(lib_rs.contains("MemoryPinIpcState::from_env_or_unavailable()"));
    assert!(memory_pin_rs.contains("MemoryPinIpcState"));
    assert!(!memory_pin_rs.contains("InMemory"));
}

#[test]
fn pin_embedded_adapter_source_scan_preserves_atomic_action_and_manifest_append() {
    let source = std::fs::read_to_string("src/memory/persistence.rs").expect("read persistence");
    assert!(source.contains("append_kernel_events_atomic"));
    assert!(source.contains("vec![action_event, manifest_event]"));
    assert!(source.contains("memory_pin_atomic_append_failed"));
    assert!(source.contains("existing_pin_submission_matches"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_pin_submitter_durable_event_replay_proof() {
    let root = tempfile::tempdir().expect("create isolated embedded pin store root");
    let config = SurrealStorageConfig::for_data_dir(root.path().join("data"))
        .expect("configure isolated embedded pin store");
    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open isolated embedded pin store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap embedded pin schema");
    let db: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));
    let memory_id = Uuid::now_v7();

    let receipt = PinSubmitter::set_pin(
        &submitter,
        PinnedItem {
            memory_id,
            pinned: true,
            reason: "operator core memory".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
            set_at_utc: Utc::now(),
        },
    )
    .expect("persist embedded pin");
    assert_eq!(receipt.action_id, PIN_MEMORY_ACTION_ID);
    assert_eq!(receipt.fr_event_kind, FR_EVT_MEMORY_PIN);

    let pinned = PinSubmitter::list_pinned(&submitter).expect("replay pinned manifest");
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].memory_id, memory_id);
    assert!(pinned[0].pinned);
    assert_eq!(pinned[0].actor_id, "KERNEL_BUILDER");
    assert_eq!(pinned[0].session_id, "session-159");

    let pin_events = db
        .list_kernel_events_for_aggregate("memory_item", &memory_id.to_string())
        .await
        .expect("read embedded pin EventLedger events");
    assert_eq!(pin_events.len(), 1);
    assert_eq!(
        pin_events[0].payload["catalog_action_id"].as_str(),
        Some(PIN_MEMORY_ACTION_ID)
    );
    assert_eq!(
        pin_events[0].payload["write_box_envelope"]["payload"]["flight_recorder_event_id"].as_str(),
        Some(FR_EVT_MEMORY_PIN)
    );
    assert_eq!(
        pin_events[0].payload["write_box_envelope"]["payload"]["pinned_item"]["memory_id"].as_str(),
        Some(memory_id.to_string().as_str())
    );
    assert_eq!(
        pin_events[0].payload["request"]["actor"]["actor_id"].as_str(),
        Some("KERNEL_BUILDER")
    );
    assert_eq!(
        pin_events[0].payload["request"]["session"]["session_id"].as_str(),
        Some("session-159")
    );
    assert_eq!(
        db.list_kernel_events_for_aggregate("memory_pin_manifest", "memory_pin_manifest_v1")
            .await
            .expect("read embedded pin manifest")
            .len(),
        1
    );

    storage.shutdown().await.expect("close embedded pin store");
    drop(submitter);
    drop(db);
    drop(storage);

    let reopened_storage = SurrealStorage::open(config.clone())
        .await
        .expect("reopen embedded pin store");
    let reopened_db: Arc<dyn Database> = Arc::new(SurrealDatabase::new(reopened_storage.clone()));
    let reopened_submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&reopened_db));
    let reopened_pinned =
        PinSubmitter::list_pinned(&reopened_submitter).expect("replay pin after close/reopen");
    assert_eq!(reopened_pinned.len(), 1);
    assert_eq!(reopened_pinned[0].memory_id, memory_id);

    let unpin = PinSubmitter::set_pin(
        &reopened_submitter,
        PinnedItem {
            memory_id,
            pinned: false,
            reason: "operator unpinned core memory".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
            set_at_utc: Utc::now(),
        },
    )
    .expect("persist embedded unpin");
    assert_eq!(unpin.action_id, UNPIN_MEMORY_ACTION_ID);
    assert_eq!(unpin.fr_event_kind, FR_EVT_MEMORY_UNPIN);
    assert!(PinSubmitter::list_pinned(&reopened_submitter)
        .expect("replay unpinned manifest")
        .is_empty());

    let action_events = reopened_db
        .list_kernel_events_for_aggregate("memory_item", &memory_id.to_string())
        .await
        .expect("read pin and unpin EventLedger events");
    assert_eq!(action_events.len(), 2);
    let latest = action_events
        .iter()
        .max_by_key(|event| event.event_sequence)
        .expect("latest embedded pin action");
    assert_eq!(
        latest.payload["catalog_action_id"].as_str(),
        Some(UNPIN_MEMORY_ACTION_ID)
    );
    assert_eq!(
        reopened_db
            .list_kernel_events_for_aggregate("memory_pin_manifest", "memory_pin_manifest_v1")
            .await
            .expect("read pin and unpin manifest events")
            .len(),
        2
    );

    reopened_storage
        .shutdown()
        .await
        .expect("close embedded pin store after unpin");
    drop(reopened_submitter);
    drop(reopened_db);
    drop(reopened_storage);

    let final_storage = SurrealStorage::open(config)
        .await
        .expect("reopen embedded pin store after unpin");
    let final_db: Arc<dyn Database> = Arc::new(SurrealDatabase::new(final_storage.clone()));
    let final_submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&final_db));
    assert!(PinSubmitter::list_pinned(&final_submitter)
        .expect("replay durable unpin after close/reopen")
        .is_empty());
    assert_eq!(
        final_db
            .list_kernel_events_for_aggregate("memory_item", &memory_id.to_string())
            .await
            .expect("read durable pin and unpin EventLedger events")
            .len(),
        2
    );
    final_storage
        .shutdown()
        .await
        .expect("close final embedded pin store");
    drop(final_submitter);
    drop(final_db);
    drop(final_storage);
}

fn build_context() -> BuildContext {
    BuildContext {
        task_type: TaskType::KernelBuilderMtImplementation,
        query: "build query".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        session_id: "session-159".to_string(),
        override_policy: None,
    }
}

fn policy(top_k: u32, capsule_budget_bytes: u64) -> RetrievalPolicy {
    RetrievalPolicy {
        top_k,
        capsule_budget_bytes,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    }
}

fn retrieved(id: &str, score: f64, capsule_bytes: u64, pinned: bool) -> RetrievedItem {
    RetrievedItem {
        item_id: id.to_string(),
        memory_class: "episodic".to_string(),
        item_type: "note".to_string(),
        summary: format!("summary {id}"),
        content: format!("content {id}"),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: 0.9,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: format!("artifact-{id}"),
            hash: None,
            selector: Some(format!("#{id}")),
            created_at: None,
            classification: None,
        }],
        score,
        score_breakdown: BTreeMap::from([("similarity".to_string(), score)]),
        capsule_bytes,
        token_estimate: capsule_bytes as u32,
        pinned,
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}
