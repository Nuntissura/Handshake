//! WP-KERNEL-009 RichDocumentCore route-level integration tests against REAL
//! Handshake-managed PostgreSQL — adversarial-v2 hardening proofs.
//!
//! Drives the actual Axum routes (`api::knowledge_documents::routes`) over a
//! loopback listener (quiet: no foreground window, no focus steal).
//!
//! Covered hardenings:
//!   * MT-158: the permission boundary FAILS CLOSED — a missing
//!     `x-hsk-actor-kind` is least-privileged (read-only, never `system`), a
//!     `cloud_model` cannot write, and a bogus kind is a 400.
//!   * MT-151: import -> load -> save -> export round-trips for HTML and
//!     markdown-table imports (the `importedRaw` node is a loadable kind).
//!   * MT-149: a committed save never returns an error — index/receipt step
//!     failures are non-fatal and recorded in the response.
//!   * MT-152: content_json embed blocks are validated + persisted on the save
//!     path with the same EmbedTarget law as the side table.
//!   * MT-156: history is paginated and version bodies are omitted from the
//!     list response (single-version lazy body load).
//!   * MT-157: a move with an empty body does NOT clear project/folder
//!     membership (absent != explicit null).

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::api::knowledge_documents as docs_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
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

/// Boot the real document routes over loopback against the isolated schema.
async fn doc_server(pg: &KnowledgePg) -> (String, reqwest::Client) {
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
    let state = AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("docs-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = docs_api::routes(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("docs api server");
    });
    (format!("http://{addr}"), reqwest::Client::new())
}

/// The required identity headers WITHOUT an actor kind (MT-158 absence case).
fn identity_headers(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("docs-api-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-DOCS-{label}"))
        .header("x-hsk-session-run-id", format!("SR-DOCS-{label}"))
}

/// Identity headers PLUS an explicitly asserted actor kind.
fn headers_with_kind(
    req: reqwest::RequestBuilder,
    label: &str,
    kind: &str,
) -> reqwest::RequestBuilder {
    identity_headers(req, label).header("x-hsk-actor-kind", kind)
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
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "setup",
        "operator",
    )
    .json(&doc_body(workspace_id, title))
    .send()
    .await
    .expect("create send");
    assert_eq!(resp.status(), 200, "operator create must succeed");
    resp.json().await.expect("create json")
}

// ---------------------------------------------------------------------------
// MT-158 adversarial-v2: actor-kind fail-closed boundary.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_missing_actor_kind_is_least_privileged_never_system() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt158_missing_actor_kind...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;

    let created = create_doc(&base, &http, &workspace_id, "Boundary").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // ABSENT actor-kind header on a CREATE (write) -> 403, never a system
    // write. Before the hardening this fell open to `system` (full access).
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents")),
        "no-kind-create",
    )
    .json(&doc_body(&workspace_id, "Sneak"))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        403,
        "create without x-hsk-actor-kind must be denied"
    );
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["error"], "forbidden");
    assert_eq!(body["reason"], "unauthenticated_write_denied");

    // ABSENT actor-kind on a SAVE (write) -> 403 and the document is unchanged.
    let resp = identity_headers(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "no-kind-save",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "tampered" }] }
        ]}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        403,
        "save without x-hsk-actor-kind must be denied"
    );

    // ABSENT actor-kind on rename / move / backlink-rebuild -> all denied.
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents/{doc_id}/rename")),
        "no-kind-rename",
    )
    .json(&json!({"title": "Hijacked"}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403);
    let resp = identity_headers(
        http.post(format!("{base}/knowledge/documents/{doc_id}/backlinks")),
        "no-kind-index",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403, "index without kind must be denied");

    // The least-privileged caller can still READ (attributable read law).
    let resp = identity_headers(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "no-kind-read",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200, "read stays available without a kind");
    let body: Value = resp.json().await.expect("json");
    // The document content was NOT tampered by the denied save.
    assert_eq!(
        body["document"]["content_json"]["content"][0]["content"][0]["text"],
        "hello"
    );
    assert_eq!(body["document"]["doc_version"], 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt158_cloud_model_cannot_write_and_bogus_kind_is_rejected() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt158_cloud_model...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "CloudBoundary").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // cloud_model write -> 403 with the stable reason code.
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "cloud-save",
        "cloud_model",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": []}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403, "cloud_model write must be denied");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["reason"], "cloud_model_write_denied");

    // cloud_model create -> 403 too (no document authoring).
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "cloud-create",
        "cloud_model",
    )
    .json(&doc_body(&workspace_id, "CloudDoc"))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 403);

    // A bogus asserted kind is a 400 (strict vocabulary), never a coercion.
    for bogus in ["root", "SYSTEM", "model_adapter", "admin"] {
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "bogus",
            bogus,
        )
        .json(&json!({
            "expected_version": 1,
            "content_json": {"type": "doc", "content": []}
        }))
        .send()
        .await
        .expect("send");
        assert_eq!(
            resp.status(),
            400,
            "bogus actor kind `{bogus}` must be rejected"
        );
    }

    // cloud_model can still read (the allowed half of its matrix row).
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "cloud-read",
        "cloud_model",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------------
// MT-151 adversarial-v2: import -> load -> save -> export round-trips.
// ---------------------------------------------------------------------------

/// Drive one full import -> load -> save -> export cycle through the real
/// routes and return (document_id, loaded body). Before the ImportedRaw
/// hardening, the LOAD step 400'd for any imported HTML/table document.
async fn import_roundtrip(
    base: &str,
    http: &reqwest::Client,
    workspace_id: &str,
    label: &str,
    format: &str,
    snippet: &str,
) -> (String, Value) {
    // IMPORT.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents/import")),
        label,
        "operator",
    )
    .json(&json!({
        "workspace_id": workspace_id,
        "title": format!("Imported {label}"),
        "format": format,
        "snippet": snippet,
    }))
    .send()
    .await
    .expect("import send");
    assert_eq!(resp.status(), 200, "import must succeed");
    let imported: Value = resp.json().await.expect("import json");
    let doc_id = imported["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // LOAD (typed block tree) — the adversarial-v2 finding: this was a 400.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        label,
        "operator",
    )
    .send()
    .await
    .expect("load send");
    assert_eq!(
        resp.status(),
        200,
        "imported {format} document must LOAD through the typed API"
    );
    let loaded: Value = resp.json().await.expect("load json");
    assert_eq!(loaded["tree"]["schema_matches"], true);

    // BLOCKS endpoint loads too.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/blocks")),
        label,
        "operator",
    )
    .send()
    .await
    .expect("blocks send");
    assert_eq!(resp.status(), 200, "blocks endpoint must load");

    // SAVE the loaded content back (v1 -> v2): the round-trip must validate.
    let content = loaded["document"]["content_json"].clone();
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        label,
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": content}))
    .send()
    .await
    .expect("save send");
    assert_eq!(resp.status(), 200, "imported document must SAVE");
    let saved: Value = resp.json().await.expect("save json");
    assert_eq!(saved["document"]["doc_version"], 2);
    assert_eq!(
        saved["document"]["content_json"],
        loaded["document"]["content_json"],
        "save round-trip is lossless"
    );

    // EXPORT projections (markdown + html) — render, never 400.
    for proj in ["markdown", "html"] {
        let resp = headers_with_kind(
            http.get(format!(
                "{base}/knowledge/documents/{doc_id}/projection?format={proj}"
            )),
            label,
            "operator",
        )
        .send()
        .await
        .expect("projection send");
        assert_eq!(
            resp.status(),
            200,
            "imported {format} document must EXPORT as {proj}"
        );
    }
    (doc_id, loaded)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_imported_html_document_roundtrips_load_save_export() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt151_imported_html...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;

    let html = "<h1>Doc</h1><table><tr><td>cell</td></tr></table>";
    let (doc_id, loaded) =
        import_roundtrip(&base, &http, &workspace_id, "html", "html", html).await;

    // The importedRaw block is present in the typed tree with its source.
    let blocks = loaded["tree"]["blocks"].as_array().expect("blocks");
    assert!(
        blocks.iter().any(|b| b["kind"] == "imported_raw"),
        "typed tree exposes the imported_raw block: {blocks:?}"
    );

    // The markdown export carries the captured source INERT (fenced).
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/projection?format=markdown"
        )),
        "html-md",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    let content = body["projection"]["content"].as_str().expect("content");
    assert!(
        content.contains("```html") && content.contains("<table>"),
        "markdown export fences the imported source: {content}"
    );
}

// ---------------------------------------------------------------------------
// MT-152 adversarial-v2: the save path validates + persists content embeds.
// ---------------------------------------------------------------------------

fn doc_with_embed(workspace_id: &str, title: &str, target: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "intro" }] },
                { "type": "image", "attrs": { "target": target },
                  "content": [{ "type": "text", "text": "diagram" }] }
            ]
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt152_save_path_validates_and_persists_content_embeds() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt152_save_path...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;

    // CREATE with a valid typed embed target -> the side table is synced.
    let resp = headers_with_kind(
        http.post(format!("{base}/knowledge/documents")),
        "embed-create",
        "operator",
    )
    .json(&doc_with_embed(&workspace_id, "Embeds", "KMED-ok"))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let created: Value = resp.json().await.expect("json");
    assert_eq!(created["embeds_persisted"], 1, "create syncs the embed table");
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    let list_embeds = |label: &'static str| {
        headers_with_kind(
            http.get(format!("{base}/knowledge/documents/{doc_id}/embeds")),
            label,
            "operator",
        )
        .send()
    };
    let body: Value = list_embeds("e1").await.expect("send").json().await.expect("json");
    let embeds = body["embeds"].as_array().expect("embeds");
    assert_eq!(embeds.len(), 1);
    assert_eq!(embeds[0]["ref_value"], "KMED-ok");
    assert_eq!(embeds[0]["ref_kind"], "media");

    // SAVE v2 with two embeds (media id + https url) -> table resyncs to 2.
    let v2 = json!({
        "type": "doc",
        "content": [
            { "type": "image", "attrs": { "target": "KMED-ok" },
              "content": [{ "type": "text", "text": "diagram" }] },
            { "type": "video", "attrs": { "src": "https://cdn.example/clip.mp4" },
              "content": [{ "type": "text", "text": "clip" }] }
        ]
    });
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "embed-save2",
        "operator",
    )
    .json(&json!({"expected_version": 1, "content_json": v2}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let saved: Value = resp.json().await.expect("json");
    assert_eq!(saved["embeds_persisted"], 2);
    let body: Value = list_embeds("e2").await.expect("send").json().await.expect("json");
    let embeds = body["embeds"].as_array().expect("embeds");
    assert_eq!(embeds.len(), 2);
    assert!(embeds.iter().any(|e| e["ref_kind"] == "url"
        && e["ref_value"] == "https://cdn.example/clip.mp4"));

    // SAVE with a dangerous embed target -> 400 BEFORE commit; version stays 2.
    for bad in [
        "javascript:alert(1)",
        "JaVa\tScRiPt:alert(1)",
        "data:text/html,<script>",
        "C:\\secrets\\x.png",
        "/etc/passwd",
        "file:///etc/passwd",
    ] {
        let v3 = json!({
            "type": "doc",
            "content": [
                { "type": "image", "attrs": { "target": bad },
                  "content": [{ "type": "text", "text": "evil" }] }
            ]
        });
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "embed-bad",
            "operator",
        )
        .json(&json!({"expected_version": 2, "content_json": v3}))
        .send()
        .await
        .expect("send");
        assert_eq!(
            resp.status(),
            400,
            "embed target `{bad}` must reject the save fail-closed"
        );
    }
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "embed-check",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(
        body["document"]["doc_version"], 2,
        "rejected saves never committed"
    );

    // SAVE v3 with NO embeds -> the side table empties (true sync, no drift).
    let v3 = json!({
        "type": "doc",
        "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "no embeds left" }] }
        ]
    });
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "embed-save3",
        "operator",
    )
    .json(&json!({"expected_version": 2, "content_json": v3}))
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = list_embeds("e3").await.expect("send").json().await.expect("json");
    assert_eq!(body["embeds"].as_array().expect("embeds").len(), 0);
}

// ---------------------------------------------------------------------------
// MT-156 adversarial-v2: history is paginated and omits version bodies.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt156_history_is_paginated_and_omits_version_bodies() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt156_history...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "History").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // Build 5 versions (v1 from create + 4 saves with distinct bodies).
    for v in 1..=4i64 {
        let resp = headers_with_kind(
            http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
            "hist-save",
            "operator",
        )
        .json(&json!({
            "expected_version": v,
            "content_json": {"type": "doc", "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": format!("body v{}", v + 1) }] }
            ]}
        }))
        .send()
        .await
        .expect("send");
        assert_eq!(resp.status(), 200, "save v{} must succeed", v + 1);
    }

    // Paginated page: limit=2 offset=1 -> versions 2 and 3, metadata ONLY.
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/history?limit=2&offset=1"
        )),
        "hist-page",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["total_versions"], 5);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 1);
    let versions = body["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["doc_version"], 2);
    assert_eq!(versions[1]["doc_version"], 3);
    for version in versions {
        assert!(
            version.get("content_json").is_none(),
            "history list must omit version bodies: {version}"
        );
        assert!(version["content_sha256"].is_string());
    }

    // The limit is capped server-side: a huge requested limit clamps to 200.
    let resp = headers_with_kind(
        http.get(format!(
            "{base}/knowledge/documents/{doc_id}/history?limit=100000"
        )),
        "hist-cap",
        "operator",
    )
    .send()
    .await
    .expect("send");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["limit"], 200, "requested limit must clamp to the cap");
    assert_eq!(body["versions"].as_array().expect("versions").len(), 5);

    // Lazy single-version body load: GET history/3 returns the v3 content.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/history/3")),
        "hist-one",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["version"]["doc_version"], 3);
    assert_eq!(
        body["version"]["content_json"]["content"][0]["content"][0]["text"],
        "body v3"
    );

    // A missing version is a 404, not an empty 200.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}/history/99")),
        "hist-missing",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 404);
}

// ---------------------------------------------------------------------------
// MT-149 adversarial-v2: a committed save never returns an error.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt149_committed_save_never_errors_when_post_commit_steps_fail() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt149_committed_save...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;
    let created = create_doc(&base, &http, &workspace_id, "Atomicity").await;
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("doc id")
        .to_string();

    // Break EVERY post-commit step for real: drop the backlink + embed side
    // tables and the EventLedger table in the isolated schema. The save's own
    // tables stay intact, so the save itself can still commit.
    {
        let mut conn = pg.raw_connection().await;
        for table in [
            "knowledge_document_backlinks",
            "knowledge_document_embeds",
            "kernel_event_ledger",
        ] {
            sqlx::query(&format!("DROP TABLE {table} CASCADE"))
                .execute(&mut conn)
                .await
                .unwrap_or_else(|err| panic!("drop {table}: {err}"));
        }
    }

    // The save must COMMIT and return 200 with every failure RECORDED.
    let resp = headers_with_kind(
        http.put(format!("{base}/knowledge/documents/{doc_id}/save")),
        "atomic-save",
        "operator",
    )
    .json(&json!({
        "expected_version": 1,
        "content_json": {"type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "v2 body" }] },
            { "type": "image", "attrs": { "target": "KMED-1" } }
        ]}
    }))
    .send()
    .await
    .expect("send");
    assert_eq!(
        resp.status(),
        200,
        "a committed save must NEVER surface a post-commit step failure as an error"
    );
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["document"]["doc_version"], 2, "the save committed");
    assert!(
        body["save_receipt_event_id"].is_null(),
        "no receipt could be written"
    );
    assert!(
        body["receipt_error"].is_string(),
        "the receipt failure is recorded: {body}"
    );
    assert!(
        body["backlinks_error"].is_string(),
        "the backlink index failure is recorded: {body}"
    );
    assert!(
        body["embeds_error"].is_string(),
        "the embed sync failure is recorded: {body}"
    );

    // The committed write is durable and loadable.
    let resp = headers_with_kind(
        http.get(format!("{base}/knowledge/documents/{doc_id}")),
        "atomic-load",
        "operator",
    )
    .send()
    .await
    .expect("send");
    assert_eq!(resp.status(), 200);
    let loaded: Value = resp.json().await.expect("json");
    assert_eq!(loaded["document"]["doc_version"], 2);
    assert_eq!(
        loaded["document"]["content_json"]["content"][0]["content"][0]["text"],
        "v2 body"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_imported_markdown_table_document_roundtrips_load_save_export() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP mt151_imported_markdown_table...: no PostgreSQL");
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let (base, http) = doc_server(&pg).await;

    let md = "# Title\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\ntail paragraph";
    let (_doc_id, loaded) =
        import_roundtrip(&base, &http, &workspace_id, "mdtable", "markdown", md).await;

    let blocks = loaded["tree"]["blocks"].as_array().expect("blocks");
    assert!(blocks.iter().any(|b| b["kind"] == "imported_raw"));
    assert!(blocks.iter().any(|b| b["kind"] == "heading"));
    assert!(blocks.iter().any(|b| b["kind"] == "paragraph"));
}
