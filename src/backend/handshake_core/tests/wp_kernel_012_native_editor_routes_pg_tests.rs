//! WP-KERNEL-012 MT-045 native-editor backend routes — route-level integration
//! proofs against REAL Handshake-managed PostgreSQL.
//!
//! These drive the actual Axum routes over a loopback listener (quiet: no
//! foreground window, no focus steal). They are `#[ignore = "requires_pg"]`:
//! the live round-trip is deferred to the live-PG batch. Each body is REAL — it
//! seeds authority, calls the route, and asserts the contract — never a stub or
//! panic placeholder.
//!
//! Coverage:
//!   * Route 3  GET  /workspaces/:ws/calendar/events           (calendar window)
//!   * Route 1  GET  /workspaces/:ws/locus/work-packets/:id     (locus resolve)
//!   * Route 1  GET  /workspaces/:ws/locus/microtasks/:id       (locus resolve)
//!   * Route 6  DELETE /knowledge/documents/:id                 (soft delete)
//!   * MT-067   POST+GET /workspaces/:ws/calendar/activity-spans (span round-trip)
//!   * MT-067   GET  /workspaces/:ws/calendar/events            (daily_note_doc_id link)
//!   * MT-045   POST /workspaces/:ws/code-nav/index             (code-nav index pipeline)

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use handshake_core::api::{
    calendar as calendar_api, code_nav_index as code_nav_index_api,
    knowledge_documents as docs_api, locus as locus_api,
};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{
    CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert, CalendarEventVisibility,
    CalendarSourceProviderType, CalendarSourceSyncState, CalendarSourceUpsert,
    CalendarSourceWritePolicy, Database, WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::{json, Value};
use sqlx::Connection;

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }
    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
    async fn get_diagnostic(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound("diagnostic"))
    }
    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }
    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

/// Build an AppState over the isolated schema and return it plus the storage arc
/// (for seeding authority directly).
async fn app_state(pg: &KnowledgePg) -> AppState {
    let storage = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect AppState pool");
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("wpk012-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

/// Serve the given router over loopback and return its base URL.
async fn serve(app: axum::Router) -> (String, reqwest::Client) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("wpk012 test server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

// ---------------------------------------------------------------------------
// Route 3 — calendar events window query.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn route3_calendar_events_returns_events_in_window() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP route3_calendar_events: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    state
        .storage
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: "cal-src-wpk012".to_string(),
                workspace_id: workspace_id.clone(),
                display_name: "WPK012 Test Calendar".to_string(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::ReadOnlyImport,
                default_tzid: "UTC".to_string(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await
        .expect("seed calendar source");

    state
        .storage
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: "cal-evt-wpk012".to_string(),
                workspace_id: workspace_id.clone(),
                source_id: "cal-src-wpk012".to_string(),
                external_id: None,
                external_etag: None,
                title: "Sprint review".to_string(),
                description: None,
                location: None,
                start_ts_utc: Utc.with_ymd_and_hms(2026, 7, 1, 9, 0, 0).unwrap(),
                end_ts_utc: Utc.with_ymd_and_hms(2026, 7, 1, 10, 0, 0).unwrap(),
                start_local: None,
                end_local: None,
                tzid: "UTC".to_string(),
                all_day: false,
                was_floating: false,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::FullExport,
                rrule: None,
                rdate: vec![],
                exdate: vec![],
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: None,
                attendees: json!([]),
                links: json!([]),
                provider_payload: None,
            },
        )
        .await
        .expect("seed calendar event");

    let (base, http) = serve(calendar_api::routes(state.clone())).await;
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/events?from=2026-07-01&to=2026-07-01"
        ))
        .send()
        .await
        .expect("events request");
    assert_eq!(resp.status(), 200, "calendar events GET must succeed");
    let events: Value = resp.json().await.expect("events json");
    let arr = events.as_array().expect("events array");
    assert!(
        arr.iter().any(|e| e["id"] == "cal-evt-wpk012"
            && e["title"] == "Sprint review"
            && e["all_day"] == false
            && e["daily_note_doc_id"].is_null()),
        "seeded event must appear with mapped wire shape: {events}"
    );

    // A bad window (end <= start) is a 400, not a 500.
    let bad = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/events?from=2026-07-02&to=2026-07-01"
        ))
        .send()
        .await
        .expect("bad window request");
    assert_eq!(bad.status(), 400, "inverted window must be a 400");
}

// ---------------------------------------------------------------------------
// Route 1 — Locus resolve (work packet + microtask).
// ---------------------------------------------------------------------------

async fn seed_work_packet(pg: &KnowledgePg, wp_id: &str, title: &str) {
    let mut conn = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect to seed work packet");
    sqlx::query(
        r#"
        INSERT INTO work_packets
            (wp_id, version, title, description, status, priority, task_board_status,
             reporter, created_at, updated_at, vector_clock, metadata)
        VALUES ($1, 1, $2, $3, 'READY', 1, 'READY', 'tester', 'now', 'now', '{}', '{}')
        "#,
    )
    .bind(wp_id)
    .bind(title)
    .bind("A native-editor backend work packet.")
    .execute(&mut conn)
    .await
    .expect("insert work_packets row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn route1_locus_work_packet_resolve() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP route1_locus_work_packet_resolve: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    seed_work_packet(&pg, "WP-KERNEL-999", "Native Editors WP").await;

    let (base, http) = serve(locus_api::routes(state.clone())).await;
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/locus/work-packets/WP-KERNEL-999"
        ))
        .send()
        .await
        .expect("wp resolve request");
    assert_eq!(resp.status(), 200, "wp resolve must succeed");
    let record: Value = resp.json().await.expect("wp json");
    assert_eq!(record["title"], "Native Editors WP");
    assert_eq!(record["status"], "READY");
    assert_eq!(record["summary"], "A native-editor backend work packet.");

    // A missing id is a 404, not a 500.
    let missing = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/locus/work-packets/WP-DOES-NOT-EXIST"
        ))
        .send()
        .await
        .expect("missing wp request");
    assert_eq!(missing.status(), 404, "missing wp must be a 404");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn route1_locus_micro_task_resolve() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP route1_locus_micro_task_resolve: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    seed_work_packet(&pg, "WP-KERNEL-998", "Parent WP").await;

    let mut conn = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect to seed micro task");
    sqlx::query(
        r#"
        INSERT INTO micro_tasks (mt_id, wp_id, name, status, metadata)
        VALUES ($1, $2, $3, 'IN_PROGRESS', '{}')
        "#,
    )
    .bind("MT-777")
    .bind("WP-KERNEL-998")
    .bind("Wire calendar route")
    .execute(&mut conn)
    .await
    .expect("insert micro_tasks row");

    let (base, http) = serve(locus_api::routes(state.clone())).await;
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/locus/microtasks/MT-777"
        ))
        .send()
        .await
        .expect("mt resolve request");
    assert_eq!(resp.status(), 200, "mt resolve must succeed");
    let record: Value = resp.json().await.expect("mt json");
    assert_eq!(record["title"], "Wire calendar route");
    assert_eq!(record["status"], "IN_PROGRESS");
}

// ---------------------------------------------------------------------------
// Route 6 — RichDocument soft delete (tombstone + receipt + stale source).
// ---------------------------------------------------------------------------

fn operator_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("wpk012-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-WPK012-{label}"))
        .header("x-hsk-session-run-id", format!("SR-WPK012-{label}"))
        .header("x-hsk-actor-kind", "operator")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn route6_document_soft_delete_tombstones_and_receipts() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP route6_document_soft_delete: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let (base, http) = serve(docs_api::routes(state.clone())).await;

    // Create a document as the operator.
    let created: Value = operator_headers(http.post(format!("{base}/knowledge/documents")), "create")
        .json(&json!({
            "workspace_id": workspace_id,
            "title": "Doomed Doc",
            "content_json": {
                "type": "doc",
                "content": [
                    { "type": "paragraph", "content": [{ "type": "text", "text": "bye" }] }
                ]
            }
        }))
        .send()
        .await
        .expect("create doc")
        .json()
        .await
        .expect("create json");
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("rich_document_id")
        .to_string();

    // An unauthenticated DELETE (no actor kind) is denied (403), never a delete.
    let denied = http
        .delete(format!("{base}/knowledge/documents/{doc_id}"))
        .header("x-hsk-actor-id", "anon")
        .header("x-hsk-kernel-task-run-id", "KTR-anon")
        .header("x-hsk-session-run-id", "SR-anon")
        .send()
        .await
        .expect("denied delete request");
    assert_eq!(denied.status(), 403, "unauthenticated delete must be denied");

    // Operator DELETE soft-deletes and returns the receipt.
    let deleted: Value = operator_headers(
        http.delete(format!("{base}/knowledge/documents/{doc_id}")),
        "delete",
    )
    .send()
    .await
    .expect("delete request")
    .json()
    .await
    .expect("delete json");
    assert_eq!(deleted["deleted"], true);
    let receipt = deleted["deleted_receipt_event_id"]
        .as_str()
        .expect("receipt id");
    assert!(receipt.starts_with("KE-"), "receipt id is a kernel event: {receipt}");

    // The tombstone landed on the authority row (deleted_at set, row not dropped).
    let mut conn = sqlx::PgConnection::connect(&pg.schema_url)
        .await
        .expect("connect to verify tombstone");
    let (deleted_at, tomb_receipt): (Option<chrono::DateTime<Utc>>, Option<String>) =
        sqlx::query_as(
            "SELECT deleted_at, deleted_receipt_event_id FROM knowledge_rich_documents WHERE rich_document_id = $1",
        )
        .bind(&doc_id)
        .fetch_one(&mut conn)
        .await
        .expect("read tombstone");
    assert!(deleted_at.is_some(), "deleted_at must be set");
    assert_eq!(tomb_receipt.as_deref(), Some(receipt));

    // The delete left a KNOWLEDGE_RICH_DOCUMENT_DELETED EventLedger receipt.
    let events = state
        .storage
        .list_kernel_events_for_aggregate("knowledge_rich_document", &doc_id)
        .await
        .expect("read ledger");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == handshake_core::kernel::KernelEventType::KnowledgeRichDocumentDeleted),
        "a delete receipt must be appended to the EventLedger"
    );
}

// ---------------------------------------------------------------------------
// MT-067 — calendar activity spans (create + query round-trip) and the
// daily-note linkage on the events response.
// ---------------------------------------------------------------------------

/// Seed a calendar source + one event for `event_id` starting at `start`.
async fn seed_calendar_event(
    state: &AppState,
    workspace_id: &str,
    event_id: &str,
    start: chrono::DateTime<Utc>,
) {
    let ctx = WriteContext::human(None);
    state
        .storage
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: format!("cal-src-{event_id}"),
                workspace_id: workspace_id.to_string(),
                display_name: "MT067 Calendar".to_string(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::ReadOnlyImport,
                default_tzid: "UTC".to_string(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await
        .expect("seed calendar source");
    state
        .storage
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: event_id.to_string(),
                workspace_id: workspace_id.to_string(),
                source_id: format!("cal-src-{event_id}"),
                external_id: None,
                external_etag: None,
                title: "Edit block".to_string(),
                description: None,
                location: None,
                start_ts_utc: start,
                end_ts_utc: start + chrono::Duration::hours(1),
                start_local: None,
                end_local: None,
                tzid: "UTC".to_string(),
                all_day: false,
                was_floating: false,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::FullExport,
                rrule: None,
                rdate: vec![],
                exdate: vec![],
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: None,
                attendees: json!([]),
                links: json!([]),
                provider_payload: None,
            },
        )
        .await
        .expect("seed calendar event");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt067_activity_span_create_and_query_round_trip() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt067_activity_span: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let start = Utc.with_ymd_and_hms(2026, 7, 3, 9, 0, 0).unwrap();
    seed_calendar_event(&state, &workspace_id, "cal-evt-mt067", start).await;

    let (base, http) = serve(calendar_api::routes(state.clone())).await;

    // POST an activity span recording an edit block during the event.
    let created: Value = http
        .post(format!(
            "{base}/workspaces/{workspace_id}/calendar/activity-spans"
        ))
        .json(&json!({
            "calendar_event_id": "cal-evt-mt067",
            "started_utc": "2026-07-03T09:05:00Z",
            "ended_utc": "2026-07-03T09:45:00Z",
            "edited_doc_ids": ["DOC-A", "DOC-B"]
        }))
        .send()
        .await
        .expect("create span request")
        .json()
        .await
        .expect("create span json");
    assert_eq!(created["calendar_event_id"], "cal-evt-mt067");
    assert_eq!(created["edited_doc_ids"], json!(["DOC-A", "DOC-B"]));
    let span_id = created["span_id"].as_str().expect("span id").to_string();
    assert!(!span_id.is_empty(), "a span id must be minted");

    // GET the activity spans for the event and assert the round-trip.
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/activity-spans?event_id=cal-evt-mt067"
        ))
        .send()
        .await
        .expect("list spans request");
    assert_eq!(resp.status(), 200, "activity-spans GET must succeed");
    let spans: Value = resp.json().await.expect("spans json");
    let arr = spans.as_array().expect("spans array");
    assert!(
        arr.iter().any(|s| s["span_id"] == span_id.as_str()
            && s["calendar_event_id"] == "cal-evt-mt067"
            && s["edited_doc_ids"] == json!(["DOC-A", "DOC-B"])
            && s["ended_utc"].is_string()),
        "the created span must be returned with its edited_doc_ids: {spans}"
    );

    // A missing event_id is a 400, not a 500.
    let bad = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/activity-spans?event_id="
        ))
        .send()
        .await
        .expect("bad event id request");
    assert_eq!(bad.status(), 400, "empty event_id must be a 400");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt067_calendar_event_populates_daily_note_doc_id() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt067_daily_note: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let start = Utc.with_ymd_and_hms(2026, 7, 4, 9, 0, 0).unwrap();
    seed_calendar_event(&state, &workspace_id, "cal-evt-dn", start).await;

    // Seed the daily journal LoomBlock for the event's date so the linkage
    // resolves (the MT-019 / MT-257 get-or-create).
    let ctx = WriteContext::human(None);
    let block = state
        .storage
        .get_or_create_daily_journal_block(&ctx, &workspace_id, "2026-07-04")
        .await
        .expect("seed daily journal block");
    let expected = block
        .document_id
        .clone()
        .unwrap_or_else(|| block.block_id.clone());

    let (base, http) = serve(calendar_api::routes(state.clone())).await;
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/events?from=2026-07-04&to=2026-07-04"
        ))
        .send()
        .await
        .expect("events request");
    assert_eq!(resp.status(), 200, "events GET must succeed");
    let events: Value = resp.json().await.expect("events json");
    let arr = events.as_array().expect("events array");
    assert!(
        arr.iter()
            .any(|e| e["id"] == "cal-evt-dn" && e["daily_note_doc_id"] == expected.as_str()),
        "an event on a journalled date must carry daily_note_doc_id={expected}: {events}"
    );
}

// ---------------------------------------------------------------------------
// MT-045 (LC-06) — the code-nav index pipeline over a small real code dir.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt045_code_nav_index_returns_symbol_count() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt045_code_nav_index: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;

    // A small real code dir (mirrors LC-06's fixture, tiny): 3 Rust files, each
    // with 5 free functions => 15 real symbols the AST indexer must produce.
    let dir = std::env::temp_dir().join(format!("wpk012-mt045-{}", uuid::Uuid::now_v7().simple()));
    std::fs::create_dir_all(&dir).expect("create temp code dir");
    for f in 0..3usize {
        let mut body = String::new();
        for i in 0..5usize {
            body.push_str(&format!("fn file{f}_sym{i}() -> u32 {{ {i} }}\n"));
        }
        std::fs::write(dir.join(format!("file_{f}.rs")), body).expect("write code file");
    }

    let (base, http) = serve(code_nav_index_api::routes(state.clone())).await;
    let resp = operator_headers(
        http.post(format!("{base}/workspaces/{workspace_id}/code-nav/index")),
        "index",
    )
    .json(&json!({ "root_path": dir.to_string_lossy() }))
    .send()
    .await
    .expect("index request");
    assert_eq!(resp.status(), 200, "code-nav index must succeed");
    let out: Value = resp.json().await.expect("index json");
    let symbol_count = out["symbol_count"].as_u64().expect("symbol_count");
    assert!(
        symbol_count > 0,
        "the pipeline must return a real symbol count (got {symbol_count}): {out}"
    );
    assert!(
        out["files_indexed"].as_u64().unwrap_or(0) >= 3,
        "all 3 code files must index: {out}"
    );

    let _ = std::fs::remove_dir_all(&dir);

    // Missing identity headers -> 400 (the mutation attribution law).
    let denied = http
        .post(format!("{base}/workspaces/{workspace_id}/code-nav/index"))
        .json(&json!({ "root_path": "unused" }))
        .send()
        .await
        .expect("unauth index request");
    assert_eq!(
        denied.status(),
        400,
        "an index without identity headers must be rejected"
    );
}
