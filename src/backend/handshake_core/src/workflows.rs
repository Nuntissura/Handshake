use crate::{
    flight_recorder::log_event,
    models::{AiJob, WorkflowRun},
    AppState,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub async fn start_workflow_for_job(
    state: &AppState,
    job: AiJob,
) -> Result<WorkflowRun, WorkflowError> {
    let workflow_run_id = Uuid::new_v4().to_string();
    let initial_status = "running".to_string();

    // Create the workflow run record
    let workflow_run = sqlx::query_as!(
        WorkflowRun,
        r#"
        INSERT INTO workflow_runs (id, job_id, status)
        VALUES ($1, $2, $3)
        RETURNING
            id as "id!",
            job_id as "job_id!",
            status as "status!",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        "#,
        workflow_run_id,
        job.id,
        initial_status
    )
    .fetch_one(&state.pool)
    .await?;

    // Log the start event
    let _ = log_event(
        state,
        "workflow_started",
        Some(&job.id),
        Some(&workflow_run.id),
        json!({ "status": workflow_run.status }),
    );

    // In a real scenario, we would now do work.
    // For this task, we will just immediately complete the job and workflow.

    let final_job_status = "completed".to_string();
    sqlx::query!(
        "UPDATE ai_jobs SET status = $1 WHERE id = $2",
        final_job_status,
        job.id
    )
    .execute(&state.pool)
    .await?;

    let final_workflow_status = "completed".to_string();
    let completed_run = sqlx::query_as!(
        WorkflowRun,
        r#"
        UPDATE workflow_runs SET status = $1 WHERE id = $2
        RETURNING
            id as "id!",
            job_id as "job_id!",
            status as "status!",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        "#,
        final_workflow_status,
        workflow_run.id
    )
    .fetch_one(&state.pool)
    .await?;

    // Log the completion event
    let _ = log_event(
        state,
        "workflow_completed",
        Some(&job.id),
        Some(&completed_run.id),
        json!({ "status": completed_run.status }),
    );

    Ok(completed_run)
}
