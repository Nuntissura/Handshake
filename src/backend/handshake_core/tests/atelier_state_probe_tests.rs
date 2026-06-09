//! WP-KERNEL-005 MT-138 state-probe catalog proof.
//!
//! The catalog is runtime state: it is persisted in PostgreSQL and emits a
//! canonical EventLedger row before model-driven visual inspection.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::AtelierStore;
use handshake_core::atelier::state_probe::{
    MODEL_WORKFLOW_STATE_PROBE_CATALOG_ID, StateProbeSurface, model_workflow_state_probe_catalog,
    state_probe_event_family,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{Database, postgres::PostgresDatabase};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashSet, sync::Arc};
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
async fn state_probe_catalog_records_required_surfaces_before_visual_inspection() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP state_probe_catalog_records_required_surfaces_before_visual_inspection: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let mut catalog = model_workflow_state_probe_catalog(format!("test-run-{}", Uuid::new_v4()));
    catalog.catalog_id = format!("{MODEL_WORKFLOW_STATE_PROBE_CATALOG_ID}-{}", Uuid::new_v4());
    let recorded = store
        .record_state_probe_catalog(&catalog)
        .await
        .expect("record model-workflow state probe catalog");

    let expected_surfaces = [
        StateProbeSurface::Character,
        StateProbeSurface::Media,
        StateProbeSurface::Intake,
        StateProbeSurface::Collection,
        StateProbeSurface::Docs,
        StateProbeSurface::Moodboard,
        StateProbeSurface::Pose,
        StateProbeSurface::ComfyUiJob,
        StateProbeSurface::Session,
        StateProbeSurface::Errors,
    ];
    let expected_surface_tokens = expected_surfaces
        .iter()
        .map(|surface| surface.as_token())
        .collect::<HashSet<_>>();
    let recorded_surface_tokens = recorded
        .entries
        .iter()
        .map(|entry| entry.surface.as_token())
        .collect::<HashSet<_>>();
    assert_eq!(
        recorded_surface_tokens, expected_surface_tokens,
        "MT-138 catalog must cover exactly the required model-workflow probe surfaces"
    );
    assert_eq!(recorded.entries.len(), expected_surfaces.len());

    for entry in &recorded.entries {
        assert_eq!(
            entry.probe_id,
            format!("MT-138.{}", expected_probe_suffix(entry.surface)),
            "{} must use the exact MT-138 probe id",
            entry.surface.as_token()
        );
        assert!(
            entry.required_before_visual_inspection,
            "{} must be required before visual inspection",
            entry.probe_id
        );
        assert_eq!(
            entry.inspection_phase, "pre_visual_inspection",
            "{} must be explicitly pre-visual-inspection",
            entry.probe_id
        );
        assert_eq!(entry.status, "ready");
        assert_eq!(entry.read_model, "postgres_event_ledger_projection");
        assert_eq!(
            entry.probe_fields["schema"],
            serde_json::json!("hsk.atelier.state_probe.fields@1")
        );
        assert_eq!(
            entry.probe_fields["state_authority"],
            serde_json::json!("postgres")
        );
        assert_eq!(
            entry.probe_fields["event_authority"],
            serde_json::json!("kernel_event_ledger")
        );
        assert!(
            entry
                .probe_fields
                .get("fields")
                .and_then(|value| value.as_array())
                .is_some_and(|fields| !fields.is_empty()),
            "{} must expose structured probe fields for no-context models",
            entry.probe_id
        );
        assert!(
            entry
                .evidence_refs
                .iter()
                .all(|reference| !reference.contains(".GOV")),
            "{} must cite product evidence, not governance paperwork",
            entry.probe_id
        );
    }

    let reloaded = store
        .get_state_probe_catalog(&catalog.catalog_id)
        .await
        .expect("reload model-workflow state probe catalog");
    assert_eq!(reloaded.entries.len(), expected_surfaces.len());

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_state_probe_catalog", &catalog.catalog_id)
        .await
        .expect("list state probe catalog kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == state_probe_event_family::STATE_PROBE_CATALOG_RECORDED
                && event.payload["atelier_payload"]["surface_count"]
                    == serde_json::json!(expected_surfaces.len())
                && event.payload["atelier_payload"]["required_before_visual_inspection_count"]
                    == serde_json::json!(expected_surfaces.len())
        }),
        "recording the state-probe catalog must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn state_probe_catalog_rejects_missing_required_surface() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP state_probe_catalog_rejects_missing_required_surface: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

    let mut catalog = model_workflow_state_probe_catalog(format!("test-run-{}", Uuid::new_v4()));
    catalog.catalog_id = format!("bad-state-probe-catalog-{}", Uuid::new_v4());
    catalog
        .entries
        .retain(|entry| entry.surface != StateProbeSurface::Errors);

    let err = store
        .record_state_probe_catalog(&catalog)
        .await
        .expect_err("state-probe catalog must reject missing required surfaces");
    assert!(
        err.to_string().contains("errors"),
        "missing-surface rejection must name the absent surface: {err}"
    );
    assert!(
        store
            .get_state_probe_catalog(&catalog.catalog_id)
            .await
            .is_err(),
        "invalid state-probe catalog must not persist partial rows"
    );
}

#[tokio::test]
async fn state_probe_catalog_rejects_bogus_ready_catalog_metadata() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP state_probe_catalog_rejects_bogus_ready_catalog_metadata: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

    let mut catalog = model_workflow_state_probe_catalog(format!("test-run-{}", Uuid::new_v4()));
    catalog.catalog_id = format!("bogus-state-probe-catalog-{}", Uuid::new_v4());
    for (index, entry) in catalog.entries.iter_mut().enumerate() {
        entry.probe_id = format!("paper-only-probe-{index}");
        entry.read_model = "spreadsheet_projection".to_string();
        entry.probe_fields["state_authority"] = serde_json::json!("markdown");
        entry.probe_fields["event_authority"] = serde_json::json!("none");
        entry.evidence_refs = vec!["src/backend/handshake_core/src/lib.rs".to_string()];
    }

    let err = store
        .record_state_probe_catalog(&catalog)
        .await
        .expect_err("bogus all-surface catalog must not be recorded as ready");
    assert!(
        err.to_string().contains("MT-138.character"),
        "bogus ready-catalog rejection must name the exact expected probe id: {err}"
    );
}

fn expected_probe_suffix(surface: StateProbeSurface) -> &'static str {
    match surface {
        StateProbeSurface::Character => "character",
        StateProbeSurface::Media => "media",
        StateProbeSurface::Intake => "intake",
        StateProbeSurface::Collection => "collection",
        StateProbeSurface::Docs => "docs",
        StateProbeSurface::Moodboard => "moodboard",
        StateProbeSurface::Pose => "pose",
        StateProbeSurface::ComfyUiJob => "comfyui-job",
        StateProbeSurface::Session => "session",
        StateProbeSurface::Errors => "errors",
    }
}
