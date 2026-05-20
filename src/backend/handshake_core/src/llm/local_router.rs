use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    // WAIVER [CX-573E]: local LLM latency measurement is observability metadata.
    time::Instant,
};

use async_trait::async_trait;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    flight_recorder::{
        FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
        LlmInferenceEvent, LlmInferenceTokenUsage, RecorderError,
    },
    memory::{
        attach_capsule_to_generate_request, InjectionDecision, MemoryCapsuleInjection,
        MemoryInjectionReceipt, ModelCallContextSource,
    },
    model_runtime::{
        CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRegistry, ModelRuntime,
        ModelRuntimeError, ProviderKind, RuntimeBinding, SamplingParams,
    },
};

use super::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage};

#[derive(Clone)]
pub struct LocalRouter {
    registry: Arc<ModelRegistry>,
    llama_runtime: Arc<dyn ModelRuntime>,
    candle_runtime: Arc<dyn ModelRuntime>,
}

impl LocalRouter {
    pub fn new(
        registry: Arc<ModelRegistry>,
        llama_runtime: Arc<dyn ModelRuntime>,
        candle_runtime: Arc<dyn ModelRuntime>,
    ) -> Self {
        Self {
            registry,
            llama_runtime,
            candle_runtime,
        }
    }

    pub fn resolve(&self, model_id: ModelId) -> Result<Arc<dyn ModelRuntime>, LlmError> {
        let registration = self.registry.lookup(model_id).ok_or_else(|| {
            LlmError::ProviderError(format!("local model is not registered: {model_id}"))
        })?;

        if registration.provider != ProviderKind::Local {
            return Err(LlmError::ProviderError(format!(
                "registered model provider is not local: {:?}",
                registration.provider
            )));
        }

        Ok(match registration.runtime_binding {
            RuntimeBinding::LlamaCpp => Arc::clone(&self.llama_runtime),
            RuntimeBinding::Candle => Arc::clone(&self.candle_runtime),
        })
    }
}

pub struct LocalModelRuntimeLlmClient {
    router: LocalRouter,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    profile: ModelProfile,
    active_cancellations: Mutex<HashMap<ModelId, CancellationToken>>,
    // MT-144 wiring: optional MemoryCapsule injection per HBR-INT-006.
    //
    // When both `capsule_injector` and `capsule_context_source` are populated,
    // every local GenerateRequest produced by `completion()` is routed through
    // `MemoryCapsuleInjection::inject_for_call` before being handed to the
    // underlying ModelRuntime. On an `Inject` decision the GenerateRequest's
    // prompt is wrapped via `attach_capsule_to_generate_request` so the
    // ModelRuntime adapter receives the capsule's MemoryPack content; on a
    // `Skip` decision the prompt is forwarded unchanged. Either field being
    // `None` preserves the legacy non-injecting code path so existing call
    // sites (and the in-tree `llm_client_local_routing_tests` fleet) do not
    // need to be reworked.
    capsule_injector: Option<Arc<dyn MemoryCapsuleInjection>>,
    capsule_context_source: Option<Arc<dyn ModelCallContextSource<CompletionRequest>>>,
}

impl LocalModelRuntimeLlmClient {
    pub fn new(
        router: LocalRouter,
        fallback: Arc<dyn LlmClient>,
        flight_recorder: Arc<dyn FlightRecorder>,
        profile: ModelProfile,
    ) -> Self {
        Self {
            router,
            fallback,
            flight_recorder,
            profile,
            active_cancellations: Mutex::new(HashMap::new()),
            capsule_injector: None,
            capsule_context_source: None,
        }
    }

    /// Wires the MemoryCapsule injection surface (MT-144) into this LocalRouter
    /// dispatcher. Both arguments must be present for injection to be active;
    /// callers that do not want injection should construct the client via
    /// [`Self::new`] and skip this builder method entirely.
    ///
    /// Operator waiver 2026-05-20T22:30:00Z (MT-070 scope expansion) authorises
    /// this wiring so the adversarial validator finding for MT-144
    /// ("CapsuleInjector exists in isolation but is never wired into the
    /// ModelRuntime generate call path") is resolved at the real
    /// runtime.generate dispatch boundary.
    pub fn with_capsule_injection(
        mut self,
        injector: Arc<dyn MemoryCapsuleInjection>,
        context_source: Arc<dyn ModelCallContextSource<CompletionRequest>>,
    ) -> Self {
        self.capsule_injector = Some(injector);
        self.capsule_context_source = Some(context_source);
        self
    }

    /// Returns the capsule injector wired into this dispatcher, if any.
    pub fn capsule_injector(&self) -> Option<&Arc<dyn MemoryCapsuleInjection>> {
        self.capsule_injector.as_ref()
    }

    /// Returns the capsule call-context source wired into this dispatcher, if any.
    pub fn capsule_context_source(
        &self,
    ) -> Option<&Arc<dyn ModelCallContextSource<CompletionRequest>>> {
        self.capsule_context_source.as_ref()
    }

    /// Applies MemoryCapsule injection to `generate_request` if the dispatcher
    /// is wired for it AND the per-request context source produces an eligible
    /// [`ModelCallContext`] for `req`.
    ///
    /// Returns the (possibly wrapped) GenerateRequest and an optional
    /// `MemoryInjectionReceipt` recording the capsule handle and prompt-hash
    /// transition. On any `Skip` decision (operator opt-out, ineligible task
    /// type, FEMS unavailable, budget overrun after pin) the request is
    /// returned unchanged with a `None` receipt; on injector error the error
    /// is mapped into `LlmError::ProviderError` so the runtime dispatch path
    /// short-circuits before `runtime.generate` is called.
    fn apply_capsule_injection(
        &self,
        req: &CompletionRequest,
        generate_request: GenerateRequest,
    ) -> Result<(GenerateRequest, Option<MemoryInjectionReceipt>), LlmError> {
        let (Some(injector), Some(context_source)) = (
            self.capsule_injector.as_ref(),
            self.capsule_context_source.as_ref(),
        ) else {
            return Ok((generate_request, None));
        };

        let Some(call_ctx) = context_source.model_call_context(req) else {
            return Ok((generate_request, None));
        };

        let decision = injector.inject_for_call(&call_ctx).map_err(|err| {
            LlmError::ProviderError(format!("HSK-500-LLM: capsule injection failed: {err}"))
        })?;

        match decision {
            InjectionDecision::Inject {
                capsule,
                capsule_handle,
            } => {
                let (wrapped, receipt) =
                    attach_capsule_to_generate_request(generate_request, &capsule, capsule_handle);
                Ok((wrapped, Some(receipt)))
            }
            InjectionDecision::Skip { .. } => Ok((generate_request, None)),
        }
    }

    fn active_tokens(&self) -> MutexGuard<'_, HashMap<ModelId, CancellationToken>> {
        match self.active_cancellations.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn parse_local_model_id(model_id: &str) -> Result<Option<ModelId>, LlmError> {
        let trimmed = model_id.trim();
        let parsed = match Uuid::parse_str(trimmed) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(None),
        };

        if parsed.get_version_num() != 7 {
            return Ok(None);
        }

        Ok(Some(ModelId::from(parsed)))
    }

    fn request_to_generate_request(
        &self,
        req: &CompletionRequest,
        model_id: ModelId,
        cancel: CancellationToken,
    ) -> GenerateRequest {
        GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from(req.prompt.clone()),
            sampling: SamplingParams {
                temperature: Some(req.temperature),
                ..Default::default()
            },
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel,
            max_tokens: req.max_tokens.unwrap_or(self.profile.max_context_tokens),
            stop_sequences: req.stop_sequences.clone(),
            structured_decoding: None,
        }
    }

    fn map_runtime_error(error: ModelRuntimeError) -> LlmError {
        LlmError::ProviderError(format!("local ModelRuntime error: {error}"))
    }

    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn estimate_prompt_tokens(prompt: &str) -> u32 {
        let count = prompt.split_whitespace().count();
        count.min(u32::MAX as usize) as u32
    }

    async fn emit_llm_inference_event(
        &self,
        req: &CompletionRequest,
        response_text: &str,
        usage: &TokenUsage,
        latency_ms: u64,
    ) {
        let payload = LlmInferenceEvent {
            event_type: "llm_inference".to_string(),
            trace_id: req.trace_id,
            model_id: req.model_id.clone(),
            token_usage: LlmInferenceTokenUsage {
                prompt_tokens: usage.prompt_tokens as u64,
                completion_tokens: usage.completion_tokens as u64,
                total_tokens: usage.total_tokens as u64,
            },
            prompt_hash: Some(Self::compute_hash(&req.prompt)),
            response_hash: Some(Self::compute_hash(response_text)),
            latency_ms: Some(latency_ms),
        };

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::LlmInference,
            FlightRecorderActor::Agent,
            req.trace_id,
            serde_json::to_value(&payload).unwrap_or_default(),
        )
        .with_model_id(&req.model_id);

        let record_result: Result<(), RecorderError> =
            self.flight_recorder.record_event(event).await;
        if let Err(err) = record_result {
            tracing::warn!(
                target: "handshake_core::llm",
                error = %err,
                trace_id = %req.trace_id,
                "Failed to record local llm_inference event"
            );
        }
    }
}

#[async_trait]
impl LlmClient for LocalModelRuntimeLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let Some(model_id) = Self::parse_local_model_id(&req.model_id)? else {
            return self.fallback.completion(req).await;
        };

        let started = Instant::now();
        let runtime = self.router.resolve(model_id)?;
        let cancel = CancellationToken::new();
        self.active_tokens().insert(model_id, cancel.clone());
        let generate_request = self.request_to_generate_request(&req, model_id, cancel);
        // MT-144: wire MemoryCapsule injection into the ModelRuntime generate
        // call path. On `Inject` the prompt is wrapped via
        // `attach_capsule_to_generate_request`; on `Skip` it is unchanged.
        // FR-EVT-CAPSULE-INJECTED is emitted inside `inject_for_call` itself.
        let (generate_request, _capsule_receipt) =
            match self.apply_capsule_injection(&req, generate_request) {
                Ok(pair) => pair,
                Err(err) => {
                    self.active_tokens().remove(&model_id);
                    return Err(err);
                }
            };
        let mut stream = runtime.generate(generate_request);
        let mut text = String::new();
        let mut completion_tokens = 0_u32;
        let mut result = Ok(());

        while let Some(token) = stream.next().await {
            let token = match token {
                Ok(token) => token,
                Err(error) => {
                    result = Err(Self::map_runtime_error(error));
                    break;
                }
            };
            text.push_str(&token.text);
            completion_tokens = completion_tokens.saturating_add(1);
            if let Some(max_tokens) = req.max_tokens {
                if completion_tokens > max_tokens {
                    result = Err(LlmError::BudgetExceeded(completion_tokens));
                    break;
                }
            }
        }
        self.active_tokens().remove(&model_id);
        result?;

        let prompt_tokens = Self::estimate_prompt_tokens(&req.prompt);
        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
        };
        let response = CompletionResponse {
            text,
            usage,
            latency_ms: (started.elapsed().as_millis() as u64).max(1),
        };

        self.emit_llm_inference_event(&req, &response.text, &response.usage, response.latency_ms)
            .await;

        Ok(response)
    }

    fn cancel(&self, model_id: &str, token: CancellationToken) {
        let route = match Self::parse_local_model_id(model_id) {
            Ok(route) => route,
            Err(_) => return,
        };

        let Some(model_id) = route else {
            self.fallback.cancel(model_id, token);
            return;
        };

        if let Ok(runtime) = self.router.resolve(model_id) {
            if let Some(active_token) = self.active_tokens().get(&model_id).cloned() {
                runtime.cancel(active_token);
            }
            runtime.cancel(token);
        }
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}
