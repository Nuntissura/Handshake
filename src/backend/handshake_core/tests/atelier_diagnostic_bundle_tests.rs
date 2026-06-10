//! WP-KERNEL-005 MT-112 (Pose Diagnostic Bundle Hook) live PostgreSQL
//! round-trip proof for the `atelier::comfy` submodule.
//!
//! No mocks: the test connects the real `AtelierStore` to a real Postgres,
//! ensures the schema, records a diagnostic bundle with REAL data spanning all
//! contract fields (request, refs, versions, logs, artifacts, error taxonomy),
//! reloads it, and asserts:
//!   * round-trip fidelity of every field,
//!   * the canonical `DIAGNOSTIC_BUNDLE_RECORDED` EventLedger family on the
//!     `workflow_run_id` aggregate (per-aggregate count, not global),
//!   * a .GOV/SQLite ref anywhere in `refs` is rejected.
//! The table persists between runs, so the run-scoped `workflow_run_id` is made
//! unique per run via `Uuid::new_v4()` to avoid cross-run collisions.
//!
//! NOTE: this proof passes only once the orchestrator wires migration 0109 into
//! `ensure_schema`; until then it is gated on PostgreSQL availability and skips.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::comfy::{comfy_event_family, DiagnosticBundle, NewDiagnosticBundle};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

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

#[tokio::test]
async fn mt112_diagnostic_bundle_records_request_refs_versions_logs_artifacts_taxonomy() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt112_diagnostic_bundle_records_request_refs_versions_logs_artifacts_taxonomy: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let workflow_run_id = Uuid::new_v4();
    let new = NewDiagnosticBundle {
        workflow_run_id,
        request: json!({
            "graph_hash": "sha256:graph-abc",
            "prompt": { "positive": "pose rig", "negative": "" },
            "seed": 42,
        }),
        refs: json!({
            "workflow_spec_ref": "artifact://workflow-spec/pose-rig-v1",
            "workflow_json_ref": "artifact://workflow-json/run-abc",
            "prompt_ref": "artifact://prompt/run-abc",
            "nested": { "image_refs": ["artifact://image/a", "artifact://image/b"] },
        }),
        versions: json!({
            "pose_model_asset_version": "pose-model@2.3.1",
            "image_tool_version": "image-tools@1.7.0",
            "comfy_model_version": "comfy-sdxl@0.9.4",
        }),
        logs_ref: "artifact://logs/run-abc".to_string(),
        artifacts: json!({
            "partial_outputs": ["artifact://image/partial-1"],
            "manifest_ref": "artifact://manifest/run-abc",
        }),
        error_taxonomy: "comfy.node_execution_failed".to_string(),
    };

    let bundle: DiagnosticBundle = store
        .record_diagnostic_bundle(&new)
        .await
        .expect("record diagnostic bundle");

    // Field fidelity on the returned record.
    assert_eq!(bundle.workflow_run_id, workflow_run_id);
    assert_eq!(bundle.request["seed"], json!(42));
    assert_eq!(
        bundle.refs["workflow_spec_ref"],
        json!("artifact://workflow-spec/pose-rig-v1")
    );
    assert_eq!(bundle.refs["nested"]["image_refs"][1], json!("artifact://image/b"));
    assert_eq!(bundle.versions["comfy_model_version"], json!("comfy-sdxl@0.9.4"));
    assert_eq!(bundle.logs_ref, "artifact://logs/run-abc");
    assert_eq!(
        bundle.artifacts["manifest_ref"],
        json!("artifact://manifest/run-abc")
    );
    assert_eq!(bundle.error_taxonomy, "comfy.node_execution_failed");

    // Round-trips by workflow run with full fidelity.
    let reloaded = store
        .get_diagnostic_bundle(workflow_run_id)
        .await
        .expect("reload diagnostic bundle")
        .expect("diagnostic bundle exists");
    assert_eq!(reloaded, bundle);

    // Idempotent on workflow_run_id: re-record refreshes the same row.
    let mut repin = new.clone();
    repin.error_taxonomy = "comfy.timeout".to_string();
    let repinned = store
        .record_diagnostic_bundle(&repin)
        .await
        .expect("re-record diagnostic bundle");
    assert_eq!(
        repinned.bundle_id, bundle.bundle_id,
        "re-record keeps the same bundle row"
    );
    assert_eq!(repinned.error_taxonomy, "comfy.timeout");

    // Reject case: a .GOV / SQLite ref anywhere in `refs` is rejected.
    let mut legacy = new.clone();
    legacy.workflow_run_id = Uuid::new_v4();
    legacy.refs = json!({
        "workflow_spec_ref": "artifact://workflow-spec/pose-rig-v1",
        "leaked": "sqlite:///.GOV/comfy.sqlite",
    });
    let err = store
        .record_diagnostic_bundle(&legacy)
        .await
        .expect_err("a .GOV/SQLite ref in refs must be rejected");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("refs"),
        "rejection must name the offending refs field: {err}"
    );

    // EventLedger evidence (MT-005 coverage): recording emits the canonical
    // family on the workflow_run_id aggregate. Count is per-aggregate, not
    // global, so it is deterministic across concurrent runs.
    let events = database
        .list_kernel_events_for_aggregate(
            "atelier_comfy_diagnostic_bundle",
            &workflow_run_id.to_string(),
        )
        .await
        .expect("list diagnostic bundle EventLedger rows");
    let recorded: Vec<_> = events
        .iter()
        .filter(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == comfy_event_family::DIAGNOSTIC_BUNDLE_RECORDED
        })
        .collect();
    assert_eq!(
        recorded.len(),
        2,
        "one DIAGNOSTIC_BUNDLE_RECORDED event per record call on this aggregate"
    );
    let last = recorded
        .last()
        .expect("at least one diagnostic bundle event");
    assert_eq!(
        last.payload["atelier_payload"]["error_taxonomy"],
        json!("comfy.timeout")
    );
    assert_eq!(
        last.payload["atelier_payload"]["logs_ref"],
        json!("artifact://logs/run-abc")
    );
}
