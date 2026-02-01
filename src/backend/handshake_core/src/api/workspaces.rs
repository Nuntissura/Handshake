use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

use crate::ace::validators::atelier_scope::{
    apply_selection_bounded_patchsets, sha256_hex, AtelierScopeError, DocPatchsetV1,
    SelectionRangeV1,
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
        Block, JobKind, JobState, NewBlock, NewDocument, NewWorkspace, StorageError,
        WriteActorKind, WriteContext,
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

const ATELIER_ROLE_SUGGESTIONS_SCHEMA_V1: &str = "hsk.atelier.role_suggestions@v1";
const ERR_ATELIER_STALE_SELECTION: &str = "HSK-409-ATELIER-STALE-SELECTION";
const ERR_ATELIER_PROVENANCE_MISMATCH: &str = "HSK-403-ATELIER-PROVENANCE-MISMATCH";

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

fn expected_contract_id(role_id: &str) -> String {
    format!("ROLE:{role_id}:C:1")
}

fn bad_request_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "bad_request",
        }),
    )
}

fn atelier_stale_selection_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: ERR_ATELIER_STALE_SELECTION,
        }),
    )
}

fn atelier_provenance_mismatch_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: ERR_ATELIER_PROVENANCE_MISMATCH,
        }),
    )
}

async fn verify_atelier_applied_suggestion_v1(
    state: &AppState,
    document_id: &str,
    payload_selection: &SelectionRangeV1,
    incoming: &AtelierSuggestionToApplyV1,
) -> Result<VerifiedAppliedSuggestionV1, (StatusCode, Json<ErrorResponse>)> {
    if incoming.role_id.trim().is_empty()
        || incoming.suggestion_id.trim().is_empty()
        || incoming.source_job_id.trim().is_empty()
    {
        return Err(bad_request_error());
    }

    let source_job_uuid = match Uuid::parse_str(incoming.source_job_id.trim()) {
        Ok(uuid) => uuid,
        Err(_) => return Err(bad_request_error()),
    };

    let job = match state.storage.get_ai_job(&source_job_uuid.to_string()).await {
        Ok(job) => job,
        Err(StorageError::NotFound(_)) => return Err(bad_request_error()),
        Err(err) => return Err(map_storage_error(err)),
    };

    if job.job_kind != JobKind::DocEdit {
        return Err(bad_request_error());
    }

    if job.state != JobState::Completed {
        return Err(bad_request_error());
    }

    let outputs = job.job_outputs.ok_or_else(bad_request_error)?;
    let parsed: AtelierRoleSuggestionsJobOutputV1 =
        serde_json::from_value(outputs).map_err(|_| bad_request_error())?;

    if parsed.schema_version != ATELIER_ROLE_SUGGESTIONS_SCHEMA_V1 {
        return Err(bad_request_error());
    }

    if parsed.doc_id != document_id {
        return Err(atelier_provenance_mismatch_error());
    }

    if parsed.selection != *payload_selection {
        return Err(atelier_stale_selection_error());
    }

    let mut matched: Option<&AtelierRoleSuggestionV1> = None;
    for by_role in parsed.by_role.iter() {
        if by_role.role_id != incoming.role_id {
            continue;
        }
        for suggestion in by_role.suggestions.iter() {
            if suggestion.suggestion_id == incoming.suggestion_id {
                matched = Some(suggestion);
                break;
            }
        }
        if matched.is_some() {
            break;
        }
    }

    let matched = match matched {
        Some(value) => value,
        None => return Err(atelier_provenance_mismatch_error()),
    };

    if matched.role_id != incoming.role_id {
        return Err(atelier_provenance_mismatch_error());
    }

    if matched.patchset != incoming.patchset {
        return Err(atelier_provenance_mismatch_error());
    }

    let expected_contract_id = expected_contract_id(&incoming.role_id);
    match matched
        .contract_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(value) if value == expected_contract_id.as_str() => {}
        _ => return Err(atelier_provenance_mismatch_error()),
    }

    let suggested_job_uuid = match Uuid::parse_str(matched.source_job_id.trim()) {
        Ok(uuid) => uuid,
        Err(_) => return Err(atelier_provenance_mismatch_error()),
    };
    if suggested_job_uuid != source_job_uuid || suggested_job_uuid != job.job_id {
        return Err(atelier_provenance_mismatch_error());
    }

    let suggested_trace_uuid = match Uuid::parse_str(matched.source_trace_id.trim()) {
        Ok(uuid) => uuid,
        Err(_) => return Err(atelier_provenance_mismatch_error()),
    };
    if suggested_trace_uuid != job.trace_id {
        return Err(atelier_provenance_mismatch_error());
    }

    if matched.protocol_id != job.protocol_id {
        return Err(atelier_provenance_mismatch_error());
    }

    if matched.source_model_id.trim().is_empty() {
        return Err(atelier_provenance_mismatch_error());
    }

    Ok(VerifiedAppliedSuggestionV1 {
        role_id: incoming.role_id.clone(),
        contract_id: expected_contract_id,
        suggestion_id: incoming.suggestion_id.clone(),
        patchset: matched.patchset.clone(),
        protocol_id: job.protocol_id.clone(),
        source_job_id: source_job_uuid.to_string(),
        source_trace_id: job.trace_id.to_string(),
        source_model_id: matched.source_model_id.clone(),
    })
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
    pub source_job_id: String,
    pub patchset: DocPatchsetV1,
}

#[derive(Debug, Deserialize)]
struct AtelierRoleSuggestionsJobOutputV1 {
    pub schema_version: String,
    pub doc_id: String,
    pub selection: SelectionRangeV1,
    pub by_role: Vec<AtelierRoleSuggestionsByRoleV1>,
}

#[derive(Debug, Deserialize)]
struct AtelierRoleSuggestionsByRoleV1 {
    pub role_id: String,
    pub suggestions: Vec<AtelierRoleSuggestionV1>,
}

#[derive(Debug, Deserialize)]
struct AtelierRoleSuggestionV1 {
    pub suggestion_id: String,
    pub role_id: String,
    #[serde(default)]
    pub contract_id: Option<String>,
    pub patchset: DocPatchsetV1,
    pub protocol_id: String,
    pub source_job_id: String,
    pub source_trace_id: String,
    pub source_model_id: String,
}

#[derive(Debug, Clone)]
struct VerifiedAppliedSuggestionV1 {
    pub role_id: String,
    pub contract_id: String,
    pub suggestion_id: String,
    pub patchset: DocPatchsetV1,
    pub protocol_id: String,
    pub source_job_id: String,
    pub source_trace_id: String,
    pub source_model_id: String,
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
    job_id: Option<String>,
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

    if payload.suggestions_to_apply.is_empty() {
        return Err(bad_request_error());
    }

    let mut verified_suggestions: Vec<VerifiedAppliedSuggestionV1> =
        Vec::with_capacity(payload.suggestions_to_apply.len());
    for incoming in payload.suggestions_to_apply.iter() {
        let verified = verify_atelier_applied_suggestion_v1(
            &state,
            document_id.as_str(),
            &payload.selection,
            incoming,
        )
        .await?;
        verified_suggestions.push(verified);
    }

    let patchsets: Vec<DocPatchsetV1> = verified_suggestions
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
            let message = err.to_string();
            match err {
                AtelierScopeError::ScopeViolation(_) => {
                    let job_id_hint = payload
                        .suggestions_to_apply
                        .first()
                        .map(|s| s.source_job_id.trim().to_string());
                    record_atelier_scope_violation_diagnostic(&state, &doc, job_id_hint, &message)
                        .await;
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            error: "ATELIER-LENS-VAL-SCOPE-001",
                        }),
                    ));
                }
                AtelierScopeError::HashMismatch(_) => return Err(atelier_stale_selection_error()),
                AtelierScopeError::InvalidSelection(_) | AtelierScopeError::InvalidPatchset(_) => {
                    return Err(bad_request_error())
                }
            }
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

    let after_selection_len = {
        let prefix_len = payload.selection.start_utf8;
        let suffix_len = doc_text_before
            .len()
            .saturating_sub(payload.selection.end_utf8);
        doc_text_after
            .len()
            .saturating_sub(prefix_len.saturating_add(suffix_len))
    };
    let after_end_utf8 = payload
        .selection
        .start_utf8
        .saturating_add(after_selection_len);

    let (after_start_line, after_start_col) =
        offset_to_line_col(&doc_text_after, payload.selection.start_utf8)
            .map_err(map_storage_error)?;
    let (after_end_line, after_end_col) =
        offset_to_line_col(&doc_text_after, after_end_utf8).map_err(map_storage_error)?;

    let applied_suggestions: Vec<Value> = verified_suggestions
        .iter()
        .map(|s| {
            json!({
                "role_id": s.role_id.as_str(),
                "contract_id": s.contract_id.as_str(),
                "suggestion_id": s.suggestion_id.as_str(),
                "source_job_id": s.source_job_id.as_str(),
                "source_trace_id": s.source_trace_id.as_str(),
                "source_model_id": s.source_model_id.as_str(),
                "source_tool_id": null,
                "protocol_id": s.protocol_id.as_str(),
                "evidence_refs": [],
                "before_span": {
                    "start_line": start_line,
                    "start_col": start_col,
                    "end_line": end_line,
                    "end_col": end_col,
                },
                "after_span": {
                    "start_line": after_start_line,
                    "start_col": after_start_col,
                    "end_line": after_end_line,
                    "end_col": after_end_col,
                },
            })
        })
        .collect();

    let event_payload = json!({
        "editor_surface": "monaco",
        "document_uri": format!("hsk://documents/{}", document_id),
        "path": null,
        "before_hash": before_hash,
        "after_hash": after_hash,
        "diff_hash": diff_hash,
        "applied_suggestions": applied_suggestions,
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
        event_payload,
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

    fn selection_v1(doc_text: &str, start: usize, end: usize) -> SelectionRangeV1 {
        let doc_hash = sha256_hex(doc_text.as_bytes());
        let selection_hash = sha256_hex(&doc_text.as_bytes()[start..end]);
        SelectionRangeV1 {
            schema_version: "hsk.selection_range@v1".to_string(),
            surface: "docs".to_string(),
            coordinate_space: "doc_text_utf8_v1".to_string(),
            start_utf8: start,
            end_utf8: end,
            doc_preimage_sha256: doc_hash,
            selection_preimage_sha256: selection_hash,
        }
    }

    #[tokio::test]
    async fn verify_atelier_apply_provenance_accepts_matching_job_output(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let doc_id = "doc-1".to_string();
        let doc_text = "Hello world\nSecond line";
        let role_id = "role-a".to_string();
        let selection = selection_v1(doc_text, 6, 11);
        let selection_len_utf8 = selection.end_utf8.saturating_sub(selection.start_utf8);

        let patchset = DocPatchsetV1 {
            schema_version: "hsk.doc_patchset@v1".to_string(),
            doc_id: doc_id.clone(),
            selection: selection.clone(),
            boundary_normalization: "disabled".to_string(),
            ops: vec![
                crate::ace::validators::atelier_scope::PatchOpV1::ReplaceRange {
                    range_utf8: crate::ace::validators::atelier_scope::RangeUtf8 {
                        start: 0,
                        end: selection_len_utf8,
                    },
                    insert_text: "earth".to_string(),
                },
            ],
            summary: None,
        };

        let job = state
            .storage
            .create_ai_job(NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocEdit,
                protocol_id: "atelier-doc-suggest-v1".into(),
                profile_id: "profile1".into(),
                capability_profile_id: "cap1".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({
                    "doc_id": doc_id,
                    "role_id": role_id,
                    "selection": selection,
                })),
            })
            .await?;

        let suggestion_id = Uuid::new_v4().to_string();
        let output = json!({
            "schema_version": ATELIER_ROLE_SUGGESTIONS_SCHEMA_V1,
            "doc_id": "doc-1",
            "selection": patchset.selection.clone(),
            "by_role": [
                {
                    "role_id": "role-a",
                    "suggestions": [
                        {
                            "suggestion_id": suggestion_id.clone(),
                            "role_id": "role-a",
                            "contract_id": "ROLE:role-a:C:1",
                            "title": "Suggested edit",
                            "rationale": null,
                            "patchset": patchset.clone(),
                            "protocol_id": job.protocol_id.clone(),
                            "source_job_id": job.job_id,
                            "source_trace_id": job.trace_id,
                            "source_model_id": "test-model",
                        }
                    ]
                }
            ]
        });

        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.job_id,
                state: JobState::Completed,
                error_message: None,
                status_reason: "completed".into(),
                metrics: None,
                workflow_run_id: None,
                trace_id: Some(job.trace_id),
                job_outputs: Some(output),
            })
            .await?;

        let incoming = AtelierSuggestionToApplyV1 {
            role_id: "role-a".to_string(),
            suggestion_id: suggestion_id.clone(),
            source_job_id: job.job_id.to_string(),
            patchset: patchset.clone(),
        };

        let verified =
            verify_atelier_applied_suggestion_v1(&state, "doc-1", &patchset.selection, &incoming)
                .await
                .map_err(|(status, _body)| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("expected verification to succeed, got status {status}"),
                    )
                })?;

        assert_eq!(verified.role_id, "role-a");
        assert_eq!(verified.contract_id, "ROLE:role-a:C:1");
        assert_eq!(verified.suggestion_id, suggestion_id);
        assert_eq!(verified.source_job_id, job.job_id.to_string());
        assert_eq!(verified.source_trace_id, job.trace_id.to_string());
        assert_eq!(verified.source_model_id, "test-model");
        assert_eq!(verified.patchset, patchset);

        Ok(())
    }

    #[tokio::test]
    async fn verify_atelier_apply_provenance_rejects_selection_mismatch_as_stale(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let doc_text = "Hello world\nSecond line";
        let selection = selection_v1(doc_text, 6, 11);
        let selection_len_utf8 = selection.end_utf8.saturating_sub(selection.start_utf8);

        let patchset = DocPatchsetV1 {
            schema_version: "hsk.doc_patchset@v1".to_string(),
            doc_id: "doc-1".to_string(),
            selection: selection.clone(),
            boundary_normalization: "disabled".to_string(),
            ops: vec![
                crate::ace::validators::atelier_scope::PatchOpV1::ReplaceRange {
                    range_utf8: crate::ace::validators::atelier_scope::RangeUtf8 {
                        start: 0,
                        end: selection_len_utf8,
                    },
                    insert_text: "earth".to_string(),
                },
            ],
            summary: None,
        };

        let job = state
            .storage
            .create_ai_job(NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocEdit,
                protocol_id: "atelier-doc-suggest-v1".into(),
                profile_id: "profile1".into(),
                capability_profile_id: "cap1".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({"doc_id": "doc-1"})),
            })
            .await?;

        let suggestion_id = Uuid::new_v4().to_string();
        let output = json!({
            "schema_version": ATELIER_ROLE_SUGGESTIONS_SCHEMA_V1,
            "doc_id": "doc-1",
            "selection": patchset.selection.clone(),
            "by_role": [
                {
                    "role_id": "role-a",
                    "suggestions": [
                        {
                            "suggestion_id": suggestion_id.clone(),
                            "role_id": "role-a",
                            "contract_id": "ROLE:role-a:C:1",
                            "patchset": patchset.clone(),
                            "protocol_id": job.protocol_id.clone(),
                            "source_job_id": job.job_id,
                            "source_trace_id": job.trace_id,
                            "source_model_id": "test-model",
                        }
                    ]
                }
            ]
        });

        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.job_id,
                state: JobState::Completed,
                error_message: None,
                status_reason: "completed".into(),
                metrics: None,
                workflow_run_id: None,
                trace_id: Some(job.trace_id),
                job_outputs: Some(output),
            })
            .await?;

        let mut mismatched_selection = selection.clone();
        mismatched_selection.end_utf8 = mismatched_selection.end_utf8.saturating_add(1);

        let incoming = AtelierSuggestionToApplyV1 {
            role_id: "role-a".to_string(),
            suggestion_id,
            source_job_id: job.job_id.to_string(),
            patchset,
        };

        let Err((status, Json(err))) =
            verify_atelier_applied_suggestion_v1(&state, "doc-1", &mismatched_selection, &incoming)
                .await
        else {
            unreachable!("expected a selection mismatch error");
        };

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.error, ERR_ATELIER_STALE_SELECTION);

        Ok(())
    }

    #[tokio::test]
    async fn verify_atelier_apply_provenance_rejects_patchset_mismatch_as_provenance_mismatch(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;

        let doc_text = "Hello world\nSecond line";
        let selection = selection_v1(doc_text, 6, 11);
        let selection_len_utf8 = selection.end_utf8.saturating_sub(selection.start_utf8);

        let patchset = DocPatchsetV1 {
            schema_version: "hsk.doc_patchset@v1".to_string(),
            doc_id: "doc-1".to_string(),
            selection: selection.clone(),
            boundary_normalization: "disabled".to_string(),
            ops: vec![
                crate::ace::validators::atelier_scope::PatchOpV1::ReplaceRange {
                    range_utf8: crate::ace::validators::atelier_scope::RangeUtf8 {
                        start: 0,
                        end: selection_len_utf8,
                    },
                    insert_text: "earth".to_string(),
                },
            ],
            summary: None,
        };

        let job = state
            .storage
            .create_ai_job(NewAiJob {
                trace_id: Uuid::new_v4(),
                job_kind: JobKind::DocEdit,
                protocol_id: "atelier-doc-suggest-v1".into(),
                profile_id: "profile1".into(),
                capability_profile_id: "cap1".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: Vec::new(),
                planned_operations: Vec::new(),
                status_reason: "queued".to_string(),
                metrics: JobMetrics::zero(),
                job_inputs: Some(json!({"doc_id": "doc-1"})),
            })
            .await?;

        let suggestion_id = Uuid::new_v4().to_string();
        let output = json!({
            "schema_version": ATELIER_ROLE_SUGGESTIONS_SCHEMA_V1,
            "doc_id": "doc-1",
            "selection": patchset.selection.clone(),
            "by_role": [
                {
                    "role_id": "role-a",
                    "suggestions": [
                        {
                            "suggestion_id": suggestion_id.clone(),
                            "role_id": "role-a",
                            "contract_id": "ROLE:role-a:C:1",
                            "patchset": patchset.clone(),
                            "protocol_id": job.protocol_id.clone(),
                            "source_job_id": job.job_id,
                            "source_trace_id": job.trace_id,
                            "source_model_id": "test-model",
                        }
                    ]
                }
            ]
        });

        state
            .storage
            .update_ai_job_status(JobStatusUpdate {
                job_id: job.job_id,
                state: JobState::Completed,
                error_message: None,
                status_reason: "completed".into(),
                metrics: None,
                workflow_run_id: None,
                trace_id: Some(job.trace_id),
                job_outputs: Some(output),
            })
            .await?;

        let mut mismatched_patchset = patchset.clone();
        if let Some(crate::ace::validators::atelier_scope::PatchOpV1::ReplaceRange {
            insert_text,
            ..
        }) = mismatched_patchset.ops.get_mut(0)
        {
            *insert_text = "mars".to_string();
        }

        let incoming = AtelierSuggestionToApplyV1 {
            role_id: "role-a".to_string(),
            suggestion_id,
            source_job_id: job.job_id.to_string(),
            patchset: mismatched_patchset,
        };

        let Err((status, Json(err))) =
            verify_atelier_applied_suggestion_v1(&state, "doc-1", &selection, &incoming).await
        else {
            unreachable!("expected a provenance mismatch error");
        };

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(err.error, ERR_ATELIER_PROVENANCE_MISMATCH);

        Ok(())
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
