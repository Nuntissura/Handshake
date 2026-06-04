//! MT-144 composition tests: verify that the production
//! `LocalModelRuntimeLlmClient` (the MT-070 LocalRouter dispatcher) actually
//! invokes the production `SharedCapsuleInjector` before delegating to a
//! `ModelRuntime`, and that the MemoryCapsule's MemoryPack content reaches
//! the runtime's `generate()` prompt on an `Inject` decision.
//!
//! These tests are deliberately end-to-end across the wiring seam (no mock
//! injector, no mock capsule builder): they exercise
//!
//!   `LocalModelRuntimeLlmClient` -> `apply_capsule_injection`
//!     -> `SharedCapsuleInjector::inject_for_call`
//!       -> real `CapsuleBuilder` -> real `CapsulePolicyTable`
//!         -> real `attach_capsule_to_generate_request`
//!           -> `ModelRuntime::generate(req)`  (RecordingRuntime captures req)
//!
//! Adversarial-validator focus (MT-144 rework, 2026-05-20T20:35:00Z):
//! "CapsuleInjector exists in isolation, fully tested with mock dependencies,
//!  but never wired into ModelRuntime::generate". The tests below close that
//! gap by asserting the runtime's captured `GenerateRequest.prompt` contains
//! the capsule envelope (Inject branch) or stays unchanged (Skip branch).

use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use handshake_core::{
    ace::{FemsSourceRef, FemsSourceRefKind},
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{
        local_router::{LocalModelRuntimeLlmClient, LocalRouter},
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile,
    },
    memory::{
        CapsuleFlightRecorderEvent, CapsulePolicyTable, DegradationTier, FemsError,
        FemsFlightRecorder, FemsFlightRecorderError, FemsRetriever, MemoryCapsuleInjection,
        ModelCallContext, ModelCallContextSource, RetrievalPolicy, RetrievedItem,
        SharedCapsuleInjector, TaskType, FR_EVT_CAPSULE_INJECTED, RETRIEVAL_SCORING_FORMULA_V0,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, FinishReason, GenPrompt, GenerateRequest,
        GeneratedToken, KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRegistration, ModelRegistry, ModelRuntime, ModelRuntimeError, OperatorId,
        ProviderKind, RuntimeBinding, Score, SteeringHookHandle, TokenStream,
    },
};

// ---------------------------------------------------------------------------
// RecordingRuntime - mock ModelRuntime that captures the GenerateRequest the
// LocalRouter dispatches. We assert against this struct's captured prompt to
// prove the MemoryCapsule envelope crossed the runtime.generate seam.
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RecordingRuntime {
    label: &'static str,
    tokens: Vec<GeneratedToken>,
    capabilities: ModelCapabilities,
    requests: Arc<Mutex<Vec<GenerateRequest>>>,
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
        }
    }

    fn last_request(&self) -> GenerateRequest {
        self.requests
            .lock()
            .expect("requests lock")
            .last()
            .cloned()
            .unwrap_or_else(|| panic!("expected {} runtime request", self.label))
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("requests lock").len()
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        self.requests.lock().expect("requests lock").push(req);
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
    }
}

// ---------------------------------------------------------------------------
// DroppingFallbackClient - the LocalRouter dispatcher requires a fallback
// LlmClient for non-local completions. These tests always route locally, so
// the fallback should never be reached; if it is, panic so the failure mode
// is loud.
// ---------------------------------------------------------------------------

struct PanickingFallbackClient {
    profile: ModelProfile,
}

impl PanickingFallbackClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new("panicking-fallback".to_string(), 4096),
        }
    }
}

#[async_trait]
impl LlmClient for PanickingFallbackClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        panic!("fallback LlmClient must not be reached when routing locally for MT-144 tests");
    }

    fn cancel(&self, _model_id: &str, _token: CancellationToken) {
        // no-op: cancel routing is exercised in llm_client_local_routing_tests.
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

// ---------------------------------------------------------------------------
// CapturingFlightRecorder - capture handshake_core::flight_recorder events
// emitted by LocalRouter (LlmInference). Not used to assert MT-144 semantics
// directly, but kept so the test mirrors the real wiring used by the
// production constructor.
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct CapturingFlightRecorder {
    events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
}

#[async_trait]
impl FlightRecorder for CapturingFlightRecorder {
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

// ---------------------------------------------------------------------------
// Production FemsRetriever fixture: returns a fixed list of retrieved items,
// same shape as memory_injection_tests::TestFemsRetriever and
// memory_builder_tests fixtures. This is the *real* FemsRetriever trait, the
// *real* CapsuleBuilder, the *real* CapsulePolicyTable - only the upstream
// data source is a fixture so the test does not depend on a live FEMS store.
// ---------------------------------------------------------------------------

struct FixtureFemsRetriever {
    items: Vec<RetrievedItem>,
    calls: Mutex<Vec<(String, u32)>>,
}

impl FixtureFemsRetriever {
    fn new(items: Vec<RetrievedItem>) -> Self {
        Self {
            items,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<(String, u32)> {
        self.calls.lock().expect("calls lock").clone()
    }
}

impl FemsRetriever for FixtureFemsRetriever {
    fn retrieve(&self, query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push((query.to_string(), top_k));
        Ok(self.items.clone())
    }
}

// ---------------------------------------------------------------------------
// Capturing FemsFlightRecorder for MT-144: records every CapsuleInjected /
// CapsuleSuppressed event so the test can assert FR-EVT-CAPSULE-INJECTED is
// emitted exactly once on the Inject branch.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CapturingFemsFlightRecorder {
    events: Mutex<Vec<CapsuleFlightRecorderEvent>>,
}

impl CapturingFemsFlightRecorder {
    fn events(&self) -> Vec<CapsuleFlightRecorderEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

impl FemsFlightRecorder for CapturingFemsFlightRecorder {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FixedContextSource - production trait implementation that returns a fixed
// ModelCallContext for every CompletionRequest. Production code would derive
// the context from the orchestrator-supplied task type, role, and session;
// here we hard-wire it so the test pins down the exact eligibility branch.
// ---------------------------------------------------------------------------

struct FixedContextSource {
    context: ModelCallContext,
}

impl ModelCallContextSource<CompletionRequest> for FixedContextSource {
    fn model_call_context(&self, _request: &CompletionRequest) -> Option<ModelCallContext> {
        Some(self.context.clone())
    }
}

// `FixedContextSource` is trivially Send + Sync because ModelCallContext is
// Send + Sync; no inner mutability.
// (Compiler proves this implicitly via the Arc<dyn Trait> bound.)

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn capabilities() -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_activation_steering: false,
        supports_speculative_draft: false,
        supports_eagle3: false,
        ..Default::default()
    }
}

fn registration(model_id: ModelId, binding: RuntimeBinding) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("fixtures/models/mt-144-capsule-injection.gguf"),
        sha256: [7; 32],
        runtime_binding: binding,
        declared_capabilities: capabilities(),
        base_model_tag: BaseModelTag::new("mt-144-capsule-injection-base"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("operator-mt144"),
        provider: ProviderKind::Local,
    }
}

fn retrieved(id: &str, score: f64, capsule_bytes: u64, pinned: bool) -> RetrievedItem {
    RetrievedItem {
        item_id: id.to_string(),
        memory_class: "episodic".to_string(),
        item_type: "note".to_string(),
        summary: format!("summary {id}"),
        content: format!("content {id}"),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: 0.9,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: format!("artifact-{id}"),
            hash: None,
            selector: Some(format!("#{}", id)),
            created_at: None,
            classification: None,
        }],
        score,
        score_breakdown: BTreeMap::from([("similarity".to_string(), score)]),
        capsule_bytes,
        token_estimate: capsule_bytes as u32,
        pinned,
    }
}

fn eligible_context() -> ModelCallContext {
    ModelCallContext::eligible(
        TaskType::KernelBuilderMtImplementation,
        "MT-144 capsule injection composition test",
        "KERNEL_BUILDER",
        "session-mt144",
    )
}

/// Eligible context with a tight override policy (top_k=2, budget=60) so the
/// fixture's two retrieved items get split: the higher-scoring one is
/// included, the lower-scoring one is suppressed. Mirrors the existing
/// `memory_injection_tests::inject_success_*` policy override.
fn eligible_context_with_tight_budget() -> ModelCallContext {
    let mut ctx = eligible_context();
    ctx.override_policy = Some(RetrievalPolicy {
        top_k: 2,
        capsule_budget_bytes: 60,
        task_type: TaskType::KernelBuilderMtImplementation,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    });
    ctx
}

fn ineligible_context() -> ModelCallContext {
    ModelCallContext::ineligible(
        "MT-144 capsule injection composition test - skip branch",
        "KERNEL_BUILDER",
        "session-mt144",
    )
}

struct WiredClient {
    client: LocalModelRuntimeLlmClient,
    runtime: Arc<RecordingRuntime>,
    fems_events: Arc<CapturingFemsFlightRecorder>,
    fems_retriever: Arc<FixtureFemsRetriever>,
    model_id: ModelId,
}

fn build_wired_client(fixture_items: Vec<RetrievedItem>, context: ModelCallContext) -> WiredClient {
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model for MT-144 composition test");

    let llama_runtime = Arc::new(RecordingRuntime::new("llama-mt144", &["t1 ", "t2 ", "t3"]));
    let candle_runtime = Arc::new(RecordingRuntime::new("candle-mt144", &["wrong"]));
    let router = LocalRouter::new(Arc::new(registry), llama_runtime.clone(), candle_runtime);

    let fems_retriever: Arc<FixtureFemsRetriever> =
        Arc::new(FixtureFemsRetriever::new(fixture_items));
    let policy_table = Arc::new(CapsulePolicyTable);
    let fems_events = Arc::new(CapturingFemsFlightRecorder::default());

    let fems_retriever_dyn: Arc<dyn FemsRetriever + Send + Sync> = fems_retriever.clone();
    let fems_events_dyn: Arc<dyn FemsFlightRecorder + Send + Sync> = fems_events.clone();

    let injector: Arc<dyn MemoryCapsuleInjection> = Arc::new(SharedCapsuleInjector::new(
        fems_retriever_dyn,
        policy_table,
        fems_events_dyn,
    ));
    let context_source: Arc<dyn ModelCallContextSource<CompletionRequest>> =
        Arc::new(FixedContextSource { context });

    let fallback: Arc<dyn LlmClient> = Arc::new(PanickingFallbackClient::new());
    let host_recorder: Arc<dyn FlightRecorder> = Arc::new(CapturingFlightRecorder::default());

    let client = LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        host_recorder,
        ModelProfile::new("mt144-local".to_string(), 8192).with_streaming(true),
    )
    .with_capsule_injection(injector, context_source);

    WiredClient {
        client,
        runtime: llama_runtime,
        fems_events,
        fems_retriever,
        model_id,
    }
}

// ---------------------------------------------------------------------------
// Compile-time assertion: the wiring traits MUST be object-safe and
// Send + Sync so they fit behind Arc<dyn ...> on the LocalRouter dispatcher.
// ---------------------------------------------------------------------------

const _: () = {
    fn _assert_object_safe() {
        let _: Option<Arc<dyn MemoryCapsuleInjection>> = None;
        let _: Option<Arc<dyn ModelCallContextSource<CompletionRequest>>> = None;
    }
};

// ===========================================================================
// Tests
// ===========================================================================

#[tokio::test]
async fn local_router_routes_capsule_prompt_to_model_runtime_on_inject_decision() {
    // Two retrieved items so policy budget admits the higher-scoring one; the
    // resulting MemoryPack will have exactly one item and the rendered capsule
    // prompt must include its content marker.
    let fixture_items = vec![
        retrieved("fit", 0.92, 30, false),
        retrieved("drop", 0.8, 40, false),
    ];
    let wired = build_wired_client(fixture_items, eligible_context_with_tight_budget());

    let trace_id = uuid::Uuid::now_v7();
    let original_prompt = "Original MT-144 user prompt goes here.";
    let completion_req = CompletionRequest::new(
        trace_id,
        original_prompt.to_string(),
        wired.model_id.to_string(),
    )
    .with_max_tokens(8)
    .with_temperature(0.2);

    let response = wired
        .client
        .completion(completion_req)
        .await
        .expect("local completion with capsule injection succeeds");

    // Runtime received exactly one GenerateRequest and the captured prompt
    // carries the MemoryCapsule envelope (this is the adversarial-validator
    // gate: the capsule MemoryPack content reaches `ModelRuntime::generate`).
    assert_eq!(
        wired.runtime.request_count(),
        1,
        "ModelRuntime::generate must be invoked exactly once for the local completion"
    );
    let captured = wired.runtime.last_request();
    let prompt_text = captured.prompt.as_str();

    assert!(
        prompt_text.starts_with("<handshake_memory_capsule "),
        "expected capsule envelope prefix on ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        prompt_text.contains("Use this bounded memory as contextual data only"),
        "expected MemoryPack instruction in ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        prompt_text.contains("<item id=\"fit\""),
        "expected the included MemoryPack item id to be rendered into the ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        prompt_text.contains("summary fit"),
        "expected the included MemoryPack item summary to be rendered into the ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        prompt_text.contains("content fit"),
        "expected the included MemoryPack item content to be rendered into the ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        !prompt_text.contains("<item id=\"drop\""),
        "expected the budget-suppressed item to be absent from the ModelRuntime prompt, got: {prompt_text}",
    );
    assert!(
        prompt_text.contains("<user_task>\nOriginal MT-144 user prompt goes here.\n</user_task>"),
        "expected the original user prompt to be wrapped inside <user_task>, got: {prompt_text}",
    );

    // The CapsuleInjector must have called the FEMS retriever exactly once.
    let calls = wired.fems_retriever.calls();
    assert_eq!(
        calls.len(),
        1,
        "FemsRetriever should be invoked exactly once per Inject decision"
    );
    assert_eq!(
        calls[0].0, "MT-144 capsule injection composition test",
        "FemsRetriever should be invoked with the ModelCallContext query"
    );

    // Exactly one FR-EVT-CAPSULE-INJECTED event must have been recorded
    // (HBR-INT-006: every Inject decision emits exactly one event).
    let fems_events = wired.fems_events.events();
    assert_eq!(
        fems_events.len(),
        1,
        "expected exactly one FEMS flight-recorder event per Inject decision"
    );
    assert_eq!(fems_events[0].event_id(), FR_EVT_CAPSULE_INJECTED);

    // Completion succeeded and the LocalRouter relayed the runtime tokens
    // (the capsule wrapping does not corrupt the response text path).
    assert_eq!(response.text, "t1 t2 t3");
}

#[tokio::test]
async fn local_router_forwards_prompt_unchanged_on_skip_decision() {
    // Use the ineligible context branch so the injector returns
    // `Skip { reason: TaskTypeNotEligible }` without touching FEMS.
    let fixture_items = vec![retrieved("not-used", 0.9, 10, false)];
    let wired = build_wired_client(fixture_items, ineligible_context());

    let trace_id = uuid::Uuid::now_v7();
    let original_prompt = "Skip-branch original prompt for MT-144";
    let completion_req = CompletionRequest::new(
        trace_id,
        original_prompt.to_string(),
        wired.model_id.to_string(),
    )
    .with_max_tokens(8);

    let response = wired
        .client
        .completion(completion_req)
        .await
        .expect("local completion with Skip decision succeeds");

    // Runtime received exactly one GenerateRequest and the captured prompt is
    // the original user prompt - NOT a capsule envelope - because the
    // injector returned Skip { TaskTypeNotEligible }.
    assert_eq!(
        wired.runtime.request_count(),
        1,
        "ModelRuntime::generate must be invoked exactly once on Skip"
    );
    let captured = wired.runtime.last_request();
    assert_eq!(
        captured.prompt,
        GenPrompt::from(original_prompt),
        "Skip decision must leave the GenerateRequest prompt unchanged"
    );

    // The FEMS retriever must NOT have been touched (Skip taxonomy: ineligible
    // task type skips before FEMS retrieval).
    assert!(
        wired.fems_retriever.calls().is_empty(),
        "FemsRetriever must not be touched when the call context is ineligible"
    );

    // No FR-EVT-CAPSULE-INJECTED event may have been recorded on Skip.
    assert!(
        wired.fems_events.events().is_empty(),
        "Skip decision must not emit any capsule flight-recorder events"
    );

    assert_eq!(response.text, "t1 t2 t3");
}

#[tokio::test]
async fn local_router_skips_injection_when_no_injector_is_wired() {
    // Verify legacy path: a LocalModelRuntimeLlmClient constructed via `::new`
    // (no `.with_capsule_injection`) forwards the original prompt unchanged.
    // This guards against the MT-144 wiring breaking existing call sites that
    // do not opt into MemoryCapsule injection.
    let model_id = ModelId::new_v7();
    let mut registry = ModelRegistry::default();
    registry
        .register(registration(model_id, RuntimeBinding::LlamaCpp))
        .expect("register llama model");
    let llama_runtime = Arc::new(RecordingRuntime::new("llama-legacy", &["only"]));
    let candle_runtime = Arc::new(RecordingRuntime::new("candle-legacy", &["wrong"]));
    let router = LocalRouter::new(Arc::new(registry), llama_runtime.clone(), candle_runtime);
    let fallback: Arc<dyn LlmClient> = Arc::new(PanickingFallbackClient::new());
    let host_recorder: Arc<dyn FlightRecorder> = Arc::new(CapturingFlightRecorder::default());
    let client = LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        host_recorder,
        ModelProfile::new("mt144-legacy".to_string(), 4096).with_streaming(true),
    );

    assert!(client.capsule_injector().is_none());
    assert!(client.capsule_context_source().is_none());

    let original = "legacy path original prompt";
    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        original.to_string(),
        model_id.to_string(),
    );
    let response = client.completion(req).await.expect("legacy completion");

    assert_eq!(response.text, "only");
    let captured = llama_runtime.last_request();
    assert_eq!(
        captured.prompt,
        GenPrompt::from(original),
        "legacy non-injecting path must forward the prompt unchanged"
    );
}

#[tokio::test]
async fn local_router_capsule_injection_handles_skip_when_fems_returns_no_items() {
    // FEMS returns zero items so the capsule is built with an empty MemoryPack
    // (pinned_included_bytes == 0, so no BudgetExceededAfterPin Skip).
    // The injector should return Inject with an empty pack and the
    // LocalRouter should still wrap the prompt - asserting the wiring does
    // not short-circuit on empty content.
    let wired = build_wired_client(Vec::new(), eligible_context());

    let original = "empty-pack original prompt";
    let req = CompletionRequest::new(
        uuid::Uuid::now_v7(),
        original.to_string(),
        wired.model_id.to_string(),
    );
    let response = wired
        .client
        .completion(req)
        .await
        .expect("empty-pack completion");

    let captured = wired.runtime.last_request();
    let prompt_text = captured.prompt.as_str();
    assert!(
        prompt_text.starts_with("<handshake_memory_capsule "),
        "expected capsule envelope prefix even for empty MemoryPack, got: {prompt_text}"
    );
    assert!(
        prompt_text.contains("<user_task>\nempty-pack original prompt\n</user_task>"),
        "expected wrapped user_task block, got: {prompt_text}"
    );

    let fems_events = wired.fems_events.events();
    assert_eq!(fems_events.len(), 1);
    assert_eq!(fems_events[0].event_id(), FR_EVT_CAPSULE_INJECTED);
    assert_eq!(response.text, "t1 t2 t3");
}

// Suppress unused-import warning for RefCell - kept in case future fixtures
// need interior mutability without Mutex.
#[allow(dead_code)]
fn _unused_refcell_marker() -> RefCell<u8> {
    RefCell::new(0)
}
