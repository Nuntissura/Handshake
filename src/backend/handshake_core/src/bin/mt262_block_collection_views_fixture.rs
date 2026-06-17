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
        postgres::PostgresDatabase, BlockViewDefinition, BlockViewField, BlockViewGroupBy,
        BlockViewKind, BlockViewQuery, BlockViewSort, BlockViewSortDirection, Database,
        LoomBlockContentType, LoomBlockDerived, LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock,
        NewLoomEdge, NewWorkspace, WriteContext,
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

/// MT-262 BlockCollectionViews offline-Playwright fixture: a REAL
/// PostgreSQL-backed Handshake server seeded with saved table / Kanban /
/// calendar view_def blocks over real Loom blocks, plus a `/proof` endpoint
/// that reads canonical state directly from PostgreSQL.

#[derive(Debug, Serialize)]
struct ReadyMessage {
    base_url: String,
    workspace_id: String,
    table_view_id: String,
    kanban_view_id: String,
    calendar_view_id: String,
    todo_tag_id: String,
    done_tag_id: String,
    kanban_card_id: String,
    table_block_count: usize,
}

#[derive(Debug, Deserialize)]
struct ViewProofQuery {
    block_id: String,
}

#[derive(Debug, Serialize)]
struct ViewProofResponse {
    /// content_type of the view block (must be "view_def").
    content_type: String,
    /// Whether the dedicated view_definition_json column is populated.
    has_view_definition: bool,
    /// Whether derived_json leaked the definition (must be false).
    derived_json_leaks_definition: bool,
    /// Whether the view block has a ProjectKnowledgeIndex bridge.
    has_knowledge_bridge: bool,
}

#[derive(Debug, Deserialize)]
struct CardTagsQuery {
    block_id: String,
}

#[derive(Debug, Serialize)]
struct CardTagsResponse {
    /// The tag (TagHub) block ids the card currently carries, from loom_edges.
    tag_target_ids: Vec<String>,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT262_FIXTURE_ERROR {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(base_url) = base_database_url().await? else {
        println!("MT262_FIXTURE_SKIP PostgreSQL binaries not found");
        return Ok(());
    };
    let schema_url = isolated_schema_url(&base_url).await?;
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;

    let ready_seed = seed_fixture(&db).await?;
    let state = app_state_for(&schema_url).await?;
    let app = app_router(state);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let addr = listener.local_addr()?;

    let ready = ReadyMessage {
        base_url: format!("http://{addr}"),
        ..ready_seed
    };
    println!("MT262_FIXTURE_READY {}", serde_json::to_string(&ready)?);

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
            eprintln!("SKIP MT-262 fixture: PostgreSQL binaries not found ({detail})");
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}

async fn isolated_schema_url(base_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let schema = format!("mt262_block_views_{}", Uuid::now_v7().simple());
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

async fn make_block(
    db: &PostgresDatabase,
    workspace_id: &str,
    title: &str,
    content_type: LoomBlockContentType,
    journal_date: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await?;
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await?;
    Ok(block.block_id)
}

async fn make_view(
    db: &PostgresDatabase,
    workspace_id: &str,
    title: &str,
    definition: BlockViewDefinition,
) -> Result<String, Box<dyn std::error::Error>> {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
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
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await?;
    db.create_block_view(&ctx, workspace_id, &block.block_id, Some(title.to_string()), definition)
        .await?;
    Ok(block.block_id)
}

async fn seed_fixture(db: &PostgresDatabase) -> Result<ReadyMessage, Box<dyn std::error::Error>> {
    let ctx = WriteContext::human(None);
    let workspace = db
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: format!("mt262-block-views-{}", Uuid::now_v7()),
            },
        )
        .await?;
    let workspace_id = workspace.id;

    // Table seed: more rows than the page limit, with deterministic titles.
    let table_block_count = 7usize;
    for i in 0..table_block_count {
        make_block(
            db,
            &workspace_id,
            &format!("Row {i:02}"),
            LoomBlockContentType::Note,
            None,
        )
        .await?;
    }

    // Kanban seed: two tag lanes + a card starting in "todo".
    let todo_tag_id = make_block(db, &workspace_id, "todo", LoomBlockContentType::TagHub, None).await?;
    let done_tag_id = make_block(db, &workspace_id, "done", LoomBlockContentType::TagHub, None).await?;
    let kanban_card_id =
        make_block(db, &workspace_id, "Ship MT-262", LoomBlockContentType::Note, None).await?;
    db.create_loom_edge(
        &ctx,
        NewLoomEdge {
            edge_id: None,
            workspace_id: workspace_id.clone(),
            source_block_id: kanban_card_id.clone(),
            target_block_id: todo_tag_id.clone(),
            edge_type: LoomEdgeType::Tag,
            created_by: LoomEdgeCreatedBy::User,
            crdt_site_id: None,
            source_anchor: None,
        },
    )
    .await?;

    // Calendar seed: three journals on distinct dates.
    for date in ["2026-06-10", "2026-06-15", "2026-06-20"] {
        db.get_or_create_daily_journal_block(&ctx, &workspace_id, date)
            .await?;
    }

    let table_view_id = make_view(
        db,
        &workspace_id,
        "All rows (A-Z)",
        BlockViewDefinition {
            kind: BlockViewKind::Table,
            query: BlockViewQuery {
                content_type: Some(LoomBlockContentType::Note),
                ..BlockViewQuery::default()
            },
            columns: vec![BlockViewField::Title, BlockViewField::Updated],
            group_by: None,
            sort: Some(BlockViewSort {
                field: BlockViewField::Title,
                direction: BlockViewSortDirection::Asc,
            }),
            calendar_date_field: None,
        },
    )
    .await?;

    let kanban_view_id = make_view(
        db,
        &workspace_id,
        "Status board",
        BlockViewDefinition {
            kind: BlockViewKind::Kanban,
            query: BlockViewQuery {
                content_type: Some(LoomBlockContentType::Note),
                tag_ids: vec![todo_tag_id.clone(), done_tag_id.clone()],
                ..BlockViewQuery::default()
            },
            columns: vec![BlockViewField::Title],
            group_by: Some(BlockViewGroupBy::Tag),
            sort: None,
            calendar_date_field: None,
        },
    )
    .await?;

    let calendar_view_id = make_view(
        db,
        &workspace_id,
        "June journal",
        BlockViewDefinition {
            kind: BlockViewKind::Calendar,
            query: BlockViewQuery {
                content_type: Some(LoomBlockContentType::Journal),
                ..BlockViewQuery::default()
            },
            columns: vec![],
            group_by: None,
            sort: Some(BlockViewSort {
                field: BlockViewField::JournalDate,
                direction: BlockViewSortDirection::Asc,
            }),
            calendar_date_field: Some(BlockViewField::JournalDate),
        },
    )
    .await?;

    Ok(ReadyMessage {
        base_url: String::new(),
        workspace_id,
        table_view_id,
        kanban_view_id,
        calendar_view_id,
        todo_tag_id,
        done_tag_id,
        kanban_card_id,
        table_block_count,
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
            "mt262-block-views-fixture".to_string(),
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
        .route("/mt262-fixture/view-proof", get(view_proof))
        .route("/mt262-fixture/card-tags", get(card_tags))
        .with_state(state)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn view_proof(
    State(state): State<AppState>,
    Query(query): Query<ViewProofQuery>,
) -> Result<Json<ViewProofResponse>, (StatusCode, String)> {
    let pool = state.postgres_pool.clone();
    let row = sqlx::query(
        "SELECT content_type, view_definition_json, derived_json FROM loom_blocks WHERE block_id = $1",
    )
    .bind(&query.block_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?;
    let content_type: String = row.get("content_type");
    let view_definition_json: Option<String> = row.get("view_definition_json");
    let derived_json: String = row.get("derived_json");

    let has_knowledge_bridge: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM loom_block_knowledge_bridge WHERE block_id = $1
        )
        "#,
    )
    .bind(&query.block_id)
    .fetch_one(&pool)
    .await
    .map_err(internal)?;

    Ok(Json(ViewProofResponse {
        content_type,
        has_view_definition: view_definition_json.is_some(),
        derived_json_leaks_definition: derived_json.contains("\"kind\""),
        has_knowledge_bridge,
    }))
}

async fn card_tags(
    State(state): State<AppState>,
    Query(query): Query<CardTagsQuery>,
) -> Result<Json<CardTagsResponse>, (StatusCode, String)> {
    let pool = state.postgres_pool.clone();
    let rows = sqlx::query(
        r#"
        SELECT target_block_id
        FROM loom_edges
        WHERE source_block_id = $1 AND edge_type = 'tag'
        ORDER BY target_block_id ASC
        "#,
    )
    .bind(&query.block_id)
    .fetch_all(&pool)
    .await
    .map_err(internal)?;
    let tag_target_ids = rows
        .iter()
        .map(|r| r.get::<String, _>("target_block_id"))
        .collect();
    Ok(Json(CardTagsResponse { tag_target_ids }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
