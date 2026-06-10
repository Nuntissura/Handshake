//! WP-KERNEL-005 MT-141 (Diagnostic Bundle Schema) live PostgreSQL
//! round-trip proof for the kernel diagnostic-bundle manifest schema/builder
//! (`diagnostics::bundle_manifest`).
//!
//! No mocks: the test records a manifest with REAL data spanning all contract
//! fields (subject, failure summary, error taxonomy, severity, evidence
//! sections, reproduction steps, isolation hints) through the real
//! `AtelierStore` into live PostgreSQL (table from migration 0120, applied by
//! the kernel migration runner), RE-READS it by manifest id and by subject,
//! and asserts the canonical `kernel.diagnostics.bundle_manifest_recorded`
//! EventLedger family. Negative paths prove the portable-ref boundary: .GOV /
//! SQLite refs and manifests without reproduction steps are rejected and
//! nothing persists.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::AtelierStore;
use handshake_core::diagnostics::bundle_manifest::{
    kernel_diagnostic_bundle_event_family, DiagnosticBundleSection, DiagnosticBundleSectionKind,
    NewDiagnosticBundleManifest, KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA,
};
use handshake_core::diagnostics::DiagnosticSeverity;
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

fn failing_job_manifest(subject_ref: &str) -> NewDiagnosticBundleManifest {
    NewDiagnosticBundleManifest {
        subject_kind: "kernel_job".to_string(),
        subject_ref: subject_ref.to_string(),
        failure_summary: "kernel job failed while applying the workflow transition".to_string(),
        error_taxonomy: "kernel.workflow_transition_failed".to_string(),
        severity: DiagnosticSeverity::Error,
        created_by: "mt141-proof".to_string(),
        sections: vec![
            DiagnosticBundleSection {
                section_id: "diagnostics".to_string(),
                kind: DiagnosticBundleSectionKind::Diagnostics,
                title: "Open diagnostics for the failing job".to_string(),
                content_ref: None,
                content_json: json!({
                    "fingerprints": ["fp-workflow-transition-1"],
                    "severity_counts": { "error": 1, "warning": 2 },
                }),
                item_count: 3,
            },
            DiagnosticBundleSection {
                section_id: "event-ledger".to_string(),
                kind: DiagnosticBundleSectionKind::EventLedger,
                title: "EventLedger aggregates to replay".to_string(),
                content_ref: None,
                content_json: json!({
                    "aggregates": [
                        { "aggregate_type": "kernel_task_run", "aggregate_id": subject_ref },
                    ],
                }),
                item_count: 1,
            },
            DiagnosticBundleSection {
                section_id: "logs".to_string(),
                kind: DiagnosticBundleSectionKind::Logs,
                title: "Failure logs".to_string(),
                content_ref: Some(format!("artifact://logs/{subject_ref}")),
                content_json: json!({ "log_lines": 412 }),
                item_count: 412,
            },
            DiagnosticBundleSection {
                section_id: "environment".to_string(),
                kind: DiagnosticBundleSectionKind::Environment,
                title: "Pinned versions at failure".to_string(),
                content_ref: None,
                content_json: json!({
                    "handshake_core": "wp-kernel-005",
                    "postgres": "16",
                }),
                item_count: 2,
            },
        ],
        reproduction_steps: vec![
            "load the captured request from the logs section content_ref".to_string(),
            "re-run the workflow transition with the pinned environment versions".to_string(),
        ],
        isolation_hints: vec![
            "check the diagnostics section fingerprints first".to_string(),
            "replay the event-ledger aggregates before touching the job".to_string(),
        ],
    }
}

#[tokio::test]
async fn mt141_kernel_diagnostic_bundle_manifest_round_trips_with_event_ledger() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt141_kernel_diagnostic_bundle_manifest_round_trips_with_event_ledger: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let subject_ref = format!("kernel-job://run/{}", Uuid::new_v4());
    let new = failing_job_manifest(&subject_ref);

    let manifest = store
        .record_kernel_diagnostic_bundle_manifest(&new)
        .await
        .expect("record kernel diagnostic bundle manifest");

    // Field fidelity on the returned record.
    assert_eq!(manifest.schema_id, KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA);
    assert_eq!(manifest.subject_kind, "kernel_job");
    assert_eq!(manifest.subject_ref, subject_ref);
    assert_eq!(
        manifest.error_taxonomy,
        "kernel.workflow_transition_failed"
    );
    assert_eq!(manifest.severity, DiagnosticSeverity::Error);
    assert_eq!(manifest.sections.len(), 4);
    let logs_section = manifest
        .sections
        .iter()
        .find(|section| section.section_id == "logs")
        .expect("logs section persists");
    assert_eq!(logs_section.kind, DiagnosticBundleSectionKind::Logs);
    assert_eq!(
        logs_section.content_ref.as_deref(),
        Some(format!("artifact://logs/{subject_ref}").as_str())
    );
    assert_eq!(logs_section.item_count, 412);
    assert_eq!(manifest.reproduction_steps.len(), 2);
    assert_eq!(manifest.isolation_hints.len(), 2);

    // RE-READ from PostgreSQL by manifest id: full fidelity.
    let reloaded = store
        .get_kernel_diagnostic_bundle_manifest(manifest.manifest_id)
        .await
        .expect("reload kernel diagnostic bundle manifest")
        .expect("kernel diagnostic bundle manifest exists");
    assert_eq!(reloaded, manifest);
    let reloaded_diagnostics = reloaded
        .sections
        .iter()
        .find(|section| section.section_id == "diagnostics")
        .expect("diagnostics section round-trips");
    assert_eq!(
        reloaded_diagnostics.content_json["severity_counts"]["warning"],
        json!(2)
    );

    // RE-READ by failing subject: a no-context model can isolate the failure
    // from the subject token alone.
    let by_subject = store
        .list_kernel_diagnostic_bundle_manifests_for_subject("kernel_job", &subject_ref)
        .await
        .expect("list manifests for failing subject");
    assert_eq!(by_subject.len(), 1);
    assert_eq!(by_subject[0].manifest_id, manifest.manifest_id);

    // A second manifest for the same subject lists newest-first.
    let mut follow_up = failing_job_manifest(&subject_ref);
    follow_up.failure_summary = "kernel job failed again after retry".to_string();
    let second = store
        .record_kernel_diagnostic_bundle_manifest(&follow_up)
        .await
        .expect("record follow-up manifest");
    let by_subject = store
        .list_kernel_diagnostic_bundle_manifests_for_subject("kernel_job", &subject_ref)
        .await
        .expect("list manifests for failing subject after retry");
    assert_eq!(by_subject.len(), 2);
    assert_eq!(
        by_subject[0].manifest_id, second.manifest_id,
        "subject listing must be newest-first"
    );

    // EventLedger proof on the manifest aggregate.
    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "kernel_diagnostic_bundle_manifest",
            &manifest.manifest_id.to_string(),
        )
        .await
        .expect("list kernel diagnostic bundle manifest events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED
                && event.payload["atelier_payload"]["schema"]
                    == json!(KERNEL_DIAGNOSTIC_BUNDLE_MANIFEST_SCHEMA)
                && event.payload["atelier_payload"]["subject_ref"] == json!(subject_ref)
                && event.payload["atelier_payload"]["section_count"] == json!(4)
                && event.payload["atelier_payload"]["error_taxonomy"]
                    == json!("kernel.workflow_transition_failed")
        }),
        "recording the manifest must emit canonical EventLedger evidence"
    );
    let second_events = database
        .list_kernel_events_for_aggregate(
            "kernel_diagnostic_bundle_manifest",
            &second.manifest_id.to_string(),
        )
        .await
        .expect("list follow-up manifest events");
    assert!(
        second_events.iter().any(|event| {
            event.payload["event_family"]
                == kernel_diagnostic_bundle_event_family::BUNDLE_MANIFEST_RECORDED
        }),
        "follow-up manifest must emit its own EventLedger aggregate"
    );
}

#[tokio::test]
async fn mt141_kernel_diagnostic_bundle_manifest_rejects_nonportable_and_incomplete_input() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt141_kernel_diagnostic_bundle_manifest_rejects_nonportable_and_incomplete_input: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, _database) = connected_store_with_ledger(&url).await;

    let subject_ref = format!("kernel-job://run/{}", Uuid::new_v4());

    // .GOV content_ref must be rejected before anything persists.
    let mut gov_ref = failing_job_manifest(&subject_ref);
    gov_ref.sections[2].content_ref = Some(".GOV/task_packets/WP-KERNEL-005/MT-141.json".into());
    let err = store
        .record_kernel_diagnostic_bundle_manifest(&gov_ref)
        .await
        .expect_err(".GOV refs must not persist in a diagnostic bundle manifest");
    assert!(
        err.to_string().contains("content_ref"),
        "rejection must name the offending field: {err}"
    );

    // SQLite refs inside inline section content must be rejected.
    let mut sqlite_inline = failing_job_manifest(&subject_ref);
    sqlite_inline.sections[1].content_json =
        json!({ "store_ref": "sqlite://machine-local/replay.db" });
    store
        .record_kernel_diagnostic_bundle_manifest(&sqlite_inline)
        .await
        .expect_err("SQLite refs must not persist inside section content");

    // A manifest a no-context model cannot act on (no reproduction steps, no
    // sections) must be rejected.
    let mut no_steps = failing_job_manifest(&subject_ref);
    no_steps.reproduction_steps.clear();
    store
        .record_kernel_diagnostic_bundle_manifest(&no_steps)
        .await
        .expect_err("manifests without reproduction steps must be rejected");
    let mut no_sections = failing_job_manifest(&subject_ref);
    no_sections.sections.clear();
    store
        .record_kernel_diagnostic_bundle_manifest(&no_sections)
        .await
        .expect_err("manifests without evidence sections must be rejected");

    // Nothing persisted for the rejected manifests.
    let by_subject = store
        .list_kernel_diagnostic_bundle_manifests_for_subject("kernel_job", &subject_ref)
        .await
        .expect("list manifests after rejections");
    assert!(
        by_subject.is_empty(),
        "rejected manifests must not leave rows behind"
    );
}
