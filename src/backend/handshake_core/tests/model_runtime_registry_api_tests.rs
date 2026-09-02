//! Embedded SurrealDB boundary tests for the scoped ModelRuntime registry API.

#[path = "surreal_test_store_support/mod.rs"]
mod surreal_test_store_support;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use axum::Extension;
use chrono::Utc;
use handshake_core::{
    api::{
        account_scope::ProductLocalResourceScope,
        model_runtime_registry::{
            self, ModelRuntimeProcessOwnershipRecord, ModelRuntimeRegistryApiState,
            ModelRuntimeRegistryProjection, ModelRuntimeRegistryRowState,
            MODEL_RUNTIME_CONTROL_INVALID_CODE, MODEL_RUNTIME_CONTROL_REJECTED_CODE,
            MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE,
            MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID, MODEL_RUNTIME_REGISTRY_SCOPE_DENIED_CODE,
            MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE, MODEL_RUNTIME_SELECTION_REJECTED_CODE,
        },
    },
    kernel::KernelActor,
    llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile,
        ModelRuntimeControlAction, ModelRuntimeControlCapabilities, ModelRuntimeControlReceipt,
        ModelRuntimeControlRequest, ModelRuntimeInspection, TokenUsage,
        MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
    },
    model_runtime::{
        BaseModelTag, ModelCapabilities, ModelCatalog, ModelId, ModelRegistration, ModelRegistry,
        ModelRuntimeRole, ModelRuntimeSelectionPurpose, OperatorId, ProviderKind,
        RoleBoundModelRegistration, RuntimeBinding, ScopedModelRegistryAuthority,
    },
    process_ledger::{
        LedgerEvent, LedgerEventKind, ProcessEngineKind, ProcessLedgerStore, ProcessStart,
        ReclaimResourceScope, SurrealProcessLedgerStore,
    },
    storage::surreal::{
        bootstrap_model_registry_schema, bootstrap_schema, SurrealModelRegistryStore,
        SurrealStorage,
    },
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    },
    workflows::ModelSwapRequestV0_4,
};
use serde_json::{json, Value};
use surrealdb::types::RecordId;
use uuid::Uuid;

use surreal_test_store_support::EmbeddedSurrealTestScope;

#[derive(Clone)]
struct AuthorityClient {
    profile: ModelProfile,
    catalog: Arc<ModelCatalog>,
    authority: ScopedModelRegistryAuthority,
    selected_model_id: Arc<Mutex<String>>,
    swap_calls: Arc<AtomicUsize>,
    control_calls: Arc<AtomicUsize>,
    control_receipts:
        Arc<Mutex<HashMap<Uuid, (ModelRuntimeControlRequest, ModelRuntimeControlReceipt)>>>,
    fail_with_sensitive_provider_error: Arc<AtomicBool>,
}

impl AuthorityClient {
    fn new(
        selected_model_id: String,
        catalog: Arc<ModelCatalog>,
        authority: ScopedModelRegistryAuthority,
    ) -> Self {
        Self {
            profile: ModelProfile::new(selected_model_id.clone(), 4_096),
            catalog,
            authority,
            selected_model_id: Arc::new(Mutex::new(selected_model_id)),
            swap_calls: Arc::new(AtomicUsize::new(0)),
            control_calls: Arc::new(AtomicUsize::new(0)),
            control_receipts: Arc::new(Mutex::new(HashMap::new())),
            fail_with_sensitive_provider_error: Arc::new(AtomicBool::new(false)),
        }
    }

    fn swap_call_count(&self) -> usize {
        self.swap_calls.load(Ordering::SeqCst)
    }

    fn control_call_count(&self) -> usize {
        self.control_calls.load(Ordering::SeqCst)
    }

    fn fail_provider_calls(&self) {
        self.fail_with_sensitive_provider_error
            .store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl LlmClient for AuthorityClient {
    async fn completion(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage::default(),
            latency_ms: 0,
        })
    }

    async fn swap_model(&self, request: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        self.swap_calls.fetch_add(1, Ordering::SeqCst);
        if self
            .fail_with_sensitive_provider_error
            .load(Ordering::SeqCst)
        {
            return Err(LlmError::ProviderError(
                "provider-secret://must-never-cross-the-api".to_owned(),
            ));
        }
        let metadata = request
            .metadata
            .as_ref()
            .ok_or_else(|| LlmError::ProviderError("selection metadata is required".to_owned()))?;
        let request_id = metadata
            .get("selection_request_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LlmError::ProviderError("selection request id is required".to_owned())
            })?;
        if request_id != request.request_id
            || Uuid::parse_str(request_id).is_err()
            || request_id == Uuid::nil().to_string()
        {
            return Err(LlmError::ProviderError(
                "selection request identity is invalid".to_owned(),
            ));
        }
        let expected_revision = metadata
            .get("expected_selection_revision")
            .and_then(Value::as_u64)
            .filter(|revision| *revision != 0)
            .ok_or_else(|| LlmError::ProviderError("selection revision is required".to_owned()))?;
        let actor = metadata
            .get("actor")
            .and_then(Value::as_str)
            .unwrap_or("model-runtime-api-test");
        let target = self
            .catalog
            .entry(&request.target_model_id)
            .ok_or_else(|| {
                LlmError::ProviderError("target is absent from the current catalog".to_owned())
            })?;
        let target_bytes = hex::decode(&target.artifact_sha256)
            .map_err(|_| LlmError::ProviderError("target artifact is malformed".to_owned()))?;
        let target_sha: [u8; 32] = target_bytes
            .try_into()
            .map_err(|_| LlmError::ProviderError("target artifact length is invalid".to_owned()))?;
        self.authority
            .store()
            .select_active_model(
                self.authority.scope(),
                ModelRuntimeSelectionPurpose::ApplicationDefault,
                target_sha,
                expected_revision,
                KernelActor::Operator(actor.to_owned()),
                &format!("selection_request_id={request_id}; {}", request.reason),
            )
            .await
            .map_err(|_| LlmError::ProviderError("durable selection was rejected".to_owned()))?;
        *self.selected_model_id.lock().expect("selected model lock") = request.target_model_id;
        Ok(())
    }

    async fn control_model_runtime(
        &self,
        request: ModelRuntimeControlRequest,
    ) -> Result<ModelRuntimeControlReceipt, LlmError> {
        self.control_calls.fetch_add(1, Ordering::SeqCst);
        if self
            .fail_with_sensitive_provider_error
            .load(Ordering::SeqCst)
        {
            return Err(LlmError::ProviderError(
                "provider-secret://must-never-cross-the-api".to_owned(),
            ));
        }
        let mut receipts = self.control_receipts.lock().expect("control receipt lock");
        if let Some((cached_request, cached_receipt)) = receipts.get(&request.request_id) {
            if cached_request != &request {
                return Err(LlmError::ProviderError(
                    "control request identity was reused with changed content".to_owned(),
                ));
            }
            return Ok(cached_receipt.clone());
        }
        let receipt = ModelRuntimeControlReceipt {
            schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
            request_id: request.request_id,
            model_id: request.model_id.clone(),
            result_model_id: None,
            action: request.action.clone(),
            runtime_adapter: "candle".to_owned(),
            quiesced: true,
            unloaded: false,
            process_stop_committed: false,
            registry_updated: false,
            selection_rebound: false,
            catalog_revision: request.expected_catalog_revision,
            reconciliation_required: false,
            reconciliation_reason: None,
        };
        receipts.insert(request.request_id, (request, receipt.clone()));
        Ok(receipt)
    }

    fn scoped_model_registry_authority(&self) -> Option<ScopedModelRegistryAuthority> {
        Some(self.authority.clone())
    }

    fn model_runtime_control_capabilities(
        &self,
        _model_id: &str,
    ) -> ModelRuntimeControlCapabilities {
        ModelRuntimeControlCapabilities {
            quiesce: true,
            unload: true,
            swap_compatible_adapter: true,
        }
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

    fn inspect_model_runtime(&self, _model_id: &str) -> ModelRuntimeInspection {
        ModelRuntimeInspection::unavailable("runtime telemetry is outside this API proof")
    }
}

#[derive(Clone)]
struct ModelSet {
    current_id: ModelId,
    target_id: ModelId,
    embedding_id: ModelId,
    current_sha: [u8; 32],
    target_sha: [u8; 32],
    embedding_sha: [u8; 32],
}

struct ApiFixture {
    allocator: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    exact_scope: ExactResourceScopeAttribution,
    process_scope: ReclaimResourceScope,
    registry_store: SurrealModelRegistryStore,
    ledger_store: SurrealProcessLedgerStore,
    catalog: Arc<ModelCatalog>,
    client: Arc<AuthorityClient>,
    models: ModelSet,
}

impl ApiFixture {
    async fn open() -> Self {
        let mut allocator = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate isolated embedded store");
        let storage = allocator
            .activate_storage()
            .await
            .expect("activate the allocated storage scope");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap shared product schema");
        bootstrap_model_registry_schema(&storage)
            .await
            .expect("bootstrap model registry schema");

        let exact_scope = exact_scope("WS-MODEL-RUNTIME-API");
        let process_scope = process_scope(&exact_scope);
        let registry_store = SurrealModelRegistryStore::new(storage.clone());
        registry_store
            .ensure_workspace_for_tests(&exact_scope)
            .await
            .expect("create exact workspace predecessor");

        let models = ModelSet {
            current_id: ModelId::new_v7(),
            target_id: ModelId::new_v7(),
            embedding_id: ModelId::new_v7(),
            current_sha: [0x31; 32],
            target_sha: [0x42; 32],
            embedding_sha: [0x53; 32],
        };
        let current = registration(
            models.current_id,
            models.current_sha,
            RuntimeBinding::Candle,
            ModelRuntimeRole::Completion,
            "current-completion",
        );
        let target = registration(
            models.target_id,
            models.target_sha,
            RuntimeBinding::LlamaCpp,
            ModelRuntimeRole::Completion,
            "target-completion",
        );
        let embedding = registration(
            models.embedding_id,
            models.embedding_sha,
            RuntimeBinding::Candle,
            ModelRuntimeRole::Embedding,
            "embedding-only",
        );
        registry_store
            .persist_role_bound_boot_set_and_read_back(
                &exact_scope,
                &[current.clone(), target.clone(), embedding.clone()],
            )
            .await
            .expect("persist exact-scope registry boot set");
        registry_store
            .ensure_active_defaults(
                &exact_scope,
                &[
                    (
                        ModelRuntimeSelectionPurpose::ApplicationDefault,
                        models.current_sha,
                    ),
                    (
                        ModelRuntimeSelectionPurpose::EmbeddingsDefault,
                        models.embedding_sha,
                    ),
                ],
            )
            .await
            .expect("persist exact-scope active defaults");

        let mut registry = ModelRegistry::default();
        for role_bound in [&current, &target, &embedding] {
            registry
                .register(role_bound.registration.clone())
                .expect("register current-boot model");
            registry
                .mark_loaded(role_bound.registration.model_id)
                .expect("mark current-boot model ready");
        }
        let registry = Arc::new(registry);
        let roles = HashMap::from([
            (models.current_id, ModelRuntimeRole::Completion),
            (models.target_id, ModelRuntimeRole::Completion),
            (models.embedding_id, ModelRuntimeRole::Embedding),
        ]);
        let catalog = ModelCatalog::from_registry_with_roles(registry, roles);

        let ledger_store = SurrealProcessLedgerStore::open(storage.clone())
            .await
            .expect("open ProcessLedger on the same storage clone");
        ledger_store
            .write_batch(vec![
                LedgerEvent::Start(process_start(
                    &process_scope,
                    models.current_id,
                    models.current_sha,
                    ProcessEngineKind::Candle,
                    "current-adapter",
                )),
                LedgerEvent::Start(process_start(
                    &process_scope,
                    models.target_id,
                    models.target_sha,
                    ProcessEngineKind::LlamaCpp,
                    "target-adapter",
                )),
                LedgerEvent::Start(process_start(
                    &process_scope,
                    models.embedding_id,
                    models.embedding_sha,
                    ProcessEngineKind::Candle,
                    "embedding-adapter",
                )),
            ])
            .await
            .expect("atomically persist model process ownership");

        let authority =
            ScopedModelRegistryAuthority::new(registry_store.clone(), exact_scope.clone());
        let client = Arc::new(AuthorityClient::new(
            models.current_id.to_string(),
            catalog.clone(),
            authority,
        ));
        Self {
            allocator,
            storage,
            exact_scope,
            process_scope,
            registry_store,
            ledger_store,
            catalog,
            client,
            models,
        }
    }

    fn api_state(&self) -> ModelRuntimeRegistryApiState {
        ModelRuntimeRegistryApiState::new(self.storage.clone(), self.client.clone())
    }

    fn client_for_storage(
        &self,
        storage: SurrealStorage,
        selected_model_id: ModelId,
    ) -> Arc<AuthorityClient> {
        Arc::new(AuthorityClient::new(
            selected_model_id.to_string(),
            self.catalog.clone(),
            ScopedModelRegistryAuthority::new(
                SurrealModelRegistryStore::new(storage),
                self.exact_scope.clone(),
            ),
        ))
    }

    async fn close(&mut self) {
        let receipt = self
            .allocator
            .cleanup()
            .await
            .expect("remove exact embedded test scope");
        assert!(receipt.database_absent);
        assert!(receipt.namespace_absent_after_reopen);
    }
}

fn exact_scope(workspace: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(workspace).expect("valid workspace id"),
    }
}

fn process_scope(scope: &ExactResourceScopeAttribution) -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: scope.owner_account_id.as_uuid(),
        actor_uuid: scope.actor_principal_id.as_uuid(),
        session_uuid: scope.authenticated_session_id.as_uuid(),
        workspace_id: scope.workspace_id.as_str().to_owned(),
        access_space_uuid: scope.access_space_id.as_uuid(),
    }
}

fn scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id.clone(),
    })
}

fn registration(
    model_id: ModelId,
    sha256: [u8; 32],
    runtime_binding: RuntimeBinding,
    role: ModelRuntimeRole,
    label: &str,
) -> RoleBoundModelRegistration {
    let declared_capabilities = ModelCapabilities {
        supports_embedding: role == ModelRuntimeRole::Embedding,
        embedding_dimension: (role == ModelRuntimeRole::Embedding).then_some(768),
        ..ModelCapabilities::default()
    };
    let registration = ModelRegistration {
        model_id,
        artifact_path: PathBuf::from(format!("fixtures/models/{label}.safetensors")),
        sha256,
        runtime_binding,
        declared_capabilities,
        base_model_tag: BaseModelTag::new(label),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("model-runtime-api-proof"),
        provider: ProviderKind::Local,
    };
    match role {
        ModelRuntimeRole::Completion => RoleBoundModelRegistration::completion(registration),
        ModelRuntimeRole::Embedding => RoleBoundModelRegistration::embedding(registration),
    }
}

fn process_start(
    scope: &ReclaimResourceScope,
    model_id: ModelId,
    sha256: [u8; 32],
    engine_kind: ProcessEngineKind,
    sandbox_adapter_id: &str,
) -> ProcessStart {
    ProcessStart::new(
        engine_kind,
        "MODEL_RUNTIME_OWNER",
        Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".to_owned()),
    )
    .with_process_uuid(model_id.as_uuid())
    .with_os_pid(7_001)
    .with_parent_session_id("model-runtime-api-session")
    .with_sandbox_adapter_id(sandbox_adapter_id)
    .with_model_artifact_sha256(hex::encode(sha256))
    .with_metadata_jsonb(scope_metadata(scope))
}

fn one_field_mismatches(
    scope: &ExactResourceScopeAttribution,
) -> [ExactResourceScopeAttribution; 5] {
    let mut account = scope.clone();
    account.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access = scope.clone();
    access.access_space_id = AccessSpaceRef::mint();
    let mut workspace = scope.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new("WS-MODEL-RUNTIME-API-FOREIGN").expect("valid workspace id");
    [account, actor, session, access, workspace]
}

async fn start_server(
    state: ModelRuntimeRegistryApiState,
    scope: Option<ExactResourceScopeAttribution>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind registry API test server");
    let address = listener.local_addr().expect("read registry server address");
    let routes = model_runtime_registry::routes(state);
    let routes = match scope {
        Some(scope) => routes.layer(Extension(
            ProductLocalResourceScope::from_exact(scope).expect("valid product-local scope"),
        )),
        None => routes,
    };
    let server = tokio::spawn(async move {
        axum::serve(listener, routes)
            .await
            .expect("serve registry API test routes");
    });
    (format!("http://{address}"), server)
}

fn selection_request(
    request_id: Uuid,
    expected_selection_revision: u64,
    target_model_id: ModelId,
) -> Value {
    json!({
        "request_id": request_id,
        "expected_selection_revision": expected_selection_revision,
        "target_model_id": target_model_id.to_string(),
        "actor": "native-model-runtime-panel",
        "reason": "operator selected an exact-scope ready completion model",
    })
}

fn control_request(request_id: Uuid, model_id: ModelId) -> ModelRuntimeControlRequest {
    ModelRuntimeControlRequest {
        schema_version: MODEL_RUNTIME_CONTROL_SCHEMA_VERSION,
        request_id,
        model_id: model_id.to_string(),
        action: ModelRuntimeControlAction::Quiesce,
        timeout_ms: 1_000,
        expected_catalog_revision: Some(0),
        expected_selection_revision: None,
    }
}

async fn error_body(response: reqwest::Response) -> (reqwest::StatusCode, String, String) {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .expect("deserialize stable error body");
    (
        status,
        body["error"].as_str().unwrap_or_default().to_owned(),
        body["detail"].as_str().unwrap_or_default().to_owned(),
    )
}

#[tokio::test]
async fn registry_and_detail_join_exact_process_identity_and_survive_reopen() {
    let mut fixture = ApiFixture::open().await;
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let http = reqwest::Client::new();

    let response = http
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET exact-scope registry");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let projection: ModelRuntimeRegistryProjection = response
        .json()
        .await
        .expect("deserialize registry projection");
    assert_eq!(
        projection.schema_id,
        MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.rows.len(), 3);

    let current_hash = hex::encode(fixture.models.current_sha);
    let current = projection
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == current_hash)
        .expect("current completion projection");
    assert_eq!(current.runtime_state, ModelRuntimeRegistryRowState::Live);
    assert_eq!(current.runtime_role, ModelRuntimeRole::Completion);
    assert!(current.selected);
    assert_eq!(current.active_selection_revision, Some(1));
    assert_eq!(
        current.live_model_id.as_deref(),
        Some(fixture.models.current_id.to_string().as_str())
    );
    assert!(matches!(
        &current.process_ownership_ledger_link,
        handshake_core::llm::ModelRuntimeValue::Available { value }
            if value.ends_with(&fixture.models.current_id.to_string())
    ));

    let embedding = projection
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == hex::encode(fixture.models.embedding_sha))
        .expect("embedding projection");
    assert_eq!(embedding.runtime_role, ModelRuntimeRole::Embedding);
    assert!(!embedding.default_selectable);
    assert!(!embedding.selected);

    let detail_response = http
        .get(format!(
            "{base_url}/model-runtime/process-ownership/{}",
            fixture.models.current_id
        ))
        .send()
        .await
        .expect("GET exact process ownership detail");
    assert_eq!(detail_response.status(), reqwest::StatusCode::OK);
    let detail: ModelRuntimeProcessOwnershipRecord = detail_response
        .json()
        .await
        .expect("deserialize process ownership detail");
    assert_eq!(detail.process_uuid, fixture.models.current_id.as_uuid());
    assert_eq!(detail.engine_kind, ProcessEngineKind::Candle.as_str());
    assert_eq!(detail.owner_role, "MODEL_RUNTIME_OWNER");
    assert_eq!(
        detail.owner_wp.as_deref(),
        Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
    );
    assert_eq!(
        detail.sandbox_adapter_id.as_deref(),
        Some("current-adapter")
    );
    assert_eq!(
        detail.model_artifact_sha256.as_deref(),
        Some(current_hash.as_str())
    );

    server.abort();
    let _ = server.await;
    fixture
        .allocator
        .shutdown_storage_for_reopen()
        .await
        .expect("close injected storage before restart");
    fixture
        .allocator
        .reopen()
        .await
        .expect("reopen exact store");
    let reopened = fixture
        .allocator
        .activate_storage()
        .await
        .expect("reactivate the same namespace and database");
    let reopened_client = fixture.client_for_storage(reopened.clone(), fixture.models.current_id);
    let reopened_state = ModelRuntimeRegistryApiState::new(reopened.clone(), reopened_client);
    fixture.storage = reopened;
    let (base_url, server) = start_server(reopened_state, Some(fixture.exact_scope.clone())).await;
    let recovered: ModelRuntimeRegistryProjection = http
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET registry after reopen")
        .json()
        .await
        .expect("deserialize reopened projection");
    let recovered_current = recovered
        .rows
        .iter()
        .find(|row| row.artifact_sha256 == current_hash)
        .expect("recover current artifact after reopen");
    assert!(recovered_current.selected);
    assert_eq!(recovered_current.active_selection_revision, Some(1));
    server.abort();
    let _ = server.await;
    fixture.close().await;
}

#[tokio::test]
async fn selection_cas_preserves_stable_identity_and_lost_response_retry() {
    let mut fixture = ApiFixture::open().await;
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let http = reqwest::Client::new();
    let request_id = Uuid::now_v7();
    let request = selection_request(request_id, 1, fixture.models.target_id);

    let first = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&request)
        .send()
        .await
        .expect("POST first selection CAS");
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let first_projection: ModelRuntimeRegistryProjection = first
        .json()
        .await
        .expect("deserialize first CAS projection");
    let first_receipt = first_projection
        .selection_receipt_ref
        .clone()
        .expect("first selection receipt");
    assert!(!request_id.is_nil());

    let retry = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&request)
        .send()
        .await
        .expect("POST identical retry after lost response");
    assert_eq!(retry.status(), reqwest::StatusCode::OK);
    let retry_projection: ModelRuntimeRegistryProjection =
        retry.json().await.expect("deserialize retry projection");
    assert_eq!(
        retry_projection.selection_receipt_ref.as_deref(),
        Some(first_receipt.as_str())
    );

    let committed = fixture
        .registry_store
        .list_active_selections(&fixture.exact_scope)
        .await
        .expect("read committed selection")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application default exists");
    assert_eq!(committed.artifact_sha256, fixture.models.target_sha);
    assert_eq!(committed.selection_revision, 2);

    let conflicting = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&selection_request(request_id, 1, fixture.models.current_id))
        .send()
        .await
        .expect("POST changed envelope with reused request id");
    let (status, code, _) = error_body(conflicting).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_SELECTION_REJECTED_CODE);

    let stale = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&selection_request(
            Uuid::now_v7(),
            1,
            fixture.models.current_id,
        ))
        .send()
        .await
        .expect("POST stale selection revision");
    let (status, code, _) = error_body(stale).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_SELECTION_REJECTED_CODE);

    let embedding = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&selection_request(
            Uuid::now_v7(),
            2,
            fixture.models.embedding_id,
        ))
        .send()
        .await
        .expect("POST embedding model as completion default");
    let (status, code, _) = error_body(embedding).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_SELECTION_REJECTED_CODE);

    let after = fixture
        .registry_store
        .list_active_selections(&fixture.exact_scope)
        .await
        .expect("read selection after denied retries")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application default remains");
    assert_eq!(after.artifact_sha256, fixture.models.target_sha);
    assert_eq!(after.selection_revision, 2);
    assert_eq!(
        after.selection_updated_event_id,
        committed.selection_updated_event_id
    );
    server.abort();
    let _ = server.await;
    fixture.close().await;
}

#[tokio::test]
async fn control_is_exact_scope_idempotent_and_rejects_changed_or_nil_identity() {
    let mut fixture = ApiFixture::open().await;
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let http = reqwest::Client::new();
    let request_id = Uuid::now_v7();
    let request = control_request(request_id, fixture.models.current_id);

    let first: ModelRuntimeControlReceipt = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&request)
        .send()
        .await
        .expect("POST first control request")
        .json()
        .await
        .expect("deserialize first control receipt");
    let retry: ModelRuntimeControlReceipt = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&request)
        .send()
        .await
        .expect("POST identical control retry")
        .json()
        .await
        .expect("deserialize cached control receipt");
    assert_eq!(first, retry);
    assert_eq!(first.request_id, request_id);

    let mut changed = request.clone();
    changed.timeout_ms += 1;
    let changed = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&changed)
        .send()
        .await
        .expect("POST changed control envelope");
    let (status, code, _) = error_body(changed).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_CONTROL_REJECTED_CODE);

    let nil = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&control_request(Uuid::nil(), fixture.models.current_id))
        .send()
        .await
        .expect("POST nil control identity");
    let (status, code, _) = error_body(nil).await;
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(code, MODEL_RUNTIME_CONTROL_INVALID_CODE);
    assert_eq!(fixture.client.control_call_count(), 3);
    server.abort();
    let _ = server.await;
    fixture.close().await;
}

#[tokio::test]
async fn all_five_authority_mismatches_and_incomplete_or_mixed_scope_deny_without_mutation() {
    let mut fixture = ApiFixture::open().await;
    let http = reqwest::Client::new();

    for mismatched_scope in one_field_mismatches(&fixture.exact_scope) {
        let (base_url, server) = start_server(fixture.api_state(), Some(mismatched_scope)).await;
        let selection = http
            .post(format!("{base_url}/model-runtime/selection"))
            .json(&selection_request(
                Uuid::now_v7(),
                1,
                fixture.models.target_id,
            ))
            .send()
            .await
            .expect("POST selection with one mismatched authority field");
        let (status, code, detail) = error_body(selection).await;
        assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
        assert_eq!(code, MODEL_RUNTIME_REGISTRY_SCOPE_DENIED_CODE);
        assert!(!detail.contains(&fixture.models.target_id.to_string()));

        let control = http
            .post(format!("{base_url}/model-runtime/control"))
            .json(&control_request(Uuid::now_v7(), fixture.models.current_id))
            .send()
            .await
            .expect("POST control with one mismatched authority field");
        let (status, code, detail) = error_body(control).await;
        assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
        assert_eq!(code, MODEL_RUNTIME_REGISTRY_SCOPE_DENIED_CODE);
        assert!(!detail.contains(&fixture.models.current_id.to_string()));
        server.abort();
        let _ = server.await;
    }

    let absent_scope_state = fixture.api_state();
    let (base_url, server) = start_server(absent_scope_state, None).await;
    let absent = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&control_request(Uuid::now_v7(), fixture.models.current_id))
        .send()
        .await
        .expect("POST without server-owned scope");
    let (status, code, detail) = error_body(absent).await;
    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(code, "RESOURCE_SCOPE_AUTHORITY_UNAVAILABLE");
    assert!(!detail.contains(&fixture.models.current_id.to_string()));
    server.abort();
    let _ = server.await;

    let foreign_a = exact_scope("WS-MIXED-A");
    let foreign_b = exact_scope("WS-MIXED-B");
    let mixed = ExactResourceScopeAttribution {
        owner_account_id: foreign_a.owner_account_id,
        actor_principal_id: foreign_b.actor_principal_id,
        authenticated_session_id: foreign_a.authenticated_session_id,
        access_space_id: foreign_b.access_space_id,
        workspace_id: foreign_a.workspace_id,
    };
    let (base_url, server) = start_server(fixture.api_state(), Some(mixed)).await;
    let mixed = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&selection_request(
            Uuid::now_v7(),
            1,
            fixture.models.target_id,
        ))
        .send()
        .await
        .expect("POST with mixed exact-scope identity");
    let (status, code, detail) = error_body(mixed).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(code, MODEL_RUNTIME_REGISTRY_SCOPE_DENIED_CODE);
    assert!(!detail.contains(&fixture.models.target_id.to_string()));
    server.abort();
    let _ = server.await;

    assert_eq!(fixture.client.swap_call_count(), 0);
    assert_eq!(fixture.client.control_call_count(), 0);
    let active = fixture
        .registry_store
        .list_active_selections(&fixture.exact_scope)
        .await
        .expect("read unchanged exact-scope selection")
        .into_iter()
        .find(|selection| selection.purpose == ModelRuntimeSelectionPurpose::ApplicationDefault)
        .expect("application default remains");
    assert_eq!(active.artifact_sha256, fixture.models.current_sha);
    assert_eq!(active.selection_revision, 1);
    fixture.close().await;
}

#[tokio::test]
async fn missing_foreign_and_tampered_process_receipts_fail_closed_without_leakage() {
    let mut fixture = ApiFixture::open().await;
    let missing_id = ModelId::new_v7();
    let foreign_id = ModelId::new_v7();
    let tampered_id = ModelId::new_v7();
    let missing_sha = [0x64; 32];
    let foreign_sha = [0x75; 32];
    let tampered_sha = [0x86; 32];
    fixture
        .ledger_store
        .write_batch(vec![
            LedgerEvent::Start(process_start(
                &fixture.process_scope,
                missing_id,
                missing_sha,
                ProcessEngineKind::Candle,
                "missing-receipt-adapter",
            )),
            LedgerEvent::Start(process_start(
                &fixture.process_scope,
                foreign_id,
                foreign_sha,
                ProcessEngineKind::Candle,
                "foreign-receipt-adapter",
            )),
            LedgerEvent::Start(process_start(
                &fixture.process_scope,
                tampered_id,
                tampered_sha,
                ProcessEngineKind::Candle,
                "tampered-receipt-adapter",
            )),
        ])
        .await
        .expect("persist receipt-corruption predecessors");
    fixture
        .ledger_store
        .test_delete_inspection_receipt(
            &fixture.process_scope,
            missing_id.as_uuid(),
            LedgerEventKind::Start,
        )
        .await
        .expect("remove exact receipt");
    let foreign_scope = process_scope(&exact_scope("WS-FOREIGN-RECEIPT"));
    fixture
        .ledger_store
        .test_move_inspection_receipt_to_scope(
            &fixture.process_scope,
            foreign_id.as_uuid(),
            LedgerEventKind::Start,
            &foreign_scope,
        )
        .await
        .expect("move receipt to a foreign exact scope");
    fixture
        .ledger_store
        .test_set_inspection_event_link(
            &fixture.process_scope,
            tampered_id.as_uuid(),
            Some(RecordId::new(
                "kernel_event_ledger",
                "tampered-process-receipt-link",
            )),
        )
        .await
        .expect("tamper lifecycle receipt link");

    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let http = reqwest::Client::new();
    for (process_id, artifact_sha) in [
        (missing_id, missing_sha),
        (foreign_id, foreign_sha),
        (tampered_id, tampered_sha),
    ] {
        let response = http
            .get(format!(
                "{base_url}/model-runtime/process-ownership/{process_id}"
            ))
            .send()
            .await
            .expect("GET process with invalid receipt linkage");
        let (status, code, detail) = error_body(response).await;
        assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE);
        assert!(!detail.contains(&process_id.to_string()));
        assert!(!detail.contains(&hex::encode(artifact_sha)));
        assert!(!detail.contains("tampered-process-receipt-link"));
    }
    server.abort();
    let _ = server.await;
    fixture.close().await;
}

#[tokio::test]
async fn provider_and_closed_storage_errors_are_redacted() {
    let mut fixture = ApiFixture::open().await;
    fixture.client.fail_provider_calls();
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let http = reqwest::Client::new();

    let control = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&control_request(Uuid::now_v7(), fixture.models.current_id))
        .send()
        .await
        .expect("POST provider-failed control request");
    let (status, code, detail) = error_body(control).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_CONTROL_REJECTED_CODE);
    assert!(!detail.contains("provider-secret"));

    let selection = http
        .post(format!("{base_url}/model-runtime/selection"))
        .json(&selection_request(
            Uuid::now_v7(),
            1,
            fixture.models.target_id,
        ))
        .send()
        .await
        .expect("POST provider-failed selection request");
    let (status, code, detail) = error_body(selection).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(code, MODEL_RUNTIME_SELECTION_REJECTED_CODE);
    assert!(!detail.contains("provider-secret"));
    server.abort();
    let _ = server.await;

    let namespace = fixture.allocator.namespace().to_owned();
    let database = fixture.allocator.database().to_owned();
    fixture
        .allocator
        .shutdown_storage_for_reopen()
        .await
        .expect("close injected storage");
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone())).await;
    let closed = http
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET registry against closed storage");
    let (status, code, detail) = error_body(closed).await;
    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(code, MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE);
    assert!(!detail.contains(&namespace));
    assert!(!detail.contains(&database));
    assert!(!detail.contains("provider-secret"));
    server.abort();
    let _ = server.await;
    fixture.close().await;
}
