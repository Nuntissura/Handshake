//! MT-003 (WP-1) Ollama-kill: engine-free proof for the default LlmClient boot
//! resolution.
//!
//! These tests exercise the real `handshake_core::llm::boot` resolution seam
//! with fake `ModelRuntime`s / fakes (no PostgreSQL, no real inference). They
//! prove the reopened acceptance:
//! - no local model configured -> DisabledLlmClient, never an Ollama/daemon
//!   adapter (there is no longer any Ollama arm to select);
//! - the default local lane routes through LocalModelRuntimeLlmClient with
//!   profile().model_id = the minted UUIDv7 (HIGH regression guard 1);
//! - embedding() is wired to ModelRuntime::embed() (HIGH regression guard 2);
//! - the external_compat OpenAI-compatible lane is retained + non-authoritative.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{
        boot::{
            assemble_local_runtime_client, assemble_local_runtime_client_with_registrations,
            build_default_local_client, build_openai_compat_client,
        },
        registry::{ProviderKind, ResolvedProvider},
        CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, LlmError, ModelProfile,
        ModelTier, TokenUsage,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, FinishReason, GenerateRequest, GeneratedToken,
        KvCacheHandle, LoraStackHandle, ModelCapabilities, ModelId, ModelRegistration,
        ModelRuntime, ModelRuntimeError, OperatorId, ProviderKind as RuntimeProviderKind,
        RuntimeBinding, Score, SteeringHookHandle, TokenStream,
    },
};
use uuid::Uuid;

#[test]
fn mt003_ollama_adapter_source_file_and_public_export_are_removed() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ollama_source = manifest_dir.join("src").join("llm").join("ollama.rs");
    assert!(
        !ollama_source.exists(),
        "WP-1 MT-003 requires src/backend/handshake_core/src/llm/ollama.rs to be removed"
    );

    let llm_mod = std::fs::read_to_string(manifest_dir.join("src").join("llm").join("mod.rs"))
        .expect("read llm/mod.rs");
    for forbidden in [
        "pub mod ollama",
        "pub use ollama::OllamaAdapter",
        "OllamaAdapter",
    ] {
        assert!(
            !llm_mod.contains(forbidden),
            "WP-1 MT-003 requires removing the public Ollama adapter export; found {forbidden:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct FakeRuntime {
    label: &'static str,
    tokens: Vec<GeneratedToken>,
    embed_vector: Vec<f32>,
    capabilities: ModelCapabilities,
    generate_requests: Arc<Mutex<usize>>,
    embed_requests: Arc<Mutex<usize>>,
}

impl FakeRuntime {
    fn new(label: &'static str, chunks: &[&str], embed_vector: Vec<f32>) -> Self {
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
            embed_vector,
            capabilities: ModelCapabilities::default(),
            generate_requests: Arc::new(Mutex::new(0)),
            embed_requests: Arc::new(Mutex::new(0)),
        }
    }

    fn generate_count(&self) -> usize {
        *self.generate_requests.lock().expect("generate lock")
    }

    fn embed_count(&self) -> usize {
        *self.embed_requests.lock().expect("embed lock")
    }
}

#[async_trait]
impl ModelRuntime for FakeRuntime {
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
        *self.generate_requests.lock().expect("generate lock") += 1;
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
        *self.embed_requests.lock().expect("embed lock") += 1;
        Ok(Embedding {
            vector: self.embed_vector.clone(),
        })
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
    }
}

#[derive(Clone)]
struct RecordingFallback {
    completion_calls: Arc<Mutex<usize>>,
    embedding_calls: Arc<Mutex<usize>>,
    profile: ModelProfile,
}

impl RecordingFallback {
    fn new() -> Self {
        Self {
            completion_calls: Arc::new(Mutex::new(0)),
            embedding_calls: Arc::new(Mutex::new(0)),
            profile: ModelProfile::new("recording-fallback".to_string(), 4096),
        }
    }

    fn completion_count(&self) -> usize {
        *self.completion_calls.lock().expect("completion lock")
    }

    fn embedding_count(&self) -> usize {
        *self.embedding_calls.lock().expect("embedding lock")
    }
}

#[async_trait]
impl LlmClient for RecordingFallback {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        *self.completion_calls.lock().expect("completion lock") += 1;
        Ok(CompletionResponse {
            text: "FALLBACK".to_string(),
            usage: TokenUsage::default(),
            latency_ms: 0,
        })
    }

    async fn embedding(
        &self,
        _req: EmbeddingRequest,
    ) -> Result<handshake_core::llm::EmbeddingResponse, LlmError> {
        *self.embedding_calls.lock().expect("embedding lock") += 1;
        Err(LlmError::EmbeddingUnsupported)
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Clone, Default)]
struct CapturingRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
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
        Ok(self.events.lock().expect("events lock").clone())
    }
}

fn local_registration(model_id: ModelId, binding: RuntimeBinding) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: std::path::PathBuf::from("fixtures/models/boot-default.gguf"),
        sha256: [7; 32],
        runtime_binding: binding,
        declared_capabilities: ModelCapabilities {
            supports_activation_steering: binding == RuntimeBinding::Candle,
            ..Default::default()
        },
        base_model_tag: BaseModelTag::new("boot-default"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("operator-test"),
        provider: RuntimeProviderKind::Local,
    }
}

// ---------------------------------------------------------------------------
// (a) No local model configured -> DisabledLlmClient, never Ollama/daemon.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_boot_with_no_local_model_returns_disabled_never_ollama_daemon() {
    // ProviderKind has exactly LocalRuntime + OpenAiCompat; there is no Ollama
    // variant to select (compile-time proof the daemon arm is gone).
    let resolved = ResolvedProvider {
        provider_id: "local_runtime".to_string(),
        kind: ProviderKind::LocalRuntime,
        tier: ModelTier::Local,
        base_url: String::new(),
        model_id: "embedded-local-unconfigured".to_string(),
        api_key_env: None,
        local_model: None,
        local_embedding_model: None,
    };

    let recorder = Arc::new(CapturingRecorder::default());
    let client = build_default_local_client(&resolved, recorder, None).await;

    // DisabledLlmClient fail-closed signature: zero context window + errored
    // completion carrying the fail-closed reason.
    assert_eq!(client.profile().max_context_tokens, 0);
    assert_eq!(client.profile().model_id, "embedded-local-unconfigured");

    let req = CompletionRequest::new(Uuid::now_v7(), "hello".to_string(), "any".to_string());
    let err = client
        .completion(req)
        .await
        .expect_err("no-local-model boot must fail closed, not call a daemon");
    let message = err.to_string();
    assert!(
        message.contains("no local model configured") || message.contains("HSK-LOCAL-DISABLED"),
        "unexpected fail-closed error: {message}"
    );
}

// ---------------------------------------------------------------------------
// (b) Default routes through LocalModelRuntimeLlmClient with profile().model_id
//     = the minted UUIDv7, and completion routes to the runtime, not fallback.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_local_runtime_client_profile_model_id_is_minted_uuid_v7_and_routes_to_runtime() {
    let model_id = ModelId::new_v7();
    let candle = Arc::new(FakeRuntime::new("candle", &["hi", " there"], vec![0.5]));
    let llama = Arc::new(FakeRuntime::new("llama", &["wrong"], vec![9.0]));
    let fallback = Arc::new(RecordingFallback::new());
    let recorder = Arc::new(CapturingRecorder::default());

    let client = assemble_local_runtime_client(
        local_registration(model_id, RuntimeBinding::Candle),
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
        8192,
        None,
    )
    .expect("assemble local runtime client");

    // HIGH regression guard 1: profile().model_id IS the minted UUIDv7.
    let profile_model_id = client.profile().model_id.clone();
    let parsed = Uuid::parse_str(&profile_model_id).expect("profile model_id is a UUID");
    assert_eq!(
        parsed.get_version_num(),
        7,
        "profile model_id must be UUIDv7"
    );
    assert_eq!(profile_model_id, model_id.to_string());

    // Completion with that id routes to the embedded runtime, not the fallback.
    let req = CompletionRequest::new(
        Uuid::now_v7(),
        "route locally".to_string(),
        profile_model_id.clone(),
    );
    let response = client.completion(req).await.expect("local completion");

    assert_eq!(response.text, "hi there");
    assert_eq!(candle.generate_count(), 1, "must route to embedded runtime");
    assert_eq!(llama.generate_count(), 0);
    assert_eq!(
        fallback.completion_count(),
        0,
        "UUIDv7 profile id must NOT silently fall back"
    );
}

// ---------------------------------------------------------------------------
// (c) embedding() is wired to ModelRuntime::embed(), not silently unsupported.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_local_runtime_client_embedding_is_supported_via_model_runtime_embed() {
    let model_id = ModelId::new_v7();
    let embed_vector = vec![0.1_f32, 0.2, 0.3, 0.4];
    let candle = Arc::new(FakeRuntime::new("candle", &["x"], embed_vector.clone()));
    let llama = Arc::new(FakeRuntime::new("llama", &["y"], vec![0.0]));
    let fallback = Arc::new(RecordingFallback::new());
    let recorder = Arc::new(CapturingRecorder::default());

    let mut registration = local_registration(model_id, RuntimeBinding::Candle);
    registration.declared_capabilities = ModelCapabilities {
        supports_embedding: true,
        embedding_dimension: Some(embed_vector.len()),
        ..Default::default()
    };

    let client = assemble_local_runtime_client(
        registration,
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
        8192,
        None,
    )
    .expect("assemble local runtime client");

    let req = EmbeddingRequest::new(
        Uuid::now_v7(),
        "embed me".to_string(),
        client.profile().model_id.clone(),
    );
    let response = client.embedding(req).await.expect(
        "embedding() must be supported via ModelRuntime::embed(), not EmbeddingUnsupported",
    );

    assert_eq!(response.vector, embed_vector);
    assert_eq!(candle.embed_count(), 1, "must route to embedded embed()");
    assert_eq!(
        fallback.embedding_count(),
        0,
        "UUIDv7 embedding id must route to the runtime, not the fallback"
    );
    assert!(
        !response.vector.is_empty(),
        "embedding must not be the empty/unsupported degradation"
    );
}

#[tokio::test]
async fn mt016_default_boot_registers_distinct_embedding_model_when_configured() {
    let chat_model_id = ModelId::new_v7();
    let embedding_model_id = ModelId::new_v7();
    let embed_vector = vec![0.25_f32; 768];
    let candle = Arc::new(FakeRuntime::new("candle", &["chat"], embed_vector.clone()));
    let llama = Arc::new(FakeRuntime::new("llama", &["wrong"], vec![9.0]));
    let fallback = Arc::new(RecordingFallback::new());
    let recorder = Arc::new(CapturingRecorder::default());

    let mut embedding_registration = local_registration(embedding_model_id, RuntimeBinding::Candle);
    embedding_registration.base_model_tag = BaseModelTag::new("boot-embedding-768");
    embedding_registration.declared_capabilities = ModelCapabilities {
        supports_embedding: true,
        embedding_dimension: Some(768),
        ..Default::default()
    };

    let client = assemble_local_runtime_client_with_registrations(
        local_registration(chat_model_id, RuntimeBinding::Candle),
        vec![embedding_registration],
        llama.clone(),
        candle.clone(),
        fallback.clone(),
        recorder,
        8192,
        None,
    )
    .expect("assemble local runtime client with dedicated embedding model");

    assert_eq!(
        client.profile().model_id,
        chat_model_id.to_string(),
        "chat/completion model remains the LlmClient profile identity"
    );
    let catalog = client.model_catalog().expect("catalog present");
    assert_eq!(catalog.len(), 2, "chat + embedding registrations enumerate");
    let selected = catalog
        .embedding_model_for_dim(768)
        .expect("ready dedicated embedding model selected");
    assert_eq!(selected.model_id, embedding_model_id.to_string());

    let chat_embedding_err = client
        .embedding(EmbeddingRequest::new(
            Uuid::now_v7(),
            "must not embed through chat".to_string(),
            chat_model_id.to_string(),
        ))
        .await
        .expect_err("chat model must not be accepted as embedding-capable");
    assert!(
        matches!(&chat_embedding_err, LlmError::EmbeddingUnsupported),
        "chat embedding must fail with EmbeddingUnsupported, got {chat_embedding_err}"
    );
    assert_eq!(
        candle.embed_count(),
        0,
        "capability guard must reject chat embedding before runtime dispatch"
    );

    let response = client
        .embedding(EmbeddingRequest::new(
            Uuid::now_v7(),
            "embed through dedicated model".to_string(),
            embedding_model_id.to_string(),
        ))
        .await
        .expect("dedicated embedding model routes to runtime");
    assert_eq!(response.vector, embed_vector);
    assert_eq!(candle.embed_count(), 1);
    assert_eq!(fallback.embedding_count(), 0);

    let wrong_dim_chat_id = ModelId::new_v7();
    let wrong_dim_embedding_id = ModelId::new_v7();
    let wrong_dim_candle = Arc::new(FakeRuntime::new("candle", &["chat"], vec![0.5_f32; 896]));
    let wrong_dim_llama = Arc::new(FakeRuntime::new("llama", &["wrong"], vec![9.0]));
    let wrong_dim_fallback = Arc::new(RecordingFallback::new());
    let wrong_dim_recorder = Arc::new(CapturingRecorder::default());
    let mut wrong_dim_registration =
        local_registration(wrong_dim_embedding_id, RuntimeBinding::Candle);
    wrong_dim_registration.base_model_tag = BaseModelTag::new("boot-embedding-wrong-dim");
    wrong_dim_registration.declared_capabilities = ModelCapabilities {
        supports_embedding: true,
        embedding_dimension: Some(768),
        ..Default::default()
    };

    let wrong_dim_client = assemble_local_runtime_client_with_registrations(
        local_registration(wrong_dim_chat_id, RuntimeBinding::Candle),
        vec![wrong_dim_registration],
        wrong_dim_llama,
        wrong_dim_candle.clone(),
        wrong_dim_fallback.clone(),
        wrong_dim_recorder,
        8192,
        None,
    )
    .expect("assemble wrong-dimension embedding client");
    let wrong_dim_err = wrong_dim_client
        .embedding(EmbeddingRequest::new(
            Uuid::now_v7(),
            "wrong dimension".to_string(),
            wrong_dim_embedding_id.to_string(),
        ))
        .await
        .expect_err("runtime vector length must be checked against declared dimension");
    assert!(
        matches!(
            wrong_dim_err,
            LlmError::EmbeddingDimensionMismatch {
                expected: 768,
                actual: 896
            }
        ),
        "declared/actual embedding dimension mismatch must be typed, got {wrong_dim_err}"
    );
    assert_eq!(
        wrong_dim_candle.embed_count(),
        1,
        "dimension guard runs after the selected embedding runtime returns"
    );
    assert_eq!(wrong_dim_fallback.embedding_count(), 0);
}

// ---------------------------------------------------------------------------
// (d) external_compat OpenAI-compatible lane retained + non-authoritative.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn external_compat_openai_lane_is_retained_and_non_authoritative() {
    let resolved = ResolvedProvider {
        provider_id: "openai_compat".to_string(),
        kind: ProviderKind::OpenAiCompat,
        tier: ModelTier::Local,
        base_url: "http://127.0.0.1:1234".to_string(),
        model_id: "gpt-4o-mini".to_string(),
        api_key_env: None,
        local_model: None,
        local_embedding_model: None,
    };
    let recorder = Arc::new(CapturingRecorder::default());

    let client = build_openai_compat_client(&resolved, recorder);

    // Retained: the boot path still produces a working OpenAI-compat LlmClient.
    assert_eq!(client.profile().model_id, "gpt-4o-mini");
    // Non-authoritative for the embedded local lane: its model_id is the external
    // provider string, NOT a minted UUIDv7, so LocalRouter would never treat it
    // as embedded-local authority.
    assert!(
        Uuid::parse_str(&client.profile().model_id).is_err(),
        "external_compat model_id must not be a local UUIDv7 authority id"
    );
}
