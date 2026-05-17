//! Typed denial evidence emitted whenever a sandbox capability check fails.
//!
//! Every denial record carries the policy version that denied it, the
//! capability name, the requested action description, and a short reason. The
//! denial id (`DEN-<uuid>`) is also stored on the originating `SandboxRunV1`
//! so the DCC projection (MT-010) can surface the denial without scraping
//! terminal logs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::policy::SandboxCapability;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DenialKind {
    PolicyDenied,
    WorkspaceBoundaryViolation,
    AdapterUnavailable,
    AuthorityModeRefused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxDenialRecordV1 {
    pub denial_id: String,
    pub run_id: String,
    pub policy_version_id: String,
    pub capability: Option<SandboxCapability>,
    pub kind: DenialKind,
    pub action_description: String,
    pub reason: String,
    pub recorded_at_utc: DateTime<Utc>,
}

impl SandboxDenialRecordV1 {
    pub fn new(
        run_id: impl Into<String>,
        policy_version_id: impl Into<String>,
        kind: DenialKind,
        capability: Option<SandboxCapability>,
        action_description: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            denial_id: format!("DEN-{}", Uuid::new_v4()),
            run_id: run_id.into(),
            policy_version_id: policy_version_id.into(),
            capability,
            kind,
            action_description: action_description.into(),
            reason: reason.into(),
            recorded_at_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denial_record_has_required_fields() {
        let den = SandboxDenialRecordV1::new(
            "SBX-x",
            "POL-y@2",
            DenialKind::PolicyDenied,
            Some(SandboxCapability::Network),
            "fetch https://example.com",
            "policy default_deny on NETWORK",
        );
        assert!(den.denial_id.starts_with("DEN-"));
        assert_eq!(den.run_id, "SBX-x");
        assert_eq!(den.policy_version_id, "POL-y@2");
        assert_eq!(den.kind, DenialKind::PolicyDenied);
        assert_eq!(den.capability, Some(SandboxCapability::Network));
        assert!(!den.action_description.is_empty());
        assert!(!den.reason.is_empty());
    }

    #[test]
    fn workspace_boundary_violation_has_no_capability() {
        let den = SandboxDenialRecordV1::new(
            "SBX-1",
            "POL-1@1",
            DenialKind::WorkspaceBoundaryViolation,
            None,
            "write /etc/passwd",
            "path escapes workspace root",
        );
        assert!(den.capability.is_none());
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
    }
}
