//! WP-1 MT-009: UserManual behavior coverage must be backed by Rust
//! coverage matrix/contract entries, real UserManual rows, and the
//! ModelLane schema registry.

mod knowledge_pg_support;

use handshake_core::swarm_orchestration::model_lane::ModelLaneStore;
use handshake_core::user_manual::seed::ensure_seeded;
use handshake_core::user_manual::store::UserManualStore;
use handshake_core::user_manual::{
    model_lane_behavior_coverage_matrix, verify_model_lane_behavior_coverage, DiagnosticTierPosture,
};
use sqlx::postgres::PgPoolOptions;
use std::collections::BTreeSet;

#[tokio::test]
async fn mixed_model_lane_behaviors_have_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for mixed_model_lane_behaviors_have_manual_coverage: \
             UserManual behavior coverage proof requires live PostgreSQL/EventLedger"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual behavior coverage corpus");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect ModelLane store to isolated schema");
    let model_lane_store = ModelLaneStore::new(pool);
    let schema_registry = model_lane_store
        .schema_registry_rows()
        .await
        .expect("read ModelLane schema registry");
    let manual_store = UserManualStore::new(&pg.db);
    let pages = manual_store
        .list_pages(None, None, 1_000)
        .await
        .expect("read UserManual pages");
    let tools = manual_store
        .list_tool_entries(None, None, 1_000)
        .await
        .expect("read UserManual tools");

    let matrix = model_lane_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        behavior_ids,
        BTreeSet::from([
            "wp1.model_lane.run",
            "wp1.model_lane.launch",
            "wp1.model_lane.message",
            "wp1.model_lane.terminal",
            "wp1.model_lane.promotion",
            "wp1.model_lane.context_bundle_artifact",
            "wp1.model_lane.context_bundle",
            "wp1.model_lane.cloud_projection_plan",
            "wp1.model_lane.cloud_consent",
            "wp1.model_lane.cloud_consent_denial",
            "wp1.model_lane.recovery",
            "wp1.model_lane.recovery_event",
            "wp1.model_lane.lease",
            "wp1.model_lane.diagnostics",
            "wp1.model_lane.mixed_validation",
        ]),
        "behavior coverage matrix must stay exact for WP-1 model-lane behaviors"
    );
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );
    let registry_schema_ids = schema_registry
        .iter()
        .map(|row| row.schema_id.as_str())
        .collect::<BTreeSet<_>>();
    let coverage_schema_ids = matrix
        .iter()
        .filter_map(|row| row.schema_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        coverage_schema_ids, registry_schema_ids,
        "every ModelLane schema registry row must have first-class UserManual behavior coverage"
    );
    for required_schema in [
        "hsk.model_lane_terminal@1",
        "hsk.model_lane_cloud_projection_plan@1",
        "hsk.model_lane_cloud_consent_denial@1",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_recovery_event@1",
        "hsk.model_lane_lease@1",
    ] {
        assert!(
            coverage_schema_ids.contains(required_schema),
            "coverage matrix must include first-class schema coverage for {required_schema}"
        );
    }
    for row in &matrix {
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::Wired,
            "{} must keep internal_diagnostics wired",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::DeferredWithReason,
            "{} must keep Palmistry explicit DEFERRED-with-reason until the watcher lands",
            row.behavior_id
        );
        assert!(
            row.deferred_reason.is_some(),
            "{} Palmistry posture requires a deferred reason",
            row.behavior_id
        );
        assert!(
            row.follow_up_ref
                .is_some_and(|value| value.starts_with("palmistry://wp1/model-lane/")),
            "{} Palmistry posture requires a model-lane follow-up ref",
            row.behavior_id
        );
    }
    verify_model_lane_behavior_coverage(&matrix, &schema_registry, &pages, &tools).unwrap_or_else(
        |errors| {
            panic!(
                "model-lane behavior coverage gaps:\n{}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        },
    );

    let mixed = matrix
        .iter()
        .find(|row| row.behavior_id == "wp1.model_lane.mixed_validation")
        .expect("mixed validation row");
    assert_eq!(mixed.schema_id, Some("hsk.model_lane_mt_runtime_status@1"));
    assert_eq!(mixed.event_family, "model_lane_mt_runtime_status");
    assert_eq!(
        mixed.runtime_surface_id,
        "mixed_model_lane_integration_pg_tests"
    );
    assert_eq!(mixed.tool_id, "mixed_model_lane_integration_pg_tests");
    assert_eq!(mixed.user_manual_slug, "model-lane-validation-harness");
    assert!(
        mixed
            .eventledger_flight_recorder_path
            .contains("kernel_event_ledger"),
        "mixed validation row must stay EventLedger/FlightRecorder-backed"
    );
    assert!(
        mixed.follow_up_ref.is_some(),
        "Palmistry deferred posture must carry a follow-up ref"
    );
}
