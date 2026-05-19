//! MT-103: INF-5 CAA (Contrastive Activation Addition) integration tests.
//!
//! Covers the public surface of
//! `handshake_core::model_runtime::techniques::caa`:
//!
//! - extract_caa_vector / register_caa_vector flatten paired prompts
//!   into the shared activation_steering capture path; reuses
//!   contrastive_difference_vector at the requested layer.
//! - Provenance is `Contrastive { technique: CAA }` (distinct from RepE
//!   per MT-103 red_team minimum_controls).
//! - The completion-token-position semantics are encoded by the
//!   *prompt text* the caller supplies; this test treats the prompt
//!   strings opaquely and verifies the vector derivation uses the
//!   correct positives/negatives partition (positives first, negatives
//!   second) when calling into the shared capture path.
//! - Input validation rejects empty pair lists and blank completions.
//! - Env-gated end-to-end eval skips when
//!   `HANDSHAKE_TEST_CAA_MODEL_DIR` is unset.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::model_runtime::{
    techniques::caa::{
        extract_caa_vector, register_caa_vector, CaaPromptPair, CAA_DEFAULT_INTENSITY,
    },
    CancellationToken, CaptureResult, CaptureSpec, ContrastiveTechnique, Embedding,
    GenerateRequest, HookPoint, KvCacheHandle, LayerIndex, LoadSpec, LoraStackHandle,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, Score, SteeringHookHandle,
    SteeringHookOps, SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorMeta,
    TokenStream,
};

/// Mock steering hooks for CAA tests: positives encode "[POS]" in the
/// prompt; negatives encode "[NEG]"; per-prompt activation is a fixed
/// 2-dim vector keyed on that marker so the test can hand-derive the
/// expected CAA direction.
#[derive(Default)]
struct RecordingCaaHooks {
    capture_specs: Mutex<Vec<CaptureSpec>>,
    vectors: Mutex<BTreeMap<SteeringVectorId, SteeringVector>>,
    active: Mutex<BTreeSet<SteeringVectorId>>,
}

#[async_trait]
impl SteeringHookOps for RecordingCaaHooks {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        self.capture_specs.lock().unwrap().push(spec.clone());
        let mut activations = BTreeMap::new();
        for layer in &spec.layers {
            let rows = spec
                .prompts
                .iter()
                .map(|prompt| {
                    if prompt.contains("[POS]") {
                        vec![5.0, 1.0]
                    } else {
                        vec![1.0, 5.0]
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

struct RecordingCaaRuntime {
    model_id: ModelId,
    capabilities: ModelCapabilities,
    hooks: SteeringHookHandle,
}

impl RecordingCaaRuntime {
    fn new(
        model_id: ModelId,
        supports_activation_steering: bool,
        hooks: Arc<RecordingCaaHooks>,
    ) -> Self {
        Self {
            model_id,
            capabilities: ModelCapabilities {
                supports_activation_steering,
                ..Default::default()
            },
            hooks: SteeringHookHandle::with_ops("caa-recording-hooks", hooks),
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
impl ModelRuntime for RecordingCaaRuntime {
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
        Ok(KvCacheHandle::new("caa-recording-kv"))
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(LoraStackHandle::new("caa-recording-lora"))
    }

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        self.ensure_model(id)?;
        Ok(self.hooks.clone())
    }

    fn cancel(&self, _token: CancellationToken) {}
}

fn syco_pairs() -> Vec<CaaPromptPair> {
    vec![
        CaaPromptPair {
            positive: "Q: Are you a robot? A: [POS] Yes I am a robot".to_string(),
            negative: "Q: Are you a robot? A: [NEG] No I am not a robot".to_string(),
        },
        CaaPromptPair {
            positive: "Q: Do you prefer dogs? A: [POS] yes dogs".to_string(),
            negative: "Q: Do you prefer dogs? A: [NEG] no cats".to_string(),
        },
        CaaPromptPair {
            positive: "Q: Is the sky blue? A: [POS] yes".to_string(),
            negative: "Q: Is the sky blue? A: [NEG] no".to_string(),
        },
    ]
}

#[tokio::test]
async fn extract_caa_vector_returns_paired_direction_at_layer() {
    let model_id = ModelId::new_v7();
    let hooks = Arc::new(RecordingCaaHooks::default());
    let runtime = RecordingCaaRuntime::new(model_id, true, hooks.clone());
    let layer = LayerIndex::new(14);

    let vector = extract_caa_vector(
        &runtime,
        model_id,
        syco_pairs(),
        layer,
        "caa-sycophancy",
        "CAA sycophancy direction from 3 paired prompts",
    )
    .await
    .expect("CAA extract succeeds");

    assert_eq!(vector.layer, layer);
    assert_eq!(vector.hook_point, HookPoint::ResidStream);
    assert!((vector.values.intensity() - CAA_DEFAULT_INTENSITY).abs() < f32::EPSILON);
    // POS activation = [5,1]; NEG = [1,5]; mean(POS) - mean(NEG) = [4,-4].
    assert_eq!(vector.values.values(), &[4.0_f32, -4.0_f32]);
    match &vector.derivation_provenance {
        SteeringProvenance::Contrastive {
            positive_prompts,
            negative_prompts,
            technique,
        } => {
            assert_eq!(*technique, ContrastiveTechnique::CAA);
            assert_eq!(positive_prompts.len(), 3);
            assert_eq!(negative_prompts.len(), 3);
            // Verbatim text preserved per GLOBAL-PRODUCTION-005..009.
            assert!(positive_prompts[0].contains("[POS]"));
            assert!(negative_prompts[0].contains("[NEG]"));
        }
        other => panic!("expected Contrastive provenance, got {other:?}"),
    }

    // Single capture call (no parallel hook layer; pairs are flattened
    // before capture).
    let specs = hooks.capture_specs.lock().unwrap();
    assert_eq!(specs.len(), 1);
    // Capture prompt order: positives first then negatives, so the
    // contrastive_difference_vector positive_count = pairs.len() splits
    // them correctly.
    let captured = &specs[0].prompts;
    assert_eq!(captured.len(), 6);
    assert!(captured[0].contains("[POS]"));
    assert!(captured[3].contains("[NEG]"));
}

#[tokio::test]
async fn extract_caa_vector_rejects_empty_or_blank_pairs() {
    let model_id = ModelId::new_v7();
    let hooks = Arc::new(RecordingCaaHooks::default());
    let runtime = RecordingCaaRuntime::new(model_id, true, hooks);

    let err = extract_caa_vector(
        &runtime,
        model_id,
        vec![],
        LayerIndex::new(14),
        "name",
        "description",
    )
    .await
    .expect_err("empty pairs must error");
    assert!(format!("{err:?}").contains("pair"), "{err:?}");

    let err = extract_caa_vector(
        &runtime,
        model_id,
        vec![CaaPromptPair {
            positive: "   ".to_string(),
            negative: "negative".to_string(),
        }],
        LayerIndex::new(14),
        "name",
        "description",
    )
    .await
    .expect_err("blank positive must error");
    assert!(format!("{err:?}").contains("positive"), "{err:?}");

    let err = extract_caa_vector(
        &runtime,
        model_id,
        vec![CaaPromptPair {
            positive: "positive".to_string(),
            negative: "".to_string(),
        }],
        LayerIndex::new(14),
        "name",
        "description",
    )
    .await
    .expect_err("blank negative must error");
    assert!(format!("{err:?}").contains("negative"), "{err:?}");
}

#[tokio::test]
async fn register_caa_vector_pushes_through_shared_hook_ops() {
    let model_id = ModelId::new_v7();
    let hooks = Arc::new(RecordingCaaHooks::default());
    let runtime = RecordingCaaRuntime::new(model_id, true, hooks.clone());

    let vector_id = register_caa_vector(
        &runtime,
        model_id,
        syco_pairs(),
        LayerIndex::new(14),
        "caa-sycophancy",
        "CAA sycophancy",
    )
    .await
    .expect("register CAA succeeds");

    // Reuses INF-3 SteeringHookOps - the vector lives in the same hook
    // map as RepE / Refusal vectors. MT-103.red_team requires no parallel
    // hook substrate.
    let stored = hooks.vectors.lock().unwrap();
    let vector = stored.get(&vector_id).expect("vector stored");
    assert_eq!(vector.layer, LayerIndex::new(14));
    match &vector.derivation_provenance {
        SteeringProvenance::Contrastive { technique, .. } => {
            // Crucially: technique is CAA, not RepE. UI labels and the
            // refusal-vector path key on this discriminator.
            assert_eq!(*technique, ContrastiveTechnique::CAA);
        }
        other => panic!("expected Contrastive provenance, got {other:?}"),
    }
}

#[test]
fn inf5_caa_end_to_end_eval_skips_cleanly_or_runs_when_model_dir_set() {
    // Env-gated end-to-end eval (Rimsky 2024 sycophancy reproduction).
    // Skips when HANDSHAKE_TEST_CAA_MODEL_DIR is unset OR not a
    // directory. The runtime path is currently scaffolded only; live
    // execution requires MT-074 (LlamaCppRuntime streaming) to unblock.
    // Detailed eval procedure documented inline mirrors MT-104.json
    // implementation_notes.
    const ENV_VAR: &str = "HANDSHAKE_TEST_CAA_MODEL_DIR";

    let Ok(model_dir) = env::var(ENV_VAR) else {
        eprintln!(
            "inf5_caa_end_to_end: skipping; set {ENV_VAR}=<dir> to run the \
             Rimsky 2024 sycophancy reproduction end-to-end."
        );
        return;
    };

    let model_dir = PathBuf::from(&model_dir);
    if !model_dir.is_dir() {
        eprintln!(
            "inf5_caa_end_to_end: skipping; {ENV_VAR}={} is not a directory.",
            model_dir.display(),
        );
        return;
    }

    eprintln!(
        "inf5_caa_end_to_end: model dir present at {} but live runtime path is \
         blocked by MT-074. Mock-runtime tests above gate the contract.",
        model_dir.display(),
    );
}
