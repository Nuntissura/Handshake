use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use handshake_core::ace::ArtifactHandle;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{EventFilter, FlightRecorderEventType};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::runtime_governance::RuntimeGovernancePaths;
use handshake_core::storage::{
    sqlite::SqliteDatabase, AccessMode, AiJobListFilter, Database, JobKind, JobMetrics, JobState,
    NewAiJob, SafetyMode, StorageError,
};
use handshake_core::workflows::{
    locus::{
        validate_structured_collaboration_record, validate_structured_collaboration_summary_join,
        StructuredCollaborationRecordFamily, StructuredCollaborationValidationCode,
        StructuredCollaborationValidationResult,
    },
    start_workflow_for_job, ModelSwapRequestV0_4, SessionRegistry, SessionSchedulerConfig,
};
use handshake_core::AppState;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use uuid::Uuid;

struct QueuedLlmClient {
    profile: ModelProfile,
    responses: Mutex<VecDeque<String>>,
    swap_calls: Mutex<Vec<ModelSwapRequestV0_4>>,
    swap_delay_ms: u64,
    swap_error: Option<String>,
}

impl QueuedLlmClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            profile: ModelProfile::new("queued-test-model".to_string(), 4096),
            responses: Mutex::new(responses.into_iter().collect()),
            swap_calls: Mutex::new(Vec::new()),
            swap_delay_ms: 0,
            swap_error: None,
        }
    }

    fn with_swap_delay_ms(mut self, swap_delay_ms: u64) -> Self {
        self.swap_delay_ms = swap_delay_ms;
        self
    }

    fn with_swap_error(mut self, swap_error: impl Into<String>) -> Self {
        self.swap_error = Some(swap_error.into());
        self
    }

    fn next_response(&self) -> String {
        let mut guard = self.responses.lock().expect("queued llm mutex poisoned");
        guard
            .pop_front()
            .unwrap_or_else(|| "<mt_complete>yes</mt_complete>".to_string())
    }
}

static TEST_SERIAL_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct WorkspaceEnvGuard {
    prev_workspace_root: Option<String>,
    prev_governance_root: Option<String>,
}

impl WorkspaceEnvGuard {
    fn activate(root: &Path) -> Self {
        let prev_workspace_root = std::env::var("HANDSHAKE_WORKSPACE_ROOT").ok();
        let prev_governance_root = std::env::var("HANDSHAKE_GOVERNANCE_ROOT").ok();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", root);
        std::env::set_var("HANDSHAKE_GOVERNANCE_ROOT", ".handshake/gov");
        Self {
            prev_workspace_root,
            prev_governance_root,
        }
    }
}

impl Drop for WorkspaceEnvGuard {
    fn drop(&mut self) {
        match &self.prev_workspace_root {
            Some(value) => std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_WORKSPACE_ROOT"),
        }
        match &self.prev_governance_root {
            Some(value) => std::env::set_var("HANDSHAKE_GOVERNANCE_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_GOVERNANCE_ROOT"),
        }
    }
}

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_SERIAL_LOCK.lock().expect("test serial mutex poisoned")
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

    async fn swap_model(&self, req: ModelSwapRequestV0_4) -> Result<(), LlmError> {
        {
            let mut guard = self.swap_calls.lock().expect("swap_calls mutex poisoned");
            guard.push(req);
        }

        if self.swap_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.swap_delay_ms)).await;
        }

        if let Some(err) = &self.swap_error {
            return Err(LlmError::ProviderError(err.clone()));
        }

        Ok(())
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

    let state = AppState {
        storage: sqlite.into_arc(),
        flight_recorder: flight_recorder.clone(),
        diagnostics: flight_recorder,
        llm_client,
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    };
    seed_locus_work_packet(&state, "WP-TEST").await?;
    Ok(state)
}

async fn run_locus_job(
    state: &AppState,
    protocol_id: &str,
    inputs: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::LocusOperation,
            protocol_id: protocol_id.to_string(),
            profile_id: "default".to_string(),
            capability_profile_id: "Coder".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(inputs),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(
        matches!(
            updated_job.state,
            JobState::Completed | JobState::CompletedWithIssues
        ),
        "expected locus job to complete, got {} ({:?})",
        updated_job.state.as_str(),
        updated_job.error_message
    );

    updated_job
        .job_outputs
        .ok_or_else(|| "missing locus job outputs".into())
}

async fn run_locus_job_expect_validation_failure(
    state: &AppState,
    protocol_id: &str,
    inputs: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::LocusOperation,
            protocol_id: protocol_id.to_string(),
            profile_id: "default".to_string(),
            capability_profile_id: "Coder".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(inputs),
        })
        .await?;
    let job_id = job.job_id;

    let _ = start_workflow_for_job(state, job).await;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(
        matches!(updated_job.state, JobState::Failed),
        "expected locus job to fail validation, got {} ({:?})",
        updated_job.state.as_str(),
        updated_job.error_message
    );
    let error_message = updated_job
        .error_message
        .as_deref()
        .ok_or("missing locus validation payload")?;
    let validation_payload = error_message
        .strip_prefix("Terminal error: ")
        .unwrap_or(error_message);
    serde_json::from_str(validation_payload).map_err(|e| {
        format!("failed to parse locus validation payload {error_message:?}: {e}").into()
    })
}

fn validate_runtime_structured_record(
    root: &Path,
    family: StructuredCollaborationRecordFamily,
    value: &Value,
) -> StructuredCollaborationValidationResult {
    let runtime_paths = RuntimeGovernancePaths::from_workspace_root(root.to_path_buf()).unwrap();
    let mut validation = validate_structured_collaboration_record(family, value);
    let authority_refs = value
        .get("authority_refs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let invalid_refs = runtime_paths.invalid_runtime_authority_refs(&authority_refs);
    if !invalid_refs.is_empty() {
        validation.push_issue(
            StructuredCollaborationValidationCode::AuthorityScopeMismatch,
            "authority_refs",
            Some(runtime_paths.governance_root_display()),
            Some(invalid_refs.join(",")),
            "authority_refs must stay within the product-runtime .handshake/gov boundary",
        );
    }
    validation
}

async fn seed_locus_work_packet(
    state: &AppState,
    wp_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = run_locus_job(
        state,
        "locus_create_wp_v1",
        json!({
            "wp_id": wp_id,
            "title": format!("Seeded {wp_id}"),
            "description": "seeded work packet for tests",
            "priority": 2,
            "type": "feature",
            "phase": "1",
            "routing": "GOV_STANDARD",
            "task_packet_path": format!(".handshake/gov/task_packets/{wp_id}.md"),
            "reporter": "micro_task_executor_tests",
        }),
    )
    .await?;
    Ok(())
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

fn base_tracked_micro_task_value(mt_id: &str) -> Value {
    json!({
        "mt_id": mt_id,
        "wp_id": "WP-TEST",
        "name": format!("Micro Task {mt_id}"),
        "scope": "registry validation test",
        "files": {
            "read": [],
            "modify": [],
            "create": [],
        },
        "done_criteria": [],
        "status": "pending",
        "active_session_ids": [],
        "iterations": [],
        "current_iteration": 0,
        "max_iterations": 1,
        "validation_result": Value::Null,
        "escalation": {
            "current_level": 0,
            "escalation_chain": [],
            "escalations_count": 0,
            "drop_backs_count": 0,
        },
        "started_at": Value::Null,
        "completed_at": Value::Null,
        "duration_ms": Value::Null,
        "depends_on": [],
        "metadata": {
            "source": "micro_task_executor_tests",
        },
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn model_swap_ref_to_abs_path(repo_root: &Path, state_ref: &str) -> PathBuf {
    let state_ref = state_ref.trim();
    let rel_path = if let Some(rest) = state_ref.strip_prefix("artifact:") {
        let mut parts = rest.splitn(2, ':');
        let _artifact_id = parts.next();
        parts.next().unwrap_or("")
    } else {
        state_ref
    };
    let rel_path = rel_path.strip_prefix('/').unwrap_or(rel_path);
    repo_root.join(rel_path)
}

fn compute_model_swap_state_hash(
    repo_root: &Path,
    refs: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut ref_hashes: Vec<(String, String)> = Vec::with_capacity(refs.len());
    for state_ref in refs {
        let abs_path = model_swap_ref_to_abs_path(repo_root, state_ref);
        let bytes = std::fs::read(&abs_path)?;
        ref_hashes.push((state_ref.clone(), sha256_hex(&bytes)));
    }

    ref_hashes.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut manifest = String::new();
    for (state_ref, file_hash) in ref_hashes {
        manifest.push_str(&state_ref);
        manifest.push('\n');
        manifest.push_str(&file_hash);
        manifest.push('\n');
    }

    Ok(sha256_hex(manifest.as_bytes()))
}

#[tokio::test]
async fn micro_task_executor_completes_single_mt_and_emits_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
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
async fn micro_task_executor_persists_locus_lifecycle_and_session_occupancy(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![
        "work complete <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let state = setup_state(llm_client).await?;
    let trace_id = Uuid::new_v4();

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id,
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
                "session_id": "sess-occupancy",
                "wp_scope": default_wp_scope(vec!["exit 0".to_string()]),
                "execution_policy": {
                    "automation_level": "FULL_HUMAN",
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let paused_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(paused_job.state, JobState::AwaitingUser));

    let mt_progress = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-001" }),
    )
    .await?;
    assert_eq!(
        mt_progress.get("status").and_then(Value::as_str),
        Some("in_progress")
    );
    assert_eq!(
        mt_progress.get("current_iteration").and_then(Value::as_i64),
        Some(1)
    );
    let metadata = mt_progress.get("metadata").expect("metadata");
    let active_session_ids = metadata
        .get("active_session_ids")
        .and_then(Value::as_array)
        .expect("active_session_ids");
    assert!(
        active_session_ids.is_empty(),
        "paused MT should unbind session occupancy"
    );
    let iterations = metadata
        .get("iterations")
        .and_then(Value::as_array)
        .expect("iterations");
    assert_eq!(iterations.len(), 1);

    let events = state
        .flight_recorder
        .list_events(EventFilter::default())
        .await?;
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::LocusMicroTasksRegistered));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::LocusMtStarted));
    assert!(events
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::LocusMtIterationCompleted));

    let resume_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    start_workflow_for_job(&state, resume_job).await?;

    let completed_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(completed_job.state, JobState::Completed));

    let mt_progress_after = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-001" }),
    )
    .await?;
    assert_eq!(
        mt_progress_after.get("status").and_then(Value::as_str),
        Some("completed")
    );
    let active_session_ids_after = mt_progress_after
        .get("metadata")
        .and_then(|metadata| metadata.get("active_session_ids"))
        .and_then(Value::as_array)
        .expect("active_session_ids after completion");
    assert!(active_session_ids_after.is_empty());

    let events_after = state
        .flight_recorder
        .list_events(EventFilter::default())
        .await?;
    assert!(events_after
        .iter()
        .any(|e| e.event_type == FlightRecorderEventType::LocusMtCompleted));

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_spec_router_creates_locus_work_packet_when_routing_metadata_present(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm_client: Arc<dyn LlmClient> =
        Arc::new(QueuedLlmClient::new(vec!["# Spec Artifact".to_string()]));
    let state = setup_state(llm_client).await?;
    let trace_id = Uuid::new_v4();
    let repo_root = handshake_core::capability_registry_workflow::repo_root_from_manifest_dir()?;
    let prompt_rel = PathBuf::from("data")
        .join("spec_router_tests")
        .join(format!("{}.md", Uuid::new_v4()));
    let prompt_abs = repo_root.join(&prompt_rel);
    std::fs::create_dir_all(prompt_abs.parent().ok_or("prompt parent missing")?)?;
    std::fs::write(&prompt_abs, "route this prompt into a work packet")?;

    let prompt_ref = ArtifactHandle::new(
        Uuid::new_v4(),
        prompt_rel.to_string_lossy().replace('\\', "/"),
    );
    let routed_wp_id = format!("WP-ROUTED-{}", Uuid::new_v4());

    let job = state
        .storage
        .create_ai_job(NewAiJob {
            trace_id,
            job_kind: JobKind::SpecRouter,
            protocol_id: "protocol-default".to_string(),
            profile_id: "default".to_string(),
            capability_profile_id: "Analyst".to_string(),
            access_mode: AccessMode::AnalysisOnly,
            safety_mode: SafetyMode::Normal,
            entity_refs: Vec::new(),
            planned_operations: Vec::new(),
            status_reason: "queued".to_string(),
            metrics: JobMetrics::zero(),
            job_inputs: Some(json!({
                "prompt_ref": prompt_ref,
                "spec_intent_id": format!("spec-{}", Uuid::new_v4()),
                "mode_override": "gov_standard",
                "spec_prompt_pack_id": "spec_router_pack@1",
                "workspace_id": Uuid::new_v4(),
                "project_id": Value::Null,
                "workflow_context": {
                    "version_control": "Git",
                    "repo_root": repo_root.to_string_lossy().to_string(),
                },
                "wp_id": routed_wp_id,
                "title": "Routed Packet",
                "description": "Spec Router should submit locus_create_wp_v1",
                "task_packet_path": ".handshake/gov/task_packets/WP-ROUTED.md",
                "spec_session_id": "spec-session-1",
                "phase": "1",
                "routing": "GOV_STANDARD",
                "type": "feature",
                "priority": 2,
                "reporter": "spec_router_test",
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Completed));
    assert!(
        updated_job
            .job_outputs
            .as_ref()
            .and_then(|value| value.get("spec_artifact_ref"))
            .is_some(),
        "spec_router output should still include spec_artifact_ref"
    );

    let wp_status = run_locus_job(
        &state,
        "locus_get_wp_status_v1",
        json!({ "wp_id": routed_wp_id }),
    )
    .await?;
    assert_eq!(
        wp_status.get("status").and_then(Value::as_str),
        Some("stub")
    );
    assert_eq!(
        wp_status.get("task_board_status").and_then(Value::as_str),
        Some("STUB")
    );

    let child_jobs = state
        .storage
        .list_ai_jobs(AiJobListFilter::default())
        .await?;
    assert_eq!(
        child_jobs
            .iter()
            .filter(|child_job| {
                matches!(child_job.job_kind, JobKind::LocusOperation)
                    && child_job.protocol_id == "locus_create_wp_v1"
                    && child_job
                        .job_inputs
                        .as_ref()
                        .and_then(|inputs| inputs.get("wp_id"))
                        .and_then(Value::as_str)
                        == Some(routed_wp_id.as_str())
            })
            .count(),
        1,
        "spec router should dispatch exactly one locus_create_wp_v1 child job for the routed WP"
    );

    Ok(())
}

#[tokio::test]
async fn locus_create_and_close_wp_emit_structured_work_packet_packet_and_summary(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let summary_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("summary.json");

    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    let summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    assert_eq!(
        packet_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.tracked_work_packet@1")
    );
    assert_eq!(
        packet_json.get("schema_version").and_then(Value::as_str),
        Some("1")
    );
    assert_eq!(
        packet_json
            .get("summary_record_path")
            .and_then(Value::as_str),
        Some(".handshake/gov/work_packets/WP-TEST/summary.json")
    );
    assert_eq!(
        packet_json
            .get("authority_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/work_packets/WP-TEST/packet.json")
    );
    assert_eq!(
        summary_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.structured_collaboration_summary@1")
    );
    assert_eq!(
        summary_json.get("status").and_then(Value::as_str),
        Some("stub")
    );

    let packet_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json,
    );
    assert!(packet_validation.ok, "{packet_validation:?}");
    let summary_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketSummary,
        &summary_json,
    );
    assert!(summary_validation.ok, "{summary_validation:?}");
    let join_validation = validate_structured_collaboration_summary_join(
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json,
        StructuredCollaborationRecordFamily::WorkPacketSummary,
        &summary_json,
    );
    assert!(join_validation.ok, "{join_validation:?}");

    run_locus_job(&state, "locus_close_wp_v1", json!({ "wp_id": "WP-TEST" })).await?;

    let closed_packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    let closed_summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    assert_eq!(
        closed_packet_json.get("status").and_then(Value::as_str),
        Some("done")
    );
    assert_eq!(
        closed_packet_json
            .get("governance")
            .and_then(|value| value.get("task_board_status"))
            .and_then(Value::as_str),
        Some("DONE")
    );
    assert_eq!(
        closed_summary_json.get("status").and_then(Value::as_str),
        Some("done")
    );

    Ok(())
}

#[tokio::test]
async fn locus_work_packet_packet_preserves_profile_extension(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let profile_extension = json!({
        "extension_schema_id": "hsk.profile_extension@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
        },
        "project_scope": {
            "repo": "handshake",
        },
    });

    run_locus_job(
        &state,
        "locus_update_wp_v1",
        json!({
            "wp_id": "WP-TEST",
            "updates": {
                "profile_extension": profile_extension.clone(),
            },
        }),
    )
    .await?;

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;

    assert_eq!(
        packet_json.get("profile_extension"),
        Some(&profile_extension)
    );
    let packet_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json,
    );
    assert!(packet_validation.ok, "{packet_validation:?}");

    Ok(())
}

#[tokio::test]
async fn locus_create_wp_returns_machine_readable_validation_for_incompatible_profile_extension_without_persisting_work_packet(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let wp_id = "WP-BREAKING-EXT-CREATE";
    let validation = run_locus_job_expect_validation_failure(
        &state,
        "locus_create_wp_v1",
        json!({
            "wp_id": wp_id,
            "title": "Breaking extension create",
            "description": "should fail before persistence",
            "priority": 2,
            "type": "feature",
            "phase": "1",
            "routing": "GOV_STANDARD",
            "task_packet_path": format!(".handshake/gov/task_packets/{wp_id}.md"),
            "profile_extension": {
                "extension_schema_id": "hsk.profile_extension@1",
                "extension_schema_version": "1",
                "compatibility": {
                    "breaking": true,
                },
            },
            "reporter": "micro_task_executor_tests",
        }),
    )
    .await?;

    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        validation.get("family").and_then(Value::as_str),
        Some("work_packet_packet")
    );
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("incompatible_profile_extension")
            && issue.get("field").and_then(Value::as_str) == Some("profile_extension.compatibility")
    }));

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join(wp_id)
        .join("packet.json");
    assert!(
        !packet_path.exists(),
        "failed invalid create must not write a work-packet packet artifact"
    );

    run_locus_job(
        &state,
        "locus_create_wp_v1",
        json!({
            "wp_id": wp_id,
            "title": "Recovered create",
            "description": "valid retry should succeed",
            "priority": 2,
            "type": "feature",
            "phase": "1",
            "routing": "GOV_STANDARD",
            "task_packet_path": format!(".handshake/gov/task_packets/{wp_id}.md"),
            "reporter": "micro_task_executor_tests",
        }),
    )
    .await?;

    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    assert_eq!(
        packet_json.get("wp_id").and_then(Value::as_str),
        Some(wp_id)
    );
    assert!(
        packet_json.get("profile_extension").is_none()
            || packet_json.get("profile_extension") == Some(&Value::Null),
        "valid retry should not inherit the rejected profile_extension from the failed create"
    );

    Ok(())
}

#[tokio::test]
async fn locus_update_wp_returns_machine_readable_validation_for_incompatible_profile_extension(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let validation = run_locus_job_expect_validation_failure(
        &state,
        "locus_update_wp_v1",
        json!({
            "wp_id": "WP-TEST",
            "updates": {
                "profile_extension": {
                    "extension_schema_id": "hsk.profile_extension@1",
                    "extension_schema_version": "1",
                    "compatibility": {
                        "breaking": true,
                    },
                },
            },
        }),
    )
    .await?;

    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        validation.get("family").and_then(Value::as_str),
        Some("work_packet_packet")
    );
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("incompatible_profile_extension")
            && issue.get("field").and_then(Value::as_str) == Some("profile_extension.compatibility")
    }));

    run_locus_job(&state, "locus_close_wp_v1", json!({ "wp_id": "WP-TEST" })).await?;

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    assert!(
        packet_json.get("profile_extension").is_none()
            || packet_json.get("profile_extension") == Some(&Value::Null),
        "failed invalid update must not persist profile_extension into the work-packet packet"
    );
    assert_eq!(
        packet_json.get("status").and_then(Value::as_str),
        Some("done")
    );

    Ok(())
}

#[tokio::test]
async fn locus_work_packet_validation_reports_unknown_schema_version(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let _state = setup_state(llm_client).await?;

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let mut packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    packet_json["schema_version"] = Value::String("999".to_string());

    let validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json,
    );
    let validation_json = serde_json::to_value(&validation)?;

    assert_eq!(
        validation_json.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        validation_json.get("family").and_then(Value::as_str),
        Some("work_packet_packet")
    );
    let issues = validation_json
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("schema_version_mismatch")
            && issue.get("field").and_then(Value::as_str) == Some("schema_version")
    }));

    Ok(())
}

#[tokio::test]
async fn locus_sync_task_board_emits_structured_index_and_view(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let gov_root = root.join(".handshake").join("gov");
    std::fs::create_dir_all(&gov_root)?;
    std::fs::write(
        gov_root.join("TASK_BOARD.md"),
        concat!(
            "# Task Board\n\n",
            "## Ready for Dev\n",
            "- **[WP-TEST]** - [ready]\n"
        ),
    )?;

    let sync_result = run_locus_job(&state, "locus_sync_task_board_v1", json!({})).await?;
    assert_eq!(
        sync_result.get("applied_updates").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        sync_result
            .get("unknown_wp_ids")
            .and_then(Value::as_array)
            .map(|items| items.len()),
        Some(0)
    );

    let index_path = gov_root.join("task_board").join("index.json");
    let view_path = gov_root
        .join("task_board")
        .join("views")
        .join("default.json");
    let index_json: Value = serde_json::from_slice(&std::fs::read(&index_path)?)?;
    let view_json: Value = serde_json::from_slice(&std::fs::read(&view_path)?)?;
    assert_eq!(
        index_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.task_board_index@1")
    );
    assert_eq!(
        view_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.task_board_view@1")
    );
    assert_eq!(
        view_json.get("view_id").and_then(Value::as_str),
        Some("default")
    );
    assert!(index_json.get("entries").is_none());
    assert!(view_json.get("lanes").is_none());
    let index_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::TaskBoardIndex,
        &index_json,
    );
    assert!(index_validation.ok, "{index_validation:?}");
    let view_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::TaskBoardView,
        &view_json,
    );
    assert!(view_validation.ok, "{view_validation:?}");

    let first_row = index_json
        .get("rows")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("missing task-board row")?;
    assert_eq!(
        first_row.get("work_packet_id").and_then(Value::as_str),
        Some("WP-TEST")
    );
    assert_eq!(
        first_row.get("lane_id").and_then(Value::as_str),
        Some("ready")
    );
    let lane_ids = view_json
        .get("lane_ids")
        .and_then(Value::as_array)
        .ok_or("missing lane_ids")?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(lane_ids.contains(&"ready"));
    let first_view_row = view_json
        .get("rows")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("missing task-board view row")?;
    assert_eq!(
        first_view_row.get("work_packet_id").and_then(Value::as_str),
        Some("WP-TEST")
    );

    let packet_path = gov_root
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    assert_eq!(
        packet_json.get("status").and_then(Value::as_str),
        Some("ready")
    );

    Ok(())
}

#[tokio::test]
async fn locus_sync_task_board_validation_reports_authority_scope_drift(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let gov_root = root.join(".handshake").join("gov");
    std::fs::create_dir_all(&gov_root)?;
    std::fs::write(
        gov_root.join("TASK_BOARD.md"),
        concat!(
            "# Task Board\n\n",
            "## Ready for Dev\n",
            "- **[WP-TEST]** - [ready]\n"
        ),
    )?;

    let _ = run_locus_job(&state, "locus_sync_task_board_v1", json!({})).await?;

    let index_path = gov_root.join("task_board").join("index.json");
    let mut index_json: Value = serde_json::from_slice(&std::fs::read(&index_path)?)?;
    index_json["authority_refs"] = json!([".GOV/roles_shared/TASK_BOARD.md"]);
    let validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::TaskBoardIndex,
        &index_json,
    );
    assert!(!validation.ok);
    assert!(validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::AuthorityScopeMismatch
            && issue.field == "authority_refs"
    }));

    Ok(())
}

#[tokio::test]
async fn locus_bind_session_normalizes_and_deduplicates_session_ids(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [{
                "mt_id": "MT-SESSION",
                "wp_id": "WP-TEST",
                "name": "Session occupancy",
                "scope": "verify session binding normalization",
                "files": {
                    "read": [],
                    "modify": [],
                    "create": [],
                },
                "done_criteria": [],
                "status": "pending",
                "active_session_ids": [],
                "iterations": [],
                "current_iteration": 0,
                "max_iterations": 1,
                "validation_result": Value::Null,
                "escalation": {
                    "current_level": 0,
                    "escalation_chain": [],
                    "escalations_count": 0,
                    "drop_backs_count": 0,
                },
                "started_at": Value::Null,
                "completed_at": Value::Null,
                "duration_ms": Value::Null,
                "depends_on": [],
                "metadata": {
                    "source": "occupancy_normalization_test",
                },
            }],
        }),
    )
    .await?;

    run_locus_job(
        &state,
        "locus_bind_session_v1",
        json!({
            "wp_id": "WP-TEST",
            "mt_id": "MT-SESSION",
            "session_id": "  sess-occupancy  ",
            "model_id": "queued-test-model",
            "lora_id": Value::Null,
            "escalation_level": 0,
        }),
    )
    .await?;
    run_locus_job(
        &state,
        "locus_bind_session_v1",
        json!({
            "wp_id": "WP-TEST",
            "mt_id": "MT-SESSION",
            "session_id": "sess-occupancy",
            "model_id": "queued-test-model",
            "lora_id": Value::Null,
            "escalation_level": 0,
        }),
    )
    .await?;

    let mt_progress = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-SESSION" }),
    )
    .await?;
    let active_session_ids = mt_progress
        .get("metadata")
        .and_then(|metadata| metadata.get("active_session_ids"))
        .and_then(Value::as_array)
        .expect("active_session_ids after bind")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(active_session_ids, vec!["sess-occupancy"]);
    let tracked_mt = mt_progress
        .get("metadata")
        .expect("tracked micro-task metadata");
    assert_eq!(
        tracked_mt.get("schema_id").and_then(Value::as_str),
        Some("hsk.tracked_micro_task@1")
    );
    assert_eq!(
        tracked_mt.get("schema_version").and_then(Value::as_str),
        Some("1")
    );
    assert_eq!(
        tracked_mt.get("record_id").and_then(Value::as_str),
        Some("MT-SESSION")
    );
    let tracked_mt_updated_at = tracked_mt
        .get("updated_at")
        .and_then(Value::as_str)
        .expect("tracked micro-task updated_at");
    assert!(
        chrono::DateTime::parse_from_rfc3339(tracked_mt_updated_at).is_ok(),
        "tracked micro-task updated_at should be RFC3339, got {tracked_mt_updated_at}"
    );
    assert_eq!(
        tracked_mt
            .get("summary_record_path")
            .and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-SESSION/summary.json")
    );
    assert_eq!(
        tracked_mt
            .get("authority_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-SESSION/packet.json")
    );
    let structured_metadata = tracked_mt
        .get("metadata")
        .and_then(Value::as_object)
        .expect("structured metadata");
    assert_eq!(
        structured_metadata
            .get("structured_collaboration_summary_path")
            .and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-SESSION/summary.json")
    );
    assert_eq!(
        structured_metadata
            .get("structured_collaboration_validation")
            .and_then(|value| value.get("ok"))
            .and_then(Value::as_bool),
        Some(true)
    );

    run_locus_job(
        &state,
        "locus_unbind_session_v1",
        json!({
            "wp_id": "WP-TEST",
            "mt_id": "MT-SESSION",
            "session_id": "  sess-occupancy  ",
            "reason": "test_cleanup",
        }),
    )
    .await?;

    let mt_progress_after = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-SESSION" }),
    )
    .await?;
    let active_session_ids_after = mt_progress_after
        .get("metadata")
        .and_then(|metadata| metadata.get("active_session_ids"))
        .and_then(Value::as_array)
        .expect("active_session_ids after unbind");
    assert!(active_session_ids_after.is_empty());

    Ok(())
}

#[tokio::test]
async fn locus_register_mts_emits_structured_micro_task_packet_and_summary(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;
    let mut tracked_mt = base_tracked_micro_task_value("MT-ARTIFACTS");
    tracked_mt["profile_extension"] = json!({
        "extension_schema_id": "hsk.profile_extension@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
        },
        "project_scope": {
            "repo": "handshake",
        },
    });

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [tracked_mt],
        }),
    )
    .await?;
    run_locus_job(
        &state,
        "locus_bind_session_v1",
        json!({
            "wp_id": "WP-TEST",
            "mt_id": "MT-ARTIFACTS",
            "session_id": "  sess-artifacts  ",
            "model_id": "queued-test-model",
            "lora_id": Value::Null,
            "escalation_level": 0,
        }),
    )
    .await?;

    let packet_path = dir
        .path()
        .join(".handshake")
        .join("gov")
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-ARTIFACTS")
        .join("packet.json");
    let summary_path = dir
        .path()
        .join(".handshake")
        .join("gov")
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-ARTIFACTS")
        .join("summary.json");
    assert!(packet_path.is_file(), "missing {}", packet_path.display());
    assert!(summary_path.is_file(), "missing {}", summary_path.display());

    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    let summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    assert_eq!(
        packet_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.tracked_micro_task@1")
    );
    assert_eq!(
        packet_json.get("schema_version").and_then(Value::as_str),
        Some("1")
    );
    assert_eq!(
        summary_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.structured_collaboration_summary@1")
    );
    assert_eq!(
        packet_json
            .get("profile_extension")
            .and_then(|value| value.get("extension_schema_id"))
            .and_then(Value::as_str),
        Some("hsk.profile_extension@1")
    );
    let active_session_ids = packet_json
        .get("active_session_ids")
        .and_then(Value::as_array)
        .expect("packet active_session_ids");
    assert_eq!(
        active_session_ids
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["sess-artifacts"]
    );

    let packet_validation = validate_runtime_structured_record(
        dir.path(),
        StructuredCollaborationRecordFamily::MicroTaskPacket,
        &packet_json,
    );
    assert!(packet_validation.ok, "{packet_validation:?}");
    let summary_validation = validate_runtime_structured_record(
        dir.path(),
        StructuredCollaborationRecordFamily::MicroTaskSummary,
        &summary_json,
    );
    assert!(summary_validation.ok, "{summary_validation:?}");
    let join_validation = validate_structured_collaboration_summary_join(
        StructuredCollaborationRecordFamily::MicroTaskPacket,
        &packet_json,
        StructuredCollaborationRecordFamily::MicroTaskSummary,
        &summary_json,
    );
    assert!(join_validation.ok, "{join_validation:?}");

    Ok(())
}

#[tokio::test]
async fn locus_written_micro_task_summary_validation_reports_authority_scope_drift(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [base_tracked_micro_task_value("MT-ARTIFACT-DRIFT")],
        }),
    )
    .await?;

    let summary_path = dir
        .path()
        .join(".handshake")
        .join("gov")
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-ARTIFACT-DRIFT")
        .join("summary.json");
    let mut summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    summary_json["authority_refs"] = json!([".GOV/roles_shared/TASK_BOARD.md"]);
    std::fs::write(&summary_path, serde_json::to_vec_pretty(&summary_json)?)?;

    let written_summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    let validation = validate_runtime_structured_record(
        dir.path(),
        StructuredCollaborationRecordFamily::MicroTaskSummary,
        &written_summary_json,
    );
    assert!(!validation.ok);
    assert!(validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::AuthorityScopeMismatch
            && issue.field == "authority_refs"
    }));

    Ok(())
}

#[tokio::test]
async fn locus_register_mts_returns_machine_readable_validation_for_summary_detail_drift(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-DRIFT");
    tracked_mt["metadata"]["structured_collaboration_summary"] = json!({
        "schema_id": "hsk.structured_collaboration_summary@1",
        "schema_version": "1",
        "record_id": "MT-OTHER",
        "record_kind": "micro_task",
        "project_profile_kind": "generic",
        "status": "pending",
        "title_or_objective": "Micro Task MT-DRIFT",
        "blockers": [],
        "next_action": "start_micro_task",
        "updated_at": "2026-03-14T00:00:00Z",
        "authority_refs": [".handshake/gov/micro_tasks/WP-TEST/MT-DRIFT/packet.json"],
        "evidence_refs": [],
    });

    let validation = run_locus_job_expect_validation_failure(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [tracked_mt],
        }),
    )
    .await?;

    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        validation.get("family").and_then(Value::as_str),
        Some("micro_task_packet")
    );
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("summary_join_mismatch")
            && issue.get("field").and_then(Value::as_str) == Some("record_id")
    }));

    Ok(())
}

#[tokio::test]
async fn locus_register_mts_returns_machine_readable_validation_for_unknown_schema_version(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-BAD-SCHEMA");
    tracked_mt["schema_version"] = Value::String("999".to_string());

    let validation = run_locus_job_expect_validation_failure(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [tracked_mt],
        }),
    )
    .await?;

    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("schema_version_mismatch")
            && issue.get("field").and_then(Value::as_str) == Some("schema_version")
    }));

    Ok(())
}

#[tokio::test]
async fn locus_register_mts_returns_machine_readable_validation_for_incompatible_profile_extension(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-BREAKING-EXT");
    tracked_mt["profile_extension"] = json!({
        "extension_schema_id": "hsk.profile_extension@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": true,
        },
    });

    let validation = run_locus_job_expect_validation_failure(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [tracked_mt],
        }),
    )
    .await?;

    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("incompatible_profile_extension")
            && issue.get("field").and_then(Value::as_str) == Some("profile_extension.compatibility")
    }));

    let packet_path = dir
        .path()
        .join(".handshake")
        .join("gov")
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-BREAKING-EXT")
        .join("packet.json");
    assert!(
        !packet_path.exists(),
        "failed invalid register_mts must not write a micro-task packet artifact"
    );

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [base_tracked_micro_task_value("MT-BREAKING-EXT")],
        }),
    )
    .await?;

    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    assert_eq!(
        packet_json.get("mt_id").and_then(Value::as_str),
        Some("MT-BREAKING-EXT")
    );
    assert!(
        packet_json.get("profile_extension").is_none()
            || packet_json.get("profile_extension") == Some(&Value::Null),
        "valid retry should not inherit the rejected profile_extension from the failed register_mts"
    );

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_escalates_and_hard_gates_after_budget_exhaustion(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
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
        .list_events(EventFilter::default())
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
    let _test_guard = test_guard();
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
    let mt_progress = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-001" }),
    )
    .await?;
    assert_eq!(
        mt_progress.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(
        mt_progress.get("current_iteration").and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        mt_progress.get("escalation_level").and_then(Value::as_i64),
        Some(1)
    );
    let metadata = mt_progress.get("metadata").expect("metadata");
    let active_session_ids = metadata
        .get("active_session_ids")
        .and_then(Value::as_array)
        .expect("active_session_ids");
    assert!(
        active_session_ids.is_empty(),
        "completed MT should not retain bound sessions"
    );
    let iterations = metadata
        .get("iterations")
        .and_then(Value::as_array)
        .expect("iterations");
    assert_eq!(
        iterations.len(),
        2,
        "expected one Locus iteration record per attempt"
    );
    let iteration_numbers = iterations
        .iter()
        .filter_map(|record| record.get("iteration").and_then(Value::as_u64))
        .collect::<Vec<_>>();
    assert_eq!(iteration_numbers, vec![1, 1]);
    let escalation_levels = iterations
        .iter()
        .filter_map(|record| record.get("escalation_level").and_then(Value::as_u64))
        .collect::<Vec<_>>();
    assert_eq!(escalation_levels, vec![0, 1]);
    let child_jobs = state
        .storage
        .list_ai_jobs(AiJobListFilter::default())
        .await?;
    assert_eq!(
        child_jobs
            .iter()
            .filter(|child_job| {
                matches!(child_job.job_kind, JobKind::LocusOperation)
                    && child_job.protocol_id == "locus_start_mt_v1"
                    && child_job
                        .job_inputs
                        .as_ref()
                        .and_then(|inputs| inputs.get("mt_id"))
                        .and_then(Value::as_str)
                        == Some("MT-001")
            })
            .count(),
        1,
        "locus_start_mt_v1 should only dispatch once per MT activation"
    );
    assert_eq!(
        child_jobs
            .iter()
            .filter(|child_job| {
                matches!(child_job.job_kind, JobKind::LocusOperation)
                    && child_job.protocol_id == "locus_record_iteration_v1"
                    && child_job
                        .job_inputs
                        .as_ref()
                        .and_then(|inputs| inputs.get("mt_id"))
                        .and_then(Value::as_str)
                        == Some("MT-001")
            })
            .count(),
        2,
        "expected one locus_record_iteration_v1 child job per attempt"
    );

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
    let _test_guard = test_guard();
    let llm = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        "done <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let llm_client: Arc<dyn LlmClient> = llm.clone();
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
                        { "level": 0, "model_id": "qwen2.5-coder:7b", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "qwen2.5-coder:13b", "is_cloud": false, "is_hard_gate": false }
                    ]
                }
            })),
        })
        .await?;
    let job_id = job.job_id;

    start_workflow_for_job(&state, job).await?;

    let updated_job = state.storage.get_ai_job(&job_id.to_string()).await?;
    assert!(matches!(updated_job.state, JobState::Completed));

    let swap_calls = llm.swap_calls.lock().expect("swap_calls mutex poisoned");
    assert_eq!(swap_calls.len(), 1, "swap_model should be invoked once");
    assert_eq!(swap_calls[0].current_model_id, "qwen2.5-coder:7b");
    assert_eq!(swap_calls[0].target_model_id, "qwen2.5-coder:13b");

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

    let request_id = requested
        .get("request_id")
        .and_then(|v| v.as_str())
        .expect("request_id present");

    assert_eq!(
        requested.get("type").and_then(|v| v.as_str()),
        Some("model_swap_requested")
    );
    assert_eq!(
        requested.get("current_model_id").and_then(|v| v.as_str()),
        Some("qwen2.5-coder:7b")
    );
    assert_eq!(
        requested.get("target_model_id").and_then(|v| v.as_str()),
        Some("qwen2.5-coder:13b")
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

    let refs: Vec<String> = requested
        .get("state_persist_refs")
        .and_then(|v| v.as_array())
        .expect("state_persist_refs array")
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let repo_root = handshake_core::capability_registry_workflow::repo_root_from_manifest_dir()?;

    assert!(
        !refs
            .iter()
            .any(|r| r.contains("/model_swap/request_") || r.contains("/model_swap/swap_state_")),
        "state_persist_refs should not include swap request/state files"
    );

    let expected_state_hash = compute_model_swap_state_hash(&repo_root, &refs)?;
    assert_eq!(expected_state_hash, state_hash);

    let swap_dir = repo_root
        .join("data")
        .join("micro_task_executor")
        .join(job_id.to_string())
        .join("model_swap");
    let request_path = swap_dir.join(format!("request_{}.json", request_id));
    let state_path = swap_dir.join(format!("swap_state_{}.json", request_id));
    assert!(request_path.exists(), "persisted swap request must exist");
    assert!(state_path.exists(), "persisted swap state must exist");

    let raw_request = std::fs::read_to_string(&request_path)?;
    let request_json: serde_json::Value = serde_json::from_str(&raw_request)?;
    assert_eq!(
        request_json.get("state_hash").and_then(|v| v.as_str()),
        Some(state_hash)
    );

    let context_compile_ref = completed
        .get("context_compile_ref")
        .and_then(|v| v.as_str())
        .expect("context_compile_ref present");
    let context_compile_path = model_swap_ref_to_abs_path(&repo_root, context_compile_ref);
    assert!(
        context_compile_path.exists(),
        "context_compile_ref must exist before ModelSwapCompleted is emitted"
    );

    let raw_context_compile = std::fs::read_to_string(&context_compile_path)?;
    let compile_json: serde_json::Value = serde_json::from_str(&raw_context_compile)?;
    assert_eq!(
        compile_json.get("target_model_id").and_then(|v| v.as_str()),
        Some("qwen2.5-coder:13b")
    );

    let context_snapshot_ref = compile_json
        .get("context_snapshot_ref")
        .expect("context_snapshot_ref present");
    let context_snapshot_path =
        if let Some(path) = context_snapshot_ref.get("path").and_then(|v| v.as_str()) {
            repo_root.join(path)
        } else if let Some(s) = context_snapshot_ref.as_str() {
            model_swap_ref_to_abs_path(&repo_root, s)
        } else {
            return Err(
                "context_snapshot_ref must be artifact handle with path or canonical string".into(),
            );
        };
    assert!(
        context_snapshot_path.exists(),
        "context snapshot must exist"
    );

    let raw_context_snapshot = std::fs::read_to_string(&context_snapshot_path)?;
    let context_snapshot_json: serde_json::Value = serde_json::from_str(&raw_context_snapshot)?;
    assert_eq!(
        context_snapshot_json
            .get("model_id")
            .and_then(|v| v.as_str()),
        Some("qwen2.5-coder:13b")
    );

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_emits_model_swap_failed_when_policy_disallows_swaps(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm = Arc::new(QueuedLlmClient::new(vec![
        "still working".to_string(),
        "done <mt_complete>yes</mt_complete>".to_string(),
    ]));
    let llm_client: Arc<dyn LlmClient> = llm.clone();
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
                        { "level": 0, "model_id": "qwen2.5-coder:7b", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "qwen2.5-coder:13b", "is_cloud": false, "is_hard_gate": false }
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

    let swap_calls = llm.swap_calls.lock().expect("swap_calls mutex poisoned");
    assert!(
        swap_calls.is_empty(),
        "swap_model should not run when swaps are disallowed by policy"
    );

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
async fn micro_task_executor_emits_model_swap_runtime_failure_and_rollback_when_fallback_allows(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm = Arc::new(
        QueuedLlmClient::new(vec![
            "still working".to_string(),
            "done <mt_complete>yes</mt_complete>".to_string(),
        ])
        .with_swap_error("boom"),
    );
    let llm_client: Arc<dyn LlmClient> = llm.clone();
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
                        { "level": 0, "model_id": "qwen2.5-coder:7b", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "qwen2.5-coder:13b", "is_cloud": false, "is_hard_gate": false }
                    ],
                    "extensions": [
                        {
                            "schema_version": "hsk.exec_policy_ext@0.4",
                            "kind": "model_swap_policy",
                            "model_swap_policy": {
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

    let swap_calls = llm.swap_calls.lock().expect("swap_calls mutex poisoned");
    assert_eq!(swap_calls.len(), 1, "swap_model should be invoked once");

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
    let mut rollback_payload: Option<serde_json::Value> = None;
    let mut completed_payload: Option<serde_json::Value> = None;

    while let Some(row) = rows.next()? {
        let event_type: String = row.get(0)?;
        let payload_str: String = row.get(1)?;
        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;

        if event_type == FlightRecorderEventType::ModelSwapRequested.to_string() {
            requested_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapFailed.to_string() {
            failed_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapRollback.to_string() {
            rollback_payload = Some(payload);
        } else if event_type == FlightRecorderEventType::ModelSwapCompleted.to_string() {
            completed_payload = Some(payload);
        }
    }

    let requested = requested_payload.expect("model_swap_requested event");
    let failed = failed_payload.expect("model_swap_failed event");
    let rollback = rollback_payload.expect("model_swap_rollback event");

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
    assert!(failed
        .get("error_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("runtime_failure:"));

    assert_eq!(
        rollback.get("type").and_then(|v| v.as_str()),
        Some("model_swap_rollback")
    );
    assert_eq!(
        rollback.get("outcome").and_then(|v| v.as_str()),
        Some("rollback")
    );

    assert!(
        completed_payload.is_none(),
        "swap should not complete on runtime failure"
    );

    Ok(())
}

#[tokio::test]
async fn micro_task_executor_emits_model_swap_timeout_and_rollback_on_runtime_timeout(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let llm = Arc::new(
        QueuedLlmClient::new(vec![
            "still working".to_string(),
            "done <mt_complete>yes</mt_complete>".to_string(),
        ])
        .with_swap_delay_ms(50),
    );
    let llm_client: Arc<dyn LlmClient> = llm.clone();
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
                        { "level": 0, "model_id": "qwen2.5-coder:7b", "is_cloud": false, "is_hard_gate": false },
                        { "level": 1, "model_id": "qwen2.5-coder:13b", "is_cloud": false, "is_hard_gate": false }
                    ],
                    "extensions": [
                        {
                            "schema_version": "hsk.exec_policy_ext@0.4",
                            "kind": "model_swap_policy",
                            "model_swap_policy": {
                                "swap_timeout_ms": 10,
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

    let swap_calls = llm.swap_calls.lock().expect("swap_calls mutex poisoned");
    assert_eq!(swap_calls.len(), 1, "swap_model should be invoked once");

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
    let _test_guard = test_guard();
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
    let _test_guard = test_guard();
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
