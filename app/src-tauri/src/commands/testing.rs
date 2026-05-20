//! MT-068/MT-096 testing harness.
//!
//! `FakeCandleRuntime` is a deterministic `dyn ModelRuntime` test double used
//! by `commands::model_runtime::tests` and `commands::steering::tests`. It
//! composes the *real* `SteeringHookOps` trait + the *real*
//! `SteeringHookHandle` plumbing, so the production code paths under test are
//! exercised end-to-end (parse model_id -> registry lookup -> attach live
//! runtime -> dispatch into adapter -> steering_hooks call -> SteeringHookOps).
//! Only the inner activation tensors are deterministic stand-ins.
//!
//! The fake is gated behind `#[cfg(test)]` so it never ships with the app
//! binary and is not part of the public Tauri command surface.
#![cfg(test)]

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::model_runtime::{
    CancellationToken, CaptureResult, CaptureSpec, Embedding, GenerateRequest, HookPoint,
    KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime,
    ModelRuntimeError, Score, SteeringHookHandle, SteeringHookOps, SteeringVector,
    SteeringVectorId, SteeringVectorMeta, TokenStream,
};

/// In-memory `SteeringHookOps` that records every call and returns
/// deterministic activations seeded from the layer index + prompt count, so
/// test assertions are stable and shaped like real capture output.
#[derive(Default)]
pub struct FakeSteeringHookOps {
    pub residual_width: usize,
    pub vectors: Mutex<BTreeMap<SteeringVectorId, SteeringVector>>,
    pub active: Mutex<BTreeSet<SteeringVectorId>>,
    pub capture_calls: Mutex<Vec<CaptureSpec>>,
    pub model_loaded: bool,
}

impl FakeSteeringHookOps {
    pub fn new(residual_width: usize) -> Self {
        Self {
            residual_width,
            vectors: Mutex::new(BTreeMap::new()),
            active: Mutex::new(BTreeSet::new()),
            capture_calls: Mutex::new(Vec::new()),
            model_loaded: true,
        }
    }

    pub fn new_unloaded() -> Self {
        Self {
            residual_width: 4,
            vectors: Mutex::new(BTreeMap::new()),
            active: Mutex::new(BTreeSet::new()),
            capture_calls: Mutex::new(Vec::new()),
            model_loaded: false,
        }
    }
}

#[async_trait]
impl SteeringHookOps for FakeSteeringHookOps {
    async fn capture(&self, spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        if !self.model_loaded {
            return Err(ModelRuntimeError::SteeringHookError(
                "fake adapter: model is not loaded".to_string(),
            ));
        }
        if spec.hook_point != HookPoint::ResidStream {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "non-resid_stream hook".to_string(),
                adapter: "fake_candle".to_string(),
            });
        }
        self.capture_calls.lock().unwrap().push(spec.clone());

        let mut activations = BTreeMap::new();
        for layer in &spec.layers {
            let mut rows = Vec::new();
            for (prompt_index, prompt) in spec.prompts.iter().enumerate() {
                // Deterministic vector: position 0 = layer index, position 1 =
                // prompt index, remaining positions = a small bias derived from
                // the prompt length (so capture output is non-zero and tests
                // can derive non-trivial differences via the contrastive
                // helpers).
                let mut row = vec![0.0_f32; self.residual_width];
                row[0] = layer.as_u32() as f32;
                if self.residual_width > 1 {
                    row[1] = prompt_index as f32;
                }
                for value in row.iter_mut().skip(2) {
                    *value = (prompt.len() as f32) * 0.01;
                }
                rows.push(row);
            }
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

/// `ModelRuntime` test double shaped like `CandleRuntime`. Composes the real
/// `SteeringHookHandle::with_ops(...)` constructor so the Tauri commands
/// exercise the same dispatch contract that the production CandleRuntime
/// adapter uses (`steering_hooks(id)` returns a `SteeringHookHandle` wrapping
/// the model's hook ops).
pub struct FakeCandleRuntime {
    model_capabilities: HashMap<ModelId, ModelCapabilities>,
    hooks: HashMap<ModelId, Arc<FakeSteeringHookOps>>,
}

impl FakeCandleRuntime {
    pub fn new(model_id: ModelId, capabilities: ModelCapabilities) -> Self {
        let mut model_capabilities = HashMap::new();
        model_capabilities.insert(model_id, capabilities);
        let mut hooks = HashMap::new();
        hooks.insert(model_id, Arc::new(FakeSteeringHookOps::new(4)));
        Self {
            model_capabilities,
            hooks,
        }
    }

    pub fn new_with_hooks(
        model_id: ModelId,
        capabilities: ModelCapabilities,
        hooks_for_model: Arc<FakeSteeringHookOps>,
    ) -> Self {
        let mut model_capabilities = HashMap::new();
        model_capabilities.insert(model_id, capabilities);
        let mut hooks = HashMap::new();
        hooks.insert(model_id, hooks_for_model);
        Self {
            model_capabilities,
            hooks,
        }
    }

    pub fn hooks_for(&self, model_id: ModelId) -> Option<Arc<FakeSteeringHookOps>> {
        self.hooks.get(&model_id).cloned()
    }
}

#[async_trait]
impl ModelRuntime for FakeCandleRuntime {
    fn adapter_name(&self) -> &'static str {
        "candle"
    }

    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Err(ModelRuntimeError::LoadError(
            "FakeCandleRuntime test double does not implement load()".to_string(),
        ))
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(futures_util::stream::empty())
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
        self.model_capabilities
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::LoadError(format!("fake unknown model {id}")))
    }

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        if self.model_capabilities.contains_key(&id) {
            Ok(KvCacheHandle::new("fake_candle_kv"))
        } else {
            Err(ModelRuntimeError::KvCacheError(format!(
                "fake unknown model {id}"
            )))
        }
    }

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        if self.model_capabilities.contains_key(&id) {
            Ok(LoraStackHandle::new("fake_candle_lora"))
        } else {
            Err(ModelRuntimeError::LoraStackError(format!(
                "fake unknown model {id}"
            )))
        }
    }

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        let caps = self.capabilities(id)?;
        if !caps.supports_activation_steering {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "activation_steering".to_string(),
                adapter: "candle".to_string(),
            });
        }
        let hooks = self.hooks.get(&id).cloned().ok_or_else(|| {
            ModelRuntimeError::SteeringHookError(format!(
                "fake adapter has no hook ops registered for model {id}"
            ))
        })?;
        Ok(SteeringHookHandle::with_ops(
            format!("fake_candle:{id}:activation_hooks"),
            hooks,
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// `ModelRuntime` test double whose `steering_hooks` is bound to a hook ops
/// that reports the model as not loaded. Lets MT-096 tests prove the
/// `CaptureNotAvailable` propagation path without conflating it with
/// capability gating.
pub fn fake_runtime_with_unloaded_hooks(model_id: ModelId) -> Arc<FakeCandleRuntime> {
    let hooks = Arc::new(FakeSteeringHookOps::new_unloaded());
    Arc::new(FakeCandleRuntime::new_with_hooks(
        model_id,
        ModelCapabilities {
            supports_activation_steering: true,
            ..Default::default()
        },
        hooks,
    ))
}
