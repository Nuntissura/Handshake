//! WP-KERNEL-005 MT-181 — Diagnostics Integration Smoke Path, executed for
//! real against Handshake-managed PostgreSQL.
//!
//! One no-context model operation path is RUN end to end (not described):
//!   session -> lease -> state -> command -> receipt -> visual -> bundle -> release
//!
//! Every stage persists through the real `AtelierStore` surface that owns it,
//! is RE-READ from PostgreSQL (never trusted from the write echo), and is
//! asserted against its canonical EventLedger family:
//!   * session  — `record_session_heartbeat` (MT-144),
//!     `atelier.diagnostics.session_heartbeat_recorded`
//!   * lease    — `claim_model_lease` (MT-143), `atelier.model_lease.claimed`
//!   * state    — `record_work_state_projection` (MT-147),
//!     `atelier.work_state_projection.recorded` family constant
//!   * command  — `record_action_receipt` against the REAL registered
//!     KERNEL-002 catalog action id (MT-139),
//!     `atelier.action_receipt.recorded`
//!   * receipt  — `record_command_log_entry` tying the session to the receipt
//!     (MT-145), `atelier.command_log.recorded`
//!   * visual   — stealth window + capture promoted to a governed screenshot
//!     artifact (MT-153/157), `SCREENSHOT_ARTIFACT_STORED`
//!   * bundle   — `record_kernel_diagnostic_bundle_manifest` (MT-141/176),
//!     `kernel.diagnostics.bundle_manifest_recorded` — the smoke path's
//!     terminal step, formerly a Planned ModelManual scaffold, now executed
//!   * release  — `release_model_lease` (MT-143),
//!     `atelier.model_lease.released`
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use std::sync::Arc;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::atelier::action_receipt::{
    action_receipt_event_family, ActionReceiptStatus, NewActionReceipt,
};
use handshake_core::atelier::command_corpus::{
    command_log_event_family, NewCommandLogEntry, SessionStatus,
};
use handshake_core::atelier::model_lease::{model_lease_event_family, NewModelLeaseClaim};
use handshake_core::atelier::state_probe::{
    diagnostics_projection_event_family, NewScreenshotArtifactStorage, NewWorkStateProjection,
};
use handshake_core::atelier::stealth_window::{NewStealthWindow, QuietFlags, VisibilityFlag};
use handshake_core::atelier::AtelierStore;
use handshake_core::diagnostics::bundle_manifest::{
    kernel_diagnostic_bundle_event_family, DiagnosticBundleSection, DiagnosticBundleSectionKind,
    NewDiagnosticBundleManifest,
};
use handshake_core::diagnostics::DiagnosticSeverity;
use handshake_core::kernel::role_mailbox_claim_lease::{
    ClaimLeaseState, RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::model_manual::{model_manual, CommandStatus};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// The registered KERNEL-002 catalog action the smoke path's command stage
/// runs: the no-context model's first real read, "show me what I can do".
const SMOKE_COMMAND_ACTION_ID: &str = "kernel.action_catalog.view";

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

/// Assert one atelier-domain EventLedger event of `event_family` exists for
/// the aggregate and return its atelier payload.
async fn assert_smoke_event(
    database: &Arc<dyn Database>,
    aggregate_type: &str,
    aggregate_id: &str,
    event_family: &str,
) -> serde_json::Value {
    let events = database
        .list_kernel_events_for_aggregate(aggregate_type, aggregate_id)
        .await
        .expect("list smoke-path EventLedger rows");
    let event = events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family
        })
        .unwrap_or_else(|| {
            panic!("smoke stage {aggregate_type}/{aggregate_id} must emit {event_family}")
        });
    event.payload["atelier_payload"].clone()
}

/// MT-181: the diagnostics integration smoke path is RUN end to end against
/// live PostgreSQL — session -> lease -> state -> command -> receipt ->
/// visual -> bundle -> release — with persisted state re-read from the
/// database and the canonical EventLedger event asserted at every stage.
#[tokio::test]
async fn mt181_diagnostics_integration_smoke_path_runs_end_to_end_on_pg() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt181_diagnostics_integration_smoke_path_runs_end_to_end_on_pg: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let run = Uuid::now_v7();
    let actor_id = format!("smoke-model-{run}");
    let session_ref = format!("smoke-session:{run}");
    let thread_id = format!("mt181-smoke-thread-{run}");

    // ---- Stage 1: SESSION — open a governed diagnostics session. ----------
    let session = store
        .record_session_heartbeat(&session_ref)
        .await
        .expect("open smoke session via heartbeat");
    assert_eq!(session.session_ref, session_ref);
    assert_eq!(session.status, SessionStatus::Active);

    // Re-read the session row from PostgreSQL, not the write echo.
    let sessions = store
        .list_diagnostics_sessions()
        .await
        .expect("list diagnostics sessions");
    let persisted_session = sessions
        .iter()
        .find(|s| s.session_ref == session_ref)
        .expect("smoke session must persist in atelier_diagnostics_session");
    assert_eq!(persisted_session.status, SessionStatus::Active);

    assert_smoke_event(
        &database,
        "atelier_diagnostics_session",
        &session_ref,
        command_log_event_family::SESSION_HEARTBEAT_RECORDED,
    )
    .await;

    // ---- Stage 2: LEASE — claim exclusive work on the thread. -------------
    let lease = store
        .claim_model_lease(&NewModelLeaseClaim {
            thread_id: thread_id.clone(),
            executor_kind: RoleMailboxExecutorKind::LocalSmallModel,
            actor_id: actor_id.clone(),
            session_id: session_ref.clone(),
            claim_mode: RoleMailboxClaimMode::ExclusiveLease,
            ttl_seconds: 3600,
            linked_work_packet_id: "WP-KERNEL-005".to_string(),
            linked_micro_task_id: "MT-181".to_string(),
        })
        .await
        .expect("claim smoke lease");
    let lease_reread = store
        .get_model_lease(lease.claim_id)
        .await
        .expect("re-read smoke lease from PostgreSQL");
    assert_eq!(lease_reread.thread_id, thread_id);
    assert_eq!(lease_reread.actor_id, actor_id);
    assert_eq!(lease_reread.effective_state, ClaimLeaseState::Active);
    assert!(!lease_reread.lease_expired);

    let lease_payload = assert_smoke_event(
        &database,
        "atelier_model_lease",
        &lease.claim_id.to_string(),
        model_lease_event_family::MODEL_LEASE_CLAIMED,
    )
    .await;
    assert_eq!(lease_payload["thread_id"], json!(thread_id));

    // ---- Stage 3: STATE — project the model-readable work state. ----------
    let projection_id = format!("smoke-wsp-{run}");
    let state = store
        .record_work_state_projection(&NewWorkStateProjection {
            projection_id: projection_id.clone(),
            active_mt: "MT-181".to_string(),
            owner: actor_id.clone(),
            status: "SMOKE_PATH_RUNNING".to_string(),
            blocker: None,
            receipts_ref: None,
            next_action: Some(format!("run {SMOKE_COMMAND_ACTION_ID} and record receipt")),
            evidence_ref: Some(format!("lease:{}", lease.claim_id)),
        })
        .await
        .expect("record smoke work-state projection");
    assert_eq!(state.status, "SMOKE_PATH_RUNNING");

    let projections = store
        .list_work_state_projections()
        .await
        .expect("list work-state projections");
    let persisted_state = projections
        .iter()
        .find(|p| p.projection_id == projection_id)
        .expect("smoke work state must persist in atelier_work_state_projection");
    assert_eq!(persisted_state.active_mt, "MT-181");
    assert_eq!(persisted_state.owner, actor_id);
    assert_eq!(
        persisted_state.evidence_ref.as_deref(),
        Some(format!("lease:{}", lease.claim_id).as_str())
    );

    assert_smoke_event(
        &database,
        "atelier_work_state_projection",
        &projection_id,
        diagnostics_projection_event_family::WORK_STATE_PROJECTION_RECORDED,
    )
    .await;

    // ---- Stage 4: COMMAND — run a registered catalog action; the receipt
    // validates the action_id against the REAL KERNEL-002 catalog. ----------
    let started_at = Utc::now();
    let receipt = store
        .record_action_receipt(&NewActionReceipt {
            action_id: SMOKE_COMMAND_ACTION_ID.to_string(),
            actor_kind: "agent".to_string(),
            actor_id: actor_id.clone(),
            session_id: session_ref.clone(),
            params: json!({ "query": "smoke-path", "limit": 5 }),
            started_at_utc: started_at,
            completed_at_utc: Utc::now(),
            status: ActionReceiptStatus::Succeeded,
            target_refs: vec!["kernel://action-catalog/kernel002-action-catalog-v1".to_string()],
            evidence_refs: vec![format!("lease:{}", lease.claim_id)],
            result_refs: vec![format!("smoke://command-result/{run}")],
            error_class: None,
            recovery_hint: None,
        })
        .await
        .expect("record smoke command action receipt");

    let receipt_reread = store
        .get_action_receipt(receipt.receipt_id)
        .await
        .expect("re-read smoke receipt from PostgreSQL");
    assert_eq!(receipt_reread.action_id, SMOKE_COMMAND_ACTION_ID);
    assert_eq!(receipt_reread.session_id, session_ref);
    assert_eq!(receipt_reread.status, ActionReceiptStatus::Succeeded);
    assert!(
        !receipt_reread.params_sha256.is_empty(),
        "receipt must persist a params hash, never raw params"
    );

    assert_smoke_event(
        &database,
        "atelier_action_receipt",
        &receipt.receipt_id.to_string(),
        action_receipt_event_family::ACTION_RECEIPT_RECORDED,
    )
    .await;

    // ---- Stage 5: RECEIPT — tie the receipt into the session's append-only
    // command log so the session evidence chain is queryable. ---------------
    let command_log_id = format!("smoke-cmdlog:{run}");
    let receipt_ref = format!("receipt:{}", receipt.receipt_id);
    store
        .record_command_log_entry(&NewCommandLogEntry {
            command_log_id: command_log_id.clone(),
            session_ref: session_ref.clone(),
            command_id: SMOKE_COMMAND_ACTION_ID.to_string(),
            status: "succeeded".to_string(),
            receipt_ref: Some(receipt_ref.clone()),
            evidence_ref: Some(format!("smoke://command-result/{run}")),
        })
        .await
        .expect("record smoke command-log entry");

    let session_log = store
        .list_command_log_for_session(&session_ref)
        .await
        .expect("list smoke session command log");
    assert_eq!(session_log.len(), 1, "exactly the smoke command is logged");
    assert_eq!(session_log[0].command_log_id, command_log_id);
    assert_eq!(
        session_log[0].receipt_ref.as_deref(),
        Some(receipt_ref.as_str()),
        "command log must point back at the persisted receipt"
    );

    assert_smoke_event(
        &database,
        "atelier_command_log",
        &command_log_id,
        command_log_event_family::COMMAND_LOG_RECORDED,
    )
    .await;

    // ---- Stage 6: VISUAL — capture GUI evidence quietly and promote it to a
    // governed, retained screenshot artifact. -------------------------------
    let window = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: actor_id.clone(),
            title: format!("smoke-visual-{run}"),
            visibility: VisibilityFlag::OffScreenOnly,
            quiet: QuietFlags::default(),
            layout: None,
        })
        .await
        .expect("create smoke stealth window");
    let manifest_ref = format!("artifact-manifest-{run}");
    let content_sha = format!("sha256-smoke-{run}");
    let capture = store
        .record_stealth_capture(window.window_ref_id, &manifest_ref, &content_sha)
        .await
        .expect("record smoke stealth capture");

    let storage_id = format!("smoke-sas-{run}");
    store
        .record_screenshot_artifact_storage(&NewScreenshotArtifactStorage {
            storage_id: storage_id.clone(),
            capture_id: capture.capture_id,
            artifact_manifest_id: manifest_ref.clone(),
            content_sha256: content_sha.clone(),
            mime: "image/png".to_string(),
            width_px: Some(1920),
            height_px: Some(1080),
            byte_len: Some(102_400),
            label: Some("mt181-smoke-path-visual".to_string()),
            retention_ttl_days: Some(30),
            pinned: false,
            retention_class: "visual-validation".to_string(),
            exportable: true,
            redaction_applied: true,
        })
        .await
        .expect("store smoke screenshot artifact");

    let stored = store
        .list_screenshot_artifact_storage()
        .await
        .expect("list screenshot artifact storage");
    let persisted_visual = stored
        .iter()
        .find(|row| row.storage_id == storage_id)
        .expect("smoke visual must persist in atelier_screenshot_artifact_storage");
    assert_eq!(persisted_visual.capture_id, capture.capture_id);
    assert_eq!(persisted_visual.content_sha256, content_sha);
    assert!(
        persisted_visual.exportable && persisted_visual.redaction_applied,
        "smoke visual evidence must be export-safe (redacted) for the bundle"
    );

    assert_smoke_event(
        &database,
        "atelier_screenshot_artifact_storage",
        &storage_id,
        diagnostics_projection_event_family::SCREENSHOT_ARTIFACT_STORED,
    )
    .await;

    // ---- Stage 7: BUNDLE — the terminal step, formerly a 'Planned' manual
    // scaffold, now the executed kernel diagnostic-bundle manifest export
    // carrying the evidence collected by every prior stage. -----------------
    let subject_ref = format!("smoke-path://session/{run}");
    let bundle = store
        .record_kernel_diagnostic_bundle_manifest(&NewDiagnosticBundleManifest {
            subject_kind: "diagnostics_smoke_path".to_string(),
            subject_ref: subject_ref.clone(),
            failure_summary: "no-context smoke handoff: end-to-end operation evidence export"
                .to_string(),
            error_taxonomy: "kernel.diagnostics_smoke_path_evidence".to_string(),
            severity: DiagnosticSeverity::Info,
            created_by: actor_id.clone(),
            sections: vec![
                DiagnosticBundleSection {
                    section_id: "session-lease".to_string(),
                    kind: DiagnosticBundleSectionKind::Diagnostics,
                    title: "Session and lease evidence".to_string(),
                    content_ref: None,
                    content_json: json!({
                        "session_ref": session_ref,
                        "lease_claim_id": lease.claim_id,
                        "thread_id": thread_id,
                    }),
                    item_count: 2,
                },
                DiagnosticBundleSection {
                    section_id: "command-receipt".to_string(),
                    kind: DiagnosticBundleSectionKind::EventLedger,
                    title: "Command, receipt, and command-log evidence".to_string(),
                    content_ref: None,
                    content_json: json!({
                        "action_id": SMOKE_COMMAND_ACTION_ID,
                        "receipt_id": receipt.receipt_id,
                        "command_log_id": command_log_id,
                        "params_sha256": receipt.params_sha256,
                    }),
                    item_count: 2,
                },
                DiagnosticBundleSection {
                    section_id: "visual".to_string(),
                    kind: DiagnosticBundleSectionKind::Logs,
                    title: "Redacted visual evidence".to_string(),
                    content_ref: Some(format!("artifact://{manifest_ref}")),
                    content_json: json!({
                        "storage_id": storage_id,
                        "capture_id": capture.capture_id,
                        "content_sha256": content_sha,
                    }),
                    item_count: 1,
                },
            ],
            reproduction_steps: vec![
                format!("heartbeat the session {session_ref} and claim the thread lease"),
                format!("run {SMOKE_COMMAND_ACTION_ID} and record its action receipt"),
                "capture the stealth visual and re-run this bundle export".to_string(),
            ],
            isolation_hints: vec![
                "replay the command-receipt EventLedger aggregates first".to_string(),
                "check the lease state before re-claiming the thread".to_string(),
            ],
        })
        .await
        .expect("export smoke diagnostic bundle manifest");

    let bundle_reread = store
        .get_kernel_diagnostic_bundle_manifest(bundle.manifest_id)
        .await
        .expect("re-read smoke bundle manifest from PostgreSQL")
        .expect("smoke bundle manifest must persist");
    assert_eq!(bundle_reread.subject_ref, subject_ref);
    assert_eq!(bundle_reread.sections.len(), 3);
    assert_eq!(bundle_reread.created_by, actor_id);

    let bundle_payload = assert_smoke_event(
        &database,
        "kernel_diagnostic_bundle_manifest",
        &bundle.manifest_id.to_string(),
        kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED,
    )
    .await;
    assert_eq!(bundle_payload["subject_ref"], json!(subject_ref));
    assert_eq!(bundle_payload["section_count"], json!(3));

    // ---- Stage 8: RELEASE — hand the thread back and confirm durably. -----
    let released = store
        .release_model_lease(lease.claim_id, &actor_id)
        .await
        .expect("release smoke lease");
    assert_eq!(released.stored_state, ClaimLeaseState::Released);

    let release_reread = store
        .get_model_lease(lease.claim_id)
        .await
        .expect("re-read released smoke lease");
    assert_eq!(release_reread.effective_state, ClaimLeaseState::Released);
    assert!(release_reread.released_at_utc.is_some());

    assert_smoke_event(
        &database,
        "atelier_model_lease",
        &lease.claim_id.to_string(),
        model_lease_event_family::MODEL_LEASE_RELEASED,
    )
    .await;
}

/// MT-181 scaffold guard: the smoke path's terminal manual step
/// `diagnostics_debug_bundle_export` is no longer a Planned placeholder — it
/// is Wired to the executable kernel diagnostic-bundle manifest surface and
/// survives the manual drift guard against the real runtime surface registry.
#[tokio::test]
async fn mt181_smoke_path_terminal_step_is_wired_not_planned_scaffold() {
    let manual = model_manual();
    let bundle_export = manual
        .command_reference
        .iter()
        .find(|command| command.id == "diagnostics_debug_bundle_export")
        .expect("manual must keep the diagnostics_debug_bundle_export row");
    assert_eq!(
        bundle_export.status,
        CommandStatus::Wired,
        "the smoke path's terminal step must be executable, not a Planned scaffold"
    );
    assert!(
        bundle_export
            .description
            .contains("src/diagnostics/bundle_manifest.rs"),
        "the terminal step must cite its executable implementation"
    );

    // The wired row must survive the MT-183 drift guard against the real
    // Handshake runtime surface registry: no unresolved-surface finding.
    use handshake_core::atelier::model_manual_merge::{
        run_manual_drift_guard, ManualDriftKind, RegisteredSurfaceIndex,
    };
    let findings =
        run_manual_drift_guard(manual, &RegisteredSurfaceIndex::handshake_runtime_default());
    let bundle_findings: Vec<_> = findings
        .iter()
        .filter(|finding| {
            finding.command_id.as_deref() == Some("diagnostics_debug_bundle_export")
                && finding.drift_kind == ManualDriftKind::WiredUnresolvedSurface
        })
        .collect();
    assert!(
        bundle_findings.is_empty(),
        "wiring the bundle export must not introduce manual drift: {bundle_findings:?}"
    );
}
