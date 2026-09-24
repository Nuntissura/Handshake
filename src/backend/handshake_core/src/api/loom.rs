use crate::flight_recorder::{
    EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::loom_fs::{loom_asset_blob_path, resolve_handshake_root};
use crate::models::ErrorResponse;
use crate::storage::block_view_outbox;
use crate::storage::{
    artifacts, Asset, BlockViewDefinition, BlockViewRecord, BlockViewResults,
    CompensateLoomCanvasStageCard, LoomBlock, LoomBlockContentType, LoomBlockDerived,
    LoomBlockMutationReceipt, LoomBlockUpdate, LoomCanvasBoard, LoomCanvasBoardView,
    LoomCanvasPlacement, LoomCanvasPlacementRemovalReceipt, LoomCanvasPlacementUpdate,
    LoomCanvasStageProvenance, LoomCanvasVisualEdge, LoomEdge, LoomEdgeCreatedBy, LoomEdgeType,
    LoomGraphSearchResult, LoomSearchFilters, LoomSearchSourceKind, LoomViewFilters,
    LoomViewResponse, LoomViewType, LoomVisualDebugSnapshot, NewAsset, NewLoomBlock,
    NewLoomCanvasPlacement, NewLoomCanvasStageCard, NewLoomEdge, PreviewStatus,
    QuickSwitcherRecent, QuickSwitcherRecentInput, StorageCapabilityStore, StorageError,
    WriteActorKind, WriteContext, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
};
use crate::AppState;
use axum::{
    extract::{Extension, FromRequestParts, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::time::{Duration, Instant};
use tracing::Instrument;
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
        StorageError::Conflict(code) | StorageError::ConflictDetails { code, .. } => {
            (StatusCode::CONFLICT, Json(ErrorResponse { error: code }))
        }
        StorageError::Guard("HSK-403-PROTECTED-RESOURCE") => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        ),
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

#[cfg(test)]
mod storage_error_mapping_tests {
    use super::*;

    #[test]
    fn loom_storage_conflicts_are_typed_http_409_responses() {
        let (status, body) = map_storage_error(StorageError::Conflict("loom_folder_sibling_name"));
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.0.error, "loom_folder_sibling_name");
    }
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

async fn loom_create_authority(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let (mut parts, body) = request.into_parts();
    let workspace_id = match Path::<std::collections::HashMap<String, String>>::from_request_parts(
        &mut parts, &state,
    )
    .await
    {
        Ok(Path(params)) => match params.get("workspace_id") {
            Some(workspace_id) if !workspace_id.is_empty() => workspace_id.clone(),
            _ => return crate::api::authority::constant_denial().into_response(),
        },
        Err(_) => return crate::api::authority::constant_denial().into_response(),
    };
    let authority = match crate::api::authority::authorize_request(
        &state,
        &parts.headers,
        "fs.write",
        ResourceKind::Workspace,
        &workspace_id,
        ResourceAction::Create,
    )
    .await
    {
        Ok(authority) => authority,
        Err(error) => return error.into_response(),
    };
    let scope = authority.record_user_scope.clone();
    let mut request = Request::from_parts(parts, body);
    request.extensions_mut().insert(authority);
    state
        .surreal
        .with_record_user_scope(scope, next.run(request))
        .await
}

pub fn routes(state: AppState) -> Router {
    spawn_block_view_reconciler(state.clone());
    Router::new()
        // Loom blocks
        .route(
            "/workspaces/:workspace_id/loom/blocks",
            post(create_loom_block_authenticated).route_layer(middleware::from_fn_with_state(
                state.clone(),
                loom_create_authority,
            )),
        )
        .route(
            "/workspaces/:workspace_id/loom/journals/:journal_date",
            put(open_daily_journal),
        )
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id",
            get(get_loom_block)
                .patch(patch_loom_block_authenticated)
                .delete(delete_loom_block_authenticated),
        )
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/metrics/recompute",
            post(recompute_loom_block_metrics_authenticated),
        )
        // MT-177: ProjectKnowledgeIndex/EventLedger authority bridge
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/knowledge",
            get(get_loom_block_knowledge_bridge_authenticated),
        )
        // MT-258: note transclusion read-through. Resolves a block to its SOURCE
        // rich document (loom_blocks.document_id -> knowledge_rich_documents) so
        // an embedding host doc renders the source content WITHOUT copying it.
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/transclusion",
            get(get_loom_block_transclusion_authenticated),
        )
        // MT-183: reorderable Pins grid ordinal
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/pin-order",
            axum::routing::put(set_loom_block_pin_order_authenticated),
        )
        // WP-KERNEL-012 MT-024 FAIL_V2: single atomic pin removal (clear pin_order
        // + unpin + durable receipt in one transaction). Collapses the old
        // two-call remove flow so no partial persisted state is possible.
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id/remove-pin",
            axum::routing::post(remove_loom_block_pin),
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
            post(create_loom_edge_authenticated),
        )
        .route(
            "/workspaces/:workspace_id/loom/edges/:edge_id",
            delete(delete_loom_edge),
        )
        // Import + assets
        .route(
            "/workspaces/:workspace_id/loom/import",
            post(import_loom_asset_authenticated),
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
            get(query_loom_view_authenticated),
        )
        .route(
            "/workspaces/:workspace_id/loom/graph/traverse",
            get(traverse_loom_graph_authenticated),
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
            post(recompute_all_loom_metrics_authenticated),
        )
        .route(
            "/workspaces/:workspace_id/loom/search",
            get(search_loom_blocks_authenticated),
        )
        // MT-191: bounded backend visual-debug snapshot for Loom navigation state.
        .route(
            "/workspaces/:workspace_id/loom/visual-debug",
            get(loom_visual_debug_snapshot),
        )
        .route(
            "/workspaces/:workspace_id/loom/graph-search",
            get(search_loom_graph_authenticated),
        )
        // MT-264: LoomSearchV2 -- store-native, graph-blended hybrid search
        // (full-text + trigram similarity + vector kNN). Supersedes/extends the MT-258/250
        // workspace search entrypoint.
        .route(
            "/workspaces/:workspace_id/loom/search-v2",
            post(loom_search_v2_authenticated),
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
            post(create_canvas_board_authenticated).route_layer(middleware::from_fn_with_state(
                state.clone(),
                loom_create_authority,
            )),
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
            "/workspaces/:workspace_id/loom/canvas-boards/:block_id/stage-cards/:placement_id/compensate",
            post(compensate_stage_canvas_card),
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
            post(create_block_view_authenticated),
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

async fn create_loom_block_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Extension(authority): Extension<crate::api::authority::AuthorizedResourceContext>,
    Json(payload): Json<CreateLoomBlockRequest>,
) -> ApiResult<Json<LoomBlock>> {
    create_record_user_loom_block(state, workspace_id, payload, authority).await
}

#[cfg(test)]
async fn create_loom_block(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateLoomBlockRequest>,
) -> ApiResult<Json<LoomBlock>> {
    create_legacy_loom_block(state, workspace_id, payload).await
}

async fn create_record_user_loom_block(
    state: AppState,
    workspace_id: String,
    payload: CreateLoomBlockRequest,
    authority: crate::api::authority::AuthorizedResourceContext,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let ctx = loom_create_write_context(&authority)?;
    let new_block = NewLoomBlock {
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
    };
    let block = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
        .create_record_user_loom_bundle(&ctx, new_block, None)
        .await
        .map_err(map_storage_error)?;

    finalize_loom_block_create(&state, &ctx, &workspace_id, block).await
}

fn loom_create_write_context(
    authority: &crate::api::authority::AuthorizedResourceContext,
) -> ApiResult<WriteContext> {
    let actor_id = Some(authority.actor_id.clone());
    match authority.actor_kind.as_str() {
        "operator" => Ok(WriteContext::human(actor_id)),
        "system" => Ok(WriteContext::system(actor_id)),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )),
    }
}

/// MT-109 C3 (Master Spec 02-system-architecture.md:2758/2773/2776, LM-RLS-001/002): the
/// authenticated account authority of one Loom request. Every protected Loom read or write runs
/// inside [`LoomAccount::run`] as the account's record user, never as root.
struct LoomAccount {
    authority: crate::api::authority::AuthorizedResourceContext,
    ctx: WriteContext,
}

fn loom_denied() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-PROTECTED-RESOURCE",
        }),
    )
}

/// Authorizes `action` on the exact protected resource through the ResourceBroker. Reads use
/// `fs.read`; create/update/delete use `fs.write` (LM-RLS-001: viewer reads, member creates and
/// edits, admin deletes). Every failure is the constant denial.
async fn loom_account(
    state: &AppState,
    headers: &HeaderMap,
    kind: crate::storage::surreal::resource_authority::ResourceKind,
    external_id: &str,
    action: crate::storage::surreal::resource_authority::ResourceAction,
) -> ApiResult<LoomAccount> {
    use crate::storage::surreal::resource_authority::ResourceAction;
    let capability = if matches!(action, ResourceAction::Read) {
        "fs.read"
    } else {
        "fs.write"
    };
    let authority = crate::api::authority::authorize_request(
        state,
        headers,
        capability,
        kind,
        external_id,
        action,
    )
    .await
    .map_err(|_| loom_denied())?;
    let ctx = loom_create_write_context(&authority)?;
    Ok(LoomAccount { authority, ctx })
}

/// The workspace grant of the account session (folders, wiki, tags, graph, search, assets,
/// collections, AI suggestions and saved views are workspace-scoped Loom surfaces).
async fn loom_workspace_account(
    state: &AppState,
    headers: &HeaderMap,
    workspace_id: &str,
    action: crate::storage::surreal::resource_authority::ResourceAction,
) -> ApiResult<LoomAccount> {
    loom_account(
        state,
        headers,
        crate::storage::surreal::resource_authority::ResourceKind::Workspace,
        workspace_id,
        action,
    )
    .await
}

/// The exact block grant: a standalone block has its own `loom_block` resource; a RichDocument's
/// same-id Loom projection is protected by its `rich_document` resource.
async fn loom_block_account(
    state: &AppState,
    headers: &HeaderMap,
    block_id: &str,
    action: crate::storage::surreal::resource_authority::ResourceAction,
) -> ApiResult<LoomAccount> {
    use crate::storage::surreal::resource_authority::ResourceKind;
    match loom_account(state, headers, ResourceKind::LoomBlock, block_id, action).await {
        Ok(account) => Ok(account),
        Err(_) => loom_account(state, headers, ResourceKind::RichDocument, block_id, action).await,
    }
}

impl LoomAccount {
    /// The session principal every receipt written by this request carries.
    fn session_actor(&self) -> crate::kernel::KernelActor {
        match self.authority.actor_kind.as_str() {
            "system" => crate::kernel::KernelActor::System(self.authority.actor_id.clone()),
            _ => crate::kernel::KernelActor::Operator(self.authority.actor_id.clone()),
        }
    }

    /// Runs `operation` as the account's record user (table permissions apply), with receipts
    /// stamped with the session principal and bound to `workspace_id`.
    async fn run<T>(
        &self,
        state: &AppState,
        workspace_id: &str,
        operation: impl std::future::Future<Output = ApiResult<T>>,
    ) -> ApiResult<T> {
        state
            .surreal
            .with_record_user_scope(
                self.authority.record_user_scope.clone(),
                crate::storage::surreal::event_ledger::with_loom_session_receipt(
                    self.session_actor(),
                    workspace_id.to_owned(),
                    operation,
                ),
            )
            .await
    }
}

#[cfg(test)]
async fn create_legacy_loom_block(
    state: AppState,
    workspace_id: String,
    payload: CreateLoomBlockRequest,
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
                content_type: payload.content_type,
                document_id: payload.document_id,
                asset_id: payload.asset_id,
                title: payload.title,
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

    finalize_loom_block_create(&state, &ctx, &workspace_id, block).await
}

async fn finalize_loom_block_create(
    state: &AppState,
    ctx: &WriteContext,
    workspace_id: &str,
    block: LoomBlock,
) -> ApiResult<Json<LoomBlock>> {
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
    .with_wsids(vec![workspace_id.to_owned()]);
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
    match crate::loom_search::reindex_block(
        state.storage.as_ref(),
        state.llm_client.as_ref(),
        ctx,
        block,
    )
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
    headers: HeaderMap,
) -> ApiResult<Json<LoomBlock>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let journal_date = parse_journal_date(&journal_date)?;
    let denied = || {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    };
    // Master Spec §2.3.13.12.4: the daily note is created and read under the authenticated
    // account session (LM-RLS-001: members create; viewers only read an existing note).
    if crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .is_err()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "HSK-401-LOOM-SESSION",
            }),
        ));
    }
    // MT-109 C3: the workspace read grant is checked before any lookup, and the natural-key lookup
    // itself runs as the record user, so an unauthorized caller learns nothing about the workspace
    // or its journal dates (constant 403).
    let reader = loom_workspace_account(&state, &headers, &workspace_id, ResourceAction::Read)
        .await
        .map_err(|_| denied())?;
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let journal_lookup = |state: AppState, workspace_id: String, journal_date: String| {
        let reader = &reader;
        async move {
            reader
                .run(&state, &workspace_id, async {
                    ensure_workspace_exists(&state, &workspace_id).await?;
                    crate::storage::surreal::loom_canvas_store::journal_block_id(
                        &state.surreal,
                        &workspace_id,
                        &journal_date,
                    )
                    .await
                    .map_err(map_storage_error)
                })
                .await
        }
    };
    let read_existing = |block_id: String| {
        let state = state.clone();
        let headers = headers.clone();
        let workspace_id = workspace_id.clone();
        let database = database.clone();
        async move {
            let authority = crate::api::authority::authorize_request(
                &state,
                &headers,
                "fs.read",
                ResourceKind::LoomBlock,
                &block_id,
                ResourceAction::Read,
            )
            .await
            .map_err(|_| denied())?;
            state
                .surreal
                .with_record_user_scope(
                    authority.record_user_scope,
                    database.get_record_user_loom_block(&workspace_id, &block_id),
                )
                .await
                .map_err(map_storage_error)
        }
    };
    // The natural-key lookup only locates the id; the read itself is re-authorized above.
    if let Some(existing) =
        journal_lookup(state.clone(), workspace_id.clone(), journal_date.clone()).await?
    {
        return read_existing(existing).await.map(Json);
    }
    let authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.write",
        ResourceKind::Workspace,
        &workspace_id,
        ResourceAction::Create,
    )
    .await
    .map_err(|_| denied())?;
    let ctx = loom_create_write_context(&authority)?;
    let title = format!("Daily Note {journal_date}");
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = Some(format!("# {title}\n\n"));
    let created = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope,
            database.create_record_user_loom_bundle(
                &ctx,
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Journal,
                    document_id: None,
                    asset_id: None,
                    title: Some(title),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: Some(journal_date.clone()),
                    imported_at: None,
                    derived,
                },
                None,
            ),
        )
        .await;
    match created {
        Ok(block) => finalize_loom_block_create(&state, &ctx, &workspace_id, block).await,
        // A concurrent open won the uq_loom_blocks_journal_key race: return that same note.
        Err(StorageError::Conflict(_)) | Err(StorageError::ConflictDetails { .. }) => {
            let existing =
                journal_lookup(state.clone(), workspace_id.clone(), journal_date.clone())
                    .await?
                    .ok_or_else(denied)?;
            read_existing(existing).await.map(Json)
        }
        Err(error) => Err(map_storage_error(error)),
    }
}

async fn get_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomBlock>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    // A rich document's Loom projection shares the document id and is protected by the account's
    // `rich_document` resource (MT-109 C1V-LOOM-READ-403); a standalone block has its own
    // `loom_block` resource. Either exact grant authorizes the read; the denial stays constant.
    let authority = match crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::LoomBlock,
        &block_id,
        ResourceAction::Read,
    )
    .await
    {
        Ok(authority) => authority,
        Err(_) => crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.read",
            ResourceKind::RichDocument,
            &block_id,
            ResourceAction::Read,
        )
        .await
        .map_err(|_| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "HSK-403-PROTECTED-RESOURCE",
                }),
            )
        })?,
    };
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let block = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope,
            database.get_record_user_loom_block(&workspace_id, &block_id),
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(block))
}

/// MT-177: read the LoomBlock <-> ProjectKnowledgeIndex/EventLedger authority
/// bridge. Returns the knowledge entity id + EventLedger receipt id that prove
/// the block resolves to store/EventLedger authority. 404 if the block does
/// not exist; a 200 with a bridge body proves the authority binding.
async fn get_loom_block_knowledge_bridge_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<crate::storage::LoomKnowledgeBridge>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::LoomBlock,
        &block_id,
        ResourceAction::Read,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    let scoped_state = state.clone();
    state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope,
            get_loom_block_knowledge_bridge_inner(scoped_state, workspace_id, block_id),
        )
        .await
}

#[cfg(test)]
async fn get_loom_block_knowledge_bridge(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<crate::storage::LoomKnowledgeBridge>> {
    get_loom_block_knowledge_bridge_inner(state, workspace_id, block_id).await
}

async fn get_loom_block_knowledge_bridge_inner(
    state: AppState,
    workspace_id: String,
    block_id: String,
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

async fn get_loom_block_transclusion_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomTransclusionResponse>> {
    use crate::storage::surreal::resource_authority::ResourceAction;

    // MT-109 C3 (C2 follow-up c): a RichDocument's same-id Loom projection is protected by its
    // `rich_document` resource, so the transclusion read accepts either exact read grant.
    let account = loom_block_account(&state, &headers, &block_id, ResourceAction::Read).await?;
    let scoped_state = state.clone();
    let scoped_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &scoped_workspace,
            get_loom_block_transclusion_inner(scoped_state, workspace_id, block_id),
        )
        .await
}

#[cfg(test)]
async fn get_loom_block_transclusion(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<LoomTransclusionResponse>> {
    get_loom_block_transclusion_inner(state, workspace_id, block_id).await
}

async fn get_loom_block_transclusion_inner(
    state: AppState,
    workspace_id: String,
    block_id: String,
) -> ApiResult<Json<LoomTransclusionResponse>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    // A missing block is a clean 404 (not an empty/unresolved 200).
    let block = state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

    // The rich-document authority lives on the KnowledgeStore implemented by
    // the embedded SurrealDB database, mirroring the
    // knowledge_documents API's `db_for`. Native RichDocuments have a same-ID
    // LoomBlock projection (`block_id == rich_document_id`) and no legacy
    // `document_id`; imported legacy blocks continue to resolve through their
    // `documents` anchor. Both are real embedded-store authority paths.
    let knowledge_db = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let document = if let Some(document_id) = block.document_id.as_deref() {
        crate::storage::knowledge::KnowledgeStore::get_knowledge_rich_document_by_document_id(
            &knowledge_db,
            &workspace_id,
            document_id,
        )
        .await
        .map_err(map_storage_error)?
    } else {
        crate::storage::knowledge::KnowledgeStore::get_knowledge_rich_document(
            &knowledge_db,
            &block.block_id,
        )
        .await
        .map_err(map_storage_error)?
        .filter(|document| document.workspace_id == workspace_id)
    };

    let Some(document) = document else {
        return Ok(Json(LoomTransclusionResponse {
            block_id,
            workspace_id,
            source_document_id: block.document_id,
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
    headers: HeaderMap,
) -> ApiResult<Json<crate::storage::LoomBreadcrumbTrail>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let trail = state
                .storage
                .loom_block_breadcrumbs(&workspace_id, &block_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(trail))
        })
        .await
}

/// MT-178: linked backlinks for a block (incoming MENTION/TAG/... edges) each
/// with the referencing source block and a surrounding-text context snippet.
async fn get_loom_block_backlinks(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<crate::storage::LoomBacklink>>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let backlinks = state
                .storage
                .get_backlinks_with_context(&workspace_id, &block_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(backlinks))
        })
        .await
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
    headers: HeaderMap,
    Query(query): Query<UnlinkedMentionQuery>,
) -> ApiResult<Json<Vec<crate::storage::LoomUnlinkedMention>>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let aliases = split_ids(query.aliases);
            let limit = query.limit.unwrap_or(100).min(500);
            let mentions = state
                .storage
                .scan_unlinked_mentions(&workspace_id, &block_id, &aliases, limit)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(mentions))
        })
        .await
}

#[derive(Debug, Deserialize)]
struct SetPinOrderRequest {
    /// The new ordinal, or `null` to clear it (un-order the pin).
    #[serde(default)]
    pin_order: Option<i32>,
}

/// MT-183: set or clear a block's Pins-grid ordinal (reorderable grid).
async fn set_loom_block_pin_order_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<SetPinOrderRequest>,
) -> ApiResult<Json<LoomBlock>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let ctx = account.ctx.clone();
    account
        .run(
            &state,
            &workspace_id,
            set_loom_block_pin_order_inner(
                state.clone(),
                workspace_id.clone(),
                block_id,
                payload,
                ctx,
            ),
        )
        .await
}

#[cfg(test)]
async fn set_loom_block_pin_order(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    Json(payload): Json<SetPinOrderRequest>,
) -> ApiResult<Json<LoomBlock>> {
    set_loom_block_pin_order_inner(
        state,
        workspace_id,
        block_id,
        payload,
        WriteContext::human(None),
    )
    .await
}

async fn set_loom_block_pin_order_inner(
    state: AppState,
    workspace_id: String,
    block_id: String,
    payload: SetPinOrderRequest,
    ctx: WriteContext,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
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

/// WP-KERNEL-012 MT-024 FAIL_V2: atomically remove a pin in ONE backend call.
/// Storage clears pin_order AND unpins the block alongside the durable
/// EventLedger receipt in a single transaction, so the running app can never
/// leave the partial `pin_order cleared but still pinned` state the old two-call
/// remove flow risked. The Flight Recorder mirror stays a best-effort Tier-1
/// mirror; the durable authority receipt is now atomic in storage.
async fn remove_loom_block_pin(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomBlockMutationReceipt>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let receipt = account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .remove_loom_block_pin(&account.ctx, &workspace_id, &block_id)
                .await
                .map_err(map_storage_error)
        })
        .await?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": receipt.block.block_id,
            "fields_changed": ["pin_order", "pinned"],
            "updated_by": "user",
        }),
    )
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    Ok(Json(receipt))
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

fn wiki_db(state: &AppState) -> std::sync::Arc<crate::storage::surreal::SurrealDatabase> {
    std::sync::Arc::new(crate::storage::surreal::SurrealDatabase::new(
        state.surreal.clone(),
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
    let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_db(state));
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
    headers: HeaderMap,
    Json(payload): Json<CompileWikiRequest>,
) -> ApiResult<Json<ServedWikiPage>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    account
        .run(
            &state,
            &workspace_id,
            compile_loom_wiki_projection_inner(&state, &workspace_id, payload),
        )
        .await
}

async fn compile_loom_wiki_projection_inner(
    state: &AppState,
    workspace_id: &str,
    payload: CompileWikiRequest,
) -> ApiResult<Json<ServedWikiPage>> {
    let workspace_id = workspace_id.to_owned();
    ensure_workspace_exists(state, &workspace_id).await?;
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
    headers: HeaderMap,
) -> ApiResult<Json<ServedWikiPage>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let projection = state
                .storage
                .get_loom_wiki_projection(&workspace_id, &projection_id)
                .await
                .map_err(map_storage_error)?;
            // LM-PWIKI-008: the single-page serve path attaches the verdict
            // fail-closed.
            Ok(Json(attach_wiki_verdict(&state, projection).await?))
        })
        .await
}

async fn loom_wiki_projection_stale(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let projection = state
                .storage
                .get_loom_wiki_projection(&workspace_id, &projection_id)
                .await
                .map_err(map_storage_error)?;
            let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_db(&state));
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
        })
        .await
}

async fn regenerate_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<ServedWikiPage>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
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
    headers: HeaderMap,
    Query(params): Query<ListWikiPagesQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let db = wiki_db(&state);
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
                .zip(verdicts)
                .map(|(page, verdict)| {
                    let mut value = serde_json::to_value(&page).unwrap_or_else(|_| json!({}));
                    value["staleness_verdict"] = serde_json::to_value(&verdict).unwrap_or_default();
                    value
                })
                .collect();
            Ok(Json(json!({ "pages": served })))
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let request = payload.map(|Json(p)| p).unwrap_or_default();
            // MT-109 C3: wiki receipts carry the authenticated session principal, never a header actor.
            let ctx = crate::knowledge_wiki::compiler::WikiCompileContext {
                actor: account.session_actor(),
                ..wiki_compile_context(&headers)
            };
            let db = wiki_db(&state);
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
                .zip(verdicts)
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
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let request = payload.map(|Json(p)| p).unwrap_or_default();
            // MT-109 C3: wiki receipts carry the authenticated session principal, never a header actor.
            let ctx = crate::knowledge_wiki::compiler::WikiCompileContext {
                actor: account.session_actor(),
                ..wiki_compile_context(&headers)
            };
            let checker = crate::knowledge_wiki::drift::WikiDriftChecker::new(wiki_db(&state));
            let report = checker
                .check_workspace(&ctx, &workspace_id, request.persist.unwrap_or(true))
                .await
                .map_err(map_wiki_error)?;
            Ok(Json(report))
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            // MT-109 C3: wiki receipts carry the authenticated session principal, never a header actor.
            let ctx = crate::knowledge_wiki::compiler::WikiCompileContext {
                actor: account.session_actor(),
                ..wiki_compile_context(&headers)
            };
            let engine = crate::knowledge_wiki::fanout::WikiFanOutEngine::new(wiki_db(&state));
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
        })
        .await
}

async fn delete_loom_wiki_projection(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Delete,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .delete_loom_wiki_projection(&workspace_id, &projection_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(json!({ "status": "deleted" })))
        })
        .await
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
    headers: HeaderMap,
    Json(payload): Json<AddWikiOverlayRequest>,
) -> ApiResult<Json<crate::storage::LoomWikiOverlay>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

async fn list_loom_wiki_overlays(
    State(state): State<AppState>,
    Path((workspace_id, projection_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<crate::storage::LoomWikiOverlay>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let overlays = state
                .storage
                .list_loom_wiki_overlays(&workspace_id, &projection_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(overlays))
        })
        .await
}

async fn delete_loom_wiki_overlay(
    State(state): State<AppState>,
    Path((workspace_id, overlay_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Delete,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .delete_loom_wiki_overlay(&workspace_id, &overlay_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(json!({ "status": "deleted" })))
        })
        .await
}

// -- MT-187 markdown import boundary handler -------------------------------

#[derive(Debug, Deserialize)]
struct ImportMarkdownRequest {
    title: String,
    markdown: String,
}

/// MT-187 + MT-109 C3: the imported markdown becomes an account-owned RichDocument (its protected
/// resource, creator grant, same-id Loom projection and backlinks) created as the record user
/// under the workspace create grant, through the same store path as the documents API import.
async fn import_markdown_to_loom(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ImportMarkdownRequest>,
) -> ApiResult<Json<crate::storage::LoomMarkdownImport>> {
    use crate::knowledge_document::import::{import_snippet, ImportFormat};
    use crate::storage::knowledge::KnowledgeStore;

    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let title = payload.title.trim().to_owned();
    if title.is_empty() {
        return Err(bad_request("HSK-400-LOOM-VALIDATION"));
    }
    let loom_workspace = workspace_id.clone();
    let imported = account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let outcome = import_snippet(&payload.markdown, ImportFormat::Markdown);
            let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
            let document = database
                .create_knowledge_rich_document(
                    crate::storage::knowledge::NewKnowledgeRichDocument {
                        workspace_id: workspace_id.clone(),
                        document_id: None,
                        title,
                        schema_version:
                            crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION
                                .to_owned(),
                        content_json: outcome.document_json.clone(),
                        crdt_document_id: None,
                        crdt_snapshot_id: None,
                        promotion_receipt_event_id: None,
                        project_ref: None,
                        folder_ref: None,
                        authority_label: Some("promoted".to_owned()),
                        owner_actor_kind: Some(account.authority.actor_kind.clone()),
                        owner_actor_id: Some(account.authority.actor_id.clone()),
                    },
                )
                .await
                .map_err(map_storage_error)?;
            let block = state
                .storage
                .get_loom_block(&workspace_id, &document.rich_document_id)
                .await
                .map_err(map_storage_error)?;
            Ok(crate::storage::LoomMarkdownImport {
                block,
                rich_document_id: document.rich_document_id,
                warnings: outcome
                    .warnings
                    .iter()
                    .map(|warning| format!("{}: {}", warning.code, warning.detail))
                    .collect(),
            })
        })
        .await?;

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

/// WP-KERNEL-012 E3 MT-022: best-effort Tier-1 Flight Recorder mirror of a Loom
/// folder mutation. The DURABLE receipt is the atomic KNOWLEDGE_LOOM_FOLDER_MUTATED
/// EventLedger row appended inside the storage transaction; this DuckDB mirror is
/// observability only and its failure never affects the committed mutation.
async fn record_loom_folder_flight_event(
    state: &AppState,
    workspace_id: &str,
    folder_id: &str,
    operation: &str,
) {
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomFolderMutated,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "type": "loom_folder_mutated",
            "workspace_id": workspace_id,
            "folder_id": folder_id,
            "operation": operation,
        }),
    )
    .with_wsids(vec![workspace_id.to_string()]);
    let _ = state.flight_recorder.record_event(event).await;
}

async fn create_loom_folder(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateLoomFolderRequest>,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
            record_loom_folder_flight_event(&state, &workspace_id, &folder.folder_id, "create")
                .await;
            Ok(Json(folder))
        })
        .await
}

async fn list_loom_folders(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<crate::storage::LoomFolder>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let folders = state
                .storage
                .list_loom_folders(&workspace_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(folders))
        })
        .await
}

async fn get_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let folder = state
                .storage
                .get_loom_folder(&workspace_id, &folder_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(folder))
        })
        .await
}

async fn update_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(update): Json<crate::storage::LoomFolderUpdate>,
) -> ApiResult<Json<crate::storage::LoomFolder>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let folder = state
                .storage
                .update_loom_folder(&workspace_id, &folder_id, update)
                .await
                .map_err(map_storage_error)?;
            record_loom_folder_flight_event(&state, &workspace_id, &folder_id, "update").await;
            Ok(Json(folder))
        })
        .await
}

async fn delete_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Delete,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .delete_loom_folder(&workspace_id, &folder_id)
                .await
                .map_err(map_storage_error)?;
            record_loom_folder_flight_event(&state, &workspace_id, &folder_id, "delete").await;
            Ok(Json(json!({ "status": "deleted" })))
        })
        .await
}

#[derive(Debug, Deserialize, Default)]
struct AddFolderMemberRequest {
    #[serde(default)]
    sort_order: Option<i32>,
}

async fn add_block_to_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id, block_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(payload): Json<AddFolderMemberRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .add_block_to_loom_folder(&workspace_id, &folder_id, &block_id, payload.sort_order)
                .await
                .map_err(map_storage_error)?;
            record_loom_folder_flight_event(&state, &workspace_id, &folder_id, "add_member").await;
            Ok(Json(json!({ "status": "added" })))
        })
        .await
}

async fn remove_block_from_loom_folder(
    State(state): State<AppState>,
    Path((workspace_id, folder_id, block_id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .remove_block_from_loom_folder(&workspace_id, &folder_id, &block_id)
                .await
                .map_err(map_storage_error)?;
            record_loom_folder_flight_event(&state, &workspace_id, &folder_id, "remove_member")
                .await;
            Ok(Json(json!({ "status": "removed" })))
        })
        .await
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
    headers: HeaderMap,
    Query(query): Query<LoomFolderBlocksQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let limit = query.limit.unwrap_or(100).min(500);
            let offset = query.offset.unwrap_or(0);
            let blocks = state
                .storage
                .list_loom_folder_blocks(&workspace_id, &folder_id, limit, offset)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(blocks))
        })
        .await
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
    headers: HeaderMap,
    Query(query): Query<LoomTagListQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let limit = query.limit.unwrap_or(100).min(500);
            let offset = query.offset.unwrap_or(0);
            let tags = state
                .storage
                .list_tag_hubs(&workspace_id, limit, offset)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(tags))
        })
        .await
}

/// MT-182: the tag-hub surface (block + sub-tags + tagged blocks + backlinks).
async fn get_loom_tag_hub(
    State(state): State<AppState>,
    Path((workspace_id, tag_block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            let request_id = Uuid::now_v7().to_string();
            let run_id =
                std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "product-runtime".to_owned());
            get_loom_tag_hub_instrumented(state.clone(), workspace_id, tag_block_id)
                .instrument(tracing::info_span!(
                    "loom_tag_hub_request",
                    run_id,
                    request_id
                ))
                .await
        })
        .await
}

async fn get_loom_tag_hub_instrumented(
    state: AppState,
    workspace_id: String,
    tag_block_id: String,
) -> ApiResult<Response> {
    let workspace_lookup_started = Instant::now();
    ensure_workspace_exists(&state, &workspace_id).await?;
    tracing::info!(
        target: "handshake_core",
        event = "loom_tag_hub_stage_timing",
        stage = "workspace_lookup",
        elapsed_us = workspace_lookup_started.elapsed().as_micros(),
        "loom_tag_hub_stage_timing"
    );
    let storage_started = Instant::now();
    let hub = state
        .storage
        .get_tag_hub(&workspace_id, &tag_block_id)
        .await
        .map_err(map_storage_error)?;
    let storage_elapsed_us = storage_started.elapsed().as_micros();
    let serialize_started = Instant::now();
    let bytes = serde_json::to_vec(&hub).map_err(internal_error)?;
    let serialize_elapsed_us = serialize_started.elapsed().as_micros();
    let response_bytes = bytes.len();
    let handoff_started = Instant::now();
    let mut response = Response::new(axum::body::Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let handoff_elapsed_us = handoff_started.elapsed().as_micros();
    tracing::info!(
        target: "handshake_core",
        event = "loom_tag_hub_stage_timing",
        stage = "tag_hub_storage_total_after_workspace_lookup",
        elapsed_us = storage_elapsed_us,
        "loom_tag_hub_stage_timing"
    );
    tracing::info!(
        target: "handshake_core",
        event = "loom_tag_hub_stage_timing",
        stage = "json_serialization",
        response_bytes,
        elapsed_us = serialize_elapsed_us,
        "loom_tag_hub_stage_timing"
    );
    tracing::info!(
        target: "handshake_core",
        event = "loom_tag_hub_stage_timing",
        stage = "response_construction",
        response_bytes,
        elapsed_us = handoff_elapsed_us,
        "loom_tag_hub_stage_timing"
    );
    Ok(response)
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
    headers: HeaderMap,
    Query(query): Query<LoomTagBlocksQuery>,
) -> ApiResult<Json<Vec<LoomBlock>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
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

/// Canonical native-view write attribution. The native client sends the shared `x-hsk-*` identity
/// vocabulary; Loom persists the actor kind/id through `WriteContext` instead of collapsing every
/// collection mutation to an anonymous human write. MT-109 C3: account-scoped routes attribute
/// writes to the session principal instead; this header form serves the legacy test handlers.
#[cfg(test)]
fn block_view_write_context(headers: &axum::http::HeaderMap) -> WriteContext {
    fn value<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
        headers
            .get(name)
            .and_then(|header| header.to_str().ok())
            .map(str::trim)
            .filter(|header| !header.is_empty())
    }

    let actor_id = value(headers, "x-hsk-actor-id").map(ToOwned::to_owned);
    match value(headers, "x-hsk-actor-kind") {
        Some("system") | Some("session_broker") | Some("toolgate") => {
            WriteContext::system(actor_id)
        }
        Some("ai") | Some("model_adapter") => WriteContext::ai(actor_id, None, None),
        _ => WriteContext::human(actor_id),
    }
}

/// Mirror the persisted write actor into the canonical Flight Recorder envelope. Actor identity is an
/// envelope field; Loom event payload schemas are deliberately closed and reject ad-hoc identity keys.
fn block_view_flight_actor(ctx: &WriteContext) -> (FlightRecorderActor, String) {
    let actor = match ctx.actor_kind {
        WriteActorKind::Human => FlightRecorderActor::Human,
        WriteActorKind::Ai => FlightRecorderActor::Agent,
        WriteActorKind::System => FlightRecorderActor::System,
    };
    let actor_id = ctx.actor_id.clone().unwrap_or_else(|| actor.to_string());
    (actor, actor_id)
}

fn spawn_block_view_reconciler(state: AppState) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = reconcile_block_view_events(&state, None, None).await {
                let (status, body) = error;
                tracing::error!(
                    target: "handshake_core::loom_api",
                    status = %status,
                    error = body.0.error,
                    "block_view_outbox_reconciliation_failed"
                );
            }
            // A bounded, quiet service-lifetime retry closes transient recorder
            // outages without quarantining or busy-looping durable intent.
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
}

async fn record_block_view_event_idempotent(
    state: &AppState,
    event: FlightRecorderEvent,
) -> ApiResult<()> {
    let existing = state
        .flight_recorder
        .list_events(EventFilter {
            event_id: Some(event.event_id),
            ..EventFilter::default()
        })
        .await
        .map_err(internal_error)?;
    if let Some(existing) = existing.first() {
        return if block_view_outbox::events_equal(existing, &event) {
            Ok(())
        } else {
            Err(map_storage_error(StorageError::Conflict(
                "block_view_flight_event_identity",
            )))
        };
    }
    if let Err(write_error) = state.flight_recorder.record_event(event.clone()).await {
        let existing = state
            .flight_recorder
            .list_events(EventFilter {
                event_id: Some(event.event_id),
                ..EventFilter::default()
            })
            .await
            .map_err(internal_error)?;
        if existing
            .first()
            .is_some_and(|existing| block_view_outbox::events_equal(existing, &event))
        {
            return Ok(());
        }
        return Err(internal_error(write_error));
    }
    Ok(())
}

async fn reconcile_block_view_events(
    state: &AppState,
    workspace_id: Option<&str>,
    event_id: Option<Uuid>,
) -> ApiResult<()> {
    if let Some(event_id) = event_id {
        let workspace_id = workspace_id.ok_or_else(|| {
            map_storage_error(StorageError::Validation(
                "scoped block-view publication requires workspace_id",
            ))
        })?;
        let event = match block_view_outbox::load_scoped_publication(
            &state.surreal,
            workspace_id,
            event_id,
        )
        .await
        .map_err(map_storage_error)?
        {
            block_view_outbox::ScopedPublicationEvent::Published => return Ok(()),
            block_view_outbox::ScopedPublicationEvent::Pending(event) => event,
        };
        if let Err(error) = record_block_view_event_idempotent(state, event.clone()).await {
            let error_summary = format!("{}:{}", error.0, error.1 .0.error);
            block_view_outbox::record_failure(
                &state.surreal,
                workspace_id,
                event.event_id,
                &error_summary,
            )
            .await
            .map_err(map_storage_error)?;
            return Err(error);
        }
        block_view_outbox::mark_published(&state.surreal, workspace_id, event.event_id)
            .await
            .map_err(map_storage_error)?;
        return Ok(());
    }

    loop {
        let pending = block_view_outbox::list_pending(&state.surreal, workspace_id, None, 200)
            .await
            .map_err(map_storage_error)?;
        let pending_count = pending.len();
        let mut first_error = None;
        for (event_workspace_id, event) in pending {
            if let Err(error) = record_block_view_event_idempotent(state, event.clone()).await {
                let error_summary = format!("{}:{}", error.0, error.1 .0.error);
                block_view_outbox::record_failure(
                    &state.surreal,
                    &event_workspace_id,
                    event.event_id,
                    &error_summary,
                )
                .await
                .map_err(map_storage_error)?;
                first_error.get_or_insert(error);
                continue;
            }
            block_view_outbox::mark_published(&state.surreal, &event_workspace_id, event.event_id)
                .await
                .map_err(map_storage_error)?;
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        if pending_count < 200 {
            return Ok(());
        }
    }
}

async fn patch_loom_block_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<LoomBlockPatchRequest>,
) -> ApiResult<Json<LoomBlock>> {
    use crate::storage::surreal::resource_authority::ResourceAction;

    // MT-109 C3: the tag-edge writes of this PATCH (Kanban card move) run in the same account
    // scope, with receipts bound to the block workspace and stamped with the session principal.
    let account = loom_block_account(&state, &headers, &block_id, ResourceAction::Update).await?;
    let ctx = account.ctx.clone();
    let scoped_state = state.clone();
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            patch_loom_block_inner(scoped_state, workspace_id, block_id, payload, ctx),
        )
        .await
}

#[cfg(test)]
async fn patch_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<LoomBlockPatchRequest>,
) -> ApiResult<Json<LoomBlock>> {
    patch_loom_block_inner(
        state,
        workspace_id,
        block_id,
        payload,
        block_view_write_context(&headers),
    )
    .await
}

async fn patch_loom_block_inner(
    state: AppState,
    workspace_id: String,
    block_id: String,
    payload: LoomBlockPatchRequest,
    ctx: WriteContext,
) -> ApiResult<Json<LoomBlock>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

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

    let (flight_actor, flight_actor_id) = block_view_flight_actor(&ctx);
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        flight_actor,
        Uuid::now_v7(),
        json!({
            "type": "loom_block_updated",
            "block_id": block_id,
            "fields_changed": fields_changed,
            "tags_added": add_tags,
            "tags_removed": remove_tags,
            "updated_by": "user",
        }),
    )
    .with_actor_id(flight_actor_id)
    .with_wsids(vec![workspace_id]);
    let _ = state.flight_recorder.record_event(event).await;

    // WP-KERNEL-009 MT-264: an edited title/text changes the block's flattened
    // search text, so refresh the semantic embedding projection too (the
    // keyword/trigram row is refreshed in storage update_loom_block). No-op
    // decline when no embedding model is configured.
    refresh_loom_block_embedding(&state, &ctx, &block).await;

    Ok(Json(block))
}

async fn delete_loom_block_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let account = loom_account(
        &state,
        &headers,
        ResourceKind::LoomBlock,
        &block_id,
        ResourceAction::Delete,
    )
    .await?;
    let ctx = account.ctx.clone();
    let scoped_state = state.clone();
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            delete_loom_block_inner(scoped_state, workspace_id, block_id, ctx),
        )
        .await
}

#[cfg(test)]
async fn delete_loom_block(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    delete_loom_block_inner(state, workspace_id, block_id, WriteContext::human(None)).await
}

async fn delete_loom_block_inner(
    state: AppState,
    workspace_id: String,
    block_id: String,
    ctx: WriteContext,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let existing = state
        .storage
        .get_loom_block(&workspace_id, &block_id)
        .await
        .map_err(map_storage_error)?;

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
    ctx: &WriteContext,
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
            let created = state
                .storage
                .create_loom_block(
                    ctx,
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
            // must resolve to ProjectKnowledgeIndex + EventLedger authority. Under an account
            // scope the create is the owned bundle, which already wrote that bridge (MT-109 C3).
            if crate::storage::surreal::current_record_user_scope().is_none() {
                state
                    .storage
                    .bridge_loom_block_to_knowledge(ctx, workspace_id, &created.block_id)
                    .await
                    .map_err(map_storage_error)?;
            }
            Ok(())
        }
        Err(err) => Err(map_storage_error(err)),
    }
}

/// MT-109 C3: edges are created as the account's record user under the workspace create grant
/// (an auto-created link/tag target is an owned block); the edge row permission re-checks the
/// source block edit grant and the target block read grant.
async fn create_loom_edge_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateLoomEdgeRequest>,
) -> ApiResult<Json<LoomEdge>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let ctx = account.ctx.clone();
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            create_loom_edge_inner(state.clone(), workspace_id, payload, ctx),
        )
        .await
}

#[cfg(test)]
async fn create_loom_edge(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateLoomEdgeRequest>,
) -> ApiResult<Json<LoomEdge>> {
    create_loom_edge_inner(state, workspace_id, payload, WriteContext::human(None)).await
}

async fn create_loom_edge_inner(
    state: AppState,
    workspace_id: String,
    payload: CreateLoomEdgeRequest,
    ctx: WriteContext,
) -> ApiResult<Json<LoomEdge>> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    state
        .storage
        .get_loom_block(&workspace_id, &payload.source_block_id)
        .await
        .map_err(map_storage_error)?;

    ensure_edge_target_exists(
        &state,
        &ctx,
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
    headers: HeaderMap,
) -> ApiResult<Json<LoomEdge>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;

            let ctx = account.ctx.clone();
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
        })
        .await
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

/// MT-109 C3 + MT-153 C4: the asset, blob and owned file block are written as the account's record
/// user under the workspace create grant, and the preview job is enqueued and run under the same
/// account authority (Master Spec 02-system-architecture.md:2773: no root job start for a user).
async fn import_loom_asset_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<LoomImportRequest>,
) -> ApiResult<Json<LoomImportResult>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let ctx = account.ctx.clone();
    let loom_workspace = workspace_id.clone();
    let (result, preview_job) = account
        .run(
            &state,
            &loom_workspace,
            import_loom_asset_inner(state.clone(), workspace_id, payload, ctx),
        )
        .await?;
    if let Some(job_inputs) = preview_job {
        account
            .run(
                &state,
                &loom_workspace,
                enqueue_loom_preview_job(&state, job_inputs),
            )
            .await?;
    }
    Ok(Json(result))
}

#[cfg(test)]
async fn import_loom_asset(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<LoomImportRequest>,
) -> ApiResult<Json<LoomImportResult>> {
    let (result, preview_job) = import_loom_asset_inner(
        state.clone(),
        workspace_id,
        payload,
        WriteContext::human(None),
    )
    .await?;
    if let Some(job_inputs) = preview_job {
        enqueue_loom_preview_job(&state, job_inputs).await?;
    }
    Ok(Json(result))
}

/// Enqueues the real background preview generation job (same protocol for import and retry).
async fn enqueue_loom_preview_job(
    state: &AppState,
    job_inputs: serde_json::Value,
) -> ApiResult<()> {
    let capability_profile_id = state
        .capability_registry
        .profile_for_job_request(
            crate::storage::JobKind::LoomPreviewGenerate.as_str(),
            "hsk.loom.preview_generate@v1",
        )
        .map_err(internal_error)?;
    let job = crate::jobs::create_job(
        state,
        crate::storage::JobKind::LoomPreviewGenerate,
        "hsk.loom.preview_generate@v1",
        capability_profile_id.id.as_str(),
        Some(job_inputs),
        Vec::new(),
    )
    .await
    .map_err(|error| match error {
        crate::jobs::JobError::Storage(StorageError::Guard("HSK-403-PROTECTED-RESOURCE")) => {
            loom_denied()
        }
        other => internal_error(other),
    })?;
    let _ = crate::workflows::start_workflow_for_job(state, job).await;
    Ok(())
}

async fn import_loom_asset_inner(
    state: AppState,
    workspace_id: String,
    payload: LoomImportRequest,
    ctx: WriteContext,
) -> ApiResult<(LoomImportResult, Option<serde_json::Value>)> {
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

        return Ok((
            LoomImportResult {
                dedup_hit: true,
                existing_block_id: Some(existing.block_id.clone()),
                block_id: existing.block_id,
                asset_id: existing.asset_id,
                content_hash,
            },
            None,
        ));
    }

    let handshake_root = resolve_handshake_root().map_err(internal_error)?;

    let mime = payload
        .mime
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let kind = "original".to_string();

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
    // EventLedger authority before we report success. Under an account scope the create is the
    // owned bundle, which already wrote that bridge (MT-109 C3).
    if crate::storage::surreal::current_record_user_scope().is_none() {
        state
            .storage
            .bridge_loom_block_to_knowledge(&ctx, &workspace_id, &block.block_id)
            .await
            .map_err(map_storage_error)?;
    }

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

    let preview_job = json!({
        "workspace_id": workspace_id.clone(),
        "block_id": block.block_id.clone(),
        "asset_id": block.asset_id.clone(),
        "content_hash": content_hash.clone(),
        "requested_tier": 1,
    });

    Ok((
        LoomImportResult {
            dedup_hit: false,
            existing_block_id: None,
            block_id: block.block_id,
            asset_id: block.asset_id,
            content_hash,
        },
        Some(preview_job),
    ))
}

async fn get_asset_metadata(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Asset>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let asset = state
                .storage
                .get_asset(&workspace_id, &asset_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(asset))
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

async fn get_asset_thumbnail(
    State(state): State<AppState>,
    Path((workspace_id, asset_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
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
    headers: HeaderMap,
) -> ApiResult<Json<ListTiersResponse>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

#[derive(Debug, Serialize)]
struct RetryTierResponse {
    tier: String,
    status: String,
    attempt_count: i32,
    requeued: bool,
}

/// MT-259 + MT-109 C3: the tier flip runs as the account's record user under the workspace update
/// grant (the deny-by-default gate for a job start); the regeneration job is then enqueued on the
/// kernel job queue.
async fn retry_asset_tier(
    State(state): State<AppState>,
    Path((workspace_id, asset_id, tier_str)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<RetryTierResponse>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let tier =
        crate::storage::MediaTier::from_str(&tier_str).map_err(|_| bad_request("invalid_tier"))?;
    let loom_workspace = workspace_id.clone();
    let (updated, block_id) = account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let block = state
                .storage
                .find_loom_block_by_asset_id(&workspace_id, &asset_id)
                .await
                .map_err(map_storage_error)?
                .ok_or_else(|| not_found("loom_block_not_found"))?;
            // Flip status -> pending; storage bumps attempt_count on the failed->pending
            // transition so the retry is recorded and never silent.
            let updated = state
                .storage
                .set_media_tier_status(
                    &account.ctx,
                    &workspace_id,
                    &asset_id,
                    tier,
                    crate::storage::MediaTierStatus::Pending,
                    None,
                )
                .await
                .map_err(map_storage_error)?;
            Ok((updated, block.block_id))
        })
        .await?;

    // Requeue the real background generation job (same protocol as import) under the same account
    // authority (MT-153 C4: never a root job start on behalf of a user).
    account
        .run(
            &state,
            &loom_workspace,
            enqueue_loom_preview_job(
                &state,
                json!({
                    "workspace_id": workspace_id.clone(),
                    "block_id": block_id,
                    "asset_id": asset_id.clone(),
                    "requested_tier": 1,
                    "retry": true,
                }),
            ),
        )
        .await?;

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
    headers: HeaderMap,
    Json(req): Json<CreateCollectionRequest>,
) -> ApiResult<Json<CollectionView>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let ctx = account.ctx.clone();
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
        })
        .await
}

async fn get_loom_collection(
    State(state): State<AppState>,
    Path((workspace_id, collection_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<CollectionView>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let result = state
                .storage
                .get_loom_collection(&workspace_id, &collection_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(result.into()))
        })
        .await
}

#[derive(Debug, Deserialize)]
struct SetCollectionOrderRequest {
    asset_ids: Vec<String>,
}

async fn set_loom_collection_order(
    State(state): State<AppState>,
    Path((workspace_id, collection_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<SetCollectionOrderRequest>,
) -> ApiResult<Json<CollectionView>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let ctx = account.ctx.clone();
            let result = state
                .storage
                .set_loom_collection_order(&ctx, &workspace_id, &collection_id, &req.asset_ids)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(result.into()))
        })
        .await
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

/// MT-109 C3: the confirming reviewer of an account-scoped accept/reject is the session principal.
fn loom_account_reviewer(account: &LoomAccount) -> ApiResult<KnowledgeActorIdV1> {
    let kind = match account.authority.actor_kind.as_str() {
        "operator" => KnowledgeActorKind::Operator,
        _ => KnowledgeActorKind::System,
    };
    KnowledgeActorIdV1::new(kind, &account.authority.actor_id)
        .map_err(|_| bad_request("HSK-400-LOOM-AI-ACTOR"))
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
                &state.surreal,
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
        })
        .await
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
/// store-native search. The response carries per-modality scores, content
/// facets, snippet highlights, and a `semantic_available` flag.
/// MT-109 C3: the account-scoped LoomSearchV2 route (a read: workspace fs.read grant).
async fn loom_search_v2_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<LoomSearchV2Body>,
) -> ApiResult<Json<crate::storage::LoomSearchV2Response>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            loom_search_v2(State(state.clone()), Path(workspace_id), Json(payload)),
        )
        .await
}

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
    headers: HeaderMap,
    Query(query): Query<ListLoomAiSuggestionsQuery>,
) -> ApiResult<Json<Vec<LoomAiSuggestionRow>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let rows = list_suggestion_rows(
                &state.surreal,
                &workspace_id,
                query.job_id.as_deref(),
                query.state.as_deref(),
            )
            .await
            .map_err(map_storage_error)?;
            Ok(Json(rows))
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            // MT-109 C3: the reviewer is the authenticated session principal, never a header actor.
            let reviewer = loom_account_reviewer(&account)?;
            let reason = body
                .and_then(|b| b.0.reason)
                .unwrap_or_else(|| "operator confirmed AI Loom suggestion".to_string());

            let outcome = accept_suggestion_flow(
                state.storage.as_ref(),
                &state.surreal,
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
                LoomAiAcceptOutcome::UnknownSuggestion { .. } => {
                    Err(not_found("loom_ai_suggestion_not_found"))
                }
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
        })
        .await
}

async fn reject_loom_ai_suggestion(
    State(state): State<AppState>,
    Path((workspace_id, suggestion_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Option<Json<ReviewLoomAiSuggestionRequest>>,
) -> ApiResult<Json<LoomAiSuggestionRow>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            // MT-109 C3: the reviewer is the authenticated session principal, never a header actor.
            let reviewer = loom_account_reviewer(&account)?;
            let reason = body
                .and_then(|b| b.0.reason)
                .unwrap_or_else(|| "operator rejected AI Loom suggestion".to_string());

            let outcome = reject_suggestion_flow(
                state.storage.as_ref(),
                &state.surreal,
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
                LoomAiRejectOutcome::UnknownSuggestion { .. } => {
                    Err(not_found("loom_ai_suggestion_not_found"))
                }
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
        })
        .await
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
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            // MT-109 C3: the reviewer is the authenticated session principal, never a header actor.
            let reviewer = loom_account_reviewer(&account)?;
            let kind_filter = body.and_then(|b| b.0.kind);

            let session = loom_ai_session(&headers);
            let correlation = loom_ai_correlation(&headers);

            // Delegate to the canonical accept-all sweep (lists the PENDING authority
            // set from SurrealDB and runs the SAME per-item flow on each), so per-item
            // authority is enforced identically for the HTTP and direct callers.
            let outcome = accept_all_suggestions_flow(
                state.storage.as_ref(),
                &state.surreal,
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
        })
        .await
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

/// MT-109 C3: the account-scoped route for [`query_loom_view`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn query_loom_view_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, view_type_raw)): Path<(String, String)>,
    Query(query): Query<LoomViewQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomViewResponse>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            query_loom_view(
                State(state.clone()),
                Path((workspace_id, view_type_raw)),
                Query(query),
            ),
        )
        .await
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

/// MT-109 C3: the account-scoped route for [`traverse_loom_graph`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn traverse_loom_graph_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomGraphTraverseQueryParams>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<LoomGraphTraversalNode>>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            traverse_loom_graph(State(state.clone()), Path(workspace_id), Query(query)),
        )
        .await
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
    headers: HeaderMap,
    Query(query): Query<LoomLocalGraphQuery>,
) -> ApiResult<Json<crate::storage::LoomGraph>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
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
    headers: HeaderMap,
    Query(query): Query<LoomGlobalGraphQuery>,
) -> ApiResult<Json<crate::storage::LoomGraph>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

/// MT-109 C3: the account-scoped route for [`recompute_loom_block_metrics`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn recompute_loom_block_metrics_authenticated(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomMetricsRecomputeResponse>> {
    let account = loom_block_account(
        &state,
        &headers,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            recompute_loom_block_metrics(State(state.clone()), Path((workspace_id, block_id))),
        )
        .await
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

/// MT-109 C3: the account-scoped route for [`recompute_all_loom_metrics`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn recompute_all_loom_metrics_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomMetricsRecomputeResponse>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            recompute_all_loom_metrics(State(state.clone()), Path(workspace_id)),
        )
        .await
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

/// MT-109 C3: the account-scoped route for [`search_loom_blocks`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn search_loom_blocks_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomSearchQueryParams>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<crate::storage::LoomBlockSearchResult>>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            search_loom_blocks(State(state.clone()), Path(workspace_id), Query(query)),
        )
        .await
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
    headers: HeaderMap,
    Query(query): Query<LoomVisualDebugQueryParams>,
) -> ApiResult<Json<LoomVisualDebugSnapshot>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

/// MT-109 C3: the account-scoped route for [`search_loom_graph`] (ResourceBroker authorization, then the
/// unchanged handler body as the account's record user).
async fn search_loom_graph_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<LoomSearchQueryParams>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<LoomGraphSearchResult>>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            search_loom_graph(State(state.clone()), Path(workspace_id), Query(query)),
        )
        .await
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
    headers: HeaderMap,
    Query(query): Query<QuickSwitcherRecentsQueryParams>,
) -> ApiResult<Json<Vec<QuickSwitcherRecent>>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let limit = query.limit.unwrap_or(20).clamp(1, 100);
            let recents = state
                .storage
                .list_quick_switcher_recents(&workspace_id, limit)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(recents))
        })
        .await
}

async fn record_quick_switcher_recent(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<QuickSwitcherRecentInput>,
) -> ApiResult<Json<QuickSwitcherRecent>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let recent = state
                .storage
                .record_quick_switcher_recent(&workspace_id, payload)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(recent))
        })
        .await
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
async fn create_canvas_board_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Extension(authority): Extension<crate::api::authority::AuthorizedResourceContext>,
    Json(payload): Json<CreateCanvasBoardRequest>,
) -> ApiResult<Json<LoomCanvasBoard>> {
    create_canvas_board_inner(state, workspace_id, payload, authority).await
}

async fn create_canvas_board_inner(
    state: AppState,
    workspace_id: String,
    payload: CreateCanvasBoardRequest,
    authority: crate::api::authority::AuthorizedResourceContext,
) -> ApiResult<Json<LoomCanvasBoard>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let board_state = payload.board_state.unwrap_or_else(default_board_state);
    let ctx = loom_create_write_context(&authority)?;
    let new_block = NewLoomBlock {
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
    };
    let block = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
        .create_record_user_loom_bundle(&ctx, new_block, Some(board_state))
        .await
        .map_err(map_storage_error)?;
    let board = state
        .storage
        .get_canvas_board(&workspace_id, &block.block_id)
        .await
        .map_err(map_storage_error)?
        .board;

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
    headers: HeaderMap,
) -> ApiResult<Json<LoomCanvasBoardView>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::LoomBlock,
        &block_id,
        ResourceAction::Read,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let view = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope,
            database.get_record_user_canvas_board(&workspace_id, &block_id),
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(view))
}

#[derive(Debug, Deserialize)]
struct UpdateBoardViewportRequest {
    board_state: serde_json::Value,
    expected_event_ledger_event_id: String,
}

/// MT-109 C3: the viewport write requires the canvas block edit grant and runs as the record user.
async fn update_canvas_board_state(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<UpdateBoardViewportRequest>,
) -> ApiResult<Json<LoomCanvasBoard>> {
    let account = loom_account(
        &state,
        &headers,
        crate::storage::surreal::resource_authority::ResourceKind::LoomBlock,
        &block_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let board = state
                .storage
                .update_canvas_board_state(
                    &account.ctx,
                    &workspace_id,
                    &block_id,
                    payload.board_state,
                    &payload.expected_event_ledger_event_id,
                )
                .await
                .map_err(map_storage_error)?;
            Ok(Json(board))
        })
        .await
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
    headers: HeaderMap,
    Json(payload): Json<PlaceBlockRequest>,
) -> ApiResult<Json<LoomCanvasPlacement>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let board_authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.write",
        ResourceKind::LoomBlock,
        &block_id,
        ResourceAction::Update,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    let source_account = loom_block_account(
        &state,
        &headers,
        &payload.placed_block_id,
        ResourceAction::Read,
    )
    .await?;
    let ctx = loom_create_write_context(&board_authority)?;
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let receipt = state
        .surreal
        .with_record_user_scope(
            board_authority.record_user_scope,
            database.place_record_user_canvas_block(
                &ctx,
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
                    // Generic block reference (existing block placed on the canvas),
                    // not the inline text-card editor path.
                    is_text_card: false,
                    stage_provenance_key: None,
                },
                source_account.authority.record_user_scope,
            ),
        )
        .await
        .map_err(|err| {
            #[cfg(test)]
            eprintln!("loom canvas placement storage error: {err:?} ({err})");
            map_storage_error(err)
        })?;
    // Preserve the established placement response while the scoped store
    // retains the canonical creation receipt for audit and test readback.
    Ok(Json(receipt.placement))
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
    /// Present only for a Stage embed-back card. The backend validates this
    /// against the persisted body and serializes all independent clients on a
    /// transactional advisory lock before importing any content.
    #[serde(default)]
    stage_provenance: Option<LoomCanvasStageProvenance>,
}

fn validated_stage_provenance_key(payload: &CreateCanvasCardRequest) -> ApiResult<Option<String>> {
    let Some(provenance) = payload.stage_provenance.as_ref() else {
        return Ok(None);
    };
    if provenance.schema_id != LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA
        || provenance.artifact_id.trim().is_empty()
        || provenance.manifest_ref.trim().is_empty()
        || provenance.causal_action_id.trim().is_empty()
        || provenance.sha256.len() != 64
        || !provenance
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(bad_request("invalid_canvas_stage_provenance"));
    }
    let expected_title = format!("Stage capture {}", provenance.artifact_id);
    let body = payload
        .body
        .as_deref()
        .ok_or_else(|| bad_request("invalid_canvas_stage_provenance"))?;
    let body_provenance = serde_json::from_str::<LoomCanvasStageProvenance>(body)
        .map_err(|_| bad_request("invalid_canvas_stage_provenance"))?;
    if payload.title != expected_title || body_provenance != *provenance {
        return Err(bad_request("invalid_canvas_stage_provenance"));
    }
    let canonical = serde_json::to_vec(provenance).map_err(internal_error)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    Ok(Some(format!("{:x}", hasher.finalize())))
}

#[derive(Debug, Serialize)]
struct CreateCanvasCardResponse {
    block: LoomBlock,
    rich_document_id: String,
    placement: LoomCanvasPlacement,
    created_by_request: bool,
}

/// Create a free-text card: a REAL note LoomBlock (content_type=note) backed by
/// a RichDocument and bridged to knowledge, then placed on the canvas as a
/// reference. The card is authority, never a board-only copy.
async fn create_canvas_card(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<CreateCanvasCardRequest>,
) -> ApiResult<Json<CreateCanvasCardResponse>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let stage_provenance_key = validated_stage_provenance_key(&payload)?;

    if let Some(stage_provenance_key) = stage_provenance_key {
        // MT-109 C3 (Master Spec 02-system-architecture.md:2758 deny by default): the Stage-card
        // branch requires the account session with the canvas edit grant and the workspace
        // create grant before anything is written.
        let board = authorize_canvas_visual_edge_write(&state, &headers, &block_id).await?;
        let workspace =
            loom_workspace_account(&state, &headers, &workspace_id, ResourceAction::Create).await?;
        let ctx = loom_create_write_context(&board)?;
        // MT-153 AC-153-7 (Master Spec 02-system-architecture.md:2773/2776): the Stage card's
        // authority read, replay lookup and create transaction run as the account's record user in
        // the workspace Create scope (receipts carry the session principal), through the shared
        // storage wrapper so the advisory lock domain and replay key are unchanged. The storage
        // layer mints the RichDocument's protected resource + creator grant in the same transaction.
        let new_card = NewLoomCanvasStageCard {
            canvas_block_id: block_id,
            workspace_id: workspace_id.clone(),
            title: payload.title,
            markdown: payload.body.unwrap_or_default(),
            stage_provenance_key,
            stage_provenance: payload
                .stage_provenance
                .expect("validated Stage provenance is present"),
            x: payload.x,
            y: payload.y,
            w: payload.w,
            h: payload.h,
            z_index: payload.z_index.unwrap_or(0),
        };
        let card = workspace
            .run(&state, &workspace_id, async {
                ensure_workspace_exists(&state, &workspace_id).await?;
                state
                    .storage
                    .create_stage_canvas_card(&ctx, new_card)
                    .await
                    .map_err(map_storage_error)
            })
            .await?;
        return Ok(Json(CreateCanvasCardResponse {
            block: card.block,
            rich_document_id: card.rich_document_id,
            placement: card.placement,
            created_by_request: card.created_by_request,
        }));
    }

    // Master Spec 02-system-architecture §2.3.13.12.4: privileged sessions MUST NOT execute ordinary
    // protected-resource flows. The text card's RichDocument, its Loom projection and the placement
    // are all written under the authenticated account session, each through its exact grant.
    let denied = || {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    };
    let title = payload.title.trim().to_owned();
    if title.is_empty() {
        return Err(bad_request("HSK-400-LOOM-VALIDATION"));
    }
    // MT-109 C2 (diagnosis d): per-step timing of the record-user text-card chain.
    let started = std::time::Instant::now();
    let step = |name: &str| {
        tracing::info!(
            target: "handshake_core",
            elapsed_ms = started.elapsed().as_millis(),
            step = name,
            "canvas text card"
        );
    };
    let board_authority = authorize_canvas_visual_edge_write(&state, &headers, &block_id).await?;
    step("board_authorized");
    let workspace_authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.write",
        ResourceKind::Workspace,
        &workspace_id,
        ResourceAction::Create,
    )
    .await
    .map_err(|_| denied())?;
    step("workspace_authorized");
    let ctx = loom_create_write_context(&board_authority)?;
    let imported = crate::knowledge_document::import::import_snippet(
        payload.body.as_deref().unwrap_or(""),
        crate::knowledge_document::import::ImportFormat::Markdown,
    );
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let document = state
        .surreal
        .with_record_user_scope(
            workspace_authority.record_user_scope,
            crate::storage::knowledge::KnowledgeStore::create_knowledge_rich_document(
                &database,
                crate::storage::knowledge::NewKnowledgeRichDocument {
                    workspace_id: workspace_id.clone(),
                    document_id: None,
                    title,
                    schema_version: crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION
                        .to_owned(),
                    content_json: imported.document_json,
                    crdt_document_id: None,
                    crdt_snapshot_id: None,
                    promotion_receipt_event_id: None,
                    project_ref: None,
                    folder_ref: None,
                    authority_label: Some("promoted".to_owned()),
                    owner_actor_kind: Some(board_authority.actor_kind.clone()),
                    owner_actor_id: Some(board_authority.actor_id.clone()),
                },
            ),
        )
        .await
        .map_err(map_storage_error)?;
    step("document_created");
    let rich_document_id = document.rich_document_id.clone();
    let source_authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::RichDocument,
        &rich_document_id,
        ResourceAction::Read,
    )
    .await
    .map_err(|_| denied())?;
    let receipt = state
        .surreal
        .with_record_user_scope(
            board_authority.record_user_scope,
            database.place_record_user_canvas_block(
                &ctx,
                NewLoomCanvasPlacement {
                    canvas_block_id: block_id,
                    workspace_id: workspace_id.clone(),
                    placed_block_id: rich_document_id.clone(),
                    x: payload.x,
                    y: payload.y,
                    w: payload.w,
                    h: payload.h,
                    z_index: payload.z_index.unwrap_or(0),
                    group_id: None,
                    // Inline text-card editor origin: mark so the frontend restores
                    // an inline-editable text card across sessions (MT-080 FIX A).
                    is_text_card: true,
                    stage_provenance_key: None,
                },
                source_authority.record_user_scope.clone(),
            ),
        )
        .await
        .map_err(map_storage_error)?;
    step("placement_recorded");
    let block = state
        .surreal
        .with_record_user_scope(
            source_authority.record_user_scope,
            database.get_record_user_loom_block(&workspace_id, &rich_document_id),
        )
        .await
        .map_err(map_storage_error)?;
    step("block_read");

    Ok(Json(CreateCanvasCardResponse {
        block,
        rich_document_id,
        placement: receipt.placement,
        created_by_request: true,
    }))
}

#[derive(Debug, Deserialize)]
struct CompensateCanvasStageCardRequest {
    placed_block_id: String,
    stage_provenance: LoomCanvasStageProvenance,
}

#[derive(Debug, Serialize)]
struct CompensateCanvasStageCardResponse {
    removed_by_request: bool,
}

/// Compensate only a Stage-created card whose complete ownership receipt still
/// matches. The storage operation shares the create advisory-lock domain and
/// removes all owned authority/projection rows in one transaction.
async fn compensate_stage_canvas_card(
    State(state): State<AppState>,
    Path((workspace_id, block_id, placement_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(payload): Json<CompensateCanvasStageCardRequest>,
) -> ApiResult<Json<CompensateCanvasStageCardResponse>> {
    // MT-109 C3: compensation deletes rows, so it requires the account session with the canvas
    // edit grant (deny by default, Master Spec 02-system-architecture.md:2758).
    let board = authorize_canvas_visual_edge_write(&state, &headers, &block_id).await?;
    // MT-153 AC-153-7: compensation hard-deletes the card's RichDocument tuple, so it also requires
    // the workspace delete grant (LM-RLS-001 "admin deletes") and runs as the record user in that
    // scope; the RichDocument/bridge/entity deletes additionally require the RichDocument's own
    // delete grant (schema predicates) and the receipt carries the session principal.
    let workspace = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Delete,
    )
    .await?;
    let ctx = loom_create_write_context(&board)?;
    let stage_provenance_key = {
        if payload.stage_provenance.schema_id != LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA {
            return Err(bad_request("invalid_canvas_stage_provenance"));
        }
        let canonical = serde_json::to_vec(&payload.stage_provenance).map_err(internal_error)?;
        let mut hasher = Sha256::new();
        hasher.update(canonical);
        format!("{:x}", hasher.finalize())
    };
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: block_id,
        workspace_id: workspace_id.clone(),
        placement_id,
        placed_block_id: payload.placed_block_id,
        stage_provenance_key,
        stage_provenance: payload.stage_provenance,
    };
    let compensated = workspace
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            state
                .storage
                .compensate_stage_canvas_card(&ctx, receipt)
                .await
                .map_err(map_storage_error)
        })
        .await?;
    Ok(Json(CompensateCanvasStageCardResponse {
        removed_by_request: compensated.removed_by_request,
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

/// MT-109 C3: the placement's canvas is resolved under the workspace read grant, then the move or
/// resize runs as the record user under that canvas block's edit grant.
async fn update_canvas_placement(
    State(state): State<AppState>,
    Path((workspace_id, placement_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<UpdatePlacementRequest>,
) -> ApiResult<Json<LoomCanvasPlacement>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let reader =
        loom_workspace_account(&state, &headers, &workspace_id, ResourceAction::Read).await?;
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let (canvas_block_id, _) = state
        .surreal
        .with_record_user_scope(
            reader.authority.record_user_scope.clone(),
            database.get_record_user_canvas_placement_identity(&workspace_id, &placement_id),
        )
        .await
        .map_err(|_| loom_denied())?;
    let account = loom_account(
        &state,
        &headers,
        ResourceKind::LoomBlock,
        &canvas_block_id,
        ResourceAction::Update,
    )
    .await?;
    let group_id = if payload.clear_group {
        Some(None)
    } else {
        payload.group_id.map(Some)
    };
    let update = LoomCanvasPlacementUpdate {
        x: payload.x,
        y: payload.y,
        w: payload.w,
        h: payload.h,
        z_index: payload.z_index,
        group_id,
    };
    account
        .run(&state, &workspace_id, async {
            let placement = state
                .storage
                .update_canvas_placement(&account.ctx, &workspace_id, &placement_id, update)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(placement))
        })
        .await
}

async fn remove_canvas_placement(
    State(state): State<AppState>,
    Path((workspace_id, placement_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<LoomCanvasPlacementRemovalReceipt>> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};

    let workspace_authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::Workspace,
        &workspace_id,
        ResourceAction::Read,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
    let (canvas_block_id, placed_block_id) = state
        .surreal
        .with_record_user_scope(
            workspace_authority.record_user_scope,
            database.get_record_user_canvas_placement_identity(&workspace_id, &placement_id),
        )
        .await
        .map_err(|_| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "HSK-403-PROTECTED-RESOURCE",
                }),
            )
        })?;
    let board_authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.write",
        ResourceKind::LoomBlock,
        &canvas_block_id,
        ResourceAction::Update,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    // MT-109 C2: a text card places its RichDocument's same-id projection, whose protected
    // resource is the rich_document (the create path authorizes it the same way).
    let source_authority = match crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.read",
        ResourceKind::LoomBlock,
        &placed_block_id,
        ResourceAction::Read,
    )
    .await
    {
        Ok(authority) => Ok(authority),
        Err(_) => {
            crate::api::authority::authorize_request(
                &state,
                &headers,
                "fs.read",
                ResourceKind::RichDocument,
                &placed_block_id,
                ResourceAction::Read,
            )
            .await
        }
    }
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })?;
    let ctx = loom_create_write_context(&board_authority)?;
    let receipt = state
        .surreal
        .with_record_user_scope(
            board_authority.record_user_scope,
            database.remove_record_user_canvas_placement(
                &ctx,
                &workspace_id,
                &placement_id,
                source_authority.record_user_scope,
            ),
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(receipt))
}

#[derive(Debug, Deserialize)]
struct AddVisualEdgeRequest {
    from_placement_id: String,
    to_placement_id: String,
    #[serde(default)]
    label: Option<String>,
}

/// Visual-edge routes use the account authorization of the other Canvas mutation routes, with the
/// MT-111 status split: 401 = no valid account session, 403 = authenticated but not permitted on the
/// owning canvas board (constant shape, so an unknown board or edge is indistinguishable).
async fn authorize_canvas_visual_edge_write(
    state: &AppState,
    headers: &HeaderMap,
    canvas_block_id: &str,
) -> ApiResult<crate::api::authority::AuthorizedResourceContext> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    if crate::api::authority::authenticated_session_credentials(state, headers)
        .await
        .is_err()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "HSK-401-LOOM-SESSION",
            }),
        ));
    }
    crate::api::authority::authorize_request(
        state,
        headers,
        "fs.write",
        ResourceKind::LoomBlock,
        canvas_block_id,
        ResourceAction::Update,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    })
}

async fn add_canvas_visual_edge(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<AddVisualEdgeRequest>,
) -> ApiResult<Json<LoomCanvasVisualEdge>> {
    let board_authority = authorize_canvas_visual_edge_write(&state, &headers, &block_id).await?;
    let ctx = loom_create_write_context(&board_authority)?;
    // MT-109 C3: the edge row is written as the record user under the canvas edit grant.
    let account = LoomAccount {
        authority: board_authority,
        ctx,
    };
    account
        .run(&state, &workspace_id, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let edge = state
                .storage
                .add_canvas_visual_edge(
                    &account.ctx,
                    &workspace_id,
                    &block_id,
                    &payload.from_placement_id,
                    &payload.to_placement_id,
                    payload.label,
                )
                .await
                .map_err(map_storage_error)?;
            Ok(Json(edge))
        })
        .await
}

async fn remove_canvas_visual_edge(
    State(state): State<AppState>,
    Path((workspace_id, visual_edge_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    if crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .is_err()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "HSK-401-LOOM-SESSION",
            }),
        ));
    }
    let denied = || {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-PROTECTED-RESOURCE",
            }),
        )
    };
    // MT-109 C3: the owning board is resolved as the record user under the workspace read grant,
    // then the delete runs as the record user under that board's edit grant.
    let reader = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await
    .map_err(|_| denied())?;
    let canvas_block_id = reader
        .run(&state, &workspace_id, async {
            crate::storage::surreal::loom_canvas_store::canvas_visual_edge_board_id(
                &state.surreal,
                &workspace_id,
                &visual_edge_id,
            )
            .await
            .map_err(|_| denied())?
            .ok_or_else(denied)
        })
        .await?;
    let board_authority =
        authorize_canvas_visual_edge_write(&state, &headers, &canvas_block_id).await?;
    let ctx = loom_create_write_context(&board_authority)?;
    let account = LoomAccount {
        authority: board_authority,
        ctx,
    };
    account
        .run(&state, &workspace_id, async {
            state
                .storage
                .remove_canvas_visual_edge(&account.ctx, &workspace_id, &visual_edge_id)
                .await
                .map_err(map_storage_error)
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// MT-262 BlockCollectionViews handlers
// =============================================================================

#[derive(Debug, Deserialize)]
struct CreateBlockViewRequest {
    block_id: String,
    #[serde(default)]
    title: Option<String>,
    definition: BlockViewDefinition,
}

/// Create a saved view in one transaction: final `view_def` block,
/// search projection, ProjectKnowledgeIndex/EventLedger bridge, mutation
/// receipt, and recoverable Flight Recorder outbox. NO parallel store.
/// MT-109 C3: a saved view is a workspace-scoped definition created as the account's record user
/// under the workspace create grant (its receipts carry the session principal).
async fn create_block_view_authenticated(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateBlockViewRequest>,
) -> ApiResult<Json<BlockViewRecord>> {
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Create,
    )
    .await?;
    let ctx = account.ctx.clone();
    let loom_workspace = workspace_id.clone();
    account
        .run(
            &state,
            &loom_workspace,
            create_block_view_inner(state.clone(), workspace_id, payload, ctx),
        )
        .await
}

#[cfg(test)]
async fn create_block_view(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateBlockViewRequest>,
) -> ApiResult<Json<BlockViewRecord>> {
    let ctx = block_view_write_context(&headers);
    create_block_view_inner(state, workspace_id, payload, ctx).await
}

async fn create_block_view_inner(
    state: AppState,
    workspace_id: String,
    payload: CreateBlockViewRequest,
    ctx: WriteContext,
) -> ApiResult<Json<BlockViewRecord>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let block_id = payload.block_id;

    let record = state
        .storage
        .create_block_view(
            &ctx,
            &workspace_id,
            &block_id,
            payload.title,
            payload.definition,
        )
        .await
        .map_err(map_storage_error)?;
    let publication_event_id = record.publication_event_id.ok_or_else(|| {
        internal_error("created block view omitted its publication event identity")
    })?;
    reconcile_block_view_events(&state, Some(&workspace_id), Some(publication_event_id)).await?;

    Ok(Json(record))
}

async fn get_block_view(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<BlockViewRecord>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let record = state
                .storage
                .get_block_view(&workspace_id, &block_id)
                .await
                .map_err(map_storage_error)?;
            Ok(Json(record))
        })
        .await
}

#[derive(Debug, Deserialize)]
struct UpdateBlockViewRequest {
    definition: BlockViewDefinition,
}

/// Persist a new definition for a saved view (e.g. a table header click that
/// re-sorts the view stores the new sort in the durable store, not localStorage).
async fn update_block_view(
    State(state): State<AppState>,
    Path((workspace_id, block_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateBlockViewRequest>,
) -> ApiResult<Json<BlockViewRecord>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Update,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
            ensure_workspace_exists(&state, &workspace_id).await?;
            let ctx = account.ctx.clone();
            let record = state
                .storage
                .update_block_view_definition(&ctx, &workspace_id, &block_id, payload.definition)
                .await
                .map_err(map_storage_error)?;
            let publication_event_id = record.publication_event_id.ok_or_else(|| {
                internal_error("updated block view omitted its publication event identity")
            })?;
            reconcile_block_view_events(&state, Some(&workspace_id), Some(publication_event_id))
                .await?;

            Ok(Json(record))
        })
        .await
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
    headers: HeaderMap,
    Json(payload): Json<BlockViewResultsRequest>,
) -> ApiResult<Json<BlockViewResults>> {
    // MT-109 C3: authorized through the ResourceBroker and run as the account's record user.
    let account = loom_workspace_account(
        &state,
        &headers,
        &workspace_id,
        crate::storage::surreal::resource_authority::ResourceAction::Read,
    )
    .await?;
    let loom_workspace = workspace_id.clone();
    account
        .run(&state, &loom_workspace, async {
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
        })
        .await
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    #[cfg(feature = "os-keychain")]
    use crate::api::MountedRequestExt;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::{duckdb::DuckDbFlightRecorder, EventFilter};
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{tests::embedded_test_backend, Database, NewWorkspace};
    #[cfg(feature = "os-keychain")]
    use axum::{
        body::{to_bytes, Body},
        http::{HeaderMap, Request},
        Router,
    };
    use once_cell::sync::Lazy;
    #[cfg(feature = "os-keychain")]
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use surrealdb::types::{RecordId, SurrealValue};
    use tempfile::TempDir;

    #[derive(Clone, SurrealValue)]
    struct LoomTestBlockBinding {
        block: RecordId,
    }

    #[derive(Clone, SurrealValue)]
    struct LoomTestEventBinding {
        workspace: RecordId,
        event_id: String,
    }

    #[derive(Clone, SurrealValue)]
    struct LoomTestCorruptionBinding {
        workspace: RecordId,
        event_id: String,
        hash: String,
    }

    #[derive(Clone, SurrealValue)]
    struct LoomTestEventListBinding {
        event_ids: Vec<String>,
    }

    #[derive(Clone, SurrealValue)]
    struct LoomStandaloneMutationBinding {
        block: RecordId,
        workspace: RecordId,
        document: RecordId,
    }

    #[derive(SurrealValue)]
    struct LoomTestCountRow {
        count: i64,
    }

    // `'static` is required by `with_data_operation`, whose closure is moved into a boxed future
    // (WP-KERNEL-012 MT-144).
    async fn loom_test_count<B: SurrealValue + Send + 'static>(
        state: &AppState,
        statement: &'static str,
        bindings: B,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let row: Option<LoomTestCountRow> = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_first(statement, bindings).await })
            })
            .await?;
        Ok(row.map_or(0, |row| row.count))
    }

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

    async fn setup_state(
    ) -> Result<(AppState, crate::storage::tests::EmbeddedTestBackend), Box<dyn std::error::Error>>
    {
        let backend = embedded_test_backend().await?;
        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        Ok((state, backend))
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

    #[cfg(feature = "os-keychain")]
    struct LoomCreateBinding {
        _lock: std::sync::MutexGuard<'static, ()>,
        _directory: TempDir,
        previous: Option<std::ffi::OsString>,
        channel: String,
    }

    #[cfg(feature = "os-keychain")]
    impl LoomCreateBinding {
        fn new() -> Self {
            let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let directory =
                tempfile::tempdir_in(crate::storage::tests::test_store_root().unwrap()).unwrap();
            let path = directory.path().join("loom-create-binding.json");
            let channel = "c4".repeat(32);
            std::fs::write(
                &path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(&channel))
                    .unwrap(),
            )
            .unwrap();
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Self {
                _lock: lock,
                _directory: directory,
                previous,
                channel,
            }
        }
    }

    #[cfg(feature = "os-keychain")]
    impl Drop for LoomCreateBinding {
        fn drop(&mut self) {
            match &self.previous {
                Some(previous) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous),
                None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
            }
        }
    }

    #[cfg(feature = "os-keychain")]
    async fn loom_create_request(
        router: &Router,
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        (
            status,
            if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&bytes).unwrap()
            },
        )
    }

    #[cfg(feature = "os-keychain")]
    async fn loom_bundle_snapshot(state: &AppState) -> Value {
        let mut snapshot = state
            .surreal
            .test_admin_query(
                "RETURN {blocks: array::len(SELECT id FROM loom_blocks), search: array::len(SELECT id FROM loom_block_search_index), resources: array::len(SELECT id FROM protected_resources), grants: array::len(SELECT id FROM resource_grants), entities: array::len(SELECT id FROM knowledge_entities), bridges: array::len(SELECT id FROM loom_block_knowledge_bridge), ledger: array::len(SELECT id FROM kernel_event_ledger), boards: array::len(SELECT id FROM loom_canvas_boards), placements: array::len(SELECT id FROM loom_canvas_placements)};".to_owned(),
            )
            .await
            .unwrap();
        snapshot.take::<Option<Value>>(0).unwrap().unwrap()
    }

    #[cfg(feature = "os-keychain")]
    /// Owner account session (setup -> login -> session exchange) plus an owned workspace created
    /// through the product route; returns the router, the account headers and the workspace id.
    async fn owned_loom_session(
        state: &AppState,
        binding: &LoomCreateBinding,
    ) -> (axum::Router, HeaderMap, String) {
        let router = crate::api::authority::routes(state.clone())
            .merge(crate::api::workspaces::routes(state.clone()))
            .merge(routes(state.clone()));
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        let credentials = serde_json::json!({
            "account_name": "Loom journal owner",
            "password": "loom journal owner runtime proof password",
        });
        let (status, _) = loom_create_request(
            &router,
            "POST",
            "/authority/setup",
            &headers,
            credentials.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owner setup");
        let (status, credential) =
            loom_create_request(&router, "POST", "/authority/login", &headers, credentials).await;
        assert_eq!(status, StatusCode::OK, "owner login");
        let (status, session) = loom_create_request(
            &router,
            "POST",
            "/authority/session",
            &headers,
            serde_json::json!({
                "account_id": credential["account_id"],
                "principal_id": credential["principal_id"],
                "access_space_id": credential["access_space_id"],
                "authentication_token": credential["token"],
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owner session exchange");
        headers.insert(
            "x-hsk-session-token",
            session["session_token"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert(
            "x-hsk-actor-id",
            session["principal_id"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert("x-hsk-actor-kind", "operator".parse().unwrap());
        headers.insert("x-hsk-kernel-task-run-id", "owned-loom".parse().unwrap());
        headers.insert("x-hsk-session-run-id", "owned-loom".parse().unwrap());
        let (status, workspace) = loom_create_request(
            &router,
            "POST",
            "/workspaces",
            &headers,
            serde_json::json!({"name": "Owned journal workspace"}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "owned workspace: {workspace}");
        let workspace_id = workspace["id"].as_str().unwrap().to_owned();
        (router, headers, workspace_id)
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mounted_record_user_loom_creates_are_atomic_and_denied_writes_leave_no_rows() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let router = crate::api::authority::routes(state.clone())
            .merge(crate::api::workspaces::routes(state.clone()))
            .merge(routes(state.clone()));
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        let credentials = serde_json::json!({
            "account_name": "Loom creator",
            "password": "loom creator runtime proof password",
        });
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                "/authority/setup",
                &headers,
                credentials.clone()
            )
            .await
            .0,
            StatusCode::OK
        );
        let (status, credential) = loom_create_request(
            &router,
            "POST",
            "/authority/login",
            &headers,
            credentials.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "real account login failed");
        let (status, session) = loom_create_request(
            &router,
            "POST",
            "/authority/session",
            &headers,
            serde_json::json!({
                "account_id": credential["account_id"],
                "principal_id": credential["principal_id"],
                "access_space_id": credential["access_space_id"],
                "authentication_token": credential["token"],
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "real session exchange failed");
        headers.insert(
            "x-hsk-session-token",
            session["session_token"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert(
            "x-hsk-actor-id",
            session["principal_id"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert("x-hsk-actor-kind", "operator".parse().unwrap());
        headers.insert(
            "x-hsk-kernel-task-run-id",
            "mounted-loom-create".parse().unwrap(),
        );
        headers.insert(
            "x-hsk-session-run-id",
            "mounted-loom-create".parse().unwrap(),
        );
        let (status, workspace) = loom_create_request(
            &router,
            "POST",
            "/workspaces",
            &headers,
            serde_json::json!({"name": "Owned Loom workspace"}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "owned workspace: {workspace}");
        let workspace_id = workspace["id"].as_str().unwrap();
        let (status, note) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &headers,
            serde_json::json!({"content_type": "note", "title": "Mounted owned note"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owned note create: {note}");
        let (status, canvas) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
            &headers,
            serde_json::json!({"title": "Mounted owned canvas"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owned Canvas create: {canvas}");
        let note_id = note["block_id"].as_str().unwrap();
        let canvas_id = canvas["block_id"].as_str().unwrap();
        let source_create_authority = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            crate::storage::surreal::resource_authority::ResourceKind::Workspace,
            workspace_id,
            crate::storage::surreal::resource_authority::ResourceAction::Create,
        )
        .await
        .unwrap();
        let source_database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
        let source_document = state
            .surreal
            .with_record_user_scope(
                source_create_authority.record_user_scope,
                crate::storage::knowledge::KnowledgeStore::create_knowledge_rich_document(
                    &source_database,
                    crate::storage::knowledge::NewKnowledgeRichDocument {
                        workspace_id: workspace_id.to_owned(),
                        document_id: None,
                        title: "Mounted owned source document".to_owned(),
                        schema_version:
                            crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION
                                .to_owned(),
                        content_json: serde_json::json!({
                            "type": "doc",
                            "content": [{
                                "type": "paragraph",
                                "content": [{"type": "text", "text": "Mounted source content"}]
                            }]
                        }),
                        crdt_document_id: None,
                        crdt_snapshot_id: None,
                        promotion_receipt_event_id: None,
                        project_ref: None,
                        folder_ref: None,
                        authority_label: Some("promoted".to_owned()),
                        owner_actor_kind: Some("operator".to_owned()),
                        owner_actor_id: session["principal_id"].as_str().map(str::to_owned),
                    },
                ),
            )
            .await
            .unwrap();
        let source_block_id = source_document.rich_document_id.clone();
        let (status, mounted_note) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/blocks/{note_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated mounted source read: {mounted_note}"
        );
        assert_eq!(mounted_note["block_id"], note_id);
        let (status, mounted_canvas) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated mounted Canvas read: {mounted_canvas}"
        );
        assert_eq!(mounted_canvas["board"]["block_id"], canvas_id);

        let (status, bridge) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/blocks/{note_id}/knowledge"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated bridge read: {bridge}"
        );
        assert_eq!(bridge["block_id"], note_id);
        assert_eq!(bridge["workspace_id"], workspace_id);
        assert!(bridge["entity_id"].is_string());
        assert!(bridge["index_event_id"].is_string());

        let (status, transclusion) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/blocks/{note_id}/transclusion"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated source transclusion read: {transclusion}"
        );
        assert_eq!(transclusion["block_id"], note_id);
        assert_eq!(transclusion["workspace_id"], workspace_id);
        assert!(transclusion["source_document_id"].is_null());
        assert!(transclusion["source_doc_version"].is_null());
        assert!(transclusion["content_json"].is_null());
        assert_eq!(transclusion["resolved"], false);
        assert_eq!(
            transclusion["unresolved_reason"],
            "source_rich_document_missing"
        );

        let (status, sourced_transclusion) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/blocks/{source_block_id}/transclusion"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated rich-document transclusion read: {sourced_transclusion}"
        );
        assert_eq!(sourced_transclusion["block_id"], source_block_id);
        assert_eq!(sourced_transclusion["workspace_id"], workspace_id);
        assert_eq!(
            sourced_transclusion["source_document_id"],
            Value::String(source_document.rich_document_id.clone())
        );
        assert_eq!(sourced_transclusion["source_doc_version"], 1);
        assert_eq!(
            sourced_transclusion["content_json"],
            source_document.content_json
        );
        assert_eq!(sourced_transclusion["resolved"], true);
        assert!(sourced_transclusion["unresolved_reason"].is_null());

        let (status, patched_canvas) = loom_create_request(
            &router,
            "PATCH",
            &format!("/workspaces/{workspace_id}/loom/blocks/{canvas_id}"),
            &headers,
            serde_json::json!({"title": "Mounted Canvas patched through authenticated route"}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated standalone Canvas patch: {patched_canvas}"
        );
        assert_eq!(
            patched_canvas["title"],
            "Mounted Canvas patched through authenticated route"
        );
        let (status, disposable) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &headers,
            serde_json::json!({"content_type": "note", "title": "Mounted authenticated delete proof"}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "disposable standalone note: {disposable}"
        );
        let disposable_id = disposable["block_id"]
            .as_str()
            .expect("disposable Loom block id")
            .to_owned();
        let (status, deleted) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{workspace_id}/loom/blocks/{disposable_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated standalone delete: {deleted}"
        );
        assert_eq!(deleted, serde_json::json!({"status": "deleted"}));
        let mut deleted_row = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_blocks', $block_id))[0];".to_owned(),
                serde_json::json!({"block_id": disposable_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            deleted_row.take::<Option<Value>>(0).unwrap(),
            None,
            "authenticated delete removes the exact standalone Loom row"
        );
        let placement_authority = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            crate::storage::surreal::resource_authority::ResourceKind::LoomBlock,
            canvas_id,
            crate::storage::surreal::resource_authority::ResourceAction::Update,
        )
        .await
        .unwrap();
        let board_update_grant = placement_authority
            .record_user_scope
            .grant_id
            .clone()
            .unwrap();
        let mut board_capabilities = state
            .surreal
            .test_admin_query_bound(
                "SELECT VALUE capability_ids FROM type::record('resource_grants', $grant_id);"
                    .to_owned(),
                serde_json::json!({"grant_id": board_update_grant.clone()}),
            )
            .await
            .unwrap();
        let mut board_capability_rows = board_capabilities.take::<Vec<Vec<String>>>(0).unwrap();
        let board_capabilities = board_capability_rows.remove(0);
        let board_without_read = board_capabilities
            .iter()
            .filter(|capability| capability.as_str() != "fs.read")
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            board_without_read
                .iter()
                .any(|capability| capability == "fs.write"),
            "the exact board grant must retain Update/fs.write"
        );
        assert!(board_without_read.len() < board_capabilities.len());
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": board_update_grant.clone(), "capabilities": board_without_read.clone()}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        assert_eq!(
            loom_create_request(
                &router,
                "GET",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
                &headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            ),
            "board Update/fs.write alone must not read board content"
        );
        let (status, placement) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
            &headers,
            serde_json::json!({
                "placed_block_id": note_id,
                "x": 24.0,
                "y": 36.0,
                "w": 320.0,
                "h": 180.0,
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated mounted Canvas placement: {placement}"
        );
        let placement_id = placement["placement_id"]
            .as_str()
            .expect("mounted Canvas placement id")
            .to_owned();
        let mut placement_event = state
            .surreal
            .test_admin_query_bound(
                "SELECT event_id, actor_id, record::id(authority_session_id) AS authority_session_id, record::id(authority_resource_id) AS authority_resource_id, authority_capability_id, authority_action, payload FROM kernel_event_ledger WHERE payload.placement_id = $placement_id AND payload.op = 'create';".to_owned(),
                serde_json::json!({"placement_id": placement_id}),
            )
            .await
            .unwrap();
        let placement_events = placement_event.take::<Vec<Value>>(0).unwrap();
        assert_eq!(
            placement_events.len(),
            1,
            "placement must have one canonical receipt"
        );
        let placement_event = &placement_events[0];
        let placement_event_id = placement_event["event_id"]
            .as_str()
            .expect("placement receipt event id")
            .to_owned();
        assert_eq!(placement_event["actor_id"], placement_authority.actor_id);
        assert_eq!(
            placement_event["authority_session_id"],
            placement_authority.session_id
        );
        assert_eq!(
            placement_event["authority_resource_id"],
            placement_authority.resource_id
        );
        assert_eq!(placement_event["authority_capability_id"], "fs.write");
        assert_eq!(placement_event["authority_action"], "update");
        assert_eq!(placement_event["payload"]["canvas_block_id"], canvas_id);
        assert_eq!(placement_event["payload"]["placed_block_id"], note_id);

        let mut second_headers = HeaderMap::new();
        second_headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        let (status, second_credential) = loom_create_request(
            &router,
            "POST",
            "/authority/login",
            &second_headers,
            credentials,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "second real account login failed");
        let (status, second_session) = loom_create_request(
            &router,
            "POST",
            "/authority/session",
            &second_headers,
            serde_json::json!({
                "account_id": second_credential["account_id"],
                "principal_id": second_credential["principal_id"],
                "access_space_id": second_credential["access_space_id"],
                "authentication_token": second_credential["token"],
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "second real session exchange failed"
        );
        assert_ne!(second_session["session_id"], session["session_id"]);
        second_headers.insert(
            "x-hsk-session-token",
            second_session["session_token"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        );
        second_headers.insert(
            "x-hsk-actor-id",
            second_session["principal_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        );
        second_headers.insert("x-hsk-actor-kind", "operator".parse().unwrap());
        second_headers.insert(
            "x-hsk-kernel-task-run-id",
            "mounted-loom-receipt-peer".parse().unwrap(),
        );
        second_headers.insert(
            "x-hsk-session-run-id",
            "mounted-loom-receipt-peer".parse().unwrap(),
        );
        let second_authority = crate::api::authority::authorize_request(
            &state,
            &second_headers,
            "fs.write",
            crate::storage::surreal::resource_authority::ResourceKind::LoomBlock,
            canvas_id,
            crate::storage::surreal::resource_authority::ResourceAction::Update,
        )
        .await
        .expect("second session receives the same exact board update grant");
        crate::api::authority::authorize_request(
            &state,
            &second_headers,
            "fs.read",
            crate::storage::surreal::resource_authority::ResourceKind::LoomBlock,
            note_id,
            crate::storage::surreal::resource_authority::ResourceAction::Read,
        )
        .await
        .expect("second session retains the placed source read grant");
        let second_scope = second_authority.record_user_scope;
        let peer_event_id = placement_event_id.clone();
        let peer_workspace_id = workspace_id.to_owned();
        let peer_receipt_count: Option<LoomTestCountRow> = state
            .surreal
            .with_record_user_scope(
                second_scope,
                state.surreal.with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_first(
                                "SELECT count() AS count FROM kernel_event_ledger WHERE event_id = $event_id AND wsids CONTAINS record::id($workspace) GROUP ALL;",
                                LoomTestEventBinding {
                                    workspace: RecordId::new("workspaces", peer_workspace_id),
                                    event_id: peer_event_id,
                                },
                            )
                            .await
                    })
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            peer_receipt_count.map_or(0, |row| row.count),
            0,
            "a distinct session must not read the creator session's write-only Canvas receipt"
        );
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                "/authority/logout",
                &second_headers,
                Value::Null,
            )
            .await
            .0,
            StatusCode::OK,
            "second receipt-peer session cleanup"
        );
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": board_update_grant.clone(), "capabilities": board_capabilities.clone()}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let (status, foreign_workspace) = loom_create_request(
            &router,
            "POST",
            "/workspaces",
            &headers,
            serde_json::json!({"name": "Mounted foreign Loom workspace"}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "foreign workspace fixture: {foreign_workspace}"
        );
        let foreign_workspace_id = foreign_workspace["id"].as_str().unwrap().to_owned();
        let (status, foreign_note) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{foreign_workspace_id}/loom/blocks"),
            &headers,
            serde_json::json!({"content_type": "note", "title": "Mounted foreign note"}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "foreign note fixture: {foreign_note}"
        );
        let (status, foreign_canvas) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{foreign_workspace_id}/loom/canvas-boards"),
            &headers,
            serde_json::json!({"title": "Mounted foreign canvas"}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "foreign Canvas fixture: {foreign_canvas}"
        );
        let foreign_note_id = foreign_note["block_id"].as_str().unwrap();
        let foreign_canvas_id = foreign_canvas["block_id"].as_str().unwrap();

        let mut standalone_before = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_blocks', $block_id))[0];".to_owned(),
                serde_json::json!({"block_id": canvas_id}),
            )
            .await
            .unwrap();
        let standalone_before = standalone_before.take::<Option<Value>>(0).unwrap();
        let mutation = LoomStandaloneMutationBinding {
            block: RecordId::new("loom_blocks", canvas_id.to_owned()),
            workspace: RecordId::new("workspaces", foreign_workspace_id.clone()),
            document: RecordId::new("knowledge_rich_documents", note_id.to_owned()),
        };
        let mut identity_outcomes = Vec::new();
        for statement in [
            "UPDATE $block SET workspace_id = $workspace RETURN AFTER;",
            "UPDATE $block SET content_type = 'note' RETURN AFTER;",
            "UPDATE $block SET source_rich_document_id = $document RETURN AFTER;",
        ] {
            let scope = placement_authority.record_user_scope.clone();
            let bindings = mutation.clone();
            let rejection = state
                .surreal
                .with_record_user_scope(
                    scope,
                    state.surreal.with_data_operation(move |database| {
                        Box::pin(async move {
                            database.execute_returning(statement, bindings).await?;
                            Ok(())
                        })
                    }),
                )
                .await;
            identity_outcomes.push(format!(
                "{statement} => {:?}",
                rejection.map_err(|error| error.to_string())
            ));
        }
        let mut standalone_after = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_blocks', $block_id))[0];".to_owned(),
                serde_json::json!({"block_id": canvas_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            standalone_after.take::<Option<Value>>(0).unwrap(),
            standalone_before,
            "source-free standalone workspace, content type, and source identity must remain immutable"
        );
        // MT-109 C3 probe: each identity mutation must report the canonical denial (not a silent
        // no-op); the outcomes name the statement that does not.
        assert!(
            identity_outcomes
                .iter()
                .all(|outcome| outcome.contains("Err(") && outcome.contains("HSK-403-PROTECTED-RESOURCE")),
            "standalone identity mutations must report the canonical denial: {identity_outcomes:#?}"
        );
        let invalid_placement = |suffix: &str| format!("LCP-{suffix:0>32}");
        for (placement_id, canvas_block_id, placement_workspace_id, placed_block_id, label) in [
            (
                invalid_placement("1"),
                "missing-canvas-board",
                workspace_id,
                note_id,
                "nonexistent board",
            ),
            (
                invalid_placement("2"),
                canvas_id,
                workspace_id,
                "missing-loom-block",
                "nonexistent source",
            ),
            (
                invalid_placement("3"),
                foreign_canvas_id,
                workspace_id,
                foreign_note_id,
                "foreign board and source",
            ),
            (
                invalid_placement("4"),
                canvas_id,
                foreign_workspace_id.as_str(),
                foreign_note_id,
                "foreign workspace",
            ),
        ] {
            let before = loom_bundle_snapshot(&state).await;
            let error = state
                .surreal
                .test_admin_query_bound(
                    "CREATE type::record('loom_canvas_placements', $placement_id) SET placement_id = $placement_id, canvas_block_id = type::record('loom_canvas_boards', $canvas_block_id), workspace_id = type::record('workspaces', $workspace_id), placed_block_id = type::record('loom_blocks', $placed_block_id), x = 1.0, y = 1.0, w = 1.0, h = 1.0 RETURN NONE;".to_owned(),
                    serde_json::json!({
                        "placement_id": placement_id,
                        "canvas_block_id": canvas_block_id,
                        "workspace_id": placement_workspace_id,
                        "placed_block_id": placed_block_id,
                    }),
                )
                .await
                .expect_err("invalid placement relation must be rejected by the integrity event");
            assert!(
                error.to_string().contains("HSK-403-PROTECTED-RESOURCE"),
                "{label} must fail at the placement integrity event: {error}"
            );
            assert_eq!(
                loom_bundle_snapshot(&state).await,
                before,
                "{label} rejection must leave all bundle rows unchanged"
            );
            let mut rejected_row = state
                .surreal
                .test_admin_query_bound(
                    "RETURN (SELECT * FROM type::record('loom_canvas_placements', $placement_id))[0];".to_owned(),
                    serde_json::json!({"placement_id": placement_id}),
                )
                .await
                .unwrap();
            assert_eq!(
                rejected_row.take::<Option<Value>>(0).unwrap(),
                None,
                "{label} rejection must leave no placement row"
            );
        }
        let mut placement_before = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_canvas_placements', $placement_id))[0];"
                    .to_owned(),
                serde_json::json!({"placement_id": placement_id}),
            )
            .await
            .unwrap();
        let placement_before = placement_before.take::<Option<Value>>(0).unwrap();
        assert!(
            placement_before.is_some(),
            "the positive placement must exist before the update rejection probe"
        );
        let update_error = state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('loom_canvas_placements', $placement_id) SET workspace_id = type::record('workspaces', $workspace_id) RETURN NONE;".to_owned(),
                serde_json::json!({
                    "placement_id": placement_id,
                    "workspace_id": foreign_workspace_id,
                }),
            )
            .await
            .expect_err("cross-workspace placement update must be rejected by the integrity event");
        assert!(
            update_error
                .to_string()
                .contains("HSK-403-PROTECTED-RESOURCE"),
            "cross-workspace update must fail at the placement integrity event: {update_error}"
        );
        let mut placement_after = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_canvas_placements', $placement_id))[0];"
                    .to_owned(),
                serde_json::json!({"placement_id": placement_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            placement_after.take::<Option<Value>>(0).unwrap(),
            placement_before,
            "rejected placement update must preserve the canonical row"
        );
        let (status, placed_board) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "read board after placement: {placed_board}"
        );
        assert!(
            placed_board["placements"]
                .as_array()
                .is_some_and(|placements| placements.iter().any(|row| {
                    row["placement_id"] == placement_id && row["placed_block_id"] == note_id
                })),
            "authenticated board read must expose the exact placed source"
        );
        let mut source_before_remove = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_blocks', $block_id))[0];".to_owned(),
                serde_json::json!({"block_id": note_id}),
            )
            .await
            .unwrap();
        let source_before_remove = source_before_remove.take::<Option<Value>>(0).unwrap();
        assert!(
            source_before_remove.is_some(),
            "the placed source must exist before removal"
        );
        let mut canonical = state
            .surreal
            .test_admin_query(format!(
                "RETURN {{blocks: array::len(SELECT id FROM loom_blocks WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(id) IN ['{note_id}', '{canvas_id}']), search: array::len(SELECT id FROM loom_block_search_index WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(block_id) IN ['{note_id}', '{canvas_id}']), bridges: array::len(SELECT id FROM loom_block_knowledge_bridge WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(block_id) IN ['{note_id}', '{canvas_id}']), bridge_receipts: array::len(SELECT id FROM kernel_event_ledger WHERE id IN (SELECT VALUE index_event_id FROM loom_block_knowledge_bridge WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(block_id) IN ['{note_id}', '{canvas_id}'])), entities: array::len(SELECT id FROM knowledge_entities WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND entity_key IN ['{note_id}', '{canvas_id}']), owned_resources: array::len(SELECT id FROM protected_resources WHERE resource_kind = 'loom_block' AND owner_account_id = type::record('local_accounts', '{}') AND external_resource_id IN ['{note_id}', '{canvas_id}']), creator_grants: array::len(SELECT id FROM resource_grants WHERE resource_id.resource_kind = 'loom_block' AND resource_id.external_resource_id IN ['{note_id}', '{canvas_id}'] AND principal_id = type::record('principals', '{}') AND status = 'active'), boards: array::len(SELECT id FROM loom_canvas_boards WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(block_id) = '{canvas_id}'), board_receipts: array::len(SELECT id FROM kernel_event_ledger WHERE id = (SELECT VALUE event_ledger_event_id FROM loom_canvas_boards WHERE workspace_id = type::record('workspaces', '{workspace_id}') AND record::id(block_id) = '{canvas_id}')[0])}};",
                credential["account_id"].as_str().unwrap(),
                session["principal_id"].as_str().unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(
            canonical.take::<Option<Value>>(0).unwrap(),
            Some(serde_json::json!({
                "blocks": 2,
                "search": 2,
                "bridges": 2,
                "bridge_receipts": 2,
                "entities": 2,
                "owned_resources": 2,
                "creator_grants": 2,
                "boards": 1,
                "board_receipts": 1,
            })),
            "mounted creates must persist complete owned lineage"
        );
        let mut denied_headers = headers.clone();
        denied_headers.remove("x-hsk-session-token");
        let before = loom_bundle_snapshot(&state).await;
        assert_eq!(
            loom_create_request(
                &router,
                "GET",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
                &denied_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
                &denied_headers,
                serde_json::json!({
                    "placed_block_id": note_id,
                    "x": 24.0,
                    "y": 36.0,
                    "w": 320.0,
                    "h": 180.0,
                }),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "DELETE",
                &format!("/workspaces/{workspace_id}/loom/canvas-placements/{placement_id}"),
                &denied_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "GET",
                &format!("/workspaces/{workspace_id}/loom/blocks/{note_id}"),
                &denied_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "GET",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
                &denied_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &denied_headers,
                serde_json::json!({"content_type": "note", "title": "Denied residue"}),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_bundle_snapshot(&state).await,
            before,
            "denied mounted write must leave no bundle rows"
        );

        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let create_grant = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::Workspace,
            workspace_id,
            ResourceAction::Create,
        )
        .await
        .unwrap()
        .record_user_scope
        .grant_id
        .unwrap();
        let before = loom_bundle_snapshot(&state).await;
        state.surreal.revoke_grant(&create_grant).await.unwrap();
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &headers,
                serde_json::json!({"content_type": "note", "title": "Revoked grant residue"}),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        state
            .surreal
            .test_admin_query(format!(
                "UPDATE type::record('resource_grants', '{create_grant}') SET status = 'active', revoked_at = NONE, grant_version -= 1, policy_version -= 1 RETURN NONE;"
            ))
            .await
            .unwrap()
            .check()
            .unwrap();

        let mut capabilities = state
            .surreal
            .test_admin_query(format!(
                "SELECT VALUE capability_ids FROM type::record('resource_grants', '{create_grant}');"
            ))
            .await
            .unwrap();
        let mut capability_rows = capabilities.take::<Vec<Vec<String>>>(0).unwrap();
        let capabilities = capability_rows.remove(0);
        let narrowed = capabilities
            .iter()
            .filter(|capability| capability.as_str() != "fs.write")
            .cloned()
            .collect::<Vec<_>>();
        assert!(narrowed.len() < capabilities.len());
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": create_grant.clone(), "capabilities": narrowed}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let mut original_note = state
            .surreal
            .test_admin_query(format!(
                "RETURN (SELECT * FROM type::record('loom_blocks', '{note_id}'))[0];"
            ))
            .await
            .unwrap();
        let original_note = original_note.take::<Option<Value>>(0).unwrap();
        assert!(
            original_note.is_some(),
            "created note must exist before adoption attempt"
        );
        let before = loom_bundle_snapshot(&state).await;
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &headers,
                serde_json::json!({"content_type": "note", "title": "Narrowed grant residue"}),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": create_grant.clone(), "capabilities": capabilities}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        let foreign_capabilities = [
            "fs.read",
            "fs.write",
            "fr.read",
            "fr.ingest.runtime_chat",
            "fr.ingest.native_editor",
            "memory.read",
            "memory.propose",
        ]
        .map(str::to_owned)
        .to_vec();
        let foreign_binding_hash = hex::encode(Sha256::digest(binding.channel.as_bytes()));
        let foreign = state
            .surreal
            .provision_principal(
                "loom-create-foreign",
                "loom-create-foreign",
                "human_account",
                "loom-create-foreign",
                "Operator",
                &foreign_capabilities,
                "loom-create-foreign",
                Some(&foreign_binding_hash),
                Duration::from_secs(3600),
            )
            .await
            .unwrap();
        let mut foreign_headers = HeaderMap::new();
        foreign_headers.insert(
            "x-hsk-session-token",
            foreign.session.token.parse().unwrap(),
        );
        foreign_headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );

        for path in [
            format!("/workspaces/{workspace_id}/loom/blocks/{note_id}/knowledge"),
            format!("/workspaces/{workspace_id}/loom/blocks/{note_id}/transclusion"),
            format!("/workspaces/{workspace_id}/loom/blocks/{canvas_id}"),
        ] {
            let method = if path.ends_with(&canvas_id) {
                "PATCH"
            } else {
                "GET"
            };
            let body = if method == "PATCH" {
                serde_json::json!({"title": "foreign mutation"})
            } else {
                Value::Null
            };
            assert_eq!(
                loom_create_request(&router, method, &path, &foreign_headers, body).await,
                (
                    StatusCode::FORBIDDEN,
                    serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
                ),
                "foreign account must not access or mutate authenticated Loom authority routes"
            );
        }
        let before = loom_bundle_snapshot(&state).await;
        assert_eq!(
            loom_create_request(
                &router,
                "GET",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
                &foreign_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "DELETE",
                &format!("/workspaces/{workspace_id}/loom/canvas-placements/{placement_id}"),
                &foreign_headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
                &foreign_headers,
                serde_json::json!({
                    "placed_block_id": note_id,
                    "x": 24.0,
                    "y": 36.0,
                    "w": 320.0,
                    "h": 180.0,
                }),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &foreign_headers,
                serde_json::json!({"content_type": "note", "title": "Foreign workspace residue"}),
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                "/authority/logout",
                &foreign_headers,
                Value::Null
            )
            .await
            .0,
            StatusCode::OK
        );

        let before = loom_bundle_snapshot(&state).await;
        assert_eq!(
            loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{workspace_id}/loom/blocks"),
                &headers,
                serde_json::json!({"block_id": note_id, "content_type": "note", "title": "Rejected adoption"}),
            )
            .await,
            (StatusCode::FORBIDDEN, serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"}))
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        let mut retained_note = state
            .surreal
            .test_admin_query(format!(
                "RETURN (SELECT * FROM type::record('loom_blocks', '{note_id}'))[0];"
            ))
            .await
            .unwrap();
        assert_eq!(
            retained_note.take::<Option<Value>>(0).unwrap(),
            original_note,
            "rejected supplied ID must preserve every existing Loom row field"
        );

        let source_read_grant = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.read",
            ResourceKind::LoomBlock,
            note_id,
            ResourceAction::Read,
        )
        .await
        .unwrap()
        .record_user_scope
        .grant_id
        .unwrap();
        let before = loom_bundle_snapshot(&state).await;
        state
            .surreal
            .revoke_grant(&source_read_grant)
            .await
            .unwrap();
        assert_eq!(
            loom_create_request(
                &router,
                "DELETE",
                &format!("/workspaces/{workspace_id}/loom/canvas-placements/{placement_id}"),
                &headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET status = 'active', revoked_at = NONE, grant_version -= 1, policy_version -= 1 RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": source_read_grant.clone()}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        let before = loom_bundle_snapshot(&state).await;
        state
            .surreal
            .revoke_grant(&board_update_grant)
            .await
            .unwrap();
        assert_eq!(
            loom_create_request(
                &router,
                "DELETE",
                &format!("/workspaces/{workspace_id}/loom/canvas-placements/{placement_id}"),
                &headers,
                Value::Null,
            )
            .await,
            (
                StatusCode::FORBIDDEN,
                serde_json::json!({"error": "HSK-403-PROTECTED-RESOURCE"})
            )
        );
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET status = 'active', revoked_at = NONE, grant_version -= 1, policy_version -= 1 RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": board_update_grant.clone()}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": placement_authority.record_user_scope.grant_id.clone().unwrap(), "capabilities": board_without_read}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        let (status, removal) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{workspace_id}/loom/canvas-placements/{placement_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "authenticated Canvas removal: {removal}"
        );
        assert_eq!(removal["workspace_id"], workspace_id);
        assert_eq!(removal["canvas_block_id"], canvas_id);
        assert_eq!(removal["placement_id"], placement_id);
        assert_eq!(removal["placed_block_id"], note_id);
        let removal_event_id = removal["event"]["event_id"]
            .as_str()
            .expect("Canvas removal receipt event id")
            .to_owned();
        let mut removal_event = state
            .surreal
            .test_admin_query_bound(
                "SELECT actor_id, record::id(authority_session_id) AS authority_session_id, record::id(authority_resource_id) AS authority_resource_id, authority_capability_id, authority_action, payload FROM kernel_event_ledger WHERE event_id = $event_id;".to_owned(),
                serde_json::json!({"event_id": removal_event_id}),
            )
            .await
            .unwrap();
        let removal_events = removal_event.take::<Vec<Value>>(0).unwrap();
        assert_eq!(
            removal_events.len(),
            1,
            "removal must have one canonical receipt"
        );
        let removal_event = &removal_events[0];
        assert_eq!(removal_event["actor_id"], session["principal_id"]);
        assert_eq!(removal_event["authority_session_id"], session["session_id"]);
        assert_eq!(
            removal_event["authority_resource_id"],
            placement_authority.resource_id
        );
        assert_eq!(removal_event["authority_capability_id"], "fs.write");
        assert_eq!(removal_event["authority_action"], "update");
        assert_eq!(removal_event["payload"]["op"], "remove_placement");
        assert_eq!(removal_event["payload"]["canvas_block_id"], canvas_id);
        assert_eq!(removal_event["payload"]["placed_block_id"], note_id);
        state
            .surreal
            .test_admin_query_bound(
                "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
                serde_json::json!({"grant_id": placement_authority.record_user_scope.grant_id.clone().unwrap(), "capabilities": board_capabilities}),
            )
            .await
            .unwrap()
            .check()
            .unwrap();

        let (status, board_after_removal) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "read board after removal: {board_after_removal}"
        );
        assert!(
            board_after_removal["placements"]
                .as_array()
                .is_some_and(|placements| placements
                    .iter()
                    .all(|row| row["placement_id"] != placement_id)),
            "removal readback must omit the exact placement"
        );
        let (status, source_after_removal) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{workspace_id}/loom/blocks/{note_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "source must remain readable after removal"
        );
        assert_eq!(source_after_removal["block_id"], note_id);
        let mut canonical_source_after_removal = state
            .surreal
            .test_admin_query_bound(
                "RETURN (SELECT * FROM type::record('loom_blocks', $block_id))[0];".to_owned(),
                serde_json::json!({"block_id": note_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            canonical_source_after_removal
                .take::<Option<Value>>(0)
                .unwrap(),
            source_before_remove,
            "removing a placement must retain every source row field"
        );

        state
            .surreal
            .test_admin_query(
                "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger TYPE string ASSERT false;".to_owned(),
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        let before = loom_bundle_snapshot(&state).await;
        let (status, body) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &headers,
            serde_json::json!({"content_type": "note", "title": "Ledger failure residue"}),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, serde_json::json!({"error": "HSK-500-LOOM"}));
        assert_eq!(loom_bundle_snapshot(&state).await, before);
        state
            .surreal
            .test_admin_query(
                "DEFINE FIELD OVERWRITE event_id ON TABLE kernel_event_ledger TYPE string ASSERT $value = record::id($this.id);".to_owned(),
            )
            .await
            .unwrap()
            .check()
            .unwrap();
        assert_eq!(
            loom_create_request(&router, "POST", "/authority/logout", &headers, Value::Null)
                .await
                .0,
            StatusCode::OK
        );
    }

    /// WP-KERNEL-009 MT-264: an AppState whose model runtime DOES expose a real
    /// (deterministic) 768-d embedding endpoint, so the API authority write path
    /// produces and persists block embeddings exactly like a configured Ollama
    /// embedding model. Used to prove blocker #2 (embedding refreshed on
    /// create/update through the real handler, not only in manual-reindex tests).
    async fn setup_state_with_embedding(
    ) -> Result<(AppState, crate::storage::tests::EmbeddedTestBackend), Box<dyn std::error::Error>>
    {
        let backend = embedded_test_backend().await?;
        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
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
        };
        Ok((state, backend))
    }

    /// The number of `loom_block_search_index` rows for a block that carry a
    /// non-NULL embedding (semantic projection populated).
    async fn embedded_index_rows(
        state: &AppState,
        block_id: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        loom_test_count(
            state,
            "SELECT count() AS count FROM loom_block_search_index \
             WHERE block_id = $block AND embedding != NONE GROUP ALL;",
            LoomTestBlockBinding {
                block: RecordId::new("loom_blocks", block_id),
            },
        )
        .await
    }

    /// MT-264 blocker #2: a block created through the REAL `create_loom_block`
    /// API handler (with a configured embedding model) gets its embedding
    /// populated on the authority write path — not only when a test manually
    /// calls `reindex_block`. Editing the title through `patch_loom_block`
    /// re-embeds the new text. Both are proven against the real durable store.
    #[tokio::test]
    async fn mt264_api_create_and_update_refresh_embedding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (state, _store) = setup_state_with_embedding().await?;
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
            axum::http::HeaderMap::new(),
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
    /// against the real durable store.
    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt264_journal_block_indexed_on_create() -> Result<(), Box<dyn std::error::Error>> {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await?;
        // The daily note is created under the authenticated account session (Master Spec
        // §2.3.13.12.4); the owner creates the workspace through the product route.
        let (_router, headers, workspace_id) = owned_loom_session(&state, &binding).await;

        let journal = open_daily_journal(
            State(state.clone()),
            Path((workspace_id.clone(), "2026-06-18".to_string())),
            headers.clone(),
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        let block_id = journal.0.block_id.clone();
        // Get-or-create is idempotent for the same account/workspace/date.
        let reopened = open_daily_journal(
            State(state.clone()),
            Path((workspace_id.clone(), "2026-06-18".to_string())),
            headers,
        )
        .await
        .map_err(|(status, Json(body))| LoomApiTestCallError {
            status,
            code: body.error.to_string(),
        })?;
        assert_eq!(reopened.0.block_id, block_id, "same daily note on reopen");

        // The journal block has an index row immediately on creation.
        let index_rows = loom_test_count(
            &state,
            "SELECT count() AS count FROM loom_block_search_index \
             WHERE block_id = $block GROUP ALL;",
            LoomTestBlockBinding {
                block: RecordId::new("loom_blocks", block_id.as_str()),
            },
        )
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
        let (state, _store) = setup_state().await?;
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
            "inline tag/mention/kind/path operators must filter against SurrealDB rows"
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

        let (state, _store) = setup_state().await?;
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
        let (state, _store) = setup_state().await?;
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
        let (state, _store) = setup_state().await?;
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
            axum::http::HeaderMap::new(),
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
            axum::http::HeaderMap::new(),
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

        let stored = state
            .storage
            .get_loom_block(&workspace_id, &block_id)
            .await?;
        assert!(!stored.pinned, "SurrealDB read after remove is unpinned");
        assert_eq!(
            stored.pin_order, None,
            "SurrealDB read after remove is unordered"
        );

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
                        .map(|fields| fields.iter().any(|changed| changed.as_str() == Some(field)))
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
    /// `patch_loom_block` route against the real store: add_tags creates `tag`
    /// loom_edges to a TagHub target, recompute makes `derived.tag_count`
    /// authoritative, remove_tags deletes the edge, add is idempotent, and a
    /// non-TagHub target is rejected with HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB.
    /// A LoomBlockUpdated receipt records the tag mutation (fields_changed=tags,
    /// tags_added/tags_removed payloads).
    #[tokio::test]
    async fn mt258_properties_tag_edit_persists_edges_and_metrics(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (state, _store) = setup_state().await?;
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
                    axum::http::HeaderMap::new(),
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

        // The edges are real `tag` loom_edges in SurrealDB, not just a counter.
        let edges_after_add = state
            .storage
            .list_loom_edges_for_block(&workspace_id, &block_id)
            .await?;
        let tag_targets: std::collections::HashSet<String> = edges_after_add
            .iter()
            .filter(|edge| edge.edge_type == LoomEdgeType::Tag && edge.source_block_id == block_id)
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
            "removed tag edge must be gone from SurrealDB"
        );
        assert!(
            edges_after_remove.iter().any(|edge| {
                edge.edge_type == LoomEdgeType::Tag
                    && edge.source_block_id == block_id
                    && edge.target_block_id == tag_b.block_id
            }),
            "the untouched tag edge must survive the remove"
        );

        // SurrealDB read after the route confirms durability (not in-memory only).
        let stored = state
            .storage
            .get_loom_block(&workspace_id, &block_id)
            .await?;
        assert_eq!(
            stored.derived.tag_count, 1,
            "SurrealDB re-read confirms the tag mutation persisted"
        );

        // TagHub guard: a non-TagHub target is rejected, no edge is created.
        let non_tag = make_block(LoomBlockContentType::Note, "not a tag").await?;
        let guard = patch(vec![non_tag.block_id.clone()], Vec::new())
            .await
            .expect_err("non-TagHub target must be rejected");
        assert_eq!(guard.status, StatusCode::BAD_REQUEST);
        assert_eq!(guard.code, "HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB");
        let after_guard = state
            .storage
            .get_loom_block(&workspace_id, &block_id)
            .await?;
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
        let (state, _store) = setup_state().await?;
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
        let (state, _store) = setup_state().await?;
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

    fn mt027_view_definition() -> crate::storage::BlockViewDefinition {
        crate::storage::BlockViewDefinition {
            kind: crate::storage::BlockViewKind::Table,
            query: crate::storage::BlockViewQuery::default(),
            columns: vec![crate::storage::BlockViewField::Title],
            group_by: None,
            sort: None,
            calendar_date_field: None,
        }
    }

    async fn mt027_create_pending_view(
        state: &AppState,
        workspace_id: &str,
        block_id: &str,
    ) -> Result<BlockViewRecord, Box<dyn std::error::Error>> {
        Ok(state
            .storage
            .create_block_view(
                &WriteContext::human(Some("mt027-recovery".to_owned())),
                workspace_id,
                block_id,
                Some(format!("Recovery {block_id}")),
                mt027_view_definition(),
            )
            .await?)
    }

    async fn mt027_is_published(
        state: &AppState,
        workspace_id: &str,
        event_id: Uuid,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(crate::storage::block_view_outbox_surreal::load_row(
            &state.surreal,
            workspace_id,
            event_id,
        )
        .await?
        .published_at
        .is_some())
    }

    /// MT-027 V3: prove the transactional outbox recovery matrix against real
    /// storage and a real DuckDB recorder. No mock recorder participates.
    #[tokio::test]
    async fn mt027_block_view_publication_survives_outage_restart_races_and_retention(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut state, _store) = setup_state().await?;
        let workspace_id = create_workspace(&state).await?;
        let recorder_dir = TempDir::new()?;
        let recorder_path = recorder_dir.path().join("mt027-flight-recorder.duckdb");
        let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&recorder_path, 7)?);
        let recorder_connection = recorder.connection();
        state.flight_recorder = recorder.clone();
        state.diagnostics = recorder;

        // Recorder outage after the SurrealDB commit: publication fails and
        // durable intent remains retryable until a reconstructed service object
        // drains it after the real DuckDB surface returns.
        let outage_id = Uuid::new_v4().to_string();
        let outage = mt027_create_pending_view(&state, &workspace_id, &outage_id).await?;
        let outage_event_id = outage.publication_event_id.expect("outage event id");
        recorder_connection
            .lock()
            .expect("recorder connection")
            .execute_batch(
                "DROP INDEX IF EXISTS idx_events_trace_id;
                 DROP INDEX IF EXISTS idx_events_job_id;
                 DROP INDEX IF EXISTS idx_events_model_session_id;
                 DROP INDEX IF EXISTS idx_events_timestamp;
                 ALTER TABLE events RENAME TO events_mt027_offline;",
            )?;
        assert!(
            reconcile_block_view_events(&state, Some(&workspace_id), Some(outage_event_id))
                .await
                .is_err(),
            "a real recorder outage must not be reported as publication success"
        );
        assert!(!mt027_is_published(&state, &workspace_id, outage_event_id).await?);
        recorder_connection
            .lock()
            .expect("recorder connection")
            .execute_batch("ALTER TABLE events_mt027_offline RENAME TO events")?;
        let restarted = Arc::new(DuckDbFlightRecorder::new(recorder_connection.clone(), 7)?);
        state.flight_recorder = restarted.clone();
        state.diagnostics = restarted;
        reconcile_block_view_events(&state, Some(&workspace_id), Some(outage_event_id))
            .await
            .map_err(|error| {
                format!("restart reconcile failed: {} {}", error.0, error.1 .0.error)
            })?;
        assert!(mt027_is_published(&state, &workspace_id, outage_event_id).await?);

        // Crash after recorder insert but before SurrealDB acknowledgement:
        // restart observes the same real event and marks it exactly once.
        let crash_id = Uuid::new_v4().to_string();
        let crash = mt027_create_pending_view(&state, &workspace_id, &crash_id).await?;
        let crash_event_id = crash.publication_event_id.expect("crash event id");
        let crash_event = match block_view_outbox::load_scoped_publication(
            &state.surreal,
            &workspace_id,
            crash_event_id,
        )
        .await?
        {
            block_view_outbox::ScopedPublicationEvent::Pending(event) => event,
            block_view_outbox::ScopedPublicationEvent::Published => {
                panic!("fresh crash-window event unexpectedly published")
            }
        };
        record_block_view_event_idempotent(&state, crash_event)
            .await
            .map_err(|error| {
                format!(
                    "pre-crash recorder insert failed: {} {}",
                    error.0, error.1 .0.error
                )
            })?;
        assert!(!mt027_is_published(&state, &workspace_id, crash_event_id).await?);
        let restarted = Arc::new(DuckDbFlightRecorder::new(recorder_connection.clone(), 7)?);
        state.flight_recorder = restarted.clone();
        state.diagnostics = restarted;
        reconcile_block_view_events(&state, Some(&workspace_id), Some(crash_event_id))
            .await
            .map_err(|error| {
                format!(
                    "crash-window reconcile failed: {} {}",
                    error.0, error.1 .0.error
                )
            })?;
        assert_eq!(
            state
                .flight_recorder
                .list_events(EventFilter {
                    event_id: Some(crash_event_id),
                    ..EventFilter::default()
                })
                .await?
                .len(),
            1,
            "crash-window replay must remain exactly-once in the real recorder"
        );

        // Concurrent reconcilers may race on one pending row; both converge.
        let concurrent_id = Uuid::new_v4().to_string();
        let concurrent = mt027_create_pending_view(&state, &workspace_id, &concurrent_id).await?;
        let concurrent_event_id = concurrent
            .publication_event_id
            .expect("concurrent publication id");
        let (left, right) = tokio::join!(
            reconcile_block_view_events(&state, Some(&workspace_id), Some(concurrent_event_id)),
            reconcile_block_view_events(&state, Some(&workspace_id), Some(concurrent_event_id))
        );
        left.map_err(|error| format!("left reconciler: {} {}", error.0, error.1 .0.error))?;
        right.map_err(|error| format!("right reconciler: {} {}", error.0, error.1 .0.error))?;
        assert!(mt027_is_published(&state, &workspace_id, concurrent_event_id).await?);

        // Deleting a block cannot erase its unpublished audit intent.
        let deleted_id = Uuid::new_v4().to_string();
        let deleted = mt027_create_pending_view(&state, &workspace_id, &deleted_id).await?;
        let deleted_event_id = deleted.publication_event_id.expect("deleted event id");
        state
            .storage
            .delete_loom_block(
                &WriteContext::human(Some("mt027-delete".to_owned())),
                &workspace_id,
                &deleted_id,
            )
            .await?;
        reconcile_block_view_events(&state, Some(&workspace_id), Some(deleted_event_id))
            .await
            .map_err(|error| {
                format!(
                    "deleted-block publication failed: {} {}",
                    error.0, error.1 .0.error
                )
            })?;
        assert!(mt027_is_published(&state, &workspace_id, deleted_event_id).await?);

        // Delete/recreate creates a new incarnation. An identical retry must
        // bind the newest create intent, never the retained deleted incarnation.
        let reincarnated_id = Uuid::new_v4().to_string();
        let old = mt027_create_pending_view(&state, &workspace_id, &reincarnated_id).await?;
        let old_event_id = old.publication_event_id.expect("old incarnation event");
        state
            .storage
            .delete_loom_block(
                &WriteContext::human(Some("mt027-delete".to_owned())),
                &workspace_id,
                &reincarnated_id,
            )
            .await?;
        let new = mt027_create_pending_view(&state, &workspace_id, &reincarnated_id).await?;
        let new_event_id = new.publication_event_id.expect("new incarnation event");
        assert_ne!(old_event_id, new_event_id);
        let retry = mt027_create_pending_view(&state, &workspace_id, &reincarnated_id).await?;
        assert_eq!(
            retry.publication_event_id,
            Some(new_event_id),
            "same-id retry must select the latest retained create intent"
        );

        // The global service reconciler drains successive 200-row pages.
        let mut batch_event_ids = Vec::with_capacity(201);
        for _ in 0..201 {
            let block_id = Uuid::new_v4().to_string();
            let record = mt027_create_pending_view(&state, &workspace_id, &block_id).await?;
            batch_event_ids.push(record.publication_event_id.expect("batch event id"));
        }
        reconcile_block_view_events(&state, None, None)
            .await
            .map_err(|error| format!("batch reconcile failed: {} {}", error.0, error.1 .0.error))?;
        let unpublished_batch = loom_test_count(
            &state,
            "SELECT count() AS count FROM loom_block_view_fr_outbox \
             WHERE event_id IN $event_ids AND published_at = NONE GROUP ALL;",
            LoomTestEventListBinding {
                event_ids: batch_event_ids.iter().map(ToString::to_string).collect(),
            },
        )
        .await?;
        assert_eq!(unpublished_batch, 0, "all rows beyond page 200 must drain");

        // Corruption is quarantined and remains a typed request failure.
        let corrupt_id = Uuid::new_v4().to_string();
        let corrupt = mt027_create_pending_view(&state, &workspace_id, &corrupt_id).await?;
        let corrupt_event_id = corrupt.publication_event_id.expect("corrupt event id");
        let corrupt_binding = LoomTestEventBinding {
            workspace: RecordId::new("workspaces", workspace_id.as_str()),
            event_id: corrupt_event_id.to_string(),
        };
        let corrupted = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE loom_block_view_fr_outbox SET event_hash = $hash \
                             WHERE workspace_id = $workspace AND event_id = $event_id RETURN AFTER;",
                            LoomTestCorruptionBinding {
                                workspace: corrupt_binding.workspace,
                                event_id: corrupt_binding.event_id,
                                hash: "0".repeat(64),
                            },
                        )
                        .await
                })
            })
            .await?;
        assert_eq!(corrupted, 1, "the exact outbox row must be corrupted");
        for attempt in 1..=2 {
            assert!(
                reconcile_block_view_events(&state, Some(&workspace_id), Some(corrupt_event_id))
                    .await
                    .is_err(),
                "corrupt/quarantined exact event must fail attempt {attempt}"
            );
        }
        let corrupt_state = crate::storage::block_view_outbox_surreal::load_row(
            &state.surreal,
            &workspace_id,
            corrupt_event_id,
        )
        .await?;
        assert!(corrupt_state.quarantined_at.is_some());
        assert!(corrupt_state.published_at.is_none());

        // Already-published exact retries are idempotent success.
        reconcile_block_view_events(&state, Some(&workspace_id), Some(outage_event_id))
            .await
            .map_err(|error| format!("published retry failed: {} {}", error.0, error.1 .0.error))?;
        Ok(())
    }

    // -- MT-109 C3: every Loom route runs as the account record user -----------------------------
    // Master Spec 02-system-architecture.md:2758/2773/2776 and LM-RLS-001/002: each representative
    // test drives the mounted routes with a real account session, proves the write returned its row
    // (a denied record-user write is silent in SurrealDB 3.2.0), and proves that an anonymous caller
    // and a second account both receive the constant denial.

    #[cfg(feature = "os-keychain")]
    async fn c3_raw_request(
        router: &Router,
        method: &str,
        uri: &str,
        headers: &HeaderMap,
    ) -> (StatusCode, Vec<u8>) {
        let mut request = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        (status, bytes.to_vec())
    }

    /// A second, separately provisioned account on the same channel binding: it holds no grant on
    /// the owner's workspace or blocks.
    #[cfg(feature = "os-keychain")]
    async fn c3_other_account(state: &AppState, binding: &LoomCreateBinding) -> HeaderMap {
        let capabilities = ["fs.read", "fs.write", "memory.read", "memory.propose"]
            .map(str::to_owned)
            .to_vec();
        let other = state
            .surreal
            .provision_principal(
                "mt109-c3-other",
                "mt109-c3-other",
                "human_account",
                "mt109-c3-other",
                "Operator",
                &capabilities,
                "mt109-c3-other",
                Some(&sha256_hex(binding.channel.as_bytes())),
                std::time::Duration::from_secs(3600),
            )
            .await
            .expect("second account");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        headers.insert("x-hsk-session-token", other.session.token.parse().unwrap());
        headers
    }

    /// Asserts the constant denial for an anonymous caller (channel binding only) and for the
    /// second account on the same request.
    #[cfg(feature = "os-keychain")]
    async fn c3_assert_denied(
        router: &Router,
        binding: &LoomCreateBinding,
        other: &HeaderMap,
        method: &str,
        uri: &str,
        body: Value,
    ) {
        let mut anonymous = HeaderMap::new();
        anonymous.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        for (who, headers) in [("anonymous", &anonymous), ("other account", other)] {
            let (status, denial) =
                loom_create_request(router, method, uri, headers, body.clone()).await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "{who} {method} {uri}: {denial}"
            );
            assert_eq!(
                denial["error"], "HSK-403-PROTECTED-RESOURCE",
                "{who} {method} {uri}"
            );
        }
    }

    #[cfg(feature = "os-keychain")]
    async fn c3_block(
        router: &Router,
        headers: &HeaderMap,
        ws: &str,
        kind: &str,
        title: &str,
    ) -> String {
        let (status, block) = loom_create_request(
            router,
            "POST",
            &format!("/workspaces/{ws}/loom/blocks"),
            headers,
            serde_json::json!({"content_type": kind, "title": title}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owned {kind} block: {block}");
        block["block_id"].as_str().unwrap().to_owned()
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_folders() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let note = c3_block(&router, &headers, &ws, "note", "C3 folder note").await;
        let (status, folder) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/folders"),
            &headers,
            serde_json::json!({"name": "C3 folder"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder create: {folder}");
        let folder_id = folder["folder_id"].as_str().unwrap().to_owned();
        let (status, folders) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/folders"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder list: {folders}");
        assert!(folders
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["folder_id"] == folder_id));
        let (status, added) = loom_create_request(
            &router,
            "PUT",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}/blocks/{note}"),
            &headers,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder member add: {added}");
        let (status, members) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}/blocks"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder members: {members}");
        assert_eq!(
            members.as_array().unwrap().len(),
            1,
            "member row is visible: {members}"
        );
        assert_eq!(members[0]["block_id"], note);
        let (status, renamed) = loom_create_request(
            &router,
            "PATCH",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}"),
            &headers,
            serde_json::json!({"name": "C3 folder renamed"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder rename: {renamed}");
        assert_eq!(renamed["name"], "C3 folder renamed");
        let mut receipts = state
            .surreal
            .test_admin_query_bound(
                "SELECT VALUE [actor_kind, actor_id, wsids, authority_action] FROM kernel_event_ledger WHERE event_type = 'KNOWLEDGE_LOOM_FOLDER_MUTATED' AND payload.folder_id = $folder_id;".to_owned(),
                serde_json::json!({"folder_id": folder_id}),
            )
            .await
            .unwrap();
        let receipts = receipts.take::<Vec<Value>>(0).unwrap();
        assert!(
            receipts.len() >= 3,
            "create, member add and rename receipts: {receipts:?}"
        );
        let principal = headers["x-hsk-actor-id"].to_str().unwrap();
        for receipt in &receipts {
            assert_eq!(receipt[0], "operator", "receipt actor kind: {receipt}");
            assert_eq!(
                receipt[1], principal,
                "receipt carries the session principal: {receipt}"
            );
            assert_eq!(
                receipt[2],
                serde_json::json!([ws]),
                "receipt workspace: {receipt}"
            );
        }
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/folders"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/loom/folders"),
            serde_json::json!({"name": "intruder"}),
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "DELETE",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}"),
            Value::Null,
        )
        .await;
        let (status, removed) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}/blocks/{note}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder member remove: {removed}");
        let (status, deleted) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/folders/{folder_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "folder delete: {deleted}");
        let mut left = state
            .surreal
            .test_admin_query_bound(
                "RETURN array::len(SELECT id FROM type::record('loom_folders', $folder_id));"
                    .to_owned(),
                serde_json::json!({"folder_id": folder_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            left.take::<Option<i64>>(0).unwrap(),
            Some(0),
            "the folder row is gone"
        );
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_wiki() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let note = c3_block(&router, &headers, &ws, "note", "C3 wiki source").await;
        let (status, page) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/wiki"),
            &headers,
            serde_json::json!({"title": "C3 wiki", "block_ids": [note]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki compile: {page}");
        let projection_id = page["projection_id"].as_str().unwrap().to_owned();
        let (status, pages) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/wiki"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki list: {pages}");
        assert!(pages["pages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["projection_id"] == projection_id));
        let (status, one) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/wiki/{projection_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki page: {one}");
        let (status, overlay) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/wiki/{projection_id}/overlays"),
            &headers,
            serde_json::json!({"annotation": "C3 overlay"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki overlay: {overlay}");
        let overlay_id = overlay["overlay_id"].as_str().unwrap().to_owned();
        let (status, overlays) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/wiki/{projection_id}/overlays"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki overlays: {overlays}");
        assert_eq!(overlays.as_array().unwrap().len(), 1);
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/wiki/{projection_id}"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/loom/wiki"),
            serde_json::json!({"title": "intruder"}),
        )
        .await;
        let (status, _) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/wiki-overlays/{overlay_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki overlay delete");
        let (status, _) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/wiki/{projection_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "wiki delete");
    }

    /// Also C2 follow-ups a and d: account-created tag edges (POST /loom/edges and the Kanban
    /// card-move PATCH add_tags/remove_tags) are written, read and deleted as the record user.
    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_tags_and_edges() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let note = c3_block(&router, &headers, &ws, "note", "C3 tagged note").await;
        let todo = c3_block(&router, &headers, &ws, "tag_hub", "c3-todo").await;
        let done = c3_block(&router, &headers, &ws, "tag_hub", "c3-done").await;
        let (status, edge) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/edges"),
            &headers,
            serde_json::json!({"source_block_id": note, "target_block_id": todo, "edge_type": "tag", "created_by": "user"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "account tag edge: {edge}");
        let edge_id = edge["edge_id"].as_str().unwrap().to_owned();
        let (status, tagged) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/tags/{todo}/blocks"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "tag blocks: {tagged}");
        assert!(
            tagged
                .as_array()
                .unwrap()
                .iter()
                .any(|b| b["block_id"] == note),
            "{tagged}"
        );
        let (status, moved) = loom_create_request(
            &router,
            "PATCH",
            &format!("/workspaces/{ws}/loom/blocks/{note}"),
            &headers,
            serde_json::json!({"add_tags": [done], "remove_tags": [todo]}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Kanban card move (tag edges): {moved}"
        );
        let (status, done_blocks) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/tags/{done}/blocks"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            done_blocks
                .as_array()
                .unwrap()
                .iter()
                .any(|b| b["block_id"] == note),
            "{done_blocks}"
        );
        let (status, todo_blocks) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/tags/{todo}/blocks"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            todo_blocks.as_array().unwrap().is_empty(),
            "the removed tag edge is gone: {todo_blocks}"
        );
        let mut edge_rows = state
            .surreal
            .test_admin_query_bound(
                "RETURN array::len(SELECT id FROM loom_edges WHERE edge_id = $edge_id);".to_owned(),
                serde_json::json!({"edge_id": edge_id}),
            )
            .await
            .unwrap();
        assert_eq!(
            edge_rows.take::<Option<i64>>(0).unwrap(),
            Some(0),
            "remove_tags deleted the account edge"
        );
        for uri in [
            format!("/workspaces/{ws}/loom/tags"),
            format!("/workspaces/{ws}/loom/tags/{done}"),
            format!("/workspaces/{ws}/loom/blocks/{done}/backlinks"),
        ] {
            let (status, body) =
                loom_create_request(&router, "GET", &uri, &headers, Value::Null).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        }
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/tags"),
            Value::Null,
        )
        .await;
        c3_assert_denied(&router, &binding, &other, "POST", &format!("/workspaces/{ws}/loom/edges"), serde_json::json!({"source_block_id": note, "target_block_id": done, "edge_type": "tag", "created_by": "user"})).await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "PATCH",
            &format!("/workspaces/{ws}/loom/blocks/{note}"),
            serde_json::json!({"add_tags": [todo]}),
        )
        .await;
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_graph_search_and_pins() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let note = c3_block(&router, &headers, &ws, "note", "Quasarfield note").await;
        let hub = c3_block(&router, &headers, &ws, "tag_hub", "quasarfield-hub").await;
        let (status, _) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/edges"),
            &headers,
            serde_json::json!({"source_block_id": note, "target_block_id": hub, "edge_type": "tag", "created_by": "user"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        for (method, uri, body) in [
            (
                "GET",
                format!("/workspaces/{ws}/loom/graph/local?start_block_id={note}"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/graph/global"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/graph/traverse?start_block_id={note}"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/views/all"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/graph-search?q=Quasarfield"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/visual-debug?start_block_id={note}&q=Quasarfield"),
                Value::Null,
            ),
            (
                "POST",
                format!("/workspaces/{ws}/loom/search-v2"),
                serde_json::json!({"query": "Quasarfield"}),
            ),
            (
                "POST",
                format!("/workspaces/{ws}/loom/metrics/recompute"),
                Value::Null,
            ),
            (
                "POST",
                format!("/workspaces/{ws}/loom/blocks/{note}/metrics/recompute"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/blocks/{note}/breadcrumbs"),
                Value::Null,
            ),
            (
                "GET",
                format!("/workspaces/{ws}/loom/blocks/{note}/unlinked-mentions"),
                Value::Null,
            ),
            (
                "PUT",
                format!("/workspaces/{ws}/loom/journals/2026-09-23"),
                Value::Null,
            ),
        ] {
            let (status, response) =
                loom_create_request(&router, method, &uri, &headers, body).await;
            assert_eq!(status, StatusCode::OK, "{method} {uri}: {response}");
        }
        let (status, found) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/search?q=Quasarfield"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "search: {found}");
        assert!(
            found
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["block"]["block_id"] == note || r["block_id"] == note),
            "record-user search sees the owned note: {found}"
        );
        let (status, pinned) = loom_create_request(
            &router,
            "PUT",
            &format!("/workspaces/{ws}/loom/blocks/{note}/pin-order"),
            &headers,
            serde_json::json!({"pin_order": 3}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "pin order: {pinned}");
        assert_eq!(pinned["pin_order"], 3);
        let (status, unpinned) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/blocks/{note}/remove-pin"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "remove pin: {unpinned}");
        let (status, recent) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/quick-switcher/recents"),
            &headers,
            serde_json::json!({"result_kind": "loom_block", "source_kind": "loom_block", "ref_id": note, "title": "Quasarfield note"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "quick switcher recent: {recent}");
        let (status, recents) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/quick-switcher/recents"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "recents: {recents}");
        assert_eq!(recents.as_array().unwrap().len(), 1, "{recents}");
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/search?q=Quasarfield"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/graph/global"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/loom/metrics/recompute"),
            Value::Null,
        )
        .await;
        // The daily-note route keeps its MT-111 status split: no session is 401, another account 403.
        let (status, _) = loom_create_request(
            &router,
            "PUT",
            &format!("/workspaces/{ws}/loom/journals/2026-09-24"),
            &other,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "other account journal open");
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_ai_suggestions() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let note = c3_block(&router, &headers, &ws, "note", "C3 AI note").await;
        let jobs_uri = format!("/workspaces/{ws}/loom/ai-jobs");
        let job_body = serde_json::json!({"kind": "auto_tag", "block_ids": [note]});
        let run_job =
            || loom_create_request(&router, "POST", &jobs_uri, &headers, job_body.clone());
        let (status, job) = run_job().await;
        assert_eq!(status, StatusCode::OK, "AI job: {job}");
        let suggestion = job["suggestions"][0]["suggestion_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (status, listed) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/ai-suggestions"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "AI suggestions: {listed}");
        assert!(listed
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["suggestion_id"] == suggestion));
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/ai-suggestions"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/loom/ai-suggestions/{suggestion}/accept"),
            Value::Null,
        )
        .await;
        let (status, accepted) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/ai-suggestions/{suggestion}/accept"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "AI accept (promotion as the session reviewer): {accepted}"
        );
        assert_eq!(accepted["review_state"], "promoted");
        let principal = headers["x-hsk-actor-id"].to_str().unwrap();
        assert_eq!(accepted["decided_by"], format!("operator:{principal}"));
        let (status, second) = run_job().await;
        assert_eq!(status, StatusCode::OK, "second AI job: {second}");
        let rejected_id = second["suggestions"][0]["suggestion_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (status, rejected) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/ai-suggestions/{rejected_id}/reject"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "AI reject: {rejected}");
        assert_eq!(rejected["review_state"], "rejected");
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_assets_collections_and_views() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        use base64::Engine as _;
        let (status, imported) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/import"),
            &headers,
            serde_json::json!({"bytes_b64": STANDARD.encode(b"mt109 c3 asset bytes"), "original_filename": "c3.txt", "mime": "text/plain"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "asset import: {imported}");
        let asset_id = imported["asset_id"].as_str().unwrap().to_owned();
        let file_block = imported["block_id"].as_str().unwrap().to_owned();
        let (status, file) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/blocks/{file_block}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "the imported file block is account-owned: {file}"
        );
        let (status, meta) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/assets/{asset_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "asset metadata: {meta}");
        let (status, bytes) = c3_raw_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/assets/{asset_id}/content"),
            &headers,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bytes, b"mt109 c3 asset bytes".to_vec());
        let (status, tiers) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/assets/{asset_id}/tiers"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "asset tiers: {tiers}");
        let (status, collection) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/collections"),
            &headers,
            serde_json::json!({"title": "C3 album", "asset_ids": [asset_id]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "collection: {collection}");
        let collection_id = collection["collection_id"].as_str().unwrap().to_owned();
        assert_eq!(collection["members"], serde_json::json!([asset_id]));
        let (status, reordered) = loom_create_request(
            &router,
            "PUT",
            &format!("/workspaces/{ws}/loom/collections/{collection_id}/order"),
            &headers,
            serde_json::json!({"asset_ids": [asset_id]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "collection order: {reordered}");
        let (status, read_back) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/collections/{collection_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "collection read: {read_back}");
        assert_eq!(read_back["members"], serde_json::json!([asset_id]));
        let view_id = Uuid::now_v7().to_string();
        let (status, view) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/views/definitions"),
            &headers,
            serde_json::json!({"block_id": view_id, "title": "C3 view", "definition": serde_json::to_value(mt027_view_definition()).unwrap()}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "saved view create: {view}");
        let (status, got) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/views/definitions/{view_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "saved view read: {got}");
        let (status, patched) = loom_create_request(
            &router,
            "PATCH",
            &format!("/workspaces/{ws}/loom/views/definitions/{view_id}"),
            &headers,
            serde_json::json!({"definition": serde_json::to_value(mt027_view_definition()).unwrap()}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "saved view update: {patched}");
        let (status, results) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/views/definitions/{view_id}/results"),
            &headers,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "saved view results: {results}");
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/assets/{asset_id}"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/collections/{collection_id}"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/views/definitions/{view_id}"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/assets/{asset_id}/tiers/poster/retry"),
            Value::Null,
        )
        .await;
    }

    /// Also C2 follow-up c: the transclusion of a RichDocument's same-id projection authorizes the
    /// `rich_document` resource; the markdown import is an account-owned RichDocument.
    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_transclusion_and_markdown_import() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let (status, imported) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/import/markdown"),
            &headers,
            serde_json::json!({"title": "C3 markdown", "markdown": "# C3 heading\n\nImported body"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "markdown import: {imported}");
        let document = imported["rich_document_id"].as_str().unwrap().to_owned();
        assert_eq!(imported["block"]["block_id"], document);
        let (status, transclusion) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/blocks/{document}/transclusion"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "rich-document transclusion: {transclusion}"
        );
        assert_eq!(transclusion["resolved"], true);
        assert_eq!(transclusion["source_document_id"], document);
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "GET",
            &format!("/workspaces/{ws}/loom/blocks/{document}/transclusion"),
            Value::Null,
        )
        .await;
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "POST",
            &format!("/workspaces/{ws}/loom/import/markdown"),
            serde_json::json!({"title": "intruder", "markdown": "x"}),
        )
        .await;
    }

    /// Also C2 follow-up b: undoing a text card removes its RichDocument-projection placement.
    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt109_c3_record_user_canvas_placements() {
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, headers, ws) = owned_loom_session(&state, &binding).await;
        let other = c3_other_account(&state, &binding).await;
        let (status, canvas) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/canvas-boards"),
            &headers,
            serde_json::json!({"title": "C3 canvas"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "canvas: {canvas}");
        let canvas_id = canvas["block_id"].as_str().unwrap().to_owned();
        let first = c3_block(&router, &headers, &ws, "note", "C3 placed one").await;
        let second = c3_block(&router, &headers, &ws, "note", "C3 placed two").await;
        let mut placements = Vec::new();
        for placed in [&first, &second] {
            let (status, placement) = loom_create_request(
                &router,
                "POST",
                &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/placements"),
                &headers,
                serde_json::json!({"placed_block_id": placed, "x": 10.0, "y": 10.0, "w": 200.0, "h": 120.0}),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "placement: {placement}");
            placements.push(placement["placement_id"].as_str().unwrap().to_owned());
        }
        let (status, moved) = loom_create_request(
            &router,
            "PATCH",
            &format!("/workspaces/{ws}/loom/canvas-placements/{}", placements[0]),
            &headers,
            serde_json::json!({"x": 321.0}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "placement move: {moved}");
        assert_eq!(moved["x"], 321.0);
        let (status, visual) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/visual-edges"),
            &headers,
            serde_json::json!({"from_placement_id": placements[0], "to_placement_id": placements[1]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "visual edge: {visual}");
        let visual_id = visual["visual_edge_id"].as_str().unwrap().to_owned();
        let (status, board) = loom_create_request(
            &router,
            "GET",
            &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "board: {board}");
        assert_eq!(
            board["visual_edges"].as_array().unwrap().len(),
            1,
            "{board}"
        );
        let (status, viewport) = loom_create_request(
            &router,
            "PUT",
            &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/viewport"),
            &headers,
            serde_json::json!({"board_state": {"schema_id": crate::storage::LOOM_CANVAS_BOARD_SCHEMA_ID, "pan_x": 5.0, "pan_y": 6.0, "zoom": 1.5}, "expected_event_ledger_event_id": board["board"]["event_ledger_event_id"]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "viewport: {viewport}");
        assert_eq!(viewport["board_state"]["zoom"], 1.5);
        c3_assert_denied(
            &router,
            &binding,
            &other,
            "PATCH",
            &format!("/workspaces/{ws}/loom/canvas-placements/{}", placements[0]),
            serde_json::json!({"x": 1.0}),
        )
        .await;
        let (status, _) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/canvas-visual-edges/{visual_id}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT, "visual edge delete");
        let (status, card) = loom_create_request(
            &router,
            "POST",
            &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/cards"),
            &headers,
            serde_json::json!({"title": "C3 text card", "body": "card body", "x": 40.0, "y": 40.0, "w": 200.0, "h": 120.0}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "text card: {card}");
        let text_placement = card["placement"]["placement_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (status, undone) = loom_create_request(
            &router,
            "DELETE",
            &format!("/workspaces/{ws}/loom/canvas-placements/{text_placement}"),
            &headers,
            Value::Null,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "text-card undo removes its placement: {undone}"
        );
        let mut left = state
            .surreal
            .test_admin_query_bound(
                "RETURN array::len(SELECT id FROM loom_canvas_placements WHERE placement_id = $placement);".to_owned(),
                serde_json::json!({"placement": text_placement}),
            )
            .await
            .unwrap();
        assert_eq!(left.take::<Option<i64>>(0).unwrap(), Some(0));
    }

    // ---- MT-153 route-family authority matrix (AC-153-2, AC-153-4..AC-153-7) -----------------
    // Master Spec 02-system-architecture.md:2758 ("Authorization is deny-by-default and evaluated on
    // every executable backend boundary"), :2773 (privileged sessions MUST NOT execute ordinary
    // protected-resource flows), :2776 (record-user permissions plus ResourceBroker are the
    // non-bypassable data boundary), LM-RLS-001 (a viewer reads but cannot create/edit) and
    // LM-RLS-002 (every grant explicit). Every family is driven through the real mounted router as
    // the owner, an anonymous caller, a second account and a same-account read-only viewer. A denied
    // record-user write is silent in SurrealDB 3.2.0, so each denial is followed by a re-read of the
    // family's canonical rows and of the workspace receipts: "constant 403 and nothing changed".

    #[cfg(feature = "os-keychain")]
    const MT153_DENIAL: &str = "HSK-403-PROTECTED-RESOURCE";

    #[cfg(feature = "os-keychain")]
    struct Mt153Matrix {
        state: AppState,
        router: Router,
        ws: String,
        principal: String,
        owner: HeaderMap,
        anonymous: HeaderMap,
        other: HeaderMap,
        viewer: HeaderMap,
    }

    /// Escapes the few characters a kernel aggregate id may carry that would break a path segment.
    #[cfg(feature = "os-keychain")]
    fn mt153_segment(raw: &str) -> String {
        raw.chars()
            .map(|c| match c {
                '%' => "%25".to_owned(),
                '/' => "%2F".to_owned(),
                ' ' => "%20".to_owned(),
                '#' => "%23".to_owned(),
                '?' => "%3F".to_owned(),
                other => other.to_string(),
            })
            .collect()
    }

    /// A second principal in the owner's account and access space holding only `read` x `fs.read`
    /// on the owner's workspace resource (LM-RLS-001 viewer).
    #[cfg(feature = "os-keychain")]
    async fn mt153_viewer(
        state: &AppState,
        binding: &LoomCreateBinding,
        principal: &str,
        ws: &str,
    ) -> HeaderMap {
        use crate::storage::surreal::resource_authority::{
            ResourceAction, ResourceGrantSpec, ResourceKind,
        };
        let mut lookup = state
            .surreal
            .test_admin_query_bound(
                "RETURN {account_id: (SELECT VALUE record::id(account_id) FROM principals WHERE record::id(id) = $principal)[0], account_key: (SELECT VALUE account_id.account_key FROM principals WHERE record::id(id) = $principal)[0], resource_id: (SELECT VALUE record::id(id) FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $ws)[0], space_id: (SELECT VALUE record::id(access_space_id) FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $ws)[0], space_key: (SELECT VALUE access_space_id.space_key FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $ws)[0]};".to_owned(),
                json!({"principal": principal, "ws": ws, "kind": ResourceKind::Workspace.as_str()}),
            )
            .await
            .expect("owner account and workspace resource lookup");
        let owner: Value = lookup
            .take::<Option<Value>>(0)
            .expect("owner account row")
            .expect("owner account row present");
        let text = |key: &str| -> String {
            owner[key]
                .as_str()
                .unwrap_or_else(|| panic!("owner {key} missing: {owner}"))
                .to_owned()
        };
        let viewer_key = format!("mt153-viewer-{}", Uuid::now_v7());
        let capabilities = vec!["fs.read".to_owned()];
        let viewer = state
            .surreal
            .provision_principal(
                &text("account_key"),
                &viewer_key,
                "human_account",
                &viewer_key,
                "Operator",
                &capabilities,
                &text("space_key"),
                Some(&sha256_hex(binding.channel.as_bytes())),
                std::time::Duration::from_secs(3600),
            )
            .await
            .expect("same-account viewer principal");
        assert_eq!(
            viewer.identity.account_id,
            text("account_id"),
            "the viewer belongs to the owner's account"
        );
        assert_eq!(
            viewer.identity.access_space_id,
            text("space_id"),
            "the viewer belongs to the owner's access space"
        );
        state
            .surreal
            .grant_resource(
                &text("account_id"),
                &text("space_id"),
                ResourceGrantSpec {
                    principal_id: viewer.identity.principal_id.clone(),
                    resource_id: text("resource_id"),
                    actions: vec![ResourceAction::Read],
                    capability_ids: capabilities,
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await
            .expect("viewer read grant on the owner's workspace resource");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        headers.insert("x-hsk-session-token", viewer.session.token.parse().unwrap());
        headers
    }

    #[cfg(feature = "os-keychain")]
    impl Mt153Matrix {
        fn uri(&self, path: &str) -> String {
            format!("/workspaces/{}{path}", self.ws)
        }

        async fn call(
            &self,
            headers: &HeaderMap,
            method: &str,
            uri: &str,
            body: Value,
        ) -> (StatusCode, Value) {
            loom_create_request(&self.router, method, uri, headers, body).await
        }

        /// (a) owner positive: the route returns its success status.
        async fn owner_ok(&self, family: &str, method: &str, path: &str, body: Value) -> Value {
            let uri = self.uri(path);
            let (status, response) = self.call(&self.owner, method, &uri, body).await;
            assert!(
                status.is_success(),
                "[{family}] owner {method} {uri} -> {status}: {response}"
            );
            response
        }

        /// Admin re-read of canonical rows (verification only; never a product flow).
        async fn row(&self, query: &str, bindings: Value) -> Value {
            let mut response = self
                .state
                .surreal
                .test_admin_query_bound(query.to_owned(), bindings)
                .await
                .unwrap_or_else(|error| panic!("canonical re-read {query}: {error}"));
            response
                .take::<Option<Value>>(0)
                .unwrap_or_else(|error| panic!("canonical re-read {query}: {error}"))
                .unwrap_or(Value::Null)
        }

        async fn ws_row(&self, query: &str) -> Value {
            self.row(query, json!({"ws": self.ws})).await
        }

        async fn ledger_sequence(&self) -> i64 {
            let mut response = self
                .state
                .surreal
                .test_admin_query(
                    "SELECT VALUE event_sequence FROM kernel_event_ledger;".to_owned(),
                )
                .await
                .expect("ledger sequence read");
            response
                .take::<Vec<Value>>(0)
                .expect("ledger sequences")
                .iter()
                .filter_map(Value::as_i64)
                .max()
                .unwrap_or(0)
        }

        async fn receipts_since(&self, since: i64) -> Vec<Value> {
            let mut response = self
                .state
                .surreal
                .test_admin_query_bound(
                    "SELECT event_sequence, event_id, event_type, aggregate_type, aggregate_id, actor_kind, actor_id FROM kernel_event_ledger WHERE event_sequence > $since AND (wsids CONTAINS $ws OR payload.workspace_id = $ws) ORDER BY event_sequence;".to_owned(),
                    json!({"since": since, "ws": self.ws}),
                )
                .await
                .expect("workspace receipts read");
            response.take::<Vec<Value>>(0).expect("workspace receipts")
        }

        /// AC-153-4: every workspace receipt the family's scoped writes appended carries the session
        /// principal; the owner reads it through GET /kernel/events/aggregates/:type/:id, the second
        /// account reads none of it and an anonymous caller is denied.
        async fn assert_receipts(&self, family: &str, since: i64, required: bool) {
            let receipts = self.receipts_since(since).await;
            if required {
                assert!(
                    !receipts.is_empty(),
                    "[{family}] the owner's scoped write appended no workspace receipt"
                );
            }
            let mut checked = std::collections::BTreeSet::new();
            for receipt in &receipts {
                assert_eq!(
                    receipt["actor_kind"], "operator",
                    "[{family}] receipt actor kind (never System/header): {receipt}"
                );
                assert_eq!(
                    receipt["actor_id"],
                    self.principal.as_str(),
                    "[{family}] receipt actor is the session principal: {receipt}"
                );
                let aggregate_type = receipt["aggregate_type"].as_str().unwrap_or_default();
                let aggregate_id = receipt["aggregate_id"].as_str().unwrap_or_default();
                if !checked.insert((aggregate_type.to_owned(), aggregate_id.to_owned())) {
                    continue;
                }
                let uri = format!(
                    "/kernel/events/aggregates/{}/{}",
                    mt153_segment(aggregate_type),
                    mt153_segment(aggregate_id)
                );
                let (status, events) = self.call(&self.owner, "GET", &uri, Value::Null).await;
                assert_eq!(status, StatusCode::OK, "[{family}] owner {uri}: {events}");
                assert!(
                    events.as_array().is_some_and(|rows| rows
                        .iter()
                        .any(|row| row["event_id"] == receipt["event_id"])),
                    "[{family}] the owner reads receipt {receipt} through {uri}: {events}"
                );
                let (status, foreign) = self.call(&self.other, "GET", &uri, Value::Null).await;
                assert_eq!(
                    status,
                    StatusCode::OK,
                    "[{family}] second account {uri}: {foreign}"
                );
                assert_eq!(
                    foreign.as_array().map(Vec::len),
                    Some(0),
                    "[{family}] the second account reads 0 of the owner's receipts via {uri}: {foreign}"
                );
                let (status, _) = self.call(&self.anonymous, "GET", &uri, Value::Null).await;
                assert_eq!(status, StatusCode::FORBIDDEN, "[{family}] anonymous {uri}");
            }
        }

        async fn assert_write_denied(
            &self,
            family: &str,
            method: &str,
            path: &str,
            body: Value,
            snapshot: &str,
        ) {
            self.assert_write_denied_with(
                family,
                method,
                path,
                body,
                snapshot,
                StatusCode::FORBIDDEN,
                MT153_DENIAL,
            )
            .await
        }

        /// (b)(c)(d): anonymous, second account and same-account viewer are denied the write with
        /// the constant body; the family's canonical rows (`snapshot`, bound to `$ws`) and the
        /// workspace receipts are unchanged afterwards.
        #[allow(clippy::too_many_arguments)]
        async fn assert_write_denied_with(
            &self,
            family: &str,
            method: &str,
            path: &str,
            body: Value,
            snapshot: &str,
            anonymous_status: StatusCode,
            anonymous_error: &str,
        ) {
            let uri = self.uri(path);
            let before = self.ws_row(snapshot).await;
            let since = self.ledger_sequence().await;
            for (who, headers, status, error) in [
                (
                    "anonymous",
                    &self.anonymous,
                    anonymous_status,
                    anonymous_error,
                ),
                (
                    "second account",
                    &self.other,
                    StatusCode::FORBIDDEN,
                    MT153_DENIAL,
                ),
                (
                    "same-account viewer",
                    &self.viewer,
                    StatusCode::FORBIDDEN,
                    MT153_DENIAL,
                ),
            ] {
                let (actual, denial) = self.call(headers, method, &uri, body.clone()).await;
                assert_eq!(
                    actual, status,
                    "[{family}] {who} {method} {uri} must be denied: {denial}"
                );
                assert_eq!(
                    denial["error"], error,
                    "[{family}] {who} {method} {uri} denial body: {denial}"
                );
            }
            let after = self.ws_row(snapshot).await;
            assert_eq!(
                before, after,
                "[{family}] denied {method} {uri} changed the canonical rows"
            );
            let leaked = self.receipts_since(since).await;
            assert!(
                leaked.is_empty(),
                "[{family}] denied {method} {uri} appended receipts: {leaked:?}"
            );
        }

        /// (b)(c) for reads: anonymous gets the constant denial; the second account gets the
        /// constant denial or an empty list, never the owner's rows.
        async fn assert_read_denied(&self, family: &str, method: &str, path: &str, body: Value) {
            let uri = self.uri(path);
            let (status, denial) = self.call(&self.anonymous, method, &uri, body.clone()).await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "[{family}] anonymous {method} {uri}: {denial}"
            );
            assert_eq!(
                denial["error"], MT153_DENIAL,
                "[{family}] anonymous {method} {uri}"
            );
            let (status, response) = self.call(&self.other, method, &uri, body).await;
            let constant = status == StatusCode::FORBIDDEN && response["error"] == MT153_DENIAL;
            let empty = status == StatusCode::OK && response.as_array().is_some_and(Vec::is_empty);
            assert!(
                constant || empty,
                "[{family}] second account {method} {uri} must get the constant denial or an empty list, got {status}: {response}"
            );
        }
    }

    #[cfg(feature = "os-keychain")]
    #[tokio::test]
    async fn mt153_loom_route_family_authority_matrix() {
        use base64::Engine as _;
        let binding = LoomCreateBinding::new();
        let (state, _store) = setup_state().await.unwrap();
        let (router, owner, ws) = owned_loom_session(&state, &binding).await;
        let router = router.merge(crate::api::kernel::routes(state.clone()));
        let principal = owner["x-hsk-actor-id"].to_str().unwrap().to_owned();
        let other = c3_other_account(&state, &binding).await;
        let viewer = mt153_viewer(&state, &binding, &principal, &ws).await;
        let mut anonymous = HeaderMap::new();
        anonymous.insert(
            "x-hsk-channel-binding-token",
            binding.channel.parse().unwrap(),
        );
        let m = Mt153Matrix {
            state: state.clone(),
            router,
            ws: ws.clone(),
            principal,
            owner,
            anonymous,
            other,
            viewer,
        };
        let block_row = "RETURN (SELECT * FROM loom_blocks WHERE record::id(id) = $id)[0];";
        let ws_blocks = "RETURN { rows: (SELECT * FROM loom_blocks WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id) };";

        // ---- blocks + pins + metrics ------------------------------------------------------
        let family = "blocks+pins+metrics";
        let note = c3_block(&m.router, &m.owner, &ws, "note", "MT153 Zephyrine note").await;
        let loose = c3_block(&m.router, &m.owner, &ws, "note", "MT153 loose note").await;
        let since = m.ledger_sequence().await;
        let renamed = m
            .owner_ok(
                family,
                "PATCH",
                &format!("/loom/blocks/{note}"),
                json!({"title": "MT153 Zephyrine renamed"}),
            )
            .await;
        assert_eq!(renamed["title"], "MT153 Zephyrine renamed", "[{family}]");
        let row = m.row(block_row, json!({"id": note})).await;
        assert_eq!(
            row["title"], "MT153 Zephyrine renamed",
            "[{family}] the canonical block row carries the rename: {row}"
        );
        let pinned = m
            .owner_ok(
                family,
                "PUT",
                &format!("/loom/blocks/{note}/pin-order"),
                json!({"pin_order": 3}),
            )
            .await;
        assert_eq!(pinned["pin_order"], 3, "[{family}] pin order: {pinned}");
        assert_eq!(
            m.row(block_row, json!({"id": note})).await["pin_order"],
            3,
            "[{family}] the canonical block row carries the pin order"
        );
        m.owner_ok(
            family,
            "POST",
            &format!("/loom/blocks/{note}/remove-pin"),
            Value::Null,
        )
        .await;
        assert!(
            m.row(block_row, json!({"id": note})).await["pin_order"].is_null(),
            "[{family}] remove-pin cleared the canonical pin order"
        );
        m.owner_ok(
            family,
            "POST",
            &format!("/loom/blocks/{note}/metrics/recompute"),
            Value::Null,
        )
        .await;
        m.assert_receipts(family, since, true).await;
        m.owner_ok(
            family,
            "PUT",
            &format!("/loom/blocks/{note}/pin-order"),
            json!({"pin_order": 5}),
        )
        .await;
        let one_block =
            format!("RETURN (SELECT * FROM loom_blocks WHERE record::id(id) = '{note}')[0];");
        for (method, path, body) in [
            (
                "POST",
                "/loom/blocks".to_owned(),
                json!({"content_type": "note", "title": "intruder"}),
            ),
            (
                "PATCH",
                format!("/loom/blocks/{note}"),
                json!({"title": "intruder"}),
            ),
            (
                "PUT",
                format!("/loom/blocks/{note}/pin-order"),
                json!({"pin_order": 9}),
            ),
            (
                "POST",
                format!("/loom/blocks/{note}/remove-pin"),
                Value::Null,
            ),
            (
                "POST",
                format!("/loom/blocks/{note}/metrics/recompute"),
                Value::Null,
            ),
            ("DELETE", format!("/loom/blocks/{note}"), Value::Null),
        ] {
            let snapshot = if path == "/loom/blocks" {
                ws_blocks
            } else {
                one_block.as_str()
            };
            m.assert_write_denied(family, method, &path, body, snapshot)
                .await;
        }
        for path in [
            format!("/loom/blocks/{note}"),
            format!("/loom/blocks/{note}/breadcrumbs"),
            format!("/loom/blocks/{note}/backlinks"),
            format!("/loom/blocks/{note}/unlinked-mentions"),
        ] {
            m.assert_read_denied(family, "GET", &path, Value::Null)
                .await;
        }
        let doomed = c3_block(&m.router, &m.owner, &ws, "note", "MT153 doomed note").await;
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/blocks/{doomed}"),
            Value::Null,
        )
        .await;
        assert!(
            m.row(block_row, json!({"id": doomed})).await.is_null(),
            "[{family}] the owner's delete removed the canonical block row"
        );

        // ---- folders ------------------------------------------------------------------------
        let family = "folders";
        let since = m.ledger_sequence().await;
        let folder = m
            .owner_ok(
                family,
                "POST",
                "/loom/folders",
                json!({"name": "MT153 folder"}),
            )
            .await;
        let folder_id = folder["folder_id"].as_str().unwrap().to_owned();
        m.owner_ok(
            family,
            "PUT",
            &format!("/loom/folders/{folder_id}/blocks/{note}"),
            json!({}),
        )
        .await;
        m.owner_ok(
            family,
            "PATCH",
            &format!("/loom/folders/{folder_id}"),
            json!({"name": "MT153 folder renamed"}),
        )
        .await;
        m.assert_receipts(family, since, true).await;
        let read_back = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/folders/{folder_id}"),
                Value::Null,
            )
            .await;
        assert_eq!(
            read_back["name"], "MT153 folder renamed",
            "[{family}] {read_back}"
        );
        let folder_rows = "RETURN {folders: (SELECT * FROM loom_folders WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), members: (SELECT * FROM loom_folder_members ORDER BY id)};";
        let rows = m.ws_row(folder_rows).await;
        assert_eq!(
            rows["folders"].as_array().map(Vec::len),
            Some(1),
            "[{family}] canonical folder row: {rows}"
        );
        assert_eq!(
            rows["members"].as_array().map(Vec::len),
            Some(1),
            "[{family}] canonical folder member row: {rows}"
        );
        for (method, path, body) in [
            (
                "POST",
                "/loom/folders".to_owned(),
                json!({"name": "intruder"}),
            ),
            (
                "PATCH",
                format!("/loom/folders/{folder_id}"),
                json!({"name": "intruder"}),
            ),
            (
                "PUT",
                format!("/loom/folders/{folder_id}/blocks/{loose}"),
                json!({}),
            ),
            (
                "DELETE",
                format!("/loom/folders/{folder_id}/blocks/{note}"),
                Value::Null,
            ),
            ("DELETE", format!("/loom/folders/{folder_id}"), Value::Null),
        ] {
            m.assert_write_denied(family, method, &path, body, folder_rows)
                .await;
        }
        for path in [
            "/loom/folders".to_owned(),
            format!("/loom/folders/{folder_id}"),
            format!("/loom/folders/{folder_id}/blocks"),
        ] {
            m.assert_read_denied(family, "GET", &path, Value::Null)
                .await;
        }
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/folders/{folder_id}/blocks/{note}"),
            Value::Null,
        )
        .await;
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/folders/{folder_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(
            m.ws_row(folder_rows).await["folders"]
                .as_array()
                .map(Vec::len),
            Some(0),
            "[{family}] the owner's delete removed the canonical folder row"
        );

        // ---- wiki + overlays + bootstrap/drift/fanout --------------------------------------
        let family = "wiki+overlays";
        let page = m
            .owner_ok(
                family,
                "POST",
                "/loom/wiki",
                json!({"title": "MT153 wiki", "block_ids": [note]}),
            )
            .await;
        let projection_id = page["projection_id"].as_str().unwrap().to_owned();
        m.owner_ok(
            family,
            "GET",
            &format!("/loom/wiki/{projection_id}"),
            Value::Null,
        )
        .await;
        let since = m.ledger_sequence().await;
        let overlay = m
            .owner_ok(
                family,
                "POST",
                &format!("/loom/wiki/{projection_id}/overlays"),
                json!({"annotation": "MT153 overlay"}),
            )
            .await;
        let overlay_id = overlay["overlay_id"].as_str().unwrap().to_owned();
        m.assert_receipts(family, since, true).await;
        let since = m.ledger_sequence().await;
        m.owner_ok(
            family,
            "POST",
            "/loom/wiki/drift-check",
            json!({"persist": false}),
        )
        .await;
        m.assert_receipts(family, since, false).await;
        let wiki_rows = "RETURN {pages: (SELECT * FROM knowledge_wiki_projections WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), overlays: (SELECT * FROM loom_wiki_overlays WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id)};";
        let rows = m.ws_row(wiki_rows).await;
        assert!(
            rows["pages"]
                .as_array()
                .is_some_and(|pages| !pages.is_empty()),
            "[{family}] canonical wiki projection row: {rows}"
        );
        assert_eq!(
            rows["overlays"].as_array().map(Vec::len),
            Some(1),
            "[{family}] canonical overlay row: {rows}"
        );
        for (method, path, body) in [
            (
                "POST",
                "/loom/wiki".to_owned(),
                json!({"title": "intruder", "block_ids": [note]}),
            ),
            (
                "POST",
                format!("/loom/wiki/{projection_id}/overlays"),
                json!({"annotation": "intruder"}),
            ),
            (
                "POST",
                format!("/loom/wiki/{projection_id}/regenerate"),
                Value::Null,
            ),
            (
                "DELETE",
                format!("/loom/wiki-overlays/{overlay_id}"),
                Value::Null,
            ),
            ("DELETE", format!("/loom/wiki/{projection_id}"), Value::Null),
            ("POST", "/loom/wiki/bootstrap".to_owned(), json!({})),
            ("POST", "/loom/wiki/drift-check".to_owned(), json!({})),
            (
                "POST",
                "/loom/wiki/fanout".to_owned(),
                json!({"source_kind": "loom_block", "source_id": note}),
            ),
        ] {
            m.assert_write_denied(family, method, &path, body, wiki_rows)
                .await;
        }
        for path in [
            "/loom/wiki".to_owned(),
            format!("/loom/wiki/{projection_id}"),
            format!("/loom/wiki/{projection_id}/stale"),
            format!("/loom/wiki/{projection_id}/overlays"),
        ] {
            m.assert_read_denied(family, "GET", &path, Value::Null)
                .await;
        }

        // ---- markdown import + asset import -------------------------------------------------
        let family = "markdown import + asset import";
        let since = m.ledger_sequence().await;
        let imported = m
            .owner_ok(
                family,
                "POST",
                "/loom/import/markdown",
                json!({"title": "MT153 markdown", "markdown": "# MT153\n\nImported body"}),
            )
            .await;
        let document = imported["rich_document_id"].as_str().unwrap().to_owned();
        assert!(
            !m.row(block_row, json!({"id": document})).await.is_null(),
            "[{family}] the markdown import's canonical block row exists"
        );
        let asset = m
            .owner_ok(
                family,
                "POST",
                "/loom/import",
                json!({"bytes_b64": STANDARD.encode(b"mt153 asset bytes"), "original_filename": "mt153.txt", "mime": "text/plain"}),
            )
            .await;
        let asset_id = asset["asset_id"].as_str().unwrap().to_owned();
        let file_block = asset["block_id"].as_str().unwrap().to_owned();
        let file = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/blocks/{file_block}"),
                Value::Null,
            )
            .await;
        assert_eq!(file["content_type"], "file", "[{family}] {file}");
        m.assert_receipts(family, since, false).await;
        let import_rows = "RETURN {blocks: array::len(SELECT id FROM loom_blocks WHERE workspace_id = type::record('workspaces', $ws)), assets: (SELECT * FROM assets WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), documents: array::len(SELECT id FROM knowledge_rich_documents)};";
        assert_eq!(
            m.ws_row(import_rows).await["assets"]
                .as_array()
                .map(Vec::len),
            Some(1),
            "[{family}] canonical asset row"
        );
        m.assert_write_denied(
            family,
            "POST",
            "/loom/import/markdown",
            json!({"title": "intruder", "markdown": "x"}),
            import_rows,
        )
        .await;
        m.assert_write_denied(
            family,
            "POST",
            "/loom/import",
            json!({"bytes_b64": STANDARD.encode(b"mt153 intruder bytes"), "original_filename": "intruder.txt", "mime": "text/plain"}),
            import_rows,
        )
        .await;

        // ---- AC-153-6: every LoomBlockContentType through the scoped create route ----------
        let family = "content types";
        for (kind, body) in [
            (
                "note",
                json!({"content_type": "note", "title": "MT153 note type"}),
            ),
            (
                "file",
                json!({"content_type": "file", "title": "MT153 file type", "asset_id": asset_id}),
            ),
            (
                "annotated_file",
                json!({"content_type": "annotated_file", "title": "MT153 annotated type", "asset_id": asset_id}),
            ),
            (
                "tag_hub",
                json!({"content_type": "tag_hub", "title": "mt153-type-hub"}),
            ),
            (
                "journal",
                json!({"content_type": "journal", "title": "MT153 journal type", "journal_date": "2026-09-21"}),
            ),
        ] {
            let created = m.owner_ok(family, "POST", "/loom/blocks", body).await;
            assert_eq!(
                created["content_type"], kind,
                "[{family}] {kind}: {created}"
            );
            let id = created["block_id"].as_str().unwrap().to_owned();
            assert_eq!(
                m.row(block_row, json!({"id": id})).await["content_type"],
                kind,
                "[{family}] canonical {kind} block row"
            );
            let read = m
                .owner_ok(family, "GET", &format!("/loom/blocks/{id}"), Value::Null)
                .await;
            assert_eq!(
                read["content_type"], kind,
                "[{family}] {kind} re-read: {read}"
            );
        }

        // ---- tags + edges -------------------------------------------------------------------
        let family = "tags+edges";
        let hub = c3_block(&m.router, &m.owner, &ws, "tag_hub", "mt153-hub").await;
        let spare_hub = c3_block(&m.router, &m.owner, &ws, "tag_hub", "mt153-spare").await;
        let since = m.ledger_sequence().await;
        let edge = m
            .owner_ok(
                family,
                "POST",
                "/loom/edges",
                json!({"source_block_id": note, "target_block_id": hub, "edge_type": "tag", "created_by": "user"}),
            )
            .await;
        let edge_id = edge["edge_id"].as_str().unwrap().to_owned();
        m.assert_receipts(family, since, true).await;
        let tagged = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/tags/{hub}/blocks"),
                Value::Null,
            )
            .await;
        assert!(
            tagged
                .as_array()
                .is_some_and(|blocks| blocks.iter().any(|b| b["block_id"] == note)),
            "[{family}] the owner's tag edge is visible: {tagged}"
        );
        let edge_row = "RETURN (SELECT * FROM loom_edges WHERE edge_id = $id)[0];";
        assert!(
            !m.row(edge_row, json!({"id": edge_id})).await.is_null(),
            "[{family}] canonical edge row"
        );
        let edge_rows = "RETURN { rows: (SELECT * FROM loom_edges WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id) };";
        m.assert_write_denied(
            family,
            "POST",
            "/loom/edges",
            json!({"source_block_id": note, "target_block_id": spare_hub, "edge_type": "tag", "created_by": "user"}),
            edge_rows,
        )
        .await;
        m.assert_write_denied(
            family,
            "DELETE",
            &format!("/loom/edges/{edge_id}"),
            Value::Null,
            edge_rows,
        )
        .await;
        for path in [
            "/loom/tags".to_owned(),
            format!("/loom/tags/{hub}"),
            format!("/loom/tags/{hub}/blocks"),
        ] {
            m.assert_read_denied(family, "GET", &path, Value::Null)
                .await;
        }
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/edges/{edge_id}"),
            Value::Null,
        )
        .await;
        assert!(
            m.row(edge_row, json!({"id": edge_id})).await.is_null(),
            "[{family}] the owner's delete removed the canonical edge row"
        );

        // ---- assets + tiers -----------------------------------------------------------------
        let family = "assets+tiers";
        m.owner_ok(family, "GET", &format!("/assets/{asset_id}"), Value::Null)
            .await;
        let (status, bytes) = c3_raw_request(
            &m.router,
            "GET",
            &m.uri(&format!("/assets/{asset_id}/content")),
            &m.owner,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "[{family}] owner asset content");
        assert_eq!(
            bytes,
            b"mt153 asset bytes".to_vec(),
            "[{family}] canonical asset bytes"
        );
        m.owner_ok(
            family,
            "GET",
            &format!("/assets/{asset_id}/tiers"),
            Value::Null,
        )
        .await;
        m.assert_write_denied(
            family,
            "POST",
            &format!("/assets/{asset_id}/tiers/poster/retry"),
            Value::Null,
            "RETURN {tiers: (SELECT * FROM media_asset_tiers ORDER BY id), jobs: array::len(SELECT id FROM ai_jobs)};",
        )
        .await;
        for path in [
            format!("/assets/{asset_id}"),
            format!("/assets/{asset_id}/content"),
            format!("/assets/{asset_id}/thumbnail"),
            format!("/assets/{asset_id}/tiers"),
        ] {
            m.assert_read_denied(family, "GET", &path, Value::Null)
                .await;
        }

        // ---- collections --------------------------------------------------------------------
        let family = "collections";
        let since = m.ledger_sequence().await;
        let collection = m
            .owner_ok(
                family,
                "POST",
                "/loom/collections",
                json!({"title": "MT153 album", "asset_ids": [asset_id]}),
            )
            .await;
        let collection_id = collection["collection_id"].as_str().unwrap().to_owned();
        m.owner_ok(
            family,
            "PUT",
            &format!("/loom/collections/{collection_id}/order"),
            json!({"asset_ids": [asset_id]}),
        )
        .await;
        m.assert_receipts(family, since, false).await;
        let read_back = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/collections/{collection_id}"),
                Value::Null,
            )
            .await;
        assert_eq!(
            read_back["members"],
            json!([asset_id]),
            "[{family}] {read_back}"
        );
        let collection_rows = "RETURN {collections: (SELECT * FROM loom_collections WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), members: (SELECT * FROM loom_collection_members ORDER BY id)};";
        assert_eq!(
            m.ws_row(collection_rows).await["collections"]
                .as_array()
                .map(Vec::len),
            Some(1),
            "[{family}] canonical collection row"
        );
        m.assert_write_denied(
            family,
            "POST",
            "/loom/collections",
            json!({"title": "intruder", "asset_ids": [asset_id]}),
            collection_rows,
        )
        .await;
        m.assert_write_denied(
            family,
            "PUT",
            &format!("/loom/collections/{collection_id}/order"),
            json!({"asset_ids": []}),
            collection_rows,
        )
        .await;
        m.assert_read_denied(
            family,
            "GET",
            &format!("/loom/collections/{collection_id}"),
            Value::Null,
        )
        .await;

        // ---- views + graph + search + visual-debug ------------------------------------------
        let family = "views+graph+search";
        let since = m.ledger_sequence().await;
        let reads = [
            ("GET", "/loom/views/all".to_owned(), Value::Null),
            (
                "GET",
                format!("/loom/graph/traverse?start_block_id={note}"),
                Value::Null,
            ),
            (
                "GET",
                format!("/loom/graph/local?start_block_id={note}"),
                Value::Null,
            ),
            ("GET", "/loom/graph/global".to_owned(), Value::Null),
            (
                "GET",
                "/loom/graph-search?q=Zephyrine".to_owned(),
                Value::Null,
            ),
            (
                "GET",
                format!("/loom/visual-debug?start_block_id={note}&q=Zephyrine"),
                Value::Null,
            ),
            (
                "POST",
                "/loom/search-v2".to_owned(),
                json!({"query": "Zephyrine"}),
            ),
        ];
        for (method, path, body) in reads.clone() {
            m.owner_ok(family, method, &path, body).await;
        }
        let found = m
            .owner_ok(family, "GET", "/loom/search?q=Zephyrine", Value::Null)
            .await;
        assert!(
            found.as_array().is_some_and(|rows| rows
                .iter()
                .any(|r| r["block"]["block_id"] == note || r["block_id"] == note)),
            "[{family}] record-user search sees the owner's note: {found}"
        );
        m.owner_ok(family, "POST", "/loom/metrics/recompute", Value::Null)
            .await;
        m.assert_receipts(family, since, false).await;
        m.assert_write_denied(
            family,
            "POST",
            "/loom/metrics/recompute",
            Value::Null,
            ws_blocks,
        )
        .await;
        for (method, path, body) in reads {
            m.assert_read_denied(family, method, &path, body).await;
        }
        m.assert_read_denied(family, "GET", "/loom/search?q=Zephyrine", Value::Null)
            .await;

        // ---- quick-switcher -----------------------------------------------------------------
        let family = "quick-switcher";
        let since = m.ledger_sequence().await;
        m.owner_ok(
            family,
            "POST",
            "/loom/quick-switcher/recents",
            json!({"result_kind": "loom_block", "source_kind": "loom_block", "ref_id": note, "title": "MT153 Zephyrine renamed"}),
        )
        .await;
        m.assert_receipts(family, since, true).await;
        let recents = m
            .owner_ok(family, "GET", "/loom/quick-switcher/recents", Value::Null)
            .await;
        assert_eq!(
            recents.as_array().map(Vec::len),
            Some(1),
            "[{family}] {recents}"
        );
        let recent_rows = "RETURN { rows: (SELECT * FROM knowledge_quick_switcher_recents WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id) };";
        assert_eq!(
            m.ws_row(recent_rows).await["rows"].as_array().map(Vec::len),
            Some(1),
            "[{family}] canonical recent row"
        );
        m.assert_write_denied(
            family,
            "POST",
            "/loom/quick-switcher/recents",
            json!({"result_kind": "loom_block", "source_kind": "loom_block", "ref_id": hub, "title": "intruder"}),
            recent_rows,
        )
        .await;
        m.assert_read_denied(family, "GET", "/loom/quick-switcher/recents", Value::Null)
            .await;

        // ---- AI jobs + suggestions ----------------------------------------------------------
        let family = "AI jobs+suggestions";
        let since = m.ledger_sequence().await;
        let job = m
            .owner_ok(
                family,
                "POST",
                "/loom/ai-jobs",
                json!({"kind": "auto_tag", "block_ids": [note]}),
            )
            .await;
        m.assert_receipts(family, since, true).await;
        let job_id = job["job_id"].as_str().unwrap().to_owned();
        let suggestion = job["suggestions"][0]["suggestion_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let listed = m
            .owner_ok(family, "GET", "/loom/ai-suggestions", Value::Null)
            .await;
        assert!(
            listed
                .as_array()
                .is_some_and(|rows| rows.iter().any(|s| s["suggestion_id"] == suggestion)),
            "[{family}] {listed}"
        );
        let suggestion_rows = "RETURN {suggestions: (SELECT * FROM loom_ai_suggestions WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), edges: array::len(SELECT id FROM loom_edges)};";
        assert!(
            m.ws_row(suggestion_rows).await["suggestions"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            "[{family}] canonical suggestion rows"
        );
        for (method, path) in [
            ("POST", format!("/loom/ai-suggestions/{suggestion}/accept")),
            ("POST", format!("/loom/ai-suggestions/{suggestion}/reject")),
            ("POST", format!("/loom/ai-jobs/{job_id}/accept-all")),
        ] {
            m.assert_write_denied(family, method, &path, Value::Null, suggestion_rows)
                .await;
        }
        m.assert_write_denied(
            family,
            "POST",
            "/loom/ai-jobs",
            json!({"kind": "auto_tag", "block_ids": [note]}),
            suggestion_rows,
        )
        .await;
        m.assert_read_denied(family, "GET", "/loom/ai-suggestions", Value::Null)
            .await;
        let accepted = m
            .owner_ok(
                family,
                "POST",
                &format!("/loom/ai-suggestions/{suggestion}/accept"),
                Value::Null,
            )
            .await;
        assert_eq!(
            accepted["review_state"], "promoted",
            "[{family}] {accepted}"
        );

        // ---- canvas viewport / cards / stage-cards / placements / visual-edges ------------
        let family = "canvas";
        let canvas = m
            .owner_ok(
                family,
                "POST",
                "/loom/canvas-boards",
                json!({"title": "MT153 canvas"}),
            )
            .await;
        let canvas_id = canvas["block_id"].as_str().unwrap().to_owned();
        assert_eq!(
            m.row(block_row, json!({"id": canvas_id})).await["content_type"],
            "canvas",
            "[{family}] AC-153-6 canonical canvas block row"
        );
        let mut placements = Vec::new();
        for placed in [&note, &loose] {
            let placement = m
                .owner_ok(
                    family,
                    "POST",
                    &format!("/loom/canvas-boards/{canvas_id}/placements"),
                    json!({"placed_block_id": placed, "x": 10.0, "y": 10.0, "w": 200.0, "h": 120.0}),
                )
                .await;
            placements.push(placement["placement_id"].as_str().unwrap().to_owned());
        }
        let moved = m
            .owner_ok(
                family,
                "PATCH",
                &format!("/loom/canvas-placements/{}", placements[0]),
                json!({"x": 321.0}),
            )
            .await;
        assert_eq!(moved["x"], 321.0, "[{family}] {moved}");
        let visual = m
            .owner_ok(
                family,
                "POST",
                &format!("/loom/canvas-boards/{canvas_id}/visual-edges"),
                json!({"from_placement_id": placements[0], "to_placement_id": placements[1]}),
            )
            .await;
        let visual_id = visual["visual_edge_id"].as_str().unwrap().to_owned();
        let board = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/canvas-boards/{canvas_id}"),
                Value::Null,
            )
            .await;
        let since = m.ledger_sequence().await;
        let viewport = m
            .owner_ok(
                family,
                "PUT",
                &format!("/loom/canvas-boards/{canvas_id}/viewport"),
                json!({"board_state": {"schema_id": crate::storage::LOOM_CANVAS_BOARD_SCHEMA_ID, "pan_x": 5.0, "pan_y": 6.0, "zoom": 1.5}, "expected_event_ledger_event_id": board["board"]["event_ledger_event_id"]}),
            )
            .await;
        assert_eq!(
            viewport["board_state"]["zoom"], 1.5,
            "[{family}] {viewport}"
        );
        m.assert_receipts(family, since, true).await;
        let card = m
            .owner_ok(
                "canvas/text-card",
                "POST",
                &format!("/loom/canvas-boards/{canvas_id}/cards"),
                json!({"title": "MT153 text card", "body": "card body", "x": 40.0, "y": 40.0, "w": 200.0, "h": 120.0}),
            )
            .await;
        let text_placement = card["placement"]["placement_id"]
            .as_str()
            .unwrap()
            .to_owned();
        // AC-153-7: the Stage path runs under the account session and record-user scope.
        // Seed real capture authority, as in loom_canvas_board_tests; invented provenance
        // is correctly rejected before the Stage card can exercise route authorization.
        // This is fixture setup only: all card/compensation requests below remain record-user HTTP.
        use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
        use crate::storage::stage_artifacts::{NewStageCaptureArtifact, StageArtifactStore};
        let stage_receipt = |event_type, key: &str| {
            NewKernelEvent::builder(
                "mt153-stage-fixture",
                "mt153-stage-fixture-session",
                event_type,
                KernelActor::Operator("mt153-stage-fixture".to_owned()),
            )
            .aggregate("stage_capture_artifact", "pending")
            .idempotency_key(key)
            .correlation_id("mt153-stage-capture")
            .source_component("mt153_route_matrix")
            .payload(json!({"proof": "stage_capture_fixture"}))
            .build()
            .expect("Stage fixture receipt")
        };
        let capture = StageArtifactStore::new(m.state.surreal.clone())
            .insert_stage_artifact(NewStageCaptureArtifact {
                workspace_id: m.ws.clone(),
                content_kind: "canvas_node".to_owned(),
                label: "MT153 capture".to_owned(),
                content_type: "text/plain".to_owned(),
                content_json: json!({"text": "MT153 capture"}),
                content_bytes: b"MT153 capture".to_vec(),
                source_ref: None,
                idempotency_key: "mt153-stage-capture".to_owned(),
                request_hash: format!("{:x}", Sha256::digest(b"MT153 capture")),
                actor_kind: "operator".to_owned(),
                actor_id: "mt153-stage-fixture".to_owned(),
                correlation_id: "mt153-stage-capture".to_owned(),
                approval_id: "mt153-fixture-approval".to_owned(),
                decision_receipt: stage_receipt(
                    KernelEventType::ToolDecisionRecorded,
                    "mt153-stage-decision",
                ),
                receipt: stage_receipt(KernelEventType::ArtifactStored, "mt153-stage-stored"),
            })
            .await
            .expect("persist authoritative Stage capture fixture")
            .artifact;
        let stage_provenance = json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": capture.artifact_id,
            "sha256": capture.content_sha256,
            "manifest_ref": capture.manifest_ref,
            "causal_action_id": capture.correlation_id,
        });
        let stage_card = || {
            let provenance = &stage_provenance;
            json!({
                "title": format!("Stage capture {}", capture.artifact_id),
                "body": provenance.to_string(),
                "x": 60.0, "y": 60.0, "w": 200.0, "h": 120.0,
                "stage_provenance": provenance,
            })
        };
        let since = m.ledger_sequence().await;
        let staged = m
            .owner_ok(
                "canvas/stage-card",
                "POST",
                &format!("/loom/canvas-boards/{canvas_id}/cards"),
                stage_card(),
            )
            .await;
        m.assert_receipts(family, since, false).await;
        let stage_placement = staged["placement"]["placement_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let stage_block = staged["block"]["block_id"].as_str().unwrap().to_owned();
        let board = m
            .owner_ok(
                family,
                "GET",
                &format!("/loom/canvas-boards/{canvas_id}"),
                Value::Null,
            )
            .await;
        let placement_row =
            "RETURN (SELECT * FROM loom_canvas_placements WHERE placement_id = $id)[0];";
        for id in [&placements[0], &text_placement, &stage_placement] {
            assert!(
                !m.row(placement_row, json!({"id": id})).await.is_null(),
                "[{family}] canonical placement row {id}"
            );
        }
        let canvas_rows = "RETURN {boards: (SELECT * FROM loom_canvas_boards WHERE workspace_id = type::record('workspaces', $ws) ORDER BY id), placements: (SELECT * FROM loom_canvas_placements ORDER BY id), visual: (SELECT * FROM loom_canvas_visual_edges ORDER BY id), blocks: array::len(SELECT id FROM loom_blocks), documents: array::len(SELECT id FROM knowledge_rich_documents)};";
        for (method, path, body) in [
            (
                "POST",
                "/loom/canvas-boards".to_owned(),
                json!({"title": "intruder"}),
            ),
            (
                "PUT",
                format!("/loom/canvas-boards/{canvas_id}/viewport"),
                json!({"board_state": {"schema_id": crate::storage::LOOM_CANVAS_BOARD_SCHEMA_ID, "pan_x": 9.0, "pan_y": 9.0, "zoom": 2.0}, "expected_event_ledger_event_id": board["board"]["event_ledger_event_id"]}),
            ),
            (
                "POST",
                format!("/loom/canvas-boards/{canvas_id}/placements"),
                json!({"placed_block_id": note, "x": 1.0, "y": 1.0, "w": 10.0, "h": 10.0}),
            ),
            (
                "PATCH",
                format!("/loom/canvas-placements/{}", placements[0]),
                json!({"x": 1.0}),
            ),
            (
                "DELETE",
                format!("/loom/canvas-placements/{}", placements[1]),
                Value::Null,
            ),
            (
                "POST",
                format!("/loom/canvas-boards/{canvas_id}/visual-edges"),
                json!({"from_placement_id": placements[1], "to_placement_id": placements[0]}),
            ),
            (
                "DELETE",
                format!("/loom/canvas-visual-edges/{visual_id}"),
                Value::Null,
            ),
            (
                "POST",
                format!("/loom/canvas-boards/{canvas_id}/cards"),
                json!({"title": "intruder", "body": "x", "x": 1.0, "y": 1.0, "w": 10.0, "h": 10.0}),
            ),
            (
                "POST",
                format!("/loom/canvas-boards/{canvas_id}/cards"),
                stage_card(),
            ),
            (
                "POST",
                format!("/loom/canvas-boards/{canvas_id}/stage-cards/{stage_placement}/compensate"),
                json!({"placed_block_id": stage_block, "stage_provenance": stage_provenance}),
            ),
        ] {
            m.assert_write_denied(family, method, &path, body, canvas_rows)
                .await;
        }
        m.assert_read_denied(
            family,
            "GET",
            &format!("/loom/canvas-boards/{canvas_id}"),
            Value::Null,
        )
        .await;
        let compensated = m
            .owner_ok(
                family,
                "POST",
                &format!(
                    "/loom/canvas-boards/{canvas_id}/stage-cards/{stage_placement}/compensate"
                ),
                json!({"placed_block_id": stage_block, "stage_provenance": stage_provenance}),
            )
            .await;
        assert_eq!(
            compensated["removed_by_request"], true,
            "[{family}] Stage compensation: {compensated}"
        );
        assert!(
            m.row(placement_row, json!({"id": stage_placement}))
                .await
                .is_null(),
            "[{family}] the owner's compensation removed the Stage placement"
        );
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/canvas-visual-edges/{visual_id}"),
            Value::Null,
        )
        .await;
        m.owner_ok(
            family,
            "DELETE",
            &format!("/loom/canvas-placements/{text_placement}"),
            Value::Null,
        )
        .await;
        assert!(
            m.row(placement_row, json!({"id": text_placement}))
                .await
                .is_null(),
            "[{family}] the owner's delete removed the text-card placement"
        );

        // ---- block-view definitions + results -----------------------------------------------
        let family = "block-view definitions";
        let view_id = Uuid::now_v7().to_string();
        let definition = serde_json::to_value(mt027_view_definition()).unwrap();
        let since = m.ledger_sequence().await;
        m.owner_ok(
            family,
            "POST",
            "/loom/views/definitions",
            json!({"block_id": view_id, "title": "MT153 view", "definition": definition}),
        )
        .await;
        m.assert_receipts(family, since, true).await;
        assert_eq!(
            m.row(block_row, json!({"id": view_id})).await["content_type"],
            "view_def",
            "[{family}] AC-153-6 canonical view_def block row"
        );
        m.owner_ok(
            family,
            "GET",
            &format!("/loom/views/definitions/{view_id}"),
            Value::Null,
        )
        .await;
        m.owner_ok(
            family,
            "PATCH",
            &format!("/loom/views/definitions/{view_id}"),
            json!({"definition": definition}),
        )
        .await;
        m.owner_ok(
            family,
            "POST",
            &format!("/loom/views/definitions/{view_id}/results"),
            json!({}),
        )
        .await;
        let view_rows = "RETURN {views: (SELECT * FROM loom_blocks WHERE workspace_id = type::record('workspaces', $ws) AND content_type = 'view_def' ORDER BY id), outbox: array::len(SELECT id FROM loom_block_view_fr_outbox)};";
        m.assert_write_denied(
            family,
            "POST",
            "/loom/views/definitions",
            json!({"block_id": Uuid::now_v7().to_string(), "title": "intruder", "definition": definition}),
            view_rows,
        )
        .await;
        m.assert_write_denied(
            family,
            "PATCH",
            &format!("/loom/views/definitions/{view_id}"),
            json!({"definition": definition}),
            view_rows,
        )
        .await;
        m.assert_read_denied(
            family,
            "GET",
            &format!("/loom/views/definitions/{view_id}"),
            Value::Null,
        )
        .await;
        m.assert_read_denied(
            family,
            "POST",
            &format!("/loom/views/definitions/{view_id}/results"),
            json!({}),
        )
        .await;

        // ---- daily journal (MT-111 status split: no session 401, other account 403) ---------
        let family = "daily journal";
        let since = m.ledger_sequence().await;
        let journal = m
            .owner_ok(family, "PUT", "/loom/journals/2026-09-23", Value::Null)
            .await;
        m.assert_receipts(family, since, false).await;
        assert_eq!(journal["content_type"], "journal", "[{family}] {journal}");
        let journal_id = journal["block_id"].as_str().unwrap().to_owned();
        let journal_row = m.row(block_row, json!({"id": journal_id})).await;
        assert_eq!(
            journal_row["journal_date"], "2026-09-23",
            "[{family}] canonical journal row: {journal_row}"
        );
        m.assert_write_denied_with(
            family,
            "PUT",
            "/loom/journals/2026-09-25",
            Value::Null,
            "RETURN { rows: (SELECT * FROM loom_blocks WHERE workspace_id = type::record('workspaces', $ws) AND content_type = 'journal' ORDER BY id) };",
            StatusCode::UNAUTHORIZED,
            "HSK-401-LOOM-SESSION",
        )
        .await;
        let (status, _) = m
            .call(
                &m.other,
                "PUT",
                &m.uri("/loom/journals/2026-09-23"),
                Value::Null,
            )
            .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "[{family}] second account cannot open the owner's existing journal"
        );
    }
}

#[cfg(test)]
mod mt149_conflict_tests {
    use super::*;

    #[test]
    fn mt149_conflict_details_preserve_http_409_and_code() {
        for error in [
            StorageError::Conflict("stable_conflict_code"),
            StorageError::ConflictDetails {
                code: "stable_conflict_code",
                detail: "private diagnostic context".to_owned(),
            },
        ] {
            let (status, Json(body)) = map_storage_error(error);
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(body.error, "stable_conflict_code");
        }
    }
}
