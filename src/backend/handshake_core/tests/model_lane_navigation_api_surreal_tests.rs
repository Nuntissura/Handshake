//! WP-1 real Axum navigation/recovery boundary over one injected embedded SurrealDB scope.

mod surreal_test_store_support;

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
use handshake_core::storage::surreal::{bootstrap_schema, SurrealDatabase, SurrealStorage};
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
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use surreal_test_store_support::EmbeddedSurrealTestScope;

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
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate navigation API scope");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate injected SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    let exact = exact_scope("owner");
    let scope = resource_scope(&exact);
    let store = ModelLaneStore::new_scoped(storage.clone(), scope);
    let posture = build_surreal_admissible_crdt_posture(
        &store,
        exact.workspace_id.as_str(),
        "navigation-api",
    )
    .await
    .expect("seed canonical run/lane/CRDT posture");
    let message = store
        .record_message(posture.message.clone())
        .await
        .expect("record canonical message");
    store
        .record_recovery_checkpoint(checkpoint(&posture, &message))
        .await
        .expect("record canonical checkpoint");
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        store
            .record_diagnostic_tier_status(diagnostic(&posture, tier))
            .await
            .expect("record complete canonical HBR diagnostics posture");
    }
    let before = store
        .test_scoped_authority_receipts(&posture.run_id, 128)
        .await
        .expect("snapshot canonical authority");

    let state = app_state(storage.clone());
    let owner =
        ProductLocalResourceScope::from_exact(exact.clone()).expect("exact owner authority");
    let (base, server) =
        start_server(api::model_lane_navigation::routes(state.clone()).layer(Extension(owner)))
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
        let response = reqwest::get(format!("{base}{path}"))
            .await
            .expect("owner route request");
        assert_eq!(response.status(), reqwest::StatusCode::OK, "{path}");
        let body: Value = response.json().await.expect("owner JSON projection");
        assert_eq!(body["run"]["run_id"], posture.run_id);
    }
    server.abort();

    for foreign in one_field_mismatches(&exact) {
        let authority =
            ProductLocalResourceScope::from_exact(foreign).expect("complete foreign scope");
        let (base, server) = start_server(
            api::model_lane_navigation::routes(state.clone()).layer(Extension(authority)),
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
            let response = reqwest::get(format!("{base}{path}"))
                .await
                .expect("foreign route request");
            assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
            assert_non_leaking(&response.text().await.expect("denial body"), &posture);
        }
        server.abort();
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
    server.abort();

    assert_eq!(
        store
            .test_scoped_authority_receipts(&posture.run_id, 128)
            .await
            .expect("denials are non-mutating"),
        before
    );
    drop(store);
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
        surreal_storage: storage,
        flight_recorder: services.clone(),
        diagnostics: services,
        llm_client: Arc::new(DisabledLlmClient::new(
            "disabled-navigation-test".into(),
            "navigation test does not invoke a model".into(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
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
