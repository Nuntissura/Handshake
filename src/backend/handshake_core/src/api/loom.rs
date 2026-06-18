use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::loom_fs::{loom_asset_blob_path, resolve_handshake_root};
use crate::models::ErrorResponse;
use crate::storage::{
    artifacts, Asset, BlockViewDefinition, BlockViewRecord, BlockViewResults, LoomBlock,
    LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate,
    LoomCanvasBoard, LoomCanvasBoardView, LoomCanvasPlacement, LoomCanvasPlacementUpdate,
    LoomCanvasVisualEdge, LoomEdge,
    LoomEdgeCreatedBy, LoomEdgeType, LoomGraphSearchResult, LoomSearchFilters,
    LoomSearchSourceKind, LoomViewFilters, LoomViewResponse, LoomViewType, LoomVisualDebugSnapshot,
    NewAsset, NewLoomBlock, NewLoomCanvasPlacement, NewLoomEdge, PreviewStatus, QuickSwitcherRecent,
    QuickSwitcherRecentInput, StorageCapabilityStore, StorageError, WriteContext,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

const DEFAULT_LOOM_GRAPH_DEPTH: u32 = 3;
const MAX_LOOM_GRAPH_DEPTH: u32 = 8;

fn bad_request(code: &'static str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: code }))
}

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core", error = %err, "loom_api_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-LOOM",
        }),
    )
}

fn map_storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Guard(_) | StorageError::Validation("HSK-403-SILENT-EDIT") => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-SILENT-EDIT",
            }),
        ),
        StorageError::Validation(_) => bad_request("HSK-400-LOOM-VALIDATION"),
        other => internal_error(other),
    }
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        // Loom blocks
        .route(
            "/workspaces/:workspace_id/loom/blocks",
            post(create_loom_block),
        )
        .route(
            "/workspaces/:workspace_id/loom/journals/:journal_date",
            put(open_daily_journal),
        )
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id",
            get(get_loom_block)
                .patch(patch_loom_block)
                .delete(delete_loom_block),
        )
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/metrics/recompute",
            post(recompute_loom_block_metrics),
        )
        // MT-177: ProjectKnowledgeIndex/EventLedger authority bridge
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/knowledge",
            get(get_loom_block_knowledge_bridge),
        )
        // MT-258: note transclusion read-through. Resolves a block to its SOURCE
        // rich document (loom_blocks.document_id -> knowledge_rich_documents) so
        // an embedding host doc renders the source content WITHOUT copying it.
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/transclusion",
            get(get_loom_block_transclusion),
        )
        // MT-183: reorderable Pins grid ordinal
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/pin-order",
            axum::routing::put(set_loom_block_pin_order),
        )
        // MT-188: navigation breadcrumbs across the entity spine
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/breadcrumbs",
            get(get_loom_block_breadcrumbs),
        )
        // MT-178: backlinks (linked, with context) + unlinked mentions
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/backlinks",
            get(get_loom_block_backlinks),
        )
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/unlinked-mentions",
            get(scan_loom_block_unlinked_mentions),
        )
        // MT-181: folder tree + color labels + sort modes
        .route(
            "/workspaces/:workspace_id/loom/folders",
            get(list_loom_folders).post(create_loom_folder),
        )
        .route(
            "/workspaces/:workspace_id/loom/folders/:folder_id",
            get(get_loom_folder)
                .patch(update_loom_folder)
                .delete(delete_loom_folder),
        )
        .route(
            "/workspaces/:workspace_id/loom/folders/:folder_id/blocks",
            get(list_loom_folder_blocks),
        )
        .route(
            "/workspaces/:workspace_id/loom/folders/:folder_id/blocks/:block_id",
            axum::routing::put(add_block_to_loom_folder).delete(remove_block_from_loom_folder),
        )
        // MT-184/185: wiki projection compiler + editable overlay.
        // MT-241/242/243 (project wiki compile layer): GET list serves every
        // page WITH its staleness verdict; bootstrap/drift-check/fanout are
        // the compile, drift, and incremental-regeneration surfaces.
        .route(
            "/workspaces/:workspace_id/loom/wiki",
            get(list_loom_wiki_pages).post(compile_loom_wiki_projection),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/bootstrap",
            post(bootstrap_project_wiki),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/drift-check",
            post(project_wiki_drift_check),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/fanout",
            post(project_wiki_fanout),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/:projection_id",
            get(get_loom_wiki_projection).delete(delete_loom_wiki_projection),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/:projection_id/regenerate",
            post(regenerate_loom_wiki_projection),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/:projection_id/stale",
            get(loom_wiki_projection_stale),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki/:projection_id/overlays",
            get(list_loom_wiki_overlays).post(add_loom_wiki_overlay),
        )
        .route(
            "/workspaces/:workspace_id/loom/wiki-overlays/:overlay_id",
            delete(delete_loom_wiki_overlay),
        )
        // MT-187: markdown import boundary (vault never authority)
        .route(
            "/workspaces/:workspace_id/loom/import/markdown",
            post(import_markdown_to_loom),
        )
        // MT-182: tag hubs (tags as first-class blocks) + nested tags
        .route(
            "/workspaces/:workspace_id/loom/tags",
            get(list_loom_tag_hubs),
        )
        .route(
            "/workspaces/:workspace_id/loom/tags/:tag_block_id",
            get(get_loom_tag_hub),
        )
        .route(
            "/workspaces/:workspace_id/loom/tags/:tag_block_id/blocks",
            get(list_loom_blocks_for_tag),
        )
        // Loom edges
        .route(
            "/workspaces/:workspace_id/loom/edges",
            post(create_loom_edge),
        )
        .route(
            "/workspaces/:workspace_id/loom/edges/:edge_id",
            delete(delete_loom_edge),
        )
        // Import + assets
        .route(
            "/workspaces/:workspace_id/loom/import",
            post(import_loom_asset),
        )
        .route(
            "/workspaces/:workspace_id/assets/:asset_id",
            get(get_asset_metadata),
        )
        .route(
            "/workspaces/:workspace_id/assets/:asset_id/content",
            get(get_asset_content),
        )
        .route(
            "/workspaces/:workspace_id/assets/:asset_id/thumbnail",
            get(get_asset_thumbnail),
        )
        // MT-259 MediaCacheTiers: tier state + visible retry queue
        .route(
            "/workspaces/:workspace_id/assets/:asset_id/tiers",
            get(list_asset_tiers),
        )
        .route(
            "/workspaces/:workspace_id/assets/:asset_id/tiers/:tier/retry",
            post(retry_asset_tier),
        )
        // MT-259 GAP-LM-244a: backend album/slideshow list-source
        .route(
            "/workspaces/:workspace_id/loom/collections",
            post(create_loom_collection),
        )
        .route(
            "/workspaces/:workspace_id/loom/collections/:collection_id",
            get(get_loom_collection),
        )
        .route(
            "/workspaces/:workspace_id/loom/collections/:collection_id/order",
            put(set_loom_collection_order),
        )
        // Views + search
        .route(
            "/workspaces/:workspace_id/loom/views/:view_type",
            get(query_loom_view),
        )
        .route(
            "/workspaces/:workspace_id/loom/graph/traverse",
            get(traverse_loom_graph),
        )
        // MT-179 local graph neighborhood (undirected, filters/depth/stale/citations)
        .route(
            "/workspaces/:workspace_id/loom/graph/local",
            get(local_loom_graph),
        )
        // MT-180 global project graph (performance limits + hub suppression)
        .route(
            "/workspaces/:workspace_id/loom/graph/global",
            get(global_loom_graph),
        )
        .route(
            "/workspaces/:workspace_id/loom/metrics/recompute",
            post(recompute_all_loom_metrics),
        )
        .route(
            "/workspaces/:workspace_id/loom/search",
            get(search_loom_blocks),
        )
        // MT-191: bounded backend visual-debug snapshot for Loom navigation state.
        .route(
            "/workspaces/:workspace_id/loom/visual-debug",
            get(loom_visual_debug_snapshot),
        )
        .route(
            "/workspaces/:workspace_id/loom/graph-search",
            get(search_loom_graph),
        )
        // MT-264: LoomSearchV2 -- Postgres-native, graph-blended hybrid search
        // (FTS + pg_trgm + pgvector kNN). Supersedes/extends the MT-258/250
        // workspace search entrypoint.
        .route(
            "/workspaces/:workspace_id/loom/search-v2",
            post(loom_search_v2),
        )
        .route(
            "/workspaces/:workspace_id/loom/quick-switcher/recents",
            get(list_quick_switcher_recents).post(record_quick_switcher_recent),
        )
        // MT-260: AI Loom jobs (auto-tag / auto-caption / link-suggest). Every
        // suggestion is a PENDING proposal requiring confirm-to-promote.
        .route(
            "/workspaces/:workspace_id/loom/ai-jobs",
            post(run_loom_ai_job),
        )
        .route(
            "/workspaces/:workspace_id/loom/ai-jobs/:job_id/accept-all",
            post(accept_all_loom_ai_suggestions),
        )
        .route(
            "/workspaces/:workspace_id/loom/ai-suggestions",
            get(list_loom_ai_suggestions),
        )
        .route(
            "/workspaces/:workspace_id/loom/ai-suggestions/:suggestion_id/accept",
            post(accept_loom_ai_suggestion),
        )
        .route(
            "/workspaces/:workspace_id/loom/ai-suggestions/:suggestion_id/reject",
            post(reject_loom_ai_suggestion),
        )
        // -- MT-261 CanvasBoard --------------------------------------------
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards",
            post(create_canvas_board),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id",
            get(get_canvas_board),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id/viewport",
            put(update_canvas_board_state),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id/placements",
            post(place_block_on_canvas),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id/cards",
            post(create_canvas_card),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-placements/:placement_id",
            patch(update_canvas_placement).delete(remove_canvas_placement),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id/visual-edges",
            post(add_canvas_visual_edge),
        )
        .route(
            "/workspaces/:workspace_id/loom/canvas-visual-edges/:visual_edge_id",
            delete(remove_canvas_visual_edge),
        )
        // MT-262 BlockCollectionViews: saved table/Kanban/calendar view defs.
        .route(
            "/workspaces/:workspace_id/loom/views/definitions",
            post(create_block_view),
        )
        .route(
            "/workspaces/:workspace_id/loom/views/definitions/:block_id",
            get(get_block_view).patch(update_block_view),
        )
        .route(
            "/workspaces/:workspace_id/loom/views/definitions/:block_id/results",
            post(query_block_view_results),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct CreateLoomBlockRequest {
    #[serde(default)]
    block_id: Option<String>,
    content_type: LoomBlockContentType,
    #[serde(default)]
    document_id: Option<String>,
    #[serde(default)]
    asset_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    pinned: Option<bool>,
    #[serde(default)]
    journal_date: Option<String>,
}

async fn create_loom_block(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateLoomBlockRequest>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let ctx = WriteContext::human(None);
    let block = state
        .storage
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: payload.block_id,
                workspace_id: workspace_id.clone(),
                content_type: payload.content_type.clone(),
                document_id: payload.document_id,
                asset_id: payload.asset_id.clone(),
                title: payload.title.clone(),
                original_filename: None,
                content_hash: None,
                pinned: payload.pinned.unwrap_or(false),
                journal_date: payload.journal_date,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    // MT-177: a LoomBlock is born resolving to ProjectKnowledgeIndex +
    // EventLedger authority. The bridge upserts a knowledge_entities row and
    // appends a KNOWLEDGE_LOOM_BLOCK_INDEXED receipt. Fail-closed: if the
    // authority bridge cannot be written the block create is an error, so a
    // block can never exist as a parallel-store-only row.
    state
        .storage
        .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?;

    let block_id = block.block_id.clone();
    let block_workspace_id = block.workspace_id.clone();
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_created",
            "block_id": block_id,
            "workspace_id": block_workspace_id,
            "content_type": block.content_type.as_str(),
            "asset_id": block.asset_id.clone(),
            "content_hash": block.content_hash.clone()
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    // WP-KERNEL-009 MT-264: refresh the semantic embedding projection so a
    // normally-created block is searchable by the semantic modality (not only
    // by tests that manually reindex). No-op decline when no model configured.
    refresh_loom_block_embedding(&state, &ctx, &block).await;

    Ok(Json(block))
}

/// WP-KERNEL-009 MT-264: refresh the LoomSearchV2 semantic (embedding)
/// projection for a block on the authority write path. The keyword/trigram
/// (`search_text`) projection is refreshed synchronously inside the storage
/// `create_loom_block` / `update_loom_block` / `get_or_create_daily_journal_block`
/// paths; the embedding modality additionally requires the model runtime, which
/// only the API layer holds (`state.llm_client`). This calls the same
/// `loom_search::reindex_block` the semantic tests use, so a created/edited
/// block's embedding is produced through the operator's configured model — and
/// is OMITTED (typed decline, no fabrication) when no embedding model is
/// configured. A reindex failure is non-fatal to the write that already
/// committed: it is recorded to the Flight Recorder so the block stays usable
/// while the embedding can be backfilled, rather than failing an otherwise
/// successful authority write.
async fn refresh_loom_block_embedding(state: &AppState, ctx: &WriteContext, block: &LoomBlock) {
    match crate::loom_search::reindex_block(state.storage.as_ref(), state.llm_client.as_ref(), ctx, block)
        .await
    {
        Ok(_wrote_embedding) => {}
        Err(err) => {
            let event = FlightRecorderEvent::new(
                FlightRecorderEventType::LoomBlockCreated,
                FlightRecorderActor::Human,
                Uuid::now_v7(),
                json!({
                    "type": "loom_search_v2_reindex_failed",
                    "block_id": block.block_id,
                    "workspace_id": block.workspace_id,
                    "error": err.to_string(),
                }),
            )
            .with_wsids(vec![block.workspace_id.clone()]);
            let _ = state.flight_recorder.record_event(event).await;
        }
    }
}

fn parse_journal_date(raw: &str) -> ApiResult<String> {
    let parsed = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| bad_request("HSK-400-LOOM-JOURNAL-DATE"))?;
    let canonical = parsed.format("%Y-%m-%d").to_string();
    if canonical != raw {
        return Err(bad_request("HSK-400-LOOM-JOURNAL-DATE"));
    }
    Ok(canonical)
}

async fn open_daily_journal(
    State(state): State<AppState>,
    Path((workspace_id, journal_date)): Path<(String, String)>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let journal_date = parse_journal_date(&journal_date)?;
    let ctx = WriteContext::human(None);
    let block = state
        .storage
        .get_or_create_daily_journal_block(&ctx, &workspace_id, &journal_date)
        .await
        .map_err(map_storage_error)?;

    // MT-177 authority still applies to the MT-257 daily journal path. The
    // bridge is idempotent, so repeated opens keep returning the same block
    // while proving it resolves through ProjectKnowledgeIndex + EventLedger.
    state
        .storage
        .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?;

    // WP-KERNEL-009 MT-264: refresh the semantic embedding projection for the
    // journal block (the keyword/trigram row is written in storage).
    refresh_loom_block_embedding(&state, &ctx, &block).await;

    Ok(Json(block))
}

async fn get_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let block = state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(block))
}

/// MT-177: read the LoomBlock <-> ProjectKnowledgeIndex/EventLedger authority
/// bridge. Returns the knowledge entity id + EventLedger receipt id that prove
/// the block resolves to Postgres/EventLedger authority. 404 if the block does
/// not exist; a 200 with a bridge body proves the authority binding.
async fn get_loom_block_knowledge_bridge(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomKnowledgeBridge>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    // Confirm the block exists first so a missing block is a clean 404 rather
    // than an empty-bridge 404.
    state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    let bridge = state
        .storage
        .get_loom_block_knowledge_bridge(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?
        .ok_or_else(|| not_found("loom_block_not_bridged"))?;
    Ok(Json(bridge))
}

/// MT-258: a note-transclusion read-through. Given a host doc that embeds a
/// LoomBlock as an atom node, the host editor needs the SOURCE document's live
/// content to render the embed. This resolves the block's source rich document
/// (`loom_blocks.document_id` -> `knowledge_rich_documents`) and returns its
/// current content JSON + version. The host doc NEVER persists this body; it
/// keeps only the atom node carrying `block_id`, so editing the source through
/// `save_knowledge_rich_document_version` flows to ONE authority document.
#[derive(Debug, Serialize)]
struct LoomTransclusionResponse {
    block_id: String,
    workspace_id: String,
    /// The source rich document id the block resolves to (the edit target).
    source_document_id: Option<String>,
    /// The current version of the source document (for optimistic save).
    source_doc_version: Option<i64>,
    /// The live source document JSON (ProseMirror doc node), or null when the
    /// block resolves to no rich document.
    content_json: Option<serde_json::Value>,
    /// `true` only when the block resolves to a real source rich document whose
    /// content was read through; `false` is a typed, visible unresolved state
    /// (never a silent blank).
    resolved: bool,
    /// A typed reason when `resolved` is false (no document_id, or the
    /// referenced rich document row is missing).
    #[serde(skip_serializing_if = "Option::is_none")]
    unresolved_reason: Option<&'static str>,
}

async fn get_loom_block_transclusion(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomTransclusionResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    // A missing block is a clean 404 (not an empty/unresolved 200).
    let block = state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    let Some(document_id) = block.document_id.clone() else {
        return Ok(Json(LoomTransclusionResponse {
            block_id,
            workspace_id,
            source_document_id: None,
            source_doc_version: None,
            content_json: None,
            resolved: false,
            unresolved_reason: Some("loom_block_has_no_source_document"),
        }));
    };

    // The rich-document authority lives on the KnowledgeStore (implemented on
    // PostgresDatabase), reached through the shared pool — mirroring the
    // knowledge_documents API's `db_for`. A LoomBlock's `document_id` is the
    // legacy `documents` anchor; the source rich document is the one anchored to
    // that same row (reuse loom_blocks.document_id + knowledge_rich_documents,
    // no new table/column).
    let knowledge_db = crate::storage::postgres::PostgresDatabase::new(state.postgres_pool.clone());
    let document =
        crate::storage::knowledge::KnowledgeStore::get_knowledge_rich_document_by_document_id(
            &knowledge_db,
            &workspace_id,
            &document_id,
        )
        .await
        .map_err(map_storage_error)?;

    let Some(document) = document else {
        return Ok(Json(LoomTransclusionResponse {
            block_id,
            workspace_id,
            source_document_id: Some(document_id),
            source_doc_version: None,
            content_json: None,
            resolved: false,
            unresolved_reason: Some("source_rich_document_missing"),
        }));
    };

    Ok(Json(LoomTransclusionResponse {
        block_id,
        workspace_id,
        source_document_id: Some(document.rich_document_id),
        source_doc_version: Some(document.doc_version),
        content_json: Some(document.content_json),
        resolved: true,
        unresolved_reason: None,
    }))
}

/// MT-188: the navigation breadcrumb trail for a block (workspace -> project ->
/// folder ancestry -> block -> ProjectKnowledgeIndex entity).
async fn get_loom_block_breadcrumbs(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomBreadcrumbTrail>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let trail = state
        .storage
        .loom_block_breadcrumbs(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(trail))
}

/// MT-178: linked backlinks for a block (incoming MENTION/TAG/... edges) each
/// with the referencing source block and a surrounding-text context snippet.
async fn get_loom_block_backlinks(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<crate::storage::LoomBacklink>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let backlinks = state
        .storage
        .get_backlinks_with_context(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(backlinks))
}

#[derive(Debug, Deserialize, Default)]
struct UnlinkedMentionQuery {
    /// Comma-separated extra alias terms to scan for beyond the block title.
    #[serde(default)]
    aliases: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

/// MT-178: unlinked mentions for a block — blocks whose text contains the
/// block's title/aliases on a word boundary but have no formal edge to it.
async fn scan_loom_block_unlinked_mentions(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Query(query): Query<UnlinkedMentionQuery>,
) -> ApiResult<Json<Vec<crate::storage::LoomUnlinkedMention>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let aliases = split_ids(query.aliases);
    let limit = query.limit.unwrap_or(100).min(500);
    let mentions = state
        .storage
        .scan_unlinked_mentions(&workspace_id, &block_id, &aliases, limit)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(mentions))
}

#[derive(Debug, Deserialize)]
struct SetPinOrderRequest {
    /// The new ordinal, or `null` to clear it (un-order the pin).
    #[serde(default)]
    pin_order: Option<i32>,
}

/// MT-183: set or clear a block's Pins-grid ordinal (reorderable grid).
async fn set_loom_block_pin_order(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<SetPinOrderRequest>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);
    let block = state
        .storage
        .set_loom_block_pin_order(&ctx, &workspace_id, &block_id, payload.pin_order)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": block.block_id,
            "fields_changed": ["pin_order"],
            "updated_by": "user",
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(block))
}

// -- MT-184/185 wiki projection + overlay handlers -------------------------
// -- MT-241/242/243 project wiki compile layer (LM-PWIKI-001..013) ----------

/// THE STALE-BADGE CONTRACT (MT-242, LM-PWIKI-008): every served wiki page
/// carries `staleness_verdict` — `{"state": "fresh", "stamp_ledger_version",
/// "current_ledger_version"}` | `{"state": "stale", …, "reasons": [{"kind",
/// "id", "stamped_content_hash", "current_content_hash", "change"}]}` |
/// `{"state": "unstamped"}`. `unstamped` must NEVER render as fresh. The
/// wrapper type makes serving without a verdict unrepresentable (fail-closed).
#[derive(Debug, Serialize)]
struct ServedWikiPage {
    #[serde(flatten)]
    page: crate::storage::LoomWikiProjection,
    staleness_verdict: crate::knowledge_wiki::WikiStalenessVerdict,
}

fn wiki_pg(state: &AppState) -> std::sync::Arc<crate::storage::postgres::PostgresDatabase> {
    std::sync::Arc::new(crate::storage::postgres::PostgresDatabase::new(
        state.postgres_pool.clone(),
    ))
}

fn map_wiki_error(err: crate::knowledge_wiki::WikiCompileError) -> ApiError {
    use crate::knowledge_wiki::WikiCompileError;
    match err {
        WikiCompileError::Validation(_) => bad_request("HSK-400-LOOM-VALIDATION"),
        WikiCompileError::PageCapExceeded(_) => bad_request("HSK-400-WIKI-PAGE-CAP"),
        WikiCompileError::Storage(inner) => map_storage_error(inner),
        WikiCompileError::Kernel(inner) => internal_error(inner),
    }
}

/// Optional caller identity for wiki compile receipts (same `x-hsk-*` headers
/// as the code-nav API; absent headers fall back to an attributable system
/// identity so EventLedger receipts are ALWAYS written).
fn wiki_compile_context(
    headers: &axum::http::HeaderMap,
) -> crate::knowledge_wiki::compiler::WikiCompileContext {
    fn hdr<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
    let actor_id = hdr(headers, "x-hsk-actor-id")
        .unwrap_or("notes-wiki")
        .to_string();
    let actor = match hdr(headers, "x-hsk-actor-kind") {
        Some("operator") => crate::kernel::KernelActor::Operator(actor_id),
        Some("model_adapter") => crate::kernel::KernelActor::ModelAdapter(actor_id),
        Some("session_broker") => crate::kernel::KernelActor::SessionBroker(actor_id),
        Some("toolgate") => crate::kernel::KernelActor::ToolGate(actor_id),
        Some("validation_runner") => crate::kernel::KernelActor::ValidationRunner(actor_id),
        Some("promotion_gate") => crate::kernel::KernelActor::PromotionGate(actor_id),
        _ => crate::kernel::KernelActor::System(actor_id),
    };
    crate::knowledge_wiki::compiler::WikiCompileContext {
        actor,
        kernel_task_run_id: hdr(headers, "x-hsk-kernel-task-run-id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("KTR-wiki-{}", Uuid::now_v7().simple())),
        session_run_id: hdr(headers, "x-hsk-session-run-id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("SR-wiki-{}", Uuid::now_v7().simple())),
        correlation_id: hdr(headers, "x-hsk-correlation-id").map(|v| v.to_string()),
    }
}

/// Attach the MT-242 verdict to a page about to be served. Fail-closed: a
/// verdict-evaluation failure fails the serve (LM-PWIKI-008) — there is no
/// "serve without verdict" path.
async fn attach_wiki_verdict(
    state: &AppState,
    page: crate::storage::LoomWikiProjection,
) -> ApiResult<ServedWikiPage> {
    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_pg(state));
    let staleness_verdict = checker
        .evaluate_stamp_value(&page.workspace_id, page.compile_stamp.as_ref())
        .await
        .map_err(map_wiki_error)?;
    Ok(ServedWikiPage {
        page,
        staleness_verdict,
    })
}

#[derive(Debug, Deserialize)]
struct CompileWikiRequest {
    title: String,
    #[serde(default)]
    block_ids: Vec<String>,
}

async fn compile_loom_wiki_projection(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CompileWikiRequest>,
) -> ApiResult<Json<ServedWikiPage>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let projection = state
        .storage
        .compile_loom_wiki_projection(&workspace_id, &payload.title, &payload.block_ids)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomProjectionRebuilt,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_projection_rebuilt",
            "workspace_id": workspace_id,
            "projection_id": projection.projection_id,
            "operation": "compile",
            "source_block_count": projection.source_block_ids.len(),
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(attach_wiki_verdict(&state, projection).await?))
}

async fn get_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<ServedWikiPage>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let projection = state
        .storage
        .get_loom_wiki_projection(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    // LM-PWIKI-008: the single-page serve path attaches the verdict
    // fail-closed.
    Ok(Json(attach_wiki_verdict(&state, projection).await?))
}

async fn loom_wiki_projection_stale(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let projection = state
        .storage
        .get_loom_wiki_projection(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_pg(&state));
    let verdict = checker
        .evaluate_stamp_value(&workspace_id, projection.compile_stamp.as_ref())
        .await
        .map_err(map_wiki_error)?;
    // `stale` is derived from the verdict: anything not provably fresh is
    // stale (unstamped pages are forbidden to read as fresh, LM-PWIKI-008).
    Ok(Json(json!({
        "projection_id": projection_id,
        "stale": !verdict.is_fresh(),
        "verdict": verdict,
    })))
}

async fn regenerate_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<ServedWikiPage>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let projection = state
        .storage
        .regenerate_loom_wiki_projection(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomProjectionRebuilt,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_projection_rebuilt",
            "workspace_id": workspace_id,
            "projection_id": projection.projection_id,
            "operation": "regenerate",
            "source_block_count": projection.source_block_ids.len(),
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(attach_wiki_verdict(&state, projection).await?))
}

// -- MT-241 bootstrap / MT-242 drift / MT-243 fan-out handlers ---------------

#[derive(Debug, Default, Deserialize)]
struct ListWikiPagesQuery {
    #[serde(default)]
    page_type: Option<String>,
    #[serde(default)]
    typed_only: Option<bool>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

/// GET /workspaces/:ws/loom/wiki — the list serve path consumed by the Notes
/// UI and by retrieval (LM-PWIKI-013): every page is returned in its FULL
/// knowledge shape (citations in `source_records`, `compile_stamp`,
/// `page_links`) with its `staleness_verdict` attached (LM-PWIKI-008).
async fn list_loom_wiki_pages(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(params): Query<ListWikiPagesQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let db = wiki_pg(&state);
    let pages = db
        .list_knowledge_wiki_pages(
            &workspace_id,
            params.page_type.as_deref(),
            params.typed_only.unwrap_or(false),
            params.limit.unwrap_or(500),
            params.offset.unwrap_or(0),
        )
        .await
        .map_err(map_storage_error)?;
    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(db);
    let verdicts = checker
        .evaluate_pages(&workspace_id, &pages)
        .await
        .map_err(map_wiki_error)?;
    let served: Vec<serde_json::Value> = pages
        .into_iter()
        .zip(verdicts.into_iter())
        .map(|(page, verdict)| {
            let mut value = serde_json::to_value(&page).unwrap_or_else(|_| json!({}));
            value["staleness_verdict"] = serde_json::to_value(&verdict).unwrap_or_default();
            value
        })
        .collect();
    Ok(Json(json!({ "pages": served })))
}

#[derive(Debug, Default, Deserialize)]
struct BootstrapWikiRequest {
    #[serde(default)]
    page_token_budget: Option<usize>,
}

/// POST /workspaces/:ws/loom/wiki/bootstrap — MT-241: compile the project
/// wiki from existing authority (code index + knowledge entities/edges + rich
/// documents). EventLedger receives the compile receipts (LM-PWIKI-012).
async fn bootstrap_project_wiki(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
    payload: Option<Json<BootstrapWikiRequest>>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let request = payload.map(|Json(p)| p).unwrap_or_default();
    let ctx = wiki_compile_context(&headers);
    let db = wiki_pg(&state);
    let compiler = crate::knowledge_wiki::compiler::ProjectWikiCompiler::new(db.clone());
    let mut options = crate::knowledge_wiki::compiler::WikiBootstrapOptions::default();
    if let Some(budget) = request.page_token_budget {
        options.page_token_budget = budget;
    }
    let outcome = compiler
        .bootstrap(&ctx, &workspace_id, &options)
        .await
        .map_err(map_wiki_error)?;

    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(db);
    let verdicts = checker
        .evaluate_pages(&workspace_id, &outcome.pages)
        .await
        .map_err(map_wiki_error)?;
    let pages: Vec<serde_json::Value> = outcome
        .pages
        .iter()
        .zip(verdicts.into_iter())
        .map(|(page, verdict)| {
            let mut value = serde_json::to_value(page).unwrap_or_else(|_| json!({}));
            value["staleness_verdict"] = serde_json::to_value(&verdict).unwrap_or_default();
            value
        })
        .collect();

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomProjectionRebuilt,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_projection_rebuilt",
            "workspace_id": workspace_id,
            "operation": "wiki_bootstrap",
            "pages": pages.len(),
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(json!({
        "workspace_id": workspace_id,
        "pages": pages,
        "started_receipt_event_id": outcome.started_receipt_event_id,
        "completed_receipt_event_id": outcome.completed_receipt_event_id,
        "ledger_version": outcome.ledger_version,
        "module_pages": outcome.module_pages,
        "concept_pages": outcome.concept_pages,
        "entity_pages": outcome.entity_pages,
        "decision_pages": outcome.decision_pages,
        "split_clusters": outcome.split_clusters,
        "oversize_files": outcome.oversize_files,
    })))
}

#[derive(Debug, Default, Deserialize)]
struct WikiDriftCheckRequest {
    /// Persist `rebuild_status = 'stale'` marks for drifted pages
    /// (default true; the drift run is the canonical mark-stale surface).
    #[serde(default)]
    persist: Option<bool>,
}

/// POST /workspaces/:ws/loom/wiki/drift-check — MT-242: diff current
/// authority against every page stamp; returns exactly which pages are stale
/// and why, persists stale marks, appends the staleness-verdict receipt.
async fn project_wiki_drift_check(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
    payload: Option<Json<WikiDriftCheckRequest>>,
) -> ApiResult<Json<crate::knowledge_wiki::drift::WikiDriftReport>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let request = payload.map(|Json(p)| p).unwrap_or_default();
    let ctx = wiki_compile_context(&headers);
    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_pg(&state));
    let report = checker
        .check_workspace(&ctx, &workspace_id, request.persist.unwrap_or(true))
        .await
        .map_err(map_wiki_error)?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize)]
struct WikiFanOutHttpRequest {
    /// `source` | `entity` | `loom_block` | `rich_document`.
    source_kind: crate::knowledge_wiki::CitedSourceKind,
    source_id: String,
    #[serde(default)]
    budget: Option<usize>,
}

/// POST /workspaces/:ws/loom/wiki/fanout — MT-243: one changed source
/// regenerates exactly the pages whose stamps cite it (set equality with the
/// drift result), bounded by the budget, with LOUD truncation receipts.
async fn project_wiki_fanout(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<WikiFanOutHttpRequest>,
) -> ApiResult<Json<crate::knowledge_wiki::fanout::WikiFanOutOutcome>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = wiki_compile_context(&headers);
    let engine = crate::knowledge_wiki::fanout::WikiFanOutEngine::new(wiki_pg(&state));
    let mut request = crate::knowledge_wiki::fanout::WikiFanOutRequest::new(
        payload.source_kind,
        payload.source_id,
    );
    if let Some(budget) = payload.budget {
        request.budget = budget;
    }
    let outcome = engine
        .run(&ctx, &workspace_id, &request)
        .await
        .map_err(map_wiki_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomProjectionRebuilt,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_projection_rebuilt",
            "workspace_id": workspace_id,
            "operation": "wiki_fanout",
            "trigger_kind": outcome.trigger_kind.as_str(),
            "trigger_id": outcome.trigger_id,
            "regenerated": outcome.regenerated.len(),
            "truncated": outcome.truncated.len(),
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(outcome))
}

async fn delete_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .delete_loom_wiki_projection(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "status": "deleted" })))
}

#[derive(Debug, Deserialize)]
struct AddWikiOverlayRequest {
    annotation: String,
    #[serde(default)]
    anchor: Option<String>,
}

async fn add_loom_wiki_overlay(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
    Json(payload): Json<AddWikiOverlayRequest>,
) -> ApiResult<Json<crate::storage::LoomWikiOverlay>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let overlay = state
        .storage
        .add_loom_wiki_overlay(
            &workspace_id,
            &projection_id,
            &payload.annotation,
            payload.anchor.as_deref(),
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(overlay))
}

async fn list_loom_wiki_overlays(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<Vec<crate::storage::LoomWikiOverlay>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let overlays = state
        .storage
        .list_loom_wiki_overlays(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(overlays))
}

async fn delete_loom_wiki_overlay(
    State(state): State<AppState>,
    Path((workspace_id, overlay_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .delete_loom_wiki_overlay(&workspace_id, &overlay_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "status": "deleted" })))
}

// -- MT-187 markdown import boundary handler -------------------------------

#[derive(Debug, Deserialize)]
struct ImportMarkdownRequest {
    title: String,
    markdown: String,
}

async fn import_markdown_to_loom(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<ImportMarkdownRequest>,
) -> ApiResult<Json<crate::storage::LoomMarkdownImport>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);
    let imported = state
        .storage
        .import_markdown_to_loom(&ctx, &workspace_id, &payload.title, &payload.markdown)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_markdown_imported",
            "workspace_id": workspace_id,
            "block_id": imported.block.block_id,
            "rich_document_id": imported.rich_document_id,
            "warning_count": imported.warnings.len(),
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(imported))
}

// -- MT-181 FolderTreeAndColorLabels handlers ------------------------------

#[derive(Debug, Deserialize)]
struct CreateLoomFolderRequest {
    name: String,
    #[serde(default)]
    parent_folder_id: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    sort_mode: Option<crate::storage::LoomFolderSortMode>,
    #[serde(default)]
    sort_order: Option<i32>,
    #[serde(default)]
    project_ref: Option<String>,
}

async fn create_loom_folder(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateLoomFolderRequest>,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let folder = state
        .storage
        .create_loom_folder(
            &workspace_id,
            crate::storage::NewLoomFolder {
                folder_id: None,
                workspace_id: workspace_id.clone(),
                parent_folder_id: payload.parent_folder_id,
                name: payload.name,
                color: payload.color,
                sort_mode: payload
                    .sort_mode
                    .unwrap_or(crate::storage::LoomFolderSortMode::UpdatedDesc),
                sort_order: payload.sort_order,
                project_ref: payload.project_ref,
            },
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(folder))
}

async fn list_loom_folders(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<Vec<crate::storage::LoomFolder>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let folders = state
        .storage
        .list_loom_folders(&workspace_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(folders))
}

async fn get_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let folder = state
        .storage
        .get_loom_folder(&workspace_id, &folder_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(folder))
}

async fn update_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
    Json(update): Json<crate::storage::LoomFolderUpdate>,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let folder = state
        .storage
        .update_loom_folder(&workspace_id, &folder_id, update)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(folder))
}

async fn delete_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .delete_loom_folder(&workspace_id, &folder_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "status": "deleted" })))
}

#[derive(Debug, Deserialize, Default)]
struct AddFolderMemberRequest {
    #[serde(default)]
    sort_order: Option<i32>,
}

async fn add_block_to_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id, block_id)): Path<(String, String, String)>,
    Json(payload): Json<AddFolderMemberRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .add_block_to_loom_folder(&workspace_id, &folder_id, &block_id, payload.sort_order)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "status": "added" })))
}

async fn remove_block_from_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id, block_id)): Path<(String, String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .remove_block_from_loom_folder(&workspace_id, &folder_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "status": "removed" })))
}

#[derive(Debug, Deserialize, Default)]
struct LoomFolderBlocksQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

async fn list_loom_folder_blocks(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
    Query(query): Query<LoomFolderBlocksQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);
    let blocks = state
        .storage
        .list_loom_folder_blocks(&workspace_id, &folder_id, limit, offset)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(blocks))
}

#[derive(Debug, Deserialize, Default)]
struct LoomTagListQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

/// MT-182: list all tag-hub blocks (tags as first-class blocks).
async fn list_loom_tag_hubs(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomTagListQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);
    let tags = state
        .storage
        .list_tag_hubs(&workspace_id, limit, offset)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(tags))
}

/// MT-182: the tag-hub surface (block + sub-tags + tagged blocks + backlinks).
async fn get_loom_tag_hub(
    State(state): State<AppState>,
    Path((workspace_id, tag_block_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomTagHub>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let hub = state
        .storage
        .get_tag_hub(&workspace_id, &tag_block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(hub))
}

#[derive(Debug, Deserialize, Default)]
struct LoomTagBlocksQuery {
    /// Include blocks tagged with descendant sub-tags (nested-tag membership).
    #[serde(default)]
    include_subtags: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

/// MT-182: blocks tagged with a tag (search filter by tag; optional nested).
async fn list_loom_blocks_for_tag(
    State(state): State<AppState>,
    Path((workspace_id, tag_block_id)): Path<(String, String)>,
    Query(query): Query<LoomTagBlocksQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let include_subtags = query.include_subtags.unwrap_or(false);
    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);
    let blocks = state
        .storage
        .list_blocks_for_tag(&workspace_id, &tag_block_id, include_subtags, limit, offset)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(blocks))
}

/// MT-258 properties-panel patch: the typed LoomBlock fields PLUS tag editing.
/// Tags are NOT a column — they are `tag` loom_edges from this block to a
/// TagHub block. `add_tags`/`remove_tags` carry TagHub block ids; adding
/// creates a tag edge (reusing the create-edge path), removing deletes the
/// matching tag edge. After tag mutations the block metrics are recomputed so
/// the returned `derived.tag_count` reflects the real edge set.
#[derive(Debug, Deserialize, Default)]
struct LoomBlockPatchRequest {
    #[serde(flatten)]
    update: LoomBlockUpdate,
    /// TagHub block ids to attach as `tag` edges (reuses create_loom_edge).
    #[serde(default)]
    add_tags: Vec<String>,
    /// TagHub block ids whose `tag` edge from this block should be removed.
    #[serde(default)]
    remove_tags: Vec<String>,
}

async fn patch_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<LoomBlockPatchRequest>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let LoomBlockPatchRequest {
        update,
        add_tags,
        remove_tags,
    } = payload;

    let mut fields_changed: Vec<&'static str> = Vec::new();
    if update.title.is_some() {
        fields_changed.push("title");
    }
    if update.pinned.is_some() {
        fields_changed.push("pinned");
    }
    if update.favorite.is_some() {
        fields_changed.push("favorite");
    }
    if update.journal_date.is_some() {
        fields_changed.push("journal_date");
    }
    if update.pin_order.is_some() {
        fields_changed.push("pin_order");
    }

    let mut block = state
        .storage
        .update_loom_block(&ctx, &workspace_id, &block_id, update)
        .await
        .map_err(map_storage_error)?;

    let tags_mutated = !add_tags.is_empty() || !remove_tags.is_empty();
    if tags_mutated {
        // The current tag edges from this block (so add is idempotent and remove
        // can locate the precise edge id to delete).
        let edges = state
            .storage
            .list_loom_edges_for_block(&workspace_id, &block_id)
            .await
            .map_err(map_storage_error)?;

        for tag_block_id in &add_tags {
            // The target must be a real TagHub block (parity with create_loom_edge).
            let target = state
                .storage
                .get_loom_block(&workspace_id, tag_block_id)
                .await
                .map_err(map_storage_error)?;
            if !matches!(target.content_type, LoomBlockContentType::TagHub) {
                return Err(bad_request("HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB"));
            }
            let already_tagged = edges.iter().any(|edge| {
                edge.edge_type == LoomEdgeType::Tag
                    && edge.source_block_id == block_id
                    && edge.target_block_id == *tag_block_id
            });
            if already_tagged {
                continue;
            }
            state
                .storage
                .create_loom_edge(
                    &ctx,
                    NewLoomEdge {
                        edge_id: None,
                        workspace_id: workspace_id.clone(),
                        source_block_id: block_id.clone(),
                        target_block_id: tag_block_id.clone(),
                        edge_type: LoomEdgeType::Tag,
                        created_by: LoomEdgeCreatedBy::User,
                        crdt_site_id: None,
                        source_anchor: None,
                    },
                )
                .await
                .map_err(map_storage_error)?;
        }

        for tag_block_id in &remove_tags {
            for edge in edges.iter().filter(|edge| {
                edge.edge_type == LoomEdgeType::Tag
                    && edge.source_block_id == block_id
                    && edge.target_block_id == *tag_block_id
            }) {
                state
                    .storage
                    .delete_loom_edge(&ctx, &workspace_id, &edge.edge_id)
                    .await
                    .map_err(map_storage_error)?;
            }
        }

        fields_changed.push("tags");
        // Recompute derived metrics so the returned tag_count is authoritative.
        state
            .storage
            .recompute_block_metrics(&workspace_id, &block_id)
            .await
            .map_err(map_storage_error)?;
        block = state
            .storage
            .get_loom_block(&workspace_id, &block_id)
            .await
            .map_err(map_storage_error)?;
    }

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": block_id,
            "fields_changed": fields_changed,
            "tags_added": add_tags,
            "tags_removed": remove_tags,
            "updated_by": "user"
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    // WP-KERNEL-009 MT-264: an edited title/text changes the block's flattened
    // search text, so refresh the semantic embedding projection too (the
    // keyword/trigram row is refreshed in storage update_loom_block). No-op
    // decline when no embedding model is configured.
    refresh_loom_block_embedding(&state, &ctx, &block).await;

    Ok(Json(block))
}

async fn delete_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let existing = state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    let ctx = WriteContext::human(None);
    state
        .storage
        .delete_loom_block(&ctx, &workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockDeleted,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_deleted",
            "block_id": block_id,
            "workspace_id": workspace_id,
            "content_type": existing.content_type.as_str(),
            "had_asset": existing.asset_id.is_some(),
        }),
    )
    .with_wsids(vec![existing.workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(json!({ "status": "deleted" })))
}

#[derive(Debug, Deserialize)]
struct CreateLoomEdgeRequest {
    #[serde(default)]
    edge_id: Option<String>,
    source_block_id: String,
    target_block_id: String,
    edge_type: LoomEdgeType,
    created_by: LoomEdgeCreatedBy,
    #[serde(default)]
    crdt_site_id: Option<String>,
    #[serde(default)]
    source_anchor: Option<crate::storage::LoomSourceAnchor>,
    #[serde(default)]
    target_title: Option<String>,
}

async fn ensure_edge_target_exists(
    state: &AppState,
    workspace_id: &str,
    edge_type: &LoomEdgeType,
    target_block_id: &str,
    target_title: Option<String>,
) -> ApiResult<()> {
    match state
        .storage
        .get_loom_block(workspace_id, target_block_id)
        .await
    {
        Ok(_) => Ok(()),
        Err(StorageError::NotFound(_)) => {
            let (content_type, title) = match edge_type {
                LoomEdgeType::Mention => (LoomBlockContentType::Note, target_title),
                LoomEdgeType::Tag | LoomEdgeType::SubTag => {
                    (LoomBlockContentType::TagHub, target_title)
                }
                LoomEdgeType::Parent | LoomEdgeType::AiSuggested => {
                    (LoomBlockContentType::Note, target_title)
                }
            };

            let title = title.ok_or_else(|| bad_request("HSK-400-LOOM-TARGET-TITLE-REQUIRED"))?;
            let ctx = WriteContext::human(None);
            let created = state
                .storage
                .create_loom_block(
                    &ctx,
                    NewLoomBlock {
                        block_id: Some(target_block_id.to_string()),
                        workspace_id: workspace_id.to_string(),
                        content_type,
                        document_id: None,
                        asset_id: None,
                        title: Some(title),
                        original_filename: None,
                        content_hash: None,
                        pinned: false,
                        journal_date: None,
                        imported_at: None,
                        derived: LoomBlockDerived::default(),
                    },
                )
                .await
                .map_err(map_storage_error)?;
            // MT-177: an auto-created link/tag target is also a LoomBlock and
            // must resolve to ProjectKnowledgeIndex + EventLedger authority.
            state
                .storage
                .bridge_loom_block_to_knowledge(&ctx, workspace_id, &created.block_id)
                .await
                .map_err(map_storage_error)?;
            Ok(())
        }
        Err(err) => Err(map_storage_error(err)),
    }
}

async fn create_loom_edge(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateLoomEdgeRequest>,
) -> ApiResult<Json<LoomEdge>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    state
        .storage
        .get_loom_block(&workspace_id, &payload.source_block_id)
        .await
        .map_err(map_storage_error)?;

    ensure_edge_target_exists(
        &state,
        &workspace_id,
        &payload.edge_type,
        &payload.target_block_id,
        payload.target_title.clone(),
    )
    .await?;

    if matches!(payload.edge_type, LoomEdgeType::Tag | LoomEdgeType::SubTag) {
        let target = state
            .storage
            .get_loom_block(&workspace_id, &payload.target_block_id)
            .await
            .map_err(map_storage_error)?;
        if !matches!(target.content_type, LoomBlockContentType::TagHub) {
            return Err(bad_request("HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB"));
        }
    }

    if let Some(anchor) = &payload.source_anchor {
        if anchor.offset_start < 0
            || anchor.offset_end < 0
            || anchor.offset_end < anchor.offset_start
        {
            return Err(bad_request("HSK-400-LOOM-INVALID-SOURCE-ANCHOR"));
        }
    }

    let ctx = WriteContext::human(None);
    let edge = state
        .storage
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: payload.edge_id,
                workspace_id: workspace_id.clone(),
                source_block_id: payload.source_block_id.clone(),
                target_block_id: payload.target_block_id.clone(),
                edge_type: payload.edge_type.clone(),
                created_by: payload.created_by.clone(),
                crdt_site_id: payload.crdt_site_id,
                source_anchor: payload.source_anchor,
            },
        )
        .await
        .map_err(map_storage_error)?;

    let edge_event = json!({
        "type": "loom_edge_created",
        "edge_id": edge.edge_id.clone(),
        "source_block_id": edge.source_block_id.clone(),
        "target_block_id": edge.target_block_id.clone(),
        "edge_type": edge.edge_type.as_str(),
        "created_by": edge.created_by.as_str(),
    });
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomEdgeCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        edge_event,
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(edge))
}

async fn delete_loom_edge(
    State(state): State<AppState>,
    Path((workspace_id, edge_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomEdge>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let ctx = WriteContext::human(None);
    let edge = state
        .storage
        .delete_loom_edge(&ctx, &workspace_id, &edge_id)
        .await
        .map_err(map_storage_error)?;

    let edge_event = json!({
        "type": "loom_edge_deleted",
        "edge_id": edge.edge_id.clone(),
        "edge_type": edge.edge_type.as_str(),
        "deleted_by": "user",
    });
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomEdgeDeleted,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        edge_event,
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(edge))
}

#[derive(Debug, Deserialize, Clone)]
struct LoomImportRequest {
    bytes_b64: String,
    #[serde(default)]
    original_filename: Option<String>,
    #[serde(default)]
    mime: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoomImportResult {
    dedup_hit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    existing_block_id: Option<String>,
    block_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_id: Option<String>,
    content_hash: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

async fn import_loom_asset(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<LoomImportRequest>,
) -> ApiResult<Json<LoomImportResult>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let bytes = STANDARD
        .decode(payload.bytes_b64.as_bytes())
        .map_err(|_| bad_request("HSK-400-LOOM-INVALID-BASE64"))?;
    let content_hash = sha256_hex(&bytes);

    if let Some(existing) = state
        .storage
        .find_loom_block_by_content_hash(&workspace_id, &content_hash)
        .await
        .map_err(map_storage_error)?
    {
        let attempted_filename = payload
            .original_filename
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let existing_block_id = existing.block_id.clone();
        let existing_workspace_id = existing.workspace_id.clone();
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::LoomDedupHit,
            FlightRecorderActor::Human,
            Uuid::now_v7(),
            json!({
                "type": "loom_dedup_hit",
                "workspace_id": workspace_id,
                "content_hash": content_hash,
                "existing_block_id": existing_block_id,
                "attempted_filename": attempted_filename,
            }),
        )
        .with_wsids(vec![existing_workspace_id]);
        let _ = state.flight_recorder.record_event(event).await;

        return Ok(Json(LoomImportResult {
            dedup_hit: true,
            existing_block_id: Some(existing.block_id.clone()),
            block_id: existing.block_id,
            asset_id: existing.asset_id,
            content_hash,
        }));
    }

    let handshake_root = resolve_handshake_root().map_err(internal_error)?;

    let mime = payload
        .mime
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let kind = "original".to_string();

    let ctx = WriteContext::human(None);

    let asset = match state
        .storage
        .find_asset_by_content_hash(&workspace_id, &content_hash)
        .await
        .map_err(map_storage_error)?
    {
        Some(existing) => existing,
        None => state
            .storage
            .create_asset(
                &ctx,
                NewAsset {
                    workspace_id: workspace_id.clone(),
                    kind: kind.clone(),
                    mime: mime.clone(),
                    original_filename: payload.original_filename.clone(),
                    content_hash: content_hash.clone(),
                    size_bytes: bytes.len() as i64,
                    width: None,
                    height: None,
                    classification: "low".to_string(),
                    exportable: true,
                    is_proxy_of: None,
                    proxy_asset_id: None,
                },
            )
            .await
            .map_err(map_storage_error)?,
    };

    let asset_path = loom_asset_blob_path(&handshake_root, &workspace_id, &kind, &content_hash);
    artifacts::write_file_atomic(&handshake_root, &asset_path, &bytes, false)
        .map_err(internal_error)?;

    let derived = LoomBlockDerived {
        preview_status: PreviewStatus::Pending,
        ..LoomBlockDerived::default()
    };

    let block = state
        .storage
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::File,
                document_id: None,
                asset_id: Some(asset.asset_id.clone()),
                title: None,
                original_filename: payload.original_filename.clone(),
                content_hash: Some(content_hash.clone()),
                pinned: false,
                journal_date: None,
                imported_at: Some(Utc::now()),
                derived,
            },
        )
        .await
        .map_err(map_storage_error)?;

    // MT-177: the imported file block resolves to ProjectKnowledgeIndex +
    // EventLedger authority before we report success.
    state
        .storage
        .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_created",
            "block_id": block.block_id.clone(),
            "workspace_id": block.workspace_id.clone(),
            "content_type": block.content_type.as_str(),
            "asset_id": asset.asset_id.clone(),
            "content_hash": content_hash.clone(),
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    let capability_profile_id = state
        .capability_registry
        .profile_for_job_request(
            crate::storage::JobKind::LoomPreviewGenerate.as_str(),
            "hsk.loom.preview_generate@v1",
        )
        .map_err(|e| internal_error(e))?;
    let job = crate::jobs::create_job(
        &state,
        crate::storage::JobKind::LoomPreviewGenerate,
        "hsk.loom.preview_generate@v1",
        capability_profile_id.id.as_str(),
        Some(json!({
            "workspace_id": workspace_id.clone(),
            "block_id": block.block_id.clone(),
            "asset_id": block.asset_id.clone(),
            "content_hash": content_hash.clone(),
            "requested_tier": 1,
        })),
        Vec::new(),
    )
    .await
    .map_err(|e| internal_error(e))?;

    let _ = crate::workflows::start_workflow_for_job(&state, job).await;

    Ok(Json(LoomImportResult {
        dedup_hit: false,
        existing_block_id: None,
        block_id: block.block_id,
        asset_id: block.asset_id,
        content_hash,
    }))
}

async fn get_asset_metadata(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
) -> ApiResult<Json<Asset>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let asset = state
        .storage
        .get_asset(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(asset))
}

#[derive(Debug, Deserialize, Default)]
struct AssetContentQuery {
    /// MT-259: serve a derived cache tier (thumb|preview|poster|full) instead of
    /// the original. Absent / `full` -> original blob.
    #[serde(default)]
    tier: Option<String>,
}

/// MT-259 GAP-LM-009b: parse a single-range `bytes=START-END` header against a
/// known total length. Returns `Ok(Some((start, end_inclusive)))` for a valid
/// satisfiable range, `Ok(None)` when there is no Range header, and
/// `Err(())` when the range is syntactically present but unsatisfiable (416).
fn parse_byte_range(headers: &axum::http::HeaderMap, total: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(value) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) else {
        return Ok(None);
    };
    let spec = match value.strip_prefix("bytes=") {
        Some(s) => s.trim(),
        None => return Err(()),
    };
    // Only the first range of a (possibly multi) spec is honored.
    let first = spec.split(',').next().unwrap_or("").trim();
    let (start_s, end_s) = match first.split_once('-') {
        Some(parts) => parts,
        None => return Err(()),
    };
    if total == 0 {
        return Err(());
    }
    let last = total - 1;
    let (start, end) = if start_s.is_empty() {
        // suffix range: bytes=-N  -> last N bytes
        let n: u64 = end_s.parse().map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        let n = n.min(total);
        (total - n, last)
    } else {
        let start: u64 = start_s.parse().map_err(|_| ())?;
        let end = if end_s.is_empty() {
            last
        } else {
            end_s.parse::<u64>().map_err(|_| ())?.min(last)
        };
        (start, end)
    };
    if start > last || start > end {
        return Err(());
    }
    Ok(Some((start, end)))
}

async fn get_asset_content(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
    Query(query): Query<AssetContentQuery>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let original = state
        .storage
        .get_asset(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?;

    // MT-259: tier selection. `full`/absent serves the original; a derived tier
    // serves the tier's blob (only when that tier row is `ready`).
    let serve_asset = match query
        .tier
        .as_deref()
        .filter(|t| !t.is_empty() && *t != "full")
    {
        None => original.clone(),
        Some(tier_str) => {
            let tier = crate::storage::MediaTier::from_str(tier_str)
                .map_err(|_| bad_request("invalid_tier"))?;
            let row = state
                .storage
                .get_media_tier(&workspace_id, &asset_id, tier)
                .await
                .map_err(map_storage_error)?
                .ok_or_else(|| not_found("tier_not_available"))?;
            if row.status != crate::storage::MediaTierStatus::Ready {
                return Err(not_found("tier_not_ready"));
            }
            let tier_asset_id = row
                .tier_asset_id
                .ok_or_else(|| not_found("tier_not_available"))?;
            state
                .storage
                .get_asset(&workspace_id, &tier_asset_id)
                .await
                .map_err(map_storage_error)?
        }
    };

    let handshake_root = resolve_handshake_root().map_err(internal_error)?;
    let path = loom_asset_blob_path(
        &handshake_root,
        &workspace_id,
        &serve_asset.kind,
        &serve_asset.content_hash,
    );

    let metadata = tokio::fs::metadata(&path).await.map_err(internal_error)?;
    let total = metadata.len();

    let content_type = HeaderValue::from_str(serve_asset.mime.as_str())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    // GAP-LM-009b: honor HTTP Range so long-video seeking streams a slice.
    match parse_byte_range(&headers, total) {
        Err(()) => {
            // Syntactically present but unsatisfiable -> 416 + Content-Range *.
            let mut response = Response::new(axum::body::Body::empty());
            *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
            response.headers_mut().insert(
                header::CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{total}"))
                    .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
            );
            response
                .headers_mut()
                .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            Ok(response)
        }
        Ok(Some((start, end))) => {
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            let len = end - start + 1;
            let mut file = tokio::fs::File::open(&path).await.map_err(internal_error)?;
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(internal_error)?;
            let mut buf = vec![0u8; len as usize];
            file.read_exact(&mut buf).await.map_err(internal_error)?;

            let mut response = Response::new(axum::body::Body::from(buf));
            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
            let h = response.headers_mut();
            h.insert(header::CONTENT_TYPE, content_type);
            h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            h.insert(
                header::CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes {start}-{end}/{total}"))
                    .unwrap_or_else(|_| HeaderValue::from_static("bytes 0-0/0")),
            );
            h.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&len.to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );
            Ok(response)
        }
        Ok(None) => {
            let bytes = tokio::fs::read(&path).await.map_err(internal_error)?;
            let mut response = Response::new(axum::body::Body::from(bytes));
            *response.status_mut() = StatusCode::OK;
            let h = response.headers_mut();
            h.insert(header::CONTENT_TYPE, content_type);
            // Advertise range support so clients (video) can seek.
            h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            h.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&total.to_string())
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );
            Ok(response)
        }
    }
}

async fn get_asset_thumbnail(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let Some(block) = state
        .storage
        .find_loom_block_by_asset_id(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?
    else {
        return Err(not_found("loom_block_not_found"));
    };

    let Some(thumbnail_asset_id) = block.derived.thumbnail_asset_id else {
        return Err(not_found("thumbnail_not_available"));
    };

    let thumb = state
        .storage
        .get_asset(&workspace_id, &thumbnail_asset_id)
        .await
        .map_err(map_storage_error)?;

    let handshake_root = resolve_handshake_root().map_err(internal_error)?;
    let path = loom_asset_blob_path(
        &handshake_root,
        &workspace_id,
        &thumb.kind,
        &thumb.content_hash,
    );
    let bytes = std::fs::read(&path).map_err(internal_error)?;

    let mut response = Response::new(axum::body::Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(thumb.mime.as_str())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    Ok(response)
}

// ===== MT-259 MediaCacheTiers API ====================================

#[derive(Debug, Serialize)]
struct MediaTierView {
    tier: String,
    status: String,
    tier_asset_id: Option<String>,
    content_hash: Option<String>,
    failure_reason: Option<String>,
    attempt_count: i32,
}

impl From<crate::storage::MediaAssetTier> for MediaTierView {
    fn from(t: crate::storage::MediaAssetTier) -> Self {
        MediaTierView {
            tier: t.tier.as_str().to_string(),
            status: t.status.as_str().to_string(),
            tier_asset_id: t.tier_asset_id,
            content_hash: t.content_hash,
            failure_reason: t.failure_reason,
            attempt_count: t.attempt_count,
        }
    }
}

#[derive(Debug, Serialize)]
struct ListTiersResponse {
    tiers: Vec<MediaTierView>,
}

async fn list_asset_tiers(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
) -> ApiResult<Json<ListTiersResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    // Confirm the asset exists (404 otherwise).
    state
        .storage
        .get_asset(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?;
    let tiers = state
        .storage
        .list_media_tiers(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(ListTiersResponse {
        tiers: tiers.into_iter().map(MediaTierView::from).collect(),
    }))
}

#[derive(Debug, Serialize)]
struct RetryTierResponse {
    tier: String,
    status: String,
    attempt_count: i32,
    requeued: bool,
}

async fn retry_asset_tier(
    State(state): State<AppState>,
    Path((workspace_id, asset_id, tier_str)): Path<(String, String, String)>,
) -> ApiResult<Json<RetryTierResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let tier =
        crate::storage::MediaTier::from_str(&tier_str).map_err(|_| bad_request("invalid_tier"))?;

    let block = state
        .storage
        .find_loom_block_by_asset_id(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?
        .ok_or_else(|| not_found("loom_block_not_found"))?;

    // Flip status -> pending; storage bumps attempt_count on the failed->pending
    // transition so the retry is recorded and never silent.
    let ctx = crate::storage::WriteContext::human(None);
    let updated = state
        .storage
        .set_media_tier_status(
            &ctx,
            &workspace_id,
            &asset_id,
            tier,
            crate::storage::MediaTierStatus::Pending,
            None,
        )
        .await
        .map_err(map_storage_error)?;

    // Requeue the real background generation job (same protocol as import).
    let capability_profile_id = state
        .capability_registry
        .profile_for_job_request(
            crate::storage::JobKind::LoomPreviewGenerate.as_str(),
            "hsk.loom.preview_generate@v1",
        )
        .map_err(internal_error)?;
    let job = crate::jobs::create_job(
        &state,
        crate::storage::JobKind::LoomPreviewGenerate,
        "hsk.loom.preview_generate@v1",
        capability_profile_id.id.as_str(),
        Some(json!({
            "workspace_id": workspace_id.clone(),
            "block_id": block.block_id.clone(),
            "asset_id": asset_id.clone(),
            "requested_tier": 1,
            "retry": true,
        })),
        Vec::new(),
    )
    .await
    .map_err(internal_error)?;
    let _ = crate::workflows::start_workflow_for_job(&state, job).await;

    Ok(Json(RetryTierResponse {
        tier: updated.tier.as_str().to_string(),
        status: updated.status.as_str().to_string(),
        attempt_count: updated.attempt_count,
        requeued: true,
    }))
}

// ===== MT-259 LoomCollections API (GAP-LM-244a) ======================

#[derive(Debug, Deserialize)]
struct CreateCollectionRequest {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    asset_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CollectionView {
    collection_id: String,
    title: Option<String>,
    members: Vec<String>,
}

impl From<crate::storage::LoomCollectionWithMembers> for CollectionView {
    fn from(c: crate::storage::LoomCollectionWithMembers) -> Self {
        CollectionView {
            collection_id: c.collection.collection_id,
            title: c.collection.title,
            members: c.members.into_iter().map(|m| m.asset_id).collect(),
        }
    }
}

async fn create_loom_collection(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateCollectionRequest>,
) -> ApiResult<Json<CollectionView>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = crate::storage::WriteContext::human(None);
    let collection = state
        .storage
        .create_loom_collection(&ctx, &workspace_id, req.title)
        .await
        .map_err(map_storage_error)?;
    let result = if req.asset_ids.is_empty() {
        crate::storage::LoomCollectionWithMembers {
            collection,
            members: Vec::new(),
        }
    } else {
        state
            .storage
            .set_loom_collection_order(
                &ctx,
                &workspace_id,
                &collection.collection_id,
                &req.asset_ids,
            )
            .await
            .map_err(map_storage_error)?
    };
    Ok(Json(result.into()))
}

async fn get_loom_collection(
    State(state): State<AppState>,
    Path((workspace_id, collection_id)): Path<(String, String)>,
) -> ApiResult<Json<CollectionView>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let result = state
        .storage
        .get_loom_collection(&workspace_id, &collection_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(result.into()))
}

#[derive(Debug, Deserialize)]
struct SetCollectionOrderRequest {
    asset_ids: Vec<String>,
}

async fn set_loom_collection_order(
    State(state): State<AppState>,
    Path((workspace_id, collection_id)): Path<(String, String)>,
    Json(req): Json<SetCollectionOrderRequest>,
) -> ApiResult<Json<CollectionView>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = crate::storage::WriteContext::human(None);
    let result = state
        .storage
        .set_loom_collection_order(&ctx, &workspace_id, &collection_id, &req.asset_ids)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(result.into()))
}

// =============================================================================
// MT-260: AI Loom jobs (GAP-LM-011)
// =============================================================================

use crate::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use crate::loom_ai::promotion::{
    accept_all_loom_ai_suggestions as accept_all_suggestions_flow,
    accept_loom_ai_suggestion as accept_suggestion_flow,
    reject_loom_ai_suggestion as reject_suggestion_flow, LoomAiAcceptOutcome, LoomAiRejectOutcome,
    LoomAiReviewError,
};
use crate::loom_ai::{run_loom_ai_job as run_loom_ai_job_flow, LoomAiJobError, LoomAiJobRequest};
use crate::storage::loom_ai::{
    list_loom_ai_suggestions as list_suggestion_rows, LoomAiJobKind, LoomAiSuggestionRow,
};

fn hdr_value<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// The MODEL actor that runs a job (`x-hsk-actor-*`). AI Loom jobs are proposed
/// by a model actor; absent headers fall back to an attributable local-model
/// identity so receipts are always written. Validation-failed headers => 400.
fn job_model_actor(headers: &axum::http::HeaderMap) -> ApiResult<KnowledgeActorIdV1> {
    let kind = match hdr_value(headers, "x-hsk-actor-kind") {
        Some("cloud_model") => KnowledgeActorKind::CloudModel,
        // Default to local model (the no-Docker, self-hosted lane).
        _ => KnowledgeActorKind::LocalModel,
    };
    let ident = hdr_value(headers, "x-hsk-actor-id").unwrap_or("loom-ai-job");
    KnowledgeActorIdV1::new(kind, ident).map_err(|_| bad_request("HSK-400-LOOM-AI-ACTOR"))
}

/// The REVIEWER actor for accept/reject (operator/validator only). A model
/// actor here is allowed through to the flow, which writes the durable denial
/// receipt (per-item authority is enforced in the flow, not just the route).
fn reviewer_actor(headers: &axum::http::HeaderMap) -> ApiResult<KnowledgeActorIdV1> {
    let kind = match hdr_value(headers, "x-hsk-actor-kind") {
        Some("validator") => KnowledgeActorKind::Validator,
        Some("local_model") => KnowledgeActorKind::LocalModel,
        Some("cloud_model") => KnowledgeActorKind::CloudModel,
        Some("system") => KnowledgeActorKind::System,
        // Default to operator (the human confirm path).
        _ => KnowledgeActorKind::Operator,
    };
    let ident = hdr_value(headers, "x-hsk-actor-id").unwrap_or("operator");
    KnowledgeActorIdV1::new(kind, ident).map_err(|_| bad_request("HSK-400-LOOM-AI-ACTOR"))
}

fn loom_ai_session(headers: &axum::http::HeaderMap) -> String {
    hdr_value(headers, "x-hsk-session-run-id")
        .map(|v| v.to_string())
        .unwrap_or_else(|| format!("SR-loom-ai-{}", Uuid::now_v7().simple()))
}

fn loom_ai_correlation(headers: &axum::http::HeaderMap) -> String {
    hdr_value(headers, "x-hsk-correlation-id")
        .map(|v| v.to_string())
        .unwrap_or_else(|| format!("corr-loom-ai-{}", Uuid::now_v7().simple()))
}

#[derive(Debug, Deserialize)]
struct RunLoomAiJobRequest {
    kind: LoomAiJobKind,
    /// The blocks to run the job over.
    block_ids: Vec<String>,
    /// Optional candidate tags for auto_tag.
    #[serde(default)]
    tag_candidates: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LoomAiJobResponse {
    job_id: String,
    kind: String,
    suggestions: Vec<LoomAiSuggestionRow>,
}

fn map_review_error(err: LoomAiReviewError) -> ApiError {
    match err {
        LoomAiReviewError::Storage(inner) => map_storage_error(inner),
        LoomAiReviewError::Internal(_) => internal_error(err),
    }
}

async fn run_loom_ai_job(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RunLoomAiJobRequest>,
) -> ApiResult<Json<LoomAiJobResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    if payload.block_ids.is_empty() {
        return Err(bad_request("HSK-400-LOOM-AI-NO-BLOCKS"));
    }
    let actor = job_model_actor(&headers)?;

    // Resolve every block (a missing block fails the whole job — no silent skip).
    let mut blocks = Vec::with_capacity(payload.block_ids.len());
    for block_id in &payload.block_ids {
        let block = state
            .storage
            .get_loom_block(&workspace_id, block_id)
            .await
            .map_err(map_storage_error)?;
        blocks.push(block);
    }

    let req = LoomAiJobRequest {
        workspace_id: workspace_id.clone(),
        kind: payload.kind,
        blocks,
        tag_candidates: payload.tag_candidates,
        session_id: loom_ai_session(&headers),
        correlation_id: loom_ai_correlation(&headers),
        actor,
    };
    let result = run_loom_ai_job_flow(
        state.storage.as_ref(),
        &state.postgres_pool,
        state.llm_client.as_ref(),
        req,
    )
    .await
    .map_err(|err| match err {
        // No model configured / provider declined -> typed 409, zero rows.
        LoomAiJobError::NoModel { .. } => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "HSK-409-LOOM-AI-NO-MODEL",
            }),
        ),
        LoomAiJobError::Storage(inner) => map_storage_error(inner),
        LoomAiJobError::Internal(_) => internal_error(err),
    })?;

    Ok(Json(LoomAiJobResponse {
        job_id: result.job_id,
        kind: result.kind,
        suggestions: result.suggestions,
    }))
}

/// MT-264 LoomSearchV2 request body.
#[derive(Debug, Deserialize, Default)]
struct LoomSearchV2Body {
    query: String,
    #[serde(default)]
    content_type: Option<crate::storage::LoomBlockContentType>,
    #[serde(default)]
    tag_ids: Vec<String>,
    #[serde(default)]
    graph_boost: f64,
    #[serde(default)]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

/// MT-264 LoomSearchV2 handler: embeds the query through the configured model
/// runtime (typed decline -> keyword/trigram fallback) and runs the hybrid
/// Postgres-native search. The response carries per-modality scores, content
/// facets, ts_headline highlights, and a `semantic_available` flag.
async fn loom_search_v2(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<LoomSearchV2Body>,
) -> ApiResult<Json<crate::storage::LoomSearchV2Response>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let request = crate::storage::LoomSearchV2Request {
        query: payload.query,
        content_type: payload.content_type,
        tag_ids: payload.tag_ids,
        query_embedding: None,
        graph_boost: payload.graph_boost,
        limit: payload.limit,
        offset: payload.offset,
    };
    let resp = crate::loom_search::search(
        state.storage.as_ref(),
        state.llm_client.as_ref(),
        &workspace_id,
        request,
    )
    .await
    .map_err(map_storage_error)?;
    Ok(Json(resp))
}

#[derive(Debug, Deserialize, Default)]
struct ListLoomAiSuggestionsQuery {
    #[serde(default)]
    job_id: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

async fn list_loom_ai_suggestions(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ListLoomAiSuggestionsQuery>,
) -> ApiResult<Json<Vec<LoomAiSuggestionRow>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let rows = list_suggestion_rows(
        &state.postgres_pool,
        &workspace_id,
        query.job_id.as_deref(),
        query.state.as_deref(),
    )
    .await
    .map_err(map_storage_error)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize, Default)]
struct ReviewLoomAiSuggestionRequest {
    #[serde(default)]
    reason: Option<String>,
}

async fn accept_loom_ai_suggestion(
    State(state): State<AppState>,
    Path((workspace_id, suggestion_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Option<Json<ReviewLoomAiSuggestionRequest>>,
) -> ApiResult<Json<LoomAiSuggestionRow>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let reviewer = reviewer_actor(&headers)?;
    let reason = body
        .and_then(|b| b.0.reason)
        .unwrap_or_else(|| "operator confirmed AI Loom suggestion".to_string());

    let outcome = accept_suggestion_flow(
        state.storage.as_ref(),
        &state.postgres_pool,
        &suggestion_id,
        &reviewer,
        &loom_ai_session(&headers),
        &loom_ai_correlation(&headers),
        &reason,
    )
    .await
    .map_err(map_review_error)?;

    match outcome {
        LoomAiAcceptOutcome::Promoted { suggestion, .. } => Ok(Json(*suggestion)),
        LoomAiAcceptOutcome::AlreadyPromoted(suggestion) => Ok(Json(*suggestion)),
        LoomAiAcceptOutcome::UnknownSuggestion { .. } => Err(not_found("loom_ai_suggestion_not_found")),
        LoomAiAcceptOutcome::Denied(_) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-LOOM-AI-PROMOTION-DENIED",
            }),
        )),
        LoomAiAcceptOutcome::NotPending { .. } => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "HSK-409-LOOM-AI-NOT-PENDING",
            }),
        )),
    }
}

async fn reject_loom_ai_suggestion(
    State(state): State<AppState>,
    Path((workspace_id, suggestion_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Option<Json<ReviewLoomAiSuggestionRequest>>,
) -> ApiResult<Json<LoomAiSuggestionRow>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let reviewer = reviewer_actor(&headers)?;
    let reason = body
        .and_then(|b| b.0.reason)
        .unwrap_or_else(|| "operator rejected AI Loom suggestion".to_string());

    let outcome = reject_suggestion_flow(
        state.storage.as_ref(),
        &state.postgres_pool,
        &suggestion_id,
        &reviewer,
        &loom_ai_session(&headers),
        &loom_ai_correlation(&headers),
        &reason,
    )
    .await
    .map_err(map_review_error)?;

    match outcome {
        LoomAiRejectOutcome::Rejected(suggestion) => Ok(Json(*suggestion)),
        LoomAiRejectOutcome::UnknownSuggestion { .. } => Err(not_found("loom_ai_suggestion_not_found")),
        LoomAiRejectOutcome::Denied(_) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-LOOM-AI-PROMOTION-DENIED",
            }),
        )),
        LoomAiRejectOutcome::NotPending { .. } => Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "HSK-409-LOOM-AI-NOT-PENDING",
            }),
        )),
    }
}

#[derive(Debug, Deserialize, Default)]
struct AcceptAllLoomAiRequest {
    /// Only accept suggestions of this kind (accept-all-of-kind).
    #[serde(default)]
    kind: Option<LoomAiJobKind>,
}

#[derive(Debug, Serialize)]
struct AcceptAllLoomAiResponse {
    promoted: Vec<String>,
    denied: Vec<String>,
    skipped: Vec<String>,
}

/// Accept-all-of-kind. Per-item authority: each suggestion goes through the
/// SAME accept flow (NOT a bulk SQL UPDATE), so a non-operator promotes NOTHING
/// (every item lands in `denied`), and each promotion is individually
/// kernel-event-backed.
async fn accept_all_loom_ai_suggestions(
    State(state): State<AppState>,
    Path((workspace_id, job_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Option<Json<AcceptAllLoomAiRequest>>,
) -> ApiResult<Json<AcceptAllLoomAiResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let reviewer = reviewer_actor(&headers)?;
    let kind_filter = body.and_then(|b| b.0.kind);

    let session = loom_ai_session(&headers);
    let correlation = loom_ai_correlation(&headers);

    // Delegate to the canonical accept-all sweep (lists the PENDING authority
    // set from PostgreSQL and runs the SAME per-item flow on each), so per-item
    // authority is enforced identically for the HTTP and direct callers.
    let outcome = accept_all_suggestions_flow(
        state.storage.as_ref(),
        &state.postgres_pool,
        &workspace_id,
        &job_id,
        kind_filter,
        &reviewer,
        &session,
        &correlation,
        "accept-all-of-kind",
    )
    .await
    .map_err(map_review_error)?;

    Ok(Json(AcceptAllLoomAiResponse {
        promoted: outcome.promoted,
        denied: outcome.denied,
        skipped: outcome.skipped,
    }))
}

#[derive(Debug, Deserialize, Default)]
struct LoomViewQuery {
    #[serde(default)]
    content_type: Option<LoomBlockContentType>,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    date_from: Option<DateTime<Utc>>,
    #[serde(default)]
    date_to: Option<DateTime<Utc>>,
    #[serde(default)]
    tag_ids: Option<String>,
    #[serde(default)]
    mention_ids: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

fn split_ids(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn split_source_kinds(value: Option<String>) -> ApiResult<Vec<LoomSearchSourceKind>> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|source_kind| !source_kind.is_empty())
        .map(|source_kind| {
            source_kind
                .parse::<LoomSearchSourceKind>()
                .map_err(|_| bad_request("HSK-400-LOOM-SOURCE-KIND"))
        })
        .collect()
}

#[derive(Debug, Default)]
struct LoomSearchOperatorQuery {
    q: String,
    tag_ids: Vec<String>,
    mention_ids: Vec<String>,
    source_kinds: Vec<LoomSearchSourceKind>,
    path: Option<String>,
}

fn unquote_loom_search_operand(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn split_loom_search_operator_values(value: &str) -> Vec<String> {
    unquote_loom_search_operand(value)
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn parse_loom_search_operator_query(raw: &str) -> ApiResult<LoomSearchOperatorQuery> {
    let mut parsed = LoomSearchOperatorQuery::default();
    let mut free_text = Vec::new();
    let mut current = String::new();
    let mut quoted = false;

    for ch in raw.chars() {
        if ch == '"' {
            quoted = !quoted;
            current.push(ch);
            continue;
        }
        if ch.is_whitespace() && !quoted {
            if !current.trim().is_empty() {
                parse_loom_search_operator_token(&current, &mut parsed, &mut free_text)?;
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }

    if !current.trim().is_empty() {
        parse_loom_search_operator_token(&current, &mut parsed, &mut free_text)?;
    }
    parsed.q = free_text.join(" ").trim().to_string();
    Ok(parsed)
}

fn parse_loom_search_operator_token(
    token: &str,
    parsed: &mut LoomSearchOperatorQuery,
    free_text: &mut Vec<String>,
) -> ApiResult<()> {
    let Some((operator, operand)) = token.split_once(':') else {
        free_text.push(unquote_loom_search_operand(token));
        return Ok(());
    };
    if operator
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))
    {
        free_text.push(unquote_loom_search_operand(token));
        return Ok(());
    }

    match operator.to_ascii_lowercase().as_str() {
        "tag" => {
            for value in split_loom_search_operator_values(operand) {
                let value = value.trim_start_matches('#').to_string();
                if !value.is_empty() {
                    push_unique(&mut parsed.tag_ids, value);
                }
            }
        }
        "mention" => {
            for value in split_loom_search_operator_values(operand) {
                push_unique(&mut parsed.mention_ids, value);
            }
        }
        "path" | "folder" => {
            let path = unquote_loom_search_operand(operand);
            if !path.trim().is_empty() {
                parsed.path = Some(path.trim().to_string());
            }
        }
        "kind" => {
            for value in split_loom_search_operator_values(operand) {
                let source_kind = value
                    .parse::<LoomSearchSourceKind>()
                    .map_err(|_| bad_request("HSK-400-LOOM-SOURCE-KIND"))?;
                push_unique(&mut parsed.source_kinds, source_kind);
            }
        }
        _ => free_text.push(unquote_loom_search_operand(token)),
    }
    Ok(())
}

fn merge_unique_strings(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    let mut merged = left;
    for value in right {
        push_unique(&mut merged, value);
    }
    merged
}

fn merge_unique_source_kinds(
    left: Vec<LoomSearchSourceKind>,
    right: Vec<LoomSearchSourceKind>,
) -> Vec<LoomSearchSourceKind> {
    let mut merged = left;
    for value in right {
        push_unique(&mut merged, value);
    }
    merged
}

fn clamp_loom_graph_depth(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DEFAULT_LOOM_GRAPH_DEPTH)
        .clamp(1, MAX_LOOM_GRAPH_DEPTH)
}

fn parse_loom_edge_types(value: Option<String>) -> ApiResult<Vec<LoomEdgeType>> {
    let raw = value.unwrap_or_default();
    let mut edge_types = Vec::new();
    for token in raw
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
    {
        let edge_type = token
            .parse::<LoomEdgeType>()
            .map_err(|_| bad_request("HSK-400-LOOM-EDGE-TYPE"))?;
        if !edge_types.contains(&edge_type) {
            edge_types.push(edge_type);
        }
    }
    Ok(edge_types)
}

fn count_view_filters(filters: &LoomViewFilters) -> u32 {
    let mut count = 0_u32;
    if filters.content_type.is_some() {
        count += 1;
    }
    if filters.mime.is_some() {
        count += 1;
    }
    if filters.date_from.is_some() {
        count += 1;
    }
    if filters.date_to.is_some() {
        count += 1;
    }
    if !filters.tag_ids.is_empty() {
        count += 1;
    }
    if !filters.mention_ids.is_empty() {
        count += 1;
    }
    count
}

fn view_result_count(resp: &LoomViewResponse) -> usize {
    match resp {
        LoomViewResponse::All { blocks }
        | LoomViewResponse::Unlinked { blocks }
        | LoomViewResponse::Pins { blocks }
        | LoomViewResponse::Favorites { blocks } => blocks.len(),
        LoomViewResponse::Sorted { groups } => groups.iter().map(|g| g.blocks.len()).sum(),
    }
}

fn parse_view_type(raw: &str) -> Option<LoomViewType> {
    match raw {
        "all" => Some(LoomViewType::All),
        "unlinked" => Some(LoomViewType::Unlinked),
        "sorted" => Some(LoomViewType::Sorted),
        "pins" => Some(LoomViewType::Pins),
        "favorites" => Some(LoomViewType::Favorites),
        _ => None,
    }
}

async fn query_loom_view(
    State(state): State<AppState>,
    Path((workspace_id, view_type_raw)): Path<(String, String)>,
    Query(query): Query<LoomViewQuery>,
) -> ApiResult<Json<LoomViewResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let view_type = parse_view_type(view_type_raw.as_str())
        .ok_or_else(|| bad_request("HSK-400-LOOM-VIEW-TYPE"))?;

    let filters = LoomViewFilters {
        content_type: query.content_type,
        mime: query.mime,
        date_from: query.date_from,
        date_to: query.date_to,
        tag_ids: split_ids(query.tag_ids),
        mention_ids: split_ids(query.mention_ids),
    };

    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);

    // WAIVER [CX-573E]: timing-only instrumentation; no determinism impact
    let start = Instant::now();
    let resp = state
        .storage
        .query_loom_view(
            &workspace_id,
            view_type.clone(),
            filters.clone(),
            limit,
            offset,
        )
        .await
        .map_err(map_storage_error)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomViewQueried,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_view_queried",
            "workspace_id": workspace_id,
            "view_type": view_type_raw,
            "filter_count": count_view_filters(&filters),
            "result_count": view_result_count(&resp),
            "duration_ms": duration_ms,
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(resp))
}

#[derive(Debug, Deserialize, Default)]
struct LoomGraphTraverseQueryParams {
    start_block_id: Option<String>,
    #[serde(default)]
    max_depth: Option<u32>,
    #[serde(default)]
    edge_types: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoomGraphTraversalNode {
    block: LoomBlock,
    depth: u32,
}

#[derive(Debug, Serialize)]
struct LoomMetricsRecomputeResponse {
    status: &'static str,
    scope: &'static str,
    workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_id: Option<String>,
}

async fn traverse_loom_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomGraphTraverseQueryParams>,
) -> ApiResult<Json<Vec<LoomGraphTraversalNode>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let start_block_id = query
        .start_block_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_request("HSK-400-LOOM-START-BLOCK-REQUIRED"))?;
    state
        .storage
        .get_loom_block(&workspace_id, &start_block_id)
        .await
        .map_err(map_storage_error)?;

    let max_depth = clamp_loom_graph_depth(query.max_depth);
    let edge_types = parse_loom_edge_types(query.edge_types)?;
    let traversed = state
        .storage
        .traverse_graph(&workspace_id, &start_block_id, max_depth, &edge_types)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(
        traversed
            .into_iter()
            .map(|(block, depth)| LoomGraphTraversalNode { block, depth })
            .collect(),
    ))
}

#[derive(Debug, Deserialize, Default)]
struct LoomLocalGraphQuery {
    start_block_id: Option<String>,
    #[serde(default)]
    max_depth: Option<u32>,
    #[serde(default)]
    edge_types: Option<String>,
    #[serde(default)]
    node_limit: Option<u32>,
}

/// MT-179: local graph neighborhood (undirected BFS) with filters, depth,
/// stale markers, and ProjectKnowledgeIndex citations.
async fn local_loom_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomLocalGraphQuery>,
) -> ApiResult<Json<crate::storage::LoomGraph>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let start_block_id = query
        .start_block_id
        .clone()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| bad_request("HSK-400-LOOM-START-BLOCK-REQUIRED"))?;
    let max_depth = clamp_loom_graph_depth(query.max_depth);
    let edge_types = parse_loom_edge_types(query.edge_types)?;
    let node_limit = query.node_limit.unwrap_or(200).min(5000);

    let graph = state
        .storage
        .local_graph(
            &workspace_id,
            &start_block_id,
            max_depth,
            &edge_types,
            node_limit,
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(graph))
}

#[derive(Debug, Deserialize, Default)]
struct LoomGlobalGraphQuery {
    #[serde(default)]
    edge_types: Option<String>,
    #[serde(default)]
    node_limit: Option<u32>,
    #[serde(default)]
    hub_degree_threshold: Option<u32>,
}

/// MT-180: project-level global graph with performance limits + hub suppression.
async fn global_loom_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomGlobalGraphQuery>,
) -> ApiResult<Json<crate::storage::LoomGraph>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let edge_types = parse_loom_edge_types(query.edge_types)?;
    let node_limit = query
        .node_limit
        .unwrap_or(crate::storage::LOOM_GLOBAL_GRAPH_DEFAULT_NODE_LIMIT)
        .min(crate::storage::LOOM_GLOBAL_GRAPH_MAX_NODE_LIMIT);
    let hub_degree_threshold = query
        .hub_degree_threshold
        .unwrap_or(crate::storage::LOOM_GLOBAL_GRAPH_DEFAULT_HUB_DEGREE);

    let graph = state
        .storage
        .global_graph(&workspace_id, &edge_types, node_limit, hub_degree_threshold)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(graph))
}

async fn recompute_loom_block_metrics(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomMetricsRecomputeResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .recompute_block_metrics(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(LoomMetricsRecomputeResponse {
        status: "ok",
        scope: "block",
        workspace_id,
        block_id: Some(block_id),
    }))
}

async fn recompute_all_loom_metrics(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<LoomMetricsRecomputeResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .recompute_all_metrics(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(LoomMetricsRecomputeResponse {
        status: "ok",
        scope: "workspace",
        workspace_id,
        block_id: None,
    }))
}

#[derive(Debug, Deserialize, Default)]
struct LoomSearchQueryParams {
    q: Option<String>,
    #[serde(default)]
    content_type: Option<LoomBlockContentType>,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    tag_ids: Option<String>,
    #[serde(default)]
    mention_ids: Option<String>,
    #[serde(default)]
    backlink_depth: Option<u32>,
    #[serde(default)]
    source_kinds: Option<String>,
    #[serde(default)]
    case_sensitive: Option<bool>,
    #[serde(default)]
    whole_word: Option<bool>,
    #[serde(default, rename = "regex")]
    is_regex: Option<bool>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
struct LoomVisualDebugQueryParams {
    start_block_id: Option<String>,
    q: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
struct QuickSwitcherRecentsQueryParams {
    #[serde(default)]
    limit: Option<u32>,
}

async fn search_loom_blocks(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomSearchQueryParams>,
) -> ApiResult<Json<Vec<crate::storage::LoomBlockSearchResult>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let operators = parse_loom_search_operator_query(query.q.as_deref().unwrap_or_default())?;
    let q = operators.q;
    if q.trim().is_empty() {
        return Err(bad_request("HSK-400-LOOM-QUERY-REQUIRED"));
    }

    let filters = LoomSearchFilters {
        content_type: query.content_type,
        mime: query.mime,
        tag_ids: merge_unique_strings(split_ids(query.tag_ids), operators.tag_ids),
        mention_ids: merge_unique_strings(split_ids(query.mention_ids), operators.mention_ids),
        backlink_depth: query
            .backlink_depth
            .map(|depth| depth.min(MAX_LOOM_GRAPH_DEPTH)),
        source_kinds: operators.source_kinds,
        case_sensitive: query.case_sensitive.unwrap_or(false),
        whole_word: query.whole_word.unwrap_or(false),
        is_regex: query.is_regex.unwrap_or(false),
        path: operators.path.or(query.path),
    };

    let limit = query.limit.unwrap_or(50).min(500);
    let offset = query.offset.unwrap_or(0);

    // WAIVER [CX-573E]: timing-only instrumentation; no determinism impact
    let start = Instant::now();
    let results = state
        .storage
        .search_loom_blocks(&workspace_id, &q, filters, limit, offset)
        .await
        .map_err(map_storage_error)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let tier_used = state
        .storage
        .storage_capabilities()
        .loom_search_observability_tier();

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomSearchExecuted,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_search_executed",
            "workspace_id": workspace_id,
            "query_length": q.trim().chars().count(),
            "tier_used": tier_used,
            "result_count": results.len(),
            "duration_ms": duration_ms,
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(results))
}

async fn loom_visual_debug_snapshot(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomVisualDebugQueryParams>,
) -> ApiResult<Json<LoomVisualDebugSnapshot>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let start_block_id = query
        .start_block_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_request("HSK-400-LOOM-START-BLOCK-REQUIRED"))?;
    let q = query
        .q
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| bad_request("HSK-400-LOOM-QUERY-REQUIRED"))?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    // WAIVER [CX-573E]: timing-only instrumentation; no determinism impact
    let start = Instant::now();
    let snapshot = state
        .storage
        .loom_visual_debug_snapshot(&workspace_id, &start_block_id, &q, limit)
        .await
        .map_err(map_storage_error)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomProjectionRebuilt,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_visual_debug_snapshot",
            "workspace_id": workspace_id,
            "start_block_id": start_block_id,
            "query_length": q.trim().chars().count(),
            "schema_id": snapshot.schema_id,
            "node_count": snapshot.graph.nodes.len(),
            "edge_count": snapshot.graph.edges.len(),
            "backlink_count": snapshot.backlinks.incoming.len(),
            "folder_count": snapshot.folders.len(),
            "search_result_count": snapshot.search.result_count,
            "duration_ms": duration_ms,
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(snapshot))
}

async fn search_loom_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomSearchQueryParams>,
) -> ApiResult<Json<Vec<LoomGraphSearchResult>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let operators = parse_loom_search_operator_query(query.q.as_deref().unwrap_or_default())?;
    let q = operators.q;
    if q.trim().is_empty() {
        return Err(bad_request("HSK-400-LOOM-QUERY-REQUIRED"));
    }

    let filters = LoomSearchFilters {
        content_type: query.content_type,
        mime: query.mime,
        tag_ids: merge_unique_strings(split_ids(query.tag_ids), operators.tag_ids),
        mention_ids: merge_unique_strings(split_ids(query.mention_ids), operators.mention_ids),
        backlink_depth: query
            .backlink_depth
            .map(|depth| depth.min(MAX_LOOM_GRAPH_DEPTH)),
        source_kinds: merge_unique_source_kinds(
            split_source_kinds(query.source_kinds)?,
            operators.source_kinds,
        ),
        case_sensitive: query.case_sensitive.unwrap_or(false),
        whole_word: query.whole_word.unwrap_or(false),
        is_regex: query.is_regex.unwrap_or(false),
        path: operators.path.or(query.path),
    };

    let limit = query.limit.unwrap_or(50).min(500);
    let offset = query.offset.unwrap_or(0);

    // WAIVER [CX-573E]: timing-only instrumentation; no determinism impact
    let start = Instant::now();
    let results = state
        .storage
        .search_loom_graph(&workspace_id, &q, filters, limit, offset)
        .await
        .map_err(map_storage_error)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let tier_used = state
        .storage
        .storage_capabilities()
        .loom_search_observability_tier();

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomSearchExecuted,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_search_executed",
            "workspace_id": workspace_id,
            "query_length": q.trim().chars().count(),
            "tier_used": tier_used,
            "result_count": results.len(),
            "duration_ms": duration_ms,
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(results))
}

async fn list_quick_switcher_recents(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<QuickSwitcherRecentsQueryParams>,
) -> ApiResult<Json<Vec<QuickSwitcherRecent>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let recents = state
        .storage
        .list_quick_switcher_recents(&workspace_id, limit)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(recents))
}

async fn record_quick_switcher_recent(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<QuickSwitcherRecentInput>,
) -> ApiResult<Json<QuickSwitcherRecent>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let recent = state
        .storage
        .record_quick_switcher_recent(&workspace_id, payload)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(recent))
}

// -- MT-261 CanvasBoard handlers ------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateCanvasBoardRequest {
    #[serde(default)]
    title: Option<String>,
    /// Optional initial viewport. Defaults to centered, zoom 1.
    #[serde(default)]
    board_state: Option<serde_json::Value>,
}

fn default_board_state() -> serde_json::Value {
    json!({
        "schema_id": crate::storage::LOOM_CANVAS_BOARD_SCHEMA_ID,
        "pan_x": 0.0,
        "pan_y": 0.0,
        "zoom": 1.0,
    })
}

/// Create a canvas: a typed LoomBlock(content_type=canvas), bridged to the
/// ProjectKnowledgeIndex (so it is authority-resolved like any block), plus its
/// board-state row.
async fn create_canvas_board(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateCanvasBoardRequest>,
) -> ApiResult<Json<LoomCanvasBoard>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let block = state
        .storage
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: payload.title.clone(),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    state
        .storage
        .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?;

    let board_state = payload.board_state.unwrap_or_else(default_board_state);
    let board = state
        .storage
        .create_canvas_board(&ctx, &workspace_id, &block.block_id, board_state)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_canvas_board_created",
            "workspace_id": workspace_id,
            "block_id": board.block_id,
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(board))
}

async fn get_canvas_board(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomCanvasBoardView>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let view = state
        .storage
        .get_canvas_board(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(view))
}

#[derive(Debug, Deserialize)]
struct UpdateBoardViewportRequest {
    board_state: serde_json::Value,
}

async fn update_canvas_board_state(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<UpdateBoardViewportRequest>,
) -> ApiResult<Json<LoomCanvasBoard>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let board = state
        .storage
        .update_canvas_board_state(
            &WriteContext::human(None),
            &workspace_id,
            &block_id,
            payload.board_state,
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(board))
}

#[derive(Debug, Deserialize)]
struct PlaceBlockRequest {
    placed_block_id: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    z_index: Option<i32>,
    #[serde(default)]
    group_id: Option<String>,
}

async fn place_block_on_canvas(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<PlaceBlockRequest>,
) -> ApiResult<Json<LoomCanvasPlacement>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let placement = state
        .storage
        .place_block_on_canvas(
            &WriteContext::human(None),
            NewLoomCanvasPlacement {
                canvas_block_id: block_id,
                workspace_id: workspace_id.clone(),
                placed_block_id: payload.placed_block_id,
                x: payload.x,
                y: payload.y,
                w: payload.w,
                h: payload.h,
                z_index: payload.z_index.unwrap_or(0),
                group_id: payload.group_id,
            },
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(placement))
}

#[derive(Debug, Deserialize)]
struct CreateCanvasCardRequest {
    title: String,
    /// Free-text card body (markdown). Becomes a real note LoomBlock backed by a
    /// RichDocument — never a board-local content copy.
    #[serde(default)]
    body: Option<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    z_index: Option<i32>,
}

#[derive(Debug, Serialize)]
struct CreateCanvasCardResponse {
    block: LoomBlock,
    rich_document_id: String,
    placement: LoomCanvasPlacement,
}

/// Create a free-text card: a REAL note LoomBlock (content_type=note) backed by
/// a RichDocument and bridged to knowledge, then placed on the canvas as a
/// reference. The card is authority, never a board-only copy.
async fn create_canvas_card(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<CreateCanvasCardRequest>,
) -> ApiResult<Json<CreateCanvasCardResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let imported = state
        .storage
        .import_markdown_to_loom(
            &ctx,
            &workspace_id,
            &payload.title,
            payload.body.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_storage_error)?;

    let placement = state
        .storage
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: block_id,
                workspace_id: workspace_id.clone(),
                placed_block_id: imported.block.block_id.clone(),
                x: payload.x,
                y: payload.y,
                w: payload.w,
                h: payload.h,
                z_index: payload.z_index.unwrap_or(0),
                group_id: None,
            },
        )
        .await
        .map_err(map_storage_error)?;

    Ok(Json(CreateCanvasCardResponse {
        block: imported.block,
        rich_document_id: imported.rich_document_id,
        placement,
    }))
}

#[derive(Debug, Deserialize)]
struct UpdatePlacementRequest {
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
    #[serde(default)]
    w: Option<f64>,
    #[serde(default)]
    h: Option<f64>,
    #[serde(default)]
    z_index: Option<i32>,
    /// `Some("g1")` sets a group; `Some(null)` (deserialized as present-but-null
    /// via `group_id_set`) clears it. To keep the wire simple we treat any
    /// provided `group_id` as set, and `clear_group=true` as clear.
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    clear_group: bool,
}

async fn update_canvas_placement(
    State(state): State<AppState>,
    Path((workspace_id, placement_id)): Path<(String, String)>,
    Json(payload): Json<UpdatePlacementRequest>,
) -> ApiResult<Json<LoomCanvasPlacement>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let group_id = if payload.clear_group {
        Some(None)
    } else {
        payload.group_id.map(Some)
    };
    let placement = state
        .storage
        .update_canvas_placement(
            &WriteContext::human(None),
            &workspace_id,
            &placement_id,
            LoomCanvasPlacementUpdate {
                x: payload.x,
                y: payload.y,
                w: payload.w,
                h: payload.h,
                z_index: payload.z_index,
                group_id,
            },
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(placement))
}

async fn remove_canvas_placement(
    State(state): State<AppState>,
    Path((workspace_id, placement_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .remove_canvas_placement(&WriteContext::human(None), &workspace_id, &placement_id)
        .await
        .map_err(map_storage_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct AddVisualEdgeRequest {
    from_placement_id: String,
    to_placement_id: String,
    #[serde(default)]
    label: Option<String>,
}

async fn add_canvas_visual_edge(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<AddVisualEdgeRequest>,
) -> ApiResult<Json<LoomCanvasVisualEdge>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let edge = state
        .storage
        .add_canvas_visual_edge(
            &WriteContext::human(None),
            &workspace_id,
            &block_id,
            &payload.from_placement_id,
            &payload.to_placement_id,
            payload.label,
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(edge))
}

async fn remove_canvas_visual_edge(
    State(state): State<AppState>,
    Path((workspace_id, visual_edge_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    state
        .storage
        .remove_canvas_visual_edge(&WriteContext::human(None), &workspace_id, &visual_edge_id)
        .await
        .map_err(map_storage_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// MT-262 BlockCollectionViews handlers
// =============================================================================

#[derive(Debug, Deserialize)]
struct CreateBlockViewRequest {
    #[serde(default)]
    block_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    definition: BlockViewDefinition,
}

/// Create a saved view: a typed `LoomBlock(content_type='view_def')` born
/// through `create_loom_block` + the ProjectKnowledgeIndex bridge (so it gets a
/// real authority receipt), then stamped with its definition. NO parallel store.
async fn create_block_view(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateBlockViewRequest>,
) -> ApiResult<Json<BlockViewRecord>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    // The view is born as a normal LoomBlock first (note type), then flipped to
    // view_def with its definition — so it picks up the same bridge + receipt
    // path every block uses. We cannot create it directly as view_def because
    // the CHECK requires the definition column to be set in the same write, and
    // create_loom_block does not write view_definition_json.
    let block = state
        .storage
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: payload.block_id,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: payload.title.clone(),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    state
        .storage
        .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?;

    let record = state
        .storage
        .create_block_view(
            &ctx,
            &workspace_id,
            &block.block_id,
            payload.title,
            payload.definition,
        )
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_view_created",
            "workspace_id": workspace_id,
            "block_id": record.block.block_id,
            "view_kind": record.definition.kind.as_str(),
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(record))
}

async fn get_block_view(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<BlockViewRecord>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .get_block_view(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
struct UpdateBlockViewRequest {
    definition: BlockViewDefinition,
}

/// Persist a new definition for a saved view (e.g. a table header click that
/// re-sorts the view stores the new sort in PostgreSQL, not localStorage).
async fn update_block_view(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<UpdateBlockViewRequest>,
) -> ApiResult<Json<BlockViewRecord>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .update_block_view_definition(
            &WriteContext::human(None),
            &workspace_id,
            &block_id,
            payload.definition,
        )
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": record.block.block_id,
            "fields_changed": ["view_definition"],
            "updated_by": "user",
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(record))
}

#[derive(Debug, Deserialize, Default)]
struct BlockViewResultsRequest {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

/// Execute a saved view's query against the REAL Loom query backend. Filtering,
/// the typed ORDER BY, and Kanban lane partitioning all run server-side.
async fn query_block_view_results(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<BlockViewResultsRequest>,
) -> ApiResult<Json<BlockViewResults>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .get_block_view(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    let limit = payload.limit.unwrap_or(100).min(500);
    let offset = payload.offset.unwrap_or(0);

    let results = state
        .storage
        .query_block_view_results(&workspace_id, &record.definition, limit, offset)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomViewQueried,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_view_queried",
            "workspace_id": workspace_id,
            "block_id": block_id,
            "view_kind": results.kind.as_str(),
            "result_count": results.total_returned,
            "lane_count": results.groups.len(),
        }),
    )
    .with_wsids(vec![workspace_id.clone()]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(results))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::{duckdb::DuckDbFlightRecorder, EventFilter};
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{tests::optional_postgres_backend_with_pool_from_env, Database, NewWorkspace};
    use once_cell::sync::Lazy;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[derive(Debug)]
    struct LoomApiTestCallError {
        status: StatusCode,
        code: String,
    }

    impl std::fmt::Display for LoomApiTestCallError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "loom api call error ({}) {}", self.status, self.code)
        }
    }

    impl std::error::Error for LoomApiTestCallError {}

    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(val) => std::env::set_var(self.key, val),
                None => std::env::remove_var(self.key),
            }
        }
    }

    async fn setup_state() -> Result<Option<AppState>, Box<dyn std::error::Error>> {
        let Some(backend) = optional_postgres_backend_with_pool_from_env().await? else {
            return Ok(None);
        };

        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        Ok(Some(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        }))
    }

    async fn create_workspace(state: &AppState) -> Result<String, Box<dyn std::error::Error>> {
        let ws = state
            .storage
            .create_workspace(
                &WriteContext::human(None),
                NewWorkspace {
                    name: "Test".to_string(),
                },
            )
            .await?;
        Ok(ws.id)
    }

    /// WP-KERNEL-009 MT-264: an AppState whose model runtime DOES expose a real
    /// (deterministic) 768-d embedding endpoint, so the API authority write path
    /// produces and persists block embeddings exactly like a configured Ollama
    /// embedding model. Used to prove blocker #2 (embedding refreshed on
    /// create/update through the real handler, not only in manual-reindex tests).
    async fn setup_state_with_embedding() -> Result<Option<AppState>, Box<dyn std::error::Error>> {
        let Some(backend) = optional_postgres_backend_with_pool_from_env().await? else {
            return Ok(None);
        };
        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
        Ok(Some(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(
                InMemoryLlmClient::new("ok".into())
                    .with_embedding_dim(crate::loom_search::LOOM_SEARCH_EMBEDDING_DIM),
            ),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        }))
    }

    /// The number of `loom_block_search_index` rows for a block that carry a
    /// non-NULL embedding (semantic projection populated).
    async fn embedded_index_rows(
        state: &AppState,
        block_id: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM loom_block_search_index \
             WHERE block_id = $1 AND embedding IS NOT NULL",
        )
        .bind(block_id)
        .fetch_one(&state.postgres_pool)
        .await?;
        Ok(n)
    }

    /// MT-264 blocker #2: a block created through the REAL `create_loom_block`
    /// API handler (with a configured embedding model) gets its embedding
    /// populated on the authority write path — not only when a test manually
    /// calls `reindex_block`. Editing the title through `patch_loom_block`
    /// re-embeds the new text. Both are proven against real PostgreSQL.
    #[tokio::test]
    async fn mt264_api_create_and_update_refresh_embedding() -> Result<(), Box<dyn std::error::Error>>
    {
        let Some(state) = setup_state_with_embedding().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let created = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                title: Some("Embedding write path note".to_string()),
                document_id: None,
                asset_id: None,
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        let block_id = created.0.block_id.clone();

        // The embedding was produced on the create authority write path.
        assert_eq!(
            embedded_index_rows(&state, &block_id).await?,
            1,
            "create_loom_block must populate the semantic embedding projection"
        );

        // The semantic modality is reachable through the real search handler.
        let resp = loom_search_v2(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(LoomSearchV2Body {
                query: "Embedding write path note".to_string(),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert!(
            resp.0.semantic_available,
            "embedding model configured -> semantic available through API"
        );

        // Edit the title through the patch handler -> embedding re-refreshed.
        let _ = patch_loom_block(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
            Json(LoomBlockPatchRequest {
                update: LoomBlockUpdate {
                    title: Some("Edited embedding write path note".to_string()),
                    ..Default::default()
                },
                add_tags: Vec::new(),
                remove_tags: Vec::new(),
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(
            embedded_index_rows(&state, &block_id).await?,
            1,
            "patch_loom_block must keep the embedding projection populated"
        );
        Ok(())
    }

    /// MT-264 blocker #5: a daily-journal block created through
    /// `open_daily_journal` (which calls `get_or_create_daily_journal_block`)
    /// gets a `loom_block_search_index` row on creation and is immediately
    /// findable by LoomSearchV2 — no stale/missing-projection drift. Proven
    /// against real PostgreSQL.
    #[tokio::test]
    async fn mt264_journal_block_indexed_on_create() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let journal = open_daily_journal(
            State(state.clone()),
            Path((workspace_id.clone(), "2026-06-18".to_string())),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        let block_id = journal.0.block_id.clone();

        // The journal block has an index row immediately on creation.
        let index_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM loom_block_search_index WHERE block_id = $1",
        )
        .bind(&block_id)
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(
            index_rows, 1,
            "journal block must get a search-index row on creation (no drift)"
        );

        // And it is findable by its title text via LoomSearchV2 (FTS over the
        // "Daily Note 2026-06-18" title). No embedding model is configured here,
        // so this proves the keyword projection alone makes it searchable.
        let resp = loom_search_v2(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(LoomSearchV2Body {
                query: "Daily Note 2026-06-18".to_string(),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert!(
            resp.0.hits.iter().any(|h| h.block.block_id == block_id),
            "journal block must be findable by LoomSearchV2 immediately after creation"
        );
        Ok(())
    }

    async fn set_loom_metrics_for_block(
        state: &AppState,
        workspace_id: &str,
        block_id: &str,
        mention_count: i64,
        tag_count: i64,
        backlink_count: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        state
            .storage
            .test_overwrite_loom_block_metrics(
                workspace_id,
                block_id,
                mention_count,
                tag_count,
                backlink_count,
            )
            .await?;
        Ok(())
    }

    #[test]
    fn search_operator_query_parser_extracts_filters() -> Result<(), Box<dyn std::error::Error>> {
        let parsed = parse_loom_search_operator_query(
            r#"Alpha roadmap tag:#tag-1,tag-2 mention:MT-258 kind:document path:"src/app notes""#,
        )
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        assert_eq!(parsed.q, "Alpha roadmap");
        assert_eq!(parsed.tag_ids, vec!["tag-1", "tag-2"]);
        assert_eq!(parsed.mention_ids, vec!["MT-258"]);
        assert_eq!(parsed.source_kinds, vec![LoomSearchSourceKind::Document]);
        assert_eq!(parsed.path.as_deref(), Some("src/app notes"));
        Ok(())
    }

    #[test]
    fn search_operator_query_parser_rejects_invalid_kind() {
        let error = parse_loom_search_operator_query("alpha kind:not-a-kind")
            .expect_err("invalid kind operator must fail closed");
        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert_eq!(error.1 .0.error, "HSK-400-LOOM-SOURCE-KIND");
    }

    #[tokio::test]
    async fn graph_search_inline_operators_filter_real_rows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;
        let ctx = WriteContext::human(None);

        let tag_block = state
            .storage
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::TagHub,
                    document_id: None,
                    asset_id: None,
                    title: Some("Operator backend tag".to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived::default(),
                },
            )
            .await?;
        let mention_target = state
            .storage
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some("Operator backend mention target".to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived::default(),
                },
            )
            .await?;
        let matching = state
            .storage
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some("OperatorPathMatch source".to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived {
                        full_text_index: Some("OperatorBackendAlpha body".to_string()),
                        ..Default::default()
                    },
                },
            )
            .await?;
        let path_miss = state
            .storage
            .create_loom_block(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some("OperatorPathMiss source".to_string()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived {
                        full_text_index: Some("OperatorBackendAlpha body".to_string()),
                        ..Default::default()
                    },
                },
            )
            .await?;

        for source_block_id in [&matching.block_id, &path_miss.block_id] {
            state
                .storage
                .create_loom_edge(
                    &ctx,
                    NewLoomEdge {
                        edge_id: None,
                        workspace_id: workspace_id.clone(),
                        source_block_id: source_block_id.clone(),
                        target_block_id: tag_block.block_id.clone(),
                        edge_type: LoomEdgeType::Tag,
                        created_by: LoomEdgeCreatedBy::User,
                        crdt_site_id: None,
                        source_anchor: None,
                    },
                )
                .await?;
            state
                .storage
                .create_loom_edge(
                    &ctx,
                    NewLoomEdge {
                        edge_id: None,
                        workspace_id: workspace_id.clone(),
                        source_block_id: source_block_id.clone(),
                        target_block_id: mention_target.block_id.clone(),
                        edge_type: LoomEdgeType::Mention,
                        created_by: LoomEdgeCreatedBy::User,
                        crdt_site_id: None,
                        source_anchor: None,
                    },
                )
                .await?;
        }

        let hits = search_loom_graph(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(LoomSearchQueryParams {
                q: Some(format!(
                    "OperatorBackendAlpha tag:{} mention:{} kind:loom_block path:OperatorPathMatch",
                    tag_block.block_id, mention_target.block_id
                )),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let hit_keys: Vec<_> = hits
            .0
            .iter()
            .map(|hit| (hit.source_kind.as_str(), hit.ref_id.as_str()))
            .collect();
        assert_eq!(
            hit_keys,
            vec![("loom_block", matching.block_id.as_str())],
            "inline tag/mention/kind/path operators must filter against PostgreSQL rows"
        );
        Ok(())
    }

    #[tokio::test]
    async fn import_dedup_emits_fr_evt_loom_006() -> Result<(), Box<dyn std::error::Error>> {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = TempDir::new()?;
        let _env = EnvVarGuard::set(
            "HANDSHAKE_WORKSPACE_ROOT",
            temp.path().to_string_lossy().as_ref(),
        );

        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let bytes = b"hello loom".to_vec();
        let req = LoomImportRequest {
            bytes_b64: STANDARD.encode(&bytes),
            original_filename: Some("hello.txt".to_string()),
            mime: Some("application/octet-stream".to_string()),
        };

        let _ = import_loom_asset(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(req.clone()),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let second = import_loom_asset(State(state.clone()), Path(workspace_id.clone()), Json(req))
            .await
            .map_err(|(status, Json(body))| LoomApiTestCallError {
                status,
                code: body.error.to_string(),
            })?;
        assert!(second.0.dedup_hit);
        assert!(second.0.existing_block_id.is_some());

        let events = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?;
        let events: Vec<_> = events
            .into_iter()
            .filter(|e| e.event_type.to_string() == "loom_dedup_hit")
            .collect();
        assert!(!events.is_empty(), "expected loom_dedup_hit event");
        Ok(())
    }

    #[tokio::test]
    async fn view_and_search_emit_events() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let _ = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Alpha".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let _ = query_loom_view(
            State(state.clone()),
            Path((workspace_id.clone(), "all".to_string())),
            Query(LoomViewQuery::default()),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let search_results = search_loom_blocks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(LoomSearchQueryParams {
                q: Some("Alpha".to_string()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(search_results.0.len(), 1);
        assert_eq!(
            search_results.0[0].block.title.as_deref(),
            Some("Alpha"),
            "legacy /loom/search remains block-only"
        );

        let all_events = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?;
        let view_events: Vec<_> = all_events
            .iter()
            .filter(|e| e.event_type.to_string() == "loom_view_queried")
            .collect();
        assert!(!view_events.is_empty(), "expected loom_view_queried event");

        let search_events: Vec<_> = all_events
            .iter()
            .filter(|e| e.event_type.to_string() == "loom_search_executed")
            .collect();
        assert!(
            !search_events.is_empty(),
            "expected loom_search_executed event"
        );

        Ok(())
    }

    #[tokio::test]
    async fn mt258_bookmark_routes_persist_add_remove_and_emit_receipts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            eprintln!("SKIP MT-258 bookmark route proof: POSTGRES_TEST_URL unavailable");
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let created = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("MT-258 bookmark proof".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        let block_id = created.block_id.clone();

        let bridge = get_loom_block_knowledge_bridge(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        assert_eq!(bridge.block_id, block_id);
        assert_eq!(bridge.workspace_id, workspace_id);
        assert!(
            !bridge.entity_id.trim().is_empty() && !bridge.index_event_id.trim().is_empty(),
            "bookmarkable LoomBlock must retain its ProjectKnowledgeIndex/EventLedger bridge"
        );

        let pinned = patch_loom_block(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
            Json(LoomBlockPatchRequest {
                update: LoomBlockUpdate {
                    pinned: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        assert!(pinned.pinned, "route pin returns pinned block");

        let ordered = set_loom_block_pin_order(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
            Json(SetPinOrderRequest { pin_order: Some(0) }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        assert_eq!(ordered.pin_order, Some(0));

        let pins = query_loom_view(
            State(state.clone()),
            Path((workspace_id.clone(), "pins".to_string())),
            Query(LoomViewQuery {
                limit: Some(100),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        let LoomViewResponse::Pins { blocks } = pins else {
            panic!("expected pins response");
        };
        let pinned_block = blocks
            .iter()
            .find(|block| block.block_id == block_id)
            .expect("pinned block appears in pins view");
        assert!(pinned_block.pinned);
        assert_eq!(pinned_block.pin_order, Some(0));

        let cleared = set_loom_block_pin_order(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
            Json(SetPinOrderRequest { pin_order: None }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        assert_eq!(cleared.pin_order, None);

        let unpinned = patch_loom_block(
            State(state.clone()),
            Path((workspace_id.clone(), block_id.clone())),
            Json(LoomBlockPatchRequest {
                update: LoomBlockUpdate {
                    pinned: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        assert!(!unpinned.pinned, "route unpin returns unpinned block");
        assert_eq!(
            unpinned.pin_order, None,
            "remove flow must clear pin_order before unpinning"
        );

        let stored = state.storage.get_loom_block(&workspace_id, &block_id).await?;
        assert!(!stored.pinned, "Postgres read after remove is unpinned");
        assert_eq!(stored.pin_order, None, "Postgres read after remove is unordered");

        let pins_after_remove = query_loom_view(
            State(state.clone()),
            Path((workspace_id.clone(), "pins".to_string())),
            Query(LoomViewQuery {
                limit: Some(100),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;
        let LoomViewResponse::Pins { blocks } = pins_after_remove else {
            panic!("expected pins response");
        };
        assert!(
            blocks.iter().all(|block| block.block_id != block_id),
            "removed bookmark must disappear from the real pins view"
        );

        let events = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?;
        let updated_events: Vec<_> = events
            .iter()
            .filter(|event| {
                event.event_type == FlightRecorderEventType::LoomBlockUpdated
                    && event.wsids == vec![workspace_id.clone()]
                    && event
                        .payload
                        .get("block_id")
                        .and_then(|value| value.as_str())
                        == Some(block_id.as_str())
            })
            .collect();
        let changed_field_count = |field: &str| -> usize {
            updated_events
                .iter()
                .filter(|event| {
                    event
                        .payload
                        .get("fields_changed")
                        .and_then(|value| value.as_array())
                        .map(|fields| {
                            fields
                                .iter()
                                .any(|changed| changed.as_str() == Some(field))
                        })
                        .unwrap_or(false)
                })
                .count()
        };
        assert!(
            changed_field_count("pinned") >= 2,
            "pin and unpin routes must emit pinned update receipts"
        );
        assert!(
            changed_field_count("pin_order") >= 2,
            "order and clear routes must emit pin_order update receipts"
        );

        Ok(())
    }

    /// MT-258 properties-panel TAG editing, proven end-to-end through the real
    /// `patch_loom_block` route against PostgreSQL: add_tags creates `tag`
    /// loom_edges to a TagHub target, recompute makes `derived.tag_count`
    /// authoritative, remove_tags deletes the edge, add is idempotent, and a
    /// non-TagHub target is rejected with HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB.
    /// A LoomBlockUpdated receipt records the tag mutation (fields_changed=tags,
    /// tags_added/tags_removed payloads).
    #[tokio::test]
    async fn mt258_properties_tag_edit_persists_edges_and_metrics(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            eprintln!("SKIP MT-258 tag-edit route proof: POSTGRES_TEST_URL unavailable");
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let make_block = |content_type: LoomBlockContentType, title: &str| {
            let state = state.clone();
            let workspace_id = workspace_id.clone();
            let title = title.to_string();
            async move {
                create_loom_block(
                    State(state),
                    Path(workspace_id),
                    Json(CreateLoomBlockRequest {
                        block_id: None,
                        content_type,
                        document_id: None,
                        asset_id: None,
                        title: Some(title),
                        pinned: None,
                        journal_date: None,
                    }),
                )
                .await
                .map_err(|(status, Json(body))| LoomApiTestCallError {
                    status,
                    code: body.error.to_string(),
                })
                .map(|response| response.0)
            }
        };

        let note = make_block(LoomBlockContentType::Note, "MT-258 tag-edit subject").await?;
        let tag_a = make_block(LoomBlockContentType::TagHub, "#alpha").await?;
        let tag_b = make_block(LoomBlockContentType::TagHub, "#beta").await?;
        let block_id = note.block_id.clone();
        assert_eq!(
            note.derived.tag_count, 0,
            "fresh note starts with zero tags"
        );

        let patch = |add: Vec<String>, remove: Vec<String>| {
            let state = state.clone();
            let workspace_id = workspace_id.clone();
            let block_id = block_id.clone();
            async move {
                patch_loom_block(
                    State(state),
                    Path((workspace_id, block_id)),
                    Json(LoomBlockPatchRequest {
                        update: LoomBlockUpdate::default(),
                        add_tags: add,
                        remove_tags: remove,
                    }),
                )
                .await
                .map_err(|(status, Json(body))| LoomApiTestCallError {
                    status,
                    code: body.error.to_string(),
                })
            }
        };

        // Add two real tag edges; recompute makes tag_count authoritative.
        let added = patch(
            vec![tag_a.block_id.clone(), tag_b.block_id.clone()],
            Vec::new(),
        )
        .await?
        .0;
        assert_eq!(
            added.derived.tag_count, 2,
            "route add_tags must create real tag edges and recompute tag_count"
        );

        // The edges are real `tag` loom_edges in Postgres, not just a counter.
        let edges_after_add = state
            .storage
            .list_loom_edges_for_block(&workspace_id, &block_id)
            .await?;
        let tag_targets: std::collections::HashSet<String> = edges_after_add
            .iter()
            .filter(|edge| {
                edge.edge_type == LoomEdgeType::Tag && edge.source_block_id == block_id
            })
            .map(|edge| edge.target_block_id.clone())
            .collect();
        assert!(
            tag_targets.contains(&tag_a.block_id) && tag_targets.contains(&tag_b.block_id),
            "both TagHub targets must have real tag edges from the block"
        );

        // Idempotent add: re-adding tag_a must not create a duplicate edge.
        let re_added = patch(vec![tag_a.block_id.clone()], Vec::new()).await?.0;
        assert_eq!(
            re_added.derived.tag_count, 2,
            "re-adding an existing tag must be idempotent (no duplicate edge)"
        );

        // Remove one tag edge; tag_count drops to 1 and the edge is gone.
        let removed = patch(Vec::new(), vec![tag_a.block_id.clone()]).await?.0;
        assert_eq!(
            removed.derived.tag_count, 1,
            "route remove_tags must delete the tag edge and recompute tag_count"
        );
        let edges_after_remove = state
            .storage
            .list_loom_edges_for_block(&workspace_id, &block_id)
            .await?;
        assert!(
            edges_after_remove.iter().all(|edge| {
                !(edge.edge_type == LoomEdgeType::Tag
                    && edge.source_block_id == block_id
                    && edge.target_block_id == tag_a.block_id)
            }),
            "removed tag edge must be gone from Postgres"
        );
        assert!(
            edges_after_remove.iter().any(|edge| {
                edge.edge_type == LoomEdgeType::Tag
                    && edge.source_block_id == block_id
                    && edge.target_block_id == tag_b.block_id
            }),
            "the untouched tag edge must survive the remove"
        );

        // Postgres read after the route confirms durability (not in-memory only).
        let stored = state.storage.get_loom_block(&workspace_id, &block_id).await?;
        assert_eq!(
            stored.derived.tag_count, 1,
            "Postgres re-read confirms the tag mutation persisted"
        );

        // TagHub guard: a non-TagHub target is rejected, no edge is created.
        let non_tag = make_block(LoomBlockContentType::Note, "not a tag").await?;
        let guard = patch(vec![non_tag.block_id.clone()], Vec::new())
            .await
            .expect_err("non-TagHub target must be rejected");
        assert_eq!(guard.status, StatusCode::BAD_REQUEST);
        assert_eq!(guard.code, "HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB");
        let after_guard = state.storage.get_loom_block(&workspace_id, &block_id).await?;
        assert_eq!(
            after_guard.derived.tag_count, 1,
            "rejected tag target must not change the tag set"
        );

        // The tag mutation emitted a LoomBlockUpdated receipt with tags payloads.
        let events = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?;
        let tag_receipts: Vec<_> = events
            .iter()
            .filter(|event| {
                event.event_type == FlightRecorderEventType::LoomBlockUpdated
                    && event
                        .payload
                        .get("block_id")
                        .and_then(|value| value.as_str())
                        == Some(block_id.as_str())
            })
            .filter(|event| {
                event
                    .payload
                    .get("fields_changed")
                    .and_then(|value| value.as_array())
                    .map(|fields| {
                        fields
                            .iter()
                            .any(|changed| changed.as_str() == Some("tags"))
                    })
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            !tag_receipts.is_empty(),
            "tag mutation must emit a LoomBlockUpdated receipt with fields_changed=tags"
        );
        let added_len = |event: &&FlightRecorderEvent, key: &str| {
            event
                .payload
                .get(key)
                .and_then(|value| value.as_array())
                .map(|items| items.len())
                .unwrap_or(0)
        };
        assert!(
            tag_receipts
                .iter()
                .any(|event| added_len(event, "tags_added") == 2),
            "the two-tag add must emit a receipt carrying both added TagHub ids"
        );
        assert!(
            tag_receipts
                .iter()
                .any(|event| added_len(event, "tags_removed") == 1),
            "the tag remove must emit a receipt carrying the removed TagHub id"
        );

        Ok(())
    }

    #[tokio::test]
    async fn loom_search_backend_tier() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let _ = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Alpha".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let _ = search_loom_blocks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(LoomSearchQueryParams {
                q: Some("Alpha".to_string()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let search_event = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?
            .into_iter()
            .rev()
            .find(|event| event.event_type == FlightRecorderEventType::LoomSearchExecuted)
            .ok_or_else(|| "expected loom_search_executed event".to_string())?;

        let tier_used = search_event
            .payload
            .get("tier_used")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "expected tier_used payload".to_string())?;

        assert_eq!(
            tier_used,
            u64::from(
                state
                    .storage
                    .storage_capabilities()
                    .loom_search_observability_tier()
            ),
            "loom search proof must assert the emitted tier_used payload contract"
        );
        assert_eq!(
            search_event
                .payload
                .get("workspace_id")
                .and_then(|value| value.as_str()),
            Some(workspace_id.as_str())
        );

        Ok(())
    }

    #[tokio::test]
    async fn graph_traversal_and_metrics_routes_work() -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = setup_state().await? else {
            return Ok(());
        };
        let workspace_id = create_workspace(&state).await?;

        let start_block = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Graph Start".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;

        let middle_block = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("Graph Middle".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;

        let tag_block = create_loom_block(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomBlockRequest {
                block_id: None,
                content_type: LoomBlockContentType::TagHub,
                document_id: None,
                asset_id: None,
                title: Some("Graph Tag".to_string()),
                pinned: None,
                journal_date: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?
        .0;

        let _mention_edge = create_loom_edge(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomEdgeRequest {
                edge_id: None,
                source_block_id: start_block.block_id.clone(),
                target_block_id: middle_block.block_id.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
                target_title: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let _tag_edge = create_loom_edge(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(CreateLoomEdgeRequest {
                edge_id: None,
                source_block_id: middle_block.block_id.clone(),
                target_block_id: tag_block.block_id.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
                target_title: None,
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;

        let traversed = traverse_loom_graph(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(LoomGraphTraverseQueryParams {
                start_block_id: Some(start_block.block_id.clone()),
                max_depth: Some(3),
                edge_types: Some("mention,tag".to_string()),
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(
            traversed
                .0
                .iter()
                .map(|node| (node.block.block_id.clone(), node.depth))
                .collect::<Vec<_>>(),
            vec![
                (middle_block.block_id.clone(), 1),
                (tag_block.block_id.clone(), 2),
            ]
        );

        set_loom_metrics_for_block(&state, &workspace_id, &start_block.block_id, 0, 0, 9).await?;
        let recomputed_block = recompute_loom_block_metrics(
            State(state.clone()),
            Path((workspace_id.clone(), start_block.block_id.clone())),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(recomputed_block.0.scope, "block");
        let refreshed_start = state
            .storage
            .get_loom_block(&workspace_id, &start_block.block_id)
            .await?;
        assert_eq!(refreshed_start.derived.mention_count, 1);
        assert_eq!(refreshed_start.derived.tag_count, 0);
        assert_eq!(refreshed_start.derived.backlink_count, 0);

        set_loom_metrics_for_block(&state, &workspace_id, &middle_block.block_id, 0, 0, 0).await?;
        let recomputed_workspace =
            recompute_all_loom_metrics(State(state.clone()), Path(workspace_id.clone()))
                .await
                .map_err(|(status, Json(body))| LoomApiTestCallError {
                    status,
                    code: body.error.to_string(),
                })?;
        assert_eq!(recomputed_workspace.0.scope, "workspace");

        let refreshed_middle = state
            .storage
            .get_loom_block(&workspace_id, &middle_block.block_id)
            .await?;
        let refreshed_tag = state
            .storage
            .get_loom_block(&workspace_id, &tag_block.block_id)
            .await?;
        assert_eq!(refreshed_middle.derived.mention_count, 0);
        assert_eq!(refreshed_middle.derived.tag_count, 1);
        assert_eq!(refreshed_middle.derived.backlink_count, 1);
        assert_eq!(refreshed_tag.derived.backlink_count, 1);

        let clamped = search_loom_blocks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(LoomSearchQueryParams {
                q: Some("Graph Start".to_string()),
                backlink_depth: Some(MAX_LOOM_GRAPH_DEPTH + 50),
                ..Default::default()
            }),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(clamped.0.len(), 1);

        Ok(())
    }
}
