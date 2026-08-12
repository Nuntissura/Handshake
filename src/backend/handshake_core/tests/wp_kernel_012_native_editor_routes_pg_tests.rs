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
//!   * MT-066   POST+GET /workspaces/:ws/stage/artifacts        (stage capture provenance)

mod knowledge_pg_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{NaiveDate, TimeZone, Utc};
use handshake_core::api::{
    calendar as calendar_api, code_nav_index as code_nav_index_api,
    knowledge_documents as docs_api, locus as locus_api, stage as stage_api,
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
    CalendarSourceWritePolicy, WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{base_database_url, knowledge_pg, KnowledgePg};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Connection;

static STAGE_BINDING_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Default)]
struct NoopRecorder;

#[derive(Default)]
struct FailSecondRecordOnceRecorder {
    attempts: AtomicUsize,
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[derive(Default)]
struct CollectingRecorder {
    attempts: AtomicUsize,
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait]
impl FlightRecorder for CollectingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| filter.event_id.is_none_or(|id| event.event_id == id))
            .cloned()
            .collect())
    }
}

#[async_trait]
impl FlightRecorder for FailSecondRecordOnceRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(RecorderError::SinkError(
                "intentional Stage second-flight-record fail-once".to_owned(),
            ));
        }
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| filter.event_id.is_none_or(|id| event.event_id == id))
            .cloned()
            .collect())
    }
}

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
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
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
    app_state_with_recorder(pg, Arc::new(NoopRecorder)).await
}

async fn app_state_with_recorder(pg: &KnowledgePg, recorder: Arc<dyn FlightRecorder>) -> AppState {
    let storage = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect AppState storage")
        .into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect AppState pool");
    let diagnostics = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder,
        diagnostics,
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
        axum::serve(listener, app)
            .await
            .expect("wpk012 test server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

struct StageUpgradeSchemaGuard {
    database_url: String,
    schema: String,
    armed: bool,
}

impl StageUpgradeSchemaGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StageUpgradeSchemaGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let database_url = self.database_url.clone();
        let schema = self.schema.clone();
        let cleanup = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async move {
                let mut conn = sqlx::PgConnection::connect(&database_url)
                    .await
                    .map_err(|error| error.to_string())?;
                sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
                    .execute(&mut conn)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })
        })
        .join()
        .unwrap_or_else(|_| Err("legacy Stage schema cleanup thread panicked".to_owned()));
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("legacy Stage schema panic cleanup failed: {error}");
            } else {
                panic!("legacy Stage schema cleanup failed: {error}");
            }
        }
    }
}

/// Upgrade proof for the exact 0341 -> 0346 -> 0348 legacy Stage path. The
/// 0341 digest describes compact canonical JSON while 0346 can recover only
/// PostgreSQL `jsonb::text` bytes; 0348 must realign every byte-derived field
/// before current native fetch consumes the row.
#[tokio::test]
#[ignore = "requires_pg"]
async fn stage_0348_repairs_legacy_recovered_byte_integrity() {
    let Some(database_url) = base_database_url().await else {
        eprintln!("SKIP stage_0348_repairs_legacy_recovered_byte_integrity: no PostgreSQL");
        return;
    };
    let schema = format!("stage_upgrade_{}", uuid::Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(&database_url)
        .await
        .expect("connect legacy Stage upgrade proof");
    sqlx::query("SELECT pg_advisory_lock(hashtextextended('mt066-stage-upgrade-schema-proof', 0))")
        .execute(&mut conn)
        .await
        .expect("serialize legacy Stage upgrade schemas");
    sqlx::raw_sql(
        r#"
        DO $cleanup$
        DECLARE stale RECORD;
        BEGIN
            FOR stale IN
                SELECT nspname FROM pg_namespace WHERE nspname LIKE 'stage_upgrade_%'
            LOOP
                EXECUTE format('DROP SCHEMA %I CASCADE', stale.nspname);
            END LOOP;
        END
        $cleanup$;
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("remove stale legacy Stage upgrade schemas");
    let preexisting: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_namespace WHERE nspname LIKE 'stage_upgrade_%'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("assert no preexisting legacy Stage upgrade schema");
    assert_eq!(preexisting, 0);
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .execute(&mut conn)
        .await
        .expect("ensure pgcrypto for Stage upgrade proof");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create legacy Stage upgrade schema");
    let mut schema_guard = StageUpgradeSchemaGuard {
        database_url: database_url.clone(),
        schema: schema.clone(),
        armed: true,
    };
    sqlx::query(&format!("SET search_path TO {schema}, public"))
        .execute(&mut conn)
        .await
        .expect("select legacy Stage upgrade schema");
    sqlx::raw_sql(
        r#"
        CREATE TABLE workspaces (id TEXT PRIMARY KEY);
        CREATE TABLE ai_jobs (id TEXT PRIMARY KEY);
        CREATE TABLE kernel_event_ledger (event_id TEXT PRIMARY KEY);
        CREATE TABLE loom_canvas_placements (
            placement_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            canvas_block_id TEXT NOT NULL
        );
        INSERT INTO workspaces (id) VALUES ('ws-legacy-stage');
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("seed pre-0341 prerequisite shape");
    sqlx::raw_sql(include_str!(
        "../migrations/0341_stage_capture_artifacts.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply actual 0341 Stage migration");

    let compact_json = br#"{"text":"legacy"}"#;
    let legacy_sha = format!("{:x}", Sha256::digest(compact_json));
    sqlx::query(
        r#"
        INSERT INTO stage_capture_artifacts (
            artifact_id, workspace_id, content_kind, label, content_type,
            content_json, content_sha256, manifest, manifest_ref
        ) VALUES (
            'STGA-00000000000000000000000000000001',
            'ws-legacy-stage', 'selection', 'legacy', 'application/json',
            '{"text":"legacy"}'::jsonb, $1,
            jsonb_build_object('sha256', $1, 'size_bytes', $2::bigint),
            'manifest://STGA-00000000000000000000000000000001'
        )
        "#,
    )
    .bind(&legacy_sha)
    .bind(compact_json.len() as i64)
    .execute(&mut conn)
    .await
    .expect("seed 0341-shaped legacy row");

    sqlx::raw_sql(include_str!(
        "../migrations/0346_stage_capture_runtime_contract.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply actual 0346 Stage recovery migration");
    let (recovered_sha, stored_sha): (String, String) = sqlx::query_as(
        r#"
        SELECT encode(digest(content_bytes, 'sha256'), 'hex'), content_sha256
        FROM stage_capture_artifacts
        WHERE artifact_id = 'STGA-00000000000000000000000000000001'
        "#,
    )
    .fetch_one(&mut conn)
    .await
    .expect("read 0346 legacy mismatch");
    assert_ne!(
        recovered_sha, stored_sha,
        "proof seed must reproduce 0346's recovered-byte/hash mismatch"
    );

    sqlx::raw_sql(include_str!(
        "../migrations/0348_stage_capture_integrity_and_canvas_provenance.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply actual 0348 Stage integrity repair");
    let (computed_sha, stored_sha, manifest_sha, computed_size, stored_size, manifest_size): (
        String,
        String,
        String,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT
            encode(digest(content_bytes, 'sha256'), 'hex'),
            content_sha256,
            manifest->>'sha256',
            octet_length(content_bytes)::bigint,
            size_bytes,
            (manifest->>'size_bytes')::bigint
        FROM stage_capture_artifacts
        WHERE artifact_id = 'STGA-00000000000000000000000000000001'
        "#,
    )
    .fetch_one(&mut conn)
    .await
    .expect("read repaired legacy integrity tuple");
    assert_eq!(stored_sha, computed_sha);
    assert_eq!(manifest_sha, computed_sha);
    assert_eq!(stored_size, computed_size);
    assert_eq!(manifest_size, computed_size);

    sqlx::query("SET search_path TO public")
        .execute(&mut conn)
        .await
        .expect("leave legacy Stage upgrade schema");
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&mut conn)
        .await
        .expect("drop legacy Stage upgrade schema");
    schema_guard.disarm();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_namespace WHERE nspname LIKE 'stage_upgrade_%'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("assert no legacy Stage upgrade schema residue");
    assert_eq!(remaining, 0);
    sqlx::query(
        "SELECT pg_advisory_unlock(hashtextextended('mt066-stage-upgrade-schema-proof', 0))",
    )
    .execute(&mut conn)
    .await
    .expect("release legacy Stage upgrade schema lock");
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
                start_local: Some("2026-07-01T09:00:00".to_owned()),
                end_local: Some("2026-07-01T10:00:00".to_owned()),
                tzid: "UTC".to_string(),
                all_day: false,
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
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

    state
        .storage
        .upsert_calendar_event(
            &ctx,
            CalendarEventUpsert {
                id: "cal-evt-wpk012-all-day".to_string(),
                workspace_id: workspace_id.clone(),
                source_id: "cal-src-wpk012".to_string(),
                external_id: None,
                external_etag: None,
                title: "All-day release window".to_string(),
                description: None,
                location: None,
                start_ts_utc: Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
                end_ts_utc: Utc.with_ymd_and_hms(2026, 7, 3, 0, 0, 0).unwrap(),
                start_local: None,
                end_local: None,
                tzid: "UTC".to_string(),
                all_day: true,
                start_date: NaiveDate::from_ymd_opt(2026, 7, 1),
                end_date_exclusive: NaiveDate::from_ymd_opt(2026, 7, 3),
                was_floating: false,
                normalization_note: None,
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
        .expect("seed canonical all-day event");

    sqlx::query(
        r#"
        INSERT INTO calendar_events (
            id, workspace_id, source_id, title, start_ts_utc, end_ts_utc,
            tzid, all_day, temporal_contract_version
        ) VALUES ($1, $2, $3, $4, $5, $6, 'UTC', FALSE, NULL)
        "#,
    )
    .bind("cal-evt-wpk012-legacy")
    .bind(&workspace_id)
    .bind("cal-src-wpk012")
    .bind("Historic incomplete event")
    .bind(Utc.with_ymd_and_hms(2026, 7, 1, 11, 0, 0).unwrap())
    .bind(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap())
    .execute(&state.postgres_pool)
    .await
    .expect("seed explicit legacy temporal row");

    let (base, http) = serve(calendar_api::routes(state.clone())).await;
    let resp = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/events?from_date=2026-07-01&to_date_exclusive=2026-07-02&from_utc=2026-07-01T00:00:00Z&to_utc=2026-07-02T00:00:00Z&view_tzid=UTC"
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
            && e["temporal"]["kind"] == "timed"
            && e["temporal"]["start_local"] == "2026-07-01T09:00:00"
            && e["temporal"]["tzid"] == "UTC"
            && e["daily_note_doc_id"].is_null()),
        "seeded event must appear with mapped wire shape: {events}"
    );
    assert!(
        arr.iter().any(|e| {
            e["id"] == "cal-evt-wpk012-all-day"
                && e["temporal"]["kind"] == "all_day"
                && e["temporal"]["start_date"] == "2026-07-01"
                && e["temporal"]["end_date_exclusive"] == "2026-07-03"
        }),
        "all-day overlap must use canonical date boundaries: {events}"
    );
    assert!(
        arr.iter().any(|e| {
            e["id"] == "cal-evt-wpk012-legacy"
                && e["temporal"]["kind"] == "legacy_incomplete"
                && e["temporal"]["recovery"] == "reimport_from_calendar_source"
        }),
        "an incomplete legacy row remains listable with typed recovery: {events}"
    );

    // A bad window (end <= start) is a 400, not a 500.
    let bad = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/events?from_date=2026-07-02&to_date_exclusive=2026-07-01&from_utc=2026-07-02T00:00:00Z&to_utc=2026-07-01T00:00:00Z&view_tzid=UTC"
        ))
        .send()
        .await
        .expect("bad window request");
    assert_eq!(bad.status(), 400, "inverted window must be a 400");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt067_storage_boundary_rejects_invalid_temporal_rows_without_authority_residue() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt067 temporal rejection proof: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let ctx = WriteContext::human(Some("mt067-temporal-rejection".to_owned()));

    let invalid_source = state
        .storage
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: "cal-src-invalid-tz".to_owned(),
                workspace_id: workspace_id.clone(),
                display_name: "Invalid timezone source".to_owned(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::ReadOnlyImport,
                default_tzid: "Europe/Not-A-Zone".to_owned(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await;
    assert!(
        invalid_source.is_err(),
        "invalid source IANA tzid must reject"
    );

    state
        .storage
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: "cal-src-temporal-reject".to_owned(),
                workspace_id: workspace_id.clone(),
                display_name: "Temporal rejection source".to_owned(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::ReadOnlyImport,
                default_tzid: "Europe/Brussels".to_owned(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await
        .expect("seed valid source");

    let candidate =
        |id: &str, tzid: &str, start_local: &str, end_local: &str| CalendarEventUpsert {
            id: id.to_owned(),
            workspace_id: workspace_id.clone(),
            source_id: "cal-src-temporal-reject".to_owned(),
            external_id: None,
            external_etag: None,
            title: "Rejected temporal event".to_owned(),
            description: None,
            location: None,
            start_ts_utc: Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap(),
            end_ts_utc: Utc.with_ymd_and_hms(2026, 3, 29, 2, 30, 0).unwrap(),
            start_local: Some(start_local.to_owned()),
            end_local: Some(end_local.to_owned()),
            tzid: tzid.to_owned(),
            all_day: false,
            start_date: None,
            end_date_exclusive: None,
            was_floating: false,
            normalization_note: None,
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
        };

    for event in [
        candidate(
            "cal-evt-invalid-tz",
            "Europe/Not-A-Zone",
            "2026-03-29T01:30:00",
            "2026-03-29T02:30:00",
        ),
        candidate(
            "cal-evt-contradiction",
            "UTC",
            "2026-03-29T10:30:00",
            "2026-03-29T11:30:00",
        ),
        candidate(
            "cal-evt-gap",
            "Europe/Brussels",
            "2026-03-29T02:30:00",
            "2026-03-29T04:30:00",
        ),
    ] {
        let event_id = event.id.clone();
        assert!(
            state
                .storage
                .upsert_calendar_event(&ctx, event)
                .await
                .is_err(),
            "{event_id} must reject"
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM calendar_events WHERE id = $1")
            .bind(&event_id)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("count rejected event rows");
        let receipts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'calendar_event' AND aggregate_id = $1",
        )
        .bind(&event_id)
        .fetch_one(&state.postgres_pool)
        .await
        .expect("count rejected event receipts");
        let outbox: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_mutation_outbox WHERE calendar_event_id = $1",
        )
        .bind(&event_id)
        .fetch_one(&state.postgres_pool)
        .await
        .expect("count rejected event outbox");
        assert_eq!((rows, receipts, outbox), (0, 0, 0));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn activity_span_write_is_event_and_workspace_scoped() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP activity_span_write_is_event_and_workspace_scoped: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_a = pg.create_workspace().await;
    let workspace_b = pg.create_workspace().await;
    let ctx = WriteContext::human(None);

    for (workspace_id, source_id, event_id) in [
        (&workspace_a, "cal-src-span-a", "cal-evt-span-a"),
        (&workspace_a, "cal-src-span-a", "cal-evt-span-a-second"),
        (&workspace_b, "cal-src-span-b", "cal-evt-span-b"),
    ] {
        state
            .storage
            .upsert_calendar_source(
                &ctx,
                CalendarSourceUpsert {
                    id: source_id.to_owned(),
                    workspace_id: workspace_id.clone(),
                    display_name: format!("Activity span source {source_id}"),
                    provider_type: CalendarSourceProviderType::Local,
                    write_policy: CalendarSourceWritePolicy::ReadOnlyImport,
                    default_tzid: "UTC".to_owned(),
                    auto_export: false,
                    credentials_ref: None,
                    provider_calendar_id: None,
                    capability_profile_id: None,
                    config: json!({}),
                    sync_state: CalendarSourceSyncState::default(),
                },
            )
            .await
            .expect("seed activity-span calendar source");
        state
            .storage
            .upsert_calendar_event(
                &ctx,
                CalendarEventUpsert {
                    id: event_id.to_owned(),
                    workspace_id: workspace_id.clone(),
                    source_id: source_id.to_owned(),
                    external_id: None,
                    external_etag: None,
                    title: format!("Activity span event {event_id}"),
                    description: None,
                    location: None,
                    start_ts_utc: Utc.with_ymd_and_hms(2026, 7, 3, 9, 0, 0).unwrap(),
                    end_ts_utc: Utc.with_ymd_and_hms(2026, 7, 3, 10, 0, 0).unwrap(),
                    start_local: Some("2026-07-03T09:00:00".to_owned()),
                    end_local: Some("2026-07-03T10:00:00".to_owned()),
                    tzid: "UTC".to_owned(),
                    all_day: false,
                    start_date: None,
                    end_date_exclusive: None,
                    was_floating: false,
                    normalization_note: None,
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
            .expect("seed activity-span calendar event");
    }

    let (base, http) = serve(calendar_api::routes(state)).await;
    let path_a = format!("{base}/workspaces/{workspace_a}/calendar/activity-spans");
    let path_b = format!("{base}/workspaces/{workspace_b}/calendar/activity-spans");
    let missing = http
        .post(&path_a)
        .json(&json!({
            "span_id": "CAS-event-scope-missing",
            "calendar_event_id": "cal-evt-does-not-exist",
            "started_utc": "2026-07-03T09:05:00Z"
        }))
        .send()
        .await
        .expect("missing-event span request");
    assert_eq!(missing.status(), 404, "a span cannot name a missing event");

    let foreign_event = http
        .post(&path_b)
        .json(&json!({
            "span_id": "CAS-event-scope-foreign",
            "calendar_event_id": "cal-evt-span-a",
            "started_utc": "2026-07-03T09:05:00Z"
        }))
        .send()
        .await
        .expect("foreign-event span request");
    assert_eq!(
        foreign_event.status(),
        404,
        "a span cannot bind an event owned by another workspace"
    );

    let shared_span_id = "CAS-workspace-collision";
    let created = http
        .post(&path_a)
        .json(&json!({
            "span_id": shared_span_id,
            "calendar_event_id": "cal-evt-span-a",
            "started_utc": "2026-07-03T09:05:00Z",
            "edited_doc_ids": ["DOC-A"]
        }))
        .send()
        .await
        .expect("workspace-A span request");
    assert_eq!(created.status(), 201);

    let same_workspace_reassignment = http
        .post(&path_a)
        .json(&json!({
            "span_id": shared_span_id,
            "calendar_event_id": "cal-evt-span-a-second",
            "started_utc": "2026-07-03T11:05:00Z",
            "edited_doc_ids": ["DOC-B"]
        }))
        .send()
        .await
        .expect("same-workspace event reassignment request");
    assert_eq!(
        same_workspace_reassignment.status(),
        409,
        "a span id is immutable to its original event even inside one workspace"
    );

    let collision = http
        .post(&path_b)
        .json(&json!({
            "span_id": shared_span_id,
            "calendar_event_id": "cal-evt-span-b",
            "started_utc": "2026-07-03T11:05:00Z",
            "edited_doc_ids": ["DOC-B"]
        }))
        .send()
        .await
        .expect("cross-workspace collision request");
    assert_eq!(
        collision.status(),
        409,
        "a global span id owned by workspace A cannot be moved to workspace B"
    );

    let retained: Value = http
        .get(format!("{path_a}?event_id=cal-evt-span-a"))
        .send()
        .await
        .expect("workspace-A retained span request")
        .json()
        .await
        .expect("workspace-A retained span json");
    assert_eq!(retained[0]["span_id"], shared_span_id);
    assert_eq!(retained[0]["edited_doc_ids"], json!(["DOC-A"]));
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

struct StageBindingEnv {
    paths: Vec<std::path::PathBuf>,
    previous: Option<std::ffi::OsString>,
    token: String,
}

struct UnpublishedStageBindingFile {
    path: std::path::PathBuf,
    armed: bool,
}

impl UnpublishedStageBindingFile {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for UnpublishedStageBindingFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(unix)]
fn restrict_stage_binding_to_owner(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .unwrap_or_else(|error| panic!("restrict Stage binding {}: {error}", path.display()));
}

#[cfg(target_os = "windows")]
fn restrict_stage_binding_to_owner(path: &std::path::Path) {
    use std::os::windows::process::CommandExt as _;
    let user = std::env::var("USERNAME").expect("USERNAME for Stage binding ACL");
    let status = std::process::Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{user}:F"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .status()
        .unwrap_or_else(|error| panic!("run icacls for {}: {error}", path.display()));
    assert!(status.success(), "icacls rejected {}", path.display());
}

#[cfg(not(any(unix, target_os = "windows")))]
fn restrict_stage_binding_to_owner(_path: &std::path::Path) {
    panic!("owner-only Stage binding permissions unsupported on this platform");
}

#[cfg(windows)]
fn test_process_birth_identity(pid: u32) -> Option<Value> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    let handle = unsafe {
        OpenProcess(
            SYNCHRONIZE_RIGHT | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return None;
    }
    let live = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let queried = live
        && unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            != 0;
    unsafe {
        let _ = CloseHandle(handle);
    }
    queried.then(|| {
        json!({
            "kind": "windows",
            "creation_time_100ns": (u64::from(creation.dwHighDateTime) << 32)
                | u64::from(creation.dwLowDateTime),
        })
    })
}

#[cfg(target_os = "linux")]
fn test_process_birth_identity(pid: u32) -> Option<Value> {
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()?
        .trim()
        .to_owned();
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, tail) = stat.rsplit_once(") ")?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let state = fields.first()?.as_bytes().first().copied()?;
    if boot_id.is_empty() || matches!(state, b'Z' | b'X' | b'x') {
        return None;
    }
    Some(json!({
        "kind": "linux",
        "boot_id": boot_id,
        "start_time_ticks": fields.get(19)?.parse::<u64>().ok()?,
    }))
}

#[cfg(target_os = "macos")]
fn test_process_birth_identity(pid: u32) -> Option<Value> {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        pbi_reserved: u32,
        pbi_comm: [u8; 16],
        pbi_name: [u8; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    #[link(name = "proc")]
    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut c_void,
            buffer_size: i32,
        ) -> i32;
    }

    const PROC_PIDTBSDINFO: i32 = 3;
    const SZOMB: u32 = 5;
    const PROC_FLAG_INEXIT: u32 = 4;
    let mut info = ProcBsdInfo::default();
    let expected_size = std::mem::size_of::<ProcBsdInfo>();
    let queried = unsafe {
        proc_pidinfo(
            i32::try_from(pid).ok()?,
            PROC_PIDTBSDINFO,
            0,
            std::ptr::from_mut(&mut info).cast::<c_void>(),
            i32::try_from(expected_size).ok()?,
        )
    };
    if queried != i32::try_from(expected_size).ok()?
        || info.pbi_pid != pid
        || info.pbi_status == SZOMB
        || info.pbi_flags & PROC_FLAG_INEXIT != 0
        || info.pbi_start_tvsec == 0
        || info.pbi_start_tvusec >= 1_000_000
    {
        return None;
    }
    Some(json!({
        "kind": "mac_os",
        "start_time_seconds": info.pbi_start_tvsec,
        "start_time_microseconds": info.pbi_start_tvusec,
    }))
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn test_process_birth_identity(_pid: u32) -> Option<Value> {
    None
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))
))]
fn test_process_birth_identity(_pid: u32) -> Option<Value> {
    None
}

#[cfg(not(any(unix, windows)))]
fn test_process_birth_identity(_pid: u32) -> Option<Value> {
    None
}

fn mismatched_process_birth(mut identity: Value) -> Value {
    match identity["kind"].as_str() {
        Some("windows") => {
            let value = identity["creation_time_100ns"]
                .as_u64()
                .expect("Windows process creation time");
            identity["creation_time_100ns"] = Value::from(value.wrapping_add(1));
        }
        Some("linux") => {
            let value = identity["start_time_ticks"]
                .as_u64()
                .expect("Linux process start ticks");
            identity["start_time_ticks"] = Value::from(value.wrapping_add(1));
        }
        Some("mac_os") => {
            let value = identity["start_time_microseconds"]
                .as_u64()
                .expect("macOS process start microseconds");
            identity["start_time_microseconds"] = Value::from(value.wrapping_add(1));
        }
        kind => panic!("unsupported process birth identity in test: {kind:?}"),
    }
    identity
}

impl StageBindingEnv {
    fn write_binding_with_birth(token: &str, pid: u32, process_birth: Value) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "handshake-stage-binding-{}.json",
            uuid::Uuid::now_v7()
        ));
        let bytes = serde_json::to_vec(&json!({
            "tcp_addr": "127.0.0.1:1",
            "token": token,
            "pid": pid,
            "process_birth": process_birth,
        }))
        .expect("serialize Stage binding");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&path).expect("create Stage binding");
        let mut unpublished = UnpublishedStageBindingFile::new(path.clone());
        restrict_stage_binding_to_owner(&path);
        use std::io::Write as _;
        file.write_all(&bytes).expect("write Stage binding");
        file.sync_all().expect("sync Stage binding");
        drop(file);
        unpublished.disarm();
        path
    }

    fn write_binding(token: &str, pid: u32) -> std::path::PathBuf {
        let process_birth = test_process_birth_identity(pid)
            .expect("test binding process must have a verifiable live birth identity");
        Self::write_binding_with_birth(token, pid, process_birth)
    }

    fn install() -> Self {
        let token = hex::encode(Sha256::digest(uuid::Uuid::now_v7().as_bytes()));
        let path = Self::write_binding(&token, std::process::id());
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        Self {
            paths: vec![path],
            previous,
            token,
        }
    }

    fn headers(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header("x-hsk-session-token", &self.token)
    }

    fn set_pid(&mut self, pid: u32) {
        let path = Self::write_binding(&self.token, pid);
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        self.paths.push(path);
    }

    fn set_pid_with_birth(&mut self, pid: u32, process_birth: Value) {
        let path = Self::write_binding_with_birth(&self.token, pid, process_birth);
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        self.paths.push(path);
    }
}

impl Drop for StageBindingEnv {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", value),
            None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
        }
        for path in self.paths.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}

struct OwnedBindingProcess {
    child: Option<std::process::Child>,
    ready_path: std::path::PathBuf,
}

impl OwnedBindingProcess {
    fn spawn() -> Self {
        let ready_path = std::env::temp_dir().join(format!(
            "handshake-stage-binding-child-ready-{}-{}",
            std::process::id(),
            uuid::Uuid::now_v7()
        ));
        let mut command = std::process::Command::new(
            std::env::current_exe().expect("current backend route test executable"),
        );
        command
            .args([
                "--exact",
                "stage_binding_owned_subprocess_helper",
                "--nocapture",
            ])
            .env("HSK_STAGE_BINDING_OWNED_HELPER", "1")
            .env("HSK_STAGE_BINDING_HELPER_READY", &ready_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let child = command
            .spawn()
            .expect("spawn owned Stage binding subprocess");
        let mut owned = Self {
            child: Some(child),
            ready_path,
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !owned.ready_path.is_file() {
            assert!(
                owned
                    .child
                    .as_mut()
                    .expect("owned Stage binding child")
                    .try_wait()
                    .expect("poll owned Stage binding child")
                    .is_none(),
                "owned Stage binding subprocess exited before readiness"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "owned Stage binding subprocess did not become ready within ten seconds"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        owned
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("owned Stage binding child").id()
    }

    fn kill_and_wait(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child
                .try_wait()
                .expect("poll owned Stage binding child before kill")
                .is_none()
            {
                child.kill().expect("kill owned Stage binding child");
            }
            child.wait().expect("reap owned Stage binding child");
        }
        let _ = std::fs::remove_file(&self.ready_path);
    }
}

impl Drop for OwnedBindingProcess {
    fn drop(&mut self) {
        self.kill_and_wait();
    }
}

#[test]
fn stage_binding_owned_subprocess_helper() {
    if std::env::var("HSK_STAGE_BINDING_OWNED_HELPER").as_deref() != Ok("1") {
        return;
    }
    let ready_path = std::env::var_os("HSK_STAGE_BINDING_HELPER_READY")
        .map(std::path::PathBuf::from)
        .expect("owned Stage binding helper ready path");
    std::fs::write(&ready_path, std::process::id().to_string())
        .expect("publish owned Stage binding helper readiness");
    loop {
        std::thread::park_timeout(std::time::Duration::from_secs(1));
    }
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
    let created: Value =
        operator_headers(http.post(format!("{base}/knowledge/documents")), "create")
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
    assert_eq!(
        denied.status(),
        403,
        "unauthenticated delete must be denied"
    );

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
    assert!(
        receipt.starts_with("KE-"),
        "receipt id is a kernel event: {receipt}"
    );

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
        events.iter().any(|e| e.event_type
            == handshake_core::kernel::KernelEventType::KnowledgeRichDocumentDeleted),
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
                start_local: Some(start.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string()),
                end_local: Some(
                    (start + chrono::Duration::hours(1))
                        .naive_utc()
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                ),
                tzid: "UTC".to_string(),
                all_day: false,
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
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

    let open_span: Value = http
        .post(format!(
            "{base}/workspaces/{workspace_id}/calendar/activity-spans"
        ))
        .json(&json!({
            "span_id": "CAS-MT067-IN-PROGRESS",
            "calendar_event_id": "cal-evt-mt067",
            "started_utc": "2026-07-03T09:50:00Z",
            "ended_utc": null,
            "edited_doc_ids": ["DOC-C"]
        }))
        .send()
        .await
        .expect("create in-progress span request")
        .json()
        .await
        .expect("create in-progress span json");
    assert!(
        open_span["ended_utc"].is_null(),
        "open span must preserve NULL"
    );

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
    assert!(
        arr.iter().any(|s| {
            s["span_id"] == "CAS-MT067-IN-PROGRESS"
                && s["calendar_event_id"] == "cal-evt-mt067"
                && s["ended_utc"].is_null()
        }),
        "the open span must remain in-progress on GET: {spans}"
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
            "{base}/workspaces/{workspace_id}/calendar/events?from_date=2026-07-04&to_date_exclusive=2026-07-05&from_utc=2026-07-04T00:00:00Z&to_utc=2026-07-05T00:00:00Z&view_tzid=UTC"
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

// ---------------------------------------------------------------------------
// MT-066 — Stage capture artifacts (create + resolve round-trip). Proves the
// privileged exact-byte, idempotency, Job History, EventLedger, and denial
// contract through the real route and managed PostgreSQL authority.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn route2_stage_artifact_create_and_resolve() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP route2_stage_artifact: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let mut stage_binding = StageBindingEnv::install();

    let (base, http) = serve(stage_api::routes(state.clone())).await;

    let path = format!("{base}/workspaces/{workspace_id}/stage/artifacts");
    let exact_bytes = b"the quick brown fox\0caf\xC3\xA9\nline two";
    let expected_sha = hex::encode(Sha256::digest(exact_bytes));
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": "mt066-selection-1",
        "correlation_id": "mt066-correlation-1",
        "content_kind": "selection",
        "label": "Selected snippet",
        "content_type": "application/octet-stream",
        "content_base64": BASE64.encode(exact_bytes),
        "source_ref": "note://DOC-A#sel-1"
    });

    // Caller-asserted operator/system identity cannot invoke the privileged create operation. The
    // request lacks the server-validated native session token and is denied before workspace lookup.
    let denied = http
        .post(&path)
        .header("x-hsk-actor-id", "wpk012-stage-denied")
        .header("x-hsk-kernel-task-run-id", "KTR-WPK012-stage-denied")
        .header("x-hsk-session-run-id", "SR-WPK012-stage-denied")
        .header("x-hsk-actor-kind", "operator")
        .json(&request)
        .send()
        .await
        .expect("denied stage capture request");
    assert_eq!(denied.status(), 401);
    let forged_system = http
        .post(format!(
            "{base}/workspaces/definitely-absent/stage/artifacts"
        ))
        .header("x-hsk-actor-id", "forged-system")
        .header("x-hsk-actor-kind", "system")
        .json(&request)
        .send()
        .await
        .expect("forged system Stage request");
    assert_eq!(
        forged_system.status(),
        401,
        "invalid authentication is rejected before workspace existence can be observed"
    );

    let mut unknown_field = request.clone();
    unknown_field["unexpected"] = Value::Bool(true);
    let strict = stage_binding
        .headers(http.post(&path))
        .json(&unknown_field)
        .send()
        .await
        .expect("strict Stage DTO request");
    assert_eq!(strict.status(), 400, "unknown DTO fields are rejected");

    let mut oversized = request.clone();
    oversized["idempotency_key"] = Value::String("mt066-oversized".to_owned());
    oversized["content_base64"] =
        Value::String(BASE64.encode(vec![b'x'; stage_api::STAGE_CAPTURE_MAX_BYTES + 1]));
    let limited = stage_binding
        .headers(http.post(&path))
        .json(&oversized)
        .send()
        .await
        .expect("bounded Stage request");
    assert_eq!(limited.status(), 413, "capture bytes are strictly bounded");

    // POST a selection capture artifact with strict privileged identity.
    let created_response = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send()
        .await
        .expect("create stage artifact request");
    assert_eq!(created_response.status(), 201);
    let created: Value = created_response
        .json()
        .await
        .expect("create stage artifact json");

    let artifact_id = created["artifact_id"]
        .as_str()
        .expect("artifact_id")
        .to_string();
    assert!(
        artifact_id.starts_with("STGA-"),
        "artifact id is a stage capture id: {artifact_id}"
    );
    let created_sha = created["sha256"].as_str().expect("created sha256");
    assert_eq!(created_sha, expected_sha);
    assert!(
        created_sha
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "sha256 is lowercase hex: {created_sha}"
    );

    // GET it back and assert the evidence-grade contract holds.
    let unauthenticated_read = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/stage/artifacts/{artifact_id}"
        ))
        .send()
        .await
        .expect("unauthenticated Stage read");
    assert_eq!(unauthenticated_read.status(), 401);
    let unauthenticated_invalid_read = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/stage/artifacts/ART-00000000000000000000000000000000"
        ))
        .send()
        .await
        .expect("unauthenticated invalid-id Stage read");
    assert_eq!(unauthenticated_invalid_read.status(), 401);
    assert_eq!(
        unauthenticated_invalid_read
            .json::<Value>()
            .await
            .expect("unauthenticated invalid-id JSON")["error"],
        "HSK-401-STAGE-SESSION"
    );
    let resp = stage_binding
        .headers(http.get(format!(
            "{base}/workspaces/{workspace_id}/stage/artifacts/{artifact_id}"
        )))
        .send()
        .await
        .expect("get stage artifact request");
    assert_eq!(resp.status(), 200, "stage artifact GET must succeed");
    let fetched: Value = resp.json().await.expect("stage artifact json");

    // Evidence-grade twin of stage_interop::is_evidence_grade: BOTH the hoisted
    // sha256 AND the manifest.manifest_ref must be non-empty.
    let sha = fetched["sha256"].as_str().expect("fetched sha256");
    assert_eq!(
        sha.len(),
        64,
        "hoisted sha256 is 64-hex and non-empty: {fetched}"
    );
    let manifest_ref = fetched["manifest"]["manifest_ref"]
        .as_str()
        .expect("manifest_ref");
    assert!(
        !manifest_ref.trim().is_empty(),
        "manifest_ref must be non-empty (evidence-grade): {fetched}"
    );
    assert_eq!(
        manifest_ref,
        format!("manifest://{artifact_id}"),
        "manifest_ref is manifest://{{artifact_id}}: {fetched}"
    );
    // The manifest sha256 matches the hoisted one.
    assert_eq!(
        fetched["manifest"]["sha256"].as_str(),
        Some(sha),
        "manifest.sha256 matches the hoisted sha256: {fetched}"
    );
    // content_type round-trips through the manifest.
    assert_eq!(
        fetched["manifest"]["content_type"], "application/octet-stream",
        "manifest.content_type round-trips: {fetched}"
    );
    assert_eq!(fetched["artifact_id"], artifact_id.as_str());
    assert_eq!(fetched["workspace_id"], workspace_id.as_str());
    assert_eq!(fetched["label"], "Selected snippet");
    assert_eq!(fetched["size_bytes"], exact_bytes.len());
    assert_eq!(fetched["correlation_id"], "mt066-correlation-1");
    let job_id = fetched["job_id"].as_str().expect("Job History id");
    let event_id = fetched["event_ledger_event_id"]
        .as_str()
        .expect("ArtifactStored EventLedger id");
    let stored_approval: String = sqlx::query_scalar(
        "SELECT approval_id FROM stage_capture_artifacts WHERE artifact_id = $1",
    )
    .bind(&artifact_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("server-derived Stage approval");
    assert!(
        stored_approval.starts_with("native-mcp-stage:") && stored_approval.len() > 64,
        "approval lineage must be derived from the validated native binding, not accepted from the DTO"
    );

    // The content route returns the exact byte sequence, including NUL and UTF-8.
    let content = stage_binding
        .headers(http.get(format!(
            "{base}/workspaces/{workspace_id}/stage/artifacts/{artifact_id}/content"
        )))
        .send()
        .await
        .expect("stage artifact content request");
    assert_eq!(content.status(), 200);
    assert_eq!(
        content.bytes().await.expect("content bytes").as_ref(),
        exact_bytes
    );

    // Same key + same request is an idempotent replay with the same durable ids.
    let replay_response = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send()
        .await
        .expect("replay stage capture request");
    assert_eq!(replay_response.status(), 200);
    let replay: Value = replay_response.json().await.expect("replay response json");
    assert_eq!(replay["artifact_id"], artifact_id);
    assert_eq!(replay["job_id"], job_id);
    assert_eq!(replay["event_ledger_event_id"], event_id);
    assert_eq!(replay["replayed"], true);

    // Process restarts change the server-derived actor id but not the semantic capture request.
    // The replay must preserve the first actor and return the same durable artifact.
    let _restarted_native_process = OwnedBindingProcess::spawn();
    stage_binding.set_pid(_restarted_native_process.pid());
    let restarted_replay = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send()
        .await
        .expect("PID-changed semantic replay request");
    assert_eq!(restarted_replay.status(), 200);
    let restarted_replay: Value = restarted_replay
        .json()
        .await
        .expect("PID-changed replay JSON");
    assert_eq!(restarted_replay["artifact_id"], artifact_id);
    assert_eq!(restarted_replay["job_id"], job_id);
    let stored_actor: String =
        sqlx::query_scalar("SELECT actor_id FROM stage_capture_artifacts WHERE artifact_id = $1")
            .bind(&artifact_id)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("first Stage actor remains persisted");
    assert!(
        stored_actor.starts_with(&format!("handshake-native:{}:", std::process::id())),
        "persisted Stage actor includes the validated process birth fingerprint"
    );

    let mut concurrent_request = request.clone();
    concurrent_request["idempotency_key"] = Value::String("mt066-concurrent-1".to_owned());
    concurrent_request["correlation_id"] = Value::String("mt066-concurrent-correlation".to_owned());
    let first = stage_binding
        .headers(http.post(&path))
        .json(&concurrent_request)
        .send();
    let second = stage_binding
        .headers(http.post(&path))
        .json(&concurrent_request)
        .send();
    let (first, second) = tokio::join!(first, second);
    let first = first.expect("first concurrent capture response");
    let second = second.expect("second concurrent capture response");
    assert_eq!(
        [first.status().as_u16(), second.status().as_u16()]
            .into_iter()
            .filter(|status| *status == 201)
            .count(),
        1,
        "one concurrent request creates the artifact"
    );
    let first: Value = first.json().await.expect("first concurrent capture JSON");
    let second: Value = second.json().await.expect("second concurrent capture JSON");
    assert_eq!(first["artifact_id"], second["artifact_id"]);
    assert_eq!(first["job_id"], second["job_id"]);
    assert_eq!(
        first["event_ledger_event_id"],
        second["event_ledger_event_id"]
    );

    // Same key + changed bytes conflicts instead of silently overwriting.
    let mut changed = request.clone();
    changed["content_base64"] = Value::String(BASE64.encode(b"different"));
    let conflict = stage_binding
        .headers(http.post(&path))
        .json(&changed)
        .send()
        .await
        .expect("conflicting replay request");
    assert_eq!(conflict.status(), 409);

    let job_status: String = sqlx::query_scalar("SELECT status FROM ai_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&state.postgres_pool)
        .await
        .expect("Stage Job History row");
    assert_eq!(job_status, "completed");
    let artifact_event_type: String =
        sqlx::query_scalar("SELECT event_type FROM kernel_event_ledger WHERE event_id = $1")
            .bind(event_id)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("Stage ArtifactStored EventLedger row");
    assert_eq!(artifact_event_type, "ARTIFACT_STORED");
    let allow_decisions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = 'stage_capture_authorization' AND aggregate_id IN ($1, $2) AND payload->>'decision_outcome' = 'allow'",
    )
    .bind(&workspace_id)
    .bind(&artifact_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("Stage allow capability decision");
    assert_eq!(
        allow_decisions, 1,
        "the authenticated allow decision is durable exactly once; pre-auth denials stay in the redacted Flight Recorder path and do not disclose a workspace aggregate"
    );

    // A missing id is a 404, not a 500.
    let missing = stage_binding
        .headers(http.get(format!(
            "{base}/workspaces/{workspace_id}/stage/artifacts/STGA-00000000000000000000000000000000"
        )))
        .send()
        .await
        .expect("missing stage artifact request");
    assert_eq!(
        missing.status(),
        404,
        "missing stage artifact must be a 404"
    );

    for invalid_id in [
        "ART-00000000000000000000000000000000",
        "STGA-0000000000000000000000000000000",
        "STGA-000000000000000000000000000000000",
        "STGA-0000000000000000000000000000000G",
    ] {
        let invalid = stage_binding
            .headers(http.get(format!(
                "{base}/workspaces/{workspace_id}/stage/artifacts/{invalid_id}"
            )))
            .send()
            .await
            .expect("invalid Stage artifact id request");
        assert_eq!(invalid.status(), 400, "invalid id {invalid_id}");
        assert_eq!(
            invalid.json::<Value>().await.expect("invalid id JSON")["error"],
            "HSK-400-STAGE-ARTIFACT-ID"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn stage_denial_limits_and_attribution_apply_at_the_authentication_boundary() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the Stage denial-boundary proof");
    let recorder = Arc::new(CollectingRecorder::default());
    let state = app_state_with_recorder(&pg, recorder.clone()).await;
    let workspace_id = pg.create_workspace().await;
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let stage_binding = StageBindingEnv::install();
    let (base, http) = serve(stage_api::routes(state.clone())).await;

    let invalid_token = format!("raw-invalid-stage-token-{}", uuid::Uuid::now_v7());
    let mut raw_workspace_hints = Vec::new();
    for group in 0..16 {
        let workspace_hint = format!("raw-preauth-workspace-{group}-{}", uuid::Uuid::now_v7());
        raw_workspace_hints.push(workspace_hint.clone());
        let raw_body = format!("raw-preauth-body-secret-{group}");
        for _ in 0..8 {
            let response = http
                .post(format!(
                    "{base}/workspaces/{workspace_hint}/stage/artifacts"
                ))
                .header("x-hsk-session-token", &invalid_token)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(raw_body.clone())
                .send()
                .await
                .expect("bounded invalid-token Stage request");
            assert_eq!(response.status(), 401);
        }
    }
    let pre_auth_events = recorder.events.lock().unwrap().clone();
    assert!(
        pre_auth_events.len() <= 128,
        "the fixed 64-bucket limiter permits at most detail+aggregate per bucket/window"
    );
    assert!(
        pre_auth_events
            .iter()
            .any(|event| event.payload["coalesced_count"].as_u64().is_some()),
        "a repeated hostile fingerprint is retained as a bounded aggregate receipt"
    );
    for event in &pre_auth_events {
        assert_eq!(event.actor_id, "unauthenticated");
        assert_eq!(event.payload["actor_id"], "unauthenticated");
        let serialized = event.payload.to_string();
        assert!(!serialized.contains(&invalid_token));
        assert!(!serialized.contains("raw-preauth-body-secret"));
        assert!(
            raw_workspace_hints
                .iter()
                .all(|workspace| !serialized.contains(workspace)),
            "pre-auth denial payloads must not disclose caller-controlled workspace hints"
        );
    }

    let authenticated_actor_prefix = format!("handshake-native:{}:", std::process::id());
    let content_type_denial = stage_binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body("authenticated-content-type-denial")
        .send()
        .await
        .expect("authenticated content-type denial");
    assert_eq!(content_type_denial.status(), 400);

    let malformed_json_denial = stage_binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{")
        .send()
        .await
        .expect("authenticated malformed JSON denial");
    assert_eq!(malformed_json_denial.status(), 400);

    let invalid_base64 = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": format!("invalid-base64-{}", uuid::Uuid::now_v7()),
        "correlation_id": format!("invalid-base64-correlation-{}", uuid::Uuid::now_v7()),
        "content_kind": "selection",
        "label": "invalid base64",
        "content_type": "text/plain",
        "content_base64": "%%%",
    });
    let invalid_base64_denial = stage_binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .json(&invalid_base64)
        .send()
        .await
        .expect("authenticated base64 denial");
    assert_eq!(invalid_base64_denial.status(), 400);

    let missing_workspace = format!("missing-stage-workspace-{}", uuid::Uuid::now_v7());
    let valid_request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": format!("missing-workspace-{}", uuid::Uuid::now_v7()),
        "correlation_id": format!("missing-workspace-correlation-{}", uuid::Uuid::now_v7()),
        "content_kind": "selection",
        "label": "rate before workspace lookup",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(b"bounded"),
    });
    for attempt in 0..=30 {
        let response = stage_binding
            .headers(http.post(format!(
                "{base}/workspaces/{missing_workspace}/stage/artifacts"
            )))
            .json(&valid_request)
            .send()
            .await
            .expect("valid-token missing-workspace Stage request");
        if attempt < 30 {
            assert_eq!(
                response.status(),
                404,
                "requests below the authenticated rate limit reach workspace lookup"
            );
        } else {
            assert_eq!(
                response.status(),
                429,
                "the limiter executes before workspace lookup for a valid binding"
            );
        }
    }

    let all_events = recorder.events.lock().unwrap();
    let authenticated_events = &all_events[pre_auth_events.len()..];
    assert!(authenticated_events.len() >= 4);
    assert!(
        authenticated_events
            .iter()
            .all(|event| event.actor_id.starts_with(&authenticated_actor_prefix)),
        "post-binding DTO, base64, and rate denials retain the native actor identity"
    );
    assert!(authenticated_events
        .iter()
        .all(|event| event.actor_id != "unauthenticated"));
    drop(all_events);

    let authenticated_denials: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE source_component = 'stage_capture_api' AND event_type = 'TOOL_DECISION_RECORDED' AND actor_id LIKE $1 AND payload->>'decision_outcome' = 'deny'",
    )
    .bind(format!("{authenticated_actor_prefix}%"))
    .fetch_one(&state.postgres_pool)
    .await
    .unwrap();
    assert!(
        authenticated_denials >= 4,
        "authenticated denials are durable under the native actor, including the rate gate"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn stage_capture_rejects_crash_stale_binding_without_residue() {
    let pg = knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the exact crash-stale binding proof");
    let recorder = Arc::new(FailSecondRecordOnceRecorder::default());
    let state = app_state_with_recorder(&pg, recorder.clone()).await;
    let workspace_id = pg.create_workspace().await;
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let mut stage_binding = StageBindingEnv::install();
    let mut crashed_native_process = OwnedBindingProcess::spawn();
    let crashed_pid = crashed_native_process.pid();
    stage_binding.set_pid(crashed_pid);
    crashed_native_process.kill_and_wait();

    let (base, http) = serve(stage_api::routes(state.clone())).await;
    let idempotency_key = format!("stage-stale-binding-{}", uuid::Uuid::now_v7());
    let correlation_id = format!("stage-stale-binding-correlation-{}", uuid::Uuid::now_v7());
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": idempotency_key,
        "correlation_id": correlation_id,
        "content_kind": "selection",
        "label": "Crash-stale binding must be denied",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(b"must not persist"),
        "source_ref": "note://stage-stale-binding"
    });
    let response = stage_binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .json(&request)
        .send()
        .await
        .expect("crash-stale Stage capture request");
    assert_eq!(
        response.status(),
        401,
        "a token from killed owned PID {crashed_pid} must be unauthorized"
    );
    assert_eq!(
        response
            .json::<Value>()
            .await
            .expect("crash-stale denial JSON")["error"],
        "HSK-401-STAGE-SESSION"
    );

    // Deterministic PID-reuse counterfactual: the numeric PID is live, but the binding carries a
    // deliberately different birth identity. PID-only authorization would accept this request.
    let reused_pid = std::process::id();
    let actual_reused_birth = test_process_birth_identity(reused_pid)
        .expect("current test process has a verifiable birth identity");
    let stale_reused_birth = mismatched_process_birth(actual_reused_birth.clone());
    assert_ne!(
        stale_reused_birth, actual_reused_birth,
        "PID-reuse counterfactual must change the process birth identity"
    );
    stage_binding.set_pid_with_birth(reused_pid, stale_reused_birth);
    let reuse_idempotency_key = format!("stage-pid-reuse-{}", uuid::Uuid::now_v7());
    let reuse_correlation_id = format!("stage-pid-reuse-correlation-{}", uuid::Uuid::now_v7());
    let mut reuse_request = request.clone();
    reuse_request["idempotency_key"] = Value::String(reuse_idempotency_key.clone());
    reuse_request["correlation_id"] = Value::String(reuse_correlation_id.clone());
    let reuse_response = stage_binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .json(&reuse_request)
        .send()
        .await
        .expect("PID-reuse Stage capture counterfactual");
    assert_eq!(
        reuse_response.status(),
        401,
        "live PID {reused_pid} with a stale birth identity must be unauthorized"
    );
    assert_eq!(
        reuse_response
            .json::<Value>()
            .await
            .expect("PID-reuse denial JSON")["error"],
        "HSK-401-STAGE-SESSION"
    );

    let artifact_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stage_capture_artifacts WHERE workspace_id = $1 AND (correlation_id = $2 OR correlation_id = $3)",
    )
    .bind(&workspace_id)
    .bind(&correlation_id)
    .bind(&reuse_correlation_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("crash-stale Stage artifact residue count");
    assert_eq!(artifact_count, 0, "stale binding creates no Stage artifact");

    let job_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_jobs WHERE protocol_id = 'hsk.stage.capture@1' AND (job_inputs::jsonb->>'correlation_id' = $1 OR job_inputs::jsonb->>'correlation_id' = $2)",
    )
    .bind(&correlation_id)
    .bind(&reuse_correlation_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("crash-stale Stage Job History residue count");
    assert_eq!(
        job_count, 0,
        "stale binding creates no Stage Job History row"
    );

    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE source_component = 'stage_capture_api' AND (correlation_id = $1 OR correlation_id = $2 OR actor_id LIKE $3)",
    )
    .bind(&correlation_id)
    .bind(&reuse_correlation_id)
    .bind(format!("handshake-native:{crashed_pid}:%"))
    .fetch_one(&state.postgres_pool)
    .await
    .expect("crash-stale Stage EventLedger residue count");
    assert_eq!(
        ledger_count, 0,
        "stale or PID-reused binding creates no stage_capture_api EventLedger row by correlation or dead-process attribution"
    );
    assert!(
        recorder.attempts.load(Ordering::SeqCst) <= 4,
        "two stale-binding denials can emit only bounded detail/aggregate Flight Recorder attempts"
    );
    for event in recorder.events.lock().unwrap().iter() {
        assert_eq!(event.actor_id, "unauthenticated");
        assert_eq!(event.payload["actor_id"], "unauthenticated");
        let serialized = event.payload.to_string();
        assert!(!serialized.contains(&workspace_id));
        assert!(!serialized.contains(&correlation_id));
        assert!(!serialized.contains(&reuse_correlation_id));
        assert!(!serialized.contains(&idempotency_key));
        assert!(!serialized.contains(&reuse_idempotency_key));
        assert!(!serialized.contains(&stage_binding.token));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn stage_flight_projection_failure_returns_500_and_retry_heals_once() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP stage_flight_projection_failure: no PostgreSQL");
        return;
    };
    let recorder = Arc::new(FailSecondRecordOnceRecorder::default());
    let state = app_state_with_recorder(&pg, recorder.clone()).await;
    let workspace_id = pg.create_workspace().await;
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let mut stage_binding = StageBindingEnv::install();
    let (base, http) = serve(stage_api::routes(state.clone())).await;
    let path = format!("{base}/workspaces/{workspace_id}/stage/artifacts");
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": "stage-fr-heal-once",
        "correlation_id": "stage-fr-heal-once-correlation",
        "content_kind": "selection",
        "label": "Flight projection heal-once",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(b"durable before projection"),
        "source_ref": "note://stage-fr-heal"
    });

    let first = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send()
        .await
        .expect("first fail-once Stage request");
    assert_eq!(first.status(), 500);
    assert_eq!(
        first.json::<Value>().await.expect("first failure JSON")["error"],
        "HSK-500-STAGE"
    );
    let (artifact_id, persisted_actor_id, job_id, stored_event_id): (
        String,
        String,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT artifact_id, actor_id, job_id, event_ledger_event_id FROM stage_capture_artifacts WHERE workspace_id = $1 AND idempotency_key = $2",
    )
    .bind(&workspace_id)
    .bind("stage-fr-heal-once")
    .fetch_one(&state.postgres_pool)
    .await
    .expect("artifact committed before projection failure");
    let job_id = job_id.expect("committed Stage artifact has Job History id before retry");
    let stored_event_id = stored_event_id
        .expect("committed Stage artifact has ArtifactStored ledger id before retry");
    let decision_event_id: String = sqlx::query_scalar(
        "SELECT payload->>'decision_event_id' FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&stored_event_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("ArtifactStored row links the allow decision before retry");
    let completed_jobs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_jobs WHERE id = $1 AND status = 'completed'")
            .bind(&job_id)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("Job History row is committed before retry");
    assert_eq!(completed_jobs, 1);
    let ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE event_id = $1 OR event_id = $2",
    )
    .bind(&stored_event_id)
    .bind(&decision_event_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("both Stage ledger rows are committed before retry");
    assert_eq!(ledger_rows, 2);
    let original_actor = persisted_actor_id.clone();
    assert!(
        original_actor.starts_with(&format!("handshake-native:{}:", std::process::id())),
        "persisted actor is bound to the validated process birth identity"
    );
    let durable_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stage_capture_artifacts WHERE workspace_id = $1 AND idempotency_key = $2",
    )
    .bind(&workspace_id)
    .bind("stage-fr-heal-once")
    .fetch_one(&state.postgres_pool)
    .await
    .unwrap();
    assert_eq!(durable_count, 1);

    let _restarted_native_process = OwnedBindingProcess::spawn();
    stage_binding.set_pid(_restarted_native_process.pid());
    let first_heal = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send();
    let second_heal = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send();
    let (first_heal, second_heal) = tokio::join!(first_heal, second_heal);
    for response in [
        first_heal.expect("first concurrent healing response"),
        second_heal.expect("second concurrent healing response"),
    ] {
        assert_eq!(response.status(), 200);
        let replay: Value = response
            .json()
            .await
            .expect("concurrent healing replay JSON");
        assert_eq!(replay["artifact_id"], artifact_id);
        assert_eq!(replay["job_id"], job_id);
        assert_eq!(replay["event_ledger_event_id"], stored_event_id);
        assert_eq!(replay["replayed"], true);
    }
    let stable = stage_binding
        .headers(http.post(&path))
        .json(&request)
        .send()
        .await
        .expect("stable replay after concurrent healing");
    assert_eq!(stable.status(), 200);
    let stable: Value = stable.json().await.expect("stable replay JSON");
    assert_eq!(stable["artifact_id"], artifact_id);
    assert_eq!(stable["job_id"], job_id);
    assert_eq!(stable["event_ledger_event_id"], stored_event_id);
    assert_eq!(stable["replayed"], true);

    let events = recorder.events.lock().unwrap();
    assert_eq!(events.len(), 2, "exactly two Stage FR projections");
    assert!(
        events.iter().all(|event| event.actor_id == original_actor),
        "healed projections keep the originally persisted actor attribution"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event.event_type,
                handshake_core::flight_recorder::FlightRecorderEventType::CapabilityAction
            ))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload["type"] == "stage.capture")
            .count(),
        1
    );
    drop(events);
    let final_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stage_capture_artifacts WHERE workspace_id = $1 AND idempotency_key = $2",
    )
    .bind(&workspace_id)
    .bind("stage-fr-heal-once")
    .fetch_one(&state.postgres_pool)
    .await
    .unwrap();
    assert_eq!(final_count, 1, "retries never duplicate durable capture");
}

// ---------------------------------------------------------------------------
// WP-KERNEL-012 MT-120 — the document-save route authenticates the SAME principal
// the Flight Recorder derives, so the `document_saved` receipt-ownership clause
// becomes SATISFIABLE without being removed.
//
//   * AC-120-1  an authenticated save stamps the SERVER-DERIVED principal into the
//               receipt payload (`minted_by_principal`) while the ledger `actor_id`
//               column keeps the CLIENT-declared per-agent attribution.
//   * MT-120    a caller may not FORGE the reserved `handshake-native:` principal
//               namespace by header (403 HSK-403-DOC-ACTOR-SPOOF).
//   * MT-120    a presented-but-invalid session token is a hard 401
//               HSK-401-DOC-SESSION and NEVER downgrades to the header identity.
// ---------------------------------------------------------------------------

/// Obtain the SERVER-DERIVED native principal as an INDEPENDENT ground truth: authenticate an
/// unrelated `stage::capture_context`-gated route with the same binding and read back the actor id it
/// attributed the request to.
///
/// This is deliberately not a local recomputation of the digest. The point of AC-120-1 is that the
/// document-save route authenticates the *same principal another capture_context route derives*, and
/// only a value produced by that code path can prove it.
async fn derived_native_principal_from_stage_route(
    base: &str,
    http: &reqwest::Client,
    binding: &StageBindingEnv,
    recorder: &CollectingRecorder,
    workspace_id: &str,
) -> String {
    // An authenticated request that is denied AFTER the auth boundary (wrong content type) still
    // records the native actor — the cheapest authenticated attribution probe in this suite.
    let denial = binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body("mt120-derived-principal-probe")
        .send()
        .await
        .expect("mt120 authenticated stage probe");
    assert_eq!(
        denial.status(),
        400,
        "the probe must pass authentication and fail on the DTO, not on the credential"
    );
    let events = recorder.events.lock().unwrap();
    let actor = events
        .iter()
        .map(|event| event.actor_id.clone())
        .find(|actor| actor.starts_with("handshake-native:"))
        .expect("an authenticated stage request records the server-derived native actor");
    drop(events);
    actor
}

/// Create a document and return `(rich_document_id, doc_version)` so saves carry the REAL
/// optimistic-concurrency token instead of a guessed one.
async fn mt120_seed_document(
    base: &str,
    http: &reqwest::Client,
    workspace_id: &str,
) -> (String, i64) {
    let created: Value = operator_headers(
        http.post(format!("{base}/knowledge/documents")),
        "mt120-create",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": "MT120 Save Attribution",
        "content_json": {
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "before" }] }]
        }
    }))
    .send()
    .await
    .expect("mt120 create doc")
    .json()
    .await
    .expect("mt120 create json");
    (
        created["document"]["rich_document_id"]
            .as_str()
            .expect("mt120 rich_document_id")
            .to_string(),
        created["document"]["doc_version"]
            .as_i64()
            .expect("mt120 doc_version"),
    )
}

fn mt120_save_body(expected_version: i64, text: &str) -> Value {
    json!({
        "expected_version": expected_version,
        "content_json": {
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }]
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt120_authenticated_save_stamps_derived_principal_without_rebinding_actor_id() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt120_authenticated_save: no PostgreSQL");
        return;
    };
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let binding = StageBindingEnv::install();
    let recorder = Arc::new(CollectingRecorder::default());
    let state = app_state_with_recorder(&pg, recorder.clone()).await;
    let workspace_id = pg.create_workspace().await;
    // Both routers share ONE AppState + ONE binding, so the two routes must resolve one principal.
    let (base, http) = serve(
        docs_api::routes(state.clone()).merge(stage_api::routes(state.clone())),
    )
    .await;
    let derived =
        derived_native_principal_from_stage_route(&base, &http, &binding, &recorder, &workspace_id)
            .await;
    let (doc_id, doc_version) = mt120_seed_document(&base, &http, &workspace_id).await;

    // A per-agent save actor that is NOT the derived principal — exactly what the product sends.
    let agent_actor = "mt120-agent-a";
    let saved = binding
        .headers(http.put(format!("{base}/knowledge/documents/{doc_id}/save")))
        .header("x-hsk-actor-id", agent_actor)
        .header("x-hsk-kernel-task-run-id", "KTR-MT120-A")
        .header("x-hsk-session-run-id", "SR-MT120-A")
        .header("x-hsk-actor-kind", "operator")
        .json(&mt120_save_body(doc_version, "after"))
        .send()
        .await
        .expect("mt120 authenticated save");
    assert_eq!(saved.status(), 200, "authenticated save must succeed");
    let saved: Value = saved.json().await.expect("mt120 save json");
    let receipt = saved["save_receipt_event_id"]
        .as_str()
        .expect("mt120 save receipt id")
        .to_string();

    let (actor_id, payload): (String, Value) =
        sqlx::query_as("SELECT actor_id, payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&receipt)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("read MT-120 save receipt");
    // AC-120-2: per-agent attribution SURVIVES in the actor_id column. Rebinding it would destroy
    // swarm attribution (and the MT-043 exact-attribution DO block in test_e7).
    assert_eq!(actor_id, agent_actor);
    assert_ne!(actor_id, derived);
    // AC-120-1: the ownership anchor is server-written and IS the Flight Recorder's derived principal.
    assert_eq!(
        payload["minted_by_principal"].as_str(),
        Some(derived.as_str()),
        "authenticated save must stamp the server-derived principal: {payload}"
    );

    // The owner of the reserved namespace may declare its own id; the guard permits exactly that.
    let owner_save = binding
        .headers(http.put(format!("{base}/knowledge/documents/{doc_id}/save")))
        .header("x-hsk-actor-id", &derived)
        .header("x-hsk-kernel-task-run-id", "KTR-MT120-OWNER")
        .header("x-hsk-session-run-id", "SR-MT120-OWNER")
        .header("x-hsk-actor-kind", "operator")
        .json(&mt120_save_body(doc_version + 1, "owner"))
        .send()
        .await
        .expect("mt120 owner save");
    assert_eq!(
        owner_save.status(),
        200,
        "the authenticated owner of the reserved namespace is not a spoofer"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt120_unauthenticated_caller_cannot_forge_the_reserved_native_principal() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt120_namespace_forgery: no PostgreSQL");
        return;
    };
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let (base, http) = serve(docs_api::routes(state.clone())).await;
    let (doc_id, doc_version) = mt120_seed_document(&base, &http, &workspace_id).await;

    let forged = "handshake-native:1:deadbeef";
    // NO session token at all — the exact positive control recorded in the MT-120 contract.
    let response = http
        .put(format!("{base}/knowledge/documents/{doc_id}/save"))
        .header("x-hsk-actor-id", forged)
        .header("x-hsk-kernel-task-run-id", "KTR-MT120-FORGE")
        .header("x-hsk-session-run-id", "SR-MT120-FORGE")
        .header("x-hsk-actor-kind", "operator")
        .json(&mt120_save_body(doc_version, "forged"))
        .send()
        .await
        .expect("mt120 forged save request");
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.expect("mt120 forged save body");
    assert_eq!(body["error"], "HSK-403-DOC-ACTOR-SPOOF");

    // The guard sits on the shared identity path, so a READ route forges nothing either.
    let read = http
        .get(format!("{base}/knowledge/documents/{doc_id}"))
        .header("x-hsk-actor-id", forged)
        .header("x-hsk-kernel-task-run-id", "KTR-MT120-FORGE")
        .header("x-hsk-session-run-id", "SR-MT120-FORGE")
        .send()
        .await
        .expect("mt120 forged read request");
    assert_eq!(read.status(), 403);

    let forged_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE actor_id = $1")
            .bind(forged)
            .fetch_one(&state.postgres_pool)
            .await
            .expect("count forged ledger rows");
    assert_eq!(forged_rows, 0, "a forged principal must leave no ledger row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires_pg"]
async fn mt120_invalid_session_token_is_401_and_never_downgrades_to_the_header_identity() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt120_invalid_session: no PostgreSQL");
        return;
    };
    let _binding_test_guard = STAGE_BINDING_TEST_LOCK.lock().await;
    let _binding = StageBindingEnv::install();
    let state = app_state(&pg).await;
    let workspace_id = pg.create_workspace().await;
    let (base, http) = serve(docs_api::routes(state.clone())).await;
    let (doc_id, doc_version) = mt120_seed_document(&base, &http, &workspace_id).await;

    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_id = $1 AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_SAVED'",
    )
    .bind(&doc_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("count saves before");

    // Well-formed but WRONG token: the binding installed above is live, this credential is not its
    // token, so `capture_context` fails — the stale/forged-credential case.
    let stale_token = "b".repeat(64);
    let response = http
        .put(format!("{base}/knowledge/documents/{doc_id}/save"))
        .header("x-hsk-session-token", &stale_token)
        .header("x-hsk-actor-id", "mt120-agent-stale")
        .header("x-hsk-kernel-task-run-id", "KTR-MT120-STALE")
        .header("x-hsk-session-run-id", "SR-MT120-STALE")
        .header("x-hsk-actor-kind", "operator")
        .json(&mt120_save_body(doc_version, "stale"))
        .send()
        .await
        .expect("mt120 stale-token save request");
    assert_eq!(
        response.status(),
        401,
        "a presented-but-invalid token must fail closed, not fall back"
    );
    let body: Value = response.json().await.expect("mt120 stale-token body");
    assert_eq!(body["error"], "HSK-401-DOC-SESSION");

    // Proof it did NOT silently continue as the header identity: no save happened at all.
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_id = $1 AND event_type = 'KNOWLEDGE_RICH_DOCUMENT_SAVED'",
    )
    .bind(&doc_id)
    .fetch_one(&state.postgres_pool)
    .await
    .expect("count saves after");
    assert_eq!(after, before, "a rejected credential must not save");
    let leaked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE actor_id = $1")
            .bind("mt120-agent-stale")
            .fetch_one(&state.postgres_pool)
            .await
            .expect("count downgraded ledger rows");
    assert_eq!(
        leaked, 0,
        "a rejected credential must never be laundered into the header identity"
    );
}
