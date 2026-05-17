//! Non-executing microVM (Firecracker / gVisor) hard-isolation adapter stub.
//!
//! Part of MT-020's adapter slot. UNSUPPORTED on Windows hosts (no
//! hypervisor backend at this stub level), BLOCKED elsewhere on missing
//! runtime dependency. Never executes anything.

use super::adapter::{
    AdapterError, AdapterKind, AdapterRunOutcome, SandboxAdapter,
};
use super::hard_isolation::{
    hard_isolation_adapter_kind, typed_unavailable_outcome, HardIsolationAdapter,
    HardIsolationAvailability,
};
use super::host_platform_probe::{HostKind, HostPlatformProbe};
use super::policy::SandboxPolicyV1;
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const MICROVM_ADAPTER_ID: &str = "hard_isolation_microvm";
pub const MICROVM_TIER_LABEL: &str = "microvm";

pub struct MicroVmAdapterStub {
    forced_host: Option<HostKind>,
}

impl MicroVmAdapterStub {
    pub fn new() -> Self {
        Self { forced_host: None }
    }

    /// Test/integration override for the detected host. Production callers must
    /// not use this; it exists so tests can assert "UNSUPPORTED on Windows"
    /// regardless of the actual build target.
    pub fn with_forced_host(host: HostKind) -> Self {
        Self {
            forced_host: Some(host),
        }
    }

    fn host(&self) -> HostKind {
        self.forced_host.unwrap_or_else(HostPlatformProbe::detect)
    }
}

impl Default for MicroVmAdapterStub {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxAdapter for MicroVmAdapterStub {
    fn kind(&self) -> AdapterKind {
        hard_isolation_adapter_kind(
            MICROVM_ADAPTER_ID,
            MICROVM_TIER_LABEL,
            "microVM stub (no firecracker/gvisor backend wired)",
        )
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        _workspace: &SandboxWorkspaceV1,
        _policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        let availability = self.probe_availability();
        typed_unavailable_outcome(run, &self.kind(), MICROVM_TIER_LABEL, &availability, None)
    }
}

impl HardIsolationAdapter for MicroVmAdapterStub {
    fn probe_availability(&self) -> HardIsolationAvailability {
        match self.host() {
            HostKind::Windows => HardIsolationAvailability::Unsupported {
                reason:
                    "microVM hard-isolation stub does not support Windows hosts at this tier"
                        .into(),
                host_kind: HostKind::Windows.as_str().to_string(),
            },
            other => HardIsolationAvailability::Blocked {
                reason:
                    "microVM hard-isolation backend is a non-executing stub under WP-KERNEL-003"
                        .to_string(),
                missing_dependency: format!("firecracker_or_gvisor_runtime_on_{}", other.as_str()),
            },
        }
    }

    fn hard_isolation_tier_label(&self) -> &'static str {
        MICROVM_TIER_LABEL
    }

    fn as_sandbox_adapter(&self) -> &dyn SandboxAdapter {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::denial::DenialKind;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", MICROVM_ADAPTER_ID, "POL-1@1", "WSP-1")
    }

    #[test]
    fn kind_is_hard_isolation_and_labels_microvm() {
        let a = MicroVmAdapterStub::new();
        let k = a.kind();
        assert!(k.label.contains("hard_isolation:microvm"));
    }

    #[test]
    fn unsupported_on_windows_forced_host() {
        let a = MicroVmAdapterStub::with_forced_host(HostKind::Windows);
        match a.probe_availability() {
            HardIsolationAvailability::Unsupported { host_kind, .. } => {
                assert_eq!(host_kind, "windows");
            }
            other => panic!("must be UNSUPPORTED on Windows, got {:?}", other),
        }
    }

    #[test]
    fn blocked_on_linux_forced_host() {
        let a = MicroVmAdapterStub::with_forced_host(HostKind::Linux);
        match a.probe_availability() {
            HardIsolationAvailability::Blocked {
                missing_dependency, ..
            } => {
                assert!(missing_dependency.contains("linux"));
            }
            other => panic!("must be BLOCKED on Linux, got {:?}", other),
        }
    }

    #[test]
    fn run_on_windows_is_typed_denial() {
        let a = MicroVmAdapterStub::with_forced_host(HostKind::Windows);
        let ws = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/x");
        let pol = SandboxPolicyV1::default_deny("baseline");
        match a.run(&run(), &ws, &pol).unwrap() {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::AdapterUnavailable);
                assert!(d.reason.contains("UNSUPPORTED"));
                assert!(d.reason.contains("windows"));
            }
            other => panic!("microvm stub on Windows MUST deny, got {:?}", other),
        }
    }
}
