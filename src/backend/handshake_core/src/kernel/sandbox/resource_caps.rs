//! MT-026 Resource Cap Policy.
//!
//! Acceptance (MT-026.json): "fold MTE resource caps into sandbox policy.
//! Acceptance: overage halts or gates deterministically with evidence."
//!
//! The runner reports observed resource usage; the policy declares caps.
//! `ResourceCapEvaluator` produces a deterministic decision:
//!   * `Allow` when usage is under every cap (or no cap is set).
//!   * `Gate` when usage hits the warn threshold (80% of the cap).
//!   * `Halt` when usage meets or exceeds the cap.
//!
//! Halt and Gate decisions carry a typed `ResourceOverageEvidenceV1` so the
//! denial sink can surface which cap fired without scraping logs.

use serde::{Deserialize, Serialize};

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::policy_default_deny::ResourceCapsV1;
use super::run::SandboxRunV1;

/// M6: deterministic safe defaults for `default_deny` resource caps.
///
/// Previously `default_deny` produced an all-`None` `ResourceCapsV1`, which
/// the cap evaluator treats as "unbounded for that dimension". That
/// contradicted the documented "default-deny" stance because a run could
/// consume unlimited wall-clock, CPU, memory, FDs, and output bytes without
/// any cap firing.
///
/// `SAFE_DEFAULT_RESOURCE_CAPS` populates every dimension this struct
/// exposes with conservative defaults. Production callers that need
/// unbounded behavior for a specific dimension must opt in by constructing
/// `ResourceCapsV1` directly with `None` on that field; explicit `None` is
/// the documented "unbounded by design" signal and is preserved on a
/// per-field basis.
///
/// Mapping vs the H5/M6 spec values:
/// - wall-clock 30s  -> `wall_ms = Some(30_000)`
/// - cpu 30s         -> `cpu_ms  = Some(30_000)`
/// - memory 4 GiB    -> `memory_bytes = Some(4 * 1024 * 1024 * 1024)`
/// - open files 256  -> `file_descriptors = Some(256)`
/// - disk-write 1 GiB-> `output_bytes = Some(1 * 1024 * 1024 * 1024)`
///   (this struct does not carry a dedicated `disk_write_bytes` field; the
///   closest existing dimension is `output_bytes`)
/// - child processes -> not exposed by this struct; not populated here
pub const SAFE_DEFAULT_RESOURCE_CAPS: ResourceCapsV1 = ResourceCapsV1 {
    wall_ms: Some(30_000_u64),
    cpu_ms: Some(30_000_u64),
    memory_bytes: Some(4_u64 * 1024 * 1024 * 1024),
    file_descriptors: Some(256_u32),
    output_bytes: Some(1024_u64 * 1024 * 1024),
};

/// Observed usage at evaluation time. Any field `None` means "not measured".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceUsageV1 {
    pub wall_ms: Option<u64>,
    pub cpu_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub file_descriptors: Option<u32>,
    pub output_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapDimension {
    WallMs,
    CpuMs,
    MemoryBytes,
    FileDescriptors,
    OutputBytes,
}

impl CapDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WallMs => "WALL_MS",
            Self::CpuMs => "CPU_MS",
            Self::MemoryBytes => "MEMORY_BYTES",
            Self::FileDescriptors => "FILE_DESCRIPTORS",
            Self::OutputBytes => "OUTPUT_BYTES",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceOverageEvidenceV1 {
    pub dimension: CapDimension,
    pub observed: u64,
    pub cap: u64,
    pub at_or_over_cap: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceDecision {
    Allow,
    Gate {
        evidence: ResourceOverageEvidenceV1,
    },
    Halt {
        denial: SandboxDenialRecordV1,
        evidence: ResourceOverageEvidenceV1,
    },
}

pub struct ResourceCapEvaluator<'a> {
    caps: &'a ResourceCapsV1,
}

impl<'a> ResourceCapEvaluator<'a> {
    pub fn new(caps: &'a ResourceCapsV1) -> Self {
        Self { caps }
    }

    pub fn evaluate(&self, run: &SandboxRunV1, usage: &ResourceUsageV1) -> ResourceDecision {
        // Check the dimensions in a stable order so decision is deterministic.
        let checks: &[(CapDimension, Option<u64>, Option<u64>)] = &[
            (CapDimension::WallMs, usage.wall_ms, self.caps.wall_ms),
            (CapDimension::CpuMs, usage.cpu_ms, self.caps.cpu_ms),
            (
                CapDimension::MemoryBytes,
                usage.memory_bytes,
                self.caps.memory_bytes,
            ),
            (
                CapDimension::FileDescriptors,
                usage.file_descriptors.map(|n| n as u64),
                self.caps.file_descriptors.map(|n| n as u64),
            ),
            (
                CapDimension::OutputBytes,
                usage.output_bytes,
                self.caps.output_bytes,
            ),
        ];

        // First pass: halt on the first observed >= cap.
        for (dim, observed, cap) in checks.iter().copied() {
            if let (Some(o), Some(c)) = (observed, cap) {
                if o >= c {
                    let evidence = ResourceOverageEvidenceV1 {
                        dimension: dim,
                        observed: o,
                        cap: c,
                        at_or_over_cap: true,
                    };
                    let denial = SandboxDenialRecordV1::new(
                        run.run_id.0.clone(),
                        run.policy_version_id.clone(),
                        DenialKind::PolicyDenied,
                        None,
                        format!(
                            "resource cap {} hit: observed {} >= cap {}",
                            dim.as_str(),
                            o,
                            c
                        ),
                        format!(
                            "deterministic halt: dimension {} observed {} reached cap {}",
                            dim.as_str(),
                            o,
                            c
                        ),
                    );
                    return ResourceDecision::Halt { denial, evidence };
                }
            }
        }

        // Second pass: warn (gate) at >= 80% of cap.
        for (dim, observed, cap) in checks.iter().copied() {
            if let (Some(o), Some(c)) = (observed, cap) {
                let warn = c.saturating_mul(8) / 10;
                if c > 0 && o >= warn {
                    return ResourceDecision::Gate {
                        evidence: ResourceOverageEvidenceV1 {
                            dimension: dim,
                            observed: o,
                            cap: c,
                            at_or_over_cap: false,
                        },
                    };
                }
            }
        }
        ResourceDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "caps", "POL-1@1", "WSP-1")
    }

    #[test]
    fn explicit_none_cap_is_unbounded_by_design() {
        // M6: explicit `None` on a per-field basis is the documented
        // "unbounded by design" signal and MUST keep allowing any observed
        // usage on that dimension. Construct caps with EXPLICIT None values
        // rather than `Default::default()` so the intent is unambiguous.
        let caps = ResourceCapsV1 {
            wall_ms: None,
            cpu_ms: None,
            memory_bytes: None,
            file_descriptors: None,
            output_bytes: None,
        };
        let usage = ResourceUsageV1 {
            wall_ms: Some(9_999_999),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        assert_eq!(dec, ResourceDecision::Allow);
    }

    #[test]
    fn default_deny_bundle_applies_safe_caps() {
        // M6 acceptance: `SandboxPolicyBundleV1::default_deny` must populate
        // every cap dimension via `SAFE_DEFAULT_RESOURCE_CAPS`. The previous
        // behavior left every field at None (unbounded), which contradicted
        // the documented default-deny stance.
        use crate::kernel::sandbox::policy_default_deny::SandboxPolicyBundleV1;
        let bundle = SandboxPolicyBundleV1::default_deny("safe-caps");
        let caps = &bundle.extensions.resource_caps;
        assert_eq!(caps.wall_ms, SAFE_DEFAULT_RESOURCE_CAPS.wall_ms);
        assert_eq!(caps.cpu_ms, SAFE_DEFAULT_RESOURCE_CAPS.cpu_ms);
        assert_eq!(caps.memory_bytes, SAFE_DEFAULT_RESOURCE_CAPS.memory_bytes);
        assert_eq!(
            caps.file_descriptors,
            SAFE_DEFAULT_RESOURCE_CAPS.file_descriptors
        );
        assert_eq!(caps.output_bytes, SAFE_DEFAULT_RESOURCE_CAPS.output_bytes);
        // Every dimension must be Some (bounded).
        assert!(caps.wall_ms.is_some());
        assert!(caps.cpu_ms.is_some());
        assert!(caps.memory_bytes.is_some());
        assert!(caps.file_descriptors.is_some());
        assert!(caps.output_bytes.is_some());
    }

    #[test]
    fn over_wall_ms_cap_halts_with_typed_evidence() {
        let caps = ResourceCapsV1 {
            wall_ms: Some(1000),
            ..Default::default()
        };
        let usage = ResourceUsageV1 {
            wall_ms: Some(1500),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            ResourceDecision::Halt { denial, evidence } => {
                assert_eq!(evidence.dimension, CapDimension::WallMs);
                assert!(evidence.at_or_over_cap);
                assert_eq!(evidence.observed, 1500);
                assert_eq!(evidence.cap, 1000);
                assert_eq!(denial.kind, DenialKind::PolicyDenied);
            }
            other => panic!("expected Halt, got {:?}", other),
        }
    }

    #[test]
    fn under_warn_threshold_allows() {
        let caps = ResourceCapsV1 {
            memory_bytes: Some(1_000_000),
            ..Default::default()
        };
        let usage = ResourceUsageV1 {
            memory_bytes: Some(500_000),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        assert_eq!(dec, ResourceDecision::Allow);
    }

    #[test]
    fn near_cap_gates_with_warning() {
        let caps = ResourceCapsV1 {
            cpu_ms: Some(1000),
            ..Default::default()
        };
        // 850ms >= 80% of 1000 = 800.
        let usage = ResourceUsageV1 {
            cpu_ms: Some(850),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            ResourceDecision::Gate { evidence } => {
                assert_eq!(evidence.dimension, CapDimension::CpuMs);
                assert!(!evidence.at_or_over_cap);
            }
            other => panic!("expected Gate, got {:?}", other),
        }
    }

    #[test]
    fn evaluation_order_is_deterministic() {
        // Both wall_ms and cpu_ms exceed cap; wall_ms is checked first.
        let caps = ResourceCapsV1 {
            wall_ms: Some(100),
            cpu_ms: Some(100),
            ..Default::default()
        };
        let usage = ResourceUsageV1 {
            wall_ms: Some(200),
            cpu_ms: Some(200),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            ResourceDecision::Halt { evidence, .. } => {
                assert_eq!(
                    evidence.dimension,
                    CapDimension::WallMs,
                    "wall_ms is the first checked dimension"
                );
            }
            other => panic!("expected Halt on wall_ms first, got {:?}", other),
        }
    }

    #[test]
    fn fd_overage_halts() {
        let caps = ResourceCapsV1 {
            file_descriptors: Some(64),
            ..Default::default()
        };
        let usage = ResourceUsageV1 {
            file_descriptors: Some(80),
            ..Default::default()
        };
        let dec = ResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            ResourceDecision::Halt { evidence, .. } => {
                assert_eq!(evidence.dimension, CapDimension::FileDescriptors);
                assert_eq!(evidence.observed, 80);
            }
            other => panic!("expected Halt, got {:?}", other),
        }
    }
}
