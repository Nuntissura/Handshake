//! WP-CKC-posekit-overhaul SurrealDB port — CKC `prompt_feedback` lane router (MT-020).
//!
//! The deterministic prompt-feedback kernel over HTTP: import CUIPP/prompt-stress rows as
//! prompt cases, list them, record reviewer verdicts, preview a deterministic rewrite against a
//! versioned rule pack, materialize a hashed JSONL export into the ArtifactStore, and list the
//! registered rule packs. Shared helpers come from `super::atelier` (`atelier_store`,
//! `atelier_error`, `internal_error`, `calling_actor`, `ErrorResponse`). Storage authority is
//! the embedded SurrealDB store through `AtelierStore`; no relational fallback exists.
//!
//! Route paths, request/response JSON shapes and error codes mirror the reference
//! `api/atelier.rs` handlers of the same names.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::atelier::prompt_feedback::adapter::{import_leeseo, CuippRow, LeeseoImportRequest};
use crate::atelier::prompt_feedback::engine::{SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION};
use crate::atelier::prompt_feedback::model::{
    NewReviewVerdict, PromptCase, PromptExport, ReviewVerdict, ReviewerKind, RewritePlan, RulePack,
    VerdictKind,
};
use crate::atelier::prompt_feedback::PromptCaseFilter;
use crate::storage::artifacts::resolve_workspace_root;
use crate::AppState;

use super::atelier::{atelier_error, atelier_store, calling_actor, internal_error, ErrorResponse};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/atelier/prompt-feedback/import",
            post(import_prompt_feedback_cases),
        )
        .route(
            "/atelier/prompt-feedback/cases",
            get(list_prompt_feedback_cases),
        )
        .route(
            "/atelier/prompt-feedback/verdicts",
            post(record_prompt_feedback_verdict),
        )
        .route(
            "/atelier/prompt-feedback/rewrite",
            post(preview_prompt_feedback_rewrite),
        )
        .route(
            "/atelier/prompt-feedback/export",
            post(materialize_prompt_feedback_export),
        )
        .route(
            "/atelier/prompt-feedback/rulepacks",
            get(list_prompt_feedback_rulepacks),
        )
        .with_state(state)
}

type ApiError = (StatusCode, Json<ErrorResponse>);

fn bad_request_static() -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "bad_request",
        }),
    )
}

#[derive(Debug, Deserialize)]
struct PromptFeedbackImportRequest {
    project_id: String,
    source_system: String,
    adapter_id: String,
    #[serde(default)]
    source_iteration_id: Option<String>,
    rows: Vec<CuippRow>,
}

#[derive(Debug, Serialize)]
struct PromptFeedbackImportResponse {
    imported_count: usize,
    cases: Vec<PromptCase>,
    seed_rule_pack: RulePack,
}

/// POST /atelier/prompt-feedback/import — normalize CUIPP/prompt-stress rows into
/// PromptCases (adapter) and persist them. Also ensures the seed rule pack.
async fn import_prompt_feedback_cases(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PromptFeedbackImportRequest>,
) -> Result<(StatusCode, Json<PromptFeedbackImportResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let req = LeeseoImportRequest {
        project_id: payload.project_id,
        source_system: payload.source_system,
        adapter_id: payload.adapter_id,
        source_iteration_id: payload.source_iteration_id,
        imported_by: actor.clone(),
        rows: payload.rows,
    };
    let new_cases = import_leeseo(&req).map_err(|err| atelier_error(err.into()))?;
    let cases = store
        .import_prompt_cases(&new_cases)
        .await
        .map_err(atelier_error)?;
    let seed_rule_pack = store
        .ensure_seed_rule_pack(&actor)
        .await
        .map_err(atelier_error)?;
    Ok((
        StatusCode::CREATED,
        Json(PromptFeedbackImportResponse {
            imported_count: cases.len(),
            cases,
            seed_rule_pack,
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct PromptFeedbackCasesQuery {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    segment: Option<String>,
    #[serde(default)]
    cell: Option<String>,
    #[serde(default)]
    render_stack: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

/// GET /atelier/prompt-feedback/cases — list prompt cases, optionally grouped by
/// segment/cell/render-stack filters.
async fn list_prompt_feedback_cases(
    State(state): State<AppState>,
    Query(query): Query<PromptFeedbackCasesQuery>,
) -> Result<Json<Vec<PromptCase>>, ApiError> {
    let store = atelier_store(&state);
    let cases = store
        .list_prompt_cases(&PromptCaseFilter {
            project_id: query.project_id,
            segment: query.segment,
            cell: query.cell,
            render_stack: query.render_stack,
            limit: query.limit,
        })
        .await
        .map_err(atelier_error)?;
    Ok(Json(cases))
}

#[derive(Debug, Deserialize)]
struct PromptFeedbackVerdictRequest {
    case_id: Uuid,
    reviewer_kind: String,
    verdict_kind: String,
    #[serde(default)]
    reviewer_id: Option<String>,
    #[serde(default)]
    failure_class: Option<String>,
    #[serde(default)]
    failure_tags: Option<Vec<String>>,
    #[serde(default)]
    is_identity_judgement: Option<bool>,
    #[serde(default)]
    note: Option<String>,
}

/// POST /atelier/prompt-feedback/verdicts — record a reviewer verdict. Rejects an
/// identity judgement on a prompt-stress case.
async fn record_prompt_feedback_verdict(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PromptFeedbackVerdictRequest>,
) -> Result<(StatusCode, Json<ReviewVerdict>), ApiError> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let reviewer_kind = ReviewerKind::from_token(&payload.reviewer_kind)
        .map_err(|err| atelier_error(err.into()))?;
    let verdict_kind =
        VerdictKind::from_token(&payload.verdict_kind).map_err(|err| atelier_error(err.into()))?;
    let reviewer_id = payload
        .reviewer_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(actor);
    let verdict = store
        .record_prompt_verdict(&NewReviewVerdict {
            case_id: payload.case_id,
            reviewer_kind,
            reviewer_id,
            verdict_kind,
            failure_class: payload.failure_class,
            failure_tags: payload.failure_tags.unwrap_or_default(),
            is_identity_judgement: payload.is_identity_judgement.unwrap_or(false),
            note: payload.note,
        })
        .await
        .map_err(atelier_error)?;
    Ok((StatusCode::CREATED, Json(verdict)))
}

#[derive(Debug, Deserialize)]
struct PromptFeedbackRewriteRequest {
    case_id: Uuid,
    rule_pack_id: String,
    #[serde(default)]
    rule_pack_version: Option<i32>,
}

/// POST /atelier/prompt-feedback/rewrite — deterministic rewrite preview against a
/// rule pack (persisted plan + trace). Rejects a rewrite with no rule-pack id.
async fn preview_prompt_feedback_rewrite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PromptFeedbackRewriteRequest>,
) -> Result<Json<RewritePlan>, ApiError> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    if payload.rule_pack_id.trim().is_empty() {
        return Err(bad_request_static());
    }
    let version = payload.rule_pack_version.unwrap_or(SEED_RULE_PACK_VERSION);
    if payload.rule_pack_id == SEED_RULE_PACK_ID && version == SEED_RULE_PACK_VERSION {
        store
            .ensure_seed_rule_pack(&actor)
            .await
            .map_err(atelier_error)?;
    }
    let plan = store
        .plan_prompt_rewrite(payload.case_id, &payload.rule_pack_id, version, &actor)
        .await
        .map_err(atelier_error)?;
    Ok(Json(plan))
}

#[derive(Debug, Deserialize)]
struct PromptFeedbackExportRequest {
    rule_pack_id: String,
    #[serde(default)]
    rule_pack_version: Option<i32>,
    case_ids: Vec<Uuid>,
}

/// POST /atelier/prompt-feedback/export — materialize corrected prompt rows as a
/// hashed ArtifactStore JSONL artifact + export receipt.
async fn materialize_prompt_feedback_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PromptFeedbackExportRequest>,
) -> Result<(StatusCode, Json<PromptExport>), ApiError> {
    let actor = calling_actor(&headers)?;
    let version = payload.rule_pack_version.unwrap_or(SEED_RULE_PACK_VERSION);
    let store = atelier_store(&state);
    if payload.rule_pack_id.trim().is_empty() {
        return Err(bad_request_static());
    }
    if payload.rule_pack_id == SEED_RULE_PACK_ID && version == SEED_RULE_PACK_VERSION {
        store
            .ensure_seed_rule_pack(&actor)
            .await
            .map_err(atelier_error)?;
    }
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let export = store
        .materialize_prompt_export(
            &payload.rule_pack_id,
            version,
            &payload.case_ids,
            &actor,
            &workspace_root,
        )
        .await
        .map_err(atelier_error)?;
    Ok((StatusCode::CREATED, Json(export)))
}

/// GET /atelier/prompt-feedback/rulepacks — list registered rule packs.
async fn list_prompt_feedback_rulepacks(
    State(state): State<AppState>,
) -> Result<Json<Vec<RulePack>>, ApiError> {
    let store = atelier_store(&state);
    let packs = store.list_rule_packs().await.map_err(atelier_error)?;
    Ok(Json(packs))
}
