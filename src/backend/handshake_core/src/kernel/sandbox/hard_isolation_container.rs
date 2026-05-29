//! Non-executing Container (Docker/Podman) hard-isolation adapter stub.
//!
//! Part of MT-020's adapter slot: returns BLOCKED with a typed missing
//! dependency whenever called. No shell-out, no docker SDK dependency. Future
//! Wave-C work replaces the body with a real container backend.

use super::adapter::{AdapterError, AdapterKind, AdapterRunOutcome, SandboxAdapter};
use super::hard_isolation::{
    hard_isolation_adapter_kind, typed_unavailable_outcome, HardIsolationAdapter,
    HardIsolationAvailability,
};
use super::policy::SandboxPolicyV1;
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const CONTAINER_ADAPTER_ID: &str = "hard_isolation_container";
pub const CONTAINER_TIER_LABEL: &str = "container";

pub struct ContainerAdapterStub {
    label: String,
    missing_dependency: String,
}

impl ContainerAdapterStub {
    pub fn new() -> Self {
        Self {
            label: "Container stub (no docker/podman backend wired)".to_string(),
            missing_dependency: "docker_or_podman_runtime".to_string(),
        }
    }
}

impl Default for ContainerAdapterStub {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxAdapter for ContainerAdapterStub {
    fn kind(&self) -> AdapterKind {
        hard_isolation_adapter_kind(CONTAINER_ADAPTER_ID, CONTAINER_TIER_LABEL, &self.label)
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        _workspace: &SandboxWorkspaceV1,
        _policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        let availability = self.probe_availability();
        typed_unavailable_outcome(run, &self.kind(), CONTAINER_TIER_LABEL, &availability, None)
    }
}

impl HardIsolationAdapter for ContainerAdapterStub {
    fn probe_availability(&self) -> HardIsolationAvailability {
        // Stub: no backend ever present. Always BLOCKED on missing dependency.
        HardIsolationAvailability::Blocked {
            reason: "container hard-isolation backend is a non-executing stub under WP-KERNEL-003"
                .to_string(),
            missing_dependency: self.missing_dependency.clone(),
        }
    }

    fn hard_isolation_tier_label(&self) -> &'static str {
        CONTAINER_TIER_LABEL
    }

    fn as_sandbox_adapter(&self) -> &dyn SandboxAdapter {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::denial::DenialKind;
    use crate::kernel::sandbox::workspace::SandboxWorkspaceV1;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", CONTAINER_ADAPTER_ID, "POL-1@1", "WSP-1")
    }

    #[test]
    fn kind_is_hard_isolation_and_labels_container() {
        let a = ContainerAdapterStub::new();
        let k = a.kind();
        assert_eq!(
            k.tier,
            crate::kernel::sandbox::adapter::AdapterIsolationTier::HardIsolation
        );
        assert!(k.label.contains("hard_isolation:container"));
    }

    #[test]
    fn probe_is_always_blocked_with_missing_dependency() {
        let a = ContainerAdapterStub::new();
        let av = a.probe_availability();
        match av {
            HardIsolationAvailability::Blocked {
                missing_dependency,
                reason,
            } => {
                assert!(!missing_dependency.is_empty());
                assert!(reason.contains("non-executing stub"));
            }
            other => panic!("container stub must be BLOCKED, got {:?}", other),
        }
    }

    #[test]
    fn run_returns_typed_denial_with_nonempty_missing_dependency() {
        let a = ContainerAdapterStub::new();
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/x");
        let pol = SandboxPolicyV1::default_deny("baseline");
        let outcome = a.run(&run(), &ws, &pol).expect("Ok(Denied) expected");
        match outcome {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::AdapterUnavailable);
                assert!(d.reason.contains("docker_or_podman_runtime"));
                assert!(d.reason.contains("BLOCKED"));
            }
            other => panic!("container stub MUST return Denied, got {:?}", other),
        }
    }
}
