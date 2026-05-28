use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::{
    kernel::{
        action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
        action_envelope::{ApprovalPosture, AuthorityEffect},
        write_boxes::WriteBoxKind,
    },
    model_runtime::{
        techniques::steering_vector_store::{
            PersistSteeringVectorRequest, ReviewStatus, SteeringVectorStore,
            SteeringVectorStoreError, STEERING_VECTOR_ARTIFACT_SCHEMA_ID,
            STEERING_VECTOR_LIST_ACTION_ID, STEERING_VECTOR_REGISTER_ACTION_ID,
        },
        CaptureResult, CaptureSpec, ContrastiveTechnique, HookPoint, LayerIndex, ModelRuntimeError,
        OperatorId, SteeringHookHandle, SteeringHookOps, SteeringProvenance, SteeringVector,
        SteeringVectorId, SteeringVectorMeta, SteeringVectorValues,
    },
};

#[test]
fn steering_vector_store_persists_write_box_artifact_and_rehydrates_after_restart() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = SteeringVectorStore::new(tempdir.path());
    let vector = vector("honesty-repe", vec![0.25, -0.5, 0.75]);
    let vector_id = vector.id;

    let receipt = store
        .persist(PersistSteeringVectorRequest {
            vector: vector.clone(),
            license_tag: "operator-local".to_string(),
            model_compat_tag: "local-test-base".to_string(),
            created_by: OperatorId::new("operator-ilja"),
            session_id: "session-mt-097".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
        })
        .expect("persist steering vector");

    assert_eq!(receipt.artifact.id, vector_id);
    assert_eq!(receipt.artifact.license_tag, "operator-local");
    assert_eq!(receipt.artifact.model_compat_tag, "local-test-base");
    assert_eq!(receipt.artifact.review_status, ReviewStatus::Pending);
    assert_eq!(receipt.artifact.values_sha256, receipt.values_blob.sha256);
    assert!(receipt.values_blob.artifact_ref.starts_with("artifact://"));
    assert_eq!(
        receipt.action_request.action_id,
        STEERING_VECTOR_REGISTER_ACTION_ID
    );
    assert_eq!(
        receipt.write_box_envelope.payload_schema_id,
        STEERING_VECTOR_ARTIFACT_SCHEMA_ID
    );
    assert_eq!(
        receipt.write_box_envelope.write_box.common.kind,
        WriteBoxKind::Artifact
    );
    assert_eq!(
        receipt.write_box_envelope.write_box.common.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        receipt.action_request.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );

    let restarted = SteeringVectorStore::new(tempdir.path());
    let artifacts = restarted
        .list_for_model("local-test-base")
        .expect("reload metadata after restart");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].id, vector_id);
    assert_eq!(
        artifacts[0].derivation_provenance,
        vector.derivation_provenance
    );

    let hooks = Arc::new(RecordingSteeringHooks::default());
    let handle = SteeringHookHandle::with_ops("rehydrate-hooks", hooks.clone());
    let rehydrated = futures::executor::block_on(
        restarted.rehydrate_registered_vectors(&handle, "local-test-base"),
    )
    .expect("rehydrate vectors into hook registry");

    assert_eq!(rehydrated, vec![vector_id]);
    assert_eq!(hooks.list_vectors().len(), 1);
    assert!(hooks.active.lock().unwrap().is_empty());
}

#[test]
fn steering_vector_store_rejects_missing_license_and_preserves_provenance_shape() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = SteeringVectorStore::new(tempdir.path());
    let err = store
        .persist(PersistSteeringVectorRequest {
            vector: vector("no-license", vec![1.0]),
            license_tag: " ".to_string(),
            model_compat_tag: "local-test-base".to_string(),
            created_by: OperatorId::new("operator-ilja"),
            session_id: "session-mt-097".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
        })
        .expect_err("license tag is required");

    assert!(err.to_string().contains("license_tag"), "{err}");
}

#[test]
fn steering_vector_store_catalog_actions_route_register_and_list_through_kernel_catalog() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let register = catalog
        .action(STEERING_VECTOR_REGISTER_ACTION_ID)
        .expect("steering register action");
    assert_eq!(
        register.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        register.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(register.expected_write_boxes.iter().any(|write_box| {
        write_box.write_box_kind == "ArtifactBox"
            && write_box.write_box_schema_id == "hsk.write_box.artifact@1"
            && write_box.target_id == "steering_vector_artifact"
    }));

    let list = catalog
        .action(STEERING_VECTOR_LIST_ACTION_ID)
        .expect("steering list action");
    assert_eq!(list.authority_effect, AuthorityEffect::ProjectionOnly);
    assert_eq!(list.approval_posture, ApprovalPosture::NoApprovalRequired);
}

#[derive(Default)]
struct RecordingSteeringHooks {
    vectors: Mutex<BTreeMap<SteeringVectorId, SteeringVector>>,
    active: Mutex<BTreeSet<SteeringVectorId>>,
}

#[async_trait]
impl SteeringHookOps for RecordingSteeringHooks {
    async fn capture(&self, _spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        Ok(CaptureResult {
            activations: BTreeMap::new(),
            tokens_seen: 0,
        })
    }

    async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        let id = vector.id;
        self.vectors.lock().unwrap().insert(id, vector);
        Ok(id)
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        self.vectors
            .lock()
            .unwrap()
            .values()
            .map(SteeringVectorMeta::from)
            .collect()
    }

    async fn set_active(&self, ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        *self.active.lock().unwrap() = ids.into_iter().collect();
        Ok(())
    }

    async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        self.vectors.lock().unwrap().remove(&id);
        self.active.lock().unwrap().remove(&id);
        Ok(())
    }
}

fn vector(name: &str, values: Vec<f32>) -> SteeringVector {
    SteeringVector::try_new(
        None,
        name,
        LayerIndex::new(12),
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(values, 1.25).expect("valid values"),
        "MT-097 steering vector",
        Some(SteeringProvenance::Contrastive {
            positive_prompts: vec!["I want to be honest".to_string()],
            negative_prompts: vec!["I want to deceive".to_string()],
            technique: ContrastiveTechnique::RepE,
        }),
    )
    .expect("valid vector")
}

fn persist(store: &SteeringVectorStore, vector: SteeringVector) {
    store
        .persist(PersistSteeringVectorRequest {
            vector,
            license_tag: "operator-local".to_string(),
            model_compat_tag: "local-test-base".to_string(),
            created_by: OperatorId::new("operator-ilja"),
            session_id: "session-mt-097".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
        })
        .expect("persist steering vector");
}

#[test]
fn steering_vector_activation_requires_operator_approval() {
    // MT-097 HIGH: ReviewStatus::Pending must be enforced before activation.
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = SteeringVectorStore::new(tempdir.path());
    let vector = vector("refusal-gate", vec![0.1, 0.2, 0.3]);
    let id = vector.id;
    persist(&store, vector);

    assert_eq!(
        store.review_status(id).expect("status"),
        ReviewStatus::Pending
    );
    let err = store
        .ensure_activatable(&[id])
        .expect_err("pending vector must not be activatable");
    assert!(
        matches!(err, SteeringVectorStoreError::ActivationNotApproved { .. }),
        "{err}"
    );

    let approver = OperatorId::new("operator-ilja");
    let approved = store.approve(id, &approver).expect("approve");
    assert_eq!(approved.review_status, ReviewStatus::Approved);
    assert_eq!(approved.reviewed_by.as_ref(), Some(&approver));
    assert!(approved.reviewed_at_utc.is_some());
    store
        .ensure_activatable(&[id])
        .expect("approved vector is activatable");

    // The approval decision is auditable: re-approving is rejected, not a
    // silent no-op.
    let err = store
        .approve(id, &approver)
        .expect_err("re-approve must fail");
    assert!(
        matches!(err, SteeringVectorStoreError::InvalidReviewTransition { .. }),
        "{err}"
    );
}

#[test]
fn activate_reviewed_vectors_gates_set_active_on_approval() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = SteeringVectorStore::new(tempdir.path());
    let vector = vector("refusal-gate-active", vec![0.4, 0.5, 0.6]);
    let id = vector.id;
    persist(&store, vector);

    let hooks = Arc::new(RecordingSteeringHooks::default());
    let handle = SteeringHookHandle::with_ops("gate-hooks", hooks.clone());
    futures::executor::block_on(store.rehydrate_registered_vectors(&handle, "local-test-base"))
        .expect("rehydrate registers the vector into the hook registry");

    // Pending: the gate rejects activation and nothing becomes active.
    let err = futures::executor::block_on(store.activate_reviewed_vectors(&handle, vec![id]))
        .expect_err("pending activation rejected");
    assert!(
        matches!(err, SteeringVectorStoreError::ActivationNotApproved { .. }),
        "{err}"
    );
    assert!(hooks.active.lock().unwrap().is_empty());

    // Approved: activation succeeds and the vector is now active.
    store
        .approve(id, &OperatorId::new("operator-ilja"))
        .expect("approve");
    futures::executor::block_on(store.activate_reviewed_vectors(&handle, vec![id]))
        .expect("approved activation succeeds");
    assert!(hooks.active.lock().unwrap().contains(&id));
}

#[test]
fn steering_vector_deny_blocks_activation() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = SteeringVectorStore::new(tempdir.path());
    let vector = vector("refusal-deny", vec![0.7, 0.8]);
    let id = vector.id;
    persist(&store, vector);

    let denied = store
        .deny(id, &OperatorId::new("operator-ilja"))
        .expect("deny");
    assert_eq!(denied.review_status, ReviewStatus::Denied);
    let err = store
        .ensure_activatable(&[id])
        .expect_err("denied vector must not be activatable");
    assert!(
        matches!(err, SteeringVectorStoreError::ActivationNotApproved { .. }),
        "{err}"
    );
}
