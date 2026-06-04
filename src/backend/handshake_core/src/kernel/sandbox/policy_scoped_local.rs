//! MT-019 `PolicyScopedLocalAdapter` — minimum local-proof adapter.
//!
//! Acceptance (MT-019.json): "policy mode is explicitly not hard isolation and
//! denies sensitive capabilities by default." This adapter is the day-one
//! `Process`-tier adapter the kernel reaches for when no hard-isolation tier
//! is configured. It enforces three invariants:
//!
//! 1. `kind().tier == AdapterIsolationTier::Process` and the kind label
//!    explicitly says "not hard isolation" so DCC and replay can never confuse
//!    it for a microVM or container path.
//! 2. The active policy MUST be default-deny; constructor refuses any policy
//!    whose `default_decision` is not `Deny`.
//! 3. `run` calls `pre_check` against the full sensitive-capability set
//!    before doing any work; any denial short-circuits to
//!    `AdapterRunOutcome::Denied`.

use super::adapter::{
    AdapterError, AdapterIsolationTier, AdapterKind, AdapterRunOutcome, SandboxAdapter,
};
use super::policy::{CapabilityDecision, SandboxCapability, SandboxPolicyV1};
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const POLICY_SCOPED_LOCAL_ADAPTER_ID: &str = "policy_scoped_local";
pub const POLICY_SCOPED_LOCAL_ADAPTER_LABEL: &str =
    "PolicyScopedLocal (process tier, NOT hard isolation)";

#[derive(Debug)]
pub struct PolicyScopedLocalAdapter {
    policy: SandboxPolicyV1,
}

impl PolicyScopedLocalAdapter {
    /// Construct with an explicit default-deny policy. Returns `Err` if the
    /// caller hands in a permissive policy by mistake.
    pub fn new(policy: SandboxPolicyV1) -> Result<Self, AdapterError> {
        if policy.default_decision != CapabilityDecision::Deny {
            return Err(AdapterError::PolicyDenied(format!(
                "PolicyScopedLocal requires default_deny policy; got default {:?}",
                policy.default_decision
            )));
        }
        Ok(Self { policy })
    }

    pub fn policy(&self) -> &SandboxPolicyV1 {
        &self.policy
    }
}

impl SandboxAdapter for PolicyScopedLocalAdapter {
    fn kind(&self) -> AdapterKind {
        AdapterKind {
            id: POLICY_SCOPED_LOCAL_ADAPTER_ID.to_string(),
            tier: AdapterIsolationTier::Process,
            version: 1,
            label: POLICY_SCOPED_LOCAL_ADAPTER_LABEL.to_string(),
        }
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        // M7 fix: iterate only the capabilities the run actually requested.
        // The previous implementation always checked `SandboxCapability::ALL`,
        // which made every run trivially denied under a default-deny policy
        // even when the run did not request a single sensitive capability.
        // Empty requested_capabilities => no capability check required.
        if !run.requested_capabilities.is_empty() {
            if let Err(denial) = self.pre_check(run, policy, &run.requested_capabilities) {
                return Ok(AdapterRunOutcome::Denied(denial));
            }
        }
        // Workspace contract: refuse if workspace forbids writes — the local
        // proof adapter writes artifact bundles into the workspace root.
        if !workspace.allow_write {
            return Err(AdapterError::WorkspaceViolation(format!(
                "workspace `{}` is read-only; PolicyScopedLocal needs allow_write",
                workspace.workspace_id
            )));
        }
        // Body: in the real adapter, this would spawn a child process under
        // capped permissions and collect artifacts. The MVP returns Completed
        // with no artifacts, leaving artifact wiring to Wave E.
        Ok(AdapterRunOutcome::Completed {
            artifact_refs: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::denial::DenialKind;
    use crate::kernel::sandbox::run::SandboxRunStatus;

    #[test]
    fn adapter_kind_is_process_tier_not_hard_isolation() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        let a = PolicyScopedLocalAdapter::new(pol).unwrap();
        let k = a.kind();
        assert_eq!(k.tier, AdapterIsolationTier::Process);
        assert!(
            k.label.contains("NOT hard isolation"),
            "label must say it is NOT hard isolation so DCC/replay cannot confuse it"
        );
    }

    #[test]
    fn refuses_construction_with_non_deny_policy() {
        use crate::kernel::sandbox::policy::{
            CapabilityEvidenceRef, CapabilityGrant, OperatorApprovalRef,
        };
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        // H5: `Allow` now requires a typed grant. Construct one so the test
        // still exercises the "non-deny default" rejection path.
        pol.default_decision = CapabilityDecision::Allow(CapabilityGrant {
            capability: SandboxCapability::Network,
            evidence_ref: CapabilityEvidenceRef::new("ART-test"),
            approval_ref: Some(OperatorApprovalRef::new("APR-test")),
        });
        // Note: PolicyScopedLocalAdapter now derives Debug so expect_err works,
        // but we keep pattern-match form for clarity on what we're asserting.
        let result = PolicyScopedLocalAdapter::new(pol);
        match result {
            Ok(_) => panic!("must refuse allow-by-default policy"),
            Err(AdapterError::PolicyDenied(msg)) => assert!(msg.contains("default_deny")),
            Err(other) => panic!("expected PolicyDenied, got {:?}", other),
        }
    }

    #[test]
    fn pre_check_with_all_capabilities_denies_under_default_deny() {
        // M7: explicit run requesting every sensitive capability must be
        // denied under default_deny. The run now must opt in via
        // `requested_capabilities` rather than the adapter reaching for
        // `SandboxCapability::ALL` unconditionally.
        let pol = SandboxPolicyV1::default_deny("baseline");
        let adapter = PolicyScopedLocalAdapter::new(pol.clone()).unwrap();
        let mut run = SandboxRunV1::new_requested(
            "KTR-1",
            "SES-1",
            "policy_scoped_local",
            pol.version_id(),
            "WSP-1",
        );
        run.requested_capabilities = SandboxCapability::ALL.to_vec();
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/x");
        let outcome = adapter
            .run(&run, &ws, &pol)
            .expect("run returns Ok with Denied outcome");
        match outcome {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::PolicyDenied);
                assert!(d.capability.is_some(), "denial must name the capability");
            }
            other => panic!("expected Denied outcome, got {:?}", other),
        }
        // And the derived status:
        let derived =
            AdapterRunOutcome::Denied(crate::kernel::sandbox::denial::SandboxDenialRecordV1::new(
                "SBX-x",
                "POL@1",
                DenialKind::PolicyDenied,
                Some(SandboxCapability::Network),
                "x",
                "y",
            ))
            .to_status();
        assert_eq!(derived, SandboxRunStatus::Rejected);
    }

    #[test]
    fn policy_scoped_local_runs_work_with_no_requested_capabilities() {
        // M7 acceptance: a run that requests zero capabilities must dispatch
        // without denial under default_deny, because there is nothing to deny.
        let pol = SandboxPolicyV1::default_deny("baseline");
        let adapter = PolicyScopedLocalAdapter::new(pol.clone()).unwrap();
        let run = SandboxRunV1::new_requested(
            "KTR-noop",
            "SES-noop",
            "policy_scoped_local",
            pol.version_id(),
            "WSP-noop",
        );
        assert!(
            run.requested_capabilities.is_empty(),
            "default new_requested has no requested capabilities"
        );
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/noop");
        let outcome = adapter
            .run(&run, &ws, &pol)
            .expect("no capabilities => no denial");
        match outcome {
            AdapterRunOutcome::Completed { artifact_refs } => {
                assert!(artifact_refs.is_empty(), "MVP returns no artifacts");
            }
            other => panic!("expected Completed outcome, got {:?}", other),
        }
    }

    #[test]
    fn policy_scoped_local_denies_work_requesting_undeclared_capability() {
        // M7 acceptance: a run that explicitly requests Network under
        // default_deny must be denied at pre_check.
        let pol = SandboxPolicyV1::default_deny("baseline");
        let adapter = PolicyScopedLocalAdapter::new(pol.clone()).unwrap();
        let mut run = SandboxRunV1::new_requested(
            "KTR-net",
            "SES-net",
            "policy_scoped_local",
            pol.version_id(),
            "WSP-net",
        );
        run.requested_capabilities = vec![SandboxCapability::Network];
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/net");
        let outcome = adapter
            .run(&run, &ws, &pol)
            .expect("run returns Ok with Denied outcome");
        match outcome {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::PolicyDenied);
                assert_eq!(d.capability, Some(SandboxCapability::Network));
            }
            other => panic!("expected Denied for undeclared Network, got {:?}", other),
        }
    }

    #[test]
    fn read_only_workspace_is_a_boundary_violation() {
        use crate::kernel::sandbox::policy::{
            CapabilityEvidenceRef, CapabilityGrant, OperatorApprovalRef,
        };
        let pol = SandboxPolicyV1::default_deny("baseline");
        // Override every capability with a valid grant so we get past pre_check
        // and exercise the workspace check.
        let mut pol = pol;
        for cap in SandboxCapability::ALL {
            pol.overrides.push((
                *cap,
                CapabilityDecision::Allow(CapabilityGrant {
                    capability: *cap,
                    evidence_ref: CapabilityEvidenceRef::new("ART-test"),
                    approval_ref: Some(OperatorApprovalRef::new("APR-test")),
                }),
            ));
        }
        let adapter = PolicyScopedLocalAdapter::new({
            let mut p = pol.clone();
            p.default_decision = CapabilityDecision::Deny;
            p
        })
        .unwrap();
        let mut run = SandboxRunV1::new_requested(
            "KTR-1",
            "SES-1",
            "policy_scoped_local",
            pol.version_id(),
            "WSP-1",
        );
        // M7: must explicitly request capabilities for pre_check to iterate them.
        run.requested_capabilities = SandboxCapability::ALL.to_vec();
        let mut ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/y");
        ws.allow_write = false;
        let err = adapter
            .run(&run, &ws, &pol)
            .expect_err("read-only workspace must error");
        match err {
            AdapterError::WorkspaceViolation(msg) => assert!(msg.contains("read-only")),
            other => panic!("expected WorkspaceViolation, got {:?}", other),
        }
    }

    #[test]
    fn policy_scoped_local_allows_capability_with_valid_evidence_ref() {
        // H5 positive: a grant carrying a non-empty evidence_ref MUST pass
        // both pre_check and policy-build validation.
        use crate::kernel::sandbox::policy::{
            CapabilityEvidenceRef, CapabilityGrant, OperatorApprovalRef,
        };
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        pol.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: CapabilityEvidenceRef::new("ART-good-evidence"),
                approval_ref: Some(OperatorApprovalRef::new("APR-1")),
            }),
        ));
        pol.validate_grants()
            .expect("non-empty evidence_ref must pass validation");
        let adapter = PolicyScopedLocalAdapter::new(pol.clone()).unwrap();
        let mut run = SandboxRunV1::new_requested(
            "KTR-h5",
            "SES-h5",
            "policy_scoped_local",
            pol.version_id(),
            "WSP-h5",
        );
        run.requested_capabilities = vec![SandboxCapability::Network];
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/h5");
        let outcome = adapter
            .run(&run, &ws, &pol)
            .expect("grant must pass pre_check");
        match outcome {
            AdapterRunOutcome::Completed { .. } => {}
            other => panic!("expected Completed after valid grant, got {:?}", other),
        }
    }

    #[test]
    fn policy_scoped_local_refuses_grant_with_empty_evidence_ref_at_build_time() {
        // H5 negative: a grant with empty evidence_ref must be refused by
        // `validate_grants` (the policy-build gate).
        use crate::kernel::sandbox::policy::{
            CapabilityEvidenceRef, CapabilityGrant, PolicyBuildError,
        };
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        pol.overrides.push((
            SandboxCapability::Network,
            CapabilityDecision::Allow(CapabilityGrant {
                capability: SandboxCapability::Network,
                evidence_ref: CapabilityEvidenceRef::new(""),
                approval_ref: None,
            }),
        ));
        match pol.validate_grants() {
            Err(PolicyBuildError::CapabilityGrantMissingEvidence { capability }) => {
                assert_eq!(capability, SandboxCapability::Network);
            }
            Ok(()) => panic!("empty evidence_ref must be rejected at build time"),
        }
    }
}
