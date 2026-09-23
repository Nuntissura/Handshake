//! WP-KERNEL-012 MT-154: non-Loom route authority matrix through the real mounted router
//! (`handshake_core::api::routes`) against an isolated embedded SurrealDB store.
//!
//! Master Spec 02-system-architecture.md:2758 (deny by default on every executable backend
//! boundary), :2773 (privileged sessions never execute ordinary protected-resource flows), :2776
//! (record-user table/field permissions plus ResourceBroker are the non-bypassable data boundary),
//! LM-RLS-001/002. Every family proves: the owner succeeds and the canonical row reflects the change;
//! an anonymous caller gets the constant denial with no side effect; a second account gets the
//! constant denial (or an empty list) with no side effect; and, where a workspace grant can express
//! it, a same-account read-only (viewer) principal is denied the write.
#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

#[path = "account_session_support/mod.rs"]
mod account_session_support;

use std::sync::Arc;

use account_session_support::{NativeBindingEnv, OwnerSession, NATIVE_BINDING_ENV_LOCK};
use async_trait::async_trait;
use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use reqwest::StatusCode;
use serde_json::{json, Value};

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

fn app_state(store: &EmbeddedKnowledgeStore) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt154-route-authority".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// One test body: the isolated store, the full mounted router on loopback, and two accounts
/// (owner A and a different account B) bound to the same live native binding.
struct Matrix {
    store: EmbeddedKnowledgeStore,
    state: AppState,
    base: String,
    owner: OwnerSession,
    other: OwnerSession,
    workspace_id: String,
    server: Option<tokio::task::JoinHandle<()>>,
    // Drop order: binding (restores the env) before the lock.
    _binding: NativeBindingEnv,
    _lock: tokio::sync::MutexGuard<'static, ()>,
}

impl Matrix {
    async fn start() -> Self {
        // Open the isolated store before taking the process-wide binding lock: store open does not
        // read HANDSHAKE_STAGE_BINDING_FILE, and a stalled open must not serialize the whole file.
        let store = open_embedded_store()
            .await
            .expect("isolated embedded store is required for the MT-154 route matrix");
        let lock = NATIVE_BINDING_ENV_LOCK.lock().await;
        let binding = NativeBindingEnv::install();
        let owner = OwnerSession::provision(&store.storage, binding.token()).await;
        let other = OwnerSession::provision(&store.storage, binding.token()).await;
        let state = app_state(&store);
        let workspace_id = owner.create_workspace(&state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind MT-154 loopback listener");
        let addr = listener.local_addr().expect("MT-154 listener addr");
        let app = handshake_core::api::routes(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("MT-154 route server");
        });
        Self {
            store,
            state,
            base: format!("http://{addr}"),
            owner,
            other,
            workspace_id,
            server: Some(server),
            _binding: binding,
            _lock: lock,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// The same Owner account's read-only principal (LM-RLS-001 viewer): a second principal in the
    /// owner's account and access space whose only grant on the workspace is Read with fs.read and
    /// fr.read. Returns `None` when the harness cannot express it.
    async fn viewer(&self) -> Option<OwnerSession> {
        use handshake_core::storage::surreal::resource_authority::{
            ProvisionedIdentity, ResourceAction, ResourceGrantSpec,
        };
        use sha2::{Digest, Sha256};
        let viewer_key = format!("route-harness-viewer-{}", uuid::Uuid::now_v7());
        let capabilities = vec!["fs.read".to_owned(), "fr.read".to_owned()];
        let principal = self
            .store
            .storage
            .provision_principal(
                &self.owner.actor_id,
                &viewer_key,
                "human_account",
                &viewer_key,
                "Operator",
                &capabilities,
                &self.owner.actor_id,
                Some(&hex::encode(Sha256::digest(
                    self.owner.channel_binding_token.as_bytes(),
                ))),
                std::time::Duration::from_secs(3600),
            )
            .await
            .ok()?;
        if principal.identity.account_id != self.owner.account_id
            || principal.identity.access_space_id != self.owner.access_space_id
        {
            return None;
        }
        let workspace_resource = self
            .store
            .storage
            .register_workspace_resource(
                &ProvisionedIdentity {
                    account_id: self.owner.account_id.clone(),
                    principal_id: self.owner.principal_id.clone(),
                    access_space_id: self.owner.access_space_id.clone(),
                },
                &self.workspace_id,
            )
            .await
            .ok()?;
        self.store
            .storage
            .grant_resource(
                &self.owner.account_id,
                &self.owner.access_space_id,
                ResourceGrantSpec {
                    principal_id: principal.identity.principal_id.clone(),
                    resource_id: workspace_resource.resource_id,
                    actions: vec![ResourceAction::Read],
                    capability_ids: capabilities,
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await
            .ok()?;
        Some(OwnerSession {
            session_token: principal.session.token,
            channel_binding_token: self.owner.channel_binding_token.clone(),
            account_id: principal.identity.account_id,
            principal_id: principal.identity.principal_id,
            access_space_id: principal.identity.access_space_id,
            session_id: principal.session.session_id,
            actor_id: viewer_key,
        })
    }
}

impl Drop for Matrix {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
    }
}

/// Anonymous: no credential headers at all.
fn anonymous() -> reqwest::Client {
    reqwest::Client::new()
}

async fn status_and_json(response: reqwest::Response) -> (StatusCode, Value) {
    let status = response.status();
    let text = response.text().await.expect("read response body");
    let body = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    (status, body)
}

/// The constant denial every protected route answers (api::authority::constant_denial).
fn assert_constant_denial(status: StatusCode, body: &Value, what: &str) {
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{what}: expected constant 403, got {body}"
    );
    assert_eq!(
        body["error"], "HSK-403-PROTECTED-RESOURCE",
        "{what}: constant denial body, got {body}"
    );
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: calendar (mt154_calendar_routes_authority)
// ---------------------------------------------------------------------------------------------

/// Seeds one timed calendar event through the storage layer (no route creates events; the calendar
/// sync workflow owns event writes). The source itself is created through the owner's PUT route.
async fn mt154_seed_calendar_event(m: &Matrix, source_id: &str, event_id: &str) {
    use chrono::TimeZone;
    use handshake_core::storage::{
        CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert, CalendarEventVisibility,
        WriteContext,
    };
    m.state
        .storage
        .upsert_calendar_event(
            &WriteContext::human(None),
            CalendarEventUpsert {
                id: event_id.to_owned(),
                workspace_id: m.workspace_id.clone(),
                source_id: source_id.to_owned(),
                external_id: None,
                external_etag: None,
                title: "MT-154 owner event".to_owned(),
                description: None,
                location: None,
                start_ts_utc: chrono::Utc.with_ymd_and_hms(2026, 7, 3, 9, 0, 0).unwrap(),
                end_ts_utc: chrono::Utc.with_ymd_and_hms(2026, 7, 3, 10, 0, 0).unwrap(),
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
        .expect("seed MT-154 calendar event");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_calendar_routes_authority() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let source_url = |id: &str| m.url(&format!("/workspaces/{ws}/calendar/sources/{id}"));
    let events_url = m.url(&format!(
        "/workspaces/{ws}/calendar/events?from_date=2026-07-03&to_date_exclusive=2026-07-04\
         &from_utc=2026-07-03T00:00:00Z&to_utc=2026-07-04T00:00:00Z&view_tzid=UTC"
    ));
    let spans_url = m.url(&format!("/workspaces/{ws}/calendar/activity-spans"));
    let source_body = |name: &str| {
        json!({
            "display_name": name,
            "provider_type": "local",
            "write_policy": "read_only_import",
            "default_tzid": "UTC",
        })
    };

    // Owner positive: the source is created as the session principal, never the header actor.
    let (status, body) = status_and_json(
        owner
            .put(source_url("mt154-src"))
            .header("x-hsk-actor-id", "mt154-forged-header-actor")
            .json(&source_body("MT-154 owner calendar"))
            .send()
            .await
            .expect("owner PUT calendar source"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner PUT calendar source: {body}");
    assert_eq!(body["id"], "mt154-src");
    assert_eq!(body["workspace_id"], ws.as_str());

    // Anonymous: constant denial on every calendar route, no side effect.
    for (what, request) in [
        (
            "anonymous PUT source",
            anonymous()
                .put(source_url("mt154-anon-src"))
                .json(&source_body("anon")),
        ),
        ("anonymous GET events", anonymous().get(events_url.clone())),
        (
            "anonymous GET spans",
            anonymous().get(format!("{spans_url}?event_id=mt154-evt")),
        ),
        (
            "anonymous POST span",
            anonymous().post(spans_url.clone()).json(&json!({
                "span_id": "CAS-mt154-anon",
                "calendar_event_id": "mt154-evt",
                "started_utc": "2026-07-03T09:05:00Z"
            })),
        ),
    ] {
        let (status, body) = status_and_json(request.send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // A different account: constant denial on the owner's workspace (read and write), no side effect.
    for (what, request) in [
        (
            "other PUT existing source",
            other
                .put(source_url("mt154-src"))
                .json(&source_body("hijacked")),
        ),
        (
            "other PUT new source",
            other
                .put(source_url("mt154-other-src"))
                .json(&source_body("other")),
        ),
        ("other GET events", other.get(events_url.clone())),
        (
            "other GET spans",
            other.get(format!("{spans_url}?event_id=mt154-evt")),
        ),
        (
            "other POST span",
            other.post(spans_url.clone()).json(&json!({
                "span_id": "CAS-mt154-other",
                "calendar_event_id": "mt154-evt",
                "started_utc": "2026-07-03T09:05:00Z"
            })),
        ),
    ] {
        let (status, body) = status_and_json(request.send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // Same-account read-only principal: may read, may not write.
    if let Some(viewer) = m.viewer().await {
        let viewer = viewer.client();
        let (status, body) = status_and_json(
            viewer
                .put(source_url("mt154-src"))
                .json(&source_body("viewer edit"))
                .send()
                .await
                .expect("viewer PUT source"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer PUT source");
        let (status, body) = status_and_json(
            viewer
                .post(spans_url.clone())
                .json(&json!({
                    "span_id": "CAS-mt154-viewer",
                    "calendar_event_id": "mt154-evt",
                    "started_utc": "2026-07-03T09:05:00Z"
                }))
                .send()
                .await
                .expect("viewer POST span"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer POST span");
        let status = viewer
            .get(events_url.clone())
            .send()
            .await
            .expect("viewer GET events")
            .status();
        assert_eq!(status, StatusCode::OK, "viewer reads calendar events");
    }

    // Owner reads its events and records an activity span as the record user.
    mt154_seed_calendar_event(&m, "mt154-src", "mt154-evt").await;
    let (status, events) = status_and_json(
        owner
            .get(events_url.clone())
            .send()
            .await
            .expect("owner GET events"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner GET events: {events}");
    let events = events.as_array().expect("events array");
    assert_eq!(events.len(), 1, "exactly the owner's event: {events:?}");
    assert_eq!(events[0]["id"], "mt154-evt");
    let (status, span) = status_and_json(
        owner
            .post(spans_url.clone())
            .json(&json!({
                "span_id": "CAS-mt154-owner",
                "calendar_event_id": "mt154-evt",
                "started_utc": "2026-07-03T09:05:00Z",
                "edited_doc_ids": ["DOC-MT154"]
            }))
            .send()
            .await
            .expect("owner POST span"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner POST span: {span}");
    assert_eq!(span["span_id"], "CAS-mt154-owner");

    // The other account can neither read nor overwrite the owner's span id or event.
    let (status, body) = status_and_json(
        other
            .get(format!("{spans_url}?event_id=mt154-evt"))
            .send()
            .await
            .expect("other GET spans after owner write"),
    )
    .await;
    assert_constant_denial(status, &body, "other GET spans after owner write");
    let (status, body) = status_and_json(
        other
            .get(events_url.clone())
            .send()
            .await
            .expect("other GET events after seed"),
    )
    .await;
    assert_constant_denial(status, &body, "other GET events after seed");

    // Canonical re-read (owner routes + the stored rows).
    let (status, spans) = status_and_json(
        owner
            .get(format!("{spans_url}?event_id=mt154-evt"))
            .send()
            .await
            .expect("owner GET spans"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner GET spans: {spans}");
    assert_eq!(spans.as_array().map(Vec::len), Some(1), "{spans}");
    assert_eq!(spans[0]["span_id"], "CAS-mt154-owner");
    assert_eq!(spans[0]["edited_doc_ids"], json!(["DOC-MT154"]));
    let stored = m
        .state
        .storage
        .get_calendar_source(&ws, "mt154-src")
        .await
        .expect("re-read calendar source")
        .expect("owner source persisted");
    assert_eq!(stored.display_name, "MT-154 owner calendar");
    assert_eq!(
        stored.last_actor_id.as_deref(),
        Some(m.owner.actor_id.as_str()),
        "the write actor is the session principal, not the header actor"
    );
    for absent in ["mt154-anon-src", "mt154-other-src"] {
        assert!(
            m.state
                .storage
                .get_calendar_source(&ws, absent)
                .await
                .expect("re-read denied source")
                .is_none(),
            "denied PUT {absent} left no row"
        );
    }
    assert_eq!(
        m.state
            .storage
            .list_calendar_sources(&ws)
            .await
            .expect("list calendar sources")
            .len(),
        1
    );
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: stage (mt154_stage_routes_authority)
// ---------------------------------------------------------------------------------------------

fn mt154_stage_capture_body(idempotency_key: &str, bytes: &[u8]) -> Value {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    json!({
        "schema_version": handshake_core::api::stage::STAGE_CAPTURE_SCHEMA,
        "idempotency_key": idempotency_key,
        "correlation_id": format!("{idempotency_key}-correlation"),
        "content_kind": "selection",
        "label": "MT-154 stage capture",
        "content_type": "text/plain",
        "content_base64": BASE64.encode(bytes),
        "source_ref": "note://mt154-stage",
    })
}

async fn mt154_stage_artifact_rows(m: &Matrix) -> u64 {
    let inspector = m.store.storage.test_inspector();
    let selector = inspector
        .table_selector("stage_capture_artifacts")
        .await
        .expect("select stage_capture_artifacts");
    inspector
        .row_count(&selector, handshake_core::storage::surreal::RowFilter::All)
        .await
        .expect("count stage_capture_artifacts rows")
}

/// AC-154-2/3/5/9 for POST /workspaces/:ws/stage/artifacts and GET .../:id, .../:id/content.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_stage_routes_authority() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let create_url = m.url(&format!("/workspaces/{ws}/stage/artifacts"));
    let binding_token = m._binding.token().to_owned();
    let bytes = b"MT-154 exact stage bytes\0\xC3\xA9";
    let request = mt154_stage_capture_body("mt154-stage-capture", bytes);

    // Anonymous, binding-only (native MCP token without an account session: no record user),
    // another account and the same-account viewer are all refused before any row is written.
    let mut denied = vec![
        (
            "anonymous POST capture",
            anonymous().post(create_url.clone()),
        ),
        (
            "binding-only POST capture",
            anonymous()
                .post(create_url.clone())
                .header("x-hsk-session-token", &binding_token),
        ),
        (
            "binding token presented as a session POST capture",
            anonymous()
                .post(create_url.clone())
                .header("x-hsk-session-token", &binding_token)
                .header("x-hsk-channel-binding-token", &binding_token),
        ),
        ("other account POST capture", other.post(create_url.clone())),
    ];
    let viewer = m.viewer().await;
    if let Some(viewer) = &viewer {
        denied.push((
            "viewer POST capture",
            viewer.client().post(create_url.clone()),
        ));
    }
    for (what, request_builder) in denied {
        let (status, body) =
            status_and_json(request_builder.json(&request).send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }
    assert_eq!(
        mt154_stage_artifact_rows(&m).await,
        0,
        "denied Stage captures leave no artifact row"
    );

    // Owner positive: created as the session principal, never the header actor.
    let (status, created) = status_and_json(
        owner
            .post(create_url.clone())
            .header("x-hsk-actor-id", "mt154-forged-stage-actor")
            .header("x-hsk-actor-kind", "system")
            .json(&request)
            .send()
            .await
            .expect("owner POST capture"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner POST capture: {created}");
    let artifact_id = created["artifact_id"]
        .as_str()
        .expect("created artifact id")
        .to_owned();
    let artifact_url = m.url(&format!("/workspaces/{ws}/stage/artifacts/{artifact_id}"));
    let content_url = format!("{artifact_url}/content");

    let (status, fetched) = status_and_json(
        owner
            .get(artifact_url.clone())
            .send()
            .await
            .expect("owner GET artifact"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner GET artifact: {fetched}");
    assert_eq!(fetched["artifact_id"], artifact_id.as_str());
    assert_eq!(fetched["workspace_id"], ws.as_str());
    assert_eq!(
        fetched["event_ledger_event_id"],
        created["event_ledger_event_id"]
    );
    let content = owner
        .get(content_url.clone())
        .send()
        .await
        .expect("owner GET content");
    assert_eq!(content.status(), StatusCode::OK);
    assert_eq!(
        content.bytes().await.expect("owner content bytes").as_ref(),
        bytes
    );

    // Reads: anonymous, binding-only and the other account get the constant denial.
    for url in [&artifact_url, &content_url] {
        for (what, request_builder) in [
            ("anonymous GET", anonymous().get(url.as_str())),
            (
                "binding-only GET",
                anonymous()
                    .get(url.as_str())
                    .header("x-hsk-session-token", &binding_token),
            ),
            ("other account GET", other.get(url.as_str())),
        ] {
            let (status, body) = status_and_json(request_builder.send().await.expect(what)).await;
            assert_constant_denial(status, &body, what);
        }
    }
    // The same-account viewer holds the workspace read grant: it may read, but not capture.
    if let Some(viewer) = &viewer {
        let status = viewer
            .client()
            .get(artifact_url.clone())
            .send()
            .await
            .expect("viewer GET artifact")
            .status();
        assert_eq!(status, StatusCode::OK, "viewer reads the Stage artifact");
    }

    // The other account cannot overwrite or replay the owner's capture key.
    let (status, body) = status_and_json(
        other
            .post(create_url.clone())
            .json(&mt154_stage_capture_body("mt154-stage-capture", b"hijack"))
            .send()
            .await
            .expect("other POST same key"),
    )
    .await;
    assert_constant_denial(status, &body, "other POST same idempotency key");

    // Canonical re-read of the stored row and both receipts.
    assert_eq!(
        mt154_stage_artifact_rows(&m).await,
        1,
        "exactly the owner's artifact"
    );
    let stored = handshake_core::storage::StageArtifactStore::new(m.state.surreal.clone())
        .get_stage_artifact(&ws, &artifact_id)
        .await
        .expect("re-read stage artifact")
        .expect("owner stage artifact persisted");
    assert_eq!(stored.content_bytes, bytes.to_vec());
    assert_eq!(
        stored.actor_id, m.owner.actor_id,
        "the artifact actor is the session principal, not the header actor"
    );
    for (aggregate, event_type) in [
        (
            "stage_capture_artifact",
            handshake_core::kernel::KernelEventType::ArtifactStored,
        ),
        (
            "stage_capture_authorization",
            handshake_core::kernel::KernelEventType::ToolDecisionRecorded,
        ),
    ] {
        let receipts = m
            .state
            .storage
            .list_kernel_events_for_aggregate(aggregate, &artifact_id)
            .await
            .expect("list Stage receipts");
        assert_eq!(receipts.len(), 1, "{aggregate}: exactly one receipt");
        assert_eq!(receipts[0].event_type, event_type);
        assert_eq!(
            receipts[0].actor.actor_id(),
            m.owner.actor_id,
            "{aggregate}: the receipt actor is the session principal"
        );
        assert_eq!(receipts[0].payload["workspace_id"], ws.as_str());
        assert_eq!(receipts[0].payload["decision_outcome"], "allow");
    }
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: canvases (mt154_canvases_routes_authority)
// ---------------------------------------------------------------------------------------------

fn mt154_canvas_graph_body() -> Value {
    json!({
        "nodes": [
            {"id": "mt154-node-a", "kind": "note", "position_x": 1.0, "position_y": 2.0, "data": {"text": "a"}},
            {"id": "mt154-node-b", "kind": "note", "position_x": 3.0, "position_y": 4.0, "data": {"text": "b"}}
        ],
        "edges": [
            {"id": "mt154-edge-ab", "from_node_id": "mt154-node-a", "to_node_id": "mt154-node-b", "kind": "link"}
        ]
    })
}

/// Owner re-read of `/canvases/:id`: returns (title, node count, edge count).
async fn mt154_owner_canvas(m: &Matrix, canvas_id: &str) -> (String, usize, usize) {
    let (status, body) = status_and_json(
        m.owner
            .client()
            .get(m.url(&format!("/canvases/{canvas_id}")))
            .send()
            .await
            .expect("owner GET canvas"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner GET canvas: {body}");
    (
        body["title"].as_str().expect("canvas title").to_owned(),
        body["nodes"].as_array().map(Vec::len).unwrap_or(0),
        body["edges"].as_array().map(Vec::len).unwrap_or(0),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_canvases_routes_authority() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let list_url = m.url(&format!("/workspaces/{ws}/canvases"));

    // Owner positive: create, replace graph, rename, read back.
    let (status, created) = status_and_json(
        owner
            .post(list_url.clone())
            .header("x-hsk-actor-id", "mt154-forged-header-actor")
            .json(&json!({"title": "MT-154 owner canvas"}))
            .send()
            .await
            .expect("owner POST canvas"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner POST canvas: {created}");
    assert_eq!(created["workspace_id"], ws.as_str());
    let canvas_id = created["id"].as_str().expect("canvas id").to_owned();
    let canvas_url = m.url(&format!("/canvases/{canvas_id}"));
    let (status, graph) = status_and_json(
        owner
            .put(canvas_url.clone())
            .json(&mt154_canvas_graph_body())
            .send()
            .await
            .expect("owner PUT canvas graph"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner PUT canvas graph: {graph}");
    assert_eq!(graph["nodes"].as_array().map(Vec::len), Some(2));
    assert_eq!(graph["edges"].as_array().map(Vec::len), Some(1));
    let (status, renamed) = status_and_json(
        owner
            .patch(canvas_url.clone())
            .json(&json!({"title": "MT-154 renamed"}))
            .send()
            .await
            .expect("owner PATCH canvas"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner PATCH canvas: {renamed}");
    assert_eq!(
        mt154_owner_canvas(&m, &canvas_id).await,
        ("MT-154 renamed".to_owned(), 2, 1)
    );
    let (status, listed) = status_and_json(
        owner
            .get(list_url.clone())
            .send()
            .await
            .expect("owner GET canvases"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner GET canvases: {listed}");
    assert_eq!(listed.as_array().map(Vec::len), Some(1), "{listed}");
    assert_eq!(listed[0]["id"], canvas_id.as_str());

    // Anonymous and a different account: constant denial everywhere, including for an unknown
    // canvas id (existence is never disclosed).
    let missing_url = m.url("/canvases/mt154-canvas-does-not-exist");
    for (who, client) in [("anonymous", anonymous()), ("other", other.clone())] {
        for (what, request) in [
            (
                "POST canvas",
                client
                    .post(list_url.clone())
                    .json(&json!({"title": "intruder"})),
            ),
            ("GET canvases", client.get(list_url.clone())),
            ("GET canvas", client.get(canvas_url.clone())),
            ("GET missing canvas", client.get(missing_url.clone())),
            (
                "PATCH canvas",
                client
                    .patch(canvas_url.clone())
                    .json(&json!({"title": "intruder"})),
            ),
            (
                "PUT canvas graph",
                client
                    .put(canvas_url.clone())
                    .json(&json!({"nodes": [], "edges": []})),
            ),
            ("DELETE canvas", client.delete(canvas_url.clone())),
        ] {
            let what = format!("{who} {what}");
            let (status, body) = status_and_json(request.send().await.expect(&what)).await;
            assert_constant_denial(status, &body, &what);
        }
    }

    // The other account works in its own workspace, and the owner cannot reach that canvas.
    let other_ws = m.other.create_workspace(&m.state).await;
    let (status, other_canvas) = status_and_json(
        other
            .post(m.url(&format!("/workspaces/{other_ws}/canvases")))
            .json(&json!({"title": "other canvas"}))
            .send()
            .await
            .expect("other POST own canvas"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "other POST own canvas: {other_canvas}"
    );
    let other_canvas_id = other_canvas["id"].as_str().expect("other canvas id");
    let (status, body) = status_and_json(
        owner
            .get(m.url(&format!("/canvases/{other_canvas_id}")))
            .send()
            .await
            .expect("owner GET other canvas"),
    )
    .await;
    assert_constant_denial(status, &body, "owner GET other account canvas");
    let (status, body) = status_and_json(
        owner
            .delete(m.url(&format!("/canvases/{other_canvas_id}")))
            .send()
            .await
            .expect("owner DELETE other canvas"),
    )
    .await;
    assert_constant_denial(status, &body, "owner DELETE other account canvas");

    // Same-account read-only principal: may read, may not write.
    if let Some(viewer) = m.viewer().await {
        let viewer = viewer.client();
        let status = viewer
            .get(canvas_url.clone())
            .send()
            .await
            .expect("viewer GET canvas")
            .status();
        assert_eq!(status, StatusCode::OK, "viewer reads the canvas");
        for (what, request) in [
            (
                "viewer POST canvas",
                viewer
                    .post(list_url.clone())
                    .json(&json!({"title": "viewer"})),
            ),
            (
                "viewer PATCH canvas",
                viewer
                    .patch(canvas_url.clone())
                    .json(&json!({"title": "viewer"})),
            ),
            (
                "viewer PUT canvas graph",
                viewer
                    .put(canvas_url.clone())
                    .json(&json!({"nodes": [], "edges": []})),
            ),
            ("viewer DELETE canvas", viewer.delete(canvas_url.clone())),
        ] {
            let (status, body) = status_and_json(request.send().await.expect(what)).await;
            assert_constant_denial(status, &body, what);
        }
    }

    // Canonical re-read: every denied request left the owner's canvas and list unchanged.
    assert_eq!(
        mt154_owner_canvas(&m, &canvas_id).await,
        ("MT-154 renamed".to_owned(), 2, 1)
    );
    let (_, listed) = status_and_json(
        owner
            .get(list_url.clone())
            .send()
            .await
            .expect("owner re-list canvases"),
    )
    .await;
    assert_eq!(listed.as_array().map(Vec::len), Some(1), "{listed}");
    let stored = m
        .state
        .storage
        .get_canvas_with_graph(&canvas_id)
        .await
        .expect("re-read stored canvas");
    assert_eq!(stored.canvas.title, "MT-154 renamed");
    assert_eq!((stored.nodes.len(), stored.edges.len()), (2, 1));

    // Owner delete; the canvas is then indistinguishable from one that never existed.
    let status = owner
        .delete(canvas_url.clone())
        .send()
        .await
        .expect("owner DELETE canvas")
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT, "owner DELETE canvas");
    let (status, body) = status_and_json(
        owner
            .get(canvas_url.clone())
            .send()
            .await
            .expect("owner GET deleted canvas"),
    )
    .await;
    assert_constant_denial(status, &body, "owner GET deleted canvas");
    let (_, listed) = status_and_json(
        owner
            .get(list_url.clone())
            .send()
            .await
            .expect("owner list after delete"),
    )
    .await;
    assert_eq!(listed, json!([]));
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: preferences (mt154_preferences_routes_authority_and_receipt_visibility)
// ---------------------------------------------------------------------------------------------

/// AC-154-3 + AC-154-5: workspace preferences authorize the workspace resource and run as the record
/// user; the PREFERENCE_RECORD_CHANGED receipt carries the session principal (never the header actor)
/// and is readable by the owner through the aggregate route, while another account reads 0 receipts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_preferences_routes_authority_and_receipt_visibility() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let pref = "view-defaults.editor.tab-size";
    let list_url = m.url(&format!("/workspaces/{ws}/preferences"));
    let pref_url = m.url(&format!("/workspaces/{ws}/preferences/{pref}"));
    let reset_url = m.url(&format!("/workspaces/{ws}/preferences/{pref}/reset"));
    let history_url = m.url(&format!("/workspaces/{ws}/preferences/{pref}/history"));
    let aggregate_url = m.url(&format!(
        "/kernel/events/aggregates/preference_record/workspace:{ws}:{pref}"
    ));

    // Owner positive: the forged header actor is ignored; the session principal is the writer.
    let (status, body) = status_and_json(
        owner
            .put(pref_url.clone())
            .header("x-hsk-actor-id", "mt154-forged-header-actor")
            .json(&json!({ "value": 2 }))
            .send()
            .await
            .expect("owner PUT preference"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner PUT preference: {body}");
    assert_eq!(body["record"]["value"], json!(2));
    assert_eq!(body["record"]["updated_by"], m.owner.actor_id.as_str());
    assert_eq!(body["receipt"]["actor"], m.owner.actor_id.as_str());
    let event_id = body["receipt"]["event_ledger_event_id"]
        .as_str()
        .expect("receipt event id")
        .to_owned();

    // Canonical re-read through every read route.
    let owner_reads = |owner: &reqwest::Client| {
        let owner = owner.clone();
        let pref_url = pref_url.clone();
        let history_url = history_url.clone();
        let list_url = list_url.clone();
        async move {
            let (status, record) = status_and_json(
                owner
                    .get(pref_url)
                    .send()
                    .await
                    .expect("owner GET preference"),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "owner GET preference: {record}");
            let (status, history) = status_and_json(
                owner
                    .get(history_url)
                    .send()
                    .await
                    .expect("owner GET history"),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "owner GET history: {history}");
            let (status, list) =
                status_and_json(owner.get(list_url).send().await.expect("owner GET list")).await;
            assert_eq!(status, StatusCode::OK, "owner GET list: {list}");
            (record, history, list)
        }
    };
    let (record, history, list) = owner_reads(&owner).await;
    assert_eq!(record["record"]["value"], json!(2));
    assert_eq!(record["record"]["revision"], json!(1));
    assert_eq!(history["receipts"].as_array().map(Vec::len), Some(1));
    assert!(
        list["preferences"]
            .as_array()
            .expect("preference projection")
            .iter()
            .any(|row| row["preference_id"] == pref && row["value"] == json!(2)),
        "projection shows the owner's value: {list}"
    );

    // AC-154-5: the owner reads the session-attributed receipt through the aggregate route.
    let (status, events) = status_and_json(
        owner
            .get(aggregate_url.clone())
            .send()
            .await
            .expect("owner GET preference aggregate"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner aggregate read: {events}");
    let events = events.as_array().expect("owner aggregate events").clone();
    assert_eq!(
        events.len(),
        1,
        "owner sees the preference receipt: {events:?}"
    );
    assert_eq!(events[0]["event_id"], event_id.as_str());
    assert_eq!(events[0]["event_type"], "PREFERENCE_RECORD_CHANGED");
    assert_eq!(
        events[0]["actor"],
        json!({ "kind": "operator", "id": m.owner.actor_id })
    );
    assert_eq!(events[0]["payload"]["workspace_id"], ws.as_str());

    // Anonymous and a different account: constant denial on every route, no side effect.
    for (label, client) in [("anonymous", anonymous()), ("other", other.clone())] {
        for (what, request) in [
            ("GET list", client.get(list_url.clone())),
            ("GET preference", client.get(pref_url.clone())),
            ("GET history", client.get(history_url.clone())),
            (
                "PUT preference",
                client.put(pref_url.clone()).json(&json!({ "value": 9 })),
            ),
            ("POST reset", client.post(reset_url.clone())),
        ] {
            let what = format!("{label} {what}");
            let (status, body) = status_and_json(request.send().await.expect(&what)).await;
            assert_constant_denial(status, &body, &what);
        }
    }
    let (status, events) = status_and_json(
        other
            .get(aggregate_url.clone())
            .send()
            .await
            .expect("other GET preference aggregate"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "other aggregate read: {events}");
    assert_eq!(
        events,
        json!([]),
        "another account reads 0 preference receipts"
    );

    // Same-account read-only principal: reads, never writes.
    if let Some(viewer) = m.viewer().await {
        let viewer = viewer.client();
        let (status, body) = status_and_json(
            viewer
                .put(pref_url.clone())
                .json(&json!({ "value": 9 }))
                .send()
                .await
                .expect("viewer PUT preference"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer PUT preference");
        let status = viewer
            .get(pref_url.clone())
            .send()
            .await
            .expect("viewer GET preference")
            .status();
        assert_eq!(status, StatusCode::OK, "viewer reads the preference");
    }

    // The owner's row is unchanged by every denied request.
    let (record, history, _) = owner_reads(&owner).await;
    assert_eq!(record["record"]["value"], json!(2));
    assert_eq!(record["record"]["revision"], json!(1));
    assert_eq!(history["receipts"].as_array().map(Vec::len), Some(1));

    // Owner reset: a second session-attributed receipt, value back to the registry default.
    let (status, body) = status_and_json(
        owner
            .post(reset_url.clone())
            .send()
            .await
            .expect("owner POST reset"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner reset: {body}");
    assert_eq!(body["record"]["value"], json!(4));
    assert_eq!(body["record"]["revision"], json!(2));
    let (status, events) = status_and_json(
        owner
            .get(aggregate_url.clone())
            .send()
            .await
            .expect("owner GET preference aggregate after reset"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(events.as_array().map(Vec::len), Some(2), "{events}");
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: locus (mt154_locus_routes_require_session)
// ---------------------------------------------------------------------------------------------

/// AC-154-2 + AC-154-4 (D-154-3): the Locus resolve routes need a live account session and the
/// path workspace's read grant, and the lookup runs as the record user. `work_packets` /
/// `micro_tasks` rows are account-owned: a row a root/system writer created (owner NONE) is
/// invisible to every account and indistinguishable from a missing id (no root-read bypass).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_locus_routes_require_session() {
    use handshake_core::storage::StructuredCollaborationStore;
    use handshake_core::workflows::locus::types as locus_types;
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let other_ws = m.other.create_workspace(&m.state).await;

    // A system-written (root) work packet: no owning account.
    m.store
        .db
        .execute_locus_operation(locus_types::LocusOperation::CreateWp(
            locus_types::LocusCreateWpParams {
                wp_id: "WP-MT154-SYSTEM".to_owned(),
                title: "MT-154 system work packet".to_owned(),
                description: "Root-written; no owner_account_id.".to_owned(),
                priority: 1,
                kind: locus_types::WorkPacketType::Test,
                phase: locus_types::WorkPacketPhase::Phase1,
                routing: locus_types::RoutingPolicy::GovStandard,
                task_packet_path: None,
                assignee: None,
                labels: None,
                spec_session_id: None,
                reporter: "mt154-locus-test".to_owned(),
            },
        ))
        .await
        .expect("root seeds a system work packet");

    let wp_url = |ws: &str, id: &str| m.url(&format!("/workspaces/{ws}/locus/work-packets/{id}"));
    let mt_url = |ws: &str, id: &str| m.url(&format!("/workspaces/{ws}/locus/microtasks/{id}"));

    // Anonymous: constant denial on both routes.
    for (what, url) in [
        ("anonymous GET work packet", wp_url(&ws, "WP-MT154-SYSTEM")),
        ("anonymous GET microtask", mt_url(&ws, "MT-MT154")),
    ] {
        let (status, body) = status_and_json(anonymous().get(url).send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // A different account on the owner's workspace: constant denial before any lookup.
    for (what, url) in [
        ("other GET work packet", wp_url(&ws, "WP-MT154-SYSTEM")),
        ("other GET microtask", mt_url(&ws, "MT-MT154")),
    ] {
        let (status, body) = status_and_json(other.get(url).send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // The owner, and the other account on its own workspace, run as record users: the unowned
    // system row is invisible and answers exactly like a missing id.
    for (what, client, url) in [
        (
            "owner system wp",
            owner.clone(),
            wp_url(&ws, "WP-MT154-SYSTEM"),
        ),
        (
            "owner missing wp",
            owner.clone(),
            wp_url(&ws, "WP-MT154-MISSING"),
        ),
        (
            "other own-workspace system wp",
            other.clone(),
            wp_url(&other_ws, "WP-MT154-SYSTEM"),
        ),
        ("owner missing mt", owner.clone(), mt_url(&ws, "MT-MT154")),
    ] {
        let (status, body) = status_and_json(client.get(url).send().await.expect(what)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{what}: {body}");
    }
    let (status, missing) = status_and_json(
        owner
            .get(wp_url(&ws, "WP-MT154-MISSING"))
            .send()
            .await
            .expect("owner missing wp body"),
    )
    .await;
    let (_, system) = status_and_json(
        owner
            .get(wp_url(&ws, "WP-MT154-SYSTEM"))
            .send()
            .await
            .expect("owner system wp body"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        system, missing,
        "an unowned row leaks nothing beyond a missing id"
    );
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: atelier (mt154_atelier_routes_require_session)
// ---------------------------------------------------------------------------------------------

/// AC-154-2 + AC-154-4 (D-154-3): every Atelier route needs a live account session and runs as the
/// account record user. Atelier rows are account-owned (owner_account_id stamped at create): account
/// B cannot list, count, read or write account A's intake batch or items, and B reads 0 of A's
/// Atelier receipts, which carry A's session principal. The loom-projection link additionally needs
/// read on the exact target block, so B cannot pin A's Loom block.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_atelier_routes_require_session() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let batches_url = m.url("/atelier/intake/batches");
    let overview_url = m.url("/atelier/overview");
    let batch_count = |overview: &Value| {
        overview["tables"]
            .as_array()
            .expect("overview tables")
            .iter()
            .find(|table| table["name"] == "atelier_intake_batch")
            .map(|table| table["rows"].clone())
            .expect("atelier_intake_batch count")
    };

    // Anonymous: constant denial on representative read and write routes, no side effect.
    let fake = uuid::Uuid::now_v7();
    for (what, request) in [
        (
            "anonymous GET overview",
            anonymous().get(overview_url.clone()),
        ),
        (
            "anonymous GET batches",
            anonymous().get(batches_url.clone()),
        ),
        (
            "anonymous POST batch",
            anonymous().post(batches_url.clone()).json(&json!({
                "idempotency_key": "mt154-anon-batch",
                "source_label": "MT-154 anonymous"
            })),
        ),
        (
            "anonymous GET command corpus",
            anonymous().get(m.url("/atelier/command-corpus")),
        ),
        (
            "anonymous GET stealth windows",
            anonymous().get(m.url("/atelier/stealth/windows")),
        ),
        (
            "anonymous PUT loom projection",
            anonymous()
                .put(m.url(&format!("/atelier/intake/items/{fake}/loom-projection")))
                .json(&json!({ "loom_block_id": "mt154-anon-block" })),
        ),
    ] {
        let (status, body) = status_and_json(request.send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // Owner positive: batch + item created as the account record user.
    let (status, batch) = status_and_json(
        owner
            .post(batches_url.clone())
            .json(&json!({
                "idempotency_key": "mt154-owner-batch",
                "source_label": "MT-154 owner intake"
            }))
            .send()
            .await
            .expect("owner POST batch"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner POST batch: {batch}");
    let batch_id = batch["batch_id"].as_str().expect("batch id").to_owned();
    let items_url = m.url(&format!("/atelier/intake/batches/{batch_id}/items"));
    let (status, item) = status_and_json(
        owner
            .post(items_url.clone())
            .json(&json!({
                "source_path": "mt154/owner/a.png",
                "file_name": "a.png",
                "byte_len": 10
            }))
            .send()
            .await
            .expect("owner POST item"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner POST item: {item}");
    let item_id = item["item_id"].as_str().expect("item id").to_owned();

    // Canonical re-read as the owner.
    let owner_view = |owner: &reqwest::Client| {
        let owner = owner.clone();
        let batches_url = batches_url.clone();
        let items_url = items_url.clone();
        let overview_url = overview_url.clone();
        async move {
            let (status, batches) = status_and_json(
                owner
                    .get(batches_url)
                    .send()
                    .await
                    .expect("owner GET batches"),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "owner GET batches: {batches}");
            let (status, items) =
                status_and_json(owner.get(items_url).send().await.expect("owner GET items")).await;
            assert_eq!(status, StatusCode::OK, "owner GET items: {items}");
            let (status, overview) = status_and_json(
                owner
                    .get(overview_url)
                    .send()
                    .await
                    .expect("owner GET overview"),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "owner GET overview: {overview}");
            (batches, items, overview)
        }
    };
    let (batches, items, overview) = owner_view(&owner).await;
    assert_eq!(batches.as_array().map(Vec::len), Some(1), "{batches}");
    assert_eq!(batches[0]["batch_id"], batch_id.as_str());
    assert_eq!(items["items"].as_array().map(Vec::len), Some(1), "{items}");
    assert_eq!(items["items"][0]["item_id"], item_id.as_str());
    assert_eq!(items["lane_counts"]["pending"], json!(1));
    assert_eq!(batch_count(&overview), json!(1));

    // The Atelier receipt carries the owner's session principal; another account reads 0 receipts.
    let aggregate_url = m.url(&format!(
        "/kernel/events/aggregates/atelier_intake_batch/{batch_id}"
    ));
    let (status, events) = status_and_json(
        owner
            .get(aggregate_url.clone())
            .send()
            .await
            .expect("owner GET atelier aggregate"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner atelier aggregate: {events}");
    let events = events.as_array().expect("owner atelier receipts").clone();
    assert!(!events.is_empty(), "owner reads its Atelier receipt");
    for event in &events {
        assert_eq!(event["event_type"], "ATELIER_DOMAIN_EVENT_RECORDED");
        assert_eq!(
            event["actor"],
            json!({ "kind": "operator", "id": m.owner.actor_id })
        );
    }
    let (status, events) = status_and_json(
        other
            .get(aggregate_url.clone())
            .send()
            .await
            .expect("other GET atelier aggregate"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "other atelier aggregate: {events}");
    assert_eq!(
        events,
        json!([]),
        "another account reads 0 Atelier receipts"
    );

    // Cross-account isolation (D-154-3): lists, counts and reads never include A's rows.
    let (status, other_batches) = status_and_json(
        other
            .get(batches_url.clone())
            .send()
            .await
            .expect("other GET batches"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "other GET batches: {other_batches}");
    assert_eq!(other_batches, json!([]), "B lists none of A's batches");
    let (status, other_overview) = status_and_json(
        other
            .get(overview_url.clone())
            .send()
            .await
            .expect("other GET overview"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "other GET overview: {other_overview}"
    );
    assert_eq!(
        batch_count(&other_overview),
        json!(0),
        "B's counts exclude A's rows"
    );
    let (status, other_items) = status_and_json(
        other
            .get(items_url.clone())
            .send()
            .await
            .expect("other GET A's items"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "other GET A's items: {other_items}");
    assert_eq!(other_items["items"], json!([]), "B reads none of A's items");
    assert_eq!(other_items["lane_counts"]["pending"], json!(0));
    let (status, body) = status_and_json(
        other
            .post(items_url.clone())
            .json(&json!({
                "source_path": "mt154/other/b.png",
                "file_name": "b.png",
                "byte_len": 5
            }))
            .send()
            .await
            .expect("other POST item into A's batch"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "B cannot write into A's batch (indistinguishable from a missing one): {body}"
    );

    // AC-154-4: B cannot pin A's Loom block through its own intake item.
    let (status, note) = status_and_json(
        owner
            .post(m.url(&format!("/workspaces/{ws}/loom/blocks")))
            .json(&json!({ "content_type": "note", "title": "MT-154 owner note" }))
            .send()
            .await
            .expect("owner creates a Loom note"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner Loom note: {note}");
    let block_id = note["block_id"].as_str().expect("block id").to_owned();
    let (status, other_batch) = status_and_json(
        other
            .post(batches_url.clone())
            .json(&json!({
                "idempotency_key": "mt154-other-batch",
                "source_label": "MT-154 other intake"
            }))
            .send()
            .await
            .expect("other POST own batch"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "other POST own batch: {other_batch}"
    );
    let other_batch_id = other_batch["batch_id"].as_str().expect("other batch id");
    let (status, other_item) = status_and_json(
        other
            .post(m.url(&format!("/atelier/intake/batches/{other_batch_id}/items")))
            .json(&json!({
                "source_path": "mt154/other/c.png",
                "file_name": "c.png",
                "byte_len": 7
            }))
            .send()
            .await
            .expect("other POST own item"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "other POST own item: {other_item}"
    );
    let other_item_id = other_item["item_id"].as_str().expect("other item id");
    let (status, body) = status_and_json(
        other
            .put(m.url(&format!(
                "/atelier/intake/items/{other_item_id}/loom-projection"
            )))
            .json(&json!({ "loom_block_id": block_id }))
            .send()
            .await
            .expect("other links A's block"),
    )
    .await;
    assert_constant_denial(status, &body, "other links A's Loom block");
    // The owner holds read on its own block, so the request passes authorization and reaches the
    // domain rule (a note has no source document or asset), which is not an authority denial.
    let (status, body) = status_and_json(
        owner
            .put(m.url(&format!("/atelier/intake/items/{item_id}/loom-projection")))
            .json(&json!({ "loom_block_id": block_id }))
            .send()
            .await
            .expect("owner links its own note"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "owner link reaches validation: {body}"
    );

    // B's own rows stay private too, and A's rows are unchanged by every denied request.
    let (_, other_batches) = status_and_json(
        other
            .get(batches_url.clone())
            .send()
            .await
            .expect("other GET own batches"),
    )
    .await;
    assert_eq!(other_batches.as_array().map(Vec::len), Some(1));
    assert_eq!(other_batches[0]["batch_id"], other_batch_id);
    let (batches, items, overview) = owner_view(&owner).await;
    assert_eq!(batches.as_array().map(Vec::len), Some(1), "{batches}");
    assert_eq!(batches[0]["batch_id"], batch_id.as_str());
    assert_eq!(items["items"].as_array().map(Vec::len), Some(1), "{items}");
    assert_eq!(items["items"][0]["loom_block_id"], Value::Null);
    assert_eq!(batch_count(&overview), json!(1));
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: knowledge (mt154_knowledge_crdt_ingestion_memory_retrieval_authority)
// ---------------------------------------------------------------------------------------------

/// Backend-navigation identity headers the knowledge routes require (the actor is ignored: every
/// receipt carries the session principal).
fn knowledge_nav(request: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-kind", "system")
        .header("x-hsk-actor-id", "mt154-header-actor")
        .header("x-hsk-kernel-task-run-id", format!("KTR-MT154-{label}"))
        .header("x-hsk-session-run-id", format!("SR-MT154-{label}"))
        .header("x-hsk-correlation-id", format!("CORR-MT154-{label}"))
}

/// One Yjs update envelope for `document_id` from the operator site `op-mt154`.
fn knowledge_crdt_envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    update_id: &str,
) -> Value {
    use base64::Engine;
    use handshake_core::kernel::crdt::actor_site::{
        derive_knowledge_site_id, KnowledgeActorIdV1, KnowledgeActorKind,
    };
    use handshake_core::kernel::crdt::persistence::sha256_hex;
    use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
    use handshake_core::kernel::crdt::yjs_bridge::{
        YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
    };
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt154").expect("actor");
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, &actor);
    let before = KnowledgeStateVectorV1::new();
    let mut after = before.clone();
    after.increment(&site.site_id);
    let bytes = format!("mt154-{update_id}").into_bytes();
    let envelope = YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id: site.site_id,
        session_id: "sr-mt154".to_string(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: "hsk.doc.rich_document@1".to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        update_sha256: sha256_hex(&bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    };
    json!({ "envelope": envelope })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_knowledge_crdt_ingestion_memory_retrieval_authority() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();
    let viewer = m.viewer().await;

    // ---- Ingestion: roots, runs, sources, receipts, repairs (workspace Create/Read + memory.*).
    let roots_url = m.url(&format!("/knowledge/ingestion/roots?workspace_id={ws}"));
    let root_body = json!({
        "workspace_id": ws,
        "display_name": "mt154 root",
        "root_kind": "project_repo",
        "repo_relative_path": "",
    });
    let (status, body) = status_and_json(
        knowledge_nav(
            anonymous().post(m.url("/knowledge/ingestion/roots")),
            "anon-root",
        )
        .json(&root_body)
        .send()
        .await
        .expect("anonymous root register"),
    )
    .await;
    assert_constant_denial(status, &body, "anonymous ingestion root register");
    let (status, body) = status_and_json(
        knowledge_nav(
            other.post(m.url("/knowledge/ingestion/roots")),
            "other-root",
        )
        .json(&root_body)
        .send()
        .await
        .expect("cross-account root register"),
    )
    .await;
    assert_constant_denial(status, &body, "cross-account ingestion root register");
    if let Some(viewer) = &viewer {
        let (status, body) = status_and_json(
            knowledge_nav(
                viewer.client().post(m.url("/knowledge/ingestion/roots")),
                "viewer-root",
            )
            .json(&root_body)
            .send()
            .await
            .expect("viewer root register"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer ingestion root register");
    }
    let (status, body) = status_and_json(
        owner
            .get(&roots_url)
            .send()
            .await
            .expect("owner roots read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner roots read: {body}");
    assert_eq!(
        body["roots"].as_array().map(Vec::len),
        Some(0),
        "denied registrations left no root: {body}"
    );

    let (status, body) = status_and_json(
        knowledge_nav(
            owner.post(m.url("/knowledge/ingestion/roots")),
            "owner-root",
        )
        .json(&root_body)
        .send()
        .await
        .expect("owner root register"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "owner root register: {body}");
    let root_id = body["root"]["root_id"]
        .as_str()
        .expect("root id")
        .to_owned();
    let (status, body) = status_and_json(
        owner
            .get(&roots_url)
            .send()
            .await
            .expect("owner roots re-read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["roots"][0]["root_id"],
        root_id.as_str(),
        "canonical root re-read: {body}"
    );
    let (status, body) = status_and_json(
        other
            .get(&roots_url)
            .send()
            .await
            .expect("other roots read"),
    )
    .await;
    assert_constant_denial(status, &body, "cross-account roots read");
    let (status, body) = status_and_json(
        anonymous()
            .get(&roots_url)
            .send()
            .await
            .expect("anon roots read"),
    )
    .await;
    assert_constant_denial(status, &body, "anonymous roots read");

    let tree = tempfile::tempdir().expect("ingestion temp dir");
    std::fs::create_dir_all(tree.path().join("notes")).expect("notes dir");
    std::fs::write(
        tree.path().join("notes/mt154.md"),
        b"# MT-154\n\nowned note\n",
    )
    .expect("write note");
    let run_body = json!({
        "workspace_id": ws,
        "root_id": root_id,
        "fs_anchor": tree.path().to_string_lossy(),
    });
    for (client, what) in [(anonymous(), "anonymous"), (other.clone(), "cross-account")] {
        let (status, body) = status_and_json(
            knowledge_nav(
                client.post(m.url("/knowledge/ingestion/runs")),
                "denied-run",
            )
            .json(&run_body)
            .send()
            .await
            .expect("denied ingestion run"),
        )
        .await;
        assert_constant_denial(status, &body, &format!("{what} ingestion run"));
    }
    let sources_url = m.url(&format!(
        "/knowledge/ingestion/roots/{root_id}/sources?workspace_id={ws}"
    ));
    let (status, body) = status_and_json(
        owner
            .get(&sources_url)
            .send()
            .await
            .expect("owner sources read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner sources read: {body}");
    assert_eq!(
        body["sources"].as_array().map(Vec::len),
        Some(0),
        "denied runs ingested nothing: {body}"
    );
    let (status, body) = status_and_json(
        knowledge_nav(owner.post(m.url("/knowledge/ingestion/runs")), "owner-run")
            .json(&run_body)
            .send()
            .await
            .expect("owner ingestion run"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner ingestion run: {body}");
    assert_eq!(body["outcomes"].as_array().map(Vec::len), Some(1), "{body}");
    let source_id = body["outcomes"][0]["source_id"]
        .as_str()
        .expect("ingested source id")
        .to_owned();
    let (status, body) = status_and_json(
        owner
            .get(&sources_url)
            .send()
            .await
            .expect("owner sources re-read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["sources"][0]["source_id"],
        source_id.as_str(),
        "canonical re-read: {body}"
    );
    let receipts_url = m.url(&format!(
        "/knowledge/ingestion/sources/{source_id}/receipts?workspace_id={ws}"
    ));
    let (status, body) = status_and_json(
        owner
            .get(&receipts_url)
            .send()
            .await
            .expect("owner receipts"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner extraction receipts: {body}");
    assert_eq!(body["receipts"].as_array().map(Vec::len), Some(1), "{body}");
    let (status, body) = status_and_json(
        other
            .get(&receipts_url)
            .send()
            .await
            .expect("other receipts"),
    )
    .await;
    assert_constant_denial(status, &body, "cross-account extraction receipts");
    let repairs_url = m.url(&format!("/knowledge/ingestion/repairs?workspace_id={ws}"));
    let (status, body) = status_and_json(
        anonymous()
            .get(&repairs_url)
            .send()
            .await
            .expect("anon repairs"),
    )
    .await;
    assert_constant_denial(status, &body, "anonymous repair queue");
    let (status, body) = status_and_json(
        knowledge_nav(
            other.post(
                m.url("/knowledge/ingestion/repairs/KIRQ-00000000000000000000000000000000/retry"),
            ),
            "other-retry",
        )
        .json(&json!({"workspace_id": ws, "fs_anchor": tree.path().to_string_lossy()}))
        .send()
        .await
        .expect("cross-account retry"),
    )
    .await;
    assert_constant_denial(status, &body, "cross-account repair retry");

    // ---- CRDT draft log: exact RichDocument grant (push Update+fs.write, pull/conflict Read).
    let (status, body) = status_and_json(
        owner
            .post(m.url("/knowledge/documents"))
            .header("x-hsk-actor-id", "mt154-doc")
            .header("x-hsk-kernel-task-run-id", "KTR-MT154-doc")
            .header("x-hsk-session-run-id", "SR-MT154-doc")
            .header("x-hsk-actor-kind", "operator")
            .json(&json!({"workspace_id": ws, "title": "mt154 crdt"}))
            .send()
            .await
            .expect("owner document create"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner document create: {body}");
    let doc = body["document"]["rich_document_id"]
        .as_str()
        .expect("rich document id")
        .to_owned();
    let crdt_doc = format!("KCRDT-mt154-{}", uuid::Uuid::now_v7().simple());
    let pull_url = m.url(&format!(
        "/knowledge/crdt/updates/pull?workspace_id={ws}&document_id={doc}&crdt_document_id={crdt_doc}\
         &document_schema_id=hsk.doc.rich_document@1&actor_id=operator:op-mt154&session_id=sr-mt154\
         &correlation_id=corr-mt154"
    ));
    let push = knowledge_crdt_envelope(&ws, &doc, &crdt_doc, "mt154-u1");
    for (client, what) in [(anonymous(), "anonymous"), (other.clone(), "cross-account")] {
        let (status, body) = status_and_json(
            client
                .post(m.url("/knowledge/crdt/updates/push"))
                .json(&push)
                .send()
                .await
                .expect("denied crdt push"),
        )
        .await;
        assert_constant_denial(status, &body, &format!("{what} crdt push"));
    }
    if let Some(viewer) = &viewer {
        let (status, body) = status_and_json(
            viewer
                .client()
                .post(m.url("/knowledge/crdt/updates/push"))
                .json(&push)
                .send()
                .await
                .expect("viewer crdt push"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer crdt push");
    }
    let (status, body) =
        status_and_json(owner.get(&pull_url).send().await.expect("owner pull")).await;
    assert_eq!(status, StatusCode::OK, "owner pull: {body}");
    assert_eq!(
        body["result"]["updates"].as_array().map(Vec::len),
        Some(0),
        "denied pushes stored nothing: {body}"
    );
    let (status, body) = status_and_json(
        owner
            .post(m.url("/knowledge/crdt/updates/push"))
            .json(&push)
            .send()
            .await
            .expect("owner push"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner crdt push: {body}");
    assert_eq!(body["result"]["outcome"], "stored", "{body}");
    let (status, body) =
        status_and_json(owner.get(&pull_url).send().await.expect("owner re-pull")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["result"]["updates"][0]["update_id"], "mt154-u1",
        "canonical re-read: {body}"
    );
    let (status, body) =
        status_and_json(other.get(&pull_url).send().await.expect("other pull")).await;
    assert_constant_denial(status, &body, "cross-account crdt pull");
    let conflict_url = m.url(&format!(
        "/knowledge/crdt/conflict_state?workspace_id={ws}&document_id={doc}&crdt_document_id={crdt_doc}\
         &actor_id=operator:op-mt154&session_id=sr-mt154&correlation_id=corr-mt154"
    ));
    let (status, body) = status_and_json(
        anonymous()
            .get(&conflict_url)
            .send()
            .await
            .expect("anon conflict state"),
    )
    .await;
    assert_constant_denial(status, &body, "anonymous crdt conflict state");
    let (status, body) = status_and_json(
        owner
            .get(&conflict_url)
            .send()
            .await
            .expect("owner conflict state"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner conflict state: {body}");
    assert_eq!(body["result"]["head_update_seq"], 1, "{body}");

    // ---- Memory graph navigation (workspace Read+fs.read; receipt attributed to the session).
    let visual_url = m.url(&format!("/knowledge/memory/visual-debug?workspace_id={ws}"));
    for (client, what) in [(anonymous(), "anonymous"), (other.clone(), "cross-account")] {
        let (status, body) = status_and_json(
            knowledge_nav(client.get(&visual_url), "denied-vd")
                .send()
                .await
                .expect("denied visual debug"),
        )
        .await;
        assert_constant_denial(status, &body, &format!("{what} memory visual-debug"));
    }
    let (status, body) = status_and_json(
        knowledge_nav(owner.get(&visual_url), "owner-vd")
            .send()
            .await
            .expect("owner visual debug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner memory visual-debug: {body}");
    let receipt_id = body["retrieval_receipt_event_id"]
        .as_str()
        .expect("memory nav receipt id")
        .to_owned();
    let aggregate_url = m.url("/kernel/events/aggregates/knowledge_memory_nav/visual_debug");
    let (status, events) = status_and_json(
        owner
            .get(&aggregate_url)
            .send()
            .await
            .expect("owner receipts"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner receipt read: {events}");
    let rows = events.as_array().cloned().unwrap_or_default();
    let row = rows
        .iter()
        .find(|row| row["event_id"] == receipt_id.as_str())
        .unwrap_or_else(|| panic!("the owner reads its own nav receipt: {events}"));
    assert_eq!(
        row["actor"]["kind"], "operator",
        "session principal actor: {row}"
    );
    assert_ne!(
        row["actor"]["id"], "mt154-header-actor",
        "header actor ignored: {row}"
    );
    let (status, events) = status_and_json(
        other
            .get(&aggregate_url)
            .send()
            .await
            .expect("other receipts"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        events.as_array().map(Vec::len),
        Some(0),
        "another account reads none of the owner's receipts: {events}"
    );
    let (status, body) = status_and_json(
        knowledge_nav(
            owner.get(m.url(&format!(
                "/knowledge/memory/claims/KCLM-00000000000000000000000000000000?workspace_id={ws}"
            ))),
            "owner-claim",
        )
        .send()
        .await
        .expect("owner unknown claim"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "unknown claim is a 404: {body}"
    );

    // ---- Retrieval debug (workspace Read+fs.read; repair Create+fs.write).
    let catalog_url = m.url(&format!("/knowledge/retrieval/catalog?workspace_id={ws}"));
    for (client, what) in [(anonymous(), "anonymous"), (other.clone(), "cross-account")] {
        let (status, body) = status_and_json(
            knowledge_nav(client.get(&catalog_url), "denied-catalog")
                .send()
                .await
                .expect("denied catalog"),
        )
        .await;
        assert_constant_denial(status, &body, &format!("{what} retrieval catalog"));
    }
    let (status, body) = status_and_json(
        knowledge_nav(owner.get(&catalog_url), "owner-catalog")
            .send()
            .await
            .expect("owner catalog"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner retrieval catalog: {body}");
    assert!(body["retrieval_receipt_event_id"].is_string(), "{body}");
    let ghost = m.url(&format!(
        "/knowledge/retrieval/bundles/CTX-ffffffffffffffff/staleness?workspace_id={ws}"
    ));
    let (status, body) = status_and_json(
        knowledge_nav(owner.get(&ghost), "owner-ghost")
            .send()
            .await
            .expect("owner ghost bundle"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "unknown bundle is a 404: {body}"
    );
    let repair = m.url(&format!(
        "/knowledge/retrieval/bundles/CTX-ffffffffffffffff/repair?workspace_id={ws}"
    ));
    for (client, what) in [(anonymous(), "anonymous"), (other.clone(), "cross-account")] {
        let (status, body) = status_and_json(
            knowledge_nav(client.post(&repair), "denied-repair")
                .send()
                .await
                .expect("denied repair"),
        )
        .await;
        assert_constant_denial(status, &body, &format!("{what} retrieval repair"));
    }
    if let Some(viewer) = &viewer {
        let (status, body) = status_and_json(
            knowledge_nav(viewer.client().post(&repair), "viewer-repair")
                .send()
                .await
                .expect("viewer repair"),
        )
        .await;
        assert_constant_denial(status, &body, "viewer retrieval repair");
    }
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: trace projection (mt154_trace_projection_requires_session)
// ---------------------------------------------------------------------------------------------

/// GET /kernel/trace_projection reads the EventLedger (a protected resource, Master Spec
/// 02-system-architecture.md:2774): anonymous callers get the constant denial before any read, and
/// an authenticated account asking for a trace it holds no readable receipt of gets the same
/// constant denial (existence is never disclosed), never another account's receipts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_trace_projection_requires_session() {
    let m = Matrix::start().await;
    // The owner's runtime-chat ingestion writes a session-attributed receipt under a known run id.
    let session_id = uuid::Uuid::now_v7().to_string();
    let (status, body) = status_and_json(
        m.owner
            .client()
            .post(m.url(&format!(
                "/workspaces/{}/flight_recorder/runtime_chat_event",
                m.workspace_id
            )))
            .json(&runtime_chat_body(&session_id))
            .send()
            .await
            .expect("owner runtime-chat ingestion"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner runtime-chat ingestion: {body}"
    );
    let path = format!(
        "/kernel/trace_projection?kernel_task_run_id={}&session_run_id={session_id}",
        m.workspace_id
    );

    let (status, body) = status_and_json(
        anonymous()
            .get(m.url(&path))
            .send()
            .await
            .expect("anonymous trace projection"),
    )
    .await;
    assert_constant_denial(status, &body, "anonymous trace projection");

    let (status, body) = status_and_json(
        m.other
            .client()
            .get(m.url(&path))
            .send()
            .await
            .expect("other-account trace projection"),
    )
    .await;
    assert_constant_denial(status, &body, "other account reading the owner's trace");

    let unknown = format!(
        "/kernel/trace_projection?kernel_task_run_id=absent&session_run_id={}",
        uuid::Uuid::now_v7()
    );
    let (status, body) = status_and_json(
        m.owner
            .client()
            .get(m.url(&unknown))
            .send()
            .await
            .expect("owner unknown trace projection"),
    )
    .await;
    assert_constant_denial(status, &body, "owner reading an absent trace");

    // The owner's own receipts are readable through the authenticated aggregate route.
    let fr_event_id = body_str(&runtime_chat_last(&m, &session_id).await, "fr_event_id");
    let (status, events) = status_and_json(
        m.owner
            .client()
            .get(m.url(&format!(
                "/kernel/events/aggregates/runtime_chat_event/{fr_event_id}"
            )))
            .send()
            .await
            .expect("owner receipt read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner receipt read: {events}");
    assert_eq!(
        events.as_array().map(Vec::len),
        Some(1),
        "the owner reads exactly its runtime-chat receipt: {events}"
    );
}

fn runtime_chat_body(session_id: &str) -> Value {
    json!({
        "schema_version": "hsk.runtime_chat@0.1",
        "event_id": uuid::Uuid::now_v7().to_string(),
        "ts_utc": "2026-09-23T12:00:00Z",
        "session_id": session_id,
        "type": "runtime_chat_message_appended",
        "message_id": uuid::Uuid::now_v7().to_string(),
        "role": "user",
    })
}

fn body_str(body: &Value, field: &str) -> String {
    body[field]
        .as_str()
        .unwrap_or_else(|| panic!("response field {field} must be a string: {body}"))
        .to_owned()
}

/// Posts one more runtime-chat event for `session_id` as the owner and returns the response body.
async fn runtime_chat_last(m: &Matrix, session_id: &str) -> Value {
    let (status, body) = status_and_json(
        m.owner
            .client()
            .post(m.url(&format!(
                "/workspaces/{}/flight_recorder/runtime_chat_event",
                m.workspace_id
            )))
            .json(&runtime_chat_body(session_id))
            .send()
            .await
            .expect("owner runtime-chat ingestion"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner runtime-chat ingestion: {body}"
    );
    body
}

// ---------------------------------------------------------------------------------------------
// MT154-FAMILY: flight recorder (mt154_flight_recorder_runtime_chat_receipt_is_session_attributed)
// ---------------------------------------------------------------------------------------------

/// POST /workspaces/:ws/flight_recorder/runtime_chat_event writes its durable receipt as the
/// account record user with the SESSION PRINCIPAL as actor (AC-154-5), the owner reads it and
/// another account reads none of it; anonymous and other-account ingestion are denied with no
/// receipt; and the DuckDB Flight Recorder read route (D-154-2) never returns another account's
/// workspace and refuses unscoped enumeration without fr.read.global.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt154_flight_recorder_runtime_chat_receipt_is_session_attributed() {
    let m = Matrix::start().await;
    let session_id = uuid::Uuid::now_v7().to_string();
    let ingest = m.url(&format!(
        "/workspaces/{}/flight_recorder/runtime_chat_event",
        m.workspace_id
    ));

    // Anonymous and other-account ingestion: denied before any durable write.
    let anonymous_status = anonymous()
        .post(&ingest)
        .json(&runtime_chat_body(&session_id))
        .send()
        .await
        .expect("anonymous ingestion")
        .status();
    assert!(
        anonymous_status == StatusCode::UNAUTHORIZED || anonymous_status == StatusCode::FORBIDDEN,
        "anonymous runtime-chat ingestion must be denied, got {anonymous_status}"
    );
    let other_status = m
        .other
        .client()
        .post(&ingest)
        .json(&runtime_chat_body(&session_id))
        .send()
        .await
        .expect("other-account ingestion")
        .status();
    assert_eq!(
        other_status,
        StatusCode::FORBIDDEN,
        "another account cannot ingest into the owner's workspace"
    );

    // Owner ingestion: one receipt attributed to the session principal.
    let accepted = runtime_chat_last(&m, &session_id).await;
    let fr_event_id = body_str(&accepted, "fr_event_id");
    let aggregate = m.url(&format!(
        "/kernel/events/aggregates/runtime_chat_event/{fr_event_id}"
    ));
    let (status, events) = status_and_json(
        m.owner
            .client()
            .get(&aggregate)
            .send()
            .await
            .expect("owner receipt read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner receipt read: {events}");
    let events = events.as_array().cloned().unwrap_or_default();
    assert_eq!(events.len(), 1, "exactly one receipt: {events:?}");
    let receipt = &events[0];
    assert_eq!(
        receipt["event_id"], accepted["receipt_event_id"],
        "the readable receipt is the one the ingestion returned"
    );
    let actor = receipt["actor"].to_string();
    assert!(
        actor.contains(&m.owner.actor_id),
        "the receipt actor is the session principal {}, got {actor}",
        m.owner.actor_id
    );
    assert!(
        !actor.contains("runtime_chat"),
        "the receipt actor is never the runtime_chat system lane: {actor}"
    );

    let (status, other_events) = status_and_json(
        m.other
            .client()
            .get(&aggregate)
            .send()
            .await
            .expect("other-account receipt read"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "other-account receipt read: {other_events}"
    );
    assert_eq!(
        other_events.as_array().map(Vec::len),
        Some(0),
        "another account reads none of the owner's receipts: {other_events}"
    );

    // D-154-2: DuckDB Flight Recorder reads stay behind fr.read on the path workspace.
    let other_workspace = m.other.create_workspace(&m.state).await;
    let (status, body) = status_and_json(
        m.other
            .client()
            .get(m.url(&format!("/flight_recorder?wsid={}", m.workspace_id)))
            .send()
            .await
            .expect("other-account FR read of the owner's workspace"),
    )
    .await;
    assert!(
        status == StatusCode::FORBIDDEN,
        "another account cannot read the owner's Flight Recorder events: {status} {body}"
    );
    let (status, body) = status_and_json(
        m.other
            .client()
            .get(m.url("/flight_recorder"))
            .send()
            .await
            .expect("other-account unscoped FR read"),
    )
    .await;
    assert!(
        status == StatusCode::FORBIDDEN,
        "unscoped enumeration requires fr.read.global, which no profile holds: {status} {body}"
    );
    let (status, body) = status_and_json(
        m.other
            .client()
            .get(m.url(&format!("/flight_recorder?wsid={other_workspace}")))
            .send()
            .await
            .expect("other-account FR read of its own workspace"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own-workspace FR read: {body}");
    assert!(
        !body.to_string().contains(&m.workspace_id),
        "an own-workspace FR read never returns the owner's workspace events: {body}"
    );
}

// ---------------------------------------------------------------------------------------------
// MT-158: the Locus job path (POST /jobs -> run_job -> locus_store) runs under the account session
// ---------------------------------------------------------------------------------------------

fn mt158_create_wp_request(workspace_id: Option<&str>, wp_id: &str) -> Value {
    use handshake_core::workflows::locus::types as locus_types;
    let inputs = serde_json::to_value(locus_types::LocusCreateWpParams {
        wp_id: wp_id.to_owned(),
        title: format!("MT-158 {wp_id}"),
        description: "Created through POST /jobs by an account session.".to_owned(),
        priority: 1,
        kind: locus_types::WorkPacketType::Test,
        phase: locus_types::WorkPacketPhase::Phase1,
        routing: locus_types::RoutingPolicy::GovStandard,
        task_packet_path: None,
        assignee: None,
        labels: None,
        spec_session_id: None,
        reporter: "mt158-locus-job-test".to_owned(),
    })
    .expect("serialize Locus create params");
    let mut request = json!({
        "job_kind": "locus_operation",
        "protocol_id": "locus_create_wp_v1",
        "job_inputs": inputs,
    });
    if let Some(workspace_id) = workspace_id {
        request["workspace_id"] = json!(workspace_id);
    }
    request
}

async fn mt158_ai_job_rows(m: &Matrix) -> u64 {
    let inspector = m.store.storage.test_inspector();
    let table = inspector
        .table_selector("ai_jobs")
        .await
        .expect("select ai_jobs table");
    inspector
        .row_count(&table, handshake_core::storage::surreal::RowFilter::All)
        .await
        .expect("count ai_jobs rows")
}

/// AC-158-1 (02:2758): POST /jobs for a Locus job needs a live account session holding the named
/// workspace's write grant, checked before any table access. Anonymous, workspace-less and
/// other-account requests get the constant denial and leave no ai_jobs row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_locus_jobs_require_account_session() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let before = mt158_ai_job_rows(&m).await;
    for (what, client, body) in [
        (
            "anonymous Locus job",
            anonymous(),
            mt158_create_wp_request(Some(&ws), "WP-MT158-ANON"),
        ),
        (
            "owner Locus job without a workspace",
            m.owner.client(),
            mt158_create_wp_request(None, "WP-MT158-NOWS"),
        ),
        (
            "other account's Locus job on the owner's workspace",
            m.other.client(),
            mt158_create_wp_request(Some(&ws), "WP-MT158-OTHER"),
        ),
    ] {
        let (status, body) = status_and_json(
            client
                .post(m.url("/jobs"))
                .json(&body)
                .send()
                .await
                .expect(what),
        )
        .await;
        assert_constant_denial(status, &body, what);
    }
    assert_eq!(
        mt158_ai_job_rows(&m).await,
        before,
        "a denied Locus job request must not create an ai_jobs row"
    );
}

/// AC-158-2..4 (02:2773/:2776, LM-RLS-001, D-154-3 extension): a Locus job created by account A runs
/// as A's record user, so the work packet is owned by A. A reads it through the Locus route; account
/// B on B's own workspace gets the same 404 as a missing id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_locus_job_rows_are_private_to_the_creating_account() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let other_ws = m.other.create_workspace(&m.state).await;
    let owner = m.owner.client();
    let other = m.other.client();

    let (status, run) = status_and_json(
        owner
            .post(m.url("/jobs"))
            .json(&mt158_create_wp_request(Some(&ws), "WP-MT158-OWNED"))
            .send()
            .await
            .expect("owner Locus job"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner Locus job accepted: {run}");
    let job_id = run["job_id"]
        .as_str()
        .expect("workflow run names its job")
        .to_owned();

    let wp_url = |ws: &str| {
        m.url(&format!(
            "/workspaces/{ws}/locus/work-packets/WP-MT158-OWNED"
        ))
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    let body = loop {
        let (status, body) = status_and_json(
            owner
                .get(wp_url(&ws))
                .send()
                .await
                .expect("owner reads its work packet"),
        )
        .await;
        if status == StatusCode::OK {
            break body;
        }
        if std::time::Instant::now() > deadline {
            // Diagnostic only (root read of the job row; not proof).
            let job = m.state.storage.get_ai_job(&job_id).await;
            panic!("owner never saw its Locus work packet: last {status} {body}; job {job:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    };
    assert_eq!(body["title"], "MT-158 WP-MT158-OWNED", "{body}");

    let (status, missing) = status_and_json(
        other
            .get(m.url(&format!(
                "/workspaces/{other_ws}/locus/work-packets/WP-MT158-MISSING"
            )))
            .send()
            .await
            .expect("other reads a missing id"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, foreign) = status_and_json(
        other
            .get(wp_url(&other_ws))
            .send()
            .await
            .expect("other reads the owner's work packet through its own workspace"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{foreign}");
    assert_eq!(
        foreign, missing,
        "another account's work packet is indistinguishable from a missing id"
    );
}

// ---------------------------------------------------------------------------------------------
// MT-159: every /jobs route and job kind runs under the account session; Locus ids are per account
// ---------------------------------------------------------------------------------------------

/// AC-159-1..3 (02:2758/:2759/:2773): every /jobs route requires the account session. A job the
/// caller cannot see answers exactly like an unknown id, lists hold only visible jobs, and a
/// non-Locus job kind needs a named, authorized workspace too.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt159_job_routes_require_account_session() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let owner = m.owner.client();
    let other = m.other.client();

    let (status, run) = status_and_json(
        owner
            .post(m.url("/jobs"))
            .json(&mt158_create_wp_request(Some(&ws), "WP-MT159-JOB"))
            .send()
            .await
            .expect("owner job"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner job accepted: {run}");
    let job_id = run["job_id"].as_str().expect("job id").to_owned();
    let unknown = uuid::Uuid::now_v7().to_string();

    let (status, body) = status_and_json(
        owner
            .get(m.url(&format!("/jobs/{job_id}")))
            .send()
            .await
            .expect("owner reads its job"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["job_id"], json!(job_id));
    let (status, list) =
        status_and_json(owner.get(m.url("/jobs")).send().await.expect("owner lists")).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert!(
        list.to_string().contains(&job_id),
        "owner list holds its job: {list}"
    );

    // Anonymous: every route is the constant denial.
    for (what, request) in [
        ("anonymous GET /jobs", anonymous().get(m.url("/jobs"))),
        (
            "anonymous GET /jobs/:id",
            anonymous().get(m.url(&format!("/jobs/{job_id}"))),
        ),
        (
            "anonymous resume",
            anonymous().post(m.url(&format!("/jobs/{job_id}/resume"))),
        ),
        (
            "anonymous consent",
            anonymous()
                .post(m.url(&format!("/jobs/{job_id}/cloud_escalation/consent")))
                .json(&json!({"request_id": "r", "approved": true, "user_id": "u"})),
        ),
        (
            "anonymous POST /jobs",
            anonymous()
                .post(m.url("/jobs"))
                .json(&json!({"job_kind": "terminal_exec", "protocol_id": "protocol-default", "workspace_id": ws})),
        ),
    ] {
        let (status, body) = status_and_json(request.send().await.expect(what)).await;
        assert_constant_denial(status, &body, what);
    }

    // Another account: the owner's job is indistinguishable from an unknown id on every route.
    for (what, path, unknown_path) in [
        (
            "other GET /jobs/:id",
            format!("/jobs/{job_id}"),
            format!("/jobs/{unknown}"),
        ),
        (
            "other resume",
            format!("/jobs/{job_id}/resume"),
            format!("/jobs/{unknown}/resume"),
        ),
        (
            "other consent",
            format!("/jobs/{job_id}/cloud_escalation/consent"),
            format!("/jobs/{unknown}/cloud_escalation/consent"),
        ),
    ] {
        let send = |path: String| {
            let request = if what == "other GET /jobs/:id" {
                other.get(m.url(&path))
            } else {
                other
                    .post(m.url(&path))
                    .json(&json!({"request_id": "r", "approved": true, "user_id": "u"}))
            };
            async move { status_and_json(request.send().await.expect("other request")).await }
        };
        let (status, foreign) = send(path).await;
        assert_constant_denial(status, &foreign, what);
        let (status, missing) = send(unknown_path).await;
        assert_constant_denial(status, &missing, what);
        assert_eq!(
            foreign, missing,
            "{what}: foreign and unknown ids answer alike"
        );
    }
    let (status, list) =
        status_and_json(other.get(m.url("/jobs")).send().await.expect("other lists")).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert!(
        !list.to_string().contains(&job_id),
        "another account's list never holds the owner's job: {list}"
    );

    // A non-Locus job kind without a workspace, or on the owner's workspace from another account,
    // is denied before any table access.
    for (what, client, body) in [
        (
            "owner terminal job without a workspace",
            owner.clone(),
            json!({"job_kind": "terminal_exec", "protocol_id": "protocol-default"}),
        ),
        (
            "other terminal job on the owner's workspace",
            other.clone(),
            json!({"job_kind": "terminal_exec", "protocol_id": "protocol-default", "workspace_id": ws}),
        ),
    ] {
        let (status, body) = status_and_json(
            client
                .post(m.url("/jobs"))
                .json(&body)
                .send()
                .await
                .expect(what),
        )
        .await;
        assert_constant_denial(status, &body, what);
    }
}

/// Polls the Locus resolve route as `client` until the work packet is visible (200) or 300 s pass.
async fn mt159_wait_for_work_packet(
    m: &Matrix,
    client: &reqwest::Client,
    workspace_id: &str,
    wp_id: &str,
) -> Value {
    let url = m.url(&format!(
        "/workspaces/{workspace_id}/locus/work-packets/{wp_id}"
    ));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    loop {
        let (status, body) = status_and_json(
            client
                .get(url.clone())
                .send()
                .await
                .expect("read work packet"),
        )
        .await;
        if status == StatusCode::OK {
            return body;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "work packet {wp_id} never became visible: last {status} {body}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// AC-159-4/5 (02:2752/:2759, LM-RLS-001): Locus ids are owner-scoped. Two accounts create the
/// same wp_id through POST /jobs; both creates succeed, each account reads only its own row, and
/// neither create reveals the other account's id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt159_locus_ids_do_not_reveal_existence_across_accounts() {
    let m = Matrix::start().await;
    let ws = m.workspace_id.clone();
    let other_ws = m.other.create_workspace(&m.state).await;
    let owner = m.owner.client();
    let other = m.other.client();
    let shared = "WP-MT159-SHARED";

    let mut owner_request = mt158_create_wp_request(Some(&ws), shared);
    owner_request["job_inputs"]["title"] = json!("owner's packet");
    let mut other_request = mt158_create_wp_request(Some(&other_ws), shared);
    other_request["job_inputs"]["title"] = json!("other's packet");
    for (what, client, request) in [
        ("owner create", &owner, &owner_request),
        ("other create of the same id", &other, &other_request),
    ] {
        let (status, run) = status_and_json(
            client
                .post(m.url("/jobs"))
                .json(request)
                .send()
                .await
                .expect(what),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{what}: {run}");
    }

    let owner_row = mt159_wait_for_work_packet(&m, &owner, &ws, shared).await;
    let other_row = mt159_wait_for_work_packet(&m, &other, &other_ws, shared).await;
    assert_eq!(owner_row["title"], "owner's packet", "{owner_row}");
    assert_eq!(
        other_row["title"], "other's packet",
        "the second account's create succeeded with its own row: {other_row}"
    );
}
