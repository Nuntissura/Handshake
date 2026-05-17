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
//! No hard-isolation adapter under this WP performs execution. They MUST surface
//! BLOCKED/UNSUPPORTED through this slot so absence is never confused with
//! success. Downstream batches (Wave E / future Wave C) may replace the
//! placeholder runtimes once Docker/Podman/Firecracker integration lands.

use serde::{Deserialize, Serialize};

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
    Unsupported {
        reason: String,
        host_kind: String,
    },
    /// Tier could theoretically run on this host, but a required dependency is
    /// missing. Example: container tier with no docker/podman binary in PATH.
    Blocked {
        reason: String,
        missing_dependency: String,
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

/// Marker trait every hard-isolation adapter implements in addition to
/// `SandboxAdapter`. It lets the selection layer (`adapter_selection`) probe
/// availability without running the sandbox, and forces every stub to declare
/// a backing-runtime story.
pub trait HardIsolationAdapter: super::adapter::SandboxAdapter {
    /// Probe the host for the backing runtime. Implementations MUST be pure
    /// (no execution, no shell-out) so adapter selection stays deterministic.
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
        let outcome =
            typed_unavailable_outcome(&run, &kind, "container", &avail, None).unwrap();
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
}
