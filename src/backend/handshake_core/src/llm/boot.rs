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

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;

use crate::flight_recorder::FlightRecorder;
use crate::loom_search::LOOM_SEARCH_EMBEDDING_DIM;
use crate::model_runtime::{
    candle::CandleRuntime, llama_cpp::LlamaCppRuntime, BaseModelTag, KvCachePolicy, LoadSpec,
    ModelCapabilities, ModelCatalog, ModelId, ModelRegistration, ModelRegistry, ModelRegistryStore,
    ModelRuntime, ModelRuntimeError, ModelRuntimeRole, ModelRuntimeSelection,
    ModelRuntimeSelectionPurpose, OperatorId, ProviderKind as RuntimeProviderKind,
    RoleBoundModelRegistration, RuntimeArtifactIntegrityReceipt, RuntimeBinding, SamplingParams,
};
use crate::process_ledger::{EmbeddedRuntimeInstanceDescriptor, LedgerBatcher};

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
// The first durable START can include PostgreSQL authority discovery, ACL
// verification, crash-durability checks, and a synchronous commit. Keep boot
// fail-closed, but do not misclassify a healthy authoritative write as failed
// under normal host/CI contention.
const EMBEDDED_START_DURABILITY_TIMEOUT: Duration = Duration::from_secs(30);

struct AttestedEmbeddedLoad {
    model_id: ModelId,
    artifact_integrity: RuntimeArtifactIntegrityReceipt,
    capabilities: ModelCapabilities,
}

/// Injection seam for proving the deployed provider resolver without forcing
/// integration tests to load a heavyweight embedded engine. Production always
/// supplies [`ProductionDefaultLocalClientFactory`].
#[async_trait]
pub trait DefaultLocalClientFactory: Send + Sync {
    async fn build(
        &self,
        resolved: &ResolvedProvider,
        flight_recorder: Arc<dyn FlightRecorder>,
        ledger: Option<LedgerBatcher>,
        model_registry_store: Option<ModelRegistryStore>,
        runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
    ) -> Arc<dyn LlmClient>;
}

#[derive(Debug, Default)]
struct ProductionDefaultLocalClientFactory;

#[async_trait]
impl DefaultLocalClientFactory for ProductionDefaultLocalClientFactory {
    async fn build(
        &self,
        resolved: &ResolvedProvider,
        flight_recorder: Arc<dyn FlightRecorder>,
        ledger: Option<LedgerBatcher>,
        model_registry_store: Option<ModelRegistryStore>,
        runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
    ) -> Arc<dyn LlmClient> {
        build_default_local_client(
            resolved,
            flight_recorder,
            ledger,
            model_registry_store,
            runtime_instance,
        )
        .await
    }
}

/// Resolves the default `LlmClient` from environment configuration.
///
/// This is the single entry point `main.rs::init_llm_client` delegates to. It
/// reads [`ProviderRegistry::from_env`], resolves the Orchestrator role, then
/// dispatches to the embedded local-runtime lane or the retained external_compat
/// compat lane. Cloud-tier clients are wrapped in [`CloudEscalationGuard`].
pub fn embedded_runtime_boot_requested_from_env() -> Result<bool, LlmError> {
    let registry = ProviderRegistry::from_env()?;
    let resolved = registry.resolve(RuntimeRole::Orchestrator)?;
    Ok(resolved_provider_requires_embedded_runtime(&resolved))
}

pub fn resolved_provider_requires_embedded_runtime(resolved: &ResolvedProvider) -> bool {
    resolved.kind == ProviderKind::LocalRuntime && resolved.local_model.is_some()
}

pub async fn resolve_default_llm_client(
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
    model_registry_store: Option<ModelRegistryStore>,
    runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
) -> Arc<dyn LlmClient> {
    resolve_default_llm_client_with_factory(
        flight_recorder,
        ledger,
        model_registry_store,
        runtime_instance,
        Arc::new(ProductionDefaultLocalClientFactory),
    )
    .await
}

/// Executes the same environment-backed provider resolution and dispatch as
/// [`resolve_default_llm_client`] while allowing an engine-free local factory
/// in integration tests. This function does not bypass registry authority.
pub async fn resolve_default_llm_client_with_factory(
    flight_recorder: Arc<dyn FlightRecorder>,
    ledger: Option<LedgerBatcher>,
    model_registry_store: Option<ModelRegistryStore>,
    runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
    local_factory: Arc<dyn DefaultLocalClientFactory>,
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
            local_factory
                .build(
                    &resolved,
                    Arc::clone(&flight_recorder),
                    ledger,
                    model_registry_store,
                    runtime_instance,
                )
                .await
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
/// - the persistent model-registry authority is absent, malformed, or rejects
///   the configured immutable artifact-to-adapter selection, OR
/// - the embedded `ModelRuntime::load` fails, OR
/// - registration into the fresh [`ModelRegistry`] fails, OR
/// - the complete primary-plus-embedding boot set cannot be atomically
///   persisted and read back after load.
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
    model_registry_store: Option<ModelRegistryStore>,
    runtime_instance: Option<EmbeddedRuntimeInstanceDescriptor>,
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
    let Some(runtime_instance) = runtime_instance else {
        let reason =
            "HSK-LOCAL-DISABLED: embedded runtime liveness lease unavailable; refusing model artifact access"
                .to_string();
        tracing::error!(
            target: "handshake_core::llm",
            "LLM disabled (embedded runtime liveness lease unavailable)"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    };

    // A configured production-local load is authority-bearing even though it
    // runs in process. Refuse artifact access unless the ProcessOwnershipLedger
    // writer is live; the pure assembly helpers remain optional-ledger seams for
    // narrow engine-free tests.
    let Some(ledger) = ledger else {
        let reason = "HSK-LOCAL-DISABLED: ProcessOwnershipLedger authority was not supplied; refusing model artifact access".to_string();
        tracing::error!(
            target: "handshake_core::llm",
            error = %reason,
            "LLM disabled before embedded model load"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    };

    let primary_load_capabilities = default_local_load_capabilities(local.runtime_binding);
    let primary_required_capabilities = default_local_required_capabilities();
    let embedding_capabilities = resolved
        .local_embedding_model
        .as_ref()
        .map(|embedding_model| {
            dedicated_embedding_capabilities(
                embedding_model
                    .embedding_dimension
                    .unwrap_or(LOOM_SEARCH_EMBEDDING_DIM),
            )
        });

    // MT-014 durable-selection gate. Environment decoding above reads only
    // configuration strings and the operator-supplied expected hash; it does
    // not open the artifact. Verify PostgreSQL authority and the complete
    // primary-plus-embedding immutable selection set before either runtime is
    // allowed to read model weights.
    let Some(model_registry_store) = model_registry_store else {
        let reason = "HSK-LOCAL-DISABLED: persistent model registry authority was not supplied; refusing model artifact access".to_string();
        tracing::error!(
            target: "handshake_core::llm",
            error = %reason,
            "LLM disabled before embedded model load"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    };
    let mut configured_selections = vec![ModelRuntimeSelection {
        artifact_sha256: local.sha256,
        runtime_binding: local.runtime_binding,
        runtime_role: ModelRuntimeRole::Completion,
        declared_capabilities: primary_load_capabilities.clone(),
        provider: RuntimeProviderKind::Local,
    }];
    if let (Some(embedding_model), Some(capabilities)) = (
        resolved.local_embedding_model.as_ref(),
        embedding_capabilities.as_ref(),
    ) {
        configured_selections.push(ModelRuntimeSelection {
            artifact_sha256: embedding_model.sha256,
            runtime_binding: embedding_model.runtime_binding,
            runtime_role: ModelRuntimeRole::Embedding,
            declared_capabilities: capabilities.clone(),
            provider: RuntimeProviderKind::Local,
        });
    }
    if let Err(err) = model_registry_store
        .recover_configured_runtime_binding_set(&configured_selections)
        .await
    {
        let reason = format!(
            "HSK-LOCAL-DISABLED: persistent model registry preflight failed before artifact access: {err}"
        );
        tracing::error!(
            target: "handshake_core::llm",
            error = %reason,
            "LLM disabled by persistent model registry authority"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    }

    // Reserve one START and one guaranteed STOP for every configured runtime
    // before either artifact is opened. The reservation set is all-or-none: a
    // closed, full, or undersized writer fails here with zero artifact access
    // and zero partial lifecycle rows. Unused reservations are simply released.
    let lifecycle_count = configured_selections.len();
    let mut lifecycle_reservations = match ledger.try_reserve_lifecycles(lifecycle_count) {
        Ok(reservations) => VecDeque::from(reservations),
        Err(err) => {
            let reason = format!(
                "HSK-LOCAL-DISABLED: ProcessOwnershipLedger could not reserve complete START/STOP authority for {lifecycle_count} configured model(s) before artifact access: {err}"
            );
            tracing::error!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled before embedded model load"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ));
        }
    };
    let Some(primary_lifecycle) = lifecycle_reservations.pop_front() else {
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            "HSK-LOCAL-DISABLED: complete lifecycle reservation returned no primary slot"
                .to_string(),
            flight_recorder,
        ));
    };
    let embedding_lifecycle = if resolved.local_embedding_model.is_some() {
        let Some(reservation) = lifecycle_reservations.pop_front() else {
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                "HSK-LOCAL-DISABLED: complete lifecycle reservation returned no embedding slot"
                    .to_string(),
                flight_recorder,
            ));
        };
        Some(reservation)
    } else {
        None
    };
    if !lifecycle_reservations.is_empty() {
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            "HSK-LOCAL-DISABLED: complete lifecycle reservation returned unexpected extra slots"
                .to_string(),
            flight_recorder,
        ));
    }

    let load_spec = LoadSpec {
        artifact_path: local.artifact_path.clone(),
        sha256_expected: hex::encode(local.sha256),
        runtime_kind: local.runtime_binding.runtime_kind(),
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: primary_load_capabilities.clone(),
        provider: RuntimeProviderKind::Local,
        engine_origin: Some(local.runtime_binding.adapter_id().to_string()),
        external_engine_import: None,
    };

    // Build BOTH runtimes; only the configured binding is loaded. LocalRouter
    // requires both handles and resolves per-model by binding, so the unloaded
    // runtime is never dispatched to for this model.
    let mut llama_runtime = LlamaCppRuntime::new(KvCachePolicy::default());
    let mut candle_runtime = CandleRuntime::default();

    // load-then-freeze: the selected production runtime boundary validates its
    // format-specific exact-byte receipt, runtime-derived capabilities,
    // required capability contract, and UUIDv7 identity before publishing the
    // model in its runtime cache.
    let primary_attested = match local.runtime_binding {
        RuntimeBinding::LlamaCpp => llama_runtime
            .load_attested(load_spec, &primary_required_capabilities)
            .await
            .map(|attested| AttestedEmbeddedLoad {
                model_id: attested.model_id,
                artifact_integrity: attested.artifact_integrity,
                capabilities: attested.capabilities,
            }),
        RuntimeBinding::Candle => candle_runtime
            .load_attested(load_spec, &primary_required_capabilities)
            .await
            .map(|attested| AttestedEmbeddedLoad {
                model_id: attested.model_id,
                artifact_integrity: attested.artifact_integrity.into(),
                capabilities: attested.capabilities,
            }),
    };
    let primary_attested = match primary_attested {
        Ok(attested) => attested,
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
    // MT-013-PRIMARY-ATTESTED-SUCCESS: only infallible field moves may occur
    // before the reserved START transition below.
    let model_id = primary_attested.model_id;
    let primary_artifact_integrity = primary_attested.artifact_integrity;
    let primary_runtime_capabilities = primary_attested.capabilities;

    // One boot-wide ACK-wait budget covers every configured embedded START.
    // Model loading and other boot work do not consume this budget: only time
    // actually spent waiting for durable ACKs does. A primary plus embedding
    // configuration therefore cannot turn two bounded waits into a sequential
    // 60-second shutdown blind spot, while a legitimate slow embedding load
    // cannot starve its own ACK wait.
    let mut start_durability_budget = EMBEDDED_START_DURABILITY_TIMEOUT;

    // START-on-load is literal: the first fallible transition after a successful
    // attested load publishes its ownership row. Boot then waits for store-level
    // acknowledgement before any registry or client surface is built.
    // MT-013-PRIMARY-START-BOUNDARY
    let (primary_process, primary_start_ack) =
        match EmbeddedModelProcess::record_reserved_load_with_durable_ack(
            primary_lifecycle,
            local.runtime_binding,
            model_id,
            &local.display_name,
            &primary_artifact_integrity,
            Some(&runtime_instance),
        ) {
            Ok(started) => started,
            Err(err) => {
                let _ = unload_loaded_model(
                    &mut llama_runtime,
                    &mut candle_runtime,
                    local.runtime_binding,
                    model_id,
                    "primary-start-transition-failed",
                )
                .await;
                let reason = format!(
                    "HSK-LOCAL-DISABLED: reserved primary ProcessOwnershipLedger START transition failed: {err}"
                );
                return Arc::new(DisabledLlmClient::new_recorded(
                    resolved.model_id.clone(),
                    reason,
                    flight_recorder,
                ));
            }
        };
    let primary_ack_wait_started = Instant::now();
    let primary_ack_result = primary_start_ack.wait(start_durability_budget).await;
    start_durability_budget =
        start_durability_budget.saturating_sub(primary_ack_wait_started.elapsed());
    if let Err(err) = primary_ack_result {
        let _ = unload_loaded_model(
            &mut llama_runtime,
            &mut candle_runtime,
            local.runtime_binding,
            model_id,
            "primary-start-durability-failed",
        )
        .await;
        // A timed-out/lost/rejected ACK cannot prove whether START committed.
        // Never publish STOP against a possibly conflicting or absent row.
        primary_process.leave_open_for_reconciliation();
        let reason = format!(
            "HSK-LOCAL-DISABLED: primary ProcessOwnershipLedger START was not durably acknowledged: {err}"
        );
        tracing::error!(
            target: "handshake_core::process_ledger",
            error = %reason,
            model_id = %model_id,
            "LLM disabled before READY exposure"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    }
    let registration = ModelRegistration {
        model_id,
        artifact_path: local.artifact_path.clone(),
        sha256: local.sha256,
        runtime_binding: local.runtime_binding,
        declared_capabilities: primary_runtime_capabilities,
        base_model_tag: BaseModelTag::new(local.display_name.clone()),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("handshake-embedded-default"),
        provider: RuntimeProviderKind::Local,
    };

    let mut ready_registry = ModelRegistry::default();
    if let Err(err) = register_ready_registration(&mut ready_registry, &registration) {
        rollback_loaded_embedded_model(
            &mut llama_runtime,
            &mut candle_runtime,
            local.runtime_binding,
            model_id,
            Some(&primary_process),
            "primary-registry-preflight-failed",
        )
        .await;
        let reason = format!(
            "HSK-LOCAL-DISABLED: primary local runtime registry transition failed after durable START: {err}"
        );
        tracing::error!(
            target: "handshake_core::llm",
            error = %reason,
            "LLM disabled before READY exposure"
        );
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            reason,
            flight_recorder,
        ));
    }
    let mut embedded_processes = vec![primary_process];

    let mut additional_registrations = Vec::new();
    if let Some(embedding_model) = resolved.local_embedding_model.as_ref() {
        let embedding_lifecycle = embedding_lifecycle
            .expect("configured embedding model was preassigned a lifecycle reservation");
        let embedding_capabilities = dedicated_embedding_capabilities(
            embedding_model
                .embedding_dimension
                .unwrap_or(LOOM_SEARCH_EMBEDDING_DIM),
        );
        let embedding_load_spec = LoadSpec {
            artifact_path: embedding_model.artifact_path.clone(),
            sha256_expected: hex::encode(embedding_model.sha256),
            runtime_kind: embedding_model.runtime_binding.runtime_kind(),
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: embedding_capabilities.clone(),
            provider: RuntimeProviderKind::Local,
            engine_origin: Some(embedding_model.runtime_binding.adapter_id().to_string()),
            external_engine_import: None,
        };
        let embedding_attested = match embedding_model.runtime_binding {
            RuntimeBinding::LlamaCpp => llama_runtime
                .load_attested(embedding_load_spec, &embedding_capabilities)
                .await
                .map(|attested| AttestedEmbeddedLoad {
                    model_id: attested.model_id,
                    artifact_integrity: attested.artifact_integrity,
                    capabilities: attested.capabilities,
                }),
            RuntimeBinding::Candle => candle_runtime
                .load_attested(embedding_load_spec, &embedding_capabilities)
                .await
                .map(|attested| AttestedEmbeddedLoad {
                    model_id: attested.model_id,
                    artifact_integrity: attested.artifact_integrity.into(),
                    capabilities: attested.capabilities,
                }),
        };
        let embedding_attested = match embedding_attested {
            Ok(attested) => attested,
            Err(err) => {
                rollback_loaded_embedded_model(
                    &mut llama_runtime,
                    &mut candle_runtime,
                    local.runtime_binding,
                    model_id,
                    Some(&embedded_processes[0]),
                    "embedding-load-failed-primary-rollback",
                )
                .await;
                let reason = format!(
                    "HSK-LOCAL-DISABLED: embedded embedding ModelRuntime load failed: {err}"
                );
                tracing::warn!(
                    target: "handshake_core::llm",
                    error = %reason,
                    binding = %embedding_model.runtime_binding.adapter_id(),
                    "LLM disabled (dedicated embedding model load failed; no daemon fallback)"
                );
                return Arc::new(DisabledLlmClient::new_recorded(
                    resolved.model_id.clone(),
                    reason,
                    flight_recorder,
                ));
            }
        };
        // MT-013-EMBEDDING-ATTESTED-SUCCESS: only infallible field moves may
        // occur before the reserved START transition below.
        let embedding_model_id = embedding_attested.model_id;
        let embedding_artifact_integrity = embedding_attested.artifact_integrity;
        let embedding_runtime_capabilities = embedding_attested.capabilities;
        // MT-013-EMBEDDING-START-BOUNDARY
        let (embedding_process, embedding_start_ack) =
            match EmbeddedModelProcess::record_reserved_load_with_durable_ack(
                embedding_lifecycle,
                embedding_model.runtime_binding,
                embedding_model_id,
                &embedding_model.display_name,
                &embedding_artifact_integrity,
                Some(&runtime_instance),
            ) {
                Ok(started) => started,
                Err(err) => {
                    let _ = unload_loaded_model(
                        &mut llama_runtime,
                        &mut candle_runtime,
                        embedding_model.runtime_binding,
                        embedding_model_id,
                        "embedding-start-transition-failed",
                    )
                    .await;
                    rollback_loaded_embedded_model(
                        &mut llama_runtime,
                        &mut candle_runtime,
                        local.runtime_binding,
                        model_id,
                        Some(&embedded_processes[0]),
                        "embedding-start-transition-failed-primary-rollback",
                    )
                    .await;
                    let reason = format!(
                    "HSK-LOCAL-DISABLED: reserved embedding ProcessOwnershipLedger START transition failed: {err}"
                );
                    return Arc::new(DisabledLlmClient::new_recorded(
                        resolved.model_id.clone(),
                        reason,
                        flight_recorder,
                    ));
                }
            };
        let embedding_ack_result = embedding_start_ack.wait(start_durability_budget).await;
        if let Err(err) = embedding_ack_result {
            let _ = unload_loaded_model(
                &mut llama_runtime,
                &mut candle_runtime,
                embedding_model.runtime_binding,
                embedding_model_id,
                "embedding-start-durability-failed",
            )
            .await;
            embedding_process.leave_open_for_reconciliation();
            rollback_loaded_embedded_model(
                &mut llama_runtime,
                &mut candle_runtime,
                local.runtime_binding,
                model_id,
                Some(&embedded_processes[0]),
                "embedding-start-durability-failed-primary-rollback",
            )
            .await;
            let reason = format!(
                "HSK-LOCAL-DISABLED: embedding ProcessOwnershipLedger START was not durably acknowledged: {err}"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ));
        }
        let embedding_registration = ModelRegistration {
            model_id: embedding_model_id,
            artifact_path: embedding_model.artifact_path.clone(),
            sha256: embedding_model.sha256,
            runtime_binding: embedding_model.runtime_binding,
            declared_capabilities: embedding_runtime_capabilities,
            base_model_tag: BaseModelTag::new(embedding_model.display_name.clone()),
            registered_at_utc: Utc::now(),
            registered_by: OperatorId::new("handshake-embedded-embedding"),
            provider: RuntimeProviderKind::Local,
        };
        if let Err(err) = register_ready_registration(&mut ready_registry, &embedding_registration)
        {
            rollback_loaded_embedded_model(
                &mut llama_runtime,
                &mut candle_runtime,
                embedding_model.runtime_binding,
                embedding_model_id,
                Some(&embedding_process),
                "embedding-registry-transition-failed",
            )
            .await;
            rollback_loaded_embedded_model(
                &mut llama_runtime,
                &mut candle_runtime,
                local.runtime_binding,
                model_id,
                Some(&embedded_processes[0]),
                "embedding-registry-transition-failed-primary-rollback",
            )
            .await;
            let reason = format!(
                "HSK-LOCAL-DISABLED: embedding local runtime registry transition failed after durable START: {err}"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ));
        }
        embedded_processes.push(embedding_process);
        additional_registrations.push(embedding_registration);
    }

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

    let mut durable_registrations = Vec::with_capacity(1 + additional_registrations.len());
    durable_registrations.push(RoleBoundModelRegistration::completion(registration.clone()));
    durable_registrations.extend(
        additional_registrations
            .iter()
            .cloned()
            .map(RoleBoundModelRegistration::embedding),
    );

    // Keep both runtimes mutable until the last fallible authority transition is
    // complete. If persistence fails, concrete unload results gate each STOP;
    // no Arc client or READY model can escape this branch.
    let committed_registrations = match model_registry_store
        .persist_role_bound_boot_set_and_read_back(&durable_registrations)
        .await
    {
        Ok(committed) => committed,
        Err(err) => {
            for (registration, process) in durable_registrations
                .iter()
                .zip(embedded_processes.iter())
                .rev()
            {
                rollback_loaded_embedded_model(
                    &mut llama_runtime,
                    &mut candle_runtime,
                    registration.registration.runtime_binding,
                    registration.registration.model_id,
                    Some(process),
                    "persistent-registry-commit-failed",
                )
                .await;
            }
            let reason = format!(
            "HSK-LOCAL-DISABLED: persistent model registry commit/read-back failed after load: {err}"
        );
            tracing::error!(
                target: "handshake_core::llm",
                error = %reason,
                "LLM disabled; loaded embedded model was shut down before exposure"
            );
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                reason,
                flight_recorder,
            ));
        }
    };
    let mut active_candidates = vec![(
        ModelRuntimeSelectionPurpose::ApplicationDefault,
        registration.sha256,
    )];
    if let Some(embedding) = additional_registrations.first() {
        active_candidates.push((
            ModelRuntimeSelectionPurpose::EmbeddingsDefault,
            embedding.sha256,
        ));
    }
    let active_defaults = match model_registry_store
        .ensure_active_defaults(&active_candidates)
        .await
    {
        Ok(active) => active,
        Err(err) => {
            for (registration, process) in durable_registrations
                .iter()
                .zip(embedded_processes.iter())
                .rev()
            {
                rollback_loaded_embedded_model(
                    &mut llama_runtime,
                    &mut candle_runtime,
                    registration.registration.runtime_binding,
                    registration.registration.model_id,
                    Some(process),
                    "active-default-recovery-failed",
                )
                .await;
            }
            return Arc::new(DisabledLlmClient::new_recorded(
                resolved.model_id.clone(),
                format!(
                    "HSK-LOCAL-DISABLED: PostgreSQL active ModelRuntime default recovery failed: {err}"
                ),
                flight_recorder,
            ));
        }
    };
    let active_application = active_defaults
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default candidate always yields one committed selection");
    let active_application_model_id = committed_registrations
        .iter()
        .find(|row| row.artifact_sha256 == active_application.artifact_sha256)
        .map(|row| row.last_observed_runtime_model_id);
    let active_embedding_model_id = active_defaults
        .iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::EmbeddingsDefault)
        .and_then(|selection| {
            committed_registrations
                .iter()
                .find(|row| row.artifact_sha256 == selection.artifact_sha256)
                .map(|row| row.last_observed_runtime_model_id)
        });
    if active_application_model_id.is_none()
        || (additional_registrations.first().is_some() && active_embedding_model_id.is_none())
    {
        for (registration, process) in durable_registrations
            .iter()
            .zip(embedded_processes.iter())
            .rev()
        {
            rollback_loaded_embedded_model(
                &mut llama_runtime,
                &mut candle_runtime,
                registration.registration.runtime_binding,
                registration.registration.model_id,
                Some(process),
                "active-default-not-ready",
            )
            .await;
        }
        return Arc::new(DisabledLlmClient::new_recorded(
            resolved.model_id.clone(),
            "HSK-LOCAL-DISABLED: PostgreSQL active ModelRuntime default does not resolve to a READY configured artifact"
                .to_owned(),
            flight_recorder,
        ));
    }
    let active_application_model_id =
        active_application_model_id.expect("checked active application model id");
    let runtime_roles = committed_registrations
        .iter()
        .map(|row| (row.last_observed_runtime_model_id, row.runtime_role))
        .collect::<HashMap<_, _>>();

    // Every fallible load/START/registry transition is now complete. Assembly is
    // pure and infallible; only at this point are runtimes frozen behind Arc and
    // the READY client exposed to AppState.
    let mut client = assemble_ready_local_runtime_client(
        active_application_model_id,
        ready_registry,
        runtime_roles,
        active_embedding_model_id,
        Arc::new(llama_runtime),
        Arc::new(candle_runtime),
        fallback,
        Arc::clone(&flight_recorder),
        DEFAULT_LOCAL_MAX_CONTEXT_TOKENS,
    )
    .with_runtime_control_authority(ledger.clone(), runtime_instance.clone())
    .with_durable_application_selection(
        model_registry_store.clone(),
        active_application.selection_revision,
    );
    for embedded_process in embedded_processes {
        client = client.with_embedded_process(embedded_process);
    }

    Arc::new(client)
}

/// Roll back one loaded in-process model without ever publishing a false STOP.
/// A matching STOP is emitted only after the concrete runtime confirms unload;
/// otherwise the durable START is deliberately left open for the boot reclaimer.
async fn rollback_loaded_embedded_model(
    llama_runtime: &mut LlamaCppRuntime,
    candle_runtime: &mut CandleRuntime,
    binding: RuntimeBinding,
    model_id: ModelId,
    process: Option<&EmbeddedModelProcess>,
    reason: &str,
) -> bool {
    match unload_loaded_model(llama_runtime, candle_runtime, binding, model_id, reason).await {
        Ok(()) => {
            if let Some(process) = process {
                if let Err(error) = process.shutdown(reason) {
                    tracing::error!(
                        target: "handshake_core::process_ledger",
                        error = %error,
                        process_uuid = %process.process_uuid(),
                        rollback_reason = reason,
                        "runtime unload succeeded but its reserved rollback STOP transition failed"
                    );
                }
            }
            true
        }
        Err(_error) => {
            if let Some(process) = process {
                process.leave_open_for_reconciliation();
                tracing::error!(
                    target: "handshake_core::process_ledger",
                    process_uuid = %process.process_uuid(),
                    rollback_reason = reason,
                    "runtime unload was not proven; leaving START open for reconciliation"
                );
            }
            false
        }
    }
}

async fn unload_loaded_model(
    llama_runtime: &mut LlamaCppRuntime,
    candle_runtime: &mut CandleRuntime,
    binding: RuntimeBinding,
    model_id: ModelId,
    reason: &str,
) -> Result<(), ModelRuntimeError> {
    let result = match binding {
        RuntimeBinding::LlamaCpp => llama_runtime.unload(model_id).await,
        RuntimeBinding::Candle => candle_runtime.unload(model_id).await,
    };
    if let Err(err) = &result {
        tracing::error!(
            target: "handshake_core::llm",
            error = %err,
            model_id = %model_id,
            binding = %binding.adapter_id(),
            rollback_reason = reason,
            "explicit embedded runtime rollback unload failed; no STOP may be recorded without proven quiescence"
        );
    }
    result
}

fn register_ready_registration(
    registry: &mut ModelRegistry,
    registration: &ModelRegistration,
) -> Result<(), LlmError> {
    let model_id = registration.model_id;
    registry.register(registration.clone()).map_err(|err| {
        LlmError::ProviderError(format!("local model registration failed: {err}"))
    })?;
    registry
        .mark_loaded(model_id)
        .map_err(|err| LlmError::ProviderError(format!("local model load-marking failed: {err}")))
}

fn assemble_ready_local_runtime_client(
    primary_model_id: ModelId,
    registry: ModelRegistry,
    runtime_roles: HashMap<ModelId, ModelRuntimeRole>,
    active_embedding_model_id: Option<ModelId>,
    llama_runtime: Arc<dyn ModelRuntime>,
    candle_runtime: Arc<dyn ModelRuntime>,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    max_context_tokens: u32,
) -> LocalModelRuntimeLlmClient {
    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry_with_roles_and_embedding_default(
        Arc::clone(&registry),
        runtime_roles,
        active_embedding_model_id,
    );
    let router = LocalRouter::new(registry, llama_runtime, candle_runtime);
    let profile =
        ModelProfile::new(primary_model_id.to_string(), max_context_tokens).with_streaming(true);
    LocalModelRuntimeLlmClient::new(router, fallback, flight_recorder, profile)
        .with_catalog(catalog)
}

#[cfg(test)]
mod boot_contract_tests {
    use super::*;

    #[test]
    fn boot_attested_success_has_no_pre_start_post_load_probe() {
        let source = include_str!("boot.rs");
        for (success_marker, start_marker) in [
            (
                "MT-013-PRIMARY-ATTESTED-SUCCESS",
                "MT-013-PRIMARY-START-BOUNDARY",
            ),
            (
                "MT-013-EMBEDDING-ATTESTED-SUCCESS",
                "MT-013-EMBEDDING-START-BOUNDARY",
            ),
        ] {
            let success = source
                .find(success_marker)
                .expect("attested success marker");
            let start = source[success..]
                .find(start_marker)
                .map(|offset| success + offset)
                .expect("reserved START boundary after attested success");
            let seam = &source[success..start];
            for forbidden in [
                ".artifact_integrity(",
                ".capabilities(",
                "validate_loaded_model_id",
                "register_ready_registration",
                "persist_boot_set_and_read_back",
            ] {
                assert!(
                    !seam.contains(forbidden),
                    "post-load {forbidden} may not fail before reserved START"
                );
            }
        }

        let attested_call = [".load_", "attested("].concat();
        assert_eq!(source.matches(&attested_call).count(), 4);
        assert!(source.contains("RuntimeBinding::LlamaCpp => llama_runtime"));
        let obsolete_llama_integrity_error = [
            "llama.cpp exact-byte artifact integrity",
            " is not yet proven",
        ]
        .concat();
        assert!(!source.contains(&obsolete_llama_integrity_error));
    }

    #[test]
    fn boot_separates_optional_candle_probe_from_minimum_primary_requirements() {
        let candle_probe = default_local_load_capabilities(RuntimeBinding::Candle);
        let llama_probe = default_local_load_capabilities(RuntimeBinding::LlamaCpp);
        let required = default_local_required_capabilities();

        assert!(candle_probe.supports_activation_steering);
        assert!(!llama_probe.supports_activation_steering);
        assert!(!required.supports_activation_steering);

        let transformer_actual = ModelCapabilities {
            supports_activation_steering: true,
            ..Default::default()
        };
        let ssm_actual = ModelCapabilities {
            supports_subquadratic: true,
            supports_activation_steering: false,
            ..Default::default()
        };
        ensure_loaded_capabilities_satisfy(&required, &transformer_actual)
            .expect("minimal primary contract accepts discovered transformer capability");
        ensure_loaded_capabilities_satisfy(&required, &ssm_actual)
            .expect("minimal primary contract accepts honest SSM capability finalizer");
        assert!(
            ensure_loaded_capabilities_satisfy(&candle_probe, &ssm_actual).is_err(),
            "the load probe must not be reused as the minimum boot requirement"
        );
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
#[doc(hidden)]
#[cfg(feature = "test-utils")]
pub fn assemble_local_runtime_client(
    registration: ModelRegistration,
    llama_runtime: Arc<dyn ModelRuntime>,
    candle_runtime: Arc<dyn ModelRuntime>,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    max_context_tokens: u32,
    ledger: Option<LedgerBatcher>,
) -> Result<LocalModelRuntimeLlmClient, LlmError> {
    assemble_local_runtime_client_with_registrations(
        registration,
        Vec::new(),
        llama_runtime,
        candle_runtime,
        fallback,
        flight_recorder,
        max_context_tokens,
        ledger,
    )
}

/// Assembles a local runtime client with one primary chat/completion model plus
/// optional additional local registrations such as a dedicated embedding model.
/// The primary registration remains the client's `profile().model_id`; all
/// registrations share the same registry/catalog/router and are marked READY.
#[doc(hidden)]
#[cfg(feature = "test-utils")]
pub fn assemble_local_runtime_client_with_registrations(
    registration: ModelRegistration,
    additional_registrations: Vec<ModelRegistration>,
    llama_runtime: Arc<dyn ModelRuntime>,
    candle_runtime: Arc<dyn ModelRuntime>,
    fallback: Arc<dyn LlmClient>,
    flight_recorder: Arc<dyn FlightRecorder>,
    max_context_tokens: u32,
    ledger: Option<LedgerBatcher>,
) -> Result<LocalModelRuntimeLlmClient, LlmError> {
    // WP-1 MT-013: capture the ProcessOwnershipLedger START-row inputs BEFORE
    // the registrations are moved into the registry. This covers the primary
    // chat/completion model and any optional dedicated embedding model loaded
    // in the same default local-runtime boot path.
    let ledger_registrations = std::iter::once(&registration)
        .chain(additional_registrations.iter())
        .map(|registration| {
            (
                registration.runtime_binding,
                registration.model_id,
                registration.base_model_tag.as_str().to_string(),
                hex::encode(registration.sha256),
            )
        })
        .collect::<Vec<_>>();
    let model_id = registration.model_id;
    let active_embedding_model_id = additional_registrations
        .first()
        .map(|registration| registration.model_id);

    let mut registry = ModelRegistry::default();
    register_ready_registration(&mut registry, &registration)?;
    let mut runtime_roles = HashMap::from([(registration.model_id, ModelRuntimeRole::Completion)]);
    for additional in additional_registrations {
        runtime_roles.insert(additional.model_id, ModelRuntimeRole::Embedding);
        register_ready_registration(&mut registry, &additional)?;
    }

    // MT-014: share ONE `Arc<ModelRegistry>` between dispatch and enumeration.
    // Every fallible registration/READY transition is complete before the
    // runtime Arcs are moved into this infallible assembly step.
    let mut client = assemble_ready_local_runtime_client(
        model_id,
        registry,
        runtime_roles,
        active_embedding_model_id,
        llama_runtime,
        candle_runtime,
        fallback,
        flight_recorder,
        max_context_tokens,
    );

    // WP-1 MT-013: emit ProcessOwnershipLedger START rows for the just-loaded
    // in-process embedded models (master-spec §3.6.2 clause 2 / §4.6.1) and
    // attach ownership handles so the client owns the STOP-on-shutdown
    // obligation. Rows are pid-less (`os_pid = None`) because the in-process
    // library loads spawn no OS process. If a ledger handle is supplied, START
    // emit failure is fail-closed: an active unledgered default embedded model
    // violates MT-013's HARD attribution/reclaim requirement.
    if let Some(ledger) = ledger {
        let reservations = ledger
            .try_reserve_lifecycles(ledger_registrations.len())
            .map_err(|err| {
                LlmError::ProviderError(format!(
                    "HSK-LOCAL-DISABLED: complete embedded ProcessOwnershipLedger lifecycle reservation failed: {err}"
                ))
            })?;
        for ((runtime_binding, model_id, display_name, artifact_sha256), reservation) in
            ledger_registrations.into_iter().zip(reservations)
        {
            match EmbeddedModelProcess::record_reserved_load(
                reservation,
                runtime_binding,
                model_id,
                &display_name,
                Some(artifact_sha256),
                None,
            ) {
                Ok(embedded_process) => {
                    client = client.with_embedded_process(embedded_process);
                }
                Err(err) => {
                    tracing::error!(
                        target: "handshake_core::llm",
                        error = %err,
                        model_id = %model_id,
                        "embedded model ProcessOwnershipLedger START emit failed; failing closed"
                    );
                    return Err(LlmError::ProviderError(format!(
                        "HSK-LOCAL-DISABLED: embedded ProcessOwnershipLedger START emit failed: {err}"
                    )));
                }
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

/// Capabilities the loader should probe and expose when its selected
/// architecture implements them. Candle transformer loads advertise the hook
/// lane; SSM finalizers may honestly turn it off after architecture detection.
fn default_local_load_capabilities(binding: RuntimeBinding) -> ModelCapabilities {
    ModelCapabilities {
        supports_activation_steering: binding == RuntimeBinding::Candle,
        ..Default::default()
    }
}

/// Minimum contract required for the default completion lane. Optional
/// architecture capabilities are discovered after load and persisted from the
/// runtime readback; they are not boot requirements.
fn default_local_required_capabilities() -> ModelCapabilities {
    ModelCapabilities::default()
}

fn dedicated_embedding_capabilities(embedding_dimension: usize) -> ModelCapabilities {
    ModelCapabilities {
        supports_embedding: true,
        embedding_dimension: Some(embedding_dimension),
        ..Default::default()
    }
}

#[cfg(test)]
fn ensure_loaded_capabilities_satisfy(
    required: &ModelCapabilities,
    actual: &ModelCapabilities,
) -> Result<(), String> {
    let required_flags = [
        (
            "supports_lora",
            required.supports_lora,
            actual.supports_lora,
        ),
        (
            "supports_kv_prefix_cache",
            required.supports_kv_prefix_cache,
            actual.supports_kv_prefix_cache,
        ),
        (
            "supports_activation_steering",
            required.supports_activation_steering,
            actual.supports_activation_steering,
        ),
        (
            "supports_subquadratic",
            required.supports_subquadratic,
            actual.supports_subquadratic,
        ),
        (
            "supports_speculative_draft",
            required.supports_speculative_draft,
            actual.supports_speculative_draft,
        ),
        (
            "supports_eagle3",
            required.supports_eagle3,
            actual.supports_eagle3,
        ),
        (
            "supports_embedding",
            required.supports_embedding,
            actual.supports_embedding,
        ),
    ];
    if let Some((name, _, _)) = required_flags
        .into_iter()
        .find(|(_, required, actual)| *required && !*actual)
    {
        return Err(format!("required capability {name} is absent"));
    }
    if required.supports_kv_quantization != crate::model_runtime::KvQuantSupport::None
        && actual.supports_kv_quantization != required.supports_kv_quantization
    {
        return Err(format!(
            "required KV quantization {:?} differs from actual {:?}",
            required.supports_kv_quantization, actual.supports_kv_quantization
        ));
    }
    if let Some(required_dimension) = required.embedding_dimension {
        if actual.embedding_dimension != Some(required_dimension) {
            return Err(format!(
                "required embedding dimension {required_dimension} differs from actual {:?}",
                actual.embedding_dimension
            ));
        }
    }
    Ok(())
}
