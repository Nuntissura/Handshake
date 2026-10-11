//! MT-014 production-composite support for the native Argus proof.
//!
//! This module is compiled only with `test-utils`. It owns the otherwise noisy
//! setup needed to run the production ModelRuntime registry route against an
//! isolated schema on Handshake-managed PostgreSQL. Native tests still drive
//! the route only through their production HTTP transport.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use futures::stream;
use uuid::Uuid;

use crate::{
    api::model_runtime_registry,
    capabilities::CapabilityRegistry,
    diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup},
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{
        local_router::{LocalModelRuntimeLlmClient, LocalRouter},
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    },
    managed_postgres::{ManagedPostgres, ManagedPostgresConfig},
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, FinishReason, GenerateRequest, GeneratedToken,
        KvCacheHandle, LoraStackHandle, ModelCapabilities, ModelCatalog, ModelId,
        ModelRegistration, ModelRegistry, ModelRegistryStore, ModelRuntime, ModelRuntimeError,
        ModelRuntimeRole, OperatorId, ProviderKind, RoleBoundModelRegistration, RuntimeBinding,
        Score, SteeringHookHandle, TokenStream,
    },
    storage::{postgres::PostgresDatabase, Database},
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};

#[derive(Default)]
struct CapturingFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
    fail_writes: AtomicBool,
}

impl CapturingFlightRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

struct RoleDriftLlmClient {
    inner: Arc<LocalModelRuntimeLlmClient>,
    honest_catalog: Arc<ModelCatalog>,
    drifted_catalog: Arc<ModelCatalog>,
    expose_drift: AtomicBool,
}

#[async_trait]
impl LlmClient for RoleDriftLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.completion(req).await
    }

    async fn swap_model(
        &self,
        req: crate::workflows::ModelSwapRequestV0_4,
    ) -> Result<(), LlmError> {
        self.inner.swap_model(req).await
    }

    fn profile(&self) -> &ModelProfile {
        self.inner.profile()
    }

    fn selected_model_id(&self) -> String {
        self.inner.selected_model_id()
    }

    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        if self.expose_drift.load(Ordering::SeqCst) {
            Some(self.drifted_catalog.clone())
        } else {
            Some(self.honest_catalog.clone())
        }
    }
}

#[async_trait]
impl FlightRecorder for CapturingFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(RecorderError::SinkError(
                "MT-014 forced audit failure".to_owned(),
            ));
        }
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event);
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

#[async_trait]
impl DiagnosticsStore for CapturingFlightRecorder {
    async fn record_diagnostic(
        &self,
        _diagnostic: Diagnostic,
    ) -> Result<(), crate::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, crate::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(&self, _id: Uuid) -> Result<Diagnostic, crate::storage::StorageError> {
        Err(crate::storage::StorageError::NotFound("diagnostic"))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, crate::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct CatalogLlmClient {
    profile: ModelProfile,
    catalog: Arc<ModelCatalog>,
}

#[async_trait]
impl LlmClient for CatalogLlmClient {
    async fn completion(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        Some(self.catalog.clone())
    }
}

#[derive(Default)]
struct ReadyRuntime {
    capabilities: ModelCapabilities,
}

#[async_trait]
impl ModelRuntime for ReadyRuntime {
    async fn load(
        &mut self,
        _spec: crate::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(stream::iter([Ok(GeneratedToken {
            token_id: 0,
            text: "ready".to_owned(),
            logprob: None,
            finish_reason: Some(FinishReason::Stop),
        })]))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: vec![0.0] })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("mt014-native-composite-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("mt014-native-composite-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("mt014-native-composite-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

fn registration(
    model_id: ModelId,
    sha256: [u8; 32],
    runtime_binding: RuntimeBinding,
    label: &str,
    declared_capabilities: ModelCapabilities,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from(format!("registry-proof/{label}.safetensors")),
        sha256,
        runtime_binding,
        declared_capabilities,
        base_model_tag: BaseModelTag::new(label),
        registered_at_utc: chrono::Utc::now(),
        registered_by: OperatorId::new("mt014-native-composite-proof"),
        provider: ProviderKind::Local,
    }
}

async fn isolated_schema(database_url: &str) -> Result<String, String> {
    let schema = format!("mt014_native_{}", Uuid::now_v7().simple());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| format!("connect managed PostgreSQL for MT-014 schema setup: {error}"))?;
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&pool)
        .await
        .map_err(|error| format!("create MT-014 isolated schema: {error}"))?;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .execute(&pool)
        .await
        .map_err(|error| format!("ensure pgcrypto for MT-014 schema: {error}"))?;
    for shim in [
        format!(
            "CREATE OR REPLACE FUNCTION {schema}.digest(input text, algorithm text) \
             RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE \
             AS $$ SELECT public.digest(input::bytea, algorithm) $$"
        ),
        format!(
            "CREATE OR REPLACE FUNCTION {schema}.digest(input bytea, algorithm text) \
             RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE \
             AS $$ SELECT public.digest(input, algorithm) $$"
        ),
    ] {
        sqlx::query(&shim)
            .execute(&pool)
            .await
            .map_err(|error| format!("install MT-014 digest shim: {error}"))?;
    }
    drop(pool);
    let separator = if database_url.contains('?') { "&" } else { "?" };
    Ok(format!(
        "{database_url}{separator}options=-csearch_path%3D{schema}"
    ))
}

/// A live MT-014 system fixture used by the native Argus composite proof.
pub struct Mt014NativeCompositeProof {
    managed_postgres: ManagedPostgres,
    client: Arc<LocalModelRuntimeLlmClient>,
    catalog_client: Arc<RoleDriftLlmClient>,
    recorder: Arc<CapturingFlightRecorder>,
    server: tokio::task::JoinHandle<()>,
    base_url: String,
    pub current_model_id: String,
    pub current_artifact_sha256: String,
    pub target_model_id: String,
    pub target_artifact_sha256: String,
    pub embedding_model_id: String,
    pub embedding_artifact_sha256: String,
    pub stale_model_id: String,
}

impl Mt014NativeCompositeProof {
    /// Start a real route backed by a proven Handshake-managed PostgreSQL endpoint.
    pub async fn start() -> Result<Self, String> {
        let managed_postgres = ManagedPostgres::ensure_running(ManagedPostgresConfig::from_env())
            .await
            .map_err(|error| format!("start Handshake-managed PostgreSQL: {error}"))?;
        if managed_postgres.proven_local_endpoint().is_none() {
            return Err(
                "MT-014 composite proof requires a proven Handshake-managed PostgreSQL endpoint"
                    .to_owned(),
            );
        }
        let schema_url = isolated_schema(&managed_postgres.database_url()).await?;
        let database = PostgresDatabase::connect(&schema_url, 5)
            .await
            .map_err(|error| format!("connect MT-014 isolated PostgresDatabase: {error}"))?;
        database
            .run_migrations()
            .await
            .map_err(|error| format!("run MT-014 migration chain: {error}"))?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&schema_url)
            .await
            .map_err(|error| format!("connect MT-014 registry pool: {error}"))?;

        let current_id = ModelId::new_v7();
        let target_id = ModelId::new_v7();
        let embedding_id = ModelId::new_v7();
        let current_sha256 = [0xa1; 32];
        let target_sha256 = [0xa2; 32];
        let embedding_sha256 = [0xa3; 32];
        let current = registration(
            current_id,
            current_sha256,
            RuntimeBinding::Candle,
            "Current Completion Model",
            ModelCapabilities::default(),
        );
        let target = registration(
            target_id,
            target_sha256,
            RuntimeBinding::LlamaCpp,
            "Target Completion Model",
            ModelCapabilities::default(),
        );
        let embedding = registration(
            embedding_id,
            embedding_sha256,
            RuntimeBinding::Candle,
            "Dedicated Embedding Model",
            ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(1),
                ..ModelCapabilities::default()
            },
        );
        ModelRegistryStore::new(pool.clone())
            .persist_role_bound_boot_set_and_read_back(&[
                RoleBoundModelRegistration::completion(current.clone()),
                RoleBoundModelRegistration::completion(target.clone()),
                RoleBoundModelRegistration::embedding(embedding.clone()),
            ])
            .await
            .map_err(|error| format!("persist MT-014 role-bound registry: {error}"))?;

        let mut registry = ModelRegistry::default();
        for model in [current, target, embedding] {
            registry
                .register(model)
                .map_err(|error| format!("register MT-014 live model: {error}"))?;
        }
        for model_id in [current_id, target_id, embedding_id] {
            registry
                .mark_loaded(model_id)
                .map_err(|error| format!("mark MT-014 model READY: {error}"))?;
        }
        let registry = Arc::new(registry);
        let catalog = ModelCatalog::from_registry_with_roles(
            registry.clone(),
            HashMap::from([
                (current_id, ModelRuntimeRole::Completion),
                (target_id, ModelRuntimeRole::Completion),
                (embedding_id, ModelRuntimeRole::Embedding),
            ]),
        );
        let drifted_catalog = ModelCatalog::from_registry_with_roles(
            registry.clone(),
            HashMap::from([
                (current_id, ModelRuntimeRole::Embedding),
                (target_id, ModelRuntimeRole::Completion),
                (embedding_id, ModelRuntimeRole::Embedding),
            ]),
        );
        let recorder = Arc::new(CapturingFlightRecorder::default());
        let router = LocalRouter::new(
            registry,
            Arc::new(ReadyRuntime::default()),
            Arc::new(ReadyRuntime::default()),
        );
        let fallback = Arc::new(CatalogLlmClient {
            profile: ModelProfile::new("mt014-native-composite-fallback".to_owned(), 4096),
            catalog: catalog.clone(),
        });
        let client = Arc::new(
            LocalModelRuntimeLlmClient::new(
                router,
                fallback,
                recorder.clone(),
                ModelProfile::new(current_id.to_string(), 4096),
            )
            .with_catalog(catalog),
        );
        let catalog_client = Arc::new(RoleDriftLlmClient {
            inner: client.clone(),
            honest_catalog: client
                .model_catalog()
                .expect("MT-014 local client has its production catalog"),
            drifted_catalog,
            expose_drift: AtomicBool::new(false),
        });
        let storage = PostgresDatabase::connect(&schema_url, 5)
            .await
            .map_err(|error| format!("connect MT-014 AppState storage: {error}"))?
            .into_arc();
        let state = AppState {
            storage,
            flight_recorder: recorder.clone(),
            diagnostics: recorder.clone(),
            llm_client: catalog_client.clone(),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
            postgres_pool: pool.clone(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("bind MT-014 production route: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("read MT-014 route address: {error}"))?;
        let server = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, model_runtime_registry::routes(state)).await {
                tracing::error!(
                    target: "handshake_core::test_harness::model_runtime_selection",
                    error = %error,
                    "MT-014 composite proof route stopped unexpectedly"
                );
            }
        });

        Ok(Self {
            managed_postgres,
            client,
            catalog_client,
            recorder,
            server,
            base_url: format!("http://{address}"),
            current_model_id: current_id.to_string(),
            current_artifact_sha256: hex::encode(current_sha256),
            target_model_id: target_id.to_string(),
            target_artifact_sha256: hex::encode(target_sha256),
            embedding_model_id: embedding_id.to_string(),
            embedding_artifact_sha256: hex::encode(embedding_sha256),
            stale_model_id: ModelId::new_v7().to_string(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn selected_model_id(&self) -> String {
        self.client.selected_model_id()
    }

    pub fn selection_event_count(&self) -> usize {
        self.recorder
            .events()
            .iter()
            .filter(|event| event.payload["fr_event"] == "FR-EVT-MODEL-SELECTION-RECORDED")
            .count()
    }

    pub fn has_native_selection_event(&self, selected_model_id: &str) -> bool {
        self.recorder.events().iter().any(|event| {
            event.payload["fr_event"] == "FR-EVT-MODEL-SELECTION-RECORDED"
                && event.payload["selected_model_id"] == selected_model_id
                && event.payload["actor"] == "native-model-runtime-panel"
                && event.payload["selection_context"]["requester"]["subsystem"] == "ui"
                && event.payload["selection_context"]["metadata"]["surface"]
                    == "native_model_runtime_panel"
        })
    }

    pub fn set_audit_failure(&self, fail: bool) {
        self.recorder.fail_writes.store(fail, Ordering::SeqCst);
    }

    pub fn drift_current_catalog_role_to_embedding(&self) {
        self.catalog_client
            .expose_drift
            .store(true, Ordering::SeqCst);
    }

    pub fn restore_current_catalog_role_to_completion(&self) {
        self.catalog_client
            .expose_drift
            .store(false, Ordering::SeqCst);
    }

    pub fn managed_postgres_is_proven(&self) -> bool {
        self.managed_postgres.proven_local_endpoint().is_some()
    }
}

impl Drop for Mt014NativeCompositeProof {
    fn drop(&mut self) {
        self.server.abort();
    }
}
