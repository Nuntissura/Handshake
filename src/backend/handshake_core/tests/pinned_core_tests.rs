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
use handshake_core::memory::persistence_postgres::PostgresKernelActionSubmitter;
use handshake_core::memory::pinned_core::{
    FR_EVT_MEMORY_PIN, FR_EVT_MEMORY_UNPIN, PIN_MEMORY_ACTION_ID, PinError, PinIpcService,
    PinReceipt, PinSubmitter, PinnedBudget, PinnedCoreSelector, PinnedItem, SetPinRequest,
    UNPIN_MEMORY_ACTION_ID,
};
use handshake_core::memory::{
    BuildContext, BuilderError, CapsuleBuilder, CapsulePolicyTable, DegradationTier, FemsError,
    FemsRetriever, RETRIEVAL_SCORING_FORMULA_V0, RetrievalPolicy, RetrievedItem, TaskType,
};
use handshake_core::storage::{Database, StorageError, StorageResult, postgres::PostgresDatabase};
use sqlx::{Connection, PgPool, Row};
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
    assert!(
        pin.validation_hooks
            .iter()
            .any(|hook| hook.hook_id == "flight_recorder_event")
    );
    assert!(
        unpin
            .validation_hooks
            .iter()
            .any(|hook| hook.hook_id == "flight_recorder_event")
    );
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
fn pinned_migration_is_postgres_only_and_guarded_for_memory_item_table() {
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
fn pin_tauri_commands_are_registered_and_postgres_only() {
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
    assert!(memory_pin_rs.contains("PostgresKernelActionSubmitter"));
    assert!(memory_pin_rs.contains("memory_pin_postgres_unavailable"));
    assert!(!memory_pin_rs.contains("InMemory"));
}

#[test]
fn pin_postgres_submitter_uses_atomic_action_and_manifest_append() {
    let source =
        std::fs::read_to_string("src/memory/persistence_postgres.rs").expect("read persistence");
    assert!(source.contains("append_kernel_events_atomic"));
    assert!(source.contains("vec![action_event, manifest_event]"));
    assert!(source.contains("memory_pin_atomic_append_failed"));
    assert!(source.contains("existing_pin_submission_matches"));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires POSTGRES_TEST_URL and an isolated live Postgres schema"]
async fn postgres_pin_submitter_persists_pin_unpin_events_and_list_replays_latest_pin_state() {
    let (db, pool) = isolated_postgres().await.expect("isolated postgres");
    let submitter = PostgresKernelActionSubmitter::with_db(db);
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
    .expect("persist pin");

    assert_eq!(receipt.action_id, PIN_MEMORY_ACTION_ID);
    assert_eq!(receipt.fr_event_kind, FR_EVT_MEMORY_PIN);

    let pinned = PinSubmitter::list_pinned(&submitter).expect("list pinned items");
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].memory_id, memory_id);
    assert!(pinned[0].pinned);
    assert_eq!(pinned[0].actor_id, "KERNEL_BUILDER");
    assert_eq!(pinned[0].session_id, "session-159");

    let row = sqlx::query(
        r#"
        SELECT payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'memory_item'
          AND aggregate_id = $1
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(memory_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("pin event row");
    let payload: serde_json::Value = row.get("payload");

    assert_eq!(
        payload["catalog_action_id"].as_str(),
        Some(PIN_MEMORY_ACTION_ID)
    );
    assert_eq!(
        payload["write_box_envelope"]["payload"]["flight_recorder_event_id"].as_str(),
        Some(FR_EVT_MEMORY_PIN)
    );
    assert_eq!(
        payload["write_box_envelope"]["payload"]["pinned_item"]["memory_id"].as_str(),
        Some(memory_id.to_string().as_str())
    );
    assert_eq!(
        payload["request"]["actor"]["actor_id"].as_str(),
        Some("KERNEL_BUILDER")
    );
    assert_eq!(
        payload["request"]["session"]["session_id"].as_str(),
        Some("session-159")
    );

    let manifest_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE aggregate_type = 'memory_pin_manifest'
          AND aggregate_id = 'memory_pin_manifest_v1'
          AND payload->>'memory_id' = $1
        "#,
    )
    .bind(memory_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("pin manifest row count");
    assert_eq!(manifest_count, 1);

    let unpin = PinSubmitter::set_pin(
        &submitter,
        PinnedItem {
            memory_id,
            pinned: false,
            reason: "operator unpinned core memory".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-159".to_string(),
            set_at_utc: Utc::now(),
        },
    )
    .expect("persist unpin");
    assert_eq!(unpin.action_id, UNPIN_MEMORY_ACTION_ID);
    assert_eq!(unpin.fr_event_kind, FR_EVT_MEMORY_UNPIN);

    let pinned = PinSubmitter::list_pinned(&submitter).expect("list pinned after unpin");
    assert!(pinned.is_empty());

    let latest_action: String = sqlx::query_scalar(
        r#"
        SELECT payload->>'catalog_action_id'
        FROM kernel_event_ledger
        WHERE aggregate_type = 'memory_item'
          AND aggregate_id = $1
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(memory_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("latest pin action");
    assert_eq!(latest_action, UNPIN_MEMORY_ACTION_ID);
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

async fn isolated_postgres() -> StorageResult<(Arc<dyn Database>, PgPool)> {
    let url = std::env::var("POSTGRES_TEST_URL")
        .map_err(|_| StorageError::Validation("POSTGRES_TEST_URL not set for postgres tests"))?;
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("mt159_pinned_core_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    let pool = PgPool::connect(&schema_url).await?;
    Ok((db.into_arc(), pool))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}
