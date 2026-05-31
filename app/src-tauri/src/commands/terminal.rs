//! Tauri IPC surface for the Integrated Terminal (spec §10.1).
//!
//! Exposes the `kernel_terminal_*` commands the off-main-window Terminal panel
//! drives, a managed [`TerminalRuntimeState`], and a streaming forwarder that
//! mirrors `spawn_swarm_board_forwarder`: it subscribes to a session's output
//! broadcast and re-emits typed `terminal://output` / `terminal://exit` events,
//! emitting `terminal://resync` when a slow consumer lags.
//!
//! Ownership note: this file owns the IPC + managed state + forwarder fn only.
//! `lib.rs` (the Integrate phase) registers the commands in the
//! `handshake_invoke_handlers!` macro, `.manage`s the state, and spawns the
//! per-session forwarder in setup.

use std::sync::Arc;

use base64::Engine;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::terminal::{
    PtySpawnConfig, SessionBinding, SessionInfo, SessionOutput, TerminalRuntime,
};
use handshake_core::terminal::TerminalSessionType;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::broadcast;

pub const KERNEL_TERMINAL_CREATE_SESSION_IPC_CHANNEL: &str = "kernel_terminal_create_session";
pub const KERNEL_TERMINAL_WRITE_STDIN_IPC_CHANNEL: &str = "kernel_terminal_write_stdin";
pub const KERNEL_TERMINAL_RESIZE_IPC_CHANNEL: &str = "kernel_terminal_resize";
pub const KERNEL_TERMINAL_CLOSE_SESSION_IPC_CHANNEL: &str = "kernel_terminal_close_session";
pub const KERNEL_TERMINAL_LIST_SESSIONS_IPC_CHANNEL: &str = "kernel_terminal_list_sessions";
pub const KERNEL_TERMINAL_RUN_COMMAND_IPC_CHANNEL: &str = "kernel_terminal_run_command";
pub const KERNEL_TERMINAL_SCROLLBACK_IPC_CHANNEL: &str = "kernel_terminal_scrollback";
pub const KERNEL_TERMINAL_AUTHORIZE_INTERACTIVE_IPC_CHANNEL: &str =
    "kernel_terminal_authorize_interactive";

/// Tauri managed state holding the shared [`TerminalRuntime`]. Cheap to clone.
pub struct TerminalRuntimeState {
    runtime: TerminalRuntime,
}

impl std::fmt::Debug for TerminalRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalRuntimeState").finish()
    }
}

impl TerminalRuntimeState {
    /// Build the production terminal runtime from the app's capability registry
    /// and Flight Recorder (the SAME recorder the swarm path uses), so terminal
    /// session/command events land in the durable FR.
    pub fn production(
        capabilities: Arc<CapabilityRegistry>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            runtime: TerminalRuntime::new(capabilities, flight_recorder),
        }
    }

    /// Construct directly from an existing runtime handle (tests, or when the
    /// app already built the runtime to hand to capture producers).
    pub fn from_runtime(runtime: TerminalRuntime) -> Self {
        Self { runtime }
    }

    /// The shared runtime handle. Capture producers (cloud CLI bridge, swarm
    /// spawn path, sandbox adapters, MCP stdio) clone this to attach streams.
    pub fn runtime(&self) -> TerminalRuntime {
        self.runtime.clone()
    }
}

// ---------------------------------------------------------------------------
// IPC payloads
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    /// "HUMAN_DEV" | "AI_JOB" | "PLUGIN_TOOL". Defaults to HUMAN_DEV.
    pub session_type: Option<String>,
    pub shell: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub rows: Option<u16>,
    pub cols: Option<u16>,
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    pub instance_id: Option<String>,
    pub title: Option<String>,
    /// Capability ids granted to this session (gates AI interactive exec).
    #[serde(default)]
    pub capability_scope: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoIpc {
    pub session_id: String,
    pub kind: String,
    pub session_type: String,
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    pub instance_id: Option<String>,
    pub trace_id: String,
    pub title: Option<String>,
    pub interactive_authorized: bool,
}

impl From<SessionInfo> for SessionInfoIpc {
    fn from(i: SessionInfo) -> Self {
        Self {
            session_id: i.session_id,
            kind: format!("{:?}", i.kind).to_uppercase(),
            session_type: i.session_type.as_str().to_string(),
            swarm_id: i.binding.swarm_id,
            worktree_id: i.binding.worktree_id,
            instance_id: i.binding.instance_id,
            trace_id: i.trace_id,
            title: i.title,
            interactive_authorized: i.interactive_authorized,
        }
    }
}

/// `terminal://output` payload: one chunk of session output, base64-encoded so
/// raw bytes (incl. control sequences) survive JSON transport intact.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputIpc {
    pub session_id: String,
    pub seq: u64,
    pub chunk_base64: String,
}

/// `terminal://exit` payload.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitIpc {
    pub session_id: String,
    pub exit_code: i32,
}

/// `terminal://resync` payload: the forwarder's broadcast receiver lagged; the
/// front end should re-pull scrollback rather than apply a partial stream.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResyncIpc {
    pub session_id: String,
    pub dropped: u64,
}

fn parse_session_type(s: Option<&str>) -> TerminalSessionType {
    match s.map(|v| v.to_ascii_uppercase()) {
        Some(ref v) if v == "AI_JOB" => TerminalSessionType::AiJob,
        Some(ref v) if v == "PLUGIN_TOOL" => TerminalSessionType::PluginTool,
        _ => TerminalSessionType::HumanDev,
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Create an interactive PTY session. AiJob sessions are inspect-only until
/// `kernel_terminal_authorize_interactive` succeeds.
#[tauri::command]
pub async fn kernel_terminal_create_session(
    req: CreateSessionRequest,
    state: State<'_, TerminalRuntimeState>,
) -> Result<SessionInfoIpc, String> {
    let _ = KERNEL_TERMINAL_CREATE_SESSION_IPC_CHANNEL;
    let binding = SessionBinding {
        swarm_id: req.swarm_id.clone(),
        worktree_id: req.worktree_id.clone(),
        instance_id: req.instance_id.clone(),
    };
    let spawn = PtySpawnConfig {
        shell: req.shell.clone(),
        args: req.args.clone(),
        cwd: req.cwd.clone().map(std::path::PathBuf::from),
        env: Vec::new(),
        rows: req.rows.unwrap_or(24),
        cols: req.cols.unwrap_or(80),
        scrollback_bytes: 0,
        broadcast_capacity: 0,
    };
    let info = state
        .runtime()
        .create_session(
            parse_session_type(req.session_type.as_deref()),
            binding,
            req.capability_scope.clone(),
            spawn,
            req.title.clone(),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(info.into())
}

/// Write base64-encoded stdin to an interactive session. `as_ai` marks AI
/// callers (front end passes false for the human operator at the keyboard).
#[tauri::command]
pub async fn kernel_terminal_write_stdin(
    session_id: String,
    data_base64: String,
    as_ai: Option<bool>,
    state: State<'_, TerminalRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_TERMINAL_WRITE_STDIN_IPC_CHANNEL;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| format!("invalid base64 stdin: {e}"))?;
    state
        .runtime()
        .write_stdin_recorded(&session_id, &bytes, as_ai.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kernel_terminal_resize(
    session_id: String,
    rows: u16,
    cols: u16,
    state: State<'_, TerminalRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_TERMINAL_RESIZE_IPC_CHANNEL;
    state
        .runtime()
        .resize(&session_id, rows, cols)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kernel_terminal_close_session(
    session_id: String,
    state: State<'_, TerminalRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_TERMINAL_CLOSE_SESSION_IPC_CHANNEL;
    state
        .runtime()
        .close_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kernel_terminal_list_sessions(
    state: State<'_, TerminalRuntimeState>,
) -> Result<Vec<SessionInfoIpc>, String> {
    let _ = KERNEL_TERMINAL_LIST_SESSIONS_IPC_CHANNEL;
    Ok(state
        .runtime()
        .list_sessions()
        .into_iter()
        .map(SessionInfoIpc::from)
        .collect())
}

/// Authorize AI interactive stdin ("Take control / interact"). Capability-gated.
#[tauri::command]
pub async fn kernel_terminal_authorize_interactive(
    session_id: String,
    state: State<'_, TerminalRuntimeState>,
) -> Result<(), String> {
    let _ = KERNEL_TERMINAL_AUTHORIZE_INTERACTIVE_IPC_CHANNEL;
    state
        .runtime()
        .authorize_interactive(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kernel_terminal_scrollback(
    session_id: String,
    state: State<'_, TerminalRuntimeState>,
) -> Result<String, String> {
    let _ = KERNEL_TERMINAL_SCROLLBACK_IPC_CHANNEL;
    let bytes = state
        .runtime()
        .scrollback(&session_id)
        .map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// One-shot command request mirroring the interactive create surface but for a
/// fire-and-collect run. Backed by an interactive PTY whose exit is awaited; the
/// captured scrollback is returned base64-encoded.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandRequest {
    pub shell: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub swarm_id: Option<String>,
    #[serde(default)]
    pub capability_scope: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandResult {
    pub session_id: String,
    pub exit_code: i32,
    pub output_base64: String,
}

/// Run a one-shot command in a fresh PTY, await its exit, and return the
/// captured output. The session is closed before returning so it does not leak.
#[tauri::command]
pub async fn kernel_terminal_run_command(
    req: RunCommandRequest,
    state: State<'_, TerminalRuntimeState>,
) -> Result<RunCommandResult, String> {
    let _ = KERNEL_TERMINAL_RUN_COMMAND_IPC_CHANNEL;
    let runtime = state.runtime();
    let binding = SessionBinding {
        swarm_id: req.swarm_id.clone(),
        ..Default::default()
    };
    let spawn = PtySpawnConfig {
        shell: req.shell.clone(),
        args: req.args.clone(),
        cwd: req.cwd.clone().map(std::path::PathBuf::from),
        env: Vec::new(),
        rows: 24,
        cols: 80,
        scrollback_bytes: 0,
        broadcast_capacity: 0,
    };
    let info = runtime
        .create_session(
            TerminalSessionType::HumanDev,
            binding,
            req.capability_scope.clone(),
            spawn,
            Some("one-shot".to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Wait for the child to exit via the latch (correct even for a fast child),
    // then read the authoritative scrollback. The blocking wait runs on a
    // blocking thread so the async runtime is not stalled.
    let rt_for_wait = runtime.clone();
    let sid = info.session_id.clone();
    let exit_code = tokio::task::spawn_blocking(move || {
        rt_for_wait
            .wait_for_exit(&sid, std::time::Duration::from_secs(300))
            .ok()
            .flatten()
            .unwrap_or(-1)
    })
    .await
    .unwrap_or(-1);
    let output = runtime.scrollback(&info.session_id).unwrap_or_default();
    let _ = runtime.close_session(&info.session_id).await;
    Ok(RunCommandResult {
        session_id: info.session_id,
        exit_code,
        output_base64: base64::engine::general_purpose::STANDARD.encode(output),
    })
}

// ---------------------------------------------------------------------------
// Streaming forwarder (mirrors spawn_swarm_board_forwarder)
// ---------------------------------------------------------------------------

/// Subscribe to ONE session's output broadcast and re-emit typed
/// `terminal://output` deltas with a monotonic seq; a lagged receiver emits
/// `terminal://resync` so the front end re-pulls scrollback rather than
/// drifting; an exit emits `terminal://exit` and ends the task.
///
/// `lib.rs` spawns one of these per created session (e.g. from a session-open
/// signal, or eagerly for capture sessions wired by producers).
pub fn spawn_terminal_forwarder(
    app: tauri::AppHandle,
    runtime: TerminalRuntime,
    session_id: String,
) {
    use tauri::Emitter;
    let mut rx = match runtime.subscribe(&session_id) {
        Ok(rx) => rx,
        Err(_) => return,
    };
    tauri::async_runtime::spawn(async move {
        let mut seq: u64 = 0;
        loop {
            match rx.recv().await {
                Ok(SessionOutput::Chunk(bytes)) => {
                    seq = seq.saturating_add(1);
                    let chunk_base64 =
                        base64::engine::general_purpose::STANDARD.encode(&bytes);
                    let _ = app.emit(
                        "terminal://output",
                        TerminalOutputIpc {
                            session_id: session_id.clone(),
                            seq,
                            chunk_base64,
                        },
                    );
                }
                Ok(SessionOutput::Exit(exit_code)) => {
                    let _ = app.emit(
                        "terminal://exit",
                        TerminalExitIpc {
                            session_id: session_id.clone(),
                            exit_code,
                        },
                    );
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(dropped)) => {
                    let _ = app.emit(
                        "terminal://resync",
                        TerminalResyncIpc {
                            session_id: session_id.clone(),
                            dropped,
                        },
                    );
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
