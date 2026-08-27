// WP-KERNEL-009 / MT-254 — DebugAdapterCore real-backend fixture.
//
// Spins up the real product debug-adapter REST surface (`api::routes`) against
// an isolated embedded SurrealDB store, AND writes a real Node fixture script
// to disk.
// The offline Playwright spec drives the built DebugSidePanel/DebugConsole
// harness against this backend:
//   * GET /debug/adapters (honesty gate: node only),
//   * POST /debug/sessions launches a REAL node child under --inspect-brk,
//   * set a breakpoint (REAL CDP verified), continue, hit it, read the stack +
//     variables (a=2, b=40), evaluate("a + b") == 42, step, continue, terminate,
//   * PUT durable breakpoints persisted to real SurrealDB + EventLedger.
//
// A proof endpoint reads the durable breakpoints + their EventLedger receipts
// back through the real storage contract so the spec can assert the persistence
// truly landed.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use handshake_core::{
    api,
    capabilities::CapabilityRegistry,
    flight_recorder::duckdb::DuckDbFlightRecorder,
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
use serde::Serialize;
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::watch};
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
    let backend = embedded_test_backend().await?;
    let script_path: PathBuf =
        std::env::temp_dir().join(format!("mt254-debug-{}.js", Uuid::now_v7().simple()));
    let body_result = run_server(&backend, &script_path).await;
    let session_cleanup_result = api::debug_adapter::terminate_all_sessions().await;
    let store_cleanup_result = backend.close_and_remove().await;
    let script_cleanup_result = remove_temp_script(&script_path);
    let mut cleanup_errors = Vec::new();
    if let Err(error) = session_cleanup_result {
        cleanup_errors.push(format!("live debug-session cleanup failed: {error}"));
    }
    if let Err(error) = store_cleanup_result {
        cleanup_errors.push(format!("embedded-store cleanup failed: {error}"));
    }
    if let Err(error) = script_cleanup_result {
        cleanup_errors.push(format!("temp-script cleanup failed: {error}"));
    }
    let cleanup_result: Result<(), Box<dyn std::error::Error>> = if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(std::io::Error::other(cleanup_errors.join("; ")).into())
    };

    match (body_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(body_error), Ok(())) => Err(body_error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error),
        (Err(body_error), Err(cleanup_error)) => Err(std::io::Error::other(format!(
            "fixture body failed: {body_error}; fixture cleanup also failed: {cleanup_error}"
        ))
        .into()),
    }
}

async fn run_server(
    backend: &EmbeddedTestBackend,
    script_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write the real Node fixture script under the OS temp root.
    std::fs::write(&script_path, FIXTURE_SCRIPT)?;
    let script_path_str = script_path.to_string_lossy().to_string();
    let script_url = path_to_file_url(&script_path_str);

    let db = SurrealDatabase::new(backend.storage.clone());

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
    let app = app_state_for(backend)?;
    let fixture: SharedFixture = Arc::new(FixtureState {
        app: app.clone(),
        rich_document_id: rich_document_id.clone(),
    });

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let router = app_router(app, fixture).route(
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
        rich_document_id,
        script_path: script_path_str,
        script_url,
        breakpoint_line: BREAKPOINT_LINE,
        node_available: node_available(),
    };
    println!("MT254_FIXTURE_READY {}", serde_json::to_string(&ready)?);

    axum::serve(listener, router)
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

fn remove_temp_script(path: &PathBuf) -> Result<(), std::io::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    if path.exists() {
        return Err(std::io::Error::other(format!(
            "temp script still exists after teardown: {}",
            path.display()
        )));
    }
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

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt254-debug-session-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

fn app_router(state: AppState, fixture: SharedFixture) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    // Real product debug-adapter routes (live sessions + SurrealDB-backed breakpoints).
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
    let breakpoints = fixture
        .app
        .storage
        .list_debug_breakpoints(&fixture.rich_document_id)
        .await
        .map_err(internal)?;
    let breakpoint_lines = breakpoints.iter().map(|b| b.line).collect();

    let events = fixture
        .app
        .storage
        .list_kernel_events_for_aggregate("debug_breakpoints", &fixture.rich_document_id)
        .await
        .map_err(internal)?;
    let receipt_event_ids = events.iter().map(|e| e.event_id.clone()).collect();
    let receipt_event_types = events
        .iter()
        .filter_map(|e| {
            e.payload
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
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
