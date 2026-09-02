// WP-1 MT-006 embedded-SurrealDB cloud authority and revocation proof.

mod cloud_model_lane_surreal_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use axum::Extension;
use cloud_model_lane_surreal_support::{
    cloud_lane, exact_scope, expired_window, projection, receipt, valid_window, Harness,
};
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::api::operator_chat::{scoped_routes, OperatorChatState};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope, ModelLaneError,
    ModelLaneStatus, ModelLaneStore,
};
use handshake_core::swarm_orchestration::operator_chat::{
    OperatorChatLaneKind, OperatorChatLaunchService, OperatorChatSelection,
    OperatorChatSingleRunCloudConsentGrant, OperatorChatSingleRunCloudLaunchRequest,
};
use handshake_core::swarm_orchestration::resource_scope::{
    ExactResourceScopeAttribution, OwnerAccountId,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError,
};

#[derive(Default)]
struct NoEgressRecorder;

#[async_trait]
impl FlightRecorder for NoEgressRecorder {
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

struct NoEgressFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for NoEgressFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "MT-006 deterministic no-egress provider recorder".into(),
        ))
    }
}

async fn start_scoped_server(
    state: OperatorChatState,
    scope: ExactResourceScopeAttribution,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind isolated MT-006 loopback server");
    let address = listener.local_addr().expect("read loopback address");
    let product_scope =
        ProductLocalResourceScope::from_exact(scope).expect("install exact server scope");
    let app = scoped_routes(state).layer(Extension(product_scope));
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve scoped MT-006 route");
    });
    (format!("http://{address}"), server)
}

fn cloud_selection(provider: &str, model_id: &str, suffix: &str) -> OperatorChatSelection {
    OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Cloud,
        model_id: model_id.into(),
        cloud_provider: Some(provider.into()),
        cli_provider: None,
        working_dir: std::env::temp_dir().to_string_lossy().into_owned(),
        worktree_id: Some(format!("worktree-mt006-{suffix}")),
        prompt: "deterministic no-egress MT-006 proof".into(),
        owner_session: "KERNEL_BUILDER-MT006".into(),
        parent_session_id: "coordinator-session-mt006".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-006".into()),
    }
}

#[tokio::test]
async fn cloud_projection_and_consent_receipts_persist_and_replay() {
    let harness = Harness::create("persist-replay").await;
    assert_eq!(
        harness
            .store
            .test_cloud_schema_state()
            .await
            .expect("read versioned MT-006 SurrealDB schema state"),
        ("mt006-cloud-authority-v1".into(), 1, "complete".into())
    );
    let (valid_from, valid_until) = valid_window();
    let plan = harness
        .store
        .record_cloud_projection_plan(projection(
            "run-surreal-ok",
            "lane-surreal-ok",
            &harness.scope,
        ))
        .await
        .expect("persist projection in embedded SurrealDB");
    let receipt = harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-surreal-ok",
            "lane-surreal-ok",
            &plan.projection_plan_hash,
            &harness.scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect("persist consent in embedded SurrealDB");
    assert!(!plan.event_ledger_event_id.is_empty());
    assert!(!receipt.event_ledger_event_id.is_empty());
    assert!(receipt.event_ledger_seq > plan.event_ledger_seq);

    let replay = harness
        .store
        .replay_cloud_consent_authority("run-surreal-ok")
        .await
        .expect("replay exact-scope SurrealDB authority");
    assert_eq!(replay.projection_plans, vec![plan]);
    assert_eq!(replay.consent_receipts, vec![receipt]);
    harness.close().await;
}

#[tokio::test]
async fn cloud_consent_revocation_and_context_switch_cancel_covered_lanes_with_eventledger_evidence(
) {
    let harness = Harness::create("revoke").await;
    let (valid_from, valid_until) = valid_window();
    let plan = harness
        .store
        .record_cloud_projection_plan(projection("run-revoke", "lane-revoke", &harness.scope))
        .await
        .expect("persist projection");
    harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-revoke",
            "lane-revoke",
            &plan.projection_plan_hash,
            &harness.scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect("persist consent");
    harness
        .store
        .record_lane(cloud_lane("run-revoke", "lane-revoke"))
        .await
        .expect("valid cloud lane uses embedded SurrealDB without a relational fallback");

    let cancelled = harness
        .store
        .test_commit_cloud_consent_revocation(
            "cloud-consent-receipt://run-revoke/lane-revoke",
            "operator-action://mt006/revoke",
            "operator withdrew cloud export",
        )
        .await
        .expect("revoke and terminate cloud lane in embedded SurrealDB");
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(cancelled[0].failstate_code.as_deref(), Some("CX-MM-007"));

    let replay = harness
        .store
        .replay_cloud_consent_authority("run-revoke")
        .await
        .expect("replay revoked receipt");
    assert_eq!(
        replay.consent_receipts[0].status,
        ModelLaneCloudConsentReceiptStatus::Revoked
    );
    assert!(!replay.consent_receipts[0].approved);
    let error = harness
        .store
        .record_lane(cloud_lane("run-revoke", "lane-revoke"))
        .await
        .expect_err("revoked authority cannot relaunch");
    assert!(error.to_string().contains("CX-MM-007"));

    let switched_scope = exact_scope("revoke-context-switch");
    let switched_store = harness.store_for_scope(switched_scope).await;
    let switched_replay = switched_store
        .replay_cloud_consent_authority("run-revoke")
        .await
        .expect("a context switch observes no foreign authority rows");
    assert!(switched_replay.projection_plans.is_empty());
    assert!(switched_replay.consent_receipts.is_empty());
    let switched_revoke = switched_store
        .test_commit_cloud_consent_revocation(
            "cloud-consent-receipt://run-revoke/lane-revoke",
            "operator-action://mt006/cross-scope-revoke",
            "cross-scope revocation attempt",
        )
        .await
        .expect_err("a switched resource scope cannot revoke another account's receipt");
    assert!(switched_revoke.to_string().contains("CX-MM-007"));
    harness.close().await;
}

#[tokio::test]
async fn cloud_lane_rejects_missing_expired_mismatched_revoked_and_unscoped_consent_before_provider_call(
) {
    let harness = Harness::create("fail-closed").await;
    let missing = harness
        .store
        .record_lane(cloud_lane("run-missing", "lane-missing"))
        .await
        .expect_err("synthetic refs cannot authorize a cloud lane");
    assert!(missing.to_string().contains("CX-MM-007"));

    let (expired_from, expired_until) = expired_window();
    let plan = harness
        .store
        .record_cloud_projection_plan(projection("run-expired", "lane-expired", &harness.scope))
        .await
        .expect("persist expired fixture projection");
    harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-expired",
            "lane-expired",
            &plan.projection_plan_hash,
            &harness.scope,
            &expired_from,
            &expired_until,
        ))
        .await
        .expect("expired receipt remains durable evidence");
    let expired = harness
        .store
        .record_lane(cloud_lane("run-expired", "lane-expired"))
        .await
        .expect_err("expired receipt fails closed");
    assert!(expired.to_string().contains("CX-MM-007"));

    let (valid_from, valid_until) = valid_window();
    let mismatch_plan = harness
        .store
        .record_cloud_projection_plan(projection("run-mismatch", "lane-mismatch", &harness.scope))
        .await
        .expect("persist mismatch fixture projection");
    harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-mismatch",
            "lane-mismatch",
            &mismatch_plan.projection_plan_hash,
            &harness.scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect("persist mismatch fixture consent");
    let mut mismatched = cloud_lane("run-mismatch", "lane-mismatch");
    mismatched.model_id = Some("model://dexterity/byok_cloud/other".into());
    let mismatch = harness
        .store
        .record_lane(mismatched)
        .await
        .expect_err("model mismatch fails closed");
    assert!(
        mismatch.to_string().contains("CX-MM-007"),
        "mismatched cloud authority must use CX-MM-007: {mismatch}"
    );

    let revoke_plan = harness
        .store
        .record_cloud_projection_plan(projection("run-revoked", "lane-revoked", &harness.scope))
        .await
        .expect("persist revocation fixture projection");
    harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-revoked",
            "lane-revoked",
            &revoke_plan.projection_plan_hash,
            &harness.scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect("persist revocation fixture consent");
    harness
        .store
        .test_commit_cloud_consent_revocation(
            "cloud-consent-receipt://run-revoked/lane-revoked",
            "operator-action://mt006/fail-closed-revoke",
            "revoked before launch",
        )
        .await
        .expect("revoke before cloud launch");
    let revoked = harness
        .store
        .record_lane(cloud_lane("run-revoked", "lane-revoked"))
        .await
        .expect_err("revoked receipt fails closed");
    assert!(revoked.to_string().contains("CX-MM-007"));

    let unscoped_scope = exact_scope("unscoped-before-provider");
    // A reader-only access context carries no exact write scope, so the cloud
    // authority must deny the projection plan before any provider is reached.
    let unscoped_store = ModelLaneStore::new(
        harness.storage.clone(),
        handshake_core::swarm_orchestration::resource_scope::ResourceAccessContext::for_reader(
            handshake_core::swarm_orchestration::resource_scope::ResourceScopeQuery::for_owner(
                unscoped_scope.owner_account_id,
            ),
        ),
    );
    let unscoped = unscoped_store
        .record_cloud_projection_plan(projection("run-unscoped", "lane-unscoped", &unscoped_scope))
        .await
        .expect_err("unscoped cloud authority fails before opening storage or calling a provider");
    assert!(matches!(&unscoped, ModelLaneError::AuthorityDenied(_)));
    assert!(unscoped.to_string().contains("CX-MM-007"));
    harness.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_http_launch_enforces_cross_account_and_delegated_audience_scope() {
    let harness = Harness::create("http-scope").await;
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope)
        .expect("complete five-dimensional test scope");
    let calls = Arc::new(AtomicUsize::new(0));
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual no-egress process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(NoEgressFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        harness.store.clone(),
    );
    let service = Arc::new(OperatorChatLaunchService::new(
        Arc::new(coordinator),
        ModelCatalog::empty(),
        Arc::new(NoEgressRecorder),
    ));
    let state = OperatorChatState::production().with_launch_service(service);
    let (base, server) = start_scoped_server(state, exact).await;

    let (valid_from, valid_until) = valid_window();
    let mut plan = projection("run-http-scope", "lane-template", &harness.scope);
    plan.projection_plan_id = "cloud-projection-plan://run-http-scope/broadcast".into();
    plan.lane_id = None;
    plan.model_session_id = None;
    plan.provider_kind = None;
    plan.requested_model_id = None;
    plan.consent_scope = ModelLaneCloudConsentScope::SingleRun;
    plan.target_bindings.clear();
    plan.fan_out_targets = vec!["provider-endpoint://caller-injected".into()];
    plan.export_delegation.audience_refs = vec!["provider-endpoint://attacker".into()];
    plan.idempotency_key = "idem-projection-run-http-scope-broadcast".into();
    let request = OperatorChatSingleRunCloudLaunchRequest {
        grant: OperatorChatSingleRunCloudConsentGrant {
            projection_plan: plan,
            consent_receipt_id: "cloud-consent-receipt://run-http-scope/broadcast".into(),
            approved_by_ref: "operator-action://mt006/http-proof".into(),
            approved_at_utc: valid_from.clone(),
            valid_from_utc: valid_from,
            valid_until_utc: valid_until,
            consent_idempotency_key: "idem-consent-run-http-scope-broadcast".into(),
            diagnostic_payload: serde_json::json!({"provider_call_attempted": false}),
        },
        selections: vec![
            cloud_selection("openai", "gpt-4o-mini", "openai"),
            cloud_selection("anthropic", "claude-3-5-haiku", "anthropic"),
        ],
    };
    let client = reqwest::Client::new();
    let cross_account = client
        .post(format!(
            "{base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .header(
            "x-handshake-owner-account",
            OwnerAccountId::mint().to_string(),
        )
        .json(&request)
        .send()
        .await
        .expect("send cross-account assertion");
    assert_eq!(cross_account.status().as_u16(), 403);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let empty = harness
        .store
        .replay_cloud_consent_authority("run-http-scope")
        .await
        .expect("cross-account HTTP denial leaves no authority rows");
    assert!(empty.projection_plans.is_empty());
    assert!(empty.consent_receipts.is_empty());

    let matching_scope = client
        .post(format!(
            "{base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .json(&request)
        .send()
        .await
        .expect("send server-scoped launch to deterministic no-egress factory");
    assert_eq!(matching_scope.status().as_u16(), 500);
    assert!(calls.load(Ordering::SeqCst) > 0);

    let replay = harness
        .store
        .replay_cloud_consent_authority("run-http-scope")
        .await
        .expect("replay server-derived HTTP grant");
    assert_eq!(replay.projection_plans.len(), 1);
    assert_eq!(replay.consent_receipts.len(), 1);
    let stored_plan = &replay.projection_plans[0];
    assert_eq!(
        stored_plan.export_delegation.audience_refs,
        stored_plan.fan_out_targets
    );
    assert_eq!(stored_plan.fan_out_targets.len(), 2);
    assert!(!stored_plan
        .fan_out_targets
        .iter()
        .any(|target| target.contains("attacker") || target.contains("caller-injected")));
    assert_eq!(
        stored_plan.export_delegation.source_scope,
        handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority::from_scope(
            &harness.scope
        )
    );

    server.abort();
    harness.close().await;
}
