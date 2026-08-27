use std::{collections::HashSet, net::SocketAddr, sync::Arc};

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
    llm::DisabledLlmClient,
    storage::{
        surreal::SurrealDatabase,
        tests::{embedded_test_backend, EmbeddedTestBackend},
        Database, LoomBlockContentType, LoomBlockDerived, NewLoomBlock, NewWorkspace, WriteContext,
        LOOM_CANVAS_BOARD_SCHEMA_ID,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpListener, sync::watch};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ReadyMessage {
    base_url: String,
    workspace_id: String,
    canvas_block_id: String,
    blocks: Vec<SeedBlock>,
}

#[derive(Debug, Serialize, Clone)]
struct SeedBlock {
    block_id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct ProofQuery {
    canvas_block_id: String,
}

#[derive(Debug, Serialize)]
struct ProofResponse {
    /// content_type of the canvas LoomBlock (must be "canvas").
    canvas_content_type: String,
    /// All loom_blocks the canvas references via placements still exist.
    placed_blocks_present: Vec<String>,
    /// loom_edges count from a placed source -> placed target (semantic only).
    semantic_edge_count: i64,
    /// visual-only edge rows for this canvas (board-local; not graph authority).
    visual_edge_count: i64,
    /// Whether the canvas board row carries an EventLedger receipt.
    board_has_event_receipt: bool,
}

#[derive(Clone)]
struct FixtureState {
    app: AppState,
    workspace_id: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT261_FIXTURE_ERROR {error}");
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

    let (workspace_id, canvas_block_id, blocks) = seed_fixture(&db).await?;
    let state = app_state_for(backend)?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let app = app_router(FixtureState {
        app: state,
        workspace_id: workspace_id.clone(),
    })
    .route(
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
        canvas_block_id,
        blocks,
    };
    println!("MT261_FIXTURE_READY {}", serde_json::to_string(&ready)?);

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

async fn seed_fixture(
    db: &SurrealDatabase,
) -> Result<(String, String, Vec<SeedBlock>), Box<dyn std::error::Error>> {
    let ctx = WriteContext::human(None);
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("mt261-loom-canvas-{}", Uuid::now_v7()),
            },
        )
        .await?;
    let workspace_id = workspace.id;

    // Two draggable source blocks.
    let mut blocks = Vec::new();
    for title in ["Roadmap note", "Risk note"] {
        let block = db
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
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
            .await?;
        db.bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
            .await?;
        blocks.push(SeedBlock {
            block_id: block.block_id,
            title: title.to_string(),
        });
    }

    // The canvas IS a typed LoomBlock(content_type=canvas).
    let canvas = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some("Project canvas".to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await?;
    db.bridge_loom_block_to_knowledge(&ctx, &workspace_id, &canvas.block_id)
        .await?;
    db.create_canvas_board(
        &ctx,
        &workspace_id,
        &canvas.block_id,
        json!({
            "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
            "pan_x": 0.0,
            "pan_y": 0.0,
            "zoom": 1.0,
        }),
    )
    .await?;

    Ok((workspace_id, canvas.block_id, blocks))
}

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt261-loom-canvas-fixture".to_string(),
            "fixture does not call an LLM".to_string(),
        )),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

fn app_router(state: FixtureState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let api_routes = api::routes(state.app.clone());
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/mt261-fixture/proof", get(fixture_proof))
        .with_state(state)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn fixture_proof(
    State(state): State<FixtureState>,
    Query(query): Query<ProofQuery>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    let canvas_id = query.canvas_block_id;
    let block = state
        .app
        .storage
        .get_loom_block(&state.workspace_id, &canvas_id)
        .await
        .map_err(internal)?;
    let board = state
        .app
        .storage
        .get_canvas_board(&state.workspace_id, &canvas_id)
        .await
        .map_err(internal)?;
    let placed_ids = board
        .placements
        .iter()
        .map(|placement| placement.placed_block_id.as_str())
        .collect::<HashSet<_>>();
    let mut placed_blocks_present = Vec::with_capacity(board.placements.len());
    let mut semantic_edge_count = 0i64;
    for placement in &board.placements {
        state
            .app
            .storage
            .get_loom_block(&state.workspace_id, &placement.placed_block_id)
            .await
            .map_err(internal)?;
        placed_blocks_present.push(placement.placed_block_id.clone());
    }
    for placed_block_id in &placed_ids {
        semantic_edge_count += state
            .app
            .storage
            .get_outgoing_edges(&state.workspace_id, placed_block_id)
            .await
            .map_err(internal)?
            .into_iter()
            .filter(|edge| placed_ids.contains(edge.target_block_id.as_str()))
            .count() as i64;
    }
    let receipt_events = state
        .app
        .storage
        .list_kernel_events_for_aggregate("loom_canvas_board", &canvas_id)
        .await
        .map_err(internal)?;
    let board_has_event_receipt = receipt_events
        .iter()
        .any(|event| event.event_id == board.board.event_ledger_event_id);

    Ok(Json(ProofResponse {
        canvas_content_type: block.content_type.as_str().to_owned(),
        placed_blocks_present,
        semantic_edge_count,
        visual_edge_count: board.visual_edges.len() as i64,
        board_has_event_receipt,
    }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
