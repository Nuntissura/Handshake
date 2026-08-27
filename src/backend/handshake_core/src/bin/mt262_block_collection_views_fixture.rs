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
    llm::DisabledLlmClient,
    storage::{
        surreal::SurrealDatabase,
        tests::{embedded_test_backend, EmbeddedTestBackend},
        BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewQuery,
        BlockViewSort, BlockViewSortDirection, Database, LoomBlockContentType, LoomBlockDerived,
        LoomEdgeCreatedBy, LoomEdgeType, NewLoomBlock, NewLoomEdge, NewWorkspace, WriteContext,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpListener, sync::watch};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

/// MT-262 BlockCollectionViews offline-Playwright fixture: a REAL
/// embedded-SurrealDB-backed Handshake server seeded with saved table / Kanban /
/// calendar view_def blocks over real Loom blocks, plus a `/proof` endpoint
/// that reads canonical state through the real storage contract.

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

#[derive(Clone)]
struct FixtureState {
    app: AppState,
    workspace_id: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("MT262_FIXTURE_ERROR {error}");
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

    let ready_seed = seed_fixture(&db).await?;
    let state = app_state_for(backend)?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let app = app_router(FixtureState {
        app: state,
        workspace_id: ready_seed.workspace_id.clone(),
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
        ..ready_seed
    };
    println!("MT262_FIXTURE_READY {}", serde_json::to_string(&ready)?);

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

async fn make_block(
    db: &SurrealDatabase,
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
    db: &SurrealDatabase,
    workspace_id: &str,
    title: &str,
    definition: BlockViewDefinition,
) -> Result<String, Box<dyn std::error::Error>> {
    let ctx = WriteContext::human(None);
    let block_id = Uuid::now_v7().to_string();
    db.create_block_view(
        &ctx,
        workspace_id,
        &block_id,
        Some(title.to_string()),
        definition,
    )
    .await?;
    Ok(block_id)
}

async fn seed_fixture(db: &SurrealDatabase) -> Result<ReadyMessage, Box<dyn std::error::Error>> {
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
    let todo_tag_id = make_block(
        db,
        &workspace_id,
        "todo",
        LoomBlockContentType::TagHub,
        None,
    )
    .await?;
    let done_tag_id = make_block(
        db,
        &workspace_id,
        "done",
        LoomBlockContentType::TagHub,
        None,
    )
    .await?;
    let kanban_card_id = make_block(
        db,
        &workspace_id,
        "Ship MT-262",
        LoomBlockContentType::Note,
        None,
    )
    .await?;
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

fn app_state_for(backend: &EmbeddedTestBackend) -> Result<AppState, Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    Ok(AppState {
        storage: backend.database.clone(),
        surreal: backend.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(DisabledLlmClient::new(
            "mt262-block-views-fixture".to_string(),
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
        .route("/mt262-fixture/view-proof", get(view_proof))
        .route("/mt262-fixture/card-tags", get(card_tags))
        .with_state(state)
        .merge(api_routes.clone())
        .nest("/api", api_routes)
        .layer(cors)
}

async fn view_proof(
    State(state): State<FixtureState>,
    Query(query): Query<ViewProofQuery>,
) -> Result<Json<ViewProofResponse>, (StatusCode, String)> {
    let view = state
        .app
        .storage
        .get_block_view(&state.workspace_id, &query.block_id)
        .await
        .map_err(internal)?;
    let definition_json = serde_json::to_value(&view.definition).map_err(internal)?;
    let derived_json = serde_json::to_string(&view.block.derived).map_err(internal)?;
    let has_knowledge_bridge = state
        .app
        .storage
        .get_loom_block_knowledge_bridge(&state.workspace_id, &query.block_id)
        .await
        .map_err(internal)?
        .is_some();

    Ok(Json(ViewProofResponse {
        content_type: view.block.content_type.as_str().to_owned(),
        has_view_definition: definition_json.get("kind").is_some(),
        derived_json_leaks_definition: derived_json.contains("\"kind\""),
        has_knowledge_bridge,
    }))
}

async fn card_tags(
    State(state): State<FixtureState>,
    Query(query): Query<CardTagsQuery>,
) -> Result<Json<CardTagsResponse>, (StatusCode, String)> {
    let mut tag_target_ids = state
        .app
        .storage
        .get_outgoing_edges(&state.workspace_id, &query.block_id)
        .await
        .map_err(internal)?
        .into_iter()
        .filter(|edge| edge.edge_type == LoomEdgeType::Tag)
        .map(|edge| edge.target_block_id)
        .collect::<Vec<_>>();
    tag_target_ids.sort();
    Ok(Json(CardTagsResponse { tag_target_ids }))
}

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
