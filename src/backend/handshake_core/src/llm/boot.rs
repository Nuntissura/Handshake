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
    LoadSpec, ModelCapabilities, ModelRegistration, ModelRegistry, ModelRuntime, OperatorId,
    ProviderKind as RuntimeProviderKind, RuntimeBinding, SamplingParams,
};

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
            return Arc::new(DisabledLlmClient::new("unknown".to_string(), reason));
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
            return Arc::new(DisabledLlmClient::new("unknown".to_string(), reason));
        }
    };

    let client: Arc<dyn LlmClient> = match resolved.kind {
        ProviderKind::LocalRuntime => {
            build_default_local_client(&resolved, Arc::clone(&flight_recorder)).await
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
                Arc::new(DisabledLlmClient::new("unknown".to_string(), reason))
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
        return Arc::new(DisabledLlmClient::new(resolved.model_id.clone(), reason));
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
            return Arc::new(DisabledLlmClient::new(resolved.model_id.clone(), reason));
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
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        local.display_name.clone(),
        "HSK-LOCAL-FALLBACK: no external fallback configured for the embedded local model"
            .to_string(),
    ));

    match assemble_local_runtime_client(
        registration,
        Arc::new(llama_runtime),
        Arc::new(candle_runtime),
        fallback,
        flight_recorder,
        DEFAULT_LOCAL_MAX_CONTEXT_TOKENS,
    ) {
        Ok(client) => Arc::new(client),
        Err(err) => {
            let reason = format!("HSK-LOCAL-DISABLED: local runtime registration failed: {err}");
            tracing::warn!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled (local model registration failed)"
            );
            Arc::new(DisabledLlmClient::new(resolved.model_id.clone(), reason))
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
) -> Result<LocalModelRuntimeLlmClient, LlmError> {
    let model_id = registration.model_id;
    let mut registry = ModelRegistry::default();
    registry.register(registration).map_err(|err| {
        LlmError::ProviderError(format!("local model registration failed: {err}"))
    })?;

    let router = LocalRouter::new(Arc::new(registry), llama_runtime, candle_runtime);
    let profile =
        ModelProfile::new(model_id.to_string(), max_context_tokens).with_streaming(true);

    Ok(LocalModelRuntimeLlmClient::new(
        router,
        fallback,
        flight_recorder,
        profile,
    ))
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
