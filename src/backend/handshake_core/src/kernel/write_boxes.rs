use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::AuthorityEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteBoxKind {
    Draft,
    CrdtWorkspace,
    Proposal,
    Patch,
    Artifact,
    MirrorAdvisory,
    Memory,
    Execution,
    Promotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteBoxLifecycleState {
    Open,
    ReadyForValidation,
    ValidationFailed,
    Validated,
    PromotionQueued,
    Promoted,
    Denied,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteBoxValidationState {
    Pending,
    Valid,
    Invalid,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxOwnerRef {
    pub actor_id: String,
    pub actor_kind: String,
    pub role_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxTargetRef {
    pub target_id: String,
    pub target_kind: String,
    pub authority_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxPayloadRef {
    pub payload_id: String,
    pub payload_kind: String,
    pub payload_ref: String,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxReplayMetadataV1 {
    pub replay_plan_ref: String,
    pub replay_order_key: String,
    pub idempotency_key: String,
    pub source_event_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxValidationStatus {
    pub state: WriteBoxValidationState,
    pub check_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteBoxTransition {
    pub from: WriteBoxLifecycleState,
    pub to: WriteBoxLifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteBoxSchemaDefinition {
    pub kind: WriteBoxKind,
    pub schema_id: &'static str,
    pub allowed_transitions: Vec<WriteBoxTransition>,
    pub authority_effect: AuthorityEffect,
    pub required_evidence_refs: Vec<&'static str>,
    pub validation_requirements: Vec<&'static str>,
    pub projection_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteBoxSchemaFamilyV1 {
    pub schema_id: &'static str,
    pub family_id: &'static str,
    pub schemas: Vec<WriteBoxSchemaDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxCommon {
    pub write_box_id: String,
    pub kind: WriteBoxKind,
    pub schema_version: String,
    pub workspace_id: String,
    pub owner: WriteBoxOwnerRef,
    pub crdt_site_id: String,
    pub target_refs: Vec<WriteBoxTargetRef>,
    pub base_snapshot_refs: Vec<String>,
    pub intent_summary: String,
    pub operation_payload_refs: Vec<WriteBoxPayloadRef>,
    pub lifecycle_state: WriteBoxLifecycleState,
    pub allowed_transitions: Vec<WriteBoxLifecycleState>,
    pub authority_effect: AuthorityEffect,
    pub evidence_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub denial_receipt_refs: Vec<String>,
    pub promotion_receipt_refs: Vec<String>,
    pub validation_status: WriteBoxValidationStatus,
    pub projection_rules: Vec<String>,
    pub replay_metadata: WriteBoxReplayMetadataV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftBox {
    pub common: WriteBoxCommon,
    pub draft_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CRDTWorkspaceBox {
    pub common: WriteBoxCommon,
    pub state_vector: String,
    pub update_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalBox {
    pub common: WriteBoxCommon,
    pub proposal_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchBox {
    pub common: WriteBoxCommon,
    pub patch_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBox {
    pub common: WriteBoxCommon,
    pub artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorAdvisoryBox {
    pub common: WriteBoxCommon,
    pub mirror_path: String,
    pub advisory_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBox {
    pub common: WriteBoxCommon,
    pub memory_extract_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionBox {
    pub common: WriteBoxCommon,
    pub execution_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionBox {
    pub common: WriteBoxCommon,
    pub promotion_target_ref: String,
    pub event_ledger_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteBoxValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteBoxSchemaFamilyError {
    DuplicateKind {
        kind: WriteBoxKind,
    },
    MissingSchemaField {
        kind: WriteBoxKind,
        field: &'static str,
    },
}

pub fn kernel002_write_box_schema_family() -> WriteBoxSchemaFamilyV1 {
    WriteBoxSchemaFamilyV1 {
        schema_id: "hsk.write_box_schema_family@1",
        family_id: "kernel002-write-box-family-v1",
        schemas: vec![
            schema(
                WriteBoxKind::Draft,
                "hsk.write_box.draft@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["draft_ref"],
                &["schema_validity"],
                &["dcc.write_box.queue", "dcc.draft.preview"],
            ),
            schema(
                WriteBoxKind::CrdtWorkspace,
                "hsk.write_box.crdt_workspace@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["update_refs", "state_vector"],
                &["schema_validity", "state_vector_freshness"],
                &["dcc.crdt_workspace", "dcc.conflict_projection"],
            ),
            schema(
                WriteBoxKind::Proposal,
                "hsk.write_box.proposal@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["proposal_ref"],
                &["actor_capability", "target_authority_class"],
                &["dcc.proposal_queue"],
            ),
            schema(
                WriteBoxKind::Patch,
                "hsk.write_box.patch@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["patch_ref"],
                &["patch_applies", "schema_validity"],
                &["dcc.patch_preview"],
            ),
            schema(
                WriteBoxKind::Artifact,
                "hsk.write_box.artifact@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["artifact_ref"],
                &["artifact_hash", "schema_validity"],
                &["dcc.artifact_viewer"],
            ),
            schema(
                WriteBoxKind::MirrorAdvisory,
                "hsk.write_box.mirror_advisory@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["mirror_path", "advisory_ref"],
                &["mirror_drift", "normalization_candidate"],
                &["dcc.mirror_advisory_queue"],
            ),
            schema(
                WriteBoxKind::Memory,
                "hsk.write_box.memory@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["memory_extract_ref"],
                &["novelty", "contradiction", "dedup"],
                &["dcc.memory_queue"],
            ),
            schema(
                WriteBoxKind::Execution,
                "hsk.write_box.execution@1",
                AuthorityEffect::PrePromotionEvidenceOnly,
                &["execution_ref"],
                &["runtime_truth", "capability_boundary"],
                &["dcc.execution_queue"],
            ),
            schema(
                WriteBoxKind::Promotion,
                "hsk.write_box.promotion@1",
                AuthorityEffect::EventLedgerAuthorityWrite,
                &["promotion_target_ref", "validation_receipt_ref"],
                &["promotion_gate", "idempotency", "event_ledger_append"],
                &["dcc.promotion_queue", "dcc.event_ledger_preview"],
            ),
        ],
    }
}

pub fn validate_write_box_schema_family(
    family: &WriteBoxSchemaFamilyV1,
) -> Result<(), Vec<WriteBoxSchemaFamilyError>> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();

    for schema in &family.schemas {
        if !seen.insert(schema.kind) {
            errors.push(WriteBoxSchemaFamilyError::DuplicateKind { kind: schema.kind });
        }
        if schema.schema_id.trim().is_empty() {
            errors.push(WriteBoxSchemaFamilyError::MissingSchemaField {
                kind: schema.kind,
                field: "schema_id",
            });
        }
        if schema.allowed_transitions.is_empty() {
            errors.push(WriteBoxSchemaFamilyError::MissingSchemaField {
                kind: schema.kind,
                field: "allowed_transitions",
            });
        }
        if schema.required_evidence_refs.is_empty() {
            errors.push(WriteBoxSchemaFamilyError::MissingSchemaField {
                kind: schema.kind,
                field: "required_evidence_refs",
            });
        }
        if schema.validation_requirements.is_empty() {
            errors.push(WriteBoxSchemaFamilyError::MissingSchemaField {
                kind: schema.kind,
                field: "validation_requirements",
            });
        }
        if schema.projection_rules.is_empty() {
            errors.push(WriteBoxSchemaFamilyError::MissingSchemaField {
                kind: schema.kind,
                field: "projection_rules",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_write_box_common(
    common: &WriteBoxCommon,
) -> Result<(), Vec<WriteBoxValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "write_box_id", &common.write_box_id);
    require_non_empty(&mut errors, "schema_version", &common.schema_version);
    require_non_empty(&mut errors, "workspace_id", &common.workspace_id);
    require_non_empty(&mut errors, "owner.actor_id", &common.owner.actor_id);
    require_non_empty(&mut errors, "owner.actor_kind", &common.owner.actor_kind);
    require_non_empty(&mut errors, "owner.role_id", &common.owner.role_id);
    require_non_empty(&mut errors, "crdt_site_id", &common.crdt_site_id);
    require_vec(&mut errors, "target_refs", &common.target_refs);
    require_vec(
        &mut errors,
        "base_snapshot_refs",
        &common.base_snapshot_refs,
    );
    require_non_empty(&mut errors, "intent_summary", &common.intent_summary);
    require_vec(
        &mut errors,
        "operation_payload_refs",
        &common.operation_payload_refs,
    );
    require_vec(
        &mut errors,
        "allowed_transitions",
        &common.allowed_transitions,
    );
    require_vec(&mut errors, "evidence_refs", &common.evidence_refs);
    require_vec(&mut errors, "receipt_refs", &common.receipt_refs);
    require_vec(
        &mut errors,
        "validation_status.check_ids",
        &common.validation_status.check_ids,
    );
    require_vec(&mut errors, "projection_rules", &common.projection_rules);
    require_non_empty(
        &mut errors,
        "replay_metadata.replay_plan_ref",
        &common.replay_metadata.replay_plan_ref,
    );
    require_non_empty(
        &mut errors,
        "replay_metadata.replay_order_key",
        &common.replay_metadata.replay_order_key,
    );
    require_non_empty(
        &mut errors,
        "replay_metadata.idempotency_key",
        &common.replay_metadata.idempotency_key,
    );
    require_vec(
        &mut errors,
        "replay_metadata.source_event_refs",
        &common.replay_metadata.source_event_refs,
    );

    for target in &common.target_refs {
        require_non_empty(&mut errors, "target_refs.target_id", &target.target_id);
        require_non_empty(&mut errors, "target_refs.target_kind", &target.target_kind);
        require_non_empty(
            &mut errors,
            "target_refs.authority_class",
            &target.authority_class,
        );
    }

    for payload in &common.operation_payload_refs {
        require_non_empty(
            &mut errors,
            "operation_payload_refs.payload_id",
            &payload.payload_id,
        );
        require_non_empty(
            &mut errors,
            "operation_payload_refs.payload_kind",
            &payload.payload_kind,
        );
        require_non_empty(
            &mut errors,
            "operation_payload_refs.payload_ref",
            &payload.payload_ref,
        );
        require_non_empty(
            &mut errors,
            "operation_payload_refs.payload_sha256",
            &payload.payload_sha256,
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_promotion_box(
    promotion: &PromotionBox,
) -> Result<(), Vec<WriteBoxValidationError>> {
    let mut errors = validate_write_box_common(&promotion.common)
        .err()
        .unwrap_or_default();

    if promotion.common.kind != WriteBoxKind::Promotion {
        errors.push(WriteBoxValidationError {
            field: "common.kind",
            message: "promotion box must use promotion kind",
        });
    }
    if promotion.common.authority_effect != AuthorityEffect::EventLedgerAuthorityWrite {
        errors.push(WriteBoxValidationError {
            field: "common.authority_effect",
            message: "promotion box must route through EventLedger authority write",
        });
    }
    require_non_empty(
        &mut errors,
        "promotion_target_ref",
        &promotion.promotion_target_ref,
    );

    if let Some(event_ledger_ref) = &promotion.event_ledger_ref {
        require_non_empty(&mut errors, "event_ledger_ref", event_ledger_ref);
    }

    if promotion.common.lifecycle_state == WriteBoxLifecycleState::Promoted {
        match &promotion.event_ledger_ref {
            Some(event_ledger_ref) => {
                require_non_empty(&mut errors, "event_ledger_ref", event_ledger_ref);
            }
            None => errors.push(WriteBoxValidationError {
                field: "event_ledger_ref",
                message: "promoted promotion box must cite EventLedger append",
            }),
        }
        require_vec(
            &mut errors,
            "common.promotion_receipt_refs",
            &promotion.common.promotion_receipt_refs,
        );
        if promotion.common.validation_status.state != WriteBoxValidationState::Valid {
            errors.push(WriteBoxValidationError {
                field: "common.validation_status.state",
                message: "promoted promotion box must retain valid validation status",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn schema(
    kind: WriteBoxKind,
    schema_id: &'static str,
    authority_effect: AuthorityEffect,
    required_evidence_refs: &[&'static str],
    validation_requirements: &[&'static str],
    projection_rules: &[&'static str],
) -> WriteBoxSchemaDefinition {
    WriteBoxSchemaDefinition {
        kind,
        schema_id,
        allowed_transitions: default_transitions(),
        authority_effect,
        required_evidence_refs: required_evidence_refs.to_vec(),
        validation_requirements: validation_requirements.to_vec(),
        projection_rules: projection_rules.to_vec(),
    }
}

fn default_transitions() -> Vec<WriteBoxTransition> {
    vec![
        transition(
            WriteBoxLifecycleState::Open,
            WriteBoxLifecycleState::ReadyForValidation,
        ),
        transition(
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::Validated,
        ),
        transition(
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::ValidationFailed,
        ),
        transition(
            WriteBoxLifecycleState::Validated,
            WriteBoxLifecycleState::PromotionQueued,
        ),
        transition(
            WriteBoxLifecycleState::PromotionQueued,
            WriteBoxLifecycleState::Promoted,
        ),
        transition(WriteBoxLifecycleState::Open, WriteBoxLifecycleState::Denied),
        transition(
            WriteBoxLifecycleState::ValidationFailed,
            WriteBoxLifecycleState::Denied,
        ),
        transition(
            WriteBoxLifecycleState::Promoted,
            WriteBoxLifecycleState::Archived,
        ),
        transition(
            WriteBoxLifecycleState::Denied,
            WriteBoxLifecycleState::Archived,
        ),
    ]
}

fn transition(from: WriteBoxLifecycleState, to: WriteBoxLifecycleState) -> WriteBoxTransition {
    WriteBoxTransition { from, to }
}

fn require_non_empty(errors: &mut Vec<WriteBoxValidationError>, field: &'static str, value: &str) {
    if value.trim().is_empty() {
        errors.push(WriteBoxValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(errors: &mut Vec<WriteBoxValidationError>, field: &'static str, value: &[T]) {
    if value.is_empty() {
        errors.push(WriteBoxValidationError {
            field,
            message: "value must not be empty",
        });
    }
}
