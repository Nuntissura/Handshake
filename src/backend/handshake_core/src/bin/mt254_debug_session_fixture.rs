// WP-KERNEL-009 / MT-254 — DebugAdapterCore real-backend fixture.
//
// Spins up the real product debug-adapter REST surface (`api::routes`) against
// an isolated PostgreSQL schema, AND writes a real Node fixture script to disk.
// The offline Playwright spec drives the built DebugSidePanel/DebugConsole
// harness against this backend:
//   * GET /debug/adapters (honesty gate: node only),
//   * POST /debug/sessions launches a REAL node child under --inspect-brk,
//   * set a breakpoint (REAL CDP verified), continue, hit it, read the stack +
//     variables (a=2, b=40), evaluate("a + b") == 42, step, continue, terminate,
//   * PUT durable breakpoints persisted to real PostgreSQL + EventLedger.
//
// A proof endpoint reads the durable breakpoints + their EventLedger receipts
// back from real PostgreSQL so the spec can assert the persistence truly landed.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
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
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::Connection;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ReadyMessage {
    base_url: String,
    workspace_id: String,
    rich_document_id: String,
    script_path: String,
    script_url: String,
    /// 1-based breakpoint line (`const sum = a + b;`).
    breakpoint_line: u32,
    node_available: bool,
}

#[derive(Debug, Serialize)]
struct ProofResponse {
    breakpoint_lines: Vec<i32>,
    receipt_event_ids: Vec<String>,
    receipt_event_types: Vec<String>,
}

struct FixtureState {
    app: AppState,
    rich_document_id: String,
}

type SharedFixture = Arc<FixtureState>;

// A deterministic script with a stable breakpoint line. Lines (1-based):
//   1: function add(a, b) {
//   2:   const sum = a + b;     <- breakpoint here; a=2, b=40 in scope
//   3:   return sum;
//   4: }
//   5: const result = add(2, 40);
//   6: console.log("result=" + result);
const FIXTURE_SCRIPT: &str = "function add(a, b) {\n  const sum = a + b;\n  return sum;\n}\nconst result = add(2, 40);\nconsole.log(\"result=\" + result);\n";
const BREAKPOINT_LINE: u32 = 2;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT254_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(base_url) = base_database_url().await? else {
        println!("MT254_FIXTURE_SKIP PostgreSQL binaries not found");
        return Ok(());
    };

    // Write the real Node fixture script under the OS temp root.
    let script_path: PathBuf =
        std::env::temp_dir().join(format!("mt254-debug-{}.js", Uuid::now_v7().simple()));
    std::fs::write(&script_path, FIXTURE_SCRIPT)?;
    let script_path_str = script_path.to_string_lossy().to_string();
    let script_url = path_to_file_url(&script_path_str);

    let schema_url = isolated_schema_url(&base_url).await?;
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;

    // Seed a workspace + rich document so durable breakpoint FK constraints hold.
    let workspace = db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("mt254-debug-{}", Uuid::now_v7()),
            },
        )
        .await?;
    let workspace_id = workspace.id;
    let doc = db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: workspace_id.clone(),
            document_id: None,
            title: "Debug Fixture Doc".to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: json!({
                "type": "doc",
                "content": [{"type": "paragraph", "content": [{"type": "text", "text": "debug"}]}]
            }),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            ..Default::default()
        })
        .await?;
    let rich_document_id = doc.rich_document_id.clone();
    drop(db);

    let app = app_state_for(&schema_url).await?;
    let fixture: SharedFixture = Arc::new(FixtureState {
        app: app.clone(),
        rich_document_id: rich_document_id.clone(),
    });

    let router = app_router(app, fixture);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        workspace_id,
        rich_document_id,
        script_path: script_path_str,
        script_url,
        breakpoint_line: BREAKPOINT_LINE,
        node_available: node_available(),
    };
    println!("MT254_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, router).await?;
    Ok(())
}

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn path_to_file_url(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.chars().nth(1) == Some(':') {
        format!("file:///{normalized}")
    } else if let Some(stripped) = normalized.strip_prefix('/') {
        format!("file:///{stripped}")
    } else {
        format!("file:///{normalized}")
    }
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
            eprintln!("SKIP MT-254 fixture: PostgreSQL binaries not found ({detail})");
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}

async fn isolated_schema_url(base_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = format!("mt254_debug_session_{}", Uuid::now_v7().simple());
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
            "mt254-debug-session-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    })
}

fn app_router(state: AppState, fixture: SharedFixture) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    // Real product debug-adapter routes (live sessions + PG-backed breakpoints).
    let api_routes = api::routes(state.clone());
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/mt254-fixture/proof", get(fixture_proof))
        .with_state(fixture)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn fixture_proof(
    State(fixture): State<SharedFixture>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    let db = PostgresDatabase::new(fixture.app.postgres_pool.clone());
    let breakpoints = db
        .list_debug_breakpoints(&fixture.rich_document_id)
        .await
        .map_err(internal)?;
    let breakpoint_lines = breakpoints.iter().map(|b| b.line).collect();

    let events = db
        .list_kernel_events_for_aggregate("debug_breakpoints", &fixture.rich_document_id)
        .await
        .map_err(internal)?;
    let receipt_event_ids = events.iter().map(|e| e.event_id.clone()).collect();
    let receipt_event_types = events
        .iter()
        .filter_map(|e| e.payload.get("type").and_then(Value::as_str).map(str::to_string))
        .collect();

    Ok(Json(ProofResponse {
        breakpoint_lines,
        receipt_event_ids,
        receipt_event_types,
    }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
