use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use futures_util::stream;
use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    model_runtime::{
        techniques::lora_hotswap::{
            self, FR_EVT_LLM_INFER_LORA_MOUNT, FR_EVT_LLM_INFER_LORA_SWAP,
            FR_EVT_LLM_INFER_LORA_UNMOUNT,
        },
        BaseModelTag, CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LicenseTag,
        LoadSpec, LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackOps,
        LoraStackSnapshot, LoraStackSnapshotEntry, LoraStrength, ModelCapabilities, ModelId,
        ModelRuntime, ModelRuntimeError, Score, SteeringHookHandle, TokenStream,
    },
};

#[tokio::test]
async fn lora_hotswap_techniques_mount_list_swap_and_unmount_dispatch_through_runtime_stack() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingLoraStack::with_base_model("local-test-base"));
    let runtime = RecordingRuntime::new(
        model_id,
        ModelCapabilities {
            supports_lora: true,
            ..Default::default()
        },
        stack.clone(),
    );
    let first = descriptor("story", "local-test-base");
    let first_id = first.id;
    let second = descriptor("domain", "local-test-base");

    let mounted = lora_hotswap::mount(&runtime, model_id, first.clone(), strength(0.75))
        .await
        .unwrap();
    assert_eq!(mounted.event_type, FR_EVT_LLM_INFER_LORA_MOUNT);
    assert_eq!(mounted.active_stack.len(), 1);
    assert_eq!(mounted.active_stack[0].id, first_id);

    let listed = lora_hotswap::list(&runtime, model_id).unwrap();
    assert_eq!(listed.active_stack.len(), 1);
    assert_eq!(listed.active_stack[0].id, first_id);

    let swapped = lora_hotswap::swap(&runtime, model_id, vec![(second.clone(), strength(0.5))])
        .await
        .unwrap();
    assert_eq!(swapped.event_type, FR_EVT_LLM_INFER_LORA_SWAP);
    assert_eq!(swapped.previous_stack.entries.len(), 1);
    assert_eq!(swapped.previous_stack.entries[0].descriptor.id, first_id);
    assert_eq!(swapped.active_stack.len(), 1);
    assert_eq!(swapped.active_stack[0].id, second.id);

    let unmounted = lora_hotswap::unmount(&runtime, model_id, second.id)
        .await
        .unwrap();
    assert_eq!(unmounted.event_type, FR_EVT_LLM_INFER_LORA_UNMOUNT);
    assert!(unmounted.active_stack.is_empty());
    assert_eq!(
        FrEventId::from_str_id(FR_EVT_LLM_INFER_LORA_SWAP)
            .unwrap()
            .as_str(),
        FR_EVT_LLM_INFER_LORA_SWAP
    );

    assert_eq!(
        stack.calls.lock().unwrap().as_slice(),
        ["mount", "swap", "unmount"]
    );
}

#[tokio::test]
async fn lora_hotswap_techniques_fail_closed_when_lora_capability_is_absent() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingLoraStack::with_base_model("local-test-base"));
    let runtime = RecordingRuntime::new(model_id, ModelCapabilities::default(), stack.clone());

    let error = lora_hotswap::mount(
        &runtime,
        model_id,
        descriptor("story", "local-test-base"),
        strength(1.0),
    )
    .await
    .expect_err("supports_lora=false must reject before mutation");

    assert!(matches!(
        error,
        ModelRuntimeError::CapabilityNotSupported { .. }
    ));
    assert!(stack.list_active().is_empty());
    assert!(
        stack.calls.lock().unwrap().is_empty(),
        "capability gating must happen before touching the adapter stack"
    );
}

struct RecordingRuntime {
    model_capabilities: HashMap<ModelId, ModelCapabilities>,
    stack: Arc<RecordingLoraStack>,
}

impl RecordingRuntime {
    fn new(
        model_id: ModelId,
        capabilities: ModelCapabilities,
        stack: Arc<RecordingLoraStack>,
    ) -> Self {
        Self {
            model_capabilities: HashMap::from([(model_id, capabilities)]),
            stack,
        }
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    fn adapter_name(&self) -> &'static str {
        "recording"
    }

    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Err(ModelRuntimeError::LoadError(
            "recording runtime does not load models".to_string(),
        ))
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(stream::empty())
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
            .ok_or_else(|| ModelRuntimeError::LoadError(format!("unknown model {id}")))
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("recording-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::with_ops(
            "recording-lora-stack",
            self.stack.clone(),
        ))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("recording-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

struct RecordingLoraStack {
    base_model: BaseModelTag,
    active: Mutex<Vec<LoraStackSnapshotEntry>>,
    calls: Mutex<Vec<&'static str>>,
}

impl RecordingLoraStack {
    fn with_base_model(base_model: impl Into<String>) -> Self {
        Self {
            base_model: BaseModelTag::new(base_model),
            active: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl LoraStackOps for RecordingLoraStack {
    async fn mount(
        &self,
        desc: LoraDescriptor,
        strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        self.calls.lock().unwrap().push("mount");
        if desc.base_model_compat != self.base_model {
            return Err(ModelRuntimeError::LoraStackError(
                "base model mismatch".to_string(),
            ));
        }
        self.active.lock().unwrap().push(LoraStackSnapshotEntry {
            descriptor: desc,
            strength,
            mounted_at_utc: Utc::now(),
        });
        Ok(())
    }

    async fn unmount(&self, id: LoraId) -> Result<(), ModelRuntimeError> {
        self.calls.lock().unwrap().push("unmount");
        self.active
            .lock()
            .unwrap()
            .retain(|entry| entry.descriptor.id != id);
        Ok(())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        self.active
            .lock()
            .unwrap()
            .iter()
            .map(|entry| LoraStackEntry {
                id: entry.descriptor.id,
                strength: entry.strength.clone(),
                mounted_at_utc: entry.mounted_at_utc,
            })
            .collect()
    }

    async fn set_strength(
        &self,
        _id: LoraId,
        _strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        Err(ModelRuntimeError::LoraStackError(
            "set_strength not used by public hotswap tests".to_string(),
        ))
    }

    async fn swap(
        &self,
        new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        self.calls.lock().unwrap().push("swap");
        let previous = LoraStackSnapshot {
            entries: self.active.lock().unwrap().clone(),
        };
        *self.active.lock().unwrap() = new_stack
            .into_iter()
            .map(|(descriptor, strength)| LoraStackSnapshotEntry {
                descriptor,
                strength,
                mounted_at_utc: Utc::now(),
            })
            .collect();
        Ok(previous)
    }
}

fn descriptor(name: &str, base_model: &str) -> LoraDescriptor {
    LoraDescriptor {
        id: LoraId::new_v7(),
        artifact_path: PathBuf::from("loras").join(format!("{name}.safetensors")),
        sha256: [7; 32],
        rank: 8,
        target_modules: vec!["q_proj".to_string()],
        base_model_compat: BaseModelTag::new(base_model),
        license_tag: LicenseTag::new("operator-local"),
    }
}

fn strength(value: f32) -> LoraStrength {
    LoraStrength::try_new(value).unwrap()
}
