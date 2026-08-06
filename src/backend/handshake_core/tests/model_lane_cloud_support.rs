//! Shared cloud ProjectionPlan/ConsentReceipt seeding for WP-1 ModelLane tests.
//!
//! Spec anchor: `04-llm-infrastructure.md` Section 4.3.9.2.5 requires that a
//! cloud/BYOK ModelLane resolve durable PostgreSQL/EventLedger
//! `ModelLaneCloudProjectionPlanRecord` and `ModelLaneCloudConsentReceiptRecord`
//! authority before the lane becomes durable and before any provider runtime is
//! created. The MT-002 schema suite persists cloud lanes directly through
//! `ModelLaneStore::record_lane` / `record_prepared_launch`, both of which
//! fail closed (`ProjectionPlan ... is not durable`) unless the matching
//! projection/consent rows already exist. This helper persists a spec-valid,
//! mutually-consistent projection + consent pair whose identity fields match a
//! specific cloud lane so the fail-closed durability gate is satisfied without
//! weakening it.
//!
//! Reusable by MT-002/MT-004/MT-005/MT-006/MT-007/MT-009 suites: any test that
//! records a cloud lane can seed its authority with one call before the lane is
//! recorded. The identity fields (`run_id`, `lane_id`, `model_session_id`,
//! `provider_kind`, `requested_model_id`, `projection_plan_id`,
//! `consent_receipt_id`) MUST equal the values the lane under test carries so
//! `ensure_cloud_launch_authority_tx` accepts them.

#![allow(dead_code)]

use handshake_core::swarm_orchestration::model_lane::{
    CloudExportDelegation, ModelLaneCloudConsentReceiptRecord, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanRecord,
    ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, ModelLaneStore,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan,
};
use handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority;
use serde_json::json;

/// Identity of the cloud lane whose durable projection/consent authority is
/// being seeded. Every field must match the lane record under test.
pub struct CloudLaneAuthoritySpec<'a> {
    /// Lane `run_id` (also the run's `run_id` for prepared launches).
    pub run_id: &'a str,
    /// Lane `lane_id`.
    pub lane_id: &'a str,
    /// Lane `model_session_id`.
    pub model_session_id: &'a str,
    /// Lane `provider_kind.as_str()` (e.g. `"openai"`).
    pub provider_kind: &'a str,
    /// Lane `model_id` (the requested model id).
    pub requested_model_id: &'a str,
    /// Lane `projection_plan_ref`; becomes the durable `projection_plan_id`.
    pub projection_plan_id: &'a str,
    /// Lane `consent_receipt_ref`; becomes the durable `consent_receipt_id`.
    pub consent_receipt_id: &'a str,
    /// EventLedger stream the projection/consent events append to.
    pub event_ledger_stream_id: &'a str,
    /// Locus work packet id for the authority rows.
    pub work_packet_id: &'a str,
    /// Locus micro task id for the authority rows.
    pub micro_task_id: &'a str,
    /// Locus task board id for the authority rows.
    pub task_board_id: &'a str,
    /// Owner session for the authority rows.
    pub owner_session: &'a str,
}

const SCOPE_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PAYLOAD_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
// Wide validity window so the consent receipt is current on any wall clock.
const VALID_FROM_UTC: &str = "2020-01-01T00:00:00Z";
const VALID_UNTIL_UTC: &str = "2099-01-01T00:00:00Z";
const USER_MANUAL_BEHAVIOR: &str = "usermanual://model-lane-cloud-projection-consent#launch";

/// Persist a durable, mutually-consistent ProjectionPlan + ConsentReceipt pair
/// that satisfies the cloud-lane durability gate for the lane described by
/// `spec`. Panics on any storage error so a broken authority path can never
/// look green.
pub async fn seed_cloud_lane_authority(
    store: &ModelLaneStore,
    spec: CloudLaneAuthoritySpec<'_>,
) -> (
    ModelLaneCloudProjectionPlanRecord,
    ModelLaneCloudConsentReceiptRecord,
) {
    let fan_out_targets = vec![format!("provider://{}/byok", spec.provider_kind)];

    let plan = NewModelLaneCloudProjectionPlan {
        projection_plan_id: spec.projection_plan_id.into(),
        run_id: spec.run_id.into(),
        trace_id: format!("trace-{}", spec.run_id),
        lane_id: Some(spec.lane_id.into()),
        model_session_id: Some(spec.model_session_id.into()),
        provider_kind: Some(spec.provider_kind.into()),
        requested_model_id: Some(spec.requested_model_id.into()),
        scope_hash: SCOPE_HASH.into(),
        source_artifact_refs: vec![
            format!(
                "artifact-store://{}/{}/{}/context.json",
                spec.work_packet_id, spec.run_id, spec.lane_id
            ),
            "context-bundle://model-lane/cloud-safe".into(),
        ],
        payload_artifact_ref: format!(
            "artifact-store://{}/{}/{}/payload.json",
            spec.work_packet_id, spec.run_id, spec.lane_id
        ),
        payload_sha256: PAYLOAD_SHA256.into(),
        redaction_policy_ref: "redaction-policy://model-lane/cloud-safe".into(),
        redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        provider_profile_ref: format!("provider-profile://model-lane/{}", spec.provider_kind),
        fan_out_targets: fan_out_targets.clone(),
        // These fixtures seed through an UNSCOPED `ModelLaneStore::new(pool)`,
        // which has no account context, so the only source scope such a store is
        // allowed to stamp is the explicitly unattributed one. That is the honest
        // record for a pre-WP-KERNEL-006 call site and it keeps this helper from
        // becoming a way to fabricate account-bound authority in tests.
        export_delegation: CloudExportDelegation {
            audience_refs: fan_out_targets.clone(),
            source_scope: AccountBoundAuthority::unattributed(
                "MODEL_LANE_TEST_FIXTURE_WITHOUT_ACCOUNT_CONTEXT",
            ),
            authorization_receipt_ref: None,
        },
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        status: ModelLaneCloudProjectionPlanStatus::Active,
        event_ledger_stream_id: spec.event_ledger_stream_id.into(),
        work_packet_id: spec.work_packet_id.into(),
        micro_task_id: spec.micro_task_id.into(),
        task_board_id: spec.task_board_id.into(),
        owner_session: spec.owner_session.into(),
        idempotency_key: format!("idem-projection-{}-{}", spec.run_id, spec.lane_id),
        created_at_utc: "2026-06-29T09:00:00Z".into(),
        user_manual_behavior_ref: USER_MANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "internal_diagnostics": "deferred: native diagnostic surface ships separately",
            "palmistry": "deferred: external watcher linked by behavior ref when available",
            "locus": format!("locus://{}/{}/{}", spec.work_packet_id, spec.run_id, spec.lane_id)
        }),
    };

    let stored_plan = store
        .record_cloud_projection_plan(plan)
        .await
        .expect("record durable cloud ProjectionPlan authority");

    let receipt = NewModelLaneCloudConsentReceipt {
        consent_receipt_id: spec.consent_receipt_id.into(),
        projection_plan_id: stored_plan.projection_plan_id.clone(),
        projection_plan_hash: stored_plan.projection_plan_hash.clone(),
        run_id: spec.run_id.into(),
        trace_id: format!("trace-{}", spec.run_id),
        lane_id: Some(spec.lane_id.into()),
        model_session_id: Some(spec.model_session_id.into()),
        provider_kind: Some(spec.provider_kind.into()),
        requested_model_id: Some(spec.requested_model_id.into()),
        scope_hash: SCOPE_HASH.into(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets,
        approved: true,
        approver: AccountBoundAuthority::unattributed(
            "MODEL_LANE_TEST_FIXTURE_WITHOUT_ACCOUNT_CONTEXT",
        ),
        approved_by_ref: "operator://model-lane/approval".into(),
        approved_at_utc: "2026-06-29T09:00:10Z".into(),
        valid_from_utc: VALID_FROM_UTC.into(),
        valid_until_utc: VALID_UNTIL_UTC.into(),
        revoked_at_utc: None,
        revocation_ref: None,
        revocation_input_hash: None,
        status: ModelLaneCloudConsentReceiptStatus::Approved,
        event_ledger_stream_id: spec.event_ledger_stream_id.into(),
        work_packet_id: spec.work_packet_id.into(),
        micro_task_id: spec.micro_task_id.into(),
        task_board_id: spec.task_board_id.into(),
        owner_session: spec.owner_session.into(),
        idempotency_key: format!("idem-consent-{}-{}", spec.run_id, spec.lane_id),
        created_at_utc: "2026-06-29T09:00:15Z".into(),
        user_manual_behavior_ref: USER_MANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "provider_call_attempted": false,
            "locus": format!("locus://{}/{}/{}", spec.work_packet_id, spec.run_id, spec.lane_id)
        }),
    };

    let stored_receipt = store
        .record_cloud_consent_receipt(receipt)
        .await
        .expect("record durable cloud ConsentReceipt authority");

    (stored_plan, stored_receipt)
}
