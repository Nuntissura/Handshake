use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{EventFilter, FlightRecorderEventType};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::mcp::gate::{
    canonical_mcp_tool_id, ConsentDecision, ConsentProvider, GateConfig, GatedMcpClient,
    McpContext, ToolPolicy, ToolRegistryEntry, ToolTransportBindings,
};
use handshake_core::mcp::jsonrpc::{JsonRpcMessage, JsonRpcResponse};
use handshake_core::mcp::transport::duplex::DuplexTransport;
use handshake_core::storage::{
    sqlite::SqliteDatabase, AccessMode, AiJob, Database, JobKind, JobMetrics, JobState, NewAiJob,
    SafetyMode,
};
use handshake_core::workflows::{start_workflow_for_job, SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, DuplexStream};
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

struct AllowAllConsent;

#[async_trait]
impl ConsentProvider for AllowAllConsent {
    async fn request_consent(
        &self,
        _ctx: &McpContext,
        _server_id: &str,
        _tool_name: &str,
        _capability_id: Option<&str>,
    ) -> ConsentDecision {
        ConsentDecision::Allow
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
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
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

async fn write_msg(
    writer: &mut BufWriter<tokio::io::WriteHalf<DuplexStream>>,
    msg: &JsonRpcMessage,
) {
    let line = serde_json::to_string(msg).expect("serialize jsonrpc");
    writer.write_all(line.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();
}

async fn stub_server_echo(stream: DuplexStream) {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();
    let mut writer = BufWriter::new(write_half);

    while let Ok(Some(line)) = lines.next_line().await {
        let msg: JsonRpcMessage = serde_json::from_str(&line).expect("parse jsonrpc");
        match msg {
            JsonRpcMessage::Request(req) => match req.method.as_str() {
                "tools/list" => {
                    let schema = json!({
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" }
                        },
                        "required": ["message"],
                        "additionalProperties": false
                    });
                    let result = json!({
                        "tools": [{
                            "name": "echo",
                            "description": "echo a string",
                            "inputSchema": schema
                        }]
                    });
                    write_msg(
                        &mut writer,
                        &JsonRpcMessage::Response(JsonRpcResponse::ok(req.id, result)),
                    )
                    .await;
                }
                "resources/list" => {
                    let result = json!({ "resources": [] });
                    write_msg(
                        &mut writer,
                        &JsonRpcMessage::Response(JsonRpcResponse::ok(req.id, result)),
                    )
                    .await;
                }
                "tools/call" => {
                    let args = req
                        .params
                        .as_ref()
                        .and_then(|value| value.get("arguments"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let echoed = args
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let result = json!({ "echoed": echoed });
                    write_msg(
                        &mut writer,
                        &JsonRpcMessage::Response(JsonRpcResponse::ok(req.id, result)),
                    )
                    .await;
                }
                _ => {
                    write_msg(
                        &mut writer,
                        &JsonRpcMessage::Response(JsonRpcResponse::err(
                            req.id,
                            -32601,
                            "method not found",
                            None,
                        )),
                    )
                    .await;
                }
            },
            JsonRpcMessage::Notification(_) | JsonRpcMessage::Response(_) => {}
        }
    }
}

fn echo_tool_registry_entry(server_id: &str) -> ToolRegistryEntry {
    let schema = json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" }
        },
        "required": ["message"],
        "additionalProperties": false
    });

    ToolRegistryEntry {
        tool_id: canonical_mcp_tool_id(server_id, "echo"),
        tool_version: "1.0.0".to_string(),
        input_schema: schema,
        output_schema: None,
        side_effect: "READ".to_string(),
        idempotency: "IDEMPOTENT".to_string(),
        determinism: "DETERMINISTIC".to_string(),
        availability: "AVAILABLE".to_string(),
        required_capabilities: vec!["fs.read".to_string()],
        transport_bindings: ToolTransportBindings {
            mcp_name: "echo".to_string(),
        },
    }
}

#[tokio::test]
async fn session_observability_spans_bind_model_runs_and_tool_calls(
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
            "prompt": "span-test",
            "model_id": "model-session-test",
            "backend": "local-test",
            "parameter_class": "default",
            "role": "assistant",
            "execution_mode": "STANDARD",
            "memory_policy": "EPHEMERAL",
            "capability_grants": ["fs.read"],
            "assistant_content_artifact_id": assistant_artifact,
            "estimated_cost_usd": 1.25,
            "max_tokens_budget": 10,
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

    let workflow_run_id = final_job
        .workflow_run_id
        .expect("workflow_run_id")
        .to_string();

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_id = "span-test-mcp";
    tokio::spawn(stub_server_echo(server_stream));

    let transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    let echo_entry = echo_tool_registry_entry(server_id);
    let tool_id = echo_entry.tool_id.clone();
    gate.tool_registry.push(echo_entry);
    gate.tool_policies.insert(
        tool_id.clone(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect_with_db(
        server_id,
        transport,
        Arc::clone(&state.flight_recorder),
        Arc::clone(&state.capability_registry),
        Arc::new(AllowAllConsent),
        gate,
        false,
        Arc::clone(&state.storage),
    )
    .await?;

    client.refresh_tools().await?;
    let tool_result = client
        .tools_call(
            McpContext {
                job_id: Some(job.job_id),
                trace_id: final_job.trace_id,
                session_id: Some(session_id.clone()),
                task_id: Some("task-1".to_string()),
                workflow_run_id: Some(workflow_run_id),
                granted_capabilities: vec!["fs.read".to_string()],
                access_mode: AccessMode::AnalysisOnly,
                human_consent_obtained: false,
                agentic_mode_enabled: false,
                allowed_roots: Vec::new(),
            },
            tool_id.as_str(),
            json!({ "message": "hi" }),
        )
        .await?;
    assert_eq!(
        tool_result.get("echoed").and_then(Value::as_str),
        Some("hi")
    );

    let events = state
        .flight_recorder
        .list_events(EventFilter {
            job_id: Some(job.job_id.to_string()),
            model_session_id: Some(session_id.clone()),
            ..Default::default()
        })
        .await?;

    let session_created = events
        .iter()
        .find(|event| matches!(event.event_type, FlightRecorderEventType::SessionCreated))
        .expect("session.created event");
    assert_eq!(
        session_created.model_session_id.as_deref(),
        Some(session_id.as_str())
    );
    let session_span = session_created
        .session_span_id
        .clone()
        .expect("session span id");
    assert!(session_created.activity_span_id.is_none());

    let scheduler_enqueue = events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionSchedulerEnqueue
            )
        })
        .expect("session scheduler enqueue");
    let scheduler_dispatch = events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionSchedulerDispatch
            )
        })
        .expect("session scheduler dispatch");
    let model_run_span = scheduler_dispatch
        .activity_span_id
        .clone()
        .expect("model run activity span");
    assert_eq!(
        scheduler_enqueue.model_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        scheduler_enqueue.session_span_id.as_deref(),
        Some(session_span.as_str())
    );
    assert_eq!(
        scheduler_enqueue.activity_span_id.as_deref(),
        Some(model_run_span.as_str())
    );
    assert_eq!(
        scheduler_dispatch.session_span_id.as_deref(),
        Some(session_span.as_str())
    );
    assert_eq!(
        scheduler_dispatch.activity_span_id.as_deref(),
        Some(model_run_span.as_str())
    );

    let created_to_active = events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionStateChange
            ) && event.payload.get("from_state").and_then(Value::as_str) == Some("CREATED")
                && event.payload.get("to_state").and_then(Value::as_str) == Some("ACTIVE")
        })
        .expect("created->active state change");
    let active_to_completed = events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionStateChange
            ) && event.payload.get("from_state").and_then(Value::as_str) == Some("ACTIVE")
                && event.payload.get("to_state").and_then(Value::as_str) == Some("COMPLETED")
        })
        .expect("active->completed state change");
    for state_change in [created_to_active, active_to_completed] {
        assert_eq!(
            state_change.model_session_id.as_deref(),
            Some(session_id.as_str())
        );
        assert_eq!(
            state_change.session_span_id.as_deref(),
            Some(session_span.as_str())
        );
        assert_eq!(
            state_change.activity_span_id.as_deref(),
            Some(model_run_span.as_str())
        );
    }

    let session_messages: Vec<_> = events
        .iter()
        .filter(|event| matches!(event.event_type, FlightRecorderEventType::SessionMessage))
        .collect();
    assert_eq!(session_messages.len(), 2);
    for message_event in session_messages {
        assert_eq!(
            message_event.model_session_id.as_deref(),
            Some(session_id.as_str())
        );
        assert_eq!(
            message_event.session_span_id.as_deref(),
            Some(session_span.as_str())
        );
        assert_eq!(
            message_event.activity_span_id.as_deref(),
            Some(model_run_span.as_str())
        );
    }

    let session_completed = events
        .iter()
        .find(|event| matches!(event.event_type, FlightRecorderEventType::SessionCompleted))
        .expect("session completed event");
    assert_eq!(
        session_completed.model_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        session_completed.session_span_id.as_deref(),
        Some(session_span.as_str())
    );
    assert_eq!(
        session_completed.activity_span_id.as_deref(),
        Some(model_run_span.as_str())
    );
    assert_eq!(
        session_completed
            .payload
            .get("total_tokens")
            .and_then(Value::as_u64),
        Some(15)
    );
    assert_eq!(
        session_completed
            .payload
            .get("messages_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        session_completed
            .payload
            .get("total_cost_usd")
            .and_then(Value::as_f64),
        Some(1.25)
    );

    let budget_warning = events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                FlightRecorderEventType::SessionBudgetWarning
            )
        })
        .expect("session budget warning event");
    assert_eq!(
        budget_warning.model_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        budget_warning.session_span_id.as_deref(),
        Some(session_span.as_str())
    );
    assert_eq!(
        budget_warning.activity_span_id.as_deref(),
        Some(model_run_span.as_str())
    );
    assert_eq!(
        budget_warning
            .payload
            .get("current_value")
            .and_then(Value::as_f64),
        Some(15.0)
    );
    assert_eq!(
        budget_warning
            .payload
            .get("threshold_value")
            .and_then(Value::as_f64),
        Some(10.0)
    );

    let tool_call = events
        .iter()
        .find(|event| {
            matches!(event.event_type, FlightRecorderEventType::ToolCall)
                && event.payload.get("ok").and_then(Value::as_bool) == Some(true)
        })
        .expect("tool call event");
    assert_eq!(
        tool_call.model_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(
        tool_call.session_span_id.as_deref(),
        Some(session_span.as_str())
    );
    let tool_span = tool_call
        .activity_span_id
        .clone()
        .expect("tool activity span");
    assert_ne!(tool_span, model_run_span);
    assert_eq!(
        tool_call
            .payload
            .get("parent_span_id")
            .and_then(Value::as_str),
        Some(model_run_span.as_str())
    );

    Ok(())
}
