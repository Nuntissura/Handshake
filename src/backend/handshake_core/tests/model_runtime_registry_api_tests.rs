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
            self, ModelRuntimeRegistryApiState,
            ModelRuntimeRegistryProjection, ModelRuntimeRegistryRowState,
            MODEL_RUNTIME_CONTROL_INVALID_CODE, MODEL_RUNTIME_CONTROL_REJECTED_CODE,
            MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE,
            MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID,
            MODEL_RUNTIME_SELECTION_REJECTED_CODE,
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
        ReclaimResourceScope,
    },
    storage::surreal::{
        bootstrap_model_registry_schema, SurrealModelRegistryStore, SurrealProcessLedgerStore,
        SurrealStorage,
    },
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    },
    workflows::ModelSwapRequestV0_4,
};
use serde_json::{json, Value};
use sha2::Digest;
use surrealdb::types::RecordId;
use uuid::Uuid;

use surreal_test_store_support::IsolatedSurrealTestStore;

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
    allocator: IsolatedSurrealTestStore,
    storage: SurrealStorage,
    exact_scope: ExactResourceScopeAttribution,
    process_scope: ReclaimResourceScope,
    registry_store: SurrealModelRegistryStore,
    ledger_store: SurrealProcessLedgerStore,
    catalog: Arc<ModelCatalog>,
    client: Arc<AuthorityClient>,
    models: ModelSet,
    account: engine_profile_account_session_support::OwnerSession,
    _binding: engine_profile_account_session_support::NativeBindingEnv,
    _binding_guard: tokio::sync::MutexGuard<'static, ()>,
}

impl ApiFixture {
    async fn open() -> Self { Self::open_with_system_principal(false).await }

    async fn open_with_system_principal(non_operator: bool) -> Self {
        use engine_profile_account_session_support::{NativeBindingEnv, OwnerSession, NATIVE_BINDING_ENV_LOCK};
        let binding_guard = NATIVE_BINDING_ENV_LOCK.lock().await;
        let binding = NativeBindingEnv::install();
        let mut allocator = IsolatedSurrealTestStore::create()
            .await
            .expect("allocate isolated embedded store");
        let storage = allocator.storage().clone();
        bootstrap_model_registry_schema(&storage)
            .await
            .expect("bootstrap model registry schema");

        let account = OwnerSession::provision(&storage, binding.token()).await;
        let workspace = account.create_workspace(&engine_profile_durable_boundary::state(storage.clone())).await;
        let account = if non_operator {
            use handshake_core::storage::surreal::resource_authority::{
                ProvisionedIdentity, ResourceAction, ResourceGrantSpec,
            };
            let key = format!("registry-system-{}", Uuid::now_v7());
            let capabilities = vec!["fs.read".to_owned(), "fs.write".to_owned()];
            let principal = storage.provision_principal(
                &account.actor_id, &key, "service_identity", &key, "Operator", &capabilities,
                &account.actor_id, Some(&hex::encode(sha2::Sha256::digest(binding.token().as_bytes()))),
                std::time::Duration::from_secs(3600),
            ).await.expect("issue genuine system principal and channel-bound session");
            assert_eq!(principal.identity.account_id, account.account_id);
            assert_eq!(principal.identity.access_space_id, account.access_space_id);
            let resource = storage.register_workspace_resource(&ProvisionedIdentity {
                account_id: account.account_id.clone(), principal_id: account.principal_id.clone(),
                access_space_id: account.access_space_id.clone(),
            }, &workspace).await.expect("canonical existing workspace resource");
            storage.grant_resource(&account.account_id, &account.access_space_id, ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(), resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read, ResourceAction::Create, ResourceAction::Update], capability_ids: capabilities,
                expires_at: None, delegation_chain: Vec::new(),
            }).await.expect("actual fs.write Update grant for non-operator");
            OwnerSession {
                session_token: principal.session.token, channel_binding_token: binding.token().to_owned(),
                account_id: principal.identity.account_id, principal_id: principal.identity.principal_id,
                access_space_id: principal.identity.access_space_id, session_id: principal.session.session_id,
                actor_id: key,
            }
        } else { account };
        let parse = |s: &str| Uuid::parse_str(s).expect("issued UUID");
        let exact_scope = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(parse(&account.account_id)),
            actor_principal_id: ActorPrincipalId::from_uuid(parse(&account.principal_id)),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&account.session_id)),
            access_space_id: AccessSpaceRef::from_uuid(parse(&account.access_space_id)),
            workspace_id: WorkspaceScopeRef::new(workspace).expect("issued workspace"),
        };
        let process_scope = process_scope(&exact_scope);
        let registry_store = SurrealModelRegistryStore::new(storage.clone());


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
        let bootstrap = handshake_core::AccountBootstrapState::from(
            &engine_profile_durable_boundary::state(storage.clone()),
        );
        handshake_core::test_harness::model_runtime_selection::with_issued_registry_initializer(
            &bootstrap, &account.headers(), &exact_scope, async {
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

            },
        ).await.expect("genuine issued registry Create authority for fixture setup");
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
                LedgerEvent::Start(unowned_negative_process_start(
                    &process_scope,
                    models.current_id,
                    models.current_sha,
                    ProcessEngineKind::Candle,
                )),
                LedgerEvent::Start(unowned_negative_process_start(
                    &process_scope,
                    models.target_id,
                    models.target_sha,
                    ProcessEngineKind::LlamaCpp,
                )),
                LedgerEvent::Start(unowned_negative_process_start(
                    &process_scope,
                    models.embedding_id,
                    models.embedding_sha,
                    ProcessEngineKind::Candle,
                )),
            ])
            .await
            .expect("persist unowned rows for fail-closed ownership denial only");

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
            account, _binding: binding, _binding_guard: binding_guard,
        }
    }

    fn api_state(&self) -> ModelRuntimeRegistryApiState {
        ModelRuntimeRegistryApiState::new(self.storage.clone(), self.client.clone())
            .with_account_bootstrap(handshake_core::AccountBootstrapState::from(
                &engine_profile_durable_boundary::state(self.storage.clone()),
            ))
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

fn unowned_negative_process_start(
    scope: &ReclaimResourceScope,
    model_id: ModelId,
    sha256: [u8; 32],
    engine_kind: ProcessEngineKind,
) -> ProcessStart {
    ProcessStart::new(
        engine_kind,
        "UNOWNED_NEGATIVE_FIXTURE",
        None,
    )
    .with_process_uuid(model_id.as_uuid())
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
    storage: SurrealStorage,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind registry API test server");
    let address = listener.local_addr().expect("read registry server address");
    let lifecycle = storage.resource_access_lifecycle();
    let routes = model_runtime_registry::routes(state)
        .merge(handshake_core::api::kernel::routes(engine_profile_durable_boundary::state(storage)))
        .layer(Extension(lifecycle));
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
async fn registry_projection_excludes_unowned_process_rows_and_survives_reopen() {
    let mut fixture = ApiFixture::open().await;
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();

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
    // Component catalog rows do not establish an actual loaded runtime or ownership.
    assert!(matches!(
        &current.process_ownership_ledger_link,
        handshake_core::llm::ModelRuntimeValue::Unavailable { .. }
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
        .expect("GET unowned negative-fixture process detail");
    assert_eq!(detail_response.status(), reqwest::StatusCode::NOT_FOUND);
    let (_, code, detail) = error_body(detail_response).await;
    assert_eq!(code, MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE);
    assert!(!detail.contains(&fixture.models.current_id.to_string()));
    assert!(!detail.contains(&current_hash));
    server.abort();
    let _ = server.await;
fixture
        .allocator
        .reopen()
        .await
        .expect("reopen exact store");
    let reopened = fixture.allocator.storage().clone();
    let reopened_client = fixture.client_for_storage(reopened.clone(), fixture.models.current_id);
    let reopened_state = ModelRuntimeRegistryApiState::new(reopened.clone(), reopened_client)
        .with_account_bootstrap(handshake_core::AccountBootstrapState::from(
            &engine_profile_durable_boundary::state(reopened.clone()),
        ));
    fixture.storage = reopened;
    let (base_url, server) = start_server(reopened_state, Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
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
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();
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
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();
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
    let http = fixture.account.client();

    for mismatched_scope in one_field_mismatches(&fixture.exact_scope) {
        let (base_url, server) = start_server(fixture.api_state(), Some(mismatched_scope), fixture.storage.clone()).await;
        for route in [
            format!("{base_url}/model-runtime/registry"),
            format!("{base_url}/model-runtime/process-ownership/{}", fixture.models.current_id),
        ] {
            let response = http.get(route).send().await.expect("mismatched read authority");
            assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
            assert_eq!(response.json::<Value>().await.expect("constant read denial"),
                json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
        }        let selection = http
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
        assert_eq!(code, "HSK-403-PROTECTED-RESOURCE");
        assert!(!detail.contains(&fixture.models.target_id.to_string()));

        let control = http
            .post(format!("{base_url}/model-runtime/control"))
            .json(&control_request(Uuid::now_v7(), fixture.models.current_id))
            .send()
            .await
            .expect("POST control with one mismatched authority field");
        let (status, code, detail) = error_body(control).await;
        assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
        assert_eq!(code, "HSK-403-PROTECTED-RESOURCE");
        assert!(!detail.contains(&fixture.models.current_id.to_string()));
        server.abort();
        let _ = server.await;
    }

    let absent_scope_state = fixture.api_state();
    let (base_url, server) = start_server(absent_scope_state, None, fixture.storage.clone()).await;
    let absent = http
        .post(format!("{base_url}/model-runtime/control"))
        .json(&control_request(Uuid::now_v7(), fixture.models.current_id))
        .send()
        .await
        .expect("POST without server-owned scope");
    let (status, code, detail) = error_body(absent).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
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
    let (base_url, server) = start_server(fixture.api_state(), Some(mixed), fixture.storage.clone()).await;
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
    assert_eq!(code, "HSK-403-PROTECTED-RESOURCE");
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
async fn unowned_rows_with_missing_foreign_or_tampered_receipts_remain_unavailable_without_leakage() {
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
            LedgerEvent::Start(unowned_negative_process_start(
                &fixture.process_scope,
                missing_id,
                missing_sha,
                ProcessEngineKind::Candle,
            )),
            LedgerEvent::Start(unowned_negative_process_start(
                &fixture.process_scope,
                foreign_id,
                foreign_sha,
                ProcessEngineKind::Candle,
            )),
            LedgerEvent::Start(unowned_negative_process_start(
                &fixture.process_scope,
                tampered_id,
                tampered_sha,
                ProcessEngineKind::Candle,
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
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();
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
        assert_eq!(status, reqwest::StatusCode::NOT_FOUND);
        assert_eq!(code, MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE);
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
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();

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
        .storage
        .shutdown()
        .await
        .expect("close injected storage");
    let (base_url, server) =
        start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let closed = http
        .get(format!("{base_url}/model-runtime/registry"))
        .send()
        .await
        .expect("GET registry against closed storage");
    let (status, code, detail) = error_body(closed).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(code, "HSK-403-PROTECTED-RESOURCE");
    assert!(!detail.contains(&namespace));
    assert!(!detail.contains(&database));
    assert!(!detail.contains("provider-secret"));
    server.abort();
    let _ = server.await;
    fixture.close().await;
}

#[tokio::test]
async fn registry_initializer_requires_current_original_create_and_never_promotes_read_or_update() {
    use handshake_core::test_harness::model_runtime_selection::with_issued_registry_initializer;
    use handshake_core::storage::surreal::resource_authority::{
        AuthorizationRequest, ProvisionedIdentity, ResourceAction, ResourceGrantSpec, ResourceKind,
    };
    use engine_profile_account_session_support::OwnerSession;
    let mut fixture = ApiFixture::open().await;
    let bootstrap = handshake_core::AccountBootstrapState::from(
        &engine_profile_durable_boundary::state(fixture.storage.clone()),
    );
    let before = fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical initialized defaults");
    // This is an actual issued request, not an attribution-to-authority constructor.
    let positive = with_issued_registry_initializer(
        &bootstrap, &fixture.account.headers(), &fixture.exact_scope, async { "issued-create" },
    ).await.expect("existing current fs.write Create grant");
    assert_eq!(positive, "issued-create");
    let key = format!("registry-no-create-{}", Uuid::now_v7());
    let capabilities = vec!["fs.read".to_owned(), "fs.write".to_owned()];
    let issued = fixture.storage.provision_principal(
        &fixture.account.actor_id, &key, "service_identity", &key, "Operator", &capabilities,
        &fixture.account.actor_id,
        Some(&hex::encode(sha2::Sha256::digest(fixture.account.channel_binding_token.as_bytes()))),
        std::time::Duration::from_secs(3600),
    ).await.expect("issue current same-account no-Create principal");
    assert_eq!(issued.identity.account_id, fixture.account.account_id);
    assert_eq!(issued.identity.access_space_id, fixture.account.access_space_id);
    let resource = fixture.storage.register_workspace_resource(&ProvisionedIdentity {
        account_id: fixture.account.account_id.clone(), principal_id: fixture.account.principal_id.clone(),
        access_space_id: fixture.account.access_space_id.clone(),
    }, fixture.exact_scope.workspace_id.as_str()).await.expect("canonical workspace resource");
    fixture.storage.grant_resource(&fixture.account.account_id, &fixture.account.access_space_id, ResourceGrantSpec {
        principal_id: issued.identity.principal_id.clone(), resource_id: resource.resource_id,
        actions: vec![ResourceAction::Read, ResourceAction::Update], capability_ids: capabilities,
        expires_at: None, delegation_chain: Vec::new(),
    }).await.expect("explicit genuine Read/Update-only grant");
    let no_create = OwnerSession {
        session_token: issued.session.token, channel_binding_token: fixture.account.channel_binding_token.clone(),
        account_id: issued.identity.account_id, principal_id: issued.identity.principal_id,
        access_space_id: issued.identity.access_space_id, session_id: issued.session.session_id, actor_id: key,
    };
    let mut no_create_scope = fixture.exact_scope.clone();
    no_create_scope.actor_principal_id = ActorPrincipalId::from_uuid(Uuid::parse_str(&no_create.principal_id).unwrap());
    no_create_scope.authenticated_session_id = AuthenticatedSessionRef::from_uuid(Uuid::parse_str(&no_create.session_id).unwrap());
    fixture.storage.authorize_protected_resource(AuthorizationRequest {
        session_token: no_create.session_token.clone(),
        channel_binding_hash: Some(hex::encode(sha2::Sha256::digest(no_create.channel_binding_token.as_bytes()))),
        capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
        external_resource_id: fixture.exact_scope.workspace_id.as_str().to_owned(), action: ResourceAction::Update,
    }).await.expect("Update is genuinely granted but cannot authorize Create");
    let polled = AtomicBool::new(false);
    let denied = with_issued_registry_initializer(
        &bootstrap, &no_create.headers(), &no_create_scope,
        async { polled.store(true, Ordering::SeqCst); },
    ).await.expect_err("Update cannot authorize initialization");
    assert_eq!(denied.0, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(denied.1.0, json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
    assert!(!polled.load(Ordering::SeqCst));
    for foreign in one_field_mismatches(&fixture.exact_scope) {
        assert!(with_issued_registry_initializer(
            &bootstrap, &fixture.account.headers(), &foreign,
            async { polled.store(true, Ordering::SeqCst); },
        ).await.is_err());
    }
    assert!(!polled.load(Ordering::SeqCst));
    fixture.storage.revoke_session(&fixture.account.session_id).await.expect("revoke actual issued session");
    assert!(with_issued_registry_initializer(
        &bootstrap, &fixture.account.headers(), &fixture.exact_scope,
        async { polled.store(true, Ordering::SeqCst); },
    ).await.is_err());
    assert!(!polled.load(Ordering::SeqCst));
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("no canonical default mutation after denied initializer"), before);
    fixture.close().await;
}
#[tokio::test]
async fn issued_non_operator_with_current_update_grant_cannot_select_or_rebind() {
    use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
    let mut fixture = ApiFixture::open_with_system_principal(true).await;
    let granted = fixture.storage.authorize_protected_resource(AuthorizationRequest {
        session_token: fixture.account.session_token.clone(),
        channel_binding_hash: Some(hex::encode(sha2::Sha256::digest(
            fixture.account.channel_binding_token.as_bytes(),
        ))),
        capability_id: "fs.write".to_owned(), resource_kind: ResourceKind::Workspace,
        external_resource_id: fixture.exact_scope.workspace_id.as_str().to_owned(),
        action: ResourceAction::Update,
    }).await.expect("current real system principal has exact workspace Update grant");
    assert_eq!(granted.actor_kind, "system");
    let before = fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical selections before non-operator denial");
    let (base, server) = start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let http = fixture.account.client();
    let mut rebind = control_request(Uuid::now_v7(), fixture.models.current_id);
    rebind.action = ModelRuntimeControlAction::SwapCompatibleAdapter { target_adapter: "llama_cpp".into() };
    rebind.expected_selection_revision = Some(1);
    for request in [
        http.post(format!("{base}/model-runtime/selection"))
            .json(&selection_request(Uuid::now_v7(), 1, fixture.models.target_id)),
        http.post(format!("{base}/model-runtime/control")).json(&rebind),
    ] {
        let response = request.send().await.expect("actual non-operator mutation denial");
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(response.json::<Value>().await.expect("constant operator admission denial"),
            json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
    }
    assert_eq!(fixture.client.swap_call_count(), 0);
    assert_eq!(fixture.client.control_call_count(), 0);
    assert_eq!(fixture.client.selected_model_id(), fixture.models.current_id.to_string());
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical selection after denial"), before);
    server.abort();
    let _ = server.await;
    fixture.close().await;
}
#[tokio::test]
async fn every_registry_leaf_denies_missing_forged_or_wrong_channel_credentials_before_provider_access() {
    let mut fixture = ApiFixture::open().await;
    let before = fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical defaults before credential denials");
    let (base, server) = start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    for http in [
        reqwest::Client::new(),
        reqwest::Client::builder().default_headers({
            let mut headers = fixture.account.headers();
            headers.insert("x-hsk-session-token", "f".repeat(64).parse().unwrap());
            headers
        }).build().unwrap(),
        reqwest::Client::builder().default_headers({
            let mut headers = fixture.account.headers();
            headers.insert("x-hsk-channel-binding-token", "f".repeat(64).parse().unwrap());
            headers
        }).build().unwrap(),
    ] {
        let requests = [
            http.get(format!("{base}/model-runtime/registry")),
            http.get(format!("{base}/model-runtime/process-ownership/{}", fixture.models.current_id)),
            http.get(format!("{base}/model-runtime/process-ownership/{}", Uuid::now_v7())),
            http.post(format!("{base}/model-runtime/selection"))
                .json(&selection_request(Uuid::now_v7(), 1, fixture.models.target_id)),
            http.post(format!("{base}/model-runtime/control"))
                .json(&control_request(Uuid::now_v7(), fixture.models.current_id)),
        ];
        for request in requests {
            let response = request.send().await.expect("unauthorized registry leaf");
            assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
            assert_eq!(response.json::<Value>().await.expect("constant denial"),
                json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
        }
    }
    assert_eq!(fixture.client.swap_call_count(), 0);
    assert_eq!(fixture.client.control_call_count(), 0);
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical defaults after credential denials"), before);
    server.abort();
    let _ = server.await;
    fixture.close().await;
}
#[tokio::test]
async fn selection_http_observer_reads_actual_kernel_receipt_with_issued_exact_read_and_denies_others() {
    let mut fixture = ApiFixture::open().await;
    let (base, server) = start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let request_id = Uuid::now_v7();
    let http = fixture.account.client();
    let selected = http.post(format!("{base}/model-runtime/selection"))
        .json(&selection_request(request_id, 1, fixture.models.target_id)).send().await.expect("selection HTTP");
    assert_eq!(selected.status(), reqwest::StatusCode::OK);
    let projection: ModelRuntimeRegistryProjection = selected.json().await.expect("selection projection");
    let event_id = projection.selection_receipt_ref.as_deref().expect("selection receipt locator")
        .strip_prefix("eventledger://kernel/").expect("canonical kernel receipt locator");
    let observer = format!("{base}/kernel/events/aggregates/model_runtime_active_selection/application%2Fdefault");
    let observed = http.get(&observer).send().await.expect("canonical kernel observer HTTP");
    assert_eq!(observed.status(), reqwest::StatusCode::OK);
    let events: Vec<Value> = observed.json().await.expect("canonical kernel event rows");
    let event = events.iter().find(|event| event["event_id"] == event_id)
        .expect("receipt locator resolved to an actual canonical event row");
    assert_eq!(event["aggregate_type"], "model_runtime_active_selection");
    assert_eq!(event["aggregate_id"], "application/default");
    assert_eq!(event["actor"]["kind"], "operator");
    assert_eq!(event["actor"]["id"], fixture.account.actor_id);
    assert_eq!(event["session_run_id"], fixture.account.session_id);
    assert_eq!(event["payload"]["authenticated_session_id"], fixture.account.session_id);
    assert_eq!(event["payload"]["workspace_id"], fixture.exact_scope.workspace_id.as_str());
    assert_eq!(event["payload"]["target_artifact_sha256"], hex::encode(fixture.models.target_sha));
    assert!(event["payload"]["reason"].as_str().expect("selection request reason")
        .starts_with(&format!("selection_request_id={request_id}; ")));
    let before = fixture.registry_store.list_active_selections(&fixture.exact_scope).await.expect("canonical state before denied observer");
    for request in [
        reqwest::Client::new().get(&observer),
        http.get(&observer).header("x-hsk-session-token", "f".repeat(64)),
        http.get(&observer).header("x-hsk-channel-binding-token", "f".repeat(64)),
    ] {
        let denied = request.send().await.expect("denied canonical observer HTTP");
        assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(denied.json::<Value>().await.expect("constant observer denial"), json!({
            "code": "HSK-403-PROTECTED-RESOURCE", "message": "authenticated account session required",
        }));
    }
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope).await.expect("canonical state after denied observer"), before);
    server.abort();
    let _ = server.await;
    fixture.close().await;
}
// Component boundary only: the selected model below is AuthorityClient state, not a real adapter.
// This assertion must never be cited as actual runtime/default restoration proof.
#[tokio::test]
async fn canonical_selection_audit_fault_is_finite_atomic_and_request_specific() {
    use surrealdb::types::SurrealValue;
    use handshake_core::test_harness::model_runtime_selection::fail_next_selection_audit_append;
    #[derive(Debug, PartialEq, surrealdb::types::SurrealValue)]
    struct AuditRow {
        id: RecordId, payload: Value, payload_hash: String,
        actor_kind: String, actor_id: String, wsids: Vec<String>,
        authority_resource_id: Option<RecordId>, authority_session_id: Option<RecordId>,
        authority_capability_id: Option<String>, authority_action: Option<String>,
    }
    async fn audits(storage: &SurrealStorage) -> Vec<AuditRow> {
        storage.with_data_operation(|db| Box::pin(async move {
            db.query_values::<AuditRow, _>(
                "SELECT id, payload, payload_hash, actor_kind, actor_id, wsids, authority_resource_id, authority_session_id, authority_capability_id, authority_action FROM kernel_event_ledger WHERE aggregate_type = 'model_runtime_active_selection' ORDER BY id;", (),
            ).await
        })).await.expect("canonical selection audit read")
    }
    let mut fixture = ApiFixture::open().await;
    let before_selection = fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical selection before failure");
    let before_audit = audits(&fixture.storage).await;
    let request_id = Uuid::now_v7();
    let request = selection_request(request_id, 1, fixture.models.target_id);
    let reason = request["reason"].as_str().expect("selection reason");
    fail_next_selection_audit_append(
        &fixture.registry_store, &fixture.exact_scope,
        ModelRuntimeSelectionPurpose::ApplicationDefault, fixture.models.target_sha, 1,
        &KernelActor::Operator(fixture.account.actor_id.clone()),
        &format!("selection_request_id={request_id}; {reason}"),
    ).expect("arm one canonical audit append fault");
    // A read does not consume this exact mutation fault.
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("read while armed"), before_selection);
    let (base, server) = start_server(fixture.api_state(), Some(fixture.exact_scope.clone()), fixture.storage.clone()).await;
    let response = fixture.account.client().post(format!("{base}/model-runtime/selection"))
        .json(&request).send().await.expect("faulted selection");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(fixture.registry_store.list_active_selections(&fixture.exact_scope)
        .await.expect("canonical selection after failure"), before_selection);
    assert_eq!(audits(&fixture.storage).await, before_audit);
    assert_eq!(fixture.client.selected_model_id(), fixture.models.current_id.to_string());
    let retry = fixture.account.client().post(format!("{base}/model-runtime/selection"))
        .json(&request).send().await.expect("same request after finite fault");
    assert_eq!(retry.status(), reqwest::StatusCode::OK);
    let after_audit = audits(&fixture.storage).await;
    assert_eq!(after_audit.len(), before_audit.len() + 1);
    let committed = after_audit.iter().find(|row| !before_audit.iter().any(|old| old.id == row.id))
        .expect("new independently read canonical receipt");
    assert_eq!(committed.actor_kind, KernelActor::Operator(String::new()).actor_kind());
    assert_eq!(committed.actor_id, fixture.account.actor_id);
    assert_eq!(committed.wsids, vec![fixture.exact_scope.workspace_id.as_str().to_owned()]);
    assert!(committed.authority_resource_id.is_some());
    assert_eq!(committed.authority_session_id, Some(RecordId::new(
        "authenticated_sessions", fixture.account.session_id.clone(),
    )));
    assert_eq!(committed.authority_capability_id.as_deref(), Some("fs.write"));
    assert_eq!(committed.authority_action.as_deref(), Some("update"));
    server.abort();
    let _ = server.await;
    fixture.close().await;
}
#[path = "account_session_support/mod.rs"]
mod engine_profile_account_session_support;

// MT196-DURABLE-AUDIT: real authenticated storage/HTTP boundary, no model execution.
mod engine_profile_durable_boundary {
    use super::*;
    use super::engine_profile_account_session_support::{AccountFixture, OwnerSession};
    use handshake_core::{
        capabilities::CapabilityRegistry,
        diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup},
        flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
        model_runtime::{EngineProfileMutationRequestV1, EngineProfileMutationV1,
            EngineProfileRegistryV1, EngineProfileSnapshotV1},
        storage::surreal::SurrealDatabase,
        workflows::{SessionRegistry, SessionSchedulerConfig},
        AppState,
    };
    use surrealdb::types::SurrealValue;

    // Neither service participates in profile persistence. Unexpected inference fails the test.
    struct BoundaryServices { profile: ModelProfile }
    #[async_trait]
    impl LlmClient for BoundaryServices {
        async fn completion(&self, _: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            panic!("profile CRUD must not invoke a model")
        }
        fn profile(&self) -> &ModelProfile { &self.profile }
    }
    #[async_trait]
    impl FlightRecorder for BoundaryServices {
        async fn record_event(&self, _: FlightRecorderEvent) -> Result<(), RecorderError> { Ok(()) }
        async fn enforce_retention(&self) -> Result<u64, RecorderError> { Ok(0) }
        async fn list_events(&self, _: EventFilter) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(Vec::new())
        }
    }
    #[async_trait]
    impl DiagnosticsStore for BoundaryServices {
        async fn record_diagnostic(&self, _: Diagnostic) -> Result<(), handshake_core::storage::StorageError> { Ok(()) }
        async fn list_problems(&self, _: DiagFilter) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> { Ok(Vec::new()) }
        async fn get_diagnostic(&self, _: Uuid) -> Result<Diagnostic, handshake_core::storage::StorageError> {
            Err(handshake_core::storage::StorageError::NotFound("diagnostic"))
        }
        async fn list_diagnostics(&self, _: DiagFilter) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> { Ok(Vec::new()) }
    }

    pub(super) fn state(storage: SurrealStorage) -> AppState {
        let services = Arc::new(BoundaryServices { profile: ModelProfile::new("no-inference".into(), 2048) });
        AppState {
            storage: Arc::new(SurrealDatabase::new(storage.clone())), surreal: storage,
            flight_recorder: services.clone(), diagnostics: services.clone(), llm_client: services,
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        }
    }

    async fn server(storage: SurrealStorage) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("profile listener");
        let base = format!("http://{}", listener.local_addr().expect("profile address"));
        let router = model_runtime_registry::engine_profile_routes(state(storage));
        let task = tokio::spawn(async move { axum::serve(listener, router).await.expect("profile router"); });
        (base, task)
    }

    struct Fixture {
        allocator: IsolatedSurrealTestStore,
        account: AccountFixture,
        workspace: String,
        base: String,
        server: tokio::task::JoinHandle<()>,
    }
    impl Fixture {
        async fn open() -> Self {
            let allocator = IsolatedSurrealTestStore::create().await.expect("isolated profile store");
            bootstrap_model_registry_schema(allocator.storage()).await.expect("profile schema");
            let account = AccountFixture::install(allocator.storage()).await;
            let workspace = account.create_workspace(&state(allocator.storage().clone())).await;
            let (base, server) = server(allocator.storage().clone()).await;
            Self { allocator, account, workspace, base, server }
        }
        fn url(&self) -> String { format!("{}/workspaces/{}/engine-profiles", self.base, self.workspace) }
        fn scope(&self) -> ExactResourceScopeAttribution {
            ExactResourceScopeAttribution {
                owner_account_id: OwnerAccountId::from_uuid(Uuid::parse_str(&self.account.account_id).expect("account UUID")),
                actor_principal_id: ActorPrincipalId::from_uuid(Uuid::parse_str(&self.account.principal_id).expect("principal UUID")),
                authenticated_session_id: AuthenticatedSessionRef::from_uuid(Uuid::parse_str(&self.account.session_id).expect("session UUID")),
                access_space_id: AccessSpaceRef::from_uuid(Uuid::parse_str(&self.account.access_space_id).expect("access-space UUID")),
                workspace_id: WorkspaceScopeRef::new(self.workspace.clone()).expect("workspace scope"),
            }
        }
        async fn read(&self) -> EngineProfileRegistryV1 {
            let response = self.account.client().get(self.url()).send().await.expect("GET profiles");
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            let routed: EngineProfileRegistryV1 = response.json().await.expect("profile registry response");
            let canonical = SurrealModelRegistryStore::new(self.allocator.storage().clone())
                .list_engine_profiles(&self.scope()).await.expect("independent canonical readback");
            assert_eq!(routed, canonical);
            routed
        }
        async fn mutate(&self, request: &EngineProfileMutationRequestV1) -> EngineProfileRegistryV1 {
            let response = self.account.client().post(self.url()).json(request).send().await.expect("POST profile mutation");
            let status = response.status();
            let body: Value = response.json().await.expect("profile mutation response");
            assert_eq!(status, reqwest::StatusCode::OK, "{body}");
            let result = serde_json::from_value(body).expect("typed registry result");
            assert_eq!(result, self.read().await);
            result
        }
        async fn reject(&self, request: &EngineProfileMutationRequestV1, status: reqwest::StatusCode, code: &str) {
            let before = self.read().await;
            let receipts = receipt_rows(self.allocator.storage()).await;
            let response = self.account.client().post(self.url()).json(request).send().await.expect("rejected mutation");
            assert_eq!(response.status(), status);
            let body: Value = response.json().await.expect("typed mutation rejection");
            assert_eq!(body["error"], code);
            assert_eq!(self.read().await, before);
            assert_eq!(receipt_rows(self.allocator.storage()).await, receipts);
        }
        async fn reopen(&mut self) {
            self.server.abort();
            let _ = (&mut self.server).await;
            self.allocator.reopen().await.expect("durable profile reopen");
            let (base, task) = server(self.allocator.storage().clone()).await;
            self.base = base;
            self.server = task;
        }
        async fn close(mut self) {
            self.server.abort();
            let _ = (&mut self.server).await;
            let receipt = self.allocator.cleanup().await.expect("profile store cleanup");
            assert!(receipt.database_absent);
            assert!(receipt.namespace_absent_after_reopen);
        }
    }

    #[derive(Debug, PartialEq, SurrealValue)]
    struct ProfileReceipt { id: RecordId, payload: Value, payload_hash: String }
    async fn receipt_rows(storage: &SurrealStorage) -> Vec<ProfileReceipt> {
        storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<ProfileReceipt, _>(
                "SELECT id, payload, payload_hash FROM kernel_event_ledger WHERE source_component = 'engine_profiles' AND aggregate_type = 'engine_profile_registry' ORDER BY id;", ()
            ).await
        })).await.expect("independent canonical profile receipts")
    }

    fn snapshot(id: &str, revision: u64) -> EngineProfileSnapshotV1 {
        EngineProfileSnapshotV1::new(handshake_dexterity::EngineProfile {
            id: id.into(), revision,
            model: handshake_dexterity::Model {
                id: "storage-only-model".into(), format: handshake_dexterity::ModelFormat::GgufQ4_0,
                content_sha256: "1".repeat(64), compiled_context: None,
            },
            engine: handshake_dexterity::Engine::GenieXLlamaCpp,
            device: handshake_dexterity::Device::Cpu,
            preset: handshake_dexterity::Preset {
                name: format!("storage-revision-{revision}"), context_limit: 2048, max_tokens: 128,
                sampling: handshake_dexterity::Sampling::LlamaCpp { temperature: 0.7, top_k: 40, top_p: 0.95, min_p: 0.05 },
            },
        }).expect("valid storage-only profile, no artifact/device proof")
    }
    fn request(state: &EngineProfileRegistryV1, mutation: EngineProfileMutationV1) -> EngineProfileMutationRequestV1 {
        EngineProfileMutationRequestV1 {
            request_id: Uuid::now_v7(), reason: "explicit profile boundary test".into(),
            expected_registry_generation: state.registry_generation,
            expected_active_generation: state.active_generation, mutation,
        }
    }

    #[tokio::test]
    async fn engine_profile_definitions_active_and_immutable_history_survive_reopen() {
        let mut fixture = Fixture::open().await;
        let empty = fixture.read().await;
        assert_eq!(empty, EngineProfileRegistryV1::default());
        let first = snapshot("durable-profile", 1);
        let created = fixture.mutate(&request(&empty, EngineProfileMutationV1::Create { snapshot: first.clone() })).await;
        assert_eq!(created.registry_generation, 1);
        let active = fixture.mutate(&request(&created, EngineProfileMutationV1::Activate { expected_head: first.identity(), expected_head_generation: 1 })).await;
        assert_eq!(active.active, Some(first.identity()));
        fixture.reopen().await;
        assert_eq!(fixture.read().await, active);
        let second = snapshot("durable-profile", 2);
        let revised = fixture.mutate(&request(&active, EngineProfileMutationV1::NewRevision {
            expected_head: first.identity(), expected_head_generation: 1, snapshot: second.clone(),
        })).await;
        assert_eq!(revised.profiles[0].revisions, vec![first.clone(), second.clone()]);
        assert_eq!(revised.profiles[0].head_generation, 2);
        assert_eq!(revised.active, None);
        assert_eq!(revised.active_generation, active.active_generation + 1);
        assert_ne!(first.fingerprint, second.fingerprint);
        fixture.reject(&request(&revised, EngineProfileMutationV1::Activate {
            expected_head: first.identity(), expected_head_generation: 1,
        }), reqwest::StatusCode::CONFLICT, "engine_profile_conflict").await;
        let selected = fixture.mutate(&request(&revised, EngineProfileMutationV1::Activate {
            expected_head: second.identity(), expected_head_generation: 2,
        })).await;
        let deleted = fixture.mutate(&request(&selected, EngineProfileMutationV1::Tombstone {
            expected_head: second.identity(), expected_head_generation: 2,
        })).await;
        assert!(deleted.profiles[0].tombstoned);
        assert_eq!(deleted.profiles[0].revisions, vec![first.clone(), second.clone()]);
        assert_eq!(deleted.active, None);
        fixture.reject(&request(&deleted, EngineProfileMutationV1::Activate {
            expected_head: second.identity(), expected_head_generation: 3,
        }), reqwest::StatusCode::NOT_FOUND, "engine_profile_unavailable").await;
        fixture.reopen().await;
        assert_eq!(fixture.read().await, deleted);
        fixture.close().await;
    }

    #[tokio::test]
    async fn engine_profile_retry_after_later_mutation_returns_current_state_without_rewriting_receipt() {
        let mut fixture = Fixture::open().await;
        let a = request(&fixture.read().await, EngineProfileMutationV1::Create { snapshot: snapshot("replay-profile", 1) });
        let state_a = fixture.mutate(&a).await;
        let receipt_a = receipt_rows(fixture.allocator.storage()).await;
        assert_eq!(receipt_a.len(), 1);
        assert_eq!(receipt_a[0].payload["request"], serde_json::to_value(&a).unwrap());
        assert_eq!(receipt_a[0].payload["after"], serde_json::to_value(&state_a).unwrap());
        let b = request(&state_a, EngineProfileMutationV1::Activate { expected_head: snapshot("replay-profile", 1).identity(), expected_head_generation: 1 });
        let state_b = fixture.mutate(&b).await;
        let receipts_b = receipt_rows(fixture.allocator.storage()).await;
        assert_eq!(receipts_b.len(), 2);
        assert_ne!(state_a, state_b);
        assert_eq!(fixture.mutate(&a).await, state_b);
        assert_eq!(receipt_rows(fixture.allocator.storage()).await, receipts_b);
        assert!(receipts_b.iter().any(|row| row == &receipt_a[0]));
        fixture.reopen().await;
        assert_eq!(fixture.mutate(&a).await, state_b);
        assert_eq!(receipt_rows(fixture.allocator.storage()).await, receipts_b);
        let mut altered = a.clone();
        altered.reason = "same request ID with changed payload".into();
        fixture.reject(&altered, reqwest::StatusCode::CONFLICT, "engine_profile_conflict").await;
        let stale_cas = request(&state_a, EngineProfileMutationV1::Create { snapshot: snapshot("stale-cas-profile", 1) });
        fixture.reject(&stale_cas, reqwest::StatusCode::CONFLICT, "engine_profile_conflict").await;
        fixture.close().await;
    }

    #[tokio::test]
    async fn engine_profile_scope_and_invalid_snapshot_denials_leave_canonical_state_unchanged() {
        let fixture = Fixture::open().await;
        let create = request(&fixture.read().await, EngineProfileMutationV1::Create { snapshot: snapshot("private-profile", 1) });
        let current = fixture.mutate(&create).await;
        let receipts = receipt_rows(fixture.allocator.storage()).await;
        let other = OwnerSession::provision(fixture.allocator.storage(), fixture.account.binding_token()).await;
        let denied = other.client().get(fixture.url()).send().await.expect("other-account GET");
        assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
        let foreign_post = other.client().post(fixture.url()).json(&create).send().await.expect("other-account POST");
        assert_eq!(foreign_post.status(), reqwest::StatusCode::FORBIDDEN);
        let anonymous = reqwest::Client::new().post(fixture.url()).json(&create).send().await.expect("anonymous POST");
        assert_eq!(anonymous.status(), reqwest::StatusCode::FORBIDDEN);
        let second_workspace = fixture.account.create_workspace(&state(fixture.allocator.storage().clone())).await;
        let other_url = format!("{}/workspaces/{second_workspace}/engine-profiles", fixture.base);
        let response = fixture.account.client().get(&other_url).send().await.expect("other workspace GET");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.json::<EngineProfileRegistryV1>().await.unwrap(), EngineProfileRegistryV1::default());
        let unavailable = request(&EngineProfileRegistryV1::default(), EngineProfileMutationV1::Activate {
            expected_head: snapshot("private-profile", 1).identity(), expected_head_generation: 1,
        });
        let response = fixture.account.client().post(&other_url).json(&unavailable).send().await.expect("other workspace activation");
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        assert_eq!(response.json::<Value>().await.unwrap()["error"], "engine_profile_unavailable");
        let mut invalid = snapshot("invalid-profile", 1);
        invalid.fingerprint = "0".repeat(64);
        fixture.reject(&request(&current, EngineProfileMutationV1::Create { snapshot: invalid }),
            reqwest::StatusCode::BAD_REQUEST, "engine_profile_fingerprint_mismatch").await;
        assert_eq!(fixture.read().await, current);
        assert_eq!(receipt_rows(fixture.allocator.storage()).await, receipts);
        fixture.close().await;
    }
}

/// Actual configured Candle producer/owner boundary. This is not the component AuthorityClient
/// fixture, and missing/incompatible operator input is an error rather than a skipped proof.
#[cfg(feature = "app-runtime")]
#[tokio::test]
async fn configured_real_candle_load_has_scoped_owned_registry_and_http_process_detail() {
    use engine_profile_account_session_support::AccountFixture;
    use handshake_core::{
        llm::registry::{ProviderRegistry, RuntimeRole},
        process_ledger::{ProcessReclaimRuntime, production_process_sandbox_registry_async, resolve_embedded_runtime_host_scope},
        test_harness::model_runtime_selection::{GenuineModelSelectionFixture, GenuineCompatibleCompletionInput},
    };
    let resolved = ProviderRegistry::from_env().expect("explicit configured provider input")
        .resolve(RuntimeRole::Orchestrator).expect("configured current model resolution");
    let local = resolved.local_model.as_ref().expect("selected Candle GGUF4B must be explicitly configured");
    assert_eq!(local.runtime_binding, RuntimeBinding::Candle);
    assert_eq!(hex::encode(local.sha256), "83df045fb0254aeb17feba1c693bd2d51bfc4f610e213d9414623d26673beafa");
    let compatible_target = GenuineCompatibleCompletionInput::read_explicit_input().expect("independently bound compatible Completion input is required; missing input fails closed");
    let mut allocator = IsolatedSurrealTestStore::create().await.expect("isolated actual-model store");
    let storage = allocator.storage().clone();
    bootstrap_model_registry_schema(&storage).await.expect("actual model registry schema");
    let account = AccountFixture::install(&storage).await;
    let workspace = account.create_workspace(&engine_profile_durable_boundary::state(storage.clone())).await;
    let parse = |value: &str| Uuid::parse_str(value).expect("issued UUID");
    let exact = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::from_uuid(parse(&account.account_id)),
        actor_principal_id: ActorPrincipalId::from_uuid(parse(&account.principal_id)),
        authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&account.session_id)),
        access_space_id: AccessSpaceRef::from_uuid(parse(&account.access_space_id)),
        workspace_id: WorkspaceScopeRef::new(workspace).expect("issued workspace"),
    };
    let bootstrap = handshake_core::AccountBootstrapState::from(&engine_profile_durable_boundary::state(storage.clone()));
    let recorder_root = tempfile::TempDir::new().expect("fixture recorder root");
    let recorder = Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_on_path(
        &recorder_root.path().join("genuine-registry-fixture.duckdb"), 7,
    ).expect("actual fixture flight recorder"));
    // Process ownership is composed outside the narrow registry Create context, as in production.
    let runtime = ProcessReclaimRuntime::production(
        storage.clone(), ReclaimResourceScope::from_exact(&exact).expect("real exact runtime scope"), None,
        production_process_sandbox_registry_async().await.expect("actual sandbox registry"),
        resolve_embedded_runtime_host_scope().expect("actual runtime host scope"), std::time::Duration::from_secs(30),
    ).await.expect("actual process runtime and OS lease");
    let fixture = match GenuineModelSelectionFixture::load_current(
        &resolved, &compatible_target, runtime, bootstrap.clone(), account.headers(), exact.clone(), recorder,
    ).await {
        Ok(fixture) => fixture,
        Err(failure) => {
            let settlement = tokio::time::timeout(std::time::Duration::from_secs(35),
                failure.retained.shutdown(std::time::Duration::from_secs(30))).await;
            let reason = failure.reason.clone();
            if !matches!(&settlement, Ok(Ok(()))) {
                let _retained = Box::leak(Box::new((failure, allocator, recorder_root, storage)));
            }
            panic!("actual configured load failed: {reason}; bounded owned settlement: {settlement:?}");
        }
    };
    use futures::FutureExt;
    let mut server_task = None;
    let mut role_guard = None;
    let mut integrity_restore = None;
    let mut selection_restore = None;
    let outcome = std::panic::AssertUnwindSafe(async {
        let actual = fixture.client().expect("actual loaded client");
        let current_id = fixture.current_model_id().expect("actual verified current model");
        let before = fixture.snapshot_default().await.expect("canonical actual-runtime/default snapshot");
        let (base, server) = start_server(
            ModelRuntimeRegistryApiState::new(storage.clone(), actual).with_account_bootstrap(bootstrap),
            Some(exact), storage.clone(),
        ).await;
        server_task = Some(server);
        let response = account.client().get(format!("{base}/model-runtime/process-ownership/{current_id}"))
            .send().await.expect("actual owned process detail HTTP");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let detail: handshake_core::api::model_runtime_registry::ModelRuntimeProcessOwnershipRecord = response.json().await.expect("real ownership detail");
        assert_eq!(detail.process_uuid, current_id.as_uuid());
        assert!(detail.os_pid.is_none());
        assert_eq!(detail.engine_kind, ProcessEngineKind::Candle.as_str());
        assert_eq!(detail.model_artifact_sha256.as_deref(), Some(hex::encode(local.sha256).as_str()));
        assert_eq!(fixture.snapshot_default().await.expect("actual default after observation"), before);
        let catalog = fixture.client().expect("real client").model_catalog().expect("real shared catalog");
        let original_entry = catalog.entry(&current_id.to_string()).expect("genuinely loaded Completion entry");
        let catalog_revision = catalog.runtime_availability_revision();
        role_guard = Some(fixture.project_current_catalog_role().await.expect("actual selected owner role projection"));
        let projected = catalog.entry(&current_id.to_string()).expect("actual projected catalog entry");
        let mut expected_projected = original_entry.clone();
        expected_projected.runtime_role = ModelRuntimeRole::Embedding;
        expected_projected.default_selectable = false;
        assert_eq!(projected, expected_projected);
        assert_eq!(catalog.runtime_availability_revision(), catalog_revision);
        assert!(fixture.project_current_catalog_role().await.is_err(), "concurrent role projection must be rejected");
        let role_drift_response = account.client().get(format!("{base}/model-runtime/registry"))
            .send().await;
        let during_role_drift = fixture.snapshot_default().await;
        // Restore and verify before evaluating responses so assertion failures do not strand the role.
        fixture.restore_current_catalog_role(role_guard.as_mut().expect("active catalog role guard")).await.expect("verified actual Completion restoration");
        assert_eq!(catalog.entry(&current_id.to_string()).as_ref(), Some(&original_entry));
        assert!(fixture.restore_current_catalog_role(role_guard.as_mut().expect("active catalog role guard")).await.is_err(), "already-restored guard is stale");
        assert_eq!(during_role_drift.expect("canonical snapshot during catalog-only drift"), before);
        let role_drift_response = role_drift_response.expect("actual role drift HTTP observation");
        assert_eq!(role_drift_response.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        let (_, role_drift_code, _) = error_body(role_drift_response).await;
        assert_eq!(role_drift_code, handshake_core::api::model_runtime_registry::MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE);
        let restored_response = account.client().get(format!("{base}/model-runtime/registry"))
            .send().await.expect("actual registry after role restoration");
        assert_eq!(restored_response.status(), reqwest::StatusCode::OK);
        let fallback_guard = fixture.project_current_catalog_role().await.expect("second finite role projection");
        drop(fallback_guard);
        assert_eq!(catalog.entry(&current_id.to_string()).as_ref(), Some(&original_entry));
        assert_eq!(catalog.runtime_availability_revision(), catalog_revision);
        assert_eq!(fixture.snapshot_default().await.expect("canonical default after role checks"), before);
        // Actual supported distinct Completion A -> B makes the prior A observation stale.
        let stale_revision = fixture.current_selection_revision().expect("actual pre-swap revision");
        let target_id = fixture.target_model_id().expect("actual compatible target loaded and owned");
        selection_restore = Some((base.clone(), current_id));
        let transition = account.client().post(format!("{base}/model-runtime/selection"))
            .json(&selection_request(Uuid::now_v7(), stale_revision, target_id))
            .send().await.expect("actual distinct-model selection CAS");
        assert_eq!(transition.status(), reqwest::StatusCode::OK);
        let after_transition = fixture.snapshot_default().await.expect("canonical selection after A to B");
        assert_eq!(after_transition.runtime_model_id, target_id.to_string());
        assert_eq!(after_transition.selection.selection_revision, stale_revision + 1);
        assert!(fixture.project_current_catalog_role_at_revision(stale_revision).await.is_err(),
            "stale selected A observation must not arm after genuine A to B");
        assert_eq!(catalog.entry(&current_id.to_string()).as_ref(), Some(&original_entry));
        assert_eq!(fixture.snapshot_default().await.expect("canonical selection after stale denial"), after_transition);
        let restore_selection = account.client().post(format!("{base}/model-runtime/selection"))
            .json(&selection_request(Uuid::now_v7(), after_transition.selection.selection_revision, current_id))
            .send().await.expect("genuine B to A restoration");
        assert_eq!(restore_selection.status(), reqwest::StatusCode::OK);
        let before = fixture.snapshot_default().await.expect("canonical selection after B to A restoration");
        assert_eq!(before.runtime_model_id, current_id.to_string());
        assert_eq!(before.selection.artifact_sha256, local.sha256);
        assert_eq!(before.selection.selection_revision, stale_revision + 2);
        selection_restore = None;
        let original_owner = fixture.observe_ownership(current_id, local.sha256, RuntimeBinding::Candle)
            .await.expect("canonical real-owned START before integrity fault");
        let ledger = SurrealProcessLedgerStore::new(storage.clone());
        let exact_process = original_owner.resource_scope.clone();
        integrity_restore = Some((exact_process.clone(), current_id.as_uuid(), original_owner.event_ledger_event_id.clone()));
        ledger.test_set_inspection_event_link(&exact_process, current_id.as_uuid(), Some(RecordId::new(
            "kernel_event_ledger", format!("fixture-unresolved-process-event-{}", Uuid::now_v7()),
        ))).await.expect("owned integrity fixture changes only the ledger reference");
        let corrupted = account.client().get(format!("{base}/model-runtime/process-ownership/{current_id}"))
            .send().await;
        // Restore the exact original reference before assertions or shutdown; the canonical START
        // event is never deleted/rewritten and no new ownership record is manufactured.
        ledger.test_set_inspection_event_link(&exact_process, current_id.as_uuid(), Some(original_owner.event_ledger_event_id.clone()))
            .await.expect("restore actual owned START reference");
        integrity_restore = None;
        let restored = fixture.observe_ownership(current_id, local.sha256, RuntimeBinding::Candle)
            .await.expect("canonical real-owned START after exact restoration");
        assert_eq!(restored, original_owner);
        let corrupted = corrupted.expect("real-owned corrupted-link HTTP request");
        assert_eq!(corrupted.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
        let (_, code, detail) = error_body(corrupted).await;
        assert_eq!(code, handshake_core::api::model_runtime_registry::MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE);
        assert!(!detail.contains(&current_id.to_string()));
        assert!(!detail.contains(&hex::encode(local.sha256)));
        assert_eq!(fixture.snapshot_default().await.expect("actual default unchanged after integrity fault"), before);
    }).catch_unwind().await;
    // Cleanup runs after assertion unwinds as well as successful observations.
    let mut cleanup_errors = Vec::new();
    if let Some((base, original_id)) = selection_restore.take() {
        let settlement = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            let observed = fixture.snapshot_default().await?;
            if observed.runtime_model_id != original_id.to_string() || observed.selection.artifact_sha256 != local.sha256 {
                let response = account.client().post(format!("{base}/model-runtime/selection"))
                    .json(&selection_request(Uuid::now_v7(), observed.selection.selection_revision, original_id))
                    .send().await.map_err(|error| error.to_string())?;
                if response.status() != reqwest::StatusCode::OK { return Err("actual default restoration request was rejected".to_owned()); }
            }
            let restored = fixture.snapshot_default().await?;
            if restored.runtime_model_id != original_id.to_string() || restored.selection.artifact_sha256 != local.sha256 {
                return Err("actual A default restoration was not verified".to_owned());
            }
            Ok::<_, String>(())
        }).await;
        if !matches!(&settlement, Ok(Ok(()))) {
            cleanup_errors.push(format!("genuine A default restoration not established: {settlement:?}"));
        }
    }
    if let Some(guard) = role_guard.as_mut() {
        // Only an explicit serialized restore qualifies. Drop below is a best-effort fallback.
        match tokio::time::timeout(std::time::Duration::from_secs(10), fixture.settle_current_catalog_role(guard)).await {
            Ok(Ok(())) => {},
            result => cleanup_errors.push(format!("serialized catalog role restoration not established: {result:?}")),
        }
    }
    drop(role_guard.take());
    if let Some((scope, process_id, event_id)) = integrity_restore.take() {
        let ledger = SurrealProcessLedgerStore::new(storage.clone());
        match tokio::time::timeout(std::time::Duration::from_secs(10),
            ledger.test_set_inspection_event_link(&scope, process_id, Some(event_id))).await {
            Ok(Ok(())) => {},
            result => cleanup_errors.push(format!("original START link restoration not established: {result:?}")),
        }
    }
    if let Some(server) = server_task.as_mut() {
        server.abort();
        match tokio::time::timeout(std::time::Duration::from_secs(5), server).await {
            Ok(Ok(())) => { server_task = None; },
            Ok(Err(error)) if error.is_cancelled() => { server_task = None; },
            result => cleanup_errors.push(format!("server join not established: {result:?}")),
        }
    }
    match tokio::time::timeout(std::time::Duration::from_secs(35),
        fixture.shutdown(std::time::Duration::from_secs(30))).await {
        Ok(Ok(())) => {},
        result => cleanup_errors.push(format!("real-model shutdown/drain not established: {result:?}")),
    }
    if !cleanup_errors.is_empty() {
        // Preserve the actual runtime owner, lease, store and recorder until settlement is known.
        let _retained = Box::leak(Box::new((fixture, allocator, recorder_root, storage, server_task)));
        panic!("actual-model cleanup failed; original test panic={}; retained actual owners: {cleanup_errors:?}", outcome.is_err());
    }
    if let Err(panic) = outcome { std::panic::resume_unwind(panic); }
    let cleanup = allocator.cleanup().await.expect("actual-model isolated-store cleanup");
    assert!(cleanup.database_absent && cleanup.namespace_absent_after_reopen);
}