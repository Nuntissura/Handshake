use crate::{
    flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType},
    storage::{
        AccessMode, AiJob, EntityRef, JobKind, JobMetrics, NewAiJob, SafetyMode, StorageError,
    },
    workflows::{self, StartupRecoveryError},
    AppState,
};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

pub use crate::storage::{JobAdmission, JobAdmissionError};

impl JobAdmission {
    pub(crate) fn submit(&mut self, state: std::sync::Arc<AppState>) -> Result<(), JobAdmissionError> {
        if !self.can_submit() {
            return Err(JobAdmissionError::AlreadySubmitted);
        }
        let command = self.command().clone();
        let cancellation = self.cancellation().clone();
        let retention = self.retention().clone();
        let writer = tokio::spawn(async move {
            workflows::wait_for_startup_recovery().await?;
            crate::storage::surreal::SurrealDatabase::execute_job_admission(
                &state.surreal, command, cancellation, retention,
            ).await
        });
        self.install_initial(writer)
    }
}

pub(crate) async fn prepare_job_admission(
    state: std::sync::Arc<AppState>,
    session: crate::api::authority::AuthenticatedLocalSession,
    input: NewAiJob,
) -> Result<JobAdmission, JobAdmissionError> {
    use crate::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    };
    use handshake_job_admission::{
        CheckedJobCreation, CreationDescriptorInput, CreationEntityRef, CreationOperationType,
        CreationPlannedOperation, ProducerJobIdentity,
    };
    let mut workspaces = input.entity_refs.iter().filter(|entity| entity.entity_kind == "workspace");
    let workspace = workspaces.next().ok_or(JobAdmissionError::Denied)?.entity_id.clone();
    if workspaces.next().is_some() {
        return Err(JobAdmissionError::Denied);
    }
    let parse = |value: &str| Uuid::parse_str(value).map_err(|_| JobAdmissionError::Denied);
    let exact = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::from_uuid(parse(&session.context.identity.account_id)?),
        actor_principal_id: ActorPrincipalId::from_uuid(parse(&session.context.identity.principal_id)?),
        authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&session.context.session_id)?),
        access_space_id: AccessSpaceRef::from_uuid(parse(&session.context.identity.access_space_id)?),
        workspace_id: WorkspaceScopeRef::new(workspace.clone()).map_err(|_| JobAdmissionError::Denied)?,
    };
    let identity = ProducerJobIdentity::allocate(matches!(input.job_kind, JobKind::ModelRun));
    let operation_id = identity.initial_trace_id();
    let descriptor = CreationDescriptorInput {
        job_kind: input.job_kind.as_str().to_owned(),
        protocol_id: input.protocol_id,
        profile_id: input.profile_id,
        capability_profile_id: input.capability_profile_id,
        access_mode: input.access_mode.as_str().to_owned(),
        safety_mode: input.safety_mode.as_str().to_owned(),
        entity_refs: input.entity_refs.into_iter().map(|entity| CreationEntityRef {
            entity_id: entity.entity_id, entity_kind: entity.entity_kind,
        }).collect(),
        planned_operations: input.planned_operations.into_iter().map(|operation| CreationPlannedOperation {
            op_type: match operation.op_type {
                crate::storage::OperationType::Read => CreationOperationType::Read,
                crate::storage::OperationType::Write => CreationOperationType::Write,
                crate::storage::OperationType::Plan => CreationOperationType::Plan,
                crate::storage::OperationType::Execute => CreationOperationType::Execute,
            },
            target: CreationEntityRef {
                entity_id: operation.target.entity_id, entity_kind: operation.target.entity_kind,
            },
            description: operation.description,
        }).collect(),
        job_inputs: input.job_inputs,
    };
    let checked = CheckedJobCreation::build(identity, exact, descriptor)?;
    state.surreal.capture_job_admission(
        AuthorizationRequest {
            session_token: session.session_token,
            channel_binding_hash: Some(session.channel_binding_hash),
            capability_id: "fs.write".to_owned(),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace,
            action: ResourceAction::Create,
        },
        &state.capability_registry, operation_id, checked,
    ).await
}

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Startup recovery gate: {0}")]
    StartupRecovery(#[from] StartupRecoveryError),
}

pub async fn create_job(
    state: &AppState,
    job_kind: JobKind,
    protocol_id: &str,
    capability_profile_id: &str,
    job_inputs: Option<Value>,
    entity_refs: Vec<EntityRef>,
) -> Result<AiJob, JobError> {
    create_job_from_input(
        state,
        NewAiJob {
            trace_id: Uuid::now_v7(),
            job_kind: job_kind.clone(),
            protocol_id: protocol_id.to_string(),
            profile_id: "default".to_string(),
            capability_profile_id: capability_profile_id.to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs,
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs,
        },
    )
    .await
}

pub(crate) async fn create_job_from_input(
    state: &AppState,
    input: NewAiJob,
) -> Result<AiJob, JobError> {
    workflows::wait_for_startup_recovery().await?;
    let job = state.storage.create_ai_job(input).await?;

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
