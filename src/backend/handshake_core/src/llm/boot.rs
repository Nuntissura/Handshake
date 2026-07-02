//! Default `LlmClient` boot resolution (MT-003, WP-1 "Ollama-kill").
//!
//! This module owns the provider-resolution logic that `main.rs::init_llm_client`
//! used to inline. It is factored out of the binary so it is unit-testable from
//! the integration test crate (the `[[bin]]` cannot be linked by `tests/`).
//!
//! Authority contract (master-spec-v02.197 §3.6.1 / §4.2.3):
//! - The default provider resolves LOCAL inference through the embedded
//!   `ModelRuntime` (Candle CPU baseline by default, llama.cpp opt-in), NOT an
//!   auto-detected third-party daemon. There is no `/api/tags` probe and no
//!   daemon fallback anywhere in this path.
//! - When no local model is configured, or the embedded load fails, the boot
//!   path fails CLOSED to [`DisabledLlmClient`] (it never selects a daemon
//!   adapter as authority and never hard-aborts startup).
//! - The generic OpenAI-compatible lane is retained as a NON-authoritative
//!   external_compat compat lane only, still mediated by [`LlmClient`].
//!
//! Engine note: this module names the adapter wrapper types `CandleRuntime` /
//! `LlamaCppRuntime` (both `ModelRuntime`), never engine-internal crate types.
//! It does NOT re-implement engines (WP-KERNEL-004 owns those); it only WIRES
//! the already-shipped runtimes into the default `LlmClient`.

use std::sync::Arc;

use chrono::Utc;

use crate::flight_recorder::FlightRecorder;
use crate::model_runtime::{
    candle::CandleRuntime, llama_cpp::LlamaCppRuntime, BaseModelTag, KvCachePolicy,
    LoadSpec, ModelCapabilities, ModelCatalog, ModelRegistration, ModelRegistry, ModelRuntime,
    OperatorId, ProviderKind as RuntimeProviderKind, RuntimeBinding, SamplingParams,
};
use crate::process_ledger::LedgerBatcher;

use super::embedded_ledger::EmbeddedModelProcess;
use super::guard::CloudEscalationGuard;
use super::local_router::{LocalModelRuntimeLlmClient, LocalRouter};
use super::openai_compat::{ApiKey, OpenAiCompatAdapter};
use super::registry::{ProviderKind, ProviderRegistry, ResolvedProvider, RuntimeRole};
use super::{DisabledLlmClient, LlmClient, LlmError, ModelProfile, ModelTier};

/// Context window used for the embedded local model's [`ModelProfile`]. This is
/// the same bound the removed Ollama default used, kept for parity.
const DEFAULT_LOCAL_MAX_CONTEXT_TOKENS: u32 = 8192;
const DEFAULT_OPENAI_COMPAT_MAX_CONTEXT_TOKENS: u32 = 8192;

/// Resolves the default `LlmClient` from environment configuration.
///
/// This is the single entry point `main.rs::init_llm_client` delegates to. It
/// reads [`ProviderRegistry::from_env`], resolves the Orchestrator role, then
/// dispatches to the embedded local-runtime lane or the retained external_compat
/// compat lane. Cloud-tier clients are wrapped in [`CloudEscalationGuard`].
pub async fn resolve_default_llm_client(
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
) -> Arc<dyn LlmClient> {
    let registry = match ProviderRegistry::from_env() {
        Ok(registry) => registry,
        Err(err) => {
            let reason = format!("LLM registry init failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (cannot load ProviderRegistry)"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                "unknown".to_string(),
                reason,
                Arc::clone(&flight_recorder),
            ));
        }
    };

    let resolved = match registry.resolve(RuntimeRole::Orchestrator) {
        Ok(resolved) => resolved,
        Err(err) => {
            let reason = format!("LLM provider resolution failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (cannot resolve provider)"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                "unknown".to_string(),
                reason,
                Arc::clone(&flight_recorder),
            ));
        }
    };

    let client: Arc<dyn LlmClient> = match resolved.kind {
        ProviderKind::LocalRuntime => {
            // Only the embedded local-runtime load path owns an in-process model,
            // so the ProcessOwnershipLedger handle is threaded here (WP-1 MT-013).
            build_default_local_client(&resolved, Arc::clone(&flight_recorder), ledger).await
        }
        ProviderKind::OpenAiCompat => {
            build_openai_compat_client(&resolved, Arc::clone(&flight_recorder))
        }
    };

    if client.profile().model_tier == ModelTier::Cloud {
        match CloudEscalationGuard::from_env(client) {
            Ok(guarded) => Arc::new(guarded),
            Err(err) => {
                let reason = format!("CloudEscalationGuard init failed: {err}");
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %reason,
                    "LLM disabled (cloud guard init failed)"
                );
                Arc::new(DisabledLlmClient::new_recorded(
                    "unknown".to_string(),
                    reason,
                    Arc::clone(&flight_recorder),
                ))
            }
        }
    } else {
        client
    }
}

/// Builds the embedded local-runtime `LlmClient`, or fails CLOSED to
/// [`DisabledLlmClient`].
///
/// Fails closed (no daemon fallback) when:
/// - no local model is configured (`resolved.local_model` is `None`), OR
/// - the embedded `ModelRuntime::load` fails, OR
/// - registration into the fresh [`ModelRegistry`] fails.
///
/// On success it mints a UUIDv7 `ModelId` via `load()` (load-then-freeze:
/// `LlamaCppRuntime::load`/`CandleRuntime::load` take `&mut self`, so we load
/// BEFORE `Arc`-wrapping), registers it (`provider=Local`, matching binding),
/// and assembles a [`LocalModelRuntimeLlmClient`] whose `profile().model_id` is
/// that minted UUIDv7 — the identity call sites forward as
/// `CompletionRequest.model_id`.
pub async fn build_default_local_client(
    resolved: &ResolvedProvider,
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
) -> Arc<dyn LlmClient> {
    let Some(local) = resolved.local_model.as_ref() else {
        let reason =
            "HSK-LOCAL-DISABLED: no local model configured (set HANDSHAKE_LOCAL_MODEL_PATH); \
             refusing third-party daemon fallback"
                .to_string();
        tracing::warn!(
            target: "handshake_core::llm",
            "LLM disabled (no embedded local model configured; no daemon fallback)"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    };

    let capabilities = default_local_capabilities(local.runtime_binding);
    let load_spec = LoadSpec {
        artifact_path: local.artifact_path.clone(),
        sha256_expected: hex::encode(local.sha256),
        runtime_kind: local.runtime_binding.runtime_kind(),
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: capabilities.clone(),
        provider: RuntimeProviderKind::Local,
        engine_origin: Some(local.runtime_binding.adapter_id().to_string()),
        external_engine_import: None,
    };

    // Build BOTH runtimes; only the configured binding is loaded. LocalRouter
    // requires both handles and resolves per-model by binding, so the unloaded
    // runtime is never dispatched to for this model.
    let mut llama_runtime = LlamaCppRuntime::new(KvCachePolicy::default());
    let mut candle_runtime = CandleRuntime::default();

    // load-then-freeze: drive `load()` on the &mut runtime BEFORE Arc-wrapping.
    let load_result = match local.runtime_binding {
        RuntimeBinding::LlamaCpp => llama_runtime.load(load_spec).await,
        RuntimeBinding::Candle => candle_runtime.load(load_spec).await,
    };
    let model_id = match load_result {
        Ok(model_id) => model_id,
        Err(err) => {
            let reason = format!("HSK-LOCAL-DISABLED: embedded ModelRuntime load failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                binding = %local.runtime_binding.adapter_id(),
                "LLM disabled (embedded model load failed; no daemon fallback)"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ));
        }
    };

    let registration = ModelRegistration {
        model_id,
        artifact_path: local.artifact_path.clone(),
        sha256: local.sha256,
        runtime_binding: local.runtime_binding,
        declared_capabilities: capabilities,
        base_model_tag: BaseModelTag::new(local.display_name.clone()),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("handshake-embedded-default"),
        provider: RuntimeProviderKind::Local,
    };

    // Fail-closed fallback: an embedded local model has no external fallback
    // provider, so a non-UUIDv7 model_id (which should never reach here for the
    // default lane) degrades to a typed DisabledLlmClient error rather than a
    // daemon.
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new_recorded(
        local.display_name.clone(),
        "HSK-LOCAL-FALLBACK: no external fallback configured for the embedded local model"
            .to_string(),
        Arc::clone(&flight_recorder),
    ));

    match assemble_local_runtime_client(
        registration,
        Arc::new(llama_runtime),
        Arc::new(candle_runtime),
        fallback,
        Arc::clone(&flight_recorder),
        DEFAULT_LOCAL_MAX_CONTEXT_TOKENS,
        ledger,
    ) {
        Ok(client) => Arc::new(client),
        Err(err) => {
            let reason = format!("HSK-LOCAL-DISABLED: local runtime registration failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (local model registration failed)"
            );
            Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ))
        }
    }
}

/// Assembles a [`LocalModelRuntimeLlmClient`] from an already-loaded
/// registration + runtimes. Pure (no I/O), so it is directly unit-testable with
/// fake `ModelRuntime`s.
///
/// The returned client's `profile().model_id` is `registration.model_id`
/// stringified — the minted UUIDv7. This is the MT-003 HIGH regression guard 1:
/// `LocalModelRuntimeLlmClient::completion` only routes to the embedded runtime
/// for UUIDv7 ids, so if the profile carried a friendly name the default lane
/// would silently fall back.
pub fn assemble_local_runtime_client(
    registration: ModelRegistration,
    llama_runtime: Arc<dyn ModelRuntime>,
    candle_runtime: Arc<dyn ModelRuntime>,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    max_context_tokens: u32,
    ledger: Option<LedgerBatcher>,
) -> Result<LocalModelRuntimeLlmClient, LlmError> {
    let model_id = registration.model_id;
    // WP-1 MT-013: capture the ProcessOwnershipLedger START-row inputs BEFORE
    // `registration` is moved into the registry.
    let runtime_binding = registration.runtime_binding;
    let display_name = registration.base_model_tag.as_str().to_string();
    let artifact_sha256 = hex::encode(registration.sha256);

    let mut registry = ModelRegistry::default();
    registry.register(registration).map_err(|err| {
        LlmError::ProviderError(format!("local model registration failed: {err}"))
    })?;
    // MT-014: the embedded model is already loaded before assemble (load then
    // freeze), so mark it loaded. The shared ModelCatalog surfaces READY state
    // (master-spec §4.3.9.4.4) from this marker; without it a genuinely-loaded
    // boot model would enumerate as not-ready.
    registry.mark_loaded(model_id).map_err(|err| {
        LlmError::ProviderError(format!("local model load-marking failed: {err}"))
    })?;

    // MT-014: share ONE `Arc<ModelRegistry>` between the router (dispatch) and
    // the catalog (enumeration/label). This makes the single boot registry
    // shared + enumerable — it does NOT create a second registry world.
    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry(Arc::clone(&registry));

    let router = LocalRouter::new(registry, llama_runtime, candle_runtime);
    let profile =
        ModelProfile::new(model_id.to_string(), max_context_tokens).with_streaming(true);

    let mut client =
        LocalModelRuntimeLlmClient::new(router, fallback, flight_recorder, profile)
            .with_catalog(catalog);

    // WP-1 MT-013: emit the ProcessOwnershipLedger START row for the just-loaded
    // in-process embedded model (master-spec §3.6.2 clause 2 / §4.6.1) and attach
    // the ownership handle so the client owns the STOP-on-shutdown obligation.
    // The row is pid-less (`os_pid = None`) because the in-process library load
    // spawns no OS process. A ledger-emit failure is logged but does NOT fail
    // client assembly — the model is already loaded and usable; the ownership
    // record is auxiliary attribution, not a load prerequisite.
    if let Some(ledger) = ledger {
        match EmbeddedModelProcess::record_load(
            ledger,
            runtime_binding,
            model_id,
            &display_name,
            Some(artifact_sha256),
        ) {
            Ok(embedded_process) => {
                client = client.with_embedded_process(embedded_process);
            }
            Err(err) => {
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %err,
                    model_id = %model_id,
                    "embedded model ProcessOwnershipLedger START emit failed; proceeding without ownership handle"
                );
            }
        }
    }

    Ok(client)
}

/// Builds the retained, NON-authoritative external_compat OpenAI-compatible lane.
pub fn build_openai_compat_client(
    resolved: &ResolvedProvider,
    flight_recorder: Arc<dyn FlightRecorder>,
) -> Arc<dyn LlmClient> {
    let api_key = resolved
        .api_key_env
        .as_deref()
        .and_then(ApiKey::from_env)
        .or_else(|| ApiKey::from_env("OPENAI_API_KEY"));

    Arc::new(OpenAiCompatAdapter::new(
        resolved.base_url.clone(),
        resolved.model_id.clone(),
        DEFAULT_OPENAI_COMPAT_MAX_CONTEXT_TOKENS,
        resolved.tier,
        api_key,
        flight_recorder,
    ))
}

/// Default declared capabilities for an embedded local model. `validate_binding`
/// forbids `LlamaCpp + activation_steering`, so steering is only declared for
/// the Candle binding (which owns the hook/steering lanes).
fn default_local_capabilities(binding: RuntimeBinding) -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_activation_steering: binding == RuntimeBinding::Candle,
        ..Default::default()
    }
}
