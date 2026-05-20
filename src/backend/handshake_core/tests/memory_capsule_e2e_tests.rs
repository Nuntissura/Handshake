use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::Utc;
use handshake_core::{
    kernel::action_envelope::{ApprovalPosture, AuthorityEffect},
    memory::{
        CapsuleAuditEntry, CapsuleBuilder, CapsuleFlightRecorderEvent, CapsulePolicyTable,
        CapsuleRecord, CapsuleRecorder, FemsError, FemsFlightRecorder, FemsFlightRecorderError,
        FemsRetriever, GetCapsuleRequest, InjectionDecision, KernelActionRejection,
        KernelActionSubmission, KernelActionSubmitter, ListRecentCapsulesRequest, MemoryCapsule,
        MemoryCapsuleIpcStore, MemoryIpcError, MemoryIpcService, ModelCallContext, RetrievedItem,
        SuppressItemRequest, TaskType, FR_EVT_CAPSULE_INJECTED, FR_EVT_CAPSULE_SUPPRESSED,
        MEMORY_CAPSULE_RECORD_ACTION_ID, MEMORY_CAPSULE_SUPPRESS_ACTION_ID,
    },
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

const QUERY: &str = "how do I add a new HBR rule applicability tag";
const ROLE_ID: &str = "KERNEL_BUILDER";
const SESSION_ID: &str = "KERNEL_BUILDER-20260519-091421";
const FIXTURE_RELATIVE_PATH: &str = "tests/fixtures/memory_capsule_e2e/sample_fems_items.json";

#[test]
fn capsule_builder_injector_recorder_and_ipc_compose_over_fixture() {
    let started_at = Instant::now();
    let fixture_items = load_fixture_items();
    let expected_included_ids = expected_included_item_ids(&fixture_items);
    let expected_policy =
        CapsulePolicyTable::default_policy_for(TaskType::KernelBuilderMtImplementation);
    assert!(
        fixture_items.len() > expected_policy.top_k as usize,
        "MT-147 fixture should contain more candidates than top_k so excluded audit entries are exercised"
    );

    let fems = TestFemsAdapter::new(fixture_items);
    let policy_table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &policy_table);

    let built_capsule = builder.build(build_context()).unwrap();
    assert_capsule_budget_discipline(&built_capsule);
    assert_fixture_selection_contract(&built_capsule, &expected_included_ids);
    assert_eq!(
        fems.calls(),
        vec![(QUERY.to_string(), expected_policy.top_k)]
    );

    let flight_recorder = RecordingFemsFlightRecorder::default();
    let injector = handshake_core::memory::CapsuleInjector::new(&builder, &flight_recorder);
    let decision = injector.inject_for_call(&model_call_context()).unwrap();
    let (capsule, capsule_handle) = match decision {
        InjectionDecision::Inject {
            capsule,
            capsule_handle,
        } => (capsule, capsule_handle),
        InjectionDecision::Skip { reason } => panic!("expected Inject, got Skip {reason:?}"),
    };
    assert_eq!(capsule_handle.capsule_id(), capsule.id);
    assert_capsule_budget_discipline(&capsule);
    assert_fixture_selection_contract(&capsule, &expected_included_ids);
    assert_eq!(
        fems.calls(),
        vec![
            (QUERY.to_string(), expected_policy.top_k),
            (QUERY.to_string(), expected_policy.top_k),
        ]
    );

    let events = flight_recorder.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id(), FR_EVT_CAPSULE_INJECTED);
    let injected = match &events[0] {
        CapsuleFlightRecorderEvent::CapsuleInjected(injected) => injected,
        other => panic!("expected injected event, got {other:?}"),
    };
    assert_eq!(injected.capsule_id, capsule.id);
    assert_eq!(injected.capsule_source_hash, capsule.source_hash);
    assert_eq!(injected.policy, capsule.policy);
    assert_eq!(injected.item_count, capsule.audit.entries.len());
    assert_eq!(
        injected.included_count,
        capsule
            .audit
            .entries
            .iter()
            .filter(|entry| entry.included)
            .count()
    );
    assert_eq!(
        injected.suppressed_count,
        capsule
            .audit
            .entries
            .iter()
            .filter(|entry| !entry.included)
            .count()
    );

    let action_catalog = CapturingSubmitter::default();
    let recorder = CapsuleRecorder {
        action_catalog: &action_catalog,
    };
    let record = CapsuleRecord::from_capsule(&capsule, Utc::now(), SESSION_ID, ROLE_ID);

    let receipt = recorder.record(record.clone()).unwrap();

    assert_eq!(receipt.record_id.get_version_num(), 7);
    assert_eq!(receipt.write_box_envelope_id.get_version_num(), 7);
    let submissions = action_catalog.submissions();
    assert_eq!(submissions.len(), 1);
    let record_submission = &submissions[0];
    assert_eq!(
        record_submission.request.action_id,
        MEMORY_CAPSULE_RECORD_ACTION_ID
    );
    assert_eq!(
        record_submission.request.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        record_submission.request.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert_eq!(
        record_submission.request.target_ids[0].target_id,
        capsule.id.to_string()
    );
    assert_eq!(record_submission.proposed_receipt, receipt);
    assert_eq!(
        record_submission.write_box_envelope.envelope_id,
        receipt.write_box_envelope_id
    );
    assert_eq!(
        record_submission.write_box_envelope.payload["record_id"],
        json!(receipt.record_id)
    );
    assert_eq!(
        record_submission.write_box_envelope.payload["record"]["capsule_id"],
        json!(record.capsule_id)
    );
    assert_eq!(
        record_submission.write_box_envelope.payload["record"]["audit_log"],
        json!(record.audit_log)
    );

    let store = InMemoryCapsuleStore::default();
    store.save_capsule_record(record.clone()).unwrap();
    let service = MemoryIpcService::new(&store, &action_catalog, &flight_recorder);

    let recent = service
        .list_recent(ListRecentCapsulesRequest { limit: 5 })
        .unwrap();
    assert_eq!(recent.capsules.len(), 1);
    assert_eq!(recent.capsules[0].capsule_id, record.capsule_id);
    assert_eq!(recent.capsules[0].task_type, record.task_type);
    assert_eq!(
        recent.capsules[0].included_count,
        record
            .audit_log
            .entries
            .iter()
            .filter(|entry| entry.included)
            .count()
    );

    let fetched = service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .unwrap();
    assert_eq!(fetched.record, record);

    let included_item_id = record
        .audit_log
        .entries
        .iter()
        .find(|entry| entry.included)
        .map(|entry| entry.item_id.clone())
        .expect("fixture should produce at least one included capsule item");

    let suppression = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: included_item_id.clone(),
            reason: "operator rejected stale HBR applicability-tag context".to_string(),
            actor_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
        })
        .unwrap();
    assert_eq!(suppression.capsule_id, record.capsule_id);
    assert_eq!(suppression.suppression_id.get_version_num(), 7);
    assert_eq!(suppression.write_box_envelope_id.get_version_num(), 7);
    assert_eq!(suppression.suppressed_item_count, 1);
    assert_eq!(
        suppression.suppressed_item_ids,
        vec![included_item_id.clone()]
    );
    assert_eq!(
        suppression.flight_recorder_event_id,
        FR_EVT_CAPSULE_SUPPRESSED
    );

    let submissions = action_catalog.submissions();
    assert_eq!(submissions.len(), 2);
    assert_eq!(
        submissions[1].request.action_id,
        MEMORY_CAPSULE_SUPPRESS_ACTION_ID
    );
    assert_eq!(
        submissions[1].request.target_ids[0].target_id,
        record.capsule_id.to_string()
    );

    let events = flight_recorder.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_id(), FR_EVT_CAPSULE_SUPPRESSED);
    let suppressed = match &events[1] {
        CapsuleFlightRecorderEvent::CapsuleSuppressed(suppressed) => suppressed,
        other => panic!("expected suppression event, got {other:?}"),
    };
    assert_eq!(suppressed.capsule_id, record.capsule_id);
    assert_eq!(
        suppressed.reason,
        "operator rejected stale HBR applicability-tag context"
    );

    let updated = service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .unwrap();
    let suppressed_entry = updated
        .record
        .audit_log
        .entry(&included_item_id)
        .expect("suppressed item should remain visible in audit log");
    assert!(!suppressed_entry.included);
    assert_eq!(
        suppressed_entry.suppression_reason.as_deref(),
        Some("operator rejected stale HBR applicability-tag context")
    );

    let elapsed = started_at.elapsed();
    if elapsed > Duration::from_millis(500) {
        eprintln!("warning: MT-147 memory capsule e2e took {elapsed:?}, above 500ms");
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

    fn calls(&self) -> Vec<(String, u32)> {
        self.calls.borrow().clone()
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

impl RecordingFemsFlightRecorder {
    fn events(&self) -> Vec<CapsuleFlightRecorderEvent> {
        self.events.borrow().clone()
    }
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

#[derive(Default)]
struct CapturingSubmitter {
    submissions: RefCell<Vec<KernelActionSubmission>>,
}

impl CapturingSubmitter {
    fn submissions(&self) -> Vec<KernelActionSubmission> {
        self.submissions.borrow().clone()
    }
}

impl KernelActionSubmitter for CapturingSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        self.submissions.borrow_mut().push(submission);
        Ok(())
    }
}

#[derive(Default)]
struct InMemoryCapsuleStore {
    records: RefCell<BTreeMap<Uuid, CapsuleRecord>>,
}

impl MemoryCapsuleIpcStore for InMemoryCapsuleStore {
    fn all_capsule_records(&self) -> Result<Vec<CapsuleRecord>, MemoryIpcError> {
        Ok(self.records.borrow().values().cloned().collect())
    }

    fn get_capsule_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<Option<CapsuleRecord>, MemoryIpcError> {
        Ok(self.records.borrow().get(&capsule_id).cloned())
    }

    fn save_capsule_record(&self, record: CapsuleRecord) -> Result<(), MemoryIpcError> {
        self.records.borrow_mut().insert(record.capsule_id, record);
        Ok(())
    }
}

fn build_context() -> handshake_core::memory::BuildContext {
    handshake_core::memory::BuildContext {
        task_type: TaskType::KernelBuilderMtImplementation,
        query: QUERY.to_string(),
        role_id: ROLE_ID.to_string(),
        session_id: SESSION_ID.to_string(),
        override_policy: None,
    }
}

fn model_call_context() -> ModelCallContext {
    ModelCallContext::eligible(
        TaskType::KernelBuilderMtImplementation,
        QUERY,
        ROLE_ID,
        SESSION_ID,
    )
}

fn assert_capsule_budget_discipline(capsule: &MemoryCapsule) {
    assert_eq!(capsule.task_type, TaskType::KernelBuilderMtImplementation);
    assert_eq!(
        capsule.policy.task_type,
        TaskType::KernelBuilderMtImplementation
    );
    assert!(
        capsule.pack.items.len() <= capsule.policy.top_k as usize,
        "capsule included {} items, above top_k {}",
        capsule.pack.items.len(),
        capsule.policy.top_k
    );
    assert!(!capsule.pack.items.is_empty());
    assert!(capsule.audit.entries.iter().any(|entry| entry.included));
    assert!(capsule.audit.entries.iter().any(|entry| !entry.included));
    assert_eq!(
        capsule
            .audit
            .entries
            .iter()
            .filter(|entry| entry.included)
            .count(),
        capsule.pack.items.len()
    );

    let included_bytes = capsule
        .audit
        .entries
        .iter()
        .filter(|entry| entry.included)
        .map(audit_entry_capsule_bytes)
        .sum::<u64>();
    assert!(
        included_bytes <= capsule.policy.capsule_budget_bytes,
        "capsule included {included_bytes} bytes, above budget {}",
        capsule.policy.capsule_budget_bytes
    );

    for item in &capsule.pack.items {
        assert!(
            capsule
                .audit
                .entry(&item.memory_id)
                .map(|entry| entry.included)
                .unwrap_or(false),
            "included pack item {} must have an included audit entry",
            item.memory_id
        );
    }

    for entry in &capsule.audit.entries {
        if entry.included {
            assert!(entry.suppression_reason.is_none());
        } else {
            assert_eq!(entry.suppression_reason.as_deref(), Some("budget"));
        }
        assert!(entry.score_breakdown.contains_key("capsule_bytes"));
    }
}

fn audit_entry_capsule_bytes(entry: &CapsuleAuditEntry) -> u64 {
    entry
        .score_breakdown
        .get("capsule_bytes")
        .copied()
        .unwrap_or(0.0)
        .max(0.0) as u64
}

fn load_fixture_items() -> Vec<RetrievedItem> {
    let fixture_path = fixture_path();
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
    assert_fixture_file_contract(&fixture);
    fixture.items
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Path::new(FIXTURE_RELATIVE_PATH))
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    schema_version: String,
    fixture_id: String,
    wp_id: String,
    mt_id: String,
    intended_task_type: String,
    items: Vec<RetrievedItem>,
}

fn assert_fixture_file_contract(fixture: &FixtureFile) {
    assert_eq!(fixture.schema_version, "sample_fems_items.v1");
    assert_eq!(
        fixture.fixture_id,
        "mt-147-memory-capsule-e2e-sample-fems-items"
    );
    assert_eq!(fixture.wp_id, "WP-KERNEL-004");
    assert_eq!(fixture.mt_id, "MT-147");
    assert_eq!(
        fixture.intended_task_type,
        "kernel_builder_mt_implementation"
    );
    assert_eq!(fixture.items.len(), 30);
    assert_eq!(fixture.items.iter().filter(|item| item.pinned).count(), 2);

    let mut seen_ids = BTreeSet::new();
    let mut selection_counts = BTreeMap::<String, usize>::new();

    for item in &fixture.items {
        assert!(
            seen_ids.insert(item.item_id.clone()),
            "duplicate fixture item"
        );
        assert!(!item.memory_class.trim().is_empty());
        assert!(!item.item_type.trim().is_empty());
        assert!(!item.summary.trim().is_empty());
        assert!(!item.content.trim().is_empty());
        assert!(!item.trust_level.trim().is_empty());
        assert!(item.confidence.is_finite());
        assert!(item.score.is_finite());
        assert!(item.capsule_bytes > 0);
        assert!(item.token_estimate > 0);
        assert!(!item.scope_refs.is_empty());
        assert!(!item.source_refs.is_empty());
        assert!(!item.score_breakdown.is_empty());
        for value in item.score_breakdown.values() {
            assert!(value.is_finite());
        }

        let selection = expected_selection(item);
        *selection_counts.entry(selection.to_string()).or_default() += 1;
    }

    assert_eq!(selection_counts.get("pinned_include"), Some(&2));
    assert_eq!(selection_counts.get("score_include"), Some(&10));
    assert_eq!(selection_counts.get("budget_drop_candidate"), Some(&2));
    assert_eq!(selection_counts.get("top_k_drop_candidate"), Some(&16));
}

fn expected_included_item_ids(items: &[RetrievedItem]) -> BTreeSet<String> {
    items
        .iter()
        .filter(|item| matches!(expected_selection(item), "pinned_include" | "score_include"))
        .map(|item| item.item_id.clone())
        .collect()
}

fn assert_fixture_selection_contract(
    capsule: &MemoryCapsule,
    expected_included_ids: &BTreeSet<String>,
) {
    let actual_included_ids = capsule
        .audit
        .entries
        .iter()
        .filter(|entry| entry.included)
        .map(|entry| entry.item_id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(&actual_included_ids, expected_included_ids);

    for entry in &capsule.audit.entries {
        if expected_included_ids.contains(&entry.item_id) {
            assert!(entry.included);
            assert_eq!(entry.suppression_reason, None);
        } else {
            assert!(!entry.included);
            assert_eq!(entry.suppression_reason.as_deref(), Some("budget"));
        }
    }
}

fn expected_selection(item: &RetrievedItem) -> &str {
    item.structured
        .as_ref()
        .and_then(|value| value.get("expected_selection"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            panic!(
                "fixture item {} must declare expected_selection",
                item.item_id
            )
        })
}
