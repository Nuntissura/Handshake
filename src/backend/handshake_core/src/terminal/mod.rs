use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, to_value, Value};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::watch;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    TerminalCommandEvent,
};
use crate::terminal::config::TerminalConfig;
use crate::terminal::guards::{DefaultTerminalGuard, TerminalGuard};
use crate::terminal::redaction::SecretRedactor;

pub mod config;
pub mod guards;
pub mod redaction;
pub mod session;

pub use session::{TerminalSession, TerminalSessionType};

#[derive(Clone, Debug)]
pub enum TerminalMode {
    NonInteractive,
    InteractiveSession,
}

#[derive(Clone, Debug, Default)]
pub struct JobContext {
    pub job_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub capability_profile_id: Option<String>,
    pub capability_id: Option<String>,
    pub wsids: Vec<String>,
}

impl JobContext {
    fn normalize(&mut self) {
        self.job_id = self.job_id.as_ref().map(|j| j.nfc().collect());
        self.model_id = self.model_id.as_ref().map(|m| m.nfc().collect());
        self.session_id = self.session_id.as_ref().map(|s| s.nfc().collect());
        self.capability_profile_id = self
            .capability_profile_id
            .as_ref()
            .map(|p| p.nfc().collect());
        self.capability_id = self.capability_id.as_ref().map(|c| c.nfc().collect());
        self.wsids = self.wsids.iter().map(|w| w.nfc().collect()).collect();
    }
}

#[derive(Clone, Debug)]
pub struct TerminalRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub mode: TerminalMode,
    pub timeout_ms: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub env_overrides: HashMap<String, Option<String>>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub stdin_chunks: Vec<Vec<u8>>,
    pub idempotency_key: Option<String>,
    pub job_context: JobContext,
    pub granted_capabilities: Vec<String>,
    pub requested_capability: Option<String>,
    pub session_type: TerminalSessionType,
    pub human_consent_obtained: bool,
}

impl TerminalRequest {
    pub fn normalize(&mut self) -> Result<(), TerminalError> {
        self.command = self.command.nfc().collect::<String>().trim().to_string();
        if self.command.is_empty() {
            return Err(TerminalError::InvalidRequest(
                "HSK-TERM-001: command is required".to_string(),
            ));
        }

        self.args = self
            .args
            .iter()
            .map(|a| a.nfc().collect::<String>())
            .collect();

        self.job_context.normalize();
        self.granted_capabilities = self
            .granted_capabilities
            .iter()
            .map(|c| c.nfc().collect::<String>())
            .collect();
        self.requested_capability = self
            .requested_capability
            .as_ref()
            .map(|c| c.nfc().collect::<String>());

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TerminalResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub cancelled: bool,
    pub truncated_bytes: u64,
    pub duration_ms: u64,
}

pub type TerminalOutput = TerminalResult;

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("{0}")]
    InvalidRequest(String), // HSK-TERM-001
    #[error("{0}")]
    CapabilityDenied(String), // HSK-TERM-002
    #[error("{0}")]
    CwdViolation(String), // HSK-TERM-003
    #[error("{0}")]
    TimeoutExceeded(String), // HSK-TERM-004
    #[error("{0}")]
    OutputTruncated(String), // HSK-TERM-005
    #[error("{0}")]
    RedactionFailed(String), // HSK-TERM-006
    #[error("{0}")]
    SpawnIo(String), // HSK-TERM-007
    #[error("{0}")]
    NormalizationError(String), // HSK-TERM-008
    #[error("{0}")]
    IsolationViolation(String), // HSK-TERM-009
}

struct CancelEntry {
    sender: watch::Sender<bool>,
    refs: usize,
}

static CANCEL_REGISTRY: Lazy<Mutex<HashMap<String, CancelEntry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn cancel_key(req: &TerminalRequest) -> Option<String> {
    if let Some(key) = req.idempotency_key.clone() {
        if !key.trim().is_empty() {
            return Some(key);
        }
    }
    if let Some(session_id) = req.job_context.session_id.clone() {
        if !session_id.trim().is_empty() {
            return Some(session_id);
        }
    }
    None
}

fn register_cancel_receiver(
    key: Option<String>,
) -> Result<Option<watch::Receiver<bool>>, TerminalError> {
    let key = match key {
        Some(value) => value,
        None => return Ok(None),
    };

    let mut registry = match CANCEL_REGISTRY.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return Err(TerminalError::SpawnIo(
                "HSK-TERM-007: cancellation registry unavailable".to_string(),
            ))
        }
    };

    let entry = registry.entry(key).or_insert_with(|| {
        let (sender, _receiver) = watch::channel(false);
        CancelEntry { sender, refs: 0 }
    });
    entry.refs = entry.refs.saturating_add(1);
    let receiver = entry.sender.subscribe();

    Ok(Some(receiver))
}

fn remove_cancel_key(key: &Option<String>) {
    let key = match key {
        Some(value) => value,
        None => return,
    };

    if let Ok(mut registry) = CANCEL_REGISTRY.lock() {
        if let Some(entry) = registry.get_mut(key) {
            if entry.refs > 0 {
                entry.refs -= 1;
            }
            if entry.refs == 0 {
                registry.remove(key);
            }
        }
    }
}

struct CancelGuard {
    key: Option<String>,
}

impl CancelGuard {
    fn new(key: Option<String>) -> Self {
        Self { key }
    }
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        remove_cancel_key(&self.key);
    }
}

/// Emits a CapabilityAction audit event to the Flight Recorder.
/// Per HSK-4001 audit requirement: every Allow/Deny must be recorded.
async fn emit_capability_audit(
    flight_recorder: &dyn FlightRecorder,
    trace_id: Uuid,
    capability_id: &str,
    profile_id: Option<&str>,
    job_id: Option<&str>,
    outcome: &str,
) -> Result<(), TerminalError> {
    let payload = json!({
        "capability_id": capability_id,
        "profile_id": profile_id,
        "job_id": job_id,
        "outcome": outcome,
    });

    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        FlightRecorderActor::Agent,
        trace_id,
        payload,
    )
    .with_capability(capability_id.to_string());

    if let Some(pid) = profile_id {
        event = event.with_actor_id(pid.to_string());
    }
    if let Some(jid) = job_id {
        event = event.with_job_id(jid.to_string());
    }

    flight_recorder.record_event(event).await.map_err(|e| {
        TerminalError::RedactionFailed(format!("HSK-TERM-006: audit event failed: {}", e))
    })
}

pub struct TerminalService;

impl TerminalService {
    pub fn request_cancel(cancel_key: &str) -> bool {
        let mut registry = match CANCEL_REGISTRY.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        let sender = if let Some(entry) = registry.get(cancel_key) {
            entry.sender.clone()
        } else {
            let (sender, _receiver) = watch::channel(false);
            registry.insert(
                cancel_key.to_string(),
                CancelEntry {
                    sender: sender.clone(),
                    refs: 0,
                },
            );
            sender
        };

        sender.send(true).is_ok()
    }

    pub async fn run_command(
        mut req: TerminalRequest,
        cfg: &TerminalConfig,
        registry: &CapabilityRegistry,
        flight_recorder: &dyn FlightRecorder,
        trace_id: Uuid,
        redactor: &dyn SecretRedactor,
        guards: &[Box<dyn TerminalGuard>],
    ) -> Result<TerminalResult, TerminalError> {
        req.normalize()
            .map_err(|e| TerminalError::NormalizationError(format!("HSK-TERM-008: {e}")))?;

        let default_guard = DefaultTerminalGuard;
        let guard: &dyn TerminalGuard = if let Some(first) = guards.first() {
            first.as_ref()
        } else {
            &default_guard
        };

        // Run pre_exec with audit on CapabilityDenied
        // Clone capability info before mutable borrow
        let pre_exec_cap_id = req
            .requested_capability
            .clone()
            .unwrap_or_else(|| "terminal.exec".to_string());
        let pre_exec_profile_id = req.job_context.capability_profile_id.clone();
        let pre_exec_job_id = req.job_context.job_id.clone();
        if let Err(e) = guard.pre_exec(&mut req, cfg) {
            if matches!(e, TerminalError::CapabilityDenied(_)) {
                emit_capability_audit(
                    flight_recorder,
                    trace_id,
                    &pre_exec_cap_id,
                    pre_exec_profile_id.as_deref(),
                    pre_exec_job_id.as_deref(),
                    "denied",
                )
                .await?;
            }
            return Err(e);
        }
        let cwd = guard.validate_cwd(&req, cfg)?;

        let session = TerminalSession::from_request(&req);

        // Determine if this is an AI-attaching-to-human scenario for audit purposes
        let is_ai_context = req.job_context.job_id.is_some() || req.job_context.model_id.is_some();
        let is_human_dev_session = matches!(session.session_type, TerminalSessionType::HumanDev);
        let is_ai_attach_scenario = is_ai_context && is_human_dev_session;

        // Check session isolation and emit audit event for attach_human if applicable
        match guard.check_session_isolation(&req, &session, registry) {
            Ok(()) => {
                if is_ai_attach_scenario {
                    emit_capability_audit(
                        flight_recorder,
                        trace_id,
                        "terminal.attach_human",
                        req.job_context.capability_profile_id.as_deref(),
                        req.job_context.job_id.as_deref(),
                        "allowed",
                    )
                    .await?;
                }
            }
            Err(e) => {
                if is_ai_attach_scenario {
                    emit_capability_audit(
                        flight_recorder,
                        trace_id,
                        "terminal.attach_human",
                        req.job_context.capability_profile_id.as_deref(),
                        req.job_context.job_id.as_deref(),
                        "denied",
                    )
                    .await?;
                }
                return Err(e);
            }
        }

        // Check capability and emit audit event
        let capability_id = req
            .requested_capability
            .as_deref()
            .unwrap_or("terminal.exec");
        match guard.check_capability(&req, registry) {
            Ok(()) => {
                emit_capability_audit(
                    flight_recorder,
                    trace_id,
                    capability_id,
                    req.job_context.capability_profile_id.as_deref(),
                    req.job_context.job_id.as_deref(),
                    "allowed",
                )
                .await?;
            }
            Err(e) => {
                emit_capability_audit(
                    flight_recorder,
                    trace_id,
                    capability_id,
                    req.job_context.capability_profile_id.as_deref(),
                    req.job_context.job_id.as_deref(),
                    "denied",
                )
                .await?;
                return Err(e);
            }
        }

        let cancel_key = cancel_key(&req);
        let cancel_rx = register_cancel_receiver(cancel_key.clone())?;
        let _cancel_guard = CancelGuard::new(cancel_key);
        let max_output_bytes = cfg.effective_max_output(req.max_output_bytes);
        let effective_timeout = cfg.effective_timeout(req.timeout_ms);

        let mut command = Command::new(&req.command);
        command.args(&req.args);
        command.current_dir(&cwd);
        command.kill_on_drop(true);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        for (k, v) in req.env_overrides.iter() {
            if let Some(value) = v {
                command.env(k, value);
            } else {
                command.env_remove(k);
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {e}")))?;

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let stdout_task = tokio::spawn(collect_output(stdout_handle, max_output_bytes));
        let stderr_task = tokio::spawn(collect_output(stderr_handle, max_output_bytes));

        // WAIVER [CX-573E]: Instant::now() is required for duration measurement (observability only).
        let start = Instant::now();
        let (exit_status, timed_out, cancelled) =
            wait_for_child(&mut child, effective_timeout, cfg.kill_grace_ms, cancel_rx).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let (stdout_bytes, stdout_trunc) = stdout_task
            .await
            .map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?
            .map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
        let (stderr_bytes, stderr_trunc) = stderr_task
            .await
            .map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?
            .map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;

        let truncated_bytes = stdout_trunc + stderr_trunc;
        let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

        #[allow(clippy::manual_unwrap_or)]
        let exit_code = match exit_status.code() {
            Some(code) => code,
            None => -1,
        };

        let event = build_flight_recorder_event(
            TerminalEventContext {
                req: &req,
                session: &session,
                cwd: &cwd,
                exit_code,
                duration_ms,
                timed_out,
                cancelled,
                truncated_bytes,
                cfg,
                stdout: &stdout_bytes,
                stderr: &stderr_bytes,
            },
            redactor,
        );

        let record_result = flight_recorder
            .record_event(event.with_trace_and_actor(trace_id))
            .await;
        if let Err(err) = record_result {
            return Err(TerminalError::RedactionFailed(format!(
                "HSK-TERM-006: failed to record event: {}",
                err
            )));
        }

        Ok(TerminalResult {
            stdout,
            stderr,
            exit_code,
            timed_out,
            cancelled,
            truncated_bytes,
            duration_ms,
        })
    }
}

struct TerminalEventContext<'a> {
    req: &'a TerminalRequest,
    session: &'a TerminalSession,
    cwd: &'a Path,
    exit_code: i32,
    duration_ms: u64,
    timed_out: bool,
    cancelled: bool,
    truncated_bytes: u64,
    cfg: &'a TerminalConfig,
    stdout: &'a [u8],
    stderr: &'a [u8],
}

fn build_flight_recorder_event(
    ctx: TerminalEventContext<'_>,
    redactor: &dyn SecretRedactor,
) -> TerminalEventEnvelope {
    let command_line = if ctx.req.args.is_empty() {
        ctx.req.command.clone()
    } else {
        format!("{} {}", ctx.req.command, ctx.req.args.join(" "))
    };
    let redacted = redactor.redact_command(&command_line);
    let (redacted_output, output_match) = if matches!(
        ctx.cfg.logging_level,
        crate::terminal::config::TerminalLogLevel::CommandsPlusRedactedOutput
    ) && ctx.cfg.redaction_enabled
    {
        let result = redactor.redact_output(ctx.stdout, ctx.stderr);
        (Some(result.redacted), result.matched)
    } else {
        (None, false)
    };
    let redaction_applied = redacted.matched || output_match;
    TerminalEventEnvelope {
        payload: TerminalCommandEvent {
            job_id: ctx.session.job_id.clone(),
            model_id: ctx.req.job_context.model_id.clone(),
            session_id: ctx.session.session_id.clone(),
            wsids: ctx.session.wsids.clone(),
            command: redacted.redacted,
            cwd: ctx.cwd.to_string_lossy().to_string(),
            exit_code: ctx.exit_code,
            duration_ms: ctx.duration_ms,
            timed_out: ctx.timed_out,
            cancelled: ctx.cancelled,
            truncated_bytes: ctx.truncated_bytes,
            capability_id: ctx
                .req
                .requested_capability
                .clone()
                .or_else(|| ctx.req.job_context.capability_id.clone()),
            redaction_applied,
            redacted_output,
            session_type: ctx.session.session_type.as_str().to_string(),
            human_consent_obtained: ctx.session.human_consent_obtained,
            capability_set: ctx.session.capability_set.clone(),
        },
    }
}

struct TerminalEventEnvelope {
    payload: TerminalCommandEvent,
}

impl TerminalEventEnvelope {
    fn with_trace_and_actor(self, trace_id: Uuid) -> FlightRecorderEvent {
        let mut payload = match to_value(&self.payload) {
            Ok(value) => value,
            Err(_) => Value::Null,
        };

        if let Value::Object(map) = &mut payload {
            map.insert(
                "type".to_string(),
                Value::String("terminal_command".to_string()),
            );
        }

        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            trace_id,
            payload,
        );

        if let Some(job_id) = &self.payload.job_id {
            event = event.with_job_id(job_id.clone());
        }
        if let Some(model_id) = &self.payload.model_id {
            event = event.with_model_id(model_id.clone());
        }
        if let Some(session_id) = &self.payload.session_id {
            event = event.with_session_span(session_id.clone());
        }
        if let Some(capability_id) = &self.payload.capability_id {
            event = event.with_capability(capability_id.clone());
        }
        if !self.payload.wsids.is_empty() {
            event = event.with_wsids(self.payload.wsids.clone());
        }
        event.normalize_payload();
        event
    }
}

async fn collect_output<R>(
    reader: Option<R>,
    max_output_bytes: u64,
) -> Result<(Vec<u8>, u64), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut buf = Vec::new();
    let mut truncated = 0u64;
    let mut reader = match reader {
        Some(r) => r,
        None => return Ok((buf, truncated)),
    };

    let mut chunk = [0u8; 4096];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        let available = max_output_bytes.saturating_sub(buf.len() as u64);
        if available > 0 {
            let to_take = std::cmp::min(n as u64, available) as usize;
            buf.extend_from_slice(&chunk[..to_take]);
            if n as u64 > available {
                truncated += n as u64 - available;
            }
        } else {
            truncated += n as u64;
        }
    }

    Ok((buf, truncated))
}

async fn wait_for_cancel(mut cancel_rx: Option<watch::Receiver<bool>>) {
    let mut receiver = match cancel_rx.take() {
        Some(value) => value,
        None => {
            std::future::pending::<()>().await;
            return;
        }
    };

    loop {
        if *receiver.borrow() {
            break;
        }

        if receiver.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

async fn wait_for_child(
    child: &mut tokio::process::Child,
    timeout_ms: u64,
    kill_grace_ms: u64,
    cancel_rx: Option<watch::Receiver<bool>>,
) -> Result<(std::process::ExitStatus, bool, bool), TerminalError> {
    let timeout_duration = Duration::from_millis(timeout_ms);
    let timeout = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout);
    let cancel_wait = wait_for_cancel(cancel_rx);
    tokio::pin!(cancel_wait);

    tokio::select! {
        status = child.wait() => {
            let status = status.map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
            Ok((status, false, false))
        }
        _ = &mut timeout => {
            let _ = child.start_kill();
            let kill_grace = tokio::time::sleep(Duration::from_millis(kill_grace_ms));
            tokio::pin!(kill_grace);
            tokio::select! {
                status = child.wait() => {
                    let status = status.map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
                    Ok((status, true, false))
                }
                _ = &mut kill_grace => {
                    let _ = child.kill().await;
                    let status = child.wait().await.map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
                    Ok((status, true, false))
                }
            }
        }
        _ = &mut cancel_wait => {
            let _ = child.start_kill();
            let kill_grace = tokio::time::sleep(Duration::from_millis(kill_grace_ms));
            tokio::pin!(kill_grace);
            tokio::select! {
                status = child.wait() => {
                    let status = status.map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
                    Ok((status, false, true))
                }
                _ = &mut kill_grace => {
                    let _ = child.kill().await;
                    let status = child.wait().await.map_err(|e| TerminalError::SpawnIo(format!("HSK-TERM-007: {}", e)))?;
                    Ok((status, false, true))
                }
            }
        }
    }
}
