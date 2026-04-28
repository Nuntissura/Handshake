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
        executor_eligibility_policy_ids_for_family, governed_action_ids_for_family,
        is_governed_action_id_allowed_for_workflow_family, is_registered_governed_action_id,
        queue_automation_rule_ids_for_reason, transition_rule_ids_for_family,
        task_board::{TaskBoardEntryRecordV1, TaskBoardIndexV1, TaskBoardViewV1},
        validate_structured_collaboration_record, validate_structured_collaboration_summary_join,
        validate_task_board_entry_authoritative_fields, StructuredCollaborationRecordFamily,
        StructuredCollaborationValidationCode, StructuredCollaborationValidationResult,
        TrackedMicroTaskArtifactV1, TrackedWorkPacketArtifactV1, WorkflowQueueReasonCode,
        WorkflowStateFamily,
    },
    start_workflow_for_job, ModelSwapRequestV0_4, SessionRegistry, SessionSchedulerConfig,
};
use handshake_core::workflows::{
    apply_software_delivery_closeout_posture_lifecycle,
    apply_software_delivery_projection_surface_lifecycle,
    apply_software_delivery_workflow_run_lifecycle,
    build_software_delivery_projection_surface_with_overlay,
    finalize_runtime_structured_work_packet_writes,
};
use handshake_core::workflows::locus::{
    derive_governed_action_preview, derive_governed_action_previews,
    derive_software_delivery_closeout_posture, derive_software_delivery_projection_surface,
    derive_software_delivery_workflow_binding_state, mirror_state_is_advisory_only,
    task_board::enforce_software_delivery_task_board_projection_authority,
    validate_software_delivery_closeout_canonical_truth,
    validate_software_delivery_projection_surface_authority,
    validate_software_delivery_projection_surface_overlay,
    validate_software_delivery_task_board_projection_against_canonical,
    GovernanceClaimLeaseRecordV1, GovernanceQueuedInstructionRecordV1,
    GovernedActionEligibility, GovernedActionPreviewV1, MirrorSyncState, ProjectProfileKind,
    SoftwareDeliveryBindingGatePosture, SoftwareDeliveryClaimTakeoverPolicy,
    SoftwareDeliveryCloseoutState, SoftwareDeliveryProjectionSurfaceV1,
    SoftwareDeliveryQueuedInstructionAction, SoftwareDeliveryWorkflowBindingState,
    SoftwareDeliveryWorkflowRunLifecycleV1, StructuredCollaborationSummaryV1,
    GOVERNED_ACTION_PREVIEW_RECORD_KIND, GOVERNED_ACTION_PREVIEW_SCHEMA_ID_V1,
    SOFTWARE_DELIVERY_CLAIM_LEASE_RECORD_KIND, SOFTWARE_DELIVERY_CLAIM_LEASE_SCHEMA_ID_V1,
    SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND,
    SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_ID_V1,
    SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_RECORD_KIND,
    SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_SCHEMA_ID_V1,
    SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND,
    SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1,
};
use handshake_core::role_mailbox::{
    build_software_delivery_overlay_triage_row, SoftwareDeliveryOverlayTriageRowV1,
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

async fn setup_state_without_seed(
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
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
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

fn json_string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn json_field_absent_or_null(value: &Value, field: &str) -> bool {
    value.get(field).map(Value::is_null).unwrap_or(true)
}

fn validate_task_board_row_against_packet_truth(
    row: &Value,
    packet: &Value,
) -> StructuredCollaborationValidationResult {
    let entry = serde_json::from_value(row.clone()).expect("task-board entry record");
    let expected_work_packet_id = packet
        .get("wp_id")
        .and_then(Value::as_str)
        .expect("work-packet id in packet")
        .to_string();
    let expected_workflow_state_family: WorkflowStateFamily = serde_json::from_value(
        packet
            .get("workflow_state_family")
            .cloned()
            .expect("workflow_state_family in packet"),
    )
    .expect("workflow_state_family enum");
    let expected_queue_reason_code: WorkflowQueueReasonCode = serde_json::from_value(
        packet
            .get("queue_reason_code")
            .cloned()
            .expect("queue_reason_code in packet"),
    )
    .expect("queue_reason_code enum");
    let expected_allowed_action_ids = json_string_array_field(packet, "allowed_action_ids");
    assert_eq!(
        expected_allowed_action_ids,
        governed_action_ids_for_family(expected_workflow_state_family)
    );

    validate_task_board_entry_authoritative_fields(
        &entry,
        &expected_work_packet_id,
        expected_workflow_state_family,
        expected_queue_reason_code,
        &expected_allowed_action_ids,
    )
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
    assert!(
        matches!(updated_job.state, JobState::Completed),
        "job did not complete: state={:?}, status_reason={}, error={:?}, output={:?}",
        updated_job.state,
        updated_job.status_reason,
        updated_job.error_message,
        updated_job.job_outputs
    );

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
async fn locus_create_and_close_wp_emit_structured_work_packet_packet_and_summary_with_profile_extension(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state_without_seed(llm_client).await?;
    let wp_id = "WP-PROFILE";
    let software_delivery_extension = json!({
        "extension_schema_id": "hsk.profile.software_delivery@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
        },
        "repository": {
            "default_branch": "main",
        },
    });
    run_locus_job(
        &state,
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
            "project_profile_kind": "software_delivery",
            "profile_extension": software_delivery_extension.clone(),
            "reporter": "micro_task_executor_tests",
        }),
    )
    .await?;

    let packet_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join(wp_id)
        .join("packet.json");
    let summary_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join(wp_id)
        .join("summary.json");

    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    let summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    assert_eq!(
        packet_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.tracked_work_packet@1")
    );
    assert_eq!(
        packet_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(json_field_absent_or_null(&packet_json, "profile_extension"));
    let packet_updated_at = packet_json
        .get("updated_at")
        .and_then(Value::as_str)
        .expect("tracked work packet updated_at");
    assert!(
        chrono::DateTime::parse_from_rfc3339(packet_updated_at).is_ok(),
        "tracked work packet updated_at should be RFC3339, got {packet_updated_at}"
    );
    let packet_metadata = packet_json
        .get("metadata")
        .and_then(Value::as_object)
        .expect("tracked work packet metadata");
    assert_eq!(
        packet_json.get("summary_ref").and_then(Value::as_str),
        Some(".handshake/gov/work_packets/WP-PROFILE/summary.json")
    );
    let legacy_packet: TrackedWorkPacketArtifactV1 =
        serde_json::from_value(packet_json.clone()).expect("legacy work-packet artifact");
    assert_eq!(
        legacy_packet.summary_ref,
        ".handshake/gov/work_packets/WP-PROFILE/summary.json"
    );
    assert!(legacy_packet.profile_extension.is_none());
    assert_eq!(
        packet_json
            .get("authority_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/work_packets/WP-PROFILE/packet.json")
    );
    assert_eq!(
        packet_json
            .get("evidence_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        packet_json
            .get("governance")
            .and_then(|value| value.get("task_packet_path"))
            .and_then(Value::as_str)
    );
    assert_eq!(
        packet_metadata
            .get("structured_collaboration_summary_path")
            .and_then(Value::as_str),
        Some(".handshake/gov/work_packets/WP-PROFILE/summary.json")
    );
    let packet_summary = packet_metadata
        .get("structured_collaboration_summary")
        .and_then(Value::as_object)
        .expect("tracked work packet embedded summary");
    assert_eq!(
        packet_summary.get("record_id").and_then(Value::as_str),
        Some("WP-PROFILE")
    );
    assert_eq!(
        packet_summary.get("status").and_then(Value::as_str),
        summary_json.get("status").and_then(Value::as_str)
    );
    assert_eq!(
        packet_summary
            .get("title_or_objective")
            .and_then(Value::as_str),
        summary_json
            .get("title_or_objective")
            .and_then(Value::as_str)
    );
    assert_eq!(
        packet_summary.get("next_action").and_then(Value::as_str),
        summary_json.get("next_action").and_then(Value::as_str)
    );
    assert_eq!(
        summary_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.structured_collaboration_summary@1")
    );
    assert_eq!(
        summary_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(json_field_absent_or_null(
        &summary_json,
        "profile_extension"
    ));
    assert_eq!(
        summary_json.get("status").and_then(Value::as_str),
        Some("stub")
    );
    assert_eq!(
        summary_json
            .get("workflow_state_family")
            .and_then(Value::as_str),
        Some("intake")
    );
    assert!(
        summary_json
            .get("next_action")
            .and_then(Value::as_str)
            .map(is_registered_governed_action_id)
            .unwrap_or(true),
        "work packet summary next_action must be a registered governed action id or be omitted"
    );
    assert!(
        summary_json
            .get("next_action")
            .and_then(Value::as_str)
            .map(|action_id| {
                is_governed_action_id_allowed_for_workflow_family(
                    WorkflowStateFamily::Intake,
                    action_id,
                )
            })
            .unwrap_or(true),
        "work packet summary next_action must be allowed for workflow_state_family=intake or be omitted"
    );

    let packet_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json,
    );
    assert!(packet_validation.ok, "{packet_validation:?}");
    let mut packet_json_missing_updated_at = packet_json.clone();
    packet_json_missing_updated_at
        .as_object_mut()
        .expect("packet json object")
        .remove("updated_at");
    let missing_updated_at_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &packet_json_missing_updated_at,
    );
    assert!(!missing_updated_at_validation.ok);
    assert!(missing_updated_at_validation.issues.iter().any(|issue| {
        matches!(
            issue.code,
            StructuredCollaborationValidationCode::MissingField
        ) && issue.field == "updated_at"
    }));
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

    run_locus_job(&state, "locus_close_wp_v1", json!({ "wp_id": wp_id })).await?;

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
async fn locus_schema_registry_rejects_unregistered_allowed_action_ids(
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

    let packet_path = gov_root
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let mut work_packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    work_packet_json["allowed_action_ids"][0] = Value::String("not_registered".to_string());
    let work_packet_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketPacket,
        &work_packet_json,
    );
    assert!(!work_packet_validation.ok);
    assert!(work_packet_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "allowed_action_ids[0]"
    }));

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [base_tracked_micro_task_value("MT-SCHEMA-REGISTRY")],
        }),
    )
    .await?;

    let micro_task_packet_path = gov_root
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-SCHEMA-REGISTRY")
        .join("packet.json");
    let micro_task_packet_json: Value =
        serde_json::from_slice(&std::fs::read(&micro_task_packet_path)?)?;
    let mt_progress = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-SCHEMA-REGISTRY" }),
    )
    .await?;
    let progress_metadata = mt_progress
        .get("metadata")
        .expect("micro-task progress metadata");
    assert_eq!(
        progress_metadata.get("allowed_action_ids"),
        micro_task_packet_json.get("allowed_action_ids")
    );

    let mut mutated_micro_task_packet_json = micro_task_packet_json.clone();
    mutated_micro_task_packet_json["allowed_action_ids"][0] =
        Value::String("not_registered".to_string());
    let micro_task_validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::MicroTaskPacket,
        &mutated_micro_task_packet_json,
    );
    assert!(!micro_task_validation.ok);
    assert!(micro_task_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "allowed_action_ids[0]"
    }));

    Ok(())
}

#[tokio::test]
async fn locus_written_work_packet_summary_validation_rejects_unregistered_next_action(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let _state = setup_state(llm_client).await?;

    let summary_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("summary.json");
    let mut summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    summary_json["next_action"] = json!("start_work_packet");
    std::fs::write(&summary_path, serde_json::to_vec_pretty(&summary_json)?)?;

    let written_summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    let validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketSummary,
        &written_summary_json,
    );
    assert!(!validation.ok);
    assert!(validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "next_action"
            && issue.actual.as_deref() == Some("start_work_packet")
    }));

    Ok(())
}

#[tokio::test]
async fn locus_written_work_packet_summary_validation_rejects_family_illegal_next_action(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let root = dir.path().to_path_buf();
    let _workspace_guard = WorkspaceEnvGuard::activate(&root);
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let _state = setup_state(llm_client).await?;

    let summary_path = root
        .join(".handshake")
        .join("gov")
        .join("work_packets")
        .join("WP-TEST")
        .join("summary.json");
    let mut summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    summary_json["next_action"] = json!("archive");
    std::fs::write(&summary_path, serde_json::to_vec_pretty(&summary_json)?)?;

    let written_summary_json: Value = serde_json::from_slice(&std::fs::read(&summary_path)?)?;
    let validation = validate_runtime_structured_record(
        &root,
        StructuredCollaborationRecordFamily::WorkPacketSummary,
        &written_summary_json,
    );
    assert!(!validation.ok);
    assert!(validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "next_action"
            && issue.actual.as_deref() == Some("archive")
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
    let state = setup_state_without_seed(llm_client).await?;
    let wp_id = "WP-RESEARCH";
    let research_extension = json!({
        "extension_schema_id": "hsk.profile.research@1",
        "extension_schema_version": "1",
        "compatibility": "compatible",
        "study_type": "literature_review",
    });
    run_locus_job(
        &state,
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
            "project_profile_kind": "research",
            "profile_extension": research_extension.clone(),
            "reporter": "micro_task_executor_tests",
        }),
    )
    .await?;

    let gov_root = root.join(".handshake").join("gov");
    std::fs::create_dir_all(&gov_root)?;
    std::fs::write(
        gov_root.join("TASK_BOARD.md"),
        concat!(
            "# Task Board\n\n",
            "## Ready for Dev\n",
            "- **[WP-RESEARCH]** - [ready]\n"
        ),
    )?;

    let sync_result = run_locus_job(&state, "locus_sync_task_board_v1", json!({})).await?;
    assert_eq!(
        sync_result
            .get("structured_projection_written")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        sync_result
            .get("work_packet_artifacts_written")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        sync_result
            .get("validation_result")
            .and_then(|value| value.get("ok"))
            .and_then(Value::as_bool),
        Some(true)
    );

    let index_path = gov_root.join("task_board").join("index.json");
    let view_path = gov_root
        .join("task_board")
        .join("views")
        .join("default.json");
    assert!(
        index_path.exists(),
        "missing task-board index artifact at {}",
        index_path.display()
    );
    assert!(
        view_path.exists(),
        "missing task-board view artifact at {}",
        view_path.display()
    );
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
        Some("WP-RESEARCH")
    );
    assert_eq!(
        first_row.get("task_board_id").and_then(Value::as_str),
        Some(".handshake/gov/TASK_BOARD.md")
    );
    assert_eq!(
        first_row.get("lane_id").and_then(Value::as_str),
        Some("ready")
    );
    assert_eq!(
        first_row
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(json_field_absent_or_null(first_row, "profile_extension"));
    assert_eq!(
        first_row.get("display_order").and_then(Value::as_u64),
        Some(0)
    );
    let first_row_view_ids = first_row
        .get("view_ids")
        .and_then(Value::as_array)
        .expect("task-board row view_ids");
    assert_eq!(
        first_row_view_ids
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["default"]
    );
    assert_eq!(
        view_json
            .get("lane_ids")
            .and_then(Value::as_array)
            .map(|lane_ids| {
                lane_ids
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            "stub",
            "ready",
            "in_progress",
            "blocked",
            "gated",
            "done",
            "cancelled",
        ])
    );
    assert_eq!(
        index_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert_eq!(
        view_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(json_field_absent_or_null(&index_json, "profile_extension"));
    assert!(json_field_absent_or_null(&view_json, "profile_extension"));
    let index_record: TaskBoardIndexV1 = serde_json::from_value(index_json.clone())?;
    let view_record: TaskBoardViewV1 = serde_json::from_value(view_json.clone())?;
    let row_record: TaskBoardEntryRecordV1 = serde_json::from_value(first_row.clone())?;
    assert_eq!(index_record.profile_extension, None);
    assert_eq!(view_record.profile_extension, None);
    assert_eq!(row_record.profile_extension, None);

    let packet_path = gov_root
        .join("work_packets")
        .join("WP-RESEARCH")
        .join("packet.json");
    assert!(
        packet_path.exists(),
        "missing work-packet artifact at {}",
        packet_path.display()
    );
    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    assert_eq!(
        packet_json.get("status").and_then(Value::as_str),
        Some("ready")
    );
    assert_eq!(
        first_row
            .get("workflow_state_family")
            .and_then(Value::as_str),
        packet_json
            .get("workflow_state_family")
            .and_then(Value::as_str)
    );
    assert_eq!(
        first_row.get("queue_reason_code").and_then(Value::as_str),
        packet_json.get("queue_reason_code").and_then(Value::as_str)
    );
    assert_eq!(
        first_row.get("allowed_action_ids"),
        packet_json.get("allowed_action_ids")
    );
    let row_truth_validation =
        validate_task_board_row_against_packet_truth(first_row, &packet_json);
    assert!(row_truth_validation.ok, "{row_truth_validation:?}");

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
async fn locus_task_board_validation_reports_authoritative_row_drift(
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
    let packet_path = gov_root
        .join("work_packets")
        .join("WP-TEST")
        .join("packet.json");
    let index_json: Value = serde_json::from_slice(&std::fs::read(&index_path)?)?;
    let packet_json: Value = serde_json::from_slice(&std::fs::read(&packet_path)?)?;
    let base_row = index_json
        .get("rows")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .ok_or("missing task-board row")?;

    let base_validation = validate_task_board_row_against_packet_truth(&base_row, &packet_json);
    assert!(base_validation.ok, "{base_validation:?}");

    let mut drifted_row = base_row;
    drifted_row["workflow_state_family"] = json!("blocked");
    drifted_row["queue_reason_code"] = json!("blocked_policy");
    drifted_row["allowed_action_ids"] = json!(["not_registered"]);

    let drift_validation = validate_task_board_row_against_packet_truth(&drifted_row, &packet_json);
    assert!(!drift_validation.ok);
    assert!(drift_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "workflow_state_family"
    }));
    assert!(drift_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "queue_reason_code"
    }));
    assert!(drift_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "allowed_action_ids"
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
        tracked_mt.get("summary_ref").and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-SESSION/summary.json")
    );
    let legacy_micro_task: TrackedMicroTaskArtifactV1 =
        serde_json::from_value(tracked_mt.clone()).expect("legacy micro-task artifact");
    assert_eq!(
        legacy_micro_task.summary_ref,
        ".handshake/gov/micro_tasks/WP-TEST/MT-SESSION/summary.json"
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
async fn locus_register_mts_emits_structured_micro_task_packet_and_summary_with_research_profile_extension(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;
    let research_extension = json!({
        "extension_schema_id": "hsk.profile.research.exploratory@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
        },
        "study_type": "exploratory",
    });
    let mut tracked_mt = base_tracked_micro_task_value("MT-ARTIFACTS");
    tracked_mt["project_profile_kind"] = Value::String("research".to_string());
    tracked_mt["profile_extension"] = research_extension.clone();

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
        packet_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("research")
    );
    assert_eq!(
        packet_json.get("profile_extension"),
        Some(&research_extension)
    );
    let packet_updated_at = packet_json
        .get("updated_at")
        .and_then(Value::as_str)
        .expect("tracked micro-task updated_at");
    assert!(
        chrono::DateTime::parse_from_rfc3339(packet_updated_at).is_ok(),
        "tracked micro-task updated_at should be RFC3339, got {packet_updated_at}"
    );
    assert_eq!(
        summary_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.structured_collaboration_summary@1")
    );
    assert_eq!(
        summary_json
            .get("workflow_state_family")
            .and_then(Value::as_str),
        Some("active")
    );
    assert!(
        summary_json
            .get("next_action")
            .and_then(Value::as_str)
            .map(|action_id| {
                is_governed_action_id_allowed_for_workflow_family(
                    WorkflowStateFamily::Active,
                    action_id,
                )
            })
            .unwrap_or(true),
        "micro-task summary next_action must be allowed for workflow_state_family=active or be omitted"
    );
    assert_eq!(
        summary_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("research")
    );
    assert_eq!(
        summary_json.get("profile_extension"),
        Some(&research_extension)
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
    assert_eq!(
        packet_json
            .get("evidence_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-ARTIFACTS/summary.json")
    );
    let packet_metadata = packet_json
        .get("metadata")
        .and_then(Value::as_object)
        .expect("tracked micro-task metadata");
    let legacy_packet: TrackedMicroTaskArtifactV1 =
        serde_json::from_value(packet_json.clone()).expect("legacy micro-task packet");
    assert_eq!(
        legacy_packet.summary_ref,
        ".handshake/gov/micro_tasks/WP-TEST/MT-ARTIFACTS/summary.json"
    );
    assert_eq!(
        legacy_packet.profile_extension.as_ref(),
        Some(&research_extension)
    );
    assert_eq!(
        packet_metadata
            .get("structured_collaboration_summary_path")
            .and_then(Value::as_str),
        Some(".handshake/gov/micro_tasks/WP-TEST/MT-ARTIFACTS/summary.json")
    );
    let packet_summary = packet_metadata
        .get("structured_collaboration_summary")
        .and_then(Value::as_object)
        .expect("tracked micro-task embedded summary");
    assert_eq!(
        packet_summary.get("record_id").and_then(Value::as_str),
        Some("MT-ARTIFACTS")
    );
    assert_eq!(
        packet_summary.get("status").and_then(Value::as_str),
        summary_json.get("status").and_then(Value::as_str)
    );
    assert_eq!(
        packet_summary
            .get("title_or_objective")
            .and_then(Value::as_str),
        summary_json
            .get("title_or_objective")
            .and_then(Value::as_str)
    );
    assert_eq!(
        packet_summary.get("next_action").and_then(Value::as_str),
        summary_json.get("next_action").and_then(Value::as_str)
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
async fn locus_register_mts_rejects_unregistered_summary_next_action(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-NEXT-ACTION-DRIFT");
    tracked_mt["metadata"]["structured_collaboration_summary"] = json!({
        "schema_id": "hsk.structured_collaboration_summary@1",
        "schema_version": "1",
        "record_id": "MT-NEXT-ACTION-DRIFT",
        "record_kind": "micro_task",
        "project_profile_kind": "generic",
        "workflow_state_family": "ready",
        "status": "pending",
        "title_or_objective": "Micro Task MT-NEXT-ACTION-DRIFT",
        "blockers": [],
        "next_action": "start_micro_task",
        "updated_at": "2026-03-14T00:00:00Z",
        "authority_refs": [".handshake/gov/micro_tasks/WP-TEST/MT-NEXT-ACTION-DRIFT/packet.json"],
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
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("invalid_field_value")
            && issue.get("field").and_then(Value::as_str) == Some("next_action")
            && issue.get("actual").and_then(Value::as_str) == Some("start_micro_task")
    }));

    Ok(())
}

#[tokio::test]
async fn locus_register_mts_rejects_family_illegal_summary_next_action(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-FAMILY-NEXT-ACTION-DRIFT");
    tracked_mt["metadata"]["structured_collaboration_summary"] = json!({
        "schema_id": "hsk.structured_collaboration_summary@1",
        "schema_version": "1",
        "record_id": "MT-FAMILY-NEXT-ACTION-DRIFT",
        "record_kind": "micro_task",
        "project_profile_kind": "generic",
        "workflow_state_family": "ready",
        "status": "pending",
        "title_or_objective": "Micro Task MT-FAMILY-NEXT-ACTION-DRIFT",
        "blockers": [],
        "next_action": "archive",
        "updated_at": "2026-03-14T00:00:00Z",
        "authority_refs": [".handshake/gov/micro_tasks/WP-TEST/MT-FAMILY-NEXT-ACTION-DRIFT/packet.json"],
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
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("invalid_field_value")
            && issue.get("field").and_then(Value::as_str) == Some("next_action")
            && issue.get("actual").and_then(Value::as_str) == Some("archive")
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
        "workflow_state_family": "ready",
        "status": "pending",
        "title_or_objective": "Micro Task MT-DRIFT",
        "blockers": [],
        "next_action": "start",
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
async fn locus_register_mts_returns_machine_readable_validation_for_unknown_profile_extension_schema(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let llm_client: Arc<dyn LlmClient> = Arc::new(QueuedLlmClient::new(vec![]));
    let state = setup_state(llm_client).await?;

    let mut tracked_mt = base_tracked_micro_task_value("MT-UNKNOWN-EXT");
    tracked_mt["project_profile_kind"] = Value::String("research".to_string());
    tracked_mt["profile_extension"] = json!({
        "extension_schema_id": "hsk.profile.unknown@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
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
        issue.get("code").and_then(Value::as_str) == Some("invalid_field_value")
            && issue.get("field").and_then(Value::as_str)
                == Some("profile_extension.extension_schema_id")
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
    tracked_mt["project_profile_kind"] = Value::String("research".to_string());
    tracked_mt["profile_extension"] = json!({
        "extension_schema_id": "hsk.profile.research@1",
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

// ── MT-001: v02.181 software-delivery projection-surface discipline tripwire ─

fn software_delivery_canonical_summary() -> StructuredCollaborationSummaryV1 {
    StructuredCollaborationSummaryV1 {
        schema_id: "hsk.structured_collaboration_summary@1".to_string(),
        schema_version: "1".to_string(),
        record_id: "WP-1-Software-Delivery-Test".to_string(),
        record_kind: "work_packet".to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: "2026-04-27T18:00:00Z".to_string(),
        mirror_state: MirrorSyncState::CanonicalOnly,
        authority_refs: vec![
            ".handshake/gov/work_packets/WP-1-Software-Delivery-Test/packet.json".to_string(),
        ],
        evidence_refs: vec![
            ".handshake/gov/validator_gates/WP-1-Software-Delivery-Test.json".to_string(),
        ],
        mirror_contract: None,
        workflow_state_family: WorkflowStateFamily::Validation,
        queue_reason_code: WorkflowQueueReasonCode::ValidationWait,
        allowed_action_ids: Vec::new(),
        transition_rule_ids: transition_rule_ids_for_family(WorkflowStateFamily::Validation),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::ValidationWait,
        ),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            WorkflowStateFamily::Validation,
        ),
        status: "validation: awaiting validator".to_string(),
        title_or_objective: "MT-001 projection-surface discipline tripwire".to_string(),
        blockers: vec!["validator_kickoff_pending".to_string()],
        next_action: Some("await_validator_review".to_string()),
        summary_ref: Some(
            ".handshake/gov/work_packets/WP-1-Software-Delivery-Test/summary.json".to_string(),
        ),
    }
}

fn stale_advisory_board_entry(record_id: &str) -> TaskBoardEntryRecordV1 {
    // Mirror lies: claims `done` lane and `Done` workflow_state_family, with
    // mirror_state=Stale. Discipline must keep canonical truth in the
    // projection surface even when the mirror disagrees.
    TaskBoardEntryRecordV1 {
        schema_id: "hsk.task_board_entry@1".to_string(),
        schema_version: "1".to_string(),
        record_id: format!("tb:{record_id}"),
        record_kind: "task_board_entry".to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        profile_extension: None,
        updated_at: "2026-04-25T12:00:00Z".to_string(),
        mirror_state: MirrorSyncState::Stale,
        authority_refs: vec![
            "/.GOV/task_packets/should-not-be-authoritative.md".to_string(),
        ],
        evidence_refs: Vec::new(),
        mirror_contract: None,
        workflow_state_family: WorkflowStateFamily::Done,
        queue_reason_code: WorkflowQueueReasonCode::ReadyForHuman,
        allowed_action_ids: vec!["mirror_only_action".to_string()],
        transition_rule_ids: transition_rule_ids_for_family(WorkflowStateFamily::Done),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::ReadyForHuman,
        ),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            WorkflowStateFamily::Done,
        ),
        task_board_id: "tb-software-delivery-v1".to_string(),
        work_packet_id: record_id.to_string(),
        lane_id: "done".to_string(),
        display_order: 0,
        view_ids: Vec::new(),
        token: "STALE-DONE".to_string(),
        status: "Done (mirror)".to_string(),
        summary_ref: format!(
            ".handshake/gov/work_packets/{record_id}/summary.json"
        ),
    }
}

#[test]
fn dcc_software_delivery_projection_surface_keeps_runtime_authority() {
    let canonical = software_delivery_canonical_summary();
    let board_entry = stale_advisory_board_entry(&canonical.record_id);
    let mailbox_thread_ids = vec![
        // Advisory chronology: latest reply hints at "completion" but is not
        // authority.  The projection surface must only carry the thread ids,
        // never let the chronology overwrite canonical authority.
        "thread-software-delivery-validator-kickoff".to_string(),
        "thread-software-delivery-coder-intent".to_string(),
        "".to_string(), // empty must be filtered
        "thread-software-delivery-coder-intent".to_string(), // duplicate must be deduped
    ];

    let projection = derive_software_delivery_projection_surface(
        &canonical,
        Some("workflow-run-1234"),
        Some("session-coder-1"),
        Some(&board_entry),
        &mailbox_thread_ids,
    )
    .expect("software-delivery canonical summary must derive a projection surface");

    // ── Schema and record-kind anchors ───────────────────────────────────────
    assert_eq!(
        projection.schema_id, SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_ID_V1,
        "projection must declare the v02.181 software-delivery schema id"
    );
    assert_eq!(
        projection.record_kind, SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND,
        "projection must declare the software-delivery record kind"
    );
    assert_eq!(
        projection.project_profile_kind,
        ProjectProfileKind::SoftwareDelivery,
        "projection must remain scoped to the software_delivery profile"
    );

    // ── Stable-id join: DCC, Task Board, and mailbox refs share the wp_id ──
    assert_eq!(
        projection.work_packet_id, canonical.record_id,
        "projection.work_packet_id must equal canonical record_id"
    );
    assert_eq!(
        projection.work_packet_id, board_entry.work_packet_id,
        "projection and board entry must agree on stable work_packet_id"
    );
    assert_eq!(
        projection.task_board_id.as_deref(),
        Some("tb-software-delivery-v1"),
        "projection must carry the linked task_board_id by stable id"
    );
    assert_eq!(
        projection.workflow_run_id.as_deref(),
        Some("workflow-run-1234"),
        "projection must carry the workflow_run_id stable id"
    );
    assert_eq!(
        projection.model_session_id.as_deref(),
        Some("session-coder-1"),
        "projection must carry the model_session_id stable id"
    );

    // ── Authoritative fields come from the canonical runtime summary ─────────
    assert_eq!(
        projection.workflow_state_family, canonical.workflow_state_family,
        "projection workflow_state_family must equal canonical (Validation), \
         not the stale mirror's claimed Done"
    );
    assert_ne!(
        projection.workflow_state_family, board_entry.workflow_state_family,
        "stale board mirror must NOT override canonical workflow_state_family"
    );
    assert_eq!(
        projection.queue_reason_code, canonical.queue_reason_code,
        "projection queue_reason_code must equal canonical (ValidationWait)"
    );
    assert_ne!(
        projection.queue_reason_code, board_entry.queue_reason_code,
        "stale board mirror must NOT override canonical queue_reason_code"
    );
    assert_eq!(
        projection.allowed_action_ids, canonical.allowed_action_ids,
        "projection allowed_action_ids must equal canonical (empty)"
    );
    assert!(
        !projection
            .allowed_action_ids
            .contains(&"mirror_only_action".to_string()),
        "mirror-only action ids must NOT leak into the projection surface"
    );
    assert_eq!(
        projection.status, canonical.status,
        "projection status must equal canonical, not the board's 'Done (mirror)' text"
    );
    assert_eq!(
        projection.title_or_objective, canonical.title_or_objective,
        "projection title_or_objective must equal canonical"
    );
    assert_eq!(
        projection.blockers, canonical.blockers,
        "projection blockers must equal canonical (validator_kickoff_pending), \
         not derived from mailbox chronology"
    );
    assert_eq!(
        projection.next_action, canonical.next_action,
        "projection next_action must equal canonical"
    );
    assert_eq!(
        projection.authority_refs, canonical.authority_refs,
        "projection authority_refs must come from canonical runtime artifact paths"
    );
    assert_eq!(
        projection.evidence_refs, canonical.evidence_refs,
        "projection evidence_refs must come from canonical evidence paths"
    );
    assert!(
        projection.authority_refs.iter().all(|r| r.starts_with(".handshake/gov/")),
        "projection authority_refs MUST point at runtime governance artifacts, \
         never at advisory /.GOV mirrors"
    );

    // ── Advisory display state survives but stays explicitly advisory ────────
    assert_eq!(
        projection.advisory_mirror_state,
        MirrorSyncState::Stale,
        "advisory_mirror_state passes through the board's Stale value"
    );
    assert!(
        mirror_state_is_advisory_only(projection.advisory_mirror_state),
        "Stale mirror_state must classify as advisory-only per v02.181"
    );
    assert_eq!(
        projection.advisory_board_lane_id.as_deref(),
        Some("done"),
        "advisory_board_lane_id passes through the board's lane string"
    );
    assert_eq!(
        projection.advisory_board_status_text.as_deref(),
        Some("Done (mirror)"),
        "advisory_board_status_text passes through the board's status text"
    );
    assert_eq!(
        projection.advisory_role_mailbox_thread_ids,
        vec![
            "thread-software-delivery-coder-intent".to_string(),
            "thread-software-delivery-validator-kickoff".to_string(),
        ],
        "advisory mailbox thread ids must be sorted, deduped, and exclude empty entries"
    );

    // ── Discipline guard: validator helper passes for canonical-aligned data ─
    let validation: StructuredCollaborationValidationResult =
        validate_software_delivery_projection_surface_authority(&projection, &canonical);
    assert!(
        validation.ok,
        "projection must pass canonical-authority validation; issues: {:?}",
        validation.issues
    );

    // ── Discipline guard: validator helper FAILS when mirror text is forced ─
    // Construct a tampered projection that copies the board's mirror values
    // into the authoritative fields. The validator must reject it so a board
    // mirror or mailbox chronology can never be silently promoted to authority.
    let tampered = SoftwareDeliveryProjectionSurfaceV1 {
        workflow_state_family: board_entry.workflow_state_family,
        queue_reason_code: board_entry.queue_reason_code,
        allowed_action_ids: board_entry.allowed_action_ids.clone(),
        status: board_entry.status.clone(),
        ..projection.clone()
    };
    let tampered_validation =
        validate_software_delivery_projection_surface_authority(&tampered, &canonical);
    assert!(
        !tampered_validation.ok,
        "validator MUST reject a projection whose authoritative fields were \
         overridden by board-mirror display state"
    );
    assert!(
        tampered_validation.issues.iter().any(|i| matches!(
            i.code,
            StructuredCollaborationValidationCode::SummaryJoinMismatch
        )),
        "validator must surface SummaryJoinMismatch issues for mirror-driven overrides"
    );
    let tampered_fields: std::collections::HashSet<&str> = tampered_validation
        .issues
        .iter()
        .map(|i| i.field.as_str())
        .collect();
    for required in [
        "workflow_state_family",
        "queue_reason_code",
        "allowed_action_ids",
        "status",
    ] {
        assert!(
            tampered_fields.contains(required),
            "validator must flag tampered field {required}"
        );
    }

    // ── Discipline guard: non-software-delivery profiles are refused ────────
    let mut non_sd = canonical.clone();
    non_sd.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        derive_software_delivery_projection_surface(&non_sd, None, None, None, &[]).is_none(),
        "projection surface MUST refuse non-software-delivery canonical summaries"
    );

    // ── Discipline guard: misaligned board entry breaks the stable-id join ──
    let mut misaligned = stale_advisory_board_entry("WP-OTHER");
    misaligned.work_packet_id = "WP-OTHER-WRONG".to_string();
    assert!(
        derive_software_delivery_projection_surface(
            &canonical,
            None,
            None,
            Some(&misaligned),
            &[],
        )
        .is_none(),
        "projection surface MUST refuse a board entry whose work_packet_id \
         does not match the canonical record_id (stable-id join broken)"
    );

    // ── Round-trip: projection survives serde without losing discipline ────
    let serialized = serde_json::to_string(&projection).expect("serialize projection");
    let round_tripped: SoftwareDeliveryProjectionSurfaceV1 =
        serde_json::from_str(&serialized).expect("deserialize projection");
    assert_eq!(
        round_tripped, projection,
        "projection surface must serde round-trip without lossy mirror promotion"
    );
    let round_validation =
        validate_software_delivery_projection_surface_authority(&round_tripped, &canonical);
    assert!(
        round_validation.ok,
        "projection surface must remain canonical-aligned after round-trip"
    );
}

// ── v02.181 governed-action preview payload tripwire ────────────────────────
// Packet contract row "Governed action preview payload": every actionable
// surface (DCC quick actions, Task Board row actions, Role Mailbox escalation
// controls) MUST inspect a preview carrying action_request_id, target record
// refs, eligibility, blockers, and evidence refs BEFORE any mutation, and
// constructing the preview MUST NOT mutate canonical runtime state.

#[test]
fn projection_surface_previews_governed_action_before_mutation() {
    // Arrange canonical with registered governed actions for the Validation
    // family AND unresolved blockers, so previews must report
    // IneligibleBlocked (the canonical-authority verdict UI surfaces inspect
    // before allowing operator escalation).
    let mut canonical = software_delivery_canonical_summary();
    canonical.allowed_action_ids = vec!["validate".to_string(), "repair".to_string()];

    // Snapshot canonical bytes before any preview construction so we can
    // prove the helper is read-only at the canonical-input boundary.
    let canonical_before = serde_json::to_vec(&canonical).expect("serialize canonical");

    let projection = derive_software_delivery_projection_surface(
        &canonical,
        Some("workflow-run-1234"),
        Some("session-coder-1"),
        None,
        &[],
    )
    .expect("software-delivery canonical must derive a projection surface");

    // Discipline guard #1: deriving the projection surface (and its embedded
    // governed-action previews) MUST NOT mutate canonical bytes.
    let canonical_after =
        serde_json::to_vec(&canonical).expect("serialize canonical after derive");
    assert_eq!(
        canonical_before, canonical_after,
        "deriving the software-delivery projection surface MUST NOT mutate \
         canonical bytes (preview construction is read-only)"
    );

    // Discipline guard #2: previews exist and match canonical's allowed
    // action set exactly (no inventing, no dropping, no dedup loss).
    assert_eq!(
        projection.governed_action_previews.len(),
        canonical.allowed_action_ids.len(),
        "exactly one preview per canonical allowed action"
    );
    let canonical_action_ids: std::collections::HashSet<String> =
        canonical.allowed_action_ids.iter().cloned().collect();
    let preview_action_ids: std::collections::HashSet<String> = projection
        .governed_action_previews
        .iter()
        .map(|p| p.action_id.clone())
        .collect();
    assert_eq!(
        preview_action_ids, canonical_action_ids,
        "preview action_ids MUST equal canonical.allowed_action_ids exactly"
    );

    // Discipline guard #3: per-preview field discipline. The packet contract
    // row "Governed action preview payload" requires action_request_id, target
    // record refs, eligibility, blockers, and evidence refs.
    for preview in &projection.governed_action_previews {
        assert_eq!(
            preview.schema_id, GOVERNED_ACTION_PREVIEW_SCHEMA_ID_V1,
            "preview must declare the v02.181 governed_action_preview schema id"
        );
        assert_eq!(
            preview.record_kind, GOVERNED_ACTION_PREVIEW_RECORD_KIND,
            "preview must declare the governed_action_preview record kind"
        );
        // action_request_id: required, non-empty, derived from canonical.
        assert!(
            !preview.action_request_id.is_empty(),
            "preview must carry a non-empty action_request_id"
        );
        assert!(
            preview.action_request_id.contains(&canonical.record_id),
            "action_request_id MUST be derived from canonical.record_id so \
             repeated preview construction across surfaces yields identical ids"
        );
        assert!(
            preview.action_request_id.contains(&preview.action_id),
            "action_request_id MUST be derived from action_id so previews for \
             different actions never collide on the same correlation id"
        );
        // target_record_refs: lifted from canonical authority_refs verbatim.
        assert_eq!(
            preview.target_record_refs, canonical.authority_refs,
            "target_record_refs MUST lift verbatim from canonical.authority_refs"
        );
        assert!(
            preview
                .target_record_refs
                .iter()
                .all(|r| r.starts_with(".handshake/gov/")),
            "target_record_refs MUST point at runtime governance artifacts, \
             never at advisory mirrors"
        );
        // evidence_refs: lifted from canonical evidence_refs verbatim.
        assert_eq!(
            preview.evidence_refs, canonical.evidence_refs,
            "preview evidence_refs MUST lift verbatim from canonical.evidence_refs"
        );
        // blockers: lifted from canonical blockers verbatim.
        assert_eq!(
            preview.blockers, canonical.blockers,
            "preview blockers MUST equal canonical blockers"
        );
        // workflow_state_family / work_packet_id: stable-id join with canonical.
        assert_eq!(
            preview.workflow_state_family, canonical.workflow_state_family,
            "preview workflow_state_family MUST equal canonical workflow_state_family"
        );
        assert_eq!(
            preview.work_packet_id, canonical.record_id,
            "preview work_packet_id MUST equal canonical.record_id (stable-id join)"
        );
        assert_eq!(
            preview.workflow_run_id.as_deref(),
            Some("workflow-run-1234"),
            "preview workflow_run_id MUST come from caller stable id"
        );
        // Eligibility: canonical has unresolved blockers, so previews are
        // IneligibleBlocked regardless of family allowance.
        assert_eq!(
            preview.eligibility,
            GovernedActionEligibility::IneligibleBlocked,
            "preview eligibility MUST be IneligibleBlocked when canonical \
             reports unresolved blockers"
        );
        // action_id: registered for the canonical family.
        assert!(
            is_governed_action_id_allowed_for_workflow_family(
                canonical.workflow_state_family,
                &preview.action_id
            ),
            "preview action_id MUST be registered in governed_action_ids_for_family \
             for the canonical workflow_state_family ({:?})",
            canonical.workflow_state_family
        );
    }

    // Discipline guard #4: clear blockers, eligibility flips to Eligible
    // (covering the policy/approval/evidence positive path).
    let mut unblocked = canonical.clone();
    unblocked.blockers.clear();
    let unblocked_projection = derive_software_delivery_projection_surface(
        &unblocked, None, None, None, &[],
    )
    .expect("derive projection for unblocked canonical");
    assert_eq!(
        unblocked_projection.governed_action_previews.len(),
        canonical.allowed_action_ids.len(),
        "unblocked canonical must still produce one preview per allowed action"
    );
    for preview in &unblocked_projection.governed_action_previews {
        assert_eq!(
            preview.eligibility,
            GovernedActionEligibility::Eligible,
            "preview eligibility MUST be Eligible when canonical has no blockers"
        );
        assert!(
            preview.blockers.is_empty(),
            "preview blockers MUST be empty when canonical has no blockers"
        );
    }

    // Discipline guard #5: action_id outside canonical family yields
    // IneligibleOutOfFamily. Preview is constructible (so UI can display
    // the rejection reason), but UI surfaces MUST refuse to escalate.
    let stray = derive_governed_action_preview(&unblocked, None, "approve").expect(
        "approve is registered (Approval family), so a preview must construct \
         even for a non-Validation canonical",
    );
    assert_eq!(
        stray.eligibility,
        GovernedActionEligibility::IneligibleOutOfFamily,
        "approve belongs to Approval family, NOT Validation; preview MUST \
         report IneligibleOutOfFamily so UI surfaces refuse escalation"
    );
    assert_eq!(stray.action_id, "approve");
    assert_eq!(
        stray.workflow_state_family, unblocked.workflow_state_family,
        "preview must report the canonical workflow_state_family it was derived under"
    );

    // Discipline guard #6: unregistered action_id refuses to materialize.
    // Previews must never invent actions outside the canonical registry.
    assert!(
        derive_governed_action_preview(&unblocked, None, "fabricated_action").is_none(),
        "preview MUST refuse to materialize an unregistered action_id"
    );

    // Discipline guard #7: non-software-delivery canonical produces no preview
    // and no projection surface. Discipline is profile-scoped end-to-end.
    let mut research = unblocked.clone();
    research.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        derive_governed_action_preview(&research, None, "validate").is_none(),
        "preview is software_delivery only; non-SD canonical must yield None"
    );
    assert!(
        derive_software_delivery_projection_surface(&research, None, None, None, &[]).is_none(),
        "non-software-delivery canonical produces no projection surface, so no \
         governed-action previews are emitted"
    );

    // Discipline guard #8: action_request_id stays deterministic across
    // repeated derives so the correlation id remains stable for callers that
    // escalate the preview into a real governed action request.
    let projection_again = derive_software_delivery_projection_surface(
        &canonical,
        Some("workflow-run-1234"),
        Some("session-coder-1"),
        None,
        &[],
    )
    .expect("re-derive projection surface");
    assert_eq!(
        projection_again.governed_action_previews, projection.governed_action_previews,
        "repeated preview derivation MUST produce identical previews"
    );

    // Discipline guard #9: previews survive serde round-trip without losing
    // any required field, so emitted projection-surface artifacts on disk keep
    // the contract for downstream readers.
    let serialized = serde_json::to_string(&projection).expect("serialize projection");
    let round_tripped: SoftwareDeliveryProjectionSurfaceV1 =
        serde_json::from_str(&serialized).expect("deserialize projection");
    assert_eq!(
        round_tripped.governed_action_previews, projection.governed_action_previews,
        "previews MUST survive serde round-trip without dropping or mutating fields"
    );

    // Discipline guard #10: the alternate producer surface
    // `derive_governed_action_previews` is also read-only and produces the
    // same payload as the projection surface's embedded preview list.
    let direct_previews: Vec<GovernedActionPreviewV1> =
        derive_governed_action_previews(&canonical, Some("workflow-run-1234"));
    let canonical_third =
        serde_json::to_vec(&canonical).expect("serialize canonical post-direct-derive");
    assert_eq!(
        canonical_before, canonical_third,
        "calling derive_governed_action_previews directly MUST also be \
         read-only (no canonical mutation)"
    );
    assert_eq!(
        direct_previews, projection.governed_action_previews,
        "direct preview helper produces the same payload as the projection \
         surface's embedded preview list"
    );

    // Discipline guard #11: tampering with target_record_refs in a stored
    // preview record MUST diverge from canonical authority. This is the UI/
    // mutation-path discipline guard: a forged preview is detectable by
    // comparison to canonical, so a UI cannot silently widen target refs.
    let mut forged = projection.governed_action_previews[0].clone();
    forged
        .target_record_refs
        .push("/.GOV/forged-authority-path.md".to_string());
    assert_ne!(
        forged.target_record_refs, canonical.authority_refs,
        "forged preview target_record_refs must diverge from canonical, \
         giving validators and downstream consumers a detectable signal"
    );
}

// MT-002: Software-delivery overlay runtime truth specialization tripwire

#[test]
fn task_board_software_delivery_projection_cannot_override_runtime_truth() {
    let canonical = software_delivery_canonical_summary();
    let stale_board = stale_advisory_board_entry(&canonical.record_id);

    let result = validate_software_delivery_task_board_projection_against_canonical(
        &stale_board,
        &canonical,
    );

    assert!(
        !result.ok,
        "stale software-delivery board projection must fail validation against canonical"
    );

    let issue_fields: Vec<&str> = result
        .issues
        .iter()
        .map(|issue| issue.field.as_str())
        .collect();

    // Authority-carrying fields disagree with canonical and MUST be flagged.
    assert!(
        issue_fields.contains(&"workflow_state_family"),
        "stale board's Done family must not override canonical Validation \
         (got issues={issue_fields:?})"
    );
    assert!(
        issue_fields.contains(&"queue_reason_code"),
        "stale board's ReadyForHuman queue must not override canonical ValidationWait \
         (got issues={issue_fields:?})"
    );
    assert!(
        issue_fields.contains(&"allowed_action_ids"),
        "stale board's mirror_only_action allowlist must not override canonical (empty) \
         (got issues={issue_fields:?})"
    );

    // Stable-id join holds: board work_packet_id matches canonical record_id.
    assert!(
        !issue_fields.contains(&"work_packet_id"),
        "stable-id join must not be flagged when board.work_packet_id == canonical.record_id"
    );

    // Advisory display state is allowed to differ -- those fields MUST NOT be
    // surfaced as authority issues by this validator.
    assert!(
        !issue_fields.contains(&"mirror_state"),
        "advisory mirror_state must not be flagged as an authority issue"
    );
    assert!(
        !issue_fields.contains(&"lane_id"),
        "advisory lane_id must not be flagged as an authority issue"
    );
    assert!(
        !issue_fields.contains(&"status"),
        "advisory status text must not be flagged as an authority issue"
    );

    // Aligned board projection (matches canonical authority fields) passes.
    let mut aligned_board = stale_board.clone();
    aligned_board.workflow_state_family = canonical.workflow_state_family;
    aligned_board.queue_reason_code = canonical.queue_reason_code;
    aligned_board.allowed_action_ids = canonical.allowed_action_ids.clone();
    let aligned_result = validate_software_delivery_task_board_projection_against_canonical(
        &aligned_board,
        &canonical,
    );
    assert!(
        aligned_result.ok,
        "aligned board projection must pass authority validation (issues={:?})",
        aligned_result.issues
    );

    // Non-software_delivery board entry must be refused by this validator.
    let mut research_board = stale_board.clone();
    research_board.project_profile_kind = ProjectProfileKind::Research;
    research_board.workflow_state_family = canonical.workflow_state_family;
    research_board.queue_reason_code = canonical.queue_reason_code;
    research_board.allowed_action_ids = canonical.allowed_action_ids.clone();
    let research_result = validate_software_delivery_task_board_projection_against_canonical(
        &research_board,
        &canonical,
    );
    assert!(
        !research_result.ok,
        "non-software_delivery board entry must be refused by the software_delivery validator"
    );
    let research_issue_fields: Vec<&str> = research_result
        .issues
        .iter()
        .map(|issue| issue.field.as_str())
        .collect();
    assert!(
        research_issue_fields.contains(&"project_profile_kind"),
        "research-profile board entry must produce a project_profile_kind issue"
    );
}

#[test]
fn task_board_software_delivery_production_validation_rejects_stale_runtime_truth() {
    // Production-path test: the integration helper called by the
    // task-board materialization loop in `workflows.rs` MUST reject a stale
    // software_delivery board row. This complements the direct
    // `validate_software_delivery_task_board_projection_against_canonical`
    // unit by exercising the wired path that production materialization uses.
    let canonical = software_delivery_canonical_summary();
    let stale = stale_advisory_board_entry(&canonical.record_id);

    let result = enforce_software_delivery_task_board_projection_authority(
        &stale,
        &canonical.record_id,
        canonical.workflow_state_family,
        canonical.queue_reason_code,
        canonical.allowed_action_ids.clone(),
    );

    assert!(
        !result.ok,
        "production helper must reject stale software_delivery board projection \
         (issues={:?})",
        result.issues
    );
    let fields: Vec<&str> = result.issues.iter().map(|i| i.field.as_str()).collect();
    assert!(
        fields.contains(&"workflow_state_family"),
        "production helper must flag workflow_state_family drift (got {fields:?})"
    );
    assert!(
        fields.contains(&"queue_reason_code"),
        "production helper must flag queue_reason_code drift (got {fields:?})"
    );
    assert!(
        fields.contains(&"allowed_action_ids"),
        "production helper must flag allowed_action_ids drift (got {fields:?})"
    );

    // Aligned software_delivery entry passes through the production helper.
    let mut aligned = stale.clone();
    aligned.workflow_state_family = canonical.workflow_state_family;
    aligned.queue_reason_code = canonical.queue_reason_code;
    aligned.allowed_action_ids = canonical.allowed_action_ids.clone();
    let aligned_result = enforce_software_delivery_task_board_projection_authority(
        &aligned,
        &canonical.record_id,
        canonical.workflow_state_family,
        canonical.queue_reason_code,
        canonical.allowed_action_ids.clone(),
    );
    assert!(
        aligned_result.ok,
        "aligned software_delivery board projection must pass production validation \
         (issues={:?})",
        aligned_result.issues
    );

    // Non-software_delivery profile is a no-op in the production helper:
    // even when the entry's authority fields disagree, the software_delivery-
    // specific check yields no issues (the profile-generic
    // `validate_task_board_entry_authoritative_fields` is the gate for those).
    let mut research_entry = stale.clone();
    research_entry.project_profile_kind = ProjectProfileKind::Research;
    let research_result = enforce_software_delivery_task_board_projection_authority(
        &research_entry,
        &canonical.record_id,
        canonical.workflow_state_family,
        canonical.queue_reason_code,
        canonical.allowed_action_ids.clone(),
    );
    assert!(
        research_result.ok,
        "production helper must be a no-op for non-software_delivery entries"
    );
}

// MT-003: Software-delivery closeout derivation tripwire

fn closeout_runtime_paths() -> (tempfile::TempDir, RuntimeGovernancePaths) {
    let dir = tempfile::tempdir().expect("tempdir for closeout runtime paths");
    let paths = RuntimeGovernancePaths::from_workspace_root(dir.path().to_path_buf())
        .expect("runtime paths from tempdir");
    (dir, paths)
}

fn canonical_software_delivery_summary_for_closeout(
    runtime_paths: &RuntimeGovernancePaths,
    wp_id: &str,
) -> StructuredCollaborationSummaryV1 {
    StructuredCollaborationSummaryV1 {
        schema_id: "hsk.structured_collaboration_summary@1".to_string(),
        schema_version: "1".to_string(),
        record_id: wp_id.to_string(),
        record_kind: "work_packet".to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: "2026-04-27T18:00:00Z".to_string(),
        mirror_state: MirrorSyncState::CanonicalOnly,
        authority_refs: vec![runtime_paths.work_packet_packet_display(wp_id)],
        evidence_refs: vec![runtime_paths.validator_gate_record_display(wp_id)],
        mirror_contract: None,
        workflow_state_family: WorkflowStateFamily::Validation,
        queue_reason_code: WorkflowQueueReasonCode::ValidationWait,
        allowed_action_ids: Vec::new(),
        transition_rule_ids: transition_rule_ids_for_family(WorkflowStateFamily::Validation),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::ValidationWait,
        ),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            WorkflowStateFamily::Validation,
        ),
        status: "validation: awaiting validator".to_string(),
        title_or_objective: "MT-003 closeout derivation tripwire".to_string(),
        blockers: vec!["validator_kickoff_pending".to_string()],
        next_action: Some("validate".to_string()),
        summary_ref: Some(runtime_paths.work_packet_summary_display(wp_id)),
    }
}

#[test]
fn closeout_projection_requires_gate_evidence_and_owner_truth() {
    let (_tmp, runtime_paths) = closeout_runtime_paths();
    let wp_id = "WP-1-Software-Delivery-Test";
    let canonical = canonical_software_delivery_summary_for_closeout(&runtime_paths, wp_id);

    // Canonical baseline carries CANONICAL validator-gate evidence and
    // CANONICAL packet.json owner authority -> derivation succeeds and lifts
    // canonical fields verbatim. Checkpoint id is propagated and reduced to
    // its canonical record ref + id pair. Governed-action resolution refs
    // are sorted + deduped.
    let governed_actions = vec![
        "governed_action_resolution:approve:1".to_string(),
        "governed_action_resolution:validate:1".to_string(),
        "".to_string(), // empty must be filtered
        "governed_action_resolution:approve:1".to_string(), // duplicate must be deduped
    ];
    let posture = derive_software_delivery_closeout_posture(
        &canonical,
        &runtime_paths,
        Some("checkpoint-mt003"),
        &governed_actions,
    )
    .expect(
        "canonical with canonical validator_gates evidence and canonical packet.json owner \
         authority must derive a closeout posture",
    );
    assert_eq!(posture.work_packet_id, canonical.record_id);
    assert_eq!(posture.project_profile_kind, ProjectProfileKind::SoftwareDelivery);
    assert_eq!(
        posture.gate_record_ref,
        runtime_paths.validator_gate_record_display(wp_id),
        "gate_record_ref must equal the canonical runtime path"
    );
    assert_eq!(
        posture.owner_authority_ref,
        runtime_paths.work_packet_packet_display(wp_id),
        "owner_authority_ref must equal the canonical runtime path"
    );
    assert_eq!(
        posture.checkpoint_record_ref.as_deref(),
        Some(runtime_paths.checkpoint_record_display("checkpoint-mt003")).as_deref(),
        "checkpoint_record_ref must come from RuntimeGovernancePaths::checkpoint_record_display"
    );
    assert_eq!(
        posture.checkpoint_id.as_deref(),
        Some("checkpoint-mt003"),
        "checkpoint_id must mirror the trailing path segment"
    );
    assert_eq!(
        posture.governed_action_resolution_refs,
        vec![
            "governed_action_resolution:approve:1".to_string(),
            "governed_action_resolution:validate:1".to_string(),
        ],
        "governed_action_resolution_refs must be sorted + deduped + filtered"
    );
    assert_eq!(posture.evidence_refs, canonical.evidence_refs);
    assert_eq!(posture.authority_refs, canonical.authority_refs);
    assert_eq!(posture.unresolved_blockers, canonical.blockers);
    assert_eq!(posture.workflow_state_family, canonical.workflow_state_family);
    assert_eq!(posture.queue_reason_code, canonical.queue_reason_code);
    assert_eq!(posture.updated_at, canonical.updated_at);
    assert!(
        matches!(posture.closeout_state, SoftwareDeliveryCloseoutState::PendingGate),
        "Validation family must classify as PendingGate (got {:?})",
        posture.closeout_state
    );

    // BLOCKER 3: spoofed validator-gate evidence ref must be rejected.
    // A substring-only match such as `.handshake/gov/work_packets/WP-X/notes/validator_gates/foo.json`
    // contains "validator_gates/" but is NOT under the canonical
    // validator_gates dir, so derivation MUST refuse.
    let mut spoofed_gate = canonical.clone();
    spoofed_gate.evidence_refs = vec![format!(
        "{}{wp_id}/notes/validator_gates/spoof.json",
        runtime_paths.work_packets_dir_display()
    )];
    assert!(
        derive_software_delivery_closeout_posture(
            &spoofed_gate,
            &runtime_paths,
            None,
            &[],
        )
        .is_none(),
        "spoofed substring 'validator_gates/' must NOT satisfy canonical gate detection"
    );
    let spoofed_gate_validation =
        validate_software_delivery_closeout_canonical_truth(&spoofed_gate, &runtime_paths);
    assert!(
        !spoofed_gate_validation.ok,
        "spoofed substring evidence ref must trip the closeout truth validator"
    );

    // BLOCKER 3: spoofed owner authority ref must be rejected.
    let mut spoofed_owner = canonical.clone();
    spoofed_owner.authority_refs = vec![format!(
        "{}fake/path/work_packets/{wp_id}/packet.json",
        runtime_paths.governance_root_display()
    )];
    assert!(
        derive_software_delivery_closeout_posture(
            &spoofed_owner,
            &runtime_paths,
            None,
            &[],
        )
        .is_none(),
        "spoofed substring '/work_packets/.../packet.json' must NOT satisfy canonical owner detection"
    );

    // FINDING 1 (stable-id binding): a foreign-WP gate ref that is itself
    // a CANONICAL validator_gates path but for a DIFFERENT WP id MUST be
    // rejected. The only legal gate ref binds to canonical.record_id.
    let mut foreign_gate = canonical.clone();
    foreign_gate.evidence_refs =
        vec![runtime_paths.validator_gate_record_display("WP-OTHER")];
    assert!(
        derive_software_delivery_closeout_posture(
            &foreign_gate,
            &runtime_paths,
            None,
            &[],
        )
        .is_none(),
        "foreign-WP canonical validator-gate ref must NOT satisfy stable-id binding"
    );
    let foreign_gate_validation =
        validate_software_delivery_closeout_canonical_truth(&foreign_gate, &runtime_paths);
    assert!(
        !foreign_gate_validation.ok,
        "foreign-WP gate ref must trip the closeout truth validator"
    );
    assert!(
        foreign_gate_validation
            .issues
            .iter()
            .any(|issue| issue.field == "evidence_refs"),
        "foreign-WP gate ref must surface evidence_refs issue (got {:?})",
        foreign_gate_validation.issues
    );

    // FINDING 1 (stable-id binding): a foreign-WP owner ref that is itself
    // a CANONICAL work_packets/<other>/packet.json path MUST be rejected.
    let mut foreign_owner = canonical.clone();
    foreign_owner.authority_refs =
        vec![runtime_paths.work_packet_packet_display("WP-OTHER")];
    assert!(
        derive_software_delivery_closeout_posture(
            &foreign_owner,
            &runtime_paths,
            None,
            &[],
        )
        .is_none(),
        "foreign-WP canonical owner ref must NOT satisfy stable-id binding"
    );
    let foreign_owner_validation =
        validate_software_delivery_closeout_canonical_truth(&foreign_owner, &runtime_paths);
    assert!(
        !foreign_owner_validation.ok,
        "foreign-WP owner ref must trip the closeout truth validator"
    );
    assert!(
        foreign_owner_validation
            .issues
            .iter()
            .any(|issue| issue.field == "authority_refs"),
        "foreign-WP owner ref must surface authority_refs issue (got {:?})",
        foreign_owner_validation.issues
    );

    // Without ANY validator-gate evidence -> refuse.
    let mut no_gate = canonical.clone();
    no_gate.evidence_refs = vec![runtime_paths.work_packet_summary_display(wp_id)];
    assert!(
        derive_software_delivery_closeout_posture(&no_gate, &runtime_paths, None, &[]).is_none(),
        "closeout derivation must refuse a canonical missing validator-gate evidence"
    );

    // Without canonical packet.json owner authority -> refuse.
    let mut no_owner = canonical.clone();
    no_owner.authority_refs = vec![runtime_paths.validator_gate_record_display(wp_id)];
    assert!(
        derive_software_delivery_closeout_posture(&no_owner, &runtime_paths, None, &[]).is_none(),
        "closeout derivation must refuse a canonical missing packet.json owner authority"
    );

    // Non-software_delivery profile -> refuse.
    let mut research = canonical.clone();
    research.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        derive_software_delivery_closeout_posture(&research, &runtime_paths, None, &[]).is_none(),
        "closeout derivation must refuse non-software_delivery profiles"
    );

    // Spoofed checkpoint candidate -> dropped (no record_ref/id), derivation
    // still succeeds because checkpoint is optional. We probe by handing in a
    // candidate that DOES NOT correspond to a canonical checkpoint path
    // (e.g. contains a slash). RuntimeGovernancePaths::checkpoint_record_display
    // will turn it into <gov_root>/checkpoints/<bad-id-with-slash>.json
    // which is_canonical_checkpoint_record_ref rejects.
    let spoofed_checkpoint = derive_software_delivery_closeout_posture(
        &canonical,
        &runtime_paths,
        Some("nested/checkpoint"),
        &[],
    )
    .expect("derivation succeeds even when checkpoint candidate is non-canonical");
    assert!(
        spoofed_checkpoint.checkpoint_record_ref.is_none(),
        "non-canonical checkpoint candidate must be dropped (got {:?})",
        spoofed_checkpoint.checkpoint_record_ref
    );
    assert!(
        spoofed_checkpoint.checkpoint_id.is_none(),
        "non-canonical checkpoint candidate must clear checkpoint_id"
    );

    // Production-path validator: same canonical must surface concrete issues
    // when truth is missing for a closeout-relevant family. This is the
    // tripwire wired into materialize_structured_collaboration_artifacts AND
    // emit_runtime_structured_work_packet_artifacts.
    let no_gate_validation =
        validate_software_delivery_closeout_canonical_truth(&no_gate, &runtime_paths);
    assert!(
        !no_gate_validation.ok,
        "production-path validator must reject canonical missing gate evidence"
    );
    assert!(
        no_gate_validation
            .issues
            .iter()
            .any(|issue| issue.field == "evidence_refs"),
        "missing-gate validator must surface evidence_refs issue (got {:?})",
        no_gate_validation.issues
    );

    let no_owner_validation =
        validate_software_delivery_closeout_canonical_truth(&no_owner, &runtime_paths);
    assert!(
        !no_owner_validation.ok,
        "production-path validator must reject canonical missing owner authority"
    );
    assert!(
        no_owner_validation
            .issues
            .iter()
            .any(|issue| issue.field == "authority_refs"),
        "missing-owner validator must surface authority_refs issue (got {:?})",
        no_owner_validation.issues
    );

    // For closeout-irrelevant families (Active, Intake, Ready, ...), the
    // production validator does NOT yet require gate/owner truth -- the
    // closeout decision is not legal there yet.
    let mut active = canonical.clone();
    active.workflow_state_family = WorkflowStateFamily::Active;
    active.evidence_refs.clear();
    active.authority_refs.clear();
    let active_validation =
        validate_software_delivery_closeout_canonical_truth(&active, &runtime_paths);
    assert!(
        active_validation.ok,
        "Active-family canonical without truth MUST NOT trip the closeout validator (issues={:?})",
        active_validation.issues
    );

    // Aligned canonical passes the production validator.
    let aligned_validation =
        validate_software_delivery_closeout_canonical_truth(&canonical, &runtime_paths);
    assert!(
        aligned_validation.ok,
        "canonical with both truth refs must pass closeout truth validation (issues={:?})",
        aligned_validation.issues
    );

    // Done with no blockers -> ReadyToClose.
    let mut done = canonical.clone();
    done.workflow_state_family = WorkflowStateFamily::Done;
    done.blockers.clear();
    let done_posture =
        derive_software_delivery_closeout_posture(&done, &runtime_paths, None, &[])
            .expect("Done with truth refs must derive a closeout posture");
    assert!(
        matches!(done_posture.closeout_state, SoftwareDeliveryCloseoutState::ReadyToClose),
        "Done + no blockers must classify ReadyToClose (got {:?})",
        done_posture.closeout_state
    );

    // Done with blockers -> PendingBlockers.
    let mut blocked = canonical.clone();
    blocked.workflow_state_family = WorkflowStateFamily::Done;
    let blocked_posture =
        derive_software_delivery_closeout_posture(&blocked, &runtime_paths, None, &[])
            .expect("Done with truth refs must derive a closeout posture even with blockers");
    assert!(
        matches!(
            blocked_posture.closeout_state,
            SoftwareDeliveryCloseoutState::PendingBlockers
        ),
        "Done + blockers must classify PendingBlockers (got {:?})",
        blocked_posture.closeout_state
    );

    // Active family with both truth refs -> NotEligible.
    let mut active_full = canonical.clone();
    active_full.workflow_state_family = WorkflowStateFamily::Active;
    let active_posture =
        derive_software_delivery_closeout_posture(&active_full, &runtime_paths, None, &[])
            .expect("Active with truth refs must still derive (state=NotEligible)");
    assert!(
        matches!(active_posture.closeout_state, SoftwareDeliveryCloseoutState::NotEligible),
        "Active family must classify NotEligible (got {:?})",
        active_posture.closeout_state
    );
}

#[test]
fn closeout_posture_artifact_lifecycle_clears_stale_state_through_production_helper() {
    // Production-path test for MT-003 closeout posture lifecycle: the same
    // helper that emit_runtime_structured_work_packet_artifacts calls must
    // also CLEAR a previously emitted closeout_posture.json when the
    // canonical authority no longer supports derivation. Without this, a
    // software_delivery WP that transitions out of closeout-relevant state
    // would leave a stale posture artifact on the runtime display surface.
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace_root = dir.path().to_path_buf();
    let runtime_paths = RuntimeGovernancePaths::from_workspace_root(workspace_root.clone())
        .expect("runtime paths");
    let wp_id = "WP-1-Software-Delivery-Lifecycle-Test";

    // Step 1: emit a posture from a fully-truthful canonical summary.
    let canonical = canonical_software_delivery_summary_for_closeout(&runtime_paths, wp_id);
    let canonical_value = serde_json::to_value(&canonical).expect("canonical to value");
    let posture_path = runtime_paths.work_packet_closeout_posture_path(wp_id);

    // Make sure the work-packet directory exists so write_json_atomic
    // succeeds (production code creates it; the lifecycle helper assumes
    // a normal materialization environment).
    std::fs::create_dir_all(posture_path.parent().expect("posture parent dir"))
        .expect("create work_packet dir");

    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("emit posture should succeed for fully-truthful canonical");
    assert!(
        posture_path.exists(),
        "closeout_posture.json must exist after lifecycle emit (path={})",
        posture_path.display()
    );
    let written = std::fs::read_to_string(&posture_path).expect("read posture json");
    assert!(
        written.contains("software_delivery_closeout_posture"),
        "emitted posture json must declare the v02.181 record_kind (got {})",
        written
    );

    // Step 2: transition to a state where derivation no longer supports the
    // posture. Strip evidence_refs so the canonical loses gate evidence
    // (still software_delivery, still closeout-relevant). The lifecycle
    // helper MUST remove the existing artifact rather than leaving it
    // stale on the runtime display surface.
    let mut without_gate = canonical.clone();
    without_gate.evidence_refs.clear();
    let without_gate_value =
        serde_json::to_value(&without_gate).expect("without-gate canonical to value");
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &without_gate_value,
    )
    .expect("clearing lifecycle should succeed even when derivation returns None");
    assert!(
        !posture_path.exists(),
        "closeout_posture.json must be REMOVED after canonical loses gate evidence \
         (stale artifact still present at {})",
        posture_path.display()
    );

    // Step 3: re-emit, then transition project_profile_kind away from
    // software_delivery. The helper MUST again remove the artifact.
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("re-emit posture should succeed");
    assert!(
        posture_path.exists(),
        "re-emit should restore closeout_posture.json"
    );

    let mut research = canonical.clone();
    research.project_profile_kind = ProjectProfileKind::Research;
    let research_value = serde_json::to_value(&research).expect("research canonical to value");
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &research_value,
    )
    .expect("clearing lifecycle should succeed for non-software_delivery profile");
    assert!(
        !posture_path.exists(),
        "closeout_posture.json must be REMOVED when project_profile_kind transitions \
         away from software_delivery"
    );

    // Step 4: foreign-WP gate ref must trigger artifact removal
    // (same-record stable-id binding violation; derive returns None).
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("re-emit posture for foreign-gate test");
    assert!(posture_path.exists(), "re-emit must restore the artifact");

    let mut foreign_gate = canonical.clone();
    foreign_gate.evidence_refs =
        vec![runtime_paths.validator_gate_record_display("WP-OTHER")];
    let foreign_gate_value =
        serde_json::to_value(&foreign_gate).expect("foreign-gate canonical to value");
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &foreign_gate_value,
    )
    .expect("clearing lifecycle must succeed when canonical fails stable-id binding");
    assert!(
        !posture_path.exists(),
        "closeout_posture.json must be REMOVED when canonical fails stable-id binding"
    );

    // Step 5: when no posture exists and derivation is not supported, the
    // lifecycle helper is idempotent (no error, no file).
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &foreign_gate_value,
    )
    .expect("idempotent clear must not error when no artifact exists");
    assert!(!posture_path.exists());
}

#[test]
fn closeout_posture_artifact_lifecycle_runs_before_validation_gate_in_production_wrapper() {
    // Production-path regression test for MT-003: even when the canonical
    // summary FAILS the closeout truth validation, the production wrapper
    // (`finalize_runtime_structured_work_packet_writes`, which is what
    // `emit_runtime_structured_work_packet_artifacts` calls) MUST clear
    // any pre-emitted closeout_posture.json before propagating the
    // validation error. Otherwise a software_delivery WP that lost
    // canonical record-id-bound gate/owner truth would leave a stale
    // closeout posture on disk after `execute_locus_work_packet_operation`
    // already committed the DB mutation.
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace_root = dir.path().to_path_buf();
    let runtime_paths = RuntimeGovernancePaths::from_workspace_root(workspace_root.clone())
        .expect("runtime paths");
    let wp_id = "WP-1-Software-Delivery-Production-Failure";

    let canonical = canonical_software_delivery_summary_for_closeout(&runtime_paths, wp_id);
    let canonical_value = serde_json::to_value(&canonical).expect("canonical to value");
    let posture_path = runtime_paths.work_packet_closeout_posture_path(wp_id);
    let packet_path = runtime_paths.work_packet_packet_path(wp_id);
    let summary_path = runtime_paths.work_packet_summary_path(wp_id);
    std::fs::create_dir_all(posture_path.parent().expect("posture parent dir"))
        .expect("create work_packet dir");

    // Pre-emit a posture from a fully-truthful canonical (simulate prior
    // valid state on disk).
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("pre-emit posture should succeed");
    assert!(posture_path.exists(), "pre-emit must produce closeout_posture.json");

    // Build a failing-validation scenario: missing-gate canonical (still
    // closeout-relevant Validation family). validate_software_delivery_closeout_canonical_truth
    // produces a non-ok validation; derive returns None.
    let mut without_gate = canonical.clone();
    without_gate.evidence_refs.clear();
    let without_gate_value =
        serde_json::to_value(&without_gate).expect("without-gate canonical to value");
    let validation = validate_software_delivery_closeout_canonical_truth(
        &without_gate,
        &runtime_paths,
    );
    assert!(
        !validation.ok,
        "missing-gate canonical must fail closeout truth validation"
    );

    let detail_value = serde_json::json!({
        "schema_id": "hsk.tracked_work_packet@1",
        "schema_version": "1",
        "record_id": wp_id,
        "record_kind": "work_packet",
        "project_profile_kind": "software_delivery",
        "updated_at": without_gate.updated_at,
        "mirror_state": "canonical_only",
        "authority_refs": without_gate.authority_refs,
        "evidence_refs": without_gate.evidence_refs,
    });

    // Production wrapper MUST: (a) return Err carrying the validation
    // payload, (b) remove the stale posture artifact, (c) NOT write
    // packet.json or summary.json.
    let result = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &validation,
        &without_gate_value,
        &detail_value,
    );
    assert!(
        result.is_err(),
        "production wrapper must propagate validation error"
    );
    assert!(
        !posture_path.exists(),
        "production wrapper must clear stale closeout_posture.json before returning Err \
         (path={})",
        posture_path.display()
    );
    assert!(
        !packet_path.exists(),
        "production wrapper must NOT write packet.json on validation failure"
    );
    assert!(
        !summary_path.exists(),
        "production wrapper must NOT write summary.json on validation failure"
    );

    // Foreign-WP gate variant: same wrapper failure path, same staleness clear.
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("re-emit for foreign-gate variant");
    assert!(posture_path.exists(), "re-emit must restore the artifact");

    let mut foreign_gate = canonical.clone();
    foreign_gate.evidence_refs =
        vec![runtime_paths.validator_gate_record_display("WP-OTHER")];
    let foreign_gate_value =
        serde_json::to_value(&foreign_gate).expect("foreign-gate canonical to value");
    let foreign_validation = validate_software_delivery_closeout_canonical_truth(
        &foreign_gate,
        &runtime_paths,
    );
    assert!(
        !foreign_validation.ok,
        "foreign-gate canonical must fail closeout truth validation"
    );
    let foreign_result = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &foreign_validation,
        &foreign_gate_value,
        &detail_value,
    );
    assert!(
        foreign_result.is_err(),
        "production wrapper must propagate foreign-gate validation error"
    );
    assert!(
        !posture_path.exists(),
        "production wrapper must clear stale closeout_posture.json on foreign-gate validation failure"
    );

    // Sanity: the wrapper SUCCESS path still writes packet.json + summary.json
    // and re-emits the closeout posture.
    let success_validation =
        validate_software_delivery_closeout_canonical_truth(&canonical, &runtime_paths);
    assert!(success_validation.ok, "fully-truthful canonical must pass validation");
    finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &success_validation,
        &canonical_value,
        &detail_value,
    )
    .expect("success path must succeed");
    assert!(posture_path.exists(), "success path must (re)emit the closeout posture");
    assert!(packet_path.exists(), "success path must write packet.json");
    assert!(summary_path.exists(), "success path must write summary.json");
}

#[test]
fn closeout_posture_not_written_when_validation_fails_for_unrelated_reason() {
    // Production-path regression test for MT-003: if validation fails for
    // an UNRELATED reason (e.g. invalid next_action, authority scope) while
    // canonical gate/owner refs remain valid (so derive would otherwise
    // produce Some), the wrapper MUST NOT write a NEW closeout_posture.json
    // for the invalid/unpublished artifact batch. Any pre-existing posture
    // MUST also be cleared so the invalid batch never leaves an advanced
    // closeout state on the runtime display surface.
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace_root = dir.path().to_path_buf();
    let runtime_paths = RuntimeGovernancePaths::from_workspace_root(workspace_root.clone())
        .expect("runtime paths");
    let wp_id = "WP-1-Software-Delivery-Unrelated-Failure";

    let canonical = canonical_software_delivery_summary_for_closeout(&runtime_paths, wp_id);
    let canonical_value = serde_json::to_value(&canonical).expect("canonical to value");
    let posture_path = runtime_paths.work_packet_closeout_posture_path(wp_id);
    let packet_path = runtime_paths.work_packet_packet_path(wp_id);
    let summary_path = runtime_paths.work_packet_summary_path(wp_id);
    std::fs::create_dir_all(posture_path.parent().expect("posture parent dir"))
        .expect("create work_packet dir");

    // Confirm derive WOULD produce Some for this canonical (gate + owner are valid).
    assert!(
        derive_software_delivery_closeout_posture(&canonical, &runtime_paths, None, &[]).is_some(),
        "test precondition: canonical must support derivation"
    );

    // Construct an UNRELATED validation failure (the closeout truth check
    // would have passed; this issue is on `next_action`, not on
    // evidence_refs / authority_refs).
    let mut unrelated_validation = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    unrelated_validation.push_issue(
        StructuredCollaborationValidationCode::InvalidFieldValue,
        "next_action",
        None,
        Some("totally_unregistered_action".to_string()),
        "next_action must be a registered governed action id",
    );
    assert!(!unrelated_validation.ok);

    let detail_value = json!({
        "schema_id": "hsk.tracked_work_packet@1",
        "schema_version": "1",
        "record_id": wp_id,
        "record_kind": "work_packet",
        "project_profile_kind": "software_delivery",
        "updated_at": canonical.updated_at,
        "mirror_state": "canonical_only",
        "authority_refs": canonical.authority_refs,
        "evidence_refs": canonical.evidence_refs,
    });

    // Case A: NO pre-existing posture. Wrapper must NOT write one.
    let result_no_pre = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &unrelated_validation,
        &canonical_value,
        &detail_value,
    );
    assert!(
        result_no_pre.is_err(),
        "unrelated validation failure must propagate Err"
    );
    assert!(
        !posture_path.exists(),
        "unrelated validation failure MUST NOT write a new closeout_posture.json \
         even when derive would produce Some"
    );
    assert!(!packet_path.exists(), "validation failure MUST NOT write packet.json");
    assert!(!summary_path.exists(), "validation failure MUST NOT write summary.json");

    // Case B: PRE-EXISTING posture from a prior valid batch. Wrapper must
    // CLEAR it so the invalid batch cannot leave an advanced closeout state.
    apply_software_delivery_closeout_posture_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("pre-emit posture for case B");
    assert!(posture_path.exists(), "case B precondition: posture must be present");

    let result_with_pre = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &unrelated_validation,
        &canonical_value,
        &detail_value,
    );
    assert!(
        result_with_pre.is_err(),
        "unrelated validation failure must propagate Err in case B"
    );
    assert!(
        !posture_path.exists(),
        "unrelated validation failure MUST clear pre-existing closeout_posture.json \
         (invalid batch cannot retain advanced closeout state)"
    );

    // Case C: success path on the same canonical re-emits the posture and
    // writes packet/summary, proving the success branch is unaffected.
    let success_validation =
        validate_software_delivery_closeout_canonical_truth(&canonical, &runtime_paths);
    assert!(success_validation.ok);
    finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &success_validation,
        &canonical_value,
        &detail_value,
    )
    .expect("success path with valid canonical must succeed");
    assert!(posture_path.exists());
    assert!(packet_path.exists());
    assert!(summary_path.exists());
}

// ── MT-004: v02.181 software-delivery overlay extension records and
// lifecycle semantics tripwire ─────────────────────────────────────────────

fn overlay_canonical_summary(
    runtime_paths: &RuntimeGovernancePaths,
    wp_id: &str,
) -> StructuredCollaborationSummaryV1 {
    StructuredCollaborationSummaryV1 {
        schema_id: "hsk.structured_collaboration_summary@1".to_string(),
        schema_version: "1".to_string(),
        record_id: wp_id.to_string(),
        record_kind: "work_packet".to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: "2026-04-28T00:00:00Z".to_string(),
        mirror_state: MirrorSyncState::CanonicalOnly,
        authority_refs: vec![runtime_paths.work_packet_packet_display(wp_id)],
        evidence_refs: vec![runtime_paths.validator_gate_record_display(wp_id)],
        mirror_contract: None,
        workflow_state_family: WorkflowStateFamily::Validation,
        queue_reason_code: WorkflowQueueReasonCode::ValidationWait,
        allowed_action_ids: Vec::new(),
        transition_rule_ids: transition_rule_ids_for_family(WorkflowStateFamily::Validation),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(
            WorkflowQueueReasonCode::ValidationWait,
        ),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            WorkflowStateFamily::Validation,
        ),
        status: "validation: awaiting validator".to_string(),
        title_or_objective: "MT-004 overlay extension records tripwire".to_string(),
        blockers: vec!["validator_kickoff_pending".to_string()],
        next_action: Some("validate".to_string()),
        summary_ref: Some(runtime_paths.work_packet_summary_display(wp_id)),
    }
}

fn overlay_claim_lease(wp_id: &str, claim_id: &str) -> GovernanceClaimLeaseRecordV1 {
    GovernanceClaimLeaseRecordV1 {
        schema_id: SOFTWARE_DELIVERY_CLAIM_LEASE_SCHEMA_ID_V1.to_string(),
        schema_version: "1".to_string(),
        record_id: claim_id.to_string(),
        record_kind: SOFTWARE_DELIVERY_CLAIM_LEASE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: wp_id.to_string(),
        workflow_run_id: Some("workflow-run-mt004".to_string()),
        workflow_binding_id: Some("workflow-binding-mt004".to_string()),
        model_session_id: Some("session-coder-mt004".to_string()),
        claim_actor_session_id: "session-coder-mt004".to_string(),
        claim_started_at: "2026-04-28T00:00:00Z".to_string(),
        lease_expires_at: Some("2026-04-28T01:00:00Z".to_string()),
        takeover_policy: SoftwareDeliveryClaimTakeoverPolicy::AutoExpire,
        updated_at: "2026-04-28T00:00:00Z".to_string(),
    }
}

fn overlay_queued_instruction(
    wp_id: &str,
    instruction_id: &str,
    action: SoftwareDeliveryQueuedInstructionAction,
) -> GovernanceQueuedInstructionRecordV1 {
    GovernanceQueuedInstructionRecordV1 {
        schema_id: SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_SCHEMA_ID_V1.to_string(),
        schema_version: "1".to_string(),
        record_id: instruction_id.to_string(),
        record_kind: SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: wp_id.to_string(),
        workflow_run_id: Some("workflow-run-mt004".to_string()),
        workflow_binding_id: Some("workflow-binding-mt004".to_string()),
        target_model_session_id: Some("session-validator-mt004".to_string()),
        instruction_id: instruction_id.to_string(),
        requested_action: action,
        queued_at: "2026-04-28T00:00:00Z".to_string(),
        updated_at: "2026-04-28T00:00:00Z".to_string(),
    }
}

#[test]
fn projection_surface_exposes_claim_and_queued_instruction_ids() {
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let wp_id = "WP-1-Software-Delivery-Test";
    let canonical = overlay_canonical_summary(&runtime_paths, wp_id);

    let claim = overlay_claim_lease(wp_id, "claim-mt004-A");
    // Build queued instructions out of order with a duplicate to exercise the
    // sort/dedup discipline; empty record_ids are rejected upstream and not
    // tested here (governance refuses empty stable ids).
    let instructions = vec![
        overlay_queued_instruction(
            wp_id,
            "instr-002-resume",
            SoftwareDeliveryQueuedInstructionAction::Resume,
        ),
        overlay_queued_instruction(
            wp_id,
            "instr-001-steer",
            SoftwareDeliveryQueuedInstructionAction::SteerNext,
        ),
        overlay_queued_instruction(
            wp_id,
            "instr-001-steer",
            SoftwareDeliveryQueuedInstructionAction::SteerNext,
        ),
    ];
    let gate_posture = SoftwareDeliveryBindingGatePosture {
        has_unresolved_governed_actions: false,
        has_active_validator_gate: true,
        has_closeout_posture: false,
        workflow_failed: false,
        workflow_canceled: false,
        workflow_settled: false,
    };

    let projection = build_software_delivery_projection_surface_with_overlay(
        &canonical,
        Some("workflow-run-mt004"),
        Some("workflow-binding-mt004"),
        Some("session-coder-mt004"),
        None,
        &[],
        Some(&claim),
        &instructions,
        gate_posture,
        &runtime_paths,
    )
    .expect("overlay projection must derive for software_delivery canonical");

    // ── Schema/anchor invariants persist across the overlay extension ────────
    assert_eq!(projection.schema_id, SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_ID_V1);
    assert_eq!(projection.record_kind, SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND);
    assert_eq!(projection.work_packet_id, canonical.record_id);
    assert_eq!(projection.workflow_run_id.as_deref(), Some("workflow-run-mt004"));
    assert_eq!(projection.workflow_binding_id.as_deref(), Some("workflow-binding-mt004"));
    assert_eq!(projection.model_session_id.as_deref(), Some("session-coder-mt004"));

    // ── Claim/lease overlay surfaces by stable id and canonical path ────────
    assert_eq!(
        projection.claim_lease_record_id.as_deref(),
        Some("claim-mt004-A"),
        "projection must expose the canonical claim/lease record id by stable id"
    );
    assert_eq!(
        projection.claim_lease_record_ref.as_deref(),
        Some(runtime_paths.claim_lease_record_display(wp_id, "claim-mt004-A").as_str()),
        "projection must expose the claim/lease record ref as a canonical \
         <gov_root>/claim_leases/<wp_id>/<claim_id>.json path"
    );
    assert!(
        runtime_paths.is_canonical_claim_lease_record_ref(
            projection.claim_lease_record_ref.as_deref().unwrap(),
            wp_id,
            "claim-mt004-A",
        ),
        "claim_lease_record_ref must validate as canonical for the same record_id binding"
    );

    // ── Queued instructions are sorted, deduped, and ref-aligned by id ──────
    assert_eq!(
        projection.queued_instruction_record_ids,
        vec![
            "instr-001-steer".to_string(),
            "instr-002-resume".to_string(),
        ],
        "queued_instruction_record_ids must be sorted by stable id and deduped"
    );
    assert_eq!(
        projection.queued_instruction_record_refs.len(),
        projection.queued_instruction_record_ids.len(),
        "queued_instruction_record_refs must align 1:1 with ids"
    );
    for (id, reference) in projection
        .queued_instruction_record_ids
        .iter()
        .zip(projection.queued_instruction_record_refs.iter())
    {
        assert!(
            runtime_paths
                .is_canonical_queued_instruction_record_ref(reference, wp_id, id),
            "queued instruction ref {reference} must resolve to a canonical \
             <gov_root>/queued_instructions/<wp_id>/<instr_id>.json path \
             bound to wp={wp_id} and instr={id}"
        );
    }

    // ── Workflow binding lifecycle state derives from canonical truth ───────
    assert_eq!(
        projection.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::ValidationWait),
        "projection workflow_binding_state must equal ValidationWait when canonical \
         is in the Validation family AND validator-gate posture is active"
    );

    // ── Spec invariants: validation_wait requires active validator gate ────
    let no_gate_posture = SoftwareDeliveryBindingGatePosture {
        has_active_validator_gate: false,
        ..gate_posture
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(&canonical, no_gate_posture, true),
        Some(SoftwareDeliveryWorkflowBindingState::NodeActive),
        "v02.181 invariant: without an active validator-gate record, Validation family \
         must NOT promote the binding to ValidationWait"
    );

    // ── Spec invariants: approval_wait requires unresolved governed actions ─
    let mut approval_canonical = canonical.clone();
    approval_canonical.workflow_state_family = WorkflowStateFamily::Approval;
    approval_canonical.queue_reason_code = WorkflowQueueReasonCode::ApprovalWait;
    let approval_no_actions = SoftwareDeliveryBindingGatePosture {
        has_active_validator_gate: false,
        has_unresolved_governed_actions: false,
        ..gate_posture
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(
            &approval_canonical,
            approval_no_actions,
            true,
        ),
        Some(SoftwareDeliveryWorkflowBindingState::NodeActive),
        "v02.181 invariant: without unresolved governed actions, Approval family \
         must NOT promote the binding to ApprovalWait"
    );
    let approval_with_actions = SoftwareDeliveryBindingGatePosture {
        has_unresolved_governed_actions: true,
        has_active_validator_gate: false,
        ..gate_posture
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(
            &approval_canonical,
            approval_with_actions,
            true,
        ),
        Some(SoftwareDeliveryWorkflowBindingState::ApprovalWait),
        "Approval family + unresolved governed actions = ApprovalWait"
    );

    // ── Spec invariants: closeout_pending derives from canonical truth ─────
    let mut done_canonical = canonical.clone();
    done_canonical.workflow_state_family = WorkflowStateFamily::Done;
    done_canonical.queue_reason_code = WorkflowQueueReasonCode::ReadyForHuman;
    done_canonical.blockers = Vec::new();
    let closeout_posture_present = SoftwareDeliveryBindingGatePosture {
        has_active_validator_gate: false,
        has_unresolved_governed_actions: false,
        has_closeout_posture: true,
        workflow_failed: false,
        workflow_canceled: false,
        workflow_settled: false,
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(
            &done_canonical,
            closeout_posture_present,
            false,
        ),
        Some(SoftwareDeliveryWorkflowBindingState::CloseoutPending),
        "Done family + canonical closeout posture = CloseoutPending"
    );
    let settled_posture = SoftwareDeliveryBindingGatePosture {
        workflow_settled: true,
        has_closeout_posture: true,
        ..closeout_posture_present
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(&done_canonical, settled_posture, false),
        Some(SoftwareDeliveryWorkflowBindingState::Settled),
        "Done family + canonical settled posture = Settled"
    );
    let failed_posture = SoftwareDeliveryBindingGatePosture {
        workflow_failed: true,
        ..closeout_posture_present
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(&done_canonical, failed_posture, false),
        Some(SoftwareDeliveryWorkflowBindingState::Failed),
        "workflow_failed wins over family lifecycle"
    );
    let canceled_posture = SoftwareDeliveryBindingGatePosture {
        workflow_canceled: true,
        ..closeout_posture_present
    };
    assert_eq!(
        derive_software_delivery_workflow_binding_state(&done_canonical, canceled_posture, false),
        Some(SoftwareDeliveryWorkflowBindingState::Canceled),
        "workflow_canceled wins over family lifecycle"
    );

    // ── Discipline guard: foreign-WP claim/lease breaks stable-id join ─────
    let foreign_claim = GovernanceClaimLeaseRecordV1 {
        work_packet_id: "WP-OTHER".to_string(),
        ..claim.clone()
    };
    assert!(
        build_software_delivery_projection_surface_with_overlay(
            &canonical,
            Some("workflow-run-mt004"),
            Some("workflow-binding-mt004"),
            Some("session-coder-mt004"),
            None,
            &[],
            Some(&foreign_claim),
            &instructions,
            gate_posture,
            &runtime_paths,
        )
        .is_none(),
        "overlay projection MUST refuse a foreign-WP claim/lease record \
         (stable-id join broken)"
    );

    // ── Discipline guard: foreign-WP queued instruction breaks stable-id join
    let foreign_instruction = GovernanceQueuedInstructionRecordV1 {
        work_packet_id: "WP-OTHER".to_string(),
        ..instructions[0].clone()
    };
    assert!(
        build_software_delivery_projection_surface_with_overlay(
            &canonical,
            Some("workflow-run-mt004"),
            Some("workflow-binding-mt004"),
            Some("session-coder-mt004"),
            None,
            &[],
            Some(&claim),
            &[foreign_instruction],
            gate_posture,
            &runtime_paths,
        )
        .is_none(),
        "overlay projection MUST refuse a foreign-WP queued-instruction record \
         (stable-id join broken)"
    );

    // ── Discipline guard: non-software-delivery profile is refused ─────────
    let mut non_sd = canonical.clone();
    non_sd.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        build_software_delivery_projection_surface_with_overlay(
            &non_sd,
            None,
            None,
            None,
            None,
            &[],
            None,
            &[],
            gate_posture,
            &runtime_paths,
        )
        .is_none(),
        "overlay projection MUST refuse non-software_delivery canonical summaries"
    );

    // ── Validator helper: passes for canonical-aligned overlay ─────────────
    let overlay_validation = validate_software_delivery_projection_surface_overlay(
        &projection,
        &canonical,
        &runtime_paths,
        gate_posture,
    );
    assert!(
        overlay_validation.ok,
        "overlay validator must pass on canonical-aligned projection; issues={:?}",
        overlay_validation.issues
    );

    // ── Validator helper: rejects spoofed claim/lease record ref ───────────
    let spoofed = SoftwareDeliveryProjectionSurfaceV1 {
        claim_lease_record_ref: Some(format!(
            "{}notes/claim_leases/{wp_id}/claim-mt004-A.json",
            runtime_paths.governance_root_display()
        )),
        ..projection.clone()
    };
    let spoofed_validation = validate_software_delivery_projection_surface_overlay(
        &spoofed,
        &canonical,
        &runtime_paths,
        gate_posture,
    );
    assert!(
        !spoofed_validation.ok,
        "overlay validator must reject a spoofed claim/lease record ref"
    );
    assert!(
        spoofed_validation
            .issues
            .iter()
            .any(|i| i.field == "claim_lease_record_ref"),
        "overlay validator must surface claim_lease_record_ref issue"
    );

    // ── Validator helper: rejects unsorted queued-instruction ids ──────────
    let unsorted = SoftwareDeliveryProjectionSurfaceV1 {
        queued_instruction_record_ids: vec![
            "instr-002-resume".to_string(),
            "instr-001-steer".to_string(),
        ],
        queued_instruction_record_refs: vec![
            runtime_paths.queued_instruction_record_display(wp_id, "instr-002-resume"),
            runtime_paths.queued_instruction_record_display(wp_id, "instr-001-steer"),
        ],
        ..projection.clone()
    };
    let unsorted_validation = validate_software_delivery_projection_surface_overlay(
        &unsorted,
        &canonical,
        &runtime_paths,
        gate_posture,
    );
    assert!(
        !unsorted_validation.ok,
        "overlay validator must reject unsorted queued_instruction_record_ids"
    );

    // ── Validator helper: rejects mismatched workflow_binding_state ────────
    let mismatched = SoftwareDeliveryProjectionSurfaceV1 {
        workflow_binding_state: Some(SoftwareDeliveryWorkflowBindingState::Settled),
        ..projection.clone()
    };
    let mismatched_validation = validate_software_delivery_projection_surface_overlay(
        &mismatched,
        &canonical,
        &runtime_paths,
        gate_posture,
    );
    assert!(
        !mismatched_validation.ok,
        "overlay validator must reject a workflow_binding_state that disagrees \
         with canonical truth and gate posture"
    );
    assert!(
        mismatched_validation
            .issues
            .iter()
            .any(|i| i.field == "workflow_binding_state"
                && matches!(
                    i.code,
                    StructuredCollaborationValidationCode::SummaryJoinMismatch
                )),
        "overlay validator must surface workflow_binding_state SummaryJoinMismatch"
    );

    // ── Base authority validator must continue to pass on the same surface ─
    let base_validation =
        validate_software_delivery_projection_surface_authority(&projection, &canonical);
    assert!(
        base_validation.ok,
        "base authority validation must still pass after overlay extension; issues={:?}",
        base_validation.issues
    );

    // ── Round-trip: serde preserves the overlay extension fields ───────────
    let serialized = serde_json::to_string(&projection).expect("serialize overlay projection");
    let round_tripped: SoftwareDeliveryProjectionSurfaceV1 =
        serde_json::from_str(&serialized).expect("deserialize overlay projection");
    assert_eq!(round_tripped, projection);
    let round_overlay_validation = validate_software_delivery_projection_surface_overlay(
        &round_tripped,
        &canonical,
        &runtime_paths,
        gate_posture,
    );
    assert!(round_overlay_validation.ok);

    // ── Role Mailbox advisory triage row surfaces same stable ids ──────────
    let triage_row: SoftwareDeliveryOverlayTriageRowV1 =
        build_software_delivery_overlay_triage_row(&projection)
            .expect("triage row must build for software_delivery projection");
    assert_eq!(triage_row.work_packet_id, projection.work_packet_id);
    assert_eq!(triage_row.workflow_run_id, projection.workflow_run_id);
    assert_eq!(triage_row.workflow_binding_id, projection.workflow_binding_id);
    assert_eq!(triage_row.workflow_binding_state, projection.workflow_binding_state);
    assert_eq!(
        triage_row.claim_lease_record_id,
        projection.claim_lease_record_id
    );
    assert_eq!(
        triage_row.claim_lease_record_ref,
        projection.claim_lease_record_ref
    );
    assert_eq!(
        triage_row.queued_instruction_record_ids,
        projection.queued_instruction_record_ids
    );
    assert_eq!(
        triage_row.queued_instruction_record_refs,
        projection.queued_instruction_record_refs
    );

    // Triage row also enforces software_delivery profile boundary.
    let mut non_sd_projection = projection.clone();
    non_sd_projection.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        build_software_delivery_overlay_triage_row(&non_sd_projection).is_none(),
        "advisory triage row MUST refuse non-software_delivery projections"
    );
}

#[test]
fn production_finalize_emits_software_delivery_projection_surface_with_overlay() {
    // MT-004 production-path regression: finalize_runtime_structured_work_packet_writes
    // is the production wrapper that emit_runtime_structured_work_packet_artifacts
    // calls. After MT-004 wiring it MUST also emit
    // <gov_root>/work_packets/<wp_id>/projection_surface.json carrying the
    // overlay extension records (claim/lease and queued instructions) read
    // from canonical runtime paths and the derived workflow_binding_state.
    // Without this wiring the overlay surface would only be reachable via
    // tests, leaving DCC/Task Board/Role Mailbox readers with no governed
    // way to view the extension state.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Production-Surface";

    // Seed canonical claim/lease overlay record on disk under
    // <gov_root>/claim_leases/<wp_id>/<claim_id>.json.
    let claim_id = "claim-prod-A";
    let claim = overlay_claim_lease(wp_id, claim_id);
    let claim_path = runtime_paths.claim_lease_record_path(wp_id, claim_id);
    std::fs::create_dir_all(claim_path.parent().expect("claim parent dir"))
        .expect("create claim_leases dir");
    std::fs::write(&claim_path, serde_json::to_vec(&claim).expect("serialize claim"))
        .expect("write claim record");

    // Seed canonical queued-instruction overlay records on disk under
    // <gov_root>/queued_instructions/<wp_id>/<instr_id>.json.
    let instr_a = overlay_queued_instruction(
        wp_id,
        "instr-prod-A",
        SoftwareDeliveryQueuedInstructionAction::Resume,
    );
    let instr_b = overlay_queued_instruction(
        wp_id,
        "instr-prod-B",
        SoftwareDeliveryQueuedInstructionAction::Cancel,
    );
    for inst in [&instr_a, &instr_b] {
        let path = runtime_paths.queued_instruction_record_path(wp_id, &inst.record_id);
        std::fs::create_dir_all(path.parent().expect("queued parent dir"))
            .expect("create queued_instructions dir");
        std::fs::write(&path, serde_json::to_vec(inst).expect("serialize instruction"))
            .expect("write queued instruction");
    }

    // Seed canonical validator gate record so gate_posture.has_active_validator_gate
    // is true; binding state should resolve to ValidationWait given the
    // canonical Validation family.
    let gate_path = runtime_paths.validator_gate_record_path(wp_id);
    std::fs::create_dir_all(gate_path.parent().expect("gate parent dir"))
        .expect("create validator_gates dir");
    std::fs::write(&gate_path, b"{\"placeholder\":\"production-path-gate\"}")
        .expect("write validator gate record");

    // Build canonical summary aligned with the seeded validator gate ref so
    // canonical-truth derivation does not refuse on missing gate evidence.
    let canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    let canonical_value = serde_json::to_value(&canonical).expect("canonical to value");
    let detail_value = json!({
        "schema_id": "hsk.tracked_work_packet@1",
        "schema_version": "1",
        "record_id": wp_id,
        "record_kind": "work_packet",
        "project_profile_kind": "software_delivery",
        "updated_at": canonical.updated_at,
        "mirror_state": "canonical_only",
        "authority_refs": canonical.authority_refs,
        "evidence_refs": canonical.evidence_refs,
    });

    let validation = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );

    finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &validation,
        &canonical_value,
        &detail_value,
    )
    .expect("production wrapper must emit projection surface artifact on success");

    let surface_path = runtime_paths.work_packet_projection_surface_path(wp_id);
    assert!(
        surface_path.exists(),
        "production wrapper MUST emit projection_surface.json (path={})",
        surface_path.display()
    );

    let surface_bytes =
        std::fs::read(&surface_path).expect("read projection surface artifact");
    let surface: SoftwareDeliveryProjectionSurfaceV1 =
        serde_json::from_slice(&surface_bytes).expect("deserialize projection surface");

    // Stable-id discipline: claim/lease record id and ref propagated from
    // the canonical disk record under canonical runtime path.
    assert_eq!(
        surface.claim_lease_record_id.as_deref(),
        Some(claim_id),
        "production projection MUST carry claim/lease record id from canonical disk"
    );
    let expected_claim_ref = runtime_paths.claim_lease_record_display(wp_id, claim_id);
    assert_eq!(
        surface.claim_lease_record_ref.as_deref(),
        Some(expected_claim_ref.as_str()),
        "production projection claim/lease ref MUST resolve to canonical runtime path"
    );

    // Queued-instruction ids/refs sorted, deduped, aligned 1:1.
    let mut expected_ids = vec![instr_a.record_id.clone(), instr_b.record_id.clone()];
    expected_ids.sort();
    assert_eq!(
        surface.queued_instruction_record_ids, expected_ids,
        "production projection MUST carry queued-instruction ids sorted/deduped"
    );
    assert_eq!(
        surface.queued_instruction_record_refs.len(),
        surface.queued_instruction_record_ids.len(),
        "queued-instruction ids and refs MUST be aligned 1:1"
    );
    for (id, r) in surface
        .queued_instruction_record_ids
        .iter()
        .zip(surface.queued_instruction_record_refs.iter())
    {
        let expected = runtime_paths.queued_instruction_record_display(wp_id, id);
        assert_eq!(
            r, &expected,
            "queued-instruction ref MUST resolve to canonical runtime path (id={id})"
        );
    }

    // workflow_binding_state derived from canonical truth: Validation family
    // plus an active validator-gate record on disk -> ValidationWait.
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::ValidationWait),
        "production projection MUST derive workflow_binding_state from canonical truth"
    );

    // Stable id propagation from claim/lease overlay record into the
    // emitted projection. The seeded overlay_claim_lease helper carries
    // workflow_run_id="workflow-run-mt004", workflow_binding_id="workflow-binding-mt004",
    // and model_session_id="session-coder-mt004"; production projection
    // MUST surface these by stable id so workflow_binding_state is anchored
    // to a binding/run/session rather than detached from canonical truth.
    assert_eq!(
        surface.workflow_run_id.as_deref(),
        Some("workflow-run-mt004"),
        "production projection MUST propagate workflow_run_id from canonical claim/lease"
    );
    assert_eq!(
        surface.workflow_binding_id.as_deref(),
        Some("workflow-binding-mt004"),
        "production projection MUST propagate workflow_binding_id from canonical claim/lease"
    );
    assert_eq!(
        surface.model_session_id.as_deref(),
        Some("session-coder-mt004"),
        "production projection MUST propagate model_session_id from canonical claim/lease"
    );

    // Foreign-WP overlay records on disk MUST NOT leak into the projection.
    // Seed a foreign claim/lease and a foreign queued-instruction at the same
    // wp_id paths but with foreign work_packet_id; the production wrapper
    // MUST silently skip them rather than promote a misaligned overlay.
    let foreign_wp = "WP-OTHER";
    let mut foreign_claim = overlay_claim_lease(foreign_wp, "claim-foreign");
    foreign_claim.work_packet_id = foreign_wp.to_string();
    // Place foreign claim under THIS wp_id directory; reader must filter by
    // work_packet_id, not by directory name alone.
    let foreign_claim_path =
        runtime_paths.claim_lease_record_path(wp_id, "claim-foreign");
    std::fs::write(
        &foreign_claim_path,
        serde_json::to_vec(&foreign_claim).expect("serialize foreign claim"),
    )
    .expect("write foreign claim record");

    finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &validation,
        &canonical_value,
        &detail_value,
    )
    .expect("production wrapper must still succeed when foreign overlay is present");

    let surface_after_foreign: SoftwareDeliveryProjectionSurfaceV1 = serde_json::from_slice(
        &std::fs::read(&surface_path).expect("read projection surface after foreign seed"),
    )
    .expect("deserialize projection surface after foreign seed");
    assert_eq!(
        surface_after_foreign.claim_lease_record_id.as_deref(),
        Some(claim_id),
        "production projection MUST keep canonical claim and refuse foreign overlay"
    );

    // Validation failure removes the artifact (mirrors closeout posture
    // staleness clearing on validation failure).
    let mut failing = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    failing.push_issue(
        StructuredCollaborationValidationCode::InvalidFieldValue,
        "schema_id",
        Some("expected".to_string()),
        Some("actual".to_string()),
        "synthetic failure for production projection lifecycle clearing",
    );
    let _ = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &failing,
        &canonical_value,
        &detail_value,
    );
    assert!(
        !surface_path.exists(),
        "production wrapper MUST clear projection_surface.json on validation failure \
         (path={})",
        surface_path.display()
    );
}

fn write_canonical_workflow_run_lifecycle(
    runtime_paths: &RuntimeGovernancePaths,
    record: &SoftwareDeliveryWorkflowRunLifecycleV1,
) {
    let path = runtime_paths.workflow_run_record_path(&record.work_packet_id);
    std::fs::create_dir_all(path.parent().expect("workflow_runs parent dir"))
        .expect("create workflow_runs dir");
    std::fs::write(
        &path,
        serde_json::to_vec(record).expect("serialize workflow_run_lifecycle"),
    )
    .expect("write workflow_run_lifecycle record");
}

fn read_emitted_projection_surface(
    runtime_paths: &RuntimeGovernancePaths,
    workspace_root: &Path,
    wp_id: &str,
    canonical: &StructuredCollaborationSummaryV1,
) -> SoftwareDeliveryProjectionSurfaceV1 {
    let canonical_value = serde_json::to_value(canonical).expect("canonical to value");
    let detail_value = json!({
        "schema_id": "hsk.tracked_work_packet@1",
        "schema_version": "1",
        "record_id": wp_id,
        "record_kind": "work_packet",
        "project_profile_kind": "software_delivery",
        "updated_at": canonical.updated_at,
        "mirror_state": "canonical_only",
        "authority_refs": canonical.authority_refs,
        "evidence_refs": canonical.evidence_refs,
    });
    let validation = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    finalize_runtime_structured_work_packet_writes(
        runtime_paths,
        workspace_root,
        wp_id,
        &validation,
        &canonical_value,
        &detail_value,
    )
    .expect("production wrapper must succeed");
    let surface_path = runtime_paths.work_packet_projection_surface_path(wp_id);
    let bytes = std::fs::read(&surface_path).expect("read projection surface");
    serde_json::from_slice(&bytes).expect("deserialize projection surface")
}

/// Emit the projection surface WITHOUT routing through
/// `apply_software_delivery_workflow_run_lifecycle`, so a test can pre-seed
/// a `<gov_root>/workflow_runs/<wp_id>.json` record (representing an
/// out-of-band runtime workflow lifecycle truth source) and verify that the
/// projection-surface emitter respects the override semantics in
/// `compute_software_delivery_projection_gate_posture_from_disk`. The
/// production-wired writer always derives the lifecycle record from
/// canonical truth and would overwrite a divergent pre-seeded sidecar; this
/// helper exists only to keep override-mechanism coverage isolated to the
/// projection emitter.
fn read_emitted_projection_surface_preserving_seeded_lifecycle(
    runtime_paths: &RuntimeGovernancePaths,
    workspace_root: &Path,
    wp_id: &str,
    canonical: &StructuredCollaborationSummaryV1,
) -> SoftwareDeliveryProjectionSurfaceV1 {
    let canonical_value = serde_json::to_value(canonical).expect("canonical to value");
    apply_software_delivery_closeout_posture_lifecycle(
        runtime_paths,
        workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("closeout posture lifecycle must succeed");
    apply_software_delivery_projection_surface_lifecycle(
        runtime_paths,
        workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("projection surface lifecycle must succeed");
    let surface_path = runtime_paths.work_packet_projection_surface_path(wp_id);
    let bytes = std::fs::read(&surface_path).expect("read projection surface");
    serde_json::from_slice(&bytes).expect("deserialize projection surface")
}

#[test]
fn production_projection_falls_back_to_queued_instruction_for_stable_ids() {
    // MT-004 production-path regression: when no canonical claim/lease record
    // is on disk for a software-delivery WP, the production projection MUST
    // fall back to the canonical queued-instruction record's stable ids
    // (workflow_run_id, workflow_binding_id, target_model_session_id) rather
    // than emitting `None`. This proves stable-id propagation does not
    // depend on a held lease and that DCC/Task Board/Role Mailbox readers
    // can still navigate to the bound workflow run/session via canonical
    // overlay records.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-StableIds-FromQueued";

    let instr = overlay_queued_instruction(
        wp_id,
        "instr-stableids-A",
        SoftwareDeliveryQueuedInstructionAction::Resume,
    );
    let path = runtime_paths.queued_instruction_record_path(wp_id, &instr.record_id);
    std::fs::create_dir_all(path.parent().expect("queued parent dir"))
        .expect("create queued_instructions dir");
    std::fs::write(&path, serde_json::to_vec(&instr).expect("serialize instr"))
        .expect("write queued instruction");

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    // Active family + no validator gate + no claim/lease: binding state
    // resolves to NodeActive, not gated; primary point is stable-id propagation.
    canonical.workflow_state_family = WorkflowStateFamily::Active;
    canonical.queue_reason_code = WorkflowQueueReasonCode::DependencyWait;
    canonical.evidence_refs.clear();
    canonical.blockers.clear();

    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);

    assert_eq!(
        surface.workflow_run_id.as_deref(),
        Some("workflow-run-mt004"),
        "production projection MUST fall back to queued instruction for workflow_run_id \
         when no claim/lease is present"
    );
    assert_eq!(
        surface.workflow_binding_id.as_deref(),
        Some("workflow-binding-mt004"),
        "production projection MUST fall back to queued instruction for workflow_binding_id \
         when no claim/lease is present"
    );
    assert_eq!(
        surface.model_session_id.as_deref(),
        Some("session-validator-mt004"),
        "production projection MUST fall back to queued instruction target_model_session_id \
         when no claim/lease is present"
    );
    assert!(
        surface.claim_lease_record_id.is_none(),
        "no claim/lease was seeded; projection MUST NOT fabricate a claim record id"
    );
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::NodeActive),
        "Active family without claim/lease and without active validator gate \
         MUST resolve to NodeActive"
    );
}

#[test]
fn projection_emitter_respects_seeded_lifecycle_override_for_approval_wait() {
    // MT-004 projection emitter regression: when an out-of-band runtime
    // workflow lifecycle source has written `<gov_root>/workflow_runs/<wp_id>.json`
    // with has_unresolved_governed_actions=true, the projection emitter
    // (`apply_software_delivery_projection_surface_lifecycle`) MUST honour
    // the seeded override and reach ApprovalWait even when canonical alone
    // does not carry an Approval-family governed action id. This isolates
    // the override path in `compute_software_delivery_projection_gate_posture_from_disk`
    // from the canonical-derived production write.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-ApprovalWait";

    let lifecycle = SoftwareDeliveryWorkflowRunLifecycleV1 {
        schema_id: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1.to_string(),
        schema_version: "1".to_string(),
        record_id: wp_id.to_string(),
        record_kind: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: wp_id.to_string(),
        workflow_run_id: Some("workflow-run-approval".to_string()),
        workflow_binding_id: Some("workflow-binding-approval".to_string()),
        model_session_id: Some("session-approval".to_string()),
        workflow_failed: false,
        workflow_canceled: false,
        workflow_settled: false,
        has_unresolved_governed_actions: true,
        updated_at: "2026-04-28T01:00:00Z".to_string(),
    };
    write_canonical_workflow_run_lifecycle(&runtime_paths, &lifecycle);

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Approval;
    canonical.queue_reason_code = WorkflowQueueReasonCode::ApprovalWait;
    canonical.blockers = vec!["awaiting_approver".to_string()];
    // No validator gate evidence: canonical.evidence_refs only carries the
    // helper's gate display, which is acceptable for the Approval branch
    // (validator-gate posture is not what gates ApprovalWait).
    canonical.evidence_refs.clear();

    let surface = read_emitted_projection_surface_preserving_seeded_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical,
    );

    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::ApprovalWait),
        "projection emitter MUST reach ApprovalWait when seeded workflow_run_lifecycle \
         carries has_unresolved_governed_actions=true (Approval family)"
    );
    assert_eq!(
        surface.workflow_run_id.as_deref(),
        Some("workflow-run-approval"),
        "projection emitter MUST surface workflow_run_id from seeded lifecycle record \
         when no overlay records carry it"
    );
    assert_eq!(
        surface.workflow_binding_id.as_deref(),
        Some("workflow-binding-approval"),
        "projection emitter MUST surface workflow_binding_id from seeded lifecycle record \
         when no overlay records carry it"
    );
}

#[test]
fn projection_emitter_respects_seeded_lifecycle_override_for_failed() {
    // MT-004 projection emitter regression: when an out-of-band runtime
    // workflow lifecycle source has written workflow_failed=true into
    // `<gov_root>/workflow_runs/<wp_id>.json`, the projection emitter MUST
    // honour the override even when canonical workflow_state_family does
    // not carry the failure signal (e.g., the canonical work-packet status
    // is still Active because a downstream flight recorder fault has not
    // yet propagated).
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Failed";

    let lifecycle = SoftwareDeliveryWorkflowRunLifecycleV1 {
        schema_id: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1.to_string(),
        schema_version: "1".to_string(),
        record_id: wp_id.to_string(),
        record_kind: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: wp_id.to_string(),
        workflow_run_id: Some("workflow-run-failed".to_string()),
        workflow_binding_id: Some("workflow-binding-failed".to_string()),
        model_session_id: None,
        workflow_failed: true,
        workflow_canceled: false,
        workflow_settled: false,
        has_unresolved_governed_actions: false,
        updated_at: "2026-04-28T02:00:00Z".to_string(),
    };
    write_canonical_workflow_run_lifecycle(&runtime_paths, &lifecycle);

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    // Family is irrelevant: workflow_failed short-circuits the binding-state
    // derivation. Use Active to make the test agnostic to family-specific
    // lifecycle gating.
    canonical.workflow_state_family = WorkflowStateFamily::Active;
    canonical.queue_reason_code = WorkflowQueueReasonCode::DependencyWait;
    canonical.blockers.clear();
    canonical.evidence_refs.clear();

    let surface = read_emitted_projection_surface_preserving_seeded_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical,
    );

    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Failed),
        "projection emitter MUST reach Failed when seeded workflow_run_lifecycle \
         carries workflow_failed=true"
    );
}

#[test]
fn projection_emitter_respects_seeded_lifecycle_override_for_settled() {
    // MT-004 projection emitter regression: when an out-of-band runtime
    // workflow lifecycle source has written workflow_settled=true into
    // `<gov_root>/workflow_runs/<wp_id>.json`, the projection emitter MUST
    // honour the override and surface Settled even when canonical alone
    // would resolve to CloseoutPending/NodeActive on a Done family.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Settled";

    let lifecycle = SoftwareDeliveryWorkflowRunLifecycleV1 {
        schema_id: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1.to_string(),
        schema_version: "1".to_string(),
        record_id: wp_id.to_string(),
        record_kind: SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: wp_id.to_string(),
        workflow_run_id: Some("workflow-run-settled".to_string()),
        workflow_binding_id: Some("workflow-binding-settled".to_string()),
        model_session_id: Some("session-settled".to_string()),
        workflow_failed: false,
        workflow_canceled: false,
        workflow_settled: true,
        has_unresolved_governed_actions: false,
        updated_at: "2026-04-28T03:00:00Z".to_string(),
    };
    write_canonical_workflow_run_lifecycle(&runtime_paths, &lifecycle);

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Done;
    canonical.queue_reason_code = WorkflowQueueReasonCode::DependencyWait;
    canonical.blockers.clear();
    canonical.status = "settled".to_string();
    canonical.evidence_refs.clear();

    let surface = read_emitted_projection_surface_preserving_seeded_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical,
    );

    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Settled),
        "projection emitter MUST reach Settled when seeded workflow_run_lifecycle \
         carries workflow_settled=true (Done family)"
    );
}

#[test]
fn production_projection_reaches_approval_wait_from_canonical_summary_alone() {
    // MT-004 production-path regression: ApprovalWait MUST be reachable from
    // the production-written canonical structured collaboration summary
    // alone (the same record `emit_runtime_structured_work_packet_artifacts`
    // already produces), without any synthetic workflow_run_lifecycle
    // sidecar. Approval family + `allowed_action_ids` containing a
    // canonical Approval-family governed action (approve/reject/await_gate_review)
    // is the runtime signal that a governed action is unresolved.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-ApprovalWait-FromCanonical";

    // No workflow_runs/<wp_id>.json sidecar; lifecycle truth comes ONLY from
    // the canonical summary that production already writes.
    assert!(
        !runtime_paths.workflow_run_record_path(wp_id).exists(),
        "test must prove derivation works without the sidecar"
    );

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Approval;
    canonical.queue_reason_code = WorkflowQueueReasonCode::ApprovalWait;
    canonical.allowed_action_ids = vec!["approve".to_string(), "reject".to_string()];
    canonical.blockers = vec!["awaiting_approver".to_string()];
    canonical.evidence_refs.clear();

    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::ApprovalWait),
        "ApprovalWait MUST be reachable from canonical Approval family + governed action ids \
         alone, without a workflow_run_lifecycle sidecar"
    );
}

#[test]
fn production_projection_reaches_failed_from_canonical_summary_alone() {
    // MT-004 production-path regression: Failed MUST be reachable from the
    // canonical summary alone, with no workflow_run_lifecycle sidecar
    // present. The runtime signal is `queue_reason_code == BlockedError`
    // (canonical error condition); BlockedPolicy and BlockedCapability are
    // intentionally NOT promoted to Failed.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Failed-FromCanonical";

    assert!(
        !runtime_paths.workflow_run_record_path(wp_id).exists(),
        "test must prove derivation works without the sidecar"
    );

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    // Family is irrelevant: workflow_failed short-circuits derivation.
    canonical.workflow_state_family = WorkflowStateFamily::Active;
    canonical.queue_reason_code = WorkflowQueueReasonCode::BlockedError;
    canonical.blockers = vec!["compile_error".to_string()];
    canonical.evidence_refs.clear();

    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Failed),
        "Failed MUST be reachable from canonical queue_reason_code=BlockedError alone, \
         without a workflow_run_lifecycle sidecar"
    );

    // Adversarial check: BlockedPolicy MUST NOT promote to Failed -- it is a
    // recoverable blocked state, not a workflow run failure.
    let wp_id_policy = "WP-1-Software-Delivery-BlockedPolicy-NotFailed";
    let mut canonical_policy = overlay_canonical_summary(&runtime_paths, wp_id_policy);
    canonical_policy.workflow_state_family = WorkflowStateFamily::Blocked;
    canonical_policy.queue_reason_code = WorkflowQueueReasonCode::BlockedPolicy;
    canonical_policy.blockers = vec!["policy_violation".to_string()];
    canonical_policy.evidence_refs.clear();
    let surface_policy = read_emitted_projection_surface(
        &runtime_paths,
        &workspace_root,
        wp_id_policy,
        &canonical_policy,
    );
    assert_ne!(
        surface_policy.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Failed),
        "BlockedPolicy MUST NOT promote to Failed; recoverable blocked states stay non-terminal"
    );
}

#[test]
fn production_projection_reaches_canceled_from_canonical_summary_alone() {
    // MT-004 production-path regression: Canceled MUST be reachable from the
    // canonical workflow_state_family=Canceled alone (production-written by
    // `emit_runtime_structured_work_packet_artifacts`).
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Canceled-FromCanonical";

    assert!(
        !runtime_paths.workflow_run_record_path(wp_id).exists(),
        "test must prove derivation works without the sidecar"
    );

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Canceled;
    canonical.queue_reason_code = WorkflowQueueReasonCode::DependencyWait;
    canonical.blockers.clear();
    canonical.evidence_refs.clear();

    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Canceled),
        "Canceled MUST be reachable from canonical workflow_state_family=Canceled alone"
    );
}

#[test]
fn production_projection_reaches_settled_from_canonical_summary_alone() {
    // MT-004 production-path regression: Settled MUST be reachable from the
    // canonical workflow_state_family=Archived alone (post-close success
    // state distinct from Done/CloseoutPending). No workflow_run_lifecycle
    // sidecar is required to surface this terminal state.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-Settled-FromCanonical";

    assert!(
        !runtime_paths.workflow_run_record_path(wp_id).exists(),
        "test must prove derivation works without the sidecar"
    );

    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Archived;
    canonical.queue_reason_code = WorkflowQueueReasonCode::DependencyWait;
    canonical.blockers.clear();
    canonical.status = "archived".to_string();
    canonical.evidence_refs.clear();

    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::Settled),
        "Settled MUST be reachable from canonical workflow_state_family=Archived alone"
    );
}

#[test]
fn production_writes_workflow_run_lifecycle_record_before_projection_surface() {
    // MT-004 production-path regression: the runtime workflow/governed-action
    // lifecycle source MUST materialize the canonical
    // `<gov_root>/workflow_runs/<wp_id>.json` record from production code,
    // not from a test fixture. This test exercises that writer end-to-end:
    // it seeds canonical overlay extension records, calls the same
    // `finalize_runtime_structured_work_packet_writes` wrapper that
    // `emit_runtime_structured_work_packet_artifacts` calls in production,
    // and asserts the canonical lifecycle record is on disk WITH the
    // expected stable ids and lifecycle flags BEFORE the emitted
    // projection_surface.json reads it.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-LifecycleWriter-Approval";

    // Pre-condition: no synthetic sidecar — the canonical record must come
    // from production code, not a test helper.
    let lifecycle_path = runtime_paths.workflow_run_record_path(wp_id);
    assert!(
        !lifecycle_path.exists(),
        "test must prove production code writes the lifecycle record (path={})",
        lifecycle_path.display()
    );

    // Seed a canonical claim/lease overlay record so the production writer
    // exercises the stable-id selection path it shares with the projection
    // surface emitter.
    let claim = overlay_claim_lease(wp_id, "claim-mt004-prod-A");
    let claim_path = runtime_paths.claim_lease_record_path(wp_id, &claim.record_id);
    std::fs::create_dir_all(claim_path.parent().expect("claim_leases parent dir"))
        .expect("create claim_leases dir");
    std::fs::write(
        &claim_path,
        serde_json::to_vec(&claim).expect("serialize claim/lease"),
    )
    .expect("write claim/lease record");

    // Approval family + canonical Approval-family governed action id is the
    // production runtime signal that a governed action is unresolved.
    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Approval;
    canonical.queue_reason_code = WorkflowQueueReasonCode::ApprovalWait;
    canonical.allowed_action_ids = vec!["approve".to_string(), "reject".to_string()];
    canonical.blockers = vec!["awaiting_approver".to_string()];
    canonical.evidence_refs.clear();

    // Drive the production wrapper that `emit_runtime_structured_work_packet_artifacts`
    // calls — no test helper materializes the lifecycle sidecar here.
    let surface =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);

    // The lifecycle record MUST exist on disk and parse as the canonical
    // SoftwareDeliveryWorkflowRunLifecycleV1.
    assert!(
        lifecycle_path.exists(),
        "production wrapper MUST write `<gov_root>/workflow_runs/<wp_id>.json` for \
         a software_delivery canonical summary (path={})",
        lifecycle_path.display()
    );
    let bytes = std::fs::read(&lifecycle_path).expect("read lifecycle record");
    let written: SoftwareDeliveryWorkflowRunLifecycleV1 =
        serde_json::from_slice(&bytes).expect("deserialize lifecycle record");

    assert_eq!(
        written.schema_id, SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1,
        "production lifecycle record MUST carry the canonical schema id"
    );
    assert_eq!(
        written.record_kind, SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND,
        "production lifecycle record MUST carry the canonical record kind"
    );
    assert_eq!(written.record_id, wp_id);
    assert_eq!(written.work_packet_id, wp_id);
    assert_eq!(written.project_profile_kind, ProjectProfileKind::SoftwareDelivery);
    assert_eq!(written.updated_at, canonical.updated_at);

    // Stable ids MUST come from the canonical claim/lease overlay (production
    // selection path), not from a synthetic sidecar.
    assert_eq!(
        written.workflow_run_id.as_deref(),
        Some("workflow-run-mt004"),
        "production lifecycle record MUST surface workflow_run_id from canonical claim/lease"
    );
    assert_eq!(
        written.workflow_binding_id.as_deref(),
        Some("workflow-binding-mt004"),
        "production lifecycle record MUST surface workflow_binding_id from canonical claim/lease"
    );
    assert_eq!(
        written.model_session_id.as_deref(),
        Some("session-coder-mt004"),
        "production lifecycle record MUST surface model_session_id from canonical claim/lease"
    );

    // Lifecycle flags MUST be derived from canonical truth (Approval family +
    // canonical governed action id => has_unresolved_governed_actions=true,
    // every other flag false).
    assert!(
        written.has_unresolved_governed_actions,
        "Approval family + canonical Approval-family allowed_action_id MUST set \
         has_unresolved_governed_actions=true on the production lifecycle record"
    );
    assert!(!written.workflow_failed);
    assert!(!written.workflow_canceled);
    assert!(!written.workflow_settled);

    // The projection surface MUST reach ApprovalWait through the runtime-backed
    // sidecar — proving the writer ran BEFORE the projection surface emitter.
    assert_eq!(
        surface.workflow_binding_state,
        Some(SoftwareDeliveryWorkflowBindingState::ApprovalWait),
        "production projection MUST reach ApprovalWait via the runtime-backed \
         workflow_run_lifecycle record written by the production wrapper"
    );
    assert_eq!(surface.workflow_run_id.as_deref(), Some("workflow-run-mt004"));
    assert_eq!(
        surface.workflow_binding_id.as_deref(),
        Some("workflow-binding-mt004")
    );
}

#[test]
fn production_clears_workflow_run_lifecycle_record_on_validation_failure() {
    // MT-004 production-path regression: validation failure MUST clear any
    // pre-existing software-delivery workflow run lifecycle record so a
    // stale runtime sidecar cannot promote phantom binding states into the
    // next emitted projection surface (artifact lifecycle parity with
    // closeout_posture.json and projection_surface.json).
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-LifecycleWriter-Cleared";

    // Materialize a canonical lifecycle record via the production writer first
    // so we are clearing a record that production owns, not a test fixture.
    let canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    let canonical_value = serde_json::to_value(&canonical).expect("canonical to value");
    apply_software_delivery_workflow_run_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &canonical_value,
    )
    .expect("production lifecycle writer must succeed for canonical software_delivery summary");
    let lifecycle_path = runtime_paths.workflow_run_record_path(wp_id);
    assert!(
        lifecycle_path.exists(),
        "pre-condition: production writer must have materialized the lifecycle record"
    );

    // Drive the production wrapper with a synthetic validation failure.
    let detail_value = json!({
        "schema_id": "hsk.tracked_work_packet@1",
        "schema_version": "1",
        "record_id": wp_id,
        "record_kind": "work_packet",
        "project_profile_kind": "software_delivery",
        "updated_at": canonical.updated_at,
        "mirror_state": "canonical_only",
        "authority_refs": canonical.authority_refs,
        "evidence_refs": canonical.evidence_refs,
    });
    let mut failing = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    failing.push_issue(
        StructuredCollaborationValidationCode::AuthorityScopeMismatch,
        "test_only",
        None,
        None,
        "synthetic failure for production lifecycle record clearing",
    );
    let _ = finalize_runtime_structured_work_packet_writes(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &failing,
        &canonical_value,
        &detail_value,
    );
    assert!(
        !lifecycle_path.exists(),
        "production wrapper MUST clear `<gov_root>/workflow_runs/<wp_id>.json` on \
         validation failure (path={})",
        lifecycle_path.display()
    );
}

#[test]
fn role_mailbox_software_delivery_triage_remains_advisory() {
    // MT-005 tripwire: the Role Mailbox advisory triage row is a STABLE-ID
    // PROJECTION ONLY. Mutating the triage row (the way an operator UI or
    // mailbox client might) MUST NOT change any canonical software-delivery
    // overlay record on disk: claim/lease, queued instructions, the
    // emitted projection surface, or the runtime workflow_run lifecycle
    // record. Mailbox replies and triage rows MAY inform linked work via
    // governed action or transcription, but they MUST NOT silently mutate
    // authoritative state. This test enforces the v02.181 sec 2.6.8.8
    // mailbox authority boundary by snapshotting canonical state, mutating
    // the triage row, and asserting no on-disk drift.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-MailboxAdvisory";

    // ── 1. Materialize canonical software-delivery overlay state ──────────
    let claim = overlay_claim_lease(wp_id, "claim-mt005-A");
    let claim_path = runtime_paths.claim_lease_record_path(wp_id, &claim.record_id);
    std::fs::create_dir_all(claim_path.parent().expect("claim_leases parent dir"))
        .expect("create claim_leases dir");
    std::fs::write(
        &claim_path,
        serde_json::to_vec(&claim).expect("serialize claim/lease"),
    )
    .expect("write claim/lease record");

    let instr = overlay_queued_instruction(
        wp_id,
        "instr-mt005-A",
        SoftwareDeliveryQueuedInstructionAction::Resume,
    );
    let instr_path = runtime_paths.queued_instruction_record_path(wp_id, &instr.record_id);
    std::fs::create_dir_all(instr_path.parent().expect("queued_instructions parent dir"))
        .expect("create queued_instructions dir");
    std::fs::write(
        &instr_path,
        serde_json::to_vec(&instr).expect("serialize queued instruction"),
    )
    .expect("write queued instruction record");

    // Drive the production wrapper so projection_surface.json + workflow_run
    // lifecycle are also on disk (representing the full canonical state the
    // mailbox advisory triage row is supposed to leave alone).
    let mut canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    canonical.workflow_state_family = WorkflowStateFamily::Active;
    canonical.queue_reason_code = WorkflowQueueReasonCode::ReadyForHuman;
    canonical.evidence_refs.clear();
    let projection =
        read_emitted_projection_surface(&runtime_paths, &workspace_root, wp_id, &canonical);

    let surface_path = runtime_paths.work_packet_projection_surface_path(wp_id);
    let lifecycle_path = runtime_paths.workflow_run_record_path(wp_id);
    assert!(
        claim_path.exists() && instr_path.exists() && surface_path.exists() && lifecycle_path.exists(),
        "pre-condition: full canonical software-delivery overlay state must be on disk"
    );

    // Snapshot canonical bytes BEFORE the triage row is touched.
    let claim_before = std::fs::read(&claim_path).expect("snapshot claim/lease");
    let instr_before = std::fs::read(&instr_path).expect("snapshot queued instruction");
    let surface_before = std::fs::read(&surface_path).expect("snapshot projection surface");
    let lifecycle_before =
        std::fs::read(&lifecycle_path).expect("snapshot workflow_run lifecycle");

    // ── 2. Build the advisory triage row from the canonical projection ────
    let triage_row = build_software_delivery_overlay_triage_row(&projection)
        .expect("triage row must build for software_delivery projection");
    assert_eq!(triage_row.work_packet_id, wp_id);
    assert_eq!(
        triage_row.claim_lease_record_id.as_deref(),
        Some("claim-mt005-A"),
        "advisory triage row MUST surface canonical claim/lease record id"
    );
    assert_eq!(
        triage_row.claim_lease_record_ref.as_deref(),
        Some(
            runtime_paths
                .claim_lease_record_display(wp_id, "claim-mt005-A")
                .as_str()
        ),
        "advisory triage row MUST surface canonical claim/lease record ref"
    );
    assert!(
        triage_row
            .queued_instruction_record_ids
            .contains(&"instr-mt005-A".to_string()),
        "advisory triage row MUST surface canonical queued instruction record id"
    );

    // ── 3. Mutate the triage row in arbitrary ways (simulating a mailbox
    //       reader/UI clobber attempt: forge stable ids, inject a settled
    //       binding state, swap claim/lease and queued instruction ids).
    let mut mutated = triage_row.clone();
    mutated.workflow_run_id = Some("forged-run-id".to_string());
    mutated.workflow_binding_id = Some("forged-binding-id".to_string());
    mutated.workflow_binding_state = Some(SoftwareDeliveryWorkflowBindingState::Settled);
    mutated.claim_lease_record_id = Some("forged-claim-id".to_string());
    mutated.claim_lease_record_ref = Some(
        runtime_paths
            .claim_lease_record_display(wp_id, "forged-claim-id"),
    );
    mutated.queued_instruction_record_ids = vec!["forged-instr-id".to_string()];
    mutated.queued_instruction_record_refs =
        vec![runtime_paths.queued_instruction_record_display(wp_id, "forged-instr-id")];
    mutated.mailbox_thread_ids = vec!["thread-forged".to_string()];

    // Round-trip the mutated triage row through serde so a UI client that
    // serialises and replays it cannot trigger any hidden side effects via
    // deserialisation.
    let serialized = serde_json::to_string(&mutated).expect("serialize mutated triage row");
    let _: SoftwareDeliveryOverlayTriageRowV1 =
        serde_json::from_str(&serialized).expect("deserialize mutated triage row");

    // ── 4. Canonical state on disk MUST be byte-identical to the snapshot ─
    let claim_after = std::fs::read(&claim_path).expect("re-read claim/lease");
    let instr_after = std::fs::read(&instr_path).expect("re-read queued instruction");
    let surface_after = std::fs::read(&surface_path).expect("re-read projection surface");
    let lifecycle_after =
        std::fs::read(&lifecycle_path).expect("re-read workflow_run lifecycle");

    assert_eq!(
        claim_before, claim_after,
        "mailbox triage row mutation MUST NOT mutate canonical claim/lease record"
    );
    assert_eq!(
        instr_before, instr_after,
        "mailbox triage row mutation MUST NOT mutate canonical queued instruction record"
    );
    assert_eq!(
        surface_before, surface_after,
        "mailbox triage row mutation MUST NOT mutate emitted projection_surface.json"
    );
    assert_eq!(
        lifecycle_before, lifecycle_after,
        "mailbox triage row mutation MUST NOT mutate runtime workflow_run lifecycle record"
    );

    // ── 5. Triage row builder remains bounded by software_delivery profile ─
    let mut foreign_profile = projection.clone();
    foreign_profile.project_profile_kind = ProjectProfileKind::Research;
    assert!(
        build_software_delivery_overlay_triage_row(&foreign_profile).is_none(),
        "mailbox triage row builder MUST refuse non-software_delivery projections \
         (authority boundary: the advisory surface only applies to software_delivery work)"
    );

    // ── 6. Re-deriving the triage row from the unchanged projection MUST
    //       continue to surface canonical truth (proves the mutated copy
    //       was a local clone with no link back to canonical state). ──────
    let re_derived = build_software_delivery_overlay_triage_row(&projection)
        .expect("triage row re-derivation must succeed");
    assert_eq!(
        re_derived, triage_row,
        "re-derived triage row MUST match the original; mailbox mutations do not \
         leak into canonical-projection-derived advisory output"
    );
}

#[test]
fn production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary() {
    // MT-004 production-path regression: when the canonical summary is not
    // software_delivery, the production lifecycle writer MUST remove any
    // pre-existing `<gov_root>/workflow_runs/<wp_id>.json` so a stale
    // sidecar from an earlier software_delivery state cannot leak into a
    // non-software_delivery projection emission.
    let (_dir, runtime_paths) = closeout_runtime_paths();
    let workspace_root = runtime_paths.workspace_root().to_path_buf();
    let wp_id = "WP-1-Software-Delivery-LifecycleWriter-NonSwDelivery";

    // Materialize a stale software_delivery lifecycle record via the
    // production writer (using a software_delivery canonical summary so the
    // writer takes the write branch).
    let sw_canonical = overlay_canonical_summary(&runtime_paths, wp_id);
    let sw_canonical_value = serde_json::to_value(&sw_canonical).expect("canonical to value");
    apply_software_delivery_workflow_run_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &sw_canonical_value,
    )
    .expect("production lifecycle writer must succeed for canonical software_delivery summary");
    let lifecycle_path = runtime_paths.workflow_run_record_path(wp_id);
    assert!(
        lifecycle_path.exists(),
        "pre-condition: production writer must have materialized the lifecycle record"
    );

    // Now drive the writer with a non-software_delivery canonical summary; it
    // MUST remove the stale record.
    let mut other = sw_canonical;
    other.project_profile_kind = ProjectProfileKind::Research;
    let other_value = serde_json::to_value(&other).expect("non-sw canonical to value");
    apply_software_delivery_workflow_run_lifecycle(
        &runtime_paths,
        &workspace_root,
        wp_id,
        &other_value,
    )
    .expect("production lifecycle writer must succeed for non-software_delivery canonical");
    assert!(
        !lifecycle_path.exists(),
        "production lifecycle writer MUST remove the stale record when canonical \
         project_profile_kind is not software_delivery (path={})",
        lifecycle_path.display()
    );
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

#[tokio::test]
async fn locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait(
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

    // Register two MTs: one normal, one with mailbox wait
    let mut mt_base = base_tracked_micro_task_value("MT-PARITY-BASE");
    mt_base["metadata"] = json!({ "source": "parity_test" });

    let mut mt_mailbox = base_tracked_micro_task_value("MT-PARITY-MAILBOX");
    mt_mailbox["metadata"] = json!({
        "source": "parity_test",
        "has_pending_mailbox_wait": true,
    });

    run_locus_job(
        &state,
        "locus_register_mts_v1",
        json!({
            "wp_id": "WP-TEST",
            "micro_tasks": [mt_base, mt_mailbox],
        }),
    )
    .await?;

    // ── Base case: no mailbox wait ──
    let progress_base = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-PARITY-BASE" }),
    )
    .await?;
    let progress_meta_base = progress_base
        .get("metadata")
        .expect("progress metadata for base MT");

    let packet_path_base = gov_root
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-PARITY-BASE")
        .join("packet.json");
    let packet_base: Value = serde_json::from_slice(&std::fs::read(&packet_path_base)?)?;

    // workflow_state_family parity
    assert_eq!(
        progress_meta_base
            .get("workflow_state_family")
            .and_then(Value::as_str),
        packet_base
            .get("workflow_state_family")
            .and_then(Value::as_str),
        "workflow_state_family must match between progress metadata and emitted packet (base)"
    );

    // queue_reason_code parity
    assert_eq!(
        progress_meta_base
            .get("queue_reason_code")
            .and_then(Value::as_str),
        packet_base
            .get("queue_reason_code")
            .and_then(Value::as_str),
        "queue_reason_code must match between progress metadata and emitted packet (base)"
    );

    // allowed_action_ids parity
    assert_eq!(
        progress_meta_base.get("allowed_action_ids"),
        packet_base.get("allowed_action_ids"),
        "allowed_action_ids must match between progress metadata and emitted packet (base)"
    );

    // Verify base case uses canonical governed registry values
    let expected_base_actions = governed_action_ids_for_family(WorkflowStateFamily::Ready);
    let actual_base_actions = json_string_array_field(&packet_base, "allowed_action_ids");
    assert_eq!(
        actual_base_actions, expected_base_actions,
        "allowed_action_ids must come from governed registry"
    );

    // Base case should NOT have mailbox_response_wait
    assert_ne!(
        progress_meta_base
            .get("queue_reason_code")
            .and_then(Value::as_str),
        Some("mailbox_response_wait"),
        "base MT without mailbox wait should not have mailbox_response_wait reason"
    );

    // ── Mailbox wait case ──
    let progress_mailbox = run_locus_job(
        &state,
        "locus_get_mt_progress_v1",
        json!({ "mt_id": "MT-PARITY-MAILBOX" }),
    )
    .await?;
    let progress_meta_mailbox = progress_mailbox
        .get("metadata")
        .expect("progress metadata for mailbox MT");

    let packet_path_mailbox = gov_root
        .join("micro_tasks")
        .join("WP-TEST")
        .join("MT-PARITY-MAILBOX")
        .join("packet.json");
    let packet_mailbox: Value = serde_json::from_slice(&std::fs::read(&packet_path_mailbox)?)?;

    // workflow_state_family parity (mailbox)
    assert_eq!(
        progress_meta_mailbox
            .get("workflow_state_family")
            .and_then(Value::as_str),
        packet_mailbox
            .get("workflow_state_family")
            .and_then(Value::as_str),
        "workflow_state_family must match between progress metadata and emitted packet (mailbox)"
    );

    // queue_reason_code parity (mailbox)
    assert_eq!(
        progress_meta_mailbox
            .get("queue_reason_code")
            .and_then(Value::as_str),
        packet_mailbox
            .get("queue_reason_code")
            .and_then(Value::as_str),
        "queue_reason_code must match between progress metadata and emitted packet (mailbox)"
    );

    // allowed_action_ids parity (mailbox)
    assert_eq!(
        progress_meta_mailbox.get("allowed_action_ids"),
        packet_mailbox.get("allowed_action_ids"),
        "allowed_action_ids must match between progress metadata and emitted packet (mailbox)"
    );

    // Mailbox case MUST have queue_reason_code overridden to mailbox_response_wait
    assert_eq!(
        progress_meta_mailbox
            .get("queue_reason_code")
            .and_then(Value::as_str),
        Some("mailbox_response_wait"),
        "MT with has_pending_mailbox_wait=true must resolve to mailbox_response_wait"
    );

    // State family should be preserved (still Ready for pending status)
    assert_eq!(
        progress_meta_mailbox
            .get("workflow_state_family")
            .and_then(Value::as_str),
        Some("ready"),
        "mailbox wait must preserve base workflow_state_family"
    );

    Ok(())
}
