//! Sandbox run lifecycle types (foundation for MT-010 onward).
//!
//! `SandboxRun` is the durable record of a single sandbox proof attempt.
//! Lifecycle states follow the KB003 event stream:
//! REQUESTED -> STARTED -> COMPLETED | REJECTED.
//!
//! Identity follows the kernel convention: `SBX-<uuid>` for run ids so that
//! receipts, projections, and replays can route on a stable prefix without
//! parsing a UUID. The run id is the *aggregate* id; the kernel's
//! `kernel_task_run_id` and `session_run_id` remain the higher-level identity
//! that ties a sandbox run to the originating coder session.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lifecycle status for a sandbox run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxRunStatus {
    Requested,
    Started,
    Completed,
    Rejected,
}

impl SandboxRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requested => "REQUESTED",
            Self::Started => "STARTED",
            Self::Completed => "COMPLETED",
            Self::Rejected => "REJECTED",
        }
    }

    /// Terminal statuses do not transition further; replay rebuilds them as-is.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Rejected)
    }

    /// Valid forward transitions; used by both runner and replay validation.
    pub fn can_transition_to(&self, next: SandboxRunStatus) -> bool {
        use SandboxRunStatus::*;
        matches!(
            (self, next),
            (Requested, Started)
                | (Requested, Rejected)
                | (Started, Completed)
                | (Started, Rejected)
        )
    }
}

/// Stable, prefixed sandbox run identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SandboxRunId(pub String);

impl SandboxRunId {
    pub fn new() -> Self {
        Self(format!("SBX-{}", Uuid::now_v7()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SandboxRunId {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable record describing a sandbox run (schema id
/// `hsk.kernel.sandbox_run@1`). Mirrors the row shape used by the storage
/// migration in MT-011 and the operator projection in MT-010.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxRunV1 {
    pub run_id: SandboxRunId,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub adapter_kind: String,
    pub policy_version_id: String,
    pub workspace_id: String,
    pub status: SandboxRunStatus,
    pub requested_at_utc: DateTime<Utc>,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub finished_at_utc: Option<DateTime<Utc>>,
    pub denial_id: Option<String>,
    pub artifact_refs: Vec<String>,
}

impl SandboxRunV1 {
    /// M-A1 fix: stable schema id for this record. Use this instead of inlining
    /// the literal string in callers (e.g. event envelopes, projection
    /// metadata, replay-bag validators).
    pub const fn schema_version() -> &'static str {
        crate::kernel::kb003_schemas::SCHEMA_KERNEL_SANDBOX_RUN_V1
    }

    pub fn new_requested(
        kernel_task_run_id: impl Into<String>,
        session_run_id: impl Into<String>,
        adapter_kind: impl Into<String>,
        policy_version_id: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Self {
        Self {
            run_id: SandboxRunId::new(),
            kernel_task_run_id: kernel_task_run_id.into(),
            session_run_id: session_run_id.into(),
            adapter_kind: adapter_kind.into(),
            policy_version_id: policy_version_id.into(),
            workspace_id: workspace_id.into(),
            status: SandboxRunStatus::Requested,
            requested_at_utc: Utc::now(),
            started_at_utc: None,
            finished_at_utc: None,
            denial_id: None,
            artifact_refs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_is_prefixed_and_unique() {
        let a = SandboxRunId::new();
        let b = SandboxRunId::new();
        assert!(a.as_str().starts_with("SBX-"), "id must use SBX- prefix for routing");
        assert_ne!(a, b, "ids must be unique");
    }

    #[test]
    fn valid_transitions_only() {
        use SandboxRunStatus::*;
        assert!(Requested.can_transition_to(Started));
        assert!(Requested.can_transition_to(Rejected));
        assert!(Started.can_transition_to(Completed));
        assert!(Started.can_transition_to(Rejected));
        // Invalid:
        assert!(!Completed.can_transition_to(Started));
        assert!(!Rejected.can_transition_to(Completed));
        assert!(!Started.can_transition_to(Requested));
    }

    #[test]
    fn terminal_states_are_flagged() {
        assert!(SandboxRunStatus::Completed.is_terminal());
        assert!(SandboxRunStatus::Rejected.is_terminal());
        assert!(!SandboxRunStatus::Started.is_terminal());
        assert!(!SandboxRunStatus::Requested.is_terminal());
    }

    #[test]
    fn new_requested_initialises_fields() {
        let run = SandboxRunV1::new_requested("KTR-1", "SES-1", "process_tier", "POL-1@1", "WS-1");
        assert_eq!(run.status, SandboxRunStatus::Requested);
        assert!(run.started_at_utc.is_none());
        assert!(run.finished_at_utc.is_none());
        assert!(run.artifact_refs.is_empty());
        assert!(run.run_id.as_str().starts_with("SBX-"));
    }
}
