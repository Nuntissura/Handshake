use std::{cell::RefCell, collections::BTreeMap};

use chrono::{DateTime, Utc};
use handshake_core::{
    ace::{
        FemsSourceRef, FemsSourceRefKind, MemoryPack, MemoryPackBudgets, MemoryPackDeterminismMode,
        MemoryPackItem, MemoryPolicy,
    },
    kernel::{
        action_envelope::{ApprovalPosture, AuthorityEffect},
        write_boxes::{WriteBoxKind, WriteBoxLifecycleState, WriteBoxValidationState},
    },
    memory::{
        CapsuleAuditEntry, CapsuleAuditLog, CapsuleRecord, CapsuleRecorder, DegradationTier,
        KernelActionRejection, KernelActionSubmission, KernelActionSubmitter,
        MEMORY_CAPSULE_RECORD_ACTION_ID, MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID,
        MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID, MemoryCapsule, RecorderError, RetrievalPolicy,
        TaskType, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
    },
};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Default)]
struct CapturingSubmitter {
    submissions: RefCell<Vec<KernelActionSubmission>>,
    rejection: Option<KernelActionRejection>,
}

impl CapturingSubmitter {
    fn rejecting(code: &str, reason: &str) -> Self {
        Self {
            submissions: RefCell::new(Vec::new()),
            rejection: Some(KernelActionRejection {
                code: code.to_string(),
                reason: reason.to_string(),
            }),
        }
    }
}

impl KernelActionSubmitter for CapturingSubmitter {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        self.submissions.borrow_mut().push(submission);
        if let Some(rejection) = &self.rejection {
            return Err(rejection.clone());
        }
        Ok(())
    }
}

#[test]
fn recorder_submits_well_formed_catalog_action_and_memory_write_box_payload() {
    let submitter = CapturingSubmitter::default();
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = sample_record();

    let receipt = recorder.record(record.clone()).unwrap();

    assert_eq!(receipt.record_id.get_version_num(), 7);
    assert_eq!(receipt.write_box_envelope_id.get_version_num(), 7);
    assert!(receipt.persisted_at_utc >= record.recorded_at_utc);

    let submissions = submitter.submissions.borrow();
    assert_eq!(submissions.len(), 1);
    let submission = &submissions[0];

    assert_eq!(
        submission.request.action_id,
        MEMORY_CAPSULE_RECORD_ACTION_ID
    );
    assert_eq!(
        submission.request.input_schema_id,
        MEMORY_CAPSULE_RECORD_INPUT_SCHEMA_ID
    );
    assert_eq!(
        submission.request.expected_write_boxes[0].write_box_kind,
        "MemoryBox"
    );
    assert_eq!(
        submission.request.expected_write_boxes[0].write_box_schema_id,
        "hsk.write_box.memory@1"
    );
    assert_eq!(
        submission.request.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        submission.request.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );

    assert_eq!(
        submission.write_box_envelope.schema_id,
        WRITE_BOX_V1_ENVELOPE_SCHEMA_ID
    );
    assert_eq!(
        submission.write_box_envelope.envelope_id,
        receipt.write_box_envelope_id
    );
    assert_eq!(
        submission.write_box_envelope.payload_schema_id,
        MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID
    );
    assert_eq!(
        submission.write_box_envelope.write_box.common.kind,
        WriteBoxKind::Memory
    );
    assert_eq!(
        submission
            .write_box_envelope
            .write_box
            .common
            .schema_version,
        "hsk.write_box.memory@1"
    );
    assert_eq!(
        submission
            .write_box_envelope
            .write_box
            .common
            .lifecycle_state,
        WriteBoxLifecycleState::Open
    );
    assert_eq!(
        submission
            .write_box_envelope
            .write_box
            .common
            .validation_status
            .state,
        WriteBoxValidationState::Pending
    );
    assert_eq!(
        submission.write_box_envelope.write_box.memory_extract_ref,
        format!("memory-capsule-record://{}", receipt.record_id)
    );

    assert_eq!(
        submission.write_box_envelope.payload["schema_id"],
        json!(MEMORY_CAPSULE_RECORD_PAYLOAD_SCHEMA_ID)
    );
    assert_eq!(
        submission.write_box_envelope.payload["record_id"],
        json!(receipt.record_id)
    );
    assert_eq!(
        submission.write_box_envelope.payload["record"]["capsule_id"],
        json!(record.capsule_id)
    );
    assert_eq!(
        submission.write_box_envelope.payload["record"]["outcome"],
        Value::Null
    );
}

#[test]
fn record_receipt_round_trips_through_json() {
    let submitter = CapturingSubmitter::default();
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let receipt = recorder.record(sample_record()).unwrap();
    let encoded = serde_json::to_string(&receipt).unwrap();
    let decoded: handshake_core::memory::RecordReceipt = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, receipt);
}

#[test]
fn action_catalog_rejection_is_typed_and_does_not_submit_twice() {
    let submitter = CapturingSubmitter::rejecting("write_box_denied", "dedup rejected capsule");
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let error = recorder.record(sample_record()).unwrap_err();

    assert!(matches!(
        error,
        RecorderError::Rejected(KernelActionRejection { code, reason })
            if code == "write_box_denied" && reason == "dedup rejected capsule"
    ));
    assert_eq!(submitter.submissions.borrow().len(), 1);
}

#[test]
fn capsule_record_from_capsule_defaults_outcome_to_none() {
    let capsule = sample_capsule("DO_NOT_PERSIST_FULL_PACK_CONTENT");
    let recorded_at_utc = Utc::now();

    let record = CapsuleRecord::from_capsule(
        &capsule,
        recorded_at_utc,
        "session-from-capsule",
        "KERNEL_BUILDER",
    );

    assert_eq!(record.capsule_id, capsule.id);
    assert_eq!(record.capsule_source_hash, capsule.source_hash);
    assert_eq!(record.audit_log, capsule.audit);
    assert_eq!(record.outcome, None);
}

#[test]
fn recorder_does_not_persist_full_memory_pack_contents() {
    let submitter = CapturingSubmitter::default();
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let capsule = sample_capsule("DO_NOT_PERSIST_FULL_PACK_CONTENT");
    let recorded_at_utc = Utc::now();
    let record = CapsuleRecord::from_capsule(
        &capsule,
        recorded_at_utc,
        "session-no-full-content",
        "KERNEL_BUILDER",
    );

    recorder.record(record).unwrap();

    let submissions = submitter.submissions.borrow();
    let payload = serde_json::to_string(&submissions[0].write_box_envelope.payload).unwrap();
    assert!(!payload.contains("pack"));
    assert!(!payload.contains("items"));
    assert!(!payload.contains("DO_NOT_PERSIST_FULL_PACK_CONTENT"));
}

#[test]
fn invalid_record_shape_is_typed_before_submit() {
    let submitter = CapturingSubmitter::default();
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let mut record = sample_record();
    record.session_id = "   ".to_string();

    let error = recorder.record(record).unwrap_err();

    assert!(matches!(
        error,
        RecorderError::InvalidRecordShape {
            field: "session_id",
            ..
        }
    ));
    assert!(submitter.submissions.borrow().is_empty());
}

#[test]
fn persistence_source_has_no_db_sqlite_or_inspector_mutation_path() {
    let source = include_str!("../src/memory/persistence.rs");

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
            "persistence.rs must not contain forbidden direct path {forbidden}"
        );
    }
}

fn sample_record() -> CapsuleRecord {
    CapsuleRecord {
        capsule_id: Uuid::parse_str("018f35f2-79b0-7cc3-98c4-dc0c0c0c0c0c").unwrap(),
        capsule_source_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_string(),
        task_type: TaskType::KernelBuilderMtImplementation,
        policy: policy(),
        audit_log: CapsuleAuditLog {
            entries: vec![CapsuleAuditEntry {
                item_id: "item-1".to_string(),
                source_uri: "fems://source/artifact/artifact-1#item-1".to_string(),
                included: true,
                suppression_reason: None,
                score: 0.92,
                score_breakdown: BTreeMap::from([("similarity".to_string(), 0.92)]),
                pinned: false,
            }],
        },
        built_at_utc: dt("2024-05-19T10:00:00Z"),
        recorded_at_utc: dt("2024-05-19T10:05:00Z"),
        session_id: "session-145".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        outcome: None,
    }
}

fn sample_capsule(content: &str) -> MemoryCapsule {
    let mut capsule = MemoryCapsule::new(
        TaskType::KernelBuilderMtImplementation,
        MemoryPack {
            schema_version: "memory_pack.v1".to_string(),
            pack_id: "pack-persistence".to_string(),
            generated_at: "2026-05-19T10:00:00Z".to_string(),
            determinism_mode: MemoryPackDeterminismMode::Strict,
            memory_policy: MemoryPolicy::WorkspaceScoped,
            scope_refs: Vec::new(),
            budgets: MemoryPackBudgets {
                max_tokens: 128,
                max_items: 1,
                max_items_per_type: BTreeMap::new(),
            },
            items: vec![MemoryPackItem {
                memory_id: "item-secret-content".to_string(),
                memory_class: "episodic".to_string(),
                item_type: "note".to_string(),
                summary: "summary".to_string(),
                content: content.to_string(),
                structured: Some(json!({ "secret": content })),
                trust_level: "trusted".to_string(),
                confidence: 0.9,
                scope_refs: Vec::new(),
                source_refs: vec![FemsSourceRef {
                    kind: FemsSourceRefKind::Artifact,
                    id: "artifact-secret".to_string(),
                    hash: None,
                    selector: Some("#secret".to_string()),
                    created_at: None,
                    classification: None,
                }],
                pinned: false,
                last_verified_at: None,
            }],
            token_estimate: 64,
            memory_pack_hash: String::new(),
            warnings: Vec::new(),
        },
        policy(),
    )
    .unwrap();
    capsule.audit = CapsuleAuditLog {
        entries: vec![CapsuleAuditEntry {
            item_id: "item-secret-content".to_string(),
            source_uri: "fems://source/artifact/artifact-secret#secret".to_string(),
            included: true,
            suppression_reason: None,
            score: 0.9,
            score_breakdown: BTreeMap::from([("similarity".to_string(), 0.9)]),
            pinned: false,
        }],
    };
    capsule
}

fn policy() -> RetrievalPolicy {
    RetrievalPolicy {
        top_k: 12,
        capsule_budget_bytes: 65_536,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: "retrieval_scoring_formula_v0".to_string(),
        graceful_degradation_tier: DegradationTier::Tiered,
    }
}

fn dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
