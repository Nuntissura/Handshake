use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
    knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION,
    llm::DisabledLlmClient,
    managed_postgres::{ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError},
    storage::{
        knowledge::{KnowledgeStore, NewKnowledgeRichDocument},
        postgres::PostgresDatabase,
        Database, NewWorkspace, WriteContext,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Connection;
use tokio::net::TcpListener;
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
    let Some(base_url) = base_database_url().await? else {
        println!("MT250_FIXTURE_SKIP PostgreSQL binaries not found");
        return Ok(());
    };
    let schema_url = isolated_schema_url(&base_url).await?;
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;

    let (workspace_id, documents) = seed_workspace_fixture(&db).await?;
    let state = app_state_for(&schema_url).await?;
    let app = app_router(state);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        workspace_id,
        documents,
    };
    println!("MT250_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn base_database_url() -> Result<Option<String>, Box<dyn std::error::Error>> {
    for var in ["POSTGRES_TEST_URL", "DATABASE_URL"] {
        if let Some(url) = std::env::var(var)
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Some(url));
        }
    }

    match ManagedPostgres::ensure_running(ManagedPostgresConfig::from_env()).await {
        Ok(managed) => Ok(Some(managed.database_url())),
        Err(ManagedPostgresError::BinariesNotFound(detail)) => {
            eprintln!("SKIP MT-250 fixture: PostgreSQL binaries not found ({detail})");
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}

async fn isolated_schema_url(base_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = format!("mt250_workspace_search_{}", Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(base_url).await?;
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .execute(&mut conn)
        .await?;
    for shim in [
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {schema}.digest(input text, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input::bytea, algorithm) $$
            "#
        ),
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {schema}.digest(input bytea, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input, algorithm) $$
            "#
        ),
    ] {
        sqlx::query(&shim).execute(&mut conn).await?;
    }
    let sep = if base_url.contains('?') { "&" } else { "?" };
    Ok(format!("{base_url}{sep}options=-csearch_path%3D{schema}"))
}

async fn seed_workspace_fixture(
    db: &PostgresDatabase,
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

async fn app_state_for(schema_url: &str) -> Result<AppState, Box<dyn std::error::Error>> {
    let storage = PostgresDatabase::connect(schema_url, 5).await?.into_arc();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await?;
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt250-workspace-search-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
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
    let db = PostgresDatabase::new(state.postgres_pool.clone());
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
