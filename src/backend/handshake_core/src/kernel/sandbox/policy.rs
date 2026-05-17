//! Sandbox policy with default-deny stance.
//!
//! Policies are versioned (`policy_version_id` = `POL-<uuid>@<n>`) so denial
//! evidence, replays, and DCC projections can show *which* policy version
//! rejected a capability. The default constructor returns a policy that
//! denies every sensitive capability (filesystem escape, network, process
//! spawn, device access, environment leak, secret read).
//!
//! This module defines the policy data model only. Enforcement happens in
//! adapters under `policy_scoped_local`, `process_tier`, and the
//! `HardIsolation` extension slot.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Named capability a sandbox might request. Defined as a closed enum so
/// policy review is bounded; new capabilities require a code change and a
/// migration of `SandboxPolicyV1` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxCapability {
    FilesystemEscape,
    Network,
    ProcessSpawn,
    Device,
    EnvironmentLeak,
    SecretRead,
}

impl SandboxCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FilesystemEscape => "FILESYSTEM_ESCAPE",
            Self::Network => "NETWORK",
            Self::ProcessSpawn => "PROCESS_SPAWN",
            Self::Device => "DEVICE",
            Self::EnvironmentLeak => "ENVIRONMENT_LEAK",
            Self::SecretRead => "SECRET_READ",
        }
    }

    pub const ALL: &'static [SandboxCapability] = &[
        Self::FilesystemEscape,
        Self::Network,
        Self::ProcessSpawn,
        Self::Device,
        Self::EnvironmentLeak,
        Self::SecretRead,
    ];
}

/// Per-capability decision recorded in the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityDecision {
    Deny,
    Allow,
    AllowWithEvidence,
}

/// Versioned sandbox policy record (schema id `hsk.kernel.sandbox_policy@1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicyV1 {
    pub policy_id: String,
    pub policy_version: u32,
    pub name: String,
    pub created_at_utc: DateTime<Utc>,
    pub default_decision: CapabilityDecision,
    pub overrides: Vec<(SandboxCapability, CapabilityDecision)>,
    pub allowed_workspace_roots: Vec<String>,
    pub provenance_note: String,
}

impl SandboxPolicyV1 {
    /// Default-deny policy: every sensitive capability returns `Deny` unless
    /// explicitly overridden. Used by MT-019 PolicyScopedLocal and by tests
    /// asserting MT-018 adapter-trait neutrality.
    pub fn default_deny(name: impl Into<String>) -> Self {
        Self {
            policy_id: format!("POL-{}", Uuid::new_v4()),
            policy_version: 1,
            name: name.into(),
            created_at_utc: Utc::now(),
            default_decision: CapabilityDecision::Deny,
            overrides: Vec::new(),
            allowed_workspace_roots: Vec::new(),
            provenance_note: "default_deny constructed; no capability overrides".to_string(),
        }
    }

    /// Stable, version-qualified id used by replays and projections.
    pub fn version_id(&self) -> String {
        format!("{}@{}", self.policy_id, self.policy_version)
    }

    /// Resolve the decision for `cap` against this policy version.
    pub fn decide(&self, cap: SandboxCapability) -> CapabilityDecision {
        for (k, v) in &self.overrides {
            if *k == cap {
                return *v;
            }
        }
        self.default_decision
    }

    /// Bump the version, returning a new policy record with the same id but
    /// `policy_version + 1`. Used by MT-012 to keep policy changes traceable.
    pub fn bump_version(&self, note: impl Into<String>) -> Self {
        Self {
            policy_id: self.policy_id.clone(),
            policy_version: self.policy_version + 1,
            name: self.name.clone(),
            created_at_utc: Utc::now(),
            default_decision: self.default_decision,
            overrides: self.overrides.clone(),
            allowed_workspace_roots: self.allowed_workspace_roots.clone(),
            provenance_note: note.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny_rejects_every_sensitive_capability() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        for cap in SandboxCapability::ALL {
            assert_eq!(
                pol.decide(*cap),
                CapabilityDecision::Deny,
                "{} must be denied by default policy",
                cap.as_str()
            );
        }
    }

    #[test]
    fn version_id_is_stable_under_overrides() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        let vid1 = pol.version_id();
        assert!(vid1.contains('@'));
        assert!(vid1.starts_with("POL-"));
        let bumped = pol.bump_version("added network for diag");
        assert_ne!(vid1, bumped.version_id());
        assert_eq!(bumped.policy_id, pol.policy_id, "policy_id is stable across versions");
        assert_eq!(bumped.policy_version, 2);
    }

    #[test]
    fn override_takes_precedence_over_default() {
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        pol.overrides.push((SandboxCapability::Network, CapabilityDecision::AllowWithEvidence));
        assert_eq!(
            pol.decide(SandboxCapability::Network),
            CapabilityDecision::AllowWithEvidence
        );
        assert_eq!(
            pol.decide(SandboxCapability::ProcessSpawn),
            CapabilityDecision::Deny,
            "unrelated capabilities stay denied"
        );
    }
}
