use crate::flight_recorder::{
    FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::loom_fs::{loom_asset_blob_path, resolve_handshake_root};
use crate::models::ErrorResponse;
use crate::storage::{
    artifacts, Asset, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate,
    LoomEdge, LoomEdgeCreatedBy, LoomEdgeType, LoomSearchFilters, LoomViewFilters, LoomViewResponse,
    LoomViewType, NewAsset, NewLoomBlock, NewLoomEdge, PreviewStatus, StorageError, WriteContext,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;
use uuid::Uuid;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

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
        .route("/workspaces/:workspace_id/loom/blocks", post(create_loom_block))
        .route(
            "/workspaces/:workspace_id/loom/blocks/:block_id",
            get(get_loom_block).patch(patch_loom_block).delete(delete_loom_block),
        )
        // Loom edges
        .route("/workspaces/:workspace_id/loom/edges", post(create_loom_edge))
        .route(
            "/workspaces/:workspace_id/loom/edges/:edge_id",
            delete(delete_loom_edge),
        )
        // Import + assets
        .route("/workspaces/:workspace_id/loom/import", post(import_loom_asset))
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
        .route("/workspaces/:workspace_id/loom/search", get(search_loom_blocks))
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

    let block_id = block.block_id.clone();
    let block_workspace_id = block.workspace_id.clone();
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
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

    let block = state
        .storage
        .update_loom_block(&ctx, &workspace_id, &block_id, update)
        .await
        .map_err(map_storage_error)?;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockUpdated,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
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
        Uuid::new_v4(),
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
                LoomEdgeType::Tag | LoomEdgeType::SubTag => (LoomBlockContentType::TagHub, target_title),
                LoomEdgeType::Parent | LoomEdgeType::AiSuggested => (LoomBlockContentType::Note, target_title),
            };

            let title = title.ok_or_else(|| bad_request("HSK-400-LOOM-TARGET-TITLE-REQUIRED"))?;
            let ctx = WriteContext::human(None);
            state
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
        if anchor.offset_start < 0 || anchor.offset_end < 0 || anchor.offset_end < anchor.offset_start
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
        Uuid::new_v4(),
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
        Uuid::new_v4(),
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
            Uuid::new_v4(),
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

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomBlockCreated,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
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
        .profile_for_job_request(crate::storage::JobKind::LoomPreviewGenerate.as_str(), "hsk.loom.preview_generate@v1")
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
    let path = loom_asset_blob_path(&handshake_root, &workspace_id, &asset.kind, &asset.content_hash);

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
    let path = loom_asset_blob_path(&handshake_root, &workspace_id, &thumb.kind, &thumb.content_hash);
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
        .query_loom_view(&workspace_id, view_type.clone(), filters.clone(), limit, offset)
        .await
        .map_err(map_storage_error)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomViewQueried,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
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

    let tier_used = if state
        .storage
        .as_any()
        .is::<crate::storage::sqlite::SqliteDatabase>()
    {
        1
    } else {
        2
    };

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::LoomSearchExecuted,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::{duckdb::DuckDbFlightRecorder, EventFilter};
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{sqlite::SqliteDatabase, Database, NewWorkspace};
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

    async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
        let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
        sqlite.run_migrations().await?;

        let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);

        Ok(AppState {
            storage: sqlite.into_arc(),
            flight_recorder: flight_recorder.clone(),
            diagnostics: flight_recorder,
            llm_client: Arc::new(InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(CapabilityRegistry::new()),
        })
    }

    async fn create_workspace(state: &AppState) -> Result<String, Box<dyn std::error::Error>> {
        let ws = state
            .storage
            .create_workspace(&WriteContext::human(None), NewWorkspace {
                name: "Test".to_string(),
            })
            .await?;
        Ok(ws.id)
    }

    #[tokio::test]
    async fn import_dedup_emits_fr_evt_loom_006() -> Result<(), Box<dyn std::error::Error>> {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = TempDir::new()?;
        let _env = EnvVarGuard::set("HANDSHAKE_WORKSPACE_ROOT", temp.path().to_string_lossy().as_ref());

        let state = setup_state().await?;
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

        let events = state.flight_recorder.list_events(EventFilter::default()).await?;
        let events: Vec<_> = events
            .into_iter()
            .filter(|e| e.event_type.to_string() == "loom_dedup_hit")
            .collect();
        assert!(!events.is_empty(), "expected loom_dedup_hit event");
        Ok(())
    }

    #[tokio::test]
    async fn view_and_search_emit_events() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
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

        let all_events = state.flight_recorder.list_events(EventFilter::default()).await?;
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
}
