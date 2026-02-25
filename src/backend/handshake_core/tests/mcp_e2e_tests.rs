use std::sync::Arc;
use std::time::Duration;

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::mcp::errors::McpError;
use handshake_core::mcp::gate::{
    canonical_mcp_tool_id, ConsentDecision, ConsentProvider, GateConfig, GatedMcpClient, McpContext,
    ToolPolicy, ToolRegistryEntry, ToolTransportBindings,
};
use handshake_core::mcp::jsonrpc::{JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcResponse};
use handshake_core::mcp::transport::duplex::DuplexTransport;
use handshake_core::storage::{AccessMode, AiJobListFilter, Database, JobKind, JobMetrics, JobState, JobStatusUpdate, NewAiJob, SafetyMode};
use serde_json::{json, Value};
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, DuplexStream};
use tokio::sync::oneshot;
use uuid::Uuid;

struct AllowAllConsent;

#[async_trait::async_trait]
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

async fn write_msg(
    writer: &mut BufWriter<tokio::io::WriteHalf<DuplexStream>>,
    msg: &JsonRpcMessage,
) {
    let line = serde_json::to_string(msg).expect("serialize jsonrpc");
    writer.write_all(line.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();
}

async fn stub_server_e2e(
    stream: DuplexStream,
    server_id: String,
    expected_job_id: String,
    ref_uri: String,
    token_tx: oneshot::Sender<String>,
    released_tx: oneshot::Sender<String>,
) {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();
    let mut writer = BufWriter::new(write_half);

    let mut token_tx = Some(token_tx);
    let mut released_tx = Some(released_tx);

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
                            "name": "echo_ref",
                            "description": "echo then return a ref:// resource",
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
                    let token = match &req.id {
                        JsonRpcId::String(s) => s.clone(),
                        JsonRpcId::Number(n) => n.to_string(),
                    };

                    let progress_token = req
                        .params
                        .as_ref()
                        .and_then(|v| v.get("progress_token"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if progress_token != token.as_str() {
                        write_msg(
                            &mut writer,
                            &JsonRpcMessage::Response(JsonRpcResponse::err(
                                req.id,
                                -32602,
                                "progress_token mismatch",
                                Some(json!({
                                    "id": token,
                                    "progress_token": progress_token
                                })),
                            )),
                        )
                        .await;
                        continue;
                    }

                    let job_id = req
                        .params
                        .as_ref()
                        .and_then(|v| v.get("job_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if job_id != expected_job_id.as_str() {
                        write_msg(
                            &mut writer,
                            &JsonRpcMessage::Response(JsonRpcResponse::err(
                                req.id,
                                -32602,
                                "job_id mismatch",
                                Some(json!({
                                    "job_id": job_id,
                                    "expected_job_id": expected_job_id,
                                })),
                            )),
                        )
                        .await;
                        continue;
                    }

                    let log = JsonRpcNotification::new(
                        "logging/message",
                        Some(json!({
                            "level": "INFO",
                            "logger": "stub",
                            "message": "stub tool called",
                            "context": {
                                "session_id": "sess-1",
                                "task_id": "task-1",
                                "job_id": job_id,
                                "workflow_run_id": "wf-1"
                            },
                            "fields": {
                                "server_id": server_id.as_str(),
                                "tool_name": "echo_ref"
                            }
                        })),
                    );
                    write_msg(&mut writer, &JsonRpcMessage::Notification(log)).await;

                    let progress = JsonRpcNotification::new(
                        "notifications/progress",
                        Some(json!({
                            "token": token,
                            "progress": 45,
                            "message": "working..."
                        })),
                    );
                    write_msg(&mut writer, &JsonRpcMessage::Notification(progress)).await;

                    if let Some(tx) = token_tx.take() {
                        let _ = tx.send(progress_token.to_string());
                    }

                    let args = req
                        .params
                        .as_ref()
                        .and_then(|v| v.get("arguments"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let echoed = args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let result = json!({
                        "echoed": echoed,
                        "resource": {
                            "uri": ref_uri,
                            "mimeType": "application/octet-stream"
                        }
                    });
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
            JsonRpcMessage::Notification(notif) => {
                if notif.method == "notifications/resource_released" {
                    let uri = notif
                        .params
                        .as_ref()
                        .and_then(|v| v.get("uri"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if let Some(tx) = released_tx.take() {
                        let _ = tx.send(uri);
                    }
                    break;
                }
            }
            JsonRpcMessage::Response(_resp) => {
                // server should not receive responses from client in this test
            }
        }
    }
}

#[tokio::test]
async fn mcp_e2e_persists_progress_mapping_records_fr_events_and_hydrates_ref() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());

    let sqlite = handshake_core::storage::sqlite::SqliteDatabase::connect("sqlite::memory:", 1).await?;
    let db: Arc<dyn Database> = Arc::new(sqlite);
    db.run_migrations().await?;

    let trace_id = Uuid::new_v4();
    let job = db
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
            job_inputs: Some(json!({ "purpose": "mcp_e2e" })),
        })
        .await?;

    let workflow_run_id = Uuid::new_v4();
    db.update_ai_job_status(JobStatusUpdate {
        job_id: job.job_id,
        state: JobState::Running,
        error_message: None,
        status_reason: "running".to_string(),
        metrics: None,
        workflow_run_id: Some(workflow_run_id),
        trace_id: None,
        job_outputs: None,
    })
    .await?;

    let tmp = tempdir()?;
    let fixture_path = tmp.path().join("fixture.bin");
    std::fs::write(&fixture_path, b"hello world")?;
    let ref_uri = "ref://fixture.bin".to_string();

    let ctx = McpContext {
        job_id: Some(job.job_id),
        trace_id,
        session_id: Some("sess-1".to_string()),
        task_id: Some("task-1".to_string()),
        workflow_run_id: Some(workflow_run_id.to_string()),
        granted_capabilities: vec!["fs.read".to_string()],
        access_mode: AccessMode::AnalysisOnly,
        human_consent_obtained: false,
        agentic_mode_enabled: false,
        allowed_roots: vec![tmp.path().to_path_buf()],
    };

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let (token_tx, token_rx) = oneshot::channel::<String>();
    let (released_tx, released_rx) = oneshot::channel::<String>();
    let server_id = "stub-mcp";
    tokio::spawn(stub_server_e2e(
        server_stream,
        server_id.to_string(),
        job.job_id.to_string(),
        ref_uri.clone(),
        token_tx,
        released_tx,
    ));

    let transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    let entry = ToolRegistryEntry {
        tool_id: canonical_mcp_tool_id(server_id, "echo_ref"),
        tool_version: "1.0.0".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"],
            "additionalProperties": false
        }),
        output_schema: None,
        side_effect: "READ".to_string(),
        idempotency: "IDEMPOTENT".to_string(),
        determinism: "DETERMINISTIC".to_string(),
        availability: "AVAILABLE".to_string(),
        required_capabilities: vec!["fs.read".to_string()],
        transport_bindings: ToolTransportBindings {
            mcp_name: "echo_ref".to_string(),
        },
    };
    let tool_id = entry.tool_id.clone();
    gate.tool_registry.push(entry);
    gate.allowed_tools = Some([tool_id.clone()].into_iter().collect());
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
        flight_recorder.clone(),
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
        Arc::clone(&db),
    )
    .await?;

    client.refresh_tools().await?;
    let result = client
        .tools_call(ctx.clone(), tool_id.as_str(), json!({ "message": "hi" }))
        .await?;

    let token = tokio::time::timeout(Duration::from_secs(2), token_rx).await??;

    let fields = db.get_ai_job_mcp_fields(job.job_id).await?;
    assert_eq!(fields.mcp_server_id.as_deref(), Some("stub-mcp"));
    assert_eq!(fields.mcp_call_id.as_deref(), Some(token.as_str()));
    assert_eq!(fields.mcp_progress_token.as_deref(), Some(token.as_str()));

    assert_eq!(
        db.find_ai_job_id_by_mcp_progress_token(&token).await?,
        Some(job.job_id)
    );

    let uri = result
        .get("resource")
        .and_then(|v| v.get("uri"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(uri, ref_uri.as_str());

    let bytes = client.resolve_ref_uri(&ctx, uri)?;
    assert_eq!(bytes, b"hello world".to_vec());

    let released_uri = tokio::time::timeout(Duration::from_secs(2), released_rx).await??;
    assert_eq!(released_uri, ref_uri);

    assert!(matches!(
        client.resolve_ref_uri(&ctx, "file://fixture.bin"),
        Err(McpError::SecurityViolation(_))
    ));

    let conn_handle = recorder.connection();
    let conn = match conn_handle.lock() {
        Ok(conn) => conn,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut stmt = conn.prepare(
        "SELECT event_kind, payload FROM fr_events WHERE job_id = ? ORDER BY event_id ASC",
    )?;
    let rows = stmt.query_map(duckdb::params![job.job_id.to_string()], |row| {
        let kind: String = row.get(0)?;
        let payload: Option<String> = row.get(1)?;
        Ok((kind, payload))
    })?;

    let mut kinds = Vec::new();
    let mut progress_payloads = Vec::new();
    for row in rows {
        let (kind, payload) = row?;
        if kind == "mcp.progress" {
            let payload = payload.unwrap_or("null".to_string());
            progress_payloads.push(serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null));
        }
        kinds.push(kind);
    }

    assert!(kinds.iter().any(|k| k == "mcp.tool_call"), "missing mcp.tool_call");
    assert!(
        kinds.iter().any(|k| k == "mcp.tool_result"),
        "missing mcp.tool_result"
    );
    assert!(kinds.iter().any(|k| k == "mcp.progress"), "missing mcp.progress");
    assert!(kinds.iter().any(|k| k == "mcp.logging"), "missing mcp.logging");

    assert!(
        progress_payloads
            .iter()
            .any(|p| p.get("token").and_then(|v| v.as_str()) == Some(token.as_str())),
        "missing mcp.progress payload token"
    );

    // Sanity: no jobs leaked by token mapping.
    let all_jobs = db.list_ai_jobs(AiJobListFilter::default()).await?;
    assert_eq!(all_jobs.len(), 1);

    Ok(())
}
