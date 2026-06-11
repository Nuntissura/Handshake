//! WP-KERNEL-005 MT-165: live PostgreSQL proof for the production
//! [`PostgresTraceSource`] + [`export_persisted_trace`] trace-export path.
//!
//! Integration validation v2 failed MT-165 because the (real) redaction and
//! tarball logic was only ever proven against in-memory `TraceSource` stubs:
//! no trace bundle was ever persisted to or re-read from a managed resource,
//! and no artifact bytes ever came from the real ArtifactStore. These tests
//! close that gap:
//!
//!  - trace bundles are persisted as `kernel_event_ledger` rows (aggregate
//!    [`MEMORY_TRACE_AGGREGATE_TYPE`]) through Handshake-managed PostgreSQL
//!    and RE-READ through a FRESH [`PostgresTraceSource`] instance, so no
//!    in-memory state can satisfy the assertions;
//!  - the tarball export loads the referenced artifact payload from the real
//!    on-disk ArtifactStore (hash-validated) and the redaction interplay is
//!    asserted on the exported bytes (secrets present in the persisted
//!    bundle/artifact never reach the export);
//!  - the EventLedger evidence (persisted bundle event + export receipt
//!    event) is asserted directly on the `Database` ledger reader.
//!
//! `PostgresTraceSource` bridges sync traits over async storage, so every
//! test runs on a multi-thread runtime. Gated on
//! `atelier_pg_support::database_url()`: when no PostgreSQL is available the
//! test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use std::sync::Arc;

use atelier_pg_support::{database_url, write_native_media_artifact_from_stored_payload};
use chrono::{DateTime, Utc};
use handshake_core::memory::progressive_retrieval::DegradationReport;
use handshake_core::memory::retrieval_mode::RetrievalMode;
use handshake_core::memory::trace_export::{
    ArtifactRef, CacheMarkers, ExportFormat, MemoryPackBudgets, QueryPlan, RedactionPolicy,
    RetrievalTrace, RouteDecision, ScoringInputSnapshot, SpanRef, TraceBundle, TraceSource,
    TRACE_EXPORT_VERSION,
};
use handshake_core::memory::{
    export_persisted_trace, DegradationTier, PostgresTraceSource, MEMORY_TRACE_AGGREGATE_TYPE,
    MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID, MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID,
    MEMORY_TRACE_SOURCE_COMPONENT,
};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// Raw secret planted in the PERSISTED query — must never reach an export.
const QUERY_SECRET: &str = "password=hunter2-mt165";
/// Raw secret planted in the PERSISTED span text.
const SPAN_SECRET: &str = "api_key=span-secret-mt165";
/// Raw secret planted in the real ArtifactStore payload bytes.
const ARTIFACT_SECRET: &str = "token:tok-mt165-artifact-secret";
/// PII planted in both the query and the span.
const PII_EMAIL: &str = "user@example.com";

async fn connected_database(url: &str) -> Arc<dyn Database> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool);
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    database.into_arc()
}

/// Build a trace bundle carrying secrets + PII in every redactable surface
/// and one referenced artifact that lives in the REAL ArtifactStore.
fn secret_bearing_bundle(trace_id: Uuid, artifact_id: &str, artifact_len: u64) -> TraceBundle {
    TraceBundle {
        trace_id,
        query_plan: QueryPlan {
            plan_id: Uuid::now_v7(),
            query: format!("ship support bundle for {PII_EMAIL} {QUERY_SECRET}"),
            task_type: "general".to_string(),
        },
        retrieval_trace: RetrievalTrace {
            trace_id,
            query_plan_id: Uuid::now_v7(),
            item_ids: vec![Uuid::now_v7().to_string()],
        },
        budgets: MemoryPackBudgets {
            max_items: 2,
            max_bytes: 4096,
        },
        cache_markers: CacheMarkers {
            cache_hit_count: 0,
            cache_miss_count: 1,
        },
        retrieval_mode: RetrievalMode::Hybrid,
        route_decision: RouteDecision {
            matched_rule_id: "general_freeform_query".to_string(),
            rationale: "general freeform query uses hybrid retrieval".to_string(),
        },
        degradation_report: DegradationReport {
            tiers_completed: vec!["full_text".to_string()],
            tiers_skipped: Vec::new(),
            load_signal_at_start: 0.0,
            started_at_utc: Utc::now(),
            total_duration_ms: 0,
            tier_chosen: DegradationTier::Tiered,
        },
        selected_spans: vec![SpanRef {
            span_id: "span-1".to_string(),
            text: format!("contact {PII_EMAIL} {SPAN_SECRET}"),
        }],
        referenced_artifacts: vec![ArtifactRef {
            // packet.json is in the DEFAULT allowlist, so this artifact
            // survives the allowlist filter and its REAL bytes must pass
            // through the redactor before tarball embedding.
            artifact_id: artifact_id.to_string(),
            kind: "packet.json".to_string(),
            bytes_estimate: artifact_len,
        }],
        scoring_inputs: vec![ScoringInputSnapshot {
            item_id: "span-1".to_string(),
            importance: 0.5,
            recency: 0.5,
            trust: 0.5,
            outcome_tuned_weight: 0.5,
            embedding_similarity: 0.5,
            formula_version: "injection_scoring_formula_v1".to_string(),
        }],
        redactions_applied: Vec::new(),
        exported_at_utc: DateTime::<Utc>::MIN_UTC,
        exporter_version: TRACE_EXPORT_VERSION.to_string(),
    }
}

/// MT-165 primary proof: persist the trace bundle to PostgreSQL, re-read it
/// through a FRESH source, export a tarball whose artifact bytes come from
/// the real ArtifactStore, assert the redaction interplay on the exported
/// bytes, and assert both EventLedger events (bundle + export receipt).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt165_trace_bundle_round_trips_and_redacts_real_artifacts_over_postgres() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt165_trace_bundle_round_trips_and_redacts_real_artifacts_over_postgres: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    // Real ArtifactStore payload (UTF-8 so the redactor can scan it) carrying
    // a secret that MUST NOT survive into the exported tarball.
    let artifact_payload = format!("MT-165 packet evidence {ARTIFACT_SECRET} end\n");
    let artifact = write_native_media_artifact_from_stored_payload(artifact_payload.as_bytes());
    let artifact_trace_id = format!("L1/{}", artifact.artifact_id);

    let trace_id = Uuid::now_v7();
    let bundle = secret_bearing_bundle(trace_id, &artifact_trace_id, artifact.byte_len as u64);

    // Persist through the production source (kernel_event_ledger row).
    let source = PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root.clone());
    source.persist_trace(&bundle).expect("persist trace bundle");

    // RE-READ through a FRESH source instance: the bundle must come back from
    // PostgreSQL, byte-equal, with the raw (unredacted) values intact —
    // redaction is an export-time concern, not a storage mutation.
    let fresh_source =
        PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root.clone());
    let reloaded = fresh_source
        .load_trace(trace_id)
        .expect("re-read persisted trace bundle from PostgreSQL");
    assert_eq!(
        reloaded, bundle,
        "persisted bundle must round-trip PostgreSQL byte-equal"
    );
    assert!(
        reloaded.query_plan.query.contains(QUERY_SECRET),
        "storage must keep the raw query; redaction happens at export time"
    );

    // Production export entry point: PostgreSQL load -> redact -> tarball
    // (artifact bytes loaded + hash-validated from the real ArtifactStore)
    // -> ledger receipt.
    let exported = export_persisted_trace(
        Arc::clone(&database),
        &artifact.workspace_root,
        trace_id,
        &RedactionPolicy::default(),
        ExportFormat::Tarball,
    )
    .expect("export persisted trace as tarball");

    let tar_text = String::from_utf8_lossy(&exported.bytes);
    assert!(
        !tar_text.contains("hunter2-mt165"),
        "query credential must not survive into the export"
    );
    assert!(
        !tar_text.contains("span-secret-mt165"),
        "span credential must not survive into the export"
    );
    assert!(
        !tar_text.contains("tok-mt165-artifact-secret"),
        "REAL ArtifactStore payload secret must be redacted before tarball embedding"
    );
    assert!(
        !tar_text.contains(PII_EMAIL),
        "PII email must not survive into the export"
    );
    assert!(
        tar_text.contains("[REDACTED-SECRET]"),
        "credential redaction placeholder must appear in the exported bytes"
    );
    assert!(
        tar_text.contains("[REDACTED-EMAIL]"),
        "PII redaction placeholder must appear in the exported bytes"
    );
    // The artifact-level redaction record must be embedded in the manifest.
    assert!(
        tar_text.contains(&format!("artifacts.{artifact_trace_id}")),
        "artifact redaction record must be embedded in the exported manifest"
    );

    // EventLedger proof: bundle event + export receipt on the trace aggregate.
    let events = database
        .list_kernel_events_for_aggregate(MEMORY_TRACE_AGGREGATE_TYPE, &trace_id.to_string())
        .await
        .expect("list trace ledger events");
    let bundle_event = events
        .iter()
        .find(|event| event.payload["schema_id"] == json!(MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID))
        .expect("persisted trace bundle event must exist in kernel_event_ledger");
    assert_eq!(
        bundle_event.source_component,
        MEMORY_TRACE_SOURCE_COMPONENT
    );
    assert_eq!(
        bundle_event.payload["bundle"]["trace_id"],
        json!(trace_id.to_string())
    );
    let receipt_event = events
        .iter()
        .find(|event| {
            event.payload["schema_id"] == json!(MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID)
        })
        .expect("export receipt event must exist in kernel_event_ledger");
    assert_eq!(
        receipt_event.payload["content_hash"],
        json!(exported.content_hash),
        "the auditable receipt must carry the exported bundle's content hash"
    );
    assert_eq!(receipt_event.payload["format"], json!("tarball"));
    assert_eq!(
        receipt_event.payload["exported_bytes"],
        json!(exported.bytes.len())
    );
}

/// MT-165 durability semantics: re-persisting the identical bundle collapses
/// onto the existing ledger row (idempotent per content hash), while a
/// changed bundle appends a new row and the FRESH re-read returns the LATEST
/// persisted version.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt165_persist_trace_is_idempotent_and_rereads_latest_from_postgres() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt165_persist_trace_is_idempotent_and_rereads_latest_from_postgres: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    let artifact = write_native_media_artifact_from_stored_payload(
        b"MT-165 idempotency packet, no secrets here\n",
    );
    let artifact_trace_id = format!("L1/{}", artifact.artifact_id);

    let trace_id = Uuid::now_v7();
    let bundle = secret_bearing_bundle(trace_id, &artifact_trace_id, artifact.byte_len as u64);

    let source = PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root.clone());
    source.persist_trace(&bundle).expect("first persist");
    source
        .persist_trace(&bundle)
        .expect("identical re-persist must collapse onto the existing row");

    let bundle_event_count = database
        .list_kernel_events_for_aggregate(MEMORY_TRACE_AGGREGATE_TYPE, &trace_id.to_string())
        .await
        .expect("list trace ledger events")
        .iter()
        .filter(|event| {
            event.payload["schema_id"] == json!(MEMORY_TRACE_BUNDLE_PAYLOAD_SCHEMA_ID)
        })
        .count();
    assert_eq!(
        bundle_event_count, 1,
        "identical re-persist must not duplicate the bundle ledger row"
    );

    // A genuinely revised bundle (same trace_id, new content) appends a new
    // row; the authoritative re-read is the latest event by sequence.
    let mut revised = bundle.clone();
    revised.query_plan.query = "revised mt-165 query without secrets".to_string();
    source.persist_trace(&revised).expect("persist revision");

    let fresh_source = PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root);
    let reloaded = fresh_source
        .load_trace(trace_id)
        .expect("re-read revised trace bundle");
    assert_eq!(
        reloaded.query_plan.query, revised.query_plan.query,
        "fresh re-read must return the LATEST persisted bundle version"
    );
}
