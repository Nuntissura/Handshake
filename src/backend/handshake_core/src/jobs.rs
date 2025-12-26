use crate::{
    flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType},
    storage::{AccessMode, AiJob, JobKind, JobMetrics, NewAiJob, SafetyMode, StorageError},
    AppState,
};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

pub async fn create_job(
    state: &AppState,
    job_kind: JobKind,
    protocol_id: &str,
    capability_profile_id: &str,
    job_inputs: Option<Value>,
) -> Result<AiJob, JobError> {
    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: job_kind.clone(),
            protocol_id: protocol_id.to_string(),
            profile_id: "default".to_string(),
            capability_profile_id: capability_profile_id.to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs,
        })
        .await?;

    // Log the creation event to the flight recorder.
    // We ignore the result for now; a logging failure shouldn't fail the job creation.
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::Agent,
        job.trace_id,
        json!({ "kind": job.job_kind.as_str(), "status": job.state.as_str() }),
    )
    .with_job_id(job.job_id.to_string());
    let _ = state.flight_recorder.record_event(event).await;

    Ok(job)
}
