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
    AdapterError, AdapterKind, AdapterIsolationTier, AdapterRunOutcome, SandboxAdapter,
};
use super::policy::{CapabilityDecision, SandboxCapability, SandboxPolicyV1};
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const POLICY_SCOPED_LOCAL_ADAPTER_ID: &str = "policy_scoped_local";
pub const POLICY_SCOPED_LOCAL_ADAPTER_LABEL: &str =
    "PolicyScopedLocal (process tier, NOT hard isolation)";

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
        // Always check the full sensitive-capability set against the policy.
        if let Err(denial) = self.pre_check(run, policy, SandboxCapability::ALL) {
            return Ok(AdapterRunOutcome::Denied(denial));
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
        let mut pol = SandboxPolicyV1::default_deny("baseline");
        pol.default_decision = CapabilityDecision::Allow;
        let err = PolicyScopedLocalAdapter::new(pol).expect_err("must refuse allow-by-default policy");
        match err {
            AdapterError::PolicyDenied(msg) => assert!(msg.contains("default_deny")),
            other => panic!("expected PolicyDenied, got {:?}", other),
        }
    }

    #[test]
    fn denies_every_sensitive_capability_by_default() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        let adapter = PolicyScopedLocalAdapter::new(pol.clone()).unwrap();
        let run = SandboxRunV1::new_requested("KTR-1", "SES-1", "policy_scoped_local", pol.version_id(), "WSP-1");
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/x");
        let outcome = adapter.run(&run, &ws, &pol).expect("run returns Ok with Denied outcome");
        match outcome {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::PolicyDenied);
                assert!(d.capability.is_some(), "denial must name the capability");
            }
            other => panic!("expected Denied outcome, got {:?}", other),
        }
        // And the derived status:
        let derived = AdapterRunOutcome::Denied(crate::kernel::sandbox::denial::SandboxDenialRecordV1::new(
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
    fn read_only_workspace_is_a_boundary_violation() {
        let pol = SandboxPolicyV1::default_deny("baseline");
        // Override one capability so we get past pre_check and exercise the workspace check.
        let mut pol = pol;
        for cap in SandboxCapability::ALL {
            pol.overrides.push((*cap, CapabilityDecision::AllowWithEvidence));
        }
        let adapter = PolicyScopedLocalAdapter::new({
            let mut p = pol.clone();
            p.default_decision = CapabilityDecision::Deny;
            p
        }).unwrap();
        let run = SandboxRunV1::new_requested("KTR-1", "SES-1", "policy_scoped_local", pol.version_id(), "WSP-1");
        let mut ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/y");
        ws.allow_write = false;
        let err = adapter.run(&run, &ws, &pol).expect_err("read-only workspace must error");
        match err {
            AdapterError::WorkspaceViolation(msg) => assert!(msg.contains("read-only")),
            other => panic!("expected WorkspaceViolation, got {:?}", other),
        }
    }
}
