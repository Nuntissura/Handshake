use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    // WAIVER [CX-573E]: local LLM latency measurement is observability metadata.
    time::{Duration, Instant},
};

use async_trait::async_trait;
use futures::{future::join_all, StreamExt};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

use crate::{
    flight_recorder::{
        FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
        LlmInferenceEvent, LlmInferenceTokenUsage, RecorderError,
    },
    kernel::KernelActor,
    memory::{
        attach_capsule_to_generate_request, InjectionDecision, MemoryCapsuleInjection,
        MemoryInjectionReceipt, ModelCallContextSource,
    },
    model_runtime::{
        CancellationToken, ExplicitModelRuntimeRebind, GenPrompt, GenerateRequest, KvCachePolicy,
        LoadSpec, ModelCatalog, ModelId, ModelRegistration, ModelRegistry, ModelRegistryStore,
        ModelRuntime, ModelRuntimeAvailability, ModelRuntimeError, ModelRuntimeSelection,
        ModelRuntimeSelectionPurpose, ProviderKind, RuntimeBinding, RuntimeVramResidency,
        SamplingParams,
    },
    process_ledger::{EmbeddedRuntimeInstanceDescriptor, LedgerBatcher},
};

use super::{
    embedded_ledger::EmbeddedModelProcess, emit_llm_call_error_event, CompletionRequest,
    CompletionResponse, EmbeddingRequest, EmbeddingResponse, LlmClient, LlmError, ModelProfile,
    ModelRuntimeControlAction, ModelRuntimeControlCapabilities, ModelRuntimeControlReceipt,
    ModelRuntimeControlRequest, ModelRuntimeInspection, ModelRuntimeKvInspection,
    ModelRuntimeLoraInspection, ModelRuntimeSteeringInspection, ModelRuntimeValue, TokenUsage,
    MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
};

const EMBEDDED_RUNTIME_QUIESCE_TIMEOUT: Duration = Duration::from_secs(5);
const MODEL_RUNTIME_CONTROL_RECEIPT_CACHE_CAPACITY: usize = 128;

#[derive(Clone)]
pub struct LocalRouter {
    registry: Arc<ModelRegistry>,
    llama_runtime: Arc<Mutex<Option<Arc<dyn ModelRuntime>>>>,
    candle_runtime: Arc<Mutex<Option<Arc<dyn ModelRuntime>>>>,
    runtime_availability: Arc<ModelRuntimeAvailability>,
}

impl LocalRouter {
    pub fn new(
        registry: Arc<ModelRegistry>,
        llama_runtime: Arc<dyn ModelRuntime>,
        candle_runtime: Arc<dyn ModelRuntime>,
    ) -> Self {
        Self {
            registry,
            llama_runtime: Arc::new(Mutex::new(Some(llama_runtime))),
            candle_runtime: Arc::new(Mutex::new(Some(candle_runtime))),
            runtime_availability: Arc::new(ModelRuntimeAvailability::default()),
        }
    }

    fn runtime_slot(&self, binding: RuntimeBinding) -> &Arc<Mutex<Option<Arc<dyn ModelRuntime>>>> {
        match binding {
            RuntimeBinding::LlamaCpp => &self.llama_runtime,
            RuntimeBinding::Candle => &self.candle_runtime,
        }
    }

    fn lock_runtime_slot(
        &self,
        binding: RuntimeBinding,
    ) -> Result<MutexGuard<'_, Option<Arc<dyn ModelRuntime>>>, LlmError> {
        self.runtime_slot(binding).lock().map_err(|_| {
            LlmError::ProviderError(format!(
                "local {} runtime ownership slot is poisoned",
                binding.adapter_id()
            ))
        })
    }

    fn take_runtime_for_unload(
        &self,
        binding: RuntimeBinding,
    ) -> Result<Arc<dyn ModelRuntime>, LlmError> {
        let mut slot = self.lock_runtime_slot(binding)?;
        slot.take().ok_or_else(|| {
            LlmError::ProviderError(format!(
                "local {} runtime ownership was already consumed",
                binding.adapter_id()
            ))
        })
    }

    fn restore_runtime_after_failed_unload(
        &self,
        binding: RuntimeBinding,
        runtime: Arc<dyn ModelRuntime>,
    ) -> Result<(), LlmError> {
        let mut slot = self.lock_runtime_slot(binding)?;
        if slot.is_some() {
            return Err(LlmError::ProviderError(format!(
                "local {} runtime ownership slot was concurrently repopulated",
                binding.adapter_id()
            )));
        }
        *slot = Some(runtime);
        Ok(())
    }

    fn registration(&self, model_id: ModelId) -> Option<crate::model_runtime::ModelRegistration> {
        self.runtime_availability
            .replacement(model_id)
            .map(|(registration, _)| registration)
            .or_else(|| self.registry.lookup(model_id).cloned())
    }

    fn registrations(&self) -> Vec<crate::model_runtime::ModelRegistration> {
        let mut registrations = self
            .registry
            .list()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        registrations.extend(
            self.runtime_availability
                .replacements()
                .into_iter()
                .map(|(registration, _)| registration),
        );
        registrations
    }

    pub fn resolve(&self, model_id: ModelId) -> Result<Arc<dyn ModelRuntime>, LlmError> {
        if !self.runtime_availability.is_available(model_id) {
            return Err(LlmError::ProviderError(format!(
                "local model is unloaded: {model_id}"
            )));
        }
        let registration = self.registration(model_id).ok_or_else(|| {
            LlmError::ProviderError(format!("local model is not registered: {model_id}"))
        })?;

        if registration.provider != ProviderKind::Local {
            return Err(LlmError::ProviderError(format!(
                "registered model provider is not local: {:?}",
                registration.provider
            )));
        }

        let slot = self.lock_runtime_slot(registration.runtime_binding)?;
        slot.as_ref().cloned().ok_or_else(|| {
            LlmError::ProviderError(format!(
                "local {} runtime is unloading or already unloaded",
                registration.runtime_binding.adapter_id()
            ))
        })
    }

    fn runtime_availability(&self) -> Arc<ModelRuntimeAvailability> {
        Arc::clone(&self.runtime_availability)
    }

    pub fn require_embedding_model(&self, model_id: ModelId) -> Result<usize, LlmError> {
        let registration = self.registration(model_id).ok_or_else(|| {
            LlmError::ProviderError(format!("local model is not registered: {model_id}"))
        })?;
        if !registration.declared_capabilities.supports_embedding {
            return Err(LlmError::EmbeddingUnsupported);
        }
        registration
            .declared_capabilities
            .embedding_dimension
            .ok_or(LlmError::EmbeddingUnsupported)
    }
}

pub struct LocalModelRuntimeLlmClient {
    router: LocalRouter,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    profile: ModelProfile,
    selected_model_id: Mutex<String>,
    durable_selection_store: Option<ModelRegistryStore>,
    active_application_selection_revision: Mutex<Option<u64>>,
    active_requests: Mutex<HashMap<Uuid, ActiveLocalRequest>>,
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
    // MT-014 wiring: the shared, enumerable, labeled ModelCatalog over the SAME
    // `Arc<ModelRegistry>` this router dispatches through. Populated by the boot
    // assembler (`assemble_local_runtime_client`) so a surface reachable from
    // `AppState.llm_client` can enumerate/label the configured local model(s).
    // `None` for clients constructed via `new` without a catalog (e.g. the
    // in-tree routing-test fleet), which preserves the legacy accessor contract.
    catalog: Option<Arc<ModelCatalog>>,
    // WP-1 MT-013: ProcessOwnershipLedger ownership handle for the in-process
    // embedded model this client's LocalRouter dispatches to. `Some` only for
    // the default boot lane assembled with a ledger handle
    // (`boot::assemble_local_runtime_client`). Graceful shutdown first closes
    // runtime admission, proves quiescence, takes the router's runtime Arc,
    // requires unique ownership, and completes `ModelRuntime::unload`. Only
    // then may the corresponding reserved STOP be emitted.
    embedded_processes: Mutex<Vec<Arc<EmbeddedModelProcess>>>,
    runtime_control_ledger: Option<LedgerBatcher>,
    runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
    graceful_shutdown_serial: AsyncMutex<()>,
    model_swap_serial: AsyncMutex<()>,
    /// Bounded idempotency surface for destructive runtime-control requests.
    /// Partial receipts are retained exactly like complete receipts so a retry
    /// cannot repeat a mutation after the process-ledger verdict was lost.
    runtime_control_receipts: Mutex<HashMap<Uuid, CachedRuntimeControlReceipt>>,
    graceful_shutdown_complete: AtomicBool,
}

#[derive(Clone)]
struct ActiveLocalRequest {
    model_id: ModelId,
    cancel: CancellationToken,
}

#[derive(Clone)]
struct CachedRuntimeControlReceipt {
    request: ModelRuntimeControlRequest,
    receipt: ModelRuntimeControlReceipt,
}

fn validate_cached_control_request(
    cached: &CachedRuntimeControlReceipt,
    req: &ModelRuntimeControlRequest,
) -> Result<(), LlmError> {
    if cached.request != *req {
        return Err(LlmError::ProviderError(format!(
            "model runtime control request_id {} was already used with a different immutable request envelope",
            req.request_id
        )));
    }
    Ok(())
}

struct ActiveLocalRequestGuard<'a> {
    active_requests: &'a Mutex<HashMap<Uuid, ActiveLocalRequest>>,
    request_id: Uuid,
    cancel: CancellationToken,
}

impl Drop for ActiveLocalRequestGuard<'_> {
    fn drop(&mut self) {
        // Dropping a request future/stream must signal the detached runtime
        // worker before removing its token from the client-level index.
        self.cancel.cancel();
        let mut requests = match self.active_requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };
        requests.remove(&self.request_id);
    }
}

impl LocalModelRuntimeLlmClient {
    pub fn new(
        router: LocalRouter,
        fallback: Arc<dyn LlmClient>,
        flight_recorder: Arc<dyn FlightRecorder>,
        profile: ModelProfile,
    ) -> Self {
        let selected_model_id = profile.model_id.clone();
        Self {
            router,
            fallback,
            flight_recorder,
            profile,
            selected_model_id: Mutex::new(selected_model_id),
            durable_selection_store: None,
            active_application_selection_revision: Mutex::new(None),
            active_requests: Mutex::new(HashMap::new()),
            capsule_injector: None,
            capsule_context_source: None,
            catalog: None,
            embedded_processes: Mutex::new(Vec::new()),
            runtime_control_ledger: None,
            runtime_instance: None,
            graceful_shutdown_serial: AsyncMutex::new(()),
            model_swap_serial: AsyncMutex::new(()),
            runtime_control_receipts: Mutex::new(HashMap::new()),
            graceful_shutdown_complete: AtomicBool::new(false),
        }
    }

    fn remaining_control_timeout(
        started: Instant,
        total: Duration,
        phase: &str,
    ) -> Result<Duration, LlmError> {
        total
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                LlmError::ProviderError(format!(
                    "model runtime control timeout expired before {phase}"
                ))
            })
    }

    fn cached_control_receipt(
        &self,
        req: &ModelRuntimeControlRequest,
    ) -> Result<Option<ModelRuntimeControlReceipt>, LlmError> {
        let receipts = self
            .runtime_control_receipts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(cached) = receipts.get(&req.request_id) else {
            return Ok(None);
        };
        validate_cached_control_request(cached, req)?;
        Ok(Some(cached.receipt.clone()))
    }

    fn cache_control_receipt(
        &self,
        request: &ModelRuntimeControlRequest,
        receipt: ModelRuntimeControlReceipt,
    ) -> ModelRuntimeControlReceipt {
        let mut receipts = self
            .runtime_control_receipts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if receipts.len() >= MODEL_RUNTIME_CONTROL_RECEIPT_CACHE_CAPACITY
            && !receipts.contains_key(&receipt.request_id)
        {
            if let Some(oldest_available) = receipts.keys().copied().min() {
                receipts.remove(&oldest_available);
            }
        }
        receipts.insert(
            receipt.request_id,
            CachedRuntimeControlReceipt {
                request: request.clone(),
                receipt: receipt.clone(),
            },
        );
        receipt
    }

    /// WP-1 MT-013: attaches the ProcessOwnershipLedger ownership handle for the
    /// in-process embedded model this client dispatches to. The boot assembler
    /// (`boot::assemble_local_runtime_client`) passes the handle returned by
    /// `EmbeddedModelProcess::record_load` (which already emitted the START
    /// row), so the client owns the STOP-on-shutdown obligation for the loaded
    /// model. Clients constructed without a ledger handle keep `None`.
    pub fn with_embedded_process(mut self, embedded_process: EmbeddedModelProcess) -> Self {
        self.embedded_processes
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(Arc::new(embedded_process));
        self
    }

    pub fn with_runtime_control_authority(
        mut self,
        ledger: LedgerBatcher,
        runtime_instance: EmbeddedRuntimeInstanceDescriptor,
    ) -> Self {
        self.runtime_control_ledger = Some(ledger);
        self.runtime_instance = Some(runtime_instance);
        self
    }

    /// Attaches the shared [`ModelCatalog`] enumeration/label surface (MT-014).
    /// The boot assembler passes the catalog built over the SAME
    /// `Arc<ModelRegistry>` this client's `LocalRouter` routes through, so the
    /// enumeration surface can never drift from the registry that dispatches.
    pub fn with_catalog(mut self, catalog: Arc<ModelCatalog>) -> Self {
        catalog.bind_runtime_availability(self.router.runtime_availability());
        self.catalog = Some(catalog);
        self
    }

    /// Attach the PostgreSQL authority that owns application/default. The
    /// process-local id/revision remain a current-boot projection only.
    pub fn with_durable_application_selection(
        mut self,
        store: ModelRegistryStore,
        selection_revision: u64,
    ) -> Self {
        self.durable_selection_store = Some(store);
        self.active_application_selection_revision = Mutex::new(Some(selection_revision));
        self
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

    fn active_requests(&self) -> MutexGuard<'_, HashMap<Uuid, ActiveLocalRequest>> {
        match self.active_requests.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn register_active_request(
        &self,
        model_id: ModelId,
        cancel: CancellationToken,
    ) -> ActiveLocalRequestGuard<'_> {
        let request_id = Uuid::now_v7();
        let guard_cancel = cancel.clone();
        self.active_requests()
            .insert(request_id, ActiveLocalRequest { model_id, cancel });
        ActiveLocalRequestGuard {
            active_requests: &self.active_requests,
            request_id,
            cancel: guard_cancel,
        }
    }

    fn cancel_all_active_requests(&self) {
        let requests = self.active_requests().values().cloned().collect::<Vec<_>>();
        for request in requests {
            if let Ok(runtime) = self.router.resolve(request.model_id) {
                runtime.cancel(request.cancel);
            } else {
                request.cancel.cancel();
            }
        }
    }

    fn embedded_processes(&self) -> Vec<Arc<EmbeddedModelProcess>> {
        self.embedded_processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn remove_embedded_process(&self, process_uuid: Uuid) -> bool {
        let mut processes = self
            .embedded_processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = processes.len();
        processes.retain(|process| process.process_uuid() != process_uuid);
        processes.len() != before
    }

    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn retire_embedded_process_for_tests(&self, process_uuid: Uuid) -> bool {
        self.remove_embedded_process(process_uuid)
    }

    fn resolve_embedded_runtimes(&self) -> Result<Vec<Arc<dyn ModelRuntime>>, LlmError> {
        let mut runtimes = Vec::<Arc<dyn ModelRuntime>>::new();
        let embedded_processes = self.embedded_processes();
        for process in &embedded_processes {
            let model_id = ModelId::from(process.process_uuid());
            let runtime = self.router.resolve(model_id).map_err(|error| {
                LlmError::ProviderError(format!(
                    "embedded lifecycle {} has no resolvable runtime for quiescence: {error}",
                    process.process_uuid()
                ))
            })?;
            if !runtimes
                .iter()
                .any(|existing| Arc::ptr_eq(existing, &runtime))
            {
                runtimes.push(runtime);
            }
        }
        if runtimes.is_empty() && !embedded_processes.is_empty() {
            return Err(LlmError::ProviderError(
                "embedded lifecycle set resolved to zero runtime quiescence authorities"
                    .to_string(),
            ));
        }
        Ok(runtimes)
    }

    async fn quiesce_embedded_runtimes(&self) -> Result<(), LlmError> {
        let runtimes = self.resolve_embedded_runtimes()?;
        // Poll every distinct runtime barrier concurrently. Each receives the
        // same wall-clock budget, so multiple adapters cannot multiply the
        // shutdown deadline and all admission gates close together.
        let results = join_all(runtimes.iter().map(|runtime| async move {
            (
                runtime.adapter_name(),
                runtime.quiesce(EMBEDDED_RUNTIME_QUIESCE_TIMEOUT).await,
            )
        }))
        .await;
        let errors = results
            .into_iter()
            .filter_map(|(adapter, result)| result.err().map(|error| format!("{adapter}: {error}")))
            .collect::<Vec<_>>();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LlmError::ProviderError(format!(
                "embedded runtime quiescence failed within {:?}: {}",
                EMBEDDED_RUNTIME_QUIESCE_TIMEOUT,
                errors.join("; ")
            )))
        }
    }

    fn embedded_models_by_binding(&self) -> Result<Vec<(RuntimeBinding, Vec<ModelId>)>, LlmError> {
        let mut llama_models = Vec::new();
        let mut candle_models = Vec::new();
        for process in self.embedded_processes() {
            let model_id = ModelId::from(process.process_uuid());
            let registration = self.router.registration(model_id).ok_or_else(|| {
                LlmError::ProviderError(format!(
                    "embedded lifecycle {} has no registry selection for unload",
                    process.process_uuid()
                ))
            })?;
            if registration.provider != ProviderKind::Local {
                return Err(LlmError::ProviderError(format!(
                    "embedded lifecycle {} selected non-local provider {:?}",
                    process.process_uuid(),
                    registration.provider
                )));
            }
            match registration.runtime_binding {
                RuntimeBinding::LlamaCpp => llama_models.push(model_id),
                RuntimeBinding::Candle => candle_models.push(model_id),
            }
        }

        let mut grouped = Vec::new();
        if !llama_models.is_empty() {
            grouped.push((RuntimeBinding::LlamaCpp, llama_models));
        }
        if !candle_models.is_empty() {
            grouped.push((RuntimeBinding::Candle, candle_models));
        }
        Ok(grouped)
    }

    async fn unload_embedded_models(&self) -> Result<(), LlmError> {
        for (binding, model_ids) in self.embedded_models_by_binding()? {
            let mut runtime = self.router.take_runtime_for_unload(binding)?;
            let strong_count = Arc::strong_count(&runtime);
            if strong_count != 1 {
                self.router
                    .restore_runtime_after_failed_unload(binding, runtime)?;
                return Err(LlmError::ProviderError(format!(
                    "embedded {} runtime cannot prove final ownership for unload: {strong_count} strong Arc owners remain",
                    binding.adapter_id()
                )));
            }

            let unload_result = async {
                let runtime = Arc::get_mut(&mut runtime).ok_or_else(|| {
                    LlmError::ProviderError(format!(
                        "embedded {} runtime lost unique ownership before unload",
                        binding.adapter_id()
                    ))
                })?;
                for model_id in model_ids {
                    runtime.unload(model_id).await.map_err(|error| {
                        LlmError::ProviderError(format!(
                            "embedded {} runtime unload failed for {model_id}: {error}",
                            binding.adapter_id()
                        ))
                    })?;
                }
                Ok(())
            }
            .await;

            if let Err(error) = unload_result {
                self.router
                    .restore_runtime_after_failed_unload(binding, runtime)?;
                return Err(error);
            }
            // Successful unload consumes the router's final runtime owner. The
            // Arc is dropped here before any ProcessOwnershipLedger STOP.
        }
        Ok(())
    }

    fn leave_embedded_lifecycles_open(&self) {
        for process in self.embedded_processes() {
            if process.leave_open_for_reconciliation() {
                tracing::warn!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process.process_uuid(),
                    "embedded STOP permit relinquished; START remains open for reconciliation"
                );
            }
        }
    }

    async fn rollback_control_replacement(
        &self,
        binding: RuntimeBinding,
        model_id: ModelId,
        process: &EmbeddedModelProcess,
        timeout: Duration,
        reason: &str,
    ) -> Result<(), LlmError> {
        let mut runtime = self.router.take_runtime_for_unload(binding)?;
        let strong_count = Arc::strong_count(&runtime);
        if strong_count != 1 {
            self.router
                .restore_runtime_after_failed_unload(binding, runtime)?;
            process.leave_open_for_reconciliation();
            return Err(LlmError::ProviderError(format!(
                "replacement rollback cannot take unique {} runtime ownership; {strong_count} Arc owners remain",
                binding.adapter_id()
            )));
        }
        let unload_result = Arc::get_mut(&mut runtime)
            .ok_or_else(|| {
                LlmError::ProviderError(
                    "replacement runtime lost unique ownership before rollback unload".to_string(),
                )
            })?
            .unload(model_id)
            .await
            .map_err(Self::map_runtime_error);
        self.router
            .restore_runtime_after_failed_unload(binding, runtime)?;
        if let Err(error) = unload_result {
            process.leave_open_for_reconciliation();
            return Err(error);
        }
        process
            .shutdown_bounded(reason, timeout)
            .await
            .map_err(|error| {
                LlmError::ProviderError(format!(
                    "replacement rollback unloaded the model but durable STOP failed: {error}"
                ))
            })
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
            speculative_mode: None,
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

    /// WP-1 MT-013: the embedding-lane Flight Recorder event.
    ///
    /// `embedding()` is a Handshake PRODUCT EXTENSION — it is NOT part of the
    /// spec §4.2.3.1 `LlmClient` trait — so this emission is a product extension
    /// of the §4.2.3.2(3)/§11.5 "correlatable model call" discipline, not a
    /// literal §4.2.3.2(3) MUST. Reuses `DataEmbeddingComputed`; embeddings carry
    /// NO `TokenUsage`, so a validator must not reject this event on "embeddings
    /// have no TokenUsage".
    async fn emit_embedding_computed_event(
        &self,
        req: &EmbeddingRequest,
        embedding_dim: usize,
        latency_ms: u64,
    ) {
        let payload = json!({
            "type": "data_embedding_computed",
            "silver_id": format!("embedding-call-{}", req.trace_id.simple()),
            "model_id": req.model_id,
            "model_version": "local-runtime",
            "dimensions": embedding_dim,
            "compute_latency_ms": latency_ms,
            "was_truncated": false,
        });
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::DataEmbeddingComputed,
            FlightRecorderActor::Agent,
            req.trace_id,
            payload,
        )
        .with_model_id(&req.model_id);
        if let Err(err) = self.flight_recorder.record_event(event).await {
            tracing::warn!(
                target: "handshake_core::llm",
                error = %err,
                trace_id = %req.trace_id,
                "failed to record local embedding data_embedding_computed event"
            );
        }
    }

    /// Local-path completion dispatch. Emits the SUCCESS `llm_inference` event on
    /// `Ok`; all error branches return `Err` so the caller (`completion`) emits
    /// the CALL-TIME error event (see spec §4.2.3.2(3)).
    async fn run_local_completion(
        &self,
        req: &CompletionRequest,
        model_id: ModelId,
    ) -> Result<CompletionResponse, LlmError> {
        let started = Instant::now();
        let runtime = self.router.resolve(model_id)?;
        let cancel = CancellationToken::new();
        let _active_request = self.register_active_request(model_id, cancel.clone());
        let generate_request = self.request_to_generate_request(req, model_id, cancel);
        // MT-144: wire MemoryCapsule injection into the ModelRuntime generate
        // call path. On `Inject` the prompt is wrapped via
        // `attach_capsule_to_generate_request`; on `Skip` it is unchanged.
        // FR-EVT-CAPSULE-INJECTED is emitted inside `inject_for_call` itself.
        let (generate_request, _capsule_receipt) =
            self.apply_capsule_injection(req, generate_request)?;
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

        self.emit_llm_inference_event(req, &response.text, &response.usage, response.latency_ms)
            .await;

        Ok(response)
    }

    /// Local-path embedding dispatch. All error branches (runtime resolve, embed
    /// failure, empty-vector rejection) return `Err` so the caller (`embedding`)
    /// emits the CALL-TIME error event. A genuinely empty vector is rejected,
    /// never fabricated.
    async fn run_local_embedding(
        &self,
        req: &EmbeddingRequest,
        model_id: ModelId,
    ) -> Result<EmbeddingResponse, LlmError> {
        let started = Instant::now();
        let expected_dim = self.router.require_embedding_model(model_id)?;
        let runtime = self.router.resolve(model_id)?;
        let embedding = runtime
            .embed(model_id, &req.input)
            .await
            .map_err(Self::map_runtime_error)?;

        if embedding.vector.is_empty() {
            return Err(LlmError::ProviderError(format!(
                "local ModelRuntime returned an empty embedding vector for {model_id}"
            )));
        }
        if embedding.vector.len() != expected_dim {
            return Err(LlmError::EmbeddingDimensionMismatch {
                expected: expected_dim,
                actual: embedding.vector.len(),
            });
        }

        Ok(EmbeddingResponse {
            vector: embedding.vector,
            model_id: req.model_id.clone(),
            latency_ms: (started.elapsed().as_millis() as u64).max(1),
        })
    }
}

#[async_trait]
impl LlmClient for LocalModelRuntimeLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let Some(model_id) = Self::parse_local_model_id(&req.model_id)? else {
            // Non-UUIDv7 id: delegate to the wrapped client, which emits its own
            // Flight Recorder event (the default fallback is a recorder-wired
            // DisabledLlmClient). Do NOT emit here — that would double-count.
            return self.fallback.completion(req).await;
        };

        // Spec §4.2.3.2(3): EVERY call on the local path emits a Flight Recorder
        // event. The success path emits `llm_inference` inside
        // `run_local_completion`; every error branch (runtime resolve, capsule
        // injection, stream error, budget-exceeded) emits a zeroed-usage
        // `llm_inference` error event here, at CALL TIME (never at construction).
        match self.run_local_completion(&req, model_id).await {
            Ok(response) => Ok(response),
            Err(err) => {
                emit_llm_call_error_event(
                    &self.flight_recorder,
                    req.trace_id,
                    &req.model_id,
                    "llm_error",
                    &err.to_string(),
                )
                .await;
                Err(err)
            }
        }
    }

    /// MT-003 (WP-1) HIGH regression guard 2: wire `LlmClient::embedding()` to
    /// the embedded `ModelRuntime::embed()` so semantic embeddings are NOT
    /// silently dropped when the Ollama adapter (which implemented a real
    /// `embedding()`) is removed as the default. Without this, the bridge would
    /// inherit the trait default `embedding() -> EmbeddingUnsupported`, silently
    /// degrading LoomSearchV2 (WP-KERNEL-009 MT-264) to keyword/trigram.
    ///
    /// Routing mirrors `completion()`: a UUIDv7 `model_id` resolves to the local
    /// ModelRuntime; any non-UUIDv7 id falls back to the wrapped client so an
    /// external embedding provider is still reachable. A genuinely empty vector
    /// is rejected (never fabricated) so callers see a typed error rather than a
    /// zero-length embedding.
    async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        let Some(model_id) = Self::parse_local_model_id(&req.model_id)? else {
            // Non-UUIDv7 id: delegate to the wrapped client (owns its emission).
            return self.fallback.embedding(req).await;
        };

        // WP-1 MT-013: the local embedding lane emits a Flight Recorder event on
        // BOTH success and error (see `emit_embedding_computed_event` for the
        // product-extension caveat). Emitted at CALL TIME.
        match self.run_local_embedding(&req, model_id).await {
            Ok(response) => {
                self.emit_embedding_computed_event(&req, response.dim(), response.latency_ms)
                    .await;
                Ok(response)
            }
            Err(err) => {
                emit_llm_call_error_event(
                    &self.flight_recorder,
                    req.trace_id,
                    &req.model_id,
                    "embedding_error",
                    &err.to_string(),
                )
                .await;
                Err(err)
            }
        }
    }

    async fn score(
        &self,
        model_id: ModelId,
        sequence: Vec<u32>,
    ) -> Result<crate::model_runtime::Score, LlmError> {
        let runtime = self.router.resolve(model_id)?;
        runtime
            .score(model_id, sequence)
            .await
            .map_err(Self::map_runtime_error)
    }

    async fn control_model_runtime(
        &self,
        req: ModelRuntimeControlRequest,
    ) -> Result<ModelRuntimeControlReceipt, LlmError> {
        let control_started = Instant::now();
        if req.schema_version != MODEL_RUNTIME_CONTROL_SCHEMA_VERSION {
            return Err(LlmError::ProviderError(format!(
                "unsupported model runtime control schema {}; expected {}",
                req.schema_version, MODEL_RUNTIME_CONTROL_SCHEMA_VERSION
            )));
        }
        if req.timeout_ms == 0 {
            return Err(LlmError::ProviderError(
                "model runtime control timeout_ms must be nonzero".to_string(),
            ));
        }
        let total_timeout = Duration::from_millis(req.timeout_ms);
        let _control_guard = tokio::time::timeout(total_timeout, self.model_swap_serial.lock())
            .await
            .map_err(|_| {
                LlmError::ProviderError(
                    "model runtime control timeout expired waiting for mutation authority"
                        .to_string(),
                )
            })?;
        let model_id = Self::parse_local_model_id(&req.model_id)?.ok_or_else(|| {
            LlmError::ProviderError(
                "model runtime control requires a current local UUIDv7 model id".to_string(),
            )
        })?;
        if let Some(receipt) = self.cached_control_receipt(&req)? {
            return Ok(receipt);
        }
        let runtime = self.router.resolve(model_id)?;
        let runtime_adapter = runtime.adapter_name().to_string();
        match &req.action {
            ModelRuntimeControlAction::Quiesce => {
                let active_tokens = self
                    .active_requests()
                    .values()
                    .filter(|request| request.model_id == model_id)
                    .map(|request| request.cancel.clone())
                    .collect::<Vec<_>>();
                for token in active_tokens {
                    runtime.cancel(token);
                }
                let quiesce_timeout = Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "model quiescence",
                )?;
                runtime
                    .quiesce_model(model_id, quiesce_timeout)
                    .await
                    .map_err(|error| LlmError::ProviderError(error.to_string()))?;
                Ok(self.cache_control_receipt(
                    &req,
                    ModelRuntimeControlReceipt {
                        schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
                        request_id: req.request_id,
                        model_id: req.model_id.clone(),
                        result_model_id: None,
                        action: req.action.clone(),
                        runtime_adapter,
                        quiesced: true,
                        unloaded: false,
                        process_stop_committed: false,
                        registry_updated: false,
                        selection_rebound: false,
                        catalog_revision: None,
                        reconciliation_required: false,
                        reconciliation_reason: None,
                    },
                ))
            }
            ModelRuntimeControlAction::Unload => {
                let expected_revision = req.expected_catalog_revision.ok_or_else(|| {
                    LlmError::ProviderError("unload requires expected_catalog_revision".to_string())
                })?;
                let availability = self.router.runtime_availability();
                let actual_revision = availability.revision();
                if expected_revision != actual_revision {
                    return Err(LlmError::ProviderError(format!(
                        "stale catalog revision {expected_revision}; current revision is {actual_revision}"
                    )));
                }
                let registration = self.router.registration(model_id).ok_or_else(|| {
                    LlmError::ProviderError(format!("local model is not registered: {model_id}"))
                })?;
                let sibling_count = self
                    .router
                    .registrations()
                    .into_iter()
                    .filter(|candidate| {
                        candidate.model_id != model_id
                            && candidate.runtime_binding == registration.runtime_binding
                            && availability.is_available(candidate.model_id)
                    })
                    .count();
                if sibling_count != 0 {
                    return Err(LlmError::ProviderError(format!(
                        "model-scoped unload cannot take shared {} runtime ownership while {sibling_count} sibling model(s) remain loaded",
                        registration.runtime_binding.adapter_id()
                    )));
                }
                if self.selected_model_id() == req.model_id {
                    return Err(LlmError::ProviderError(
                        "selected application model must be rebound before unload".to_string(),
                    ));
                }
                let embedded_process = self
                    .embedded_processes()
                    .into_iter()
                    .find(|process| process.process_uuid() == model_id.as_uuid())
                    .ok_or_else(|| {
                        LlmError::ProviderError(
                            "embedded model has no ProcessOwnershipLedger authority".to_string(),
                        )
                    })?;
                let quiesce_timeout = Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "model quiescence",
                )?;
                if let Err(error) = runtime.quiesce_model(model_id, quiesce_timeout).await {
                    let _ = runtime.resume_model_admission(model_id);
                    return Err(LlmError::ProviderError(error.to_string()));
                }
                drop(runtime);
                let mut owned_runtime = self
                    .router
                    .take_runtime_for_unload(registration.runtime_binding)?;
                let strong_count = Arc::strong_count(&owned_runtime);
                if strong_count != 1 {
                    let _ = owned_runtime.resume_model_admission(model_id);
                    self.router.restore_runtime_after_failed_unload(
                        registration.runtime_binding,
                        owned_runtime,
                    )?;
                    return Err(LlmError::ProviderError(format!(
                        "verified unload requires unique {} runtime ownership; {strong_count} Arc owners remain",
                        registration.runtime_binding.adapter_id()
                    )));
                }
                let unload_result = Arc::get_mut(&mut owned_runtime)
                    .expect("strong_count == 1 proves unique unload runtime ownership")
                    .unload(model_id)
                    .await
                    .map_err(Self::map_runtime_error);
                if let Err(error) = unload_result {
                    let _ = owned_runtime.resume_model_admission(model_id);
                    self.router.restore_runtime_after_failed_unload(
                        registration.runtime_binding,
                        owned_runtime,
                    )?;
                    return Err(error);
                }
                drop(owned_runtime);
                let catalog_revision = availability.mark_unloaded(model_id);
                let stop_result = match Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "durable ProcessOwnershipLedger STOP",
                ) {
                    Ok(stop_timeout) => embedded_process
                        .shutdown_bounded("model-runtime-control-unload", stop_timeout)
                        .await
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                };
                let (process_stop_committed, reconciliation_reason) = match stop_result {
                    Ok(()) => (true, None),
                    Err(error) => (
                        false,
                        Some(format!(
                            "model unloaded and catalog updated, but durable STOP requires runtime/boot reconciliation: {error}"
                        )),
                    ),
                };
                self.remove_embedded_process(model_id.as_uuid());
                Ok(self.cache_control_receipt(
                    &req,
                    ModelRuntimeControlReceipt {
                        schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
                        request_id: req.request_id,
                        model_id: req.model_id.clone(),
                        result_model_id: None,
                        action: req.action.clone(),
                        runtime_adapter,
                        quiesced: true,
                        unloaded: true,
                        process_stop_committed,
                        registry_updated: true,
                        selection_rebound: false,
                        catalog_revision: Some(catalog_revision),
                        reconciliation_required: reconciliation_reason.is_some(),
                        reconciliation_reason,
                    },
                ))
            }
            ModelRuntimeControlAction::SwapCompatibleAdapter { target_adapter } => {
                let expected_catalog_revision = req.expected_catalog_revision.ok_or_else(|| {
                    LlmError::ProviderError(
                        "adapter swap requires expected_catalog_revision".to_string(),
                    )
                })?;
                let expected_selection_revision =
                    req.expected_selection_revision.ok_or_else(|| {
                        LlmError::ProviderError(
                            "adapter swap requires expected_selection_revision".to_string(),
                        )
                    })?;
                let availability = self.router.runtime_availability();
                let actual_revision = availability.revision();
                if actual_revision != expected_catalog_revision {
                    return Err(LlmError::ProviderError(format!(
                        "stale catalog revision {expected_catalog_revision}; current revision is {actual_revision}"
                    )));
                }
                let source = self.router.registration(model_id).ok_or_else(|| {
                    LlmError::ProviderError(format!("local model is not registered: {model_id}"))
                })?;
                let source_process = self
                    .embedded_processes()
                    .into_iter()
                    .find(|process| process.process_uuid() == model_id.as_uuid())
                    .ok_or_else(|| {
                        LlmError::ProviderError(
                            "adapter swap source has no ProcessOwnershipLedger authority"
                                .to_string(),
                        )
                    })?;
                let target_binding = match target_adapter.trim() {
                    "llama_cpp" => RuntimeBinding::LlamaCpp,
                    "candle" => RuntimeBinding::Candle,
                    other => {
                        return Err(LlmError::ProviderError(format!(
                            "unsupported compatible target adapter: {other}"
                        )))
                    }
                };
                if target_binding == source.runtime_binding {
                    return Err(LlmError::ProviderError(format!(
                        "target adapter {} is already active for {model_id}",
                        target_binding.adapter_id()
                    )));
                }
                let durable_store = self.durable_selection_store.as_ref().ok_or_else(|| {
                    LlmError::ProviderError(
                        "adapter swap has no PostgreSQL model-selection authority".to_string(),
                    )
                })?;
                let rebind_request = ExplicitModelRuntimeRebind::new(
                    KernelActor::Operator("model-runtime-control".to_string()),
                    format!(
                        "verified adapter swap request {} from {} to {}",
                        req.request_id,
                        source.runtime_binding.adapter_id(),
                        target_binding.adapter_id()
                    ),
                    expected_selection_revision,
                )
                .map_err(|error| LlmError::ProviderError(error.to_string()))?;
                let source_role = self
                    .catalog
                    .as_ref()
                    .and_then(|catalog| catalog.entry(&req.model_id))
                    .map(|entry| entry.runtime_role)
                    .ok_or_else(|| {
                        LlmError::ProviderError(
                            "adapter swap requires the shared live catalog role authority"
                                .to_string(),
                        )
                    })?;
                let source_siblings = self
                    .router
                    .registrations()
                    .into_iter()
                    .filter(|candidate| {
                        candidate.model_id != model_id
                            && candidate.runtime_binding == source.runtime_binding
                            && availability.is_available(candidate.model_id)
                    })
                    .count();
                if source_siblings != 0 {
                    return Err(LlmError::ProviderError(format!(
                        "adapter swap cannot take shared {} runtime ownership while {source_siblings} sibling model(s) remain loaded",
                        source.runtime_binding.adapter_id()
                    )));
                }
                let ledger = self.runtime_control_ledger.as_ref().ok_or_else(|| {
                    LlmError::ProviderError(
                        "adapter swap has no ProcessOwnershipLedger reservation authority"
                            .to_string(),
                    )
                })?;
                let runtime_instance = self.runtime_instance.as_ref().ok_or_else(|| {
                    LlmError::ProviderError(
                        "adapter swap has no embedded runtime liveness identity".to_string(),
                    )
                })?;
                let reservation = ledger
                    .try_reserve_lifecycles(1)
                    .map_err(|error| {
                        LlmError::ProviderError(format!(
                            "adapter swap could not reserve START/STOP authority before artifact access: {error}"
                        ))
                    })?
                    .pop()
                    .ok_or_else(|| {
                        LlmError::ProviderError(
                            "adapter swap lifecycle reservation was empty".to_string(),
                        )
                    })?;

                let load_spec = LoadSpec {
                    artifact_path: source.artifact_path.clone(),
                    sha256_expected: hex::encode(source.sha256),
                    runtime_kind: target_binding.runtime_kind(),
                    sampling_defaults: SamplingParams::default(),
                    kv_cache_policy: KvCachePolicy::default(),
                    declared_capabilities: source.declared_capabilities.clone(),
                    provider: ProviderKind::Local,
                    engine_origin: Some(target_binding.adapter_id().to_string()),
                    external_engine_import: None,
                };
                let mut target_runtime = self.router.take_runtime_for_unload(target_binding)?;
                let target_strong_count = Arc::strong_count(&target_runtime);
                if target_strong_count != 1 {
                    self.router
                        .restore_runtime_after_failed_unload(target_binding, target_runtime)?;
                    return Err(LlmError::ProviderError(format!(
                        "adapter swap requires unique {} target runtime ownership; {target_strong_count} Arc owners remain",
                        target_binding.adapter_id()
                    )));
                }
                let target_runtime_mut = Arc::get_mut(&mut target_runtime)
                    .expect("strong_count == 1 proves unique target runtime ownership");
                let replacement_id = match target_runtime_mut.load(load_spec).await {
                    Ok(id) => id,
                    Err(error) => {
                        self.router
                            .restore_runtime_after_failed_unload(target_binding, target_runtime)?;
                        return Err(Self::map_runtime_error(error));
                    }
                };
                let artifact_integrity = match target_runtime_mut.artifact_integrity(replacement_id)
                {
                    Ok(receipt) => receipt,
                    Err(error) => {
                        let _ = target_runtime_mut.unload(replacement_id).await;
                        self.router
                            .restore_runtime_after_failed_unload(target_binding, target_runtime)?;
                        return Err(LlmError::ProviderError(format!(
                            "target adapter loaded but exact artifact attestation failed: {error}"
                        )));
                    }
                };
                let (replacement_process, start_ack) =
                    match EmbeddedModelProcess::record_reserved_load_with_durable_ack(
                        reservation,
                        target_binding,
                        replacement_id,
                        source.base_model_tag.as_str(),
                        &artifact_integrity,
                        Some(runtime_instance),
                    ) {
                        Ok(started) => started,
                        Err(error) => {
                            let unload = target_runtime_mut.unload(replacement_id).await;
                            self.router.restore_runtime_after_failed_unload(
                                target_binding,
                                target_runtime,
                            )?;
                            return Err(LlmError::ProviderError(format!(
                                "target adapter loaded but reserved START transition failed: {error}; rollback unload: {unload:?}"
                            )));
                        }
                    };
                let start_timeout = match Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "target START acknowledgement",
                ) {
                    Ok(timeout) => timeout,
                    Err(error) => {
                        let unloaded = target_runtime_mut.unload(replacement_id).await.is_ok();
                        self.router
                            .restore_runtime_after_failed_unload(target_binding, target_runtime)?;
                        if unloaded {
                            let _ = replacement_process
                                .shutdown_bounded(
                                    "adapter-swap-start-timeout",
                                    Duration::from_millis(1),
                                )
                                .await;
                        } else {
                            replacement_process.leave_open_for_reconciliation();
                        }
                        return Err(error);
                    }
                };
                if let Err(error) = start_ack.wait(start_timeout).await {
                    let unloaded = target_runtime_mut.unload(replacement_id).await.is_ok();
                    self.router
                        .restore_runtime_after_failed_unload(target_binding, target_runtime)?;
                    if unloaded {
                        let _ = replacement_process
                            .shutdown_bounded("adapter-swap-start-ack-failed", start_timeout)
                            .await;
                    } else {
                        replacement_process.leave_open_for_reconciliation();
                    }
                    return Err(LlmError::ProviderError(format!(
                        "target ProcessOwnershipLedger START was not durably acknowledged: {error}"
                    )));
                }
                self.router
                    .restore_runtime_after_failed_unload(target_binding, target_runtime)?;

                for token in self
                    .active_requests()
                    .values()
                    .filter(|request| request.model_id == model_id)
                    .map(|request| request.cancel.clone())
                    .collect::<Vec<_>>()
                {
                    runtime.cancel(token);
                }
                let source_quiesce_timeout = match Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "source model quiescence",
                ) {
                    Ok(timeout) => timeout,
                    Err(error) => {
                        let rollback_timeout = Self::remaining_control_timeout(
                            control_started,
                            total_timeout,
                            "replacement rollback STOP",
                        )
                        .unwrap_or(Duration::from_millis(1));
                        let rollback = self
                            .rollback_control_replacement(
                                target_binding,
                                replacement_id,
                                &replacement_process,
                                rollback_timeout,
                                "adapter-swap-source-quiesce-timeout",
                            )
                            .await;
                        let _ = runtime.resume_model_admission(model_id);
                        return Err(LlmError::ProviderError(format!(
                            "{error}; replacement rollback: {rollback:?}"
                        )));
                    }
                };
                if let Err(error) = runtime
                    .quiesce_model(model_id, source_quiesce_timeout)
                    .await
                {
                    let rollback_timeout = Self::remaining_control_timeout(
                        control_started,
                        total_timeout,
                        "replacement rollback STOP",
                    )
                    .unwrap_or(Duration::from_millis(1));
                    let rollback = self
                        .rollback_control_replacement(
                            target_binding,
                            replacement_id,
                            &replacement_process,
                            rollback_timeout,
                            "adapter-swap-source-quiesce-failed",
                        )
                        .await;
                    let _ = runtime.resume_model_admission(model_id);
                    return Err(LlmError::ProviderError(format!(
                        "source quiescence failed: {error}; replacement rollback: {rollback:?}"
                    )));
                }
                drop(runtime);
                let mut source_runtime = self
                    .router
                    .take_runtime_for_unload(source.runtime_binding)?;
                let source_strong_count = Arc::strong_count(&source_runtime);
                if source_strong_count != 1 {
                    let _ = source_runtime.resume_model_admission(model_id);
                    self.router.restore_runtime_after_failed_unload(
                        source.runtime_binding,
                        source_runtime,
                    )?;
                    let rollback_timeout = Self::remaining_control_timeout(
                        control_started,
                        total_timeout,
                        "replacement rollback STOP",
                    )
                    .unwrap_or(Duration::from_millis(1));
                    let rollback = self
                        .rollback_control_replacement(
                            target_binding,
                            replacement_id,
                            &replacement_process,
                            rollback_timeout,
                            "adapter-swap-source-ownership-failed",
                        )
                        .await;
                    return Err(LlmError::ProviderError(format!(
                        "verified source unload requires unique {} runtime ownership; {source_strong_count} Arc owners remain; replacement rollback: {rollback:?}",
                        source.runtime_binding.adapter_id(),
                    )));
                }
                let source_unload = Arc::get_mut(&mut source_runtime)
                    .expect("strong_count == 1 proves unique source runtime ownership")
                    .unload(model_id)
                    .await
                    .map_err(Self::map_runtime_error);
                if let Err(error) = source_unload {
                    let _ = source_runtime.resume_model_admission(model_id);
                    self.router.restore_runtime_after_failed_unload(
                        source.runtime_binding,
                        source_runtime,
                    )?;
                    let rollback_timeout = Self::remaining_control_timeout(
                        control_started,
                        total_timeout,
                        "replacement rollback STOP",
                    )
                    .unwrap_or(Duration::from_millis(1));
                    let rollback = self
                        .rollback_control_replacement(
                            target_binding,
                            replacement_id,
                            &replacement_process,
                            rollback_timeout,
                            "adapter-swap-source-unload-failed",
                        )
                        .await;
                    return Err(LlmError::ProviderError(format!(
                        "source unload failed: {error}; replacement rollback: {rollback:?}"
                    )));
                }
                self.router
                    .restore_runtime_after_failed_unload(source.runtime_binding, source_runtime)?;

                let replacement_registration = ModelRegistration {
                    model_id: replacement_id,
                    artifact_path: source.artifact_path.clone(),
                    sha256: source.sha256,
                    runtime_binding: target_binding,
                    declared_capabilities: source.declared_capabilities.clone(),
                    base_model_tag: source.base_model_tag.clone(),
                    registered_at_utc: source.registered_at_utc,
                    registered_by: source.registered_by.clone(),
                    provider: ProviderKind::Local,
                };
                let rebind_result = durable_store
                    .rebind_selection_after_verified_unload(
                        &ModelRuntimeSelection {
                            artifact_sha256: source.sha256,
                            runtime_binding: target_binding,
                            runtime_role: source_role,
                            declared_capabilities: source.declared_capabilities.clone(),
                            provider: ProviderKind::Local,
                        },
                        rebind_request,
                    )
                    .await;
                if let Err(error) = rebind_result {
                    availability.mark_unloaded(model_id);
                    let rollback_timeout = Self::remaining_control_timeout(
                        control_started,
                        total_timeout,
                        "replacement rollback STOP",
                    )
                    .unwrap_or(Duration::from_millis(1));
                    let rollback = self
                        .rollback_control_replacement(
                            target_binding,
                            replacement_id,
                            &replacement_process,
                            rollback_timeout,
                            "adapter-swap-durable-rebind-failed",
                        )
                        .await;
                    let source_stop = match Self::remaining_control_timeout(
                        control_started,
                        total_timeout,
                        "source reconciliation STOP",
                    ) {
                        Ok(timeout) => {
                            source_process
                                .shutdown_bounded("adapter-swap-durable-rebind-failed", timeout)
                                .await
                        }
                        Err(stop_timeout_error) => {
                            Err(crate::process_ledger::ProcessLedgerError::InvalidConfig(
                                format!("source STOP not attempted: {stop_timeout_error}"),
                            ))
                        }
                    };
                    self.remove_embedded_process(source_process.process_uuid());
                    return Err(LlmError::ProviderError(format!(
                        "source unloaded but durable adapter rebind failed: {error}; replacement rollback: {rollback:?}; source STOP: {source_stop:?}"
                    )));
                }
                let catalog_revision = availability.publish_replacement(
                    model_id,
                    replacement_registration,
                    source_role,
                );
                if self.selected_model_id() == req.model_id {
                    *self
                        .selected_model_id
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        replacement_id.to_string();
                }
                self.embedded_processes
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(Arc::new(replacement_process));
                let stop_result = match Self::remaining_control_timeout(
                    control_started,
                    total_timeout,
                    "source durable ProcessOwnershipLedger STOP",
                ) {
                    Ok(stop_timeout) => source_process
                        .shutdown_bounded("model-runtime-control-adapter-swap", stop_timeout)
                        .await
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                };
                let (process_stop_committed, reconciliation_reason) = match stop_result {
                    Ok(()) => (true, None),
                    Err(error) => (
                        false,
                        Some(format!(
                            "source unloaded, selection rebound, and replacement published, but durable source STOP requires runtime/boot reconciliation: {error}"
                        )),
                    ),
                };
                self.remove_embedded_process(source_process.process_uuid());
                Ok(self.cache_control_receipt(
                    &req,
                    ModelRuntimeControlReceipt {
                        schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
                        request_id: req.request_id,
                        model_id: req.model_id.clone(),
                        result_model_id: Some(replacement_id.to_string()),
                        action: req.action.clone(),
                        runtime_adapter: target_binding.adapter_id().to_string(),
                        quiesced: true,
                        unloaded: true,
                        process_stop_committed,
                        registry_updated: true,
                        selection_rebound: true,
                        catalog_revision: Some(catalog_revision),
                        reconciliation_required: reconciliation_reason.is_some(),
                        reconciliation_reason,
                    },
                ))
            }
        }
    }

    fn model_runtime_control_capabilities(
        &self,
        model_id: &str,
    ) -> ModelRuntimeControlCapabilities {
        let parsed_model_id = Self::parse_local_model_id(model_id).ok().flatten();
        let has_lifecycle = parsed_model_id.is_some_and(|model_id| {
            self.embedded_processes()
                .iter()
                .any(|process| process.process_uuid() == model_id.as_uuid())
        });
        ModelRuntimeControlCapabilities {
            quiesce: parsed_model_id.is_some(),
            unload: has_lifecycle,
            swap_compatible_adapter: has_lifecycle
                && self.runtime_control_ledger.is_some()
                && self.runtime_instance.is_some()
                && self.durable_selection_store.is_some(),
        }
    }

    async fn swap_model(
        &self,
        req: crate::workflows::ModelSwapRequestV0_4,
    ) -> Result<(), LlmError> {
        req.validate().map_err(|error| {
            LlmError::ProviderError(format!("CX-MM-003: invalid model swap request: {error}"))
        })?;
        let _swap_guard = self.model_swap_serial.lock().await;
        let current_model_id = self.selected_model_id();
        if req.current_model_id != current_model_id {
            return Err(LlmError::ProviderError(format!(
                "CX-MM-003: stale model swap current_model_id {}; active selection is {}",
                req.current_model_id, current_model_id
            )));
        }
        let target_model_id =
            Self::parse_local_model_id(&req.target_model_id)?.ok_or_else(|| {
                LlmError::ProviderError(
                    "CX-MM-003: target model must be a current local UUIDv7".to_owned(),
                )
            })?;
        let catalog = self.catalog.as_ref().ok_or_else(|| {
            LlmError::ProviderError(
                "CX-MM-003: local model catalog is unavailable for deterministic selection"
                    .to_owned(),
            )
        })?;
        let target = catalog.entry(&req.target_model_id).ok_or_else(|| {
            LlmError::ProviderError(format!(
                "CX-MM-001: target model {} is not in the current catalog",
                req.target_model_id
            ))
        })?;
        if !target.ready {
            return Err(LlmError::ProviderError(format!(
                "CX-MM-001: target model {} is not READY",
                req.target_model_id
            )));
        }
        if !target.default_selectable {
            return Err(LlmError::ProviderError(format!(
                "CX-MM-001: target model {} has runtime role {:?} and is not eligible as the default completion model",
                req.target_model_id, target.runtime_role
            )));
        }
        self.router.resolve(target_model_id)?;

        let selection_actor = req
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("actor"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|actor| {
                !actor.is_empty() && actor.len() <= 128 && !actor.chars().any(char::is_control)
            })
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{:?}", req.requester.subsystem).to_ascii_lowercase());
        let store = self.durable_selection_store.as_ref().ok_or_else(|| {
            LlmError::ProviderError(
                "CX-MM-003: PostgreSQL active-selection authority is unavailable".to_owned(),
            )
        })?;
        let expected_revision = self
            .active_application_selection_revision
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .ok_or_else(|| {
                LlmError::ProviderError(
                    "CX-MM-003: application/default revision is unavailable".to_owned(),
                )
            })?;
        let target_bytes = hex::decode(&target.artifact_sha256).map_err(|error| {
            LlmError::ProviderError(format!(
                "CX-MM-003: target stable artifact anchor is invalid: {error}"
            ))
        })?;
        let target_sha256: [u8; 32] = target_bytes.try_into().map_err(|bytes: Vec<u8>| {
            LlmError::ProviderError(format!(
                "CX-MM-003: target stable artifact anchor is {} bytes, expected 32",
                bytes.len()
            ))
        })?;
        let committed = store
            .select_active_model(
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                target_sha256,
                expected_revision,
                KernelActor::Operator(selection_actor),
                &req.reason,
            )
            .await
            .map_err(|error| {
                LlmError::ProviderError(format!(
                    "CX-MM-003: durable active selection failed: {error}"
                ))
            })?;

        if let Some(current) = Self::parse_local_model_id(&current_model_id)? {
            let active_tokens = self
                .active_requests()
                .values()
                .filter(|request| request.model_id == current)
                .map(|request| request.cancel.clone())
                .collect::<Vec<_>>();
            if let Ok(runtime) = self.router.resolve(current) {
                for token in active_tokens {
                    runtime.cancel(token);
                }
            }
        }
        *self
            .selected_model_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = req.target_model_id;
        *self
            .active_application_selection_revision
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(committed.selection_revision);
        Ok(())
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

        let active_tokens = self
            .active_requests()
            .values()
            .filter(|request| request.model_id == model_id)
            .map(|request| request.cancel.clone())
            .collect::<Vec<_>>();
        if let Ok(runtime) = self.router.resolve(model_id) {
            for active_token in active_tokens {
                runtime.cancel(active_token);
            }
            runtime.cancel(token);
        } else {
            for active_token in active_tokens {
                active_token.cancel();
            }
            token.cancel();
        }
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    fn selected_model_id(&self) -> String {
        self.selected_model_id
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// MT-014: expose the shared, enumerable, labeled model catalog so a surface
    /// reachable from `AppState.llm_client` can list/label the configured local
    /// model(s) and resolve the stable cross-session anchor. `None` when this
    /// client was constructed without a catalog.
    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        self.catalog.clone()
    }

    fn inspect_model_runtime(&self, model_id: &str) -> ModelRuntimeInspection {
        let Some(model_id) = Self::parse_local_model_id(model_id).ok().flatten() else {
            return ModelRuntimeInspection::unavailable(
                "model id is not a current local UUIDv7 routing identity",
            );
        };
        let runtime = match self.router.resolve(model_id) {
            Ok(runtime) => runtime,
            Err(error) => return ModelRuntimeInspection::unavailable(error.to_string()),
        };
        let kv_cache = match runtime.kv_cache(model_id) {
            Ok(handle) => {
                let stats = handle.occupancy();
                let attempts = stats
                    .prefix_cache_hit_count
                    .saturating_add(stats.prefix_cache_miss_count);
                let prefix_cache_hit_rate = if attempts == 0 {
                    ModelRuntimeValue::unavailable(
                        "no prefix-cache lookup has completed for this loaded model",
                    )
                } else {
                    ModelRuntimeValue::available(
                        stats.prefix_cache_hit_count as f64 / attempts as f64,
                    )
                };
                let quantization = serde_json::to_value(stats.quant_level_current)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{:?}", stats.quant_level_current));
                ModelRuntimeValue::available(ModelRuntimeKvInspection {
                    bytes_used: stats.bytes_used,
                    bytes_capacity: stats.bytes_capacity,
                    prefix_cache_hit_rate,
                    quantization,
                })
            }
            Err(error) => ModelRuntimeValue::unavailable(error.to_string()),
        };
        let lora_stack = match runtime.lora_stack(model_id) {
            Ok(handle) => ModelRuntimeValue::available(
                handle
                    .list_active()
                    .into_iter()
                    .map(|entry| ModelRuntimeLoraInspection {
                        lora_id: entry.id.to_string(),
                        strength: entry.strength.value(),
                    })
                    .collect(),
            ),
            Err(error) => ModelRuntimeValue::unavailable(error.to_string()),
        };
        // Section 10.13.1 "Steering vectors active": the runtime steering handle
        // now exposes the applied (not merely registered) vector set. An adapter
        // that does not host steering fails typed and surfaces that reason.
        let active_steering = match runtime.steering_hooks(model_id) {
            Ok(handle) => ModelRuntimeValue::available(
                handle
                    .list_active()
                    .into_iter()
                    .map(|meta| ModelRuntimeSteeringInspection {
                        steering_vector_id: meta.id.to_string(),
                        layer: meta.layer.as_u32(),
                        intensity: meta.intensity,
                    })
                    .collect(),
            ),
            Err(error) => ModelRuntimeValue::unavailable(error.to_string()),
        };
        // Section 10.13.1 live perf stats: tokens/sec, VRAM residency, and
        // time-since-last-call are derived from the runtime's real recorded
        // generation activity. Each sub-field is honestly typed unavailable when
        // no call has completed or the device exposes no residency, never a fake
        // zero. A runtime that records no activity fails the whole snapshot typed.
        let (tokens_per_second, vram_resident_bytes, last_call_at_utc) =
            match runtime.perf_snapshot(model_id) {
                Ok(snapshot) => {
                    let tokens_per_second = match snapshot.tokens_per_second {
                        Some(value) => ModelRuntimeValue::available(value),
                        None => ModelRuntimeValue::unavailable(
                            "no completed generation has produced a throughput sample for this loaded model yet",
                        ),
                    };
                    let vram_resident_bytes = match snapshot.vram_resident_bytes {
                        RuntimeVramResidency::DeviceReported { bytes } => {
                            ModelRuntimeValue::available(bytes)
                        }
                        RuntimeVramResidency::NotApplicable { reason } => {
                            ModelRuntimeValue::unavailable(reason)
                        }
                    };
                    let last_call_at_utc = match snapshot.last_call_at_utc {
                        Some(completed_at) => {
                            ModelRuntimeValue::available(completed_at.to_rfc3339())
                        }
                        None => ModelRuntimeValue::unavailable(
                            "no generation call has completed for this loaded model yet",
                        ),
                    };
                    (tokens_per_second, vram_resident_bytes, last_call_at_utc)
                }
                Err(error) => {
                    let reason = error.to_string();
                    (
                        ModelRuntimeValue::unavailable(reason.clone()),
                        ModelRuntimeValue::unavailable(reason.clone()),
                        ModelRuntimeValue::unavailable(reason),
                    )
                }
            };
        // Section 10.13.2 "Inspect engine internals": adapter-specific drilldown
        // of the real engine-known configuration. Typed unavailable when the
        // active adapter exposes no internals.
        let engine_internals = match runtime.engine_internals(model_id) {
            Ok(internals) => ModelRuntimeValue::available(internals),
            Err(error) => ModelRuntimeValue::unavailable(error.to_string()),
        };
        ModelRuntimeInspection {
            kv_cache,
            lora_stack,
            active_steering,
            tokens_per_second,
            vram_resident_bytes,
            last_call_at_utc,
            engine_internals,
        }
    }

    /// Immediate cancellation seam. This deliberately does not emit an
    /// embedded-model STOP: cancellation request delivery alone does not prove
    /// detached generation or blocking inference workers have terminated.
    fn shutdown(&self) {
        self.cancel_all_active_requests();
    }

    fn leave_open_for_reconciliation(&self) {
        self.cancel_all_active_requests();
        self.leave_embedded_lifecycles_open();
    }

    async fn shutdown_gracefully(&self) -> Result<(), LlmError> {
        let _shutdown_guard = self.graceful_shutdown_serial.lock().await;
        if self.graceful_shutdown_complete.load(Ordering::Acquire) {
            return Ok(());
        }
        self.cancel_all_active_requests();
        let embedded_processes = self.embedded_processes();
        if embedded_processes.is_empty() {
            self.graceful_shutdown_complete
                .store(true, Ordering::Release);
            return Ok(());
        }
        if let Err(error) = self.quiesce_embedded_runtimes().await {
            self.leave_embedded_lifecycles_open();
            return Err(error);
        }
        if let Err(error) = self.unload_embedded_models().await {
            self.leave_embedded_lifecycles_open();
            return Err(error);
        }
        for embedded_process in &embedded_processes {
            if let Err(error) = embedded_process.shutdown("llm-client-shutdown") {
                self.leave_embedded_lifecycles_open();
                return Err(LlmError::ProviderError(format!(
                    "embedded model ProcessOwnershipLedger reserved STOP emission failed for {}: {error}",
                    embedded_process.process_uuid()
                )));
            }
        }
        self.graceful_shutdown_complete
            .store(true, Ordering::Release);
        Ok(())
    }
}

#[cfg(test)]
mod runtime_control_receipt_tests {
    use super::*;

    fn request(request_id: Uuid) -> ModelRuntimeControlRequest {
        ModelRuntimeControlRequest {
            schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
            request_id,
            model_id: ModelId::new_v7().to_string(),
            action: ModelRuntimeControlAction::Unload,
            timeout_ms: 5_000,
            expected_catalog_revision: Some(7),
            expected_selection_revision: None,
        }
    }

    fn receipt(req: &ModelRuntimeControlRequest) -> ModelRuntimeControlReceipt {
        ModelRuntimeControlReceipt {
            schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
            request_id: req.request_id,
            model_id: req.model_id.clone(),
            result_model_id: None,
            action: req.action.clone(),
            runtime_adapter: "candle".to_string(),
            quiesced: true,
            unloaded: true,
            process_stop_committed: false,
            registry_updated: true,
            selection_rebound: false,
            catalog_revision: Some(8),
            reconciliation_required: true,
            reconciliation_reason: Some(
                "durable STOP requires runtime/boot reconciliation".to_string(),
            ),
        }
    }

    #[test]
    fn cached_control_request_id_rejects_changed_cas_or_timeout_envelope() {
        let original = request(Uuid::now_v7());
        let cached = CachedRuntimeControlReceipt {
            request: original.clone(),
            receipt: receipt(&original),
        };

        validate_cached_control_request(&cached, &original)
            .expect("identical retry returns the original truthful receipt");

        let mut changed_catalog_cas = original.clone();
        changed_catalog_cas.expected_catalog_revision = Some(8);
        let error = validate_cached_control_request(&cached, &changed_catalog_cas)
            .expect_err("same request_id cannot replay across a changed catalog CAS");
        assert!(error.to_string().contains("immutable request envelope"));

        let mut changed_timeout = original;
        changed_timeout.timeout_ms = 10_000;
        validate_cached_control_request(&cached, &changed_timeout)
            .expect_err("same request_id cannot replay across a changed timeout");
    }

    #[test]
    fn partial_stop_receipt_is_truthful_and_reconciliation_actionable() {
        let req = request(Uuid::now_v7());
        let receipt = receipt(&req);
        assert!(receipt.quiesced);
        assert!(receipt.unloaded);
        assert!(receipt.registry_updated);
        assert!(!receipt.process_stop_committed);
        assert!(receipt.reconciliation_required);
        assert!(receipt
            .reconciliation_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("runtime/boot reconciliation")));
    }
}
