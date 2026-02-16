use serde_json::{json, Value};
use std::sync::Arc;

use crate::flight_recorder::FlightRecorder;

use super::errors::{McpError, McpResult};
use super::gate::McpContext;

fn next_event_id(conn: &duckdb::Connection) -> McpResult<i64> {
    conn.prepare("SELECT COALESCE(MAX(event_id), 0) + 1 FROM fr_events")
        .map_err(|e| McpError::FlightRecorder(e.to_string()))?
        .query_row([], |row| row.get(0))
        .map_err(|e| McpError::FlightRecorder(e.to_string()))
}

fn insert_fr_event(
    flight_recorder: &dyn FlightRecorder,
    ctx: Option<&McpContext>,
    event_kind: &str,
    source: &str,
    level: &str,
    message: &str,
    payload: Value,
) -> McpResult<()> {
    let Some(conn) = flight_recorder.duckdb_connection() else {
        return Ok(());
    };
    let conn = conn
        .lock()
        .map_err(|_| McpError::FlightRecorder("duckdb connection lock error".to_string()))?;
    let next_id = next_event_id(&conn)?;
    let payload_str = serde_json::to_string(&payload)?;

    let (session_id, task_id, job_id, workflow_run_id) = ctx
        .map(|c| {
            (
                c.session_id.as_deref(),
                c.task_id.as_deref(),
                c.job_id.map(|j| j.to_string()),
                c.workflow_run_id.as_deref(),
            )
        })
        .unwrap_or((None, None, None, None));

    conn.execute(
        r#"
        INSERT INTO fr_events (
            event_id,
            ts_utc,
            session_id,
            task_id,
            job_id,
            workflow_run_id,
            event_kind,
            source,
            level,
            message,
            payload
        ) VALUES (?, CURRENT_TIMESTAMP, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#,
        duckdb::params![
            next_id,
            session_id,
            task_id,
            job_id.as_deref(),
            workflow_run_id,
            event_kind,
            source,
            level,
            message,
            payload_str
        ],
    )
    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

    Ok(())
}

pub fn record_tool_call(
    flight_recorder: Arc<dyn FlightRecorder>,
    ctx: &McpContext,
    server_id: &str,
    tool_name: &str,
    capability_id: Option<&str>,
    arguments: &Value,
) -> McpResult<()> {
    let payload = json!({
        "tool_name": format!("mcp:{server_id}:{tool_name}"),
        "tool_version": null,
        "inputs": [],
        "outputs": [],
        "status": "success",
        "duration_ms": null,
        "error_code": null,
        "job_id": ctx.job_id.map(|j| j.to_string()),
        "workflow_run_id": ctx.workflow_run_id,
        "trace_id": ctx.trace_id.to_string(),
        "capability_id": capability_id,
        "server_id": server_id,
        "tool": tool_name,
        "arguments": arguments,
    });
    insert_fr_event(
        flight_recorder.as_ref(),
        Some(ctx),
        "tool.call",
        server_id,
        "INFO",
        "mcp tool invocation started",
        payload,
    )
}

pub fn record_tool_result(
    flight_recorder: Arc<dyn FlightRecorder>,
    ctx: &McpContext,
    server_id: &str,
    tool_name: &str,
    capability_id: Option<&str>,
    status: &str,
    duration_ms: Option<u128>,
    error_code: Option<&str>,
    result: &Value,
) -> McpResult<()> {
    let payload = json!({
        "tool_name": format!("mcp:{server_id}:{tool_name}"),
        "tool_version": null,
        "inputs": [],
        "outputs": [],
        "status": status,
        "duration_ms": duration_ms.map(|d| d as u64),
        "error_code": error_code,
        "job_id": ctx.job_id.map(|j| j.to_string()),
        "workflow_run_id": ctx.workflow_run_id,
        "trace_id": ctx.trace_id.to_string(),
        "capability_id": capability_id,
        "server_id": server_id,
        "tool": tool_name,
        "result": result,
    });
    let level = if status == "success" { "INFO" } else { "ERROR" };
    insert_fr_event(
        flight_recorder.as_ref(),
        Some(ctx),
        "tool.result",
        server_id,
        level,
        "mcp tool invocation finished",
        payload,
    )
}

pub fn record_logging_message(
    flight_recorder: Arc<dyn FlightRecorder>,
    server_id: &str,
    params: &Value,
) -> McpResult<()> {
    let level = params
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("INFO");
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let logger = params
        .get("logger")
        .and_then(|v| v.as_str())
        .unwrap_or("mcp");

    let ctx = params.get("context").unwrap_or(&Value::Null);
    let fields = params.get("fields").unwrap_or(&Value::Null);

    let job_id = ctx.get("job_id").and_then(|v| v.as_str());
    let task_id = ctx.get("task_id").and_then(|v| v.as_str());
    let workflow_run_id = ctx.get("workflow_run_id").and_then(|v| v.as_str());
    let session_id = ctx.get("session_id").and_then(|v| v.as_str());

    let event_kind = fields
        .get("event_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("mcp.logging");
    let source = fields
        .get("server_id")
        .and_then(|v| v.as_str())
        .unwrap_or(logger);

    let Some(conn) = flight_recorder.duckdb_connection() else {
        return Ok(());
    };
    let conn = conn
        .lock()
        .map_err(|_| McpError::FlightRecorder("duckdb connection lock error".to_string()))?;
    let next_id = next_event_id(&conn)?;
    let payload_str = serde_json::to_string(params)?;

    conn.execute(
        r#"
        INSERT INTO fr_events (
            event_id,
            ts_utc,
            session_id,
            task_id,
            job_id,
            workflow_run_id,
            event_kind,
            source,
            level,
            message,
            payload
        ) VALUES (?, CURRENT_TIMESTAMP, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#,
        duckdb::params![
            next_id,
            session_id,
            task_id,
            job_id,
            workflow_run_id,
            event_kind,
            source,
            level,
            message,
            payload_str
        ],
    )
    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

    drop(conn);

    if event_kind != "mcp.logging" {
        let breadcrumb = json!({
            "logger": logger,
            "event_kind": event_kind,
            "server_id": server_id,
        });
        let _ = insert_fr_event(
            flight_recorder.as_ref(),
            None,
            "mcp.logging",
            server_id,
            level,
            message,
            breadcrumb,
        );
    }

    Ok(())
}

pub fn record_gate_decision(
    flight_recorder: Arc<dyn FlightRecorder>,
    ctx: &McpContext,
    server_id: &str,
    tool_name: Option<&str>,
    decision: &str,
    reason: &str,
    details: Value,
) -> McpResult<()> {
    let level = if decision == "allow" { "INFO" } else { "WARN" };
    let message = format!("mcp gate decision: {decision} ({reason})");
    let payload = json!({
        "server_id": server_id,
        "tool": tool_name,
        "decision": decision,
        "reason": reason,
        "trace_id": ctx.trace_id.to_string(),
        "job_id": ctx.job_id.map(|j| j.to_string()),
        "details": details,
    });
    insert_fr_event(
        flight_recorder.as_ref(),
        Some(ctx),
        "mcp.gate.decision",
        server_id,
        level,
        &message,
        payload,
    )
}
