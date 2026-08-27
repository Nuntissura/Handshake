use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
    knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION,
    llm::DisabledLlmClient,
    storage::{
        knowledge::{KnowledgeStore, NewKnowledgeRichDocument},
        surreal::SurrealDatabase,
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database, NewWorkspace, WriteContext,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::watch};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

const FIRST_BODY: &str = "ReplaceAlpha beta ReplaceAlpha";
const SECOND_BODY: &str = "Unopened canonical document contains ReplaceAlpha too.";

#[derive(Debug, Serialize)]
struct ReadyMessage {
    base_url: String,
    workspace_id: String,
    documents: Vec<SeedDocument>,
}

#[derive(Debug, Serialize)]
struct SeedDocument {
    rich_document_id: String,
    title: String,
    initial_text: String,
}

#[derive(Debug, Deserialize)]
struct ProofQuery {
    doc_ids: String,
}

#[derive(Debug, Serialize)]
struct ProofResponse {
    documents: Vec<ProofDocument>,
}

#[derive(Debug, Serialize)]
struct ProofDocument {
    rich_document_id: String,
    title: String,
    doc_version: i64,
    text: String,
    event_count: usize,
    save_events: Vec<ProofEvent>,
}

#[derive(Debug, Serialize)]
struct ProofEvent {
    event_id: String,
    event_type: String,
    payload: Value,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT250_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let backend = embedded_test_backend().await?;
    let body_result = run_server(&backend).await;
    let cleanup_result = backend.close_and_remove().await;

    match (body_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(body_error), Ok(())) => Err(body_error),
        (Ok(()), Err(cleanup_error)) => Err(Box::new(cleanup_error)),
        (Err(body_error), Err(cleanup_error)) => Err(std::io::Error::other(format!(
            "fixture body failed: {body_error}; embedded-store cleanup also failed: {cleanup_error}"
        ))
        .into()),
    }
}

async fn run_server(backend: &EmbeddedTestBackend) -> Result<(), Box<dyn std::error::Error>> {
    let db = SurrealDatabase::new(backend.storage.clone());

    let (workspace_id, documents) = seed_workspace_fixture(&db).await?;
    let state = app_state_for(backend)?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let app = app_router(state).route(
        "/__fixture/shutdown",
        post(move || {
            let shutdown_tx = shutdown_tx.clone();
            async move {
                let _ = shutdown_tx.send(true);
            }
        }),
    );
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        workspace_id,
        documents,
    };
    println!("MT250_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            loop {
                if *shutdown_rx.borrow_and_update() {
                    break;
                }
                if shutdown_rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await?;
    Ok(())
}

async fn seed_workspace_fixture(
    db: &SurrealDatabase,
) -> Result<(String, Vec<SeedDocument>), Box<dyn std::error::Error>> {
    let workspace = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("mt250-workspace-search-{}", Uuid::now_v7()),
            },
        )
        .await?;
    let workspace_id = workspace.id;
    let docs = [
        ("Loaded search note", FIRST_BODY),
        ("Unopened canonical note", SECOND_BODY),
    ];
    let mut seeded = Vec::with_capacity(docs.len());
    for (title, body) in docs {
        let document = db
            .create_knowledge_rich_document(NewKnowledgeRichDocument {
                workspace_id: workspace_id.clone(),
                document_id: None,
                title: title.to_string(),
                schema_version: DOCUMENT_SCHEMA_VERSION.to_string(),
                content_json: paragraph_doc(body),
                crdt_document_id: None,
                crdt_snapshot_id: None,
                promotion_receipt_event_id: None,
                project_ref: None,
                folder_ref: None,
                authority_label: Some("promoted".to_string()),
                owner_actor_kind: Some("operator".to_string()),
                owner_actor_id: Some("mt250-fixture".to_string()),
            })
            .await?;
        seeded.push(SeedDocument {
            rich_document_id: document.rich_document_id,
            title: title.to_string(),
            initial_text: body.to_string(),
        });
    }
    Ok((workspace_id, seeded))
}

fn paragraph_doc(text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [
            {
                "type": "paragraph",
                "content": [{ "type": "text", "text": text }]
            }
        ]
    })
}

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt250-workspace-search-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

fn app_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let api_routes = api::routes(state.clone());
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/mt250-fixture/proof", get(fixture_proof))
        .with_state(state)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn fixture_proof(
    State(state): State<AppState>,
    Query(query): Query<ProofQuery>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    let db = SurrealDatabase::new(state.surreal.clone());
    let mut documents = Vec::new();
    for doc_id in query
        .doc_ids
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        let document = db
            .get_knowledge_rich_document(doc_id)
            .await
            .map_err(internal_fixture_error)?
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("missing document {doc_id}")))?;
        let events = db
            .list_kernel_events_for_aggregate("knowledge_rich_document", doc_id)
            .await
            .map_err(internal_fixture_error)?;
        let save_events = events
            .iter()
            .filter(|event| {
                event.event_type.as_str() == "KNOWLEDGE_RICH_DOCUMENT_SAVED"
                    && event.payload.get("event").and_then(Value::as_str) == Some("saved")
            })
            .map(|event| ProofEvent {
                event_id: event.event_id.clone(),
                event_type: event.event_type.as_str().to_string(),
                payload: event.payload.clone(),
            })
            .collect();
        documents.push(ProofDocument {
            rich_document_id: document.rich_document_id,
            title: document.title,
            doc_version: document.doc_version,
            text: document_text(&document.content_json),
            event_count: events.len(),
            save_events,
        });
    }
    Ok(Json(ProofResponse { documents }))
}

fn internal_fixture_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn document_text(content: &Value) -> String {
    let mut out = String::new();
    collect_text(content, &mut out);
    out
}

fn collect_text(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                out.push_str(text);
            }
            for child in map.values() {
                collect_text(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_text(child, out);
            }
        }
        _ => {}
    }
}
