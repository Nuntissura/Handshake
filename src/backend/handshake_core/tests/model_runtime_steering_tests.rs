use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use handshake_core::model_runtime::{
    CaptureResult, CaptureSpec, ContrastiveTechnique, HookPoint, LayerIndex, ModelRuntimeError,
    SteeringHookHandle, SteeringHookOps, SteeringProvenance, SteeringVector, SteeringVectorId,
    SteeringVectorMeta, SteeringVectorValues,
};

#[derive(Default)]
struct InMemorySteeringHooks {
    vectors: Mutex<BTreeMap<SteeringVectorId, SteeringVector>>,
    active: Mutex<BTreeSet<SteeringVectorId>>,
}

#[async_trait::async_trait]
impl SteeringHookOps for InMemorySteeringHooks {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        let mut activations = BTreeMap::new();
        for layer in &spec.layers {
            activations.insert(*layer, vec![vec![layer.as_u32() as f32, 1.0]]);
        }
        Ok(CaptureResult {
            activations,
            tokens_seen: spec.prompts.iter().map(|prompt| prompt.len() as u32).sum(),
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
        let known = self.vectors.lock().unwrap();
        for id in &ids {
            if !known.contains_key(id) {
                return Err(ModelRuntimeError::SteeringHookError(format!(
                    "unknown steering vector {id}"
                )));
            }
        }
        *self.active.lock().unwrap() = ids.into_iter().collect();
        Ok(())
    }

    async fn unregister(&self, id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        self.vectors.lock().unwrap().remove(&id);
        self.active.lock().unwrap().remove(&id);
        Ok(())
    }
}

#[test]
fn model_runtime_steering_tests_ops_are_object_safe_and_intensity_is_validated() {
    fn assert_object_safe(_: Box<dyn SteeringHookOps>) {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Box<dyn SteeringHookOps>>();
    assert_send_sync::<Arc<dyn SteeringHookOps>>();
    assert_object_safe(Box::new(InMemorySteeringHooks::default()));

    assert_eq!(
        SteeringVectorValues::try_new(vec![1.0], -10.0)
            .unwrap()
            .intensity(),
        -10.0
    );
    assert_eq!(
        SteeringVectorValues::try_new(vec![1.0], 10.0)
            .unwrap()
            .intensity(),
        10.0
    );
    assert!(SteeringVectorValues::try_new(vec![1.0], -10.1).is_err());
    assert!(SteeringVectorValues::try_new(vec![1.0], 10.1).is_err());
    assert!(SteeringVectorValues::try_new(vec![1.0], f32::NAN).is_err());
    assert!(SteeringVectorValues::try_new(vec![1.0], f32::INFINITY).is_err());
    assert!(SteeringVectorValues::try_new(Vec::new(), 1.0).is_err());
}

#[test]
fn model_runtime_steering_tests_capture_register_activate_and_unregister() {
    let hooks = InMemorySteeringHooks::default();

    let capture = futures::executor::block_on(hooks.capture(CaptureSpec {
        prompts: vec!["positive".to_string(), "negative".to_string()],
        layers: vec![LayerIndex::new(2), LayerIndex::new(4)],
        hook_point: HookPoint::ResidStream,
    }))
    .unwrap();
    assert_eq!(capture.tokens_seen, 16);
    assert_eq!(capture.activations.len(), 2);
    assert_eq!(capture.activations[&LayerIndex::new(4)][0][0], 4.0);

    let vector = vector("refusal-direction");
    let id = vector.id;
    assert_eq!(id.as_uuid().get_version_num(), 7);
    futures::executor::block_on(hooks.register_vector(vector)).unwrap();
    assert_eq!(hooks.list_vectors().len(), 1);
    assert_eq!(hooks.list_vectors()[0].id, id);

    futures::executor::block_on(hooks.set_active(vec![id])).unwrap();
    futures::executor::block_on(hooks.unregister(id)).unwrap();
    assert!(hooks.list_vectors().is_empty());
    assert!(futures::executor::block_on(hooks.set_active(vec![id])).is_err());
}

#[test]
fn model_runtime_steering_tests_handle_forwards_to_ops_and_rejects_unprovenanced_vectors() {
    let hooks = Arc::new(InMemorySteeringHooks::default());
    let handle = SteeringHookHandle::with_ops("steering", hooks);
    let vector = vector("caa-positive-minus-negative");
    let id = vector.id;

    futures::executor::block_on(handle.register_vector(vector)).unwrap();
    assert_eq!(handle.list_vectors()[0].id, id);
    futures::executor::block_on(handle.set_active(vec![id])).unwrap();
    let capture = futures::executor::block_on(handle.capture(CaptureSpec {
        prompts: vec!["prompt".to_string()],
        layers: vec![LayerIndex::new(1)],
        hook_point: HookPoint::MlpOut,
    }))
    .unwrap();
    assert_eq!(capture.activations[&LayerIndex::new(1)][0], vec![1.0, 1.0]);
    futures::executor::block_on(handle.unregister(id)).unwrap();
    assert!(handle.list_vectors().is_empty());

    assert!(SteeringVector::try_new(
        None,
        "missing-provenance",
        LayerIndex::new(1),
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![1.0], 1.0).unwrap(),
        "must fail",
        None,
    )
    .is_err());
}

#[test]
fn model_runtime_steering_tests_vector_ids_are_v7_and_timestamp_prefix_monotonic() {
    let first = SteeringVectorId::new_v7();
    let second = SteeringVectorId::new_v7();

    assert_eq!(first.as_uuid().get_version_num(), 7);
    assert_eq!(second.as_uuid().get_version_num(), 7);
    assert!(
        uuid_v7_unix_millis(second.as_uuid()) >= uuid_v7_unix_millis(first.as_uuid()),
        "SteeringVectorId v7 timestamp prefix must not move backwards"
    );
    assert_ne!(first, second);
}

#[test]
fn model_runtime_steering_tests_public_surface_is_engine_agnostic() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model_runtime/steering.rs"),
    )
    .expect("read steering.rs");
    let normalized = source.to_ascii_lowercase();
    for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
        assert!(
            !normalized.contains(banned),
            "steering surface must not leak engine-specific type `{banned}`"
        );
    }
}

fn vector(name: &str) -> SteeringVector {
    SteeringVector::try_new(
        None,
        name,
        LayerIndex::new(12),
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![0.1, 0.2, 0.3], 1.5).unwrap(),
        "test vector",
        Some(SteeringProvenance::Contrastive {
            positive_prompts: vec!["helpful".to_string()],
            negative_prompts: vec!["refusal".to_string()],
            technique: ContrastiveTechnique::CAA,
        }),
    )
    .unwrap()
}

fn uuid_v7_unix_millis(id: uuid::Uuid) -> u64 {
    let bytes = *id.as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | bytes[5] as u64
}

mod model_runtime {
    pub mod steering {
        use handshake_core::model_runtime::{
            SteeringHookHandle, SteeringVectorId, SteeringVectorValues,
        };

        #[test]
        fn filter_visible_contract_smoke() {
            assert_eq!(SteeringVectorId::new_v7().as_uuid().get_version_num(), 7);
            assert!(SteeringVectorValues::try_new(vec![1.0], f32::INFINITY).is_err());
            assert!(SteeringVectorValues::try_new(Vec::new(), 1.0).is_err());
            assert!(SteeringHookHandle::new("unbound").list_vectors().is_empty());
        }
    }
}
