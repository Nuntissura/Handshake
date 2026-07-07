//! WP-1 MT-009: UserManual behavior coverage must be backed by Rust
//! coverage matrix/contract entries, real UserManual rows, and the
//! ModelLane schema registry.

mod knowledge_pg_support;

use handshake_core::swarm_orchestration::model_lane::ModelLaneStore;
use handshake_core::user_manual::seed::ensure_seeded;
use handshake_core::user_manual::store::UserManualStore;
use handshake_core::user_manual::{
    cloud_model_access_behavior_coverage_matrix,
    dedicated_embedding_model_behavior_coverage_matrix, embedded_model_behavior_coverage_matrix,
    model_lane_behavior_coverage_matrix, operator_chat_launch_behavior_coverage_matrix,
    verify_cloud_model_access_behavior_coverage, verify_embedded_model_behavior_coverage,
    verify_model_lane_behavior_coverage, BehaviorCoverageError, DiagnosticTierPosture,
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

    let matrix = model_lane_behavior_coverage_matrix();
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
        assert!(
            row.self_consistency_result().starts_with("verified:"),
            "{} must expose an explicit self-consistency result",
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
    let baseline = model_lane_behavior_coverage_matrix();

    let mut missing_manual = baseline.clone();
    missing_manual[0].user_manual_slug = "missing-model-lane-manual-page";
    let errors =
        verify_model_lane_behavior_coverage(&missing_manual, &schema_registry, &pages, &tools)
            .expect_err("missing manual page must fail MT-011 coverage proof");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "UserManual page");

    let mut missing_diagnostic = baseline.clone();
    missing_diagnostic[0].internal_diagnostics_posture = DiagnosticTierPosture::DeferredWithReason;
    let errors =
        verify_model_lane_behavior_coverage(&missing_diagnostic, &schema_registry, &pages, &tools)
            .expect_err(
                "missing wired internal_diagnostics posture must fail MT-011 coverage proof",
            );
    assert_coverage_error_contains(
        &errors,
        "wp1.model_lane.run",
        "internal_diagnostics posture",
    );

    let mut missing_runtime = baseline.clone();
    missing_runtime[0].runtime_surface_id = "";
    let errors =
        verify_model_lane_behavior_coverage(&missing_runtime, &schema_registry, &pages, &tools)
            .expect_err("missing runtime surface id must fail MT-011 coverage proof");
    assert_coverage_error_contains(&errors, "wp1.model_lane.run", "runtime_surface_id missing");

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

/// WP-1 MT-013 (AC#5): the embedded-model lifecycle ledger + fail-closed/
/// embedding Flight Recorder behaviors have first-class UserManual coverage rows
/// backed by real seeded pages/tools, with the MT-013 HBR-INT-009 posture
/// (Flight Recorder WIRED; internal_diagnostics + Palmistry DEFERRED-with-reason).
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
    assert_eq!(
        behavior_ids,
        BTreeSet::from([
            "wp1.embedded_model.ledger_start",
            "wp1.embedded_model.ledger_stop",
            "wp1.llm.fail_closed_fr",
            "wp1.llm.embedding_fr",
        ]),
        "MT-013 embedded-model behavior coverage matrix must stay exact"
    );
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );

    for row in &matrix {
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::DeferredWithReason,
            "{} internal_diagnostics must be DEFERRED-with-reason for MT-013 (NOT Wired like MT-011)",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::DeferredWithReason,
            "{} Palmistry must be DEFERRED-with-reason",
            row.behavior_id
        );
        assert!(
            row.deferred_reason.is_some(),
            "{} DEFERRED tiers require a deferred_reason",
            row.behavior_id
        );
        assert!(
            row.follow_up_ref
                .is_some_and(|value| value.starts_with("palmistry://wp1/embedded-model/")),
            "{} DEFERRED tiers require an embedded-model Palmistry follow-up ref",
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
/// coverage (page + tool seeded) and the MT-013-style HBR-INT-009 posture
/// (Flight Recorder/EventLedger WIRED; internal_diagnostics + Palmistry
/// DEFERRED-with-reason).
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
    assert_eq!(
        behavior_ids,
        BTreeSet::from([
            "wp1.operator_chat.launch",
            "wp1.operator_chat.capture_message",
            "wp1.operator_chat.agent_activity_fr",
            "wp1.operator_chat.selection_audit",
        ]),
        "MT-012 operator-chat behavior coverage matrix must stay exact"
    );
    assert_eq!(
        behavior_ids.len(),
        matrix.len(),
        "behavior coverage matrix must not contain duplicate behavior_id rows"
    );

    for row in &matrix {
        assert_eq!(
            row.internal_diagnostics_posture,
            DiagnosticTierPosture::DeferredWithReason,
            "{} internal_diagnostics must be DEFERRED-with-reason for MT-012",
            row.behavior_id
        );
        assert_eq!(
            row.palmistry_posture,
            DiagnosticTierPosture::DeferredWithReason,
            "{} Palmistry must be DEFERRED-with-reason",
            row.behavior_id
        );
        assert!(
            row.follow_up_ref
                .is_some_and(|value| value.starts_with("palmistry://wp1/operator-chat/")),
            "{} DEFERRED tiers require an operator-chat Palmistry follow-up ref",
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
    assert_eq!(
        behavior_ids,
        BTreeSet::from([
            "wp1.cloud_access.providers_enumeration",
            "wp1.cloud_access.byok_store",
            "wp1.cloud_access.byok_delete",
            "wp1.cloud_access.secret_leak_guard",
            "wp1.cloud_access.settings_argus",
            "wp1.cloud_access.cli_bridge_login",
        ]),
        "MT-015 cloud-model access behavior coverage matrix must stay exact"
    );
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
        behavior_ids,
        BTreeSet::from(["wp1.llm.dedicated_embedding_model"]),
        "MT-016 dedicated embedding model behavior coverage matrix must stay exact"
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
        DiagnosticTierPosture::DeferredWithReason
    );
    assert_eq!(
        row.palmistry_posture,
        DiagnosticTierPosture::DeferredWithReason
    );
    assert!(
        row.follow_up_ref
            .is_some_and(|value| value.starts_with("palmistry://wp1/dedicated-embedding-model/")),
        "deferred Palmistry posture requires a dedicated follow-up ref"
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
}
