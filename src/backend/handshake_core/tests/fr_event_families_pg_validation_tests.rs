//! WP-KERNEL-005 MT-196 proof — Flight Recorder event validation matrix vs
//! the real PostgreSQL EventLedger.
//!
//! The registry half (`FrEventRegistry::from_rust_enum()` vs the on-disk
//! manifest) is covered by `fr_event_registry_tests.rs`; the span-guard half
//! in-memory by `flight_recorder_span_tests.rs`. What the contract's
//! proof_target ("runtime proof vs PostgreSQL/EventLedger") demanded — and
//! v2 flagged as missing — is proven here:
//!
//!   * one event per Flight Recorder event family is emitted through the
//!     PRODUCTION `PostgresFrRecorder` (the batched mpsc -> flusher ->
//!     `kernel_event_ledger` pipeline), flushed, and RE-READ from
//!     Handshake-managed PostgreSQL;
//!   * every persisted `event_type` resolves back through
//!     `FrEventId::from_str_id` and has a validation row (id + kind +
//!     subsystem + schema fields) in the registry built from the Rust enum —
//!     i.e. validation rows exist for ALL families and for ONLY the families
//!     the runtime ledger actually carries;
//!   * the span context the events cite is itself durable: the parent
//!     `ModelSessionSpan` + `ActivitySpan` are persisted via the production
//!     `SpanRepo` and re-read from `kernel_model_session_span` /
//!     `kernel_activity_span`.
//!
//! Gated on `atelier_pg_support::database_url()` (Handshake-managed
//! PostgreSQL); prints SKIP when unavailable. Never SQLite.

mod atelier_pg_support;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::flight_recorder::fr_emitter::{
    FrRecorder, PostgresFrRecorder, PostgresFrRecorderConfig, SpanContextRef,
};
use handshake_core::flight_recorder::fr_event_registry::{FrEventId, FrEventRegistry};
use handshake_core::flight_recorder::span_repo::SpanRepo;
use handshake_core::flight_recorder::spans::{
    ActivityKind, ActivitySpan, AttributeValue, ModelSessionSpan, SpanId, SpanStatus,
};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn migrated_pool(url: &str) -> sqlx::PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    PostgresDatabase::new(pool.clone())
        .run_migrations()
        .await
        .expect("run kernel migrations (event ledger + observability spans)");
    pool
}

fn session_span() -> ModelSessionSpan {
    let mut attributes: BTreeMap<String, AttributeValue> = BTreeMap::new();
    attributes.insert(
        "scope".to_string(),
        AttributeValue::String("mt196_fr_event_families".to_string()),
    );
    ModelSessionSpan {
        span_id: SpanId::new_v7(),
        model_session_id: Uuid::now_v7(),
        session_id: Uuid::now_v7(),
        started_at_utc: Utc::now(),
        ended_at_utc: None,
        attributes,
        status: SpanStatus::Active,
    }
}

/// MT-196: every FR event family round-trips through the production
/// PostgresFrRecorder into `kernel_event_ledger`, and the registry's
/// validation rows cover exactly the families the ledger carries.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt196_all_fr_event_families_persist_to_event_ledger_and_match_validation_rows() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt196_fr_event_families: PostgreSQL unavailable");
        return;
    };
    let pool = migrated_pool(&url).await;

    // --- The span the FR events cite is durable via the production repo. --
    let repo = SpanRepo::new(pool.clone());
    let span = session_span();
    repo.insert_session_span(&span)
        .await
        .expect("insert session span");
    let activity = ActivitySpan {
        span_id: SpanId::new_v7(),
        parent_span_id: span.span_id,
        activity_kind: ActivityKind::ToolInvocation,
        started_at_utc: Utc::now(),
        ended_at_utc: None,
        attributes: BTreeMap::new(),
        status: SpanStatus::Active,
    };
    repo.insert_activity_span(&activity)
        .await
        .expect("insert activity span");

    let fetched_session = repo
        .get_session_span(span.span_id)
        .await
        .expect("re-read session span")
        .expect("session span persisted");
    assert_eq!(fetched_session.model_session_id, span.model_session_id);
    assert_eq!(fetched_session.status, "active");
    let fetched_activity = repo
        .get_activity_span(activity.span_id)
        .await
        .expect("re-read activity span")
        .expect("activity span persisted");
    assert_eq!(fetched_activity.parent_span_id, span.span_id);
    assert_eq!(fetched_activity.activity_kind, "tool_invocation");

    // --- Emit ONE event per family through the production recorder. -------
    let run_id = Uuid::now_v7().simple().to_string();
    let session_run_id = format!("mt196-session-{run_id}");
    let cfg = PostgresFrRecorderConfig {
        channel_capacity: 256,
        batch_size: 16,
        flush_interval: Duration::from_millis(25),
        kernel_task_run_id: format!("mt196-task-{run_id}"),
        session_run_id: session_run_id.clone(),
    };
    let mut recorder = PostgresFrRecorder::spawn(pool.clone(), cfg);
    let span_context =
        SpanContextRef::for_session_span(span.span_id, span.model_session_id, span.session_id);
    for event_id in FrEventId::all() {
        recorder
            .record(
                *event_id,
                serde_json::json!({
                    "mt": "MT-196",
                    "family": event_id.as_str(),
                    "subsystem": event_id.subsystem(),
                }),
                Some(span_context.clone()),
            )
            .await
            .unwrap_or_else(|err| panic!("queue {} into the recorder: {err:?}", event_id.as_str()));
    }
    // Deterministic flush: shutdown drains the channel into the ledger.
    recorder.shutdown().await;

    // --- RE-READ from PostgreSQL: one ledger row per family. --------------
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        r#"SELECT event_type, aggregate_type, aggregate_id
           FROM kernel_event_ledger
           WHERE session_run_id = $1
           ORDER BY event_type ASC"#,
    )
    .bind(&session_run_id)
    .fetch_all(&pool)
    .await
    .expect("re-read FR events from kernel_event_ledger");

    let all_families = FrEventId::all();
    assert_eq!(
        rows.len(),
        all_families.len(),
        "exactly one ledger row per FR event family must persist"
    );
    let persisted: BTreeSet<&str> = rows.iter().map(|(event_type, _, _)| event_type.as_str()).collect();
    let expected: BTreeSet<&str> = all_families.iter().map(|id| id.as_str()).collect();
    assert_eq!(
        persisted, expected,
        "the persisted event_type set must be ALL families and ONLY the FR families"
    );
    for (event_type, aggregate_type, aggregate_id) in &rows {
        FrEventId::from_str_id(event_type)
            .unwrap_or_else(|err| panic!("ledger event_type {event_type} must resolve: {err:?}"));
        assert_eq!(
            aggregate_type, "flight_recorder_span",
            "FR events aggregate on the span, got {aggregate_type} for {event_type}"
        );
        assert_eq!(
            aggregate_id,
            &span.span_id.as_uuid().to_string(),
            "FR events must cite the persisted session span"
        );
    }

    // --- Validation rows: registry covers exactly the persisted families. -
    let registry = FrEventRegistry::from_rust_enum();
    assert_eq!(
        registry.events.len(),
        all_families.len(),
        "the registry must carry one validation row per FR event family"
    );
    for (event_type, _, _) in &rows {
        let row = registry
            .events
            .iter()
            .find(|entry| entry.id == *event_type)
            .unwrap_or_else(|| {
                panic!("persisted family {event_type} must have a registry validation row")
            });
        assert!(
            !row.kind.trim().is_empty(),
            "{event_type} validation row must declare a kind"
        );
        assert!(
            !row.subsystem.trim().is_empty(),
            "{event_type} validation row must declare a subsystem"
        );
    }
    let registry_ids: BTreeSet<&str> = registry
        .events
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(
        registry_ids, persisted,
        "validation rows exist for ALL FR event families and ONLY for them"
    );

    // --- Close the spans on PG so the cited context is complete. ----------
    repo.update_activity_span_end(activity.span_id, Utc::now(), &SpanStatus::Completed)
        .await
        .expect("end activity span");
    repo.update_session_span_end(span.span_id, Utc::now(), &SpanStatus::Completed, None)
        .await
        .expect("end session span");
    let ended = repo
        .get_session_span(span.span_id)
        .await
        .expect("re-read ended session span")
        .expect("ended session span present");
    assert_eq!(ended.status, "completed");
    assert!(ended.ended_at_utc.is_some());
}
