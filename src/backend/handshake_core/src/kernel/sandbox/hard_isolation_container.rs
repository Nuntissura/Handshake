//! Non-executing Container (Docker/Podman) hard-isolation adapter stub.
//!
//! Part of MT-020's adapter slot: probes Docker/Podman with fixed bounded
//! `--version` commands, then returns BLOCKED with typed evidence whenever
//! called. No Docker SDK dependency and no workload execution. Future Wave-C
//! work replaces the body with a real container backend.

use super::adapter::{AdapterError, AdapterKind, AdapterRunOutcome, SandboxAdapter};
use super::hard_isolation::{
    HardIsolationAdapter, HardIsolationAvailability, RuntimeProbeCandidate, RuntimeProbeOutcome,
    hard_isolation_adapter_kind, probe_runtime_candidates, typed_unavailable_outcome,
};
use super::policy::SandboxPolicyV1;
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

pub const CONTAINER_ADAPTER_ID: &str = "hard_isolation_container";
pub const CONTAINER_TIER_LABEL: &str = "container";

pub struct ContainerAdapterStub {
    label: String,
    probe_candidates: Vec<RuntimeProbeCandidate>,
}

impl ContainerAdapterStub {
    pub fn new() -> Self {
        Self {
            label: "Container stub (no docker/podman backend wired)".to_string(),
            probe_candidates: default_container_probe_candidates(),
        }
    }

    #[cfg(test)]
    pub fn with_probe_candidates(mut self, probe_candidates: Vec<RuntimeProbeCandidate>) -> Self {
        self.probe_candidates = probe_candidates;
        self
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
        match probe_runtime_candidates(&self.probe_candidates) {
            RuntimeProbeOutcome::Detected {
                runtime_id,
                command_line,
                runtime_version,
            } => HardIsolationAvailability::Blocked {
                reason: format!(
                    "container runtime probe detected `{runtime_id}` via `{command_line}`: \
                     {runtime_version}; hard-isolation workload execution is still a \
                     non-executing MT-020 stub"
                ),
                missing_dependency: "container_sandbox_executor".to_string(),
            },
            RuntimeProbeOutcome::Missing { detail } => HardIsolationAvailability::Blocked {
                reason: format!(
                    "no Docker/Podman runtime passed the automation-first availability probe: \
                     {detail}"
                ),
                missing_dependency: "docker_or_podman_runtime".to_string(),
            },
        }
    }

    fn hard_isolation_tier_label(&self) -> &'static str {
        CONTAINER_TIER_LABEL
    }

    fn as_sandbox_adapter(&self) -> &dyn SandboxAdapter {
        self
    }
}

fn default_container_probe_candidates() -> Vec<RuntimeProbeCandidate> {
    vec![
        RuntimeProbeCandidate::new("docker", "docker", &["--version"]),
        RuntimeProbeCandidate::new("podman", "podman", &["--version"]),
    ]
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
    fn probe_is_blocked_with_missing_runtime_dependency() {
        let a =
            ContainerAdapterStub::new().with_probe_candidates(vec![RuntimeProbeCandidate::new(
                "missing_container_runtime",
                "hsk-missing-container-runtime-probe",
                &["--version"],
            )]);
        let av = a.probe_availability();
        match av {
            HardIsolationAvailability::Blocked {
                missing_dependency,
                reason,
            } => {
                assert_eq!(missing_dependency, "docker_or_podman_runtime");
                assert!(reason.contains("automation-first availability probe"));
                assert!(reason.contains("hsk-missing-container-runtime-probe"));
            }
            other => panic!("container stub must be BLOCKED, got {:?}", other),
        }
    }

    #[test]
    fn probe_records_detected_runtime_but_keeps_stub_blocked() {
        let current_exe = std::env::current_exe().expect("current test binary path");
        let a =
            ContainerAdapterStub::new().with_probe_candidates(vec![RuntimeProbeCandidate::new(
                "current_test_binary",
                current_exe.to_string_lossy().to_string(),
                &["--help"],
            )]);
        match a.probe_availability() {
            HardIsolationAvailability::Blocked {
                missing_dependency,
                reason,
            } => {
                assert_eq!(missing_dependency, "container_sandbox_executor");
                assert!(reason.contains("current_test_binary"));
                assert!(reason.contains("non-executing MT-020 stub"));
            }
            other => panic!("probe-only container stub must stay BLOCKED, got {other:?}"),
        }
    }

    #[test]
    fn run_returns_typed_denial_with_nonempty_missing_dependency() {
        let a =
            ContainerAdapterStub::new().with_probe_candidates(vec![RuntimeProbeCandidate::new(
                "missing_container_runtime",
                "hsk-missing-container-runtime-probe",
                &["--version"],
            )]);
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
