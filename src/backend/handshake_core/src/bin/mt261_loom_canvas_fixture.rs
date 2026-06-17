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
    llm::DisabledLlmClient,
    managed_postgres::{ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError},
    storage::{
        postgres::PostgresDatabase, Database, LoomBlockContentType, LoomBlockDerived, NewLoomBlock,
        NewWorkspace, WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Connection, Row};
use tokio::net::TcpListener;
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

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT261_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(base_url) = base_database_url().await? else {
        println!("MT261_FIXTURE_SKIP PostgreSQL binaries not found");
        return Ok(());
    };
    let schema_url = isolated_schema_url(&base_url).await?;
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;

    let (workspace_id, canvas_block_id, blocks) = seed_fixture(&db).await?;
    let state = app_state_for(&schema_url).await?;
    let app = app_router(state);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        workspace_id,
        canvas_block_id,
        blocks,
    };
    println!("MT261_FIXTURE_READY {}", serde_json::to_string(&ready)?);

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
            eprintln!("SKIP MT-261 fixture: PostgreSQL binaries not found ({detail})");
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}

async fn isolated_schema_url(base_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = format!("mt261_loom_canvas_{}", Uuid::now_v7().simple());
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

async fn seed_fixture(
    db: &PostgresDatabase,
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
            "mt261-loom-canvas-fixture".to_string(),
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
        .route("/mt261-fixture/proof", get(fixture_proof))
        .with_state(state)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn fixture_proof(
    State(state): State<AppState>,
    Query(query): Query<ProofQuery>,
) -> Result<Json<ProofResponse>, (StatusCode, String)> {
    let pool = state.postgres_pool.clone();
    let canvas_id = query.canvas_block_id;

    let canvas_content_type: String = sqlx::query(
        "SELECT content_type FROM loom_blocks WHERE block_id = $1",
    )
    .bind(&canvas_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?
    .get("content_type");

    let placed_rows = sqlx::query(
        r#"
        SELECT p.placed_block_id
        FROM loom_canvas_placements p
        JOIN loom_blocks b ON b.block_id = p.placed_block_id
        WHERE p.canvas_block_id = $1
        ORDER BY p.created_at ASC
        "#,
    )
    .bind(&canvas_id)
    .fetch_all(&pool)
    .await
    .map_err(internal)?;
    let placed_blocks_present = placed_rows
        .iter()
        .map(|r| r.get::<String, _>("placed_block_id"))
        .collect();

    // Semantic edges connecting two blocks that are both placed on this canvas.
    let semantic_edge_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM loom_edges e
        WHERE e.source_block_id IN (
                SELECT placed_block_id FROM loom_canvas_placements WHERE canvas_block_id = $1)
          AND e.target_block_id IN (
                SELECT placed_block_id FROM loom_canvas_placements WHERE canvas_block_id = $1)
        "#,
    )
    .bind(&canvas_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?;

    let visual_edge_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM loom_canvas_visual_edges WHERE canvas_block_id = $1",
    )
    .bind(&canvas_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?;

    let board_has_event_receipt: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM loom_canvas_boards b
            JOIN kernel_event_ledger k ON k.event_id = b.event_ledger_event_id
            WHERE b.block_id = $1
        )
        "#,
    )
    .bind(&canvas_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?;

    Ok(Json(ProofResponse {
        canvas_content_type,
        placed_blocks_present,
        semantic_edge_count,
        visual_edge_count,
        board_has_event_receipt,
    }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
