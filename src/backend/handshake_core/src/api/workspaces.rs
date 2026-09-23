use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, time::Instant};
use uuid::Uuid;

use crate::ace::validators::atelier_scope::{
    apply_selection_bounded_patchsets, sha256_hex, AtelierScopeError, DocPatchsetV1,
    SelectionRangeV1,
};
use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::runtime_governance::RuntimeGovernancePaths;
use crate::workflows::build_dcc_control_plane_snapshot;
use crate::{
    diagnostics::{
        DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface, LinkConfidence,
    },
    models::{
        BlockResponse, CreateWorkspaceRequest, DocumentWithBlocksResponse, ErrorResponse,
        UpsertBlocksRequest, WorkspaceResponse,
    },
    storage::{
        Block, JobKind, JobState, NewBlock, NewWorkspace, StorageError, WorkbenchLayoutStateInput,
        WorkspaceSearchBookmarkStateInput, WorkspaceSettingsStateInput, WriteActorKind,
        WriteContext,
    },
    AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/workspaces", post(create_workspace).get(list_workspaces))
        // MT-154 D-154-1 (Master Spec 02-system-architecture.md:2680-2705): the legacy
        // POST/GET /workspaces/:workspace_id/documents route is retired; RichDocument
        // (/knowledge/documents) is the canonical document surface.
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
        .route(
            "/workspaces/:workspace_id/workbench/layout",
            get(get_workbench_layout).put(save_workbench_layout),
        )
        .route(
            "/workspaces/:workspace_id/settings",
            get(get_workspace_settings).put(save_workspace_settings),
        )
        .route(
            "/workspaces/:workspace_id/search-bookmarks",
            get(get_workspace_search_bookmarks).put(save_workspace_search_bookmarks),
        )
        .route("/workspaces/:workspace_id", delete(delete_workspace))
        .route("/dcc/control-plane", get(dcc_control_plane_snapshot))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            workspace_authority,
        ))
        .with_state(state)
}

/// Workspace-specific routes run under the same exact resource grant and record-user boundary.
async fn workspace_authority(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    if request.uri().path().starts_with("/documents/") {
        return protected_workspace_denial().into_response();
    }
    let Some(path) = request.uri().path().strip_prefix("/workspaces/") else {
        // Collection handlers authenticate internally; this boundary covers workspace-specific routes.
        return next.run(request).await;
    };
    let workspace = path.split('/').next().unwrap_or("");
    if workspace.is_empty() {
        return protected_workspace_denial().into_response();
    }
    let read = request.method() == Method::GET;
    let action = if read {
        ResourceAction::Read
    } else if request.method() == Method::DELETE {
        ResourceAction::Delete
    } else if request.method() == Method::POST {
        ResourceAction::Create
    } else {
        ResourceAction::Update
    };
    let authority = match crate::api::authority::authorize_request(
        &state,
        request.headers(),
        if read { "fs.read" } else { "fs.write" },
        ResourceKind::Workspace,
        workspace,
        action,
    )
    .await
    {
        Ok(authority) => authority,
        Err(_) => return protected_workspace_denial().into_response(),
    };
    state
        .surreal
        .with_record_user_scope(authority.record_user_scope, next.run(request))
        .await
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
                    return Ok(WriteContext::ai(actor_id, job_id, workflow_id));
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

fn protected_workspace_denial() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-PROTECTED-RESOURCE",
        }),
    )
}

async fn create_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let started = Instant::now();
    tracing::info!(target: "handshake_core", route = "/workspaces", "workspace create request entered");
    let credentials = crate::api::authority::authenticated_session_credentials(&state, &headers)
        .await
        .map_err(|error| {
            tracing::warn!(
                target: "handshake_core",
                route = "/workspaces",
                elapsed_ms = started.elapsed().as_millis(),
                "workspace create session authentication denied"
            );
            #[cfg(test)]
            eprintln!("workspace-create credentialsfailed: {error:?}");
            #[cfg(not(test))]
            let _ = error;
            protected_workspace_denial()
        })?;
    tracing::info!(
        target: "handshake_core",
        route = "/workspaces",
        elapsed_ms = started.elapsed().as_millis(),
        "workspace create session authentication completed"
    );
    let scope = crate::storage::surreal::resource_authority::RecordUserScope {
        workspace_id: Some(uuid::Uuid::now_v7().to_string()),
        resource_id: uuid::Uuid::now_v7().to_string(),
        session_id: credentials.context.session_id.clone(),
        session_token: credentials.session_token,
        channel_binding_hash: Some(credentials.channel_binding_hash),
        capability_id: "fs.write".to_owned(),
        action: crate::storage::surreal::resource_authority::ResourceAction::Create,
        grant_id: None,
    };

    let workspace = state
        .surreal
        .create_account_workspace(
            &credentials.context,
            &scope,
            NewWorkspace { name: payload.name },
        )
        .await
        .map_err(|error| {
            tracing::warn!(
                target: "handshake_core",
                route = "/workspaces",
                elapsed_ms = started.elapsed().as_millis(),
                "workspace create atomic transaction failed"
            );
            #[cfg(test)]
            eprintln!("workspace-create atomic-createfailed: {error}");
            #[cfg(not(test))]
            let _ = error;
            protected_workspace_denial()
        })?;
    tracing::info!(
        target: "handshake_core",
        route = "/workspaces",
        elapsed_ms = started.elapsed().as_millis(),
        "workspace create atomic transaction completed"
    );
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
    headers: HeaderMap,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<ErrorResponse>)> {
    crate::api::authority::authenticated_session(&state, &headers)
        .await
        .map_err(|_| protected_workspace_denial())?;
    let channel = crate::api::stage::capture_channel_binding(&headers)
        .map_err(|_| protected_workspace_denial())?;
    let token = headers
        .get("x-hsk-session-token")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(protected_workspace_denial)?;
    let rows = state
        .surreal
        .list_account_workspaces(token, &channel.binding_hash)
        .await
        .map_err(|_| protected_workspace_denial())?;
    Ok(Json(
        rows.into_iter()
            .map(|row| WorkspaceResponse {
                id: row.id,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect(),
    ))
}

/// `policy_decision_id` prefix of the one Flight Recorder audit event a workspace delete emits.
pub(crate) const WORKSPACE_DELETE_AUDIT_PREFIX: &str = "workspace-delete:";

async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    let authority = crate::api::authority::authorize_request(
        &state,
        &headers,
        "fs.write",
        ResourceKind::Workspace,
        &workspace_id,
        ResourceAction::Delete,
    )
    .await
    .map_err(|_| protected_workspace_denial())?;
    state
        .surreal
        .delete_account_workspace(&authority.record_user_scope, &workspace_id)
        .await
        .map_err(|_| protected_workspace_denial())?;

    if let Err(error) = state
        .flight_recorder
        .delete_workspace_events(&workspace_id)
        .await
    {
        // The durable delete is already committed. Keep DELETE idempotent/successful and leave an
        // attributable recovery log instead of returning a false 500 after the workspace is gone.
        tracing::error!(target: "handshake_core", %workspace_id, %error, "workspace deleted but Flight Recorder workspace purge failed");
    }

    // Operator decision 2026-09-22 (MT-109 C1-FDELETE): an owner-authorized workspace delete
    // cascades to its rich documents, their version history and its Canvas boards, and is recorded
    // as one auditable capability decision. Written AFTER the workspace purge so it survives it.
    let audit = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        FlightRecorderActor::Human,
        Uuid::now_v7(),
        json!({
            "capability_id": "fs.write",
            "actor_id": authority.actor_id,
            "job_id": null,
            "decision_outcome": "allow",
        }),
    )
    .with_actor_id(authority.actor_id.clone())
    .with_capability_id("fs.write")
    .with_policy_decision_id(format!("{WORKSPACE_DELETE_AUDIT_PREFIX}{workspace_id}"))
    .with_wsids(vec![workspace_id.clone()]);
    if let Err(error) = state.flight_recorder.record_event(audit).await {
        // The durable delete is committed; keep DELETE idempotent and log the audit gap loudly.
        tracing::error!(target: "handshake_core", %workspace_id, %error, "workspace deleted but delete audit event failed");
    }

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id", status = "deleted", workspace_id = %workspace_id, "workspace deleted");

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct WorkbenchLayoutResponse {
    workspace_id: String,
    layout_state: Option<Value>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    event_ledger_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveWorkbenchLayoutRequest {
    layout_state: Value,
}

async fn get_workbench_layout(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkbenchLayoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .get_workbench_layout_state(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(match record {
        Some(record) => WorkbenchLayoutResponse {
            workspace_id: record.workspace_id,
            layout_state: Some(record.layout_state),
            updated_at: Some(record.updated_at),
            event_ledger_event_id: Some(record.event_ledger_event_id),
        },
        None => WorkbenchLayoutResponse {
            workspace_id,
            layout_state: None,
            updated_at: None,
            event_ledger_event_id: None,
        },
    }))
}

async fn save_workbench_layout(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<SaveWorkbenchLayoutRequest>,
) -> Result<Json<WorkbenchLayoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .save_workbench_layout_state(
            &workspace_id,
            WorkbenchLayoutStateInput {
                layout_state: payload.layout_state,
            },
        )
        .await
        .map_err(map_storage_error)?;

    Ok(Json(WorkbenchLayoutResponse {
        workspace_id: record.workspace_id,
        layout_state: Some(record.layout_state),
        updated_at: Some(record.updated_at),
        event_ledger_event_id: Some(record.event_ledger_event_id),
    }))
}

#[derive(Debug, Serialize)]
struct WorkspaceSettingsResponse {
    workspace_id: String,
    settings_state: Option<Value>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    event_ledger_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveWorkspaceSettingsRequest {
    settings_state: Value,
}

async fn get_workspace_settings(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkspaceSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .get_workspace_settings_state(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(match record {
        Some(record) => WorkspaceSettingsResponse {
            workspace_id: record.workspace_id,
            settings_state: Some(record.settings_state),
            updated_at: Some(record.updated_at),
            event_ledger_event_id: Some(record.event_ledger_event_id),
        },
        None => WorkspaceSettingsResponse {
            workspace_id,
            settings_state: None,
            updated_at: None,
            event_ledger_event_id: None,
        },
    }))
}

async fn save_workspace_settings(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<SaveWorkspaceSettingsRequest>,
) -> Result<Json<WorkspaceSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .save_workspace_settings_state(
            &workspace_id,
            WorkspaceSettingsStateInput {
                settings_state: payload.settings_state,
            },
        )
        .await
        .map_err(map_storage_error)?;

    Ok(Json(WorkspaceSettingsResponse {
        workspace_id: record.workspace_id,
        settings_state: Some(record.settings_state),
        updated_at: Some(record.updated_at),
        event_ledger_event_id: Some(record.event_ledger_event_id),
    }))
}

#[derive(Debug, Serialize)]
struct WorkspaceSearchBookmarksResponse {
    workspace_id: String,
    bookmark_state: Option<Value>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    event_ledger_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveWorkspaceSearchBookmarksRequest {
    bookmark_state: Value,
}

async fn get_workspace_search_bookmarks(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkspaceSearchBookmarksResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .get_workspace_search_bookmark_state(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    Ok(Json(match record {
        Some(record) => WorkspaceSearchBookmarksResponse {
            workspace_id: record.workspace_id,
            bookmark_state: Some(record.bookmark_state),
            updated_at: Some(record.updated_at),
            event_ledger_event_id: Some(record.event_ledger_event_id),
        },
        None => WorkspaceSearchBookmarksResponse {
            workspace_id,
            bookmark_state: None,
            updated_at: None,
            event_ledger_event_id: None,
        },
    }))
}

async fn save_workspace_search_bookmarks(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<SaveWorkspaceSearchBookmarksRequest>,
) -> Result<Json<WorkspaceSearchBookmarksResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let record = state
        .storage
        .save_workspace_search_bookmark_state(
            &workspace_id,
            WorkspaceSearchBookmarkStateInput {
                bookmark_state: payload.bookmark_state,
            },
        )
        .await
        .map_err(map_storage_error)?;

    Ok(Json(WorkspaceSearchBookmarksResponse {
        workspace_id: record.workspace_id,
        bookmark_state: Some(record.bookmark_state),
        updated_at: Some(record.updated_at),
        event_ledger_event_id: Some(record.event_ledger_event_id),
    }))
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
                    return Err(bad_request_error());
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
        Uuid::now_v7(),
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

/// GET /dcc/control-plane — thin read-only projection endpoint.
async fn dcc_control_plane_snapshot(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let runtime_paths = RuntimeGovernancePaths::resolve().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "HSK-500-DCC-RESOLVE",
            }),
        )
    })?;
    let snapshot = build_dcc_control_plane_snapshot(
        &state.session_registry,
        &runtime_paths,
        &state.capability_registry,
        state.storage.as_ref(),
    )
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "HSK-500-DCC-BUILD",
            }),
        )
    })?;
    let value = serde_json::to_value(&snapshot).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "HSK-500-DCC-SERIALIZE",
            }),
        )
    })?;
    Ok(Json(value))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
pub(crate) mod tests {
    use super::*;
    use crate::api::MountedRequestExt;
    use crate::capabilities::CapabilityRegistry;
    use crate::diagnostics::DiagFilter;
    use crate::flight_recorder::{
        duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorderEventType,
    };
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::knowledge::{
        KnowledgeRichDocument, KnowledgeStore, NewKnowledgeRichDocument,
    };
    use crate::storage::{
        fems_memory, tests::embedded_test_backend, AccessMode, Database, EntityRef, JobKind,
        JobMetrics, JobState, JobStatusUpdate, NewAiJob, NewDocument, PlannedOperation, SafetyMode,
    };
    use axum::extract::{Path, State};
    use serde_json::json;
    use std::sync::Arc;
    use surrealdb::types::SurrealValue;

    /// WP-KERNEL-012 MT-144: see the equivalent helper in `api::jobs`. The state is now created
    /// unconditionally from an isolated authoritative store, so there is no skip branch.
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

    /// MT-159: shared with the api::jobs tests (account-session fixtures).
    pub(crate) struct WorkspaceBindingFixture {
        _lock: std::sync::MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        token: String,
    }
    impl WorkspaceBindingFixture {
        pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let directory = tempfile::tempdir()?;
            let path = directory.path().join("workspace-binding.json");
            let token = "c7".repeat(32);
            std::fs::write(
                &path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(&token))?,
            )?;
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Ok(Self {
                _lock: lock,
                _directory: directory,
                previous,
                token,
            })
        }
    }
    impl Drop for WorkspaceBindingFixture {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
        }
    }
    pub(crate) async fn workspace_test_principal(
        state: &AppState,
        binding: &WorkspaceBindingFixture,
        key: &str,
    ) -> Result<
        (
            crate::storage::surreal::resource_authority::ProvisionedPrincipal,
            HeaderMap,
        ),
        Box<dyn std::error::Error>,
    > {
        if !state
            .surreal
            .reconciliation_principal_is_provisioned()
            .await?
        {
            state
                .surreal
                .provision_reconciliation_principal(&[], None)
                .await?;
        }
        let capabilities = [
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
        let principal = state
            .surreal
            .provision_principal(
                key,
                key,
                "human_account",
                key,
                "Operator",
                &capabilities,
                key,
                Some(&sha256_hex(binding.token.as_bytes())),
                std::time::Duration::from_secs(3600),
            )
            .await?;
        let mut headers = HeaderMap::new();
        headers.insert("x-hsk-session-token", principal.session.token.parse()?);
        headers.insert("x-hsk-channel-binding-token", binding.token.parse()?);
        Ok((principal, headers))
    }
    pub(crate) async fn create_owned_test_workspace(
        state: &AppState,
        headers: &HeaderMap,
    ) -> Result<WorkspaceResponse, String> {
        create_workspace(
            State(state.clone()),
            headers.clone(),
            Json(CreateWorkspaceRequest {
                name: "owned-delete-proof".to_owned(),
            }),
        )
        .await
        .map(|(_, Json(row))| row)
        .map_err(|(status, Json(error))| format!("{status}: {}", error.error))
    }

    async fn post_owned_rich_document(
        state: &AppState,
        headers: &HeaderMap,
        workspace_id: &str,
        title: &str,
    ) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri("/knowledge/documents")
            .header("content-type", "application/json")
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-actor-id", "workspace-document-proof")
            .header("x-hsk-kernel-task-run-id", "workspace-document-proof")
            .header("x-hsk-session-run-id", "workspace-document-proof");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = crate::api::knowledge_documents::routes(state.clone())
            .oneshot(request.body(axum::body::Body::from(serde_json::to_vec(
                &json!({"workspace_id": workspace_id, "title": title}),
            )?))?)
            .await?;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
        Ok((status, serde_json::from_slice(&body)?))
    }

    #[derive(Debug, PartialEq, SurrealValue)]
    struct OwnedRichDocumentRows {
        documents: i64,
        resources: i64,
        grants: i64,
        versions: i64,
        loom_blocks: i64,
        loom_search_rows: i64,
        title_anchors: i64,
    }

    async fn owned_rich_document_rows(
        state: &AppState,
    ) -> Result<OwnedRichDocumentRows, Box<dyn std::error::Error>> {
        let mut result = state
            .surreal
            .test_admin_query_bound(
                "RETURN { documents: array::len(SELECT VALUE id FROM knowledge_rich_documents), resources: array::len(SELECT VALUE id FROM protected_resources), grants: array::len(SELECT VALUE id FROM resource_grants), versions: array::len(SELECT VALUE id FROM knowledge_rich_document_versions), loom_blocks: array::len(SELECT VALUE id FROM loom_blocks), loom_search_rows: array::len(SELECT VALUE id FROM loom_block_search_index), title_anchors: array::len(SELECT VALUE id FROM knowledge_rich_document_title_anchors) };".to_owned(),
                json!({}),
            )
            .await?
            .check()?;
        result
            .take::<Option<OwnedRichDocumentRows>>(0)?
            .ok_or_else(|| "owned rich document row snapshot missing".into())
    }

    async fn owned_rich_document_authority_lineage(
        state: &AppState,
        document_id: &str,
        workspace_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut result = state
            .surreal
            .test_admin_query_bound(
                "RETURN array::len((SELECT VALUE id FROM protected_resources WHERE resource_kind = 'rich_document' AND external_resource_id = $document_id AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.external_resource_id = $workspace_id AND lifecycle_state = 'active')) = 1 AND array::len((SELECT VALUE id FROM resource_grants WHERE resource_id = (SELECT VALUE id FROM protected_resources WHERE resource_kind = 'rich_document' AND external_resource_id = $document_id)[0] AND actions = ['read', 'create', 'update', 'delete'] AND capability_ids = ['fs.read', 'fs.write'] AND status = 'active')) = 1;".to_owned(),
                json!({"document_id": document_id, "workspace_id": workspace_id}),
            )
            .await?
            .check()?;
        result
            .take::<Option<bool>>(0)?
            .ok_or_else(|| "owned rich document authority lineage result missing".into())
    }

    async fn resource_grant_capabilities(
        state: &AppState,
        grant_id: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut result = state
            .surreal
            .test_admin_query_bound(
                "SELECT VALUE capability_ids FROM type::record('resource_grants', $grant_id);"
                    .to_owned(),
                json!({"grant_id": grant_id}),
            )
            .await?
            .check()?;
        let rows = result.take::<Vec<Vec<String>>>(0)?;
        match rows.as_slice() {
            [capabilities] => Ok(capabilities.clone()),
            _ => Err("workspace create grant capabilities must have exactly one row".into()),
        }
    }

    async fn scoped_owned_rich_document(
        state: &AppState,
        scope: crate::storage::surreal::resource_authority::RecordUserScope,
        workspace_id: &str,
        title: &str,
    ) -> crate::storage::StorageResult<KnowledgeRichDocument> {
        let database = crate::storage::surreal::SurrealDatabase::new(state.surreal.clone());
        state
            .surreal
            .with_record_user_scope(
                scope,
                database.create_knowledge_rich_document(NewKnowledgeRichDocument {
                    workspace_id: workspace_id.to_owned(),
                    document_id: None,
                    title: title.to_owned(),
                    schema_version: crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION
                        .to_owned(),
                    content_json: json!({"type": "doc", "content": []}),
                    crdt_document_id: None,
                    crdt_snapshot_id: None,
                    promotion_receipt_event_id: None,
                    project_ref: None,
                    folder_ref: None,
                    authority_label: Some("promoted".to_owned()),
                    owner_actor_kind: Some("operator".to_owned()),
                    owner_actor_id: Some("workspace-document-proof".to_owned()),
                }),
            )
            .await
            .map_err(Into::into)
    }

    #[tokio::test]
    async fn delete_workspace_route_atomically_cascades_fems_and_retries_not_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (owner, headers) =
            workspace_test_principal(&state, &binding, "workspace-delete-owner").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let create_scope = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::Workspace,
            &workspace.id,
            ResourceAction::Create,
        )
        .await
        .map_err(|_| "workspace create grant missing")?
        .record_user_scope;
        let create_grant = create_scope
            .grant_id
            .clone()
            .ok_or("workspace create grant missing")?;
        let original_capabilities = resource_grant_capabilities(&state, &create_grant).await?;
        let narrowed = original_capabilities
            .iter()
            .filter(|capability| capability.as_str() != "fs.write")
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            narrowed.len() < original_capabilities.len(),
            "workspace create grant must contain fs.write before narrowing"
        );
        let (foreign, foreign_headers) =
            workspace_test_principal(&state, &binding, "workspace-document-foreign").await?;
        let foreign_scope = crate::storage::surreal::resource_authority::RecordUserScope {
            grant_id: Some(create_grant.clone()),
            workspace_id: Some(workspace.id.clone()),
            session_token: foreign.session.token,
            channel_binding_hash: Some(sha256_hex(binding.token.as_bytes())),
            resource_id: create_scope.resource_id.clone(),
            session_id: foreign.session.session_id,
            capability_id: "fs.write".to_owned(),
            action: ResourceAction::Create,
        };
        let before_rows = owned_rich_document_rows(&state).await?;
        state.surreal.test_admin_query_bound(
            "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
            json!({"grant_id": create_grant.clone(), "capabilities": narrowed}),
        ).await?.check()?;
        let narrowed_error = scoped_owned_rich_document(
            &state,
            create_scope.clone(),
            &workspace.id,
            "scoped denied narrowed capability",
        )
        .await
        .expect_err("narrowed capability must fail at the record-user producer boundary");
        assert!(
            narrowed_error
                .to_string()
                .contains("HSK-403-PROTECTED-RESOURCE"),
            "narrowed producer failure must come from protected storage: {narrowed_error}"
        );
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            before_rows,
            "narrowed capability must roll back source, authority, and projection rows"
        );
        let (status, _) = post_owned_rich_document(
            &state,
            &headers,
            &workspace.id,
            "denied narrowed capability",
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            before_rows,
            "narrowed route denial must not create rows"
        );
        state.surreal.test_admin_query_bound(
            "UPDATE type::record('resource_grants', $grant_id) SET capability_ids = $capabilities RETURN NONE;".to_owned(),
            json!({"grant_id": create_grant.clone(), "capabilities": original_capabilities}),
        ).await?.check()?;
        let scoped_document = scoped_owned_rich_document(
            &state,
            create_scope.clone(),
            &workspace.id,
            "Scoped source and Loom projection",
        )
        .await?;
        assert!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&scoped_document.rich_document_id)
                .await?
                .is_some()
        );
        assert!(
            owned_rich_document_authority_lineage(
                &state,
                &scoped_document.rich_document_id,
                &workspace.id,
            )
            .await?,
            "record-user rich-document creation must persist its exact protected-resource and grant lineage"
        );
        let established_rows = owned_rich_document_rows(&state).await?;
        state.surreal.test_admin_query_bound(
            "UPDATE type::record('resource_grants', $grant_id) SET status = 'revoked', revoked_at = time::now() RETURN NONE;".to_owned(),
            json!({"grant_id": create_grant.clone()}),
        ).await?.check()?;
        let revoked_error = scoped_owned_rich_document(
            &state,
            create_scope.clone(),
            &workspace.id,
            "scoped denied revoked grant",
        )
        .await
        .expect_err("revoked grant must fail at the record-user producer boundary");
        let revoked_rendered = revoked_error.to_string();
        assert!(
            revoked_rendered.contains("knowledge_rich_documents")
                && revoked_rendered.contains("field `workspace_id`")
                && revoked_rendered.contains("record::exists($value)"),
            "revoked producer failure must fail closed at the rich-document workspace field boundary: {revoked_error}"
        );
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            established_rows,
            "revoked grant must roll back source, authority, and projection rows"
        );
        let (status, _) =
            post_owned_rich_document(&state, &headers, &workspace.id, "denied revoked grant")
                .await?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            established_rows,
            "revoked route denial must not create rows"
        );
        state.surreal.test_admin_query_bound(
            "UPDATE type::record('resource_grants', $grant_id) SET status = 'active', revoked_at = NONE RETURN NONE;".to_owned(),
            json!({"grant_id": create_grant.clone()}),
        ).await?.check()?;
        let foreign_error = scoped_owned_rich_document(
            &state,
            foreign_scope,
            &workspace.id,
            "scoped denied foreign workspace",
        )
        .await
        .expect_err("foreign principal must fail at the record-user producer boundary");
        let foreign_rendered = foreign_error.to_string();
        assert!(
            foreign_rendered.contains("knowledge_rich_documents")
                && foreign_rendered.contains("field `workspace_id`")
                && foreign_rendered.contains("record::exists($value)"),
            "foreign producer failure must fail closed at the rich-document workspace field boundary: {foreign_error}"
        );
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            established_rows,
            "foreign principal must roll back source, authority, and projection rows"
        );
        let (status, _) = post_owned_rich_document(
            &state,
            &foreign_headers,
            &workspace.id,
            "denied foreign owner",
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(
            owned_rich_document_rows(&state).await?,
            established_rows,
            "foreign route denial must not create rows"
        );
        #[derive(SurrealValue)]
        struct RichDocumentTombstoneBindings {
            document: String,
        }
        let document_id = scoped_document.rich_document_id.clone();
        let live_document_authority = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::RichDocument,
            &document_id,
            ResourceAction::Update,
        )
        .await
        .map_err(|_| "owned rich document update grant missing")?;
        let live_update_count = state
            .surreal
            .with_record_user_scope(
                live_document_authority.record_user_scope,
                state.surreal.with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .execute_returning(
                                "UPDATE type::record('knowledge_rich_documents', $document) SET updated_at = time::now() RETURN AFTER;",
                                RichDocumentTombstoneBindings { document: document_id },
                            )
                            .await
                    })
                }),
            )
            .await?;
        assert_eq!(
            live_update_count, 1,
            "authorized live-document update must pass its record-user event guard"
        );
        let canonical_after_live_update =
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&scoped_document.rich_document_id)
                .await?
                .ok_or("live document missing after authorized update")?;
        let mut document_delete_authority = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::RichDocument,
            &scoped_document.rich_document_id,
            ResourceAction::Delete,
        )
        .await
        .map_err(|_| "owned rich document delete grant missing")?;
        document_delete_authority.record_user_scope.workspace_id = Some(
            state
                .surreal
                .authorized_document_workspace(
                    &document_delete_authority.resource_id,
                    &document_delete_authority.account_id,
                    &document_delete_authority.access_space_id,
                )
                .await?
                .ok_or("owned rich document workspace missing")?,
        );
        let document_id = scoped_document.rich_document_id.clone();
        let malformed_tombstone = state
            .surreal
            .with_record_user_scope(
                document_delete_authority.record_user_scope,
                state.surreal.with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .execute_returning(
                                "UPDATE type::record('knowledge_rich_documents', $document) SET deleted_at = time::now() RETURN AFTER;",
                                RichDocumentTombstoneBindings {
                                    document: document_id,
                                },
                            )
                            .await
                    })
                }),
            )
            .await
            .expect_err("malformed rich-document tombstone must fire the delete guard");
        assert!(
            malformed_tombstone
                .to_string()
                .contains("HSK-403-PROTECTED-RESOURCE"),
            "malformed tombstone must fail at the document update guard: {malformed_tombstone}"
        );
        assert_eq!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&scoped_document.rich_document_id)
                .await?,
            Some(canonical_after_live_update),
            "rejected tombstone must leave the canonical rich document unchanged"
        );
        let (status, created) = post_owned_rich_document(
            &state,
            &headers,
            &workspace.id,
            "Owned source and Loom projection",
        )
        .await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "real document create failed: {created}"
        );
        let document_id = created["document"]["rich_document_id"]
            .as_str()
            .ok_or("created document id missing")?
            .to_owned();
        assert!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&document_id)
                .await?
                .is_some()
        );
        use crate::storage::surreal::resource_authority::ResourceGrantSpec;
        let root = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::Workspace,
            &workspace.id,
            ResourceAction::Delete,
        )
        .await
        .map_err(|_| "workspace grant missing")?;
        let memory = state
            .surreal
            .register_protected_resource(
                &owner.identity,
                ResourceKind::MemoryItem,
                &workspace.id,
                Some(&root.resource_id),
                "account_private",
            )
            .await?;
        state
            .surreal
            .grant_resource(
                &owner.identity.account_id,
                &owner.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: owner.identity.principal_id.clone(),
                    resource_id: memory.resource_id,
                    actions: vec![ResourceAction::Delete],
                    capability_ids: vec!["fs.write".to_owned()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
        fems_memory::upsert_memory_item(
            &state.surreal,
            &workspace.id,
            &format!("MEM-DELETE-ROUTE-{}", Uuid::now_v7()),
            &json!({"content": "cascade me"}),
        )
        .await?;
        // A revoked descendant grant must abort the entire workspace cascade.
        let descendant_grant = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::RichDocument,
            &document_id,
            ResourceAction::Delete,
        )
        .await
        .map_err(|_| "rich-document delete grant missing")?
        .record_user_scope
        .grant_id
        .ok_or("rich-document delete grant missing")?;
        #[derive(Clone, Debug, SurrealValue, PartialEq)]
        struct DescendantGrantSnapshot {
            status: String,
            revoked_at: Option<surrealdb::types::Datetime>,
            grant_version: i64,
            policy_version: i64,
            updated_at: surrealdb::types::Datetime,
        }
        #[derive(SurrealValue)]
        struct DescendantGrantRestoreBindings {
            grant_id: String,
            status: String,
            revoked_at: Option<surrealdb::types::Datetime>,
            grant_version: i64,
            policy_version: i64,
            updated_at: surrealdb::types::Datetime,
        }
        let mut original_descendant = state
            .surreal
            .test_admin_query_bound(
                "SELECT status, revoked_at, grant_version, policy_version, updated_at FROM type::record('resource_grants', $grant_id);".to_owned(),
                json!({"grant_id": descendant_grant.clone()}),
            )
            .await?
            .check()?;
        let original_descendant_rows =
            original_descendant.take::<Vec<DescendantGrantSnapshot>>(0)?;
        let original_descendant = match original_descendant_rows.as_slice() {
            [grant] => grant.clone(),
            _ => return Err("exact descendant grant canonical read must return one row".into()),
        };
        assert_eq!(original_descendant.status, "active");
        assert_eq!(original_descendant.revoked_at, None);
        state.surreal.revoke_grant(&descendant_grant).await?;
        let mut revoked_descendant = state
            .surreal
            .test_admin_query_bound(
                "SELECT status, revoked_at, grant_version, policy_version, updated_at FROM type::record('resource_grants', $grant_id);".to_owned(),
                json!({"grant_id": descendant_grant.clone()}),
            )
            .await?
            .check()?;
        let revoked_descendant_rows = revoked_descendant.take::<Vec<DescendantGrantSnapshot>>(0)?;
        let revoked_descendant = match revoked_descendant_rows.as_slice() {
            [grant] => grant,
            _ => return Err("revoked descendant canonical read must return one row".into()),
        };
        assert_eq!(revoked_descendant.status, "revoked");
        assert!(revoked_descendant.revoked_at.is_some());
        assert_eq!(
            revoked_descendant.grant_version,
            original_descendant.grant_version + 1
        );
        assert_eq!(
            revoked_descendant.policy_version,
            original_descendant.policy_version + 1
        );
        assert_eq!(
            delete_workspace(
                State(state.clone()),
                Path(workspace.id.clone()),
                headers.clone()
            )
            .await
            .expect_err("revoked descendant")
            .0,
            StatusCode::FORBIDDEN
        );
        assert!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&document_id)
                .await?
                .is_some()
        );
        state.surreal.test_admin_query_bound(
            "UPDATE type::record('resource_grants', $grant_id) SET status = $status, revoked_at = $revoked_at, grant_version = $grant_version, policy_version = $policy_version, updated_at = $updated_at RETURN NONE;".to_owned(),
            DescendantGrantRestoreBindings {
                grant_id: descendant_grant.clone(),
                status: original_descendant.status.clone(),
                revoked_at: original_descendant.revoked_at.clone(),
                grant_version: original_descendant.grant_version,
                policy_version: original_descendant.policy_version,
                updated_at: original_descendant.updated_at.clone(),
            },
        ).await?.check()?;
        let mut restored_descendant = state
            .surreal
            .test_admin_query_bound(
                "SELECT status, revoked_at, grant_version, policy_version, updated_at FROM type::record('resource_grants', $grant_id);".to_owned(),
                json!({"grant_id": descendant_grant}),
            )
            .await?
            .check()?;
        assert_eq!(
            restored_descendant.take::<Vec<DescendantGrantSnapshot>>(0)?,
            vec![original_descendant]
        );
        // A draft in another workspace would be removed transitively via its document reference.
        let foreign = create_owned_test_workspace(&state, &headers).await?;
        let params = json!({"document": document_id, "workspace": foreign.id});
        state.surreal.test_admin_query_bound("CREATE type::record('knowledge_rich_document_drafts', $document) SET rich_document_id = type::record('knowledge_rich_documents', $document), workspace_id = type::record('workspaces', $workspace), base_doc_version = 1, base_content_sha256 = string::repeat('a', 64), draft_content_json = {}, draft_content_sha256 = string::repeat('b', 64), actor_kind = 'operator', actor_id = 'cross-workspace-fixture', kernel_task_run_id = 'cascade-proof', session_run_id = 'cascade-proof';".to_owned(), params).await?;
        assert_eq!(
            delete_workspace(
                State(state.clone()),
                Path(workspace.id.clone()),
                headers.clone()
            )
            .await
            .expect_err("cross-workspace cascade")
            .0,
            StatusCode::FORBIDDEN
        );
        assert!(state.storage.get_workspace(&workspace.id).await?.is_some());
        let doc_external = document_id.clone();
        state
            .surreal
            .test_admin_query_bound(
                "DELETE type::record('knowledge_rich_document_drafts', $document);".to_owned(),
                json!({"document": doc_external}),
            )
            .await?;
        let status = delete_workspace(
            State(state.clone()),
            Path(workspace.id.clone()),
            headers.clone(),
        )
        .await
        .map_err(|(status, Json(body))| format!("delete route failed: {status} {}", body.error))?;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&document_id)
                .await?
                .is_none()
        );
        assert!(
            crate::storage::surreal::SurrealDatabase::new(state.surreal.clone())
                .get_knowledge_rich_document(&scoped_document.rich_document_id)
                .await?
                .is_none()
        );
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &workspace.id).await?,
            0
        );
        let retry = delete_workspace(State(state.clone()), Path(workspace.id.clone()), headers)
            .await
            .expect_err("deleted and unauthorized targets use the same denial");
        assert_eq!(retry.0, StatusCode::FORBIDDEN);
        assert_eq!(retry.1 .0.error, "HSK-403-PROTECTED-RESOURCE");
        Ok(())
    }

    #[tokio::test]
    async fn owned_workspace_create_narrowed_session_rolls_back_every_row(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (owner, headers) =
            workspace_test_principal(&state, &binding, "workspace-create-rollback").await?;
        let snapshot = "RETURN { workspaces: (SELECT VALUE id FROM workspaces ORDER BY id), resources: (SELECT VALUE id FROM protected_resources ORDER BY id), grants: (SELECT VALUE id FROM resource_grants ORDER BY id) };";
        let mut before = state.surreal.test_admin_query(snapshot.to_owned()).await?;
        let before = before
            .take::<Option<serde_json::Value>>(0)?
            .expect("pre-mutation snapshot");
        state.surreal.test_admin_query_bound(
            "UPDATE authenticated_sessions SET delegated_capabilities = ['fs.write'] WHERE account_id = type::record('local_accounts', $account);".to_owned(),
            json!({"account": owner.identity.account_id}),
        ).await?;
        assert!(
            create_owned_test_workspace(&state, &headers).await.is_err(),
            "narrowed session cannot mint the full creator grants"
        );
        let mut after = state.surreal.test_admin_query(snapshot.to_owned()).await?;
        let after = after
            .take::<Option<serde_json::Value>>(0)?
            .expect("post-mutation snapshot");
        assert_eq!(
            after, before,
            "rejected grants must roll back source, resources, and both grants"
        );
        Ok(())
    }

    #[tokio::test]
    async fn owned_workspace_create_issues_exact_service_queue_and_revoked_root_rolls_back(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_owner, headers) =
            workspace_test_principal(&state, &binding, "workspace-service-queue-proof").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        let mut issued = state
            .surreal
            .test_admin_query_bound(
                "LET $account = (SELECT * FROM ONLY local_accounts WHERE account_key = 'mt109-reconciliation-service-account' LIMIT 1); LET $principal = (SELECT * FROM ONLY principals WHERE account_id = $account.id AND principal_key = 'mt109-reconciliation-service-principal' AND principal_kind = 'service_identity' AND status = 'enabled' LIMIT 1); LET $space = (SELECT * FROM ONLY access_spaces WHERE account_id = $account.id AND space_key = 'mt109-reconciliation-space' AND status = 'active' LIMIT 1); LET $root = (SELECT * FROM ONLY protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation' AND owner_account_id = $account.id AND created_by_principal_id = $principal.id AND access_space_id = $space.id AND lifecycle_state = 'active' LIMIT 1); LET $child = (SELECT * FROM ONLY protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation:' + $workspace AND parent_resource_id = $root.id AND owner_account_id = $account.id AND created_by_principal_id = $principal.id AND access_space_id = $space.id AND lifecycle_state = 'active' LIMIT 1); RETURN { child: array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation:' + $workspace AND parent_resource_id = $root.id AND owner_account_id = $account.id AND created_by_principal_id = $principal.id AND access_space_id = $space.id AND lifecycle_state = 'active'), owner: record::id($child.owner_account_id), expected_owner: record::id($account.id), principal: record::id($child.created_by_principal_id), expected_principal: record::id($principal.id), space: record::id($child.access_space_id), expected_space: record::id($space.id), root: record::id($root.id), grant: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id = $child.id AND account_id = $account.id AND principal_id = $principal.id AND access_space_id = $space.id AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] AND delegation_chain = [record::id($principal.id)] AND status = 'active' AND revoked_at = NONE AND expires_at = NONE) };".to_owned(),
                json!({"workspace": workspace.id.clone()}),
            )
            .await?;
        let issued = issued
            .take::<Option<serde_json::Value>>(5)?
            .expect("service queue result");
        assert_eq!(
            issued["child"], 1,
            "workspace event creates one exact child queue resource"
        );
        assert_eq!(
            issued["grant"], 1,
            "workspace event creates one exact child queue grant"
        );
        assert_eq!(
            issued["owner"], issued["expected_owner"],
            "child queue belongs to the canonical service account record"
        );
        assert_eq!(
            issued["principal"], issued["expected_principal"],
            "child queue uses the canonical service principal record"
        );
        assert_eq!(
            issued["space"], issued["expected_space"],
            "child queue uses the canonical service access-space record"
        );
        assert!(issued["root"].is_string());

        let mut root = state.surreal.test_admin_query(
            "RETURN (SELECT VALUE record::id(id) FROM resource_grants WHERE resource_id.resource_kind = 'reconciliation_queue' AND resource_id.external_resource_id = 'mt109-protected-reconciliation' AND status = 'active' AND revoked_at = NONE AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] LIMIT 1)[0];".to_owned(),
        ).await?;
        let root_grant = root
            .take::<Option<String>>(0)?
            .expect("canonical active root grant");
        state.surreal.revoke_grant(&root_grant).await?;
        let snapshot = "RETURN { workspaces: (SELECT VALUE id FROM workspaces ORDER BY id), resources: (SELECT VALUE id FROM protected_resources ORDER BY id), grants: (SELECT VALUE id FROM resource_grants ORDER BY id) };";
        let mut before = state.surreal.test_admin_query(snapshot.to_owned()).await?;
        let before = before
            .take::<Option<serde_json::Value>>(0)?
            .expect("before revoked-root create");
        assert!(
            create_owned_test_workspace(&state, &headers).await.is_err(),
            "a revoked canonical root grant must deny a new real workspace route"
        );
        let mut after = state.surreal.test_admin_query(snapshot.to_owned()).await?;
        assert_eq!(
            after.take::<Option<serde_json::Value>>(0)?.expect("after revoked-root create"),
            before,
            "the event fence must roll back workspace, human authority, and child queue issuance after root revocation"
        );
        let mut root_after = state.surreal.test_admin_query_bound(
            "RETURN { revoked: array::len(SELECT VALUE id FROM resource_grants WHERE id = type::record('resource_grants', $grant) AND status = 'revoked' AND revoked_at != NONE), active_equivalent: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id.resource_kind = 'reconciliation_queue' AND resource_id.external_resource_id = 'mt109-protected-reconciliation' AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] AND status = 'active' AND revoked_at = NONE) };".to_owned(),
            json!({"grant": root_grant}),
        ).await?;
        assert_eq!(
            root_after.take::<Option<serde_json::Value>>(0)?,
            Some(json!({"revoked": 1, "active_equivalent": 0})),
            "revoked root grant is never recreated or revived by workspace creation"
        );
        Ok(())
    }

    #[tokio::test]
    async fn owned_workspace_state_routes_roundtrip_and_deny_other_account(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::api::MountedRequestExt;
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_, headers) =
            workspace_test_principal(&state, &binding, "workspace-state-owner").await?;
        let (_, other_headers) =
            workspace_test_principal(&state, &binding, "workspace-state-other").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        let foreign_workspace = create_owned_test_workspace(&state, &other_headers).await?;
        let authority = crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            crate::storage::surreal::resource_authority::ResourceKind::Workspace,
            &workspace.id,
            crate::storage::surreal::resource_authority::ResourceAction::Update,
        )
        .await
        .map_err(|(status, body)| {
            std::io::Error::other(format!(
                "owner workspace update scope: {status}; {}",
                body.0
            ))
        })?;
        #[derive(surrealdb::types::SurrealValue)]
        struct StateSwapBindings {
            workspace: String,
            foreign_workspace: String,
        }
        let panes: Vec<Value> = ["pane-a", "pane-b", "pane-c", "pane-d"].into_iter().map(|id| json!({"id":id,"module":"MAIN","activeTab":"workspace","tabs":["workspace"],"locked":false,"projectRef":"","activeDocumentId":null,"activeCanvasId":null,"openDocuments":[]})).collect();
        for (route, field, value) in [
            (
                "workbench/layout",
                "layout_state",
                json!({"schema_id":"hsk.workbench_layout_state@1","activePaneId":"pane-a","activeModule":"MAIN","splitWeights":{"vertical":0.5,"horizontal":0.5},"drawers":{"project":true,"file":true,"bottom":false},"panes":panes}),
            ),
            (
                "settings",
                "settings_state",
                json!({"schema_id":"hsk.workspace_settings_state@1","theme":"dark","custom_theme_tokens":{},"keybindings":{"app.quick_switcher.open":"Mod-k","app.command_palette.open":"Mod-p"},"settings":{"view_mode":"SFW","swarm_board_default_open":false}}),
            ),
            (
                "search-bookmarks",
                "bookmark_state",
                json!({"schema_id":"hsk.workspace_search_bookmark_state@1","bookmarks":[]}),
            ),
        ] {
            let uri = format!("/workspaces/{}/{route}", workspace.id);
            for method in ["PUT", "PUT", "GET"] {
                let mut request = axum::http::Request::builder()
                    .method(method)
                    .uri(&uri)
                    .header("content-type", "application/json");
                for (name, value) in &headers {
                    request = request.header(name, value);
                }
                let payload = if method == "PUT" {
                    serde_json::to_vec(&json!({field:value}))?
                } else {
                    Vec::new()
                };
                let response = routes(state.clone())
                    .oneshot(request.body(axum::body::Body::from(payload))?)
                    .await?;
                let status = response.status();
                let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
                let body: Value = serde_json::from_slice(&body)?;
                assert_eq!(status, StatusCode::OK, "{method} {route}: {body}");
                assert_eq!(body[field], value);
            }
            for method in ["GET", "PUT"] {
                let mut request = axum::http::Request::builder()
                    .method(method)
                    .uri(&uri)
                    .header("content-type", "application/json");
                for (name, value) in &other_headers {
                    request = request.header(name, value);
                }
                let response = routes(state.clone())
                    .oneshot(request.body(axum::body::Body::from(serde_json::to_vec(
                        &json!({field:value}),
                    )?))?)
                    .await?;
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            }
            let scope = authority.record_user_scope.clone();
            let bindings = StateSwapBindings {
                workspace: workspace.id.clone(),
                foreign_workspace: foreign_workspace.id.clone(),
            };
            let (statement, state_table) = match route {
                "workbench/layout" => ("UPDATE type::record('knowledge_workbench_layout_states', $workspace) SET workspace_id = type::record('workspaces', $foreign_workspace) RETURN AFTER;", "knowledge_workbench_layout_states"),
                "settings" => ("UPDATE type::record('knowledge_workspace_settings_states', $workspace) SET workspace_id = type::record('workspaces', $foreign_workspace) RETURN AFTER;", "knowledge_workspace_settings_states"),
                "search-bookmarks" => ("UPDATE type::record('knowledge_workspace_search_bookmark_states', $workspace) SET workspace_id = type::record('workspaces', $foreign_workspace) RETURN AFTER;", "knowledge_workspace_search_bookmark_states"),
                _ => unreachable!("state route fixture is exhaustive"),
            };
            let error = state
                .surreal
                .with_record_user_scope(
                    scope,
                    state.surreal.with_data_operation(move |database| {
                        Box::pin(
                            async move { database.execute_returning(statement, bindings).await },
                        )
                    }),
                )
                .await
                .expect_err("record-user workspace swap must fail at the immutable field boundary");
            let rendered = error.to_string();
            assert!(
                rendered.contains(state_table)
                    && rendered.contains("record::exists($value) AND record::id($value) = record::id($this.id)"),
                "workspace swap must be rejected by the immutable workspace field boundary: {error}"
            );
            let mut request = axum::http::Request::builder().method("GET").uri(&uri);
            for (name, value) in &headers {
                request = request.header(name, value);
            }
            let response = routes(state.clone())
                .oneshot(request.body(axum::body::Body::empty())?)
                .await?;
            let status = response.status();
            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
            let body: Value = serde_json::from_slice(&body)?;
            assert_eq!(
                status,
                StatusCode::OK,
                "canonical state reread {route}: {body}"
            );
            assert_eq!(
                body[field], value,
                "rejected workspace swap must not persist"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn owned_workspace_delete_denies_other_account_ownerless_and_foreign_descendants(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (owner, owner_headers) =
            workspace_test_principal(&state, &binding, "workspace-owner-a").await?;
        let (other, other_headers) =
            workspace_test_principal(&state, &binding, "workspace-owner-b").await?;
        let workspace = create_owned_test_workspace(&state, &owner_headers).await?;
        let other_list = list_workspaces(State(state.clone()), other_headers.clone())
            .await
            .map_err(|_| "other-account listing failed")?
            .0;
        assert!(!other_list.iter().any(|row| row.id == workspace.id));
        let denied = delete_workspace(
            State(state.clone()),
            Path(workspace.id.clone()),
            other_headers,
        )
        .await
        .expect_err("other account cannot delete");
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert!(state.storage.get_workspace(&workspace.id).await?.is_some());
        let legacy = create_owned_test_workspace(&state, &owner_headers).await?;
        let owner_account_id = owner.identity.account_id.clone();
        let owner_principal_id = owner.identity.principal_id.clone();
        let owner_space_id = owner.identity.access_space_id.clone();
        state
            .surreal
            .test_admin_query_bound(
                "BEGIN TRANSACTION; \
                 LET $account = type::record('local_accounts', $account_id); \
                 LET $principal = type::record('principals', $principal_id); \
                 LET $space = type::record('access_spaces', $space_id); \
                 LET $resources = SELECT VALUE id FROM protected_resources WHERE resource_kind = 'workspace' AND external_resource_id = $workspace_id AND owner_account_id = $account AND created_by_principal_id = $principal AND access_space_id = $space AND lifecycle_state = 'active' LIMIT 2; \
                 IF array::len($resources) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                 LET $resource = $resources[0]; \
                 LET $grants = SELECT VALUE id FROM resource_grants WHERE resource_id = $resource AND account_id = $account AND principal_id = $principal AND access_space_id = $space AND status = 'active' AND revoked_at = NONE LIMIT 2; \
                 IF array::len($grants) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                 LET $grant = $grants[0]; \
                 DELETE $grant RETURN NONE; \
                 DELETE $resource RETURN NONE; \
                COMMIT TRANSACTION;"
                    .to_owned(),
                json!({
                    "workspace_id": legacy.id.clone(),
                    "account_id": owner_account_id.clone(),
                    "principal_id": owner_principal_id.clone(),
                    "space_id": owner_space_id.clone(),
                }),
            )
            .await?
            .check()?;
        let mut ownerless_authority = state
            .surreal
            .test_admin_query_bound(
                "RETURN { human_resources: array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'workspace' AND external_resource_id = $workspace_id AND owner_account_id = type::record('local_accounts', $account_id) AND created_by_principal_id = type::record('principals', $principal_id) AND access_space_id = type::record('access_spaces', $space_id)), human_grants: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id.resource_kind = 'workspace' AND resource_id.external_resource_id = $workspace_id AND account_id = type::record('local_accounts', $account_id) AND principal_id = type::record('principals', $principal_id) AND access_space_id = type::record('access_spaces', $space_id)), service_queue_resources: array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation:' + $workspace_id AND lifecycle_state = 'active'), service_queue_grants: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id.resource_kind = 'reconciliation_queue' AND resource_id.external_resource_id = 'mt109-protected-reconciliation:' + $workspace_id AND status = 'active' AND revoked_at = NONE) };".to_owned(),
                json!({
                    "workspace_id": legacy.id.clone(),
                    "account_id": owner_account_id,
                    "principal_id": owner_principal_id,
                    "space_id": owner_space_id,
                }),
            )
            .await?
            .check()?;
        let ownerless_authority = ownerless_authority
            .take::<Option<Value>>(0)?
            .expect("ownerless authority snapshot");
        assert_eq!(ownerless_authority["human_resources"], json!(0));
        assert_eq!(ownerless_authority["human_grants"], json!(0));
        assert_eq!(ownerless_authority["service_queue_resources"], json!(1));
        assert_eq!(ownerless_authority["service_queue_grants"], json!(1));
        assert_eq!(
            delete_workspace(
                State(state.clone()),
                Path(legacy.id.clone()),
                owner_headers.clone()
            )
            .await
            .expect_err("ownerless cannot be claimed")
            .0,
            StatusCode::FORBIDDEN
        );
        assert!(state.storage.get_workspace(&legacy.id).await?.is_some());
        let authority = crate::api::authority::authorize_request(
            &state,
            &owner_headers,
            "fs.write",
            ResourceKind::Workspace,
            &workspace.id,
            ResourceAction::Delete,
        )
        .await
        .map_err(|_| "owner grant missing")?;
        state
            .surreal
            .register_protected_resource(
                &other.identity,
                ResourceKind::RichDocument,
                "foreign-descendant",
                Some(&authority.resource_id),
                "account_private",
            )
            .await?;
        assert_eq!(
            delete_workspace(
                State(state.clone()),
                Path(workspace.id.clone()),
                owner_headers
            )
            .await
            .expect_err("foreign descendant must prevent cascade")
            .0,
            StatusCode::FORBIDDEN
        );
        assert!(state.storage.get_workspace(&workspace.id).await?.is_some());
        Ok(())
    }

    async fn owned_route_json(
        router: axum::Router,
        method: &str,
        uri: &str,
        headers: &HeaderMap,
        body: Option<Value>,
    ) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
        let mut request = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-actor-id", "workspace-cascade-proof")
            .header("x-hsk-kernel-task-run-id", "workspace-cascade-proof")
            .header("x-hsk-session-run-id", "workspace-cascade-proof");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let payload = match body {
            Some(body) => serde_json::to_vec(&body)?,
            None => Vec::new(),
        };
        let response = router
            .oneshot(request.body(axum::body::Body::from(payload))?)
            .await?;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        Ok((status, value))
    }

    /// MT-109 C2 items 6 and 8: the production workspace create provisions the owner's memory
    /// surfaces (Master Spec 02:2776), and the EventLedger aggregate, debug-session and
    /// source-control routes deny an unauthenticated caller (02:2758 deny by default).
    #[tokio::test]
    async fn mt109_c2_memory_surfaces_provisioned_and_process_routes_deny_by_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_owner, headers) =
            workspace_test_principal(&state, &binding, "mt109-c2-owner").await?;
        let (_other, other_headers) =
            workspace_test_principal(&state, &binding, "mt109-c2-other").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        let ws = workspace.id.clone();
        for (capability, kind, action) in [
            (
                "memory.read",
                ResourceKind::MemoryPack,
                ResourceAction::Read,
            ),
            (
                "memory.read",
                ResourceKind::MemoryProposal,
                ResourceAction::Read,
            ),
            (
                "memory.propose",
                ResourceKind::MemoryProposal,
                ResourceAction::Create,
            ),
            (
                "memory.read",
                ResourceKind::MemoryCommitReport,
                ResourceAction::Read,
            ),
            (
                "memory.read",
                ResourceKind::MemoryItemCount,
                ResourceAction::Read,
            ),
        ] {
            let owner = crate::api::authority::authorize_request(
                &state, &headers, capability, kind, &ws, action,
            )
            .await;
            assert!(
                owner.is_ok(),
                "owner {capability} {} must be provisioned by the production workspace create",
                kind.as_str()
            );
            let other = crate::api::authority::authorize_request(
                &state,
                &other_headers,
                capability,
                kind,
                &ws,
                action,
            )
            .await;
            assert!(
                other.is_err(),
                "another account stays denied on {}",
                kind.as_str()
            );
        }
        let (status, body) = owned_route_json(
            crate::api::memory::routes(state.clone()),
            "GET",
            &format!("/workspaces/{ws}/memory/proposals"),
            &headers,
            None,
        )
        .await?;
        assert_ne!(
            status,
            StatusCode::FORBIDDEN,
            "owner memory read route: {body}"
        );

        // Item 2 (diagnosis b): an owner rename of a standalone block commits with its
        // KNOWLEDGE_LOOM_BLOCK_MUTATED receipt stamped with the session principal.
        let (status, note) = owned_route_json(
            crate::api::loom::routes(state.clone()),
            "POST",
            &format!("/workspaces/{ws}/loom/blocks"),
            &headers,
            Some(json!({"content_type": "note", "title": "C2 rename source"})),
        )
        .await?;
        assert!(
            status.is_success(),
            "create standalone note -> {status}: {note}"
        );
        let note_id = note["block_id"].as_str().ok_or("note block_id")?.to_owned();
        let (status, renamed) = owned_route_json(
            crate::api::loom::routes(state.clone()),
            "PATCH",
            &format!("/workspaces/{ws}/loom/blocks/{note_id}"),
            &headers,
            Some(json!({"title": "C2 renamed"})),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "owner rename PATCH: {renamed}");
        assert_eq!(renamed["title"], "C2 renamed");

        // Item 7: tag, mention and unresolved-wikilink backlinks no longer fail the create.
        let (status, created) = owned_route_json(
            crate::api::knowledge_documents::routes(state.clone()),
            "POST",
            "/knowledge/documents",
            &headers,
            Some(json!({
                "workspace_id": ws,
                "title": "C2 link kinds",
                "content_json": {"type": "doc", "content": [{"type": "paragraph", "content": [
                    {"type": "text", "text": "see [[C2 Missing Target]] and #ops"}
                ]}]}
            })),
        )
        .await?;
        assert!(
            status.is_success(),
            "record-user document create with non-document links -> {status}: {created}"
        );

        let anonymous = HeaderMap::new();
        let aggregate = format!("/kernel/events/aggregates/workspace/{ws}");
        let (status, _) = owned_route_json(
            crate::api::kernel::routes(state.clone()),
            "GET",
            &aggregate,
            &anonymous,
            None,
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "anonymous aggregate read");
        let (status, body) = owned_route_json(
            crate::api::kernel::routes(state.clone()),
            "GET",
            &aggregate,
            &headers,
            None,
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "owner aggregate read: {body}");
        let (status, events) = owned_route_json(
            crate::api::kernel::routes(state.clone()),
            "GET",
            &format!("/kernel/events/aggregates/loom_block/{note_id}"),
            &headers,
            None,
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "owner block ledger read: {events}");
        assert!(
            events.as_array().is_some_and(|rows| rows.iter().any(|row| {
                row["event_type"] == "KNOWLEDGE_LOOM_BLOCK_MUTATED"
                    && row["actor"]["kind"] == "operator"
            })),
            "the rename receipt is visible to its owner with the session actor: {events}"
        );
        let (status, other_events) = owned_route_json(
            crate::api::kernel::routes(state.clone()),
            "GET",
            &format!("/kernel/events/aggregates/loom_block/{note_id}"),
            &other_headers,
            None,
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            other_events.as_array().map(Vec::len),
            Some(0),
            "another account reads none of the owner's receipts: {other_events}"
        );
        let (status, _) = owned_route_json(
            crate::api::debug_adapter::routes(state.clone()),
            "POST",
            "/debug/sessions",
            &anonymous,
            Some(json!({})),
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "anonymous debug launch");
        let (status, _) = owned_route_json(
            crate::api::source_control::routes(state.clone()),
            "GET",
            "/source-control/status?repo_path=.",
            &anonymous,
            None,
        )
        .await?;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "anonymous source-control read"
        );
        Ok(())
    }

    async fn workspace_cascade_rows(
        state: &AppState,
        workspace_id: &str,
        document_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let mut result = state
            .surreal
            .test_admin_query_bound(
                "LET $ws = type::record('workspaces', $workspace_id); \
                 RETURN { \
                   workspace: array::len(SELECT VALUE id FROM $ws), \
                   documents: array::len(SELECT VALUE id FROM knowledge_rich_documents WHERE workspace_id = $ws), \
                   versions: array::len(SELECT VALUE id FROM knowledge_rich_document_versions WHERE rich_document_id = type::record('knowledge_rich_documents', $document_id)), \
                   boards: array::len(SELECT VALUE id FROM loom_canvas_boards WHERE workspace_id = $ws), \
                   placements: array::len(SELECT VALUE id FROM loom_canvas_placements WHERE workspace_id = $ws), \
                   visual_edges: array::len(SELECT VALUE id FROM loom_canvas_visual_edges WHERE workspace_id = $ws), \
                   loom_blocks: array::len(SELECT VALUE id FROM loom_blocks WHERE workspace_id = $ws) \
                 };"
                .to_owned(),
                json!({"workspace_id": workspace_id, "document_id": document_id}),
            )
            .await?
            .check()?;
        Ok(result
            .take::<Option<Value>>(1)?
            .ok_or("workspace cascade row snapshot missing")?)
    }

    /// MT-109 C1-FDELETE (Operator decision 2026-09-22): an owner-authorized workspace delete cascades
    /// to its rich documents, their immutable version history and its Canvas boards with zero residue
    /// and one audit event; a non-owner delete is 403 and leaves every row intact. Canvas visual-edge
    /// routes are 401 without a session and 403 for an authenticated non-owner (MT-111 split).
    #[tokio::test]
    async fn owned_workspace_delete_cascades_documents_versions_and_canvas_with_audit(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_owner, headers) =
            workspace_test_principal(&state, &binding, "workspace-cascade-owner").await?;
        let (_other, other_headers) =
            workspace_test_principal(&state, &binding, "workspace-cascade-other").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        let ws = workspace.id.clone();

        // Created with content, as the native editor does (see MT-109 finding C1-EMPTY-SAVE).
        let (status, created) = owned_route_json(
            crate::api::knowledge_documents::routes(state.clone()),
            "POST",
            "/knowledge/documents",
            &headers,
            Some(json!({
                "workspace_id": ws,
                "title": "Cascade source document",
                "content_json": {"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "first version"}]}]}
            })),
        )
        .await?;
        assert!(
            status.is_success(),
            "create rich document -> {status}: {created}"
        );
        let document_id = created["document"]["rich_document_id"]
            .as_str()
            .ok_or("created rich_document_id")?
            .to_owned();
        // Version history is proven by the create-time version row. A route-level save in this
        // embedded harness returns 500 (MT-109 finding C1-SAVE-500); not needed for the cascade proof.

        let loom = || crate::api::loom::routes(state.clone());
        let mut note_ids = Vec::new();
        for title in ["Cascade note A", "Cascade note B"] {
            let (status, note) = owned_route_json(
                loom(),
                "POST",
                &format!("/workspaces/{ws}/loom/blocks"),
                &headers,
                Some(json!({"content_type": "note", "title": title})),
            )
            .await?;
            assert!(status.is_success(), "create note block -> {status}: {note}");
            note_ids.push(note["block_id"].as_str().ok_or("note block_id")?.to_owned());
        }
        let (status, canvas) = owned_route_json(
            loom(),
            "POST",
            &format!("/workspaces/{ws}/loom/canvas-boards"),
            &headers,
            Some(json!({"title": "Cascade canvas"})),
        )
        .await?;
        assert!(status.is_success(), "create canvas -> {status}: {canvas}");
        let canvas_id = canvas["block_id"]
            .as_str()
            .ok_or("canvas block_id")?
            .to_owned();
        let mut placements = Vec::new();
        for (note_id, x) in note_ids.iter().zip([0.0, 300.0]) {
            let (status, placed) = owned_route_json(
                loom(),
                "POST",
                &format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/placements"),
                &headers,
                Some(json!({"placed_block_id": note_id, "x": x, "y": 0.0, "w": 200.0, "h": 120.0})),
            )
            .await?;
            assert!(status.is_success(), "place block -> {status}: {placed}");
            placements.push(
                placed["placement"]["placement_id"]
                    .as_str()
                    .or_else(|| placed["placement_id"].as_str())
                    .ok_or_else(|| format!("placement id in {placed}"))?
                    .to_owned(),
            );
        }
        let edge_uri = format!("/workspaces/{ws}/loom/canvas-boards/{canvas_id}/visual-edges");
        let edge_body =
            json!({"from_placement_id": placements[0], "to_placement_id": placements[1]});

        // Visual edges: no session -> 401, authenticated non-owner -> 403, owner -> created.
        let (status, body) = owned_route_json(
            loom(),
            "POST",
            &edge_uri,
            &HeaderMap::new(),
            Some(edge_body.clone()),
        )
        .await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "HSK-401-LOOM-SESSION");
        let (status, body) = owned_route_json(
            loom(),
            "POST",
            &edge_uri,
            &other_headers,
            Some(edge_body.clone()),
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "HSK-403-PROTECTED-RESOURCE");
        let (status, edge) =
            owned_route_json(loom(), "POST", &edge_uri, &headers, Some(edge_body)).await?;
        assert!(status.is_success(), "owner visual edge -> {status}: {edge}");
        let edge_id = edge["visual_edge_id"]
            .as_str()
            .ok_or("visual_edge_id")?
            .to_owned();
        let edge_delete_uri = format!("/workspaces/{ws}/loom/canvas-visual-edges/{edge_id}");
        let (status, _) =
            owned_route_json(loom(), "DELETE", &edge_delete_uri, &HeaderMap::new(), None).await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, body) =
            owned_route_json(loom(), "DELETE", &edge_delete_uri, &other_headers, None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        let unknown_uri = format!(
            "/workspaces/{ws}/loom/canvas-visual-edges/LCV-00000000000000000000000000000000"
        );
        let (status, _) = owned_route_json(loom(), "DELETE", &unknown_uri, &headers, None).await?;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "unknown edge uses the constant denial"
        );

        let before = workspace_cascade_rows(&state, &ws, &document_id).await?;
        assert_eq!(before["workspace"], json!(1), "{before}");
        assert_eq!(before["documents"], json!(1), "{before}");
        assert!(
            before["versions"].as_i64().unwrap_or(0) >= 1,
            "document keeps its version history: {before}"
        );
        assert_eq!(before["boards"], json!(1), "{before}");
        assert_eq!(before["placements"], json!(2), "{before}");
        assert_eq!(before["visual_edges"], json!(1), "{before}");

        // A non-owner delete is denied and leaves every row intact.
        let denied = delete_workspace(State(state.clone()), Path(ws.clone()), other_headers)
            .await
            .expect_err("non-owner cannot delete");
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert_eq!(
            workspace_cascade_rows(&state, &ws, &document_id).await?,
            before
        );

        // MT-109 C1-FDELETE probe (env-gated): install a numbered copy of the workspace-delete guard;
        // the real delete route evaluates it on its own record-user connection if it fails.
        if std::env::var_os("HSK_C1_FDELETE_PROBE").is_some() {
            let schema = include_str!("../storage/surreal/schema.surql");
            let start = schema
                .find("DEFINE FUNCTION OVERWRITE fn::mt120_workspace_delete($external: string) {")
                .ok_or("guard start")?;
            let rest = &schema[start..];
            let end = rest[1..].find("\nDEFINE ").ok_or("guard end")? + 1;
            let mut clauses = 0usize;
            let mut probe = String::new();
            for line in rest[..end].lines() {
                let mut line = line.replace(
                    "fn::mt120_workspace_delete($external",
                    "fn::c1_probe_workspace_delete($external",
                );
                while let Some(at) = line.find("RETURN false;") {
                    clauses += 1;
                    eprintln!(
                        "C1_FDELETE_CLAUSE {clauses}: {}",
                        line.trim().chars().take(200).collect::<String>()
                    );
                    line.replace_range(
                        at..at + "RETURN false;".len(),
                        &format!("RETURN 'clause-{clauses}';"),
                    );
                }
                probe.push_str(&line);
                probe.push('\n');
            }
            state
                .surreal
                .test_admin_query_bound(probe, json!({}))
                .await?
                .check()?;
        }

        // The owner delete cascades with zero residue.
        let status = delete_workspace(State(state.clone()), Path(ws.clone()), headers.clone())
            .await
            .map_err(|(status, Json(body))| format!("owner delete: {status} {}", body.error))?;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let after = workspace_cascade_rows(&state, &ws, &document_id).await?;
        assert_eq!(
            after,
            json!({"workspace": 0, "documents": 0, "versions": 0, "boards": 0, "placements": 0, "visual_edges": 0, "loom_blocks": 0}),
            "workspace delete must leave zero residue"
        );
        let audit_id = format!("{WORKSPACE_DELETE_AUDIT_PREFIX}{ws}");
        let audits = state
            .flight_recorder
            .list_events(crate::flight_recorder::EventFilter {
                wsid: Some(ws.clone()),
                ..Default::default()
            })
            .await?;
        assert_eq!(
            audits
                .iter()
                .filter(|event| event.policy_decision_id.as_deref() == Some(audit_id.as_str()))
                .count(),
            1,
            "exactly one workspace-delete audit event survives the purge"
        );
        Ok(())
    }

    /// MT-154 AC-154-6 / MT-153 AC-153-8 (Operator decision 2026-09-22 extended as C2 did): an owner's
    /// DELETE /workspaces/:id succeeds when the workspace holds calendar, Stage, Canvas and Loom folder
    /// rows created through the account-scoped routes, and removes every one of them; a non-owner delete
    /// is denied and leaves them intact.
    #[tokio::test]
    async fn mt154_owner_workspace_delete_removes_calendar_stage_canvas_rows(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use base64::Engine as _;
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_owner, headers) =
            workspace_test_principal(&state, &binding, "mt154-delete-owner").await?;
        let (_other, other_headers) =
            workspace_test_principal(&state, &binding, "mt154-delete-other").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        let ws = workspace.id.clone();

        let (status, source) = owned_route_json(
            crate::api::calendar::routes(state.clone()),
            "PUT",
            &format!("/workspaces/{ws}/calendar/sources/mt154-delete-src"),
            &headers,
            Some(json!({
                "display_name": "MT-154 delete calendar",
                "provider_type": "local",
                "write_policy": "read_only_import",
                "default_tzid": "UTC",
            })),
        )
        .await?;
        assert!(status.is_success(), "calendar source -> {status}: {source}");
        let (status, canvas) = owned_route_json(
            crate::api::canvases::routes(state.clone()),
            "POST",
            &format!("/workspaces/{ws}/canvases"),
            &headers,
            Some(json!({"title": "MT-154 delete canvas"})),
        )
        .await?;
        assert!(status.is_success(), "canvas -> {status}: {canvas}");
        let (status, artifact) = owned_route_json(
            crate::api::stage::routes(state.clone()),
            "POST",
            &format!("/workspaces/{ws}/stage/artifacts"),
            &headers,
            Some(json!({
                "schema_version": crate::api::stage::STAGE_CAPTURE_SCHEMA,
                "idempotency_key": format!("mt154-delete-{}", Uuid::now_v7()),
                "correlation_id": format!("mt154-delete-corr-{}", Uuid::now_v7()),
                "content_kind": "selection",
                "label": "MT-154 delete capture",
                "content_type": "text/plain",
                "content_base64": base64::engine::general_purpose::STANDARD.encode(b"mt154 capture"),
            })),
        )
        .await?;
        assert!(
            status.is_success(),
            "stage artifact -> {status}: {artifact}"
        );
        let (status, folder) = owned_route_json(
            crate::api::loom::routes(state.clone()),
            "POST",
            &format!("/workspaces/{ws}/loom/folders"),
            &headers,
            Some(json!({"name": "MT-154 delete folder"})),
        )
        .await?;
        assert!(status.is_success(), "loom folder -> {status}: {folder}");

        let rows = |state: AppState, ws: String| async move {
            let mut result = state
                .surreal
                .test_admin_query_bound(
                    "LET $ws = type::record('workspaces', $workspace_id); \
                     RETURN { \
                       workspace: array::len(SELECT VALUE id FROM $ws), \
                       calendar_sources: array::len(SELECT VALUE id FROM calendar_sources WHERE workspace_id = $ws), \
                       canvases: array::len(SELECT VALUE id FROM canvases WHERE workspace_id = $ws), \
                       stage_artifacts: array::len(SELECT VALUE id FROM stage_capture_artifacts WHERE workspace_id = $ws), \
                       loom_folders: array::len(SELECT VALUE id FROM loom_folders WHERE workspace_id = $ws) \
                     };"
                    .to_owned(),
                    json!({"workspace_id": ws}),
                )
                .await?
                .check()?;
            result
                .take::<Option<Value>>(1)?
                .ok_or_else(|| "MT-154 delete row snapshot missing".into())
        };
        let before: Value = rows(state.clone(), ws.clone())
            .await
            .map_err(|error: Box<dyn std::error::Error>| error.to_string())?;
        assert_eq!(
            before,
            json!({"workspace": 1, "calendar_sources": 1, "canvases": 1, "stage_artifacts": 1, "loom_folders": 1}),
            "every scoped surface wrote its row: {before}"
        );

        let denied = delete_workspace(State(state.clone()), Path(ws.clone()), other_headers)
            .await
            .expect_err("non-owner cannot delete");
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        let unchanged: Value = rows(state.clone(), ws.clone())
            .await
            .map_err(|error: Box<dyn std::error::Error>| error.to_string())?;
        assert_eq!(unchanged, before, "a denied delete leaves every row intact");

        let status = delete_workspace(State(state.clone()), Path(ws.clone()), headers.clone())
            .await
            .map_err(|(status, Json(body))| format!("owner delete: {status} {}", body.error))?;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let after: Value = rows(state.clone(), ws.clone())
            .await
            .map_err(|error: Box<dyn std::error::Error>| error.to_string())?;
        assert_eq!(
            after,
            json!({"workspace": 0, "calendar_sources": 0, "canvases": 0, "stage_artifacts": 0, "loom_folders": 0}),
            "the owner's workspace delete removes calendar, Stage, Canvas and Loom folder rows"
        );
        Ok(())
    }

    #[tokio::test]
    async fn owned_workspace_delete_rolls_back_source_and_grant_revocation_on_cascade_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let binding = WorkspaceBindingFixture::new()?;
        let (state, _store) = setup_state().await?;
        let (_, headers) =
            workspace_test_principal(&state, &binding, "workspace-rollback-owner").await?;
        let workspace = create_owned_test_workspace(&state, &headers).await?;
        state.surreal.test_admin_query("DEFINE EVENT OVERWRITE workspace_delete_proof_failure ON TABLE workspaces WHEN $event = 'DELETE' THEN { THROW 'INJECTED_WORKSPACE_DELETE_ROLLBACK'; };".to_owned()).await?;
        assert_eq!(
            delete_workspace(
                State(state.clone()),
                Path(workspace.id.clone()),
                headers.clone()
            )
            .await
            .expect_err("injected cascade failure")
            .0,
            StatusCode::FORBIDDEN
        );
        assert!(state.storage.get_workspace(&workspace.id).await?.is_some());
        crate::api::authority::authorize_request(
            &state,
            &headers,
            "fs.write",
            ResourceKind::Workspace,
            &workspace.id,
            ResourceAction::Delete,
        )
        .await
        .map_err(|_| "rolled-back deletion must retain active grant")?;
        state
            .surreal
            .test_admin_query(
                "REMOVE EVENT workspace_delete_proof_failure ON TABLE workspaces;".to_owned(),
            )
            .await?;
        assert_eq!(
            delete_workspace(State(state.clone()), Path(workspace.id), headers)
                .await
                .map_err(|_| "retry deletion failed")?,
            StatusCode::NO_CONTENT
        );
        Ok(())
    }

    #[tokio::test]
    async fn verify_atelier_apply_provenance_accepts_matching_job_output(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (state, _store) = setup_state().await?;

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
                trace_id: Uuid::now_v7(),
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

        let suggestion_id = Uuid::now_v7().to_string();
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
        let (state, _store) = setup_state().await?;

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
                trace_id: Uuid::now_v7(),
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

        let suggestion_id = Uuid::now_v7().to_string();
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
        let (state, _store) = setup_state().await?;

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
                trace_id: Uuid::now_v7(),
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

        let suggestion_id = Uuid::now_v7().to_string();
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
        let (state, _store) = setup_state().await?;

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
        let (state, _store) = setup_state().await?;

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
                trace_id: Uuid::now_v7(),
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

    /// MT-258 saved-search durability: the search-bookmarks route must persist to
    /// the durable store + EventLedger (NOT localStorage), survive a re-read, replace the
    /// blob on a second save, reject a malformed blob, and emit a kernel
    /// EventLedger receipt per save. This proves saved searches are canonical
    /// state and the UI is a projection.
    #[tokio::test]
    async fn mt258_search_bookmarks_persist_to_surreal_and_event_ledger(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::kernel::KernelEventType;

        let (state, _store) = setup_state().await?;
        let workspace = state
            .storage
            .create_workspace(
                &WriteContext::human(Some("tester".into())),
                NewWorkspace {
                    name: "search-bookmark-ws".into(),
                },
            )
            .await?;
        let workspace_id = workspace.id.clone();

        // No saved searches yet: GET returns an empty projection.
        let empty =
            get_workspace_search_bookmarks(State(state.clone()), Path(workspace_id.clone()))
                .await
                .map_err(|(status, Json(body))| format!("get failed {status}: {}", body.error))?
                .0;
        assert!(
            empty.bookmark_state.is_none(),
            "fresh workspace has no saved searches"
        );

        let bookmark = json!({
            "schema_id": "hsk.workspace_search_bookmark_state@1",
            "bookmarks": [{
                "id": "bm-1",
                "label": "TODO blocks",
                "query": "TODO",
                "kind": "all",
                "tagFilter": "",
                "pathFilter": "",
                "caseSensitive": false,
                "wholeWord": true,
                "isRegex": false,
                "savedAt": "2026-06-17T00:00:00Z"
            }]
        });
        let saved = save_workspace_search_bookmarks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(SaveWorkspaceSearchBookmarksRequest {
                bookmark_state: bookmark.clone(),
            }),
        )
        .await
        .map_err(|(status, Json(body))| format!("save failed {status}: {}", body.error))?
        .0;
        let first_event_id = saved
            .event_ledger_event_id
            .clone()
            .expect("save returns an EventLedger receipt id");
        assert!(!first_event_id.trim().is_empty());

        // Re-read straight from the embedded store (new GET) confirms durability.
        let reread =
            get_workspace_search_bookmarks(State(state.clone()), Path(workspace_id.clone()))
                .await
                .map_err(|(status, Json(body))| format!("re-read failed {status}: {}", body.error))?
                .0;
        assert_eq!(
            reread.bookmark_state.as_ref(),
            Some(&bookmark),
            "saved searches must round-trip through the embedded store"
        );

        // Second save replaces the blob (the route is the canonical authority).
        let updated = json!({
            "schema_id": "hsk.workspace_search_bookmark_state@1",
            "bookmarks": []
        });
        let cleared = save_workspace_search_bookmarks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(SaveWorkspaceSearchBookmarksRequest {
                bookmark_state: updated.clone(),
            }),
        )
        .await
        .map_err(|(status, Json(body))| format!("update failed {status}: {}", body.error))?
        .0;
        assert_eq!(cleared.bookmark_state.as_ref(), Some(&updated));
        assert_ne!(
            cleared.event_ledger_event_id, saved.event_ledger_event_id,
            "each save must append a fresh EventLedger receipt"
        );

        // A malformed blob (missing required schema_id) is rejected, not stored.
        let bad = save_workspace_search_bookmarks(
            State(state.clone()),
            Path(workspace_id.clone()),
            Json(SaveWorkspaceSearchBookmarksRequest {
                bookmark_state: json!({ "bookmarks": [] }),
            }),
        )
        .await;
        assert!(bad.is_err(), "blob without schema_id must be rejected");

        // The EventLedger holds a durable receipt for this workspace aggregate.
        let events = state
            .storage
            .list_kernel_events_for_aggregate("workspace_search_bookmark_state", &workspace_id)
            .await?;
        let receipts: Vec<_> = events
            .iter()
            .filter(|event| {
                event.event_type == KernelEventType::KnowledgeWorkspaceSearchBookmarkStateRecorded
            })
            .collect();
        assert!(
            receipts.len() >= 2,
            "two successful saves must yield at least two EventLedger receipts, got {}",
            receipts.len()
        );

        Ok(())
    }
}
