//! WP-KERNEL-005 MT-171..MT-175: real PostgreSQL round-trip proofs for the typed
//! diagnostics "validation matrix".
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
    DiagnosticsValidationRow, DiagnosticsValidationStatus, NewDiagnosticsValidationRow,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use std::collections::HashMap;

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
