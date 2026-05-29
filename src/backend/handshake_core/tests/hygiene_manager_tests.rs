use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use handshake_core::kernel::{
    action_catalog::kernel002_action_catalog,
    action_envelope::{ApprovalPosture, AuthorityEffect},
};
use handshake_core::memory::hygiene::{
    FemsAccessor, HygieneActionSubmitter, HygieneConfig, HygieneError, HygieneItemView,
    HygieneJobRunner, HygieneTask, ProceduralPromotion, HYGIENE_CONSOLIDATION_ACTION_ID,
    HYGIENE_FLAG_ACTION_ID, HYGIENE_PROMOTE_ACTION_ID, HYGIENE_PRUNE_ACTION_ID,
};
use handshake_core::memory::{
    pinned_core::MEMORY_PIN_AGGREGATE_TYPE, PostgresKernelActionSubmitter,
};
use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};
use uuid::Uuid;

#[derive(Default)]
struct StubFemsAccessor {
    items: Vec<HygieneItemView>,
    list_calls: AtomicUsize,
    invalidated: Mutex<Vec<Uuid>>,
}

impl StubFemsAccessor {
    fn with_items(items: Vec<HygieneItemView>) -> Self {
        Self {
            items,
            list_calls: AtomicUsize::new(0),
            invalidated: Mutex::new(Vec::new()),
        }
    }
}

impl FemsAccessor for StubFemsAccessor {
    fn list_items(&self) -> Result<Vec<HygieneItemView>, HygieneError> {
        self.list_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.items.clone())
    }

    fn invalidate(&self, memory_id: Uuid, _at: DateTime<Utc>) -> Result<(), HygieneError> {
        self.invalidated.lock().unwrap().push(memory_id);
        Ok(())
    }
}

#[derive(Default)]
struct StubSubmitter {
    consolidations: Mutex<Vec<(Uuid, Uuid)>>,
    prunes: Mutex<Vec<Uuid>>,
    flags: Mutex<Vec<(Uuid, Uuid)>>,
    promotions: Mutex<Vec<ProceduralPromotion>>,
}

impl HygieneActionSubmitter for StubSubmitter {
    fn submit_consolidation_candidate(
        &self,
        left: Uuid,
        right: Uuid,
    ) -> Result<Uuid, HygieneError> {
        self.consolidations.lock().unwrap().push((left, right));
        Ok(Uuid::now_v7())
    }

    fn submit_prune(&self, memory_id: Uuid, _at: DateTime<Utc>) -> Result<Uuid, HygieneError> {
        self.prunes.lock().unwrap().push(memory_id);
        Ok(Uuid::now_v7())
    }

    fn submit_contradiction_flag(&self, left: Uuid, right: Uuid) -> Result<Uuid, HygieneError> {
        self.flags.lock().unwrap().push((left, right));
        Ok(Uuid::now_v7())
    }

    fn submit_procedural_promotion(
        &self,
        candidate: ProceduralPromotion,
    ) -> Result<Uuid, HygieneError> {
        self.promotions.lock().unwrap().push(candidate);
        Ok(Uuid::now_v7())
    }
}

fn item(id: u128, fp: u64, pinned: bool, use_count: u32, pass_count: u32) -> HygieneItemView {
    HygieneItemView {
        memory_id: Uuid::from_u128(id),
        recorded_at: Utc::now() - chrono::Duration::days(30),
        score: 0.1,
        use_count,
        pass_count,
        pinned,
        content_fingerprint: fp,
        embedding: None,
    }
}

#[test]
fn kernel_action_catalog_registers_all_hygiene_actions_as_review_gated() {
    let catalog = kernel002_action_catalog();

    for action_id in [
        HYGIENE_CONSOLIDATION_ACTION_ID,
        HYGIENE_PRUNE_ACTION_ID,
        HYGIENE_FLAG_ACTION_ID,
        HYGIENE_PROMOTE_ACTION_ID,
    ] {
        let action = catalog
            .action(action_id)
            .unwrap_or_else(|| panic!("missing hygiene catalog action {action_id}"));
        assert_eq!(
            action.authority_effect,
            AuthorityEffect::PrePromotionEvidenceOnly
        );
        assert_eq!(
            action.approval_posture,
            ApprovalPosture::RequiresPromotionGate
        );
        assert!(
            action
                .validation_hooks
                .iter()
                .any(|hook| hook.hook_id == "write_box_review_gate"),
            "hygiene action {action_id} must be review-gated"
        );
        assert!(
            action
                .expected_write_boxes
                .iter()
                .any(|write_box| write_box.write_box_kind == "MemoryBox"),
            "hygiene action {action_id} must produce MemoryBox evidence"
        );
    }
}

#[test]
fn flag_and_promote_skip_pinned_items() {
    let fems =
        StubFemsAccessor::with_items(vec![item(1, 7, true, 10, 9), item(2, 7, false, 10, 1)]);
    let submitter = StubSubmitter::default();
    let runner = HygieneJobRunner::new(&fems, &submitter);

    let report = runner
        .run_once(HygieneConfig {
            tasks: vec![
                HygieneTask::FlagContradictions,
                HygieneTask::PromoteProcedural {
                    min_use_count: 5,
                    min_pass_rate: 0.7,
                },
            ],
        })
        .expect("hygiene report");

    assert_eq!(report.tasks.len(), 2);
    assert_eq!(
        submitter.flags.lock().unwrap().len(),
        0,
        "contradiction flagging must not act on pinned pairs"
    );
    assert_eq!(
        submitter.promotions.lock().unwrap().len(),
        0,
        "procedural promotion must not act on pinned items"
    );
}

#[test]
fn invalid_thresholds_are_rejected_before_loading_fems_items() {
    let fems = StubFemsAccessor::default();
    let submitter = StubSubmitter::default();
    let runner = HygieneJobRunner::new(&fems, &submitter);

    assert!(
        runner
            .run_once(HygieneConfig {
                tasks: vec![HygieneTask::PruneStale {
                    older_than_secs: 60,
                    min_score: f64::NAN,
                }],
            })
            .is_err(),
        "NaN prune score threshold must be rejected"
    );
    assert_eq!(
        fems.list_calls.load(Ordering::SeqCst),
        0,
        "invalid config must fail before expensive FEMS enumeration"
    );

    assert!(
        runner
            .run_once(HygieneConfig {
                tasks: vec![HygieneTask::PromoteProcedural {
                    min_use_count: 5,
                    min_pass_rate: 1.1,
                }],
            })
            .is_err(),
        "procedural promotion pass-rate threshold must stay within 0..=1"
    );
}

#[test]
fn prune_emits_review_candidate_without_invalidating_until_approval() {
    let stale = Uuid::from_u128(11);
    let fems = StubFemsAccessor::with_items(vec![HygieneItemView {
        memory_id: stale,
        recorded_at: Utc::now() - chrono::Duration::days(30),
        score: 0.05,
        use_count: 1,
        pass_count: 0,
        pinned: false,
        content_fingerprint: 99,
        embedding: None,
    }]);
    let submitter = StubSubmitter::default();
    let runner = HygieneJobRunner::new(&fems, &submitter);

    let report = runner
        .run_once(HygieneConfig {
            tasks: vec![HygieneTask::PruneStale {
                older_than_secs: 60,
                min_score: 0.5,
            }],
        })
        .expect("hygiene report");

    assert_eq!(report.tasks[0].items_acted_on, 1);
    assert_eq!(submitter.prunes.lock().unwrap().as_slice(), &[stale]);
    assert!(
        fems.invalidated.lock().unwrap().is_empty(),
        "hygiene runner must not mutate bitemporal validity until the review-gated prune is approved"
    );
}

#[test]
fn consolidation_uses_embedding_similarity_before_fingerprint_fallback() {
    let fems = StubFemsAccessor::with_items(vec![
        HygieneItemView {
            memory_id: Uuid::from_u128(21),
            recorded_at: Utc::now(),
            score: 0.8,
            use_count: 1,
            pass_count: 1,
            pinned: false,
            content_fingerprint: 1,
            embedding: Some(vec![1.0, 0.0, 0.0]),
        },
        HygieneItemView {
            memory_id: Uuid::from_u128(22),
            recorded_at: Utc::now(),
            score: 0.8,
            use_count: 1,
            pass_count: 1,
            pinned: false,
            content_fingerprint: 2,
            embedding: Some(vec![0.999, 0.001, 0.0]),
        },
    ]);
    let submitter = StubSubmitter::default();
    let runner = HygieneJobRunner::new(&fems, &submitter);

    let report = runner
        .run_once(HygieneConfig {
            tasks: vec![HygieneTask::Consolidate { max_pairs: 10 }],
        })
        .expect("hygiene report");

    assert_eq!(report.tasks[0].items_acted_on, 1);
    assert_eq!(submitter.consolidations.lock().unwrap().len(), 1);
}

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: MT-160 hygiene manager tests require POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test hygiene_manager_tests -- --ignored`"]
async fn postgres_submitter_persists_hygiene_prune_candidate() {
    let db = postgres_or_environment_blocked().await;
    let submitter = PostgresKernelActionSubmitter::with_db(std::sync::Arc::clone(&db));
    let memory_id = Uuid::now_v7();

    let receipt_id = HygieneActionSubmitter::submit_prune(&submitter, memory_id, Utc::now())
        .expect("submit hygiene prune candidate");

    let events = db
        .list_kernel_events_for_aggregate(MEMORY_PIN_AGGREGATE_TYPE, &memory_id.to_string())
        .await
        .expect("list memory-item aggregate events");
    assert!(
        events.iter().any(|event| {
            event.payload["catalog_action_id"].as_str() == Some(HYGIENE_PRUNE_ACTION_ID)
                && event.payload["proposed_receipt"]["record_id"].as_str()
                    == Some(receipt_id.to_string().as_str())
        }),
        "PostgresKernelActionSubmitter must persist hygiene prune candidates through KernelActionCatalogV1"
    );
}
