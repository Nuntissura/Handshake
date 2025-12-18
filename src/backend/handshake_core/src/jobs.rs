use crate::{flight_recorder::log_event, models::AiJob, AppState};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub async fn create_job(
    state: &AppState,
    job_kind: &str,
    protocol_id: &str,
    // We'll fill in the other params like inputs later
) -> Result<AiJob, JobError> {
    let job_id = Uuid::new_v4().to_string();
    let status = "queued".to_string();

    // These are hardcoded for now as per the task packet.
    let profile_id = "default".to_string();
    let capability_profile_id = "default".to_string();
    let access_mode = "default".to_string();
    let safety_mode = "default".to_string();

    let job = sqlx::query_as!(
        AiJob,
        r#"
        INSERT INTO ai_jobs (id, job_kind, status, protocol_id, profile_id, capability_profile_id, access_mode, safety_mode)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING
            id as "id!",
            job_kind as "job_kind!",
            status as "status!",
            error_message,
            protocol_id as "protocol_id!",
            profile_id as "profile_id!",
            capability_profile_id as "capability_profile_id!",
            access_mode as "access_mode!",
            safety_mode as "safety_mode!",
            job_inputs,
            job_outputs,
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        "#,
        job_id,
        job_kind,
        status,
        protocol_id,
        profile_id,
        capability_profile_id,
        access_mode,
        safety_mode
    )
    .fetch_one(&state.pool)
    .await?;

    // Log the creation event to the flight recorder.
    // We ignore the result for now; a logging failure shouldn't fail the job creation.
    let _ = log_event(
        state,
        "job_created",
        Some(&job.id),
        None,
        json!({ "kind": job.job_kind, "status": job.status }),
    );

    Ok(job)
}
