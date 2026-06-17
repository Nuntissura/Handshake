//! WP-KERNEL-009 MT-254 DebugAdapterCore.
//!
//! A Handshake-native Debug Adapter Protocol (DAP) client core: launch a real
//! debuggee, set breakpoints, read the paused call stack / scopes / variables,
//! step / continue / pause, and evaluate console expressions in the paused
//! frame. The FIRST and only shipped adapter is Node.js, driven over its
//! built-in V8 Inspector protocol (no external adapter binary) in
//! [`node_inspector`].
//!
//! Honesty gate: [`registry::listable_adapters`] returns ONLY adapters that
//! drive a real process today (Node). The registry is the single source the
//! UI/API read so there are never dead "python"/"lldb" dropdown entries.
//!
//! NAMING: `debug_adapter`/`dap`, never `inspector`, to avoid colliding with the
//! kernel `inspector.rs` / `InspectorReadV1` replay surface.

use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

pub mod node_inspector;
pub mod protocol;
pub mod registry;

pub use protocol::{
    AdapterKind, Breakpoint, DebugEvent, DebugSessionId, Scope, SourceBreakpoint, StackFrame,
    StepKind, StoppedReason, Variable,
};

/// What to launch.
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub adapter: AdapterKind,
    /// Absolute path to the script to debug.
    pub program: String,
    /// Optional working directory for the debuggee.
    pub cwd: Option<String>,
    /// Optional explicit runtime binary (e.g. a pinned `node` path); defaults to
    /// `node` on PATH.
    pub runtime_path: Option<String>,
    pub env: HashMap<String, String>,
}

impl LaunchRequest {
    pub fn node(program: impl Into<String>) -> Self {
        Self {
            adapter: AdapterKind::Node,
            program: program.into(),
            cwd: None,
            runtime_path: None,
            env: HashMap::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum DebugAdapterError {
    #[error("launch failed: {0}")]
    Launch(String),
    #[error("transport failed: {0}")]
    Transport(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("timed out: {0}")]
    Timeout(String),
    #[error("session is not paused")]
    NotPaused,
    #[error("unsupported adapter: {0}")]
    Unsupported(String),
}

/// The adapter-agnostic debug session contract the API/IPC drive.
#[async_trait]
pub trait DebugAdapter: Send + Sync {
    fn session_id(&self) -> &DebugSessionId;

    /// Set the breakpoints for a source; returns the adapter's `verified`
    /// verdict per breakpoint (REAL binding, never faked).
    async fn set_breakpoints(
        &self,
        source: &str,
        breakpoints: &[SourceBreakpoint],
    ) -> Result<Vec<Breakpoint>, DebugAdapterError>;

    /// The call stack at the current pause (errors if not paused).
    async fn stack_trace(&self) -> Result<Vec<StackFrame>, DebugAdapterError>;

    /// The variable scopes of a paused frame.
    async fn scopes(&self, frame_id: &str) -> Result<Vec<Scope>, DebugAdapterError>;

    /// The variables behind a scope/object handle (real runtime values).
    async fn variables(
        &self,
        variables_reference: &str,
    ) -> Result<Vec<Variable>, DebugAdapterError>;

    /// Evaluate an expression in the context of a paused frame (debug console).
    async fn evaluate(
        &self,
        frame_id: &str,
        expression: &str,
    ) -> Result<String, DebugAdapterError>;

    /// Step; resolves once the debuggee is paused again.
    async fn step(&self, kind: StepKind) -> Result<(), DebugAdapterError>;

    /// Resume execution.
    async fn continue_(&self) -> Result<(), DebugAdapterError>;

    /// Pause a running debuggee; resolves once paused.
    async fn pause(&self) -> Result<(), DebugAdapterError>;

    /// Terminate the session; returns the real process exit code if known.
    async fn terminate(&self) -> Result<Option<i32>, DebugAdapterError>;
}

/// Launch a session for the requested adapter. Today only Node is supported; any
/// other (future) kind returns [`DebugAdapterError::Unsupported`] rather than a
/// stub session — the registry already hides non-runnable kinds from the UI.
pub async fn launch(req: LaunchRequest) -> Result<node_inspector::NodeInspectorSession, DebugAdapterError> {
    match req.adapter {
        AdapterKind::Node => node_inspector::launch_node_session(&req).await,
    }
}
