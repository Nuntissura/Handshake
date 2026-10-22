mod surreal_test_store_support;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use futures::FutureExt;
use handshake_core::api::{authority, stage, workspaces};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::Database;
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::AccountBootstrapState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;



use surreal_test_store_support::IsolatedSurrealTestStore;

static CHANNEL_LOCK: Mutex<()> = Mutex::new(());

struct NativeChannel {
    _lock: MutexGuard<'static, ()>,
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    token: String,
}

impl NativeChannel {
    fn create(root: &Path) -> Self {
        let lock = CHANNEL_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let directory = tempfile::tempdir_in(root).expect("own native binding under artifact root");
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let path = directory.path().join("native-binding.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&stage::current_process_native_session_binding(&token))
                .expect("serialize actual current process binding"),
        )
        .expect("write owned channel binding");
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        Self { _lock: lock, _directory: directory, previous, token }
    }
}

impl Drop for NativeChannel {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous),
            None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
        }
    }
}

struct PublicWorkspace {
    session_token: String,
    exact: ExactResourceScopeAttribution,
}

type PanicPayload = Box<dyn std::any::Any + Send>;

struct OwnedSession {
    token: String,
    identity: Value,
}

struct CleanupFailure {
    stage: &'static str,
    cause: PanicPayload,
}

struct CleanupReport {
    failures: Vec<CleanupFailure>,
    unresolved_sessions: Vec<OwnedSession>,
}

struct FixtureFailure {
    primary: Option<PanicPayload>,
    cleanup: CleanupReport,
}

struct PublicFixture {
    isolated: IsolatedSurrealTestStore,
    bootstrap: Option<AccountBootstrapState>,
    database: Option<Arc<dyn Database>>,
    capabilities: Arc<CapabilityRegistry>,
    channel: NativeChannel,
    account_name: String,
    password: String,
    sessions: Vec<OwnedSession>,
}

impl PublicFixture {
    async fn create() -> Self {
        static TRACING: std::sync::Once = std::sync::Once::new();
        TRACING.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter("handshake_core::resource_authority=error,handshake_core::local_account_vault=error")
                .with_writer(std::io::stderr)
                .try_init()
                .expect("install public session failure diagnostics");
        });
        let root = surreal_test_store_support::canonical_artifacts_root_for_proof()
            .expect("verified artifact root");
        let owner = root.join("WP-KERNEL-012")
            .join("MT-174").join("chat");
        assert!(owner.starts_with(&root), "owned output stays below the verified root");
        std::fs::create_dir_all(&owner).expect("create only the assigned artifact descendant");
        let isolated = IsolatedSurrealTestStore::create_in(&owner).await.expect("own isolated durable store");
        let channel = NativeChannel::create(&owner);
        let capabilities = Arc::new(CapabilityRegistry::new());
        capabilities.profile_by_id("Operator").expect("canonical Operator capabilities");
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(isolated.storage().clone()));
        let bootstrap = AccountBootstrapState::new(
            database.clone(), isolated.storage().clone(), capabilities.clone(),
        );
        Self {
            isolated, bootstrap: Some(bootstrap), database: Some(database), capabilities, channel,
            account_name: format!("chat-capture-proof-{}", Uuid::now_v7().simple()),
            password: format!("chat-capture-proof-password-{}", Uuid::new_v4()),
            sessions: Vec::new(),
        }
    }

    fn router(&self) -> Router {
        self.activation_router().0
    }

    fn activation_router(&self) -> (Router, tokio::sync::mpsc::Receiver<authority::WorkspaceActivationRequest>) {
        let state = self.bootstrap.as_ref().expect("open bootstrap").clone();
        let (activation, receiver) = tokio::sync::mpsc::channel(1);
        (authority::bootstrap_routes(state.clone(), activation)
            .merge(workspaces::bootstrap_routes(state)), receiver)
    }

    async fn login(&mut self) -> (String, Value) {
        let router = self.router();
        let (status_code, status) = dispatch(&router, "GET", "/authority/setup", &self.channel.token, None, Value::Null).await;
        assert_eq!(status_code, StatusCode::OK, "public setup status");
        if status["setup_required"].as_bool().expect("typed setup status") {
            let (status, _) = dispatch(&router, "POST", "/authority/setup", &self.channel.token, None,
                json!({"account_name": self.account_name, "password": self.password})).await;
            assert_eq!(status, StatusCode::OK, "public one-time account setup");
        }
        let (status, credential) = dispatch(&router, "POST", "/authority/login", &self.channel.token, None,
            json!({"account_name": self.account_name, "password": self.password})).await;
        assert_eq!(status, StatusCode::OK, "public password authentication");
        let (status, session) = dispatch(&router, "POST", "/authority/session", &self.channel.token, None,
            json!({"account_id": field(&credential,"account_id"), "principal_id": field(&credential,"principal_id"),
                "access_space_id": field(&credential,"access_space_id"), "authentication_token": field(&credential,"token")})).await;
        assert_eq!(status, StatusCode::OK, "public durable session exchange");
        let token = field(&session, "session_token").to_owned();
        self.sessions.push(OwnedSession { token: token.clone(), identity: session.clone() });
        for key in ["account_id", "principal_id", "access_space_id"] {
            assert_eq!(field(&credential, key), field(&session, key));
        }
        (token, session)
    }

    async fn workspace(&mut self) -> PublicWorkspace {
        let (session_token, session) = self.login().await;
        let (status, workspace) = dispatch(&self.router(), "POST", "/workspaces", &self.channel.token,
            Some(&session_token), json!({"name": "Operator Chat capture restoration proof"})).await;
        assert_eq!(status, StatusCode::CREATED, "canonical workspace and grant producer");
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(uuid(&session, "account_id")),
            ActorPrincipalId::from_uuid(uuid(&session, "principal_id")),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(uuid(&session, "session_id")))
        .with_access_space(AccessSpaceRef::from_uuid(uuid(&session, "access_space_id")))
        .with_workspace(WorkspaceScopeRef::new(field(&workspace, "id")).expect("actual workspace ID"));
        PublicWorkspace { session_token, exact: ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("all five fields came from public producers") }
    }

    async fn logout(&mut self, token: &str) {
        let (status, _) = dispatch(&self.router(), "POST", "/authority/logout", &self.channel.token,
            Some(token), Value::Null).await;
        assert_eq!(status, StatusCode::OK, "revoke session and remove its vault credential");
        self.sessions.retain(|session| session.token != token);
    }

    async fn authorized_operation(
        &self, workspace: &PublicWorkspace,
        action: handshake_core::storage::surreal::resource_authority::ResourceAction,
        capability_id: &str,
    ) -> handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation {
        use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceKind};
        handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation::authorize(
            self.isolated.storage().clone(), AuthorizationRequest {
                session_token: workspace.session_token.clone(),
                channel_binding_hash: Some(sha(self.channel.token.as_bytes())),
                capability_id: capability_id.to_owned(),
                resource_kind: ResourceKind::Workspace,
                external_resource_id: workspace.exact.workspace_id.as_str().to_owned(), action,
            }, workspace.exact.clone(),
        ).await.expect("reauthorize actual workspace grant and native session")
    }

    async fn reopen(&mut self) {
        assert!(self.sessions.is_empty(), "settle vault/session custody before durable restart");
        let identity = (self.isolated.store_path().to_owned(), self.isolated.namespace().to_owned(),
            self.isolated.database().to_owned());
        drop(self.bootstrap.take());
        drop(self.database.take());
        self.isolated.reopen().await.expect("all derived clones dropped before same-store reopen");
        assert_eq!(identity, (self.isolated.store_path().to_owned(), self.isolated.namespace().to_owned(),
            self.isolated.database().to_owned()));
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(self.isolated.storage().clone()));
        self.bootstrap = Some(AccountBootstrapState::new(database.clone(), self.isolated.storage().clone(),
            self.capabilities.clone()));
        self.database = Some(database);
    }

    async fn cleanup(&mut self) -> CleanupReport {
        let mut failures = Vec::new();
        let tokens: Vec<_> = self.sessions.iter().map(|session| session.token.clone()).collect();
        for token in tokens {
            if let Err(cause) = std::panic::AssertUnwindSafe(self.logout(&token)).catch_unwind().await {
                failures.push(CleanupFailure { stage: "public logout", cause });
            }
        }
        for (stage, result) in [
            ("bootstrap handles", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drop(self.bootstrap.take());
            }))),
            ("database handles", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drop(self.database.take());
            }))),
        ] {
            if let Err(cause) = result { failures.push(CleanupFailure { stage, cause }); }
        }
        match std::panic::AssertUnwindSafe(self.isolated.cleanup()).catch_unwind().await {
            Ok(Ok(_)) => {},
            Ok(Err(error)) => failures.push(CleanupFailure { stage: "isolated store", cause: Box::new(error) }),
            Err(cause) => failures.push(CleanupFailure { stage: "isolated store", cause }),
        }
        CleanupReport { failures, unresolved_sessions: std::mem::take(&mut self.sessions) }
    }
}

async fn dispatch(router: &Router, method: &str, path: &str, channel: &str, session: Option<&str>, value: Value)
    -> (StatusCode, Value)
{
    let mut request = Request::builder().method(method).uri(path)
        .header("content-type", "application/json").header("x-hsk-channel-binding-token", channel);
    if let Some(session) = session { request = request.header("x-hsk-session-token", session); }
    let response = router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&value)
        .expect("serialize public request"))).expect("public request")).await.expect("dispatch public route");
    let status = response.status();
    let body = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded response");
    (status, serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned())))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().filter(|value| !value.is_empty()).expect("required actual response field")
}

fn uuid(value: &Value, key: &str) -> Uuid {
    Uuid::parse_str(field(value, key)).expect("actual returned UUID")
}

fn sha(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

use std::time::Duration;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::model_runtime::cloud::{parse_agent_activity_line, AgentActivity, CliKind};
use handshake_core::process_ledger::{
    production_process_sandbox_registry_async, LedgerDrainJoinOutcome, ProcessReclaimRuntime,
    ReclaimResourceScope,
};
use handshake_core::swarm_orchestration::model_lane::ModelLaneMessageKind;
use handshake_core::swarm_orchestration::events::DurableSwarmFrDrain;
use handshake_core::swarm_orchestration::operator_chat::plan_from_activity;
use handshake_core::swarm_orchestration::production_factory::{
    build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle,
    CloudLaneFactoryConfig,
};

struct OriginalChatOwners {
    fixture: PublicFixture,
    runtime: Option<ProcessReclaimRuntime>,
    service: Option<Arc<handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchService>>,
    drain: Option<DurableSwarmFrDrain>,
    recorder: Option<Arc<DuckDbFlightRecorder>>,
    llm: Option<Arc<dyn handshake_core::llm::LlmClient>>,
    app_state: Option<handshake_core::AppState>,
    owned_jobs: Vec<Uuid>,
    settlement_failures: Vec<CleanupFailure>,
    llm_shutdown: LlmShutdownObservation,
    staged_acquisition: Option<StagedAcquisition>,
    parent_observation: Option<handshake_core::swarm_orchestration::operator_chat::OperatorChatParentObservation>,
    acquisition_deadline: Option<tokio::time::Instant>,
}

enum StagedAcquisition {
    First(handshake_core::api::operator_chat::PreparedOperatorChatFirst),
    Parent(handshake_core::swarm_orchestration::operator_chat::OwnedOperatorChatOperation<
        handshake_core::swarm_orchestration::operator_chat::AwaitingParent>),
    Second(handshake_core::api::operator_chat::PreparedOperatorChatSecond),
    Pair(handshake_core::swarm_orchestration::operator_chat::OwnedOperatorChatOperation),
}

impl StagedAcquisition {
    async fn cancel_and_drain(&mut self, failures: &mut Vec<CleanupFailure>) -> bool {
        let cancelled = match self {
            Self::First(_) => return false,
            Self::Parent(operation) => operation.request_cancel(),
            Self::Second(preparation) => preparation.operation_mut().request_cancel(),
            Self::Pair(operation) => operation.request_cancel(),
        };
        let cancellation_returned = cancelled.is_ok();
        if let Err(error) = cancelled {
            failures.push(CleanupFailure { stage: "staged acquisition cancellation", cause: Box::new(error) });
        }
        let drained = match self {
            Self::First(_) => unreachable!("unsubmitted preparation returns before engine settlement"),
            Self::Parent(operation) => operation.drain().await,
            Self::Second(preparation) => preparation.operation_mut().drain().await,
            Self::Pair(operation) => operation.drain().await,
        };
        let drain_returned = drained.is_ok();
        if let Err(error) = drained {
            failures.push(CleanupFailure { stage: "staged acquisition drain", cause: Box::new(error) });
        }
        let child_preparation_settled = match self {
            Self::Second(preparation) => preparation.settle_child_preparation().await,
            _ => true,
        };
        cancellation_returned && drain_returned && child_preparation_settled
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LlmShutdownObservation {
    NotRequested,
    NoClientAcquired,
    ReturnedWithoutWorkerClosureProof,
    Failed,
    Unknown,
}

struct FixtureSessionObservation<'a> {
    unresolved_sessions: &'a [OwnedSession],
    failures: &'a [CleanupFailure],
}

impl OriginalChatOwners {
    fn new(fixture: PublicFixture) -> Self {
        Self {
            fixture,
            runtime: None,
            service: None,
            drain: None,
            recorder: None,
            llm: None,
            app_state: None,
            owned_jobs: Vec::new(),
            settlement_failures: Vec::new(),
            llm_shutdown: LlmShutdownObservation::NotRequested,
            staged_acquisition: None,
            parent_observation: None,
            acquisition_deadline: None,
        }
    }

    async fn observe_llm_shutdown_until(
        &mut self,
        deadline: tokio::time::Instant,
    ) -> &LlmShutdownObservation {
        if self.llm_shutdown != LlmShutdownObservation::NotRequested {
            return &self.llm_shutdown;
        }
        let Some(client) = self.llm.as_ref() else {
            self.llm_shutdown = LlmShutdownObservation::NoClientAcquired;
            return &self.llm_shutdown;
        };
        match std::panic::AssertUnwindSafe(tokio::time::timeout_at(
            deadline, client.shutdown_gracefully(),
        )).catch_unwind().await {
            Ok(Ok(Ok(()))) => {
                self.llm_shutdown = LlmShutdownObservation::ReturnedWithoutWorkerClosureProof;
            }
            Ok(Ok(Err(error))) => {
                self.settlement_failures.push(CleanupFailure {
                    stage: "production LLM shutdown", cause: Box::new(error),
                });
                self.llm_shutdown = LlmShutdownObservation::Failed;
            }
            Ok(Err(error)) => {
                self.settlement_failures.push(CleanupFailure {
                    stage: "production LLM shutdown deadline", cause: Box::new(error),
                });
                self.llm_shutdown = LlmShutdownObservation::Unknown;
            }
            Err(cause) => {
                self.settlement_failures.push(CleanupFailure {
                    stage: "production LLM shutdown panic", cause,
                });
                self.llm_shutdown = LlmShutdownObservation::Unknown;
            }
        }
        &self.llm_shutdown
    }

    async fn settle_fixture_sessions_until(
        &mut self,
        deadline: tokio::time::Instant,
    ) -> FixtureSessionObservation<'_> {
        let tokens: Vec<_> = self.fixture.sessions.iter()
            .map(|session| session.token.clone()).collect();
        for token in tokens {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            match std::panic::AssertUnwindSafe(tokio::time::timeout_at(
                deadline, self.fixture.logout(&token),
            )).catch_unwind().await {
                Ok(Ok(())) => {},
                Ok(Err(error)) => {
                    self.settlement_failures.push(CleanupFailure {
                        stage: "public logout deadline", cause: Box::new(error),
                    });
                }
                Err(cause) => {
                    self.settlement_failures.push(CleanupFailure {
                        stage: "public logout", cause,
                    });
                }
            }
        }
        FixtureSessionObservation {
            unresolved_sessions: &self.fixture.sessions,
            failures: &self.settlement_failures,
        }
    }
}

struct CaptureFailure {
    fixture: FixtureFailure,
    retained_drain: Option<DurableSwarmFrDrain>,
    retained_runtime: Option<ProcessReclaimRuntime>,
    retained_recorder: Option<Arc<DuckDbFlightRecorder>>,
    retained_llm: Option<Arc<dyn handshake_core::llm::LlmClient>>,
    retained_app_state: Option<handshake_core::AppState>,
    unresolved_jobs: Vec<Uuid>,
}

fn model_session_request(
    workspace: &PublicWorkspace,
    model: &handshake_core::model_runtime::ModelCatalogEntry,
    parent: Option<(&str, i32)>,
) -> handshake_core::api::jobs::CreateJobRequest {
    let mut inputs = json!({"model_id": model.model_id, "backend": "local",
        "prompt": "Explain the purpose of the currently selected workspace.",
        "wp_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "mt_id": "MT-017"});
    if let Some((session_id, spawn_depth)) = parent {
        inputs["parent_session_id"] = json!(session_id);
        inputs["spawn_depth"] = json!(spawn_depth.checked_add(1).expect("actual parent depth must permit a child"));
    }
    serde_json::from_value(json!({"job_kind":"model_run", "protocol_id":"protocol-default",
        "workspace_id":workspace.exact.workspace_id.as_str(), "job_inputs": inputs}))
        .expect("unchanged public model-run request shape")
}

#[test]
fn completed_stdout_blocks_preserve_typed_conversation_thought_tool_and_result_payloads() {
    let line = json!({"type":"assistant", "message":{"content":[
        {"type":"text", "text":"captured answer"},
        {"type":"thinking", "thinking":"model-exposed thought"},
        {"type":"tool_use", "id":"tool-call-1", "name":"read_file", "input":{"path":"example.rs"}},
        {"type":"tool_result", "tool_use_id":"tool-call-1", "content":"captured tool result"}
    ]}}).to_string();
    let activities = parse_agent_activity_line(CliKind::ClaudeCode, &line);
    assert_eq!(activities.len(), 4, "one typed activity per completed block");
    assert_eq!(activities[0], AgentActivity::Text { text: "captured answer".into() });
    assert_eq!(activities[1], AgentActivity::Thinking { text: "model-exposed thought".into() });
    assert_eq!(activities[2], AgentActivity::ToolCall {
        name: "read_file".into(), input: json!({"path":"example.rs"}), call_id: Some("tool-call-1".into()),
    });
    let plans: Vec<_> = activities.iter().map(plan_from_activity).collect();
    assert_eq!(plans[0].kind, ModelLaneMessageKind::Status);
    assert_eq!(plans[0].activity_kind, "text");
    assert_eq!(plans[0].payload["text"], "captured answer");
    assert_eq!(plans[1].kind, ModelLaneMessageKind::Status);
    assert_eq!(plans[1].activity_kind, "thinking");
    assert_eq!(plans[1].payload["text"], "model-exposed thought");
    assert_eq!(plans[2].kind, ModelLaneMessageKind::ToolRequest);
    assert!(plans[2].is_tool);
    assert_eq!(plans[2].payload["name"], "read_file");
    assert_eq!(plans[2].payload["input"], json!({"path":"example.rs"}));
    assert_eq!(plans[3].kind, ModelLaneMessageKind::ToolResult);
    assert!(plans[3].is_tool);
    assert_eq!(plans[3].payload["rendered_tool_result"], true);
    assert!(plans[3].payload["text"].as_str().expect("typed tool result").contains("captured tool result"));
    for plan in plans {
        let bytes = serde_json::to_vec(&plan.payload).expect("serialize typed capture payload");
        let reloaded: Value = serde_json::from_slice(&bytes).expect("round-trip actual payload bytes");
        assert_eq!(reloaded, plan.payload);
        assert_eq!(sha(&bytes), sha(&serde_json::to_vec(&reloaded).expect("same payload encoding")));
    }
    for lifecycle in [json!({"type":"system","subtype":"init"}), json!({"type":"result","result":"done"})] {
        assert!(parse_agent_activity_line(CliKind::ClaudeCode, &lifecycle.to_string()).is_empty(),
            "pure lifecycle envelopes must not become duplicate captured activities");
    }
    let malformed = "not-json: retain stdout bytes";
    assert_eq!(parse_agent_activity_line(CliKind::ClaudeCode, malformed),
        vec![AgentActivity::Other { raw: malformed.into() }]);
}

#[tokio::test]
async fn scoped_production_factory_retains_actual_scope_and_paired_durable_drain() {
    let mut owners = OriginalChatOwners::new(PublicFixture::create().await);
    let OriginalChatOwners {
        fixture, runtime, service, drain, recorder, llm, app_state, owned_jobs,
        staged_acquisition, parent_observation, acquisition_deadline, settlement_failures, ..
    } = &mut owners;
    let result = std::panic::AssertUnwindSafe(async {
        let workspace = fixture.workspace().await;
        let storage = fixture.isolated.storage().clone();
        let reclaim = ReclaimResourceScope {
            account_uuid: Uuid::parse_str(&workspace.exact.owner_account_id.to_string()).expect("actual account"),
            actor_uuid: Uuid::parse_str(&workspace.exact.actor_principal_id.to_string()).expect("actual principal"),
            session_uuid: Uuid::parse_str(&workspace.exact.authenticated_session_id.to_string()).expect("actual session"),
            access_space_uuid: Uuid::parse_str(&workspace.exact.access_space_id.to_string()).expect("actual Space"),
            workspace_id: workspace.exact.workspace_id.as_str().to_owned(),
        };
        let registry = production_process_sandbox_registry_async().await.expect("production sandbox registry");
        *runtime = Some(ProcessReclaimRuntime::production(storage.clone(), reclaim, None, registry,
            format!("capture-proof:{}", fixture.isolated.namespace()), Duration::from_secs(30))
            .await.expect("actual retained process-ledger runtime"));
        *recorder = Some(Arc::new(DuckDbFlightRecorder::new_on_path(
            &fixture.isolated.scope_path().join("operator-chat-flight-recorder.duckdb"), 7)
            .expect("owned durable Flight Recorder")));
        let process = runtime.as_ref().expect("retained process runtime");
        *llm = Some(handshake_core::llm::boot::resolve_default_llm_client(
            recorder.as_ref().expect("owned recorder").clone(),
            Some(process.ledger().ledger()),
            Some(handshake_core::model_runtime::ScopedModelRegistryAuthority::new(
                handshake_core::model_runtime::ModelRegistryStore::new(storage.clone()),
                workspace.exact.clone(),
            )),
            Some(process.runtime_instance().clone()),
        ).await);
        let catalog = llm.as_ref().expect("actual production provider resolution").model_catalog();
        let (actual_service, actual_drain) = build_scoped_operator_chat_launch_service_with_sandbox_registry_and_lifecycle(
            storage.clone(), recorder.as_ref().expect("owned recorder").clone(), catalog.clone(),
            process.ledger(), process.reclaim(), CloudLaneFactoryConfig::unconfigured(),
            Some(process.sandbox_registry()), storage.resource_access_lifecycle(), workspace.exact.clone(), Uuid::now_v7(),
        );
        *service = Some(actual_service);
        *drain = Some(actual_drain);
        let (activation_router, mut activation_receiver) = fixture.activation_router();
        let operation_id = Uuid::now_v7();
        let activation_input = json!({"operation_id": operation_id,
            "workspace_id": workspace.exact.workspace_id.as_str()});
        let request = dispatch(&activation_router, "POST", "/authority/session", &fixture.channel.token,
            Some(&workspace.session_token), activation_input);
        let publication = async {
            let activation = activation_receiver.recv().await.expect("public activation producer request");
            assert_eq!(activation.scope().exact(), &workspace.exact);
            assert_eq!(activation.operation_id(), operation_id);
            activation.authorize_for_publication().await.expect("reauthorize actual activation request");
            let actual_service = service.as_ref().expect("factory service retained before publication").clone();
            let mut published_service = None;
            activation.acknowledge_with(process.runtime_instance(), || {
                published_service = Some(actual_service);
                Ok(())
            }).await.expect("durable activation and protected service publication");
            published_service.expect("publication owns the actual factory service")
        };
        let ((activation_status, acknowledgement), published_service) = tokio::time::timeout(
            Duration::from_secs(30), async { tokio::join!(request, publication) },
        ).await.expect("public activation request and publication must both settle");
        assert_eq!(activation_status, StatusCode::OK, "actual public activation acknowledgement");
        assert_eq!(acknowledgement["operation_id"], json!(operation_id));
        *service = Some(published_service);
        let store = service.as_ref().expect("actual scoped service").coordinator().model_lane_store()
            .expect("production factory owns a ModelLane store");
        assert_eq!(store.access().exact_read_scope(), Some(&workspace.exact),
            "factory must carry the publicly acquired five-field scope unchanged");
        assert_ne!(workspace.exact.workspace_id.as_str(), "", "actual workspace producer ran");
        let selected = catalog.as_ref().expect("production boot must expose a local catalog")
            .list().into_iter().find(|entry| entry.ready && entry.default_selectable
                && entry.runtime_role == handshake_core::model_runtime::ModelRuntimeRole::Completion)
            .expect("a genuinely loaded completion model must be available");
        *app_state = Some(handshake_core::AppState {
            storage: fixture.database.as_ref().expect("open database").clone(),
            surreal: storage.clone(),
            flight_recorder: recorder.as_ref().expect("actual durable recorder").clone(),
            diagnostics: recorder.as_ref().expect("actual diagnostics recorder").clone(),
            llm_client: llm.as_ref().expect("actual production provider").clone(),
            capability_registry: fixture.capabilities.clone(),
            session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new_from_env()),
        });
        let state = app_state.as_ref().expect("retain actual acquisition state");
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().expect("literal content type"));
        headers.insert("x-hsk-channel-binding-token", fixture.channel.token.parse().expect("actual channel header"));
        headers.insert("x-hsk-session-token", workspace.session_token.parse().expect("actual session header"));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        *acquisition_deadline = Some(deadline);
        *staged_acquisition = Some(StagedAcquisition::First(
            handshake_core::api::operator_chat::prepare_operator_chat_first(
                Arc::new(state.clone()), headers.clone(), model_session_request(&workspace, &selected, None), deadline,
            ),
        ));
        if let Some(StagedAcquisition::First(preparation)) = staged_acquisition.as_mut() {
            preparation.prepare().await;
        }
        let Some(StagedAcquisition::First(preparation)) = staged_acquisition.take() else {
            unreachable!("original first preparation is retained before synchronous transfer")
        };
        *staged_acquisition = Some(match preparation.into_operation() {
            Ok(operation) => StagedAcquisition::Parent(operation),
            Err(preparation) => StagedAcquisition::First(preparation),
        });
        let Some(StagedAcquisition::Parent(operation)) = staged_acquisition.as_mut() else {
            panic!("original first preparation remains unresolved")
        };
        owned_jobs.push(operation.first_job_id());
        operation.submit_first_prepared().await.expect("reserve and submit the actual parent under the original deadline");
        *parent_observation = Some(operation.first_parent_observation().await
            .expect("canonical parent readback and retained Active pin"));
        let parent = parent_observation.as_ref().expect("retain actual parent observation through launch");
        assert!(parent.is_active(), "the actual parent must remain Active");
        let second = model_session_request(&workspace, &selected, Some((parent.session_id(), parent.spawn_depth())));
        let Some(StagedAcquisition::Parent(operation)) = staged_acquisition.take() else {
            unreachable!("original parent is retained before synchronous extension")
        };
        *staged_acquisition = Some(StagedAcquisition::Second(
            handshake_core::api::operator_chat::prepare_operator_chat_second(operation, headers, second),
        ));
        if let Some(StagedAcquisition::Second(preparation)) = staged_acquisition.as_mut() {
            preparation.prepare().await;
        }
        let Some(StagedAcquisition::Second(preparation)) = staged_acquisition.take() else {
            unreachable!("original child preparation is retained before synchronous transfer")
        };
        *staged_acquisition = Some(match preparation.into_operation() {
            Ok(operation) => StagedAcquisition::Pair(operation),
            Err(preparation) => StagedAcquisition::Second(preparation),
        });
        let Some(StagedAcquisition::Pair(operation)) = staged_acquisition.as_mut() else {
            panic!("original child preparation and parent remain unresolved")
        };
        use handshake_core::swarm_orchestration::operator_chat::{OperatorChatAdmissionSlot, OperatorChatInitialState, OperatorChatWriterResult};
        let owner_job_id = operation.job_id(OperatorChatAdmissionSlot::Second);
        owned_jobs.push(owner_job_id);
        operation.submit_second_prepared().await.expect("reserve and submit the actual child under the same deadline");
        let initial = operation.settle_initial().await;
        for slot in [OperatorChatAdmissionSlot::First, OperatorChatAdmissionSlot::Second] {
            let slot = || match &slot {
                OperatorChatAdmissionSlot::First => OperatorChatAdmissionSlot::First,
                OperatorChatAdmissionSlot::Second => OperatorChatAdmissionSlot::Second,
            };
            assert!(matches!(initial.state(slot()), OperatorChatInitialState::Observed));
            assert!(initial.observation_error(slot()).is_none() && initial.rejected(slot()).is_none());
            assert!(matches!(initial.primary(slot()), Some(OperatorChatWriterResult::WriteReturnedCreated | OperatorChatWriterResult::WriteReturnedAlreadyCreated)));
        }
        drop(initial);
        let owner = state.session_registry.get_model_session_by_job_id(state, owner_job_id).await
            .expect("hydrate only the actual admitted child job's server-issued session");
        let lineage = handshake_core::api::operator_chat::resolve_operator_chat_lineage(
            &state.session_registry, &owner.session_id,
        ).await.expect("public producer must have made both real sessions active with valid depth");
        let selection = handshake_core::swarm_orchestration::operator_chat::OperatorChatLaunchRequest {
            lane_kind: handshake_core::swarm_orchestration::operator_chat::OperatorChatLaneKind::Local,
            model_id: selected.model_id,
            cloud_provider: None, cli_provider: None,
            working_dir: fixture.isolated.scope_path().to_string_lossy().into_owned(),
            worktree_id: None,
            prompt: "Explain the purpose of the currently selected workspace.".to_owned(),
            owner_session_id: owner.session_id,
            work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
            micro_task_id: Some("MT-017".into()),
        }.into_governed_selection(lineage.owner_session_id, lineage.parent_session_id);
        let launch_authority = fixture.authorized_operation(&workspace,
            handshake_core::storage::surreal::resource_authority::ResourceAction::Create, "fs.write").await;
        let launched = launch_authority.run(async {
            Ok(service.as_ref().expect("actual factory service").launch_scoped(&selection, &workspace.exact).await)
        }).await.expect("original authorized workspace operation")
            .expect("genuine selected runtime launch and stdout capture");
        assert!(launched.captured_message_count > 0, "the actual launched runtime must produce captured activity");
        let replay_authority = fixture.authorized_operation(&workspace,
            handshake_core::storage::surreal::resource_authority::ResourceAction::Read, "fs.read").await;
        let replay = replay_authority.run(store.replay_run(&launched.run_id)).await
            .expect("canonical replay of the actual returned run");
        assert_eq!(replay.run.run_id, launched.run_id);
        let captured: Vec<_> = replay.messages.iter().filter(|message|
            message.diagnostic_payload["turn_role"] == "model").collect();
        assert_eq!(captured.len(), launched.captured_message_count,
            "real captured activity and canonical replay must agree");
        assert_eq!(replay.messages.iter().filter(|message|
            message.diagnostic_payload["turn_role"] == "operator"
                && message.diagnostic_payload["capture"]["text"] == selection.prompt).count(), 1,
            "the actual operator prompt must have one canonical capture");
        let mut activity_identities = Vec::new();
        for message in captured {
            assert_eq!(replay.messages.iter().filter(|row|
                row.from_lane_id == message.from_lane_id
                    && row.diagnostic_payload["turn_role"] == "model"
                    && row.diagnostic_payload["ordered_index"] == message.diagnostic_payload["ordered_index"]
                    && row.diagnostic_payload["activity_kind"] == message.diagnostic_payload["activity_kind"]
            ).count(), 1, "each completed activity must have exactly one canonical message");
            assert!(!message.event_ledger_event_id.is_empty());
            assert!(!message.payload_ref.is_empty());
            assert_eq!(message.payload_sha256.len(), 64);
            let lane = replay.lanes.iter().find(|lane| lane.lane_id == message.from_lane_id)
                .expect("actual canonical capture sender lane");
            let events = handshake_core::flight_recorder::FlightRecorder::list_session_scoped_events(
                recorder.as_ref().expect("owned durable Flight Recorder").as_ref(),
                &lane.model_session_id, None, None,
            ).await.expect("read actual owned recorder rows for the captured runtime session");
            let matched: Vec<_> = events.iter().filter(|event|
                event.payload["type"] == "agent_activity"
                    && event.payload["ordered_index"] == message.diagnostic_payload["ordered_index"]
                    && event.payload["activity_kind"] == message.diagnostic_payload["activity_kind"]
            ).collect();
            assert_eq!(matched.len(), 1, "each completed model activity must have exactly one ancillary business event");
            activity_identities.push((lane.model_session_id.clone(),
                message.diagnostic_payload["ordered_index"].clone(),
                message.diagnostic_payload["activity_kind"].clone(), matched[0].event_id));
        }
        assert_eq!(replay_authority.run(store.replay_run(&launched.run_id)).await
            .expect("repeat actual canonical replay"), replay);
        for (session_id, ordered_index, activity_kind, event_id) in activity_identities {
            let reread = handshake_core::flight_recorder::FlightRecorder::list_session_scoped_events(
                recorder.as_ref().expect("owned durable Flight Recorder").as_ref(),
                &session_id, None, None,
            ).await.expect("reread actual ancillary recorder after canonical replay");
            let matched: Vec<_> = reread.iter().filter(|event|
                event.payload["type"] == "agent_activity"
                    && event.payload["ordered_index"] == ordered_index
                    && event.payload["activity_kind"] == activity_kind
            ).collect();
            assert_eq!(matched.len(), 1, "canonical replay must not duplicate a business activity under any event UUID");
            assert_eq!(matched[0].event_id, event_id, "canonical replay must preserve the original ancillary event identity");
        }
    }).catch_unwind().await;
    let parent_pin_released = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(parent_observation.take());
    })) {
        Ok(()) => true,
        Err(cause) => {
            settlement_failures.push(CleanupFailure { stage: "actual parent observation release", cause });
            false
        }
    };
    let staged_settled = match staged_acquisition.as_mut() {
        None => true,
        Some(acquisition) => match std::panic::AssertUnwindSafe(acquisition.cancel_and_drain(settlement_failures))
            .catch_unwind().await {
                Ok(settled) => settled,
                Err(cause) => {
                    settlement_failures.push(CleanupFailure { stage: "staged acquisition settlement panic", cause });
                    false
                }
            },
    };
    if !parent_pin_released || !staged_settled {
        eprintln!("Operator Chat FAILED_INCOMPLETE: original staged owner and complete fixture retained; no dependent cleanup or frame release");
        let _retained_primary = &result;
        std::future::pending::<()>().await;
    }
    drop(service.take());
    let mut failures = Vec::new();
    failures.append(settlement_failures);
    let mut unresolved_jobs = Vec::new();
    if let Some(state) = app_state.as_ref() {
        for job_id in owned_jobs.drain(..).rev() {
            match std::panic::AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(30),
                handshake_core::workflows::cancel_model_run_job(
                    state, job_id, "MT-017 owned fixture".into(), "settle acquired model-run jobs".into(),
                ),
            )).catch_unwind().await {
                Ok(Ok(Ok(()))) => {},
                Ok(Ok(Err(error))) => {
                    failures.push(CleanupFailure { stage: "acquired model-run cancellation", cause: Box::new(error) });
                    unresolved_jobs.push(job_id);
                },
                Ok(Err(error)) => {
                    failures.push(CleanupFailure { stage: "acquired model-run cancellation", cause: Box::new(error) });
                    unresolved_jobs.push(job_id);
                },
                Err(cause) => {
                    failures.push(CleanupFailure { stage: "acquired model-run cancellation", cause });
                    unresolved_jobs.push(job_id);
                },
            }
        }
    }
    let retained_app_state = if unresolved_jobs.is_empty() { drop(app_state.take()); None } else { app_state.take() };
    let mut retained_llm = None;
    if let Some(owned) = llm.take() {
        match std::panic::AssertUnwindSafe(tokio::time::timeout(
            Duration::from_secs(30), owned.shutdown_gracefully(),
        )).catch_unwind().await {
            Ok(Ok(Ok(()))) => {},
            Ok(Ok(Err(error))) => {
                failures.push(CleanupFailure { stage: "production LLM shutdown", cause: Box::new(error) });
                retained_llm = Some(owned);
            },
            Ok(Err(error)) => {
                failures.push(CleanupFailure { stage: "production LLM shutdown", cause: Box::new(error) });
                retained_llm = Some(owned);
            },
            Err(cause) => {
                failures.push(CleanupFailure { stage: "production LLM shutdown", cause });
                retained_llm = Some(owned);
            },
        }
    }
    let mut retained_drain = None;
    let mut retained_runtime = None;
    if let Some(owned) = drain.take() {
        let settled = std::panic::AssertUnwindSafe(owned.drain_and_join(Duration::from_secs(30))).catch_unwind().await;
        match settled {
            Ok(true) => {},
            Ok(false) => {
                failures.push(CleanupFailure { stage: "paired swarm Flight Recorder drain", cause: Box::new(false) });
                retained_drain = Some(owned);
            },
            Err(cause) => {
                failures.push(CleanupFailure { stage: "paired swarm Flight Recorder drain", cause });
                retained_drain = Some(owned);
            },
        }
    }
    if let Some(owned) = runtime.take() {
        let settled = std::panic::AssertUnwindSafe(owned.shutdown_and_drain(Duration::from_secs(30))).catch_unwind().await;
        match settled {
            Ok(report) if report.reclaim_task_quiesced && report.lease_released
                && matches!(report.ledger, LedgerDrainJoinOutcome::Flushed) => {},
            Ok(report) => {
                failures.push(CleanupFailure { stage: "retained process runtime", cause: Box::new(report) });
                retained_runtime = Some(owned);
            },
            Err(cause) => {
                failures.push(CleanupFailure { stage: "retained process runtime", cause });
                retained_runtime = Some(owned);
            },
        }
    }
    let retained_recorder = if retained_drain.is_some() || retained_runtime.is_some() || retained_llm.is_some() {
        recorder.take()
    } else {
        drop(recorder.take());
        None
    };
    let mut cleanup = fixture.cleanup().await;
    cleanup.failures.extend(failures);
    if !cleanup.failures.is_empty() || !cleanup.unresolved_sessions.is_empty() {
        let stages: Vec<_> = cleanup.failures.iter().map(|failure| failure.stage).collect();
        eprintln!("Operator Chat cleanup INCOMPLETE: stages={stages:?}; unresolved_sessions={}; retained_factory={}",
            !cleanup.unresolved_sessions.is_empty(), retained_drain.is_some() || retained_runtime.is_some());
        std::panic::panic_any(CaptureFailure {
            fixture: FixtureFailure { primary: result.err(), cleanup },
            retained_drain, retained_runtime, retained_recorder, retained_llm,
            retained_app_state, unresolved_jobs,
        });
    }
    if let Err(primary) = result { std::panic::resume_unwind(primary); }
}
