use serde::{Deserialize, Serialize};

use super::action_catalog::KernelActionCatalogV1;
use super::action_envelope::AuthorityEffect;
use super::write_boxes::{
    MirrorAdvisoryBox, WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef,
    WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef, WriteBoxValidationState,
    WriteBoxValidationStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorAdvisoryEditV1 {
    pub advisory_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub role_id: String,
    pub mirror_path: String,
    pub source_projection_hash: String,
    pub proposed_patch_ref: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorAdvisoryRecordV1 {
    pub schema_id: &'static str,
    pub advisory_id: String,
    pub mirror_advisory_box: MirrorAdvisoryBox,
    pub normalization_action_id: String,
    pub promotion_action_id: String,
    pub authority_mutation: bool,
    pub accepted_event_ledger_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorAdvisoryPromotionInputV1 {
    pub schema_id: &'static str,
    pub advisory_id: String,
    pub action_id: String,
    pub promotion_action_id: String,
    pub validation_receipt_ref: String,
    pub authority_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorAdvisoryCaptureError {
    MissingCatalogAction { action_id: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorAdvisoryPromotionError {
    ValidationNotAccepted,
}

pub fn capture_mirror_advisory_edit(
    edit: &MirrorAdvisoryEditV1,
    catalog: &KernelActionCatalogV1,
) -> Result<MirrorAdvisoryRecordV1, MirrorAdvisoryCaptureError> {
    require_catalog_action(catalog, "kernel.mirror_advisory.capture")?;
    require_catalog_action(catalog, "kernel.mirror_advisory.normalize")?;
    require_catalog_action(catalog, "kernel.write_box.promote")?;

    Ok(MirrorAdvisoryRecordV1 {
        schema_id: "hsk.mirror_advisory_record@1",
        advisory_id: edit.advisory_id.clone(),
        mirror_advisory_box: MirrorAdvisoryBox {
            common: WriteBoxCommon {
                write_box_id: format!("mirror-advisory-box-{}", edit.advisory_id),
                kind: WriteBoxKind::MirrorAdvisory,
                schema_version: "hsk.write_box.v1".to_string(),
                workspace_id: "generated-mirror-workspace".to_string(),
                owner: WriteBoxOwnerRef {
                    actor_id: edit.actor_id.clone(),
                    actor_kind: edit.actor_kind.clone(),
                    role_id: edit.role_id.clone(),
                },
                crdt_site_id: format!("mirror-advisory-site-{}", edit.actor_id),
                target_refs: vec![WriteBoxTargetRef {
                    target_id: edit.mirror_path.clone(),
                    target_kind: "generated_mirror".to_string(),
                    authority_class: "generated_projection".to_string(),
                }],
                base_snapshot_refs: vec![format!(
                    "mirror-snapshot://{}",
                    edit.source_projection_hash
                )],
                intent_summary: "Capture generated mirror drift without direct authority mutation"
                    .to_string(),
                operation_payload_refs: vec![WriteBoxPayloadRef {
                    payload_id: format!("payload-{}", edit.advisory_id),
                    payload_kind: "mirror_advisory_patch".to_string(),
                    payload_ref: edit.proposed_patch_ref.clone(),
                    payload_sha256: edit.source_projection_hash.clone(),
                }],
                lifecycle_state: WriteBoxLifecycleState::Open,
                allowed_transitions: vec![
                    WriteBoxLifecycleState::ReadyForValidation,
                    WriteBoxLifecycleState::Denied,
                ],
                authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
                evidence_refs: vec![
                    edit.source_projection_hash.clone(),
                    edit.proposed_patch_ref.clone(),
                    edit.trace_id.clone(),
                ],
                receipt_refs: vec![format!(
                    "receipt://mirror-advisory-captured/{}",
                    edit.advisory_id
                )],
                denial_receipt_refs: Vec::new(),
                promotion_receipt_refs: Vec::new(),
                validation_status: WriteBoxValidationStatus {
                    state: WriteBoxValidationState::Pending,
                    check_ids: vec![
                        "mirror_drift".to_string(),
                        "normalization_candidate".to_string(),
                    ],
                },
                projection_rules: vec!["dcc.mirror_advisory_queue".to_string()],
                replay_metadata: WriteBoxReplayMetadataV1 {
                    replay_plan_ref: format!("mirror-advisory-replay://{}", edit.advisory_id),
                    replay_order_key: format!(
                        "generated-mirror-workspace/{}/{}",
                        edit.mirror_path, edit.advisory_id
                    ),
                    idempotency_key: format!("mirror-advisory:{}:capture", edit.advisory_id),
                    source_event_refs: vec![format!("trace://{}", edit.trace_id)],
                },
            },
            mirror_path: edit.mirror_path.clone(),
            advisory_ref: edit.proposed_patch_ref.clone(),
        },
        normalization_action_id: "kernel.mirror_advisory.normalize".to_string(),
        promotion_action_id: "kernel.write_box.promote".to_string(),
        authority_mutation: false,
        accepted_event_ledger_ref: None,
    })
}

pub fn promote_mirror_advisory_if_valid(
    record: &MirrorAdvisoryRecordV1,
    validation_receipt_ref: &str,
) -> Result<MirrorAdvisoryPromotionInputV1, MirrorAdvisoryPromotionError> {
    if record.mirror_advisory_box.common.validation_status.state != WriteBoxValidationState::Valid {
        return Err(MirrorAdvisoryPromotionError::ValidationNotAccepted);
    }

    Ok(MirrorAdvisoryPromotionInputV1 {
        schema_id: "hsk.mirror_advisory_promotion_input@1",
        advisory_id: record.advisory_id.clone(),
        action_id: record.normalization_action_id.clone(),
        promotion_action_id: record.promotion_action_id.clone(),
        validation_receipt_ref: validation_receipt_ref.to_string(),
        authority_mutation_allowed: true,
    })
}

fn require_catalog_action(
    catalog: &KernelActionCatalogV1,
    action_id: &'static str,
) -> Result<(), MirrorAdvisoryCaptureError> {
    if catalog.action(action_id).is_some() {
        Ok(())
    } else {
        Err(MirrorAdvisoryCaptureError::MissingCatalogAction { action_id })
    }
}
