//! WP-KERNEL-009 MT-254 DebugAdapterCore — the Node.js adapter.
//!
//! Drives a REAL `node` child process over the V8 Inspector / Chrome DevTools
//! Protocol (CDP). No external adapter binary, no mock: the session launches
//! `node --inspect-brk=127.0.0.1:0 <script>`, discovers the inspector
//! `webSocketDebuggerUrl`, holds a PERSISTENT websocket (a debug session is
//! stateful — breakpoints, paused frames, and scope object ids only live for the
//! life of one connection), and maps CDP onto the Handshake DAP shapes:
//!
//! * `Debugger.setBreakpointByUrl` -> [`Breakpoint`] (`verified` = a real
//!   `location` was returned),
//! * `Debugger.paused` -> [`DebugEvent::Stopped`] + [`StackFrame`]s,
//! * `Runtime.getProperties` -> [`Variable`]s (real runtime values),
//! * `Debugger.evaluateOnCallFrame` -> console eval at the paused frame,
//! * `Debugger.stepOver/stepInto/stepOut/resume/pause` -> stepping/continue.
//!
//! The persistent connection runs a background reader task that demultiplexes
//! command responses (matched by `id`) from protocol events (`Debugger.paused`,
//! `Runtime.consoleAPICalled`, ...). Events are forwarded on a broadcast channel
//! exactly like the terminal forwarder; the REST surface in
//! [`crate::api::debug_adapter`] drains that channel into the
//! `GET /debug/sessions/:id/events` response so the UI sees
//! `dap://stopped|output|continued|terminated` over plain HTTP.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::debug_adapter::protocol::{
    Breakpoint, DebugEvent, DebugSessionId, Scope, SourceBreakpoint, StackFrame, StepKind,
    StoppedReason, Variable,
};
use crate::debug_adapter::{DebugAdapter, DebugAdapterError, LaunchRequest};

/// How long to wait for the inspector ws url to appear on the child's stderr.
const WS_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
/// How long to wait for a single CDP command response.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
/// How long to wait for the first `Debugger.paused` after launch/continue/step.
const STOP_TIMEOUT: Duration = Duration::from_secs(20);

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

/// Shared state describing the most recent paused location so callers can read
/// the call stack / scopes / variables without racing the reader task.
#[derive(Default)]
struct PausedState {
    /// CDP callFrames from the last `Debugger.paused`.
    call_frames: Vec<Value>,
    paused: bool,
}

/// A live Node debug session over a persistent inspector websocket.
pub struct NodeInspectorSession {
    session_id: DebugSessionId,
    child: Arc<Mutex<Child>>,
    /// Outbound CDP command sink (the websocket write half is owned by a task;
    /// we send through this).
    ws_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    next_id: AtomicU64,
    pending: PendingMap,
    paused: Arc<Mutex<PausedState>>,
    events_tx: broadcast::Sender<DebugEvent>,
    /// scriptId -> url, learned from `Debugger.scriptParsed`.
    scripts: Arc<Mutex<HashMap<String, String>>>,
    script_path: String,
    reader_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl NodeInspectorSession {
    /// Subscribe to the session's `dap://` event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<DebugEvent> {
        self.events_tx.subscribe()
    }

    async fn send_command(&self, method: &str, params: Value) -> Result<Value, DebugAdapterError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let frame = json!({"id": id, "method": method, "params": params});
        self.ws_tx
            .send(Message::Text(frame.to_string().into()))
            .map_err(|e| DebugAdapterError::Transport(format!("ws send {method}: {e}")))?;
        let result = tokio::time::timeout(COMMAND_TIMEOUT, rx)
            .await
            .map_err(|_| DebugAdapterError::Timeout(format!("CDP {method} timed out")))?
            .map_err(|_| DebugAdapterError::Transport(format!("CDP {method} channel closed")))?;
        if let Some(err) = result.get("error") {
            return Err(DebugAdapterError::Protocol(format!(
                "CDP {method} error: {err}"
            )));
        }
        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Block until the next `Debugger.paused` is observed, returning the top
    /// frame line + source. Returns the already-paused state if the session is
    /// currently paused (e.g. the initial `--inspect-brk` entry pause).
    async fn await_stopped(&self) -> Result<(Option<u32>, Option<String>), DebugAdapterError> {
        let mut rx = self.events_tx.subscribe();
        // Fast path: already paused.
        {
            let state = self.paused.lock().await;
            if state.paused {
                if let Some(frame) = state.call_frames.first() {
                    return Ok(top_frame_line_source(frame));
                }
                return Ok((None, None));
            }
        }
        let deadline = tokio::time::Instant::now() + STOP_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(DebugAdapterError::Timeout(
                    "timed out waiting for Debugger.paused".into(),
                ));
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(DebugEvent::Stopped {
                    top_frame_line,
                    top_frame_source,
                    ..
                })) => return Ok((top_frame_line, top_frame_source)),
                Ok(Ok(DebugEvent::Terminated { exit_code })) => {
                    return Err(DebugAdapterError::Protocol(format!(
                        "session terminated (exit {exit_code:?}) before stopping"
                    )))
                }
                Ok(Ok(_)) => continue,
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return Err(DebugAdapterError::Transport("event stream closed".into()))
                }
                Err(_) => {
                    return Err(DebugAdapterError::Timeout(
                        "timed out waiting for Debugger.paused".into(),
                    ))
                }
            }
        }
    }

    fn script_url(&self) -> String {
        path_to_file_url(&self.script_path)
    }
}

/// Launch a real Node process under `--inspect-brk` and connect the inspector.
pub async fn launch_node_session(
    req: &LaunchRequest,
) -> Result<NodeInspectorSession, DebugAdapterError> {
    let program = req.program.clone();
    if !Path::new(&program).exists() {
        return Err(DebugAdapterError::Launch(format!(
            "node program does not exist: {program}"
        )));
    }
    let node_bin = req.runtime_path.clone().unwrap_or_else(|| "node".into());

    let mut command = Command::new(&node_bin);
    command
        .arg("--inspect-brk=127.0.0.1:0")
        .arg(&program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = &req.cwd {
        command.current_dir(cwd);
    }
    for (k, v) in &req.env {
        command.env(k, v);
    }

    let mut child = command
        .spawn()
        .map_err(|e| DebugAdapterError::Launch(format!("spawn {node_bin}: {e}")))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DebugAdapterError::Launch("node stderr unavailable".into()))?;
    let stdout = child.stdout.take();
    let child = Arc::new(Mutex::new(child));

    let (events_tx, _events_rx0) = broadcast::channel(256);

    // Discover the inspector ws url from stderr ("Debugger listening on ws://..").
    let ws_url = discover_ws_url(stderr, events_tx.clone()).await?;

    // Forward debuggee stdout as DAP output events.
    if let Some(stdout) = stdout {
        let out_tx = events_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = out_tx.send(DebugEvent::Output {
                    category: "stdout".into(),
                    output: format!("{line}\n"),
                });
            }
        });
    }

    // Connect persistent websocket.
    let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url.as_str())
        .await
        .map_err(|e| DebugAdapterError::Transport(format!("connect inspector ws: {e}")))?;
    let (mut write, mut read) = ws_stream.split();

    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    // Writer task: drains ws_rx to the socket.
    tokio::spawn(async move {
        while let Some(msg) = ws_rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
    let paused = Arc::new(Mutex::new(PausedState::default()));
    let scripts: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // Reader task: demux responses vs events.
    let reader_pending = pending.clone();
    let reader_paused = paused.clone();
    let reader_scripts = scripts.clone();
    let reader_events = events_tx.clone();
    let reader_child = child.clone();
    let reader_ws_tx = ws_tx.clone();
    let trace = std::env::var("HSK_DAP_TRACE").is_ok();
    let reader_handle = tokio::spawn(async move {
        // The default (main script) execution context id; when it is destroyed
        // the user program has finished. Node with --inspect-brk keeps the
        // inspector ws open after the program ends (waiting for the debugger to
        // disconnect), so we proactively close the ws to let node exit cleanly.
        let mut default_context_id: Option<u64> = None;
        loop {
            let next = read.next().await;
            let msg = match next {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => {
                    if trace {
                        eprintln!("[dap] ws read error: {e}");
                    }
                    break;
                }
                None => {
                    if trace {
                        eprintln!("[dap] ws closed (None)");
                    }
                    break;
                }
            };
            let Message::Text(text) = msg else { continue };
            if trace {
                eprintln!("[dap] <- {}", &text[..text.len().min(200)]);
            }
            let Ok(value): Result<Value, _> = serde_json::from_str(&text) else {
                continue;
            };
            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                if let Some(tx) = reader_pending.lock().await.remove(&id) {
                    let _ = tx.send(value);
                }
                continue;
            }
            let Some(method) = value.get("method").and_then(Value::as_str) else {
                continue;
            };
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            match method {
                "Runtime.executionContextCreated" => {
                    let ctx = params.get("context");
                    let is_default = ctx
                        .and_then(|c| c.get("auxData"))
                        .and_then(|a| a.get("isDefault"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if is_default && default_context_id.is_none() {
                        default_context_id =
                            ctx.and_then(|c| c.get("id")).and_then(Value::as_u64);
                    }
                }
                "Runtime.executionContextDestroyed" => {
                    let destroyed = params
                        .get("executionContextId")
                        .and_then(Value::as_u64);
                    if destroyed.is_some() && destroyed == default_context_id {
                        if trace {
                            eprintln!("[dap] main context destroyed; closing ws to let node exit");
                        }
                        let _ = reader_ws_tx.send(Message::Close(None));
                        break;
                    }
                }
                "Debugger.scriptParsed" => {
                    if let (Some(id), Some(url)) = (
                        params.get("scriptId").and_then(Value::as_str),
                        params.get("url").and_then(Value::as_str),
                    ) {
                        reader_scripts
                            .lock()
                            .await
                            .insert(id.to_string(), url.to_string());
                    }
                }
                "Debugger.paused" => {
                    let call_frames = params
                        .get("callFrames")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let reason = match params.get("reason").and_then(Value::as_str) {
                        Some("other") | Some("debuggerStatement") => StoppedReason::Breakpoint,
                        Some("Break on start") | Some("entry") => StoppedReason::Entry,
                        Some("exception") | Some("promiseRejection") => StoppedReason::Exception,
                        _ => StoppedReason::Breakpoint,
                    };
                    let (line, source) =
                        call_frames.first().map(top_frame_line_source).unwrap_or((None, None));
                    {
                        let mut state = reader_paused.lock().await;
                        state.call_frames = call_frames;
                        state.paused = true;
                    }
                    let _ = reader_events.send(DebugEvent::Stopped {
                        reason,
                        top_frame_line: line,
                        top_frame_source: source,
                    });
                }
                "Debugger.resumed" => {
                    {
                        let mut state = reader_paused.lock().await;
                        state.paused = false;
                        state.call_frames.clear();
                    }
                    let _ = reader_events.send(DebugEvent::Continued);
                }
                "Runtime.consoleAPICalled" => {
                    let level = params
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("log")
                        .to_string();
                    let output = params
                        .get("args")
                        .and_then(Value::as_array)
                        .map(|args| {
                            args.iter()
                                .filter_map(|a| {
                                    a.get("value")
                                        .map(value_to_display)
                                        .or_else(|| {
                                            a.get("description")
                                                .and_then(Value::as_str)
                                                .map(str::to_string)
                                        })
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();
                    let _ = reader_events.send(DebugEvent::Output {
                        category: format!("console.{level}"),
                        output: format!("{output}\n"),
                    });
                }
                _ => {}
            }
        }
        // The inspector websocket closed: the debuggee process is exiting (it
        // ran to completion or was killed). Reap the real exit code and emit a
        // single Terminated event so consumers see the genuine process status.
        {
            let mut state = reader_paused.lock().await;
            state.paused = false;
            state.call_frames.clear();
        }
        let code = {
            let mut child = reader_child.lock().await;
            match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => status.code(),
                _ => None,
            }
        };
        let _ = reader_events.send(DebugEvent::Terminated { exit_code: code });
    });

    let session = NodeInspectorSession {
        session_id: DebugSessionId::new(),
        child,
        ws_tx,
        next_id: AtomicU64::new(1),
        pending,
        paused,
        events_tx,
        scripts,
        script_path: program,
        reader_handle: Mutex::new(Some(reader_handle)),
    };

    // Enable the inspector domains. The child is paused at entry due to
    // --inspect-brk; it will not run until we resume.
    session.send_command("Runtime.enable", json!({})).await?;
    session.send_command("Debugger.enable", json!({})).await?;
    session
        .send_command("Runtime.runIfWaitingForDebugger", json!({}))
        .await?;
    // After runIfWaitingForDebugger Node pauses at the first line (entry). Wait
    // for that entry pause so breakpoints can be bound before any user code runs.
    session.await_stopped().await?;

    Ok(session)
}

/// Read the child's stderr until the inspector "ws://" url appears.
async fn discover_ws_url(
    stderr: tokio::process::ChildStderr,
    events_tx: broadcast::Sender<DebugEvent>,
) -> Result<String, DebugAdapterError> {
    let mut lines = BufReader::new(stderr).lines();
    let deadline = tokio::time::Instant::now() + WS_DISCOVERY_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(DebugAdapterError::Launch(
                "timed out discovering inspector ws url".into(),
            ));
        }
        match tokio::time::timeout(remaining, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                if let Some(url) = parse_ws_url(&line) {
                    // Continue forwarding remaining stderr as output events.
                    let err_tx = events_tx.clone();
                    tokio::spawn(async move {
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = err_tx.send(DebugEvent::Output {
                                category: "stderr".into(),
                                output: format!("{line}\n"),
                            });
                        }
                    });
                    return Ok(url);
                }
            }
            Ok(Ok(None)) => {
                return Err(DebugAdapterError::Launch(
                    "node exited before inspector ws url appeared".into(),
                ))
            }
            Ok(Err(e)) => return Err(DebugAdapterError::Launch(format!("read stderr: {e}"))),
            Err(_) => {
                return Err(DebugAdapterError::Launch(
                    "timed out discovering inspector ws url".into(),
                ))
            }
        }
    }
}

/// Parse `Debugger listening on ws://127.0.0.1:PORT/UUID` (the line node prints
/// on stderr for `--inspect-brk`).
fn parse_ws_url(line: &str) -> Option<String> {
    let idx = line.find("ws://")?;
    let url: String = line[idx..]
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect();
    if url.len() > "ws://".len() {
        Some(url)
    } else {
        None
    }
}

fn top_frame_line_source(frame: &Value) -> (Option<u32>, Option<String>) {
    let line = frame
        .get("location")
        .and_then(|l| l.get("lineNumber"))
        .and_then(Value::as_u64)
        .map(|l| (l as u32) + 1); // CDP is 0-based; DAP/UI is 1-based.
    let source = frame
        .get("url")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    (line, source)
}

fn value_to_display(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Convert an OS path to a `file://` url the way Node's inspector reports script
/// urls, so `setBreakpointByUrl` matches the parsed script.
fn path_to_file_url(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with("file://") {
        return normalized;
    }
    // Windows drive paths get an extra leading slash: file:///C:/...
    if normalized.chars().nth(1) == Some(':') {
        format!("file:///{normalized}")
    } else if let Some(stripped) = normalized.strip_prefix('/') {
        format!("file:///{stripped}")
    } else {
        format!("file:///{normalized}")
    }
}

#[async_trait]
impl DebugAdapter for NodeInspectorSession {
    fn session_id(&self) -> &DebugSessionId {
        &self.session_id
    }

    async fn set_breakpoints(
        &self,
        source: &str,
        breakpoints: &[SourceBreakpoint],
    ) -> Result<Vec<Breakpoint>, DebugAdapterError> {
        let url = if source.is_empty() {
            self.script_url()
        } else {
            path_to_file_url(source)
        };
        let mut out = Vec::with_capacity(breakpoints.len());
        for bp in breakpoints {
            let mut params = json!({
                "lineNumber": bp.line.saturating_sub(1), // 1-based -> CDP 0-based
                "url": url,
            });
            if let Some(col) = bp.column {
                params["columnNumber"] = json!(col);
            }
            if let Some(cond) = &bp.condition {
                params["condition"] = json!(cond);
            }
            let result = self.send_command("Debugger.setBreakpointByUrl", params).await?;
            let id = result
                .get("breakpointId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let locations = result
                .get("locations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let verified = !locations.is_empty();
            let bound_line = locations
                .first()
                .and_then(|l| l.get("lineNumber"))
                .and_then(Value::as_u64)
                .map(|l| (l as u32) + 1);
            out.push(Breakpoint {
                id,
                verified,
                line: bound_line.or(Some(bp.line)),
                message: if verified {
                    None
                } else {
                    Some("breakpoint did not bind to executable code".into())
                },
            });
        }
        Ok(out)
    }

    async fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugAdapterError> {
        let state = self.paused.lock().await;
        if !state.paused {
            return Err(DebugAdapterError::NotPaused);
        }
        let scripts = self.scripts.lock().await;
        let frames = state
            .call_frames
            .iter()
            .map(|frame| {
                let id = frame
                    .get("callFrameId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let name = frame
                    .get("functionName")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("(anonymous)")
                    .to_string();
                let loc = frame.get("location");
                let line = loc
                    .and_then(|l| l.get("lineNumber"))
                    .and_then(Value::as_u64)
                    .map(|l| (l as u32) + 1)
                    .unwrap_or(1);
                let column = loc
                    .and_then(|l| l.get("columnNumber"))
                    .and_then(Value::as_u64)
                    .map(|c| c as u32)
                    .unwrap_or(0);
                let source = frame
                    .get("url")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        loc.and_then(|l| l.get("scriptId"))
                            .and_then(Value::as_str)
                            .and_then(|sid| scripts.get(sid).cloned())
                    });
                StackFrame {
                    id,
                    name,
                    source,
                    line,
                    column,
                }
            })
            .collect();
        Ok(frames)
    }

    async fn scopes(&self, frame_id: &str) -> Result<Vec<Scope>, DebugAdapterError> {
        let state = self.paused.lock().await;
        if !state.paused {
            return Err(DebugAdapterError::NotPaused);
        }
        let frame = state
            .call_frames
            .iter()
            .find(|f| f.get("callFrameId").and_then(Value::as_str) == Some(frame_id))
            .ok_or_else(|| DebugAdapterError::Protocol(format!("unknown frame {frame_id}")))?;
        let scopes = frame
            .get("scopeChain")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let out = scopes
            .iter()
            .filter_map(|scope| {
                let object_id = scope
                    .get("object")
                    .and_then(|o| o.get("objectId"))
                    .and_then(Value::as_str)?
                    .to_string();
                let name = scope
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("scope")
                    .to_string();
                Some(Scope {
                    name: name.clone(),
                    variables_reference: object_id,
                    expensive: name == "global",
                })
            })
            .collect();
        Ok(out)
    }

    async fn variables(
        &self,
        variables_reference: &str,
    ) -> Result<Vec<Variable>, DebugAdapterError> {
        let result = self
            .send_command(
                "Runtime.getProperties",
                json!({
                    "objectId": variables_reference,
                    "ownProperties": true,
                    "generatePreview": true,
                }),
            )
            .await?;
        let props = result
            .get("result")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for prop in props {
            let name = prop
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let value_obj = prop.get("value");
            let (value, type_name, child_ref) = match value_obj {
                Some(v) => {
                    let type_name = v.get("type").and_then(Value::as_str).map(str::to_string);
                    let child_ref = v
                        .get("objectId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let value = v
                        .get("value")
                        .map(value_to_display)
                        .or_else(|| v.get("description").and_then(Value::as_str).map(str::to_string))
                        .or_else(|| {
                            v.get("unserializableValue")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                        .unwrap_or_default();
                    (value, type_name, child_ref)
                }
                None => ("undefined".to_string(), Some("undefined".to_string()), String::new()),
            };
            out.push(Variable {
                name,
                value,
                type_name,
                variables_reference: child_ref,
            });
        }
        Ok(out)
    }

    async fn evaluate(
        &self,
        frame_id: &str,
        expression: &str,
    ) -> Result<String, DebugAdapterError> {
        let result = self
            .send_command(
                "Debugger.evaluateOnCallFrame",
                json!({
                    "callFrameId": frame_id,
                    "expression": expression,
                    "returnByValue": true,
                }),
            )
            .await?;
        if let Some(details) = result.get("exceptionDetails") {
            return Err(DebugAdapterError::Protocol(format!(
                "evaluate threw: {}",
                details.get("text").and_then(Value::as_str).unwrap_or("error")
            )));
        }
        let v = result.get("result");
        let display = v
            .and_then(|r| r.get("value"))
            .map(value_to_display)
            .or_else(|| {
                v.and_then(|r| r.get("description"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "undefined".to_string());
        Ok(display)
    }

    async fn step(&self, kind: StepKind) -> Result<(), DebugAdapterError> {
        let method = match kind {
            StepKind::Over => "Debugger.stepOver",
            StepKind::Into => "Debugger.stepInto",
            StepKind::Out => "Debugger.stepOut",
        };
        {
            let mut state = self.paused.lock().await;
            state.paused = false;
            state.call_frames.clear();
        }
        self.send_command(method, json!({})).await?;
        // Wait for the resulting paused event.
        self.await_stopped().await?;
        Ok(())
    }

    async fn continue_(&self) -> Result<(), DebugAdapterError> {
        {
            let mut state = self.paused.lock().await;
            state.paused = false;
            state.call_frames.clear();
        }
        self.send_command("Debugger.resume", json!({})).await?;
        Ok(())
    }

    async fn pause(&self) -> Result<(), DebugAdapterError> {
        self.send_command("Debugger.pause", json!({})).await?;
        self.await_stopped().await?;
        Ok(())
    }

    async fn terminate(&self) -> Result<Option<i32>, DebugAdapterError> {
        // Best-effort graceful resume so the process can exit, then kill.
        let _ = self.send_command("Debugger.resume", json!({})).await;
        let mut child = self.child.lock().await;
        // Give the process a moment to exit on its own after resume.
        let exit = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        let code = match exit {
            Ok(Ok(status)) => status.code(),
            _ => {
                let _ = child.kill().await;
                child.wait().await.ok().and_then(|s| s.code())
            }
        };
        if let Some(handle) = self.reader_handle.lock().await.take() {
            handle.abort();
        }
        let _ = self.events_tx.send(DebugEvent::Terminated { exit_code: code });
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ws_url_from_node_stderr() {
        let line = "Debugger listening on ws://127.0.0.1:51234/abcd-1234";
        assert_eq!(
            parse_ws_url(line).as_deref(),
            Some("ws://127.0.0.1:51234/abcd-1234")
        );
        assert_eq!(parse_ws_url("For help, see: https://x").as_deref(), None);
    }

    #[test]
    fn path_to_file_url_handles_windows_and_posix() {
        assert_eq!(
            path_to_file_url("C:/tmp/x.js"),
            "file:///C:/tmp/x.js".to_string()
        );
        assert_eq!(
            path_to_file_url("C:\\tmp\\x.js"),
            "file:///C:/tmp/x.js".to_string()
        );
        assert_eq!(
            path_to_file_url("/home/u/x.js"),
            "file:///home/u/x.js".to_string()
        );
    }
}
