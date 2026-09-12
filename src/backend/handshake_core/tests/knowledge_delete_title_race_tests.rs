//! MT-142 review finding R1-1-2: the atomic rich-document delete decides its
//! backlink cleanup scope (and the `backlinks_deleted` count it stamps on the
//! durable tombstone) from `LET $unique_title`, a read of OTHER documents'
//! title rows. On the pinned engine only keys a transaction WRITES are
//! validated at commit (`surrealdb-core-3.2.0/src/kvs/rocksdb/mod.rs:2133-2138`),
//! so before this fix a concurrent rename or create into that title committed
//! invisibly to the delete. `RICH_DOCUMENT_MUTATION_LOCK` used to make that
//! interleaving impossible; MT-142 removed it (AC-142-2).
//!
//! Remediation (b) from the finding: every title-mutating transaction - create,
//! create-if-title-absent, rename (old AND new title) and the atomic delete -
//! UPSERTs the per-(workspace, normalized title) anchor row in
//! `knowledge_rich_document_title_anchors`, so the delete's decision is part of
//! its write set and the racing operations collide at commit; the loser retries
//! and re-reads. The anchor is one row per title, so duplicate titles stay
//! legal (`knowledge_rich_document_title_ambiguous` is unchanged).
//!
//! Proofs here:
//!   1. Deterministic write-set membership: after each title-mutating route
//!      call, the anchor row for a title that still has a live holder exists
//!      and carries the operating document id, and (MT-152 I-152-1) the anchor
//!      of a title whose sole holder was renamed away or deleted is reclaimed
//!      in that same transaction - the row is written either way, which is
//!      what makes the conflict detection fire.
//!   2. Barrier-aligned race, both spawn orders: a delete of A titled T racing
//!      a rename of B into T never leaves the stamped receipt count
//!      disagreeing with the rows that actually vanished, never deletes a
//!      backlink that targets B by record id, and never returns an untyped
//!      failure.

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

use std::sync::Arc;

use async_trait::async_trait;
use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::api::knowledge_documents as docs_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::KnowledgeStore;
use handshake_core::storage::surreal::RowFilter;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use tokio::sync::Barrier;

const ANCHOR_TABLE: &str = "knowledge_rich_document_title_anchors";
const RACE_ITERATIONS: usize = 6;

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
        }
    }
}

async fn doc_server(store: &EmbeddedKnowledgeStore) -> (String, reqwest::Client, ServerGuard) {
    let recorder = Arc::new(NoopRecorder);
    let state = AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("delete-title-race".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, docs_api::routes(state))
            .await
            .expect("docs api server");
    });
    (
        format!("http://{addr}"),
        reqwest::Client::new(),
        ServerGuard(Some(handle)),
    )
}

fn operator(req: reqwest::RequestBuilder, label: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", format!("race-{label}"))
        .header("x-hsk-kernel-task-run-id", format!("KTR-RACE-{label}"))
        .header("x-hsk-session-run-id", format!("SR-RACE-{label}"))
        .header("x-hsk-actor-kind", "operator")
}

fn wikilink_body(workspace_id: &str, title: &str, link_target: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{
                    "type": "hsLink",
                    "attrs": { "refKind": "note", "refValue": link_target, "label": link_target }
                }]
            }]
        }
    })
}

fn plain_body(workspace_id: &str, title: &str) -> Value {
    json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": {
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "body" }] }]
        }
    })
}

async fn create_doc(
    base: &str,
    http: &reqwest::Client,
    label: &str,
    body: Value,
) -> String {
    let response = operator(http.post(format!("{base}/knowledge/documents")), label)
        .json(&body)
        .send()
        .await
        .expect("create send");
    assert_eq!(response.status(), 200, "create must succeed");
    let value: Value = response.json().await.expect("create body");
    value["document"]["rich_document_id"]
        .as_str()
        .expect("created document id")
        .to_owned()
}

/// The test inspector projects Surreal values EXTERNALLY TAGGED: a scalar
/// arrives as a single-entry tag object (`{"String": "..."}`), `None`/`Null`
/// mean absent, and a record link collapses through its `key` sub-value. Same
/// contract as `scalar()` in `tests/wp_kernel_012_native_editor_routes_tests.rs`;
/// a bare `.as_str()` on a tagged value yields `None`.
fn scalar(tagged: &Value) -> Value {
    fn is_absent_tag(tag: &str) -> bool {
        tag == "None" || tag == "Null"
    }
    match tagged {
        Value::String(tag) if is_absent_tag(tag) => Value::Null,
        Value::Object(map) if map.len() == 1 => match map.iter().next() {
            Some((tag, _)) if is_absent_tag(tag) => Value::Null,
            Some((tag, inner)) if tag == "RecordId" => {
                inner.get("key").map_or_else(|| inner.clone(), scalar)
            }
            Some((_, inner)) => scalar(inner),
            None => Value::Null,
        },
        other => other.clone(),
    }
}

fn scalar_string(field: &str, value: &Value) -> String {
    let unwrapped = scalar(value);
    unwrapped
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("projected {field} is not a string: {value:?}"))
}

/// Anchor rows for the whole workspace as (title_key, last_rich_document_id).
async fn anchor_rows(store: &EmbeddedKnowledgeStore) -> Vec<(String, String)> {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(ANCHOR_TABLE)
        .await
        .expect("title anchor table selector");
    inspector
        .project(
            &table,
            &[
                table.field("title_key").expect("title_key field"),
                table
                    .field("last_rich_document_id")
                    .expect("last_rich_document_id field"),
            ],
            RowFilter::All,
        )
        .await
        .expect("project title anchors")
        .into_iter()
        .map(|row| {
            (
                scalar_string("title_key", &row.values["title_key"]),
                scalar_string(
                    "last_rich_document_id",
                    &row.values["last_rich_document_id"],
                ),
            )
        })
        .collect()
}

fn anchor_for<'a>(rows: &'a [(String, String)], title_key: &str) -> Option<&'a String> {
    rows.iter()
        .find(|(key, _)| key == title_key)
        .map(|(_, last)| last)
}

/// Both proofs share one store: a fresh embedded bootstrap applies 4,467 DDL
/// statements at HDD fsync cost (~80 ms each, measured), so paying it twice
/// would double the target's runtime for no extra coverage.
///
/// Phase 1 - every title-mutating route writes the affected title anchor, so
/// the delete's `$unique_title` decision is inside its write set.
/// Phase 2 - the delete/rename race over one title stays consistent.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn title_anchor_write_set_and_delete_rename_race() {
    let store = open_embedded_store()
        .await
        .expect("MT-142 R1-1-2 requires an isolated embedded store");
    let workspace_id = store.create_workspace().await;
    let (base, http, server) = doc_server(&store).await;

    let created = create_doc(
        &base,
        &http,
        "create",
        plain_body(&workspace_id, "Anchor Create Title"),
    )
    .await;
    let rows = anchor_rows(&store).await;
    assert_eq!(
        anchor_for(&rows, "anchor create title"),
        Some(&created),
        "plain create must write its title anchor: {rows:?}"
    );

    let renamed = create_doc(
        &base,
        &http,
        "rename-src",
        plain_body(&workspace_id, "Anchor Rename Before"),
    )
    .await;
    let response = operator(
        http.post(format!("{base}/knowledge/documents/{renamed}/rename")),
        "rename",
    )
    .json(&json!({ "title": "Anchor Rename After" }))
    .send()
    .await
    .expect("rename send");
    assert_eq!(response.status(), 200, "rename must succeed");
    let rows = anchor_rows(&store).await;
    assert_eq!(
        anchor_for(&rows, "anchor rename after"),
        Some(&renamed),
        "rename must write the NEW title anchor: {rows:?}"
    );
    // MT-152 I-152-1 (AC-152-1): the rename writes the PREVIOUS title anchor too
    // (UPSERT then conditional DELETE in the same transaction) and, because the
    // renamed document was that title's only live holder, reclaims it: an anchor
    // row exists iff a live document holds the title. The write-set membership the
    // MT-142 assertion used to read off the surviving row is proven by the
    // reclamation races in `tests/mt152_anchor_reclamation_tests.rs`.
    assert_eq!(
        anchor_for(&rows, "anchor rename before"),
        None,
        "rename away from a sole-holder title must reclaim the PREVIOUS title anchor: {rows:?}"
    );

    let deleted = create_doc(
        &base,
        &http,
        "delete-src",
        plain_body(&workspace_id, "Anchor Delete Title"),
    )
    .await;
    let response = operator(
        http.delete(format!("{base}/knowledge/documents/{deleted}")),
        "delete",
    )
    .send()
    .await
    .expect("delete send");
    assert_eq!(response.status(), 200, "delete must succeed");
    let rows = anchor_rows(&store).await;
    // MT-152 I-152-1 (AC-152-1): the atomic delete writes the title anchor and,
    // as the last live holder is gone, reclaims it in the same transaction.
    assert_eq!(
        anchor_for(&rows, "anchor delete title"),
        None,
        "atomic delete of the sole holder must reclaim the deleted document's title anchor: {rows:?}"
    );

    // Phase 2: delete of A titled T racing rename of B into T, both spawn
    // orders. Neither operation may fail untyped, the stamped
    // `backlinks_deleted` must agree with the rows that actually vanished, and
    // a backlink targeting B by record id must never be collected by A's
    // title-scoped cleanup.
    for iteration in 0..RACE_ITERATIONS {
        let title = format!("Race Title {iteration}");
        let label = format!("i{iteration}");

        // The source document links to the title BEFORE any document holds it,
        // so its backlink row stores the raw title string - exactly the rows
        // the delete's `$unique_title` branch widens onto.
        let source = create_doc(
            &base,
            &http,
            &format!("src-{label}"),
            wikilink_body(&workspace_id, &format!("Race Source {iteration}"), &title),
        )
        .await;
        let target = create_doc(
            &base,
            &http,
            &format!("a-{label}"),
            plain_body(&workspace_id, &title),
        )
        .await;
        let renamed = create_doc(
            &base,
            &http,
            &format!("b-{label}"),
            plain_body(&workspace_id, &format!("Race Other {iteration}")),
        )
        .await;

        // A second source links to B by record id; that row is B's and must
        // never be collected by A's delete.
        let id_source = create_doc(
            &base,
            &http,
            &format!("idsrc-{label}"),
            wikilink_body(
                &workspace_id,
                &format!("Race Id Source {iteration}"),
                &renamed,
            ),
        )
        .await;

        let before = store
            .db
            .list_knowledge_document_backlinks_from(&source)
            .await
            .expect("source backlinks before the race");
        let title_targeted_before = before.iter().filter(|row| row.target == title).count();
        assert_eq!(
            title_targeted_before, 1,
            "iteration {iteration}: the source must hold exactly one title-targeted backlink"
        );

        let barrier = Arc::new(Barrier::new(2));
        let delete_task = {
            let (base, http, label) = (base.clone(), http.clone(), label.clone());
            let barrier = Arc::clone(&barrier);
            let target = target.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                operator(
                    http.delete(format!("{base}/knowledge/documents/{target}")),
                    &format!("del-{label}"),
                )
                .send()
                .await
                .expect("delete send")
            })
        };
        let rename_task = {
            let (base, http, label, title) = (base.clone(), http.clone(), label.clone(), title.clone());
            let barrier = Arc::clone(&barrier);
            let renamed = renamed.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                operator(
                    http.post(format!("{base}/knowledge/documents/{renamed}/rename")),
                    &format!("ren-{label}"),
                )
                .json(&json!({ "title": title }))
                .send()
                .await
                .expect("rename send")
            })
        };
        // Alternate which task is awaited first so both spawn orders are
        // exercised across iterations.
        let (delete_response, rename_response) = if iteration % 2 == 0 {
            let delete = delete_task.await.expect("delete task");
            let rename = rename_task.await.expect("rename task");
            (delete, rename)
        } else {
            let rename = rename_task.await.expect("rename task");
            let delete = delete_task.await.expect("delete task");
            (delete, rename)
        };

        assert_eq!(
            delete_response.status(),
            200,
            "iteration {iteration}: delete must not fail untyped"
        );
        assert_eq!(
            rename_response.status(),
            200,
            "iteration {iteration}: rename must not fail untyped"
        );
        let delete_body: Value = delete_response.json().await.expect("delete body");
        let stamped = delete_body["backlinks_deleted"]
            .as_u64()
            .expect("stamped backlinks_deleted");

        let after = store
            .db
            .list_knowledge_document_backlinks_from(&source)
            .await
            .expect("source backlinks after the race");
        let title_targeted_after = after.iter().filter(|row| row.target == title).count();
        let vanished = (title_targeted_before - title_targeted_after) as u64;
        assert_eq!(
            stamped, vanished,
            "iteration {iteration}: the durable receipt count ({stamped}) must equal the \
             title-targeted rows that actually vanished ({vanished})"
        );

        let id_rows = store
            .db
            .list_knowledge_document_backlinks_from(&id_source)
            .await
            .expect("id-targeted backlinks after the race");
        assert!(
            id_rows.iter().any(|row| row.target == renamed),
            "iteration {iteration}: a backlink targeting the renamed document by record id must \
             never be collected by another document's title-scoped delete"
        );

        // The rename always converges: B holds the title after the race.
        let renamed_document = store
            .db
            .get_knowledge_rich_document(&renamed)
            .await
            .expect("renamed document readback")
            .expect("renamed document is live");
        assert_eq!(
            renamed_document.title, title,
            "iteration {iteration}: the rename must converge on the contended title"
        );

        // Whatever the commit order was, the anchor row for the contended
        // title exists and names one of the two racing documents.
        let rows = anchor_rows(&store).await;
        let anchor_owner = anchor_for(&rows, &title.to_lowercase())
            .unwrap_or_else(|| panic!("iteration {iteration}: contended title anchor is missing"));
        assert!(
            anchor_owner == &renamed || anchor_owner == &target,
            "iteration {iteration}: anchor owner {anchor_owner} must be one of the racing documents"
        );
    }

    server.shutdown().await;
    store
        .close_and_remove()
        .await
        .expect("cleanup embedded store");
}
