use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{EventFilter, FlightRecorderEventType};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::{
    sqlite::SqliteDatabase, AccessMode, Database, JobKind, JobMetrics, JobState, NewAiJob,
    SafetyMode,
};
use handshake_core::workflows::start_workflow_for_job;
use handshake_core::AppState;
use serde_json::json;
use uuid::Uuid;

struct QueuedLlmClient {
    profile: ModelProfile,
    responses: Mutex<VecDeque<String>>,
}

impl QueuedLlmClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            profile: ModelProfile::new("queued-test-model".to_string(), 4096),
            responses: Mutex::new(responses.into_iter().collect()),
        }
    }

    fn next_response(&self) -> String {
        let mut guard = self.responses.lock().expect("queued llm mutex poisoned");
        guard
            .pop_front()
            .unwrap_or_else(|| "<mt_complete>yes</mt_complete>".to_string())
    }
}

#[async_trait]
impl LlmClient for QueuedLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let text = self.next_response();
        Ok(CompletionResponse {
            text,
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

async fn setup_state(
    llm_client: Arc<dyn LlmClient>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
    sqlite.run_migrations().await?;

    let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);

    Ok(AppState {
        storage: sqlite.into_arc(),
        flight_recorder: flight_recorder.clone(),
        diagnostics: flight_recorder,
        llm_client,
        capability_registry: Arc::new(CapabilityRegistry::new()),
    })
}

fn default_wp_scope(test_plan: Vec<String>) -> serde_json::Value {
    json!({
        "in_scope_paths": ["src/backend/handshake_core/src/workflows.rs"],
        "out_of_scope": [],
        "done_means": ["DONE_MEANS placeholder"],
        "test_plan": test_plan,
        "description": "test scope",
    })
}

#[tokio::test]
async fn micro_task_executor_completes_single_mt_and_emits_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "work complete <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "protocol-default".to_string(),
            profile_id: "micro_task_executor_v1".to_string(),
            capability_profile_id: "Coder".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({
                "wp_id": "WP-TEST",
                "wp_scope": default_wp_scope(vec!["exit 0".to_string()]),
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Completed));

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job_id.to_string()),
            ..Default::default()
        })
        .await?;

    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskLoopStarted));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskIterationStarted));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskValidation));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskIterationComplete));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskComplete));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskLoopCompleted));

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_escalates_and_hard_gates_after_budget_exhaustion(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "attempted complete <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "protocol-default".to_string(),
            profile_id: "micro_task_executor_v1".to_string(),
            capability_profile_id: "Coder".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({
                "wp_id": "WP-TEST",
                "wp_scope": default_wp_scope(vec!["exit 1".to_string()]),
                "execution_policy": {
                    "max_iterations_per_mt": 1,
                    "max_total_iterations": 1,
                    "enable_distillation": true,
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::AwaitingUser));

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job_id.to_string()),
            ..Default::default()
        })
        .await?;

    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskEscalated));
    assert!(events
        .iter()
        .any(|e| { e.event_type == FlightRecorderEventType::MicroTaskDistillationCandidate }));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskHardGate));

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_resumes_from_pause_and_emits_workflow_recovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "resume and complete <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "protocol-default".to_string(),
            profile_id: "micro_task_executor_v1".to_string(),
            capability_profile_id: "Coder".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({
                "wp_id": "WP-TEST",
                "wp_scope": default_wp_scope(vec!["exit 0".to_string()]),
                "execution_policy": {
                    "pause_points": ["MT-001"],
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let paused_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(paused_job.state, JobState::AwaitingUser));

    let repo_root = handshake_core::capability_registry_workflow::repo_root_from_manifest_dir()?;
    let run_ledger_path = repo_root
        .join("data")
        .join("micro_task_executor")
        .join(job_id.to_string())
        .join("run_ledger.json");

    let raw = std::fs::read_to_string(&run_ledger_path)?;
    let mut ledger: serde_json::Value = serde_json::from_str(&raw)?;
    let steps = ledger
        .get_mut("steps")
        .and_then(|v| v.as_array_mut())
        .ok_or("run_ledger.steps missing")?;
    steps.push(json!({
        "step_id": "injected_step",
        "idempotency_key": "injected_key",
        "status": "in_progress",
        "recoverable": true,
    }));
    std::fs::write(&run_ledger_path, serde_json::to_vec_pretty(&ledger)?)?;

    let resume_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    start_workflow_for_job(&state, resume_job).await?;

    let completed_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(completed_job.state, JobState::Completed));

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job_id.to_string()),
            ..Default::default()
        })
        .await?;

    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskResumed));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::WorkflowRecovery));

    Ok(())
}
