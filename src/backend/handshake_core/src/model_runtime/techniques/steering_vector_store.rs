use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    kernel::{
        action_envelope::{
            validate_kernel_action_request, ApprovalPosture, AuthorityEffect, ExpectedWriteBoxRef,
            KernelActionRequestV1, KernelActorRef, KernelSessionRef, KernelTargetRef,
            ValidationRequirement,
        },
        context_bundle::{canonical_json_bytes, sha256_hex},
        write_boxes::{
            validate_write_box_common, ArtifactBox, WriteBoxCommon, WriteBoxKind,
            WriteBoxLifecycleState, WriteBoxOwnerRef, WriteBoxPayloadRef, WriteBoxReplayMetadataV1,
            WriteBoxTargetRef, WriteBoxValidationState, WriteBoxValidationStatus,
        },
    },
    model_runtime::{
        HookPoint, LayerIndex, ModelRuntimeError, OperatorId, SteeringHookHandle,
        SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
    },
};

pub const STEERING_VECTOR_REGISTER_ACTION_ID: &str = "kernel.steering.register";
pub const STEERING_VECTOR_LIST_ACTION_ID: &str = "kernel.steering.list";
pub const STEERING_VECTOR_REGISTER_INPUT_SCHEMA_ID: &str =
    "hsk.kernel.steering_vector_register_input@1";
pub const STEERING_VECTOR_REGISTER_RESULT_SCHEMA_ID: &str =
    "hsk.kernel.steering_vector_register_result@1";
pub const STEERING_VECTOR_LIST_INPUT_SCHEMA_ID: &str = "hsk.kernel.steering_vector_list_input@1";
pub const STEERING_VECTOR_LIST_RESULT_SCHEMA_ID: &str = "hsk.kernel.steering_vector_list_result@1";
pub const KERNEL_ACTION_REQUEST_SCHEMA_ID: &str = "hsk.kernel_action_request@1";
pub const WRITE_BOX_V1_ENVELOPE_SCHEMA_ID: &str = "hsk.write_box_v1_envelope@1";
pub const STEERING_VECTOR_ARTIFACT_SCHEMA_ID: &str = "hsk.steering_vector.artifact@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteeringVectorArtifact {
    pub schema_id: String,
    pub id: SteeringVectorId,
    pub name: String,
    pub layer: LayerIndex,
    pub hook_point: HookPoint,
    pub description: String,
    pub values_blob_path: String,
    pub values_sha256: String,
    pub value_count: usize,
    pub intensity: f32,
    pub derivation_provenance: SteeringProvenance,
    pub license_tag: String,
    pub model_compat_tag: String,
    pub created_at_utc: DateTime<Utc>,
    pub created_by: OperatorId,
    pub review_status: ReviewStatus,
    /// Operator who recorded the review decision (set on approve/deny). `None`
    /// while the artifact is still `Pending`.
    #[serde(default)]
    pub reviewed_by: Option<OperatorId>,
    /// When the review decision was recorded. `None` while `Pending`.
    #[serde(default)]
    pub reviewed_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBlobRef {
    pub artifact_ref: String,
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SteeringVectorWriteBoxEnvelope {
    pub schema_id: String,
    pub envelope_id: Uuid,
    pub payload_schema_id: String,
    pub payload: Value,
    pub payload_sha256: String,
    pub write_box: ArtifactBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SteeringVectorStoreReceipt {
    pub artifact: SteeringVectorArtifact,
    pub values_blob: ArtifactBlobRef,
    pub metadata_blob: ArtifactBlobRef,
    pub action_request: KernelActionRequestV1,
    pub write_box_envelope: SteeringVectorWriteBoxEnvelope,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistSteeringVectorRequest {
    pub vector: SteeringVector,
    pub license_tag: String,
    pub model_compat_tag: String,
    pub created_by: OperatorId,
    pub session_id: String,
    pub role_id: String,
}

#[derive(Debug, Clone)]
pub struct SteeringVectorStore {
    root: PathBuf,
}

impl SteeringVectorStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn persist(
        &self,
        request: PersistSteeringVectorRequest,
    ) -> Result<SteeringVectorStoreReceipt, SteeringVectorStoreError> {
        validate_persist_request(&request)?;
        let values_relative = relative_values_path(request.vector.id);
        let values_bytes =
            serde_json::to_vec(request.vector.values.values()).map_err(serialization_error)?;
        let values_blob = self.put_artifact_blob(&values_relative, &values_bytes)?;

        let artifact = SteeringVectorArtifact {
            schema_id: STEERING_VECTOR_ARTIFACT_SCHEMA_ID.to_string(),
            id: request.vector.id,
            name: request.vector.name.clone(),
            layer: request.vector.layer,
            hook_point: request.vector.hook_point,
            description: request.vector.description.clone(),
            values_blob_path: values_blob.relative_path.clone(),
            values_sha256: values_blob.sha256.clone(),
            value_count: request.vector.values.values().len(),
            intensity: request.vector.values.intensity(),
            derivation_provenance: request.vector.derivation_provenance.clone(),
            license_tag: request.license_tag.trim().to_string(),
            model_compat_tag: request.model_compat_tag.trim().to_string(),
            created_at_utc: Utc::now(),
            created_by: request.created_by.clone(),
            review_status: ReviewStatus::Pending,
            reviewed_by: None,
            reviewed_at_utc: None,
        };

        let metadata_relative = relative_metadata_path(artifact.id);
        let metadata_bytes = serde_json::to_vec_pretty(&artifact).map_err(serialization_error)?;
        let metadata_blob = self.put_artifact_blob(&metadata_relative, &metadata_bytes)?;
        let payload = json!({
            "schema_id": STEERING_VECTOR_ARTIFACT_SCHEMA_ID,
            "artifact": artifact,
            "values_blob": values_blob,
            "metadata_blob": metadata_blob,
        });
        let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload));
        let action_request = action_request(&request, &payload_sha256);
        validate_kernel_action_request(&action_request).map_err(|errors| {
            SteeringVectorStoreError::InvalidWriteBox(format!(
                "invalid kernel action request: {errors:?}"
            ))
        })?;
        let write_box = artifact_write_box(
            &request,
            &artifact,
            &metadata_blob,
            &values_blob,
            &payload_sha256,
        );
        validate_write_box_common(&write_box.common).map_err(|errors| {
            SteeringVectorStoreError::InvalidWriteBox(format!(
                "invalid steering vector write box: {errors:?}"
            ))
        })?;
        let write_box_envelope = SteeringVectorWriteBoxEnvelope {
            schema_id: WRITE_BOX_V1_ENVELOPE_SCHEMA_ID.to_string(),
            envelope_id: Uuid::now_v7(),
            payload_schema_id: STEERING_VECTOR_ARTIFACT_SCHEMA_ID.to_string(),
            payload,
            payload_sha256,
            write_box,
        };

        Ok(SteeringVectorStoreReceipt {
            artifact,
            values_blob,
            metadata_blob,
            action_request,
            write_box_envelope,
        })
    }

    pub fn list_for_model(
        &self,
        model_compat_tag: &str,
    ) -> Result<Vec<SteeringVectorArtifact>, SteeringVectorStoreError> {
        let model_compat_tag = model_compat_tag.trim();
        if model_compat_tag.is_empty() {
            return Err(SteeringVectorStoreError::InvalidInput(
                "model_compat_tag must not be empty".to_string(),
            ));
        }
        let metadata_dir = self.root.join("steering-vectors").join("metadata");
        if !metadata_dir.exists() {
            return Ok(Vec::new());
        }
        let mut artifacts = Vec::new();
        for entry in fs::read_dir(&metadata_dir).map_err(|error| {
            SteeringVectorStoreError::Io(format!(
                "failed to read metadata dir {}: {error}",
                metadata_dir.display()
            ))
        })? {
            let entry = entry.map_err(|error| {
                SteeringVectorStoreError::Io(format!(
                    "failed to read metadata dir entry {}: {error}",
                    metadata_dir.display()
                ))
            })?;
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let raw = fs::read_to_string(entry.path()).map_err(|error| {
                SteeringVectorStoreError::Io(format!(
                    "failed to read steering metadata {}: {error}",
                    entry.path().display()
                ))
            })?;
            let artifact: SteeringVectorArtifact =
                serde_json::from_str(&raw).map_err(serialization_error)?;
            if artifact.model_compat_tag == model_compat_tag {
                artifacts.push(artifact);
            }
        }
        artifacts.sort_by_key(|artifact| artifact.id.to_string());
        Ok(artifacts)
    }

    pub async fn rehydrate_registered_vectors(
        &self,
        hooks: &SteeringHookHandle,
        model_compat_tag: &str,
    ) -> Result<Vec<SteeringVectorId>, SteeringVectorStoreError> {
        let mut ids = Vec::new();
        for artifact in self.list_for_model(model_compat_tag)? {
            let vector = self.vector_from_artifact(&artifact)?;
            let id = hooks.register_vector(vector).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Current review status for a persisted steering vector.
    pub fn review_status(
        &self,
        id: SteeringVectorId,
    ) -> Result<ReviewStatus, SteeringVectorStoreError> {
        Ok(self.load_artifact(id)?.review_status)
    }

    /// Operator approval transition: `Pending -> Approved`, recording the
    /// approver + timestamp on the persisted artifact. Only a `Pending`
    /// artifact may transition, so a review decision is auditable and the
    /// call is idempotent-rejecting (re-approving or approving a denied
    /// vector errors rather than silently succeeding).
    pub fn approve(
        &self,
        id: SteeringVectorId,
        approver: &OperatorId,
    ) -> Result<SteeringVectorArtifact, SteeringVectorStoreError> {
        self.transition_review(id, ReviewStatus::Approved, approver)
    }

    /// Operator denial transition: `Pending -> Denied`.
    pub fn deny(
        &self,
        id: SteeringVectorId,
        approver: &OperatorId,
    ) -> Result<SteeringVectorArtifact, SteeringVectorStoreError> {
        self.transition_review(id, ReviewStatus::Denied, approver)
    }

    /// Review gate: every id must resolve to an `Approved` artifact, else the
    /// first offending id is returned as
    /// [`SteeringVectorStoreError::ActivationNotApproved`]. This is what makes
    /// `ReviewStatus` load-bearing — before MT-097 remediation the status was
    /// persisted but never read before activation.
    pub fn ensure_activatable(
        &self,
        ids: &[SteeringVectorId],
    ) -> Result<(), SteeringVectorStoreError> {
        for id in ids {
            let status = self.load_artifact(*id)?.review_status;
            if status != ReviewStatus::Approved {
                return Err(SteeringVectorStoreError::ActivationNotApproved { id: *id, status });
            }
        }
        Ok(())
    }

    /// Review-gated activation: rejects unless every vector is `Approved`,
    /// then activates them through the shared `SteeringHookOps` surface.
    /// Activation call sites (the steering Tauri command, rehydrate-on-start)
    /// MUST route through here rather than calling `set_active` directly, so a
    /// refusal-disabling vector cannot become active without operator review.
    pub async fn activate_reviewed_vectors(
        &self,
        hooks: &SteeringHookHandle,
        ids: Vec<SteeringVectorId>,
    ) -> Result<(), SteeringVectorStoreError> {
        self.ensure_activatable(&ids)?;
        hooks.set_active(ids).await?;
        Ok(())
    }

    fn transition_review(
        &self,
        id: SteeringVectorId,
        to: ReviewStatus,
        approver: &OperatorId,
    ) -> Result<SteeringVectorArtifact, SteeringVectorStoreError> {
        let mut artifact = self.load_artifact(id)?;
        if artifact.review_status != ReviewStatus::Pending {
            return Err(SteeringVectorStoreError::InvalidReviewTransition {
                id,
                from: artifact.review_status,
                to,
            });
        }
        artifact.review_status = to;
        artifact.reviewed_by = Some(approver.clone());
        artifact.reviewed_at_utc = Some(Utc::now());
        let metadata_relative = relative_metadata_path(id);
        let metadata_bytes = serde_json::to_vec_pretty(&artifact).map_err(serialization_error)?;
        self.put_artifact_blob(&metadata_relative, &metadata_bytes)?;
        Ok(artifact)
    }

    fn load_artifact(
        &self,
        id: SteeringVectorId,
    ) -> Result<SteeringVectorArtifact, SteeringVectorStoreError> {
        let path = self.root.join(relative_metadata_path(id));
        if !path.exists() {
            return Err(SteeringVectorStoreError::ArtifactNotFound { id });
        }
        let raw = fs::read_to_string(&path).map_err(|error| {
            SteeringVectorStoreError::Io(format!(
                "failed to read steering metadata {}: {error}",
                path.display()
            ))
        })?;
        serde_json::from_str(&raw).map_err(serialization_error)
    }

    fn vector_from_artifact(
        &self,
        artifact: &SteeringVectorArtifact,
    ) -> Result<SteeringVector, SteeringVectorStoreError> {
        let values_path = self.root.join(&artifact.values_blob_path);
        let values_bytes = fs::read(&values_path).map_err(|error| {
            SteeringVectorStoreError::Io(format!(
                "failed to read steering values {}: {error}",
                values_path.display()
            ))
        })?;
        let actual_sha = sha256_hex(&values_bytes);
        if actual_sha != artifact.values_sha256 {
            return Err(SteeringVectorStoreError::InvalidInput(format!(
                "steering values hash mismatch for {}: expected {}, got {actual_sha}",
                artifact.id, artifact.values_sha256
            )));
        }
        let values: Vec<f32> =
            serde_json::from_slice(&values_bytes).map_err(serialization_error)?;
        Ok(SteeringVector::try_new(
            Some(artifact.id),
            artifact.name.clone(),
            artifact.layer,
            artifact.hook_point,
            SteeringVectorValues::try_new(values, artifact.intensity)?,
            artifact.description.clone(),
            Some(artifact.derivation_provenance.clone()),
        )?)
    }

    fn put_artifact_blob(
        &self,
        relative_path: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobRef, SteeringVectorStoreError> {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SteeringVectorStoreError::Io(format!(
                    "failed to create artifact dir {}: {error}",
                    parent.display()
                ))
            })?;
        }
        fs::write(&path, bytes).map_err(|error| {
            SteeringVectorStoreError::Io(format!(
                "failed to write steering artifact {}: {error}",
                path.display()
            ))
        })?;
        Ok(ArtifactBlobRef {
            artifact_ref: format!("artifact://{relative_path}"),
            relative_path: relative_path.to_string(),
            sha256: sha256_hex(bytes),
        })
    }
}

fn validate_persist_request(
    request: &PersistSteeringVectorRequest,
) -> Result<(), SteeringVectorStoreError> {
    if request.license_tag.trim().is_empty() {
        return Err(SteeringVectorStoreError::InvalidInput(
            "license_tag must not be empty".to_string(),
        ));
    }
    if request.model_compat_tag.trim().is_empty() {
        return Err(SteeringVectorStoreError::InvalidInput(
            "model_compat_tag must not be empty".to_string(),
        ));
    }
    if request.session_id.trim().is_empty() {
        return Err(SteeringVectorStoreError::InvalidInput(
            "session_id must not be empty".to_string(),
        ));
    }
    if request.role_id.trim().is_empty() {
        return Err(SteeringVectorStoreError::InvalidInput(
            "role_id must not be empty".to_string(),
        ));
    }
    if request.vector.id.as_uuid().get_version_num() != 7 {
        return Err(SteeringVectorStoreError::InvalidInput(
            "steering vector id must be UUID v7".to_string(),
        ));
    }
    Ok(())
}

fn action_request(
    request: &PersistSteeringVectorRequest,
    payload_sha256: &str,
) -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: KERNEL_ACTION_REQUEST_SCHEMA_ID.to_string(),
        action_id: STEERING_VECTOR_REGISTER_ACTION_ID.to_string(),
        actor: KernelActorRef {
            actor_id: request.created_by.as_str().to_string(),
            actor_kind: "operator".to_string(),
            role_id: request.role_id.clone(),
        },
        session: KernelSessionRef {
            session_id: request.session_id.clone(),
            work_profile_id: "activation-steering".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: request.vector.id.to_string(),
            target_kind: "steering_vector".to_string(),
            authority_class: "pre_promotion_artifact".to_string(),
        }],
        input_schema_id: STEERING_VECTOR_REGISTER_INPUT_SCHEMA_ID.to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "ArtifactBox".to_string(),
            write_box_schema_id: "hsk.write_box.artifact@1".to_string(),
            target_id: "steering_vector_artifact".to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: validation_requirements(),
        trace_id: format!("steering-vector-register:{}", request.vector.id),
        idempotency_key: format!(
            "steering-vector-register:{}:{}",
            request.vector.id, payload_sha256
        ),
    }
}

fn artifact_write_box(
    request: &PersistSteeringVectorRequest,
    artifact: &SteeringVectorArtifact,
    metadata_blob: &ArtifactBlobRef,
    values_blob: &ArtifactBlobRef,
    payload_sha256: &str,
) -> ArtifactBox {
    let write_box_id = Uuid::now_v7().to_string();
    ArtifactBox {
        common: WriteBoxCommon {
            write_box_id: write_box_id.clone(),
            kind: WriteBoxKind::Artifact,
            schema_version: "hsk.write_box.artifact@1".to_string(),
            workspace_id: request.session_id.clone(),
            owner: WriteBoxOwnerRef {
                actor_id: request.created_by.as_str().to_string(),
                actor_kind: "operator".to_string(),
                role_id: request.role_id.clone(),
            },
            crdt_site_id: "steering-vector-store".to_string(),
            target_refs: vec![WriteBoxTargetRef {
                target_id: artifact.id.to_string(),
                target_kind: "steering_vector".to_string(),
                authority_class: "pre_promotion_artifact".to_string(),
            }],
            base_snapshot_refs: vec![format!("model-compat-tag://{}", artifact.model_compat_tag)],
            intent_summary: "Persist steering vector as governed ArtifactBox evidence".to_string(),
            operation_payload_refs: vec![
                WriteBoxPayloadRef {
                    payload_id: format!("metadata-{}", artifact.id),
                    payload_kind: "steering_vector_metadata".to_string(),
                    payload_ref: metadata_blob.artifact_ref.clone(),
                    payload_sha256: metadata_blob.sha256.clone(),
                },
                WriteBoxPayloadRef {
                    payload_id: format!("values-{}", artifact.id),
                    payload_kind: "steering_vector_values".to_string(),
                    payload_ref: values_blob.artifact_ref.clone(),
                    payload_sha256: values_blob.sha256.clone(),
                },
            ],
            lifecycle_state: WriteBoxLifecycleState::Open,
            allowed_transitions: vec![
                WriteBoxLifecycleState::ReadyForValidation,
                WriteBoxLifecycleState::Denied,
            ],
            authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
            evidence_refs: vec![format!("steering-vector://{}", artifact.id)],
            receipt_refs: vec![format!("receipt://steering-vector-register/{write_box_id}")],
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: Vec::new(),
            validation_status: WriteBoxValidationStatus {
                state: WriteBoxValidationState::Pending,
                check_ids: validation_check_ids(),
            },
            projection_rules: vec![
                "dcc.write_box.queue".to_string(),
                "dcc.steering_vector.preview".to_string(),
            ],
            replay_metadata: WriteBoxReplayMetadataV1 {
                replay_plan_ref: format!("steering-vector-register://{}", artifact.id),
                replay_order_key: format!(
                    "{}/{}/{}",
                    request.session_id,
                    artifact.created_at_utc.to_rfc3339(),
                    artifact.id
                ),
                idempotency_key: format!("steering-vector-register:{}", artifact.id),
                source_event_refs: vec![format!("steering-vector://{}", artifact.id)],
            },
        },
        artifact_ref: metadata_blob.artifact_ref.clone(),
    }
}

fn relative_values_path(id: SteeringVectorId) -> String {
    format!("steering-vectors/values/{id}.json")
}

fn relative_metadata_path(id: SteeringVectorId) -> String {
    format!("steering-vectors/metadata/{id}.json")
}

fn validation_requirements() -> Vec<ValidationRequirement> {
    validation_check_ids()
        .into_iter()
        .map(|check_id| ValidationRequirement {
            check_id,
            required: true,
        })
        .collect()
}

fn validation_check_ids() -> Vec<String> {
    [
        "steering_vector_schema",
        "steering_vector_license_tag",
        "steering_vector_provenance",
        "write_box_v1_required",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn serialization_error(error: impl std::fmt::Display) -> SteeringVectorStoreError {
    SteeringVectorStoreError::Serialization(error.to_string())
}

#[derive(Debug, Error)]
pub enum SteeringVectorStoreError {
    #[error("invalid steering vector artifact input: {0}")]
    InvalidInput(String),
    #[error("steering vector artifact I/O failed: {0}")]
    Io(String),
    #[error("steering vector artifact serialization failed: {0}")]
    Serialization(String),
    #[error("invalid steering vector write-box contract: {0}")]
    InvalidWriteBox(String),
    #[error("steering vector {id} is not approved for activation (review status: {status:?})")]
    ActivationNotApproved {
        id: SteeringVectorId,
        status: ReviewStatus,
    },
    #[error("invalid review transition for steering vector {id}: {from:?} -> {to:?}")]
    InvalidReviewTransition {
        id: SteeringVectorId,
        from: ReviewStatus,
        to: ReviewStatus,
    },
    #[error("steering vector artifact {id} not found")]
    ArtifactNotFound { id: SteeringVectorId },
    #[error(transparent)]
    Runtime(#[from] ModelRuntimeError),
}
