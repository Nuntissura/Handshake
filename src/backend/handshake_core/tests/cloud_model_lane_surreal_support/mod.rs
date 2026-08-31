use std::path::PathBuf;

use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
use handshake_core::swarm_orchestration::model_lane::{
    CloudExportDelegation, LaunchAuthority, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneKind, ModelLaneLocusBinding, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneStatus, ModelLaneStore, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, AccountBoundAuthority, ActorPrincipalId, AuthenticatedSessionRef,
    OwnerAccountId, ResourceAccessContext, ResourceScope, WorkspaceScopeRef,
};
use serde_json::json;
use uuid::Uuid;

pub const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
pub const MT_ID: &str = "MT-006";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const OWNER: &str = "KERNEL_BUILDER-MT006";
const MANUAL: &str = "usermanual://model-lane-cloud-projection-consent#launch";

pub struct Harness {
    pub root: PathBuf,
    pub storage: SurrealStorage,
    pub store: ModelLaneStore,
    pub scope: ResourceScope,
}

impl Harness {
    pub async fn create(slug: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "handshake-mt006-surreal-{slug}-{}",
            Uuid::now_v7().simple()
        ));
        std::fs::create_dir_all(&root).expect("create isolated MT-006 SurrealDB root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(&root)
                .expect("construct isolated embedded SurrealDB config"),
        )
        .await
        .expect("open isolated embedded SurrealDB");
        let scope = exact_scope(slug);
        let store = ModelLaneStore::new_surreal_cloud_authority_only(
            ResourceAccessContext::for_account(scope.clone()),
            storage.clone(),
        )
        .await
        .expect("bootstrap MT-006 SurrealDB authority schema");
        Self {
            root,
            storage,
            store,
            scope,
        }
    }

    pub async fn store_for_scope(&self, scope: ResourceScope) -> ModelLaneStore {
        ModelLaneStore::new_surreal_cloud_authority_only(
            ResourceAccessContext::for_account(scope),
            self.storage.clone(),
        )
        .await
        .expect("bind second exact scope to the same embedded database")
    }

    pub async fn close(self) {
        self.storage
            .shutdown()
            .await
            .expect("close isolated embedded SurrealDB");
        std::fs::remove_dir_all(&self.root).expect("remove owned isolated MT-006 test root");
    }
}

pub fn exact_scope(slug: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt006-{slug}"))
                .expect("nonblank workspace scope"),
        )
}

pub fn valid_window() -> (String, String) {
    let now = chrono::Utc::now();
    (
        (now - chrono::Duration::minutes(5)).to_rfc3339(),
        (now + chrono::Duration::hours(12)).to_rfc3339(),
    )
}

pub fn expired_window() -> (String, String) {
    let now = chrono::Utc::now();
    (
        (now - chrono::Duration::hours(24)).to_rfc3339(),
        (now - chrono::Duration::hours(12)).to_rfc3339(),
    )
}

pub fn projection(
    run_id: &str,
    lane_id: &str,
    scope: &ResourceScope,
) -> NewModelLaneCloudProjectionPlan {
    NewModelLaneCloudProjectionPlan {
        projection_plan_id: projection_id(run_id, lane_id),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: Some(lane_id.into()),
        model_session_id: Some(model_session_id(lane_id)),
        provider_kind: Some("openai".into()),
        requested_model_id: Some(model_id()),
        scope_hash: sha('a'),
        source_artifact_refs: vec![format!("artifact-store://mt006/{run_id}/context.json")],
        payload_artifact_ref: format!("artifact-store://mt006/{run_id}/payload.json"),
        payload_sha256: sha('b'),
        redaction_policy_ref: "redaction-policy://mt006/cloud-safe".into(),
        redaction_summary: "local-only secrets excluded".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        provider_profile_ref: "provider-profile://mt006/openai".into(),
        fan_out_targets: vec!["provider://openai/byok".into()],
        export_delegation: CloudExportDelegation {
            audience_refs: vec!["provider://openai/byok".into()],
            source_scope: AccountBoundAuthority::from_scope(scope),
            authorization_receipt_ref: None,
        },
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        status: ModelLaneCloudProjectionPlanStatus::Active,
        event_ledger_stream_id: stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-projection-{run_id}-{lane_id}"),
        created_at_utc: "2026-08-30T12:00:00Z".into(),
        user_manual_behavior_ref: MANUAL.into(),
        diagnostic_payload: json!({"storage_authority": "embedded_surrealdb"}),
    }
}

pub fn receipt(
    run_id: &str,
    lane_id: &str,
    projection_hash: &str,
    scope: &ResourceScope,
    valid_from: &str,
    valid_until: &str,
) -> NewModelLaneCloudConsentReceipt {
    NewModelLaneCloudConsentReceipt {
        consent_receipt_id: receipt_id(run_id, lane_id),
        projection_plan_id: projection_id(run_id, lane_id),
        projection_plan_hash: projection_hash.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_id: Some(lane_id.into()),
        model_session_id: Some(model_session_id(lane_id)),
        provider_kind: Some("openai".into()),
        requested_model_id: Some(model_id()),
        scope_hash: sha('a'),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets: vec!["provider://openai/byok".into()],
        approved: true,
        approver: AccountBoundAuthority::from_scope(scope),
        approved_by_ref: "operator-action://mt006/proof-approval".into(),
        approved_at_utc: "2026-08-30T12:00:01Z".into(),
        valid_from_utc: valid_from.into(),
        valid_until_utc: valid_until.into(),
        revoked_at_utc: None,
        revocation_ref: None,
        revocation_input_hash: None,
        status: ModelLaneCloudConsentReceiptStatus::Approved,
        event_ledger_stream_id: stream_id(run_id),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-consent-{run_id}-{lane_id}"),
        created_at_utc: "2026-08-30T12:00:02Z".into(),
        user_manual_behavior_ref: MANUAL.into(),
        diagnostic_payload: json!({"provider_call_attempted": false}),
    }
}

pub fn cloud_lane(run_id: &str, lane_id: &str) -> NewModelLane {
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: stream_id(run_id),
        kind: ModelLaneKind::CloudModel,
        role: "cloud-review-lane".into(),
        backend: "cloud_lane_openai".into(),
        model_id: Some(model_id()),
        session_id: format!("session-{lane_id}"),
        model_session_id: model_session_id(lane_id),
        adapter_id: "openai_byok".into(),
        runtime_binding: RuntimeBinding::Cloud,
        launch_authority: LaunchAuthority::CloudLane,
        provider_kind: ModelLaneProviderKind::OpenAi,
        capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://{lane_id}")),
        provider_feature_profile_ref: Some("provider-profile://mt006/openai".into()),
        requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane_id}")),
        effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane_id}")),
        projection_plan_ref: Some(projection_id(run_id, lane_id)),
        consent_receipt_ref: Some(receipt_id(run_id, lane_id)),
        tool_gate_decision_refs: vec!["toolgate://mt006/cloud-read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-08-30T12:01:00Z".into()),
        lease_expires_at_utc: Some("2026-08-30T12:10:00Z".into()),
        reclaim_after_utc: Some("2026-08-30T12:11:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt006/cloud".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt006/cloud".into()),
        process_ownership_ref: Some(format!("process-ledger://{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt006".into()),
        last_runtime_status_ref: Some("runtime-status://cloud-ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-cloud-projection-consent#recovery".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some(TASK_BOARD_ID.into()),
        owner_session: OWNER.into(),
        locus_binding: Some(ModelLaneLocusBinding {
            work_packet_id: WP_ID.into(),
            micro_task_id: MT_ID.into(),
            task_board_id: Some(TASK_BOARD_ID.into()),
            coordinator_session_id: format!("coordinator-session-{run_id}"),
            session_id: format!("session-{lane_id}"),
            model_session_id: model_session_id(lane_id),
            owner_session: OWNER.into(),
            locus_binding_ref: format!("locus://wp1/mt006/{run_id}/{lane_id}"),
        }),
    }
}

fn projection_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-projection-plan://{run_id}/{lane_id}")
}

fn receipt_id(run_id: &str, lane_id: &str) -> String {
    format!("cloud-consent-receipt://{run_id}/{lane_id}")
}

fn model_session_id(lane_id: &str) -> String {
    format!("model-session-{lane_id}")
}

fn stream_id(run_id: &str) -> String {
    format!("mlane-stream-{run_id}")
}

fn model_id() -> String {
    "model://dexterity/byok_cloud/gpt-4o-mini".into()
}

fn sha(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}
