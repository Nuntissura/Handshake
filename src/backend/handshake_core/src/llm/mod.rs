//! LLM Client Adapter [HSK-TRAIT-004]
//!
//! Per Master Spec §4.2.3: All application code MUST interact with LLMs
//! through the `LlmClient` trait. This ensures provider portability and
//! centralized observability via Flight Recorder.

pub mod boot;
pub mod embedded_ledger;
pub mod guard;
#[cfg(any(test, feature = "test-utils"))]
pub mod in_memory;
pub mod local_router;
pub mod openai_compat;
pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use std::sync::{Arc, OnceLock};

use crate::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    LlmInferenceEvent, LlmInferenceTokenUsage, RecorderError,
};
use crate::model_runtime::{
    CancellationToken, GenerateRequest, ModelCatalog, ModelId, ModelRuntime, ModelRuntimeError,
    Score, ScopedModelRegistryAuthority, TokenStream,
};
use crate::workflows::ModelSwapRequestV0_4;
use guard::CloudEscalationBundleV0_4;

#[cfg(any(test, feature = "test-utils"))]
pub use in_memory::InMemoryLlmClient;

/// Explicit availability envelope for ModelRuntime operator projections.
/// Missing instrumentation is data, never an invented zero or empty value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ModelRuntimeValue<T> {
    Available { value: T },
    Unavailable { reason: String },
}

impl<T> ModelRuntimeValue<T> {
    pub fn available(value: T) -> Self {
        Self::Available { value }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeKvInspection {
    pub bytes_used: u64,
    pub bytes_capacity: u64,
    pub prefix_cache_hit_rate: ModelRuntimeValue<f64>,
    pub quantization: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeLoraInspection {
    pub lora_id: String,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeSteeringInspection {
    pub steering_vector_id: String,
    pub layer: u32,
    pub intensity: f32,
}

/// Truthful snapshot available through the object-safe LlmClient boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRuntimeInspection {
    pub kv_cache: ModelRuntimeValue<ModelRuntimeKvInspection>,
    pub lora_stack: ModelRuntimeValue<Vec<ModelRuntimeLoraInspection>>,
    pub active_steering: ModelRuntimeValue<Vec<ModelRuntimeSteeringInspection>>,
    pub tokens_per_second: ModelRuntimeValue<f64>,
    pub vram_resident_bytes: ModelRuntimeValue<u64>,
    pub last_call_at_utc: ModelRuntimeValue<String>,
    pub engine_internals: ModelRuntimeValue<Value>,
}

impl ModelRuntimeInspection {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            kv_cache: ModelRuntimeValue::unavailable(reason.clone()),
            lora_stack: ModelRuntimeValue::unavailable(reason.clone()),
            active_steering: ModelRuntimeValue::unavailable(reason.clone()),
            tokens_per_second: ModelRuntimeValue::unavailable(reason.clone()),
            vram_resident_bytes: ModelRuntimeValue::unavailable(reason.clone()),
            last_call_at_utc: ModelRuntimeValue::unavailable(reason.clone()),
            engine_internals: ModelRuntimeValue::unavailable(reason),
        }
    }
}

pub const MODEL_RUNTIME_CONTROL_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeControlCapabilities {
    pub quiesce: bool,
    pub unload: bool,
    pub swap_compatible_adapter: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ModelRuntimeControlAction {
    Quiesce,
    Unload,
    SwapCompatibleAdapter { target_adapter: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeControlRequest {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub model_id: String,
    pub action: ModelRuntimeControlAction,
    pub timeout_ms: u64,
    pub expected_catalog_revision: Option<u64>,
    pub expected_selection_revision: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeControlReceipt {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub model_id: String,
    pub result_model_id: Option<String>,
    pub action: ModelRuntimeControlAction,
    pub runtime_adapter: String,
    pub quiesced: bool,
    pub unloaded: bool,
    pub process_stop_committed: bool,
    pub registry_updated: bool,
    pub selection_rebound: bool,
    pub catalog_revision: Option<u64>,
    /// True when the requested runtime/catalog mutation completed but the
    /// matching process-ledger durability verdict requires reconciliation.
    #[serde(default)]
    pub reconciliation_required: bool,
    #[serde(default)]
    pub reconciliation_reason: Option<String>,
}

/// Explicit correlation identity for one streamed model invocation. This is
/// separate from [`GenerateRequest`] so runtime adapters remain transport-only
/// while governed callers can propagate trace/run/session identity without
/// thread-local or other hidden context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmInvocationContext {
    pub trace_id: Uuid,
    pub run_id: String,
    pub session_id: String,
    pub evidence_owner: LlmInvocationEvidenceOwner,
}

/// Selects the sole terminal inference-evidence owner for a governed stream.
/// Coordinator-owned streams already emit correlated lifecycle/usage events;
/// direct client-owned streams emit `LlmInference` themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmInvocationEvidenceOwner {
    Client,
    Coordinator,
}

/// HSK-TRAIT-004: LLM Client Adapter
///
/// All LLM interactions MUST go through this trait to satisfy [CX-101].
/// Implementations are responsible for:
/// - Token budget enforcement
/// - Flight Recorder event emission
/// - Provider-specific API translation
#[async_trait]
pub trait LlmClient: Send + Sync + 'static {
    /// Executes a completion request.
    ///
    /// Returns:
    /// - `Ok(CompletionResponse)`: The generated text and usage metadata.
    /// - `Err(LlmError)`: If the request fails or budget is exceeded.
    ///
    /// Implementers MUST emit a Flight Recorder event with `trace_id`,
    /// `model_id`, and `TokenUsage` per §4.2.3.2.
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Starts a streaming completion while preserving the complete
    /// [`GenerateRequest`] contract used by coordinator-owned model sessions.
    ///
    /// Application code must call this LlmClient surface instead of invoking
    /// [`ModelRuntime::generate`] directly. Implementations that do not support
    /// the full streaming contract fail with a typed capability error; the
    /// facade must never silently collapse LoRA/steering/KV/speculative/
    /// structured-decoding state into the aggregate completion API.
    fn stream_completion(self: Arc<Self>, req: GenerateRequest) -> TokenStream {
        let adapter = self.profile().model_id.clone();
        let result = if req.cancel.is_cancelled() {
            Err(crate::model_runtime::ModelRuntimeError::Cancelled)
        } else {
            Err(
                crate::model_runtime::ModelRuntimeError::CapabilityNotSupported {
                    capability: "full_streaming_completion".to_string(),
                    adapter,
                },
            )
        };
        Box::pin(futures::stream::once(async move { result }))
    }

    /// Governed streaming entrypoint with explicit invocation identity. The
    /// default preserves provider compatibility; coordinator-owned clients can
    /// override it to validate or record the same correlation envelope before
    /// dispatching the runtime stream.
    fn stream_completion_with_context(
        self: Arc<Self>,
        req: GenerateRequest,
        _context: LlmInvocationContext,
    ) -> TokenStream {
        self.stream_completion(req)
    }

    /// Produces a real embedding vector for the given text via the configured
    /// model runtime. This is the model-runtime surface LoomSearchV2
    /// (WP-KERNEL-009 MT-264) uses to embed block text and search queries for
    /// the semantic (pgvector kNN) modality.
    ///
    /// The default implementation returns a typed `ProviderError` so providers
    /// that do not expose an embedding endpoint compile unchanged. Callers MUST
    /// treat the typed error as "no embedding model configured" and degrade to
    /// the keyword/trigram modalities — they must NEVER fabricate a vector.
    async fn embedding(&self, _req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        Err(LlmError::EmbeddingUnsupported)
    }

    /// Score an explicit engine token sequence through the same traced/provider
    /// facade as generation. The default fails closed instead of exposing a
    /// generation-capable ModelRuntime handle to application callers.
    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, LlmError> {
        Err(LlmError::ProviderError(
            "model scoring is unavailable for this LLM client".to_string(),
        ))
    }

    /// Object-safe governed runtime control. The default fails closed; only a
    /// client that owns runtime admission, unload, registry/catalog mutation,
    /// selection rebind, and process STOP ordering may return a receipt.
    async fn control_model_runtime(
        &self,
        _req: ModelRuntimeControlRequest,
    ) -> Result<ModelRuntimeControlReceipt, LlmError> {
        Err(LlmError::ProviderError(
            "model runtime control is unavailable for this LLM client".to_string(),
        ))
    }

    /// Exact durable registry authority bound into this runtime owner.
    ///
    /// Mutation chokepoints must compare this authority's five-field scope to
    /// the authenticated request scope before calling runtime control or swap.
    /// Providers without a durable registry authority fail closed with `None`.
    fn scoped_model_registry_authority(&self) -> Option<ScopedModelRegistryAuthority> {
        None
    }

    /// Truthful per-model lifecycle controls that this concrete client can
    /// receipt. The default exposes no actions.
    fn model_runtime_control_capabilities(
        &self,
        _model_id: &str,
    ) -> ModelRuntimeControlCapabilities {
        ModelRuntimeControlCapabilities::default()
    }

    /// Cancels an in-flight request when the underlying provider exposes a
    /// model-specific cancellation path. The default implementation cancels
    /// the supplied token so callers can rely on fail-closed local state even
    /// for providers without remote cancellation support.
    fn cancel(&self, _model_id: &str, token: CancellationToken) {
        token.cancel();
    }

    /// Swaps the active model in the underlying provider runtime (best-effort),
    /// honoring the Model Swap Protocol budgets and timeout.
    ///
    /// Default implementation returns an "unsupported" provider error so that
    /// clients without runtime swap support can compile without implementing
    /// swap semantics.
    async fn swap_model(&self, _req: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        Err(LlmError::ProviderError(
            "HSK-501-UNSUPPORTED: model swap unsupported".to_string(),
        ))
    }

    /// Returns the model profile (capabilities, token limits).
    fn profile(&self) -> &ModelProfile;

    /// Returns the model currently selected for new default-routed calls.
    /// Providers without runtime selection inherit the static profile model.
    fn selected_model_id(&self) -> String {
        self.profile().model_id.clone()
    }

    /// Returns the shared, enumerable, labeled [`ModelCatalog`] over this
    /// client's model registry, when it has one (WP-1 MT-014).
    ///
    /// The default embedded local lane
    /// ([`local_router::LocalModelRuntimeLlmClient`]) returns `Some` so a
    /// backend surface reachable from `AppState.llm_client` can enumerate and
    /// label the configured local model(s) — including the STABLE cross-session
    /// anchor alongside the per-boot UUIDv7. Providers without a local registry
    /// (external OpenAI-compat, disabled) return the default `None`, so callers
    /// must treat the catalog as optional and degrade to no-enumeration rather
    /// than assuming a registry exists.
    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        None
    }

    /// Inspect one current-boot ModelRuntime without bypassing LlmClient.
    /// Non-runtime providers return explicit typed unavailability.
    fn inspect_model_runtime(&self, _model_id: &str) -> ModelRuntimeInspection {
        ModelRuntimeInspection::unavailable(
            "the selected LLM provider does not expose local ModelRuntime internals",
        )
    }

    /// Immediate provider cancellation seam. This is not a liveness proof and
    /// must not publish a ProcessOwnershipLedger STOP for an embedded runtime.
    /// The default implementation is a no-op.
    fn shutdown(&self) {}

    /// Relinquish any reserved embedded STOP authority without publishing STOP.
    /// Call only when another liveness owner (normally the OS process lease)
    /// must remain authoritative until process death and next-boot reconcile.
    /// Non-embedded providers fall back to immediate cancellation.
    fn leave_open_for_reconciliation(&self) {
        self.shutdown();
    }

    /// Graceful-shutdown seam used while the ProcessOwnershipLedger writer is
    /// still live. Providers without an embedded model inherit a no-op success.
    /// The local embedded client overrides this to close work admission, wait
    /// for actual runtime workers to terminate, prove final runtime ownership,
    /// complete `ModelRuntime::unload`, and only then consume the STOP capacity
    /// reserved before artifact access.
    async fn shutdown_gracefully(&self) -> Result<(), LlmError> {
        self.shutdown();
        Ok(())
    }
}

/// LlmClient mediation for one coordinator-owned ModelRuntime session.
///
/// The runtime remains separately owned by the session for cancellation,
/// teardown, scoring, and other lifecycle operations. Application generation
/// crosses this facade exactly once; adapter dispatch occurs here, inside the
/// central LLM module, satisfying CX-101 without buffering the runtime stream.
pub struct ModelRuntimeLlmClient {
    runtime: Arc<dyn ModelRuntime>,
    model_id: ModelId,
    profile: ModelProfile,
    flight_recorder: Arc<dyn FlightRecorder>,
}

/// Finite default request ceiling for the coordinator's single-runtime adapter.
/// Run-level limits may be lower and are enforced by `SwarmCoordinator` before
/// this adapter is reached.
const MODEL_RUNTIME_LLM_MAX_TOKENS: u32 = 4_096;

static MODEL_RUNTIME_FLIGHT_RECORDER: OnceLock<Arc<dyn FlightRecorder>> = OnceLock::new();

/// Install the process-wide durable recorder used by direct ModelRuntime
/// facades created from legacy two-argument call sites. The Tauri startup path
/// installs the same recorder used by swarm lifecycle capture before commands
/// can dispatch inference.
pub fn install_model_runtime_flight_recorder(
    recorder: Arc<dyn FlightRecorder>,
) -> Result<(), &'static str> {
    MODEL_RUNTIME_FLIGHT_RECORDER
        .set(recorder)
        .map_err(|_| "ModelRuntime Flight Recorder is already installed")
}

#[derive(Debug, Default)]
struct InstalledModelRuntimeFlightRecorder;

#[async_trait]
impl FlightRecorder for InstalledModelRuntimeFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        match MODEL_RUNTIME_FLIGHT_RECORDER.get() {
            Some(recorder) => recorder.record_event(event).await,
            None => {
                eprintln!(
                    "FR-EVT-LLM-INFERENCE: {}",
                    serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string())
                );
                Ok(())
            }
        }
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        match MODEL_RUNTIME_FLIGHT_RECORDER.get() {
            Some(recorder) => recorder.enforce_retention().await,
            None => Ok(0),
        }
    }

    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        match MODEL_RUNTIME_FLIGHT_RECORDER.get() {
            Some(recorder) => recorder.list_events(filter).await,
            None => Ok(Vec::new()),
        }
    }
}

impl ModelRuntimeLlmClient {
    /// Construct a direct-call facade bound to the recorder installed during
    /// application startup.
    pub fn new(runtime: Arc<dyn ModelRuntime>, model_id: ModelId) -> Self {
        Self::new_recorded(
            runtime,
            model_id,
            Arc::new(InstalledModelRuntimeFlightRecorder),
        )
    }

    /// Construct a direct-call facade with an explicit Flight Recorder owner.
    pub fn new_recorded(
        runtime: Arc<dyn ModelRuntime>,
        model_id: ModelId,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            runtime,
            model_id,
            profile: ModelProfile::new(model_id.to_string(), MODEL_RUNTIME_LLM_MAX_TOKENS),
            flight_recorder,
        }
    }

    /// Coordinator-only construction. The explicit contextual stream marks
    /// the coordinator as evidence owner, so this sink must never account a
    /// second inference event for the same call.
    pub(crate) fn new_coordinator_delegated(
        runtime: Arc<dyn ModelRuntime>,
        model_id: ModelId,
    ) -> Self {
        Self::new_recorded(runtime, model_id, Arc::new(NoopFlightRecorder))
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_tests(runtime: Arc<dyn ModelRuntime>, model_id: ModelId) -> Self {
        Self::new_coordinator_delegated(runtime, model_id)
    }

    fn validate_model_id(&self, model_id: ModelId) -> Result<(), String> {
        if model_id == self.model_id {
            Ok(())
        } else {
            Err(format!(
                "runtime adapter is bound to model {}, not {model_id}",
                self.model_id
            ))
        }
    }

    fn validate_max_tokens(&self, max_tokens: u32) -> Result<(), String> {
        if max_tokens == 0 {
            return Err("runtime adapter max_tokens must be greater than zero".to_string());
        }
        if max_tokens > self.profile.max_context_tokens {
            return Err(format!(
                "runtime adapter max_tokens {max_tokens} exceeds finite ceiling {}",
                self.profile.max_context_tokens
            ));
        }
        Ok(())
    }

    /// The sole raw ModelRuntime generation dispatch owned by this LlmClient
    /// adapter. Every public trait path validates identity and a finite request
    /// ceiling before reaching it.
    fn dispatch_generate(&self, req: GenerateRequest) -> TokenStream {
        self.runtime.generate(req)
    }

    fn validated_dispatch(&self, req: GenerateRequest) -> TokenStream {
        if let Err(error) = self
            .validate_model_id(req.id)
            .and_then(|()| self.validate_max_tokens(req.max_tokens))
        {
            return Box::pin(futures::stream::once(async move {
                Err(ModelRuntimeError::GenerateError(error))
            }));
        }
        self.dispatch_generate(req)
    }

    fn recorded_stream(
        self: Arc<Self>,
        req: GenerateRequest,
        trace_id: Uuid,
        run_id: Option<String>,
        session_id: Option<String>,
    ) -> TokenStream {
        use futures::StreamExt as _;

        let prompt = req.prompt.text.clone();
        let stream = self.validated_dispatch(req);
        let finalizer = RuntimeInferenceFinalizer::new(
            self.flight_recorder.clone(),
            trace_id,
            self.model_id.to_string(),
            prompt,
            run_id,
            session_id,
        );
        Box::pin(futures::stream::unfold(
            (stream, finalizer),
            |(mut stream, mut finalizer)| async move {
                match stream.next().await {
                    Some(Ok(token)) => {
                        finalizer.record_token(&token.text);
                        Some((Ok(token), (stream, finalizer)))
                    }
                    Some(Err(error)) => {
                        let outcome = if matches!(&error, ModelRuntimeError::Cancelled) {
                            "cancelled"
                        } else {
                            "error"
                        };
                        if let Some(event) = finalizer.finish(outcome, Some(error.to_string())) {
                            record_runtime_inference_event(&finalizer.flight_recorder, event).await;
                        }
                        Some((Err(error), (stream, finalizer)))
                    }
                    None => {
                        if let Some(event) = finalizer.finish("success", None) {
                            record_runtime_inference_event(&finalizer.flight_recorder, event).await;
                        }
                        None
                    }
                }
            },
        ))
    }
}

struct RuntimeInferenceFinalizer {
    flight_recorder: Arc<dyn FlightRecorder>,
    trace_id: Uuid,
    model_id: String,
    prompt_hash: String,
    response_hasher: Sha256,
    prompt_tokens: u64,
    completion_tokens: u64,
    started: std::time::Instant,
    run_id: Option<String>,
    session_id: Option<String>,
    finalized: bool,
}

impl RuntimeInferenceFinalizer {
    fn new(
        flight_recorder: Arc<dyn FlightRecorder>,
        trace_id: Uuid,
        model_id: String,
        prompt: String,
        run_id: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            flight_recorder,
            trace_id,
            model_id,
            prompt_hash: sha256_hex(prompt.as_bytes()),
            response_hasher: Sha256::new(),
            prompt_tokens: prompt.split_whitespace().count() as u64,
            completion_tokens: 0,
            started: std::time::Instant::now(),
            run_id,
            session_id,
            finalized: false,
        }
    }

    fn record_token(&mut self, text: &str) {
        self.completion_tokens = self.completion_tokens.saturating_add(1);
        self.response_hasher.update(text.as_bytes());
    }

    fn finish(
        &mut self,
        outcome: &'static str,
        error: Option<String>,
    ) -> Option<FlightRecorderEvent> {
        if self.finalized {
            return None;
        }
        self.finalized = true;
        let response_hash = hex::encode(self.response_hasher.clone().finalize());
        Some(runtime_inference_event(
            self.trace_id,
            &self.model_id,
            self.prompt_tokens,
            self.completion_tokens,
            Some(self.prompt_hash.clone()),
            Some(response_hash),
            (self.started.elapsed().as_millis() as u64).max(1),
            outcome,
            error,
            self.run_id.clone(),
            self.session_id.clone(),
        ))
    }
}

impl Drop for RuntimeInferenceFinalizer {
    fn drop(&mut self) {
        if let Some(event) = self.finish(
            "dropped",
            Some("caller dropped inference stream before terminal frame".into()),
        ) {
            dispatch_runtime_inference_event(self.flight_recorder.clone(), event);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn runtime_inference_event(
    trace_id: Uuid,
    model_id: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    prompt_hash: Option<String>,
    response_hash: Option<String>,
    latency_ms: u64,
    outcome: &str,
    error: Option<String>,
    run_id: Option<String>,
    session_id: Option<String>,
) -> FlightRecorderEvent {
    let base = LlmInferenceEvent {
        event_type: "llm_inference".to_string(),
        trace_id,
        model_id: model_id.to_string(),
        token_usage: LlmInferenceTokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
        },
        prompt_hash,
        response_hash,
        latency_ms: Some(latency_ms),
    };
    let mut payload = serde_json::to_value(base).unwrap_or_else(|_| json!({}));
    if let Value::Object(map) = &mut payload {
        map.insert("outcome".to_string(), Value::String(outcome.to_string()));
        map.insert(
            "evidence_owner".to_string(),
            Value::String("model_runtime_llm_client".to_string()),
        );
        if let Some(error) = error {
            map.insert("error".to_string(), Value::String(error));
        }
        if let Some(run_id) = run_id {
            map.insert("run_id".to_string(), Value::String(run_id));
        }
        if let Some(session_id) = session_id {
            map.insert("session_id".to_string(), Value::String(session_id));
        }
    }
    FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::Agent,
        trace_id,
        payload,
    )
    .with_model_id(model_id)
}

fn dispatch_runtime_inference_event(
    flight_recorder: Arc<dyn FlightRecorder>,
    event: FlightRecorderEvent,
) {
    let record = move || async move {
        record_runtime_inference_event(&flight_recorder, event).await;
    };
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(record());
    } else {
        // Drop may run after a Tokio runtime has shut down. A detached standard
        // thread keeps the terminal event observable without panicking or
        // silently abandoning the Flight Recorder future.
        std::thread::spawn(move || {
            match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime.block_on(record()),
                Err(error) => tracing::error!(
                    target: "handshake_core::llm",
                    %error,
                    "failed to construct terminal inference recorder runtime"
                ),
            }
        });
    }
}

async fn record_runtime_inference_event(
    flight_recorder: &Arc<dyn FlightRecorder>,
    event: FlightRecorderEvent,
) {
    const ATTEMPTS: usize = 5;
    const ATTEMPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
    for attempt in 1..=ATTEMPTS {
        match tokio::time::timeout(ATTEMPT_TIMEOUT, flight_recorder.record_event(event.clone()))
            .await
        {
            Ok(Ok(())) => return,
            Ok(Err(error)) if attempt < ATTEMPTS => {
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %error,
                    attempt,
                    "ModelRuntimeLlmClient inference event persistence failed; retrying"
                );
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            Ok(Err(error)) => {
                tracing::error!(
                    target: "handshake_core::llm",
                    error = %error,
                    attempts = ATTEMPTS,
                    "ModelRuntimeLlmClient inference event exhausted persistence retries"
                );
            }
            Err(_) if attempt < ATTEMPTS => {
                tracing::warn!(
                    target: "handshake_core::llm",
                    attempt,
                    timeout_ms = ATTEMPT_TIMEOUT.as_millis(),
                    "ModelRuntimeLlmClient inference event persistence timed out; retrying"
                );
            }
            Err(_) => tracing::error!(
                target: "handshake_core::llm",
                attempts = ATTEMPTS,
                timeout_ms = ATTEMPT_TIMEOUT.as_millis(),
                "ModelRuntimeLlmClient inference event exhausted timed persistence retries"
            ),
        }
    }
}

#[async_trait]
impl LlmClient for ModelRuntimeLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        use futures::StreamExt;

        let started = std::time::Instant::now();
        let prompt_tokens = req.prompt.split_whitespace().count() as u64;
        let prompt_hash = Some(sha256_hex(req.prompt.as_bytes()));
        if req.model_id != self.model_id.to_string() {
            let error = LlmError::ProviderError(format!(
                "runtime adapter is bound to model {}, not {}",
                self.model_id, req.model_id
            ));
            record_runtime_inference_event(
                &self.flight_recorder,
                runtime_inference_event(
                    req.trace_id,
                    &req.model_id,
                    prompt_tokens,
                    0,
                    prompt_hash,
                    None,
                    (started.elapsed().as_millis() as u64).max(1),
                    "error",
                    Some(error.to_string()),
                    None,
                    None,
                ),
            )
            .await;
            return Err(error);
        }
        let max_tokens = req.max_tokens.unwrap_or(self.profile.max_context_tokens);
        if let Err(reason) = self.validate_max_tokens(max_tokens) {
            let error = LlmError::ProviderError(reason);
            record_runtime_inference_event(
                &self.flight_recorder,
                runtime_inference_event(
                    req.trace_id,
                    &req.model_id,
                    prompt_tokens,
                    0,
                    prompt_hash,
                    None,
                    (started.elapsed().as_millis() as u64).max(1),
                    "error",
                    Some(error.to_string()),
                    None,
                    None,
                ),
            )
            .await;
            return Err(error);
        }
        let mut stream = self.dispatch_generate(GenerateRequest {
            id: self.model_id,
            prompt: req.prompt.clone().into(),
            sampling: crate::model_runtime::SamplingParams {
                temperature: Some(req.temperature),
                ..Default::default()
            },
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens,
            stop_sequences: req.stop_sequences,
            speculative_mode: None,
            structured_decoding: None,
        });
        let mut text = String::new();
        let mut completion_tokens = 0_u64;
        while let Some(token) = stream.next().await {
            let token = match token {
                Ok(token) => token,
                Err(runtime_error) => {
                    let outcome = if matches!(&runtime_error, ModelRuntimeError::Cancelled) {
                        "cancelled"
                    } else {
                        "error"
                    };
                    let error = LlmError::ProviderError(runtime_error.to_string());
                    record_runtime_inference_event(
                        &self.flight_recorder,
                        runtime_inference_event(
                            req.trace_id,
                            &req.model_id,
                            prompt_tokens,
                            completion_tokens,
                            prompt_hash,
                            Some(sha256_hex(text.as_bytes())),
                            (started.elapsed().as_millis() as u64).max(1),
                            outcome,
                            Some(error.to_string()),
                            None,
                            None,
                        ),
                    )
                    .await;
                    return Err(error);
                }
            };
            text.push_str(&token.text);
            completion_tokens = completion_tokens.saturating_add(1);
        }
        let usage = TokenUsage {
            prompt_tokens: u32::try_from(prompt_tokens).unwrap_or(u32::MAX),
            completion_tokens: u32::try_from(completion_tokens).unwrap_or(u32::MAX),
            total_tokens: u32::try_from(prompt_tokens.saturating_add(completion_tokens))
                .unwrap_or(u32::MAX),
        };
        let latency_ms = (started.elapsed().as_millis() as u64).max(1);
        record_runtime_inference_event(
            &self.flight_recorder,
            runtime_inference_event(
                req.trace_id,
                &req.model_id,
                prompt_tokens,
                completion_tokens,
                prompt_hash,
                Some(sha256_hex(text.as_bytes())),
                latency_ms,
                "success",
                None,
                None,
                None,
            ),
        )
        .await;
        Ok(CompletionResponse {
            text,
            usage,
            latency_ms,
        })
    }

    fn stream_completion(self: Arc<Self>, req: GenerateRequest) -> TokenStream {
        self.recorded_stream(req, Uuid::now_v7(), None, None)
    }

    fn stream_completion_with_context(
        self: Arc<Self>,
        req: GenerateRequest,
        context: LlmInvocationContext,
    ) -> TokenStream {
        if context.run_id.trim().is_empty() || context.session_id.trim().is_empty() {
            return Box::pin(futures::stream::once(async {
                Err(ModelRuntimeError::GenerateError(
                    "runtime invocation context requires non-empty run_id and session_id"
                        .to_string(),
                ))
            }));
        }
        match context.evidence_owner {
            LlmInvocationEvidenceOwner::Client => self.recorded_stream(
                req,
                context.trace_id,
                Some(context.run_id),
                Some(context.session_id),
            ),
            LlmInvocationEvidenceOwner::Coordinator => self.validated_dispatch(req),
        }
    }

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, LlmError> {
        self.validate_model_id(id)
            .map_err(LlmError::ProviderError)?;
        let sequence_len = u32::try_from(sequence.len()).unwrap_or(u32::MAX);
        if sequence_len > self.profile.max_context_tokens {
            return Err(LlmError::BudgetExceeded(sequence_len));
        }
        self.runtime
            .score(id, sequence)
            .await
            .map_err(|error| LlmError::ProviderError(error.to_string()))
    }

    fn cancel(&self, _model_id: &str, token: CancellationToken) {
        self.runtime.cancel(token);
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// Request payload for LLM completion.
///
/// Per §4.2.3.1 with §11.5 traceability requirement.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionRequest {
    /// Unique trace identifier for Flight Recorder correlation.
    /// Required per §11.5: "Every model call MUST emit a Flight Recorder
    /// event containing trace_id."
    pub trace_id: Uuid,
    /// The prompt text to send to the model.
    pub prompt: String,
    /// Model identifier (e.g., "llama3.2", "mistral").
    pub model_id: String,
    /// Maximum tokens to generate. If `None`, uses model default.
    /// Budget enforcement checks this against `ModelProfile::max_context_tokens`.
    pub max_tokens: Option<u32>,
    /// Sampling temperature (0.0 = deterministic, 1.0+ = creative).
    pub temperature: f32,
    /// Sequences that cause generation to stop.
    pub stop_sequences: Vec<String>,
    /// Cloud escalation consent bundle required for any outbound cloud invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_escalation: Option<CloudEscalationBundleV0_4>,
}

impl CompletionRequest {
    /// Creates a new completion request with required trace_id.
    pub fn new(trace_id: Uuid, prompt: String, model_id: String) -> Self {
        Self {
            trace_id,
            prompt,
            model_id,
            max_tokens: None,
            temperature: 0.7,
            stop_sequences: Vec::new(),
            cloud_escalation: None,
        }
    }

    /// Builder: set max_tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Builder: set temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Builder: set stop sequences.
    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = stop_sequences;
        self
    }
}

/// Response from LLM completion.
///
/// Per §4.2.3.1.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionResponse {
    /// The generated text.
    pub text: String,
    /// Token usage metrics for budgeting and observability.
    pub usage: TokenUsage,
    /// Request latency in milliseconds.
    pub latency_ms: u64,
}

/// Request payload for an embedding call (WP-KERNEL-009 MT-264 LoomSearchV2).
///
/// Carries the same `trace_id` discipline as [`CompletionRequest`] so the
/// embedding call is Flight-Recorder-correlatable per §11.5.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingRequest {
    /// Flight Recorder correlation id.
    pub trace_id: Uuid,
    /// The text to embed (block content or a search query).
    pub input: String,
    /// Embedding model identifier (e.g. "nomic-embed-text").
    pub model_id: String,
}

impl EmbeddingRequest {
    pub fn new(trace_id: Uuid, input: String, model_id: String) -> Self {
        Self {
            trace_id,
            input,
            model_id,
        }
    }
}

/// Response from an embedding call. `vector` is a real dense embedding produced
/// by the configured model — never fabricated by the search layer.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingResponse {
    /// The dense embedding vector.
    pub vector: Vec<f32>,
    /// The model that produced it (for receipts/provenance).
    pub model_id: String,
    /// Request latency in milliseconds.
    pub latency_ms: u64,
}

impl EmbeddingResponse {
    /// The embedding dimensionality.
    pub fn dim(&self) -> usize {
        self.vector.len()
    }
}

/// Token usage metrics for budgeting and Flight Recorder.
///
/// Per §4.2.3.1.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TokenUsage {
    /// Tokens consumed by the prompt.
    pub prompt_tokens: u32,
    /// Tokens generated in the completion.
    pub completion_tokens: u32,
    /// Total tokens (prompt + completion).
    pub total_tokens: u32,
}

/// Model deployment tier for security gating [§2.6.6.7.11.5].
///
/// CloudLeakageGuard only enforces leakage restrictions for Cloud tier models.
/// Local models are trusted and not subject to cloud export restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelTier {
    /// Local/on-premise model - no cloud leakage restrictions
    #[default]
    Local,
    /// Cloud-hosted model - subject to CloudLeakageGuard restrictions
    Cloud,
}

/// Model capabilities and limits.
///
/// Per §4.2.3.1.
#[derive(Debug, Clone)]
pub struct ModelProfile {
    /// Model identifier.
    pub model_id: String,
    /// Maximum context window size in tokens.
    pub max_context_tokens: u32,
    /// Whether the model supports streaming responses.
    pub supports_streaming: bool,
    /// Deployment tier for security gating [HSK-ACE-VAL-100]
    pub model_tier: ModelTier,
}

impl ModelProfile {
    /// Creates a new model profile.
    pub fn new(model_id: String, max_context_tokens: u32) -> Self {
        Self {
            model_id,
            max_context_tokens,
            supports_streaming: false,
            model_tier: ModelTier::Local,
        }
    }

    /// Builder: set streaming support.
    pub fn with_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    /// Builder: set model tier for security gating.
    pub fn with_tier(mut self, tier: ModelTier) -> Self {
        self.model_tier = tier;
        self
    }
}

/// LLM error types with stable HSK error codes.
///
/// Per §4.2.3.1.
#[derive(Debug, Error)]
pub enum LlmError {
    /// HSK-429-RATE-LIMIT: Provider rate limit exceeded.
    #[error("HSK-429-RATE-LIMIT: Provider rate limit exceeded")]
    RateLimit,

    /// HSK-402-BUDGET-EXCEEDED: Token budget exceeded.
    /// Contains the number of tokens that exceeded the budget.
    #[error("HSK-402-BUDGET-EXCEEDED: Token budget exceeded: {0}")]
    BudgetExceeded(u32),

    /// HSK-400-INVALID-BASE-URL: Invalid/unparseable provider base_url configuration.
    #[error("HSK-400-INVALID-BASE-URL: Invalid base_url: {0}")]
    InvalidBaseUrl(String),

    /// HSK-403-SSRF-BLOCKED: base_url blocked by SSRF protections (Cloud tier).
    #[error("HSK-403-SSRF-BLOCKED: base_url blocked by SSRF guard: {0}")]
    SsrBlocked(String),

    /// HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED => cloud escalation denied.
    #[error("HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED; cloud escalation denied")]
    GovernanceLocked,

    /// HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by runtime policy.
    #[error("HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by policy")]
    CloudEscalationDenied,

    /// HSK-403-CLOUD-CONSENT-REQUIRED: Missing consent artifacts for cloud escalation.
    #[error("HSK-403-CLOUD-CONSENT-REQUIRED: Missing ProjectionPlan + ConsentReceipt")]
    CloudConsentRequired,

    /// HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts do not bind or hash mismatch.
    #[error("HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts invalid: {0}")]
    CloudConsentMismatch(String),

    /// HSK-500-LLM: Internal provider error.
    #[error("HSK-500-LLM: Internal provider error: {0}")]
    ProviderError(String),

    /// HSK-501-EMBEDDING-UNSUPPORTED: The configured model runtime does not
    /// expose an embedding endpoint (no embedding model configured). Callers
    /// MUST degrade to keyword/trigram modalities — NEVER fabricate a vector.
    #[error("HSK-501-EMBEDDING-UNSUPPORTED: no embedding model configured")]
    EmbeddingUnsupported,

    /// HSK-409-EMBEDDING-DIMENSION-MISMATCH: The selected embedding model was
    /// declared for one vector dimension but returned another. Callers that can
    /// degrade should surface a typed semantic-degrade result instead of
    /// comparing incompatible vectors.
    #[error("HSK-409-EMBEDDING-DIMENSION-MISMATCH: expected {expected} dimensions, got {actual}")]
    EmbeddingDimensionMismatch { expected: usize, actual: usize },
}

/// A Flight Recorder sink that discards events.
///
/// Used ONLY as the default sink for [`DisabledLlmClient::new`] so
/// non-default-boot callers (bin fixtures, embedding-only tests) that construct
/// a disabled client without a recorder still compile. The DEFAULT boot path
/// (`llm::boot`) always constructs the disabled client via
/// [`DisabledLlmClient::new_recorded`] with the REAL Flight Recorder, so the
/// spec §4.2.3.2(3) "every call emits a Flight Recorder event" obligation holds
/// on the default LLM path.
#[derive(Debug, Default)]
struct NoopFlightRecorder;

#[async_trait]
impl FlightRecorder for NoopFlightRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

/// WP-1 MT-013 (spec §4.2.3.2(3)): emit a Flight Recorder event on a fail-closed
/// / error LLM call path so "every LlmClient call emits a Flight Recorder event"
/// holds on the error path too.
///
/// Reuses [`FlightRecorderEventType::LlmInference`] with a ZEROED token usage
/// (there are no real tokens on an error/disabled call) plus an explicit
/// `error_kind` + `reason` in the payload. It MUST be called at CALL TIME by the
/// caller (inside `completion`/`embedding`), never at construction — a
/// construction-time emit fires once at boot regardless of whether a call is
/// ever made, which is a false-green.
pub(crate) async fn emit_llm_call_error_event(
    flight_recorder: &Arc<dyn FlightRecorder>,
    trace_id: Uuid,
    model_id: &str,
    error_kind: &str,
    reason: &str,
) {
    let base = LlmInferenceEvent {
        event_type: "llm_inference".to_string(),
        trace_id,
        model_id: model_id.to_string(),
        token_usage: LlmInferenceTokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
        prompt_hash: None,
        response_hash: None,
        latency_ms: None,
    };
    let mut payload = serde_json::to_value(&base).unwrap_or_else(|_| json!({}));
    if let Value::Object(map) = &mut payload {
        map.insert(
            "error_kind".to_string(),
            Value::String(error_kind.to_string()),
        );
        map.insert("reason".to_string(), Value::String(reason.to_string()));
    }
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::System,
        trace_id,
        payload,
    )
    .with_model_id(model_id);
    if let Err(err) = flight_recorder.record_event(event).await {
        tracing::warn!(
            target: "handshake_core::llm",
            error = %err,
            trace_id = %trace_id,
            error_kind,
            "failed to record fail-closed/error llm Flight Recorder event"
        );
    }
}

/// LLM client used when the provider is unavailable at startup.
pub struct DisabledLlmClient {
    reason: String,
    profile: ModelProfile,
    flight_recorder: Arc<dyn FlightRecorder>,
}

impl DisabledLlmClient {
    /// Backward-compatible constructor with a no-op Flight Recorder sink.
    ///
    /// Use [`Self::new_recorded`] on the default boot path so the fail-closed
    /// `completion` emits a Flight Recorder event (spec §4.2.3.2(3)). This
    /// no-op-sink form exists only for non-default callers (bin fixtures,
    /// embedding-only tests) that are not "the default LLM path".
    pub fn new(model_id: String, reason: String) -> Self {
        Self::new_recorded(model_id, reason, Arc::new(NoopFlightRecorder))
    }

    /// WP-1 MT-013: construct a disabled client whose fail-closed `completion`
    /// emits a Flight Recorder event through `flight_recorder` at CALL TIME.
    pub fn new_recorded(
        model_id: String,
        reason: String,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            reason,
            profile: ModelProfile::new(model_id, 0),
            flight_recorder,
        }
    }
}

#[async_trait]
impl LlmClient for DisabledLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Spec §4.2.3.2(3): emit a Flight Recorder event on this fail-closed
        // path too, at CALL TIME (here) — NOT at construction, which would fire
        // once at boot regardless of whether any call is made (false-green).
        emit_llm_call_error_event(
            &self.flight_recorder,
            req.trace_id,
            &req.model_id,
            "llm_disabled",
            &self.reason,
        )
        .await;
        Err(LlmError::ProviderError(self.reason.clone()))
    }

    /// WP-1 MT-013 (F2): the default `embedding()` trait impl returns
    /// `EmbeddingUnsupported` WITHOUT emitting a Flight Recorder event. That path
    /// is reachable on the default lane: `LocalModelRuntimeLlmClient::embedding`
    /// delegates any non-UUIDv7 embed id to this fallback DisabledLlmClient, so
    /// the silent trait-default return would leave the embedding call
    /// Flight-Recorder-invisible — the same false-green the `completion` override
    /// closes. Emit the CALL-TIME FR event here (reusing `emit_llm_call_error_event`
    /// with an `embedding_disabled` kind) BEFORE returning the typed error.
    async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        emit_llm_call_error_event(
            &self.flight_recorder,
            req.trace_id,
            &req.model_id,
            "embedding_disabled",
            &self.reason,
        )
        .await;
        Err(LlmError::EmbeddingUnsupported)
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

// =============================================================================
// Canonical JSON + Hashing Helpers (Spec §2.6.6.7.0)
// =============================================================================

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(crate) fn canonical_json_bytes_nfc(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_json_value_nfc(&mut out, value);
    out.into_bytes()
}

fn write_canonical_json_value_nfc(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(num) => {
            if let Some(v) = num.as_i64() {
                out.push_str(&v.to_string());
            } else if let Some(v) = num.as_u64() {
                out.push_str(&v.to_string());
            } else if let Some(v) = num.as_f64() {
                // Spec §2.6.6.7.5: fixed float precision (recommend 6 decimals).
                let normalized = if v == 0.0 { 0.0 } else { v };
                out.push_str(&format!("{normalized:.6}"));
            } else {
                out.push_str(&num.to_string());
            }
        }
        Value::String(s) => write_canonical_json_string_nfc(out, s),
        Value::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_value_nfc(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<(&String, String)> = map
                .keys()
                .map(|key| (key, key.nfc().collect::<String>()))
                .collect();
            keys.sort_by(|(a_raw, a_norm), (b_raw, b_norm)| {
                a_norm.cmp(b_norm).then_with(|| a_raw.cmp(b_raw))
            });
            for (idx, (key, _)) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_string_nfc(out, key);
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical_json_value_nfc(out, v);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_canonical_json_string_nfc(out: &mut String, value: &str) {
    let normalized: String = value.nfc().collect();
    out.push('"');
    for ch in normalized.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c if (c as u32) <= 0x7F => out.push(c),
            c if (c as u32) <= 0xFFFF => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => {
                let code = (c as u32) - 0x1_0000;
                let high = 0xD800 + ((code >> 10) & 0x3FF);
                let low = 0xDC00 + (code & 0x3FF);
                out.push_str(&format!("\\u{:04X}\\u{:04X}", high, low));
            }
        }
    }
    out.push('"');
}

pub(crate) fn openai_compat_chat_completion_body_json(
    req: &CompletionRequest,
    resolved_model_id: &str,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "model".to_string(),
        Value::String(resolved_model_id.to_string()),
    );
    map.insert(
        "messages".to_string(),
        Value::Array(vec![serde_json::json!({
            "role": "user",
            "content": req.prompt.clone(),
        })]),
    );
    if let Some(max_tokens) = req.max_tokens {
        map.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
    }
    map.insert(
        "temperature".to_string(),
        serde_json::json!(req.temperature),
    );
    if !req.stop_sequences.is_empty() {
        map.insert(
            "stop".to_string(),
            serde_json::to_value(&req.stop_sequences).unwrap_or(Value::Array(Vec::new())),
        );
    }
    map.insert("stream".to_string(), Value::Bool(false));
    Value::Object(map)
}

pub(crate) fn openai_compat_canonical_request_bytes(
    req: &CompletionRequest,
    resolved_model_id: &str,
) -> Vec<u8> {
    canonical_json_bytes_nfc(&openai_compat_chat_completion_body_json(
        req,
        resolved_model_id,
    ))
}

/// Computes the canonical OpenAI-compatible request payload hash used for consent binding.
///
/// This is the SHA-256 (hex) of `openai_compat_canonical_request_bytes`.
pub fn openai_compat_request_payload_sha256(
    req: &CompletionRequest,
    resolved_model_id: &str,
) -> String {
    sha256_hex(&openai_compat_canonical_request_bytes(
        req,
        resolved_model_id,
    ))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::StreamExt as _;
    use serde_json::json;
    use std::{
        sync::{Condvar, Mutex},
        time::Duration,
    };

    #[derive(Default)]
    struct CapturingRecorder {
        events: Mutex<Vec<FlightRecorderEvent>>,
        changed: Condvar,
    }

    impl CapturingRecorder {
        fn wait_for_events(&self, count: usize) -> Vec<FlightRecorderEvent> {
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            let mut events = self.events.lock().expect("capture lock");
            while events.len() < count {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                let (next, _) = self
                    .changed
                    .wait_timeout(events, remaining)
                    .expect("capture wait");
                events = next;
            }
            events.clone()
        }
    }

    #[async_trait]
    impl FlightRecorder for CapturingRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.events.lock().expect("capture lock").push(event);
            self.changed.notify_all();
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self.events.lock().expect("capture lock").clone())
        }
    }

    struct StreamRuntime {
        model_id: ModelId,
        capabilities: crate::model_runtime::ModelCapabilities,
        items: Mutex<Option<Vec<Result<crate::model_runtime::GeneratedToken, ModelRuntimeError>>>>,
    }

    impl StreamRuntime {
        fn new(
            model_id: ModelId,
            items: Vec<Result<crate::model_runtime::GeneratedToken, ModelRuntimeError>>,
        ) -> Self {
            Self {
                model_id,
                capabilities: crate::model_runtime::ModelCapabilities::default(),
                items: Mutex::new(Some(items)),
            }
        }
    }

    #[async_trait]
    impl ModelRuntime for StreamRuntime {
        async fn load(
            &mut self,
            _spec: crate::model_runtime::LoadSpec,
        ) -> Result<ModelId, ModelRuntimeError> {
            Ok(self.model_id)
        }

        async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
            Ok(())
        }

        fn generate(&self, _req: GenerateRequest) -> TokenStream {
            let items = self
                .items
                .lock()
                .expect("runtime items lock")
                .take()
                .unwrap_or_default();
            Box::pin(futures::stream::iter(items))
        }

        async fn score(
            &self,
            _id: ModelId,
            _sequence: Vec<u32>,
        ) -> Result<Score, ModelRuntimeError> {
            Err(ModelRuntimeError::ScoreError(
                "unused test score".to_string(),
            ))
        }

        async fn embed(
            &self,
            _id: ModelId,
            _text: &str,
        ) -> Result<crate::model_runtime::Embedding, ModelRuntimeError> {
            Err(ModelRuntimeError::EmbedError(
                "unused test embed".to_string(),
            ))
        }

        fn capabilities(
            &self,
            _id: ModelId,
        ) -> Result<&crate::model_runtime::ModelCapabilities, ModelRuntimeError> {
            Ok(&self.capabilities)
        }

        fn kv_cache(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::KvCacheHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "unused_test_kv".to_string(),
                adapter: "stream_test".to_string(),
            })
        }

        fn lora_stack(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::LoraStackHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "unused_test_lora".to_string(),
                adapter: "stream_test".to_string(),
            })
        }

        fn steering_hooks(
            &self,
            _id: ModelId,
        ) -> Result<crate::model_runtime::SteeringHookHandle, ModelRuntimeError> {
            Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "unused_test_steering".to_string(),
                adapter: "stream_test".to_string(),
            })
        }

        fn cancel(&self, token: CancellationToken) {
            token.cancel();
        }
    }

    fn generated(text: &str) -> crate::model_runtime::GeneratedToken {
        crate::model_runtime::GeneratedToken {
            token_id: 1,
            text: text.to_string(),
            logprob: None,
            finish_reason: None,
        }
    }

    fn stream_request(model_id: ModelId) -> GenerateRequest {
        GenerateRequest {
            id: model_id,
            prompt: "two prompt tokens".into(),
            sampling: Default::default(),
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 8,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        }
    }

    fn payload_outcome(event: &FlightRecorderEvent) -> &str {
        event.payload["outcome"].as_str().expect("outcome")
    }

    #[tokio::test]
    async fn runtime_client_direct_stream_records_success_error_cancel_and_drop_once() {
        for (items, expected, expected_tokens) in [
            (
                vec![Ok(generated("a")), Ok(generated("b"))],
                "success",
                2_u64,
            ),
            (
                vec![
                    Ok(generated("a")),
                    Err(ModelRuntimeError::GenerateError("boom".into())),
                ],
                "error",
                1,
            ),
            (
                vec![Ok(generated("a")), Err(ModelRuntimeError::Cancelled)],
                "cancelled",
                1,
            ),
        ] {
            let model_id = ModelId::new_v7();
            let recorder = Arc::new(CapturingRecorder::default());
            let client = Arc::new(ModelRuntimeLlmClient::new_recorded(
                Arc::new(StreamRuntime::new(model_id, items)),
                model_id,
                recorder.clone(),
            ));
            let _ = client
                .stream_completion(stream_request(model_id))
                .collect::<Vec<_>>()
                .await;
            let events = recorder.wait_for_events(1);
            assert_eq!(events.len(), 1);
            assert_eq!(payload_outcome(&events[0]), expected);
            assert_eq!(
                events[0].payload["token_usage"]["completion_tokens"].as_u64(),
                Some(expected_tokens)
            );
        }

        let model_id = ModelId::new_v7();
        let recorder = Arc::new(CapturingRecorder::default());
        let client = Arc::new(ModelRuntimeLlmClient::new_recorded(
            Arc::new(StreamRuntime::new(
                model_id,
                vec![Ok(generated("a")), Ok(generated("b"))],
            )),
            model_id,
            recorder.clone(),
        ));
        let mut stream = client.stream_completion(stream_request(model_id));
        assert!(stream.next().await.is_some());
        drop(stream);
        let events = recorder.wait_for_events(1);
        assert_eq!(events.len(), 1);
        assert_eq!(payload_outcome(&events[0]), "dropped");
        assert_eq!(
            events[0].payload["token_usage"]["completion_tokens"].as_u64(),
            Some(1)
        );
    }

    #[test]
    fn runtime_client_drop_without_tokio_runtime_records_terminal_evidence() {
        let model_id = ModelId::new_v7();
        let recorder = Arc::new(CapturingRecorder::default());
        let client = Arc::new(ModelRuntimeLlmClient::new_recorded(
            Arc::new(StreamRuntime::new(model_id, vec![Ok(generated("unused"))])),
            model_id,
            recorder.clone(),
        ));
        let stream = client.stream_completion(stream_request(model_id));
        drop(stream);

        let events = recorder.wait_for_events(1);
        assert_eq!(events.len(), 1);
        assert_eq!(payload_outcome(&events[0]), "dropped");
        assert_eq!(
            events[0].payload["token_usage"]["completion_tokens"].as_u64(),
            Some(0)
        );
    }

    #[tokio::test]
    async fn coordinator_owned_context_suppresses_duplicate_client_evidence() {
        let model_id = ModelId::new_v7();
        let recorder = Arc::new(CapturingRecorder::default());
        let client = Arc::new(ModelRuntimeLlmClient::new_recorded(
            Arc::new(StreamRuntime::new(model_id, vec![Ok(generated("a"))])),
            model_id,
            recorder.clone(),
        ));
        let context = LlmInvocationContext {
            trace_id: Uuid::now_v7(),
            run_id: "run".to_string(),
            session_id: "session".to_string(),
            evidence_owner: LlmInvocationEvidenceOwner::Coordinator,
        };
        let _ = client
            .stream_completion_with_context(stream_request(model_id), context)
            .collect::<Vec<_>>()
            .await;
        assert!(recorder.wait_for_events(1).is_empty());
    }

    #[test]
    fn test_completion_request_builder() {
        let trace_id = Uuid::now_v7();
        let req = CompletionRequest::new(
            trace_id,
            "Hello, world!".to_string(),
            "llama3.2".to_string(),
        )
        .with_max_tokens(100)
        .with_temperature(0.5)
        .with_stop_sequences(vec!["###".to_string()]);

        assert_eq!(req.trace_id, trace_id);
        assert_eq!(req.prompt, "Hello, world!");
        assert_eq!(req.model_id, "llama3.2");
        assert_eq!(req.max_tokens, Some(100));
        assert_eq!(req.temperature, 0.5);
        assert_eq!(req.stop_sequences, vec!["###".to_string()]);
    }

    #[test]
    fn test_model_profile_builder() {
        let profile = ModelProfile::new("llama3.2".to_string(), 8192).with_streaming(true);

        assert_eq!(profile.model_id, "llama3.2");
        assert_eq!(profile.max_context_tokens, 8192);
        assert!(profile.supports_streaming);
    }

    #[test]
    fn test_llm_error_display() {
        let rate_limit = LlmError::RateLimit;
        assert_eq!(
            rate_limit.to_string(),
            "HSK-429-RATE-LIMIT: Provider rate limit exceeded"
        );

        let budget = LlmError::BudgetExceeded(1500);
        assert_eq!(
            budget.to_string(),
            "HSK-402-BUDGET-EXCEEDED: Token budget exceeded: 1500"
        );

        let invalid_base_url = LlmError::InvalidBaseUrl("bad".to_string());
        assert_eq!(
            invalid_base_url.to_string(),
            "HSK-400-INVALID-BASE-URL: Invalid base_url: bad"
        );

        let ssrf = LlmError::SsrBlocked("http://127.0.0.1".to_string());
        assert_eq!(
            ssrf.to_string(),
            "HSK-403-SSRF-BLOCKED: base_url blocked by SSRF guard: http://127.0.0.1"
        );

        let locked = LlmError::GovernanceLocked;
        assert_eq!(
            locked.to_string(),
            "HSK-403-GOVERNANCE-LOCKED: GovernanceMode LOCKED; cloud escalation denied"
        );

        let denied = LlmError::CloudEscalationDenied;
        assert_eq!(
            denied.to_string(),
            "HSK-403-CLOUD-ESCALATION-DENIED: Cloud escalation disallowed by policy"
        );

        let consent_required = LlmError::CloudConsentRequired;
        assert_eq!(
            consent_required.to_string(),
            "HSK-403-CLOUD-CONSENT-REQUIRED: Missing ProjectionPlan + ConsentReceipt"
        );

        let mismatch = LlmError::CloudConsentMismatch("hash mismatch".to_string());
        assert_eq!(
            mismatch.to_string(),
            "HSK-403-CLOUD-CONSENT-MISMATCH: Consent artifacts invalid: hash mismatch"
        );

        let provider = LlmError::ProviderError("Connection timeout".to_string());
        assert_eq!(
            provider.to_string(),
            "HSK-500-LLM: Internal provider error: Connection timeout"
        );
    }

    #[test]
    fn canonical_json_bytes_nfc_normalizes_strings() {
        let input = format!("e\u{0301}");
        let value = json!({ "s": input });
        let bytes = canonical_json_bytes_nfc(&value);
        let rendered = match String::from_utf8(bytes) {
            Ok(rendered) => rendered,
            Err(err) => {
                assert!(
                    false,
                    "expected UTF-8 canonical JSON bytes, got error: {err}"
                );
                return;
            }
        };

        assert!(
            rendered.contains("\\u00E9"),
            "expected NFC normalization to compose e + combining acute to \\u00E9, got: {rendered}"
        );
        assert!(
            !rendered.contains("\\u0301"),
            "expected combining acute to be removed by NFC normalization, got: {rendered}"
        );
    }

    #[test]
    fn canonical_json_bytes_nfc_formats_floats_with_fixed_precision() {
        let value = json!({ "t": 0.7 });
        let bytes = canonical_json_bytes_nfc(&value);
        let rendered = match String::from_utf8(bytes) {
            Ok(rendered) => rendered,
            Err(err) => {
                assert!(
                    false,
                    "expected UTF-8 canonical JSON bytes, got error: {err}"
                );
                return;
            }
        };
        assert!(
            rendered.contains("0.700000"),
            "expected fixed 6-decimal float formatting, got: {rendered}"
        );
    }
}
