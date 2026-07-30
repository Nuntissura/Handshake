//! WP-1 MT-009: UserManual behavior coverage must be backed by Rust
//! coverage matrix/contract entries, real UserManual rows, and the
//! ModelLane schema registry.

mod knowledge_pg_support;

use handshake_core::process_ledger::PIDLESS_RECLAIM_INSTANCE_CAP;
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState, ModelLaneLocusBinding,
    ModelLaneRecoveryState, ModelLaneStore, NewModelLaneDiagnosticTierStatus, NewModelLaneRun,
};
use handshake_core::user_manual::registry::{wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::{ensure_seeded, seed_corpus};
use handshake_core::user_manual::store::{
    NewManualSection, NewUserManualPage, UserManualFeatureEntry, UserManualStore,
};
use handshake_core::user_manual::{
    cloud_model_access_behavior_coverage_matrix,
    dedicated_embedding_model_behavior_coverage_matrix, embedded_model_behavior_coverage_matrix,
    model_lane_behavior_coverage_matrix, model_runtime_registry_behavior_coverage_matrix,
    operator_chat_launch_behavior_coverage_matrix, verify_cloud_model_access_behavior_coverage,
    verify_embedded_model_behavior_coverage, verify_manual_named_surface_existence,
    verify_model_lane_behavior_coverage, verify_model_lane_behavior_evidence,
    verify_model_runtime_registry_behavior_coverage, BehaviorCoverageError, DiagnosticTierPosture,
    ModelRuntimeProofExecutionStatus, MANUAL_NAMED_SURFACE_BEHAVIOR_ID,
    MODEL_RUNTIME_REGISTRY_DECLARED_PROOF_SCOPE, MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID,
    USER_MANUAL_VERSION,
};
use serde_json::json;
use handshake_core::{
    api::model_runtime_registry::{
        MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE, MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID,
        MODEL_RUNTIME_REGISTRY_ROUTE, MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE,
        MODEL_RUNTIME_SELECTION_INVALID_CODE, MODEL_RUNTIME_SELECTION_REJECTED_CODE,
        MODEL_RUNTIME_SELECTION_ROUTE,
    },
    kernel::KernelEventType,
    model_runtime::MODEL_RUNTIME_REGISTRY_SCHEMA_ID,
};
use sqlx::postgres::PgPoolOptions;
use std::collections::BTreeSet;

#[tokio::test]
async fn behavior_coverage_matrix_generated_from_model_lane_registries() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for behavior_coverage_matrix_generated_from_model_lane_registries: \
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

    let manual_text = pages
        .iter()
        .map(|page| page.body.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        manual_text.contains(&format!(
            "Each boot examines at most {PIDLESS_RECLAIM_INSTANCE_CAP} eligible runtime-instance groups"
        )),
        "UserManual reclaim cap must match the runtime constant"
    );
    assert!(
        manual_text.contains("kill_succeeded_pending_stop")
            && manual_text.contains("reclaim_kill_in_progress")
            && manual_text.contains("UUIDv7-plus-generation fenced claim")
            && manual_text.contains("PostgreSQL store acknowledgement")
            && manual_text.contains("kill_operation_uuid")
            && manual_text.contains("bounded session recovery sweep")
            && manual_text.contains("typed per-operation outcomes")
            && manual_text.contains("never panic or poison later rows")
            && manual_text.contains("never fabricate STOP from unknown evidence"),
        "UserManual must explain lossless fenced reclaim and pending-STOP recovery"
    );

    let matrix = model_lane_behavior_coverage_matrix(&schema_registry)
        .expect("generate ModelLane behavior coverage from PostgreSQL schema registry");
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
        "MT-011 coverage matrix must be generated against every ModelLane schema registry row"
    );
    for row in &matrix {
        assert!(
            !row.runtime_surface_id.trim().is_empty(),
            "{} must name the implemented command/API/IPC/runtime surface",
            row.behavior_id
        );
        let proof = row
            .self_consistency_result(&schema_registry, &pages, &tools)
            .unwrap_or_else(|errors| panic!("{} consistency errors: {errors:?}", row.behavior_id));
        assert!(proof
            .checked_authorities
            .contains("compiled_internal_symbol"));
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
}

#[tokio::test]
async fn behavior_coverage_fails_on_missing_manual_diagnostic_or_runtime_route() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for behavior_coverage_fails_on_missing_manual_diagnostic_or_runtime_route: \
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
    let baseline = model_lane_behavior_coverage_matrix(&schema_registry)
        .expect("generate ModelLane behavior coverage from PostgreSQL schema registry");

    let mut missing_manual = baseline.clone();
    missing_manual[0].user_manual_slug = "missing-model-lane-manual-page";
    let errors =
        verify_model_lane_behavior_coverage(&missing_manual, &schema_registry, &pages, &tools)
            .expect_err("missing manual page must fail MT-011 coverage proof");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "UserManual page");

    let mut missing_diagnostic = baseline.clone();
    // Inject a REAL fault: a model_lane behavior whose internal_diagnostics tier
    // is downgraded away from the run-level HBR-INT-009 declaration must fail the
    // MT-011 coverage proof. The run-level posture is RUN_LEVEL_WIRED; any other
    // value (here DeferredWithReason) is rejected by the structural declaration
    // check. This is the static declaration guard; liveness is proven separately
    // by the run-level evidence gate (verify_model_lane_behavior_evidence).
    missing_diagnostic[0].internal_diagnostics_posture = DiagnosticTierPosture::DeferredWithReason;
    let errors =
        verify_model_lane_behavior_coverage(&missing_diagnostic, &schema_registry, &pages, &tools)
            .expect_err(
                "non-RUN_LEVEL_WIRED internal_diagnostics posture must fail MT-011 coverage proof",
            );
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "must be RUN_LEVEL_WIRED");

    let mut missing_runtime = baseline.clone();
    missing_runtime[0].runtime_surface_id = "";
    let errors =
        verify_model_lane_behavior_coverage(&missing_runtime, &schema_registry, &pages, &tools)
            .expect_err("missing runtime surface id must fail MT-011 coverage proof");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "runtime_surface_id missing");

    let mut renamed_symbol = baseline.clone();
    renamed_symbol[0].runtime_surface_id = "ModelLaneStore::deleted_record_run";
    let errors =
        verify_model_lane_behavior_coverage(&renamed_symbol, &schema_registry, &pages, &tools)
            .expect_err("a nonempty but nonexistent Rust symbol must fail");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "compile-anchored symbol");

    let mut missing_tool = baseline;
    missing_tool[0].tool_id = "missing_runtime_route_tool";
    let errors =
        verify_model_lane_behavior_coverage(&missing_tool, &schema_registry, &pages, &tools)
            .expect_err("missing runtime/tool proof target must fail MT-011 coverage proof");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "UserManual tool");
}

fn assert_coverage_error_contains(
    errors: &[BehaviorCoverageError],
    behavior_id: &str,
    needle: &str,
) {
    assert!(
        errors
            .iter()
            .any(|error| { error.behavior_id == behavior_id && error.reason.contains(needle) }),
        "expected {behavior_id} coverage error containing `{needle}`; got {:?}",
        errors
    );
}

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

    let matrix = model_lane_behavior_coverage_matrix(&schema_registry)
        .expect("generate ModelLane behavior coverage from PostgreSQL schema registry");
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        behavior_ids,
        BTreeSet::from([
            "wp1.model_lane.run",
            "wp1.model_lane.launch",
            "wp1.model_lane.official_cli_spawn",
            "wp1.model_lane.official_cli_attached_sandbox",
            "wp1.model_lane.message",
            "wp1.model_lane.terminal",
            "wp1.model_lane.promotion",
            "wp1.model_lane.context_bundle_artifact",
            "wp1.model_lane.context_bundle",
            "wp1.model_lane.cloud_projection_plan",
            "wp1.model_lane.cloud_projection_plan_v2",
            "wp1.model_lane.cloud_consent",
            "wp1.model_lane.cloud_consent_v2",
            "wp1.model_lane.cloud_consent_denial",
            "wp1.model_lane.recovery",
            "wp1.model_lane.recovery_event",
            "wp1.model_lane.recovery_event_v2",
            "wp1.model_lane.lease",
            "wp1.model_lane.diagnostics",
            "wp1.model_lane.mixed_validation",
            "wp1.model_lane.routing_execution",
            "wp1.model_lane.routing_outbox",
            "wp1.model_lane.routing_stage_attempt",
            "wp1.model_lane.run_extension",
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
            DiagnosticTierPosture::RunLevelWired,
            "{} internal_diagnostics must be RUN_LEVEL_WIRED (covered by the single run-level HBR-INT-009 envelope, not a per-behavior WIRED literal)",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::RunLevelWired,
            "{} Palmistry must be RUN_LEVEL_WIRED (covered by the single run-level HBR-INT-009 envelope, not a per-behavior WIRED literal)",
            row.behavior_id
        );
        assert!(
            row.deferred_reason
                .is_some_and(|reason| reason.contains("wired observers")),
            "{} RUN_LEVEL_WIRED diagnostics posture requires explicit observer/authority separation",
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
    assert_eq!(mixed.runtime_surface_id, "ModelLaneStore");
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
        "Palmistry wired posture must carry a stable diagnostic correlation ref"
    );
}

/// WP-1 MT-013 (AC#5): the embedded-model lifecycle ledger + fail-closed/
/// embedding Flight Recorder behaviors have first-class UserManual coverage rows
/// backed by real seeded pages/tools, with all HBR-INT-009 tiers WIRED.
#[tokio::test]
async fn embedded_model_behaviors_have_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for embedded_model_behaviors_have_manual_coverage: \
             UserManual behavior coverage proof requires live PostgreSQL/EventLedger"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual behavior coverage corpus");

    let manual_store = UserManualStore::new(&pg.db);
    let pages = manual_store
        .list_pages(None, None, 1_000)
        .await
        .expect("read UserManual pages");
    let tools = manual_store
        .list_tool_entries(None, None, 1_000)
        .await
        .expect("read UserManual tools");

    let matrix = embedded_model_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    assert!(
        !behavior_ids.is_empty(),
        "canonical embedded-model registry is nonempty"
    );
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );

    for row in &matrix {
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::Wired,
            "{} internal_diagnostics must be WIRED through the native producer",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::Wired,
            "{} Palmistry must be WIRED through the authenticated watcher",
            row.behavior_id
        );
        assert!(
            row.deferred_reason
                .is_some_and(|reason| reason.contains("wired")),
            "{} WIRED tiers require explicit observer/authority separation",
            row.behavior_id
        );
        assert!(
            row.follow_up_ref
                .is_some_and(|value| value.starts_with("palmistry://wp1/embedded-model/")),
            "{} WIRED tiers require an embedded-model Palmistry correlation ref",
            row.behavior_id
        );
        assert!(
            row.schema_id.is_none(),
            "{} is not a ModelLane schema-registry row",
            row.behavior_id
        );
    }

    let tool_ids = matrix
        .iter()
        .map(|row| row.tool_id)
        .collect::<BTreeSet<_>>();
    assert!(
        tool_ids.contains("embedded_model_ledger_tests"),
        "ledger behaviors must point at the embedded_model_ledger_tests proof suite"
    );
    assert!(
        tool_ids.contains("llm_client_local_routing_tests"),
        "FR behaviors must point at the llm_client_local_routing_tests proof suite"
    );
    verify_embedded_model_behavior_coverage(&matrix, &pages, &tools).unwrap_or_else(|errors| {
        panic!(
            "embedded-model behavior coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
}

/// WP-1 MT-012: the operator chat/launch surface behaviors have UserManual
/// coverage (page + tool seeded) and all HBR-INT-009 tiers WIRED.
#[tokio::test]
async fn operator_chat_launch_behaviors_have_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for operator_chat_launch_behaviors_have_manual_coverage: \
             UserManual behavior coverage proof requires live PostgreSQL/EventLedger"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual behavior coverage corpus");

    let manual_store = UserManualStore::new(&pg.db);
    let pages = manual_store
        .list_pages(None, None, 1_000)
        .await
        .expect("read UserManual pages");
    let tools = manual_store
        .list_tool_entries(None, None, 1_000)
        .await
        .expect("read UserManual tools");

    let matrix = operator_chat_launch_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    for surface in wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::OperatorChat)
    {
        assert!(
            behavior_ids.contains(surface.surface_id),
            "shipped Operator Chat route {} {} escaped behavior coverage",
            surface.method,
            surface.route
        );
    }
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );

    for row in &matrix {
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::Wired,
            "{} internal_diagnostics must be WIRED through the native producer",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::Wired,
            "{} Palmistry must be WIRED through the authenticated watcher",
            row.behavior_id
        );
        assert!(
            row.follow_up_ref
                .is_some_and(|value| value.starts_with("palmistry://wp1/operator-chat/")),
            "{} WIRED tiers require an operator-chat Palmistry correlation ref",
            row.behavior_id
        );
        assert_eq!(
            row.user_manual_slug, "operator-chat-launch",
            "operator-chat behaviors point at the operator-chat-launch manual page"
        );
        assert_eq!(
            row.tool_id, "operator_chat_capture_tests",
            "operator-chat behaviors point at the operator_chat_capture_tests proof suite"
        );
    }

    verify_embedded_model_behavior_coverage(&matrix, &pages, &tools).unwrap_or_else(|errors| {
        panic!(
            "operator-chat behavior coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    let mut stale_route = matrix.clone();
    let route_row = stale_route
        .iter_mut()
        .find(|row| row.behavior_id == "operator_chat.models.list")
        .expect("canonical Operator Chat models route row");
    route_row.runtime_surface_id = "/operator-chat/deleted-models-route";
    let errors = verify_embedded_model_behavior_coverage(&stale_route, &pages, &tools)
        .expect_err("a stale nonempty route must fail computed consistency");
    assert_coverage_error_contains(
        &errors,
        "operator_chat.models.list",
        "does not equal canonical",
    );

    let operator_chat_page = pages
        .iter()
        .find(|page| page.slug == "operator-chat-launch")
        .expect("operator-chat-launch manual page exists");
    let operator_chat_body = operator_chat_page.body.to_string();
    assert!(
        operator_chat_body.contains("operator-chat.model.<lane>.<provider>.<model>"),
        "manual must document the actual Operator Chat model-row author_id prefix"
    );
    assert!(
        operator_chat_body.contains("SUBAGENT")
            && operator_chat_body.contains("launch_operator_subagent_model_lane")
            && operator_chat_body.contains("SubagentManager"),
        "manual must document the Operator Chat subagent lane, no-OS launch helper, and authority"
    );
    assert!(
        !operator_chat_body.contains("operator-chat.picker.model.<lane>.<provider>.<model>"),
        "manual must not point no-context models at the obsolete picker author_id prefix"
    );
    assert!(
        operator_chat_body.contains(
            "internal_diagnostics is WIRED through the native producer and Problems projection"
        ) && operator_chat_body.contains(
            "Palmistry is WIRED through the authenticated watcher and survivor recovery importer",
        ) && !operator_chat_body.contains("DEFERRED-with-reason"),
        "Operator Chat manual must keep the implemented diagnostics tiers WIRED"
    );
}

#[tokio::test]
async fn cloud_model_access_behaviors_have_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for cloud_model_access_behaviors_have_manual_coverage: \
             UserManual behavior coverage proof requires live PostgreSQL/EventLedger"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual behavior coverage corpus");

    let manual_store = UserManualStore::new(&pg.db);
    let pages = manual_store
        .list_pages(None, None, 1_000)
        .await
        .expect("read UserManual pages");
    let tools = manual_store
        .list_tool_entries(None, None, 1_000)
        .await
        .expect("read UserManual tools");

    let matrix = cloud_model_access_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    for surface in wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::ModelAccess)
    {
        assert!(
            behavior_ids.contains(surface.surface_id),
            "shipped Model Access route {} {} escaped behavior coverage",
            surface.method,
            surface.route
        );
    }
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );

    for row in &matrix {
        assert_eq!(
            row.user_manual_slug, "cloud-model-access",
            "MT-015 cloud-access rows point at the cloud-model-access manual page"
        );
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::NotApplicableWithReason,
            "{} is a settings/keychain surface, not a ModelLane internal_diagnostics tier",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::NotApplicableWithReason,
            "{} is a settings/keychain surface, not a Palmistry-observed runtime lane",
            row.behavior_id
        );
        assert!(
            row.deferred_reason.is_some(),
            "{} NOT_APPLICABLE-with-reason rows require an explicit reason",
            row.behavior_id
        );
    }

    verify_cloud_model_access_behavior_coverage(&matrix, &pages, &tools).unwrap_or_else(|errors| {
        panic!(
            "cloud-model access behavior coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    let page = pages
        .iter()
        .find(|page| page.slug == "cloud-model-access")
        .expect("cloud-model-access manual page exists");
    let body = page.body.to_string();
    for required in [
        "model_access_route_tests",
        "put_store_returns_200_and_never_echoes_the_key",
        "delete_byok_key_is_idempotent_and_updates_status",
        "get_providers_reflects_configured_and_excludes_gemini",
        "keychain_unavailable_is_503",
        "cloud_byok_access_config_leak_tests",
        "byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain",
        "test_cloud_models_settings_argus",
        "cloud_models_controls_are_addressable_and_gemini_is_never_offered",
        "cloud_models_key_entry_renders_when_backend_unreachable",
        "typed_byok_key_is_wiped_from_egui_memory_after_close",
        "cli_bridge_login_records_the_official_command_without_stealing_focus",
    ] {
        assert!(
            body.contains(required),
            "cloud-model-access manual page must cite current proof target `{required}`"
        );
    }
    assert!(
        body.contains("DELETE /model-access/byok/{provider}/key"),
        "manual must document the DELETE/rotate route proved by model_access_route_tests"
    );
    assert!(
        !body.contains("cloud_access_config_tests"),
        "manual must not cite the obsolete/nonexistent cloud_access_config_tests proof target"
    );
}

#[tokio::test]
async fn dedicated_embedding_model_behaviors_have_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for dedicated_embedding_model_behaviors_have_manual_coverage: \
             UserManual behavior coverage proof requires live PostgreSQL/EventLedger"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual behavior coverage corpus");

    let manual_store = UserManualStore::new(&pg.db);
    let pages = manual_store
        .list_pages(None, None, 1_000)
        .await
        .expect("read UserManual pages");
    let tools = manual_store
        .list_tool_entries(None, None, 1_000)
        .await
        .expect("read UserManual tools");

    let matrix = dedicated_embedding_model_behavior_coverage_matrix();
    let behavior_ids = matrix
        .iter()
        .map(|row| row.behavior_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "canonical registry ids are unique"
    );

    let row = matrix
        .first()
        .expect("dedicated embedding behavior row exists");
    assert_eq!(row.user_manual_slug, "dedicated-embedding-model-routing");
    assert_eq!(row.tool_id, "dedicated_embedding_model_tests");
    assert_eq!(
        row.event_family, "data_embedding_computed",
        "coverage row must cite the emitted embedding success event family"
    );
    assert!(
        row.eventledger_flight_recorder_path
            .contains("loom_block_search_index.embedding_model"),
        "coverage row must cite the stored embedding_model provenance path"
    );
    assert_eq!(
        row.internal_diagnostics_posture,
        DiagnosticTierPosture::Wired
    );
    assert_eq!(row.palmistry_posture, DiagnosticTierPosture::Wired);
    assert!(
        row.deferred_reason.is_some_and(|reason| {
            reason.contains("wired observers") && !reason.contains("follow-up worktrees")
        }),
        "wired embedding diagnostics must not regress to stale follow-up-worktree wording"
    );
    assert!(
        row.follow_up_ref
            .is_some_and(|value| value.starts_with("palmistry://wp1/dedicated-embedding-model/")),
        "wired Palmistry posture requires a dedicated correlation ref"
    );

    verify_embedded_model_behavior_coverage(&matrix, &pages, &tools).unwrap_or_else(|errors| {
        panic!(
            "dedicated embedding model behavior coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    let page = pages
        .iter()
        .find(|page| page.slug == "dedicated-embedding-model-routing")
        .expect("dedicated embedding model manual page exists");
    let page_body = page.body.to_string();
    assert!(
        page_body.contains("HANDSHAKE_LOCAL_EMBEDDING_MODEL_PATH"),
        "manual page must document the dedicated embedding model config"
    );
    assert!(
        page_body.contains("query_embedding_model"),
        "manual page must document query embedding model provenance"
    );
    assert!(
        page_body.contains("embedspace:<artifact_sha256>:dim:<dimension>"),
        "manual page must document the stable embedding-space key, not only the per-boot UUID"
    );
    assert!(
        page_body.contains(
            "Tier-2 internal_diagnostics is WIRED through the native producer and Problems projection",
        ) && page_body.contains(
            "Tier-3 Palmistry is WIRED through the authenticated watcher and survivor recovery importer",
        ),
        "dedicated embedding manual must keep both implemented diagnostic tiers WIRED"
    );
    assert!(
        !page_body.contains("follow-up worktrees"),
        "dedicated embedding manual must not regress to stale worktree deferral wording"
    );
}

#[test]
fn model_runtime_selection_failure_recovery_rows_match_compiled_api_contract() {
    let _compiled_router = handshake_core::api::model_runtime_registry::routes;
    let matrix = model_runtime_registry_behavior_coverage_matrix();
    assert_eq!(matrix.len(), 17, "MT-014 behavior matrix row count drifted");
    let selection_rows = matrix
        .iter()
        .filter(|row| {
            row.behavior_id
                .starts_with("wp1.model_runtime.selection.post.")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selection_rows.len(),
        11,
        "MT-014 selection coverage must include success, every reachable failure class, and recovery/re-observation"
    );

    for (behavior_id, runtime_surface_id, response_code, evidence_marker, proof_tool_id) in [
        (
            "wp1.model_runtime.selection.post.failure.invalid_input",
            MODEL_RUNTIME_SELECTION_ROUTE,
            Some(MODEL_RUNTIME_SELECTION_INVALID_CODE),
            "invalid target_model_id, actor, or reason",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
        ),
        (
            "wp1.model_runtime.selection.post.failure.non_ready_target",
            MODEL_RUNTIME_SELECTION_ROUTE,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
            "non-READY",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
        ),
        (
            "wp1.model_runtime.selection.post.failure.timeout",
            MODEL_RUNTIME_SELECTION_ROUTE,
            Some(MODEL_RUNTIME_SELECTION_REJECTED_CODE),
            "timeout keeps the prior active model",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
        ),
        (
            "wp1.model_runtime.selection.post.failure.unavailable",
            MODEL_RUNTIME_SELECTION_ROUTE,
            Some(MODEL_RUNTIME_REGISTRY_UNAVAILABLE_CODE),
            "503 MODEL_RUNTIME_REGISTRY_UNAVAILABLE",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
        ),
        (
            "wp1.model_runtime.selection.post.recovery.preserve_prior",
            MODEL_RUNTIME_SELECTION_ROUTE,
            None,
            "keeps the prior active model",
            "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract",
        ),
        (
            "wp1.model_runtime.selection.post.recovery.reobserve",
            MODEL_RUNTIME_REGISTRY_ROUTE,
            None,
            "Refresh re-reads the durable projection",
            "mt014_stable_switch_author_id_posts_then_reobserves_backend_projection",
        ),
    ] {
        let row = selection_rows
            .iter()
            .find(|row| row.behavior_id == behavior_id)
            .unwrap_or_else(|| panic!("missing exact MT-014 behavior row {behavior_id}"));
        assert_eq!(row.runtime_surface_id, runtime_surface_id, "{behavior_id}");
        assert_eq!(row.response_code, response_code, "{behavior_id}");
        assert_eq!(row.manual_evidence_marker, evidence_marker, "{behavior_id}");
        assert_eq!(row.proof_tool_id, proof_tool_id, "{behavior_id}");
        assert!(
            row.recovery_instruction_marker
                .starts_with("Restore the current migration chain/database authority"),
            "{behavior_id} must carry an explicit recovery instruction"
        );
    }

    let reobserve = selection_rows
        .iter()
        .find(|row| row.behavior_id == "wp1.model_runtime.selection.post.recovery.reobserve")
        .expect("re-observation row exists");
    assert_eq!(
        reobserve.schema_id,
        Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID),
        "GET re-observation returns the canonical typed registry projection"
    );
    let integrity = selection_rows
        .iter()
        .find(|row| row.behavior_id == "wp1.model_runtime.selection.post.failure.integrity")
        .expect("integrity row exists");
    assert_eq!(
        integrity.response_code,
        Some(MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR_CODE)
    );
}

#[tokio::test]
async fn model_runtime_registry_behaviors_have_canonical_manual_coverage() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for model_runtime_registry_behaviors_have_canonical_manual_coverage: \
             MT-014 UserManual coverage proof requires the seeded PostgreSQL authority"
        );
    };
    ensure_seeded(&pg.db)
        .await
        .expect("seed UserManual MT-014 coverage corpus");

    let manual_store = UserManualStore::new(&pg.db);
    let features = manual_store
        .list_feature_entries(500)
        .await
        .expect("read UserManual feature entries");
    let matrix = model_runtime_registry_behavior_coverage_matrix();

    assert!(
        serde_json::to_value(&matrix).is_ok(),
        "MT-014 behavior coverage matrix must remain machine-readable"
    );
    assert_eq!(
        matrix
            .iter()
            .map(|row| row.behavior_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "wp1.model_runtime_registry.persistent_adapter_selection",
            "wp1.model_runtime_registry.restart_recovery",
            "wp1.model_runtime_registry.fail_closed_selection_conflict",
            "wp1.model_runtime_registry.api_projection",
            "wp1.model_runtime_registry.native_panel",
            "wp1.model_runtime_registry.eventledger_selection_evidence",
            "wp1.model_runtime.selection.post.success",
            "wp1.model_runtime.selection.post.failure.audit",
            "wp1.model_runtime.selection.post.failure.stale_target",
            "wp1.model_runtime.selection.post.failure.embedding_role",
            "wp1.model_runtime.selection.post.failure.integrity",
            "wp1.model_runtime.selection.post.failure.invalid_input",
            "wp1.model_runtime.selection.post.failure.non_ready_target",
            "wp1.model_runtime.selection.post.failure.timeout",
            "wp1.model_runtime.selection.post.failure.unavailable",
            "wp1.model_runtime.selection.post.recovery.preserve_prior",
            "wp1.model_runtime.selection.post.recovery.reobserve",
        ]),
        "MT-014 ModelRuntime registry coverage matrix must stay exact"
    );
    verify_model_runtime_registry_behavior_coverage(&matrix, &features).unwrap_or_else(|errors| {
        panic!(
            "ModelRuntime registry UserManual coverage gaps:\n{}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    let feature = features
        .iter()
        .find(|feature| feature.feature_id == MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID)
        .expect("current MT-014 UserManual feature exists");
    assert_eq!(feature.manual_version, USER_MANUAL_VERSION);
    assert!(feature
        .description
        .contains(MODEL_RUNTIME_REGISTRY_DECLARED_PROOF_SCOPE));
    assert!(matrix.iter().all(|row| {
        row.proof_execution_status == ModelRuntimeProofExecutionStatus::DeclaredNotExecuted
    }));
    let persistent = matrix
        .iter()
        .find(|row| row.behavior_id == "wp1.model_runtime_registry.persistent_adapter_selection")
        .expect("persistent registry coverage row");
    assert_eq!(persistent.schema_id, Some(MODEL_RUNTIME_REGISTRY_SCHEMA_ID));
    assert_eq!(
        persistent.eventledger_event_type,
        Some(KernelEventType::ModelRuntimeSelectionRecorded.as_str())
    );
    let api = matrix
        .iter()
        .find(|row| row.behavior_id == "wp1.model_runtime_registry.api_projection")
        .expect("registry API coverage row");
    assert_eq!(api.runtime_surface_id, MODEL_RUNTIME_REGISTRY_ROUTE);
    assert_eq!(
        api.schema_id,
        Some(MODEL_RUNTIME_REGISTRY_PROJECTION_SCHEMA_ID)
    );
    let native = matrix
        .iter()
        .find(|row| row.behavior_id == "wp1.model_runtime_registry.native_panel")
        .expect("native ModelRuntime panel coverage row");
    assert!(native.runtime_surface_id.contains("PaneType::ModelRuntime"));
    assert!(native
        .runtime_surface_id
        .contains("model-runtime.registry.*"));
}

#[tokio::test]
async fn model_runtime_registry_stale_deployed_row_fails_read_only_freshness_check() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for stale MT-014 UserManual freshness proof: live PostgreSQL is required"
        );
    };
    let manual_store = UserManualStore::new(&pg.db);
    let matrix = model_runtime_registry_behavior_coverage_matrix();
    let mut markers = BTreeSet::new();
    for row in &matrix {
        markers.insert(row.manual_evidence_marker);
        markers.insert(row.recovery_instruction_marker);
    }
    markers.extend([
        MODEL_RUNTIME_REGISTRY_DECLARED_PROOF_SCOPE,
        "Tier-1 Flight Recorder events are WIRED",
        "internal_diagnostics is WIRED through the native producer and Problems projection",
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer",
    ]);
    let stale_feature = UserManualFeatureEntry {
        feature_id: MODEL_RUNTIME_REGISTRY_MANUAL_FEATURE_ID.to_owned(),
        title: "stale deployed MT-014 feature".to_owned(),
        description: markers.into_iter().collect::<Vec<_>>().join(" | "),
        tool_ids: matrix
            .iter()
            .map(|row| row.proof_tool_id.to_owned())
            .collect(),
        origin: "wp1_mt014".to_owned(),
        content_hash: "0".repeat(64),
        manual_version: "2.0.9-stale".to_owned(),
    };
    manual_store
        .upsert_feature_entry(&stale_feature)
        .await
        .expect("install stale deployed row without invoking ensure_seeded");

    let deployed_rows = manual_store
        .list_feature_entries(500)
        .await
        .expect("read deployed UserManual rows without reseeding");
    let errors = verify_model_runtime_registry_behavior_coverage(&matrix, &deployed_rows)
        .expect_err("stale deployed row must fail the read-only freshness check");
    assert!(errors.iter().any(|error| {
        error.behavior_id == "wp1.model_runtime_registry.manual_version"
            && error.reason.contains(USER_MANUAL_VERSION)
    }));
}

const EVIDENCE_WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const EVIDENCE_OWNER: &str = "KERNEL_BUILDER-mt011-evidence";

/// A minimal-but-valid `ModelLaneRun` whose locus fields are consistent with the
/// run identity, so `ModelLaneStore::record_run` accepts it and later diagnostic
/// tier records can attach to it.
fn evidence_run(run_id: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        routing_policy: "mixed_local_cloud_subagent".into(),
        context_bundle_id: format!("ctx-{run_id}"),
        lane_ids: vec![format!("lane-{run_id}")],
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        artifact_namespace: format!("artifact://model-lane/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(EVIDENCE_WP_ID.into()),
        micro_task_id: Some("MT-011".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: EVIDENCE_OWNER.into(),
        idempotency_key: format!("idem-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics".into()),
        locus_binding: Some(ModelLaneLocusBinding {
            work_packet_id: EVIDENCE_WP_ID.into(),
            micro_task_id: "MT-011".into(),
            task_board_id: Some("task-board://wp-1".into()),
            coordinator_session_id: format!("coordinator-{run_id}"),
            session_id: format!("session-{run_id}"),
            model_session_id: format!("model-session-{run_id}"),
            owner_session: EVIDENCE_OWNER.into(),
            locus_binding_ref: format!("locus://wp1/mt011/{run_id}"),
        }),
        memory_pack_ref: format!("memory-pack://fems/{run_id}"),
        memory_pack_hash: "a1".repeat(32),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt011".into(),
        selected_model_id: Some("model://mt011/local".into()),
        candidate_model_ids: vec!["model://mt011/local".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

/// One tier of the run-level HBR-INT-009 envelope (`behavior_id = "HBR-INT-009"`).
fn evidence_tier(
    run_id: &str,
    tier: ModelLaneDiagnosticTier,
    state: ModelLaneDiagnosticTierState,
    evidence_ref: &str,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{run_id}-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("run-level HBR-INT-009 tier {} for {run_id}", tier.as_str()),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("palmistry://wp1/model-lane/run".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: EVIDENCE_WP_ID.into(),
        micro_task_id: "MT-011".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: EVIDENCE_OWNER.into(),
        idempotency_key: format!("idem-diag-{run_id}-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id}),
    }
}

/// MT-011 run-level evidence proof (POSITIVE): when a real ModelLaneRun carries
/// the durable run-level HBR-INT-009 triplet (Flight Recorder + internal_diagnostics
/// + Palmistry, with `internal-diagnostics://session/` + `palmistry-observation://session/`
/// evidence refs), `verify_model_lane_behavior_evidence` PASSES for the full
/// RUN_LEVEL_WIRED model-lane matrix. This replaces the tautological static WIRED
/// flip with evidence validated against real durable records.
#[tokio::test]
async fn model_lane_run_level_hbr_int_009_evidence_passes_coverage_when_durable_records_exist() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for model_lane_run_level_hbr_int_009_evidence_passes_coverage_when_durable_records_exist: \
             MT-011 run-level evidence proof requires live PostgreSQL/EventLedger"
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
    let store = ModelLaneStore::new(pool);
    let schema_registry = store
        .schema_registry_rows()
        .await
        .expect("read ModelLane schema registry");
    let matrix = model_lane_behavior_coverage_matrix(&schema_registry)
        .expect("generate ModelLane behavior coverage from PostgreSQL schema registry");

    let run_id = "run-mt011-evidence-positive";
    store
        .record_run(evidence_run(run_id))
        .await
        .expect("record the run that carries the run-level HBR-INT-009 envelope");
    for (tier, evidence_ref) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            "eventledger://kernel/model-lane/run/run-mt011-evidence-positive",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            "internal-diagnostics://session/run-mt011-evidence-positive/panic-heartbeat-frame-resource-open-event",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            "palmistry-observation://session/run-mt011-evidence-positive/watcher",
        ),
    ] {
        store
            .record_diagnostic_tier_status(evidence_tier(
                run_id,
                tier,
                ModelLaneDiagnosticTierState::Wired,
                evidence_ref,
            ))
            .await
            .expect("record run-level HBR-INT-009 tier");
    }

    let postures = verify_model_lane_behavior_evidence(&store, run_id, &matrix)
        .await
        .unwrap_or_else(|errors| {
            panic!(
                "run-level HBR-INT-009 evidence gate must PASS when durable records exist:\n{}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
    assert_eq!(
        postures.len(),
        1,
        "the 24 model-lane behaviors share ONE run-level HBR-INT-009 envelope, not per-behavior evidence"
    );
    assert_eq!(postures[0].run_id, run_id);
    assert_eq!(postures[0].behavior_id, "HBR-INT-009");
    assert_eq!(
        postures[0].tiers.len(),
        3,
        "run-level HBR-INT-009 envelope must carry all three correlated tiers"
    );
}

/// MT-011 run-level evidence proof (NEGATIVE): the gate FAILS CLOSED when the
/// run-level HBR-INT-009 records are absent or incomplete, and it also rejects a
/// gamed per-behavior `Wired` literal before any DB lookup. This replaces the
/// tautological negative test that could never fail on hardcoded WIRED literals.
#[tokio::test]
async fn model_lane_run_level_hbr_int_009_evidence_fails_closed_when_absent_or_incomplete() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "PostgreSQL unavailable for model_lane_run_level_hbr_int_009_evidence_fails_closed_when_absent_or_incomplete: \
             MT-011 run-level evidence proof requires live PostgreSQL/EventLedger"
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
    let store = ModelLaneStore::new(pool);
    let schema_registry = store
        .schema_registry_rows()
        .await
        .expect("read ModelLane schema registry");
    let matrix = model_lane_behavior_coverage_matrix(&schema_registry)
        .expect("generate ModelLane behavior coverage from PostgreSQL schema registry");

    // (a) ABSENT: a run with no durable HBR-INT-009 records fails closed.
    let absent_run = "run-mt011-evidence-absent";
    let errors = verify_model_lane_behavior_evidence(&store, absent_run, &matrix)
        .await
        .expect_err("absent run-level HBR-INT-009 evidence must fail closed");
    assert!(
        errors.iter().any(|error| error.behavior_id == "HBR-INT-009"),
        "absent evidence must fail on the HBR-INT-009 run-level envelope, got {errors:?}"
    );

    // (b) INCOMPLETE: only the FlightRecorder tier recorded (missing
    // internal_diagnostics + Palmistry) fails closed — Flight-Recorder-only
    // evidence is not enough.
    let incomplete_run = "run-mt011-evidence-incomplete";
    store
        .record_run(evidence_run(incomplete_run))
        .await
        .expect("record the incomplete run");
    store
        .record_diagnostic_tier_status(evidence_tier(
            incomplete_run,
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/run/run-mt011-evidence-incomplete",
        ))
        .await
        .expect("record only the FlightRecorder tier");
    let errors = verify_model_lane_behavior_evidence(&store, incomplete_run, &matrix)
        .await
        .expect_err("Flight-Recorder-only run-level evidence must fail closed");
    assert!(
        errors.iter().any(|error| error.behavior_id == "HBR-INT-009"),
        "incomplete evidence must fail on the HBR-INT-009 run-level envelope, got {errors:?}"
    );

    // (c) GAMED LITERAL: a matrix row that declares a per-behavior `Wired`
    // posture (the exact anti-pattern this MT closes) is rejected by the gate
    // before any DB lookup.
    let mut gamed = matrix.clone();
    gamed[0].internal_diagnostics_posture = DiagnosticTierPosture::Wired;
    let errors = verify_model_lane_behavior_evidence(&store, absent_run, &gamed)
        .await
        .expect_err("a per-behavior WIRED literal must be rejected by the run-level evidence gate");
    assert!(
        errors
            .iter()
            .any(|error| error.reason.contains("RUN_LEVEL_WIRED")),
        "gamed per-behavior WIRED must be rejected with a RUN_LEVEL_WIRED declaration error, got {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// MT-022 (AC-1 / AC-2 / AC-5): HBR-MAN-003 for the CANONICAL UserManual corpus.
//
// These are pure compiled-corpus checks, so they run on every invocation of this
// proof binary and cannot be skipped by an unavailable resource.
// ---------------------------------------------------------------------------

fn scanned_body(corpus: &handshake_core::user_manual::seed::SeedCorpus, slug: &str) -> String {
    let page = corpus
        .pages
        .iter()
        .find(|page| page.slug == slug)
        .unwrap_or_else(|| panic!("UserManual page {slug} missing from the seeded corpus"));
    page.sections
        .iter()
        .map(|section| section.body_md.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn user_manual_names_only_surfaces_that_exist_in_compiled_product_code() {
    let corpus = seed_corpus();
    let proof = verify_manual_named_surface_existence(&corpus.pages).unwrap_or_else(|errors| {
        panic!(
            "HBR-MAN-003: the canonical UserManual corpus names {} surface(s) that do not exist: {}",
            errors.len(),
            errors
                .iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    });

    assert_eq!(
        proof.pages_scanned,
        corpus.pages.len(),
        "the gate must scan every seeded page, not a subset"
    );
    // Anti-vacuous-pass: a gate that scanned nothing is not a gate.
    assert!(
        proof.symbol_claims_checked >= 40,
        "expected the corpus to make a substantial number of product-symbol claims, got {}",
        proof.symbol_claims_checked
    );
    assert!(
        proof.flight_recorder_event_claims_checked >= 4,
        "expected the corpus to make Flight Recorder event claims, got {}",
        proof.flight_recorder_event_claims_checked
    );
    assert!(
        proof.compiled_flight_recorder_vocabulary >= 50,
        "the compiled Flight Recorder vocabulary looks truncated: {}",
        proof.compiled_flight_recorder_vocabulary
    );
}

#[test]
fn user_manual_named_surface_gate_fails_on_drift() {
    // The AC-5 requirement: a gate nobody has seen fail is not a gate. Inject a
    // symbol that does not exist and an event id that was never registered, and
    // prove BOTH are reported.
    let mut corpus = seed_corpus();
    corpus.pages.push(NewUserManualPage {
        slug: "mt022-drift-probe".into(),
        title: "MT-022 drift probe (test-only)".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![NewManualSection {
            section_kind: "purpose",
            title: "Deliberately false claims".into(),
            body_md: "Call `SwarmCoordinator::totally_not_a_real_method` and watch for \
                      FR-EVT-SWARM-SPAWN-DENIED in the Flight Recorder."
                .into(),
            body_json: None,
        }],
        anchors: vec![],
    });

    let errors = verify_manual_named_surface_existence(&corpus.pages)
        .expect_err("a manual page naming a non-existent surface MUST fail HBR-MAN-003");

    assert!(
        errors
            .iter()
            .any(|error| error.reason.contains("SwarmCoordinator::totally_not_a_real_method")),
        "the bogus product symbol must be reported, got {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| error.reason.contains("FR-EVT-SWARM-SPAWN-DENIED")),
        "the bogus Flight Recorder event id must be reported, got {errors:?}"
    );
    assert!(
        errors
            .iter()
            .all(|error| error.behavior_id == MANUAL_NAMED_SURFACE_BEHAVIOR_ID),
        "named-surface failures must carry the named-surface behavior id, got {errors:?}"
    );

    // Removing the drifted page returns the corpus to PASS: the failure is
    // caused by the injected claim, not by an unrelated corpus problem.
    corpus.pages.pop();
    verify_manual_named_surface_existence(&corpus.pages)
        .expect("the unmodified canonical corpus must pass once the drift is removed");
}

#[test]
fn external_compat_engine_import_lane_is_documented_and_toc_reachable() {
    let corpus = seed_corpus();
    let body = scanned_body(&corpus, "external-compat-engine-import");

    // AC-1: purpose, non-swarm-spawnability, the Dexterity rejection path,
    // inputs/outputs, failure modes, recovery.
    for required in [
        "ProviderKind::ExternalCompat",
        "ExternalEngineImportRecord",
        "operator_signature",
        "OPENAI_COMPAT_BASE_URL",
        "Dexterity rejects external_compat launch bypass",
        "Dexterity model-lane schema does not support external_compat provider",
        "external-compat imports are offline registrations, not swarm-spawnable",
        "SwarmError::ProviderNotConfigured",
        "ModelRuntimeError::AdapterMismatch",
        // AC-3 HBR-INT-009 posture: all three tiers stated, none silently skipped.
        "Tier-1 Flight Recorder",
        "Tier-2 internal_diagnostics",
        "Tier-3 Palmistry",
    ] {
        assert!(
            body.contains(required),
            "external-compat-engine-import must document `{required}`"
        );
    }

    let toc = corpus
        .pages
        .iter()
        .find(|page| page.slug == "manual-toc")
        .expect("manual-toc page");
    assert!(
        toc.anchors.iter().any(|anchor| {
            anchor.anchor_kind == "page_link" && anchor.anchor_value == "external-compat-engine-import"
        }),
        "the new page must be reachable from the UserManual table of contents"
    );
}

#[test]
fn swarm_budget_rejection_and_recovery_is_documented_and_toc_reachable() {
    let corpus = seed_corpus();
    let body = scanned_body(&corpus, "swarm-budget-and-spawn-rejection");

    // AC-2: the budget dimensions, every emitted rejection reason, the durable
    // event, what the operator sees, and how to recover.
    for required in [
        "max_concurrent",
        "max_concurrent_cold_starts",
        "max_lifetime_spawns",
        "max_committed_memory_bytes",
        "concurrency_cap",
        "lifetime_ceiling",
        "breaker_open",
        "budget:tokens",
        "budget:cost",
        "budget:committed_memory",
        "budget:committed_memory_unestimated",
        "FR-EVT-SWARM-SPAWN-REJECTED",
        "FR-EVT-SWARM-BREAKER-TRIPPED",
        "/wp1/diagnostics/console/stream",
        "ConsoleCategory::SpawnRejected",
        "SwarmError::ConcurrencyCapReached",
        "SwarmError::LifetimeSpawnCeilingReached",
        "BACK OFF AND RETRY",
        "TERMINAL for this run",
        // AC-3 HBR-INT-009 posture: all three tiers stated, none silently skipped.
        "Tier-1 Flight Recorder",
        "Tier-2 internal_diagnostics",
        "Tier-3 Palmistry",
    ] {
        assert!(
            body.contains(required),
            "swarm-budget-and-spawn-rejection must document `{required}`"
        );
    }

    let toc = corpus
        .pages
        .iter()
        .find(|page| page.slug == "manual-toc")
        .expect("manual-toc page");
    assert!(
        toc.anchors.iter().any(|anchor| {
            anchor.anchor_kind == "page_link"
                && anchor.anchor_value == "swarm-budget-and-spawn-rejection"
        }),
        "the new page must be reachable from the UserManual table of contents"
    );
}
