use std::sync::Arc;
use std::time::Duration;

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::mcp::errors::McpError;
use handshake_core::mcp::gate::{
    ConsentDecision, ConsentProvider, GateConfig, GatedMcpClient, McpContext, ToolPolicy,
};
use handshake_core::mcp::jsonrpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use handshake_core::mcp::security::canonicalize_under_roots;
use handshake_core::mcp::transport::duplex::DuplexTransport;
use handshake_core::mcp::transport::stdio::StdioTransport;
use handshake_core::mcp::transport::McpTransport;
use handshake_core::storage::AccessMode;
use serde_json::{json, Value};
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

struct DenyAllConsent;

#[async_trait::async_trait]
impl ConsentProvider for DenyAllConsent {
    async fn request_consent(
        &self,
        _ctx: &McpContext,
        _server_id: &str,
        _tool_name: &str,
        _capability_id: Option<&str>,
    ) -> ConsentDecision {
        ConsentDecision::Deny
    }
}

struct SlowConsent(Duration);

#[async_trait::async_trait]
impl ConsentProvider for SlowConsent {
    async fn request_consent(
        &self,
        _ctx: &McpContext,
        _server_id: &str,
        _tool_name: &str,
        _capability_id: Option<&str>,
    ) -> ConsentDecision {
        tokio::time::sleep(self.0).await;
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

async fn stub_server_basic(stream: DuplexStream, server_id: String, job_id: String) {
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
                            "message": { "type": "string" },
                            "path": { "type": "string" }
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
                    let log = JsonRpcNotification::new(
                        "logging/message",
                        Some(json!({
                            "level": "INFO",
                            "logger": "stub",
                            "message": "stub tool called",
                            "context": {
                                "session_id": "sess-1",
                                "task_id": "task-1",
                                "job_id": job_id.as_str(),
                                "workflow_run_id": "wf-1"
                            },
                            "fields": {
                                "server_id": server_id.as_str(),
                                "tool_name": "echo"
                            }
                        })),
                    );
                    write_msg(&mut writer, &JsonRpcMessage::Notification(log)).await;

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
            JsonRpcMessage::Notification(_notif) => {
                // ignore
            }
            JsonRpcMessage::Response(_resp) => {
                // server should not receive responses from client in these tests
            }
        }
    }
}

async fn stub_server_logging_custom_kind(stream: DuplexStream, server_id: String, job_id: String) {
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
                    let log = JsonRpcNotification::new(
                        "logging/message",
                        Some(json!({
                            "level": "INFO",
                            "logger": "stub",
                            "message": "stub tool called (custom event kind)",
                            "context": {
                                "session_id": "sess-1",
                                "task_id": "task-1",
                                "job_id": job_id.as_str(),
                                "workflow_run_id": "wf-1"
                            },
                            "fields": {
                                "event_kind": "stub.custom",
                                "server_id": server_id.as_str(),
                                "tool_name": "echo"
                            }
                        })),
                    );
                    write_msg(&mut writer, &JsonRpcMessage::Notification(log)).await;

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
            JsonRpcMessage::Notification(_) => {}
            JsonRpcMessage::Response(_) => {}
        }
    }
}

async fn stub_server_hang_until_cancelled(stream: DuplexStream, cancelled_tx: oneshot::Sender<()>) {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();
    let mut writer = BufWriter::new(write_half);

    let mut pending_call_id: Option<Value> = None;
    let mut cancelled_tx = Some(cancelled_tx);

    while let Ok(Some(line)) = lines.next_line().await {
        let msg: JsonRpcMessage = serde_json::from_str(&line).expect("parse jsonrpc");
        match msg {
            JsonRpcMessage::Request(req) => match req.method.as_str() {
                "tools/list" => {
                    let schema = json!({
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" },
                            "path": { "type": "string" }
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
                "tools/call" => {
                    pending_call_id = Some(req.id.to_value());
                    // Intentionally do not respond; wait for notifications/cancelled.
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
                if notif.method == "notifications/cancelled" {
                    let request_id = notif
                        .params
                        .as_ref()
                        .and_then(|p| p.get("requestId"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    if Some(request_id) == pending_call_id {
                        if let Some(tx) = cancelled_tx.take() {
                            let _ = tx.send(());
                        }
                        break;
                    }
                }
            }
            JsonRpcMessage::Response(_resp) => {}
        }
    }
}

fn make_ctx(
    job_id: Uuid,
    trace_id: Uuid,
    granted: Vec<String>,
    access_mode: AccessMode,
) -> McpContext {
    McpContext {
        job_id: Some(job_id),
        trace_id,
        session_id: Some("sess-1".to_string()),
        task_id: Some("task-1".to_string()),
        workflow_run_id: Some("wf-1".to_string()),
        granted_capabilities: granted,
        access_mode,
        human_consent_obtained: false,
        agentic_mode_enabled: false,
        allowed_roots: Vec::new(),
    }
}

#[tokio::test]
async fn mcp_tool_call_records_fr_events_and_logging() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());

    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::AnalysisOnly,
    );

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_id = "stub-mcp";
    tokio::spawn(stub_server_basic(
        server_stream,
        server_id.to_string(),
        job_id_str.clone(),
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        server_id,
        &mut transport,
        flight_recorder.clone(),
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;

    client.refresh_tools().await?;
    let result = client
        .tools_call(ctx, "echo", json!({ "message": "hi" }))
        .await?;
    assert_eq!(result.get("echoed").and_then(|v| v.as_str()), Some("hi"));

    let conn_handle = recorder.connection();
    let conn = match conn_handle.lock() {
        Ok(conn) => conn,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut stmt = conn.prepare(
        "SELECT event_kind, job_id, payload FROM fr_events WHERE job_id = ? ORDER BY event_id ASC",
    )?;
    let rows = stmt.query_map(duckdb::params![job_id_str.clone()], |row| {
        let kind: String = row.get(0)?;
        let job_id: Option<String> = row.get(1)?;
        let payload: Option<String> = row.get(2)?;
        Ok((kind, job_id, payload))
    })?;

    let mut kinds = Vec::new();
    let mut payloads = Vec::new();
    for row in rows {
        let (kind, jid, payload_str) = row?;
        assert_eq!(jid.as_deref(), Some(job_id_str.as_str()));
        kinds.push(kind);
        if let Some(payload_str) = payload_str {
            payloads.push(serde_json::from_str::<Value>(&payload_str).unwrap_or(Value::Null));
        }
    }

    assert!(kinds.iter().any(|k| k == "tool.call"), "missing tool.call");
    assert!(
        kinds.iter().any(|k| k == "tool.result"),
        "missing tool.result"
    );
    assert!(
        kinds.iter().any(|k| k == "mcp.logging"),
        "missing mcp.logging"
    );

    let required = [
        "tool_name",
        "tool_version",
        "inputs",
        "outputs",
        "status",
        "duration_ms",
        "error_code",
        "job_id",
        "workflow_run_id",
        "trace_id",
        "capability_id",
    ];
    for payload in payloads
        .iter()
        .filter(|p| p.get("tool").is_some() || p.get("tool_name").is_some())
    {
        for key in required {
            assert!(
                payload.get(key).is_some(),
                "missing required payload key: {} (payload={})",
                key,
                payload
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn mcp_logging_message_custom_event_kind_creates_breadcrumb() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());

    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::AnalysisOnly,
    );

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    let server_id = "stub-mcp";
    tokio::spawn(stub_server_logging_custom_kind(
        server_stream,
        server_id.to_string(),
        job_id_str.clone(),
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        server_id,
        &mut transport,
        flight_recorder,
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;

    client.refresh_tools().await?;
    let _ = client
        .tools_call(ctx, "echo", json!({ "message": "hi" }))
        .await?;

    let conn_handle = recorder.connection();
    let conn = match conn_handle.lock() {
        Ok(conn) => conn,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut stmt = conn.prepare(
        "SELECT event_kind, source, job_id FROM fr_events WHERE message = ? ORDER BY event_id ASC",
    )?;
    let rows = stmt.query_map(
        duckdb::params!["stub tool called (custom event kind)"],
        |row| {
            let kind: String = row.get(0)?;
            let source: String = row.get(1)?;
            let job_id: Option<String> = row.get(2)?;
            Ok((kind, source, job_id))
        },
    )?;

    let mut found_custom = false;
    let mut found_breadcrumb = false;
    for row in rows {
        let (kind, source, job_id) = row?;
        if kind == "stub.custom" && job_id.as_deref() == Some(job_id_str.as_str()) {
            found_custom = true;
        }
        if kind == "mcp.logging" && source == server_id && job_id.is_none() {
            found_breadcrumb = true;
        }
    }

    assert!(found_custom, "missing stub.custom");
    assert!(found_breadcrumb, "missing mcp.logging breadcrumb");

    Ok(())
}

#[tokio::test]
async fn mcp_schema_validation_failure_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());
    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::AnalysisOnly,
    );

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_basic(
        server_stream,
        "stub-mcp".to_string(),
        job_id_str,
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        flight_recorder,
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;
    client.refresh_tools().await?;

    let err = client
        .tools_call(ctx, "echo", json!({ "message": 123 }))
        .await
        .expect_err("expected schema validation error");
    assert!(matches!(err, McpError::SchemaValidation { .. }));
    Ok(())
}

#[tokio::test]
async fn mcp_capability_denied_blocks_tool_call() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());
    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(job_id, trace_id, vec![], AccessMode::AnalysisOnly);

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_basic(
        server_stream,
        "stub-mcp".to_string(),
        job_id_str,
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        flight_recorder,
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;
    client.refresh_tools().await?;

    let err = client
        .tools_call(ctx, "echo", json!({ "message": "hi" }))
        .await
        .expect_err("expected capability denied");
    assert!(matches!(err, McpError::CapabilityDenied(_)));
    Ok(())
}

#[tokio::test]
async fn mcp_consent_deny_and_timeout_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());
    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::ApplyScoped,
    );

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_basic(
        server_stream,
        "stub-mcp".to_string(),
        job_id_str.clone(),
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.consent_timeout = Duration::from_millis(10);
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: true,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        flight_recorder,
        registry.clone(),
        Arc::new(DenyAllConsent),
        gate.clone(),
        false,
    )
    .await?;
    client.refresh_tools().await?;
    let err = client
        .tools_call(ctx.clone(), "echo", json!({ "message": "hi" }))
        .await
        .expect_err("expected consent denied");
    assert!(matches!(err, McpError::ConsentDenied(_)));

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_basic(
        server_stream,
        "stub-mcp".to_string(),
        job_id_str,
    ));
    let mut transport = DuplexTransport::new(client_stream);
    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        Arc::new(DuckDbFlightRecorder::new_in_memory(7)?),
        registry,
        Arc::new(SlowConsent(Duration::from_millis(100))),
        gate,
        false,
    )
    .await?;
    client.refresh_tools().await?;
    let err = client
        .tools_call(ctx, "echo", json!({ "message": "hi" }))
        .await
        .expect_err("expected consent timeout");
    assert!(matches!(err, McpError::ConsentDenied(_)));

    Ok(())
}

#[tokio::test]
async fn mcp_timeout_sends_notifications_cancelled() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());
    let job_id = Uuid::new_v4();
    let trace_id = Uuid::new_v4();
    let ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::AnalysisOnly,
    );

    let (cancelled_tx, cancelled_rx) = oneshot::channel::<()>();
    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_hang_until_cancelled(
        server_stream,
        cancelled_tx,
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.request_timeout = Duration::from_millis(30);
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: None,
        },
    );

    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        flight_recorder,
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;
    client.refresh_tools().await?;

    let err = client
        .tools_call(ctx, "echo", json!({ "message": "hi" }))
        .await
        .expect_err("expected timeout");
    assert!(matches!(err, McpError::Timeout(_)));

    tokio::time::timeout(Duration::from_secs(1), cancelled_rx)
        .await
        .expect("expected cancel notification")
        .expect("cancel receiver dropped");

    Ok(())
}

#[tokio::test]
async fn mcp_path_escape_and_symlink_are_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let registry = Arc::new(CapabilityRegistry::new());
    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();
    let trace_id = Uuid::new_v4();

    let root = tempfile::tempdir()?;
    let root_path = root.path().to_path_buf();
    let inside = root.path().join("inside.txt");
    std::fs::write(&inside, b"ok")?;

    let mut ctx = make_ctx(
        job_id,
        trace_id,
        vec!["fs.read".to_string()],
        AccessMode::AnalysisOnly,
    );
    ctx.allowed_roots = vec![root_path.clone()];

    let (client_stream, server_stream) = tokio::io::duplex(64 * 1024);
    tokio::spawn(stub_server_basic(
        server_stream,
        "stub-mcp".to_string(),
        job_id_str,
    ));

    let mut transport = DuplexTransport::new(client_stream);
    let mut gate = GateConfig::minimal();
    gate.tool_policies.insert(
        "echo".to_string(),
        ToolPolicy {
            required_capability: Some("fs.read".to_string()),
            requires_consent: false,
            path_argument: Some("path".to_string()),
        },
    );

    let client = GatedMcpClient::connect(
        "stub-mcp",
        &mut transport,
        flight_recorder,
        registry,
        Arc::new(AllowAllConsent),
        gate,
        false,
    )
    .await?;
    client.refresh_tools().await?;

    // Allowed path under root.
    let ok = client
        .tools_call(
            ctx.clone(),
            "echo",
            json!({ "message": "hi", "path": inside.to_string_lossy() }),
        )
        .await?;
    assert!(ok.get("echoed").is_some());

    // Path traversal blocked.
    let err = client
        .tools_call(
            ctx.clone(),
            "echo",
            json!({ "message": "hi", "path": "../escape.txt" }),
        )
        .await
        .expect_err("expected traversal blocked");
    assert!(matches!(err, McpError::SecurityViolation(_)));

    // Gate decision is recorded for security denials.
    let conn_handle = recorder.connection();
    let conn = match conn_handle.lock() {
        Ok(conn) => conn,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM fr_events WHERE job_id = ? AND event_kind = 'mcp.gate.decision'",
    )?;
    let decision_count: i64 =
        stmt.query_row(duckdb::params![job_id.to_string()], |row| row.get(0))?;
    assert!(
        decision_count >= 1,
        "expected at least one mcp.gate.decision row"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside_dir = tempfile::tempdir()?;
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, b"secret")?;
        let link = root.path().join("link.txt");
        symlink(&outside_file, &link)?;

        let err = client
            .tools_call(
                ctx,
                "echo",
                json!({ "message": "hi", "path": link.to_string_lossy() }),
            )
            .await
            .expect_err("expected symlink blocked");
        assert!(matches!(err, McpError::SecurityViolation(_)));
    }

    Ok(())
}

#[tokio::test]
async fn stdio_transport_roundtrips_jsonrpc_and_rejects_double_connect(
) -> Result<(), Box<dyn std::error::Error>> {
    let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("cmd", vec!["/c", "more"])
    } else {
        ("cat", vec![])
    };

    let mut transport = StdioTransport::new(cmd, args.into_iter().map(|s| s.to_string()).collect());
    let mut connected = transport.connect().await?;

    connected.io.outgoing.send(JsonRpcMessage::Notification(
        JsonRpcNotification::new("ping", Some(json!({ "n": 1 }))),
    ))?;

    let echoed = tokio::time::timeout(Duration::from_secs(2), connected.io.incoming.recv())
        .await
        .expect("expected echoed jsonrpc message")
        .expect("transport closed");

    match echoed {
        JsonRpcMessage::Notification(n) => {
            assert_eq!(n.method, "ping");
            assert_eq!(n.params.as_ref().and_then(|p| p.get("n")), Some(&json!(1)));
        }
        other => panic!("unexpected echoed message: {other:?}"),
    }

    let err = match transport.connect().await {
        Ok(_) => panic!("expected double connect error"),
        Err(e) => e,
    };
    assert!(matches!(err, McpError::Transport(_)));

    drop(connected);
    drop(transport);
    Ok(())
}

#[tokio::test]
async fn duplex_transport_rejects_double_connect() {
    let (client_stream, _server_stream) = tokio::io::duplex(1024);
    let mut transport = DuplexTransport::new(client_stream);
    let _connected = transport.connect().await.expect("connect ok");
    let err = match transport.connect().await {
        Ok(_) => panic!("expected double connect error"),
        Err(e) => e,
    };
    assert!(matches!(err, McpError::Transport(_)));
}

#[test]
fn jsonrpc_helpers_cover_constructors_and_into_result() {
    let id_num = JsonRpcId::Number(7);
    assert_eq!(id_num.to_value(), json!(7));
    let id_str = JsonRpcId::String("x".to_string());
    assert_eq!(id_str.to_value(), json!("x"));

    let req = JsonRpcRequest::new(id_num.clone(), "tools/list", Some(json!({ "a": 1 })));
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.method, "tools/list");

    let notif = JsonRpcNotification::new("logging/message", None);
    assert_eq!(notif.jsonrpc, "2.0");
    assert_eq!(notif.method, "logging/message");

    let cancelled = JsonRpcNotification::cancelled(id_num.clone());
    assert_eq!(cancelled.method, "notifications/cancelled");
    assert_eq!(
        cancelled.params.as_ref().and_then(|p| p.get("requestId")),
        Some(&json!(7))
    );

    let ok = JsonRpcResponse::ok(id_num.clone(), json!({ "k": true }));
    let ok_value = ok.into_result().expect("expected ok result");
    assert_eq!(ok_value.get("k"), Some(&json!(true)));

    let err = JsonRpcResponse::err(id_str.clone(), -32601, "method not found", None);
    let json_err = err.into_result().expect_err("expected json-rpc error");
    assert_eq!(json_err.code, -32601);

    let missing = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: id_num,
        result: None,
        error: None,
    };
    let missing_err = missing.into_result().expect_err("expected missing result error");
    assert_eq!(missing_err.code, -32603);
}

#[test]
fn mcp_error_from_serde_json_error_is_json_variant() {
    let parse_err = serde_json::from_str::<Value>("{").expect_err("parse must fail");
    let mcp: McpError = parse_err.into();
    assert!(matches!(mcp, McpError::Json(_)));
}

#[test]
fn canonicalize_under_roots_covers_error_branches() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let root_path = root.path().to_path_buf();
    let canonical_root = root_path.canonicalize()?;
    let inside = root.path().join("inside.txt");
    std::fs::write(&inside, b"ok")?;

    let ok = canonicalize_under_roots(&inside.to_string_lossy(), &[root_path.clone()])?;
    assert!(ok.starts_with(&canonical_root));

    let err = canonicalize_under_roots("", &[root_path.clone()]).expect_err("empty rejected");
    assert!(matches!(err, McpError::SecurityViolation(_)));

    let missing = canonicalize_under_roots("missing.txt", &[root_path.clone()])
        .expect_err("missing candidate rejected");
    assert!(matches!(missing, McpError::SecurityViolation(_)));

    let bogus_root = root.path().join("no-such-root");
    let err = canonicalize_under_roots("anything.txt", &[bogus_root]).expect_err("bad root rejected");
    assert!(matches!(err, McpError::SecurityViolation(_)));

    let outside_dir = tempfile::tempdir()?;
    let outside_file = outside_dir.path().join("outside.txt");
    std::fs::write(&outside_file, b"nope")?;
    let escaped = canonicalize_under_roots(&outside_file.to_string_lossy(), &[root_path])
        .expect_err("absolute path outside root rejected");
    assert!(matches!(escaped, McpError::SecurityViolation(_)));

    Ok(())
}

#[cfg(windows)]
#[test]
fn canonicalize_under_roots_rejects_windows_drive_relative_prefix() {
    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path().to_path_buf();
    let err = canonicalize_under_roots("C:relative.txt", &[root_path]).expect_err("expected reject");
    assert!(matches!(err, McpError::SecurityViolation(_)));
}
