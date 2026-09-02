use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod surreal_test_store_support;

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use handshake_core::{
    flight_recorder::{
        EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
        FlightRecorderEventType, RecorderError,
    },
    llm::{
        local_router::{LocalModelRuntimeLlmClient, LocalRouter},
        CompletionRequest, CompletionResponse, DisabledLlmClient, EmbeddingRequest, LlmClient,
        LlmError, ModelProfile, TokenUsage,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, FinishReason, GenPrompt, GenerateRequest,
        GeneratedToken, KvCacheHandle, LoraStackHandle, ModelCapabilities, ModelCatalog, ModelId,
        ModelRegistration, ModelRegistry, ModelRegistryStore, ModelRuntime, ModelRuntimeError,
        ModelRuntimeSelectionPurpose, OperatorId, ProviderKind, RuntimeBinding,
        ScopedModelRegistryAuthority, Score, SteeringHookHandle, TokenStream,
    },
    storage::surreal::{bootstrap_model_registry_schema, bootstrap_schema},
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    },
    workflows::{
        ModelSwapPriority, ModelSwapRequestV0_4, ModelSwapRequesterSubsystem,
        ModelSwapRequesterV0_4, ModelSwapRole, ModelSwapStrategy,
    },
};
use surreal_test_store_support::EmbeddedSurrealTestScope;
use tokio::{sync::Notify, time::Duration};

#[derive(Clone)]
struct RecordingRuntime {
    label: &'static str,
    tokens: Vec<GeneratedToken>,
    capabilities: ModelCapabilities,
    requests: Arc<Mutex<Vec<GenerateRequest>>>,
    cancelled: Arc<Mutex<Vec<CancellationToken>>>,
    release_after_tokens: Option<Arc<Notify>>,
}

impl RecordingRuntime {
    fn new(label: &'static str, chunks: &[&str]) -> Self {
        let tokens = chunks
            .iter()
            .enumerate()
            .map(|(index, text)| GeneratedToken {
                token_id: index as u32,
                text: (*text).to_string(),
                logprob: None,
                finish_reason: (index + 1 == chunks.len()).then_some(FinishReason::Stop),
            })
            .collect();
        Self {
            label,
            tokens,
            capabilities: ModelCapabilities::default(),
            requests: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(Mutex::new(Vec::new())),
            release_after_tokens: None,
        }
    }

    fn new_blocking(
        label: &'static str,
        chunks: &[&str],
        release_after_tokens: Arc<Notify>,
    ) -> Self {
        let mut runtime = Self::new(label, chunks);
        runtime.release_after_tokens = Some(release_after_tokens);
        runtime
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests lock").len()
    }

    fn last_request(&self) -> GenerateRequest {
        self.requests
            .lock()
            .expect("requests lock")
            .last()
            .cloned()
            .unwrap_or_else(|| panic!("expected {} runtime request", self.label))
    }

    fn cancel_count(&self) -> usize {
        self.cancelled.lock().expect("cancel lock").len()
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.requests.lock().expect("requests lock").push(req);
        let tokens = self.tokens.clone();
        let Some(release_after_tokens) = self.release_after_tokens.clone() else {
            return Box::pin(stream::iter(tokens.into_iter().map(Ok)));
        };

        Box::pin(stream::unfold(
            (0_usize, tokens, release_after_tokens),
            |(index, tokens, release_after_tokens)| async move {
                if index < tokens.len() {
                    return Some((
                        Ok(tokens[index].clone()),
                        (index + 1, tokens, release_after_tokens),
                    ));
                }
                release_after_tokens.notified().await;
                None
            },
        ))
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

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new(format!("{}-kv", self.label)))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new(format!("{}-lora", self.label)))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new(format!("{}-steering", self.label)))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
        self.cancelled.lock().expect("cancel lock").push(token);
    }
}

#[derive(Clone)]
struct RecordingFallbackClient {
    response: String,
    profile: ModelProfile,
    requests: Arc<Mutex<Vec<CompletionRequest>>>,
    cancelled_model_ids: Arc<Mutex<Vec<String>>>,
}

impl RecordingFallbackClient {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            profile: ModelProfile::new("fallback".to_string(), 4096),
            requests: Arc::new(Mutex::new(Vec::new())),
            cancelled_model_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests lock").len()
    }

    fn cancel_count(&self) -> usize {
        self.cancelled_model_ids.lock().expect("cancel lock").len()
    }
}

#[async_trait]
impl LlmClient for RecordingFallbackClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.requests.lock().expect("requests lock").push(req);
        Ok(CompletionResponse {
            text: self.response.clone(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 2,
                total_tokens: 3,
            },
            latency_ms: 0,
        })
    }

    fn cancel(&self, model_id: &str, token: CancellationToken) {
        token.cancel();
        self.cancelled_model_ids
            .lock()
            .expect("cancel lock")
            .push(model_id.to_string());
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Clone, Default)]
struct CapturingRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
}

impl CapturingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

fn capabilities(activation_steering: bool) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_activation_steering: activation_steering,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    }
}

fn registration(model_id: ModelId, binding: RuntimeBinding) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("fixtures/models/local-routing.gguf"),
        sha256: [9; 32],
        runtime_binding: binding,
        declared_capabilities: capabilities(binding == RuntimeBinding::Candle),
        base_model_tag: BaseModelTag::new("local-routing-base"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("operator-ilja"),
        provider: ProviderKind::Local,
    }
}

fn embedding_registration(
    model_id: ModelId,
    binding: RuntimeBinding,
    embedding_dimension: usize,
) -> ModelRegistration {
    let mut registration = registration(model_id, binding);
    registration.declared_capabilities.supports_embedding = true;
    registration.declared_capabilities.embedding_dimension = Some(embedding_dimension);
    registration
}

fn client_for_registry(
    registry: ModelRegistry,
    llama: Arc<RecordingRuntime>,
    candle: Arc<RecordingRuntime>,
    fallback: Arc<RecordingFallbackClient>,
    recorder: Arc<CapturingRecorder>,
) -> LocalModelRuntimeLlmClient {
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder,
        ModelProfile::new("local-router".to_string(), 8192).with_streaming(true),
    )
}

async fn wait_for_runtime_request(runtime: &RecordingRuntime) -> GenerateRequest {
    for _ in 0..100 {
        if runtime.request_count() > 0 {
            return runtime.last_request();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for runtime request");
}

fn exact_local_routing_scope() -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new("WS-LLM-LOCAL-ROUTING")
            .expect("valid local-routing workspace id"),
    }
}

fn ready_model_swap_request(
    current: ModelId,
    target: ModelId,
    expected_selection_revision: u64,
) -> ModelSwapRequestV0_4 {
    ready_model_swap_request_with_id(
        current,
        target,
        expected_selection_revision,
        uuid::Uuid::now_v7(),
    )
}

fn ready_model_swap_request_with_id(
    current: ModelId,
    target: ModelId,
    expected_selection_revision: u64,
    selection_request_id: uuid::Uuid,
) -> ModelSwapRequestV0_4 {
    let request_id = selection_request_id.to_string();
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert(
        "actor".to_owned(),
        serde_json::json!("native-model-runtime-panel"),
    );
    metadata.insert(
        "selection_request_id".to_owned(),
        serde_json::json!(request_id.clone()),
    );
    metadata.insert(
        "expected_selection_revision".to_owned(),
        serde_json::json!(expected_selection_revision),
    );
    ModelSwapRequestV0_4 {
        schema_version: "hsk.model_swap@0.4".to_owned(),
        request_id,
        current_model_id: current.to_string(),
        target_model_id: target.to_string(),
        role: ModelSwapRole::Orchestrator,
        priority: ModelSwapPriority::Normal,
        reason: "operator READY-model selection".to_owned(),
        swap_strategy: ModelSwapStrategy::KeepHotSwap,
        state_persist_refs: vec!["model-runtime-selection://state/current".to_owned()],
        state_hash: "7".repeat(64),
        context_compile_ref: "model-runtime-panel://selection/test".to_owned(),
        max_vram_mb: 0,
        max_ram_mb: 0,
        timeout_ms: 10_000,
        requester: ModelSwapRequesterV0_4 {
            subsystem: ModelSwapRequesterSubsystem::Ui,
            job_id: None,
            wp_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".to_owned()),
            mt_id: Some("MT-014".to_owned()),
        },
        metadata: Some(metadata),
    }
}

#[tokio::test]
async fn ready_model_swap_is_serialized_audited_and_changes_default_routing() {
    let mut surreal_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated embedded-Surreal local-routing scope");
    let storage = surreal_scope
        .activate_storage()
        .await
        .expect("activate embedded-Surreal local-routing storage");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap shared embedded-Surreal schema");
    bootstrap_model_registry_schema(&storage)
        .await
        .expect("bootstrap embedded-Surreal model-registry schema");
    let exact_scope = exact_local_routing_scope();
    let registry_store = ModelRegistryStore::new(storage.clone());
    registry_store
        .ensure_workspace_for_tests(&exact_scope)
        .await
        .expect("seed exact local-routing workspace");

    let current = ModelId::new_v7();
    let target = ModelId::new_v7();
    let not_ready = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    let mut current_registration = registration(current, RuntimeBinding::Candle);
    current_registration.sha256 = [1; 32];
    current_registration.base_model_tag = BaseModelTag::new("current-ready");
    registry
        .register(current_registration.clone())
        .expect("register current READY model");
    let mut target_registration = registration(target, RuntimeBinding::Candle);
    target_registration.sha256 = [2; 32];
    target_registration.base_model_tag = BaseModelTag::new("target-ready");
    registry
        .register(target_registration.clone())
        .expect("register target READY model");
    let mut dormant_registration = registration(not_ready, RuntimeBinding::Candle);
    dormant_registration.sha256 = [3; 32];
    dormant_registration.base_model_tag = BaseModelTag::new("not-ready");
    registry
        .register(dormant_registration.clone())
        .expect("register non-READY model");

    registry_store
        .persist_boot_set_and_read_back(
            &exact_scope,
            &[
                current_registration.clone(),
                target_registration.clone(),
                dormant_registration,
            ],
        )
        .await
        .expect("persist local-routing registry in embedded Surreal");
    let initial_application_default = registry_store
        .ensure_active_defaults(
            &exact_scope,
            &[(
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                current_registration.sha256,
            )],
        )
        .await
        .expect("seed durable application default")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("seeded durable application default remains present");
    let initial_selection_revision = initial_application_default.selection_revision;

    registry.mark_loaded(current).expect("mark current loaded");
    registry.mark_loaded(target).expect("mark target loaded");

    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry(registry.clone());
    let runtime = Arc::new(RecordingRuntime::new("candle", &["target response"]));
    let router = LocalRouter::new(
        registry,
        Arc::new(RecordingRuntime::new("llama", &["wrong"])),
        runtime.clone(),
    );
    let recorder = Arc::new(CapturingRecorder::default());
    let client = LocalModelRuntimeLlmClient::new(
        router,
        Arc::new(RecordingFallbackClient::new("fallback")),
        recorder.clone(),
        ModelProfile::new(current.to_string(), 8192).with_streaming(true),
    )
    .with_catalog(catalog)
    .with_durable_application_selection(
        ScopedModelRegistryAuthority::new(registry_store.clone(), exact_scope.clone()),
        initial_selection_revision,
    );

    let selection_request_id = uuid::Uuid::now_v7();
    client
        .swap_model(ready_model_swap_request_with_id(
            current,
            target,
            initial_selection_revision,
            selection_request_id,
        ))
        .await
        .expect("switch between current READY models");
    assert_eq!(client.selected_model_id(), target.to_string());
    let durable_application_default = registry_store
        .list_active_selections(&exact_scope)
        .await
        .expect("read durable application default from the injected Surreal store")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("durable application default remains present");
    assert_eq!(durable_application_default.artifact_sha256, [2; 32]);
    assert_eq!(
        durable_application_default.selection_revision,
        initial_selection_revision + 1
    );

    let cancellation_count_before_retry = runtime.cancel_count();
    client
        .swap_model(ready_model_swap_request_with_id(
            target,
            target,
            initial_selection_revision,
            selection_request_id,
        ))
        .await
        .expect("identical durable selection retry returns stable success");
    assert_eq!(
        runtime.cancel_count(),
        cancellation_count_before_retry,
        "identical durable retry must not repeat runtime cancellation"
    );
    let durable_after_retry = registry_store
        .list_active_selections(&exact_scope)
        .await
        .expect("read durable application default after identical retry")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("durable application default remains present after retry");
    assert_eq!(durable_after_retry.artifact_sha256, [2; 32]);
    assert_eq!(
        durable_after_retry.selection_revision,
        initial_selection_revision + 1,
        "identical retry must preserve the committed revision"
    );

    let conflicting_retry = client
        .swap_model(ready_model_swap_request(
            target,
            target,
            initial_selection_revision,
        ))
        .await
        .expect_err("same stale CAS with a different request id must fail closed");
    assert!(
        conflicting_retry
            .to_string()
            .contains("durable active selection failed"),
        "got {conflicting_retry}"
    );
    let durable_after_conflict = registry_store
        .list_active_selections(&exact_scope)
        .await
        .expect("read durable application default after conflicting retry")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("durable application default remains present after conflicting retry");
    assert_eq!(durable_after_conflict.artifact_sha256, [2; 32]);
    assert_eq!(
        durable_after_conflict.selection_revision,
        initial_selection_revision + 1,
        "conflicting retry must not mutate the durable revision"
    );

    let response = client
        .completion(CompletionRequest::new(
            uuid::Uuid::now_v7(),
            "use the active default".to_owned(),
            client.selected_model_id(),
        ))
        .await
        .expect("new default-routed call uses the selected target");
    assert_eq!(response.text, "target response");
    assert_eq!(runtime.last_request().id, target);
    assert!(recorder.events().iter().any(|event| {
        event.payload["fr_event"] == "FR-EVT-MODEL-SELECTION-RECORDED"
            && event.payload["selected_model_id"] == target.to_string()
            && event.payload["actor"] == "native-model-runtime-panel"
    }));

    let stale = client
        .swap_model(ready_model_swap_request(
            current,
            target,
            initial_selection_revision,
        ))
        .await
        .expect_err("stale concurrent selector state must fail closed");
    assert!(
        stale.to_string().contains("stale model swap"),
        "got {stale}"
    );
    assert_eq!(client.selected_model_id(), target.to_string());

    let not_ready_error = client
        .swap_model(ready_model_swap_request(
            target,
            not_ready,
            initial_selection_revision + 1,
        ))
        .await
        .expect_err("a catalog row without READY state must not be selectable");
    assert!(
        not_ready_error.to_string().contains("not READY"),
        "got {not_ready_error}"
    );
    assert_eq!(client.selected_model_id(), target.to_string());

    drop(client);
    drop(registry_store);
    drop(storage);
    surreal_scope
        .cleanup()
        .await
        .expect("clean embedded-Surreal local-routing scope");
}

#[tokio::test]
async fn local_llamacpp_model_completion_routes_through_model_runtime_and_emits_fr_event() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["llama ", "ok"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder.clone(),
    );

    let trace_id = uuid::Uuid::now_v7();
    let req = CompletionRequest::new(
        trace_id,
        "route this locally".to_string(),
        model_id.to_string(),
    )
    .with_max_tokens(8)
    .with_temperature(0.2)
    .with_stop_sequences(vec!["</s>".to_string()]);

    let response = client.completion(req).await.expect("local completion");

    assert_eq!(response.text, "llama ok");
    assert_eq!(response.usage.prompt_tokens, 3);
    assert_eq!(response.usage.completion_tokens, 2);
    assert_eq!(response.usage.total_tokens, 5);
    assert_eq!(llama.request_count(), 1);
    assert_eq!(candle.request_count(), 0);
    assert_eq!(fallback.request_count(), 0);
    let routed_req = llama.last_request();
    assert_eq!(routed_req.id, model_id);
    assert_eq!(routed_req.prompt, GenPrompt::from("route this locally"));
    assert_eq!(routed_req.max_tokens, 8);
    assert_eq!(routed_req.sampling.temperature, Some(0.2));
    assert_eq!(routed_req.stop_sequences, vec!["</s>".to_string()]);

    let events = recorder.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].actor, FlightRecorderActor::Agent);
    assert_eq!(events[0].trace_id, trace_id);
    assert_eq!(
        events[0].model_id.as_deref(),
        Some(model_id.to_string().as_str())
    );
    assert_eq!(events[0].payload["type"], "llm_inference");
    assert_eq!(events[0].payload["model_id"], model_id.to_string());
    assert_eq!(events[0].payload["token_usage"]["prompt_tokens"], 3);
    assert_eq!(events[0].payload["token_usage"]["completion_tokens"], 2);
    assert_eq!(events[0].payload["token_usage"]["total_tokens"], 5);
    assert!(events[0].payload["latency_ms"].as_u64().unwrap_or(0) > 0);
    assert!(events[0].payload["prompt_hash"].is_string());
    assert!(events[0].payload["response_hash"].is_string());
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_candle_model_completion_routes_to_candle_runtime() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::Candle))
        .expect("register candle model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "route to candle".to_string(),
        model_id.to_string(),
    );

    let response = client.completion(req).await.expect("candle completion");

    assert_eq!(response.text, "candle");
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 1);
    assert_eq!(fallback.request_count(), 0);
}

#[tokio::test]
async fn non_uuid_provider_model_ids_stay_on_fallback_llm_client() {
    let registry = ModelRegistry::default();
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("cloud response"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "cloud path".to_string(),
        "gpt-4o-mini".to_string(),
    );

    let response = client.completion(req).await.expect("fallback completion");

    assert_eq!(response.text, "cloud response");
    assert_eq!(fallback.request_count(), 1);
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 0);
}

#[tokio::test]
async fn uuid_like_non_v7_model_ids_stay_on_fallback_llm_client() {
    let registry = ModelRegistry::default();
    let llama = Arc::new(RecordingRuntime::new("llama", &["wrong"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["wrong"]));
    let fallback = Arc::new(RecordingFallbackClient::new("uuid fallback response"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(
        registry,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
    );

    let uuid_like_model_id = uuid::Uuid::nil().to_string();
    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "fallback uuid-shaped model id".to_string(),
        uuid_like_model_id.clone(),
    );

    let response = client.completion(req).await.expect("fallback completion");

    assert_eq!(response.text, "uuid fallback response");
    assert_eq!(fallback.request_count(), 1);
    assert_eq!(llama.request_count(), 0);
    assert_eq!(candle.request_count(), 0);

    let fallback_token = CancellationToken::new();
    client.cancel(&uuid_like_model_id, fallback_token.clone());
    assert!(fallback_token.is_cancelled());
    assert_eq!(fallback.cancel_count(), 1);
}

#[test]
fn cancel_uses_same_llm_client_surface_for_local_and_fallback_models() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama = Arc::new(RecordingRuntime::new("llama", &["llama"]));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = client_for_registry(registry, llama.clone(), candle, fallback.clone(), recorder);

    let local_token = CancellationToken::new();
    client.cancel(&model_id.to_string(), local_token.clone());
    assert!(local_token.is_cancelled());
    assert_eq!(llama.cancel_count(), 1);

    let fallback_token = CancellationToken::new();
    client.cancel("gpt-4o-mini", fallback_token.clone());
    assert!(fallback_token.is_cancelled());
    assert_eq!(fallback.cancel_count(), 1);
}

#[tokio::test]
async fn cancel_cancels_the_active_local_generate_request_token() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let release = Arc::new(Notify::new());
    let llama = Arc::new(RecordingRuntime::new_blocking(
        "llama",
        &["partial"],
        release.clone(),
    ));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = Arc::new(client_for_registry(
        registry,
        llama.clone(),
        candle,
        fallback,
        recorder,
    ));

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "cancel while active".to_string(),
        model_id.to_string(),
    );
    let completion_task = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.completion(req).await }
    });

    let routed_req = wait_for_runtime_request(&llama).await;
    assert!(!routed_req.cancel.is_cancelled());

    let caller_token = CancellationToken::new();
    client.cancel(&model_id.to_string(), caller_token.clone());

    assert!(caller_token.is_cancelled());
    assert!(
        llama.last_request().cancel.is_cancelled(),
        "LlmClient::cancel must cancel the token attached to the active GenerateRequest"
    );

    release.notify_waiters();
    let response = completion_task
        .await
        .expect("completion task joins")
        .expect("completion succeeds after release");
    assert_eq!(response.text, "partial");
}

#[tokio::test]
async fn cancel_cancels_every_concurrent_request_for_the_same_local_model() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let release = Arc::new(Notify::new());
    let llama = Arc::new(RecordingRuntime::new_blocking(
        "llama",
        &["partial"],
        release.clone(),
    ));
    let candle = Arc::new(RecordingRuntime::new("candle", &["candle"]));
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let recorder = Arc::new(CapturingRecorder::default());
    let client = Arc::new(client_for_registry(
        registry,
        llama.clone(),
        candle,
        fallback,
        recorder,
    ));

    let mut tasks = Vec::new();
    for trace in [uuid::Uuid::now_v7(), uuid::Uuid::now_v7()] {
        let request = CompletionRequest::new(
            trace,
            "cancel all concurrent requests".to_string(),
            model_id.to_string(),
        );
        let client = Arc::clone(&client);
        tasks.push(tokio::spawn(
            async move { client.completion(request).await },
        ));
    }

    for _ in 0..100 {
        if llama.request_count() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(llama.request_count(), 2, "both requests must be active");

    let caller_token = CancellationToken::new();
    client.cancel(&model_id.to_string(), caller_token.clone());

    assert!(caller_token.is_cancelled());
    let requests = llama.requests.lock().expect("requests lock").clone();
    assert!(
        requests.iter().all(|request| request.cancel.is_cancelled()),
        "same-model cancellation must not overwrite or miss a concurrent request token"
    );
    assert_eq!(
        llama.cancel_count(),
        3,
        "two active tokens and the caller token must reach the runtime cancellation seam"
    );

    for task in tasks {
        task.abort();
        let _ = task.await;
    }
}

#[tokio::test]
async fn aborting_completion_future_cancels_its_detached_runtime_request_token() {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama = Arc::new(RecordingRuntime::new_blocking(
        "llama",
        &["partial"],
        Arc::new(Notify::new()),
    ));
    let client = Arc::new(client_for_registry(
        registry,
        llama.clone(),
        Arc::new(RecordingRuntime::new("candle", &["candle"])),
        Arc::new(RecordingFallbackClient::new("fallback")),
        Arc::new(CapturingRecorder::default()),
    ));
    let task = tokio::spawn({
        let client = Arc::clone(&client);
        async move {
            client
                .completion(CompletionRequest::new(
                    uuid::Uuid::now_v7(),
                    "caller disappears".to_string(),
                    model_id.to_string(),
                ))
                .await
        }
    });

    let routed_request = wait_for_runtime_request(&llama).await;
    assert!(!routed_request.cancel.is_cancelled());
    task.abort();
    let _ = task.await;
    assert!(
        routed_request.cancel.is_cancelled(),
        "dropping the caller future must cancel the token owned by the detached runtime worker"
    );
}

// ===========================================================================
// WP-1 MT-013: Flight Recorder emission on EVERY LlmClient call path
// (spec §4.2.3.2(3)) — fail-closed/disabled, completion error branches, and the
// embedding lane (success + error). Emitted at CALL TIME, never at construction.
// ===========================================================================

/// A `ModelRuntime` whose behavior is configurable per test: it can succeed with
/// a fixed token stream, fail the generate stream, return a fixed embedding
/// vector, or fail the embed call.
struct ConfigurableRuntime {
    tokens: Vec<GeneratedToken>,
    fail_generate: bool,
    embed_vector: Vec<f32>,
    fail_embed: bool,
    capabilities: ModelCapabilities,
}

impl ConfigurableRuntime {
    fn generating(chunks: &[&str]) -> Self {
        let tokens = chunks
            .iter()
            .enumerate()
            .map(|(index, text)| GeneratedToken {
                token_id: index as u32,
                text: (*text).to_string(),
                logprob: None,
                finish_reason: (index + 1 == chunks.len()).then_some(FinishReason::Stop),
            })
            .collect();
        Self {
            tokens,
            fail_generate: false,
            embed_vector: Vec::new(),
            fail_embed: false,
            capabilities: ModelCapabilities::default(),
        }
    }

    fn stream_error() -> Self {
        Self {
            tokens: Vec::new(),
            fail_generate: true,
            embed_vector: Vec::new(),
            fail_embed: false,
            capabilities: ModelCapabilities::default(),
        }
    }

    fn embedding(vector: Vec<f32>) -> Self {
        let dimension = vector.len();
        Self {
            tokens: Vec::new(),
            fail_generate: false,
            embed_vector: vector,
            fail_embed: false,
            capabilities: ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(dimension),
                ..Default::default()
            },
        }
    }

    fn embed_error(embedding_dimension: usize) -> Self {
        Self {
            tokens: Vec::new(),
            fail_generate: false,
            embed_vector: Vec::new(),
            fail_embed: true,
            capabilities: ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(embedding_dimension),
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl ModelRuntime for ConfigurableRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        if self.fail_generate {
            let err: Result<GeneratedToken, ModelRuntimeError> =
                Err(ModelRuntimeError::GenerateError("stream boom".to_string()));
            return Box::pin(stream::iter(vec![err]));
        }
        let tokens = self.tokens.clone();
        Box::pin(stream::iter(tokens.into_iter().map(Ok)))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        if self.fail_embed {
            return Err(ModelRuntimeError::EmbedError("embed boom".to_string()));
        }
        Ok(Embedding {
            vector: self.embed_vector.clone(),
        })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("cfg-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("cfg-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("cfg-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// Builds a local client whose registered (Candle) model routes to `candle`.
fn candle_client(
    model_id: ModelId,
    candle: Arc<dyn ModelRuntime>,
    recorder: Arc<CapturingRecorder>,
    max_context_tokens: u32,
) -> LocalModelRuntimeLlmClient {
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::Candle))
        .expect("register candle model");
    let llama: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("llama", &["x"]));
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder,
        ModelProfile::new("local-router".to_string(), max_context_tokens).with_streaming(true),
    )
}

fn candle_embedding_client(
    model_id: ModelId,
    embedding_dimension: usize,
    candle: Arc<dyn ModelRuntime>,
    recorder: Arc<CapturingRecorder>,
    max_context_tokens: u32,
) -> LocalModelRuntimeLlmClient {
    let mut registry = ModelRegistry::default();
    registry
        .register(embedding_registration(
            model_id,
            RuntimeBinding::Candle,
            embedding_dimension,
        ))
        .expect("register embedding-capable candle model");
    let llama: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("llama", &["x"]));
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder,
        ModelProfile::new("local-router".to_string(), max_context_tokens).with_streaming(true),
    )
}

#[tokio::test]
async fn disabled_client_completion_emits_fr_event_at_call_time_each_call() {
    let recorder = Arc::new(CapturingRecorder::default());
    let disabled = DisabledLlmClient::new_recorded(
        "unknown".to_string(),
        "HSK-LOCAL-DISABLED: test disabled".to_string(),
        recorder.clone(),
    );

    // Construction alone MUST NOT emit — proves call-time, not construction-time.
    assert_eq!(
        recorder.events().len(),
        0,
        "DisabledLlmClient construction must not emit a Flight Recorder event"
    );

    let trace_a = uuid::Uuid::now_v7();
    let err_a = disabled
        .completion(CompletionRequest::new(trace_a, "p".into(), "m".into()))
        .await
        .expect_err("disabled completion must fail closed");
    assert!(matches!(err_a, LlmError::ProviderError(_)));

    let trace_b = uuid::Uuid::now_v7();
    let _ = disabled
        .completion(CompletionRequest::new(trace_b, "p2".into(), "m2".into()))
        .await
        .expect_err("disabled completion must fail closed");

    let events = recorder.events();
    assert_eq!(
        events.len(),
        2,
        "two disabled calls must emit two FR events (call-time, not once at construction)"
    );
    for event in &events {
        assert!(matches!(
            event.event_type,
            FlightRecorderEventType::LlmInference
        ));
        assert_eq!(event.payload["error_kind"], "llm_disabled");
        assert_eq!(event.payload["token_usage"]["total_tokens"], 0);
        assert!(event.payload["reason"].is_string());
        assert!(event.validate().is_ok());
    }
    assert_eq!(events[0].trace_id, trace_a);
    assert_eq!(events[1].trace_id, trace_b);
}

#[tokio::test]
async fn local_completion_stream_error_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::stream_error());
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .completion(CompletionRequest::new(
            trace_id,
            "boom".into(),
            model_id.to_string(),
        ))
        .await
        .expect_err("stream error must surface as LlmError");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "a completion stream error must emit exactly one FR event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "llm_error");
    assert_eq!(events[0].payload["token_usage"]["total_tokens"], 0);
    assert_eq!(events[0].trace_id, trace_id);
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_completion_budget_exceeded_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::generating(&["a", "b", "c"]));
    let client = candle_client(model_id, candle, recorder.clone(), 8192);

    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        "over budget".into(),
        model_id.to_string(),
    )
    .with_max_tokens(1);
    let err = client
        .completion(req)
        .await
        .expect_err("budget exceeded must surface as LlmError");
    assert!(matches!(err, LlmError::BudgetExceeded(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "budget-exceeded must emit the error FR event, not the success event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "llm_error");
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_completion_unregistered_model_emits_fr_error_event() {
    let recorder = Arc::new(CapturingRecorder::default());
    // Empty registry: a UUIDv7 id parses as local but resolves to "not
    // registered" inside run_local_completion (the runtime-resolve error branch).
    let registry = ModelRegistry::default();
    let llama: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("llama", &["x"]));
    let candle: Arc<dyn ModelRuntime> = Arc::new(RecordingRuntime::new("candle", &["x"]));
    let router = LocalRouter::new(Arc::new(registry), llama, candle);
    let fallback = Arc::new(RecordingFallbackClient::new("fallback"));
    let client = LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        recorder.clone(),
        ModelProfile::new("local-router".to_string(), 8192),
    );

    let unregistered = ModelId::new_v7();
    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .completion(CompletionRequest::new(
            trace_id,
            "p".into(),
            unregistered.to_string(),
        ))
        .await
        .expect_err("unregistered model must fail");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "runtime-resolve error must emit exactly one FR event"
    );
    assert_eq!(events[0].payload["error_kind"], "llm_error");
    assert_eq!(events[0].trace_id, trace_id);
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_embedding_success_emits_data_embedding_computed_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> =
        Arc::new(ConfigurableRuntime::embedding(vec![0.1, 0.2, 0.3]));
    let client = candle_embedding_client(model_id, 3, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let response = client
        .embedding(EmbeddingRequest::new(
            trace_id,
            "embed me".into(),
            model_id.to_string(),
        ))
        .await
        .expect("embedding success");
    assert_eq!(response.vector, vec![0.1, 0.2, 0.3]);

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "embedding success must emit exactly one FR event"
    );
    assert!(
        matches!(
            events[0].event_type,
            FlightRecorderEventType::DataEmbeddingComputed
        ),
        "embedding success must reuse DataEmbeddingComputed, got {:?}",
        events[0].event_type
    );
    assert_eq!(events[0].trace_id, trace_id);
    assert_eq!(
        events[0].payload["silver_id"],
        format!("embedding-call-{}", trace_id.simple())
    );
    assert_eq!(events[0].payload["model_id"], model_id.to_string());
    assert_eq!(events[0].payload["model_version"], "local-runtime");
    assert_eq!(events[0].payload["dimensions"], 3);
    assert_eq!(events[0].payload["compute_latency_ms"], response.latency_ms);
    assert_eq!(events[0].payload["was_truncated"], false);
    // Embeddings carry NO TokenUsage (product-extension FR shape).
    assert!(
        events[0].payload.get("token_usage").is_none(),
        "embedding FR event must not carry a token_usage field"
    );
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn local_embedding_error_emits_fr_error_event() {
    let model_id = ModelId::new_v7();
    let recorder = Arc::new(CapturingRecorder::default());
    let candle: Arc<dyn ModelRuntime> = Arc::new(ConfigurableRuntime::embed_error(3));
    let client = candle_embedding_client(model_id, 3, candle, recorder.clone(), 8192);

    let trace_id = uuid::Uuid::now_v7();
    let err = client
        .embedding(EmbeddingRequest::new(
            trace_id,
            "embed me".into(),
            model_id.to_string(),
        ))
        .await
        .expect_err("embed failure must surface as LlmError");
    assert!(matches!(err, LlmError::ProviderError(_)));

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "embedding error must emit exactly one FR event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "embedding_error");
    assert_eq!(events[0].trace_id, trace_id);
    assert!(events[0].validate().is_ok());
}

#[tokio::test]
async fn disabled_client_embedding_emits_fr_event_at_call_time() {
    // WP-1 MT-013 (F2): the trait-default `embedding()` returns
    // EmbeddingUnsupported WITHOUT an FR event — a silent path reachable when the
    // default lane delegates a non-UUIDv7 embed id to the DisabledLlmClient
    // fallback. The override must emit a CALL-TIME FR event before returning.
    let recorder = Arc::new(CapturingRecorder::default());
    let disabled = DisabledLlmClient::new_recorded(
        "unknown".to_string(),
        "HSK-LOCAL-DISABLED: test disabled".to_string(),
        recorder.clone(),
    );

    // Construction alone MUST NOT emit — proves call-time, not construction-time.
    assert_eq!(
        recorder.events().len(),
        0,
        "DisabledLlmClient construction must not emit a Flight Recorder event"
    );

    let trace_id = uuid::Uuid::now_v7();
    let err = disabled
        .embedding(EmbeddingRequest::new(
            trace_id,
            "embed me".into(),
            "nomic-embed-text".into(),
        ))
        .await
        .expect_err("disabled embedding must fail closed with EmbeddingUnsupported");
    assert!(
        matches!(err, LlmError::EmbeddingUnsupported),
        "disabled embedding must return the typed EmbeddingUnsupported error, got {err:?}"
    );

    let events = recorder.events();
    assert_eq!(
        events.len(),
        1,
        "the disabled embedding call must emit exactly one CALL-TIME FR event"
    );
    assert!(matches!(
        events[0].event_type,
        FlightRecorderEventType::LlmInference
    ));
    assert_eq!(events[0].payload["error_kind"], "embedding_disabled");
    assert_eq!(events[0].payload["token_usage"]["total_tokens"], 0);
    assert!(events[0].payload["reason"].is_string());
    assert_eq!(events[0].trace_id, trace_id);
    assert!(events[0].validate().is_ok());
}

#[test]
fn llm_routing_surface_stays_engine_agnostic() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [["src", "llm", "mod.rs"], ["src", "llm", "local_router.rs"]] {
        let path = relative
            .iter()
            .fold(manifest_dir.clone(), |acc, item| acc.join(item));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let normalized = source.to_ascii_lowercase();

        for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
            assert!(
                !normalized.contains(banned),
                "LlmClient local routing surface must not leak engine-specific type `{banned}` in {}",
                path.display()
            );
        }
    }
}
