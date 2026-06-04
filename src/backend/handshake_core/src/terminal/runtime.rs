//! `TerminalRuntime`: the registry of many concurrent Integrated Terminal
//! sessions (spec §10.1), plus the capture seam that lets background producers
//! (cloud CLI bridge, sandbox adapters, MCP stdio, swarm spawns) mirror their
//! piped stdout/stderr into a read-only AiJob capture session.
//!
//! This is the CORE deliverable: the "inspect all background work" seam. Each
//! session carries:
//!   * a [`TerminalSessionType`] (HumanDev / AiJob / PluginTool),
//!   * optional `swarm_id` / `worktree_id` / `instance_id` binding (board
//!     swimlane + Flight Recorder correlation),
//!   * a capability scope (granted capability ids),
//!   * a Flight Recorder trace id.
//!
//! Invariants enforced here (TERM-INVARIANTS):
//!   * AI interactive exec is capability-checked before stdin is wired.
//!   * AI MUST NOT type into a HumanDev session by default.
//!   * Every AI-run command / session lifecycle appears in the Flight Recorder.
//!   * Captured output is secret-redacted before it leaves the runtime.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::json;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::terminal::pty::{PtyError, PtyOutput, PtySession, PtySpawnConfig};
use crate::terminal::redaction::{PatternRedactor, SecretRedactor};
use crate::terminal::session::TerminalSessionType;

/// Capability id required before an AI session may write interactive stdin.
pub const CAP_TERMINAL_INTERACT: &str = "terminal.interact";
/// Capability id required before an AI session may attach to a human terminal.
pub const CAP_TERMINAL_ATTACH_HUMAN: &str = "terminal.attach_human";

/// Errors surfaced by the runtime IPC surface.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("HSK-TRT-001: unknown session: {0}")]
    UnknownSession(String),
    #[error("HSK-TRT-002: capability denied: {0}")]
    CapabilityDenied(String),
    #[error("HSK-TRT-003: isolation violation: {0}")]
    Isolation(String),
    #[error("HSK-TRT-004: capture session is read-only and cannot accept stdin")]
    CaptureReadOnly,
    #[error(transparent)]
    Pty(#[from] PtyError),
}

/// How a session's output originates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionKind {
    /// A live interactive PTY (a real shell).
    Interactive,
    /// A read-only capture session fed by a background producer's piped output.
    Capture,
}

/// Board/Flight-Recorder binding for a session.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SessionBinding {
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    pub instance_id: Option<String>,
}

/// Public, serializable descriptor of a live session (for `list`).
#[derive(Clone, Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub kind: SessionKind,
    pub session_type: TerminalSessionType,
    pub binding: SessionBinding,
    pub trace_id: String,
    pub title: Option<String>,
    /// True when AI stdin has been authorized for an interactive AiJob session.
    pub interactive_authorized: bool,
}

/// One output fan-out item exposed to the Tauri forwarder. Mirrors
/// [`PtyOutput`] but also carries capture-session chunks.
#[derive(Clone, Debug)]
pub enum SessionOutput {
    Chunk(Vec<u8>),
    Exit(i32),
}

impl From<PtyOutput> for SessionOutput {
    fn from(v: PtyOutput) -> Self {
        match v {
            PtyOutput::Chunk(b) => SessionOutput::Chunk(b),
            PtyOutput::Exit(c) => SessionOutput::Exit(c),
        }
    }
}

/// Internal per-session record.
struct SessionEntry {
    info: SessionInfo,
    capability_scope: Vec<String>,
    // Exactly one of these is populated depending on `kind`.
    pty: Option<Arc<PtySession>>,
    capture_tx: Option<broadcast::Sender<SessionOutput>>,
    capture_scrollback: Option<Arc<Mutex<Vec<u8>>>>,
    // Forwarder relay: interactive PTY output is re-broadcast through a
    // SessionOutput channel so the runtime exposes one uniform stream type.
    relay_tx: broadcast::Sender<SessionOutput>,
}

/// The shared runtime. Cheaply clonable (`Arc` inside) so it can be both the
/// Tauri managed state and the handle handed to capture producers.
#[derive(Clone)]
pub struct TerminalRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    sessions: Mutex<HashMap<String, SessionEntry>>,
    capabilities: Arc<CapabilityRegistry>,
    flight_recorder: Arc<dyn FlightRecorder>,
    redactor: Arc<dyn SecretRedactor>,
    redaction_enabled: bool,
    broadcast_capacity: usize,
    scrollback_bytes: usize,
}

impl TerminalRuntime {
    pub fn new(
        capabilities: Arc<CapabilityRegistry>,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                sessions: Mutex::new(HashMap::new()),
                capabilities,
                flight_recorder,
                redactor: Arc::new(PatternRedactor),
                redaction_enabled: true,
                broadcast_capacity: crate::terminal::pty::DEFAULT_BROADCAST_CAPACITY,
                scrollback_bytes: crate::terminal::pty::DEFAULT_SCROLLBACK_BYTES,
            }),
        }
    }

    /// Subscribe to one session's output stream (chunks + exit).
    pub fn subscribe(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Receiver<SessionOutput>, RuntimeError> {
        let sessions = self.lock_sessions();
        let entry = sessions
            .get(session_id)
            .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
        Ok(entry.relay_tx.subscribe())
    }

    /// Create an interactive PTY session (a real shell). For an AiJob session,
    /// stdin remains BLOCKED until [`authorize_interactive`] succeeds — the
    /// session is inspect-only on creation, honoring TERM-INVARIANTS.
    pub async fn create_session(
        &self,
        session_type: TerminalSessionType,
        binding: SessionBinding,
        capability_scope: Vec<String>,
        spawn: PtySpawnConfig,
        title: Option<String>,
    ) -> Result<SessionInfo, RuntimeError> {
        let mut spawn = spawn;
        spawn.broadcast_capacity = spawn.broadcast_capacity.max(self.inner.broadcast_capacity);
        spawn.scrollback_bytes = if spawn.scrollback_bytes == 0 {
            self.inner.scrollback_bytes
        } else {
            spawn.scrollback_bytes
        };

        let pty = Arc::new(PtySession::spawn(spawn)?);
        let session_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7();

        let (relay_tx, _r) = broadcast::channel(self.inner.broadcast_capacity.max(1));

        // Relay PTY output through the uniform SessionOutput channel. A
        // dedicated thread bridges the blocking-free async broadcast; redaction
        // is applied to chunk bytes before they leave the runtime.
        let relay = relay_tx.clone();
        let mut pty_rx = pty.subscribe();
        let redactor = Arc::clone(&self.inner.redactor);
        let redaction_enabled = self.inner.redaction_enabled;
        tokio::spawn(async move {
            loop {
                match pty_rx.recv().await {
                    Ok(PtyOutput::Chunk(bytes)) => {
                        let out = redact_chunk(&*redactor, redaction_enabled, &bytes);
                        let _ = relay.send(SessionOutput::Chunk(out));
                    }
                    Ok(PtyOutput::Exit(code)) => {
                        let _ = relay.send(SessionOutput::Exit(code));
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        let info = SessionInfo {
            session_id: session_id.clone(),
            kind: SessionKind::Interactive,
            session_type,
            binding: binding.clone(),
            trace_id: trace_id.to_string(),
            title,
            interactive_authorized: matches!(session_type, TerminalSessionType::HumanDev),
        };

        {
            let mut sessions = self.lock_sessions();
            sessions.insert(
                session_id.clone(),
                SessionEntry {
                    info: info.clone(),
                    capability_scope,
                    pty: Some(pty),
                    capture_tx: None,
                    capture_scrollback: None,
                    relay_tx,
                },
            );
        }

        self.record_session_open(&info).await;
        Ok(info)
    }

    /// Create a read-only capture session bound to a background producer (the
    /// CORE seam). Returns both the session info and a [`CaptureSink`] the
    /// producer feeds piped bytes into.
    pub async fn create_capture_session(
        &self,
        binding: SessionBinding,
        title: Option<String>,
    ) -> (SessionInfo, CaptureSink) {
        let session_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7();
        let (relay_tx, _r) = broadcast::channel(self.inner.broadcast_capacity.max(1));
        let scrollback = Arc::new(Mutex::new(Vec::<u8>::new()));

        let info = SessionInfo {
            session_id: session_id.clone(),
            kind: SessionKind::Capture,
            // Background producers are AI work; capture sessions are AiJob and
            // read-only by construction.
            session_type: TerminalSessionType::AiJob,
            binding: binding.clone(),
            trace_id: trace_id.to_string(),
            title,
            interactive_authorized: false,
        };

        {
            let mut sessions = self.lock_sessions();
            sessions.insert(
                session_id.clone(),
                SessionEntry {
                    info: info.clone(),
                    capability_scope: Vec::new(),
                    pty: None,
                    capture_tx: Some(relay_tx.clone()),
                    capture_scrollback: Some(Arc::clone(&scrollback)),
                    relay_tx,
                },
            );
        }

        self.record_session_open(&info).await;

        let sink = CaptureSink {
            runtime: self.clone(),
            session_id,
            trace_id,
            tx: self.capture_tx_for(&info.session_id),
            scrollback,
            binding,
            redactor: Arc::clone(&self.inner.redactor),
            redaction_enabled: self.inner.redaction_enabled,
            scrollback_cap: self.inner.scrollback_bytes,
            closed: std::sync::atomic::AtomicBool::new(false),
        };
        (info, sink)
    }

    fn capture_tx_for(&self, session_id: &str) -> broadcast::Sender<SessionOutput> {
        let sessions = self.lock_sessions();
        sessions
            .get(session_id)
            .and_then(|e| e.capture_tx.clone())
            .expect("capture session just inserted")
    }

    /// Authorize interactive stdin for an AiJob session. This is the explicit
    /// "Take control / interact" gate: it MUST pass a capability check before
    /// any stdin is accepted (TERM-INVARIANTS: AI command exec is
    /// capability-checked + trace-linked).
    pub async fn authorize_interactive(&self, session_id: &str) -> Result<(), RuntimeError> {
        let (session_type, scope, trace_id) = {
            let sessions = self.lock_sessions();
            let entry = sessions
                .get(session_id)
                .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
            (
                entry.info.session_type,
                entry.capability_scope.clone(),
                entry.info.trace_id.clone(),
            )
        };

        // HumanDev sessions are interactive for the human by default.
        if matches!(session_type, TerminalSessionType::HumanDev) {
            return Ok(());
        }

        let allowed = self
            .inner
            .capabilities
            .enforce_can_perform(CAP_TERMINAL_INTERACT, &scope)
            .map_err(|e| RuntimeError::CapabilityDenied(e.to_string()))?;
        let outcome = if allowed { "allow" } else { "deny" };
        self.record_capability_action(
            CAP_TERMINAL_INTERACT,
            session_id,
            &trace_id,
            outcome,
        )
        .await;
        if !allowed {
            return Err(RuntimeError::CapabilityDenied(format!(
                "{CAP_TERMINAL_INTERACT} not granted for AI session {session_id}"
            )));
        }

        let mut sessions = self.lock_sessions();
        if let Some(entry) = sessions.get_mut(session_id) {
            entry.info.interactive_authorized = true;
        }
        Ok(())
    }

    /// Write stdin to an interactive session. Enforces:
    ///   * capture sessions are read-only (HSK-TRT-004),
    ///   * AI may not type into a HumanDev session unless explicitly attached
    ///     (HSK-TRT-003) — by default AI is BLOCKED,
    ///   * AI interactive write requires prior [`authorize_interactive`].
    ///
    /// `as_ai` marks whether the caller acts on behalf of an AI job (vs. the
    /// human operator at the keyboard).
    pub fn write_stdin(
        &self,
        session_id: &str,
        bytes: &[u8],
        as_ai: bool,
    ) -> Result<(), RuntimeError> {
        let sessions = self.lock_sessions();
        let entry = sessions
            .get(session_id)
            .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;

        if entry.info.kind == SessionKind::Capture {
            return Err(RuntimeError::CaptureReadOnly);
        }

        // TERM-INVARIANT: AI MUST NOT type into a human terminal by default.
        if as_ai && matches!(entry.info.session_type, TerminalSessionType::HumanDev) {
            return Err(RuntimeError::Isolation(
                "AI cannot write to a HUMAN_DEV terminal".to_string(),
            ));
        }

        // TERM-INVARIANT: AI interactive exec must be authorized first.
        if as_ai && !entry.info.interactive_authorized {
            return Err(RuntimeError::CapabilityDenied(format!(
                "AI session {session_id} not authorized for interactive stdin"
            )));
        }

        let pty = entry
            .pty
            .as_ref()
            .ok_or(RuntimeError::CaptureReadOnly)?;
        pty.write_stdin(bytes)?;
        Ok(())
    }

    /// Write stdin AND, when the caller is an AI, record a per-command Flight
    /// Recorder event so EVERY AI interactive command is trace-linked (not just
    /// the one-time `authorize_interactive` decision).
    ///
    /// TERM-INVARIANT ("AI command exec MUST be capability-checked + trace-linked";
    /// "every AI-run command MUST appear in the Flight Recorder"): the one-time
    /// capability gate at [`authorize_interactive`] authorizes the *channel*; this
    /// method records each individual AI-typed command so the per-command audit
    /// trail is complete. The (redacted) stdin payload is carried so the FR shows
    /// what the AI ran, with secrets stripped. Human writes (`as_ai == false`) are
    /// not per-command recorded here (they are the operator at the keyboard, not
    /// AI-attributable exec).
    pub async fn write_stdin_recorded(
        &self,
        session_id: &str,
        bytes: &[u8],
        as_ai: bool,
    ) -> Result<(), RuntimeError> {
        // Enforce the gates + perform the write synchronously first; only on a
        // successful AI write do we emit the per-command FR event (so a denied
        // command is not recorded as executed).
        self.write_stdin(session_id, bytes, as_ai)?;
        if as_ai {
            let (session_type, trace_id) = {
                let sessions = self.lock_sessions();
                match sessions.get(session_id) {
                    Some(e) => (e.info.session_type, e.info.trace_id.clone()),
                    None => return Ok(()),
                }
            };
            self.record_interactive_command(session_id, &trace_id, session_type, bytes)
                .await;
        }
        Ok(())
    }

    /// Resize an interactive session. No-op for capture sessions.
    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), RuntimeError> {
        let sessions = self.lock_sessions();
        let entry = sessions
            .get(session_id)
            .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
        if let Some(pty) = &entry.pty {
            pty.resize(rows, cols)?;
        }
        Ok(())
    }

    /// Block until an interactive session's child exits (or `timeout` elapses),
    /// returning the exit code. Latch-based, so it is correct even for a fast
    /// child that finished before the caller attached. Capture sessions have no
    /// child and return `None`.
    pub fn wait_for_exit(
        &self,
        session_id: &str,
        timeout: std::time::Duration,
    ) -> Result<Option<i32>, RuntimeError> {
        let pty = {
            let sessions = self.lock_sessions();
            let entry = sessions
                .get(session_id)
                .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
            entry.pty.clone()
        };
        match pty {
            Some(pty) => Ok(pty.wait_for_exit(timeout)),
            None => Ok(None),
        }
    }

    /// Scrollback snapshot for backfilling a freshly-attached xterm.js terminal.
    pub fn scrollback(&self, session_id: &str) -> Result<Vec<u8>, RuntimeError> {
        let sessions = self.lock_sessions();
        let entry = sessions
            .get(session_id)
            .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
        if let Some(pty) = &entry.pty {
            Ok(pty.scrollback())
        } else if let Some(sb) = &entry.capture_scrollback {
            Ok(sb.lock().map(|b| b.clone()).unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }

    /// Close a session: kill the child (interactive), drop the entry, and record
    /// the close to the Flight Recorder.
    pub async fn close_session(&self, session_id: &str) -> Result<(), RuntimeError> {
        let info = {
            let mut sessions = self.lock_sessions();
            let entry = sessions
                .remove(session_id)
                .ok_or_else(|| RuntimeError::UnknownSession(session_id.to_string()))?;
            if let Some(pty) = &entry.pty {
                pty.kill();
            }
            entry.info
        };
        self.record_session_close(&info).await;
        Ok(())
    }

    /// Synchronously remove a capture session from the registry without the
    /// async FR close event. Used by [`CaptureSink`]'s [`Drop`] leak guard, which
    /// runs in a synchronous (possibly non-async) context — a dropped producer
    /// must not leave a ghost session in the "inspect all background work" panel.
    /// Returns true if a session was actually removed.
    fn reap_capture_session(&self, session_id: &str) -> bool {
        let mut sessions = self.lock_sessions();
        sessions.remove(session_id).is_some()
    }

    /// List all live sessions (for the panel's tab strip / board affordance).
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.lock_sessions();
        let mut out: Vec<SessionInfo> = sessions.values().map(|e| e.info.clone()).collect();
        out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        out
    }

    fn lock_sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, SessionEntry>> {
        // Poisoned mutex: recover the guard; session map is plain data.
        self.inner
            .sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    // ---- Flight Recorder emission ----------------------------------------

    async fn record_session_open(&self, info: &SessionInfo) {
        // FR-EVT-TERMINAL-SESSION-OPEN
        let payload = json!({
            "type": "terminal_command",
            "fr_event": "FR-EVT-TERMINAL-SESSION-OPEN",
            "session_id": info.session_id,
            "session_type": info.session_type.as_str(),
            "kind": info.kind,
            "swarm_id": info.binding.swarm_id,
            "worktree_id": info.binding.worktree_id,
            "instance_id": info.binding.instance_id,
            "command": "<session-open>",
            "cwd": "",
            "exit_code": 0,
            "duration_ms": 0,
            "timed_out": false,
            "cancelled": false,
            "truncated_bytes": 0,
            "redaction_applied": false,
            "human_consent_obtained": false,
        });
        self.emit(info, FlightRecorderEventType::TerminalCommand, payload)
            .await;
    }

    async fn record_session_close(&self, info: &SessionInfo) {
        // FR-EVT-TERMINAL-SESSION-CLOSE
        let payload = json!({
            "type": "terminal_command",
            "fr_event": "FR-EVT-TERMINAL-SESSION-CLOSE",
            "session_id": info.session_id,
            "session_type": info.session_type.as_str(),
            "kind": info.kind,
            "swarm_id": info.binding.swarm_id,
            "worktree_id": info.binding.worktree_id,
            "instance_id": info.binding.instance_id,
            "command": "<session-close>",
            "cwd": "",
            "exit_code": 0,
            "duration_ms": 0,
            "timed_out": false,
            "cancelled": false,
            "truncated_bytes": 0,
            "redaction_applied": false,
            "human_consent_obtained": false,
        });
        self.emit(info, FlightRecorderEventType::TerminalCommand, payload)
            .await;
    }

    async fn emit(
        &self,
        info: &SessionInfo,
        event_type: FlightRecorderEventType,
        payload: serde_json::Value,
    ) {
        let trace = Uuid::parse_str(&info.trace_id).unwrap_or_else(|_| Uuid::now_v7());
        let mut event =
            FlightRecorderEvent::new(event_type, FlightRecorderActor::Agent, trace, payload)
                .with_actor_id("terminal_runtime")
                .with_session_span(info.session_id.clone());
        if let Some(swarm) = &info.binding.swarm_id {
            event = event.with_job_id(swarm.clone());
        }
        let _ = self.inner.flight_recorder.record_event(event).await;
    }

    /// FR-EVT-TERMINAL-COMMAND-EXEC for a single AI interactive stdin write. The
    /// payload carries the redacted command so the Flight Recorder records what
    /// the AI ran (secrets stripped), trace-linked to the session.
    async fn record_interactive_command(
        &self,
        session_id: &str,
        trace_id: &str,
        session_type: TerminalSessionType,
        bytes: &[u8],
    ) {
        let trace = Uuid::parse_str(trace_id).unwrap_or_else(|_| Uuid::now_v7());
        let redacted = if self.inner.redaction_enabled {
            self.inner.redactor.redact_chunk(bytes).redacted
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        };
        let payload = json!({
            "type": "terminal_command",
            "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
            "session_id": session_id,
            "session_type": session_type.as_str(),
            "command": redacted.trim_end_matches(['\r', '\n']),
            "origin": "ai_interactive_stdin",
            "cwd": "",
            "exit_code": 0,
            "duration_ms": 0,
            "timed_out": false,
            "cancelled": false,
            "truncated_bytes": 0,
            "redaction_applied": self.inner.redaction_enabled,
            "human_consent_obtained": false,
        });
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            trace,
            payload,
        )
        .with_actor_id("terminal_runtime")
        .with_session_span(session_id.to_string());
        let _ = self.inner.flight_recorder.record_event(event).await;
    }

    async fn record_capability_action(
        &self,
        capability_id: &str,
        session_id: &str,
        trace_id: &str,
        outcome: &str,
    ) {
        let trace = Uuid::parse_str(trace_id).unwrap_or_else(|_| Uuid::now_v7());
        // CapabilityAction payload is validated with exact keys
        // (capability_id, actor_id, job_id, decision_outcome). We thread the
        // session id through job_id so the action stays trace/session-linked.
        let payload = json!({
            "capability_id": capability_id,
            "actor_id": "terminal_runtime",
            "job_id": session_id,
            "decision_outcome": outcome,
        });
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::CapabilityAction,
            FlightRecorderActor::Agent,
            trace,
            payload,
        )
        .with_actor_id("terminal_runtime")
        .with_capability(capability_id.to_string())
        .with_session_span(session_id.to_string());
        let _ = self.inner.flight_recorder.record_event(event).await;
    }
}

fn redact_chunk(redactor: &dyn SecretRedactor, enabled: bool, bytes: &[u8]) -> Vec<u8> {
    if !enabled {
        return bytes.to_vec();
    }
    // Chunk-oriented redaction: a single stream with no stdout/stderr join, so a
    // redacted chunk stays byte-faithful (no spurious trailing newline). A
    // non-matching chunk is returned unchanged.
    let result = redactor.redact_chunk(bytes);
    if result.matched {
        result.redacted.into_bytes()
    } else {
        bytes.to_vec()
    }
}

/// A producer-facing handle for feeding a background stream into a read-only
/// capture session. Cheap to hold; the producer calls [`CaptureSink::feed`] for
/// each chunk and [`CaptureSink::close`] when the stream ends.
///
/// Fan-out per chunk: (1) redact secrets, (2) append to the capture scrollback
/// under a byte cap, (3) broadcast to the relay (Tauri forwarder + board),
/// (4) record a Flight Recorder command-exec event.
pub struct CaptureSink {
    runtime: TerminalRuntime,
    session_id: String,
    trace_id: Uuid,
    tx: broadcast::Sender<SessionOutput>,
    scrollback: Arc<Mutex<Vec<u8>>>,
    binding: SessionBinding,
    redactor: Arc<dyn SecretRedactor>,
    redaction_enabled: bool,
    scrollback_cap: usize,
    /// Set once [`CaptureSink::close`] has run. Guards the [`Drop`] reaper so a
    /// normally-closed sink is not double-removed.
    closed: std::sync::atomic::AtomicBool,
}

impl CaptureSink {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Feed one chunk of a background producer's piped output into the capture
    /// session. Redacts, caps scrollback, fans out to broadcast + FR.
    pub async fn feed(&self, bytes: &[u8]) {
        let redacted = redact_chunk(&*self.redactor, self.redaction_enabled, bytes);
        {
            if let Ok(mut sb) = self.scrollback.lock() {
                sb.extend_from_slice(&redacted);
                if sb.len() > self.scrollback_cap {
                    let overflow = sb.len() - self.scrollback_cap;
                    sb.drain(0..overflow);
                }
            }
        }
        let _ = self.tx.send(SessionOutput::Chunk(redacted));

        // FR-EVT-TERMINAL-COMMAND-EXEC: every captured background stream chunk
        // is trace-linked so AI background work appears in the Flight Recorder.
        let payload = json!({
            "type": "terminal_command",
            "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
            "session_id": self.session_id,
            "session_type": TerminalSessionType::AiJob.as_str(),
            "swarm_id": self.binding.swarm_id,
            "instance_id": self.binding.instance_id,
            "command": "<captured-output>",
            "cwd": "",
            "exit_code": 0,
            "duration_ms": 0,
            "timed_out": false,
            "cancelled": false,
            "truncated_bytes": 0,
            "redaction_applied": self.redaction_enabled,
            "human_consent_obtained": false,
        });
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            self.trace_id,
            payload,
        )
        .with_actor_id("terminal_capture")
        .with_session_span(self.session_id.clone());
        let _ = self
            .runtime
            .inner
            .flight_recorder
            .record_event(event)
            .await;
    }

    /// Close the capture session when the background stream ends. Marks the sink
    /// closed so the [`Drop`] reaper does not also try to remove the (now gone)
    /// session.
    pub async fn close(self, exit_code: i32) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
        let _ = self.tx.send(SessionOutput::Exit(exit_code));
        let _ = self.runtime.close_session(&self.session_id).await;
    }
}

impl Drop for CaptureSink {
    fn drop(&mut self) {
        // Leak guard (TERM hardening: "session leak -> teardown"): if a producer
        // is dropped WITHOUT awaiting `close()` — task cancellation, a producer
        // panic, or the future being dropped mid-stream — the capture session
        // would otherwise stay in the registry forever and accumulate as an
        // orphaned ghost in the "inspect all background work" panel. Best-effort,
        // synchronous reap: signal exit on the relay and remove the session entry
        // directly (no async needed, since `close_session`'s only extra work is
        // the FR close event, which we emit via the synchronous reap path).
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        // Signal consumers the stream ended abnormally (exit code -1).
        let _ = self.tx.send(SessionOutput::Exit(-1));
        self.runtime.reap_capture_session(&self.session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::{EventFilter, RecorderError};
    use async_trait::async_trait;

    // ---- Minimal in-memory recorder that counts events by fr_event tag -----
    #[derive(Default)]
    struct CountingRecorder {
        events: Mutex<Vec<FlightRecorderEvent>>,
    }

    #[async_trait]
    impl FlightRecorder for CountingRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }
        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self.events.lock().unwrap().clone())
        }
    }

    impl CountingRecorder {
        fn fr_events(&self) -> Vec<String> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .filter_map(|e| {
                    e.payload
                        .get("fr_event")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        }
        fn capability_outcomes(&self) -> Vec<(String, String)> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .filter_map(|e| {
                    let cap = e.payload.get("capability_id")?.as_str()?.to_string();
                    let outcome = e.payload.get("decision_outcome")?.as_str()?.to_string();
                    Some((cap, outcome))
                })
                .collect()
        }
    }

    fn registry_with(_caps: &[&str]) -> Arc<CapabilityRegistry> {
        // The default registry already knows the canonical capability ids
        // (including terminal.interact / terminal.attach_human). Grants are
        // supplied per-session via `capability_scope`, so the registry itself
        // needs no per-test seeding.
        Arc::new(CapabilityRegistry::new())
    }

    fn echo_spawn() -> PtySpawnConfig {
        let mut cfg = PtySpawnConfig::default();
        if cfg!(windows) {
            cfg.shell = Some("cmd.exe".to_string());
            cfg.args = vec!["/C".to_string(), "echo RUNTIME_OK".to_string()];
        } else {
            cfg.shell = Some("/bin/sh".to_string());
            cfg.args = vec!["-c".to_string(), "printf 'RUNTIME_OK\\n'".to_string()];
        }
        cfg
    }

    #[tokio::test]
    async fn capture_seam_fans_to_broadcast_and_fr() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder.clone());
        let binding = SessionBinding {
            swarm_id: Some("swarm-1".to_string()),
            instance_id: Some("inst-1".to_string()),
            ..Default::default()
        };
        let (info, sink) = rt.create_capture_session(binding, Some("cloud-cli".into())).await;
        let mut rx = rt.subscribe(&info.session_id).unwrap();
        sink.feed(b"hello from background\n").await;

        let received = rx.recv().await.unwrap();
        match received {
            SessionOutput::Chunk(b) => {
                assert!(String::from_utf8_lossy(&b).contains("hello from background"))
            }
            _ => panic!("expected chunk"),
        }
        let fr = recorder.fr_events();
        assert!(fr.contains(&"FR-EVT-TERMINAL-SESSION-OPEN".to_string()));
        assert!(fr.contains(&"FR-EVT-TERMINAL-COMMAND-EXEC".to_string()));
        // scrollback backfill works
        let sb = rt.scrollback(&info.session_id).unwrap();
        assert!(String::from_utf8_lossy(&sb).contains("hello from background"));
    }

    #[tokio::test]
    async fn captured_output_is_redacted() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder.clone());
        let (info, sink) = rt
            .create_capture_session(SessionBinding::default(), None)
            .await;
        let mut rx = rt.subscribe(&info.session_id).unwrap();
        sink.feed(b"export API_KEY=supersecretvalue123\n").await;
        let received = rx.recv().await.unwrap();
        match received {
            SessionOutput::Chunk(b) => {
                let text = String::from_utf8_lossy(&b);
                assert!(text.contains("REDACTED"), "secret must be redacted: {text}");
                assert!(!text.contains("supersecretvalue123"));
            }
            _ => panic!("expected chunk"),
        }
    }

    #[tokio::test]
    async fn capture_session_is_read_only() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder);
        let (info, _sink) = rt
            .create_capture_session(SessionBinding::default(), None)
            .await;
        let err = rt
            .write_stdin(&info.session_id, b"whoami\n", true)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::CaptureReadOnly));
    }

    #[tokio::test]
    async fn ai_cannot_write_to_human_session() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder);
        let info = rt
            .create_session(
                TerminalSessionType::HumanDev,
                SessionBinding::default(),
                vec![],
                echo_spawn(),
                None,
            )
            .await
            .unwrap();
        // Human (as_ai=false) is allowed; AI (as_ai=true) is blocked.
        let err = rt
            .write_stdin(&info.session_id, b"echo x\n", true)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::Isolation(_)));
        let _ = rt.close_session(&info.session_id).await;
    }

    #[tokio::test]
    async fn ai_interactive_requires_capability_gate() {
        let recorder = Arc::new(CountingRecorder::default());
        // No CAP_TERMINAL_INTERACT granted -> deny.
        let rt = TerminalRuntime::new(registry_with(&[CAP_TERMINAL_INTERACT]), recorder.clone());
        let info = rt
            .create_session(
                TerminalSessionType::AiJob,
                SessionBinding::default(),
                vec![], // empty scope: not granted
                echo_spawn(),
                None,
            )
            .await
            .unwrap();
        // Unauthorized AI write is blocked.
        let err = rt
            .write_stdin(&info.session_id, b"echo x\n", true)
            .unwrap_err();
        assert!(matches!(err, RuntimeError::CapabilityDenied(_)));
        // authorize_interactive denies because scope lacks the capability.
        let auth = rt.authorize_interactive(&info.session_id).await;
        assert!(auth.is_err());
        let outcomes = recorder.capability_outcomes();
        assert!(outcomes
            .iter()
            .any(|(c, o)| c == CAP_TERMINAL_INTERACT && o == "deny"));
        let _ = rt.close_session(&info.session_id).await;
    }

    #[tokio::test]
    async fn ai_interactive_allowed_with_capability() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[CAP_TERMINAL_INTERACT]), recorder.clone());
        let info = rt
            .create_session(
                TerminalSessionType::AiJob,
                SessionBinding::default(),
                vec![CAP_TERMINAL_INTERACT.to_string()],
                echo_spawn(),
                None,
            )
            .await
            .unwrap();
        rt.authorize_interactive(&info.session_id)
            .await
            .expect("authorize ok");
        // Now AI stdin is accepted (the echo child may have exited; write may
        // surface a pipe error, which is fine — the gate itself passed).
        let _ = rt.write_stdin(&info.session_id, b"echo x\n", true);
        let outcomes = recorder.capability_outcomes();
        assert!(outcomes
            .iter()
            .any(|(c, o)| c == CAP_TERMINAL_INTERACT && o == "allow"));
        let _ = rt.close_session(&info.session_id).await;
    }

    #[tokio::test]
    async fn ai_interactive_command_is_recorded_per_command() {
        // TERM-INVARIANT: every AI interactive command (not just the one-time
        // authorize) must appear in the Flight Recorder as a COMMAND-EXEC.
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[CAP_TERMINAL_INTERACT]), recorder.clone());
        let info = rt
            .create_session(
                TerminalSessionType::AiJob,
                SessionBinding::default(),
                vec![CAP_TERMINAL_INTERACT.to_string()],
                echo_spawn(),
                None,
            )
            .await
            .unwrap();
        rt.authorize_interactive(&info.session_id)
            .await
            .expect("authorize ok");
        // Count COMMAND-EXEC events before the write.
        let before = recorder
            .fr_events()
            .iter()
            .filter(|e| *e == "FR-EVT-TERMINAL-COMMAND-EXEC")
            .count();
        // The AI typing a command must be recorded even if the underlying write
        // fails (fast echo child may have closed stdin); the gate passed, so the
        // command is an attributable AI exec. Record happens only on a successful
        // write, so spawn a long-lived shell instead to ensure the write lands.
        // Use a shell that stays open (no /C) so stdin is writable.
        let mut cfg = PtySpawnConfig::default();
        if cfg!(windows) {
            cfg.shell = Some("cmd.exe".to_string());
        } else {
            cfg.shell = Some("/bin/sh".to_string());
        }
        let shell = rt
            .create_session(
                TerminalSessionType::AiJob,
                SessionBinding::default(),
                vec![CAP_TERMINAL_INTERACT.to_string()],
                cfg,
                None,
            )
            .await
            .unwrap();
        rt.authorize_interactive(&shell.session_id)
            .await
            .expect("authorize shell");
        rt.write_stdin_recorded(&shell.session_id, b"echo HELLO\n", true)
            .await
            .expect("ai write recorded");
        let after = recorder
            .fr_events()
            .iter()
            .filter(|e| *e == "FR-EVT-TERMINAL-COMMAND-EXEC")
            .count();
        assert!(
            after > before,
            "AI interactive command must emit FR-EVT-TERMINAL-COMMAND-EXEC (before={before}, after={after})"
        );
        // The recorded command must carry the (redacted) payload.
        let has_cmd = recorder
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|e| {
                e.payload
                    .get("origin")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "ai_interactive_stdin")
                    .unwrap_or(false)
            });
        assert!(has_cmd, "interactive command FR event must be tagged ai_interactive_stdin");
        let _ = rt.close_session(&info.session_id).await;
        let _ = rt.close_session(&shell.session_id).await;
    }

    #[tokio::test]
    async fn human_interactive_write_is_not_ai_command_recorded() {
        // A human keystroke (as_ai=false) must NOT be recorded as an AI command.
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder.clone());
        let mut cfg = PtySpawnConfig::default();
        if cfg!(windows) {
            cfg.shell = Some("cmd.exe".to_string());
        } else {
            cfg.shell = Some("/bin/sh".to_string());
        }
        let info = rt
            .create_session(
                TerminalSessionType::HumanDev,
                SessionBinding::default(),
                vec![],
                cfg,
                None,
            )
            .await
            .unwrap();
        let _ = rt
            .write_stdin_recorded(&info.session_id, b"echo HUMAN\n", false)
            .await;
        let ai_cmds = recorder
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| {
                e.payload
                    .get("origin")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "ai_interactive_stdin")
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(ai_cmds, 0, "human keystrokes must not be AI-command-recorded");
        let _ = rt.close_session(&info.session_id).await;
    }

    #[tokio::test]
    async fn dropped_capture_sink_reaps_session() {
        // Leak guard: a CaptureSink dropped WITHOUT close() must not leave a ghost
        // session in the registry (orphaned "inspect all background work" tab).
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder);
        let session_id = {
            let (info, sink) = rt
                .create_capture_session(SessionBinding::default(), Some("leaky".into()))
                .await;
            assert_eq!(rt.list_sessions().len(), 1, "session present while sink live");
            sink.feed(b"partial output before drop\n").await;
            // Drop the sink WITHOUT calling close() (simulates task cancellation /
            // producer panic / future dropped mid-stream).
            drop(sink);
            info.session_id
        };
        // The Drop leak guard must have reaped the session synchronously.
        assert_eq!(
            rt.list_sessions().len(),
            0,
            "dropped-without-close sink must reap its session"
        );
        assert!(
            rt.subscribe(&session_id).is_err(),
            "reaped session must be unknown"
        );
    }

    #[tokio::test]
    async fn closed_capture_sink_does_not_double_reap() {
        // A normally-closed sink removes its session via close(); the Drop guard
        // must then be a no-op (no panic, no double-remove side effects).
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder);
        let (info, sink) = rt
            .create_capture_session(SessionBinding::default(), None)
            .await;
        sink.feed(b"done\n").await;
        sink.close(0).await; // consumes + sets closed flag, then Drop runs
        assert_eq!(rt.list_sessions().len(), 0);
        assert!(rt.subscribe(&info.session_id).is_err());
    }

    #[tokio::test]
    async fn session_lifecycle_records_open_and_close() {
        let recorder = Arc::new(CountingRecorder::default());
        let rt = TerminalRuntime::new(registry_with(&[]), recorder.clone());
        let info = rt
            .create_session(
                TerminalSessionType::HumanDev,
                SessionBinding::default(),
                vec![],
                echo_spawn(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(rt.list_sessions().len(), 1);
        rt.close_session(&info.session_id).await.unwrap();
        assert_eq!(rt.list_sessions().len(), 0);
        let fr = recorder.fr_events();
        assert!(fr.contains(&"FR-EVT-TERMINAL-SESSION-OPEN".to_string()));
        assert!(fr.contains(&"FR-EVT-TERMINAL-SESSION-CLOSE".to_string()));
    }
}
