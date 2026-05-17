//! MT-072 Capability Audit Evidence Link.
//!
//! Acceptance (MT-072.json): "every sensitive grant has provenance."
//!
//! Renders every sandbox capability grant or denial as a typed audit row
//! that carries:
//!
//! - the capability,
//! - the decision (DENY / ALLOW / ALLOW_WITH_EVIDENCE),
//! - the policy version that decided it,
//! - the **provenance evidence** (artifact handle, operator id, decision id)
//!   that justifies a non-default grant.
//!
//! For `CapabilityDecision::Allow(grant)` the audit row is required to carry
//! at least one provenance entry AND the grant must carry a non-empty
//! `evidence_ref`; the [`CapabilityAuditRowV1::is_well_formed`] check enforces
//! this. For DENY rows, the provenance points to the denial id.
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::sandbox::policy::{CapabilityDecision, SandboxCapability};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityProvenanceV1 {
    /// Reference to a stored evidence artifact (manifest, screenshot, log).
    ArtifactRef { artifact_ref: String },
    /// Operator approval evidence (typically captured at policy-edit time).
    OperatorApproval { operator_id: String, approval_id: String },
    /// Denial id when the capability was denied.
    DenialRef { denial_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAuditRowV1 {
    pub capability: SandboxCapability,
    pub decision: CapabilityDecision,
    pub policy_version_id: String,
    pub provenance: Vec<CapabilityProvenanceV1>,
    pub recorded_at_utc: DateTime<Utc>,
}

impl CapabilityAuditRowV1 {
    pub fn new(
        capability: SandboxCapability,
        decision: CapabilityDecision,
        policy_version_id: impl Into<String>,
        provenance: Vec<CapabilityProvenanceV1>,
    ) -> Self {
        Self {
            capability,
            decision,
            policy_version_id: policy_version_id.into(),
            provenance,
            recorded_at_utc: Utc::now(),
        }
    }

    /// Acceptance: every sensitive grant has provenance. A grant is
    /// sensitive when it is `Allow(grant)` (which must carry at least one
    /// provenance row AND the grant must carry a non-empty `evidence_ref`)
    /// or `Deny` (which must carry a `DenialRef`).
    pub fn is_well_formed(&self) -> bool {
        match &self.decision {
            CapabilityDecision::Allow(grant) => {
                grant.evidence_ref.is_present()
                    && self.provenance.iter().any(|p| {
                        matches!(
                            p,
                            CapabilityProvenanceV1::ArtifactRef { .. }
                                | CapabilityProvenanceV1::OperatorApproval { .. }
                        )
                    })
            }
            CapabilityDecision::Deny => self
                .provenance
                .iter()
                .any(|p| matches!(p, CapabilityProvenanceV1::DenialRef { .. })),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003CapabilityAuditV1 {
    pub projection_family_id: String,
    pub sandbox_run_id: String,
    pub rows: Vec<CapabilityAuditRowV1>,
}

impl DccKb003CapabilityAuditV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.capability_audit@1";

    pub fn new(sandbox_run_id: impl Into<String>, rows: Vec<CapabilityAuditRowV1>) -> Self {
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            sandbox_run_id: sandbox_run_id.into(),
            rows,
        }
    }

    /// Audit-level acceptance: every row is well-formed.
    pub fn every_row_well_formed(&self) -> bool {
        self.rows.iter().all(|r| r.is_well_formed())
    }

    pub fn rows_for(&self, capability: SandboxCapability) -> impl Iterator<Item = &CapabilityAuditRowV1> {
        self.rows.iter().filter(move |r| r.capability == capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::policy::{
        CapabilityEvidenceRef, CapabilityGrant, OperatorApprovalRef,
    };

    fn grant(cap: SandboxCapability, evidence: &str) -> CapabilityGrant {
        CapabilityGrant {
            capability: cap,
            evidence_ref: CapabilityEvidenceRef::new(evidence),
            approval_ref: Some(OperatorApprovalRef::new("APR-test")),
        }
    }

    #[test]
    fn allow_with_evidence_requires_provenance() {
        let bad = CapabilityAuditRowV1::new(
            SandboxCapability::Network,
            CapabilityDecision::Allow(grant(SandboxCapability::Network, "ART-1")),
            "POL-1@1",
            vec![],
        );
        assert!(!bad.is_well_formed());
        let good = CapabilityAuditRowV1::new(
            SandboxCapability::Network,
            CapabilityDecision::Allow(grant(SandboxCapability::Network, "ART-1")),
            "POL-1@1",
            vec![CapabilityProvenanceV1::OperatorApproval {
                operator_id: "ilja".into(),
                approval_id: "APR-1".into(),
            }],
        );
        assert!(good.is_well_formed());
    }

    #[test]
    fn deny_requires_denial_ref_provenance() {
        let bad = CapabilityAuditRowV1::new(
            SandboxCapability::SecretRead,
            CapabilityDecision::Deny,
            "POL-1@1",
            vec![CapabilityProvenanceV1::ArtifactRef { artifact_ref: "ART-1".into() }],
        );
        assert!(!bad.is_well_formed());
        let good = CapabilityAuditRowV1::new(
            SandboxCapability::SecretRead,
            CapabilityDecision::Deny,
            "POL-1@1",
            vec![CapabilityProvenanceV1::DenialRef { denial_id: "DEN-1".into() }],
        );
        assert!(good.is_well_formed());
    }

    #[test]
    fn allow_with_empty_evidence_is_not_well_formed() {
        // H5: an Allow(grant) row with an EMPTY evidence_ref must be rejected
        // even if a provenance entry is present, because the grant itself
        // lacks the audit handle.
        let r = CapabilityAuditRowV1::new(
            SandboxCapability::ProcessSpawn,
            CapabilityDecision::Allow(grant(SandboxCapability::ProcessSpawn, "")),
            "POL-1@1",
            vec![CapabilityProvenanceV1::ArtifactRef { artifact_ref: "ART-1".into() }],
        );
        assert!(!r.is_well_formed());
    }

    #[test]
    fn audit_rolls_up_row_well_formedness() {
        let audit = DccKb003CapabilityAuditV1::new(
            "SBX-1",
            vec![
                CapabilityAuditRowV1::new(
                    SandboxCapability::Network,
                    CapabilityDecision::Allow(grant(SandboxCapability::Network, "ART-net-grant")),
                    "POL-1@1",
                    vec![CapabilityProvenanceV1::ArtifactRef { artifact_ref: "ART-net-eval".into() }],
                ),
                CapabilityAuditRowV1::new(
                    SandboxCapability::SecretRead,
                    CapabilityDecision::Deny,
                    "POL-1@1",
                    vec![CapabilityProvenanceV1::DenialRef { denial_id: "DEN-2".into() }],
                ),
            ],
        );
        assert!(audit.every_row_well_formed());
        assert_eq!(audit.rows_for(SandboxCapability::Network).count(), 1);
    }
}
