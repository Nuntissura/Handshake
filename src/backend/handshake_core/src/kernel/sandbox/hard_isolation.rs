//! MT-020 HardIsolation Adapter Stub.
//!
//! Acceptance (MT-020.json): "add non-executing adapter slot for hard
//! isolation. Acceptance: hard isolation absence is typed BLOCKED/UNSUPPORTED,
//! not success."
//!
//! This module defines the typed availability surface every HardIsolation
//! adapter (container, microVM, future tiers) reuses, plus a helper that
//! converts an "unavailable backing runtime" condition into a typed
//! `AdapterRunOutcome::Denied` (`DenialKind::AdapterUnavailable`).
//!
//! No hard-isolation adapter under this WP performs workload execution. They
//! MUST surface BLOCKED/UNSUPPORTED through this slot so absence is never
//! confused with success. Availability probes are allowed to run fixed,
//! bounded, non-workload commands such as `docker --version`; downstream batches
//! may replace the placeholder runtimes once Docker/Podman/Firecracker
//! integration lands.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use super::adapter::{AdapterError, AdapterIsolationTier, AdapterKind, AdapterRunOutcome};
use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::policy::SandboxCapability;
use super::run::SandboxRunV1;

/// Typed availability of a hard-isolation backing runtime.
///
/// Every hard-isolation adapter MUST resolve to one of these states before
/// claiming to run. The `Available` variant is reserved for future Wave-C
/// integration work; no MT-020..MT-029 adapter returns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardIsolationAvailability {
    /// Backing runtime detected and ready (reserved; no stub returns this).
    Available {
        runtime_id: String,
        runtime_version: String,
    },
    /// Host platform / build configuration cannot support this isolation tier.
    /// Example: microVM tier on Windows when no hypervisor backend is present.
    Unsupported { reason: String, host_kind: String },
    /// Tier could theoretically run on this host, but a required dependency is
    /// missing. Example: container tier with no docker/podman binary in PATH.
    Blocked {
        reason: String,
        missing_dependency: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProbeCandidate {
    pub runtime_id: &'static str,
    pub command: String,
    pub args: Vec<String>,
}

impl RuntimeProbeCandidate {
    pub fn new(runtime_id: &'static str, command: impl Into<String>, args: &[&str]) -> Self {
        Self {
            runtime_id,
            command: command.into(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    fn command_line(&self) -> String {
        std::iter::once(self.command.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeProbeOutcome {
    Detected {
        runtime_id: String,
        command_line: String,
        runtime_version: String,
    },
    Missing {
        detail: String,
    },
}

impl HardIsolationAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported { .. })
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Available { .. } => "AVAILABLE",
            Self::Unsupported { .. } => "UNSUPPORTED",
            Self::Blocked { .. } => "BLOCKED",
        }
    }
}

const RUNTIME_PROBE_TIMEOUT: Duration = Duration::from_millis(750);

pub fn probe_runtime_candidates(candidates: &[RuntimeProbeCandidate]) -> RuntimeProbeOutcome {
    let mut failures = Vec::new();
    for candidate in candidates {
        match probe_runtime_candidate(candidate) {
            Ok(version) => {
                return RuntimeProbeOutcome::Detected {
                    runtime_id: candidate.runtime_id.to_string(),
                    command_line: candidate.command_line(),
                    runtime_version: version,
                };
            }
            Err(error) => failures.push(format!("{} => {error}", candidate.command_line())),
        }
    }
    RuntimeProbeOutcome::Missing {
        detail: if failures.is_empty() {
            "no runtime probe candidates were configured".to_string()
        } else {
            failures.join("; ")
        },
    }
}

fn probe_runtime_candidate(candidate: &RuntimeProbeCandidate) -> Result<String, String> {
    if is_shell_like_dispatcher(&candidate.command) {
        return Err(format!(
            "rejected shell-like dispatcher `{}`; runtime probes must use direct \
             fixed binary commands",
            candidate.command
        ));
    }
    let mut command = Command::new(&candidate.command);
    command
        .args(&candidate.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn failed: {error}"))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|error| format!("output collection failed after exit: {error}"))?;
                if status.success() {
                    return Ok(first_probe_line(&output.stdout, &output.stderr));
                }
                return Err(format!(
                    "exit {status}; stdout=`{}` stderr=`{}`",
                    compact_probe_text(&output.stdout),
                    compact_probe_text(&output.stderr)
                ));
            }
            Ok(None) if started.elapsed() >= RUNTIME_PROBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "timed out after {}ms",
                    RUNTIME_PROBE_TIMEOUT.as_millis()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("wait failed: {error}"));
            }
        }
    }
}

fn is_shell_like_dispatcher(command: &str) -> bool {
    let name = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "cmd"
            | "cmd.exe"
            | "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "wsl"
            | "wsl.exe"
            | "sh"
            | "bash"
            | "zsh"
    )
}

fn first_probe_line(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = compact_probe_text(stdout);
    if !stdout.is_empty() {
        return stdout;
    }
    let stderr = compact_probe_text(stderr);
    if !stderr.is_empty() {
        return stderr;
    }
    "version command exited successfully without output".to_string()
}

fn compact_probe_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .take(240)
        .collect()
}

/// Marker trait every hard-isolation adapter implements in addition to
/// `SandboxAdapter`. It lets the selection layer (`adapter_selection`) probe
/// availability without running the sandbox, and forces every stub to declare
/// a backing-runtime story.
pub trait HardIsolationAdapter: super::adapter::SandboxAdapter {
    /// Probe the host for the backing runtime. Implementations may run fixed
    /// bounded version/status commands, but MUST NOT execute operator payloads
    /// or mint successful workload outcomes from probe success alone.
    fn probe_availability(&self) -> HardIsolationAvailability;

    /// Stable hard-isolation tier label, e.g. "container", "microvm".
    fn hard_isolation_tier_label(&self) -> &'static str;

    /// Explicit upcast to `&dyn SandboxAdapter`. Stable rust did not have
    /// trait upcasting until 1.86; this method makes selection code compile
    /// on any toolchain we target. Implementors should write
    /// `fn as_sandbox_adapter(&self) -> &dyn SandboxAdapter { self }`.
    fn as_sandbox_adapter(&self) -> &dyn super::adapter::SandboxAdapter;
}

/// Build a typed `Denied` outcome for a hard-isolation adapter whose backing
/// runtime is not available. The denial carries:
///   * `DenialKind::AdapterUnavailable`
///   * the failing capability (when known) for DCC routing
///   * an action description naming the adapter and tier
///   * a reason that quotes the availability state so replays/audits can see
///     whether the cause was UNSUPPORTED vs BLOCKED.
///
/// Caller MUST NOT pass an `Available` state into this helper; it panics in
/// debug builds and returns a typed-internal denial in release builds.
pub fn typed_unavailable_denial(
    run: &SandboxRunV1,
    adapter_kind: &AdapterKind,
    tier_label: &str,
    availability: &HardIsolationAvailability,
    capability_hint: Option<SandboxCapability>,
) -> SandboxDenialRecordV1 {
    debug_assert!(
        !availability.is_available(),
        "typed_unavailable_denial called with AVAILABLE state; that is a programmer error"
    );

    let (action, reason) = match availability {
        HardIsolationAvailability::Unsupported { reason, host_kind } => (
            format!(
                "hard-isolation adapter `{}` (tier {}) refused run: host {} cannot host this tier",
                adapter_kind.id, tier_label, host_kind
            ),
            format!("UNSUPPORTED on host `{}`: {}", host_kind, reason),
        ),
        HardIsolationAvailability::Blocked {
            reason,
            missing_dependency,
        } => (
            format!(
                "hard-isolation adapter `{}` (tier {}) refused run: missing `{}`",
                adapter_kind.id, tier_label, missing_dependency
            ),
            format!(
                "BLOCKED: dependency `{}` not present: {}",
                missing_dependency, reason
            ),
        ),
        HardIsolationAvailability::Available { runtime_id, .. } => (
            format!(
                "hard-isolation adapter `{}` reported AVAILABLE for runtime `{}` but caller forced denial",
                adapter_kind.id, runtime_id
            ),
            "internal: typed_unavailable_denial invoked with AVAILABLE state".to_string(),
        ),
    };

    SandboxDenialRecordV1::new(
        run.run_id.0.clone(),
        run.policy_version_id.clone(),
        DenialKind::AdapterUnavailable,
        capability_hint,
        action,
        reason,
    )
}

/// Convenience wrapper: produce a full `AdapterRunOutcome::Denied` from a
/// non-available availability. Hard-isolation stubs call this from their
/// `run(...)` implementations to guarantee absence is typed denial, not
/// `Completed`.
pub fn typed_unavailable_outcome(
    run: &SandboxRunV1,
    adapter_kind: &AdapterKind,
    tier_label: &str,
    availability: &HardIsolationAvailability,
    capability_hint: Option<SandboxCapability>,
) -> Result<AdapterRunOutcome, AdapterError> {
    if availability.is_available() {
        return Err(AdapterError::Internal(
            "typed_unavailable_outcome cannot be used with AVAILABLE state".to_string(),
        ));
    }
    Ok(AdapterRunOutcome::Denied(typed_unavailable_denial(
        run,
        adapter_kind,
        tier_label,
        availability,
        capability_hint,
    )))
}

/// Build an `AdapterKind` for hard-isolation stubs. Forces
/// `tier = HardIsolation` and embeds the tier label in the kind label so DCC
/// and replay can never confuse a microVM stub for a container stub.
pub fn hard_isolation_adapter_kind(
    id: impl Into<String>,
    tier_label: &str,
    label: impl Into<String>,
) -> AdapterKind {
    let id = id.into();
    let label = label.into();
    AdapterKind {
        id,
        tier: AdapterIsolationTier::HardIsolation,
        version: 1,
        label: format!("{} [hard_isolation:{}]", label, tier_label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::denial::DenialKind;

    fn fixture_run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "hi-stub", "POL-1@1", "WSP-1")
    }

    #[test]
    fn availability_short_label_is_stable() {
        let avail = HardIsolationAvailability::Available {
            runtime_id: "rt".into(),
            runtime_version: "0".into(),
        };
        assert_eq!(avail.short_label(), "AVAILABLE");
        let unsupp = HardIsolationAvailability::Unsupported {
            reason: "no kvm".into(),
            host_kind: "windows".into(),
        };
        assert_eq!(unsupp.short_label(), "UNSUPPORTED");
        let blocked = HardIsolationAvailability::Blocked {
            reason: "missing".into(),
            missing_dependency: "docker".into(),
        };
        assert_eq!(blocked.short_label(), "BLOCKED");
    }

    #[test]
    fn hard_isolation_kind_encodes_tier_label() {
        let k = hard_isolation_adapter_kind("hi-container", "container", "Container stub");
        assert_eq!(k.tier, AdapterIsolationTier::HardIsolation);
        assert!(k.label.contains("hard_isolation:container"));
    }

    #[test]
    fn unsupported_state_produces_typed_denial_not_success() {
        let run = fixture_run();
        let kind = hard_isolation_adapter_kind("hi-microvm", "microvm", "microVM stub");
        let avail = HardIsolationAvailability::Unsupported {
            reason: "no hypervisor backend".into(),
            host_kind: "windows".into(),
        };
        let denial = typed_unavailable_denial(&run, &kind, "microvm", &avail, None);
        assert_eq!(denial.kind, DenialKind::AdapterUnavailable);
        assert!(denial.reason.contains("UNSUPPORTED"));
        assert!(denial.reason.contains("windows"));
        assert!(denial.action_description.contains("hi-microvm"));
    }

    #[test]
    fn blocked_state_carries_missing_dependency_in_reason() {
        let run = fixture_run();
        let kind = hard_isolation_adapter_kind("hi-container", "container", "container stub");
        let avail = HardIsolationAvailability::Blocked {
            reason: "binary not on PATH".into(),
            missing_dependency: "docker".into(),
        };
        let denial = typed_unavailable_denial(
            &run,
            &kind,
            "container",
            &avail,
            Some(SandboxCapability::ProcessSpawn),
        );
        assert_eq!(denial.kind, DenialKind::AdapterUnavailable);
        assert_eq!(denial.capability, Some(SandboxCapability::ProcessSpawn));
        assert!(denial.reason.contains("docker"));
        assert!(denial.reason.contains("BLOCKED"));
    }

    #[test]
    fn outcome_helper_returns_denied_not_completed_for_blocked() {
        let run = fixture_run();
        let kind = hard_isolation_adapter_kind("hi-container", "container", "container stub");
        let avail = HardIsolationAvailability::Blocked {
            reason: "not installed".into(),
            missing_dependency: "podman".into(),
        };
        let outcome = typed_unavailable_outcome(&run, &kind, "container", &avail, None).unwrap();
        match outcome {
            AdapterRunOutcome::Denied(d) => {
                assert_eq!(d.kind, DenialKind::AdapterUnavailable);
                assert!(!d.action_description.is_empty());
                assert!(
                    !d.reason.is_empty(),
                    "reason must carry typed BLOCKED/UNSUPPORTED detail"
                );
            }
            other => panic!(
                "absent hard-isolation runtime MUST be typed denial, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn outcome_helper_refuses_available_state() {
        let run = fixture_run();
        let kind = hard_isolation_adapter_kind("hi-x", "container", "x");
        let avail = HardIsolationAvailability::Available {
            runtime_id: "docker".into(),
            runtime_version: "1.0".into(),
        };
        let err = typed_unavailable_outcome(&run, &kind, "container", &avail, None)
            .expect_err("AVAILABLE must not produce a denial outcome");
        match err {
            AdapterError::Internal(msg) => assert!(msg.contains("AVAILABLE")),
            other => panic!("expected Internal error, got {:?}", other),
        }
    }

    #[test]
    fn runtime_probe_detects_current_test_binary_help() {
        let current_exe = std::env::current_exe().expect("current test binary path");
        let probe = RuntimeProbeCandidate::new(
            "current_test_binary",
            current_exe.to_string_lossy().to_string(),
            &["--help"],
        );
        match probe_runtime_candidates(&[probe]) {
            RuntimeProbeOutcome::Detected {
                runtime_id,
                runtime_version,
                ..
            } => {
                assert_eq!(runtime_id, "current_test_binary");
                assert!(!runtime_version.is_empty());
            }
            other => panic!("expected detected test binary probe, got {other:?}"),
        }
    }

    #[test]
    fn runtime_probe_reports_missing_candidates_without_passing_static_audit() {
        let probe = RuntimeProbeCandidate::new(
            "definitely_missing",
            "hsk-definitely-missing-runtime-probe-binary",
            &["--version"],
        );
        match probe_runtime_candidates(&[probe]) {
            RuntimeProbeOutcome::Missing { detail } => {
                assert!(detail.contains("hsk-definitely-missing-runtime-probe-binary"));
                assert!(detail.contains("spawn failed"));
            }
            other => panic!("expected missing probe detail, got {other:?}"),
        }
    }

    #[test]
    fn runtime_probe_rejects_shell_like_dispatcher_candidates() {
        let probe = RuntimeProbeCandidate::new("wsl_dispatcher", "wsl.exe", &["-e", "true"]);
        match probe_runtime_candidates(&[probe]) {
            RuntimeProbeOutcome::Missing { detail } => {
                assert!(detail.contains("rejected shell-like dispatcher"));
                assert!(detail.contains("wsl.exe"));
            }
            other => panic!("expected dispatcher rejection, got {other:?}"),
        }
    }
}
