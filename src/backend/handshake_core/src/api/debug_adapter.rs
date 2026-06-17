//! WP-KERNEL-009 MT-254 DebugAdapterCore — REST surface.
//!
//! Product-callable wrapper over [`crate::debug_adapter`]:
//! * `GET  /debug/adapters` — the listable adapters (Node only; honesty gate),
//! * `GET  /debug/documents/:rich_document_id/breakpoints` — durable breakpoints,
//! * `PUT  /debug/documents/:rich_document_id/breakpoints` — replace the set
//!   (PostgreSQL + EventLedger authority, receipt per write),
//! * `POST   /debug/sessions` — launch a REAL debuggee (Node today),
//! * `POST   /debug/sessions/:id/breakpoints` — bind breakpoints on the live session,
//! * `GET    /debug/sessions/:id/stack` — the paused call stack,
//! * `GET    /debug/sessions/:id/frames/:frame_id/scopes` — a frame's scopes,
//! * `GET    /debug/sessions/:id/variables/:reference` — real runtime variables,
//! * `POST   /debug/sessions/:id/evaluate` — console eval in the paused frame,
//! * `POST   /debug/sessions/:id/step` — step over/into/out,
//! * `POST   /debug/sessions/:id/continue` — resume,
//! * `POST   /debug/sessions/:id/pause` — pause a running debuggee,
//! * `GET    /debug/sessions/:id/events` — drain the buffered `dap://` events
//!   (`stopped`/`output`/`continued`/`terminated`) so a polling UI can react,
//! * `DELETE /debug/sessions/:id` — terminate the session.
//!
//! Live sessions are stateful (a session owns an inspector websocket and a
//! streaming forwarder), so they are held in a process-global
//! [`session_registry`] keyed by [`DebugSessionId`] and driven over these HTTP
//! routes — the SAME transport the rest of the product UI speaks (axum @
//! 127.0.0.1:37501), reachable from the frontend `dap_client.ts`. There is no
//! Tauri IPC bridge: the operator-facing debug loop is the REST surface here.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex};

use crate::debug_adapter::node_inspector::NodeInspectorSession;
use crate::debug_adapter::registry::listable_adapters;
use crate::debug_adapter::{
    launch, AdapterKind, DebugAdapter, DebugAdapterError, DebugEvent, LaunchRequest,
    SourceBreakpoint, StepKind,
};
use crate::storage::{DebugBreakpointInput, Database, StorageError};
use crate::AppState;

type ApiError = (StatusCode, Json<Value>);

/// A live debug session plus a drained-event buffer. The session's broadcast
/// stream is consumed by a forwarder task into `events` so the polling
/// `GET .../events` route can return events that arrived between polls without
/// the UI holding a websocket open.
struct LiveSession {
    session: Arc<NodeInspectorSession>,
    events: Arc<Mutex<Vec<DebugEvent>>>,
    _forwarder: tokio::task::JoinHandle<()>,
}

/// Process-global registry of live debug sessions. Sessions are stateful and do
/// not fit `AppState` (which is rebuilt per test/fixture); a single global keyed
/// by session id is the analogue of the terminal-session table.
type SessionMap = Mutex<HashMap<String, Arc<LiveSession>>>;

fn session_registry() -> &'static SessionMap {
    static REGISTRY: std::sync::OnceLock<SessionMap> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/debug/adapters", get(list_adapters))
        .route(
            "/debug/documents/:rich_document_id/breakpoints",
            get(get_breakpoints).put(put_breakpoints),
        )
        .route("/debug/sessions", post(launch_session))
        .route("/debug/sessions/:id", axum::routing::delete(terminate_session))
        .route(
            "/debug/sessions/:id/breakpoints",
            post(session_set_breakpoints),
        )
        .route("/debug/sessions/:id/stack", get(session_stack))
        .route(
            "/debug/sessions/:id/frames/:frame_id/scopes",
            get(session_scopes),
        )
        .route(
            "/debug/sessions/:id/variables/:reference",
            get(session_variables),
        )
        .route("/debug/sessions/:id/evaluate", post(session_evaluate))
        .route("/debug/sessions/:id/step", post(session_step))
        .route("/debug/sessions/:id/continue", post(session_continue))
        .route("/debug/sessions/:id/pause", post(session_pause))
        .route("/debug/sessions/:id/events", get(session_events))
        .with_state(state)
}

fn bad_request(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "detail": detail.into()})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::Validation(detail) => bad_request(detail),
        StorageError::NotFound(what) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "internal_error", "detail": other.to_string()})),
        ),
    }
}

fn adapter_error(err: DebugAdapterError) -> ApiError {
    let status = match err {
        DebugAdapterError::NotPaused => StatusCode::CONFLICT,
        DebugAdapterError::Unsupported(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({"error": "debug_adapter_error", "detail": err.to_string()})),
    )
}

fn session_not_found(id: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "session_not_found", "detail": id})),
    )
}

async fn lookup_session(id: &str) -> Result<Arc<LiveSession>, ApiError> {
    session_registry()
        .lock()
        .await
        .get(id)
        .cloned()
        .ok_or_else(|| session_not_found(id))
}

/// `GET /debug/adapters` — exactly the runnable adapters. The list IS the
/// honesty gate: it must equal `listable_adapters()` (Node only), with no
/// disabled / python / lldb entries.
async fn list_adapters() -> Result<Json<Value>, ApiError> {
    let adapters = listable_adapters();
    Ok(Json(json!({ "adapters": adapters })))
}

#[derive(Debug, Deserialize)]
struct BreakpointQuery {
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PutBreakpointsBody {
    workspace_id: String,
    breakpoints: Vec<BreakpointReq>,
}

#[derive(Debug, Deserialize)]
struct BreakpointReq {
    source_url: String,
    line: i32,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    verified: bool,
}

/// `GET /debug/documents/:rich_document_id/breakpoints` — durable breakpoints.
async fn get_breakpoints(
    State(state): State<AppState>,
    Path(rich_document_id): Path<String>,
    Query(_query): Query<BreakpointQuery>,
) -> Result<Json<Value>, ApiError> {
    let breakpoints = state
        .storage
        .list_debug_breakpoints(&rich_document_id)
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({
        "rich_document_id": rich_document_id,
        "breakpoints": breakpoints,
    })))
}

/// `PUT /debug/documents/:rich_document_id/breakpoints` — replace the full set.
async fn put_breakpoints(
    State(state): State<AppState>,
    Path(rich_document_id): Path<String>,
    Json(body): Json<PutBreakpointsBody>,
) -> Result<Json<Value>, ApiError> {
    if body.workspace_id.trim().is_empty() {
        return Err(bad_request("workspace_id is required"));
    }
    let inputs: Vec<DebugBreakpointInput> = body
        .breakpoints
        .into_iter()
        .map(|b| DebugBreakpointInput {
            source_url: b.source_url,
            line: b.line,
            condition: b.condition,
            verified: b.verified,
        })
        .collect();
    let stored = state
        .storage
        .set_debug_breakpoints(&rich_document_id, &body.workspace_id, inputs)
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({
        "rich_document_id": rich_document_id,
        "breakpoints": stored,
    })))
}

// --------------------------------------------------------------------------
// Live debug session surface (REAL process; no mock). These drive
// `crate::debug_adapter::launch` and the `NodeInspectorSession` directly.
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LaunchSessionBody {
    /// Adapter id (must be a runnable adapter; "node" today).
    adapter: String,
    /// Absolute path to the program to debug.
    program: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    runtime_path: Option<String>,
}

/// `POST /debug/sessions` — launch a real debuggee, register it, and return the
/// session id plus the initial paused location (`--inspect-brk` entry stop).
async fn launch_session(Json(body): Json<LaunchSessionBody>) -> Result<Json<Value>, ApiError> {
    let adapter = match body.adapter.as_str() {
        "node" => AdapterKind::Node,
        other => return Err(bad_request(format!("unknown or non-runnable adapter '{other}'"))),
    };
    let mut req = match adapter {
        AdapterKind::Node => LaunchRequest::node(body.program.clone()),
    };
    req.cwd = body.cwd;
    req.runtime_path = body.runtime_path;

    let session = launch(req).await.map_err(adapter_error)?;
    let session = Arc::new(session);
    let id = session.session_id().as_str().to_string();

    // Forward the session's broadcast events into a drained buffer the polling
    // `/events` route reads. This captures events (stopped/output/continued/
    // terminated) that arrive between polls without holding a socket open.
    let events: Arc<Mutex<Vec<DebugEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let mut rx = session.subscribe();
    let buffer = events.clone();
    let forwarder = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => buffer.lock().await.push(event),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let live = Arc::new(LiveSession {
        session: session.clone(),
        events,
        _forwarder: forwarder,
    });
    session_registry().lock().await.insert(id.clone(), live);

    // The session is paused at entry after launch; report the entry frame.
    let stack = session.stack_trace().await.ok();
    let top_line = stack
        .as_ref()
        .and_then(|frames| frames.first())
        .map(|frame| frame.line);
    Ok(Json(json!({
        "session_id": id,
        "adapter": adapter.as_str(),
        "paused": true,
        "top_frame_line": top_line,
    })))
}

#[derive(Debug, Deserialize)]
struct SessionBreakpointsBody {
    /// Source url/path the breakpoints apply to. Empty means the launched script.
    #[serde(default)]
    source: String,
    breakpoints: Vec<SourceBreakpoint>,
}

/// `POST /debug/sessions/:id/breakpoints` — bind breakpoints on the live session
/// (REAL CDP binding; the returned `verified` is never faked).
async fn session_set_breakpoints(
    Path(id): Path<String>,
    Json(body): Json<SessionBreakpointsBody>,
) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let bound = live
        .session
        .set_breakpoints(&body.source, &body.breakpoints)
        .await
        .map_err(adapter_error)?;
    Ok(Json(json!({ "breakpoints": bound })))
}

/// `GET /debug/sessions/:id/stack` — the paused call stack.
async fn session_stack(Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let frames = live.session.stack_trace().await.map_err(adapter_error)?;
    Ok(Json(json!({ "frames": frames })))
}

/// `GET /debug/sessions/:id/frames/:frame_id/scopes` — a paused frame's scopes.
async fn session_scopes(Path((id, frame_id)): Path<(String, String)>) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let scopes = live.session.scopes(&frame_id).await.map_err(adapter_error)?;
    Ok(Json(json!({ "scopes": scopes })))
}

/// `GET /debug/sessions/:id/variables/:reference` — real runtime variables.
async fn session_variables(
    Path((id, reference)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let variables = live
        .session
        .variables(&reference)
        .await
        .map_err(adapter_error)?;
    Ok(Json(json!({ "variables": variables })))
}

#[derive(Debug, Deserialize)]
struct EvaluateBody {
    frame_id: String,
    expression: String,
}

/// `POST /debug/sessions/:id/evaluate` — debug-console eval in the paused frame.
async fn session_evaluate(
    Path(id): Path<String>,
    Json(body): Json<EvaluateBody>,
) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let result = live
        .session
        .evaluate(&body.frame_id, &body.expression)
        .await
        .map_err(adapter_error)?;
    Ok(Json(json!({ "result": result })))
}

#[derive(Debug, Deserialize)]
struct StepBody {
    /// "over" | "into" | "out".
    kind: String,
}

/// `POST /debug/sessions/:id/step` — step; resolves once paused again.
async fn session_step(
    Path(id): Path<String>,
    Json(body): Json<StepBody>,
) -> Result<Json<Value>, ApiError> {
    let kind = match body.kind.as_str() {
        "over" => StepKind::Over,
        "into" => StepKind::Into,
        "out" => StepKind::Out,
        other => return Err(bad_request(format!("unknown step kind '{other}'"))),
    };
    let live = lookup_session(&id).await?;
    live.session.step(kind).await.map_err(adapter_error)?;
    let frames = live.session.stack_trace().await.map_err(adapter_error)?;
    let top_line = frames.first().map(|frame| frame.line);
    Ok(Json(json!({ "paused": true, "top_frame_line": top_line })))
}

/// `POST /debug/sessions/:id/continue` — resume execution.
async fn session_continue(Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    live.session.continue_().await.map_err(adapter_error)?;
    Ok(Json(json!({ "resumed": true })))
}

/// `POST /debug/sessions/:id/pause` — pause a running debuggee.
async fn session_pause(Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    live.session.pause().await.map_err(adapter_error)?;
    Ok(Json(json!({ "paused": true })))
}

/// `GET /debug/sessions/:id/events` — drain and return the buffered dap events.
async fn session_events(Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let live = lookup_session(&id).await?;
    let drained: Vec<DebugEvent> = {
        let mut buffer = live.events.lock().await;
        std::mem::take(&mut *buffer)
    };
    Ok(Json(json!({ "events": drained })))
}

/// `DELETE /debug/sessions/:id` — terminate the session; returns the real exit code.
async fn terminate_session(Path(id): Path<String>) -> Result<Json<Value>, ApiError> {
    let live = session_registry()
        .lock()
        .await
        .remove(&id)
        .ok_or_else(|| session_not_found(&id))?;
    let exit_code = live.session.terminate().await.map_err(adapter_error)?;
    Ok(Json(json!({ "terminated": true, "exit_code": exit_code })))
}
