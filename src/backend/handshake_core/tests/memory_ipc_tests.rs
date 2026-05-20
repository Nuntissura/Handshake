use std::{cell::RefCell, collections::BTreeMap};

use chrono::{DateTime, Utc};
use handshake_core::memory::{
    ipc::{
        GetCapsuleRequest, ListRecentCapsulesRequest, MemoryCapsuleIpcStore, MemoryIpcError,
        MemoryIpcService, SuppressCapsuleRequest, SuppressItemRequest,
        MEMORY_CAPSULE_SUPPRESS_ACTION_ID,
    },
    CapsuleAuditEntry, CapsuleAuditLog, CapsuleFlightRecorderEvent, CapsuleRecord, DegradationTier,
    FemsFlightRecorder, FemsFlightRecorderError, KernelActionRejection, KernelActionSubmission,
    KernelActionSubmitter, RetrievalPolicy, TaskType, FR_EVT_CAPSULE_SUPPRESSED,
    RETRIEVAL_SCORING_FORMULA_V0,
};
use uuid::Uuid;

#[derive(Default)]
struct InMemoryCapsuleStore {
    records: RefCell<BTreeMap<Uuid, CapsuleRecord>>,
}

impl InMemoryCapsuleStore {
    fn with_records(records: Vec<CapsuleRecord>) -> Self {
        Self {
            records: RefCell::new(
                records
                    .into_iter()
                    .map(|record| (record.capsule_id, record))
                    .collect(),
            ),
        }
    }

    fn stored(&self, capsule_id: Uuid) -> CapsuleRecord {
        self.records
            .borrow()
            .get(&capsule_id)
            .cloned()
            .unwrap_or_else(|| panic!("missing stored capsule {capsule_id}"))
    }
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

#[test]
fn list_recent_orders_by_built_at_desc_and_applies_limit() {
    let older = sample_record(
        uuid(1),
        "2026-05-19T09:00:00Z",
        vec![audit_entry("older-item", true, None)],
    );
    let newest = sample_record(
        uuid(2),
        "2026-05-19T11:00:00Z",
        vec![audit_entry("newest-item", true, None)],
    );
    let middle = sample_record(
        uuid(3),
        "2026-05-19T10:00:00Z",
        vec![audit_entry("middle-item", true, None)],
    );
    let store = InMemoryCapsuleStore::with_records(vec![older, newest.clone(), middle.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    let response = service
        .list_recent(ListRecentCapsulesRequest { limit: 2 })
        .unwrap();

    assert_eq!(
        response
            .capsules
            .iter()
            .map(|capsule| capsule.capsule_id)
            .collect::<Vec<_>>(),
        vec![newest.capsule_id, middle.capsule_id]
    );
    assert!(catalog.submissions().is_empty());
    assert!(fems.events().is_empty());
}

#[test]
fn get_returns_full_capsule_record_with_audit_log() {
    let expected = sample_record(
        uuid(10),
        "2026-05-19T12:00:00Z",
        vec![
            audit_entry("included-item", true, None),
            audit_entry("suppressed-item", false, Some("already stale")),
        ],
    );
    let store = InMemoryCapsuleStore::with_records(vec![expected.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    let response = service
        .get(GetCapsuleRequest {
            capsule_id: expected.capsule_id,
        })
        .unwrap();

    assert_eq!(response.record, expected);
    assert_eq!(response.record.audit_log.entries.len(), 2);
    assert_eq!(
        response
            .record
            .audit_log
            .entry("suppressed-item")
            .unwrap()
            .suppression_reason
            .as_deref(),
        Some("already stale")
    );
    assert!(catalog.submissions().is_empty());
    assert!(fems.events().is_empty());
}

#[test]
fn suppress_item_updates_only_that_items_included_flag_and_reason() {
    let record = sample_record(
        uuid(20),
        "2026-05-19T13:00:00Z",
        vec![
            audit_entry("suppress-me", true, None),
            audit_entry("keep-me", true, None),
        ],
    );
    let untouched_entry = record.audit_log.entry("keep-me").unwrap().clone();
    let store = InMemoryCapsuleStore::with_records(vec![record.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    let receipt = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: "suppress-me".to_string(),
            reason: "operator rejected stale item".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap();

    assert_eq!(receipt.capsule_id, record.capsule_id);
    assert_eq!(receipt.suppressed_item_count, 1);

    let updated = store.stored(record.capsule_id);
    let suppressed = updated.audit_log.entry("suppress-me").unwrap();
    assert!(!suppressed.included);
    assert_eq!(
        suppressed.suppression_reason.as_deref(),
        Some("operator rejected stale item")
    );
    assert_eq!(updated.audit_log.entry("keep-me"), Some(&untouched_entry));
}

#[test]
fn suppress_capsule_marks_all_entries_suppressed() {
    let record = sample_record(
        uuid(30),
        "2026-05-19T14:00:00Z",
        vec![
            audit_entry("first", true, None),
            audit_entry("second", false, Some("previously filtered")),
            audit_entry("third", true, None),
        ],
    );
    let store = InMemoryCapsuleStore::with_records(vec![record.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    let receipt = service
        .suppress_capsule(SuppressCapsuleRequest {
            capsule_id: record.capsule_id,
            reason: "operator rejected whole capsule".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap();

    assert_eq!(receipt.capsule_id, record.capsule_id);
    assert_eq!(receipt.suppressed_item_count, 3);

    let updated = store.stored(record.capsule_id);
    assert!(updated
        .audit_log
        .entries
        .iter()
        .all(|entry| !entry.included));
    assert!(updated.audit_log.entries.iter().all(|entry| {
        entry.suppression_reason.as_deref() == Some("operator rejected whole capsule")
    }));
}

#[test]
fn suppress_unknown_capsule_returns_clean_not_found_error_without_side_effects() {
    let missing_capsule_id = uuid(404);
    let store = InMemoryCapsuleStore::default();
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    let error = service
        .suppress_capsule(SuppressCapsuleRequest {
            capsule_id: missing_capsule_id,
            reason: "operator rejected missing capsule".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        MemoryIpcError::CapsuleNotFound { capsule_id } if capsule_id == missing_capsule_id
    ));
    assert!(catalog.submissions().is_empty());
    assert!(fems.events().is_empty());
}

#[test]
fn suppression_does_not_bypass_catalog_and_records_every_suppression() {
    let first = sample_record(
        uuid(50),
        "2026-05-19T15:00:00Z",
        vec![audit_entry("first-item", true, None)],
    );
    let second = sample_record(
        uuid(51),
        "2026-05-19T16:00:00Z",
        vec![audit_entry("second-item", true, None)],
    );
    let store = InMemoryCapsuleStore::with_records(vec![first.clone(), second.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    service
        .suppress_item(SuppressItemRequest {
            capsule_id: first.capsule_id,
            item_id: "first-item".to_string(),
            reason: "operator rejected item".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap();
    service
        .suppress_capsule(SuppressCapsuleRequest {
            capsule_id: second.capsule_id,
            reason: "operator rejected capsule".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap();

    let submissions = catalog.submissions();
    assert_eq!(submissions.len(), 2);
    assert!(submissions
        .iter()
        .all(|submission| submission.request.action_id == MEMORY_CAPSULE_SUPPRESS_ACTION_ID));
    assert_eq!(
        submissions
            .iter()
            .map(|submission| submission.request.target_ids[0].target_id.as_str())
            .collect::<Vec<_>>(),
        vec![first.capsule_id.to_string(), second.capsule_id.to_string()]
    );
}

#[test]
fn suppression_emits_capsule_suppressed_event_through_fems_recorder() {
    let record = sample_record(
        uuid(60),
        "2026-05-19T17:00:00Z",
        vec![audit_entry("event-item", true, None)],
    );
    let store = InMemoryCapsuleStore::with_records(vec![record.clone()]);
    let catalog = CapturingSubmitter::default();
    let fems = RecordingFemsFlightRecorder::default();
    let service = MemoryIpcService::new(&store, &catalog, &fems);

    service
        .suppress_capsule(SuppressCapsuleRequest {
            capsule_id: record.capsule_id,
            reason: "operator rejected capsule for retry".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-ipc".to_string(),
        })
        .unwrap();

    let events = fems.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id(), FR_EVT_CAPSULE_SUPPRESSED);
    let suppressed = match &events[0] {
        CapsuleFlightRecorderEvent::CapsuleSuppressed(suppressed) => suppressed,
        other => panic!("expected suppression event, got {other:?}"),
    };
    assert_eq!(suppressed.capsule_id, record.capsule_id);
    assert_eq!(suppressed.reason, "operator rejected capsule for retry");
}

#[test]
fn memory_ipc_source_has_no_direct_db_sqlite_or_llm_path() {
    let source = include_str!("../src/memory/ipc.rs");

    for forbidden in [
        "sqlx",
        "sqlite",
        "rusqlite",
        "Connection",
        "Database",
        "crate::storage",
        "inspector_read",
        "ReplayDrive",
        "reqwest",
        "LlmClient",
    ] {
        assert!(
            !source.contains(forbidden),
            "ipc.rs must not contain forbidden direct path {forbidden}"
        );
    }
}

fn sample_record(
    capsule_id: Uuid,
    recorded_at_utc: &str,
    entries: Vec<CapsuleAuditEntry>,
) -> CapsuleRecord {
    CapsuleRecord {
        capsule_id,
        capsule_source_hash: format!("{:064x}", capsule_id.as_u128()),
        task_type: TaskType::KernelBuilderMtImplementation,
        policy: policy(),
        audit_log: CapsuleAuditLog { entries },
        built_at_utc: dt(recorded_at_utc),
        recorded_at_utc: dt(recorded_at_utc),
        session_id: "session-ipc".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        outcome: None,
    }
}

fn audit_entry(
    item_id: &str,
    included: bool,
    suppression_reason: Option<&str>,
) -> CapsuleAuditEntry {
    CapsuleAuditEntry {
        item_id: item_id.to_string(),
        source_uri: format!("fems://source/artifact/artifact-{item_id}#{item_id}"),
        included,
        suppression_reason: suppression_reason.map(str::to_string),
        score: 0.91,
        score_breakdown: BTreeMap::from([("similarity".to_string(), 0.91)]),
        pinned: false,
    }
}

fn policy() -> RetrievalPolicy {
    RetrievalPolicy {
        top_k: 12,
        capsule_budget_bytes: 65_536,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Tiered,
    }
}

fn dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
