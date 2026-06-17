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

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use base64::Engine;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::FlightRecorder;
use handshake_core::terminal::TerminalSessionType;
use handshake_core::terminal::{
    PtySpawnConfig, SessionBinding, SessionInfo, SessionOutput, TerminalRuntime,
};
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
pub const KERNEL_TERMINAL_CONTEXT_IPC_CHANNEL: &str = "kernel_terminal_context";
pub const KERNEL_TERMINAL_DIAGNOSTICS_IPC_CHANNEL: &str = "kernel_terminal_diagnostics";

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
    pub interactive_allowed: bool,
    pub interactive_authorized: bool,
    pub exited: bool,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalContextIpc {
    pub cwd: String,
    pub default_shell: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDiagnosticsIpc {
    pub receipt_failure_count: u64,
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
            interactive_allowed: matches!(
                i.kind,
                handshake_core::terminal::SessionKind::Interactive
            ) && !i.exited,
            interactive_authorized: i.interactive_authorized,
            exited: i.exited,
            exit_code: i.exit_code,
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

/// Return the backend-resolved context for new HumanDev terminal sessions.
#[tauri::command]
pub fn kernel_terminal_context() -> TerminalContextIpc {
    let _ = KERNEL_TERMINAL_CONTEXT_IPC_CHANNEL;
    let root = crate::workspace_root();
    let root = root.canonicalize().unwrap_or(root);
    TerminalContextIpc {
        cwd: root.to_string_lossy().to_string(),
        // Keep this null so the PTY backend owns platform-default shell
        // resolution (`pwsh.exe` -> `powershell.exe` -> `cmd.exe` on Windows).
        default_shell: None,
    }
}

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

#[tauri::command]
pub async fn kernel_terminal_diagnostics(
    state: State<'_, TerminalRuntimeState>,
) -> Result<TerminalDiagnosticsIpc, String> {
    let _ = KERNEL_TERMINAL_DIAGNOSTICS_IPC_CHANNEL;
    Ok(terminal_diagnostics_with_runtime(state.runtime()))
}

fn terminal_diagnostics_with_runtime(runtime: TerminalRuntime) -> TerminalDiagnosticsIpc {
    TerminalDiagnosticsIpc {
        receipt_failure_count: runtime.terminal_receipt_failure_count(),
    }
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
    pub timeout_ms: Option<u64>,
    pub swarm_id: Option<String>,
    #[serde(default)]
    pub capability_scope: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandResult {
    pub session_id: String,
    pub exit_code: i32,
    pub timed_out: bool,
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
    run_command_with_runtime(req, state.runtime()).await
}

async fn run_command_with_runtime(
    req: RunCommandRequest,
    runtime: TerminalRuntime,
) -> Result<RunCommandResult, String> {
    let binding = SessionBinding {
        swarm_id: req.swarm_id.clone(),
        ..Default::default()
    };
    let cwd_path = req.cwd.clone().map(PathBuf::from);
    let command_line = one_shot_command_line(req.shell.as_deref(), &req.args);
    let spawn = PtySpawnConfig {
        shell: req.shell.clone(),
        args: req.args.clone(),
        cwd: cwd_path.clone(),
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
    let timeout =
        std::time::Duration::from_millis(req.timeout_ms.unwrap_or(300_000).clamp(1, 3_600_000));
    let started_at = Instant::now();
    let observed_exit = tokio::task::spawn_blocking(move || {
        rt_for_wait.wait_for_exit(&sid, timeout).ok().flatten()
    })
    .await
    .unwrap_or(None);
    let duration = started_at.elapsed();
    let timed_out = observed_exit.is_none();
    let exit_code = observed_exit.unwrap_or(-1);
    let output = runtime.scrollback(&info.session_id).unwrap_or_default();
    runtime
        .close_session(&info.session_id)
        .await
        .map_err(|e| format!("terminal cleanup after one-shot command failed: {e}"))?;
    runtime
        .record_one_shot_command_result(
            &info,
            &command_line,
            cwd_path.as_deref(),
            exit_code,
            timed_out,
            duration,
        )
        .await;
    Ok(RunCommandResult {
        session_id: info.session_id,
        exit_code,
        timed_out,
        output_base64: base64::engine::general_purpose::STANDARD.encode(output),
    })
}

fn one_shot_command_line(shell: Option<&str>, args: &[String]) -> String {
    std::iter::once(shell.unwrap_or("<default-shell>").to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
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
                    let chunk_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
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

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
    use handshake_core::flight_recorder::EventFilter;

    #[cfg(windows)]
    fn slow_pty_command() -> (String, Vec<String>) {
        (
            "cmd.exe".to_string(),
            vec![
                "/C".to_string(),
                "ping".to_string(),
                "127.0.0.1".to_string(),
                "-n".to_string(),
                "5".to_string(),
            ],
        )
    }

    #[cfg(windows)]
    fn successful_pty_command() -> (String, Vec<String>) {
        (
            "cmd.exe".to_string(),
            vec![
                "/C".to_string(),
                "echo HANDSHAKE_ONE_SHOT_OK && dir".to_string(),
            ],
        )
    }

    #[cfg(not(windows))]
    fn slow_pty_command() -> (String, Vec<String>) {
        (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), "sleep 2".to_string()],
        )
    }

    #[cfg(not(windows))]
    fn successful_pty_command() -> (String, Vec<String>) {
        (
            "/bin/sh".to_string(),
            vec![
                "-c".to_string(),
                "printf 'HANDSHAKE_ONE_SHOT_OK\\n'; ls".to_string(),
            ],
        )
    }

    #[tokio::test]
    async fn run_command_success_returns_output_records_receipt_and_reclaims_session(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
        let runtime = TerminalRuntime::new(Arc::new(CapabilityRegistry::new()), recorder.clone());
        let (shell, args) = successful_pty_command();

        let result = run_command_with_runtime(
            RunCommandRequest {
                shell: Some(shell),
                args,
                cwd: None,
                timeout_ms: Some(30_000),
                swarm_id: Some("mt-252-success-proof".to_string()),
                capability_scope: Vec::new(),
            },
            runtime.clone(),
        )
        .await?;

        assert!(
            !result.timed_out,
            "successful one-shot must not be marked timeout"
        );
        assert_eq!(result.exit_code, 0);
        let output = String::from_utf8(
            base64::engine::general_purpose::STANDARD.decode(result.output_base64.as_bytes())?,
        )?;
        assert!(
            output.contains("HANDSHAKE_ONE_SHOT_OK"),
            "one-shot command output must include the echo marker, got: {output:?}"
        );
        let events = recorder.list_events(EventFilter::default()).await?;
        assert!(
            events.iter().any(|event| {
                event.payload.get("origin").and_then(|value| value.as_str())
                    == Some("one_shot_run_command")
                    && event
                        .payload
                        .get("timed_out")
                        .and_then(|value| value.as_bool())
                        == Some(false)
                    && event
                        .payload
                        .get("exit_code")
                        .and_then(|value| value.as_i64())
                        == Some(0)
                    && event
                        .payload
                        .get("command")
                        .and_then(|value| value.as_str())
                        .is_some_and(|command| command.contains("HANDSHAKE_ONE_SHOT_OK"))
            }),
            "successful one-shot execution receipt must include command, exit, and timeout=false metadata"
        );
        assert!(
            runtime.list_sessions().is_empty(),
            "successful one-shot PTY session must be closed and removed"
        );
        Ok(())
    }

    #[tokio::test]
    async fn run_command_timeout_is_explicit_and_reclaims_session(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
        let runtime = TerminalRuntime::new(Arc::new(CapabilityRegistry::new()), recorder.clone());
        let (shell, args) = slow_pty_command();

        let result = run_command_with_runtime(
            RunCommandRequest {
                shell: Some(shell),
                args,
                cwd: None,
                timeout_ms: Some(25),
                swarm_id: Some("mt-252-timeout-proof".to_string()),
                capability_scope: Vec::new(),
            },
            runtime.clone(),
        )
        .await?;

        assert!(result.timed_out, "timeout must be explicit, not inferred");
        assert_eq!(result.exit_code, -1);
        let events = recorder.list_events(EventFilter::default()).await?;
        assert!(
            events.iter().any(|event| {
                event.payload.get("origin").and_then(|value| value.as_str())
                    == Some("one_shot_run_command")
                    && event
                        .payload
                        .get("timed_out")
                        .and_then(|value| value.as_bool())
                        == Some(true)
                    && event
                        .payload
                        .get("exit_code")
                        .and_then(|value| value.as_i64())
                        == Some(-1)
            }),
            "one-shot command execution receipt must include timeout metadata"
        );
        assert!(
            runtime.list_sessions().is_empty(),
            "timed-out one-shot PTY session must be closed and removed"
        );
        Ok(())
    }

    struct BlockingOneShotRecorder {
        entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl FlightRecorder for BlockingOneShotRecorder {
        async fn record_event(
            &self,
            event: handshake_core::flight_recorder::FlightRecorderEvent,
        ) -> Result<(), handshake_core::flight_recorder::RecorderError> {
            event.validate()?;
            if event.payload.get("origin").and_then(|value| value.as_str())
                == Some("one_shot_run_command")
            {
                if let Some(sender) = self.entered.lock().expect("entered lock").take() {
                    let _ = sender.send(());
                }
                self.release.notified().await;
            }
            Ok(())
        }

        async fn enforce_retention(
            &self,
        ) -> Result<u64, handshake_core::flight_recorder::RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<
            Vec<handshake_core::flight_recorder::FlightRecorderEvent>,
            handshake_core::flight_recorder::RecorderError,
        > {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn timed_out_run_command_reclaims_session_before_receipt_write_completes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let release = Arc::new(tokio::sync::Notify::new());
        let runtime = TerminalRuntime::new(
            Arc::new(CapabilityRegistry::new()),
            Arc::new(BlockingOneShotRecorder {
                entered: std::sync::Mutex::new(Some(entered_tx)),
                release: release.clone(),
            }),
        );
        let (shell, args) = slow_pty_command();
        let task_runtime = runtime.clone();

        let command_task = tokio::spawn(run_command_with_runtime(
            RunCommandRequest {
                shell: Some(shell),
                args,
                cwd: None,
                timeout_ms: Some(25),
                swarm_id: Some("mt-252-timeout-cleanup-proof".to_string()),
                capability_scope: Vec::new(),
            },
            task_runtime,
        ));

        tokio::time::timeout(std::time::Duration::from_secs(10), entered_rx)
            .await
            .expect("one-shot receipt write should start")?;
        let session_reclaimed_while_receipt_blocked = runtime.list_sessions().is_empty();
        release.notify_waiters();

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), command_task)
            .await
            .expect("one-shot command task should finish after releasing recorder")
            .expect("one-shot command task should not panic")?;

        assert!(result.timed_out);
        assert!(
            session_reclaimed_while_receipt_blocked,
            "timed-out one-shot PTY session must be closed before awaiting receipt persistence"
        );
        Ok(())
    }

    struct FailingRecorder;

    #[async_trait::async_trait]
    impl FlightRecorder for FailingRecorder {
        async fn record_event(
            &self,
            _event: handshake_core::flight_recorder::FlightRecorderEvent,
        ) -> Result<(), handshake_core::flight_recorder::RecorderError> {
            Err(handshake_core::flight_recorder::RecorderError::SinkError(
                "forced receipt failure".to_string(),
            ))
        }

        async fn enforce_retention(
            &self,
        ) -> Result<u64, handshake_core::flight_recorder::RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<
            Vec<handshake_core::flight_recorder::FlightRecorderEvent>,
            handshake_core::flight_recorder::RecorderError,
        > {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn terminal_diagnostics_reports_receipt_failures() {
        let runtime = TerminalRuntime::new(
            Arc::new(CapabilityRegistry::new()),
            Arc::new(FailingRecorder),
        );
        let info = SessionInfo {
            session_id: "diagnostic-session".to_string(),
            kind: handshake_core::terminal::SessionKind::Interactive,
            session_type: TerminalSessionType::HumanDev,
            binding: SessionBinding::default(),
            trace_id: uuid::Uuid::now_v7().to_string(),
            title: Some("diagnostics".to_string()),
            interactive_authorized: false,
            exited: false,
            exit_code: None,
        };

        runtime
            .record_one_shot_command_result(
                &info,
                "cmd.exe /C echo diagnostics",
                None,
                0,
                false,
                std::time::Duration::from_millis(1),
            )
            .await;

        let diagnostics = terminal_diagnostics_with_runtime(runtime);
        assert_eq!(diagnostics.receipt_failure_count, 1);
    }

    #[test]
    fn session_info_ipc_carries_exit_state_for_completed_terminal_tabs() {
        let info = SessionInfo {
            session_id: "exited-session".to_string(),
            kind: handshake_core::terminal::SessionKind::Interactive,
            session_type: TerminalSessionType::HumanDev,
            binding: SessionBinding::default(),
            trace_id: uuid::Uuid::now_v7().to_string(),
            title: Some("exited".to_string()),
            interactive_authorized: true,
            exited: true,
            exit_code: Some(7),
        };

        let ipc: SessionInfoIpc = info.into();

        assert!(ipc.exited);
        assert_eq!(ipc.exit_code, Some(7));
        assert!(
            !ipc.interactive_allowed,
            "completed sessions stay inspectable but cannot accept stdin"
        );
    }

    #[test]
    fn session_info_ipc_keeps_interactive_ai_take_control_reachable_before_authorization() {
        let info = SessionInfo {
            session_id: "interactive-ai".to_string(),
            kind: handshake_core::terminal::SessionKind::Interactive,
            session_type: TerminalSessionType::AiJob,
            binding: SessionBinding::default(),
            trace_id: uuid::Uuid::now_v7().to_string(),
            title: Some("ai shell".to_string()),
            interactive_authorized: false,
            exited: false,
            exit_code: None,
        };

        let ipc: SessionInfoIpc = info.into();

        assert!(
            ipc.interactive_allowed,
            "interactive AiJob PTYs must expose a reachable Take-control path before authorization"
        );
        assert!(!ipc.interactive_authorized);
    }

    #[test]
    fn session_info_ipc_keeps_capture_sessions_non_interactive() {
        let info = SessionInfo {
            session_id: "capture-ai".to_string(),
            kind: handshake_core::terminal::SessionKind::Capture,
            session_type: TerminalSessionType::AiJob,
            binding: SessionBinding::default(),
            trace_id: uuid::Uuid::now_v7().to_string(),
            title: Some("capture".to_string()),
            interactive_authorized: false,
            exited: false,
            exit_code: None,
        };

        let ipc: SessionInfoIpc = info.into();

        assert!(!ipc.interactive_allowed);
        assert!(!ipc.interactive_authorized);
    }
}
