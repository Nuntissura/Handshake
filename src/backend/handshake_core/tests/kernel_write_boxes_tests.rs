use std::collections::HashSet;

use handshake_core::kernel::{
    action_envelope::AuthorityEffect,
    write_boxes::{
        kernel002_write_box_schema_family, validate_promotion_box, validate_write_box_common,
        validate_write_box_schema_family, ArtifactBox, CRDTWorkspaceBox, DraftBox, ExecutionBox,
        MemoryBox, MirrorAdvisoryBox, PatchBox, PromotionBox, ProposalBox, WriteBoxCommon,
        WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef, WriteBoxPayloadRef,
        WriteBoxReplayMetadataV1, WriteBoxTargetRef, WriteBoxValidationState,
        WriteBoxValidationStatus,
    },
};

#[test]
fn kernel002_write_box_family_defines_required_box_types() {
    let family = kernel002_write_box_schema_family();
    validate_write_box_schema_family(&family).expect("write-box family must validate");

    let kinds: HashSet<WriteBoxKind> = family.schemas.iter().map(|schema| schema.kind).collect();
    for kind in [
        WriteBoxKind::Draft,
        WriteBoxKind::CrdtWorkspace,
        WriteBoxKind::Proposal,
        WriteBoxKind::Patch,
        WriteBoxKind::Artifact,
        WriteBoxKind::MirrorAdvisory,
        WriteBoxKind::Memory,
        WriteBoxKind::Execution,
        WriteBoxKind::Promotion,
    ] {
        assert!(kinds.contains(&kind), "missing write-box kind: {:?}", kind);
    }
}

#[test]
fn every_write_box_schema_declares_lifecycle_and_projection_contract() {
    let family = kernel002_write_box_schema_family();

    for schema in &family.schemas {
        assert!(schema.schema_id.starts_with("hsk.write_box."));
        assert!(!schema.allowed_transitions.is_empty());
        assert!(!schema.required_evidence_refs.is_empty());
        assert!(!schema.validation_requirements.is_empty());
        assert!(!schema.projection_rules.is_empty());
        assert!(
            schema
                .allowed_transitions
                .iter()
                .any(|transition| transition.from == WriteBoxLifecycleState::Open),
            "schema must define transitions out of Open: {:?}",
            schema.kind
        );
    }
}

#[test]
fn promotion_box_is_the_only_authority_write_box() {
    let family = kernel002_write_box_schema_family();

    for schema in &family.schemas {
        if schema.kind == WriteBoxKind::Promotion {
            assert_eq!(
                schema.authority_effect,
                AuthorityEffect::EventLedgerAuthorityWrite
            );
        } else {
            assert_ne!(
                schema.authority_effect,
                AuthorityEffect::EventLedgerAuthorityWrite
            );
        }
    }
}

#[test]
fn concrete_write_box_types_share_required_common_contract() {
    let common = sample_common(WriteBoxKind::CrdtWorkspace);
    validate_write_box_common(&common).expect("complete common write-box state must validate");

    assert_eq!(common.schema_version, "hsk.write_box.v1");
    assert_eq!(common.crdt_site_id, "site-kernel-builder");
    assert_eq!(
        common.target_refs[0].authority_class,
        "pre_promotion_workspace"
    );
    assert_eq!(
        common.base_snapshot_refs,
        vec!["snapshot://workspace-001/base-sv-0"]
    );
    assert_eq!(
        common.intent_summary,
        "Propose CRDT patch through a write box"
    );
    assert_eq!(
        common.operation_payload_refs[0].payload_ref,
        "postgres://kernel_crdt_updates/crdt-update-1/update_bytes"
    );
    assert_eq!(
        common.receipt_refs,
        vec!["receipt://write-box-created/wb-001"]
    );
    assert_eq!(
        common.replay_metadata.replay_order_key,
        "workspace-001/document-001/00000000000000000001"
    );

    let crdt = CRDTWorkspaceBox {
        common: common.clone(),
        state_vector: "sv-001".to_string(),
        update_refs: vec!["update-001".to_string()],
    };
    assert_eq!(crdt.common.kind, WriteBoxKind::CrdtWorkspace);

    let _draft = DraftBox {
        common: sample_common(WriteBoxKind::Draft),
        draft_ref: "draft-ref".to_string(),
    };
    let _proposal = ProposalBox {
        common: sample_common(WriteBoxKind::Proposal),
        proposal_ref: "proposal-ref".to_string(),
    };
    let _patch = PatchBox {
        common: sample_common(WriteBoxKind::Patch),
        patch_ref: "patch-ref".to_string(),
    };
    let _artifact = ArtifactBox {
        common: sample_common(WriteBoxKind::Artifact),
        artifact_ref: "artifact-ref".to_string(),
    };
    let _mirror = MirrorAdvisoryBox {
        common: sample_common(WriteBoxKind::MirrorAdvisory),
        mirror_path: "mirror.md".to_string(),
        advisory_ref: "advisory-ref".to_string(),
    };
    let _memory = MemoryBox {
        common: sample_common(WriteBoxKind::Memory),
        memory_extract_ref: "memory-ref".to_string(),
    };
    let _execution = ExecutionBox {
        common: sample_common(WriteBoxKind::Execution),
        execution_ref: "execution-ref".to_string(),
    };
    let _promotion = PromotionBox {
        common: sample_common(WriteBoxKind::Promotion),
        promotion_target_ref: "promotion-target".to_string(),
        event_ledger_ref: None,
    };
}

#[test]
fn promotion_box_validation_allows_queued_preview_without_event_ref() {
    let promotion = sample_promotion_box(WriteBoxLifecycleState::PromotionQueued, None);

    validate_promotion_box(&promotion)
        .expect("queued promotion preview can exist before EventLedger append");
}

#[test]
fn promotion_box_validation_requires_event_ref_after_promotion() {
    let missing_event_ref = sample_promotion_box(WriteBoxLifecycleState::Promoted, None);
    let errors = validate_promotion_box(&missing_event_ref)
        .expect_err("promoted write box must cite EventLedger append");

    assert!(errors.iter().any(|error| error.field == "event_ledger_ref"));

    let promoted = sample_promotion_box(
        WriteBoxLifecycleState::Promoted,
        Some("eventledger://kernel-promotion/event-001".to_string()),
    );
    validate_promotion_box(&promoted).expect("promoted write box with EventLedger ref validates");
}

#[test]
fn write_box_common_validation_rejects_missing_owner_evidence_or_projection() {
    let mut missing_owner = sample_common(WriteBoxKind::Draft);
    missing_owner.owner.actor_id.clear();
    let errors = validate_write_box_common(&missing_owner)
        .expect_err("missing owner must fail write-box validation");
    assert!(errors.iter().any(|error| error.field == "owner.actor_id"));

    let mut missing_evidence = sample_common(WriteBoxKind::Draft);
    missing_evidence.evidence_refs.clear();
    let errors = validate_write_box_common(&missing_evidence)
        .expect_err("missing evidence must fail write-box validation");
    assert!(errors.iter().any(|error| error.field == "evidence_refs"));

    let mut missing_projection = sample_common(WriteBoxKind::Draft);
    missing_projection.projection_rules.clear();
    let errors = validate_write_box_common(&missing_projection)
        .expect_err("missing projection rules must fail write-box validation");
    assert!(errors.iter().any(|error| error.field == "projection_rules"));

    let mut missing_target = sample_common(WriteBoxKind::Draft);
    missing_target.target_refs.clear();
    let errors = validate_write_box_common(&missing_target)
        .expect_err("missing target refs must fail write-box validation");
    assert!(errors.iter().any(|error| error.field == "target_refs"));

    let mut missing_payload = sample_common(WriteBoxKind::Draft);
    missing_payload.operation_payload_refs.clear();
    let errors = validate_write_box_common(&missing_payload)
        .expect_err("missing operation payload refs must fail write-box validation");
    assert!(errors
        .iter()
        .any(|error| error.field == "operation_payload_refs"));

    let mut missing_replay = sample_common(WriteBoxKind::Draft);
    missing_replay.replay_metadata.replay_order_key.clear();
    let errors = validate_write_box_common(&missing_replay)
        .expect_err("missing replay metadata must fail write-box validation");
    assert!(errors
        .iter()
        .any(|error| error.field == "replay_metadata.replay_order_key"));
}

fn sample_promotion_box(
    lifecycle_state: WriteBoxLifecycleState,
    event_ledger_ref: Option<String>,
) -> PromotionBox {
    let mut common = sample_common(WriteBoxKind::Promotion);
    common.lifecycle_state = lifecycle_state;
    common.authority_effect = AuthorityEffect::EventLedgerAuthorityWrite;
    common.validation_status.state = WriteBoxValidationState::Valid;
    common.validation_status.check_ids = vec![
        "promotion_gate".to_string(),
        "idempotency".to_string(),
        "event_ledger_append".to_string(),
    ];
    common.projection_rules = vec![
        "dcc.promotion_queue".to_string(),
        "dcc.event_ledger_preview".to_string(),
    ];
    common.promotion_receipt_refs = match lifecycle_state {
        WriteBoxLifecycleState::Promoted => {
            vec!["receipt://promotion-accepted/wb-001".to_string()]
        }
        _ => vec!["receipt://promotion-requested/wb-001".to_string()],
    };

    PromotionBox {
        common,
        promotion_target_ref: "authority://kernel/document/document-001".to_string(),
        event_ledger_ref,
    }
}

fn sample_common(kind: WriteBoxKind) -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: "wb-001".to_string(),
        kind,
        schema_version: "hsk.write_box.v1".to_string(),
        workspace_id: "workspace-001".to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "actor-001".to_string(),
            actor_kind: "model".to_string(),
            role_id: "CODER".to_string(),
        },
        crdt_site_id: "site-kernel-builder".to_string(),
        target_refs: vec![WriteBoxTargetRef {
            target_id: "document-001".to_string(),
            target_kind: "crdt_document".to_string(),
            authority_class: "pre_promotion_workspace".to_string(),
        }],
        base_snapshot_refs: vec!["snapshot://workspace-001/base-sv-0".to_string()],
        intent_summary: "Propose CRDT patch through a write box".to_string(),
        operation_payload_refs: vec![WriteBoxPayloadRef {
            payload_id: "payload-crdt-update-1".to_string(),
            payload_kind: "yjs_update".to_string(),
            payload_ref: "postgres://kernel_crdt_updates/crdt-update-1/update_bytes".to_string(),
            payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        }],
        lifecycle_state: WriteBoxLifecycleState::Open,
        allowed_transitions: vec![
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::Denied,
        ],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        evidence_refs: vec!["evidence-001".to_string()],
        receipt_refs: vec!["receipt://write-box-created/wb-001".to_string()],
        denial_receipt_refs: Vec::new(),
        promotion_receipt_refs: Vec::new(),
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Pending,
            check_ids: vec!["schema_validity".to_string()],
        },
        projection_rules: vec!["dcc.write_box.queue".to_string()],
        replay_metadata: WriteBoxReplayMetadataV1 {
            replay_plan_ref: "crdt-replay-plan://workspace-001/document-001".to_string(),
            replay_order_key: "workspace-001/document-001/00000000000000000001".to_string(),
            idempotency_key: "write-box:wb-001:create".to_string(),
            source_event_refs: vec!["eventledger://stream-crdt/evt-crdt-update-1".to_string()],
        },
    }
}
