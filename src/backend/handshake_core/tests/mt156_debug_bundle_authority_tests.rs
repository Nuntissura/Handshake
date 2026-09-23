//! WP-KERNEL-012 MT-156: debug bundle export / status / validate / download / exportable run under
//! the authenticated account session through the real mounted router against an isolated embedded
//! SurrealDB store (Master Spec 02-system-architecture.md:2758 deny by default, :2773 no privileged
//! protected flows, :2774 Flight Recorder / diagnostics are protected resources, :2776).
#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

#[path = "account_session_support/mod.rs"]
mod account_session_support;

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

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
use handshake_core::storage::surreal::RowFilter;
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
            profile: ModelProfile::new("mt156-bundle-authority".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// Isolated store + full router on loopback + owner A (with a workspace) + account B (with its own
/// workspace), both bound to the same live native binding.
struct Harness {
    store: EmbeddedKnowledgeStore,
    base: String,
    owner: OwnerSession,
    other: OwnerSession,
    owner_workspace: String,
    other_workspace: String,
    server: Option<tokio::task::JoinHandle<()>>,
    _binding: NativeBindingEnv,
    _lock: tokio::sync::MutexGuard<'static, ()>,
}

impl Harness {
    async fn start() -> Self {
        let lock = NATIVE_BINDING_ENV_LOCK.lock().await;
        let binding = NativeBindingEnv::install();
        let store = open_embedded_store()
            .await
            .expect("isolated embedded store is required for the MT-156 bundle proof");
        let owner = OwnerSession::provision(&store.storage, binding.token()).await;
        let other = OwnerSession::provision(&store.storage, binding.token()).await;
        let state = app_state(&store);
        let owner_workspace = owner.create_workspace(&state).await;
        let other_workspace = other.create_workspace(&state).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind MT-156 loopback listener");
        let addr = listener.local_addr().expect("MT-156 listener addr");
        let app = handshake_core::api::routes(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("MT-156 route server");
        });
        Self {
            store,
            base: format!("http://{addr}"),
            owner,
            other,
            owner_workspace,
            other_workspace,
            server: Some(server),
            _binding: binding,
            _lock: lock,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn ai_job_rows(&self) -> u64 {
        let inspector = self.store.storage.test_inspector();
        let table = inspector
            .table_selector("ai_jobs")
            .await
            .expect("select ai_jobs table");
        inspector
            .row_count(&table, RowFilter::All)
            .await
            .expect("count ai_jobs rows")
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
    }
}

fn workspace_export(wsid: &str) -> Value {
    json!({
        "scope": {"kind": "workspace", "wsid": wsid},
        "redaction_mode": "SAFE_DEFAULT",
    })
}

async fn status_and_text(response: reqwest::Response) -> (StatusCode, String) {
    let status = response.status();
    let text = response.text().await.expect("read response body");
    (status, text)
}

fn assert_denied(status: StatusCode, body: &str, what: &str) {
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{what}: expected 403, got {body}"
    );
    assert!(
        body.contains("HSK-403-PROTECTED-RESOURCE"),
        "{what}: constant denial body, got {body}"
    );
}

/// Owner A exports its own workspace and waits for the bundle to be ready.
async fn owner_export_ready(h: &Harness) -> String {
    let (status, body) = status_and_text(
        h.owner
            .client()
            .post(h.url("/api/bundles/debug/export"))
            .json(&workspace_export(&h.owner_workspace))
            .send()
            .await
            .expect("owner export"),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "owner export: {body}");
    let bundle_id = serde_json::from_str::<Value>(&body).expect("export JSON")["export_job_id"]
        .as_str()
        .expect("export_job_id")
        .to_owned();
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let (status, body) = status_and_text(
            h.owner
                .client()
                .get(h.url(&format!("/api/bundles/debug/{bundle_id}")))
                .send()
                .await
                .expect("owner bundle status"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owner bundle status: {body}");
        let parsed: Value = serde_json::from_str(&body).expect("status JSON");
        match parsed["status"].as_str() {
            Some("ready") => return bundle_id,
            Some("failed") | Some("expired") => panic!("owner export did not complete: {body}"),
            _ => {}
        }
        assert!(
            std::time::Instant::now() < deadline,
            "owner export not ready within 120 s: {body}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_anonymous_bundle_routes_denied() {
    let h = Harness::start().await;
    let jobs_before = h.ai_job_rows().await;
    let bundle = uuid::Uuid::now_v7().to_string();
    let anonymous = reqwest::Client::new();
    for (what, request) in [
        (
            "anonymous export",
            anonymous
                .post(h.url("/api/bundles/debug/export"))
                .json(&workspace_export(&h.owner_workspace)),
        ),
        (
            "anonymous exportable",
            anonymous.get(h.url(&format!(
                "/api/bundles/debug/exportable?wsid={}",
                h.owner_workspace
            ))),
        ),
        (
            "anonymous status",
            anonymous.get(h.url(&format!("/api/bundles/debug/{bundle}"))),
        ),
        (
            "anonymous validate",
            anonymous.post(h.url(&format!("/api/bundles/debug/{bundle}/validate"))),
        ),
        (
            "anonymous download",
            anonymous.get(h.url(&format!("/api/bundles/debug/{bundle}/download"))),
        ),
    ] {
        let (status, body) = status_and_text(request.send().await.expect(what)).await;
        assert_denied(status, &body, what);
    }
    assert_eq!(
        h.ai_job_rows().await,
        jobs_before,
        "anonymous requests create no job row"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_cross_account_cannot_export_or_read_bundle() {
    let h = Harness::start().await;
    let bundle_id = owner_export_ready(&h).await;
    let jobs_before = h.ai_job_rows().await;
    let other = h.other.client();

    let (status, body) = status_and_text(
        other
            .post(h.url("/api/bundles/debug/export"))
            .json(&workspace_export(&h.owner_workspace))
            .send()
            .await
            .expect("other export of the owner's workspace"),
    )
    .await;
    assert_denied(status, &body, "account B exporting account A's workspace");
    assert_eq!(
        h.ai_job_rows().await,
        jobs_before,
        "a denied export creates no ai_jobs row"
    );

    for (what, request) in [
        (
            "account B status of A's bundle",
            other.get(h.url(&format!("/api/bundles/debug/{bundle_id}"))),
        ),
        (
            "account B validate of A's bundle",
            other.post(h.url(&format!("/api/bundles/debug/{bundle_id}/validate"))),
        ),
        (
            "account B download of A's bundle",
            other.get(h.url(&format!("/api/bundles/debug/{bundle_id}/download"))),
        ),
        (
            "account B inventory of A's workspace",
            other.get(h.url(&format!(
                "/api/bundles/debug/exportable?wsid={}",
                h.owner_workspace
            ))),
        ),
    ] {
        let (status, body) = status_and_text(request.send().await.expect(what)).await;
        assert_denied(status, &body, what);
    }

    // B's own inventory never contains A's items.
    let (status, body) = status_and_text(
        other
            .get(h.url(&format!(
                "/api/bundles/debug/exportable?wsid={}",
                h.other_workspace
            )))
            .send()
            .await
            .expect("account B own inventory"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "account B own inventory: {body}");
    assert!(
        !body.contains(&h.owner_workspace) && !body.contains(&bundle_id),
        "account B's inventory contains none of account A's items: {body}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_owner_export_download_round_trip_contains_only_owner_workspace() {
    let h = Harness::start().await;
    let bundle_id = owner_export_ready(&h).await;
    let response = h
        .owner
        .client()
        .get(h.url(&format!("/api/bundles/debug/{bundle_id}/download")))
        .send()
        .await
        .expect("owner download");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.bytes().await.expect("download bytes");
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("bundle zip");
    let mut manifest = String::new();
    archive
        .by_name("bundle_manifest.json")
        .expect("zip carries bundle_manifest.json")
        .read_to_string(&mut manifest)
        .expect("read manifest");
    let manifest: Value = serde_json::from_str(&manifest).expect("manifest JSON");
    assert_eq!(
        manifest["scope"]["wsid"], h.owner_workspace,
        "the manifest names exactly the owner's workspace: {manifest}"
    );
    assert!(
        !manifest.to_string().contains(&h.other_workspace),
        "the manifest never names another account's workspace"
    );
    let (status, body) = status_and_text(
        h.owner
            .client()
            .post(h.url(&format!("/api/bundles/debug/{bundle_id}/validate")))
            .send()
            .await
            .expect("owner validate"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner validate: {body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_unresolvable_scope_denied_without_job_row() {
    let h = Harness::start().await;
    let jobs_before = h.ai_job_rows().await;
    for (what, request) in [
        (
            "time_window without wsid",
            json!({
                "scope": {"kind": "time_window", "time_range": {"start": "2026-09-01T00:00:00Z", "end": "2026-09-02T00:00:00Z"}},
                "redaction_mode": "SAFE_DEFAULT",
            }),
        ),
        (
            "problem without wsid",
            json!({"scope": {"kind": "problem", "problem_id": "diag-mt156"}, "redaction_mode": "SAFE_DEFAULT"}),
        ),
        (
            "job that does not exist",
            json!({"scope": {"kind": "job", "job_id": uuid::Uuid::now_v7().to_string()}, "redaction_mode": "SAFE_DEFAULT"}),
        ),
    ] {
        let (status, body) = status_and_text(
            h.owner
                .client()
                .post(h.url("/api/bundles/debug/export"))
                .json(&request)
                .send()
                .await
                .expect(what),
        )
        .await;
        assert_denied(status, &body, what);
    }
    let (status, body) = status_and_text(
        h.owner
            .client()
            .get(h.url("/api/bundles/debug/exportable"))
            .send()
            .await
            .expect("unscoped inventory"),
    )
    .await;
    assert_denied(status, &body, "unscoped exportable inventory");
    assert_eq!(
        h.ai_job_rows().await,
        jobs_before,
        "unresolvable scopes create no job row"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_bundle_id_path_traversal_rejected() {
    let h = Harness::start().await;
    for bundle in [
        "..%2F..%2Fsecret",
        "bundle-..",
        "00000000-0000-0000-0000-00000000000",
        "not-a-uuid",
    ] {
        for path in [
            format!("/api/bundles/debug/{bundle}"),
            format!("/api/bundles/debug/{bundle}/download"),
        ] {
            let (status, body) = status_and_text(
                h.owner
                    .client()
                    .get(h.url(&path))
                    .send()
                    .await
                    .expect("traversal attempt"),
            )
            .await;
            assert!(
                status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
                "{path}: a non-UUID bundle id is rejected before any path join, got {status} {body}"
            );
            assert!(
                !body.contains("PK"),
                "{path}: no bundle bytes are ever returned"
            );
        }
    }
}
