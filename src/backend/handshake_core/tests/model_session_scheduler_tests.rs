use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::{
    sqlite::SqliteDatabase, AccessMode, AiJob, Database, JobKind, JobMetrics, JobState,
    ModelSessionState, NewAiJob, SafetyMode, SessionMessageRole,
};
use handshake_core::workflows::{cancel_model_run_job, start_workflow_for_job};
use handshake_core::AppState;
use serde_json::{json, Value};
use uuid::Uuid;

struct EchoLlmClient {
    profile: ModelProfile,
}

impl EchoLlmClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new("model-session-test".to_string(), 4096),
        }
    }
}

#[async_trait]
impl LlmClient for EchoLlmClient {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: format!("assistant: {}", req.prompt),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            latency_ms: 1,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
    sqlite.run_migrations().await?;

    let flight_recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);
    let llm_client: Arc<dyn LlmClient> = Arc::new(EchoLlmClient::new());

    Ok(AppState {
        storage: sqlite.into_arc(),
        flight_recorder: flight_recorder.clone(),
        diagnostics: flight_recorder,
        llm_client,
        capability_registry: Arc::new(CapabilityRegistry::new()),
    })
}

fn hex64(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}

fn is_terminal_state(state: &JobState) -> bool {
    matches!(
        state,
        JobState::Completed
            | JobState::CompletedWithIssues
            | JobState::Failed
            | JobState::Cancelled
            | JobState::Poisoned
            | JobState::Stalled
    )
}

async fn wait_for_state(state: &AppState, job_id: Uuid, target: JobState, timeout_ms: u64) -> AiJob {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let job = state
            .storage
            .get_ai_job(job_id.to_string().as_str())
            .await
            .expect("job lookup");
        if job.state == target {
            return job;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for state {} for job {} (current={})",
            target.as_str(),
            job_id,
            job.state.as_str()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn wait_for_terminal_job(state: &AppState, job_id: Uuid, timeout_ms: u64) -> AiJob {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let job = state
            .storage
            .get_ai_job(job_id.to_string().as_str())
            .await
            .expect("job lookup");
        if is_terminal_state(&job.state) {
            return job;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for terminal state for job {} (current={})",
            job_id,
            job.state.as_str()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn create_model_run_job(
    state: &AppState,
    inputs: Value,
) -> Result<AiJob, Box<dyn std::error::Error>> {
    Ok(state
        .storage
        .create_ai_job(NewAiJob {
            trace_id: Uuid::new_v4(),
            job_kind: JobKind::ModelRun,
            protocol_id: "protocol-default".to_string(),
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
        .await?)
}

fn model_run_priority(job: &AiJob) -> i64 {
    job.job_inputs
        .as_ref()
        .and_then(|inputs| inputs.get("priority"))
        .and_then(Value::as_i64)
        .unwrap_or(100)
}

#[tokio::test]
async fn model_run_persists_session_and_artifact_first_messages(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = setup_state().await?;
    let session_id = format!("sess-{}", Uuid::new_v4());
    let assistant_artifact = format!("artifact:{session_id}:assistant");

    let job = create_model_run_job(
        &state,
        json!({
            "session_id": session_id,
            "lane": "PRIMARY",
            "priority": 5,
            "prompt": "persist-session",
            "model_id": "model-session-test",
            "backend": "local-test",
            "parameter_class": "default",
            "role": "assistant",
            "execution_mode": "STANDARD",
            "memory_policy": "EPHEMERAL",
            "assistant_content_artifact_id": assistant_artifact,
            "session_messages": [
                {
                    "message_id": format!("msg-{}", Uuid::new_v4()),
                    "role": "USER",
                    "content_hash": hex64('a'),
                    "content_artifact_id": format!("artifact:{session_id}:user-1")
                }
            ]
        }),
    )
    .await?;

    let run = start_workflow_for_job(&state, job.clone()).await?;
    assert!(matches!(run.status, JobState::Queued | JobState::Running));

    let final_job = wait_for_terminal_job(&state, job.job_id, 8_000).await;
    assert_eq!(final_job.state, JobState::Completed);

    let session = state.storage.get_model_session(&session_id).await?;
    assert_eq!(session.job_id, Some(job.job_id));
    assert_eq!(session.state, ModelSessionState::Completed);

    let session_by_job = state.storage.get_model_session_by_job_id(job.job_id).await?;
    assert_eq!(session_by_job.session_id, session_id);

    let messages = state.storage.list_session_messages(&session_by_job.session_id).await?;
    assert_eq!(messages.len(), 2);
    assert!(messages
        .iter()
        .any(|m| matches!(m.role, SessionMessageRole::User)));
    assert!(messages
        .iter()
        .any(|m| matches!(m.role, SessionMessageRole::Assistant)));
    for message in &messages {
        assert_eq!(message.content_hash.len(), 64);
        assert!(message.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!message.content_artifact_id.trim().is_empty());
    }

    let response_artifact = final_job
        .job_outputs
        .as_ref()
        .and_then(|v| v.get("response_ref"))
        .and_then(|v| v.get("content_artifact_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_artifact.contains("assistant"));

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job.job_id.to_string()),
            ..Default::default()
        })
        .await?;
    assert!(events
        .iter()
        .any(|e| matches!(e.event_type, FlightRecorderEventType::SessionSchedulerEnqueue)));
    assert!(events
        .iter()
        .any(|e| matches!(e.event_type, FlightRecorderEventType::SessionSchedulerDispatch)));
    for event in events.iter().filter(|event| {
        matches!(
            event.event_type,
            FlightRecorderEventType::SessionSchedulerEnqueue
                | FlightRecorderEventType::SessionSchedulerDispatch
                | FlightRecorderEventType::SessionSchedulerRateLimited
                | FlightRecorderEventType::SessionSchedulerCancelled
        )
    }) {
        assert!(event.payload.get("content").is_none());
        assert!(event.payload.get("text").is_none());
    }

    Ok(())
}

#[tokio::test]
async fn model_run_scheduler_queues_not_drop_and_dispatch_is_deterministic(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = setup_state().await?;

    let first = create_model_run_job(
        &state,
        json!({
            "session_id": format!("sess-{}", Uuid::new_v4()),
            "lane": "PRIMARY",
            "priority": 1,
            "prompt": "first",
            "simulate_duration_ms": 300,
            "model_id": "model-session-test",
            "backend": "local-test"
        }),
    )
    .await?;

    let second = create_model_run_job(
        &state,
        json!({
            "session_id": format!("sess-{}", Uuid::new_v4()),
            "lane": "PRIMARY",
            "priority": 1,
            "prompt": "second",
            "model_id": "model-session-test",
            "backend": "local-test"
        }),
    )
    .await?;

    let third = create_model_run_job(
        &state,
        json!({
            "session_id": format!("sess-{}", Uuid::new_v4()),
            "lane": "PRIMARY",
            "priority": 1,
            "prompt": "third",
            "model_id": "model-session-test",
            "backend": "local-test"
        }),
    )
    .await?;

    start_workflow_for_job(&state, first.clone()).await?;
    start_workflow_for_job(&state, second.clone()).await?;
    start_workflow_for_job(&state, third.clone()).await?;

    let done_first = wait_for_terminal_job(&state, first.job_id, 10_000).await;
    let done_second = wait_for_terminal_job(&state, second.job_id, 10_000).await;
    let done_third = wait_for_terminal_job(&state, third.job_id, 10_000).await;
    assert_eq!(done_first.state, JobState::Completed);
    assert_eq!(done_second.state, JobState::Completed);
    assert_eq!(done_third.state, JobState::Completed);

    let events = state
        .flight_recorder
        .list_events(EventFilter::default())
        .await?;

    let rate_limited_for_queued = events.iter().any(|event| {
        matches!(
            event.event_type,
            FlightRecorderEventType::SessionSchedulerRateLimited
        ) && matches!(
            event.job_id.as_deref(),
            Some(job_id)
                if job_id == second.job_id.to_string() || job_id == third.job_id.to_string()
        )
    });
    assert!(rate_limited_for_queued);

    let mut dispatch_events: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionSchedulerDispatch
            ) && matches!(
                event.job_id.as_deref(),
                Some(job_id)
                    if job_id == first.job_id.to_string()
                        || job_id == second.job_id.to_string()
                        || job_id == third.job_id.to_string()
            )
        })
        .collect();
    dispatch_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    assert_eq!(dispatch_events.len(), 3);
    let actual_dispatch_order: Vec<String> = dispatch_events
        .iter()
        .map(|event| event.job_id.clone().expect("dispatch job_id"))
        .collect();

    let mut expected_jobs = vec![first.clone(), second.clone(), third.clone()];
    expected_jobs.sort_by(|a, b| {
        model_run_priority(a)
            .cmp(&model_run_priority(b))
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.job_id.as_hyphenated().cmp(&b.job_id.as_hyphenated()))
    });
    let expected_dispatch_order: Vec<String> = expected_jobs
        .iter()
        .map(|job| job.job_id.to_string())
        .collect();
    assert_eq!(actual_dispatch_order, expected_dispatch_order);

    for job in [&first, &second, &third] {
        let has_enqueue = events.iter().any(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionSchedulerEnqueue
            ) && event.job_id.as_deref() == Some(job.job_id.to_string().as_str())
        });
        let has_dispatch = events.iter().any(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionSchedulerDispatch
            ) && event.job_id.as_deref() == Some(job.job_id.to_string().as_str())
        });
        assert!(has_enqueue);
        assert!(has_dispatch);
    }

    Ok(())
}

#[tokio::test]
async fn model_run_cancellation_is_cooperative_and_cancelled_not_failed(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = setup_state().await?;
    let session_id = format!("sess-{}", Uuid::new_v4());

    let job = create_model_run_job(
        &state,
        json!({
            "session_id": session_id,
            "lane": "PRIMARY",
            "priority": 10,
            "prompt": "cancel-me",
            "simulate_duration_ms": 500,
            "model_id": "model-session-test",
            "backend": "local-test"
        }),
    )
    .await?;

    start_workflow_for_job(&state, job.clone()).await?;
    wait_for_state(&state, job.job_id, JobState::Running, 5_000).await;

    cancel_model_run_job(
        &state,
        job.job_id,
        "operator".to_string(),
        "user_requested".to_string(),
    )
    .await?;

    let final_job = wait_for_terminal_job(&state, job.job_id, 10_000).await;
    assert_eq!(final_job.state, JobState::Cancelled);
    assert_ne!(final_job.state, JobState::Failed);

    let session = state.storage.get_model_session(&session_id).await?;
    assert_eq!(session.state, ModelSessionState::Cancelled);

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job.job_id.to_string()),
            ..Default::default()
        })
        .await?;
    let cancelled_event = events.iter().find(|event| {
        matches!(
            event.event_type,
            FlightRecorderEventType::SessionSchedulerCancelled
        )
    });
    assert!(cancelled_event.is_some());
    let cancelled_event = cancelled_event.expect("cancelled event present");
    assert_eq!(
        cancelled_event
            .payload
            .get("event_id")
            .and_then(Value::as_str),
        Some("FR-EVT-SESS-SCHED-004")
    );
    assert_eq!(
        cancelled_event
            .payload
            .get("cancelled_by")
            .and_then(Value::as_str),
        Some("operator")
    );
    assert_eq!(
        cancelled_event.payload.get("reason").and_then(Value::as_str),
        Some("user_requested")
    );

    Ok(())
}

#[test]
fn session_scheduler_event_payloads_are_validated() {
    let trace_id = Uuid::new_v4();

    let valid_payloads = vec![
        (
            FlightRecorderEventType::SessionSchedulerEnqueue,
            json!({
                "type": "session_scheduler_enqueue",
                "event_id": "FR-EVT-SESS-SCHED-001",
                "session_id": "sess-1",
                "job_id": Uuid::new_v4().to_string(),
                "job_kind": "model_run",
                "lane": "PRIMARY",
                "priority": 0,
                "concurrency_group": Value::Null,
                "queue_depth": 0,
                "attempt": 0,
                "max_retries": 0,
                "retry_backoff": "fixed",
                "cancellation_token": Value::Null
            }),
        ),
        (
            FlightRecorderEventType::SessionSchedulerDispatch,
            json!({
                "type": "session_scheduler_dispatch",
                "event_id": "FR-EVT-SESS-SCHED-002",
                "session_id": "sess-2",
                "job_id": Uuid::new_v4().to_string(),
                "job_kind": "model_run",
                "lane": "PRIMARY",
                "priority": 1,
                "concurrency_group": Value::Null,
                "queue_wait_ms": 12,
                "attempt": 1
            }),
        ),
        (
            FlightRecorderEventType::SessionSchedulerRateLimited,
            json!({
                "type": "session_scheduler_rate_limited",
                "event_id": "FR-EVT-SESS-SCHED-003",
                "session_id": "sess-3",
                "job_id": Uuid::new_v4().to_string(),
                "job_kind": "model_run",
                "lane": "PRIMARY",
                "priority": 2,
                "concurrency_group": Value::Null,
                "queue_wait_ms": 99,
                "attempt": 1,
                "backoff_ms": 1000,
                "reason": "concurrency_limit_exceeded_queued"
            }),
        ),
        (
            FlightRecorderEventType::SessionSchedulerCancelled,
            json!({
                "type": "session_scheduler_cancelled",
                "event_id": "FR-EVT-SESS-SCHED-004",
                "session_id": "sess-4",
                "job_id": Uuid::new_v4().to_string(),
                "job_kind": "model_run",
                "lane": "PRIMARY",
                "priority": 3,
                "concurrency_group": Value::Null,
                "attempt": 1,
                "cancelled_by": "operator",
                "reason": "user_requested"
            }),
        ),
    ];

    for (event_type, payload) in valid_payloads {
        let event = FlightRecorderEvent::new(
            event_type,
            FlightRecorderActor::System,
            trace_id,
            payload,
        );
        assert!(event.validate().is_ok());
    }

    let invalid_inline_content = FlightRecorderEvent::new(
        FlightRecorderEventType::SessionSchedulerEnqueue,
        FlightRecorderActor::System,
        trace_id,
        json!({
            "type": "session_scheduler_enqueue",
            "event_id": "FR-EVT-SESS-SCHED-001",
            "session_id": "sess-inline",
            "job_id": Uuid::new_v4().to_string(),
            "job_kind": "model_run",
            "lane": "PRIMARY",
            "priority": 0,
            "concurrency_group": Value::Null,
            "queue_depth": 0,
            "attempt": 0,
            "max_retries": 0,
            "retry_backoff": "fixed",
            "cancellation_token": Value::Null,
            "content": "forbidden"
        }),
    );
    assert!(invalid_inline_content.validate().is_err());
}
