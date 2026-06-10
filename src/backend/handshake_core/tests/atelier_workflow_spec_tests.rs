//! WP-KERNEL-005 MT-106 (workflow spec registry) + MT-110 (external tool/model
//! version policy) live PostgreSQL round-trip proofs for the `atelier::comfy`
//! submodule.
//!
//! No mocks: each test connects the real `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the new records with REAL data, and asserts the
//! load-bearing invariants:
//!   * MT-106: a versioned spec registers, round-trips by id and via list, the
//!     upsert is idempotent on (workflow_kind, spec_version), and a legacy/SQLite
//!     source_ref is rejected.
//!   * MT-110: version metadata pins the three named provenance versions (pose
//!     model-asset, image tool, ComfyUI model) per run, round-trips, and links to
//!     a registered spec.
//! Each emits the canonical Atelier EventLedger family so MT-005 coverage holds.
//! Tables persist between runs, so all ids/hashes are made unique per run via
//! `Uuid::new_v4()` to avoid cross-run collisions.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::comfy::{
    comfy_event_family, ComfyVersionMetadata, NewComfyVersionMetadata, NewWorkflowSpec, WorkflowSpec,
};
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

fn new_spec(suffix: &str) -> NewWorkflowSpec {
    NewWorkflowSpec {
        workflow_kind: format!("pose_rig_v1_{suffix}"),
        spec_version: "1.0.0".to_string(),
        spec_hash: format!("sha256:spec-{suffix}"),
        handler_id: "engine.comfyui.pose_rig".to_string(),
        compatibility_pin: Some("bridge-protocol/1.0.0".to_string()),
        spec_json: json!({
            "nodes": [{ "id": "1", "class_type": "PoseRig" }],
            "links": [],
        }),
        source_ref: Some("artifact://workflow-spec/pose-rig-v1".to_string()),
    }
}

#[tokio::test]
async fn mt106_workflow_spec_registers_with_version_and_round_trips() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt106_workflow_spec_registers_with_version_and_round_trips: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let suffix = Uuid::new_v4().simple().to_string();
    let spec: WorkflowSpec = store
        .register_workflow_spec(&new_spec(&suffix))
        .await
        .expect("register versioned workflow spec");

    assert_eq!(spec.workflow_kind, format!("pose_rig_v1_{suffix}"));
    assert_eq!(spec.spec_version, "1.0.0");
    assert_eq!(spec.spec_hash, format!("sha256:spec-{suffix}"));
    assert_eq!(spec.handler_id, "engine.comfyui.pose_rig");
    assert_eq!(
        spec.compatibility_pin.as_deref(),
        Some("bridge-protocol/1.0.0")
    );
    assert_eq!(spec.spec_json["nodes"][0]["class_type"], json!("PoseRig"));

    // Round-trips by id.
    let reloaded = store
        .get_workflow_spec(spec.spec_id)
        .await
        .expect("reload workflow spec")
        .expect("workflow spec exists");
    assert_eq!(reloaded, spec);

    // Round-trips via list filtered by kind.
    let listed = store
        .list_workflow_specs(Some(&format!("pose_rig_v1_{suffix}")))
        .await
        .expect("list workflow specs by kind");
    assert!(listed.iter().any(|candidate| candidate.spec_id == spec.spec_id));

    // Idempotent upsert on (workflow_kind, spec_version): re-register with a new
    // handler refreshes the same row rather than duplicating identity.
    let mut updated = new_spec(&suffix);
    updated.handler_id = "engine.comfyui.pose_rig_v2".to_string();
    let upserted = store
        .register_workflow_spec(&updated)
        .await
        .expect("idempotent re-register of same kind+version");
    assert_eq!(upserted.spec_id, spec.spec_id, "upsert keeps the same spec_id");
    assert_eq!(upserted.handler_id, "engine.comfyui.pose_rig_v2");
    let after = store
        .list_workflow_specs(Some(&format!("pose_rig_v1_{suffix}")))
        .await
        .expect("list after upsert");
    assert_eq!(
        after.len(),
        1,
        "idempotent upsert must not create a duplicate identity"
    );

    // Reject case: a legacy/SQLite source_ref is rejected.
    let mut legacy = new_spec(&format!("{suffix}leg"));
    legacy.source_ref = Some("sqlite:///tmp/comfy.sqlite".to_string());
    let err = store
        .register_workflow_spec(&legacy)
        .await
        .expect_err("legacy/SQLite source_ref must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("source_ref"),
        "rejection must name the offending source_ref field: {err}"
    );

    // EventLedger evidence (MT-005 coverage): registration emits the canonical
    // family on the spec_id aggregate.
    let events = database
        .list_kernel_events_for_aggregate(
            "atelier_comfy_workflow_spec",
            &spec.spec_id.to_string(),
        )
        .await
        .expect("list workflow spec EventLedger rows");
    let event = events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == comfy_event_family::WORKFLOW_SPEC_REGISTERED
        })
        .expect("workflow spec registration must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["spec_version"],
        json!("1.0.0")
    );
}

#[tokio::test]
async fn mt110_version_metadata_pins_pose_image_comfy_versions() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt110_version_metadata_pins_pose_image_comfy_versions: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    // Link version metadata to a registered spec for spec-level provenance.
    let suffix = Uuid::new_v4().simple().to_string();
    let spec = store
        .register_workflow_spec(&new_spec(&suffix))
        .await
        .expect("register workflow spec for version metadata link");

    let workflow_run_id = Uuid::new_v4();
    let metadata: ComfyVersionMetadata = store
        .record_version_metadata(&NewComfyVersionMetadata {
            workflow_run_id,
            spec_id: Some(spec.spec_id),
            pose_model_asset_version: "pose-model@2.3.1".to_string(),
            image_tool_version: "image-tools@1.7.0".to_string(),
            comfy_model_version: "comfy-sdxl@0.9.4".to_string(),
            preflight_evidence: json!({
                "probe": "preflight",
                "discovered": { "pose": "2.3.1", "image_tool": "1.7.0", "comfy": "0.9.4" }
            }),
        })
        .await
        .expect("record version metadata");

    assert_eq!(metadata.workflow_run_id, workflow_run_id);
    assert_eq!(metadata.spec_id, Some(spec.spec_id));
    assert_eq!(metadata.pose_model_asset_version, "pose-model@2.3.1");
    assert_eq!(metadata.image_tool_version, "image-tools@1.7.0");
    assert_eq!(metadata.comfy_model_version, "comfy-sdxl@0.9.4");

    // Round-trips by workflow run.
    let reloaded = store
        .get_version_metadata(workflow_run_id)
        .await
        .expect("reload version metadata")
        .expect("version metadata exists");
    assert_eq!(reloaded, metadata);

    // Idempotent on workflow_run_id: re-pin refreshes versions in place.
    let repinned = store
        .record_version_metadata(&NewComfyVersionMetadata {
            workflow_run_id,
            spec_id: Some(spec.spec_id),
            pose_model_asset_version: "pose-model@2.4.0".to_string(),
            image_tool_version: "image-tools@1.7.0".to_string(),
            comfy_model_version: "comfy-sdxl@0.9.4".to_string(),
            preflight_evidence: json!({ "probe": "preflight-repin" }),
        })
        .await
        .expect("re-pin version metadata");
    assert_eq!(
        repinned.version_metadata_id, metadata.version_metadata_id,
        "re-pin keeps the same metadata row"
    );
    assert_eq!(repinned.pose_model_asset_version, "pose-model@2.4.0");

    // Empty pose version is rejected so provenance can never be half-pinned.
    let err = store
        .record_version_metadata(&NewComfyVersionMetadata {
            workflow_run_id: Uuid::new_v4(),
            spec_id: None,
            pose_model_asset_version: "  ".to_string(),
            image_tool_version: "image-tools@1.7.0".to_string(),
            comfy_model_version: "comfy-sdxl@0.9.4".to_string(),
            preflight_evidence: json!({}),
        })
        .await
        .expect_err("empty pose_model_asset_version must be rejected");
    assert!(
        err.to_string()
            .to_lowercase()
            .contains("pose_model_asset_version"),
        "rejection must name the offending version field: {err}"
    );

    // EventLedger evidence (MT-005 coverage): recording emits the canonical
    // family on the workflow_run_id aggregate.
    let events = database
        .list_kernel_events_for_aggregate(
            "atelier_comfy_version_metadata",
            &workflow_run_id.to_string(),
        )
        .await
        .expect("list version metadata EventLedger rows");
    let event = events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == comfy_event_family::VERSION_METADATA_RECORDED
        })
        .expect("version metadata must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["comfy_model_version"],
        json!("comfy-sdxl@0.9.4")
    );
}
