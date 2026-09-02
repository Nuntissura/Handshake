//! WP-1 diagnostics projection, HBR posture, and exact-scope proof over embedded SurrealDB.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneAuthorityTestCorruption, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneError, ModelLaneStore, NewModelLaneDiagnosticTierStatus,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    exact: ExactResourceScopeAttribution,
    store: ModelLaneStore,
    posture: SurrealAdmissibleCrdtPosture,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate diagnostics database");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate injected SurrealStorage");
        bootstrap_schema(&storage).await.expect("bootstrap schema");
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

    async fn close(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("cleanup diagnostics database");
    }
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
    harness.close().await;
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
