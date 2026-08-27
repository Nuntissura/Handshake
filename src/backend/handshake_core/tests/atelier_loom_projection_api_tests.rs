//! WP-KERNEL-012 MT-033: real embedded-SurrealDB proof for the canonical
//! Atelier-intake-item -> Loom-block bridge consumed by native Canvas drops.
//!
//! The test drives the production Axum Atelier routes. It does not inject a
//! `loom_block_id` into a frontend fixture: it creates real authority rows,
//! publishes the relation through the production PUT endpoint, reloads the
//! real batch-items endpoint, and asserts that endpoint carries the identity.

mod atelier_surreal_support;

use std::sync::Arc;

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::api::atelier as atelier_api;
use handshake_core::atelier::intake::intake_event_family;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, NewDocument, NewLoomBlock, NewWorkspace,
    WriteContext,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use uuid::Uuid;

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
        _id: Uuid,
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

async fn app_state(harness: &AtelierSurrealHarness) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: harness.database.clone(),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt033-atelier-loom-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, atelier_api::routes(state))
            .await
            .expect("Atelier API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

async fn source_backed_block(storage: &dyn Database, workspace_id: &str, title: &str) -> String {
    let ctx = WriteContext::human(None);
    let document = storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace_id.to_string(),
                title: title.to_string(),
            },
        )
        .await
        .expect("create source document");
    storage
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type: LoomBlockContentType::Note,
                document_id: Some(document.id),
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create source-backed Loom block")
        .block_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_atelier_endpoint_returns_durable_canonical_loom_identity() {
    let harness = AtelierSurrealHarness::create().await;
    let state = app_state(&harness).await;
    let store = harness.atelier.clone();
    let storage = SurrealDatabase::new(harness.storage.clone());
    let workspace_id = storage
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("MT-033 Atelier Loom {}", Uuid::new_v4()),
            },
        )
        .await
        .expect("create isolated workspace")
        .id;

    let (base, http, server) = serve(state).await;
    let batch_response = http
        .post(format!("{base}/atelier/intake/batches"))
        .json(&json!({
            "idempotency_key": format!("mt140-loom-projection-{}", Uuid::new_v4()),
            "source_label": "MT-140 canonical Loom projection",
            "mode": "manual",
            "profile_mode": "loose_profile"
        }))
        .send()
        .await
        .expect("create intake batch through product API");
    assert_eq!(batch_response.status(), reqwest::StatusCode::CREATED);
    let batch: Value = batch_response.json().await.expect("batch response JSON");
    let batch_id = Uuid::parse_str(
        batch["batch_id"]
            .as_str()
            .expect("batch response carries batch_id"),
    )
    .expect("batch_id is a UUID");
    let item_source = format!("source://mt140/{}/canonical.png", Uuid::new_v4());
    let item_url = format!("{base}/atelier/intake/batches/{batch_id}/items");
    let item_body = json!({
        "source_path": item_source,
        "file_name": "mt140-canonical.png",
        "byte_len": 33,
        "content_hash": null
    });
    let item_response = http
        .post(&item_url)
        .header("x-hsk-actor-id", "mt140-api-test")
        .json(&item_body)
        .send()
        .await
        .expect("create intake item through product API");
    assert_eq!(item_response.status(), reqwest::StatusCode::CREATED);
    let item: Value = item_response.json().await.expect("item response JSON");
    let item_id = Uuid::parse_str(
        item["item_id"]
            .as_str()
            .expect("item response carries item_id"),
    )
    .expect("item_id is a UUID");
    assert_eq!(item["lane"], "pending");
    assert_eq!(item["loom_block_id"], Value::Null);

    let replay = http
        .post(&item_url)
        .header("x-hsk-actor-id", "mt140-api-test-retry")
        .json(&item_body)
        .send()
        .await
        .expect("retry intake item creation");
    assert_eq!(replay.status(), reqwest::StatusCode::CREATED);
    let replay: Value = replay.json().await.expect("retry response JSON");
    assert_eq!(replay["item_id"], item_id.to_string());
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_ADDED,
                "atelier_intake_item",
                &item_id.to_string(),
            )
            .await
            .expect("count item-added events"),
        1,
        "idempotent retry must not duplicate item-added evidence"
    );

    let unknown_batch = Uuid::new_v4();
    let missing = http
        .post(format!(
            "{base}/atelier/intake/batches/{unknown_batch}/items"
        ))
        .header("x-hsk-actor-id", "mt140-api-test")
        .json(&item_body)
        .send()
        .await
        .expect("unknown-batch request");
    assert_eq!(missing.status(), reqwest::StatusCode::NOT_FOUND);

    let no_actor = http
        .post(&item_url)
        .json(&json!({
            "source_path": "source://mt140/no-actor.png",
            "file_name": "no-actor.png",
            "byte_len": 1,
            "content_hash": null
        }))
        .send()
        .await
        .expect("missing-actor request");
    assert_eq!(no_actor.status(), reqwest::StatusCode::BAD_REQUEST);

    let block_id = source_backed_block(&storage, &workspace_id, "MT-033 canonical block").await;
    let link_url = format!(
        "{base}/atelier/intake/items/{}/loom-projection",
        item_id
    );
    let linked = http
        .put(&link_url)
        .header("x-hsk-actor-id", "mt033-api-test")
        .json(&json!({"loom_block_id": block_id}))
        .send()
        .await
        .expect("link request");
    assert_eq!(linked.status(), reqwest::StatusCode::OK);
    let linked_json: Value = linked.json().await.expect("link response JSON");
    assert_eq!(linked_json["item_id"], item_id.to_string());
    assert_eq!(linked_json["loom_block_id"], block_id);
    assert_eq!(linked_json["workspace_id"], workspace_id);

    let listed = http
        .get(format!(
            "{base}/atelier/intake/batches/{}/items",
            batch_id
        ))
        .send()
        .await
        .expect("list batch items");
    assert_eq!(listed.status(), reqwest::StatusCode::OK);
    let listed_json: Value = listed.json().await.expect("items response JSON");
    let listed_item = listed_json["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|row| row["item_id"] == item_id.to_string())
        .expect("linked item returned by real endpoint");
    assert_eq!(listed_item["loom_block_id"], block_id);

    let idempotent = http
        .put(&link_url)
        .header("x-hsk-actor-id", "mt033-api-test-retry")
        .json(&json!({"loom_block_id": block_id}))
        .send()
        .await
        .expect("idempotent link retry");
    assert_eq!(idempotent.status(), reqwest::StatusCode::OK);
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_ITEM_LOOM_PROJECTION_LINKED,
                "atelier_intake_item",
                &item_id.to_string(),
            )
            .await
            .expect("count relation events"),
        1,
        "an idempotent retry must not duplicate EventLedger evidence"
    );

    let different_block =
        source_backed_block(&storage, &workspace_id, "MT-033 conflicting block").await;
    let conflict = http
        .put(&link_url)
        .header("x-hsk-actor-id", "mt033-api-test")
        .json(&json!({"loom_block_id": different_block}))
        .send()
        .await
        .expect("conflicting relink request");
    assert_eq!(conflict.status(), reqwest::StatusCode::CONFLICT);

    server.abort();
    let _ = server.await;
    harness.shutdown().await;
}
