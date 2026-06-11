//! WP-KERNEL-005 MT-171..MT-180, MT-182, MT-195: real PostgreSQL round-trip
//! proofs for the typed diagnostics "validation matrix".
//!
//! Each MT is "record the required checks for a model-workflow diagnostic
//! surface" as TYPED RUNTIME RECORDS, not governance markdown. Each test
//! connects the real `AtelierStore` to a live Postgres, ensures the schema,
//! records the catalog (idempotent upsert), reloads the rows for that MT's
//! `matrix_kind`, and asserts the required surface checks are present with their
//! requirement text + status. A negative test proves a blank `requirement` is
//! rejected, so a check can never be silent.
//!
//! Run-scoped: every assertion is keyed off this run's catalog `row_id` prefixes
//! and the MT's `matrix_kind`; no global row counts are asserted, so the test is
//! safe against rows left by other runs (the table persists between runs).
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is available
//! the test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::state_probe::{
    diagnostics_validation_matrix_catalog, diagnostics_validation_matrix_kind as kind,
    state_probe_event_family, DiagnosticsValidationRow, DiagnosticsValidationStatus,
    NewDiagnosticsValidationRow,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres. The matrix table has no character FK, so no fixture entity is
/// needed.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Connect with the kernel EventLedger wired (mirrors
/// `atelier_state_probe_tests::connected_store_with_ledger`), so the
/// MT-176..MT-195 proofs can assert the `DIAGNOSTICS_VALIDATION_ROW_RECORDED`
/// kernel events alongside the persisted rows.
async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

/// Assert that recording `row_id` left canonical EventLedger evidence: an
/// `AtelierDomainEventRecorded` kernel event in the
/// `DIAGNOSTICS_VALIDATION_ROW_RECORDED` family carrying the row's
/// `matrix_kind` in its payload.
async fn assert_row_recorded_event(database: &Arc<dyn Database>, row_id: &str, matrix_kind: &str) {
    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_diagnostics_validation_matrix", row_id)
        .await
        .expect("list diagnostics validation matrix kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == state_probe_event_family::DIAGNOSTICS_VALIDATION_ROW_RECORDED
                && event.payload["atelier_payload"]["row_id"] == serde_json::json!(row_id)
                && event.payload["atelier_payload"]["matrix_kind"]
                    == serde_json::json!(matrix_kind)
        }),
        "recording matrix row {row_id} must emit canonical EventLedger evidence"
    );
}

/// Record the catalog (idempotent upsert) and return the rows for one
/// `matrix_kind`, keyed by `row_id`. Filtering by `matrix_kind` keeps the
/// assertion run-scoped: it only sees this MT's rows.
async fn record_and_reload_kind(
    store: &AtelierStore,
    matrix_kind: &str,
) -> HashMap<String, DiagnosticsValidationRow> {
    for new in diagnostics_validation_matrix_catalog() {
        store
            .record_diagnostics_validation_row(&new)
            .await
            .expect("record diagnostics validation row");
    }
    store
        .list_diagnostics_validation_matrix(Some(matrix_kind))
        .await
        .expect("list diagnostics validation matrix by kind")
        .into_iter()
        .map(|r| (r.row_id.clone(), r))
        .collect()
}

/// Assert that each expected `(row_id, surface)` is present, COVERED, carries a
/// non-empty requirement, cites an evidence_ref, and belongs to `matrix_kind`.
fn assert_required_rows(
    by_id: &HashMap<String, DiagnosticsValidationRow>,
    matrix_kind: &str,
    expected: &[(&str, &str)],
) {
    for (row_id, surface) in expected {
        let row = by_id
            .get(*row_id)
            .unwrap_or_else(|| panic!("matrix row {row_id} must be present"));
        assert_eq!(
            row.matrix_kind, matrix_kind,
            "row {row_id} must belong to matrix_kind {matrix_kind}"
        );
        assert_eq!(
            row.surface, *surface,
            "row {row_id} must name surface {surface}"
        );
        assert_eq!(
            row.status,
            DiagnosticsValidationStatus::Covered,
            "row {row_id} must be COVERED"
        );
        assert!(
            !row.requirement.trim().is_empty(),
            "row {row_id} must carry a non-empty requirement"
        );
        assert!(
            row.evidence_ref
                .as_deref()
                .map(|r| !r.trim().is_empty())
                .unwrap_or(false),
            "row {row_id} must cite a product evidence_ref"
        );
    }
}

/// MT-171: model manual + action catalog validation rows.
#[tokio::test]
async fn mt171_manual_and_action_catalog_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt171_manual_and_action_catalog_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_and_reload_kind(&store, kind::MANUAL_ACTION_CATALOG).await;

    let expected = [
        ("mt-171.model-manual.purpose", "model_manual"),
        ("mt-171.model-manual.core-workflows", "model_manual"),
        (
            "mt-171.action-catalog.actions-enumerated",
            "kernel_action_catalog",
        ),
        (
            "mt-171.action-catalog.inputs-outputs",
            "kernel_action_catalog",
        ),
    ];
    assert_required_rows(&by_id, kind::MANUAL_ACTION_CATALOG, &expected);
}

/// MT-172: session lease + heartbeat validation rows.
#[tokio::test]
async fn mt172_session_lease_and_heartbeat_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt172_session_lease_and_heartbeat_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_and_reload_kind(&store, kind::SESSION_LEASE_HEARTBEAT).await;

    let expected = [
        ("mt-172.session.lease-acquire", "session_broker"),
        ("mt-172.session.heartbeat", "session_broker"),
        ("mt-172.role-mailbox.lease-contract", "role_mailbox"),
        ("mt-172.role-mailbox.lease-persistence", "role_mailbox"),
    ];
    assert_required_rows(&by_id, kind::SESSION_LEASE_HEARTBEAT, &expected);
}

/// MT-173: command log + error class + state probe validation rows.
#[tokio::test]
async fn mt173_command_log_error_and_state_probe_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt173_command_log_error_and_state_probe_matrix_persists: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_and_reload_kind(&store, kind::COMMAND_LOG_ERROR_STATE_PROBE).await;

    let expected = [
        ("mt-173.command-log.recorded", "diagnostics"),
        ("mt-173.error-class.classified", "diagnostics"),
        ("mt-173.state-probe.pre-visual", "state_probe"),
    ];
    assert_required_rows(&by_id, kind::COMMAND_LOG_ERROR_STATE_PROBE, &expected);
}

/// MT-174: DCC + Flight Recorder projection validation rows.
#[tokio::test]
async fn mt174_dcc_and_flight_recorder_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt174_dcc_and_flight_recorder_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_and_reload_kind(&store, kind::DCC_FLIGHT_RECORDER).await;

    let expected = [
        ("mt-174.dcc.runtime-surface", "dcc"),
        ("mt-174.flight-recorder.projection", "flight_recorder"),
    ];
    assert_required_rows(&by_id, kind::DCC_FLIGHT_RECORDER, &expected);
}

/// MT-175: visual evidence validation rows (capture, diff, ADR, STEER, comparison).
#[tokio::test]
async fn mt175_visual_evidence_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt175_visual_evidence_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let by_id = record_and_reload_kind(&store, kind::VISUAL_EVIDENCE).await;

    let expected = [
        ("mt-175.visual.capture", "visual_debugging_loop"),
        ("mt-175.visual.diff", "visual_debugging_loop"),
        ("mt-175.visual.adr", "visual_debugging_loop"),
        ("mt-175.visual.steer", "visual_debugging_loop"),
        ("mt-175.visual.comparison", "visual_debugging_loop"),
    ];
    assert_required_rows(&by_id, kind::VISUAL_EVIDENCE, &expected);
}

/// Negative: an empty `requirement` is rejected with a Validation error so a
/// blank check can never be persisted.
#[tokio::test]
async fn empty_requirement_is_rejected() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP empty_requirement_is_rejected: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let blank = NewDiagnosticsValidationRow {
        row_id: "mt-171.model-manual.blank-requirement-probe".to_string(),
        matrix_kind: kind::MANUAL_ACTION_CATALOG.to_string(),
        surface: "model_manual".to_string(),
        check_id: "model_manual.blank_probe".to_string(),
        requirement: "   ".to_string(),
        status: DiagnosticsValidationStatus::Required,
        evidence_ref: None,
    };
    let err = store
        .record_diagnostics_validation_row(&blank)
        .await
        .expect_err("empty requirement must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "empty requirement must produce a Validation error, got {err:?}"
    );
}

/// MT-176: diagnostic-bundle validation rows (creation + contents) persist in
/// PostgreSQL, reload by `matrix_kind`, and leave EventLedger evidence.
#[tokio::test]
async fn mt176_diagnostic_bundle_matrix_persists_with_ledger_evidence() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt176_diagnostic_bundle_matrix_persists_with_ledger_evidence: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::DIAGNOSTIC_BUNDLE).await;

    let expected = [
        ("mt-176.bundle.export-created", "diagnostic_bundle"),
        ("mt-176.bundle.manifest-contents", "diagnostic_bundle"),
        ("mt-176.bundle.secret-redaction", "diagnostic_bundle"),
        ("mt-176.bundle.contents-validated", "diagnostic_bundle"),
    ];
    assert_required_rows(&by_id, kind::DIAGNOSTIC_BUNDLE, &expected);
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::DIAGNOSTIC_BUNDLE).await;
    }
}

/// MT-177: local-LLM config + local chat-proposal boundary validation rows
/// persist, reload, and leave EventLedger evidence.
#[tokio::test]
async fn mt177_local_llm_and_chat_proposal_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt177_local_llm_and_chat_proposal_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::LOCAL_LLM_CHAT_PROPOSAL).await;

    let expected = [
        ("mt-177.local-llm.config-registry", "local_llm"),
        ("mt-177.local-llm.local-routing", "local_llm"),
        ("mt-177.chat-proposal.propose-only", "chat_proposal"),
        (
            "mt-177.chat-proposal.cloud-escalation-consent",
            "chat_proposal",
        ),
    ];
    assert_required_rows(&by_id, kind::LOCAL_LLM_CHAT_PROPOSAL, &expected);
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::LOCAL_LLM_CHAT_PROPOSAL).await;
    }
}

/// MT-178: AI-tagging proposal/apply validation rows (no silent media mutation)
/// persist, reload, and leave EventLedger evidence.
#[tokio::test]
async fn mt178_ai_tagging_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt178_ai_tagging_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::AI_TAGGING).await;

    let expected = [
        ("mt-178.tagging.proposal-recorded", "ai_tagging"),
        ("mt-178.tagging.apply-gated", "ai_tagging"),
        ("mt-178.tagging.no-silent-media-mutation", "ai_tagging"),
        ("mt-178.tagging.apply-evidence", "ai_tagging"),
    ];
    assert_required_rows(&by_id, kind::AI_TAGGING, &expected);
    // The no-silent-mutation row is the contract heart: it must spell out that
    // tagging never touches media bytes.
    let mutation_row = &by_id["mt-178.tagging.no-silent-media-mutation"];
    assert!(
        mutation_row.requirement.contains("never mutate media bytes"),
        "MT-178 must forbid silent media mutation, got: {}",
        mutation_row.requirement
    );
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::AI_TAGGING).await;
    }
}

/// MT-179: build-diagnostics + packaging/release validation rows (no repo-local
/// output leakage) persist, reload, and leave EventLedger evidence.
#[tokio::test]
async fn mt179_build_and_package_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt179_build_and_package_matrix_persists: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::BUILD_PACKAGE).await;

    let expected = [
        ("mt-179.build.diagnostics-recorded", "build_diagnostics"),
        ("mt-179.package.manifest-evidence", "packaging"),
        ("mt-179.package.no-repo-local-leakage", "packaging"),
        ("mt-179.package.release-evidence", "packaging"),
    ];
    assert_required_rows(&by_id, kind::BUILD_PACKAGE, &expected);
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::BUILD_PACKAGE).await;
    }
}

/// MT-180: no-focus automation, synthetic-input guard, and parallel
/// coordination validation rows persist, reload, and leave EventLedger
/// evidence.
#[tokio::test]
async fn mt180_no_focus_synthetic_input_and_parallel_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt180_no_focus_synthetic_input_and_parallel_matrix_persists: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION).await;

    let expected = [
        ("mt-180.no-focus.stealth-window", "no_focus_automation"),
        ("mt-180.no-focus.focus-audit", "no_focus_automation"),
        ("mt-180.synthetic-input.injection-flagged", "synthetic_input"),
        ("mt-180.parallel.lease-coordination", "parallel_coordination"),
        ("mt-180.parallel.session-broker", "parallel_coordination"),
    ];
    assert_required_rows(&by_id, kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION, &expected);
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::SYNTHETIC_INPUT_PARALLEL_COORDINATION)
            .await;
    }
}

/// MT-182: red-team automation-authority guard rows persist as NEGATIVE checks
/// (hidden UI automation, focus steal, direct LLM execution, unbounded
/// synthetic input), reload, and leave EventLedger evidence.
#[tokio::test]
async fn mt182_red_team_automation_authority_negative_checks_persist() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt182_red_team_automation_authority_negative_checks_persist: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::RED_TEAM_AUTOMATION_AUTHORITY).await;

    let expected = [
        ("mt-182.red-team.hidden-ui-automation", "automation_authority"),
        ("mt-182.red-team.focus-steal", "automation_authority"),
        ("mt-182.red-team.direct-llm-execution", "automation_authority"),
        (
            "mt-182.red-team.unbounded-synthetic-input",
            "automation_authority",
        ),
    ];
    assert_required_rows(&by_id, kind::RED_TEAM_AUTOMATION_AUTHORITY, &expected);
    // Every red-team row must be phrased as a prohibition (negative check).
    for (row_id, _) in &expected {
        let requirement = by_id[*row_id].requirement.to_ascii_lowercase();
        assert!(
            requirement.contains("must not")
                || requirement.contains("must never")
                || requirement.contains("must be denied")
                || requirement.contains("must be rejected"),
            "red-team row {row_id} must state a negative invariant: {requirement}"
        );
        assert_row_recorded_event(&database, row_id, kind::RED_TEAM_AUTOMATION_AUTHORITY).await;
    }
}

/// MT-182 negative: a positively-phrased "guard" row in the red-team kind is
/// rejected, so an automation-authority risk can never be recorded as if it
/// were a feature claim.
#[tokio::test]
async fn mt182_positively_phrased_red_team_row_is_rejected() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt182_positively_phrased_red_team_row_is_rejected: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let positive = NewDiagnosticsValidationRow {
        row_id: "mt-182.red-team.positive-phrasing-probe".to_string(),
        matrix_kind: kind::RED_TEAM_AUTOMATION_AUTHORITY.to_string(),
        surface: "automation_authority".to_string(),
        check_id: "red_team.positive_phrasing_probe".to_string(),
        requirement: "Automation is allowed to drive any window it can find.".to_string(),
        status: DiagnosticsValidationStatus::Required,
        evidence_ref: None,
    };
    let err = store
        .record_diagnostics_validation_row(&positive)
        .await
        .expect_err("positively-phrased red-team row must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "positive red-team phrasing must produce a Validation error, got {err:?}"
    );
}

/// MT-195: stale-source (README/spec drift) + path-portability validation rows
/// persist, reload, and leave EventLedger evidence.
#[tokio::test]
async fn mt195_stale_source_and_path_portability_matrix_persists() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt195_stale_source_and_path_portability_matrix_persists: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;
    let by_id = record_and_reload_kind(&store, kind::STALE_SOURCE_PATH_PORTABILITY).await;

    let expected = [
        ("mt-195.stale-source.readme-drift", "stale_source"),
        ("mt-195.stale-source.mirror-sync", "stale_source"),
        (
            "mt-195.path-portability.drive-letter-rejected",
            "path_portability",
        ),
        (
            "mt-195.path-portability.user-profile-rejected",
            "path_portability",
        ),
    ];
    assert_required_rows(&by_id, kind::STALE_SOURCE_PATH_PORTABILITY, &expected);
    for (row_id, _) in &expected {
        assert_row_recorded_event(&database, row_id, kind::STALE_SOURCE_PATH_PORTABILITY).await;
    }
}

/// MT-195 negative: a portability-kind row citing a Windows drive-letter path
/// or a user-profile path as evidence is rejected by the path-portability
/// probe, so the matrix can never certify portability with a machine-local ref.
#[tokio::test]
async fn mt195_nonportable_evidence_refs_are_rejected() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt195_nonportable_evidence_refs_are_rejected: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    // Layering: drive-letter, `~/`, and rooted `/home/...` refs are stopped by
    // the canonical machine-local boundary (`reject_legacy_runtime_ref` ->
    // ForbiddenStorage). The MT-195 portability layer on top of it catches the
    // user-profile shapes the canonical gate misses (embedded `%appdata%`,
    // `$home`) and raises Validation. Both layers fail closed; each probe
    // asserts the variant its layer raises.
    for (probe_suffix, bad_ref, expect_forbidden_storage) in [
        ("drive-letter", "D:/Projects/handshake/src/lib.rs", true),
        ("user-profile", "~/handshake/src/lib.rs", true),
        ("home-dir", "/home/operator/handshake/src/lib.rs", true),
        ("appdata", "cache/%appdata%/handshake/baseline.json", false),
        ("dollar-home", "deploy/$home/handshake/baseline.json", false),
    ] {
        let nonportable = NewDiagnosticsValidationRow {
            row_id: format!("mt-195.path-portability.bad-ref-probe-{probe_suffix}"),
            matrix_kind: kind::STALE_SOURCE_PATH_PORTABILITY.to_string(),
            surface: "path_portability".to_string(),
            check_id: format!("path_portability.bad_ref_probe_{probe_suffix}"),
            requirement: "Probe row: nonportable evidence refs must be rejected.".to_string(),
            status: DiagnosticsValidationStatus::Required,
            evidence_ref: Some(bad_ref.to_string()),
        };
        let err = store
            .record_diagnostics_validation_row(&nonportable)
            .await
            .unwrap_err();
        if expect_forbidden_storage {
            assert!(
                matches!(err, AtelierError::ForbiddenStorage(_)),
                "nonportable {probe_suffix} evidence ref must be stopped by the canonical \
                 machine-local boundary (ForbiddenStorage), got {err:?}"
            );
        } else {
            assert!(
                matches!(err, AtelierError::Validation(_)),
                "nonportable {probe_suffix} evidence ref must be stopped by the MT-195 \
                 portability layer (Validation), got {err:?}"
            );
        }
    }
}
