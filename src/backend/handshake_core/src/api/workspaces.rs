use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use uuid::Uuid;

use crate::ace::validators::atelier_scope::{
    apply_selection_bounded_patchsets, sha256_hex, DocPatchsetV1, SelectionRangeV1,
};
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::{
    diagnostics::{
        DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface, LinkConfidence,
    },
    models::{
        BlockResponse, CreateDocumentRequest, CreateWorkspaceRequest, DocumentResponse,
        DocumentWithBlocksResponse, ErrorResponse, UpsertBlocksRequest, WorkspaceResponse,
    },
    storage::{
        Block, NewBlock, NewDocument, NewWorkspace, StorageError, WriteActorKind, WriteContext,
    },
    AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/workspaces", post(create_workspace).get(list_workspaces))
        .route(
            "/workspaces/:workspace_id/documents",
            post(create_document).get(list_documents),
        )
        .route(
            "/documents/:document_id",
            get(get_document).delete(delete_document),
        )
        .route("/documents/:document_id/blocks", put(replace_blocks))
        .route(
            "/documents/:document_id/atelier/apply",
            post(apply_atelier_patchsets),
        )
        .route("/atelier/roles", get(list_atelier_roles))
        .route("/workspaces/:workspace_id", delete(delete_workspace))
        .with_state(state)
}

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_JOB_ID: &str = "x-hsk-job-id";
const HSK_HEADER_WORKFLOW_ID: &str = "x-hsk-workflow-id";

fn is_silent_edit(err: &StorageError) -> bool {
    matches!(
        err,
        StorageError::Guard("HSK-403-SILENT-EDIT")
            | StorageError::Validation("HSK-403-SILENT-EDIT")
    )
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn parse_actor_kind(raw: Option<&str>) -> Result<WriteActorKind, StorageError> {
    let Some(value) = raw else {
        return Ok(WriteActorKind::Human);
    };

    let normalized = value.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "HUMAN" => Ok(WriteActorKind::Human),
        "AI" => Ok(WriteActorKind::Ai),
        "SYSTEM" => Ok(WriteActorKind::System),
        _ => Err(StorageError::Validation("invalid_actor_kind")),
    }
}

fn parse_uuid(raw: Option<&str>) -> Option<Uuid> {
    raw.and_then(|value| Uuid::parse_str(value.trim()).ok())
}

#[derive(Debug, Deserialize)]
struct AtelierApplyRequestV1 {
    pub doc_id: String,
    pub selection: SelectionRangeV1,
    pub suggestions_to_apply: Vec<AtelierSuggestionToApplyV1>,
}

#[derive(Debug, Deserialize)]
struct AtelierSuggestionToApplyV1 {
    pub role_id: String,
    pub suggestion_id: String,
    pub patchset: DocPatchsetV1,
}

#[derive(Debug, Deserialize)]
struct RolePackV1 {
    pub roles: Vec<RolePackRoleV1>,
}

#[derive(Debug, Deserialize)]
struct RolePackRoleV1 {
    pub role_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct AtelierRolesResponseV1 {
    pub roles: Vec<AtelierRoleSummaryV1>,
}

#[derive(Debug, Serialize)]
struct AtelierRoleSummaryV1 {
    pub role_id: String,
    pub display_name: String,
}

async fn list_atelier_roles(
    State(_state): State<AppState>,
) -> Result<Json<AtelierRolesResponseV1>, (StatusCode, Json<ErrorResponse>)> {
    let repo_root = crate::api::paths::repo_root().map_err(internal_error)?;
    let rolepack_path = repo_root
        .join("assets")
        .join("atelier_rolepack_digital_production_studio_v1.json");

    let raw = fs::read_to_string(&rolepack_path).map_err(internal_error)?;
    let parsed: RolePackV1 = serde_json::from_str(&raw).map_err(internal_error)?;

    let roles = parsed
        .roles
        .into_iter()
        .map(|role| AtelierRoleSummaryV1 {
            role_id: role.role_id.clone(),
            display_name: role.display_name.unwrap_or(role.role_id),
        })
        .collect();

    Ok(Json(AtelierRolesResponseV1 { roles }))
}

async fn record_silent_edit_diagnostic(
    state: &AppState,
    headers: &HeaderMap,
    wsid_hint: Option<&str>,
    ctx_hint: Option<&WriteContext>,
    err: &StorageError,
    route_tag: &'static str,
) {
    if !is_silent_edit(err) {
        return;
    }

    let ctx_job_id = ctx_hint.and_then(|ctx| ctx.job_id);
    let header_job_id = parse_uuid(header_str(headers, HSK_HEADER_JOB_ID));
    let job_id = ctx_job_id.or(header_job_id).map(|id| id.to_string());

    let ctx_workflow_id = ctx_hint.and_then(|ctx| ctx.workflow_id);
    let header_workflow_id = parse_uuid(header_str(headers, HSK_HEADER_WORKFLOW_ID));
    let workflow_id = ctx_workflow_id.or(header_workflow_id);

    let missing_context = ctx_hint.is_some_and(|ctx| {
        ctx.actor_kind == WriteActorKind::Ai && (ctx.job_id.is_none() || ctx.workflow_id.is_none())
    });

    let failure_mode_tag = if missing_context {
        "silent_edit:missing_context"
    } else {
        "silent_edit:context_invalid"
    };

    let message = if missing_context {
        "AI write rejected by StorageGuard: missing required job/workflow context."
    } else {
        "AI write rejected by StorageGuard: job/workflow context invalid."
    };

    let mut tags = vec![
        "hsk:guard".to_string(),
        "hsk:silent_edit".to_string(),
        failure_mode_tag.to_string(),
        format!("route:{}", route_tag),
    ];
    if let Some(workflow_id) = workflow_id {
        tags.push(format!("workflow_id:{}", workflow_id));
    }

    let input = DiagnosticInput {
        title: "No Silent Edits: StorageGuard blocked AI write".to_string(),
        message: message.to_string(),
        severity: DiagnosticSeverity::Error,
        source: DiagnosticSource::Engine,
        surface: DiagnosticSurface::System,
        tool: Some("storage_guard".to_string()),
        code: Some("HSK-403-SILENT-EDIT".to_string()),
        tags: Some(tags),
        wsid: wsid_hint.map(str::to_string),
        job_id,
        model_id: None,
        actor: None,
        capability_id: None,
        policy_decision_id: None,
        locations: None,
        evidence_refs: None,
        link_confidence: LinkConfidence::Unlinked,
        status: None,
        count: None,
        first_seen: None,
        last_seen: None,
        timestamp: None,
        updated_at: None,
    };

    let diagnostic = match input.into_diagnostic() {
        Ok(diagnostic) => diagnostic,
        Err(error) => {
            tracing::error!(
                target: "handshake_core",
                route = route_tag,
                error = %error,
                "failed to build silent-edit diagnostic"
            );
            return;
        }
    };

    if let Err(error) = state.diagnostics.record_diagnostic(diagnostic).await {
        tracing::error!(
            target: "handshake_core",
            route = route_tag,
            error = %error,
            "failed to record silent-edit diagnostic"
        );
    }
}

async fn write_context_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<WriteContext, StorageError> {
    let actor_kind = parse_actor_kind(header_str(headers, HSK_HEADER_ACTOR_KIND))?;
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID).map(ToOwned::to_owned);

    match actor_kind {
        WriteActorKind::Human => Ok(WriteContext::human(actor_id)),
        WriteActorKind::System => Ok(WriteContext::system(actor_id)),
        WriteActorKind::Ai => {
            let job_id = parse_uuid(header_str(headers, HSK_HEADER_JOB_ID));
            let workflow_id = parse_uuid(header_str(headers, HSK_HEADER_WORKFLOW_ID));

            let (job_id, workflow_id) = match (job_id, workflow_id) {
                (Some(job_id), Some(workflow_id)) => (job_id, workflow_id),
                (job_id, workflow_id) => {
                    return Ok(WriteContext::ai(actor_id, job_id, workflow_id))
                }
            };

            let job = state.storage.get_ai_job(&job_id.to_string()).await;
            match job {
                Ok(job) => {
                    if job.workflow_run_id != Some(workflow_id) {
                        return Err(StorageError::Guard("HSK-403-SILENT-EDIT"));
                    }
                }
                Err(StorageError::NotFound(_)) => {
                    return Err(StorageError::Guard("HSK-403-SILENT-EDIT"));
                }
                Err(err) => return Err(err),
            }

            Ok(WriteContext::ai(actor_id, Some(job_id), Some(workflow_id)))
        }
    }
}

fn offset_to_line_col(text: &str, offset: usize) -> Result<(usize, usize), StorageError> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(StorageError::Validation("invalid_selection_offset"));
    }

    let prefix = &text[..offset];
    let mut line = 1usize;
    let mut last_newline = None;
    for (idx, ch) in prefix.char_indices() {
        if ch == '\n' {
            line += 1;
            last_newline = Some(idx);
        }
    }

    let col_start = match last_newline {
        Some(idx) => idx + 1,
        None => 0,
    };
    let col = text[col_start..offset].chars().count() + 1;
    Ok((line, col))
}

async fn record_atelier_scope_violation_diagnostic(
    state: &AppState,
    doc: &crate::storage::Document,
    message: &str,
) {
    let input = DiagnosticInput {
        title: "Atelier selection scope violation".to_string(),
        message: message.to_string(),
        severity: DiagnosticSeverity::Error,
        source: DiagnosticSource::Engine,
        surface: DiagnosticSurface::System,
        tool: Some("atelier_scope".to_string()),
        code: Some("ATELIER-LENS-VAL-SCOPE-001".to_string()),
        tags: Some(vec!["hsk:atelier".to_string(), "hsk:scope".to_string()]),
        wsid: Some(doc.workspace_id.clone()),
        job_id: None,
        model_id: None,
        actor: None,
        capability_id: None,
        policy_decision_id: None,
        locations: None,
        evidence_refs: None,
        link_confidence: LinkConfidence::Unlinked,
        status: None,
        count: None,
        first_seen: None,
        last_seen: None,
        timestamp: None,
        updated_at: None,
    };

    let diagnostic = match input.into_diagnostic() {
        Ok(diagnostic) => diagnostic,
        Err(error) => {
            tracing::error!(target: "handshake_core", error = %error, "failed to build atelier scope diagnostic");
            return;
        }
    };

    if let Err(error) = state.diagnostics.record_diagnostic(diagnostic).await {
        tracing::error!(target: "handshake_core", error = %error, "failed to record atelier scope diagnostic");
    }
}

async fn create_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(&state, &headers, None, None, &err, "/workspaces").await;
            return Err(map_storage_error(err));
        }
    };

    let workspace = match state
        .storage
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: payload.name.clone(),
            },
        )
        .await
    {
        Ok(workspace) => workspace,
        Err(err) => {
            record_silent_edit_diagnostic(&state, &headers, None, Some(&ctx), &err, "/workspaces")
                .await;
            return Err(map_storage_error(err));
        }
    };

    tracing::info!(target: "handshake_core", route = "/workspaces", status = "created", workspace_id = %workspace.id, "workspace created");

    Ok((
        StatusCode::CREATED,
        Json(WorkspaceResponse {
            id: workspace.id,
            name: workspace.name,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
        }),
    ))
}

async fn list_workspaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = state
        .storage
        .list_workspaces()
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces", status = "ok", count = rows.len(), "list workspaces");

    let workspaces = rows
        .into_iter()
        .map(|row| WorkspaceResponse {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(workspaces))
}

async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&workspace_id),
                None,
                &err,
                "/workspaces/:workspace_id",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    if let Err(err) = state.storage.delete_workspace(&ctx, &workspace_id).await {
        record_silent_edit_diagnostic(
            &state,
            &headers,
            Some(&workspace_id),
            Some(&ctx),
            &err,
            "/workspaces/:workspace_id",
        )
        .await;
        return Err(map_storage_error(err));
    }

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id", status = "deleted", workspace_id = %workspace_id, "workspace deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn create_document(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&workspace_id),
                None,
                &err,
                "/workspaces/:workspace_id/documents",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    let document = match state
        .storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace_id.clone(),
                title: payload.title.clone(),
            },
        )
        .await
    {
        Ok(document) => document,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&workspace_id),
                Some(&ctx),
                &err,
                "/workspaces/:workspace_id/documents",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/documents", status = "created", workspace_id = %workspace_id, document_id = %document.id, "document created");

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            id: document.id,
            workspace_id: document.workspace_id,
            title: document.title,
            created_at: document.created_at,
            updated_at: document.updated_at,
        }),
    ))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<DocumentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let rows = state
        .storage
        .list_documents(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/documents", status = "ok", workspace_id = %workspace_id, count = rows.len(), "list documents");

    let docs = rows
        .into_iter()
        .map(|row| DocumentResponse {
            id: row.id,
            workspace_id: row.workspace_id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(docs))
}

async fn get_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Json<DocumentWithBlocksResponse>, (StatusCode, Json<ErrorResponse>)> {
    let document = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    let blocks = state
        .storage
        .get_blocks(&document_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/documents/:document_id", status = "ok", document_id = %document.id, "get document");

    let blocks: Vec<BlockResponse> = blocks.into_iter().map(block_to_response).collect();

    Ok(Json(DocumentWithBlocksResponse {
        id: document.id,
        workspace_id: document.workspace_id,
        title: document.title,
        created_at: document.created_at,
        updated_at: document.updated_at,
        blocks,
    }))
}

async fn delete_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                None,
                None,
                &err,
                "/documents/:document_id",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    if let Err(err) = state.storage.delete_document(&ctx, &document_id).await {
        record_silent_edit_diagnostic(
            &state,
            &headers,
            None,
            Some(&ctx),
            &err,
            "/documents/:document_id",
        )
        .await;
        return Err(map_storage_error(err));
    }

    tracing::info!(target: "handshake_core", route = "/documents/:document_id", status = "deleted", document_id = %document_id, "document deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn replace_blocks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpsertBlocksRequest>,
) -> Result<Json<Vec<BlockResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Ensure document exists first to provide 404 instead of recreating.
    let doc = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&doc.workspace_id),
                None,
                &err,
                "/documents/:document_id/blocks",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    let incoming_blocks: Vec<NewBlock> = payload
        .blocks
        .into_iter()
        .map(|incoming| NewBlock {
            id: incoming.id,
            document_id: document_id.clone(),
            kind: incoming.kind,
            sequence: incoming.sequence,
            raw_content: incoming.raw_content.clone(),
            display_content: incoming.display_content,
            derived_content: incoming.derived_content,
            sensitivity: None,
            exportable: None,
        })
        .collect();

    let result_blocks = match state
        .storage
        .replace_blocks(&ctx, &document_id, incoming_blocks)
        .await
    {
        Ok(blocks) => blocks,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&doc.workspace_id),
                Some(&ctx),
                &err,
                "/documents/:document_id/blocks",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    tracing::info!(target: "handshake_core", route = "/documents/:document_id/blocks", status = "ok", document_id = %document_id, blocks = result_blocks.len(), "replace blocks");

    Ok(Json(
        result_blocks.into_iter().map(block_to_response).collect(),
    ))
}

async fn apply_atelier_patchsets(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<AtelierApplyRequestV1>,
) -> Result<Json<Vec<BlockResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let doc = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    if payload.doc_id != document_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "bad_request",
            }),
        ));
    }

    let ctx = match write_context_from_headers(&state, &headers).await {
        Ok(ctx) => ctx,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&doc.workspace_id),
                None,
                &err,
                "/documents/:document_id/atelier/apply",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    let blocks = state
        .storage
        .get_blocks(&document_id)
        .await
        .map_err(map_storage_error)?;

    let mut sorted_blocks = blocks;
    sorted_blocks.sort_by_key(|b| b.sequence);

    let doc_text_before = sorted_blocks
        .iter()
        .map(|b| b.raw_content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let patchsets: Vec<DocPatchsetV1> = payload
        .suggestions_to_apply
        .iter()
        .map(|s| s.patchset.clone())
        .collect();

    let doc_text_after = match apply_selection_bounded_patchsets(
        &doc_text_before,
        &payload.selection,
        patchsets.as_slice(),
    ) {
        Ok(value) => value,
        Err(err) => {
            record_atelier_scope_violation_diagnostic(&state, &doc, &err.to_string()).await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "ATELIER-LENS-VAL-SCOPE-001",
                }),
            ));
        }
    };

    let next_texts: Vec<&str> = doc_text_after.split('\n').collect();
    let mut incoming_blocks: Vec<NewBlock> = Vec::with_capacity(next_texts.len());
    for (idx, text) in next_texts.iter().enumerate() {
        let existing = sorted_blocks.get(idx);
        incoming_blocks.push(NewBlock {
            id: existing.map(|b| b.id.to_string()),
            document_id: document_id.clone(),
            kind: existing
                .map(|b| b.kind.clone())
                .unwrap_or_else(|| "paragraph".to_string()),
            sequence: idx as i64,
            raw_content: (*text).to_string(),
            display_content: Some((*text).to_string()),
            derived_content: None,
            sensitivity: None,
            exportable: None,
        });
    }

    let result_blocks = match state
        .storage
        .replace_blocks(&ctx, &document_id, incoming_blocks)
        .await
    {
        Ok(blocks) => blocks,
        Err(err) => {
            record_silent_edit_diagnostic(
                &state,
                &headers,
                Some(&doc.workspace_id),
                Some(&ctx),
                &err,
                "/documents/:document_id/atelier/apply",
            )
            .await;
            return Err(map_storage_error(err));
        }
    };

    // Emit FR-EVT-002 editor_edit (selection-scoped apply) with hashes, no raw text.
    let before_hash = sha256_hex(doc_text_before.as_bytes());
    let after_hash = sha256_hex(doc_text_after.as_bytes());
    let diff_hash = {
        let patch_json = serde_json::to_string(&patchsets).unwrap_or_default();
        sha256_hex(patch_json.as_bytes())
    };

    let (start_line, start_col) =
        offset_to_line_col(&doc_text_before, payload.selection.start_utf8)
            .map_err(map_storage_error)?;
    let (end_line, end_col) = offset_to_line_col(&doc_text_before, payload.selection.end_utf8)
        .map_err(map_storage_error)?;

    let payload = json!({
        "editor_surface": "monaco",
        "document_uri": format!("hsk://documents/{}", document_id),
        "path": null,
        "before_hash": before_hash,
        "after_hash": after_hash,
        "diff_hash": diff_hash,
        "ops": [
            {
                "range": {
                    "startLine": start_line,
                    "startColumn": start_col,
                    "endLine": end_line,
                    "endColumn": end_col
                }
            }
        ]
    });

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::EditorEdit,
        FlightRecorderActor::Human,
        Uuid::new_v4(),
        payload,
    )
    .with_wsids(vec![doc.workspace_id.clone()]);

    state
        .flight_recorder
        .record_event(event)
        .await
        .map_err(internal_error)?;

    Ok(Json(
        result_blocks.into_iter().map(block_to_response).collect(),
    ))
}

fn block_to_response(block: Block) -> BlockResponse {
    BlockResponse {
        id: block.id,
        kind: block.kind,
        sequence: block.sequence,
        raw_content: block.raw_content,
        display_content: block.display_content,
        derived_content: block.derived_content,
        created_at: block.created_at,
        updated_at: block.updated_at,
    }
}

async fn ensure_workspace_exists(
    state: &AppState,
    workspace_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

fn map_storage_error(err: StorageError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Guard(_) | StorageError::Validation("HSK-403-SILENT-EDIT") => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-SILENT-EDIT",
            }),
        ),
        StorageError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "bad_request",
            }),
        ),
        _ => internal_error(err),
    }
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(target: "handshake_core", error = %err, "db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

fn not_found(code: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::diagnostics::DiagFilter;
    use crate::flight_recorder::{
        duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorderEventType,
    };
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{
        sqlite::SqliteDatabase, AccessMode, Database, EntityRef, JobKind, JobMetrics, JobState,
        JobStatusUpdate, NewAiJob, PlannedOperation, SafetyMode,
    };
    use axum::extract::{Path, State};
    use serde_json::json;
    use std::sync::Arc;

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

    #[tokio::test]
    async fn replace_blocks_rejects_ai_when_context_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let seed_ctx = WriteContext::human(Some("tester".into()));
        let workspace = state
            .storage
            .create_workspace(&seed_ctx, NewWorkspace { name: "w1".into() })
            .await?;
        let document = state
            .storage
            .create_document(
                &seed_ctx,
                NewDocument {
                    workspace_id: workspace.id,
                    title: "Doc".into(),
                },
            )
            .await?;

        let payload = UpsertBlocksRequest {
            blocks: vec![crate::models::IncomingBlock {
                id: None,
                kind: "paragraph".into(),
                sequence: 0,
                raw_content: "hello".into(),
                display_content: None,
                derived_content: None,
            }],
        };

        let mut headers = HeaderMap::new();
        headers.insert(HSK_HEADER_ACTOR_KIND, "AI".parse()?);

        let result = replace_blocks(
            State(state.clone()),
            Path(document.id),
            headers,
            Json(payload),
        )
        .await;

        let Err((status, Json(err))) = result else {
            unreachable!("expected replace_blocks to be rejected");
        };
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(err.error, "HSK-403-SILENT-EDIT");

        let diagnostics = state
            .diagnostics
            .list_diagnostics(DiagFilter::default())
            .await?;
        let silent_edit_diag = diagnostics
            .into_iter()
            .find(|diag| diag.code.as_deref() == Some("HSK-403-SILENT-EDIT"))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "expected a recorded diagnostic for HSK-403-SILENT-EDIT",
                )
            })?;

        let diagnostic_id = silent_edit_diag.id.to_string();
        let events = state
            .flight_recorder
            .list_events(EventFilter::default())
            .await?;
        let maybe_event = events.iter().find(|event| {
            event.event_type == FlightRecorderEventType::Diagnostic
                && event.payload.get("diagnostic_id").and_then(|v| v.as_str())
                    == Some(diagnostic_id.as_str())
        });
        assert!(
            maybe_event.is_some(),
            "expected FR-EVT-003 Diagnostic event with payload.diagnostic_id matching the recorded diagnostic"
        );
        if let Some(event) = maybe_event {
            assert!(
                event.payload.get("title").is_none() && event.payload.get("message").is_none(),
                "FR-EVT-003 payload must not duplicate full diagnostic fields"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn replace_blocks_accepts_ai_and_persists_traceability(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let seed_ctx = WriteContext::human(Some("tester".into()));
        let workspace = state
            .storage
            .create_workspace(&seed_ctx, NewWorkspace { name: "w1".into() })
            .await?;
        let document = state
            .storage
            .create_document(
                &seed_ctx,
                NewDocument {
                    workspace_id: workspace.id,
                    title: "Doc".into(),
                },
            )
            .await?;

        let job = state
            .storage
            .create_ai_job(NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::WorkflowRun,
                protocol_id: "p1".into(),
                profile_id: "profile1".into(),
                capability_profile_id: "cap1".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: vec![EntityRef {
                    entity_id: document.id.clone(),
                    entity_kind: "document".into(),
                }],
                planned_operations: vec![PlannedOperation {
                    op_type: crate::storage::OperationType::Write,
                    target: EntityRef {
                        entity_id: document.id.clone(),
                        entity_kind: "document".into(),
                    },
                    description: None,
                }],
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({"input": true})),
            })
            .await?;
        let run = state
            .storage
            .create_workflow_run(job.job_id, JobState::Queued, None)
            .await?;
        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.job_id,
                state: JobState::Running,
                error_message: None,
                status_reason: "running".into(),
                metrics: None,
                workflow_run_id: Some(run.id),
                trace_id: None,
                job_outputs: None,
            })
            .await?;

        let payload = UpsertBlocksRequest {
            blocks: vec![crate::models::IncomingBlock {
                id: None,
                kind: "paragraph".into(),
                sequence: 0,
                raw_content: "hello".into(),
                display_content: None,
                derived_content: None,
            }],
        };

        let mut headers = HeaderMap::new();
        headers.insert(HSK_HEADER_ACTOR_KIND, "AI".parse()?);
        headers.insert(HSK_HEADER_JOB_ID, job.job_id.to_string().parse()?);
        headers.insert(HSK_HEADER_WORKFLOW_ID, run.id.to_string().parse()?);

        let result = replace_blocks(
            State(state.clone()),
            Path(document.id.clone()),
            headers,
            Json(payload),
        )
        .await;
        assert!(
            result.is_ok(),
            "expected replace_blocks to accept a valid AI write context"
        );

        let blocks = state.storage.get_blocks(&document.id).await?;
        assert_eq!(blocks.len(), 1, "expected one inserted block");

        Ok(())
    }
}
