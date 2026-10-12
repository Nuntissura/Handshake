//! WP-1 real Axum navigation/recovery boundary over one injected embedded SurrealDB scope.

mod surreal_test_store_support;
mod account_session_support;

use std::sync::Arc;

use axum::{Extension, Router};
use handshake_core::api;
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::DisabledLlmClient;
use handshake_core::storage::surreal::{SurrealDatabase, SurrealStorage};
use handshake_core::storage::StorageError;
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState, ModelLaneRecoveryState,
    ModelLaneRecoveryStatus, ModelLaneStatus, ModelLaneStore, NewModelLaneDiagnosticTierStatus,
    NewModelLaneRecoveryCheckpoint,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    SurrealAdmissibleCrdtPosture,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use surreal_test_store_support::IsolatedSurrealTestStore;

struct NoopServices;

#[async_trait::async_trait]
impl FlightRecorder for NoopServices {
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

#[async_trait::async_trait]
impl DiagnosticsStore for NoopServices {
    async fn record_diagnostic(&self, _diag: Diagnostic) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_problems(&self, _filter: DiagFilter) -> Result<Vec<ProblemGroup>, StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(&self, _id: uuid::Uuid) -> Result<Diagnostic, StorageError> {
        Err(StorageError::NotFound("diagnostic"))
    }

    async fn list_diagnostics(&self, _filter: DiagFilter) -> Result<Vec<Diagnostic>, StorageError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn real_axum_navigation_and_recovery_are_exact_scoped_absent_shaped_and_non_mutating() {
    let mut isolated = IsolatedSurrealTestStore::create()
        .await
        .expect("allocate navigation API scope");
    let storage = isolated.storage().clone();
    let fixture = NavigationFixture::create(&storage, "navigation-api").await;
    let posture = fixture.posture("navigation-api").await;
    let message = fixture.create.run(fixture.store.record_message(posture.message.clone())).await.expect("record canonical message");
    fixture.context("navigation-api", &posture, &message).await;
    let NavigationFixture { account, scope, lifecycle, store, create, read, nav, document_update,
        document_id: _, model_id: _, catalog } = fixture;
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("actual exact API scope");
    create.run(store.record_recovery_checkpoint(checkpoint(&posture, &message))).await.expect("record canonical checkpoint");
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        create.run(store.record_diagnostic_tier_status(diagnostic(&posture, tier)))
            .await
            .expect("record complete canonical HBR diagnostics posture");
    }
    let before = read.run(store.test_scoped_authority_receipts(&posture.run_id, 128))
        .await
        .expect("snapshot canonical authority");

    let state = app_state(storage.clone());
    let owner =
        ProductLocalResourceScope::from_exact(exact.clone()).expect("exact owner authority");
    let (base, server) =
        start_server(api::model_lane_navigation::routes(state.clone()).layer(Extension(owner)).layer(Extension(lifecycle.clone())))
            .await;
    for path in [
        format!("/swarm/model-lanes/navigation/runs/{}", posture.run_id),
        format!("/swarm/model-lanes/navigation/lanes/{}", posture.lane_id),
        format!("/swarm/model-lanes/navigation/messages/{}", message.message_id),
        format!("/swarm/model-lanes/navigation/recovery/{}", posture.run_id),
        format!(
            "/swarm/model-lanes/navigation/diagnostics/{}?behavior_id=HBR-INT-009&tier=internal_diagnostics&mt_id=MT-007",
            posture.run_id
        ),
        format!("/swarm/model-lanes/navigation/lookup?run_id={}", posture.run_id),
    ] {
        let response = account.client().get(format!("{base}{path}")).send()
            .await
            .expect("owner route request");
        assert_eq!(response.status(), reqwest::StatusCode::OK, "{path}");
        let body: Value = response.json().await.expect("owner JSON projection");
        assert_eq!(body["run"]["run_id"], posture.run_id);
    }
    server.abort(); let _ = server.await;

    for foreign in one_field_mismatches(&exact) {
        let authority =
            ProductLocalResourceScope::from_exact(foreign).expect("complete foreign scope");
        let (base, server) = start_server(
            api::model_lane_navigation::routes(state.clone()).layer(Extension(authority)).layer(Extension(lifecycle.clone())),
        )
        .await;
        for path in [
            format!("/swarm/model-lanes/navigation/runs/{}", posture.run_id),
            format!("/swarm/model-lanes/navigation/recovery/{}", posture.run_id),
            format!(
                "/swarm/model-lanes/navigation/diagnostics/{}",
                posture.run_id
            ),
        ] {
            let response = account.client().get(format!("{base}{path}")).send()
                .await
                .expect("foreign route request");
            assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
            assert_non_leaking(&response.text().await.expect("denial body"), &posture);
        }
        server.abort(); let _ = server.await;
    }

    let (base, server) = start_server(api::model_lane_navigation::routes(state)).await;
    let response = reqwest::get(format!(
        "{base}/swarm/model-lanes/navigation/recovery/{}",
        posture.run_id
    ))
    .await
    .expect("missing-authority request");
    assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_non_leaking(
        &response.text().await.expect("missing-authority body"),
        &posture,
    );
    server.abort(); let _ = server.await;

    assert_eq!(
        read.run(store.test_scoped_authority_receipts(&posture.run_id, 128))
            .await
            .expect("denials are non-mutating"),
        before
    );
    drop(store); drop(create); drop(read); drop(nav); drop(document_update); drop(catalog);
    drop(storage);
    isolated
        .cleanup()
        .await
        .expect("cleanup navigation API scope");
}

fn app_state(storage: SurrealStorage) -> AppState {
    let services = Arc::new(NoopServices);
    AppState {
        storage: Arc::new(SurrealDatabase::new(storage.clone())),
        surreal: storage,
        flight_recorder: services.clone(),
        diagnostics: services,
        llm_client: Arc::new(DisabledLlmClient::new(
            "disabled-navigation-test".into(),
            "navigation test does not invoke a model".into(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new_from_env()),
    }
}

async fn start_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind quiet loopback listener");
    let address = listener.local_addr().expect("loopback address");
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve navigation API");
    });
    (format!("http://{address}"), server)
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: "checkpoint-navigation-api".into(),
        run_id: posture.run_id.clone(),
        lane_id: Some(posture.lane_id.clone()),
        session_id: posture.session_id.clone(),
        model_session_id: posture.model_session_id.clone(),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq: message.event_ledger_seq,
        last_message_id: Some(message.message_id.clone()),
        open_payload_refs: vec![message.payload_ref.clone()],
        lease_id: None,
        idempotency_scope: format!("recovery:{}:navigation-api", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some("recovery-event://navigation-api".into()),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: "idem-checkpoint-navigation-api".into(),
        created_at_utc: "2026-09-02T02:00:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn diagnostic(
    posture: &SurrealAdmissibleCrdtPosture,
    tier: ModelLaneDiagnosticTier,
) -> NewModelLaneDiagnosticTierStatus {
    let tier_name = tier.as_str();
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diagnostic-navigation-api-{tier_name}"),
        behavior_id: "HBR-INT-009".into(),
        run_id: posture.run_id.clone(),
        tier,
        state: ModelLaneDiagnosticTierState::Wired,
        reason: "private navigation diagnosis".into(),
        evidence_ref: format!("eventledger://navigation-api/diagnostic/{tier_name}"),
        follow_up_ref: Some("usermanual://model-lane/diagnostics".into()),
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("idem-diagnostic-navigation-api-{tier_name}"),
        diagnostic_payload: json!({"private": "navigation-secret"}),
    }
}

fn exact_scope(label: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-navigation-{label}"))
            .expect("workspace"),
    }
}

fn resource_scope(exact: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(exact.owner_account_id, exact.actor_principal_id)
        .with_session(exact.authenticated_session_id)
        .with_access_space(exact.access_space_id)
        .with_workspace(exact.workspace_id.clone())
}

fn one_field_mismatches(
    exact: &ExactResourceScopeAttribution,
) -> Vec<ExactResourceScopeAttribution> {
    let mut owner = exact.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = exact.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = exact.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access = exact.clone();
    access.access_space_id = AccessSpaceRef::mint();
    let mut workspace = exact.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new("workspace-navigation-foreign").expect("foreign workspace");
    vec![owner, actor, session, access, workspace]
}

fn assert_non_leaking(body: &str, posture: &SurrealAdmissibleCrdtPosture) {
    for secret in [
        posture.run_id.as_str(),
        posture.lane_id.as_str(),
        posture.message.message_id.as_str(),
        "private navigation diagnosis",
        "navigation-secret",
    ] {
        assert!(!body.contains(secret), "denial leaked {secret}: {body}");
    }
}

use handshake_core::storage::surreal::{bootstrap_production_schemas};
use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceKind, ResourceAction, ResourceGrantSpec};
use handshake_core::swarm_orchestration::resource_scope::{ResourceAccessLifecycleRegistry};
use handshake_core::test_harness::crdt_workspace::{AuthenticatedModelLaneTestOperation,
    build_surreal_admissible_crdt_posture_authenticated_with_context};
use handshake_core::swarm_orchestration::model_lane as lane;

struct NavigationFixture {
    account: account_session_support::AccountFixture, scope: ResourceScope,
    lifecycle: ResourceAccessLifecycleRegistry, store: ModelLaneStore,
    create: AuthenticatedModelLaneTestOperation, read: AuthenticatedModelLaneTestOperation,
    nav: AuthenticatedModelLaneTestOperation, document_update: AuthenticatedModelLaneTestOperation,
    document_id: String, model_id: String, catalog: std::sync::Arc<handshake_core::model_runtime::ModelCatalog>,
}
impl NavigationFixture {
    async fn create(storage: &SurrealStorage, label: &str) -> Self {
        bootstrap_production_schemas(storage).await.expect("bootstrap canonical navigation producers");
        let mut account = account_session_support::AccountFixture::install(storage).await;
        account.owner = account_session_support::OwnerSession::provision_with_capabilities(storage,
            account.binding_token(), &["model_lane.navigation.read"]).await;
        let recorder = std::sync::Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).expect("bounded navigation fixture recorder"));
        let state = handshake_core::AppState {
            storage: std::sync::Arc::new(handshake_core::storage::surreal::SurrealDatabase::new(storage.clone())),
            surreal: storage.clone(), flight_recorder: recorder.clone(), diagnostics: recorder,
            llm_client: std::sync::Arc::new(handshake_core::llm::DisabledLlmClient::new("navigation-component".into(), "no model invocation asserted".into())),
            capability_registry: std::sync::Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
            session_registry: std::sync::Arc::new(handshake_core::workflows::SessionRegistry::new_from_env()),
        };
        let workspace_id = account.create_workspace(&state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("owned document fixture listener");
        let address = listener.local_addr().expect("document fixture address");
        let app = handshake_core::api::knowledge_documents::routes(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("owned document fixture server"); });
        let response = account.apply(reqwest::Client::new().post(format!("http://{address}/knowledge/documents")))
            .header("x-hsk-actor-id", &account.actor_id).header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", format!("navigation-fixture-{label}"))
            .header("x-hsk-session-run-id", &account.session_id)
            .json(&json!({"workspace_id": workspace_id, "title": format!("Navigation {label}"),
                "content_json": {"type": "doc", "content": []}})).send().await.expect("real authenticated document producer");
        let status = response.status(); let body: serde_json::Value = response.json().await.expect("canonical document response");
        server.abort(); let _ = server.await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "document producer: {body}");
        let document_id = body["document"]["rich_document_id"].as_str().expect("actual document ID").to_owned();
        let scope = ResourceScope::new(OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&account.account_id).expect("actual account")),
            ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&account.principal_id).expect("actual principal")))
            .with_session(AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&account.session_id).expect("actual session")))
            .with_access_space(AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&account.access_space_id).expect("actual Space")))
            .with_workspace(WorkspaceScopeRef::new(workspace_id.clone()).expect("actual workspace"));
        let exact = handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(&scope).expect("actual exact tuple");
        let lifecycle = storage.resource_access_lifecycle(); lifecycle.register_active(exact.clone()).expect("register actual fixture lifecycle");
        let request = navigation_request(&account, ResourceKind::Workspace, &workspace_id, ResourceAction::Read, "fs.read");
        let actual = storage.authorize_protected_resource(request).await.expect("resolve canonical owning workspace grant");
        storage.grant_resource(&account.account_id, &account.access_space_id, ResourceGrantSpec {
            principal_id: account.principal_id.clone(), resource_id: actual.resource_id,
            actions: vec![ResourceAction::Read], capability_ids: vec!["model_lane.navigation.read".into()],
            expires_at: None, delegation_chain: actual.delegation_chain,
        }).await.expect("canonical exact navigation grant producer");
        // Component catalog input only: no artifact loading, readiness, or model invocation proof.
        let model = handshake_core::model_runtime::ModelId::new_v7();
        let model_id = model.to_string();
        let mut registry = handshake_core::model_runtime::ModelRegistry::default();
        registry.register(handshake_core::model_runtime::ModelRegistration {
            model_id: model.clone(), artifact_path: "models/navigation-component-fixture.gguf".into(), sha256: [0x5a; 32],
            runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
            declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
            base_model_tag: handshake_core::model_runtime::BaseModelTag::new("navigation-component-fixture"),
            registered_at_utc: chrono::Utc::now(),
            registered_by: handshake_core::model_runtime::OperatorId::new("navigation-component-fixture"),
            provider: handshake_core::model_runtime::ProviderKind::Local,
        }).expect("register component catalog identity");
        registry.mark_loaded(model).expect("mark component catalog identity loaded");
        let catalog = handshake_core::model_runtime::ModelCatalog::from_registry(Arc::new(registry));

        let store = ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), scope.clone(), lifecycle.clone()).with_model_catalog(catalog.clone());
        let create = navigation_operation(storage, &account, &scope, ResourceKind::Workspace, &workspace_id, ResourceAction::Create, "fs.write").await;
        let read = navigation_operation(storage, &account, &scope, ResourceKind::Workspace, &workspace_id, ResourceAction::Read, "fs.read").await;
        let nav = navigation_operation(storage, &account, &scope, ResourceKind::Workspace, &workspace_id, ResourceAction::Read, "model_lane.navigation.read").await;
        let document_update = navigation_operation(storage, &account, &scope, ResourceKind::RichDocument, &document_id, ResourceAction::Update, "fs.write").await;
        bootstrap_production_schemas(storage).await.expect("administratively reconcile exact actual fixture metadata");
        Self { account, scope, lifecycle, store, create, read, nav, document_update, document_id, model_id, catalog }
    }
    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        // Draft producer input: only its known canonical run/locus/session identity
        // participates in the production context-ID derivation before record_run.
        let draft = navigation_handoff_draft(label);
        let context_id = lane::model_lane_context_bundle_id_for_handoff(&draft).expect("production context identity before run creation");
        let mut posture = build_surreal_admissible_crdt_posture_authenticated_with_context(&self.store,
            self.scope.workspace.as_ref().expect("workspace").as_str(), label, &self.model_id, &self.document_id,
            &self.create, &self.document_update, &context_id).await.expect("actual authenticated CRDT producer chain");
        use sha2::{Digest, Sha256};
        let payload = json!({"schema_id": "hsk.model_lane_context_payload@1", "message_id": posture.message.message_id, "summary": posture.message.summary});
        posture.message.payload_sha256 = hex::encode(Sha256::digest(handshake_storage_support::canonical_json::canonical_json_bytes(&payload)));
        posture
    }
    async fn context(&self, label: &str, posture: &SurrealAdmissibleCrdtPosture, source: &lane::ModelLaneMessageRecord) {
        let artifact = lane::NewModelLaneContextBundleArtifactBinding {
            artifact_binding_id: format!("navigation-artifact-{}", source.message_id), run_id: source.run_id.clone(), trace_id: source.trace_id.clone(),
            artifact_ref: source.payload_ref.clone(), artifact_sha256: source.payload_sha256.clone(), content_hash: source.payload_sha256.clone(),
            artifact_kind: "model_lane_message_payload".into(), artifact_manifest_ref: format!("artifact-manifest://{}", source.message_id),
            artifact_payload_ref: source.payload_ref.clone(), payload_json: json!({"schema_id":"hsk.model_lane_context_payload@1", "message_id":source.message_id,"summary":source.summary}),
            event_ledger_stream_id: source.event_ledger_stream_id.clone(), work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
            micro_task_id: "MT-018".into(), task_board_id: "task-board://wp-1".into(), owner_session: source.owner_session.clone(),
            idempotency_key: format!("navigation-artifact-{}", source.message_id), created_at_utc: "2026-09-02T00:03:00Z".into(), diagnostic_payload: json!({}),
        };
        self.create.run(self.store.record_context_bundle_artifact_binding(artifact)).await.expect("canonical artifact producer");
        let binding = source.crdt_authority_binding.as_ref().expect("canonical CRDT message binding");
        let mut input = navigation_handoff_draft(label);
        input.context_bundle_id = lane::model_lane_context_bundle_id_for_handoff(&input).expect("independent actual context identity");
        input.source_message_id = source.message_id.clone(); input.source_lane_id = source.from_lane_id.clone();
        input.artifact_ref = source.payload_ref.clone(); input.artifact_sha256 = source.payload_sha256.clone(); input.content_hash = source.payload_sha256.clone();
        input.parent_span_id = Some(source.message_span_id.clone()); input.linked_span_contexts = vec![source.trace_id.clone()];
        input.crdt_payload = Some(lane::ModelLaneCrdtHandoffMetadata {
            schema_id: "hsk.model_lane_crdt_payload@1".into(), document_id: binding.document_id.clone(), workspace_id: binding.workspace_id.clone(),
            actor_id: binding.actor_id.clone(), actor_kind: binding.actor_kind.clone(), lane_id: binding.lane_id.clone(), crdt_site_id: binding.crdt_site_id.clone(),
            update_seq: binding.update_seq, update_bytes_ref: binding.update_bytes_ref.clone(), update_sha256: posture.update.update_sha256.clone(),
            state_vector: binding.state_vector.clone(), base_snapshot_ref: binding.base_snapshot_ref.clone(), materialized_projection_hash: binding.materialized_projection_hash.clone(),
            replay_metadata: json!({"format":"yjs_update_v1", "yjs_compatible":true,"replay_order_key":posture.update.replay_order_key,
                "dependency_update_ids":posture.update.dependency_update_ids,"schema_version":posture.update.replay_schema_version}),
            promotion_gate_ref: format!("promotion-gate://model-lane-message/{}",source.message_id), promotion_receipt_ref: None,
            validation_runner_ref: format!("eventledger://{}",posture.update.event_ledger_event_id), authority_effect: "advisory_only".into(),
        });
        self.create.run(self.store.record_context_bundle_handoff(input)).await.expect("actual authorized canonical handoff producer");
    }
}
fn navigation_request(account: &account_session_support::AccountFixture, kind: ResourceKind, id: &str, action: ResourceAction, capability: &str) -> AuthorizationRequest {
    use sha2::{Digest, Sha256};
    AuthorizationRequest { session_token: account.session_token.clone(), channel_binding_hash: Some(hex::encode(Sha256::digest(account.channel_binding_token.as_bytes()))),
        capability_id: capability.into(), resource_kind: kind, external_resource_id: id.into(), action }
}
async fn navigation_operation(storage: &SurrealStorage, account: &account_session_support::AccountFixture, scope: &ResourceScope,
    kind: ResourceKind, id: &str, action: ResourceAction, capability: &str) -> AuthenticatedModelLaneTestOperation {
    AuthenticatedModelLaneTestOperation::authorize(storage.clone(), navigation_request(account, kind, id, action, capability),
        handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("same real tuple"))
        .await.expect("actual canonical fixture operation")
}
fn navigation_handoff_draft(label: &str) -> lane::NewModelLaneContextBundleHandoff {
    lane::NewModelLaneContextBundleHandoff {
        handoff_id: format!("navigation-handoff-{label}"), context_bundle_id: String::new(), run_id: format!("run-mt018-{label}"), trace_id: format!("trace-mt018-{label}"),
        handoff_span_id: format!("navigation-handoff-span-{label}"), parent_span_id: None, linked_span_contexts: Vec::new(),
        downstream_lane_id: format!("lane-mt018-{label}"), source_lane_id: String::new(), source_message_id: String::new(),
        artifact_ref: String::new(), artifact_sha256: String::new(), content_hash: String::new(), source_kind: lane::ModelLaneHandoffSourceKind::Proposal,
        authority_state: lane::ModelLaneAuthority::PromotionCandidate, selection_state: lane::ModelLaneHandoffSelectionState::Selected,
        reason_code: "navigation_source_selected".into(), decision_ref: None, reviewer_ref: None, replay_hint: "read canonical authorized source".into(),
        crdt_payload: None, loom_refs: Vec::new(), memory_pack_refs: Vec::new(), event_ledger_stream_id: format!("model-lane://mt018/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(), micro_task_id: "MT-018".into(), task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-mt018-{label}"), idempotency_key: format!("navigation-handoff-{label}"), replay_order_key: format!("0040-{label}"),
        created_at_utc: "2026-09-02T00:04:00Z".into(), diagnostic_payload: json!({}),
    }
}
