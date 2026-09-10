#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: HTTP route coverage restored from the suite deleted by 4f92cc25.
//!
//! Restores 13 route-level proofs from the deleted PostgreSQL-backed
//! `wp_kernel_012_native_editor_routes_pg_tests.rs` onto the real, embedded,
//! on-disk SurrealDB store. Every body runs unconditionally (the original
//! `SKIP ... no PostgreSQL` early returns and `#[ignore = "requires_pg"]`
//! gates are removed) and drives the actual Axum routes over a loopback
//! listener (quiet: no foreground window, no focus steal).
//!
//! Where the original inspected managed PostgreSQL directly via `sqlx`
//! (`kernel_event_ledger`, `stage_capture_artifacts`, `ai_jobs`,
//! `calendar_events`, `work_packets`, `micro_tasks`), this file uses the
//! equivalent backend-agnostic `Database` trait accessors
//! (`list_kernel_events_for_aggregate`, `execute_locus_operation`,
//! `upsert_calendar_source`/`upsert_calendar_event`) or the feature-gated
//! `SurrealTestInspector`/`SurrealTestMutator` read/mutation facade
//! (`store.storage.test_inspector()` / `.test_mutator()`) in place of raw SQL.
//!
//! Coverage:
//!   * Route 1  GET  /workspaces/:ws/locus/work-packets/:id     (locus resolve)
//!   * Route 1  GET  /workspaces/:ws/locus/microtasks/:id       (locus resolve)
//!   * Route 2  POST+GET /workspaces/:ws/stage/artifacts        (stage capture provenance)
//!   * Route 3  GET  /workspaces/:ws/calendar/events             (calendar window)
//!   * Route 6  DELETE /knowledge/documents/:id                  (soft delete)
//!   * MT-067   POST+GET /workspaces/:ws/calendar/activity-spans (span round-trip)
//!   * MT-067   GET  /workspaces/:ws/calendar/events             (daily_note_doc_id link)
//!   * MT-120   the document-save route authenticates the SAME native principal an
//!              independent `stage::capture_context`-gated route derives

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex as StdMutex,
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{NaiveDate, TimeZone, Utc};
use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::api::{
    calendar as calendar_api, knowledge_documents as docs_api, locus as locus_api,
    stage as stage_api,
};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::surreal::{RowFilter, ScalarValue, TestFieldMutation, TestMutationValue};
use handshake_core::storage::{
    CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert, CalendarEventVisibility,
    CalendarSourceProviderType, CalendarSourceSyncState, CalendarSourceUpsert,
    CalendarSourceWritePolicy, StageArtifactStore, StructuredCollaborationStore, WriteContext,
};
use handshake_core::workflows::locus::types as locus_types;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Shared harness: mocks, AppState, loopback server, identity headers.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NoopRecorder;

#[derive(Default)]
struct CollectingRecorder {
    attempts: AtomicUsize,
    events: StdMutex<Vec<FlightRecorderEvent>>,
}

#[derive(Default)]
struct FailSecondRecordOnceRecorder {
    attempts: AtomicUsize,
    events: StdMutex<Vec<FlightRecorderEvent>>,
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
                "intentional WPK012 second-flight-record fail-once".to_owned(),
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

/// Build an `AppState` over the isolated embedded store with an explicit Flight Recorder.
async fn test_state(store: &EmbeddedKnowledgeStore, recorder: Arc<dyn FlightRecorder>) -> AppState {
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder,
        diagnostics: Arc::new(NoopRecorder),
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("wpk012-routes-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn default_test_state(store: &EmbeddedKnowledgeStore) -> AppState {
    test_state(store, Arc::new(NoopRecorder)).await
}

/// Boot the given router over loopback and return its base URL, plus a guard that aborts
/// (and joins) the server task on drop so no test leaks a listening socket.
struct ServerGuard(Option<tokio::task::JoinHandle<()>>);

impl ServerGuard {
    async fn shutdown(mut self) {
        let handle = self.0.take().expect("server handle owned");
        handle.abort();
        let error = handle
            .await
            .expect_err("aborted server must not complete normally");
        assert!(error.is_cancelled(), "server shutdown must be cancellation");
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
            if !std::thread::panicking() {
                let runtime = tokio::runtime::Handle::current();
                tokio::task::block_in_place(|| {
                    let result = runtime.block_on(handle);
                    assert!(
                        result.is_err_and(|error| error.is_cancelled()),
                        "dropped server must abort and join"
                    );
                });
            }
        }
    }
}

async fn route_server(app: axum::Router) -> (String, reqwest::Client, ServerGuard) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("wpk012 routes test server");
    });
    (
        format!("http://{addr}"),
        reqwest::Client::new(),
        ServerGuard(Some(handle)),
    )
}

/// Identity headers WITHOUT an actor kind (MT-158 least-privilege absence case).
fn identity_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("wpk012-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-WPK012-{label}"))
        .header("x-hsk-session-run-id", format!("SR-WPK012-{label}"))
}

/// Identity headers plus an explicit actor kind.
fn headers_with_kind(
    req: reqwest::RequestBuilder,
    label: &str,
    kind: &str,
) -> reqwest::RequestBuilder {
    identity_headers(req, label).header("x-hsk-actor-kind", kind)
}

fn operator_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    headers_with_kind(req, label, "operator")
}

fn doc_body(workspace_id: &str, title: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "hello" }] }
            ]
        }
    })
}

/// Create a document as the operator (the privileged setup path).
async fn create_doc(base: &str, http: &reqwest::Client, workspace_id: &str, title: &str) -> Value {
    let resp = operator_headers(http.post(format!("{base}/knowledge/documents")), "doc-setup")
        .json(&doc_body(workspace_id, title))
        .send()
        .await
        .expect("create doc request");
    assert_eq!(resp.status(), 200, "operator create must succeed");
    resp.json().await.expect("create doc json")
}

// ---------------------------------------------------------------------------
// Embedded-store inspection helpers (replace raw `sqlx` residue/readback
// queries against managed PostgreSQL with the feature-gated, read-only
// `SurrealTestInspector` / mutation-gated `SurrealTestMutator` facade).
// ---------------------------------------------------------------------------

async fn table_selector(
    store: &EmbeddedKnowledgeStore,
    table: &str,
) -> handshake_core::storage::surreal::TableSelector {
    store
        .storage
        .test_inspector()
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("select embedded table `{table}`: {error}"))
}

async fn row_count(store: &EmbeddedKnowledgeStore, table: &str, filter: RowFilter) -> u64 {
    let selector = table_selector(store, table).await;
    store
        .storage
        .test_inspector()
        .row_count(&selector, filter)
        .await
        .unwrap_or_else(|error| panic!("count rows in `{table}`: {error}"))
}

async fn field_equals_count(
    store: &EmbeddedKnowledgeStore,
    table: &str,
    field: &str,
    value: ScalarValue,
) -> u64 {
    let selector = table_selector(store, table).await;
    let field = selector
        .field(field)
        .unwrap_or_else(|error| panic!("select field `{table}.{field}`: {error}"));
    store
        .storage
        .test_inspector()
        .row_count(&selector, RowFilter::FieldEquals { field, value })
        .await
        .unwrap_or_else(|error| panic!("count rows in `{table}` by field: {error}"))
}

async fn project_one_row(
    store: &EmbeddedKnowledgeStore,
    table: &str,
    fields: &[&str],
    filter: RowFilter,
) -> BTreeMap<String, Value> {
    let selector = table_selector(store, table).await;
    let field_selectors: Vec<_> = fields
        .iter()
        .map(|name| {
            selector
                .field(*name)
                .unwrap_or_else(|error| panic!("select field `{table}.{name}`: {error}"))
        })
        .collect();
    let mut rows = store
        .storage
        .test_inspector()
        .project(&selector, &field_selectors, filter)
        .await
        .unwrap_or_else(|error| panic!("project row from `{table}`: {error}"));
    assert_eq!(rows.len(), 1, "expected exactly one row in {table}");
    rows.remove(0).values
}

async fn kernel_events_for(
    state: &AppState,
    aggregate_type: &str,
    aggregate_id: &str,
) -> Vec<handshake_core::kernel::KernelEvent> {
    state
        .storage
        .list_kernel_events_for_aggregate(aggregate_type, aggregate_id)
        .await
        .expect("list kernel events for aggregate")
}

// ---------------------------------------------------------------------------
// Locus seeding (replaces raw `INSERT INTO work_packets` / `micro_tasks`):
// the current embedded backend's `work_packets` / `micro_tasks` tables are
// owned by the Locus workflow engine (`execute_locus_operation`), not by a
// generic storage insert, so tests seed through that engine's real
// create/update/register/start operations exactly as
// `storage::tests::locus_and_structured_collaboration_roundtrip_real_store_and_reopen`
// does.
// ---------------------------------------------------------------------------

async fn seed_ready_work_packet(store: &EmbeddedKnowledgeStore, wp_id: &str, title: &str) {
    store
        .db
        .execute_locus_operation(locus_types::LocusOperation::CreateWp(
            locus_types::LocusCreateWpParams {
                wp_id: wp_id.to_owned(),
                title: title.to_owned(),
                description: "A native-editor backend work packet.".to_owned(),
                priority: 1,
                kind: locus_types::WorkPacketType::Test,
                phase: locus_types::WorkPacketPhase::Phase1,
                routing: locus_types::RoutingPolicy::GovStandard,
                task_packet_path: Some(format!(".GOV/task_packets/{wp_id}/packet.json")),
                assignee: None,
                labels: None,
                spec_session_id: None,
                reporter: "wpk012-routes-test".to_owned(),
            },
        ))
        .await
        .expect("create work packet");
    store
        .db
        .execute_locus_operation(locus_types::LocusOperation::UpdateWp(
            locus_types::LocusUpdateWpParams {
                wp_id: wp_id.to_owned(),
                updates: BTreeMap::from([
                    ("status".to_owned(), json!("ready")),
                    ("task_board_status".to_owned(), json!("READY")),
                ]),
                source: Some("wpk012-routes-test".to_owned()),
            },
        ))
        .await
        .expect("promote work packet to ready");
}

fn wpk012_sample_micro_task(wp_id: &str, mt_id: &str, name: &str) -> locus_types::TrackedMicroTask {
    locus_types::TrackedMicroTask {
        schema_id: String::new(),
        schema_version: String::new(),
        record_id: String::new(),
        record_kind: String::new(),
        project_profile_kind: locus_types::ProjectProfileKind::SoftwareDelivery,
        updated_at: Utc::now(),
        mirror_state: locus_types::MirrorSyncState::CanonicalOnly,
        authority_refs: vec![format!("authority:{wp_id}")],
        evidence_refs: Vec::new(),
        summary_record_path: None,
        profile_extension: None,
        mt_id: mt_id.to_owned(),
        wp_id: wp_id.to_owned(),
        name: name.to_owned(),
        scope: "WP-KERNEL-012 MT-150 route coverage proof".to_owned(),
        files: locus_types::MicroTaskFiles {
            read: Vec::new(),
            modify: Vec::new(),
            create: Vec::new(),
        },
        done_criteria: vec!["the locus route resolves the microtask".to_owned()],
        status: locus_types::MicroTaskStatus::Pending,
        active_session_ids: Vec::new(),
        iterations: Vec::new(),
        current_iteration: 0,
        max_iterations: 3,
        validation_result: None,
        escalation: locus_types::MicroTaskEscalation {
            current_level: 0,
            escalation_chain: Vec::new(),
            escalations_count: 0,
            drop_backs_count: 0,
        },
        started_at: None,
        completed_at: None,
        duration_ms: None,
        depends_on: Vec::new(),
        metadata: json!({}),
    }
}

/// Create `wp_id` (default "stub" status; no status transition required — `RegisterMts`
/// only requires the work-packet record to exist) and register + start `mt_id`, landing it
/// in the engine's canonical `in_progress` state.
async fn seed_running_micro_task(store: &EmbeddedKnowledgeStore, wp_id: &str, mt_id: &str, name: &str) {
    store
        .db
        .execute_locus_operation(locus_types::LocusOperation::CreateWp(
            locus_types::LocusCreateWpParams {
                wp_id: wp_id.to_owned(),
                title: "Parent WP".to_owned(),
                description: "Parent work packet for a WPK012 MT-150 microtask proof.".to_owned(),
                priority: 1,
                kind: locus_types::WorkPacketType::Test,
                phase: locus_types::WorkPacketPhase::Phase1,
                routing: locus_types::RoutingPolicy::GovStandard,
                task_packet_path: Some(format!(".GOV/task_packets/{wp_id}/packet.json")),
                assignee: None,
                labels: None,
                spec_session_id: None,
                reporter: "wpk012-routes-test".to_owned(),
            },
        ))
        .await
        .expect("create parent work packet");
    store
        .db
        .execute_locus_operation(locus_types::LocusOperation::RegisterMts(
            locus_types::LocusRegisterMtsParams {
                wp_id: wp_id.to_owned(),
                micro_tasks: vec![wpk012_sample_micro_task(wp_id, mt_id, name)],
            },
        ))
        .await
        .expect("register microtask");
    store
        .db
        .execute_locus_operation(locus_types::LocusOperation::StartMt(
            locus_types::LocusStartMtParams {
                wp_id: wp_id.to_owned(),
                mt_id: mt_id.to_owned(),
                model_id: "wpk012-routes-test".to_owned(),
                lora_id: None,
                escalation_level: 0,
            },
        ))
        .await
        .expect("start microtask");
}

// ---------------------------------------------------------------------------
// Calendar seeding (unchanged Database-trait surface from the deleted suite).
// ---------------------------------------------------------------------------

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
                display_name: "WPK012 Calendar".to_string(),
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

// ---------------------------------------------------------------------------
// Native Stage session-binding harness (process-birth-identity backed
// `HANDSHAKE_STAGE_BINDING_FILE`). The env var and the `stage.rs` capture-rate
// / concurrency / pre-auth-denial limiter statics are PROCESS-GLOBAL, so every
// test below that installs a binding holds `WPK012_STAGE_BINDING_LOCK` for its
// whole body.
// ---------------------------------------------------------------------------

static WPK012_STAGE_BINDING_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(target_os = "windows")]
fn test_process_birth_identity(pid: u32) -> Option<Value> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    if pid == 0 {
        return None;
    }
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
    if pid == 0 {
        return None;
    }
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()?
        .trim()
        .to_owned();
    if boot_id.is_empty() {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, tail) = stat.rsplit_once(") ")?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let state = fields.first()?.as_bytes().first().copied()?;
    if matches!(state, b'Z' | b'X' | b'x') {
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
    if pid == 0 {
        return None;
    }
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

struct StageBindingEnv {
    paths: Vec<std::path::PathBuf>,
    previous: Option<std::ffi::OsString>,
    token: String,
}

impl StageBindingEnv {
    fn write_binding_with_birth(token: &str, pid: u32, process_birth: Value) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "handshake-wpk012-stage-binding-{}.json",
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
            "handshake-wpk012-stage-binding-child-ready-{}-{}",
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

/// Obtain the SERVER-DERIVED native principal as an INDEPENDENT ground truth: authenticate an
/// unrelated `stage::capture_context`-gated route with the same binding and read back the actor id
/// it attributed the request to. Deliberately not a local recomputation of the digest: the point is
/// that another route derives the SAME principal, and only a value produced by that code path proves it.
async fn derived_native_principal_from_stage_route(
    base: &str,
    http: &reqwest::Client,
    binding: &StageBindingEnv,
    recorder: &CollectingRecorder,
    workspace_id: &str,
) -> String {
    let denial = binding
        .headers(http.post(format!("{base}/workspaces/{workspace_id}/stage/artifacts")))
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body("wpk012-derived-principal-probe")
        .send()
        .await
        .expect("authenticated stage probe");
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

// ---------------------------------------------------------------------------
// 1. [SEC] stage_capture_rejects_crash_stale_binding_without_residue
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_capture_rejects_crash_stale_binding_without_residue() {
    let _binding_test_guard = WPK012_STAGE_BINDING_LOCK.lock().await;
    let store = open_embedded_store()
        .await
        .expect("crash-stale Stage binding proof requires an isolated embedded store");
    let recorder = Arc::new(CollectingRecorder::default());
    let state = test_state(&store, recorder.clone()).await;
    let workspace_id = store.create_workspace().await;
    let mut stage_binding = StageBindingEnv::install();
    let mut crashed_native_process = OwnedBindingProcess::spawn();
    let crashed_pid = crashed_native_process.pid();
    stage_binding.set_pid(crashed_pid);
    crashed_native_process.kill_and_wait();

    let (base, http, server) = route_server(stage_api::routes(state.clone())).await;
    let idempotency_key = format!("wpk012-stage-stale-binding-{}", uuid::Uuid::now_v7());
    let correlation_id = format!(
        "wpk012-stage-stale-binding-correlation-{}",
        uuid::Uuid::now_v7()
    );
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": idempotency_key,
        "correlation_id": correlation_id,
        "content_kind": "selection",
        "label": "Crash-stale binding must be denied",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(b"must not persist"),
        "source_ref": "note://wpk012-stage-stale-binding"
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
    let reuse_idempotency_key = format!("wpk012-stage-pid-reuse-{}", uuid::Uuid::now_v7());
    let reuse_correlation_id = format!(
        "wpk012-stage-pid-reuse-correlation-{}",
        uuid::Uuid::now_v7()
    );
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

    // Residue proof over the isolated embedded store: `capture_context` failed on both attempts,
    // so `create_stage_artifact` never reaches the insert transaction and `record_stage_denial`
    // (the only path that writes `kernel_event_ledger` under `source_component = stage_capture_api`)
    // never runs either -- both denials take the pre-workspace, Flight-Recorder-only path.
    assert_eq!(
        row_count(&store, "stage_capture_artifacts", RowFilter::All).await,
        0,
        "stale binding creates no Stage artifact"
    );
    assert_eq!(
        row_count(&store, "ai_jobs", RowFilter::All).await,
        0,
        "stale binding creates no Stage Job History row"
    );
    assert_eq!(
        field_equals_count(
            &store,
            "kernel_event_ledger",
            "source_component",
            ScalarValue::from("stage_capture_api"),
        )
        .await,
        0,
        "stale or PID-reused binding creates no stage_capture_api EventLedger row"
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
    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 2. [SEC] the document-save route authenticates the SAME native principal an
//    independent stage route derives (cross-route AC-120-1 identity proof).
// ---------------------------------------------------------------------------

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
async fn document_save_authenticates_same_native_principal_as_independent_stage_route() {
    let _binding_test_guard = WPK012_STAGE_BINDING_LOCK.lock().await;
    let store = open_embedded_store()
        .await
        .expect("cross-route native principal proof requires an isolated embedded store");
    let recorder = Arc::new(CollectingRecorder::default());
    let state = test_state(&store, recorder.clone()).await;
    let workspace_id = store.create_workspace().await;
    let stage_binding = StageBindingEnv::install();
    let (base, http, server) = route_server(
        docs_api::routes(state.clone()).merge(stage_api::routes(state.clone())),
    )
    .await;

    let created = create_doc(&base, &http, &workspace_id, "WPK012 Cross-Route Principal").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("rich_document_id")
        .to_string();
    let doc_version = created["document"]["doc_version"]
        .as_i64()
        .expect("doc_version");

    // Ground truth: what does an UNRELATED capture_context-gated route derive for this exact
    // binding?
    let derived_from_stage_route =
        derived_native_principal_from_stage_route(&base, &http, &stage_binding, &recorder, &workspace_id)
            .await;

    // The document save is authenticated with the SAME binding but a DIFFERENT client-declared
    // per-agent actor id.
    let agent_actor = "wpk012-cross-route-agent";
    let saved = stage_binding
        .headers(http.put(format!("{base}/knowledge/documents/{doc_id}/save")))
        .header("x-hsk-actor-id", agent_actor)
        .header("x-hsk-kernel-task-run-id", "KTR-WPK012-CROSS-ROUTE")
        .header("x-hsk-session-run-id", "SR-WPK012-CROSS-ROUTE")
        .header("x-hsk-actor-kind", "operator")
        .json(&mt120_save_body(doc_version, "after"))
        .send()
        .await
        .expect("authenticated save");
    assert_eq!(saved.status(), 200, "authenticated save must succeed");
    let saved: Value = saved.json().await.expect("save json");
    let receipt_id = saved["save_receipt_event_id"]
        .as_str()
        .expect("save receipt id")
        .to_string();

    let events = kernel_events_for(&state, "knowledge_rich_document", &doc_id).await;
    let receipt = events
        .into_iter()
        .find(|event| event.event_id == receipt_id)
        .expect("save receipt readback");
    let minted_by_principal = receipt.payload["minted_by_principal"]
        .as_str()
        .expect("minted_by_principal present")
        .to_string();

    assert_eq!(
        minted_by_principal, derived_from_stage_route,
        "the document-save route must authenticate the SAME principal an independent \
         capture_context route derives for the identical binding, not a locally recomputed one"
    );
    assert_ne!(
        minted_by_principal, agent_actor,
        "the server-derived principal is never the client-declared per-agent actor"
    );

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 3. [SEC] activity_span_write_is_event_and_workspace_scoped
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_span_write_is_event_and_workspace_scoped() {
    let store = open_embedded_store()
        .await
        .expect("activity-span scoping proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_a = store.create_workspace().await;
    let workspace_b = store.create_workspace().await;
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

    let (base, http, server) = route_server(calendar_api::routes(state)).await;
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

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 4. route1_locus_work_packet_resolve
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route1_locus_work_packet_resolve() {
    let store = open_embedded_store()
        .await
        .expect("locus work-packet resolve proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    seed_ready_work_packet(&store, "WP-KERNEL-999", "Native Editors WP").await;

    let (base, http, server) = route_server(locus_api::routes(state)).await;
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
    // The embedded Locus engine's canonical on-disk status vocabulary is lower snake_case
    // (`canonical_work_packet_status_for_storage`), unlike the deleted suite's PostgreSQL
    // raw-inserted uppercase literal ("READY"). The route reads the column verbatim.
    assert_eq!(record["status"], "ready");
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

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 5. route1_locus_micro_task_resolve
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route1_locus_micro_task_resolve() {
    let store = open_embedded_store()
        .await
        .expect("locus microtask resolve proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    seed_running_micro_task(&store, "WP-KERNEL-998", "MT-777", "Wire calendar route").await;

    let (base, http, server) = route_server(locus_api::routes(state)).await;
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
    // `LocusOperation::StartMt` lands the tracked microtask in its canonical `in_progress` state
    // (lower snake_case; see `micro_task_status_str`), which the route reads verbatim.
    assert_eq!(record["status"], "in_progress");

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 6. route2_stage_artifact_create_and_resolve
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route2_stage_artifact_create_and_resolve() {
    let _binding_test_guard = WPK012_STAGE_BINDING_LOCK.lock().await;
    let store = open_embedded_store()
        .await
        .expect("stage artifact create/resolve proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    let mut stage_binding = StageBindingEnv::install();

    let (base, http, server) = route_server(stage_api::routes(state.clone())).await;

    let path = format!("{base}/workspaces/{workspace_id}/stage/artifacts");
    let exact_bytes = b"the quick brown fox\0caf\xC3\xA9\nline two";
    let expected_sha = hex::encode(Sha256::digest(exact_bytes));
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": "wpk012-selection-1",
        "correlation_id": "wpk012-correlation-1",
        "content_kind": "selection",
        "label": "Selected snippet",
        "content_type": "application/octet-stream",
        "content_base64": BASE64.encode(exact_bytes),
        "source_ref": "note://DOC-A#sel-1"
    });

    // Caller-asserted operator/system identity cannot invoke the privileged create operation.
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
    oversized["idempotency_key"] = Value::String("wpk012-oversized".to_owned());
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
    assert_eq!(
        fetched["manifest"]["sha256"].as_str(),
        Some(sha),
        "manifest.sha256 matches the hoisted sha256: {fetched}"
    );
    assert_eq!(
        fetched["manifest"]["content_type"], "application/octet-stream",
        "manifest.content_type round-trips: {fetched}"
    );
    assert_eq!(fetched["artifact_id"], artifact_id.as_str());
    assert_eq!(fetched["workspace_id"], workspace_id.as_str());
    assert_eq!(fetched["label"], "Selected snippet");
    assert_eq!(fetched["size_bytes"], exact_bytes.len());
    assert_eq!(fetched["correlation_id"], "wpk012-correlation-1");
    let job_id = fetched["job_id"].as_str().expect("Job History id").to_string();
    let event_id = fetched["event_ledger_event_id"]
        .as_str()
        .expect("ArtifactStored EventLedger id")
        .to_string();

    let stored = StageArtifactStore::new(state.surreal.clone())
        .get_stage_artifact(&workspace_id, &artifact_id)
        .await
        .expect("read persisted stage artifact")
        .expect("persisted stage artifact exists");
    assert!(
        stored.approval_id.starts_with("native-mcp-stage:") && stored.approval_id.len() > 64,
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
    let stored_again = StageArtifactStore::new(state.surreal.clone())
        .get_stage_artifact(&workspace_id, &artifact_id)
        .await
        .expect("re-read persisted stage artifact")
        .expect("persisted stage artifact still exists");
    assert!(
        stored_again
            .actor_id
            .starts_with(&format!("handshake-native:{}:", std::process::id())),
        "persisted Stage actor includes the validated process birth fingerprint"
    );

    let mut concurrent_request = request.clone();
    concurrent_request["idempotency_key"] = Value::String("wpk012-concurrent-1".to_owned());
    concurrent_request["correlation_id"] = Value::String("wpk012-concurrent-correlation".to_owned());
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

    let job_status = project_one_row(
        &store,
        "ai_jobs",
        &["status"],
        RowFilter::IdEquals(job_id.clone()),
    )
    .await;
    assert_eq!(job_status["status"], json!("completed"));

    let artifact_events = kernel_events_for(&state, "stage_capture_artifact", &artifact_id).await;
    let artifact_event = artifact_events
        .iter()
        .find(|event| event.event_id == event_id)
        .expect("ArtifactStored EventLedger row readback");
    assert_eq!(artifact_event.event_type, KernelEventType::ArtifactStored);

    let allow_decisions = kernel_events_for(&state, "stage_capture_authorization", &artifact_id)
        .await
        .into_iter()
        .filter(|event| event.payload["decision_outcome"] == "allow")
        .count();
    assert_eq!(
        allow_decisions, 1,
        "the authenticated allow decision is durable exactly once; pre-auth denials stay in the \
         redacted Flight Recorder path and do not disclose a workspace aggregate"
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

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 7. stage_flight_projection_failure_returns_500_and_retry_heals_once
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_flight_projection_failure_returns_500_and_retry_heals_once() {
    let _binding_test_guard = WPK012_STAGE_BINDING_LOCK.lock().await;
    let store = open_embedded_store()
        .await
        .expect("Stage flight-projection heal proof requires an isolated embedded store");
    let recorder = Arc::new(FailSecondRecordOnceRecorder::default());
    let state = test_state(&store, recorder.clone()).await;
    let workspace_id = store.create_workspace().await;
    let mut stage_binding = StageBindingEnv::install();
    let (base, http, server) = route_server(stage_api::routes(state.clone())).await;
    let path = format!("{base}/workspaces/{workspace_id}/stage/artifacts");
    let request = json!({
        "schema_version": stage_api::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": "wpk012-fr-heal-once",
        "correlation_id": "wpk012-fr-heal-once-correlation",
        "content_kind": "selection",
        "label": "Flight projection heal-once",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(b"durable before projection"),
        "source_ref": "note://wpk012-stage-fr-heal"
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

    let persisted = project_one_row(
        &store,
        "stage_capture_artifacts",
        &["artifact_id", "actor_id", "job_id", "event_ledger_event_id"],
        RowFilter::FieldEquals {
            field: table_selector(&store, "stage_capture_artifacts")
                .await
                .field("idempotency_key")
                .expect("idempotency_key field"),
            value: ScalarValue::from("wpk012-fr-heal-once"),
        },
    )
    .await;
    let artifact_id = persisted["artifact_id"]
        .as_str()
        .expect("artifact committed before projection failure")
        .to_string();
    let original_actor = persisted["actor_id"]
        .as_str()
        .expect("persisted actor before retry")
        .to_string();
    let job_id = persisted["job_id"]
        .as_str()
        .expect("committed Stage artifact has Job History id before retry")
        .to_string();
    let stored_event_id = persisted["event_ledger_event_id"]
        .as_str()
        .expect("committed Stage artifact has ArtifactStored ledger id before retry")
        .to_string();
    assert!(
        original_actor.starts_with(&format!("handshake-native:{}:", std::process::id())),
        "persisted actor is bound to the validated process birth identity"
    );

    let artifact_events = kernel_events_for(&state, "stage_capture_artifact", &artifact_id).await;
    let artifact_event = artifact_events
        .iter()
        .find(|event| event.event_id == stored_event_id)
        .expect("ArtifactStored row committed before retry");
    let decision_event_id = artifact_event.payload["decision_event_id"]
        .as_str()
        .expect("ArtifactStored row links the allow decision before retry")
        .to_string();
    let decision_events = kernel_events_for(&state, "stage_capture_authorization", &artifact_id).await;
    assert!(
        decision_events
            .iter()
            .any(|event| event.event_id == decision_event_id),
        "the linked allow decision is committed before retry"
    );

    let job_status = project_one_row(
        &store,
        "ai_jobs",
        &["status"],
        RowFilter::IdEquals(job_id.clone()),
    )
    .await;
    assert_eq!(job_status["status"], json!("completed"));

    let durable_count = field_equals_count(
        &store,
        "stage_capture_artifacts",
        "idempotency_key",
        ScalarValue::from("wpk012-fr-heal-once"),
    )
    .await;
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
    let final_count = field_equals_count(
        &store,
        "stage_capture_artifacts",
        "idempotency_key",
        ScalarValue::from("wpk012-fr-heal-once"),
    )
    .await;
    assert_eq!(final_count, 1, "retries never duplicate durable capture");

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 8/9. mt067_calendar_event_populates_daily_note_doc_id +
//      mt067_activity_span_create_and_query_round_trip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt067_calendar_event_populates_daily_note_doc_id() {
    let store = open_embedded_store()
        .await
        .expect("mt067 daily-note linkage proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    let start = Utc.with_ymd_and_hms(2026, 7, 4, 9, 0, 0).unwrap();
    seed_calendar_event(&state, &workspace_id, "cal-evt-dn", start).await;

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

    let (base, http, server) = route_server(calendar_api::routes(state)).await;
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

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt067_activity_span_create_and_query_round_trip() {
    let store = open_embedded_store()
        .await
        .expect("mt067 activity-span round-trip proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    let start = Utc.with_ymd_and_hms(2026, 7, 3, 9, 0, 0).unwrap();
    seed_calendar_event(&state, &workspace_id, "cal-evt-mt067", start).await;

    let (base, http, server) = route_server(calendar_api::routes(state)).await;

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

    let bad = http
        .get(format!(
            "{base}/workspaces/{workspace_id}/calendar/activity-spans?event_id="
        ))
        .send()
        .await
        .expect("bad event id request");
    assert_eq!(bad.status(), 400, "empty event_id must be a 400");

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 10. mt067_storage_boundary_rejects_invalid_temporal_rows_without_authority_residue
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt067_storage_boundary_rejects_invalid_temporal_rows_without_authority_residue() {
    let store = open_embedded_store()
        .await
        .expect("mt067 temporal rejection proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
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
        assert_eq!(
            row_count(
                &store,
                "calendar_events",
                RowFilter::IdEquals(event_id.clone()),
            )
            .await,
            0,
            "{event_id} must leave no calendar_events row"
        );
        assert!(
            kernel_events_for(&state, "calendar_event", &event_id)
                .await
                .is_empty(),
            "{event_id} must leave no EventLedger receipt"
        );
        assert_eq!(
            field_equals_count(
                &store,
                "calendar_mutation_outbox",
                "calendar_event_id",
                ScalarValue::from(event_id.as_str()),
            )
            .await,
            0,
            "{event_id} must leave no calendar mutation outbox row"
        );
    }

    server_noop_placeholder(&state).await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

/// No route is exercised in the temporal-rejection proof above (it targets the storage
/// boundary directly, exactly as the deleted suite did); this no-op keeps the test's shutdown
/// shape consistent with every other test in this file for readability.
async fn server_noop_placeholder(_state: &AppState) {}

// ---------------------------------------------------------------------------
// 11. stage_denial_limits_and_attribution_apply_at_the_authentication_boundary
//     (stage.rs:1337 rate_limit_accepts_thirty_and_rejects_thirty_first only
//     proves `check_rate` in isolation; this restores the route-level
//     pre-auth fingerprint-bucket limiter, the authenticated rate/DTO/base64
//     denial boundary, and the "denials stay attributed to the native actor,
//     never leak secrets" proof.)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_denial_limits_and_attribution_apply_at_the_authentication_boundary() {
    let _binding_test_guard = WPK012_STAGE_BINDING_LOCK.lock().await;
    let store = open_embedded_store()
        .await
        .expect("Stage denial-boundary proof requires an isolated embedded store");
    let recorder = Arc::new(CollectingRecorder::default());
    let state = test_state(&store, recorder.clone()).await;
    let workspace_id = store.create_workspace().await;
    let stage_binding = StageBindingEnv::install();
    let (base, http, server) = route_server(stage_api::routes(state.clone())).await;

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

    let mut authenticated_denials = kernel_events_for(&state, "stage_capture_authorization", &workspace_id).await;
    authenticated_denials.extend(
        kernel_events_for(&state, "stage_capture_authorization", &missing_workspace).await,
    );
    let authenticated_denial_count = authenticated_denials
        .iter()
        .filter(|event| {
            event.actor.actor_id().starts_with(&authenticated_actor_prefix)
                && event.event_type == KernelEventType::ToolDecisionRecorded
                && event.payload["decision_outcome"] == "deny"
        })
        .count();
    assert!(
        authenticated_denial_count >= 4,
        "authenticated denials are durable under the native actor, including the rate gate"
    );

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 12. route3_calendar_events_returns_events_in_window
//     (mex_tests.rs:910 exercises the `calendar_sync` WORKFLOW, requires the
//     `duckdb` feature, and has not been executed in this WP's runs; it does
//     not touch this HTTP route at all, so the windowed-route assertions are
//     restored in full here.)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route3_calendar_events_returns_events_in_window() {
    let store = open_embedded_store()
        .await
        .expect("calendar window route proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
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

    // Recreate a legacy/incomplete row (missing start_local/end_local on a non-all-day event) the
    // way managed PostgreSQL migrations left some historic rows, but through the embedded store's
    // feature-gated `SurrealTestMutator` (schema-governed clone + field override) rather than raw
    // SQL, since `upsert_calendar_event` itself validates and rejects an incomplete timed row.
    let events_table = table_selector(&store, "calendar_events").await;
    store
        .storage
        .test_mutator()
        .duplicate_row(
            &events_table,
            "cal-evt-wpk012",
            "cal-evt-wpk012-legacy",
            &[
                TestFieldMutation::new(
                    events_table.field("title").expect("title field"),
                    TestMutationValue::string("Historic incomplete event"),
                ),
                TestFieldMutation::new(
                    events_table.field("start_local").expect("start_local field"),
                    TestMutationValue::none(),
                ),
                TestFieldMutation::new(
                    events_table.field("end_local").expect("end_local field"),
                    TestMutationValue::none(),
                ),
            ],
        )
        .await
        .expect("seed explicit legacy temporal row via the test mutator");

    let (base, http, server) = route_server(calendar_api::routes(state)).await;
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

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}

// ---------------------------------------------------------------------------
// 13. [SEC] route6_document_soft_delete_tombstones_and_receipts
//     (knowledge_documents_api_tests.rs:1568 mt032_delete_is_* already proves
//     the atomic tombstone/backlink/canvas/search-projection cleanup and that
//     `deleted_receipt_event_id` + the tombstone's own `deleted_at` /
//     `deleted_receipt_event_id` columns are non-null; this restores the two
//     assertions it does not make: an unauthenticated delete is DENIED and
//     never mutates the document, and the delete's EventLedger receipt is
//     actually typed `KNOWLEDGE_RICH_DOCUMENT_DELETED`.)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route6_document_soft_delete_tombstones_and_receipts() {
    let store = open_embedded_store()
        .await
        .expect("document soft-delete proof requires an isolated embedded store");
    let state = default_test_state(&store).await;
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = route_server(docs_api::routes(state.clone())).await;

    let created = create_doc(&base, &http, &workspace_id, "Doomed Doc").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("rich_document_id")
        .to_string();

    // An unauthenticated DELETE (missing actor-kind, per MT-158 least-privilege) is denied
    // (403), never a delete.
    let denied = identity_headers(
        http.delete(format!("{base}/knowledge/documents/{doc_id}")),
        "delete-denied",
    )
    .send()
    .await
    .expect("denied delete request");
    assert_eq!(
        denied.status(),
        403,
        "unauthenticated delete must be denied"
    );
    let not_yet_deleted = project_one_row(
        &store,
        "knowledge_rich_documents",
        &["deleted_at"],
        RowFilter::IdEquals(doc_id.clone()),
    )
    .await;
    assert!(
        not_yet_deleted["deleted_at"].is_null(),
        "a denied delete request must never tombstone the document"
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

    // The delete left a KNOWLEDGE_RICH_DOCUMENT_DELETED EventLedger receipt.
    let events = state
        .storage
        .list_kernel_events_for_aggregate("knowledge_rich_document", &doc_id)
        .await
        .expect("read ledger");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == KernelEventType::KnowledgeRichDocumentDeleted
                && e.event_id == receipt),
        "a KNOWLEDGE_RICH_DOCUMENT_DELETED receipt with this id must be appended to the EventLedger"
    );

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded knowledge test store");
}
