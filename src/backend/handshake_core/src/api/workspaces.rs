use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::{
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
        .route("/workspaces/:workspace_id", delete(delete_workspace))
        .with_state(state)
}

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_JOB_ID: &str = "x-hsk-job-id";
const HSK_HEADER_WORKFLOW_ID: &str = "x-hsk-workflow-id";

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

            if job_id.is_none() || workflow_id.is_none() {
                return Ok(WriteContext::ai(actor_id, job_id, workflow_id));
            }

            let job_id = job_id.expect("checked above");
            let workflow_id = workflow_id.expect("checked above");

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

async fn create_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;
    let workspace = state
        .storage
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: payload.name.clone(),
            },
        )
        .await
        .map_err(map_storage_error)?;

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
    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;
    state
        .storage
        .delete_workspace(&ctx, &workspace_id)
        .await
        .map_err(map_storage_error)?;

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
    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;

    let document = state
        .storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace_id.clone(),
                title: payload.title.clone(),
            },
        )
        .await
        .map_err(map_storage_error)?;

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
    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;
    state
        .storage
        .delete_document(&ctx, &document_id)
        .await
        .map_err(map_storage_error)?;

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
    let _doc = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;

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

    let result_blocks = state
        .storage
        .replace_blocks(&ctx, &document_id, incoming_blocks)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/documents/:document_id/blocks", status = "ok", document_id = %document_id, blocks = result_blocks.len(), "replace blocks");

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
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::llm::ollama::InMemoryLlmClient;
    use crate::storage::{
        sqlite::SqliteDatabase, AccessMode, Database, EntityRef, JobKind, JobMetrics, JobState,
        JobStatusUpdate, NewAiJob, PlannedOperation, SafetyMode,
    };
    use axum::extract::{Path, State};
    use serde_json::json;
    use sqlx::Row;
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

        let result = replace_blocks(State(state), Path(document.id), headers, Json(payload)).await;

        let Err((status, Json(err))) = result else {
            return Err("expected replace_blocks to be rejected".into());
        };
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(err.error, "HSK-403-SILENT-EDIT");
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

        let sqlite = state
            .storage
            .as_any()
            .downcast_ref::<SqliteDatabase>()
            .ok_or("expected sqlite backend")?;

        let block_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_job_id, last_workflow_id
            FROM blocks
            WHERE document_id = ?1
            LIMIT 1
            "#,
        )
        .bind(&document.id)
        .fetch_one(sqlite.pool())
        .await?;

        let last_actor_kind: String = block_row.get("last_actor_kind");
        let last_job_id: Option<String> = block_row.get("last_job_id");
        let last_workflow_id: Option<String> = block_row.get("last_workflow_id");
        assert_eq!(last_actor_kind, "AI");
        assert_eq!(
            last_job_id.as_deref(),
            Some(job.job_id.to_string().as_str())
        );
        assert_eq!(
            last_workflow_id.as_deref(),
            Some(run.id.to_string().as_str())
        );

        let doc_row = sqlx::query(
            r#"
            SELECT last_actor_kind, last_job_id, last_workflow_id
            FROM documents
            WHERE id = ?1
            "#,
        )
        .bind(&document.id)
        .fetch_one(sqlite.pool())
        .await?;

        let doc_actor_kind: String = doc_row.get("last_actor_kind");
        let doc_job_id: Option<String> = doc_row.get("last_job_id");
        let doc_workflow_id: Option<String> = doc_row.get("last_workflow_id");
        assert_eq!(doc_actor_kind, "AI");
        assert_eq!(doc_job_id.as_deref(), Some(job.job_id.to_string().as_str()));
        assert_eq!(
            doc_workflow_id.as_deref(),
            Some(run.id.to_string().as_str())
        );

        Ok(())
    }
}
