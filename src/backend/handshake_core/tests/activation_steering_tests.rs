use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::model_runtime::{
    techniques::activation_steering::{
        capture, contrastive_difference_vector, list_vectors, register_steering_vector,
        set_active_steering_vectors, unregister, FR_EVT_LLM_INFER_STEER_ACTIVE,
        FR_EVT_LLM_INFER_STEER_APPLY, FR_EVT_LLM_INFER_STEER_CAPTURE,
        FR_EVT_LLM_INFER_STEER_REGISTER, FR_EVT_LLM_INFER_STEER_WITHDRAW,
    },
    CancellationToken, CaptureResult, CaptureSpec, ContrastiveTechnique, Embedding,
    GenerateRequest, HookPoint, KvCacheHandle, KvCachePolicy, LayerIndex, LoadSpec,
    LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind,
    RuntimeKind, SamplingParams, Score, SteeringHookHandle, SteeringHookOps, SteeringProvenance,
    SteeringVector, SteeringVectorId, SteeringVectorMeta, SteeringVectorValues, TokenStream,
};

#[tokio::test]
async fn activation_steering_public_surface_dispatches_capture_register_active_unregister() {
    let model_id = ModelId::new_v7();
    let hooks = Arc::new(RecordingSteeringHooks::default());
    let runtime = RecordingRuntime::new(model_id, true, hooks.clone());
    let layers = vec![LayerIndex::new(3), LayerIndex::new(7)];

    let capture_result = capture(
        &runtime,
        model_id,
        vec![
            "I want to be honest".to_string(),
            "I want to deceive".to_string(),
        ],
        layers.clone(),
    )
    .await
    .expect("capture dispatches through runtime steering hooks");

    assert_eq!(hooks.capture_specs.lock().unwrap().len(), 1);
    assert_eq!(capture_result.tokens_seen, 2);
    assert_eq!(
        capture_result.activations[&LayerIndex::new(7)][0],
        vec![7.0, 2.0]
    );

    let direction = contrastive_difference_vector(&capture_result, LayerIndex::new(7), 1)
        .expect("positive-minus-negative direction can be derived from capture rows");
    assert_eq!(direction, vec![0.0, 1.0]);

    let vector = vector_with_values(
        "honesty-repe",
        LayerIndex::new(7),
        direction,
        SteeringProvenance::Contrastive {
            positive_prompts: vec!["I want to be honest".to_string()],
            negative_prompts: vec!["I want to deceive".to_string()],
            technique: ContrastiveTechnique::RepE,
        },
    );
    let vector_id = vector.id;

    let registered_id = register_steering_vector(&runtime, model_id, vector)
        .await
        .expect("register dispatches through runtime steering hooks");
    assert_eq!(registered_id, vector_id);

    set_active_steering_vectors(&runtime, model_id, vec![registered_id])
        .await
        .expect("active set dispatches through runtime steering hooks");
    assert_eq!(
        hooks
            .active
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![registered_id]
    );

    let listed = list_vectors(&runtime, model_id).expect("list vectors succeeds");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, registered_id);
    assert_eq!(listed[0].intensity, 1.0);

    unregister(&runtime, model_id, registered_id)
        .await
        .expect("unregister dispatches through runtime steering hooks");
    assert!(list_vectors(&runtime, model_id).unwrap().is_empty());
}

#[tokio::test]
async fn activation_steering_public_surface_fails_closed_for_capability_and_provenance() {
    let model_id = ModelId::new_v7();
    let runtime =
        RecordingRuntime::new(model_id, false, Arc::new(RecordingSteeringHooks::default()));

    let err = capture(
        &runtime,
        model_id,
        vec!["prompt".to_string()],
        vec![LayerIndex::new(0)],
    )
    .await
    .expect_err("unsupported runtime is rejected before hook dispatch");
    match err {
        ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert_eq!(capability, "activation_steering");
            assert!(
                adapter.contains("RecordingRuntime"),
                "adapter name must explain which runtime rejected the technique: {adapter}"
            );
        }
        other => panic!("expected capability error, got {other:?}"),
    }

    let supported =
        RecordingRuntime::new(model_id, true, Arc::new(RecordingSteeringHooks::default()));
    let empty_contrastive = vector_with_values(
        "empty-contrastive",
        LayerIndex::new(0),
        vec![0.1, 0.2],
        SteeringProvenance::Contrastive {
            positive_prompts: Vec::new(),
            negative_prompts: vec!["negative".to_string()],
            technique: ContrastiveTechnique::RepE,
        },
    );

    let err = register_steering_vector(&supported, model_id, empty_contrastive)
        .await
        .expect_err("empty contrastive provenance is rejected at public technique boundary");
    assert!(err.to_string().contains("positive_prompts"), "{err}");
}

#[test]
fn activation_steering_public_surface_exposes_flight_recorder_event_families() {
    assert_eq!(
        FR_EVT_LLM_INFER_STEER_CAPTURE,
        "FR-EVT-LLM-INFER-STEER-CAPTURE"
    );
    assert_eq!(
        FR_EVT_LLM_INFER_STEER_REGISTER,
        "FR-EVT-LLM-INFER-STEER-REGISTER"
    );
    assert_eq!(
        FR_EVT_LLM_INFER_STEER_ACTIVE,
        "FR-EVT-LLM-INFER-STEER-ACTIVE"
    );
    assert_eq!(FR_EVT_LLM_INFER_STEER_APPLY, "FR-EVT-LLM-INFER-STEER-APPLY");
    // MT-096: WITHDRAW completes the register/withdraw lifecycle symmetry.
    assert_eq!(
        FR_EVT_LLM_INFER_STEER_WITHDRAW,
        "FR-EVT-LLM-INFER-STEER-WITHDRAW"
    );
}

#[test]
fn activation_steering_contrastive_difference_validates_capture_shape() {
    let mut activations = BTreeMap::new();
    activations.insert(
        LayerIndex::new(4),
        vec![
            vec![2.0, 4.0],
            vec![4.0, 8.0],
            vec![1.0, 1.0],
            vec![3.0, 3.0],
        ],
    );
    let capture_result = CaptureResult {
        activations,
        tokens_seen: 4,
    };

    let direction = contrastive_difference_vector(&capture_result, LayerIndex::new(4), 2)
        .expect("mean positive minus mean negative works");
    assert_eq!(direction, vec![1.0, 4.0]);

    let err = contrastive_difference_vector(&capture_result, LayerIndex::new(4), 4)
        .expect_err("positive count must leave at least one negative row");
    assert!(err.to_string().contains("positive_count"), "{err}");
}

#[derive(Default)]
struct RecordingSteeringHooks {
    capture_specs: Mutex<Vec<CaptureSpec>>,
    vectors: Mutex<BTreeMap<SteeringVectorId, SteeringVector>>,
    active: Mutex<BTreeSet<SteeringVectorId>>,
}

#[async_trait]
impl SteeringHookOps for RecordingSteeringHooks {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        self.capture_specs.lock().unwrap().push(spec.clone());
        let mut activations = BTreeMap::new();
        for layer in &spec.layers {
            let rows = spec
                .prompts
                .iter()
                .map(|prompt| {
                    if prompt.contains("honest") {
                        vec![layer.as_u32() as f32, 2.0]
                    } else {
                        vec![layer.as_u32() as f32, 1.0]
                    }
                })
                .collect::<Vec<_>>();
            activations.insert(*layer, rows);
        }
        Ok(CaptureResult {
            activations,
            tokens_seen: spec.prompts.len() as u32,
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
        let vectors = self.vectors.lock().unwrap();
        for id in &ids {
            if !vectors.contains_key(id) {
                return Err(ModelRuntimeError::SteeringHookError(format!(
                    "unknown vector {id}"
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

struct RecordingRuntime {
    model_id: ModelId,
    capabilities: ModelCapabilities,
    hooks: SteeringHookHandle,
}

impl RecordingRuntime {
    fn new(
        model_id: ModelId,
        supports_activation_steering: bool,
        hooks: Arc<RecordingSteeringHooks>,
    ) -> Self {
        Self {
            model_id,
            capabilities: ModelCapabilities {
                supports_activation_steering,
                ..Default::default()
            },
            hooks: SteeringHookHandle::with_ops("recording-hooks", hooks),
        }
    }

    fn ensure_model(&self, id: ModelId) -> Result<(), ModelRuntimeError> {
        if id == self.model_id {
            Ok(())
        } else {
            Err(ModelRuntimeError::LoadError(format!("unknown model {id}")))
        }
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(self.model_id)
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(futures::stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(KvCacheHandle::new("recording-kv"))
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(LoraStackHandle::new("recording-lora"))
    }

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(self.hooks.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

fn vector_with_values(
    name: &str,
    layer: LayerIndex,
    values: Vec<f32>,
    provenance: SteeringProvenance,
) -> SteeringVector {
    SteeringVector::try_new(
        None,
        name,
        layer,
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(values, 1.0).expect("valid vector values"),
        "activation steering test vector",
        Some(provenance),
    )
    .expect("valid vector")
}

fn _load_spec() -> LoadSpec {
    LoadSpec {
        artifact_path: PathBuf::from("fixtures/models/unused.safetensors"),
        sha256_expected: "00".repeat(32),
        runtime_kind: RuntimeKind::Candle,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}
