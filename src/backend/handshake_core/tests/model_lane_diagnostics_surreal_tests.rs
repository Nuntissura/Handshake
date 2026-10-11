//! WP-1 diagnostics projection, HBR posture, and exact-scope proof over embedded SurrealDB.

mod account_session_support;
mod surreal_test_store_support;

use handshake_core::storage::surreal::SurrealStorage;
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneAuthorityTestCorruption, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneError, ModelLaneStore, NewModelLaneDiagnosticTierStatus,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, build_surreal_admissible_crdt_posture_with_model,
    SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::IsolatedSurrealTestStore;

struct Harness {
    isolated: IsolatedSurrealTestStore,
    storage: SurrealStorage,
    exact: ExactResourceScopeAttribution,
    store: ModelLaneStore,
    posture: SurrealAdmissibleCrdtPosture,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let isolated = IsolatedSurrealTestStore::create()
            .await
            .expect("allocate diagnostics database");
        let storage = isolated.storage().clone();
        let exact = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), resource_scope(&exact));
        let posture = build_surreal_admissible_crdt_posture(
            &store,
            exact.workspace_id.as_str(),
            &format!("diagnostics-{label}"),
        )
        .await
        .expect("seed canonical diagnostics run and lane");
        store
            .record_message(posture.message.clone())
            .await
            .expect("record canonical diagnostics message");
        Self {
            isolated,
            storage,
            exact,
            store,
            posture,
        }
    }

    async fn close(self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .shutdown_and_cleanup()
            .await
            .expect("cleanup diagnostics database");
    }
}

fn anchor_catalog(
    model_id: handshake_core::model_runtime::ModelId,
    sha: [u8; 32],
    loaded: bool,
) -> std::sync::Arc<handshake_core::model_runtime::ModelCatalog> {
    let mut registry = handshake_core::model_runtime::ModelRegistry::default();
    registry
        .register(handshake_core::model_runtime::ModelRegistration {
            model_id: model_id.clone(),
            artifact_path: "models/diagnostics-fixture.gguf".into(),
            sha256: sha,
            runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
            declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
            base_model_tag: handshake_core::model_runtime::BaseModelTag::new(
                "diagnostics-catalog-fixture",
            ),
            registered_at_utc: chrono::Utc::now(),
            registered_by: handshake_core::model_runtime::OperatorId::new("diagnostics-fixture"),
            provider: handshake_core::model_runtime::ProviderKind::Local,
        })
        .unwrap();
    if loaded {
        registry.mark_loaded(model_id).unwrap();
    }
    handshake_core::model_runtime::ModelCatalog::from_registry(std::sync::Arc::new(registry))
}

async fn assert_original_lane_anchor(
    storage: &SurrealStorage,
    exact: &ExactResourceScopeAttribution,
    lane_id: &str,
    anchor: Option<&str>,
) {
    use surrealdb::types::SurrealValue;
    #[derive(SurrealValue)]
    struct ReadScope {
        lane_id: String,
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
        workspace_id: String,
    }
    #[derive(SurrealValue)]
    struct Row {
        record_json: String,
        event_id: String,
        event_seq: i64,
        event_payload_json: String,
        event_sequence: i64,
    }
    let rows = storage.query_values::<Row,_>(
        "SELECT record_json, event_id, event_seq, event_ledger_event_id.payload.event_payload_json AS event_payload_json, event_ledger_event_id.event_sequence AS event_sequence FROM model_lane_authority WHERE record_kind = 'lane' AND aggregate_id = $lane_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id;",
        ReadScope { lane_id: lane_id.into(), owner_account_id: exact.owner_account_id.to_string(), actor_principal_id: exact.actor_principal_id.to_string(), authenticated_session_id: exact.authenticated_session_id.to_string(), access_space_id: exact.access_space_id.to_string(), workspace_id: exact.workspace_id.to_string() }).await.unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert!(!row.event_id.is_empty());
    assert_eq!(row.event_seq, row.event_sequence);
    let lane: serde_json::Value = serde_json::from_str(&row.record_json).unwrap();
    let event: serde_json::Value = serde_json::from_str(&row.event_payload_json).unwrap();
    assert_eq!(lane["model_stable_anchor"].as_str(), anchor);
    assert_eq!(event["record"]["model_stable_anchor"].as_str(), anchor);
    assert_eq!(event["record"]["lane_id"], lane_id);
}

#[tokio::test]
async fn diagnostics_require_complete_hbr_posture_and_project_canonical_receipts() {
    let harness = Harness::create("hbr").await;
    harness
        .store
        .record_diagnostic_tier_status(tier_status(
            &harness.posture,
            ModelLaneDiagnosticTier::FlightRecorder,
        ))
        .await
        .expect("record FlightRecorder tier");
    let flight_only = harness
        .store
        .diagnostics_projection(&harness.posture.run_id)
        .await
        .expect_err("FlightRecorder-only posture must fail HBR-INT-009");
    assert!(
        flight_only.to_string().contains("internal_diagnostics")
            || flight_only.to_string().contains("palmistry")
    );

    for tier in [
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        harness
            .store
            .record_diagnostic_tier_status(tier_status(&harness.posture, tier))
            .await
            .expect("complete HBR tier posture");
    }
    let projection = harness
        .store
        .diagnostics_projection(&harness.posture.run_id)
        .await
        .expect("project canonical diagnostics");
    assert_eq!(projection.run.run_id, harness.posture.run_id);
    assert_eq!(projection.diagnostic_tiers.len(), 3);
    assert_eq!(
        harness
            .store
            .latest_diagnostics_projection()
            .await
            .expect("resolve latest diagnostics inside owner scope"),
        projection
    );
    assert!(projection
        .diagnostic_tiers
        .iter()
        .all(|tier| tier.evidence_ref.starts_with("eventledger://")));
    let model_id = handshake_core::model_runtime::ModelId::new_v7();
    let catalog = anchor_catalog(model_id.clone(), [0x5a; 32], true);
    let bound = ModelLaneStore::new_scoped(harness.storage.clone(), resource_scope(&harness.exact))
        .with_model_catalog(catalog.clone());
    let legacy = bound
        .replay_run(&harness.posture.run_id)
        .await
        .unwrap()
        .lanes
        .into_iter()
        .find(|lane| lane.lane_id == harness.posture.lane_id)
        .unwrap();
    bound.record_lane(legacy.inner).await.unwrap();
    assert_original_lane_anchor(
        &harness.storage,
        &harness.exact,
        &harness.posture.lane_id,
        None,
    )
    .await;
    let missing = bound
        .diagnostics_projection_with_model_catalog(&harness.posture.run_id, Some(&catalog))
        .await
        .unwrap();
    let legacy_lane = missing
        .lanes
        .iter()
        .find(|lane| lane.lane_id == harness.posture.lane_id)
        .unwrap();
    assert!(legacy_lane.model_stable_anchor.is_none());
    assert_eq!(
        legacy_lane.model_display_name,
        handshake_core::model_runtime::UNKNOWN_MODEL_LABEL
    );
    assert!(legacy_lane
        .model_anchor_unavailable_reason
        .as_ref()
        .unwrap()
        .contains("persisted artifact"));

    let unknown = build_surreal_admissible_crdt_posture(
        &bound,
        harness.exact.workspace_id.as_str(),
        "diagnostics-unknown-registered-origin",
    )
    .await
    .unwrap();
    assert_original_lane_anchor(&harness.storage, &harness.exact, &unknown.lane_id, None).await;

    let registered = build_surreal_admissible_crdt_posture_with_model(
        &bound,
        harness.exact.workspace_id.as_str(),
        "diagnostics-original-anchor",
        &model_id.to_string(),
    )
    .await
    .unwrap();
    bound
        .record_message(registered.message.clone())
        .await
        .unwrap();
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        bound
            .record_diagnostic_tier_status(tier_status(&registered, tier))
            .await
            .unwrap();
    }
    assert_original_lane_anchor(
        &harness.storage,
        &harness.exact,
        &registered.lane_id,
        Some(&"5a".repeat(32)),
    )
    .await;
    let replay = bound.replay_run(&registered.run_id).await.unwrap();
    let lane = replay
        .lanes
        .into_iter()
        .find(|lane| lane.lane_id == registered.lane_id)
        .unwrap();
    let retried = bound.record_lane(lane.inner.clone()).await.unwrap();
    assert_eq!(retried.event_ledger_event_id, lane.event_ledger_event_id);
    let different_id = handshake_core::model_runtime::ModelId::new_v7();
    assert_ne!(different_id.to_string(), model_id.to_string());
    let reminted_catalog = anchor_catalog(different_id, [0x5a; 32], true);
    for current in [&catalog, &reminted_catalog] {
        let projected = bound
            .diagnostics_projection_with_model_catalog(&registered.run_id, Some(current))
            .await
            .unwrap();
        let projected_lane = projected
            .lanes
            .iter()
            .find(|item| item.lane_id == registered.lane_id)
            .unwrap();
        assert_eq!(
            projected_lane.model_stable_anchor.as_deref(),
            Some("5a".repeat(32).as_str())
        );
        assert_eq!(
            projected_lane.model_display_name,
            "diagnostics-catalog-fixture"
        );
        assert!(projected_lane.model_anchor_unavailable_reason.is_none());
    }
    let unloaded = anchor_catalog(model_id.clone(), [0x5a; 32], false);
    let deleted = handshake_core::model_runtime::ModelCatalog::from_registry(std::sync::Arc::new(
        handshake_core::model_runtime::ModelRegistry::default(),
    ));
    for current in [Some(unloaded.as_ref()), Some(deleted.as_ref()), None] {
        let projected = bound
            .diagnostics_projection_with_model_catalog(&registered.run_id, current)
            .await
            .unwrap();
        let projected_lane = projected
            .lanes
            .iter()
            .find(|item| item.lane_id == registered.lane_id)
            .unwrap();
        assert_eq!(
            projected_lane.model_display_name,
            handshake_core::model_runtime::UNKNOWN_MODEL_LABEL
        );
        assert!(projected_lane.model_anchor_unavailable_reason.is_some());
        assert_eq!(
            projected_lane.model_stable_anchor.as_deref(),
            Some("5a".repeat(32).as_str())
        );
    }
    let drifted =
        ModelLaneStore::new_scoped(harness.storage.clone(), resource_scope(&harness.exact))
            .with_model_catalog(anchor_catalog(model_id, [0x6b; 32], true));
    assert!(matches!(
        drifted.record_lane(lane.inner.clone()).await,
        Err(ModelLaneError::IdempotencyConflict(_))
    ));
    let mut changed_model = lane.inner.clone();
    changed_model.model_id = Some(handshake_core::model_runtime::ModelId::new_v7().to_string());
    assert!(matches!(
        bound.record_lane(changed_model).await,
        Err(ModelLaneError::IdempotencyConflict(_))
    ));
    bound
        .record_lane_terminal_status(
            &registered.lane_id,
            handshake_core::swarm_orchestration::model_lane::ModelLaneStatus::Failed,
            "anchor-restart-proof",
        )
        .await
        .unwrap();
    assert_original_lane_anchor(
        &harness.storage,
        &harness.exact,
        &registered.lane_id,
        Some(&"5a".repeat(32)),
    )
    .await;
    let mut restart = lane.inner;
    restart.restart_generation += 1;
    assert!(matches!(
        drifted.record_lane(restart.clone()).await,
        Err(ModelLaneError::IdempotencyConflict(_))
    ));
    bound.record_lane(restart).await.unwrap();
    assert_original_lane_anchor(
        &harness.storage,
        &harness.exact,
        &registered.lane_id,
        Some(&"5a".repeat(32)),
    )
    .await;
    for (kind, binding, provider) in [
        ("human_operator", "human", "human"),
        ("cloud_model", "cloud", "openai"),
    ] {
        let (label, reason) =
            handshake_core::swarm_orchestration::model_lane::diagnostics_model_identity_label(
                kind,
                binding,
                provider,
                Some("unchanged-provider-id"),
                None,
                Some(&catalog),
            );
        assert!(label.contains("unchanged-provider-id"));
        assert!(reason.is_none());
    }
    let original_lane = bound
        .replay_run(&registered.run_id)
        .await
        .unwrap()
        .lanes
        .into_iter()
        .find(|item| item.lane_id == registered.lane_id)
        .unwrap();
    drop(drifted);
    drop(bound);
    let Harness {
        mut isolated,
        storage,
        store,
        exact,
        posture,
    } = harness;
    let original_scope_path = isolated.scope_path().to_owned();
    let original_marker_path = isolated.owner_marker_path_for_proof().to_owned();
    let original_marker = std::fs::read(&original_marker_path).unwrap();
    let original_config = storage.config().clone();
    drop(store);
    drop(storage);
    isolated
        .reopen()
        .await
        .expect("same owner closes and reopens exact storage config");
    assert_eq!(isolated.scope_path(), original_scope_path);
    assert_eq!(isolated.owner_marker_path_for_proof(), original_marker_path);
    assert_eq!(
        std::fs::read(&original_marker_path).unwrap(),
        original_marker
    );
    let reopened_storage = isolated.storage().clone();
    assert_eq!(reopened_storage.config(), &original_config);
    let reopened = ModelLaneStore::new_scoped(reopened_storage.clone(), resource_scope(&exact))
        .with_model_catalog(reminted_catalog.clone());
    assert_original_lane_anchor(
        &reopened_storage,
        &exact,
        &registered.lane_id,
        Some(&"5a".repeat(32)),
    )
    .await;
    assert_original_lane_anchor(&reopened_storage, &exact, &posture.lane_id, None).await;
    let reopened_lane = reopened
        .replay_run(&registered.run_id)
        .await
        .unwrap()
        .lanes
        .into_iter()
        .find(|item| item.lane_id == registered.lane_id)
        .unwrap();
    assert_eq!(reopened_lane.inner, original_lane.inner);
    assert_eq!(
        reopened_lane.event_ledger_event_id,
        original_lane.event_ledger_event_id
    );
    assert_eq!(
        reopened_lane.event_ledger_seq,
        original_lane.event_ledger_seq
    );
    let reopened_projection = reopened
        .diagnostics_projection_with_model_catalog(&registered.run_id, Some(&reminted_catalog))
        .await
        .unwrap();
    let reopened_projected_lane = reopened_projection
        .lanes
        .iter()
        .find(|item| item.lane_id == registered.lane_id)
        .unwrap();
    assert_eq!(
        reopened_projected_lane.model_stable_anchor.as_deref(),
        Some("5a".repeat(32).as_str())
    );
    assert_eq!(
        reopened_projected_lane.model_display_name,
        "diagnostics-catalog-fixture"
    );
    assert_eq!(
        reopened_lane.model_id, original_lane.model_id,
        "reopen retains original model UUID despite new catalog UUID"
    );
    drop(reopened);
    drop(reopened_storage);
    isolated.shutdown_and_cleanup().await.unwrap();
    assert!(!original_scope_path.exists());
    assert!(!original_marker_path.exists());
}

#[tokio::test]
async fn diagnostics_by_id_and_latest_are_absent_shaped_for_every_foreign_scope_field() {
    let harness = Harness::create("owner-filter").await;
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        harness
            .store
            .record_diagnostic_tier_status(tier_status(&harness.posture, tier))
            .await
            .expect("record complete owner diagnostics");
    }
    let owner_projection = harness
        .store
        .diagnostics_projection(&harness.posture.run_id)
        .await
        .expect("positive owner projection");
    let owner_receipts = harness
        .store
        .test_scoped_authority_receipts(&harness.posture.run_id, 128)
        .await
        .expect("owner authority watermark");

    for foreign_exact in one_field_mismatches(&harness.exact) {
        let foreign =
            ModelLaneStore::new_scoped(harness.storage.clone(), resource_scope(&foreign_exact));
        assert!(matches!(
            foreign
                .diagnostics_projection(&harness.posture.run_id)
                .await,
            Err(ModelLaneError::NotFound(_))
        ));
        assert!(matches!(
            foreign.latest_diagnostics_projection().await,
            Err(ModelLaneError::NotFound(_))
        ));
    }
    assert_eq!(
        harness
            .store
            .diagnostics_projection(&harness.posture.run_id)
            .await
            .expect("owner projection remains available"),
        owner_projection
    );
    assert_eq!(
        harness
            .store
            .test_scoped_authority_receipts(&harness.posture.run_id, 128)
            .await
            .expect("foreign denials are non-mutating"),
        owner_receipts
    );
    harness.close().await;
}

#[tokio::test]
async fn diagnostics_projection_rejects_tampered_projection_event_link() {
    let harness = Harness::create("tamper").await;
    let mut records = Vec::new();
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        records.push(
            harness
                .store
                .record_diagnostic_tier_status(tier_status(&harness.posture, tier))
                .await
                .expect("record canonical diagnostic tier"),
        );
    }
    harness
        .store
        .diagnostics_projection(&harness.posture.run_id)
        .await
        .expect("positive projection before corruption");
    harness
        .store
        .test_corrupt_scoped_authority(
            "diagnostic_tier",
            &records[1].diagnostic_status_id,
            ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        )
        .await
        .expect("inject deterministic receipt corruption");
    assert!(harness
        .store
        .diagnostics_projection(&harness.posture.run_id)
        .await
        .is_err());
    harness.close().await;
}

fn tier_status(
    posture: &SurrealAdmissibleCrdtPosture,
    tier: ModelLaneDiagnosticTier,
) -> NewModelLaneDiagnosticTierStatus {
    let tier_name = tier.as_str();
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diagnostic-{}-{tier_name}", posture.run_id),
        behavior_id: "HBR-INT-009".into(),
        run_id: posture.run_id.clone(),
        tier,
        state: ModelLaneDiagnosticTierState::Wired,
        reason: format!("{tier_name} is wired to canonical authority"),
        evidence_ref: format!("eventledger://diagnostics/{}/{tier_name}", posture.run_id),
        follow_up_ref: Some("usermanual://model-lane/diagnostics".into()),
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("idem-diagnostic-{}-{tier_name}", posture.run_id),
        diagnostic_payload: json!({"tier": tier_name, "private": "diagnostic-secret"}),
    }
}

fn exact_scope(label: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-diagnostics-{label}"))
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
        WorkspaceScopeRef::new("workspace-diagnostics-foreign").expect("workspace");
    vec![owner, actor, session, access, workspace]
}

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::storage::surreal::resource_authority::{ResourceAction, ResourceGrantSpec};
use std::sync::Arc;
use tower::ServiceExt;
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
        _diag: Diagnostic,
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

struct DiagnosticsCatalogGate {
    armed: std::sync::atomic::AtomicBool,
    entered: tokio::sync::Notify,
    released: std::sync::Mutex<bool>,
    resume: std::sync::Condvar,
}

impl DiagnosticsCatalogGate {
    fn new() -> Self {
        Self {
            armed: std::sync::atomic::AtomicBool::new(false),
            entered: tokio::sync::Notify::new(),
            released: std::sync::Mutex::new(false),
            resume: std::sync::Condvar::new(),
        }
    }
}

struct DiagnosticsCatalogClient {
    inner: handshake_core::llm::InMemoryLlmClient,
    catalog: Arc<handshake_core::model_runtime::ModelCatalog>,
    calls: std::sync::atomic::AtomicUsize,
    gate: DiagnosticsCatalogGate,
}

#[async_trait]
impl handshake_core::llm::LlmClient for DiagnosticsCatalogClient {
    async fn completion(
        &self,
        request: handshake_core::llm::CompletionRequest,
    ) -> Result<handshake_core::llm::CompletionResponse, handshake_core::llm::LlmError> {
        handshake_core::llm::LlmClient::completion(&self.inner, request).await
    }
    fn profile(&self) -> &handshake_core::llm::ModelProfile {
        handshake_core::llm::LlmClient::profile(&self.inner)
    }
    fn model_catalog(&self) -> Option<Arc<handshake_core::model_runtime::ModelCatalog>> {
        use std::sync::atomic::Ordering;
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.gate.armed.swap(false, Ordering::SeqCst) {
            // This hook is reached inside the admitted handler, before its real store read.
            self.gate.entered.notify_one();
            let released = self.gate.released.lock().expect("catalog gate lock");
            let (released, deadline) = self
                .gate
                .resume
                .wait_timeout_while(released, std::time::Duration::from_secs(5), |released| {
                    !*released
                })
                .expect("bounded catalog gate");
            assert!(
                *released && !deadline.timed_out(),
                "fixture must release admitted store-read gate"
            );
        }
        Some(self.catalog.clone())
    }
}

// The mounted route uses real persisted credentials and grants. Host scope supplies only workspace.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_route_requires_persisted_session_capability_and_workspace_grant() {
    let harness = Harness::create("route-admission").await;
    let _binding_lock = account_session_support::NATIVE_BINDING_ENV_LOCK
        .lock()
        .await;
    let binding = account_session_support::NativeBindingEnv::install();
    let token_hash = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(binding.token().as_bytes()))
    };
    let key = format!("diagnostics-analyst-{}", Uuid::now_v7());
    let principal = harness
        .storage
        .provision_principal(
            &key,
            &key,
            "human_account",
            &key,
            "Analyst",
            &["diagnostics.read".to_owned()],
            &key,
            Some(&token_hash),
            std::time::Duration::from_secs(3600),
        )
        .await
        .expect("persist diagnostics Analyst session");
    let resource = harness
        .storage
        .register_workspace_resource(&principal.identity, harness.exact.workspace_id.as_str())
        .await
        .expect("register owned diagnostics workspace");
    let catalog_model_id = handshake_core::model_runtime::ModelId::new_v7();
    let mut registry = handshake_core::model_runtime::ModelRegistry::default();
    registry
        .register(handshake_core::model_runtime::ModelRegistration {
            model_id: catalog_model_id.clone(),
            artifact_path: "models/diagnostics-fixture.gguf".into(),
            sha256: [0x5a; 32],
            runtime_binding: handshake_core::model_runtime::RuntimeBinding::LlamaCpp,
            declared_capabilities: handshake_core::model_runtime::ModelCapabilities::default(),
            base_model_tag: handshake_core::model_runtime::BaseModelTag::new(
                "diagnostics-catalog-fixture",
            ),
            registered_at_utc: chrono::Utc::now(),
            registered_by: handshake_core::model_runtime::OperatorId::new("diagnostics-fixture"),
            provider: handshake_core::model_runtime::ProviderKind::Local,
        })
        .expect("register actual catalog fixture model");
    registry
        .mark_loaded(catalog_model_id.clone())
        .expect("mark catalog model loaded");
    let catalog = handshake_core::model_runtime::ModelCatalog::from_registry(Arc::new(registry));
    assert_eq!(
        catalog
            .entry_for_stable_anchor(&"5a".repeat(32))
            .unwrap()
            .display_name,
        "diagnostics-catalog-fixture",
        "actual current catalog has a loaded stable anchor"
    );
    let catalog_client = Arc::new(DiagnosticsCatalogClient {
        inner: handshake_core::llm::InMemoryLlmClient::new("diagnostics-admission".into()),
        catalog: Arc::clone(&catalog),
        calls: std::sync::atomic::AtomicUsize::new(0),
        gate: DiagnosticsCatalogGate::new(),
    });
    let recorder = Arc::new(NoopRecorder);
    let state = handshake_core::AppState {
        storage: Arc::new(handshake_core::storage::surreal::SurrealDatabase::new(
            harness.storage.clone(),
        )),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: catalog_client.clone(),
        capability_registry: Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
        session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new(
            handshake_core::workflows::SessionSchedulerConfig::default(),
        )),
    };
    // The four host identity dimensions deliberately differ from the actual principal.
    let app = handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
        handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
            harness.exact.clone(),
        )
        .expect("validated host workspace selection"),
    ));
    let send = |token: Option<String>,
                channel: Option<String>,
                owner: Option<String>,
                method: &str,
                uri: &str| {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header(account_session_support::SESSION_TOKEN_HEADER, token);
        }
        if let Some(channel) = channel {
            request = request.header(
                account_session_support::CHANNEL_BINDING_TOKEN_HEADER,
                channel,
            );
        }
        if let Some(owner) = owner {
            request = request.header("x-handshake-owner-account", owner);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).expect("diagnostics request"))
    };
    let path = "/swarm/model-lanes/diagnostics/latest";
    for (token, channel) in [
        (None, None),
        (Some("malformed".into()), Some(binding.token().to_owned())),
        (None, Some(binding.token().to_owned())),
    ] {
        assert_eq!(
            send(token, channel, None, "GET", path)
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    for (profile, delegated) in [("Analyst", "fs.read"), ("Operator", "diagnostics.read")] {
        let key = format!("diagnostics-negative-{profile}-{}", Uuid::now_v7());
        let negative = harness
            .storage
            .provision_principal(
                &key,
                &key,
                "human_account",
                &key,
                profile,
                &[delegated.to_owned()],
                &key,
                Some(&token_hash),
                std::time::Duration::from_secs(3600),
            )
            .await
            .expect("persist exact negative authority principal");
        let workspace = format!("diagnostics-negative-workspace-{}", Uuid::now_v7());
        let resource = harness
            .storage
            .register_workspace_resource(&negative.identity, &workspace)
            .await
            .expect("register negative principal workspace");
        harness
            .storage
            .grant_resource(
                &negative.identity.account_id,
                &negative.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: negative.identity.principal_id.clone(),
                    resource_id: resource.resource_id,
                    actions: vec![ResourceAction::Read],
                    capability_ids: vec!["diagnostics.read".into()],
                    expires_at: None,
                    delegation_chain: vec![],
                },
            )
            .await
            .expect("persist negative exact read grant");
        let mut host_selection = harness.exact.clone();
        host_selection.workspace_id = WorkspaceScopeRef::new(workspace).unwrap();
        let negative_app =
            handshake_core::api::diagnostics::routes(state.clone()).layer(Extension(
                handshake_core::api::account_scope::ProductLocalResourceScope::from_exact(
                    host_selection,
                )
                .unwrap(),
            ));
        let response = negative_app
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header(
                        account_session_support::SESSION_TOKEN_HEADER,
                        &negative.session.token,
                    )
                    .header(
                        account_session_support::CHANNEL_BINDING_TOKEN_HEADER,
                        binding.token(),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN,
            "actual session plus grant must not bypass missing delegated capability or profile admission");
    }
    let credentials = || {
        (
            Some(principal.session.token.clone()),
            Some(binding.token().to_owned()),
        )
    };
    let (token, channel) = credentials();
    assert_eq!(
        send(token, channel, None, "GET", path)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN,
        "persisted session and profile cannot substitute for a workspace grant"
    );
    harness
        .storage
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id: resource.resource_id,
                actions: vec![ResourceAction::Read],
                capability_ids: vec!["diagnostics.read".into()],
                expires_at: None,
                delegation_chain: vec![],
            },
        )
        .await
        .expect("grant diagnostics read");
    let (token, channel) = credentials();
    assert_eq!(
        send(
            token,
            channel,
            Some(principal.identity.account_id.clone()),
            "GET",
            path
        )
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND,
        "foreign host identity must not select its seeded diagnostics run"
    );
    let (token, channel) = credentials();
    assert_eq!(
        send(
            token,
            channel,
            Some(harness.exact.owner_account_id.as_uuid().to_string()),
            "GET",
            path
        )
        .await
        .unwrap()
        .status(),
        StatusCode::FORBIDDEN,
        "host owner assertion cannot replace persisted owner"
    );
    for (method, uri) in [
        ("GET", "/diagnostics"),
        ("GET", "/diagnostics/problems"),
        ("GET", "/diagnostics/00000000-0000-0000-0000-000000000001"),
        ("POST", "/diagnostics"),
    ] {
        let (token, channel) = credentials();
        assert_eq!(
            send(token, channel, None, method, uri)
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN,
            "legacy contents and mutation are not admitted by a read grant"
        );
    }
    let admitted_exact: ExactResourceScopeAttribution = serde_json::from_value(json!({
        "owner_account_id": principal.identity.account_id,
        "actor_principal_id": principal.identity.principal_id,
        "authenticated_session_id": principal.session.session_id,
        "access_space_id": principal.identity.access_space_id,
        "workspace_id": harness.exact.workspace_id.as_str(),
    }))
    .expect("original persisted principal scope");
    let admitted_store =
        ModelLaneStore::new_scoped(harness.storage.clone(), resource_scope(&admitted_exact))
            .with_model_catalog(Arc::clone(&catalog));
    let admitted_posture = build_surreal_admissible_crdt_posture_with_model(
        &admitted_store,
        admitted_exact.workspace_id.as_str(),
        "diagnostics-admitted-route",
        &catalog_model_id.to_string(),
    )
    .await
    .expect("seed admitted diagnostic run");
    admitted_store
        .record_message(admitted_posture.message.clone())
        .await
        .expect("record admitted message");
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        admitted_store
            .record_diagnostic_tier_status(tier_status(&admitted_posture, tier))
            .await
            .expect("complete admitted HBR posture");
    }
    for uri in [
        path.to_owned(),
        format!("/swarm/model-lanes/diagnostics/{}", admitted_posture.run_id),
    ] {
        let (token, channel) = credentials();
        let response = send(token, channel, None, "GET", &uri).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "actual Analyst/session/grant admits its projection"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let projection: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(projection["run"]["run_id"], admitted_posture.run_id);
        let lane = projection["lanes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|lane| lane["lane_id"] == admitted_posture.lane_id)
            .expect("mounted local lane");
        assert_eq!(
            lane["model_stable_anchor"],
            "5a".repeat(32),
            "original registered producer persisted artifact anchor"
        );
        assert_eq!(lane["model_display_name"], "diagnostics-catalog-fixture");
        assert!(lane["model_anchor_unavailable_reason"].is_null());

        for field in [
            "owner_account_fingerprint",
            "actor_principal_fingerprint",
            "authenticated_session_fingerprint",
            "access_space_fingerprint",
            "workspace_fingerprint",
        ] {
            assert_eq!(
                projection["resource_scope"][field].as_str().unwrap().len(),
                64
            );
        }
        let text = String::from_utf8(body.to_vec()).unwrap();
        for raw in [
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.session.session_id,
            &principal.identity.access_space_id,
        ] {
            assert!(
                !text.contains(raw),
                "diagnostics must not disclose raw authority identity"
            );
        }
    }
    let (token, channel) = credentials();
    let foreign_response = send(
        token,
        channel,
        None,
        "GET",
        &format!("/swarm/model-lanes/diagnostics/{}", harness.posture.run_id),
    )
    .await
    .unwrap();
    assert_eq!(foreign_response.status(), StatusCode::NOT_FOUND);
    let foreign_body = axum::body::to_bytes(foreign_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(!String::from_utf8(foreign_body.to_vec())
        .unwrap()
        .contains(&harness.posture.run_id));
    assert!(
        catalog_client
            .calls
            .load(std::sync::atomic::Ordering::SeqCst)
            >= 2,
        "both admitted endpoint handlers consume the actual mounted catalog"
    );
    for (owner_assertion, workspace_assertion, expected) in [
        (
            None,
            Some(harness.exact.workspace_id.as_str()),
            StatusCode::OK,
        ),
        (None, Some("another-workspace"), StatusCode::FORBIDDEN),
        (None, Some(" "), StatusCode::FORBIDDEN),
        (Some("not-a-uuid"), None, StatusCode::FORBIDDEN),
        (Some(" "), None, StatusCode::FORBIDDEN),
    ] {
        let mut request = Request::builder()
            .uri(path)
            .header(
                account_session_support::SESSION_TOKEN_HEADER,
                &principal.session.token,
            )
            .header(
                account_session_support::CHANNEL_BINDING_TOKEN_HEADER,
                binding.token(),
            );
        if let Some(owner) = owner_assertion {
            request = request.header("x-handshake-owner-account", owner);
        }
        if let Some(workspace) = workspace_assertion {
            request = request.header("x-handshake-workspace", workspace);
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let rotated = harness
        .storage
        .issue_authenticated_session(
            &principal.identity.account_id,
            &principal.identity.principal_id,
            &principal.identity.access_space_id,
            Some(&token_hash),
            std::time::Duration::from_secs(3600),
        )
        .await
        .expect("issue real second authenticated session");
    assert_ne!(rotated.session_id, principal.session.session_id);
    let response = send(
        Some(rotated.token.clone()),
        Some(binding.token().to_owned()),
        None,
        "GET",
        path,
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "new actual session must not impersonate the original exact-session projection"
    );
    let rotated_exact: ExactResourceScopeAttribution = serde_json::from_value(json!({
        "owner_account_id": principal.identity.account_id, "actor_principal_id": principal.identity.principal_id,
        "authenticated_session_id": rotated.session_id, "access_space_id": principal.identity.access_space_id,
        "workspace_id": harness.exact.workspace_id.as_str(),
    })).unwrap();
    let rotated_store =
        ModelLaneStore::new_scoped(harness.storage.clone(), resource_scope(&rotated_exact))
            .with_model_catalog(Arc::clone(&catalog));
    let rotated_posture = build_surreal_admissible_crdt_posture_with_model(
        &rotated_store,
        rotated_exact.workspace_id.as_str(),
        "diagnostics-rotated-route",
        &catalog_model_id.to_string(),
    )
    .await
    .expect("seed original second-session projection");
    rotated_store
        .record_message(rotated_posture.message.clone())
        .await
        .unwrap();
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        rotated_store
            .record_diagnostic_tier_status(tier_status(&rotated_posture, tier))
            .await
            .unwrap();
    }
    let response = send(
        Some(rotated.token.clone()),
        Some(binding.token().to_owned()),
        None,
        "GET",
        path,
    )
    .await
    .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "new persisted session admits only its own exact projection"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let projection: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(projection["run"]["run_id"], rotated_posture.run_id);
    assert!(!String::from_utf8(body.to_vec())
        .unwrap()
        .contains(&rotated.session_id));
    harness
        .storage
        .revoke_session(&principal.session.session_id)
        .await
        .expect("revoke actual persisted session");
    let (token, channel) = credentials();
    assert_eq!(
        send(token, channel, None, "GET", path)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN,
        "revoked persisted credentials are denied before projection lookup"
    );
    catalog_client
        .gate
        .armed
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let outage_request = Request::builder()
        .uri(path)
        .header(
            account_session_support::SESSION_TOKEN_HEADER,
            &rotated.token,
        )
        .header(
            account_session_support::CHANNEL_BINDING_TOKEN_HEADER,
            binding.token(),
        )
        .body(Body::empty())
        .unwrap();
    let outage_app = app.clone();
    let request_task = tokio::spawn(async move { outage_app.oneshot(outage_request).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        catalog_client.gate.entered.notified(),
    )
    .await
    .expect("wait for post-admission handler store-read barrier");
    let shutdown = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        harness.storage.shutdown(),
    )
    .await;
    *catalog_client.gate.released.lock().unwrap() = true;
    catalog_client.gate.resume.notify_one();
    shutdown
        .expect("bounded owned storage shutdown")
        .expect("close only fixture-owned storage");
    let outage_response = request_task
        .await
        .expect("join owned route request")
        .expect("outage route response");
    assert_eq!(
        outage_response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "actual post-admission embedded store outage keeps authority-unavailable 503"
    );
    let body = axum::body::to_bytes(outage_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"],
        "MODEL_LANE_DIAGNOSTICS_AUTHORITY_UNAVAILABLE"
    );
    drop(rotated_store);
    drop(admitted_store);
    drop(send);
    drop(app);
    drop(binding);
    harness.close().await;
}
