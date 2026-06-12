use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::loom_fs::{loom_asset_blob_path, resolve_handshake_root};
use crate::models::ErrorResponse;
use crate::storage::{
    artifacts, Asset, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdge,
    LoomEdgeCreatedBy, LoomEdgeType, LoomSearchFilters, LoomViewFilters, LoomViewResponse,
    LoomViewType, NewAsset, NewLoomBlock, NewLoomEdge, PreviewStatus, StorageCapabilityStore,
    StorageError, WriteContext,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
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
        // MT-183: reorderable Pins grid ordinal
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/pin-order",
            axum::routing::put(set_loom_block_pin_order),
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
            axum::routing::put(add_block_to_loom_folder)
                .delete(remove_block_from_loom_folder),
        )
        // MT-184/185: wiki projection compiler + editable overlay
        .route(
            "/workspaces/:workspace_id/loom/wiki",
            post(compile_loom_wiki_projection),
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
        .route("/workspaces/:workspace_id/loom/tags", get(list_loom_tag_hubs))
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
) -> ApiResult<Json<crate::storage::LoomWikiProjection>> {
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

    Ok(Json(projection))
}

async fn get_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomWikiProjection>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let projection = state
        .storage
        .get_loom_wiki_projection(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(projection))
}

async fn loom_wiki_projection_stale(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let stale = state
        .storage
        .loom_wiki_projection_is_stale(&workspace_id, &projection_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(json!({ "projection_id": projection_id, "stale": stale })))
}

async fn regenerate_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomWikiProjection>> {
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

    Ok(Json(projection))
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

async fn patch_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(update): Json<LoomBlockUpdate>,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let mut fields_changed: Vec<&'static str> = Vec::new();
    if update.title.is_some() {
        fields_changed.push("title");
    }
    if update.pinned.is_some() {
        fields_changed.push("pinned");
    }
    if update.journal_date.is_some() {
        fields_changed.push("journal_date");
    }
    if update.pin_order.is_some() {
        fields_changed.push("pin_order");
    }

    let block = state
        .storage
        .update_loom_block(&ctx, &workspace_id, &block_id, update)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": block_id,
            "fields_changed": fields_changed,
            "updated_by": "user"
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

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

async fn get_asset_content(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let asset = state
        .storage
        .get_asset(&workspace_id, &asset_id)
        .await
        .map_err(map_storage_error)?;
    let handshake_root = resolve_handshake_root().map_err(internal_error)?;
    let path = loom_asset_blob_path(
        &handshake_root,
        &workspace_id,
        &asset.kind,
        &asset.content_hash,
    );

    let bytes = std::fs::read(&path).map_err(internal_error)?;

    let mut response = Response::new(axum::body::Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(asset.mime.as_str())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    Ok(response)
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
        | LoomViewResponse::Pins { blocks } => blocks.len(),
        LoomViewResponse::Sorted { groups } => groups.iter().map(|g| g.blocks.len()).sum(),
    }
}

fn parse_view_type(raw: &str) -> Option<LoomViewType> {
    match raw {
        "all" => Some(LoomViewType::All),
        "unlinked" => Some(LoomViewType::Unlinked),
        "sorted" => Some(LoomViewType::Sorted),
        "pins" => Some(LoomViewType::Pins),
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
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

async fn search_loom_blocks(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomSearchQueryParams>,
) -> ApiResult<Json<Vec<crate::storage::LoomBlockSearchResult>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let q = query.q.clone().unwrap_or_default();
    if q.trim().is_empty() {
        return Err(bad_request("HSK-400-LOOM-QUERY-REQUIRED"));
    }

    let filters = LoomSearchFilters {
        content_type: query.content_type,
        mime: query.mime,
        tag_ids: split_ids(query.tag_ids),
        mention_ids: split_ids(query.mention_ids),
        backlink_depth: query
            .backlink_depth
            .map(|depth| depth.min(MAX_LOOM_GRAPH_DEPTH)),
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
