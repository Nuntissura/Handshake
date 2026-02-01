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
    SafetyMode, StorageError,
};
use handshake_core::workflows::start_workflow_for_job;
use handshake_core::AppState;
use serde_json::json;
use sha2::{Digest, Sha256};
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
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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

    let retrieval_event = events
        .iter()
        .find(|e| e.event_type == FlightRecorderEventType::DataRetrievalExecuted)
        .expect("data_retrieval_executed event");
    let query_hash = retrieval_event
        .payload
        .get("query_hash")
        .and_then(|v| v.as_str())
        .expect("query_hash present");
    assert_eq!(query_hash.len(), 64);
    assert!(query_hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(retrieval_event.payload.get("query").is_none());

    let context_event = events
        .iter()
        .find(|e| e.event_type == FlightRecorderEventType::DataContextAssembled)
        .expect("data_context_assembled event");
    let retrieval_request_id = retrieval_event
        .payload
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("retrieval request_id present");
    let context_request_id = context_event
        .payload
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("context request_id present");
    assert_eq!(retrieval_request_id, context_request_id);

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
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskHardGate));

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_generates_distillation_candidate_after_escalation_success(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        r#"<mt_complete>
MT_ID: MT-001
EVIDENCE:
- "done means placeholder" -> src/backend/handshake_core/src/workflows.rs:1-1
</mt_complete>"#
            .to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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
                    "max_iterations_per_mt": 1,
                    "max_total_iterations": 2,
                    "enable_distillation": true,
                }
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
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskEscalated));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::MicroTaskDistillationCandidate));

    let repo_root = handshake_core::capability_registry_workflow::repo_root_from_manifest_dir()?;
    let progress_path = repo_root
        .join("data")
        .join("micro_task_executor")
        .join(job_id.to_string())
        .join("progress_artifact.json");
    let raw_progress = std::fs::read_to_string(&progress_path)?;
    let progress: serde_json::Value = serde_json::from_str(&raw_progress)?;

    let candidate_path = progress
        .get("micro_tasks")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|mt| mt.get("distillation_candidate"))
        .and_then(|d| d.get("candidate_ref"))
        .and_then(|h| h.get("path"))
        .and_then(|v| v.as_str())
        .expect("candidate_ref.path missing");

    let candidate_abs = repo_root.join(candidate_path);
    let raw_candidate = std::fs::read_to_string(&candidate_abs)?;
    let candidate: serde_json::Value = serde_json::from_str(&raw_candidate)?;

    assert!(candidate
        .get("skill_log_entry_id")
        .and_then(|v| v.as_str())
        .is_some());
    assert!(candidate.get("student_attempt").is_some());
    assert!(candidate.get("teacher_success").is_some());

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_emits_model_swap_events_on_model_change(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        "done <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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
                    "max_iterations_per_mt": 1,
                    "max_total_iterations": 2,
                    "enable_distillation": false,
                    "escalation_chain": [
                        { "level": 0, "model_id": "test-model-a", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "test-model-b", "is_cloud": false, "is_hard_gate": false }
                    ]
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Completed));

    let duckdb = state
        .flight_recorder
        .duckdb_connection()
        .expect("duckdb connection");
    let conn = duckdb.lock().expect("duckdb mutex poisoned");

    let mut stmt =
        conn.prepare("SELECT event_type, payload FROM events WHERE job_id = ? ORDER BY timestamp")?;
    let mut rows = stmt.query(duckdb::params![job_id.to_string()])?;

    let mut requested_payload: Option<serde_json::Value> = None;
    let mut completed_payload: Option<serde_json::Value> = None;

    while let Some(row) = rows.next()? {
        let event_type: String = row.get(0)?;
        let payload_str: String = row.get(1)?;
        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;

        if event_type == FlightRecorderEventType::ModelSwapRequested.to_string() {
            requested_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapCompleted.to_string() {
            completed_payload = Some(payload);
        }
    }

    let requested = requested_payload.expect("model_swap_requested event");
    let completed = completed_payload.expect("model_swap_completed event");

    assert_eq!(
        requested.get("type").and_then(|v| v.as_str()),
        Some("model_swap_requested")
    );
    assert_eq!(
        requested.get("current_model_id").and_then(|v| v.as_str()),
        Some("test-model-a")
    );
    assert_eq!(
        requested.get("target_model_id").and_then(|v| v.as_str()),
        Some("test-model-b")
    );

    assert_eq!(
        completed.get("type").and_then(|v| v.as_str()),
        Some("model_swap_completed")
    );
    assert_eq!(
        completed.get("outcome").and_then(|v| v.as_str()),
        Some("success")
    );

    let state_hash = requested
        .get("state_hash")
        .and_then(|v| v.as_str())
        .expect("state_hash present");
    assert_eq!(state_hash.len(), 64);
    assert!(state_hash
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f')));

    let refs: Vec<&str> = requested
        .get("state_persist_refs")
        .and_then(|v| v.as_array())
        .expect("state_persist_refs array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    let request_rel = refs
        .iter()
        .find(|r| r.contains("/model_swap/request_"))
        .expect("persisted request ref");
    let state_rel = refs
        .iter()
        .find(|r| r.contains("/model_swap/swap_state_"))
        .expect("persisted state ref");

    let repo_root = handshake_core::capability_registry_workflow::repo_root_from_manifest_dir()?;
    let request_path = repo_root.join(request_rel);
    let state_path = repo_root.join(state_rel);
    assert!(request_path.exists(), "persisted request must exist");
    assert!(state_path.exists(), "persisted state must exist");

    let raw_request = std::fs::read_to_string(&request_path)?;
    let request_json: serde_json::Value = serde_json::from_str(&raw_request)?;
    assert_eq!(
        request_json.get("state_hash").and_then(|v| v.as_str()),
        Some(state_hash)
    );

    let bytes = std::fs::read(&state_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual_hash = hex::encode(hasher.finalize());
    assert_eq!(actual_hash, state_hash);

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_emits_model_swap_failed_when_policy_disallows_swaps(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        "done <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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
                    "max_iterations_per_mt": 1,
                    "max_total_iterations": 2,
                    "enable_distillation": false,
                    "escalation_chain": [
                        { "level": 0, "model_id": "test-model-a", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "test-model-b", "is_cloud": false, "is_hard_gate": false }
                    ],
                    "extensions": [
                        {
                            "schema_version": "hsk.exec_policy_ext@0.4",
                            "kind": "model_swap_policy",
                            "model_swap_policy": {
                                "allow_swaps": false,
                                "fallback_strategy": "abort"
                            }
                        }
                    ]
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Failed));

    let duckdb = state
        .flight_recorder
        .duckdb_connection()
        .expect("duckdb connection");
    let conn = duckdb.lock().expect("duckdb mutex poisoned");

    let mut stmt =
        conn.prepare("SELECT event_type, payload FROM events WHERE job_id = ? ORDER BY timestamp")?;
    let mut rows = stmt.query(duckdb::params![job_id.to_string()])?;

    let mut requested_payload: Option<serde_json::Value> = None;
    let mut failed_payload: Option<serde_json::Value> = None;
    let mut completed_payload: Option<serde_json::Value> = None;
    let mut rollback_payload: Option<serde_json::Value> = None;

    while let Some(row) = rows.next()? {
        let event_type: String = row.get(0)?;
        let payload_str: String = row.get(1)?;
        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;

        if event_type == FlightRecorderEventType::ModelSwapRequested.to_string() {
            requested_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapFailed.to_string() {
            failed_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapCompleted.to_string() {
            completed_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapRollback.to_string() {
            rollback_payload = Some(payload);
        }
    }

    let requested = requested_payload.expect("model_swap_requested event");
    let failed = failed_payload.expect("model_swap_failed event");

    assert_eq!(
        requested.get("type").and_then(|v| v.as_str()),
        Some("model_swap_requested")
    );

    assert_eq!(
        failed.get("type").and_then(|v| v.as_str()),
        Some("model_swap_failed")
    );
    assert_eq!(
        failed.get("outcome").and_then(|v| v.as_str()),
        Some("failure")
    );
    assert_eq!(
        failed.get("error_summary").and_then(|v| v.as_str()),
        Some("swap_disallowed_by_policy")
    );

    assert!(
        completed_payload.is_none(),
        "swap should not complete on policy failure"
    );
    assert!(
        rollback_payload.is_none(),
        "default fallback should not rollback"
    );

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_emits_model_swap_timeout_and_rollback_when_timeout_ms_zero(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        "done <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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
                    "max_iterations_per_mt": 1,
                    "max_total_iterations": 2,
                    "enable_distillation": false,
                    "escalation_chain": [
                        { "level": 0, "model_id": "test-model-a", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "test-model-b", "is_cloud": false, "is_hard_gate": false }
                    ],
                    "extensions": [
                        {
                            "schema_version": "hsk.exec_policy_ext@0.4",
                            "kind": "model_swap_policy",
                            "model_swap_policy": {
                                "swap_timeout_ms": 0,
                                "fallback_strategy": "continue_with_current"
                            }
                        }
                    ]
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Completed));

    let duckdb = state
        .flight_recorder
        .duckdb_connection()
        .expect("duckdb connection");
    let conn = duckdb.lock().expect("duckdb mutex poisoned");

    let mut stmt =
        conn.prepare("SELECT event_type, payload FROM events WHERE job_id = ? ORDER BY timestamp")?;
    let mut rows = stmt.query(duckdb::params![job_id.to_string()])?;

    let mut requested_payload: Option<serde_json::Value> = None;
    let mut timeout_payload: Option<serde_json::Value> = None;
    let mut rollback_payload: Option<serde_json::Value> = None;
    let mut completed_payload: Option<serde_json::Value> = None;

    while let Some(row) = rows.next()? {
        let event_type: String = row.get(0)?;
        let payload_str: String = row.get(1)?;
        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;

        if event_type == FlightRecorderEventType::ModelSwapRequested.to_string() {
            requested_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapTimeout.to_string() {
            timeout_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapRollback.to_string() {
            rollback_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapCompleted.to_string() {
            completed_payload = Some(payload);
        }
    }

    let requested = requested_payload.expect("model_swap_requested event");
    let timeout = timeout_payload.expect("model_swap_timeout event");
    let rollback = rollback_payload.expect("model_swap_rollback event");

    assert_eq!(
        requested.get("type").and_then(|v| v.as_str()),
        Some("model_swap_requested")
    );

    assert_eq!(
        timeout.get("type").and_then(|v| v.as_str()),
        Some("model_swap_timeout")
    );
    assert_eq!(
        timeout.get("outcome").and_then(|v| v.as_str()),
        Some("timeout")
    );
    assert_eq!(
        timeout.get("error_summary").and_then(|v| v.as_str()),
        Some("swap_timeout")
    );

    assert_eq!(
        rollback.get("type").and_then(|v| v.as_str()),
        Some("model_swap_rollback")
    );
    assert_eq!(
        rollback.get("outcome").and_then(|v| v.as_str()),
        Some("rollback")
    );
    assert_eq!(
        rollback.get("error_summary").and_then(|v| v.as_str()),
        Some("swap_timeout")
    );

    assert!(
        completed_payload.is_none(),
        "swap should not complete on timeout"
    );

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
            job_kind: JobKind::MicroTaskExecution,
            protocol_id: "micro_task_executor_v1".to_string(),
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

#[tokio::test]
async fn micro_task_executor_rejects_legacy_workflow_run_job_kind_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let result = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::WorkflowRun,
            protocol_id: "micro_task_executor_v1".to_string(),
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
        .await;

    assert!(matches!(result, Err(StorageError::Validation(_))));

    Ok(())
}
