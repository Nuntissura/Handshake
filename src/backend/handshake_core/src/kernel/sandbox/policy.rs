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

/// Typed reference to the evidence artifact backing a capability grant. Wraps
/// a string id (typically `ART-...`, `evidence://...`, or similar) so the
/// type system makes "this is an evidence handle" explicit at the call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidenceRef(pub String);

impl CapabilityEvidenceRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn is_present(&self) -> bool {
        !self.0.trim().is_empty()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed reference to an operator approval row backing a capability grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorApprovalRef(pub String);

impl OperatorApprovalRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn is_present(&self) -> bool {
        !self.0.trim().is_empty()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// H5: a typed grant authorising a single sensitive capability. Every grant
/// MUST carry a non-empty `evidence_ref`; an empty `evidence_ref` is a
/// policy-build error (`PolicyBuildError::CapabilityGrantMissingEvidence`) and
/// is also refused by the default `SandboxAdapter::pre_check` impl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub capability: SandboxCapability,
    pub evidence_ref: CapabilityEvidenceRef,
    pub approval_ref: Option<OperatorApprovalRef>,
}

/// Per-capability decision recorded in the policy.
///
/// H5 fix: `AllowWithEvidence` was removed because callers could construct
/// it without any evidence at all. The replacement, `Allow(CapabilityGrant)`,
/// makes the evidence handle a required field on the grant. Plain `Allow`
/// without a grant is intentionally gone; either deny, or allow with typed
/// evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityDecision {
    Deny,
    Allow(CapabilityGrant),
}

/// H5: errors produced while building / validating a `SandboxPolicyV1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyBuildError {
    /// A grant was registered with an empty `evidence_ref`.
    CapabilityGrantMissingEvidence { capability: SandboxCapability },
}

impl std::fmt::Display for PolicyBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapabilityGrantMissingEvidence { capability } => write!(
                f,
                "capability grant for {} is missing a non-empty evidence_ref",
                capability.as_str()
            ),
        }
    }
}

impl std::error::Error for PolicyBuildError {}

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
    /// M-A1 fix: stable schema id for this record. Use this instead of inlining
    /// the literal string in callers.
    pub const fn schema_version() -> &'static str {
        crate::kernel::kb003_schemas::SCHEMA_KERNEL_SANDBOX_POLICY_V1
    }

    /// Default-deny policy: every sensitive capability returns `Deny` unless
    /// explicitly overridden. Used by MT-019 PolicyScopedLocal and by tests
    /// asserting MT-018 adapter-trait neutrality.
    pub fn default_deny(name: impl Into<String>) -> Self {
        Self {
            policy_id: format!("POL-{}", Uuid::now_v7()),
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
                return v.clone();
            }
        }
        self.default_decision.clone()
    }

    /// Bump the version, returning a new policy record with the same id but
    /// `policy_version + 1`. Used by MT-012 to keep policy changes traceable.
    pub fn bump_version(&self, note: impl Into<String>) -> Self {
        Self {
            policy_id: self.policy_id.clone(),
            policy_version: self.policy_version + 1,
            name: self.name.clone(),
            created_at_utc: Utc::now(),
            default_decision: self.default_decision.clone(),
            overrides: self.overrides.clone(),
            allowed_workspace_roots: self.allowed_workspace_roots.clone(),
            provenance_note: note.into(),
        }
    }

    /// H5: validate that every `Allow(grant)` override carries a non-empty
    /// `evidence_ref`. Called by `SandboxPolicyBundleV1` constructors that
    /// register grants. Returns the first violation found.
    pub fn validate_grants(&self) -> Result<(), PolicyBuildError> {
        if let CapabilityDecision::Allow(g) = &self.default_decision {
            if !g.evidence_ref.is_present() {
                return Err(PolicyBuildError::CapabilityGrantMissingEvidence {
                    capability: g.capability,
                });
            }
        }
        for (_, dec) in &self.overrides {
            if let CapabilityDecision::Allow(g) = dec {
                if !g.evidence_ref.is_present() {
                    return Err(PolicyBuildError::CapabilityGrantMissingEvidence {
                        capability: g.capability,
                    });
                }
            }
        }
        Ok(())
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
        assert_eq!(
            bumped.policy_id, pol.policy_id,
            "policy_id is stable across versions"
        );
        assert_eq!(bumped.policy_version, 2);
    }

    #[test]
    fn override_takes_precedence_over_default() {
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        let grant = CapabilityGrant {
            capability: SandboxCapability::Network,
            evidence_ref: CapabilityEvidenceRef::new("ART-evidence-1"),
            approval_ref: Some(OperatorApprovalRef::new("APR-1")),
        };
        pol.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(grant.clone()),
        ));
        match pol.decide(SandboxCapability::Network) {
            CapabilityDecision::Allow(g) => assert_eq!(g, grant),
            other => panic!("expected Allow(grant), got {:?}", other),
        }
        assert_eq!(
            pol.decide(SandboxCapability::ProcessSpawn),
            CapabilityDecision::Deny,
            "unrelated capabilities stay denied"
        );
    }

    #[test]
    fn validate_grants_rejects_empty_evidence_ref() {
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        let bad_grant = CapabilityGrant {
            capability: SandboxCapability::Network,
            evidence_ref: CapabilityEvidenceRef::new(""),
            approval_ref: None,
        };
        pol.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(bad_grant),
        ));
        match pol.validate_grants() {
            Err(PolicyBuildError::CapabilityGrantMissingEvidence { capability }) => {
                assert_eq!(capability, SandboxCapability::Network);
            }
            Ok(()) => panic!("empty evidence_ref must be rejected"),
        }
    }
}
