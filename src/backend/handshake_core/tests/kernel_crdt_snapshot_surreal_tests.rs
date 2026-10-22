mod surreal_test_store_support;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use futures::FutureExt;
use handshake_core::api::{authority, stage, workspaces};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::storage::Database;
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::AccountBootstrapState;
use handshake_core::process_ledger::{acquire_embedded_runtime_instance_lease, EmbeddedRuntimeInstanceLease};
use handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation;
use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

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
    runtime_lease: Option<EmbeddedRuntimeInstanceLease>,
    published_router: Option<Router>,
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
            .join("MT-174").join("snapshot");
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
            account_name: format!("crdt-proof-{}", Uuid::now_v7().simple()),
            password: format!("crdt-proof-password-{}", Uuid::new_v4()),
            sessions: Vec::new(),
            runtime_lease: None, published_router: None,
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

    async fn authorized_operation(&self, workspace: &PublicWorkspace, kind: ResourceKind,
        external_id: &str, action: ResourceAction, capability: &str) -> AuthenticatedModelLaneTestOperation {
        AuthenticatedModelLaneTestOperation::authorize(self.isolated.storage().clone(), AuthorizationRequest {
            session_token: workspace.session_token.clone(),
            channel_binding_hash: Some(sha(self.channel.token.as_bytes())),
            capability_id: capability.to_owned(), resource_kind: kind,
            external_resource_id: external_id.to_owned(), action,
        }, workspace.exact.clone()).await.expect("genuine original protected-resource operation")
    }

    async fn activate_documents(&mut self, workspace: &PublicWorkspace, state: handshake_core::AppState) {
        self.runtime_lease = Some(acquire_embedded_runtime_instance_lease(Uuid::now_v7(),
            format!("crdt-proof:{}", self.isolated.namespace())).expect("own actual embedded runtime instance lease"));
        let runtime = self.runtime_lease.as_ref().expect("retained runtime lease").descriptor().clone();
        let prepared = handshake_core::api::knowledge_documents::routes(state);
        let (router, mut receiver) = self.activation_router();
        let operation_id = Uuid::now_v7();
        let request = dispatch(&router, "POST", "/authority/session", &self.channel.token,
            Some(&workspace.session_token), json!({"operation_id":operation_id,
                "workspace_id":workspace.exact.workspace_id.as_str()}));
        let publication_slot = &mut self.published_router;
        let publication = async {
            let activation = receiver.recv().await.expect("public workspace activation request");
            assert_eq!(activation.scope().exact(), &workspace.exact);
            assert_eq!(activation.operation_id(), operation_id);
            activation.authorize_for_publication().await.expect("reauthorize real document router publication");
            activation.acknowledge_with(&runtime, || { *publication_slot = Some(prepared); Ok(()) })
                .await.expect("durable activation publishes actual protected document router");
        };
        let ((status, acknowledgement), ()) = tokio::time::timeout(std::time::Duration::from_secs(30),
            async { tokio::join!(request, publication) }).await.expect("public activation settles");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(acknowledgement["operation_id"], json!(operation_id));
    }

    async fn create_document(&self, workspace: &PublicWorkspace) -> String {
        let request = Request::builder().method("POST").uri("/knowledge/documents")
            .header("content-type", "application/json")
            .header("x-hsk-channel-binding-token", &self.channel.token)
            .header("x-hsk-session-token", &workspace.session_token)
            .header("x-hsk-actor-id", workspace.exact.actor_principal_id.to_string())
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", format!("crdt-document-create:{}", workspace.exact.workspace_id))
            .header("x-hsk-session-run-id", workspace.exact.authenticated_session_id.to_string())
            .body(Body::from(serde_json::to_vec(&json!({"workspace_id":workspace.exact.workspace_id.as_str(),
                "title":"MT-017 canonical CRDT producer"})).expect("public document input"))).expect("public document request");
        let response = self.published_router.as_ref().expect("actually published document router").clone()
            .oneshot(request).await.expect("dispatch genuine document producer");
        assert!(response.status().is_success(), "public document creation must succeed");
        let bytes = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded document response");
        let body: Value = serde_json::from_slice(&bytes).expect("typed document response");
        field(&body["document"], "rich_document_id").to_owned()
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
            Some(&session_token), json!({"name": "CRDT snapshot restoration proof"})).await;
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
        drop(self.published_router.take());
        let tokens: Vec<_> = self.sessions.iter().map(|session| session.token.clone()).collect();
        for token in tokens {
            if let Err(cause) = std::panic::AssertUnwindSafe(self.logout(&token)).catch_unwind().await {
                failures.push(CleanupFailure { stage: "public logout", cause });
            }
        }
        if let Some(lease) = self.runtime_lease.take() {
            match std::panic::AssertUnwindSafe(lease.release()).catch_unwind().await {
                Ok(Ok(())) => {},
                Ok(Err(error)) => failures.push(CleanupFailure { stage: "runtime lease release", cause: Box::new(error) }),
                Err(cause) => failures.push(CleanupFailure { stage: "runtime lease release", cause }),
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
    (status, serde_json::from_slice(&body).expect("public response JSON"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().filter(|value| !value.is_empty()).expect("required actual response field")
}

fn uuid(value: &Value, key: &str) -> Uuid {
    Uuid::parse_str(field(value, key)).expect("actual returned UUID")
}

fn sha(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

struct AuthoredUpdate {
    client_id: u64,
    text: &'static str,
    bytes: Vec<u8>,
    sha256: String,
    vector: Vec<u8>,
}

fn author(client_id: u64, text: &'static str) -> AuthoredUpdate {
    let document = Doc::with_client_id(client_id);
    let body = document.get_or_insert_text("body");
    body.insert(&mut document.transact_mut(), 0, text);
    let transaction = document.transact();
    let bytes = transaction.encode_state_as_update_v1(&StateVector::default());
    AuthoredUpdate { client_id, text, sha256: sha(&bytes), bytes,
        vector: transaction.state_vector().encode_v1() }
}

fn apply(document: &Doc, bytes: &[u8]) {
    document.transact_mut().apply_update(Update::decode_v1(bytes).expect("actual Yjs v1 update"))
        .expect("apply actual update");
}

#[test]
fn real_model_and_operator_yjs_bytes_converge_under_order_and_duplicate_delivery() {
    let updates = [author(101, "local-model"), author(202, "cloud-model"), author(303, "operator")];
    let orders = [[0,1,2], [0,2,1], [1,0,2], [1,2,0], [2,0,1], [2,1,0]];
    let mut expected = None;
    for order in orders {
        let document = Doc::new();
        let body = document.get_or_insert_text("body");
        for index in order {
            let update = &updates[index];
            assert_eq!(sha(&update.bytes), update.sha256);
            let independently_decoded = Doc::new();
            apply(&independently_decoded, &update.bytes);
            assert_eq!(independently_decoded.transact().state_vector().encode_v1(), update.vector);
            assert_eq!(independently_decoded.transact().state_vector().get(&yrs::ClientID::new(update.client_id)), update.text.len() as u32);
            apply(&document, &update.bytes);
            let before_duplicate = document.transact().state_vector().encode_v1();
            apply(&document, &update.bytes);
            assert_eq!(document.transact().state_vector().encode_v1(), before_duplicate);
        }
        let transaction = document.transact();
        let text = body.get_string(&transaction);
        for update in &updates {
            assert_eq!(text.matches(update.text).count(), 1);
        }
        let state = transaction.encode_state_as_update_v1(&StateVector::default());
        let result = (text, transaction.state_vector().encode_v1(), sha(&state));
        if let Some(expected) = &expected { assert_eq!(&result, expected); } else { expected = Some(result); }
        let restored = Doc::new();
        let restored_body = restored.get_or_insert_text("body");
        apply(&restored, &state);
        for update in updates.iter().rev() { apply(&restored, &update.bytes); }
        let restored_transaction = restored.transact();
        let restored_state = restored_transaction.encode_state_as_update_v1(&StateVector::default());
        assert_eq!((restored_body.get_string(&restored_transaction), restored_transaction.state_vector().encode_v1(),
            sha(&restored_state)), expected.clone().expect("nonempty convergence observation"));
    }
}

#[tokio::test]
async fn public_account_workspace_survives_same_durable_store_reopen() {
    let mut fixture = PublicFixture::create().await;
    let result = std::panic::AssertUnwindSafe(async {
    let original = fixture.workspace().await;
    let (status, before) = dispatch(&fixture.router(), "GET", "/workspaces", &fixture.channel.token,
        Some(&original.session_token), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let workspace = original.exact.workspace_id.as_str();
    assert!(before.to_string().contains(workspace), "actual workspace visible before restart");
    fixture.logout(&original.session_token).await;
    fixture.reopen().await;
    let (new_token, session) = fixture.login().await;
    assert_eq!(field(&session,"account_id"), original.exact.owner_account_id.to_string());
    assert_eq!(field(&session,"principal_id"), original.exact.actor_principal_id.to_string());
    assert_eq!(field(&session,"access_space_id"), original.exact.access_space_id.to_string());
    let (status, after) = dispatch(&fixture.router(), "GET", "/workspaces", &fixture.channel.token,
        Some(&new_token), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after, before, "canonical workspace persisted rather than being re-created");
    let (denied, _) = dispatch(&fixture.router(), "GET", "/workspaces", &fixture.channel.token,
        Some(&original.session_token), Value::Null).await;
    assert_eq!(denied, StatusCode::FORBIDDEN, "revoked original credential stays denied across restart");
    fixture.logout(&new_token).await;
    }).catch_unwind().await;
    let cleanup = fixture.cleanup().await;
    if !cleanup.failures.is_empty() || !cleanup.unresolved_sessions.is_empty() {
        let stages: Vec<_> = cleanup.failures.iter().map(|failure| failure.stage).collect();
        eprintln!("CRDT cleanup INCOMPLETE: stages={stages:?}; unresolved_sessions={}",
            !cleanup.unresolved_sessions.is_empty());
        std::panic::panic_any(FixtureFailure { primary: result.err(), cleanup });
    }
    if let Err(primary) = result { std::panic::resume_unwind(primary); }
}

#[tokio::test]
async fn authenticated_document_admits_real_yjs_canonical_producers_and_revoked_operation_is_denied() {
    let mut fixture = PublicFixture::create().await;
    let result = std::panic::AssertUnwindSafe(async {
        let workspace = fixture.workspace().await;
        let recorder = Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_on_path(
            &fixture.isolated.scope_path().join("crdt-document-flight-recorder.duckdb"), 7,
        ).expect("actual owned ancillary recorder for document API"));
        let state = handshake_core::AppState {
            storage: fixture.database.as_ref().expect("open canonical database").clone(),
            surreal: fixture.isolated.storage().clone(),
            flight_recorder: recorder.clone(), diagnostics: recorder.clone(),
            llm_client: Arc::new(handshake_core::llm::DisabledLlmClient::new_recorded(
                "crdt-document-no-inference".into(),
                "canonical Yjs producer fixture does not invoke inference".into(), recorder,
            )),
            capability_registry: fixture.capabilities.clone(),
            session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new(
                handshake_core::workflows::SessionSchedulerConfig::default())),
        };
        fixture.activate_documents(&workspace, state).await;
        let document_id = fixture.create_document(&workspace).await;
        let workspace_create = fixture.authorized_operation(&workspace, ResourceKind::Workspace,
            workspace.exact.workspace_id.as_str(), ResourceAction::Create, "fs.write").await;
        let document_update = fixture.authorized_operation(&workspace, ResourceKind::RichDocument,
            &document_id, ResourceAction::Update, "fs.write").await;
        let scope = ResourceScope::new(workspace.exact.owner_account_id, workspace.exact.actor_principal_id)
            .with_session(workspace.exact.authenticated_session_id)
            .with_access_space(workspace.exact.access_space_id)
            .with_workspace(workspace.exact.workspace_id.clone());
        let store = handshake_core::swarm_orchestration::model_lane::ModelLaneStore::new_scoped_with_lifecycle(
            fixture.isolated.storage().clone(), scope, fixture.isolated.storage().resource_access_lifecycle());
        let label = format!("mt017-{}", Uuid::now_v7().simple());
        let posture = handshake_core::test_harness::crdt_workspace::build_surreal_admissible_crdt_posture_authenticated(
            &store, workspace.exact.workspace_id.as_str(), &label, "model://local/mt018", &document_id,
            &workspace_create, &document_update,
        ).await.expect("actual authenticated lease, Yjs snapshot/update and proposal producers");
        assert_eq!(posture.document_id, document_id);
        assert_eq!(sha(&posture.yjs_update_bytes), posture.update.update_sha256);
        assert_eq!(sha(&posture.approved_diff_bytes), posture.proposal.diff_sha256);
        assert_eq!(posture.approved_diff_sha256, posture.proposal.diff_sha256);
        assert_eq!(posture.snapshot.covered_update_seq + 1, posture.update.update_seq);
        assert_eq!(posture.snapshot.state_vector, posture.update.state_vector_before);
        assert!(!posture.snapshot.event_ledger_event_id.is_empty());
        assert!(!posture.update.event_ledger_event_id.is_empty());
        assert!(!posture.proposal.recorded_event_id.is_empty());
        assert!(!posture.proposal.last_transition_event_id.is_empty());
        Update::decode_v1(&posture.yjs_update_bytes).expect("producer returned genuine encoded Yjs bytes");
        assert_eq!(posture.proposal.applied_update_id.as_deref(), Some(posture.update.update_id.as_str()));
        assert_eq!(posture.proposal.applied_event_id.as_deref(), Some(posture.update.event_ledger_event_id.as_str()));
        let channel_hash = sha(fixture.channel.token.as_bytes());
        let document_events_before = fixture.isolated.storage().list_account_kernel_events_for_aggregate(
            &workspace.session_token, &channel_hash, "knowledge_crdt_document", &posture.crdt_document_id,
        ).await.expect("authenticated canonical CRDT document EventLedger read");
        let proposal_events_before = fixture.isolated.storage().list_account_kernel_events_for_aggregate(
            &workspace.session_token, &channel_hash, "knowledge_crdt_ai_edit_proposal", &posture.proposal.proposal_id,
        ).await.expect("authenticated canonical proposal EventLedger read");
        let snapshot_events: Vec<_> = document_events_before.iter().filter(|event|
            event.event_id == posture.snapshot.event_ledger_event_id).collect();
        let update_events: Vec<_> = document_events_before.iter().filter(|event|
            event.event_id == posture.update.event_ledger_event_id).collect();
        let recorded_events: Vec<_> = proposal_events_before.iter().filter(|event|
            event.event_id == posture.proposal.recorded_event_id).collect();
        let applied_events: Vec<_> = proposal_events_before.iter().filter(|event|
            event.event_id == posture.proposal.last_transition_event_id).collect();
        let actor = handshake_core::kernel::crdt::actor_site::KnowledgeActorIdV1::parse(&posture.actor_id)
            .expect("canonical producer actor");
        let expected_actor = actor.to_kernel_actor();
        for events in [&snapshot_events, &update_events, &recorded_events, &applied_events] {
            assert_eq!(events.len(), 1, "producer reference must independently resolve to one canonical event");
            let event = events[0];
            assert!(event.event_sequence > 0);
            assert_eq!(event.kernel_task_run_id, posture.run_id);
            assert_eq!(event.session_run_id, posture.session_id);
            assert_eq!(event.actor, expected_actor, "canonical CRDT actor must use the actual EventLedger taxonomy mapping");
            assert_eq!(event.source_component, "model_lane_crdt");
            assert_eq!(sha(&handshake_storage_support::canonical_json::canonical_json_bytes(&event.payload)),
                event.payload_hash, "actual canonical event payload must match its retained hash");
            assert_eq!(event.payload["document_id"], json!(document_id));
            assert_eq!(event.payload["crdt_document_id"], json!(posture.crdt_document_id));
        }
        let snapshot_event = snapshot_events[0];
        assert_eq!(snapshot_event.event_type, handshake_core::kernel::KernelEventType::KnowledgeCrdtSnapshotRecorded);
        assert_eq!(snapshot_event.aggregate_type, "knowledge_crdt_document");
        assert_eq!(snapshot_event.aggregate_id, posture.crdt_document_id);
        assert_eq!(snapshot_event.correlation_id, None);
        assert_eq!(snapshot_event.payload["snapshot_id"], json!(posture.snapshot.snapshot_id));
        assert_eq!(snapshot_event.payload["snapshot_sha256"], json!(posture.snapshot.snapshot_sha256));
        assert_eq!(snapshot_event.payload["covered_update_seq"], json!(posture.snapshot.covered_update_seq));
        assert_eq!(snapshot_event.payload["state_vector"], json!(posture.snapshot.state_vector));
        let update_event = update_events[0];
        assert_eq!(update_event.event_type, handshake_core::kernel::KernelEventType::KnowledgeCrdtUpdateRecorded);
        assert_eq!(update_event.aggregate_type, "knowledge_crdt_document");
        assert_eq!(update_event.aggregate_id, posture.crdt_document_id);
        assert_eq!(update_event.correlation_id.as_deref(), Some(posture.trace_id.as_str()));
        assert_eq!(update_event.payload["update_id"], json!(posture.update.update_id));
        assert_eq!(update_event.payload["update_seq"], json!(posture.update.update_seq));
        assert_eq!(update_event.payload["update_sha256"], json!(posture.update.update_sha256));
        assert_eq!(update_event.payload["state_vector_before"], json!(posture.update.state_vector_before));
        assert_eq!(update_event.payload["state_vector_after"], json!(posture.update.state_vector_after));
        for event in [recorded_events[0], applied_events[0]] {
            assert_eq!(event.aggregate_type, "knowledge_crdt_ai_edit_proposal");
            assert_eq!(event.aggregate_id, posture.proposal.proposal_id);
            assert_eq!(event.correlation_id.as_deref(), Some(posture.trace_id.as_str()));
            assert_eq!(event.payload["proposal_id"], json!(posture.proposal.proposal_id));
        }
        assert_eq!(recorded_events[0].event_type, handshake_core::kernel::KernelEventType::AiEditProposalRecorded);
        assert_eq!(recorded_events[0].payload["diff_sha256"], json!(posture.proposal.diff_sha256));
        assert_eq!(recorded_events[0].payload["proposed_diff"], posture.proposal.proposed_diff);
        assert_eq!(applied_events[0].event_type, handshake_core::kernel::KernelEventType::AiEditProposalDecided);
        assert_eq!(applied_events[0].payload["applied_update_id"], json!(posture.update.update_id));
        assert_eq!(applied_events[0].payload["yjs_update_sha256"], json!(posture.update.update_sha256));
        assert_eq!(applied_events[0].payload["approved_diff_sha256"], json!(posture.proposal.diff_sha256));
        assert_eq!(applied_events[0].payload["update_event_ledger_event_id"], json!(update_event.event_id));
        assert_eq!(applied_events[0].payload["update_ledger_payload_hash"], json!(update_event.payload_hash));
        let site = handshake_core::kernel::crdt::actor_site::derive_knowledge_site_id(
            workspace.exact.workspace_id.as_str(), &posture.crdt_document_id, &actor);
        let stale = handshake_core::swarm_orchestration::model_lane::NewModelLaneCrdtUpdate {
            schema_id: "hsk.kernel.crdt_update@1".into(), document_id: document_id.clone(),
            crdt_document_id: posture.crdt_document_id.clone(), update_id: format!("stale-attempt-{label}"),
            update_seq: posture.update.update_seq, update_bytes: posture.yjs_update_bytes.clone(),
            actor_id: posture.actor_id.clone(), actor_kind: posture.actor_kind.clone(),
            session_id: posture.session_id.clone(), trace_id: posture.trace_id.clone(),
            state_vector_before: posture.update.state_vector_before.clone(),
            state_vector_after: posture.update.state_vector_after.clone(),
            replay_order_key: posture.update.replay_order_key.clone(),
            dependency_update_ids: posture.update.dependency_update_ids.clone(), site_id: site.site_id,
            kernel_task_run_id: posture.run_id.clone(), idempotency_key: format!("stale-attempt-{label}"),
        };
        assert_eq!(document_update.run(store.append_crdt_update(stale.clone())).await
            .expect("authenticated stale append returns the actual typed head outcome"),
            handshake_core::swarm_orchestration::model_lane::ModelLaneCrdtUpdateAppendOutcome::StaleHead {
                head_update_seq: posture.update.update_seq,
                head_state_vector: posture.update.state_vector_after.clone(),
            });
        assert_eq!(fixture.isolated.storage().list_account_kernel_events_for_aggregate(
            &workspace.session_token, &channel_hash, "knowledge_crdt_document", &posture.crdt_document_id,
        ).await.expect("authenticated CRDT events after stale append"), document_events_before,
            "stale append must not change the actual canonical document event set");
        assert_eq!(fixture.isolated.storage().list_account_kernel_events_for_aggregate(
            &workspace.session_token, &channel_hash, "knowledge_crdt_ai_edit_proposal", &posture.proposal.proposal_id,
        ).await.expect("authenticated proposal events after stale append"), proposal_events_before,
            "stale append must not change the actual canonical proposal event set");
        fixture.logout(&workspace.session_token).await;
        let revoked = document_update.run(store.append_crdt_update(stale)).await;
        assert!(matches!(revoked, Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(_))),
            "retained original document operation must reauthorize and reject the revoked credential");
    }).catch_unwind().await;
    let cleanup = fixture.cleanup().await;
    if !cleanup.failures.is_empty() || !cleanup.unresolved_sessions.is_empty() {
        let stages: Vec<_> = cleanup.failures.iter().map(|failure| failure.stage).collect();
        eprintln!("CRDT producer cleanup INCOMPLETE: stages={stages:?}; unresolved_sessions={}",
            !cleanup.unresolved_sessions.is_empty());
        std::panic::panic_any(FixtureFailure { primary: result.err(), cleanup });
    }
    if let Err(primary) = result { std::panic::resume_unwind(primary); }
}
