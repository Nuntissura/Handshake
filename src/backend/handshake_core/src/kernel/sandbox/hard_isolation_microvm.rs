//! Non-executing microVM (Cloud Hypervisor / Firecracker / gVisor)
//! hard-isolation adapter stub.
//!
//! Part of MT-020's adapter slot. Probes backing runtimes with fixed bounded
//! version commands, but never executes operator workloads. A detected runtime
//! is still BLOCKED until the microVM workload executor is wired; hosts with no
//! plausible backend surface UNSUPPORTED/BLOCKED with the probe details.

use super::adapter::{AdapterError, AdapterKind, AdapterRunOutcome, SandboxAdapter};
use super::hard_isolation::{
    hard_isolation_adapter_kind, probe_runtime_candidates, typed_unavailable_outcome,
    HardIsolationAdapter, HardIsolationAvailability, RuntimeProbeCandidate, RuntimeProbeOutcome,
};
use super::host_platform_probe::{HostKind, HostPlatformProbe};
use super::policy::SandboxPolicyV1;
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const MICROVM_ADAPTER_ID: &str = "hard_isolation_microvm";
pub const MICROVM_TIER_LABEL: &str = "microvm";

pub struct MicroVmAdapterStub {
    forced_host: Option<HostKind>,
    probe_candidates: Vec<RuntimeProbeCandidate>,
}

impl MicroVmAdapterStub {
    pub fn new() -> Self {
        Self {
            forced_host: None,
            probe_candidates: default_microvm_probe_candidates(HostPlatformProbe::detect()),
        }
    }

    /// Test/integration override for the detected host. Production callers must
    /// not use this; it exists so tests can assert "UNSUPPORTED on Windows"
    /// regardless of the actual build target.
    pub fn with_forced_host(host: HostKind) -> Self {
        Self {
            forced_host: Some(host),
            probe_candidates: default_microvm_probe_candidates(host),
        }
    }

    fn host(&self) -> HostKind {
        self.forced_host.unwrap_or_else(HostPlatformProbe::detect)
    }

    #[cfg(test)]
    pub fn with_probe_candidates(mut self, probe_candidates: Vec<RuntimeProbeCandidate>) -> Self {
        self.probe_candidates = probe_candidates;
        self
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
        match probe_runtime_candidates(&self.probe_candidates) {
            RuntimeProbeOutcome::Detected {
                runtime_id,
                command_line,
                runtime_version,
            } => HardIsolationAvailability::Blocked {
                reason: format!(
                    "microVM runtime probe detected `{runtime_id}` via `{command_line}`: \
                     {runtime_version}; microVM workload execution is still a non-executing \
                     MT-020 stub"
                ),
                missing_dependency: "microvm_sandbox_executor".to_string(),
            },
            RuntimeProbeOutcome::Missing { detail } => match self.host() {
                HostKind::Windows | HostKind::MacOs | HostKind::Other => {
                    HardIsolationAvailability::Unsupported {
                        reason: format!(
                            "no supported microVM runtime passed the automation-first probe: \
                             {detail}"
                        ),
                        host_kind: self.host().as_str().to_string(),
                    }
                }
                HostKind::Linux => HardIsolationAvailability::Blocked {
                    reason: format!(
                        "no Cloud Hypervisor/Firecracker/gVisor runtime passed the \
                         automation-first probe on Linux: {detail}"
                    ),
                    missing_dependency: "cloud_hypervisor_or_firecracker_or_runsc_runtime"
                        .to_string(),
                },
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

fn default_microvm_probe_candidates(_host: HostKind) -> Vec<RuntimeProbeCandidate> {
    vec![
        RuntimeProbeCandidate::new("cloud_hypervisor", "cloud-hypervisor", &["--version"]),
        RuntimeProbeCandidate::new("firecracker", "firecracker", &["--version"]),
        RuntimeProbeCandidate::new("gvisor_runsc", "runsc", &["--version"]),
    ]
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
        let a =
            MicroVmAdapterStub::with_forced_host(HostKind::Windows).with_probe_candidates(vec![
                RuntimeProbeCandidate::new(
                    "missing_microvm_runtime",
                    "hsk-missing-microvm-runtime-probe",
                    &["--version"],
                ),
            ]);
        match a.probe_availability() {
            HardIsolationAvailability::Unsupported { host_kind, reason } => {
                assert_eq!(host_kind, "windows");
                assert!(reason.contains("automation-first probe"));
                assert!(reason.contains("hsk-missing-microvm-runtime-probe"));
            }
            other => panic!("must be UNSUPPORTED on Windows, got {:?}", other),
        }
    }

    #[test]
    fn blocked_on_linux_forced_host() {
        let a = MicroVmAdapterStub::with_forced_host(HostKind::Linux).with_probe_candidates(vec![
            RuntimeProbeCandidate::new(
                "missing_microvm_runtime",
                "hsk-missing-microvm-runtime-probe",
                &["--version"],
            ),
        ]);
        match a.probe_availability() {
            HardIsolationAvailability::Blocked {
                missing_dependency,
                reason,
            } => {
                assert_eq!(
                    missing_dependency,
                    "cloud_hypervisor_or_firecracker_or_runsc_runtime"
                );
                assert!(reason.contains("automation-first probe on Linux"));
            }
            other => panic!("must be BLOCKED on Linux, got {:?}", other),
        }
    }

    #[test]
    fn probe_records_detected_runtime_but_keeps_stub_blocked() {
        let current_exe = std::env::current_exe().expect("current test binary path");
        let a = MicroVmAdapterStub::with_forced_host(HostKind::Linux).with_probe_candidates(vec![
            RuntimeProbeCandidate::new(
                "current_test_binary",
                current_exe.to_string_lossy().to_string(),
                &["--help"],
            ),
        ]);
        match a.probe_availability() {
            HardIsolationAvailability::Blocked {
                missing_dependency,
                reason,
            } => {
                assert_eq!(missing_dependency, "microvm_sandbox_executor");
                assert!(reason.contains("current_test_binary"));
                assert!(reason.contains("non-executing MT-020 stub"));
            }
            other => panic!("probe-only microVM stub must stay BLOCKED, got {other:?}"),
        }
    }

    #[test]
    fn run_on_windows_is_typed_denial() {
        let a =
            MicroVmAdapterStub::with_forced_host(HostKind::Windows).with_probe_candidates(vec![
                RuntimeProbeCandidate::new(
                    "missing_microvm_runtime",
                    "hsk-missing-microvm-runtime-probe",
                    &["--version"],
                ),
            ]);
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
