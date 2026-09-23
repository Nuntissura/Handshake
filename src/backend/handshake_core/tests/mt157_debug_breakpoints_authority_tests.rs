//! WP-KERNEL-012 MT-157: durable debugger breakpoints (`GET`/`PUT
//! /debug/documents/:rich_document_id/breakpoints`) run under the authenticated account session
//! through the real mounted router against an isolated embedded SurrealDB store.
//!
//! Master Spec 02-system-architecture.md:2758 (deny by default on every executable backend
//! boundary), :2773 (privileged sessions never execute ordinary protected-resource flows), :2776
//! (record-user table permissions plus ResourceBroker are the non-bypassable data boundary),
//! LM-RLS-002 (authority tables default to `PERMISSIONS NONE`; every grant explicit). Every positive
//! path runs as an authenticated record user; denied writes are proven by re-reading the owner's
//! set (SurrealDB 3.2 silently ignores a denied write).
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
use handshake_core::storage::surreal::{SurrealDatabase, SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::Database;
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

fn app_state(db: SurrealDatabase, surreal: SurrealStorage) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(db),
        surreal,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt157-breakpoint-authority".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn serve(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MT-157 loopback listener");
    let addr = listener.local_addr().expect("MT-157 listener addr");
    let app = handshake_core::api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("MT-157 route server");
    });
    (format!("http://{addr}"), server)
}

/// Isolated store + full router on loopback + owner A (workspace + rich document created through
/// the routes) + a second account B, both bound to the same live native binding.
struct Harness {
    store: EmbeddedKnowledgeStore,
    state: AppState,
    base: String,
    owner: OwnerSession,
    other: OwnerSession,
    workspace_id: String,
    document_id: String,
    server: Option<tokio::task::JoinHandle<()>>,
    /// The store handle opened by [`Harness::reopen_store`]; shut down by [`Harness::finish`].
    reopened: Option<SurrealStorage>,
    // Drop order: binding (restores the env) before the lock.
    _binding: NativeBindingEnv,
    _lock: tokio::sync::MutexGuard<'static, ()>,
}

impl Harness {
    async fn start() -> Self {
        let lock = NATIVE_BINDING_ENV_LOCK.lock().await;
        let binding = NativeBindingEnv::install();
        let store = open_embedded_store()
            .await
            .expect("isolated embedded store is required for the MT-157 breakpoint proof");
        let owner = OwnerSession::provision(&store.storage, binding.token()).await;
        let other = OwnerSession::provision(&store.storage, binding.token()).await;
        let state = app_state(store.db.clone(), store.storage.clone());
        let workspace_id = owner.create_workspace(&state).await;
        let (base, server) = serve(state.clone()).await;
        let (status, body) = status_and_json(
            owner
                .client()
                .post(format!("{base}/knowledge/documents"))
                .header("x-hsk-actor-id", "mt157-doc")
                .header("x-hsk-kernel-task-run-id", "KTR-MT157-doc")
                .header("x-hsk-session-run-id", "SR-MT157-doc")
                .header("x-hsk-actor-kind", "operator")
                .json(&json!({"workspace_id": workspace_id, "title": "mt157 breakpoints"}))
                .send()
                .await
                .expect("owner document create"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owner document create: {body}");
        let document_id = body["document"]["rich_document_id"]
            .as_str()
            .expect("rich document id")
            .to_owned();
        Self {
            store,
            state,
            base,
            owner,
            other,
            workspace_id,
            document_id,
            server: Some(server),
            reopened: None,
            _binding: binding,
            _lock: lock,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn breakpoints_url(&self) -> String {
        self.url(&format!(
            "/debug/documents/{}/breakpoints",
            self.document_id
        ))
    }

    fn receipts_url(&self) -> String {
        self.url(&format!(
            "/kernel/events/aggregates/debug_breakpoints/{}",
            self.document_id
        ))
    }

    async fn put(&self, client: &reqwest::Client, breakpoints: Value) -> (StatusCode, Value) {
        status_and_json(
            client
                .put(self.breakpoints_url())
                .json(&json!({"workspace_id": self.workspace_id, "breakpoints": breakpoints}))
                .send()
                .await
                .expect("send PUT breakpoints"),
        )
        .await
    }

    async fn get(&self, client: &reqwest::Client) -> (StatusCode, Value) {
        status_and_json(
            client
                .get(self.breakpoints_url())
                .send()
                .await
                .expect("send GET breakpoints"),
        )
        .await
    }

    /// The owner's canonical set, re-read as the owner record user through the route.
    async fn owner_set(&self) -> Vec<Value> {
        let (status, body) = self.get(&self.owner.client()).await;
        assert_eq!(status, StatusCode::OK, "owner GET breakpoints: {body}");
        body["breakpoints"]
            .as_array()
            .expect("breakpoints array")
            .clone()
    }

    async fn receipts(&self, client: &reqwest::Client) -> (StatusCode, Value) {
        status_and_json(
            client
                .get(self.receipts_url())
                .send()
                .await
                .expect("send GET breakpoint receipts"),
        )
        .await
    }

    /// Close the embedded store, reopen a fresh process-local handle on the SAME data directory,
    /// and serve a rebuilt router over it (no handle or cache survives the close).
    async fn reopen_store(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
            let _ = server.await;
        }
        self.store
            .shutdown()
            .await
            .expect("close the embedded store before reopen");
        let config = SurrealStorageConfig::for_data_dir(&self.store.data_dir)
            .expect("store config for the same data directory");
        let storage = SurrealStorage::open(config)
            .await
            .expect("reopen the embedded store on the same data directory");
        self.state = app_state(SurrealDatabase::new(storage.clone()), storage.clone());
        let (base, server) = serve(self.state.clone()).await;
        self.base = base;
        self.server = Some(server);
        self.reopened = Some(storage);
    }

    /// The same Owner account's read-only principal: a second principal in the owner's account and
    /// access space whose only grants are Read + fs.read on the workspace and on the document.
    async fn read_only_principal(&self) -> OwnerSession {
        use handshake_core::storage::surreal::resource_authority::{
            ProvisionedIdentity, ResourceAction, ResourceGrantSpec, ResourceKind,
        };
        use sha2::{Digest, Sha256};
        let viewer_key = format!("mt157-viewer-{}", uuid::Uuid::now_v7());
        let capabilities = vec!["fs.read".to_owned()];
        let storage = &self.store.storage;
        let principal = storage
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
            .expect("provision the same-account read-only principal");
        assert_eq!(principal.identity.account_id, self.owner.account_id);
        assert_eq!(
            principal.identity.access_space_id,
            self.owner.access_space_id
        );
        let owner_identity = ProvisionedIdentity {
            account_id: self.owner.account_id.clone(),
            principal_id: self.owner.principal_id.clone(),
            access_space_id: self.owner.access_space_id.clone(),
        };
        let workspace_resource = storage
            .register_workspace_resource(&owner_identity, &self.workspace_id)
            .await
            .expect("owner workspace protected resource");
        let document_resource = storage
            .register_protected_resource(
                &owner_identity,
                ResourceKind::RichDocument,
                &self.document_id,
                Some(&workspace_resource.resource_id),
                "private",
            )
            .await
            .expect("owner rich document protected resource");
        for resource_id in [
            workspace_resource.resource_id,
            document_resource.resource_id,
        ] {
            storage
                .grant_resource(
                    &self.owner.account_id,
                    &self.owner.access_space_id,
                    ResourceGrantSpec {
                        principal_id: principal.identity.principal_id.clone(),
                        resource_id,
                        actions: vec![ResourceAction::Read],
                        capability_ids: capabilities.clone(),
                        expires_at: None,
                        delegation_chain: Vec::new(),
                    },
                )
                .await
                .expect("grant the read-only principal Read + fs.read");
        }
        OwnerSession {
            session_token: principal.session.token,
            channel_binding_token: self.owner.channel_binding_token.clone(),
            account_id: principal.identity.account_id,
            principal_id: principal.identity.principal_id,
            access_space_id: principal.identity.access_space_id,
            session_id: principal.session.session_id,
            actor_id: viewer_key,
        }
    }

    /// Stop the server and close a reopened store handle so the isolated directory can be removed.
    async fn finish(mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
            let _ = server.await;
        }
        if let Some(reopened) = self.reopened.take() {
            reopened
                .shutdown()
                .await
                .expect("close the reopened embedded store");
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
    }
}

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
        body,
        &json!({"error": "HSK-403-PROTECTED-RESOURCE"}),
        "{what}: constant denial body"
    );
}

/// (source_url, line, condition, verified) of each returned breakpoint, in response order.
fn shape(set: &[Value]) -> Vec<(String, i64, Option<String>, bool)> {
    set.iter()
        .map(|bp| {
            (
                bp["source_url"].as_str().expect("source_url").to_owned(),
                bp["line"].as_i64().expect("line"),
                bp["condition"].as_str().map(str::to_owned),
                bp["verified"].as_bool().expect("verified"),
            )
        })
        .collect()
}

fn initial_set() -> Value {
    json!([
        {"source_url": "file:///mt157/b.js", "line": 3, "condition": null, "verified": false},
        {"source_url": "file:///mt157/a.js", "line": 12, "condition": "n > 2", "verified": true},
        {"source_url": "file:///mt157/a.js", "line": 4, "condition": null, "verified": true}
    ])
}

fn initial_shape() -> Vec<(String, i64, Option<String>, bool)> {
    vec![
        ("file:///mt157/a.js".to_owned(), 4, None, true),
        (
            "file:///mt157/a.js".to_owned(),
            12,
            Some("n > 2".to_owned()),
            true,
        ),
        ("file:///mt157/b.js".to_owned(), 3, None, false),
    ]
}

/// AC-157-2/-3/-6: the owner's PUT persists through the account session; after closing and
/// reopening the embedded store on the same data directory, GET returns the same set in
/// (source_url, line) order with condition and verified preserved.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_owner_breakpoints_persist_across_store_reopen() {
    let mut h = Harness::start().await;
    assert!(h.owner_set().await.is_empty(), "no breakpoints before PUT");

    let (status, body) = h.put(&h.owner.client(), initial_set()).await;
    assert_eq!(status, StatusCode::OK, "owner PUT breakpoints: {body}");
    let stored = body["breakpoints"].as_array().expect("stored set").clone();
    assert_eq!(
        shape(&stored),
        initial_shape(),
        "PUT returns the ordered set"
    );
    for bp in &stored {
        assert_eq!(bp["rich_document_id"], json!(h.document_id));
        assert_eq!(bp["workspace_id"], json!(h.workspace_id));
    }
    assert_eq!(shape(&h.owner_set().await), initial_shape());

    h.reopen_store().await;

    let reloaded = h.owner_set().await;
    assert_eq!(
        shape(&reloaded),
        initial_shape(),
        "breakpoints survive closing and reopening the store"
    );
    let ids = |set: &[Value]| -> Vec<String> {
        set.iter()
            .map(|bp| bp["breakpoint_id"].as_str().expect("id").to_owned())
            .collect()
    };
    assert_eq!(
        ids(&reloaded),
        ids(&stored),
        "same durable rows after reopen"
    );
    // The reopened store still enforces the account boundary.
    let (status, body) = h.get(&h.other.client()).await;
    assert_constant_denial(status, &body, "cross-account GET after reopen");
    h.finish().await;
}

/// AC-157-4/-6: a second PUT replaces the whole set (no leftovers); a repeated (source_url, line)
/// pair is refused by the unique index and rolls the PUT back; the owner can still delete the
/// workspace that holds breakpoints (the table cascades from the document/workspace).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_put_replaces_set_and_keeps_uniqueness() {
    let h = Harness::start().await;
    let owner = h.owner.client();
    let (status, body) = h.put(&owner, initial_set()).await;
    assert_eq!(status, StatusCode::OK, "first PUT: {body}");

    let replacement = json!([
        {"source_url": "file:///mt157/c.js", "line": 1, "condition": "ok", "verified": true}
    ]);
    let (status, body) = h.put(&owner, replacement).await;
    assert_eq!(status, StatusCode::OK, "replacing PUT: {body}");
    let expected = vec![(
        "file:///mt157/c.js".to_owned(),
        1,
        Some("ok".to_owned()),
        true,
    )];
    assert_eq!(
        shape(body["breakpoints"].as_array().expect("set")),
        expected
    );
    assert_eq!(shape(&h.owner_set().await), expected, "no leftovers");

    let duplicate = json!([
        {"source_url": "file:///mt157/d.js", "line": 9, "condition": null, "verified": false},
        {"source_url": "file:///mt157/d.js", "line": 9, "condition": "again", "verified": true}
    ]);
    let (status, body) = h.put(&owner, duplicate).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "duplicate (source_url, line) is refused: {body}"
    );
    assert_eq!(
        shape(&h.owner_set().await),
        expected,
        "the refused PUT rolled back and left the previous set"
    );

    let (status, body) = h.put(&owner, json!([])).await;
    assert_eq!(status, StatusCode::OK, "clearing PUT: {body}");
    assert!(h.owner_set().await.is_empty(), "empty PUT clears the set");

    let (status, body) = h
        .put(
            &owner,
            json!([{"source_url": "file:///mt157/e.js", "line": 2, "verified": true}]),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "PUT before workspace delete: {body}"
    );
    let response = owner
        .delete(h.url(&format!("/workspaces/{}", h.workspace_id)))
        .send()
        .await
        .expect("owner DELETE workspace");
    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "owner deletes a workspace that holds breakpoints"
    );
    // Verification-only root read of the cascade result (not a privacy proof).
    assert!(
        h.state
            .storage
            .list_debug_breakpoints(&h.document_id)
            .await
            .expect("read breakpoints after workspace delete")
            .is_empty(),
        "workspace delete cascades the document's breakpoints"
    );
}

/// AC-157-7: anonymous GET/PUT and another account's GET/PUT are the constant denial, and none of
/// them changes the owner's set (re-read as the owner).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_anonymous_and_cross_account_denied_without_side_effects() {
    let h = Harness::start().await;
    let (status, body) = h.put(&h.owner.client(), initial_set()).await;
    assert_eq!(status, StatusCode::OK, "owner PUT: {body}");
    let before = h.owner_set().await;
    assert_eq!(shape(&before), initial_shape());

    let hostile = json!([
        {"source_url": "file:///mt157/evil.js", "line": 66, "condition": null, "verified": true}
    ]);
    for (client, what) in [
        (anonymous(), "anonymous"),
        (h.other.client(), "cross-account"),
    ] {
        let (status, body) = h.get(&client).await;
        assert_constant_denial(status, &body, &format!("{what} GET"));
        assert!(
            body.get("breakpoints").is_none(),
            "{what} GET never returns the owner's breakpoints"
        );
        let (status, body) = h.put(&client, hostile.clone()).await;
        assert_constant_denial(status, &body, &format!("{what} PUT"));
        let (status, body) = h.put(&client, json!([])).await;
        assert_constant_denial(status, &body, &format!("{what} clearing PUT"));
        assert_eq!(
            h.owner_set().await,
            before,
            "{what} requests leave the owner's set unchanged"
        );
    }
    // A forged session token is the same constant denial.
    let forged = reqwest::Client::new();
    let (status, body) = status_and_json(
        forged
            .put(h.breakpoints_url())
            .header(
                account_session_support::SESSION_TOKEN_HEADER,
                "0".repeat(64),
            )
            .header(
                account_session_support::CHANNEL_BINDING_TOKEN_HEADER,
                &h.owner.channel_binding_token,
            )
            .json(&json!({"workspace_id": h.workspace_id, "breakpoints": hostile}))
            .send()
            .await
            .expect("send forged-session PUT"),
    )
    .await;
    assert_constant_denial(status, &body, "forged-session PUT");
    assert_eq!(h.owner_set().await, before, "forged PUT changed nothing");
}

/// AC-157-7 (LM-RLS-001 viewer): a same-account principal with only Read + fs.read can GET the set
/// (and read its receipt) but its PUT is the constant denial and changes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_read_only_principal_can_get_but_not_put() {
    let h = Harness::start().await;
    let (status, body) = h.put(&h.owner.client(), initial_set()).await;
    assert_eq!(status, StatusCode::OK, "owner PUT: {body}");
    let before = h.owner_set().await;

    let viewer = h.read_only_principal().await;
    let viewer_client = viewer.client();
    let (status, body) = h.get(&viewer_client).await;
    assert_eq!(status, StatusCode::OK, "read-only principal GET: {body}");
    assert_eq!(
        shape(body["breakpoints"].as_array().expect("viewer set")),
        initial_shape(),
        "the read-only principal reads the document's breakpoints"
    );

    for breakpoints in [
        json!([{"source_url": "file:///mt157/viewer.js", "line": 5, "verified": true}]),
        json!([]),
    ] {
        let (status, body) = h.put(&viewer_client, breakpoints).await;
        assert_constant_denial(status, &body, "read-only principal PUT");
        assert_eq!(
            h.owner_set().await,
            before,
            "the read-only principal's PUT changed nothing"
        );
    }

    let (status, body) = h.receipts(&viewer_client).await;
    assert_eq!(status, StatusCode::OK, "viewer receipt read: {body}");
    assert_eq!(
        body.as_array().expect("receipt list").len(),
        1,
        "the document reader sees the one owner receipt, and no receipt from its denied PUTs: {body}"
    );
}

/// AC-157-5: the PUT receipt is accepted by the record-user ledger predicates, carries the session
/// principal (not the constant `debug-breakpoints-ui`) and the document's workspace, and is readable
/// only by accounts that can read the document.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt157_breakpoint_receipt_is_session_attributed_and_private() {
    let h = Harness::start().await;
    let (status, body) = h.put(&h.owner.client(), initial_set()).await;
    assert_eq!(status, StatusCode::OK, "owner PUT: {body}");
    let receipt_id = body["breakpoints"][0]["event_ledger_event_id"]
        .as_str()
        .expect("breakpoint receipt id")
        .to_owned();
    assert!(
        body["breakpoints"]
            .as_array()
            .expect("set")
            .iter()
            .all(|bp| bp["event_ledger_event_id"] == json!(receipt_id)),
        "every row of one PUT references the same receipt"
    );

    let (status, events) = h.receipts(&h.owner.client()).await;
    assert_eq!(status, StatusCode::OK, "owner receipt read: {events}");
    let events = events.as_array().expect("receipt list").clone();
    assert_eq!(events.len(), 1, "one receipt per PUT: {events:?}");
    let receipt = &events[0];
    assert_eq!(receipt["event_id"], json!(receipt_id));
    assert_eq!(receipt["aggregate_type"], json!("debug_breakpoints"));
    assert_eq!(receipt["aggregate_id"], json!(h.document_id));
    assert_eq!(receipt["source_component"], json!("debug_breakpoints"));
    assert_eq!(
        receipt["actor"],
        json!({"kind": "operator", "id": h.owner.actor_id}),
        "the receipt actor is the session principal"
    );
    assert_ne!(receipt["actor"]["id"], json!("debug-breakpoints-ui"));
    assert_eq!(receipt["session_run_id"], json!(h.owner.session_id));
    let payload = &receipt["payload"];
    assert_eq!(
        payload["type"],
        json!("knowledge_debug_breakpoints_recorded")
    );
    assert_eq!(payload["workspace_id"], json!(h.workspace_id));
    assert_eq!(payload["rich_document_id"], json!(h.document_id));
    assert_eq!(payload["breakpoint_count"], json!(3));
    assert_eq!(payload["minted_by_principal"], json!(h.owner.principal_id));
    assert_eq!(payload["producer_actor_id"], json!("debug-breakpoints-ui"));

    let (status, body) = h.receipts(&h.other.client()).await;
    assert_eq!(status, StatusCode::OK, "other-account receipt read: {body}");
    assert_eq!(
        body,
        json!([]),
        "another account reads none of the owner's breakpoint receipts"
    );
    let (status, _body) = h.receipts(&anonymous()).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "anonymous receipt read is denied"
    );

    // A second PUT appends its own receipt, again as the session principal.
    let (status, body) = h.put(&h.owner.client(), json!([])).await;
    assert_eq!(status, StatusCode::OK, "owner clearing PUT: {body}");
    let (status, events) = h.receipts(&h.owner.client()).await;
    assert_eq!(status, StatusCode::OK, "owner receipt re-read: {events}");
    let events = events.as_array().expect("receipt list");
    assert_eq!(events.len(), 2, "one receipt per PUT: {events:?}");
    assert!(events
        .iter()
        .all(|event| event["actor"] == json!({"kind": "operator", "id": h.owner.actor_id})));
}
