use super::*;
use futures::StreamExt;
use handshake_core::llm::InvocationDrainOutcome;
use handshake_core::model_runtime::{
    activity::{RuntimeActivityKind, RuntimeActivityTracker},
    ModelArtifactComponentIntegrity, ModelArtifactIntegrityReceipt,
    RuntimeArtifactIntegrityReceipt, RuntimeExecutionProfile,
};
use handshake_core::storage::artifacts::{
    bundle_index_content_hash, bundle_index_json, BundleIndexEntry,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct ObservedRuntime {
    recording: RecordingRuntime,
    contexts: Mutex<HashMap<ModelId, u32>>,
    activity: RuntimeActivityTracker,
}

impl ObservedRuntime {
    fn new(models: &[(ModelId, u32)]) -> Arc<Self> {
        Arc::new(Self {
            recording: RecordingRuntime::new("component-bound-runtime", &["bound response"]),
            contexts: Mutex::new(models.iter().copied().collect()),
            activity: RuntimeActivityTracker::new(),
        })
    }

    fn weights(id: ModelId) -> Vec<u8> {
        format!("component fixture weights {id}").into_bytes()
    }
}

#[async_trait]
impl ModelRuntime for ObservedRuntime {
    async fn load(
        &mut self,
        spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        self.recording.load(spec).await
    }
    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError> {
        self.recording.unload(id).await
    }
    fn generate(&self, req: GenerateRequest) -> TokenStream {
        let guard = self
            .activity
            .try_register(
                req.id,
                RuntimeActivityKind::Generate,
                Some(req.cancel.clone()),
            )
            .expect("component runtime admits owned generation");
        Box::pin(self.recording.generate(req).map(move |item| {
            let _ = &guard;
            item
        }))
    }
    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        self.recording.score(id, sequence).await
    }
    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        self.recording.embed(id, text).await
    }
    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.recording.capabilities(id)
    }
    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        self.recording.kv_cache(id)
    }
    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        self.recording.lora_stack(id)
    }
    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        self.recording.steering_hooks(id)
    }
    fn cancel(&self, token: CancellationToken) {
        self.recording.cancel(token);
    }
    fn execution_profile(&self, id: ModelId) -> Result<RuntimeExecutionProfile, ModelRuntimeError> {
        let context = self
            .contexts
            .lock()
            .unwrap()
            .get(&id)
            .copied()
            .ok_or_else(|| ModelRuntimeError::CapabilityNotSupported {
                capability: "loaded_runtime_execution_profile".into(),
                adapter: "component fixture".into(),
            })?;
        Ok(RuntimeExecutionProfile {
            max_context_tokens: context,
            context_source: "component fixture loaded configuration".into(),
            supports_streaming: true,
            configuration: serde_json::json!({"model": id, "context": context}),
        })
    }
    fn artifact_integrity(
        &self,
        id: ModelId,
    ) -> Result<RuntimeArtifactIntegrityReceipt, ModelRuntimeError> {
        let weights_bytes = Self::weights(id);
        let config_bytes = serde_json::to_vec(&self.execution_profile(id)?.configuration).unwrap();
        let component = |bytes: &[u8]| ModelArtifactComponentIntegrity {
            sha256: hex::encode(Sha256::digest(bytes)),
            length_bytes: bytes.len() as u64,
        };
        let weights = component(&weights_bytes);
        let config = component(&config_bytes);
        let index = bundle_index_json(&[
            BundleIndexEntry {
                path: "model.safetensors".into(),
                content_hash: weights.sha256.clone(),
                size_bytes: weights.length_bytes,
            },
            BundleIndexEntry {
                path: "config.json".into(),
                content_hash: config.sha256.clone(),
                size_bytes: config.length_bytes,
            },
        ])
        .unwrap();
        Ok(RuntimeArtifactIntegrityReceipt::Candle(
            ModelArtifactIntegrityReceipt {
                schema_id: "handshake.model_artifact_integrity.candle.v1".into(),
                bundle_sha256: bundle_index_content_hash(&index),
                weights,
                config,
                tokenizer: None,
            },
        ))
    }
}

fn make_client(
    registry: Arc<ModelRegistry>,
    runtime: Arc<ObservedRuntime>,
    selected: ModelId,
) -> LocalModelRuntimeLlmClient {
    LocalModelRuntimeLlmClient::new(
        LocalRouter::new(
            registry.clone(),
            Arc::new(RecordingRuntime::new("unused llama", &[])),
            runtime,
        ),
        Arc::new(RecordingFallbackClient::new("unused fallback")),
        Arc::new(CapturingRecorder::default()),
        ModelProfile::new(selected.to_string(), 8192),
    )
    .with_catalog(ModelCatalog::from_registry(registry))
}

fn registered(models: &[ModelId]) -> (Arc<ModelRegistry>, Vec<ModelRegistration>) {
    let mut registry = ModelRegistry::default();
    let mut registrations = Vec::new();
    for id in models {
        let mut entry = registration(*id, RuntimeBinding::Candle);
        entry.sha256 = Sha256::digest(ObservedRuntime::weights(*id)).into();
        entry.base_model_tag = BaseModelTag::new(format!("component-{id}"));
        registry.register(entry.clone()).unwrap();
        registry.mark_loaded(*id).unwrap();
        registrations.push(entry);
    }
    (Arc::new(registry), registrations)
}

#[tokio::test]
async fn bound_selection_uses_current_profile_and_retains_swap_custody() {
    let mut store_scope = IsolatedSurrealTestStore::create().await.unwrap();
    let storage = store_scope.storage().clone();
    bootstrap_model_registry_schema(&storage).await.unwrap();
    let scope = exact_local_routing_scope();
    let store = ModelRegistryStore::new(storage);
    store.ensure_workspace_for_tests(&scope).await.unwrap();
    let first = ModelId::new_v7();
    let second = ModelId::new_v7();
    let (registry, entries) = registered(&[first, second]);
    store
        .persist_boot_set_and_read_back(&scope, &entries)
        .await
        .unwrap();
    let selection = store
        .ensure_active_defaults(
            &scope,
            &[(
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                entries[0].sha256,
            )],
        )
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .unwrap();
    let runtime = ObservedRuntime::new(&[(first, 4096), (second, 16384)]);
    let client = Arc::new(
        make_client(registry, runtime.clone(), first).with_durable_application_selection(
            ScopedModelRegistryAuthority::new(store.clone(), scope.clone()),
            selection.selection_revision,
        ),
    );
    client
        .swap_model(ready_model_swap_request(
            first,
            second,
            selection.selection_revision,
        ))
        .await
        .unwrap();
    let binding = client
        .bind_selected_model(tokio::time::Instant::now() + Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(binding.profile().model_id, second.to_string());
    assert_eq!(binding.profile().max_context_tokens, 16384);
    assert_eq!(
        client.profile().max_context_tokens,
        8192,
        "construction profile deliberately differs"
    );
    assert_eq!(
        binding.provenance()["selection_revision"],
        selection.selection_revision + 1
    );
    assert_eq!(
        binding.provenance()["runtime_execution_profile"]["configuration"]["model"],
        second.to_string()
    );
    let (completion, control) = client
        .clone()
        .owned_completion_with_binding(
            CompletionRequest::new(
                uuid::Uuid::now_v7(),
                "bound component call".into(),
                second.to_string(),
            ),
            binding.clone(),
            tokio::time::Instant::now() + Duration::from_secs(5),
        )
        .into_parts();
    drop(binding);
    assert_eq!(completion.await.unwrap().text, "bound response");
    assert!(matches!(
        control.drain().await,
        InvocationDrainOutcome::LocalResourcesSettled {
            deadline_exceeded: false,
            ..
        }
    ));
    assert_eq!(runtime.recording.last_request().id, second);
    {
        let swap = client.swap_model(ready_model_swap_request(
            second,
            first,
            selection.selection_revision + 1,
        ));
        tokio::pin!(swap);
        tokio::select! {
            biased;
            result = &mut swap => panic!("swap escaped retained invocation custody: {result:?}"),
            _ = tokio::time::sleep(Duration::from_millis(30)) => {}
        }
        assert_eq!(client.selected_model_id(), second.to_string());
        drop(control);
        tokio::time::timeout(Duration::from_secs(5), &mut swap)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(client.selected_model_id(), first.to_string());
    let current = store.list_active_selections(&scope).await.unwrap();
    let current = current
        .iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .unwrap();
    assert_eq!(current.selection_revision, selection.selection_revision + 2);
    assert_eq!(current.artifact_sha256, entries[0].sha256);
    drop(client);
    drop(store);
    store_scope.cleanup().await.unwrap();
}

#[tokio::test]
async fn bound_selection_rejects_foreign_owner_changed_configuration_and_unknown_capacity() {
    let model = ModelId::new_v7();
    let (registry, _) = registered(&[model]);
    let runtime = ObservedRuntime::new(&[(model, 4096)]);
    let client = Arc::new(make_client(registry.clone(), runtime.clone(), model));
    let foreign = make_client(registry, runtime.clone(), model);
    let binding = client
        .bind_selected_model(tokio::time::Instant::now() + Duration::from_secs(5))
        .await
        .unwrap();
    assert!(foreign.revalidate_model_binding(&binding).is_err());
    runtime.contexts.lock().unwrap().insert(model, 8192);
    assert!(client.revalidate_model_binding(&binding).is_err());
    let (completion, control) = client
        .clone()
        .owned_completion_with_binding(
            CompletionRequest::new(
                uuid::Uuid::now_v7(),
                "must not dispatch".into(),
                model.to_string(),
            ),
            binding.clone(),
            tokio::time::Instant::now() + Duration::from_secs(5),
        )
        .into_parts();
    assert!(completion.await.is_err());
    assert_eq!(
        control.drain().await,
        InvocationDrainOutcome::NeverSubmitted
    );
    assert_eq!(runtime.recording.request_count(), 0);
    drop(control);
    drop(binding);
    runtime.contexts.lock().unwrap().insert(model, 0);
    assert!(client
        .bind_selected_model(tokio::time::Instant::now() + Duration::from_secs(5))
        .await
        .is_err());
    runtime.contexts.lock().unwrap().remove(&model);
    assert!(client
        .bind_selected_model(tokio::time::Instant::now() + Duration::from_secs(5))
        .await
        .is_err());
    assert_eq!(runtime.recording.request_count(), 0);
}
