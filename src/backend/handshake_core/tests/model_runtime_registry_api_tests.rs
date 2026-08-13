//! WP-1 MT-014: real PostgreSQL + live HTTP ModelRuntime registry projection.

#[path = "knowledge_pg_support.rs"]
mod knowledge_pg_support;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use async_trait::async_trait;
use futures::stream;
use handshake_core::{
    api::{
        account_scope::ProductLocalResourceScope,
        model_runtime_registry::{
            self, ModelRuntimeRegistryProjection, ModelRuntimeRegistryRowState,
            MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID,
        },
    },
    capabilities::CapabilityRegistry,
    diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup},
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    kernel::KernelActor,
    llm::{
        local_router::{LocalModelRuntimeLlmClient, LocalRouter},
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile,
        ModelRuntimeControlAction, ModelRuntimeControlReceipt, ModelRuntimeControlRequest,
        ModelRuntimeValue, TokenUsage, MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, CaptureResult, CaptureSpec, Embedding,
        ExplicitModelRuntimeRebind, FinishReason, GenerateRequest, GeneratedToken, HookPoint,
        KvCacheHandle, LayerIndex, LoraStackHandle, ModelCapabilities, ModelCatalog, ModelId,
        ModelRegistration, ModelRegistry, ModelRegistryStore, ModelRuntime, ModelRuntimeError,
        ModelRuntimeRole, ModelRuntimeSelection, ModelRuntimeSelectionPurpose, OperatorId,
        ProviderKind, RoleBoundModelRegistration, RuntimeBinding, RuntimePerfCall,
        RuntimePerfRecorder, RuntimePerfSnapshot, Score, SteeringHookHandle, SteeringHookOps,
        SteeringVector, SteeringVectorId, SteeringVectorMeta, TokenStream,
    },
    process_ledger::{
        drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
        NoopOverflowSink, PostgresProcessLedgerStore, StopRecordOutcome,
    },
    storage::postgres::PostgresDatabase,
    swarm_orchestration::{
        model_lane::ModelLaneStore,
        resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
            ExactResourceScopeAttribution, OwnerAccountId, ResourceScope, WorkspaceScopeRef,
        },
        CloudLaneFactoryConfig, CloudLiveRuntime, CloudRuntimeBuilder, ModelInstanceId,
        ModelSessionFactory, ProductionModelSessionFactory, SpawnRequest,
    },
    workflows::{ModelSwapRequestV0_4, SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
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

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diagnostic: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct CatalogLlmClient {
    profile: ModelProfile,
    catalog: Arc<ModelCatalog>,
}

struct CountingControlClient {
    profile: ModelProfile,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LlmClient for CountingControlClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::ProviderError(
            "not used by control proof".to_owned(),
        ))
    }

    async fn control_model_runtime(
        &self,
        request: ModelRuntimeControlRequest,
    ) -> Result<ModelRuntimeControlReceipt, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ModelRuntimeControlReceipt {
            schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
            request_id: request.request_id,
            model_id: request.model_id,
            result_model_id: None,
            action: request.action,
            runtime_adapter: "counting-control-proof".to_owned(),
            quiesced: true,
            unloaded: false,
            process_stop_committed: false,
            registry_updated: false,
            selection_rebound: false,
            catalog_revision: None,
            reconciliation_required: false,
            reconciliation_reason: None,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl CapturingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("event recorder lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.lock().expect("event recorder lock").push(event);
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

struct FailingRecorder;

#[async_trait]
impl FlightRecorder for FailingRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Err(RecorderError::SinkError(
            "MT-014 forced audit failure".to_owned(),
        ))
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

/// Minimal `SteeringHookOps` that exposes a fixed applied (active) steering set
/// for the Section 10.13.1 "Steering vectors active" projection field. Only
/// `list_active` is exercised by `inspect_model_runtime`; the mutation surface
/// fails typed because this double hosts no real activation store.
struct StaticSteeringOps {
    active: Vec<SteeringVectorMeta>,
}

#[async_trait]
impl SteeringHookOps for StaticSteeringOps {
    async fn capture(&self, _spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "static steering telemetry double does not capture".to_owned(),
        ))
    }

    async fn register_vector(
        &self,
        _vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "static steering telemetry double does not register".to_owned(),
        ))
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        self.active.clone()
    }

    fn list_active(&self) -> Vec<SteeringVectorMeta> {
        self.active.clone()
    }

    async fn set_active(&self, _ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "static steering telemetry double does not mutate activation".to_owned(),
        ))
    }

    async fn unregister(&self, _id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "static steering telemetry double does not unregister".to_owned(),
        ))
    }
}

struct ReadyRuntime {
    capabilities: ModelCapabilities,
    perf: Option<RuntimePerfSnapshot>,
    engine_internals: Option<Value>,
    active_steering: Vec<SteeringVectorMeta>,
}

impl Default for ReadyRuntime {
    fn default() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            perf: None,
            engine_internals: None,
            active_steering: Vec::new(),
        }
    }
}

impl ReadyRuntime {
    /// A runtime double that reports real Section 10.13 live telemetry: a perf
    /// snapshot derived from the product `RuntimePerfRecorder`, an engine
    /// internals document, and an applied steering set.
    fn with_live_telemetry(
        perf: RuntimePerfSnapshot,
        engine_internals: Value,
        active_steering: Vec<SteeringVectorMeta>,
    ) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            perf: Some(perf),
            engine_internals: Some(engine_internals),
            active_steering,
        }
    }
}

#[async_trait]
impl ModelRuntime for ReadyRuntime {
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
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("mt014-proof-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("mt014-proof-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        if self.active_steering.is_empty() {
            Ok(SteeringHookHandle::new("mt014-proof-steering"))
        } else {
            Ok(SteeringHookHandle::with_ops(
                "mt014-proof-steering",
                Arc::new(StaticSteeringOps {
                    active: self.active_steering.clone(),
                }),
            ))
        }
    }

    fn perf_snapshot(&self, _id: ModelId) -> Result<RuntimePerfSnapshot, ModelRuntimeError> {
        self.perf.clone().ok_or_else(|| {
            // Match the object-safe trait default so a runtime that records no
            // activity fails typed instead of fabricating telemetry.
            ModelRuntimeError::CapabilityNotSupported {
                capability: "runtime_perf_snapshot".to_owned(),
                adapter: self.adapter_name().to_owned(),
            }
        })
    }

    fn engine_internals(&self, _id: ModelId) -> Result<Value, ModelRuntimeError> {
        self.engine_internals
            .clone()
            .ok_or_else(|| ModelRuntimeError::CapabilityNotSupported {
                capability: "engine_internals".to_owned(),
                adapter: self.adapter_name().to_owned(),
            })
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

struct CapturingSelectionLlmClient {
    profile: ModelProfile,
    catalog: Arc<ModelCatalog>,
    selected_model_id: Mutex<String>,
    swap_requests: Mutex<Vec<ModelSwapRequestV0_4>>,
}

impl CapturingSelectionLlmClient {
    fn new(selected_model_id: String, catalog: Arc<ModelCatalog>) -> Self {
        Self {
            profile: ModelProfile::new(selected_model_id.clone(), 4096),
            catalog,
            selected_model_id: Mutex::new(selected_model_id),
            swap_requests: Mutex::new(Vec::new()),
        }
    }

    fn swap_requests(&self) -> Vec<ModelSwapRequestV0_4> {
        self.swap_requests
            .lock()
            .expect("selection requests lock")
            .clone()
    }
}

#[async_trait]
impl LlmClient for CapturingSelectionLlmClient {
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

    async fn swap_model(&self, request: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        let mut selected = self.selected_model_id.lock().expect("selected model lock");
        if request.current_model_id != *selected {
            return Err(LlmError::ProviderError(format!(
                "stale selection {}; current selection is {}",
                request.current_model_id, *selected
            )));
        }
        let target_is_ready_and_selectable = self.catalog.list().into_iter().any(|entry| {
            entry.model_id == request.target_model_id && entry.ready && entry.default_selectable
        });
        if !target_is_ready_and_selectable {
            return Err(LlmError::ProviderError(format!(
                "target model {} is not READY",
                request.target_model_id
            )));
        }
        *selected = request.target_model_id.clone();
        self.swap_requests
            .lock()
            .expect("selection requests lock")
            .push(request);
        Ok(())
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    fn selected_model_id(&self) -> String {
        self.selected_model_id
            .lock()
            .expect("selected model lock")
            .clone()
    }

    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        Some(self.catalog.clone())
    }
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

fn registration(
    model_id: ModelId,
    sha256: [u8; 32],
    runtime_binding: RuntimeBinding,
    label: &str,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from(format!("registry-proof/{label}.safetensors")),
        sha256,
        runtime_binding,
        declared_capabilities: ModelCapabilities::default(),
        base_model_tag: BaseModelTag::new(label),
        registered_at_utc: chrono::Utc::now(),
        registered_by: OperatorId::new("mt014-registry-api-proof"),
        provider: ProviderKind::Local,
    }
}

async fn app_state_for(schema_url: &str, catalog: Arc<ModelCatalog>) -> AppState {
    let selected_model_id = catalog
        .list()
        .into_iter()
        .find(|entry| entry.ready)
        .map(|entry| entry.model_id)
        .unwrap_or_else(|| "mt014-registry-api-proof".to_owned());
    app_state_for_client(
        schema_url,
        Arc::new(CatalogLlmClient {
            profile: ModelProfile::new(selected_model_id, 4096),
            catalog,
        }),
    )
    .await
}

async fn app_state_for_client(schema_url: &str, llm_client: Arc<dyn LlmClient>) -> AppState {
    let storage = PostgresDatabase::connect(schema_url, 5)
        .await
        .expect("connect AppState storage to isolated schema")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool to isolated schema");
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client,
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

async fn start_server(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ModelRuntime registry proof server");
    let address = listener
        .local_addr()
        .expect("registry proof server address");
    let server = tokio::spawn(async move {
        axum::serve(listener, model_runtime_registry::routes(state))
            .await
            .expect("serve ModelRuntime registry route");
    });
    (format!("http://{address}"), server)
}

async fn start_scoped_server(
    state: AppState,
    scope: ExactResourceScopeAttribution,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind exactly scoped ModelRuntime registry proof server");
    let address = listener
        .local_addr()
        .expect("scoped registry proof server address");
    let server_scope =
        ProductLocalResourceScope::from_exact(scope).expect("valid server-owned exact scope");
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            model_runtime_registry::routes(state).layer(axum::Extension(server_scope)),
        )
        .await
        .expect("serve exactly scoped ModelRuntime registry route");
    });
    (format!("http://{address}"), server)
}

fn pg_required(
    value: Option<knowledge_pg_support::KnowledgePg>,
) -> knowledge_pg_support::KnowledgePg {
    value.unwrap_or_else(|| {
        panic!(
            "PostgreSQL unavailable: MT-014 registry API proof requires the real Handshake-managed PostgreSQL authority"
        )
    })
}

#[tokio::test]
async fn mt023_runtime_control_requires_server_scope_before_client_side_effect() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let calls = Arc::new(AtomicUsize::new(0));
    let client: Arc<dyn LlmClient> = Arc::new(CountingControlClient {
        profile: ModelProfile::new("control-scope-proof".to_owned(), 4096),
        calls: calls.clone(),
    });
    let state = app_state_for_client(&pg.schema_url, client).await;
    let exact = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("control-scope-{}", Uuid::now_v7()))
            .expect("valid control workspace"),
    };
    let request = json!({
        "schema_version": MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
        "request_id": Uuid::now_v7(),
        "model_id": "control-scope-proof",
        "action": { "action": "quiesce" },
        "timeout_ms": 1000,
        "expected_catalog_revision": null,
        "expected_selection_revision": null
    });
    let (missing_base, missing_server) = start_server(state.clone()).await;
    let missing = reqwest::Client::new()
        .post(format!("{missing_base}/model-runtime/control"))
        .json(&request)
        .send()
        .await
        .expect("missing-scope control request");
    assert_eq!(missing.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    missing_server.abort();

    let (scoped_base, scoped_server) = start_scoped_server(state, exact).await;
    let mismatched = reqwest::Client::new()
        .post(format!("{scoped_base}/model-runtime/control"))
        .header(
            "x-handshake-owner-account",
            OwnerAccountId::mint().to_string(),
        )
        .json(&request)
        .send()
        .await
        .expect("mismatched-assertion control request");
    assert_eq!(mismatched.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let authorized = reqwest::Client::new()
        .post(format!("{scoped_base}/model-runtime/control"))
        .json(&request)
        .send()
        .await
        .expect("server-scoped control request");
    assert_eq!(authorized.status(), reqwest::StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    scoped_server.abort();
}

#[tokio::test]
async fn mt023_process_ownership_route_enforces_exact_account_workspace_scope_without_leaks() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect process-ownership scope proof store");
    let owner = OwnerAccountId::mint();
    let other_owner = OwnerAccountId::mint();
    let actor = ActorPrincipalId::mint();
    let session = AuthenticatedSessionRef::mint();
    let access_space = AccessSpaceRef::mint();
    let workspace = format!("process-private-{}", Uuid::now_v7());
    let exact_scope = ExactResourceScopeAttribution {
        owner_account_id: owner,
        actor_principal_id: actor,
        authenticated_session_id: session,
        access_space_id: access_space,
        workspace_id: WorkspaceScopeRef::new(&workspace).expect("valid process workspace"),
    };
    let process_uuid = Uuid::now_v7();
    let artifact_sha = hex::encode([0xabu8; 32]);
    let metadata = json!({
        "owner_account_id": owner,
        "actor_principal_id": actor,
        "authenticated_session_id": session,
        "access_space_id": access_space,
        "workspace_id": workspace,
    });
    sqlx::query(
        "INSERT INTO kernel_process_lifecycle \
         (process_uuid, engine_kind, started_at, owner_role, owner_wp, \
          model_artifact_sha256, metadata_jsonb) \
         VALUES ($1, 'llama_cpp', now(), 'KERNEL_BUILDER', 'MT-023', $2, $3)",
    )
    .bind(process_uuid)
    .bind(&artifact_sha)
    .bind(metadata)
    .execute(&pool)
    .await
    .expect("insert exactly scoped process ownership row");

    let malformed_uuid = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO kernel_process_lifecycle \
         (process_uuid, engine_kind, started_at, owner_role, owner_wp, metadata_jsonb) \
         VALUES ($1, 'llama_cpp', now(), 'KERNEL_BUILDER', 'MT-023', $2)",
    )
    .bind(malformed_uuid)
    .bind(json!({
        "owner_account_id": owner,
        "workspace_id": workspace,
        "restricted_identifier": "must-never-leak",
    }))
    .execute(&pool)
    .await
    .expect("insert partial-attribution decoy row");

    let catalog = ModelCatalog::from_registry(Arc::new(ModelRegistry::default()));
    let state = app_state_for(&pg.schema_url, catalog).await;
    let (base_url, server) = start_scoped_server(state.clone(), exact_scope).await;
    let http = reqwest::Client::new();
    let route = |id| format!("{base_url}/model-runtime/process-ownership/{id}");

    let owner_response = http
        .get(route(process_uuid))
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", &workspace)
        .send()
        .await
        .expect("GET owner process ownership record");
    assert_eq!(owner_response.status(), reqwest::StatusCode::OK);
    let owner_record: Value = owner_response.json().await.expect("owner process JSON");
    assert_eq!(
        owner_record.get("process_uuid").and_then(Value::as_str),
        Some(process_uuid.to_string().as_str())
    );

    let assertion_denied = [
        http.get(route(process_uuid))
            .header("x-handshake-owner-account", other_owner.to_string())
            .header("x-handshake-workspace", &workspace)
            .send()
            .await
            .expect("GET cross-account process record"),
        http.get(route(process_uuid))
            .header("x-handshake-owner-account", owner.to_string())
            .header("x-handshake-workspace", format!("{workspace}-other"))
            .send()
            .await
            .expect("GET wrong-workspace process record"),
    ];
    for response in assertion_denied {
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
        let body = response.text().await.expect("denial response body");
        assert!(!body.contains(&process_uuid.to_string()));
        assert!(!body.contains(&malformed_uuid.to_string()));
        assert!(!body.contains(&artifact_sha));
        assert!(!body.contains("must-never-leak"));
    }

    let malformed = http
        .get(route(malformed_uuid))
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", &workspace)
        .send()
        .await
        .expect("GET partial-attribution process decoy");
    assert_eq!(malformed.status(), reqwest::StatusCode::NOT_FOUND);
    let malformed_body = malformed.text().await.expect("malformed denial body");
    assert!(!malformed_body.contains(&malformed_uuid.to_string()));
    assert!(!malformed_body.contains("must-never-leak"));

    let without_assertion_headers = http
        .get(route(process_uuid))
        .send()
        .await
        .expect("GET process record using only server-owned exact scope");
    assert_eq!(without_assertion_headers.status(), reqwest::StatusCode::OK);
    let without_assertion_headers_body = without_assertion_headers
        .text()
        .await
        .expect("server-scoped body without optional assertion headers");
    assert!(without_assertion_headers_body.contains(&process_uuid.to_string()));
    assert!(without_assertion_headers_body.contains(&artifact_sha));

    let missing_workspace = http
        .get(route(process_uuid))
        .header("x-handshake-owner-account", owner.to_string())
        .send()
        .await
        .expect("GET process record without optional workspace assertion");
    assert_eq!(missing_workspace.status(), reqwest::StatusCode::OK);
    let missing_workspace_body = missing_workspace
        .text()
        .await
        .expect("server-scoped body without optional workspace assertion");
    assert!(missing_workspace_body.contains(&process_uuid.to_string()));

    let (unscoped_base, unscoped_server) = start_server(state).await;
    let missing_server_scope = http
        .get(format!(
            "{unscoped_base}/model-runtime/process-ownership/{process_uuid}"
        ))
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", &workspace)
        .send()
        .await
        .expect("GET process record without server scope authority");
    assert_eq!(
        missing_server_scope.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let missing_server_body = missing_server_scope
        .text()
        .await
        .expect("missing server scope body");
    assert!(!missing_server_body.contains(&process_uuid.to_string()));
    assert!(!missing_server_body.contains(&artifact_sha));
    unscoped_server.abort();
    server.abort();
}

#[tokio::test]
async fn mt023_registry_projection_resolves_same_sha_processes_inside_exact_scope() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect same-SHA projection collision proof store");

    let owner_a = OwnerAccountId::mint();
    let owner_b = OwnerAccountId::mint();
    let actor_a = ActorPrincipalId::mint();
    let actor_b = ActorPrincipalId::mint();
    let session_a = AuthenticatedSessionRef::mint();
    let session_b = AuthenticatedSessionRef::mint();
    let access_a = AccessSpaceRef::mint();
    let access_b = AccessSpaceRef::mint();
    let workspace_a = format!("registry-collision-a-{}", Uuid::now_v7());
    let workspace_b = format!("registry-collision-b-{}", Uuid::now_v7());
    let exact_a = ExactResourceScopeAttribution {
        owner_account_id: owner_a,
        actor_principal_id: actor_a,
        authenticated_session_id: session_a,
        access_space_id: access_a,
        workspace_id: WorkspaceScopeRef::new(&workspace_a).expect("valid owner-A workspace"),
    };
    let exact_b = ExactResourceScopeAttribution {
        owner_account_id: owner_b,
        actor_principal_id: actor_b,
        authenticated_session_id: session_b,
        access_space_id: access_b,
        workspace_id: WorkspaceScopeRef::new(&workspace_b).expect("valid owner-B workspace"),
    };
    let sha256 = [0xacu8; 32];
    let artifact_sha = hex::encode(sha256);
    let model_id = ModelId::new_v7();
    let durable_registration = registration(
        model_id,
        sha256,
        RuntimeBinding::LlamaCpp,
        "MT-023 Scoped Process Collision",
    );
    let store = ModelRegistryStore::new_scoped(
        pool.clone(),
        ResourceScope::new(owner_a, actor_a)
            .with_session(session_a)
            .with_access_space(access_a)
            .with_workspace(WorkspaceScopeRef::new(&workspace_a).expect("valid owner-A workspace")),
    );
    store
        .persist_and_read_back(&durable_registration)
        .await
        .expect("persist owner-A durable registry row");
    store
        .ensure_active_defaults(&[(ModelRuntimeSelectionPurpose::ApplicationDefault, sha256)])
        .await
        .expect("persist owner-A default selection");

    let process_a = Uuid::now_v7();
    let process_b = Uuid::now_v7();
    let wrong_workspace_process = Uuid::now_v7();
    let wrong_actor_process = Uuid::now_v7();
    let wrong_session_process = Uuid::now_v7();
    let wrong_access_process = Uuid::now_v7();
    let malformed_process = Uuid::now_v7();
    let exact_metadata = |owner, actor, session, access, workspace: &str| {
        json!({
            "owner_account_id": owner,
            "actor_principal_id": actor,
            "authenticated_session_id": session,
            "access_space_id": access,
            "workspace_id": workspace,
        })
    };
    for (process_uuid, seconds_ago, metadata) in [
        (
            process_a,
            8_i64,
            exact_metadata(owner_a, actor_a, session_a, access_a, &workspace_a),
        ),
        (
            process_b,
            7_i64,
            exact_metadata(owner_b, actor_b, session_b, access_b, &workspace_b),
        ),
        (
            wrong_workspace_process,
            6_i64,
            exact_metadata(
                owner_a,
                actor_a,
                session_a,
                access_a,
                &format!("{workspace_a}-wrong"),
            ),
        ),
        (
            wrong_actor_process,
            5_i64,
            exact_metadata(
                owner_a,
                ActorPrincipalId::mint(),
                session_a,
                access_a,
                &workspace_a,
            ),
        ),
        (
            wrong_session_process,
            4_i64,
            exact_metadata(
                owner_a,
                actor_a,
                AuthenticatedSessionRef::mint(),
                access_a,
                &workspace_a,
            ),
        ),
        (
            wrong_access_process,
            3_i64,
            exact_metadata(
                owner_a,
                actor_a,
                session_a,
                AccessSpaceRef::mint(),
                &workspace_a,
            ),
        ),
        (
            malformed_process,
            1_i64,
            json!({
                "owner_account_id": owner_a,
                "workspace_id": workspace_a,
                "restricted_identifier": "malformed-newest-must-not-shadow",
            }),
        ),
    ] {
        sqlx::query(
            "INSERT INTO kernel_process_lifecycle \
             (process_uuid, engine_kind, started_at, owner_role, owner_wp, \
              model_artifact_sha256, metadata_jsonb) \
             VALUES ($1, 'llama_cpp', now() - ($2 * interval '1 second'), \
                     'KERNEL_BUILDER', 'MT-023', $3, $4)",
        )
        .bind(process_uuid)
        .bind(seconds_ago)
        .bind(&artifact_sha)
        .bind(metadata)
        .execute(&pool)
        .await
        .expect("insert same-artifact process collision candidate");
    }

    let mut registry = ModelRegistry::default();
    registry
        .register(durable_registration)
        .expect("register owner-A current-boot model");
    registry
        .mark_loaded(model_id)
        .expect("mark owner-A current-boot model READY");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_scoped_server(state.clone(), exact_a).await;
    let (base_url_b, server_b) = start_scoped_server(state, exact_b).await;
    let http = reqwest::Client::new();

    let projection_response = http
        .get(format!("{base_url}/model-runtime/registry"))
        .header("x-handshake-owner-account", owner_a.to_string())
        .header("x-handshake-workspace", &workspace_a)
        .send()
        .await
        .expect("GET owner-A scoped registry projection");
    assert_eq!(projection_response.status(), reqwest::StatusCode::OK);
    let projection_body = projection_response
        .text()
        .await
        .expect("owner-A projection body");
    let projection: ModelRuntimeRegistryProjection =
        serde_json::from_str(&projection_body).expect("owner-A projection JSON");
    assert_eq!(projection.rows.len(), 1);
    assert_eq!(
        projection.rows[0].process_ownership_ledger_link,
        ModelRuntimeValue::available(format!(
            "process-ownership-ledger://process/{process_a}"
        )),
        "the newest foreign, wrong-workspace, and malformed rows must not shadow owner A's exact authorized process"
    );
    for hidden in [
        process_b,
        wrong_workspace_process,
        wrong_actor_process,
        wrong_session_process,
        wrong_access_process,
        malformed_process,
    ] {
        assert!(
            !projection_body.contains(&hidden.to_string()),
            "the projection leaked a denied process UUID {hidden}"
        );
    }
    assert!(!projection_body.contains("malformed-newest-must-not-shadow"));

    let detail_route =
        |process_uuid| format!("{base_url}/model-runtime/process-ownership/{process_uuid}");
    let detail_a = http
        .get(detail_route(process_a))
        .header("x-handshake-owner-account", owner_a.to_string())
        .header("x-handshake-workspace", &workspace_a)
        .send()
        .await
        .expect("GET owner-A process detail");
    assert_eq!(detail_a.status(), reqwest::StatusCode::OK);
    let detail_a_body = detail_a.text().await.expect("owner-A detail body");
    assert!(detail_a_body.contains(&process_a.to_string()));

    let detail_b = http
        .get(format!(
            "{base_url_b}/model-runtime/process-ownership/{process_b}"
        ))
        .header("x-handshake-owner-account", owner_b.to_string())
        .header("x-handshake-workspace", &workspace_b)
        .send()
        .await
        .expect("GET owner-B process detail");
    assert_eq!(detail_b.status(), reqwest::StatusCode::OK);
    let detail_b_body = detail_b.text().await.expect("owner-B detail body");
    assert!(detail_b_body.contains(&process_b.to_string()));
    assert!(!detail_b_body.contains(&process_a.to_string()));

    for hidden in [
        process_b,
        wrong_workspace_process,
        wrong_actor_process,
        wrong_session_process,
        wrong_access_process,
        malformed_process,
    ] {
        let denied = http
            .get(detail_route(hidden))
            .header("x-handshake-owner-account", owner_a.to_string())
            .header("x-handshake-workspace", &workspace_a)
            .send()
            .await
            .expect("GET denied owner-A process detail");
        assert_eq!(denied.status(), reqwest::StatusCode::NOT_FOUND);
        let body = denied.text().await.expect("denied detail body");
        assert!(!body.contains(&hidden.to_string()));
        assert!(!body.contains(&artifact_sha));
        assert!(!body.contains("malformed-newest-must-not-shadow"));
    }

    server.abort();
    server_b.abort();
}

#[tokio::test]
async fn mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect registry proof store");
    let store = ModelRegistryStore::new(pool.clone());

    let live_sha = [0x41; 32];
    let alternate_ready_sha = [0x52; 32];
    let dormant_sha = [0x92; 32];
    let first_live_id = ModelId::new_v7();
    let first_alternate_ready_id = ModelId::new_v7();
    let stale_dormant_id = ModelId::new_v7();
    let initial_live = registration(
        first_live_id,
        live_sha,
        RuntimeBinding::LlamaCpp,
        "Candle Registry Proof",
    );
    let dormant = registration(
        stale_dormant_id,
        dormant_sha,
        RuntimeBinding::LlamaCpp,
        "Dormant Registry Proof",
    );
    let initial_alternate_ready = registration(
        first_alternate_ready_id,
        alternate_ready_sha,
        RuntimeBinding::LlamaCpp,
        "Llama Ready Alternative",
    );
    store
        .persist_boot_set_and_read_back(&[initial_live, initial_alternate_ready, dormant])
        .await
        .expect("persist initial real PostgreSQL registry rows");
    store
        .ensure_active_defaults(&[(ModelRuntimeSelectionPurpose::ApplicationDefault, live_sha)])
        .await
        .expect("persist PostgreSQL-authoritative application/default");

    let target_selection = ModelRuntimeSelection {
        artifact_sha256: live_sha,
        runtime_binding: RuntimeBinding::Candle,
        runtime_role: ModelRuntimeRole::Completion,
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
    };
    let rebound = store
        .rebind_selection_for_tests(
            &target_selection,
            ExplicitModelRuntimeRebind::new(
                KernelActor::Operator("mt014-control-panel-proof".to_owned()),
                "prove the native panel surfaces the current audited selection revision",
                1,
            )
            .expect("valid explicit rebind evidence"),
        )
        .await
        .expect("audited compare-and-swap rebind");
    assert_eq!(rebound.selection_revision, 2);

    let current_live_id = ModelId::new_v7();
    let current_live = registration(
        current_live_id,
        live_sha,
        RuntimeBinding::Candle,
        "Candle Registry Proof",
    );
    store
        .persist_and_read_back(&current_live)
        .await
        .expect("record the current successful live observation");

    let current_alternate_ready_id = ModelId::new_v7();
    let current_alternate_ready = registration(
        current_alternate_ready_id,
        alternate_ready_sha,
        RuntimeBinding::LlamaCpp,
        "Llama Ready Alternative",
    );
    store
        .persist_and_read_back(&current_alternate_ready)
        .await
        .expect("record the alternative READY model observation");

    let mut registry = ModelRegistry::default();
    registry
        .register(current_live)
        .expect("register current live model in the production catalog shape");
    registry
        .register(current_alternate_ready)
        .expect("register alternative live model in the production catalog shape");
    registry
        .mark_loaded(current_live_id)
        .expect("mark the current model READY");
    registry
        .mark_loaded(current_alternate_ready_id)
        .expect("mark the alternative model READY");
    let catalog = ModelCatalog::from_registry(Arc::new(registry));

    let state = app_state_for(&pg.schema_url, catalog).await;
    let (base_url, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET real ModelRuntime registry route");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await.expect("registry response body");
    let projection: ModelRuntimeRegistryProjection =
        serde_json::from_str(&body).expect("deserialize registry projection");
    assert_eq!(
        projection.schema_id,
        MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.rows.len(), 3);

    let live_hash = hex::encode(live_sha);
    let live = projection
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == live_hash)
        .expect("live durable row");
    let current_live_id_string = current_live_id.to_string();
    assert_eq!(live.runtime_state, ModelRuntimeRegistryRowState::Live);
    assert!(
        live.selected,
        "the PostgreSQL application/default row is selected"
    );
    assert_eq!(
        live.active_purposes,
        vec![ModelRuntimeSelectionPurpose::ApplicationDefault]
    );
    assert_eq!(live.active_selection_revision, Some(1));
    assert_eq!(
        live.live_model_id.as_deref(),
        Some(current_live_id_string.as_str())
    );
    assert_eq!(live.selected_adapter, "candle");
    assert_eq!(live.selection_revision, 2);
    assert_eq!(live.display_label, "Candle Registry Proof");
    assert_eq!(live.artifact_locator, format!("sha256:{live_hash}"));
    assert_eq!(
        live.selection_audit_event_ref,
        format!(
            "eventledger://kernel/{}",
            rebound.selection_updated_event_id
        )
    );
    assert!(matches!(
        &live.canonical_artifact_path,
        ModelRuntimeValue::Unavailable { reason }
            if reason.contains("could not be canonicalized")
    ));
    assert!(matches!(
        &live.kv_cache,
        ModelRuntimeValue::Unavailable { reason }
            if reason.contains("does not expose local ModelRuntime internals")
    ));
    assert!(matches!(
        &live.last_call_age_seconds,
        ModelRuntimeValue::Unavailable { reason }
            if reason.contains("last-call time is unavailable")
    ));
    assert!(!live.quiesce_action.enabled);
    assert!(live.quiesce_action.reason.is_some());
    assert!(!live.unload_action.enabled);
    assert!(live.unload_action.reason.is_some());
    assert!(!live.compatible_adapter_swap_action.enabled);
    assert!(live.compatible_adapter_swap_action.reason.is_some());
    assert!(!live.inspect_engine_internals_action.enabled);
    assert!(live.inspect_engine_internals_action.reason.is_some());

    let alternate_ready_hash = hex::encode(alternate_ready_sha);
    let alternate_ready = projection
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == alternate_ready_hash)
        .expect("alternative READY durable row");
    assert_eq!(
        alternate_ready.runtime_state,
        ModelRuntimeRegistryRowState::Live
    );
    assert!(
        !alternate_ready.selected,
        "only the audited default model is selected"
    );
    assert_eq!(
        alternate_ready.live_model_id.as_deref(),
        Some(current_alternate_ready_id.to_string().as_str())
    );
    assert_eq!(alternate_ready.selected_adapter, "llama_cpp");
    assert_eq!(alternate_ready.selection_revision, 1);
    assert_eq!(alternate_ready.display_label, "Llama Ready Alternative");

    let dormant_hash = hex::encode(dormant_sha);
    let dormant = projection
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == dormant_hash)
        .expect("dormant durable row");
    assert_eq!(dormant.runtime_state, ModelRuntimeRegistryRowState::Dormant);
    assert!(!dormant.selected, "a dormant row can never be selected");
    assert_eq!(dormant.live_model_id, None);
    assert_eq!(dormant.selected_adapter, "llama_cpp");
    assert_eq!(dormant.selection_revision, 1);
    assert!(!dormant.quiesce_action.enabled);
    assert!(dormant
        .quiesce_action
        .reason
        .as_deref()
        .is_some_and(|reason| !reason.is_empty()));
    assert!(
        !body.contains(&stale_dormant_id.to_string()),
        "a dormant row must not serialize its last-observed runtime UUID as loaded state"
    );
    assert!(
        !body.contains(&first_live_id.to_string()),
        "the projection must join current readiness by SHA-256, never by a stale boot UUID"
    );
    assert!(
        !body.contains(&first_alternate_ready_id.to_string()),
        "the alternative READY row must also hide its stale boot UUID"
    );

    if let Ok(proof_nonce) = std::env::var("HANDSHAKE_MT014_PROOF_NONCE") {
        let artifact_root = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
            .expect("HANDSHAKE_ARTIFACTS_DIR is required when publishing MT-014 proof artifacts");
        let artifact_root = std::fs::canonicalize(artifact_root)
            .expect("HANDSHAKE_ARTIFACTS_DIR must resolve to an existing directory");
        let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
            .expect("backend crate manifest directory must resolve");
        let worktree_root = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::parent)
            .expect("backend crate must live below the worktree src directory");
        let expected_root = std::fs::canonicalize(
            worktree_root
                .parent()
                .expect("worktree must have a parent")
                .join("Handshake_Artifacts"),
        )
        .expect("canonical sibling Handshake_Artifacts directory must exist");
        assert_eq!(
            artifact_root, expected_root,
            "MT-014 proof artifacts must use the canonical sibling Handshake_Artifacts root"
        );
        let path = artifact_root
            .join("handshake-test")
            .join("wp1-final-audit")
            .join("mt014-model-runtime-registry-projection.json");
        if let Ok(configured_path) = std::env::var("HANDSHAKE_MT014_PROJECTION_ARTIFACT") {
            let configured_path = PathBuf::from(configured_path);
            let configured_parent = std::fs::canonicalize(
                configured_path
                    .parent()
                    .expect("configured MT-014 projection must have a parent directory"),
            )
            .expect("configured MT-014 projection parent must resolve");
            let configured_path = configured_parent.join(
                configured_path
                    .file_name()
                    .expect("configured MT-014 projection must have a file name"),
            );
            assert_eq!(
                configured_path, path,
                "HANDSHAKE_MT014_PROJECTION_ARTIFACT must name the canonical MT-014 projection"
            );
        }
        let value: Value = serde_json::from_str(&body).expect("projection JSON value");
        let pretty = serde_json::to_vec_pretty(&value).expect("pretty registry projection");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create projection artifact parent");
        }
        let artifact_sha256 = hex::encode(Sha256::digest(&pretty));
        std::fs::write(&path, &pretty).expect("write real registry projection artifact");
        let producer_completed_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_millis() as u64;
        std::fs::write(
            path.with_extension("provenance.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_id": "hsk.mt014_model_runtime_projection_provenance@1",
                "proof_nonce": proof_nonce,
                "projection_schema_id": projection.schema_id,
                "artifact_sha256": artifact_sha256,
                "producer_test_id": "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256",
                "producer_status": "passed_all_backend_assertions",
                "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
            }))
            .expect("serialize MT-014 projection provenance"),
        )
        .expect("write MT-014 projection provenance");
        println!("REAL_MODEL_RUNTIME_PROJECTION={}", path.display());
    }
    server.abort();
}

#[tokio::test]
async fn mt014_selection_post_prevalidates_then_returns_audited_projection() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect selection route proof store");
    let store = ModelRegistryStore::new(pool.clone());
    let current_id = ModelId::new_v7();
    let target_id = ModelId::new_v7();
    let registrations = vec![
        registration(
            current_id,
            [0x61; 32],
            RuntimeBinding::Candle,
            "Current READY Model",
        ),
        registration(
            target_id,
            [0x62; 32],
            RuntimeBinding::LlamaCpp,
            "Target READY Model",
        ),
    ];
    store
        .persist_boot_set_and_read_back(&registrations)
        .await
        .expect("persist both READY selection-route rows");
    store
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            registrations[0].sha256,
        )])
        .await
        .expect("persist current application/default before route exposure");
    // The audited projection surfaces the target model's ProcessOwnershipLedger
    // link. Record a real kernel_process_lifecycle row for the target artifact
    // so the projection's SHA-256 join populates process_ownership_ledger_link
    // (the projection reads process ownership from PostgreSQL, not the catalog).
    sqlx::query(
        "INSERT INTO kernel_process_lifecycle \
         (process_uuid, engine_kind, started_at, owner_role, model_artifact_sha256) \
         VALUES ($1::uuid, 'candle', now(), 'KERNEL_BUILDER', $2)",
    )
    .bind(target_id.as_uuid())
    .bind(hex::encode([0x62u8; 32]))
    .execute(&pool)
    .await
    .expect("record target ProcessOwnershipLedger lifecycle row");

    let mut registry = ModelRegistry::default();
    for registration in registrations {
        registry
            .register(registration)
            .expect("register selection-route model");
    }
    registry
        .mark_loaded(current_id)
        .expect("mark current model READY");
    registry
        .mark_loaded(target_id)
        .expect("mark target model READY");
    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry(registry.clone());
    let recorder = Arc::new(CapturingRecorder::default());
    let router = LocalRouter::new(
        registry,
        Arc::new(ReadyRuntime::default()),
        Arc::new(ReadyRuntime::default()),
    );
    let fallback = Arc::new(CatalogLlmClient {
        profile: ModelProfile::new("mt014-proof-fallback".to_owned(), 4096),
        catalog: catalog.clone(),
    });
    let client = Arc::new(
        LocalModelRuntimeLlmClient::new(
            router,
            fallback,
            recorder.clone(),
            ModelProfile::new(current_id.to_string(), 4096),
        )
        .with_catalog(catalog)
        .with_durable_application_selection(store.clone(), 1),
    );
    let state = app_state_for_client(&pg.schema_url, client.clone()).await;
    let (base_url, server) = start_server(state).await;
    let reason = "operator selected the alternate READY runtime";
    let response = reqwest::Client::new()
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&json!({
            "target_model_id": target_id.to_string(),
            "actor": "native-model-runtime-panel",
            "reason": reason,
        }))
        .send()
        .await
        .expect("POST real ModelRuntime selection route");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let projection: ModelRuntimeRegistryProjection = response
        .json()
        .await
        .expect("deserialize selection response projection");
    let target_row = projection
        .rows
        .iter()
        .find(|row| row.live_model_id.as_deref() == Some(target_id.to_string().as_str()))
        .expect("target READY projection row");
    assert!(target_row.selected, "target row becomes the active default");
    assert_eq!(target_row.active_selection_revision, Some(2));
    assert!(matches!(
        &target_row.kv_cache,
        ModelRuntimeValue::Available { value }
            if value.bytes_used == 0
                && value.bytes_capacity == 0
                && matches!(
                    &value.prefix_cache_hit_rate,
                    ModelRuntimeValue::Unavailable { .. }
                )
    ));
    assert!(matches!(
        &target_row.lora_stack,
        ModelRuntimeValue::Available { value } if value.is_empty()
    ));
    // The runtime steering handle now surfaces the applied (active) vector set.
    // This default runtime double hosts no applied vectors, so the field is
    // truthfully Available-and-empty rather than the prior "not exposed" stub.
    assert!(matches!(
        &target_row.active_steering,
        ModelRuntimeValue::Available { value } if value.is_empty()
    ));
    assert!(matches!(
        &target_row.process_ownership_ledger_link,
        ModelRuntimeValue::Available { value }
            if value.ends_with(&target_id.to_string())
    ));
    // This default runtime double records no generation activity, so perf is a
    // typed unavailable derived from the object-safe perf_snapshot boundary,
    // never a fabricated zero.
    assert!(matches!(
        &target_row.tokens_per_second,
        ModelRuntimeValue::Unavailable { reason }
            if reason.contains("runtime_perf_snapshot")
    ));
    assert!(projection.rows.iter().any(|row| {
        row.live_model_id.as_deref() == Some(current_id.to_string().as_str()) && !row.selected
    }));
    assert!(projection
        .selection_receipt_ref
        .as_deref()
        .is_some_and(|receipt| { receipt.starts_with("model-runtime-selection://receipt/") }));

    assert_eq!(client.selected_model_id(), target_id.to_string());
    let committed = ModelRegistryStore::new(pool)
        .list_active_selections()
        .await
        .expect("fresh store recovers route selection after simulated restart");
    let application = committed
        .iter()
        .find(|row| row.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application/default remains durable");
    assert_eq!(application.artifact_sha256, [0x62; 32]);
    assert_eq!(application.selection_revision, 2);
    assert_ne!(
        application.selection_created_event_id,
        application.selection_updated_event_id
    );
    assert!(recorder.events().is_empty());
    server.abort();
}

#[tokio::test]
async fn mt014_selection_post_rejects_embedding_role_before_swap() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect embedding-role selection proof store");
    let store = ModelRegistryStore::new(pool);
    let current_id = ModelId::new_v7();
    let embedding_id = ModelId::new_v7();
    let current = registration(
        current_id,
        [0x71; 32],
        RuntimeBinding::Candle,
        "Current Completion Model",
    );
    let embedding = registration(
        embedding_id,
        [0x72; 32],
        RuntimeBinding::Candle,
        "Dedicated Embedding Model",
    );
    store
        .persist_role_bound_boot_set_and_read_back(&[
            RoleBoundModelRegistration::completion(current.clone()),
            RoleBoundModelRegistration::embedding(embedding.clone()),
        ])
        .await
        .expect("persist explicit completion and embedding roles");
    store
        .ensure_active_defaults(&[
            (
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                current.sha256,
            ),
            (
                ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                embedding.sha256,
            ),
        ])
        .await
        .expect("persist both active purpose defaults");

    let mut registry = ModelRegistry::default();
    registry
        .register(current)
        .expect("register completion model");
    registry
        .register(embedding)
        .expect("register embedding model");
    registry.mark_loaded(current_id).expect("completion READY");
    registry.mark_loaded(embedding_id).expect("embedding READY");
    let catalog = ModelCatalog::from_registry_with_roles(
        Arc::new(registry),
        std::collections::HashMap::from([
            (current_id, ModelRuntimeRole::Completion),
            (embedding_id, ModelRuntimeRole::Embedding),
        ]),
    );
    let client = Arc::new(CapturingSelectionLlmClient::new(
        current_id.to_string(),
        catalog,
    ));
    let state = app_state_for_client(&pg.schema_url, client.clone()).await;
    let (base_url, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&json!({
            "target_model_id": embedding_id.to_string(),
            "actor": "native-model-runtime-panel",
            "reason": "negative role-boundary proof",
        }))
        .send()
        .await
        .expect("POST embedding-role target");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let body: Value = response.json().await.expect("typed rejection body");
    assert_eq!(body["error"], "MODEL_RUNTIME_SELECTION_REJECTED");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| { detail.contains("Embedding") && detail.contains("not eligible") }));
    assert!(
        client.swap_requests().is_empty(),
        "rejected target never swaps"
    );
    assert_eq!(client.selected_model_id(), current_id.to_string());
    server.abort();
}

#[tokio::test]
async fn mt014_selection_post_rejects_stale_target_before_swap() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect stale-target proof store");
    let current_id = ModelId::new_v7();
    let current = registration(
        current_id,
        [0x73; 32],
        RuntimeBinding::Candle,
        "Current Completion Model",
    );
    let store = ModelRegistryStore::new(pool);
    store
        .persist_and_read_back(&current)
        .await
        .expect("persist current selection");
    store
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            current.sha256,
        )])
        .await
        .expect("persist application/default");
    let mut registry = ModelRegistry::default();
    registry.register(current).expect("register current model");
    registry.mark_loaded(current_id).expect("current READY");
    let catalog = ModelCatalog::from_registry(Arc::new(registry));
    let client = Arc::new(CapturingSelectionLlmClient::new(
        current_id.to_string(),
        catalog,
    ));
    let state = app_state_for_client(&pg.schema_url, client.clone()).await;
    let (base_url, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&json!({
            "target_model_id": ModelId::new_v7().to_string(),
            "actor": "native-model-runtime-panel",
            "reason": "negative stale-target proof",
        }))
        .send()
        .await
        .expect("POST stale target");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert!(client.swap_requests().is_empty());
    assert_eq!(client.selected_model_id(), current_id.to_string());
    server.abort();
}

#[tokio::test]
async fn mt014_selection_post_integrity_failure_occurs_before_swap() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect role-integrity proof store");
    let current_id = ModelId::new_v7();
    let target_id = ModelId::new_v7();
    let current = registration(
        current_id,
        [0x74; 32],
        RuntimeBinding::Candle,
        "Current Completion Model",
    );
    let target = registration(
        target_id,
        [0x75; 32],
        RuntimeBinding::Candle,
        "Role Drift Target",
    );
    let store = ModelRegistryStore::new(pool);
    store
        .persist_boot_set_and_read_back(&[current.clone(), target.clone()])
        .await
        .expect("persist completion-role authority");
    store
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            current.sha256,
        )])
        .await
        .expect("persist application/default");
    let mut registry = ModelRegistry::default();
    registry.register(current).expect("register current model");
    registry.register(target).expect("register target model");
    registry.mark_loaded(current_id).expect("current READY");
    registry.mark_loaded(target_id).expect("target READY");
    let catalog = ModelCatalog::from_registry_with_roles(
        Arc::new(registry),
        std::collections::HashMap::from([
            (current_id, ModelRuntimeRole::Completion),
            (target_id, ModelRuntimeRole::Embedding),
        ]),
    );
    let client = Arc::new(CapturingSelectionLlmClient::new(
        current_id.to_string(),
        catalog,
    ));
    let state = app_state_for_client(&pg.schema_url, client.clone()).await;
    let (base_url, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&json!({
            "target_model_id": target_id.to_string(),
            "actor": "native-model-runtime-panel",
            "reason": "negative role-integrity proof",
        }))
        .send()
        .await
        .expect("POST target with catalog/durable role drift");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(client.swap_requests().is_empty());
    assert_eq!(client.selected_model_id(), current_id.to_string());
    server.abort();
}

#[tokio::test]
async fn mt014_selection_post_stale_durable_revision_preserves_prior_selection() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect audit-failure proof store");
    let current_id = ModelId::new_v7();
    let target_id = ModelId::new_v7();
    let registrations = vec![
        registration(
            current_id,
            [0x76; 32],
            RuntimeBinding::Candle,
            "Current Completion Model",
        ),
        registration(
            target_id,
            [0x77; 32],
            RuntimeBinding::Candle,
            "Target Completion Model",
        ),
    ];
    let store = ModelRegistryStore::new(pool);
    store
        .persist_boot_set_and_read_back(&registrations)
        .await
        .expect("persist audit-failure selections");
    store
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            registrations[0].sha256,
        )])
        .await
        .expect("persist prior application/default");
    let mut registry = ModelRegistry::default();
    for registration in registrations {
        registry.register(registration).expect("register model");
    }
    registry.mark_loaded(current_id).expect("current READY");
    registry.mark_loaded(target_id).expect("target READY");
    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry(registry.clone());
    let router = LocalRouter::new(
        registry,
        Arc::new(ReadyRuntime::default()),
        Arc::new(ReadyRuntime::default()),
    );
    let fallback = Arc::new(CatalogLlmClient {
        profile: ModelProfile::new("mt014-proof-fallback".to_owned(), 4096),
        catalog: catalog.clone(),
    });
    let client = Arc::new(
        LocalModelRuntimeLlmClient::new(
            router,
            fallback,
            Arc::new(FailingRecorder),
            ModelProfile::new(current_id.to_string(), 4096),
        )
        .with_catalog(catalog)
        .with_durable_application_selection(store, 2),
    );
    let state = app_state_for_client(&pg.schema_url, client.clone()).await;
    let (base_url, server) = start_server(state).await;
    let response = reqwest::Client::new()
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&json!({
            "target_model_id": target_id.to_string(),
            "actor": "native-model-runtime-panel",
            "reason": "negative stale durable revision proof",
        }))
        .send()
        .await
        .expect("POST target with stale durable revision");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(client.selected_model_id(), current_id.to_string());
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_ready_catalog_row_without_durable_authority() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let orphan_id = ModelId::new_v7();
    let orphan = registration(
        orphan_id,
        [0x7c; 32],
        RuntimeBinding::Candle,
        "Unpersisted READY Row",
    );
    let mut registry = ModelRegistry::default();
    registry
        .register(orphan)
        .expect("register orphan catalog row");
    registry
        .mark_loaded(orphan_id)
        .expect("mark orphan catalog row READY");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET registry integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response.json().await.expect("registry error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("no durable model registry row")));
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_unloaded_catalog_row_without_durable_authority() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let orphan = registration(
        ModelId::new_v7(),
        [0x7d; 32],
        RuntimeBinding::Candle,
        "Unpersisted Unloaded Row",
    );
    let mut registry = ModelRegistry::default();
    registry
        .register(orphan)
        .expect("register unloaded orphan catalog row");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET unloaded catalog integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response.json().await.expect("registry error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("no durable model registry row")));
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_duplicate_ready_and_unloaded_catalog_sha() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect duplicate-catalog registry store");
    let store = ModelRegistryStore::new(pool);
    let sha256 = [0x7e; 32];
    let ready_id = ModelId::new_v7();
    let ready = registration(
        ready_id,
        sha256,
        RuntimeBinding::Candle,
        "Duplicate READY Row",
    );
    store
        .persist_and_read_back(&ready)
        .await
        .expect("persist durable authority for duplicate-catalog proof");
    let unloaded = registration(
        ModelId::new_v7(),
        sha256,
        RuntimeBinding::Candle,
        "Duplicate Unloaded Row",
    );
    let mut registry = ModelRegistry::default();
    registry
        .register(ready)
        .expect("register READY duplicate catalog row");
    registry
        .register(unloaded)
        .expect("register unloaded duplicate catalog row");
    registry
        .mark_loaded(ready_id)
        .expect("mark one duplicate catalog row READY");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET duplicate catalog integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response.json().await.expect("duplicate catalog error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("multiple catalog entries")));
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_unloaded_catalog_adapter_drift() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect unloaded adapter-drift registry store");
    let store = ModelRegistryStore::new(pool);
    let sha256 = [0x7f; 32];
    store
        .persist_and_read_back(&registration(
            ModelId::new_v7(),
            sha256,
            RuntimeBinding::Candle,
            "Persisted Candle Selection",
        ))
        .await
        .expect("persist durable Candle authority");

    let mut registry = ModelRegistry::default();
    registry
        .register(registration(
            ModelId::new_v7(),
            sha256,
            RuntimeBinding::LlamaCpp,
            "Unloaded Drifted llama.cpp Row",
        ))
        .expect("register unloaded adapter-drift catalog row");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET unloaded adapter-drift integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response.json().await.expect("adapter-drift error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("catalog adapter")));
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_ready_catalog_capability_drift() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect capability-drift registry store");
    let store = ModelRegistryStore::new(pool);
    let sha256 = [0x6d; 32];
    let persisted = registration(
        ModelId::new_v7(),
        sha256,
        RuntimeBinding::Candle,
        "Persisted Non-Embedding Selection",
    );
    store
        .persist_and_read_back(&persisted)
        .await
        .expect("persist non-embedding selection");

    let live_id = ModelId::new_v7();
    let mut drifted = registration(
        live_id,
        sha256,
        RuntimeBinding::Candle,
        "Drifted Embedding Catalog Entry",
    );
    drifted.declared_capabilities.supports_embedding = true;
    drifted.declared_capabilities.embedding_dimension = Some(768);
    let mut registry = ModelRegistry::default();
    registry
        .register(drifted)
        .expect("register capability-drift catalog row");
    registry
        .mark_loaded(live_id)
        .expect("mark capability-drift catalog row READY");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET capability-drift integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response.json().await.expect("capability-drift error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("embedding capability")));
    server.abort();
}

#[tokio::test]
async fn mt014_registry_api_rejects_ready_uuid_without_committed_observation() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect committed-observation registry store");
    let store = ModelRegistryStore::new(pool);
    let sha256 = [0x8a; 32];
    let last_observed_id = ModelId::new_v7();
    store
        .persist_and_read_back(&registration(
            last_observed_id,
            sha256,
            RuntimeBinding::Candle,
            "Committed Observation",
        ))
        .await
        .expect("persist the previous boot observation");

    let uncommitted_ready_id = ModelId::new_v7();
    let uncommitted_ready = registration(
        uncommitted_ready_id,
        sha256,
        RuntimeBinding::Candle,
        "Committed Observation",
    );
    let mut registry = ModelRegistry::default();
    registry
        .register(uncommitted_ready)
        .expect("register same-selection current boot row without durable readback");
    registry
        .mark_loaded(uncommitted_ready_id)
        .expect("mark the uncommitted current UUID READY");
    let state = app_state_for(
        &pg.schema_url,
        ModelCatalog::from_registry(Arc::new(registry)),
    )
    .await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET uncommitted READY observation integrity failure");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: Value = response
        .json()
        .await
        .expect("uncommitted observation error JSON");
    assert_eq!(body["error"], "MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR");
    assert!(body["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("not committed as the durable last-observed")));
    server.abort();
}

/// PART 2 (MT-014 V5): the ModelRuntime registry projection surfaces the live
/// Section 10.13.1 telemetry that the shipped panel previously omitted. This
/// proves the backend/API wiring end-to-end (GET /model-runtime/registry) with a
/// runtime that reports a real product `RuntimePerfRecorder` snapshot, engine
/// internals, and an applied steering set. The real in-process engine proof
/// (Candle recording live generations) is the feature-gated candle_e2e_smoke
/// suite; this headless proof covers the object-safe wiring the panel reads.
#[tokio::test]
async fn mt014_registry_projection_surfaces_live_runtime_telemetry_through_backend() {
    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect telemetry projection proof store");
    let store = ModelRegistryStore::new(pool.clone());
    let current_id = ModelId::new_v7();
    let registrations = vec![registration(
        current_id,
        [0x91; 32],
        RuntimeBinding::Candle,
        "Telemetry Completion Model",
    )];
    store
        .persist_boot_set_and_read_back(&registrations)
        .await
        .expect("persist READY telemetry row");
    store
        .ensure_active_defaults(&[(
            ModelRuntimeSelectionPurpose::ApplicationDefault,
            registrations[0].sha256,
        )])
        .await
        .expect("persist application/default before telemetry exposure");

    let mut registry = ModelRegistry::default();
    for registration in registrations {
        registry
            .register(registration)
            .expect("register telemetry model");
    }
    registry
        .mark_loaded(current_id)
        .expect("mark telemetry model READY");
    let registry = Arc::new(registry);
    let catalog = ModelCatalog::from_registry(registry.clone());

    // Derive live perf from the real product recorder: 40 decode tokens over
    // 100ms == 400 tokens/sec, with device-reported VRAM residency.
    let mut perf_recorder = RuntimePerfRecorder::new();
    let recorded_at = chrono::Utc::now();
    perf_recorder.record_call(RuntimePerfCall {
        tokens_generated: 40,
        gen_eval_ms: 100,
        vram_resident_bytes: 2_147_483_648,
        completed_at_utc: recorded_at,
    });
    let perf = perf_recorder.snapshot("device VRAM residency was measured for this proof");

    let steering_id = SteeringVectorId::new_v7();
    let active_steering = vec![SteeringVectorMeta {
        id: steering_id,
        name: "telemetry-proof-vector".to_owned(),
        layer: LayerIndex::new(12),
        hook_point: HookPoint::ResidStream,
        intensity: 1.5,
        description: "applied steering vector for the 10.13.1 projection".to_owned(),
    }];
    let engine_internals = json!({
        "adapter": "telemetry-double",
        "device": "Cpu",
        "backend_architecture": "llama",
        "note": "Section 10.13.2 engine internals drilldown",
    });

    // Completion model is Candle-bound, so the router resolves the second
    // (candle) runtime; give that one the live telemetry.
    let router = LocalRouter::new(
        registry,
        Arc::new(ReadyRuntime::default()),
        Arc::new(ReadyRuntime::with_live_telemetry(
            perf,
            engine_internals,
            active_steering,
        )),
    );
    let fallback = Arc::new(CatalogLlmClient {
        profile: ModelProfile::new("mt014-telemetry-fallback".to_owned(), 4096),
        catalog: catalog.clone(),
    });
    let client = Arc::new(
        LocalModelRuntimeLlmClient::new(
            router,
            fallback,
            Arc::new(NoopRecorder),
            ModelProfile::new(current_id.to_string(), 4096),
        )
        .with_catalog(catalog)
        .with_durable_application_selection(store, 1),
    );
    let state = app_state_for_client(&pg.schema_url, client).await;
    let (base_url, server) = start_server(state).await;

    let response = reqwest::Client::new()
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET telemetry projection");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let projection: ModelRuntimeRegistryProjection = response
        .json()
        .await
        .expect("deserialize telemetry projection");
    let row = projection
        .rows
        .iter()
        .find(|row| row.live_model_id.as_deref() == Some(current_id.to_string().as_str()))
        .expect("live telemetry projection row");

    // Active steering set surfaces the applied vector (id, layer, intensity).
    match &row.active_steering {
        ModelRuntimeValue::Available { value } => {
            assert_eq!(value.len(), 1, "one applied steering vector");
            assert_eq!(value[0].steering_vector_id, steering_id.to_string());
            assert_eq!(value[0].layer, 12);
            assert!((value[0].intensity - 1.5).abs() < f32::EPSILON);
        }
        other => panic!("active steering must be available: {other:?}"),
    }

    // Live perf stats surface real recorded throughput, VRAM, and last-call.
    match &row.tokens_per_second {
        ModelRuntimeValue::Available { value } => {
            assert!((value - 400.0).abs() < 1e-6, "throughput is 400 tokens/sec");
        }
        other => panic!("tokens/sec must be available: {other:?}"),
    }
    assert!(matches!(
        &row.vram_resident_bytes,
        ModelRuntimeValue::Available { value } if *value == 2_147_483_648
    ));
    match &row.last_call_at_utc {
        ModelRuntimeValue::Available { value } => {
            chrono::DateTime::parse_from_rfc3339(value).expect("last-call timestamp is RFC3339");
        }
        other => panic!("last-call time must be available: {other:?}"),
    }

    // Engine internals drilldown and its enabling action are both live.
    match &row.engine_internals {
        ModelRuntimeValue::Available { value } => {
            assert_eq!(value["adapter"], "telemetry-double");
        }
        other => panic!("engine internals must be available: {other:?}"),
    }
    assert!(
        row.inspect_engine_internals_action.enabled,
        "inspect-engine-internals action is enabled when internals are live"
    );

    server.abort();
}

// MT-006 follow-up: production cloud ProcessLedger receipt -> fresh account API replay.
// This deliberately lives in the account-facing route suite rather than the provider
// suite so the proof crosses the storage/API boundary instead of re-reading SQL directly.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt006_cloud_stop_process_ownership_route_replays_exact_scope_without_identifier_leaks() {
    const CLOUD_MODEL_NAME: &str = "mt006-route-proof-cloud-model";
    const PARENT_SESSION_ID: &str = "mt006-route-proof-parent-session";
    const STOP_REASON: &str = "mt006-route-proof-complete";

    let pg = pg_required(knowledge_pg_support::knowledge_pg().await);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect MT-006 process-ownership route proof store");

    let owner = OwnerAccountId::mint();
    let actor = ActorPrincipalId::mint();
    let session = AuthenticatedSessionRef::mint();
    let access_space = AccessSpaceRef::mint();
    let workspace = WorkspaceScopeRef::new(format!("mt006-route-proof-{}", Uuid::now_v7()))
        .expect("valid MT-006 route proof workspace");
    let exact_scope = ExactResourceScopeAttribution {
        owner_account_id: owner,
        actor_principal_id: actor,
        authenticated_session_id: session,
        access_space_id: access_space,
        workspace_id: workspace.clone(),
    };
    let write_scope = ResourceScope::new(owner, actor)
        .with_session(session)
        .with_access_space(access_space)
        .with_workspace(workspace.clone());
    let lane_store = ModelLaneStore::new_scoped(pool.clone(), write_scope);

    let process_store = Arc::new(PostgresProcessLedgerStore::new(pool));
    process_store
        .apply_migration()
        .await
        .expect("real PostgreSQL ProcessLedger authority is ready");
    let (ledger, writer) = LedgerBatcher::spawn(
        process_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );

    let builder_model_id = ModelId::new_v7();
    let factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(Arc::new(Mt006OwnershipCloudBuilder {
                model_id: builder_model_id,
            })),
            official_cli: None,
            official_cli_by_provider: Default::default(),
        },
        None,
    )
    .with_durable_worktree_vm_store(&lane_store);
    let mut request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 6),
        RuntimeBinding::Candle,
        "KERNEL_BUILDER-MT006",
        PARENT_SESSION_ID,
    )
    .with_cloud_provider(ProviderKind::ByokCloud, CLOUD_MODEL_NAME);
    request.owner_wp = Some("MT-006".to_owned());
    request.mt_id = Some("MT-006".to_owned());

    let live = factory
        .create(&request)
        .await
        .expect("actual scoped production factory creates the BYOK cloud receipt");
    assert_eq!(
        live.model_id, builder_model_id,
        "the proof must retain the concrete provider model identifier"
    );
    let process_uuid = live.process_record_id.as_uuid();
    let stop = live
        .ledger_lifecycle
        .as_ref()
        .expect("pidless cloud session owns its complete ProcessLedger lifecycle")
        .stop_with_durable_ack(Some(0), STOP_REASON, Duration::from_secs(5))
        .await
        .expect("cloud STOP reaches durable acknowledgement");
    assert!(
        matches!(stop, StopRecordOutcome::Recorded),
        "the first cloud STOP must be newly recorded, not replayed: {stop:?}"
    );
    ledger.begin_close();
    let drain = drain_and_join_ledger_writer(&ledger, writer, Duration::from_secs(5)).await;
    assert!(
        matches!(drain, LedgerDrainJoinOutcome::Flushed),
        "the fresh API consumer must start only after the ledger writer flushes: {drain:?}"
    );

    // Construct the API state only after the terminal write and writer drain. This is a
    // clean replay through a new pool/server, not visibility through the producer handle.
    let catalog = ModelCatalog::from_registry(Arc::new(ModelRegistry::default()));
    let state = app_state_for(&pg.schema_url, catalog).await;
    let http = reqwest::Client::new();
    let (owner_base, owner_server) = start_scoped_server(state.clone(), exact_scope.clone()).await;
    let owner_response = http
        .get(format!(
            "{owner_base}/model-runtime/process-ownership/{process_uuid}"
        ))
        .send()
        .await
        .expect("fresh owner server replays the terminal cloud receipt");
    assert_eq!(owner_response.status(), reqwest::StatusCode::OK);
    let owner_record: Value = owner_response
        .json()
        .await
        .expect("deserialize owner-visible terminal cloud receipt");
    assert_eq!(owner_record["process_uuid"], process_uuid.to_string());
    assert!(
        owner_record["stopped_at_utc"].as_str().is_some(),
        "fresh route replay must expose the final STOP timestamp: {owner_record}"
    );
    assert_eq!(owner_record["exit_code"], 0);
    assert_eq!(owner_record["stop_reason"], STOP_REASON);
    assert_eq!(owner_record["owner_role"], "KERNEL_BUILDER-MT006");
    assert_eq!(owner_record["owner_wp"], "MT-006");
    owner_server.abort();

    let wrong_scopes = [
        (
            "owner",
            ExactResourceScopeAttribution {
                owner_account_id: OwnerAccountId::mint(),
                ..exact_scope.clone()
            },
        ),
        (
            "actor",
            ExactResourceScopeAttribution {
                actor_principal_id: ActorPrincipalId::mint(),
                ..exact_scope.clone()
            },
        ),
        (
            "session",
            ExactResourceScopeAttribution {
                authenticated_session_id: AuthenticatedSessionRef::mint(),
                ..exact_scope.clone()
            },
        ),
        (
            "access-space",
            ExactResourceScopeAttribution {
                access_space_id: AccessSpaceRef::mint(),
                ..exact_scope.clone()
            },
        ),
        (
            "workspace",
            ExactResourceScopeAttribution {
                workspace_id: WorkspaceScopeRef::new(format!(
                    "mt006-route-proof-wrong-{}",
                    Uuid::now_v7()
                ))
                .expect("valid wrong workspace"),
                ..exact_scope
            },
        ),
    ];
    let forbidden_identifiers = [
        process_uuid.to_string(),
        builder_model_id.to_string(),
        owner.to_string(),
        CLOUD_MODEL_NAME.to_owned(),
        PARENT_SESSION_ID.to_owned(),
    ];
    for (dimension, wrong_scope) in wrong_scopes {
        let (wrong_base, wrong_server) = start_scoped_server(state.clone(), wrong_scope).await;
        let denied = http
            .get(format!(
                "{wrong_base}/model-runtime/process-ownership/{process_uuid}"
            ))
            .send()
            .await
            .unwrap_or_else(|error| panic!("query fresh wrong-{dimension} server: {error}"));
        assert_eq!(
            denied.status(),
            reqwest::StatusCode::NOT_FOUND,
            "same-record lookup under wrong {dimension} must be absent-shaped"
        );
        let denied_body = denied
            .text()
            .await
            .unwrap_or_else(|error| panic!("read wrong-{dimension} denial body: {error}"));
        assert_eq!(
            serde_json::from_str::<Value>(&denied_body).expect("denial is JSON"),
            json!({
                "error": "MODEL_RUNTIME_REGISTRY_UNAVAILABLE",
                "detail": "the requested process ownership record is unavailable",
            }),
            "wrong {dimension} must receive only the canonical absent shape"
        );
        for identifier in &forbidden_identifiers {
            assert!(
                !denied_body.contains(identifier),
                "wrong {dimension} response leaked identifier {identifier}: {denied_body}"
            );
        }
        wrong_server.abort();
    }
}

struct Mt006OwnershipCloudBuilder {
    model_id: ModelId,
}

#[async_trait]
impl CloudRuntimeBuilder for Mt006OwnershipCloudBuilder {
    fn provider(&self) -> ProviderKind {
        ProviderKind::ByokCloud
    }

    async fn build_loaded(
        &self,
        _model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        Ok(CloudLiveRuntime {
            runtime: Arc::new(Mt006OwnershipCloudRuntime),
            model_id: self.model_id,
        })
    }
}

struct Mt006OwnershipCloudRuntime;

#[async_trait]
impl ModelRuntime for Mt006OwnershipCloudRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _request: GenerateRequest) -> TokenStream {
        Box::pin(stream::empty())
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
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "mt006-route-proof".into(),
            adapter: "mt006-route-proof-runtime".into(),
        })
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::KvCacheError("mt006-route-proof".into()))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::LoraStackError(
            "mt006-route-proof".into(),
        ))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "mt006-route-proof".into(),
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}
