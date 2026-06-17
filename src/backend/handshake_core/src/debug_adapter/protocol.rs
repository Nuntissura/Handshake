//! WP-KERNEL-009 MT-254 DebugAdapterCore — protocol types.
//!
//! These are the Handshake-native, adapter-agnostic Debug Adapter Protocol (DAP)
//! shapes the product UI and IPC speak. They are deliberately a small, honest
//! subset: only what a real debug session needs (sessions, breakpoints,
//! stack/scopes/variables, stepping, console eval, lifecycle events). The Node
//! adapter (`node_inspector`) maps these onto the V8 Inspector / Chrome DevTools
//! Protocol (CDP). No mock shapes: every field is populated from real adapter
//! state.
//!
//! NAMING: this module is `debug_adapter`/`dap`, NOT `inspector`, to avoid the
//! collision with the kernel `inspector.rs` / `InspectorReadV1` replay surface.

use serde::{Deserialize, Serialize};

/// Opaque id for one live debug session (one launched debuggee).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugSessionId(pub String);

impl DebugSessionId {
    pub fn new() -> Self {
        DebugSessionId(format!("dap-{}", uuid::Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for DebugSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DebugSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The concrete adapter kinds Handshake ships. ONLY kinds that drive a real
/// process appear here AND in [`crate::debug_adapter::registry::listable_adapters`].
/// Future kinds (Python `debugpy`, `lldb`) are intentionally absent until they
/// run — the registry is the honesty gate (no dead UI entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    /// Node.js via its built-in V8 Inspector (`--inspect-brk`), no external
    /// adapter binary.
    Node,
}

impl AdapterKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdapterKind::Node => "node",
        }
    }

    /// Human-facing label for the adapter picker.
    pub fn display_name(&self) -> &'static str {
        match self {
            AdapterKind::Node => "Node.js (built-in inspector)",
        }
    }
}

/// A breakpoint request against a source: a 1-based line, optional column, and an
/// optional condition expression evaluated by the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBreakpoint {
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// The adapter's verdict on a requested breakpoint. `verified` is REAL: it is
/// `true` only when the adapter actually bound the breakpoint to executable code
/// (for Node: `Debugger.setBreakpointByUrl` returned at least one `location`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Adapter-assigned breakpoint id (CDP `breakpointId` for Node).
    pub id: String,
    pub verified: bool,
    /// The resolved 1-based line the adapter actually bound to (may differ from
    /// the requested line if the requested line was not executable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// One frame of a paused call stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackFrame {
    /// Adapter frame id; used to scope `scopes`/`variables`/`evaluate`.
    pub id: String,
    pub name: String,
    /// Source url/path the frame executes in (CDP script url for Node).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 1-based line of the frame's current statement.
    pub line: u32,
    pub column: u32,
}

/// A variable scope within a paused frame (`local`, `global`, `closure`, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    /// Handle that resolves to this scope's variables via `variables`.
    pub variables_reference: String,
    pub expensive: bool,
}

/// One variable. `variables_reference` is non-empty when the value is structured
/// (object/array) and can be expanded with a further `variables` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Empty string when the value is a leaf (not expandable).
    pub variables_reference: String,
}

/// Why the debuggee stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppedReason {
    Breakpoint,
    Step,
    Pause,
    Entry,
    Exception,
    Other,
}

impl StoppedReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoppedReason::Breakpoint => "breakpoint",
            StoppedReason::Step => "step",
            StoppedReason::Pause => "pause",
            StoppedReason::Entry => "entry",
            StoppedReason::Exception => "exception",
            StoppedReason::Other => "other",
        }
    }
}

/// Stepping granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Over,
    Into,
    Out,
}

/// Lifecycle/output events streamed from a session, mirroring the
/// `terminal://output|exit` forwarder convention. Serialized to
/// `dap://stopped|output|continued|terminated` by the IPC layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugEvent {
    /// The debuggee paused. Carries the reason and the top frame line so the
    /// editor can place the current-stop decoration without a round-trip.
    Stopped {
        reason: StoppedReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_frame_line: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_frame_source: Option<String>,
    },
    /// stdout/stderr/console output from the debuggee.
    Output { category: String, output: String },
    /// The debuggee resumed.
    Continued,
    /// The session ended; `exit_code` is the real process exit status.
    Terminated {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_kind_roundtrips_as_snake_case() {
        let json = serde_json::to_string(&AdapterKind::Node).unwrap();
        assert_eq!(json, "\"node\"");
        assert_eq!(AdapterKind::Node.as_str(), "node");
    }

    #[test]
    fn stopped_event_serializes_with_kind_tag() {
        let evt = DebugEvent::Stopped {
            reason: StoppedReason::Breakpoint,
            top_frame_line: Some(7),
            top_frame_source: Some("file:///x.js".into()),
        };
        let v = serde_json::to_value(&evt).unwrap();
        assert_eq!(v["kind"], "stopped");
        assert_eq!(v["reason"], "breakpoint");
        assert_eq!(v["top_frame_line"], 7);
    }

    #[test]
    fn session_id_has_prefix() {
        assert!(DebugSessionId::new().as_str().starts_with("dap-"));
    }
}
