//! Atelier read/navigation HTTP surface (WP-KERNEL-005).
//!
//! Exposes the WP-KERNEL-005 atelier store over the existing Axum
//! server so a React panel can navigate it: a store overview, intake batches +
//! items, the command-corpus catalog, and the stealth-window registry. The
//! routes mirror the conventions in `api/workspaces.rs` exactly: a `routes`
//! builder, `State(AppState)` handlers, and a private `ErrorResponse` with
//! `internal_error` / `bad_request` helpers.
//!
//! Storage authority is the embedded SurrealDB store. Read handlers use the
//! typed `AtelierStore` surface or its lease-bound Surreal data context; no
//! relational fallback exists.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::atelier::intake::{
    IntakeBatchMode, IntakeItemLoomProjection, IntakeLaneCounts, IntakeProfileMode, NewIntakeBatch,
    NewIntakeItem,
};
use crate::atelier::search::{
    AiTagSuggestion, AiTagSuggestionDecision, AiTagSuggestionStatus, NewAiTagSuggestion,
};
use crate::atelier::stealth_window::ResolvedContentRef;
use crate::atelier::{
    AtelierError, AtelierStore, BulkOperationReceipt, ClipboardImageImportRequest,
    DeletionArchiveRequest, DeletionImpactPreview, DeletionImpactPreviewRequest,
    DeletionRestoreRequest, DeletionTargetRef, ImageImportRecord, MediaAssetBytesError,
    NewMediaAsset, UrlImageImportRequest,
};
use crate::storage::artifacts::{
    artifact_root_rel, remove_file_artifact, resolve_workspace_root,
    write_file_artifact_streaming, ArtifactClassification, ArtifactError, ArtifactLayer,
    StreamingFileArtifactSpec,
};
use crate::AppState;

/// Env override (bytes) for the streaming media ingest ceiling; default 4 GiB. The ceiling is
/// enforced while the body streams, so an over-limit upload never reaches disk in full and never
/// reaches memory at all.
pub(crate) const HSK_MEDIA_INGEST_MAX_BYTES_ENV: &str = "HANDSHAKE_MEDIA_INGEST_MAX_BYTES";
const DEFAULT_MEDIA_INGEST_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Optional request header naming where the bytes came from (a path, URL, capture source); falls
/// back to `http-ingest:<actor>` so the catalog row always carries provenance.
const HSK_HEADER_SOURCE_PROVENANCE: &str = "x-hsk-source-provenance";
/// Optional request header carrying the original filename for the manifest `filename_hint`.
const HSK_HEADER_FILENAME_HINT: &str = "x-hsk-filename-hint";

pub(crate) fn media_ingest_max_bytes() -> u64 {
    std::env::var(HSK_MEDIA_INGEST_MAX_BYTES_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MEDIA_INGEST_MAX_BYTES)
}

pub(crate) const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
/// Response header carrying the native ArtifactStore payload ref the served bytes came from, so a
/// consumer (Studio placed-asset link, CKC viewport) can record `artifact_manifest_id` without a
/// second catalog round-trip.
const HSK_HEADER_ARTIFACT_REF: &str = "x-hsk-artifact-ref";
/// Response header carrying the bare lowercase sha256 hex of the served payload (the value a Studio
/// placed-asset link stores as `resolved_content_hash`).
const HSK_HEADER_CONTENT_SHA256: &str = "x-hsk-content-sha256";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/atelier/overview", get(overview))
        .route(
            "/atelier/intake/batches",
            get(list_intake_batches).post(create_intake_batch),
        )
        .route(
            "/atelier/intake/batches/:batch_id/items",
            get(list_intake_batch_items).post(create_intake_item),
        )
        .route(
            "/atelier/intake/items/:item_id/loom-projection",
            put(link_intake_item_loom_projection),
        )
        .route("/atelier/command-corpus", get(list_command_corpus))
        .route(
            "/atelier/filesystem-health/checks",
            post(run_filesystem_health_check),
        )
        .route(
            "/atelier/filesystem-health/checks/:check_id/findings",
            get(list_filesystem_health_findings),
        )
        .route(
            "/atelier/deletion/impact-preview",
            post(preview_deletion_impact),
        )
        .route("/atelier/deletion/archive", post(archive_deletion_targets))
        .route("/atelier/deletion/restore", post(restore_deletion_targets))
        .route(
            "/atelier/image-import/clipboard",
            post(import_clipboard_image),
        )
        .route("/atelier/image-import/url", post(record_url_image_import))
        .route(
            "/atelier/ai-tag-suggestions",
            post(record_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/characters/:character_internal_id",
            get(list_ai_tag_suggestions_for_character),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/accept",
            post(accept_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/reject",
            post(reject_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/apply",
            post(apply_ai_tag_suggestion),
        )
        .route("/atelier/stealth/windows", get(list_stealth_windows))
        .route(
            "/atelier/stealth/windows/:window_ref_id/refs/:ref_id",
            get(resolve_stealth_ref),
        )
        .route(
            "/atelier/media-assets/:asset_id/bytes",
            get(get_media_asset_bytes),
        )
        .route("/atelier/media-assets", post(ingest_media_asset_bytes))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct MediaAssetIngestResponse {
    asset_id: Uuid,
    content_hash: String,
    byte_len: i64,
    mime: String,
    artifact_ref: String,
    /// True when identical bytes were already catalogued: the existing asset is returned and the
    /// freshly streamed duplicate payload was removed from the artifact tier.
    dedup_hit: bool,
}

/// POST /atelier/media-assets — streaming, size-capped ingest of one media asset. The raw request
/// body is the payload (no base64, no JSON envelope, no multipart); `Content-Type` is the catalog
/// MIME; `x-hsk-actor-id` is required; `x-hsk-source-provenance` and `x-hsk-filename-hint` are
/// optional. Bytes stream straight into the ArtifactStore (`write_file_artifact_streaming`), so peak
/// memory is one chunk regardless of payload size, and the ceiling
/// (`HANDSHAKE_MEDIA_INGEST_MAX_BYTES`, default 4 GiB) aborts the write before it completes.
///
/// Ordering and crash consistency (the Atelier pattern, made explicit): blob first, then verify,
/// then catalog row. If the row write fails or dedups to an existing asset, the just-written
/// artifact is removed (`remove_file_artifact`) so the blob tier never accumulates payloads with no
/// catalog row. The reverse failure (row committed, blob missing) cannot happen on this path because
/// `materialize_media_asset` re-verifies the ArtifactStore binding before it commits. A crash
/// between blob rename and row commit leaves an orphan artifact directory; that orphan is
/// unreferenced, hash-addressed, and swept by the existing artifact GC — it is never served, because
/// serving always starts from a catalog row.
async fn ingest_media_asset_bytes(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Result<(StatusCode, Json<MediaAssetIngestResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let mime = header_str(&headers, header::CONTENT_TYPE.as_str())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_owned())
        .filter(|value| !value.is_empty() && !value.starts_with("multipart/"))
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_or_unsupported_content_type",
            }),
        ))?;
    let max_bytes = media_ingest_max_bytes();
    if let Some(declared) = header_str(&headers, header::CONTENT_LENGTH.as_str())
        .and_then(|value| value.parse::<u64>().ok())
    {
        if declared > max_bytes {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ErrorResponse {
                    error: "payload_too_large",
                }),
            ));
        }
    }
    let source_provenance = header_str(&headers, HSK_HEADER_SOURCE_PROVENANCE)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("http-ingest:{actor}"));
    let filename_hint = header_str(&headers, HSK_HEADER_FILENAME_HINT).map(ToOwned::to_owned);

    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let artifact_id = Uuid::now_v7();
    let layer = ArtifactLayer::L1;
    let spec = StreamingFileArtifactSpec {
        artifact_id,
        layer,
        mime: mime.clone(),
        filename_hint,
        created_by_job_id: None,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: None,
        max_bytes,
    };
    let manifest =
        match write_file_artifact_streaming(&workspace_root, spec, body.into_data_stream()).await {
            Ok(manifest) => manifest,
            Err(ArtifactError::SizeLimitExceeded { .. }) => {
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(ErrorResponse {
                        error: "payload_too_large",
                    }),
                ));
            }
            Err(ArtifactError::Stream(detail)) => {
                tracing::warn!(target: "handshake_core::atelier", %detail, "media ingest stream failed");
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "payload_stream_failed",
                    }),
                ));
            }
            Err(ArtifactError::Io(io_err)) if io_err.kind() == std::io::ErrorKind::InvalidData => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "empty_payload",
                    }),
                ));
            }
            Err(other) => return Err(internal_error(other)),
        };
    let artifact_ref = format!("artifact://{}/payload", artifact_root_rel(layer, artifact_id));

    let store = atelier_store(&state);
    let asset = match store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: manifest.content_hash.clone(),
            mime: mime.clone(),
            byte_len: manifest.size_bytes as i64,
            source_provenance: Some(source_provenance),
            artifact_ref: artifact_ref.clone(),
        })
        .await
    {
        Ok(asset) => asset,
        Err(err) => {
            // Compensate: the catalog did not take the row, so the blob must not stay behind.
            if let Err(cleanup) = remove_file_artifact(&workspace_root, layer, artifact_id) {
                tracing::error!(
                    target: "handshake_core::atelier",
                    %artifact_id,
                    error = %cleanup,
                    "media ingest compensation failed: orphan artifact left for GC"
                );
            }
            return Err(atelier_error(err));
        }
    };
    let dedup_hit = asset.artifact_ref != artifact_ref;
    if dedup_hit {
        // Identical bytes were already catalogued under another artifact; ours is a duplicate blob.
        if let Err(cleanup) = remove_file_artifact(&workspace_root, layer, artifact_id) {
            tracing::error!(
                target: "handshake_core::atelier",
                %artifact_id,
                error = %cleanup,
                "media ingest dedup cleanup failed: duplicate artifact left for GC"
            );
        }
    }
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/media-assets",
        actor = %actor,
        asset_id = %asset.asset_id,
        byte_len = asset.byte_len,
        dedup_hit,
        "media asset ingested"
    );
    Ok((
        if dedup_hit {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(MediaAssetIngestResponse {
            asset_id: asset.asset_id,
            content_hash: asset.content_hash,
            byte_len: asset.byte_len,
            mime: asset.mime,
            artifact_ref: asset.artifact_ref,
            dedup_hit,
        }),
    ))
}

/// GET /atelier/media-assets/:asset_id/bytes — return the raw payload bytes of a catalog media asset,
/// read from the native ArtifactStore payload its `artifact_ref` points at, with the catalog row's
/// MIME as `Content-Type`. This is the byte-fetch READ path the ArtifactStore was missing over HTTP
/// (it was write-only), the route the Posekit source viewport consumes, and the byte resolver the
/// Studio placed-asset link ([STU-ASSET-005] / `asset.resolve_bytes`) binds to.
///
/// Fail-closed, never fabricated: an unknown asset or a missing payload is 404; a row whose
/// `artifact_ref` is not a native ArtifactStore payload is 400; a catalog-vs-manifest hash/size
/// mismatch, a bundle payload, or a payload-vs-manifest hash/size mismatch is a hard 500 — never a
/// partial or empty 200 body. All integrity checks live in
/// `AtelierStore::read_media_asset_bytes`; this handler only maps outcomes to HTTP.
///
/// Response headers: `Content-Type` (catalog MIME), `Content-Length`, `ETag` (`"sha256-<hex>"`),
/// `Cache-Control: private, immutable` (UUID-addressed payloads are write-once, so a client may cache
/// by `asset_id` + ETag), `x-hsk-artifact-ref`, `x-hsk-content-sha256`.
async fn get_media_asset_bytes(
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let resolved = match store.read_media_asset_bytes(asset_id).await {
        Ok(Some(resolved)) => resolved,
        Ok(None) => {
            return Err(atelier_error(AtelierError::NotFound(format!(
                "media asset_id={asset_id}"
            ))));
        }
        Err(MediaAssetBytesError::PayloadMissing) => {
            tracing::error!(
                target: "handshake_core::atelier",
                %asset_id,
                "media asset catalog row exists but its ArtifactStore payload/manifest is missing"
            );
            return Err(atelier_error(AtelierError::NotFound(
                "media artifact payload".to_owned(),
            )));
        }
        Err(MediaAssetBytesError::Validation(detail)) => {
            return Err(atelier_error(AtelierError::Validation(detail)));
        }
        Err(MediaAssetBytesError::Store(err)) => return Err(atelier_error(err)),
        Err(other) => return Err(internal_error(other)),
    };

    let content_hash = resolved.manifest.content_hash.to_ascii_lowercase();
    let mime = resolved.asset.mime.trim().to_owned();
    let artifact_ref = resolved.asset.artifact_ref.clone();
    let mut response = resolved.bytes.into_response();
    let headers = response.headers_mut();
    // application/octet-stream by default from Vec<u8>; override with the catalog-authoritative MIME
    // so the consumer decodes the real type. A non-encodable MIME degrades to octet-stream rather
    // than failing the response.
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(etag) = HeaderValue::from_str(&format!("\"sha256-{content_hash}\"")) {
        headers.insert(header::ETAG, etag);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, immutable"),
    );
    if let Ok(value) = HeaderValue::from_str(&artifact_ref) {
        headers.insert(HSK_HEADER_ARTIFACT_REF, value);
    }
    if let Ok(value) = HeaderValue::from_str(&content_hash) {
        headers.insert(HSK_HEADER_CONTENT_SHA256, value);
    }
    Ok(response)
}

/// Curated atelier tables surfaced by the overview row-count projection. This is
/// a fixed allowlist paired with literal SurrealQL count statements, so no
/// caller input reaches the query text.
const OVERVIEW_TABLES: &[&str] = &[
    "atelier_character",
    "atelier_media_asset",
    "atelier_media_source_provenance_ref",
    "atelier_media_sidecar",
    "atelier_bulk_operation_receipt",
    "atelier_trash_marker",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_image_import_request",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_pose_rig",
    "atelier_comfy_intake_output",
    "atelier_sourcing_spec",
    "atelier_transcript_artifact",
    "atelier_md_download_session",
    "atelier_command_corpus_entry",
    "atelier_ai_tag_suggestion",
    "atelier_stealth_window",
];

fn overview_count_statement(table: &str) -> &'static str {
    match table {
        "atelier_character" => "RETURN count(SELECT id FROM atelier_character);",
        "atelier_media_asset" => "RETURN count(SELECT id FROM atelier_media_asset);",
        "atelier_media_source_provenance_ref" => {
            "RETURN count(SELECT id FROM atelier_media_source_provenance_ref);"
        }
        "atelier_media_sidecar" => "RETURN count(SELECT id FROM atelier_media_sidecar);",
        "atelier_bulk_operation_receipt" => {
            "RETURN count(SELECT id FROM atelier_bulk_operation_receipt);"
        }
        "atelier_trash_marker" => "RETURN count(SELECT id FROM atelier_trash_marker);",
        "atelier_filesystem_health_check" => {
            "RETURN count(SELECT id FROM atelier_filesystem_health_check);"
        }
        "atelier_filesystem_health_finding" => {
            "RETURN count(SELECT id FROM atelier_filesystem_health_finding);"
        }
        "atelier_image_import_request" => {
            "RETURN count(SELECT id FROM atelier_image_import_request);"
        }
        "atelier_intake_batch" => "RETURN count(SELECT id FROM atelier_intake_batch);",
        "atelier_intake_item" => "RETURN count(SELECT id FROM atelier_intake_item);",
        "atelier_pose_rig" => "RETURN count(SELECT id FROM atelier_pose_rig);",
        "atelier_comfy_intake_output" => {
            "RETURN count(SELECT id FROM atelier_comfy_intake_output);"
        }
        "atelier_sourcing_spec" => "RETURN count(SELECT id FROM atelier_sourcing_spec);",
        "atelier_transcript_artifact" => {
            "RETURN count(SELECT id FROM atelier_transcript_artifact);"
        }
        "atelier_md_download_session" => {
            "RETURN count(SELECT id FROM atelier_md_download_session);"
        }
        "atelier_command_corpus_entry" => {
            "RETURN count(SELECT id FROM atelier_command_corpus_entry);"
        }
        "atelier_ai_tag_suggestion" => "RETURN count(SELECT id FROM atelier_ai_tag_suggestion);",
        "atelier_stealth_window" => "RETURN count(SELECT id FROM atelier_stealth_window);",
        _ => unreachable!("overview table must come from OVERVIEW_TABLES"),
    }
}

/// Shared by the CKC lane routers (`api::atelier_ckc_*`), which is why these helpers are
/// `pub(crate)`: one store constructor, one error envelope, one actor-header contract across
/// every Atelier route regardless of which file declares it.
pub(crate) fn atelier_store(state: &AppState) -> AtelierStore {
    AtelierStore::with_observability(
        state.surreal.clone(),
        state.storage.clone(),
        state.flight_recorder.clone(),
    )
}

/// Cap on list endpoints so a React panel never pulls an unbounded result set.
pub(crate) const LIST_CAP: i64 = 200;

#[derive(Debug, Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: &'static str,
}

pub(crate) fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(target: "handshake_core::atelier", error = %err, "db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

/// Map an ArtifactStore read failure for a byte route: a payload/manifest that is not on disk is
/// 404 (the catalog row may exist, the bytes do not); every other failure, including a content
/// hash or size mismatch, is a hard 500 — never a partial or empty 200 body.
pub(crate) fn artifact_byte_read_error(
    err: crate::storage::artifacts::ArtifactError,
) -> (StatusCode, Json<ErrorResponse>) {
    use crate::storage::artifacts::ArtifactError;
    match err {
        ArtifactError::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
            atelier_error(AtelierError::NotFound("artifact payload".to_owned()))
        }
        other => internal_error(other),
    }
}

pub(crate) fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn calling_actor(
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    header_str(headers, HSK_HEADER_ACTOR_ID)
        .map(ToOwned::to_owned)
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_actor",
            }),
        ))
}

/// Map an `AtelierError` to an HTTP status, mirroring `workspaces.rs`
/// `map_storage_error`: a missing aggregate is 404, a semantically-bad input is
/// 400, and infra/storage failures are 500. (Malformed `Path<Uuid>` / JSON body
/// inputs are already rejected with a 400 by Axum's extractors before a handler
/// runs.) The body never leaks internals — it is a fixed `&'static str` code.
pub(crate) fn atelier_error(
    err: crate::atelier::AtelierError,
) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        AtelierError::NotFound(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "not_found");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "not_found" }),
            )
        }
        AtelierError::Validation(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "bad_request");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "bad_request",
                }),
            )
        }
        AtelierError::Conflict(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "conflict");
            (
                StatusCode::CONFLICT,
                Json(ErrorResponse { error: "conflict" }),
            )
        }
        other => internal_error(other),
    }
}

#[derive(Debug, Serialize)]
struct TableCount {
    name: &'static str,
    rows: i64,
}

#[derive(Debug, Serialize)]
struct EventFamilyCount {
    family: String,
    count: i64,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    tables: Vec<TableCount>,
    event_families: Vec<EventFamilyCount>,
}

#[derive(SurrealValue)]
struct NoBindings {}

#[derive(SurrealValue)]
struct EventFamilyCountRow {
    event_family: String,
    count: i64,
}

/// GET /atelier/overview — row counts for the curated atelier tables plus
/// per-family atelier event counts.
async fn overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);

    let mut tables = Vec::with_capacity(OVERVIEW_TABLES.len());
    for name in OVERVIEW_TABLES {
        let statement = overview_count_statement(name);
        let rows = store
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first::<i64, _>(statement, NoBindings {}).await })
            })
            .await
            .map_err(atelier_error)?;
        let rows = rows.unwrap_or(0);
        tables.push(TableCount { name, rows });
    }

    let family_rows: Vec<EventFamilyCountRow> = store
        .with_data(|ctx| {
            Box::pin(async move {
                ctx.query_values(
                    "SELECT event_family, count() AS count FROM atelier_event \
                     GROUP BY event_family ORDER BY event_family;",
                    NoBindings {},
                )
                .await
            })
        })
        .await
        .map_err(atelier_error)?;

    let event_families = family_rows
        .into_iter()
        .map(|row| EventFamilyCount {
            family: row.event_family,
            count: row.count,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/overview", status = "ok", "atelier overview");

    Ok(Json(OverviewResponse {
        tables,
        event_families,
    }))
}

#[derive(Debug, Serialize)]
struct IntakeBatchResponse {
    batch_id: Uuid,
    idempotency_key: String,
    source_label: String,
    source_ref: String,
    mode: String,
    profile_mode: String,
    target_character_id: Option<Uuid>,
    target_sheet_version_id: Option<Uuid>,
    target_collection_id: Option<Uuid>,
    status: String,
    resume_cursor: Option<String>,
    resumed_at_utc: Option<DateTime<Utc>>,
    created_at_utc: DateTime<Utc>,
}

fn intake_batch_response(batch: crate::atelier::intake::IntakeBatch) -> IntakeBatchResponse {
    IntakeBatchResponse {
        batch_id: batch.batch_id,
        idempotency_key: batch.idempotency_key,
        source_label: batch.source_label,
        source_ref: batch.source_ref,
        mode: batch.mode.as_str().to_string(),
        profile_mode: batch.profile_mode.as_str().to_string(),
        target_character_id: batch.target_character_id,
        target_sheet_version_id: batch.target_sheet_version_id,
        target_collection_id: batch.target_collection_id,
        status: batch.status.as_str().to_string(),
        resume_cursor: batch.resume_cursor,
        resumed_at_utc: batch.resumed_at_utc,
        created_at_utc: batch.created_at_utc,
    }
}

/// GET /atelier/intake/batches — newest first, capped.
async fn list_intake_batches(
    State(state): State<AppState>,
) -> Result<Json<Vec<IntakeBatchResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let batches = store
        .list_intake_batches(None, LIST_CAP)
        .await
        .map_err(atelier_error)?;

    let out = batches.into_iter().map(intake_batch_response).collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "ok", "list intake batches");

    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct CreateIntakeBatchRequest {
    idempotency_key: String,
    source_label: String,
    source_ref: Option<String>,
    mode: Option<String>,
    profile_mode: Option<String>,
    target_character_id: Option<Uuid>,
    target_sheet_version_id: Option<Uuid>,
    target_collection_id: Option<Uuid>,
    resume_cursor: Option<String>,
}

/// POST /atelier/intake/batches — open (idempotently) an intake batch.
async fn create_intake_batch(
    State(state): State<AppState>,
    Json(payload): Json<CreateIntakeBatchRequest>,
) -> Result<(StatusCode, Json<IntakeBatchResponse>), (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let mode = match payload.mode.as_deref().unwrap_or("manual") {
        "manual" => IntakeBatchMode::Manual,
        "folder_scan" => IntakeBatchMode::FolderScan,
        "sourcing_run" => IntakeBatchMode::SourcingRun,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_mode",
                }),
            ));
        }
    };
    let profile_mode = match payload.profile_mode.as_deref().unwrap_or("loose_profile") {
        "loose_profile" => IntakeProfileMode::LooseProfile,
        "character_linked" => IntakeProfileMode::CharacterLinked,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_profile_mode",
                }),
            ));
        }
    };
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: payload.idempotency_key,
            source_label: payload.source_label,
            source_ref: payload.source_ref,
            mode,
            profile_mode,
            character_internal_id: payload.target_character_id,
            target_character_id: payload.target_character_id,
            target_sheet_version_id: payload.target_sheet_version_id,
            target_collection_id: payload.target_collection_id,
            resume_cursor: payload.resume_cursor,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "created", batch_id = %batch.batch_id, "open intake batch");

    Ok((StatusCode::CREATED, Json(intake_batch_response(batch))))
}

#[derive(Debug, Deserialize)]
struct RunFilesystemHealthCheckRequest {
    scope_label: Option<String>,
}

async fn run_filesystem_health_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RunFilesystemHealthCheckRequest>,
) -> Result<
    (StatusCode, Json<crate::atelier::FilesystemHealthReport>),
    (StatusCode, Json<ErrorResponse>),
> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let report = store
        .run_filesystem_health_check(&crate::atelier::FilesystemHealthCheckRequest {
            requested_by: actor.clone(),
            scope_label: payload.scope_label,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/filesystem-health/checks",
        status = "created",
        check_id = %report.check.check_id,
        actor = %actor,
        "run filesystem health check"
    );

    Ok((StatusCode::CREATED, Json(report)))
}

async fn list_filesystem_health_findings(
    State(state): State<AppState>,
    Path(check_id): Path<Uuid>,
) -> Result<Json<Vec<crate::atelier::FilesystemHealthFinding>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let findings = store
        .list_filesystem_health_findings(check_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/filesystem-health/checks/:check_id/findings",
        status = "ok",
        check_id = %check_id,
        "list filesystem health findings"
    );

    Ok(Json(findings))
}

#[derive(Debug, Deserialize)]
struct DeletionControlsRequest {
    targets: Vec<DeletionTargetRef>,
    reason: String,
}

async fn preview_deletion_impact(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<Json<DeletionImpactPreview>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let preview = store
        .preview_deletion_impact(&DeletionImpactPreviewRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/impact-preview",
        status = "ok",
        actor = %actor,
        target_count = preview.target_count,
        "preview deletion impact"
    );

    Ok(Json(preview))
}

async fn archive_deletion_targets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<(StatusCode, Json<BulkOperationReceipt>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let receipt = store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/archive",
        status = "created",
        actor = %actor,
        receipt_id = %receipt.receipt_id,
        "archive deletion targets"
    );

    Ok((StatusCode::CREATED, Json(receipt)))
}

async fn restore_deletion_targets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<(StatusCode, Json<BulkOperationReceipt>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let receipt = store
        .restore_deletion_targets(&DeletionRestoreRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/restore",
        status = "created",
        actor = %actor,
        receipt_id = %receipt.receipt_id,
        "restore deletion targets"
    );

    Ok((StatusCode::CREATED, Json(receipt)))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipboardImageImportApiRequest {
    idempotency_key: String,
    mime: String,
    content_hash: String,
    byte_len: i64,
    artifact_ref: String,
    source_application: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UrlImageImportApiRequest {
    idempotency_key: String,
    source_url: String,
    expected_mime: Option<String>,
    source_label: Option<String>,
    capability_profile_id: String,
    capability_grant_ref: String,
}

async fn import_clipboard_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ClipboardImageImportApiRequest>,
) -> Result<(StatusCode, Json<ImageImportRecord>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let record = store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: payload.idempotency_key,
            mime: payload.mime,
            content_hash: payload.content_hash,
            byte_len: payload.byte_len,
            artifact_ref: payload.artifact_ref,
            source_application: payload.source_application,
            requested_by: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/image-import/clipboard",
        status = "created",
        actor = %actor,
        import_id = %record.import_id,
        "import clipboard image"
    );

    Ok((StatusCode::CREATED, Json(record)))
}

async fn record_url_image_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UrlImageImportApiRequest>,
) -> Result<(StatusCode, Json<ImageImportRecord>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let record = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: payload.idempotency_key,
            source_url: payload.source_url,
            expected_mime: payload.expected_mime,
            source_label: payload.source_label,
            capability_profile_id: payload.capability_profile_id,
            capability_grant_ref: payload.capability_grant_ref,
            requested_by: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/image-import/url",
        status = "created",
        actor = %actor,
        import_id = %record.import_id,
        "record URL image import"
    );

    Ok((StatusCode::CREATED, Json(record)))
}

#[derive(Debug, Serialize)]
struct IntakeLaneCountsResponse {
    pending: i64,
    accepted: i64,
    rejected: i64,
    deferred: i64,
    skipped: i64,
    failed: i64,
}

impl From<IntakeLaneCounts> for IntakeLaneCountsResponse {
    fn from(c: IntakeLaneCounts) -> Self {
        Self {
            pending: c.pending,
            accepted: c.accepted,
            rejected: c.rejected,
            deferred: c.deferred,
            skipped: c.skipped,
            failed: c.failed,
        }
    }
}

#[derive(Debug, Serialize)]
struct IntakeItemResponse {
    item_id: Uuid,
    source_path: String,
    file_name: String,
    lane: String,
    byte_len: i64,
    loom_block_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct IntakeBatchItemsResponse {
    lane_counts: IntakeLaneCountsResponse,
    items: Vec<IntakeItemResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateIntakeItemRequest {
    source_path: String,
    file_name: String,
    byte_len: i64,
    content_hash: Option<String>,
}

/// POST /atelier/intake/batches/:batch_id/items — register one source item.
/// Replaying the same `(batch_id, source_path)` converges on the canonical item.
async fn create_intake_item(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<CreateIntakeItemRequest>,
) -> Result<(StatusCode, Json<IntakeItemResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let item = atelier_store(&state)
        .add_intake_item(
            batch_id,
            &NewIntakeItem {
                source_path: payload.source_path,
                file_name: payload.file_name,
                byte_len: payload.byte_len,
                content_hash: payload.content_hash,
            },
        )
        .await
        .map_err(atelier_error)?;
    let response = IntakeItemResponse {
        item_id: item.item_id,
        source_path: item.source_path,
        file_name: item.file_name,
        lane: item.lane.as_str().to_owned(),
        byte_len: item.byte_len,
        loom_block_id: None,
    };

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/batches/:batch_id/items",
        status = "created",
        actor = %actor,
        batch_id = %batch_id,
        item_id = %response.item_id,
        "register intake batch item"
    );

    Ok((StatusCode::CREATED, Json(response)))
}

#[derive(SurrealValue)]
struct IntakeProjectionBinding {
    batch_ref: RecordId,
    limit: i64,
}

#[derive(SurrealValue)]
struct IntakeProjectionRow {
    item_id: SurrealUuid,
    loom_block_id: String,
}

/// GET /atelier/intake/batches/:batch_id/items — lane counts + items for a batch.
async fn list_intake_batch_items(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<IntakeBatchItemsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);

    let lane_counts = store
        .intake_lane_counts(batch_id)
        .await
        .map_err(atelier_error)?;

    let projections: Vec<IntakeProjectionRow> = store
        .with_data(move |ctx| {
            Box::pin(async move {
                ctx.query_values(
                    "SELECT record::id(item_id) AS item_id, \
                     record::id(loom_block_id) AS loom_block_id \
                     FROM atelier_intake_item_loom_projection \
                     WHERE item_id IN (SELECT VALUE id FROM atelier_intake_item \
                     WHERE batch_id = $batch_ref \
                     ORDER BY created_at_utc ASC LIMIT $limit);",
                    IntakeProjectionBinding {
                        batch_ref: RecordId::new(
                            "atelier_intake_batch",
                            SurrealUuid::from(batch_id),
                        ),
                        limit: LIST_CAP,
                    },
                )
                .await
            })
        })
        .await
        .map_err(atelier_error)?;
    let projections = projections
        .into_iter()
        .map(|row| (Uuid::from(row.item_id), row.loom_block_id))
        .collect::<std::collections::HashMap<_, _>>();
    let items = store
        .list_intake_items_limited(batch_id, None, LIST_CAP)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(|item| IntakeItemResponse {
            loom_block_id: projections.get(&item.item_id).cloned(),
            item_id: item.item_id,
            source_path: item.source_path,
            file_name: item.file_name,
            lane: item.lane.as_str().to_owned(),
            byte_len: item.byte_len,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches/:batch_id/items", status = "ok", batch_id = %batch_id, "list intake batch items");

    Ok(Json(IntakeBatchItemsResponse {
        lane_counts: lane_counts.into(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkIntakeItemLoomProjectionRequest {
    loom_block_id: String,
}

/// PUT /atelier/intake/items/:item_id/loom-projection — publish the durable
/// canonical Loom identity consumed by editor/canvas drag payloads.
async fn link_intake_item_loom_projection(
    State(state): State<AppState>,
    Path(item_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<LinkIntakeItemLoomProjectionRequest>,
) -> Result<Json<IntakeItemLoomProjection>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let projection = atelier_store(&state)
        .link_intake_item_loom_projection(item_id, &payload.loom_block_id, &actor)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/items/:item_id/loom-projection",
        status = "ok",
        item_id = %item_id,
        loom_block_id = %projection.loom_block_id,
        "linked intake item Loom projection"
    );
    Ok(Json(projection))
}

#[derive(Debug, Serialize)]
struct CommandCorpusEntryResponse {
    entry_id: Uuid,
    action_id: String,
    owner: String,
    execution_class: String,
    foreground_flag: bool,
    manual_anchor: String,
}

/// GET /atelier/command-corpus — catalog descriptors ordered by action_id, capped.
async fn list_command_corpus(
    State(state): State<AppState>,
) -> Result<Json<Vec<CommandCorpusEntryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let out = atelier_store(&state)
        .list_command_corpus_entries_limited(None, LIST_CAP)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(|entry| CommandCorpusEntryResponse {
            entry_id: entry.entry_id,
            action_id: entry.action_id,
            owner: entry.owner,
            execution_class: entry.execution_class.as_token().to_owned(),
            foreground_flag: entry.foreground_flag,
            manual_anchor: entry.manual_anchor,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/command-corpus", status = "ok", "list command corpus");

    Ok(Json(out))
}

#[derive(Debug, Serialize)]
struct AiTagSuggestionResponse {
    suggestion_id: Uuid,
    character_internal_id: Uuid,
    asset_id: Option<Uuid>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
    status: String,
    decided_by: Option<String>,
    decision_reason: Option<String>,
    applied_tag_id: Option<Uuid>,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RecordAiTagSuggestionRequest {
    character_internal_id: Uuid,
    asset_id: Option<Uuid>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
}

#[derive(Debug, Deserialize)]
struct AiTagSuggestionDecisionRequest {
    reason: Option<String>,
}

fn ai_tag_suggestion_status_token(status: AiTagSuggestionStatus) -> &'static str {
    match status {
        AiTagSuggestionStatus::Proposed => "proposed",
        AiTagSuggestionStatus::Accepted => "accepted",
        AiTagSuggestionStatus::Rejected => "rejected",
        AiTagSuggestionStatus::Applied => "applied",
    }
}

fn ai_tag_suggestion_response(suggestion: AiTagSuggestion) -> AiTagSuggestionResponse {
    AiTagSuggestionResponse {
        suggestion_id: suggestion.suggestion_id,
        character_internal_id: suggestion.character_internal_id,
        asset_id: suggestion.asset_id,
        tag_text: suggestion.tag_text,
        confidence: suggestion.confidence,
        model_receipt_ref: suggestion.model_receipt_ref,
        tool_receipt_ref: suggestion.tool_receipt_ref,
        suggested_by: suggestion.suggested_by,
        status: ai_tag_suggestion_status_token(suggestion.status).to_string(),
        decided_by: suggestion.decided_by,
        decision_reason: suggestion.decision_reason,
        applied_tag_id: suggestion.applied_tag_id,
        created_at_utc: suggestion.created_at_utc,
        updated_at_utc: suggestion.updated_at_utc,
    }
}

async fn record_ai_tag_suggestion(
    State(state): State<AppState>,
    Json(payload): Json<RecordAiTagSuggestionRequest>,
) -> Result<(StatusCode, Json<AiTagSuggestionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let suggestion = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: payload.character_internal_id,
            asset_id: payload.asset_id,
            tag_text: payload.tag_text,
            confidence: payload.confidence,
            model_receipt_ref: payload.model_receipt_ref,
            tool_receipt_ref: payload.tool_receipt_ref,
            suggested_by: payload.suggested_by,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions",
        status = "created",
        suggestion_id = %suggestion.suggestion_id,
        "record AI tag suggestion"
    );

    Ok((
        StatusCode::CREATED,
        Json(ai_tag_suggestion_response(suggestion)),
    ))
}

async fn list_ai_tag_suggestions_for_character(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
) -> Result<Json<Vec<AiTagSuggestionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let suggestions = store
        .list_ai_tag_suggestions_for_character(character_internal_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/characters/:character_internal_id",
        status = "ok",
        character_internal_id = %character_internal_id,
        "list AI tag suggestions"
    );

    Ok(Json(
        suggestions
            .into_iter()
            .map(ai_tag_suggestion_response)
            .collect(),
    ))
}

async fn accept_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
    Json(payload): Json<AiTagSuggestionDecisionRequest>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .accept_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id,
            decided_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/accept",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "accept AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
}

async fn reject_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
    Json(payload): Json<AiTagSuggestionDecisionRequest>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .reject_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id,
            decided_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/reject",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "reject AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
}

async fn apply_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .apply_ai_tag_suggestion(suggestion_id, &actor)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/apply",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "apply AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
}

#[derive(Debug, Serialize)]
struct StealthWindowResponse {
    window_ref_id: Uuid,
    owner_actor: String,
    title: String,
    visibility: String,
    status: String,
    revision: i64,
}

/// GET /atelier/stealth/windows — registry entries visible to the calling actor,
/// newest first, capped.
async fn list_stealth_windows(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<StealthWindowResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let windows = store
        .list_stealth_windows(&actor, None, LIST_CAP)
        .await
        .map_err(atelier_error)?;

    let out = windows
        .into_iter()
        .map(|window| StealthWindowResponse {
            window_ref_id: window.window_ref_id,
            owner_actor: window.owner_actor,
            title: window.title,
            visibility: window.visibility.as_token().to_string(),
            status: window.status.as_token().to_string(),
            revision: window.revision,
        })
        .collect();

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/stealth/windows",
        status = "ok",
        actor = %actor,
        "list stealth windows"
    );

    Ok(Json(out))
}

/// GET /atelier/stealth/windows/:window_ref_id/refs/:ref_id — governed,
/// redacted single-reference view. This is a read-only projection over
/// durable authority and never includes raw payload fields.
async fn resolve_stealth_ref(
    State(state): State<AppState>,
    Path((window_ref_id, ref_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ResolvedContentRef>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let resolved = store
        .resolve_stealth_ref(window_ref_id, ref_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/stealth/windows/:window_ref_id/refs/:ref_id",
        status = "ok",
        window_ref_id = %window_ref_id,
        ref_id = %ref_id,
        "resolve stealth ref"
    );

    Ok(Json(resolved))
}
