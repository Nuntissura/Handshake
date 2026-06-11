//! WP-KERNEL-005 MT-190 / MT-191 / MT-192 / MT-193 / MT-194: real PostgreSQL
//! round-trip proofs for the DCC/Flight-Recorder model-workflow diagnostics
//! surfaces.
//!
//! These MTs are TYPED RUNTIME surfaces (Postgres rows + EventLedger events),
//! never governance markdown:
//!   * MT-190 -- DCC Approvals + Visual-Capture panel projections (projection,
//!     GUI later), one typed row per panel kind.
//!   * MT-191 -- Flight Recorder events for tool calls / proposals / apply
//!     decisions (FR-EVT-TOOL-CALL / FR-EVT-TOOL-PROPOSAL /
//!     FR-EVT-TOOL-APPLY-DECISION).
//!   * MT-192 -- Flight Recorder events for visual capture / validation /
//!     recovery (FR-EVT-VISUAL-CAPTURE / FR-EVT-VISUAL-VALIDATION /
//!     FR-EVT-VISUAL-RECOVERY).
//!   * MT-193 -- Flight Recorder events for build/package guard and stale-doc
//!     detection (FR-EVT-BUILD-GUARD / FR-EVT-PACKAGE-GUARD /
//!     FR-EVT-STALE-DOC-DETECTED).
//!   * MT-194 -- reset/orphan diagnostic validation-matrix rows
//!     (matrix_kind `MT-194.reset-orphan`) recorded through the existing
//!     atelier_diagnostics_validation_matrix runtime surface.
//!
//! Every test persists through the real `AtelierStore` against
//! Handshake-managed PostgreSQL, RE-READS the rows from PostgreSQL, and
//! asserts the canonical EventLedger evidence. Gated on
//! `atelier_pg_support::database_url()`: when no PostgreSQL is available the
//! test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::dcc_flight_recorder::{
    dcc_flight_recorder_event_family, reset_orphan_diagnostics_validation_catalog,
    reset_orphan_validation_matrix_kind, DccWorkflowPanelKind, NewDccWorkflowPanelProjection,
    NewFrWorkflowEvent,
};
use handshake_core::atelier::state_probe::{
    state_probe_event_family, DiagnosticsValidationStatus,
};
use handshake_core::atelier::{event_family, AtelierError, AtelierStore};
use handshake_core::flight_recorder::workflow_event_kinds::FrWorkflowEventKind;
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

/// Connect with a real kernel EventLedger so every test can assert the
/// canonical ledger evidence, then ensure the atelier schema (which wires
/// migration 0116). Idempotent.
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

/// Assert one `AtelierDomainEventRecorded` ledger row exists for the given
/// aggregate with the given event family.
async fn assert_ledger_event(
    database: &Arc<dyn Database>,
    aggregate_type: &str,
    aggregate_id: &str,
    expected_family: &str,
) {
    let kernel_events = database
        .list_kernel_events_for_aggregate(aggregate_type, aggregate_id)
        .await
        .expect("list kernel events for aggregate");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == serde_json::json!(expected_family)
        }),
        "aggregate {aggregate_type}/{aggregate_id} must carry an EventLedger row \
         with event_family {expected_family}"
    );
}

/// MT-190: a DCC workflow panel projection round-trips for both panel kinds
/// (APPROVALS + VISUAL_CAPTURE) and emits canonical EventLedger evidence.
#[tokio::test]
async fn mt190_dcc_approvals_and_visual_capture_panels_round_trip() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt190_dcc_approvals_and_visual_capture_panels_round_trip: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    for kind in DccWorkflowPanelKind::ALL.iter().copied() {
        let panel_id = format!("dcc-wf-{}-{}", kind.as_token(), Uuid::new_v4());
        let state = match kind {
            DccWorkflowPanelKind::Approvals => serde_json::json!({
                "panel_kind": kind.as_token(),
                "pending_approvals": [
                    { "proposal_id": "prop-001", "requested_by": "coder-lane", "decision": null }
                ],
                "decided_approvals": [
                    { "proposal_id": "prop-000", "decision": "approved" }
                ],
            }),
            DccWorkflowPanelKind::VisualCapture => serde_json::json!({
                "panel_kind": kind.as_token(),
                "captures": [
                    { "capture_ref": "artifact://atelier/captures/018f7848-0001", "verdict": "pass" }
                ],
            }),
        };
        let recorded = store
            .record_dcc_workflow_panel_projection(&NewDccWorkflowPanelProjection {
                panel_id: panel_id.clone(),
                panel_kind: kind,
                state_json: state.clone(),
            })
            .await
            .unwrap_or_else(|err| panic!("record DCC workflow panel for {kind:?}: {err:?}"));
        assert_eq!(recorded.panel_id, panel_id);
        assert_eq!(recorded.panel_kind, kind);
        assert_eq!(recorded.state_json, state);

        // RE-READ from PostgreSQL via the kind-scoped list projection.
        let listed = store
            .list_dcc_workflow_panel_projections_by_kind(kind)
            .await
            .unwrap_or_else(|err| panic!("list DCC workflow panels for {kind:?}: {err:?}"));
        let found = listed
            .iter()
            .find(|p| p.panel_id == panel_id)
            .unwrap_or_else(|| panic!("recorded {kind:?} panel must be listed by its kind"));
        assert_eq!(found, &recorded, "round-trip must preserve every field");
        assert!(
            listed.iter().all(|p| p.panel_kind == kind),
            "list_by_kind({kind:?}) must only return rows of that kind"
        );

        // Canonical EventLedger evidence.
        assert_ledger_event(
            &database,
            "atelier_dcc_workflow_panel_projection",
            &panel_id,
            dcc_flight_recorder_event_family::DCC_WORKFLOW_PANEL_PROJECTION_RECORDED,
        )
        .await;
    }

    // The MT-190 vocabulary is approvals + visual captures ONLY: an MT-148
    // session-panel token is rejected by the typed kind.
    assert!(DccWorkflowPanelKind::from_token("SESSION").is_err());

    // A scalar panel state is rejected (a projection is structured state).
    let err = store
        .record_dcc_workflow_panel_projection(&NewDccWorkflowPanelProjection {
            panel_id: format!("dcc-wf-bad-{}", Uuid::new_v4()),
            panel_kind: DccWorkflowPanelKind::Approvals,
            state_json: serde_json::json!("not-an-object"),
        })
        .await
        .expect_err("scalar panel state must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "scalar panel state must be a Validation error, got {err:?}"
    );

    // The event family is folded into the canonical atelier family set.
    assert!(event_family::ALL
        .contains(&dcc_flight_recorder_event_family::DCC_WORKFLOW_PANEL_PROJECTION_RECORDED));
}

/// Shared MT-191/192/193 driver: persist one FR workflow event per kind,
/// re-read by kind and by mt_owner, and assert EventLedger evidence.
async fn prove_fr_workflow_kinds(
    store: &AtelierStore,
    database: &Arc<dyn Database>,
    mt_owner: &str,
    cases: &[(FrWorkflowEventKind, serde_json::Value)],
) {
    let mut record_ids = Vec::new();
    for (kind, payload) in cases {
        assert_eq!(
            kind.mt_owner(),
            mt_owner,
            "{kind} must be owned by {mt_owner}"
        );
        let record_id = format!("frwf-{}-{}", kind.as_str(), Uuid::new_v4());
        let recorded = store
            .record_fr_workflow_event(&NewFrWorkflowEvent {
                record_id: record_id.clone(),
                event_kind: *kind,
                session_ref: Some(format!("session://handshake/{}", Uuid::new_v4())),
                payload: payload.clone(),
            })
            .await
            .unwrap_or_else(|err| panic!("record FR workflow event {kind}: {err:?}"));
        assert_eq!(recorded.record_id, record_id);
        assert_eq!(recorded.event_kind, *kind);
        assert_eq!(recorded.mt_owner, mt_owner);
        assert_eq!(&recorded.payload, payload);

        // RE-READ from PostgreSQL via the kind-scoped list.
        let listed = store
            .list_fr_workflow_events_by_kind(*kind)
            .await
            .unwrap_or_else(|err| panic!("list FR workflow events for {kind}: {err:?}"));
        let found = listed
            .iter()
            .find(|e| e.record_id == record_id)
            .unwrap_or_else(|| panic!("recorded {kind} event must be listed by its kind"));
        assert_eq!(found, &recorded, "round-trip must preserve every field");
        assert!(
            listed.iter().all(|e| e.event_kind == *kind),
            "list_by_kind({kind}) must only return rows of that kind"
        );

        // Canonical EventLedger evidence carries the typed kind + owner.
        let kernel_events = database
            .list_kernel_events_for_aggregate("atelier_fr_workflow_event", &record_id)
            .await
            .expect("list FR workflow event kernel events");
        assert!(
            kernel_events.iter().any(|event| {
                event.event_type == KernelEventType::AtelierDomainEventRecorded
                    && event.payload["event_family"]
                        == serde_json::json!(
                            dcc_flight_recorder_event_family::FR_WORKFLOW_EVENT_RECORDED
                        )
                    && event.payload["atelier_payload"]["event_kind"]
                        == serde_json::json!(kind.as_str())
                    && event.payload["atelier_payload"]["mt_owner"]
                        == serde_json::json!(mt_owner)
            }),
            "recording {kind} must emit canonical EventLedger evidence naming kind + owner"
        );

        record_ids.push(record_id);
    }

    // RE-READ by owner: every record of this MT is grouped under its owner.
    let owned = store
        .list_fr_workflow_events_by_mt_owner(mt_owner)
        .await
        .expect("list FR workflow events by mt_owner");
    for record_id in &record_ids {
        assert!(
            owned.iter().any(|e| &e.record_id == record_id),
            "{record_id} must be listed under {mt_owner}"
        );
    }
    assert!(
        owned.iter().all(|e| e.mt_owner == mt_owner),
        "list_by_mt_owner({mt_owner}) must only return rows of that owner"
    );
}

/// MT-191: Flight Recorder events for tool calls, proposals, and apply
/// decisions persist on PostgreSQL and emit EventLedger evidence.
#[tokio::test]
async fn mt191_fr_tool_proposal_apply_events_persist_and_emit() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt191_fr_tool_proposal_apply_events_persist_and_emit: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    prove_fr_workflow_kinds(
        &store,
        &database,
        "MT-191",
        &[
            (
                FrWorkflowEventKind::ToolCall,
                serde_json::json!({ "tool_id": "fs.read", "status": "ok", "duration_ms": 12 }),
            ),
            (
                FrWorkflowEventKind::ToolProposal,
                serde_json::json!({ "tool_id": "fs.write", "proposal_id": "prop-001" }),
            ),
            (
                FrWorkflowEventKind::ToolApplyDecision,
                serde_json::json!({ "proposal_id": "prop-001", "decision": "approved" }),
            ),
        ],
    )
    .await;

    // A hollow tool-call payload (missing `status`) is rejected: the event
    // kinds are contracts, not free-form strings.
    let err = store
        .record_fr_workflow_event(&NewFrWorkflowEvent {
            record_id: format!("frwf-bad-{}", Uuid::new_v4()),
            event_kind: FrWorkflowEventKind::ToolCall,
            session_ref: None,
            payload: serde_json::json!({ "tool_id": "fs.read" }),
        })
        .await
        .expect_err("hollow tool-call payload must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "hollow payload must be a Validation error, got {err:?}"
    );
}

/// MT-192: Flight Recorder events for visual capture, validation, and
/// recovery persist on PostgreSQL and emit EventLedger evidence.
#[tokio::test]
async fn mt192_fr_visual_validation_recovery_events_persist_and_emit() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt192_fr_visual_validation_recovery_events_persist_and_emit: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let capture_ref = format!("artifact://atelier/captures/{}", Uuid::new_v4());
    prove_fr_workflow_kinds(
        &store,
        &database,
        "MT-192",
        &[
            (
                FrWorkflowEventKind::VisualCapture,
                serde_json::json!({ "capture_ref": &capture_ref, "surface": "dcc" }),
            ),
            (
                FrWorkflowEventKind::VisualValidation,
                serde_json::json!({ "capture_ref": &capture_ref, "verdict": "fail" }),
            ),
            (
                FrWorkflowEventKind::VisualRecovery,
                serde_json::json!({
                    "capture_ref": &capture_ref,
                    "recovery_action": "recapture_after_layout_fix"
                }),
            ),
        ],
    )
    .await;

    // A machine-local session ref is rejected at the persistence boundary.
    let err = store
        .record_fr_workflow_event(&NewFrWorkflowEvent {
            record_id: format!("frwf-bad-{}", Uuid::new_v4()),
            event_kind: FrWorkflowEventKind::VisualCapture,
            session_ref: Some("C:/Users/op/session.json".to_string()),
            payload: serde_json::json!({ "capture_ref": "artifact://atelier/captures/x" }),
        })
        .await
        .expect_err("machine-local session ref must be rejected");
    assert!(
        matches!(
            err,
            AtelierError::Validation(_) | AtelierError::ForbiddenStorage(_)
        ),
        "machine-local session ref must be rejected, got {err:?}"
    );
}

/// MT-193: Flight Recorder events for build guard, package guard, and
/// stale-doc detection persist on PostgreSQL and emit EventLedger evidence.
#[tokio::test]
async fn mt193_fr_build_package_stale_doc_events_persist_and_emit() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt193_fr_build_package_stale_doc_events_persist_and_emit: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    prove_fr_workflow_kinds(
        &store,
        &database,
        "MT-193",
        &[
            (
                FrWorkflowEventKind::BuildGuard,
                serde_json::json!({ "build_id": "build-2026-06-10-01", "verdict": "pass" }),
            ),
            (
                FrWorkflowEventKind::PackageGuard,
                serde_json::json!({ "package_id": "handshake-core-0.1.0", "verdict": "blocked" }),
            ),
            (
                FrWorkflowEventKind::StaleDocDetected,
                serde_json::json!({
                    "doc_ref": "manual://handshake/model-manual",
                    "staleness_kind": "generated_doc_behind_source"
                }),
            ),
        ],
    )
    .await;

    // The event family is folded into the canonical atelier family set.
    assert!(event_family::ALL
        .contains(&dcc_flight_recorder_event_family::FR_WORKFLOW_EVENT_RECORDED));
}

/// MT-194: the reset/orphan diagnostic validation-matrix rows persist on
/// PostgreSQL under matrix_kind `MT-194.reset-orphan`, re-read with their
/// requirement + evidence, and emit EventLedger evidence per row.
#[tokio::test]
async fn mt194_reset_orphan_validation_matrix_rows_persist() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt194_reset_orphan_validation_matrix_rows_persist: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    // Record the MT-194 catalog through the existing validation-matrix write
    // path (idempotent upsert on row_id).
    let catalog = reset_orphan_diagnostics_validation_catalog();
    for new in &catalog {
        store
            .record_diagnostics_validation_row(new)
            .await
            .unwrap_or_else(|err| panic!("record MT-194 row {}: {err:?}", new.row_id));
    }

    // RE-READ from PostgreSQL scoped to the MT-194 matrix kind.
    let rows = store
        .list_diagnostics_validation_matrix(Some(reset_orphan_validation_matrix_kind::RESET_ORPHAN))
        .await
        .expect("list MT-194 reset/orphan validation matrix rows");

    let expected: &[(&str, &str)] = &[
        ("mt-194.reset.operation-recorded", "intake_reset"),
        ("mt-194.reset.event-emitted", "intake_reset"),
        ("mt-194.reset.invariants-enforced", "kernel_reset"),
        ("mt-194.orphan.manifest-recorded", "intake_orphan"),
        ("mt-194.orphan.item-adoption", "intake_orphan"),
        ("mt-194.reset-orphan.diagnostics-projection", "diagnostics"),
    ];
    for (row_id, surface) in expected {
        let row = rows
            .iter()
            .find(|r| r.row_id == *row_id)
            .unwrap_or_else(|| panic!("MT-194 matrix row {row_id} must persist"));
        assert_eq!(
            row.matrix_kind,
            reset_orphan_validation_matrix_kind::RESET_ORPHAN,
            "row {row_id} must belong to the MT-194 matrix kind"
        );
        assert_eq!(row.surface, *surface, "row {row_id} must name {surface}");
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
                .is_some_and(|r| !r.trim().is_empty() && !r.contains(".GOV")),
            "row {row_id} must cite product evidence, not governance paperwork"
        );

        // Canonical EventLedger evidence per recorded row.
        assert_ledger_event(
            &database,
            "atelier_diagnostics_validation_matrix",
            row_id,
            state_probe_event_family::DIAGNOSTICS_VALIDATION_ROW_RECORDED,
        )
        .await;
    }

    // Run-scoped negative control: the MT-194 kind never absorbs rows of the
    // MT-171..175 catalog kinds.
    assert!(
        rows.iter()
            .all(|r| r.matrix_kind == reset_orphan_validation_matrix_kind::RESET_ORPHAN),
        "the MT-194 list must only return MT-194 rows"
    );
}
