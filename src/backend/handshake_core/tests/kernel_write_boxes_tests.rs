use std::collections::HashSet;

use handshake_core::kernel::{
    action_envelope::AuthorityEffect,
    write_boxes::{
        kernel002_write_box_schema_family, validate_write_box_common,
        validate_write_box_schema_family, ArtifactBox, CRDTWorkspaceBox, DraftBox, ExecutionBox,
        MemoryBox, MirrorAdvisoryBox, PatchBox, PromotionBox, ProposalBox, WriteBoxCommon,
        WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef, WriteBoxValidationState,
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
}

fn sample_common(kind: WriteBoxKind) -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: "wb-001".to_string(),
        kind,
        workspace_id: "workspace-001".to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "actor-001".to_string(),
            actor_kind: "model".to_string(),
            role_id: "CODER".to_string(),
        },
        lifecycle_state: WriteBoxLifecycleState::Open,
        allowed_transitions: vec![
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::Denied,
        ],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        evidence_refs: vec!["evidence-001".to_string()],
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Pending,
            check_ids: vec!["schema_validity".to_string()],
        },
        projection_rules: vec!["dcc.write_box.queue".to_string()],
    }
}
