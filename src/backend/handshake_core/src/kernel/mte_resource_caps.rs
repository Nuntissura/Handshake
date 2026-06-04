//! MT-059: MTE Run Cap Integration.
//!
//! Acceptance (MT-059.json): "wire resource caps into sandboxed microtask
//! execution. Acceptance: cap overage halts bounded run and writes evidence."
//!
//! `MteResourceCapsV1` is the MTE-layer caps contract. It is a thin wrapper
//! over the sandbox-side `ResourceCapsV1` (Batch B,
//! `kernel/sandbox/policy_default_deny.rs` + `kernel/sandbox/resource_caps.rs`)
//! that records *per-MT* caps rather than per-sandbox-policy caps. The MTE
//! scheduler can override the policy default for a single MT (e.g. allow a
//! migration MT more wall-time than the lane default) and the resulting
//! per-MT caps are evaluated through `ResourceCapEvaluator` so the
//! deterministic Allow / Gate / Halt semantics defined in MT-026 are
//! preserved.
//!
//! Hand-off: the per-MT caps record can be folded into the per-MT summary's
//! `evidence_refs` so the closeout bundle (MT-058) carries the exact caps
//! the MT ran under.

use serde::{Deserialize, Serialize};

use crate::kernel::sandbox::policy_default_deny::ResourceCapsV1;
use crate::kernel::sandbox::resource_caps::{
    CapDimension, ResourceCapEvaluator, ResourceDecision, ResourceOverageEvidenceV1,
    ResourceUsageV1,
};
use crate::kernel::sandbox::run::SandboxRunV1;

/// Per-MT resource caps. Maps directly onto the sandbox-side `ResourceCapsV1`
/// shape but carries an `mt_id` so the MTE scheduler can attribute the cap
/// set to the MT that ran under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MteResourceCapsV1 {
    pub mt_id: String,
    pub cpu_ms: Option<u64>,
    pub wall_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub output_bytes: Option<u64>,
    pub file_descriptors: Option<u32>,
    /// M-C3: tracked but NOT honored by the sandbox layer yet.
    /// `to_sandbox_caps()` discards this value. Future MT will route it
    /// through a process-spawn limiter at the adapter boundary.
    pub child_processes: Option<u32>,
}

impl MteResourceCapsV1 {
    pub fn for_mt(mt_id: impl Into<String>) -> Self {
        Self {
            mt_id: mt_id.into(),
            ..Default::default()
        }
    }

    pub fn with_cpu_ms(mut self, cpu_ms: u64) -> Self {
        self.cpu_ms = Some(cpu_ms);
        self
    }
    pub fn with_wall_ms(mut self, wall_ms: u64) -> Self {
        self.wall_ms = Some(wall_ms);
        self
    }
    pub fn with_memory_bytes(mut self, memory_bytes: u64) -> Self {
        self.memory_bytes = Some(memory_bytes);
        self
    }
    pub fn with_output_bytes(mut self, output_bytes: u64) -> Self {
        self.output_bytes = Some(output_bytes);
        self
    }
    pub fn with_file_descriptors(mut self, fds: u32) -> Self {
        self.file_descriptors = Some(fds);
        self
    }
    /// M-C3: setter exists for forward-compat but the value is dropped by
    /// `to_sandbox_caps()` until the sandbox adapter learns to enforce it.
    /// Callers SHOULD treat a non-None value as advisory, not enforceable.
    pub fn with_child_processes(mut self, n: u32) -> Self {
        self.child_processes = Some(n);
        self
    }

    /// Convert to the sandbox-side cap shape so the existing
    /// `ResourceCapEvaluator` (MT-026) does the actual evaluation.
    pub fn to_sandbox_caps(&self) -> ResourceCapsV1 {
        ResourceCapsV1 {
            wall_ms: self.wall_ms,
            cpu_ms: self.cpu_ms,
            memory_bytes: self.memory_bytes,
            file_descriptors: self.file_descriptors,
            output_bytes: self.output_bytes,
        }
    }
}

/// Verdict produced by the MTE cap evaluator. Mirrors `ResourceDecision` but
/// carries the per-MT identity so the receipt sink can attribute halts to
/// the MT, not just to the sandbox run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MteResourceDecision {
    /// Usage is well under every cap.
    Allow,
    /// Usage is at or above the warn threshold (80%) for some cap.
    Throttle {
        mt_id: String,
        evidence: ResourceOverageEvidenceV1,
    },
    /// Usage met or exceeded a cap; run is halted and a typed evidence record
    /// is emitted. The MTE-layer evidence carries the MT id; the sandbox-layer
    /// denial record is captured separately when callers need it.
    Halt {
        mt_id: String,
        evidence: ResourceOverageEvidenceV1,
    },
}

impl MteResourceDecision {
    pub fn is_halt(&self) -> bool {
        matches!(self, Self::Halt { .. })
    }
    pub fn is_throttle(&self) -> bool {
        matches!(self, Self::Throttle { .. })
    }

    pub fn evidence(&self) -> Option<&ResourceOverageEvidenceV1> {
        match self {
            Self::Allow => None,
            Self::Throttle { evidence, .. } | Self::Halt { evidence, .. } => Some(evidence),
        }
    }
}

pub struct MteResourceCapEvaluator<'a> {
    caps: &'a MteResourceCapsV1,
}

impl<'a> MteResourceCapEvaluator<'a> {
    pub fn new(caps: &'a MteResourceCapsV1) -> Self {
        Self { caps }
    }

    /// Evaluate usage against caps. Delegates to the sandbox-side
    /// `ResourceCapEvaluator` (MT-026) so the deterministic order and
    /// evidence shape stay identical across both layers.
    pub fn evaluate(&self, run: &SandboxRunV1, usage: &ResourceUsageV1) -> MteResourceDecision {
        let sandbox_caps = self.caps.to_sandbox_caps();
        let inner = ResourceCapEvaluator::new(&sandbox_caps).evaluate(run, usage);
        match inner {
            ResourceDecision::Allow => MteResourceDecision::Allow,
            ResourceDecision::Gate { evidence } => MteResourceDecision::Throttle {
                mt_id: self.caps.mt_id.clone(),
                evidence,
            },
            ResourceDecision::Halt { evidence, .. } => MteResourceDecision::Halt {
                mt_id: self.caps.mt_id.clone(),
                evidence,
            },
        }
    }

    /// Convenience: name the dimension that fired (used in receipt rationale).
    pub fn dimension_name(d: CapDimension) -> &'static str {
        d.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "process_tier", "POL-1@1", "WSP-1")
    }

    // MT-059 acceptance: cap overage halts and writes evidence.
    #[test]
    fn wall_ms_overage_halts_with_mt_attribution() {
        let caps = MteResourceCapsV1::for_mt("MT-7").with_wall_ms(1000);
        let usage = ResourceUsageV1 {
            wall_ms: Some(1500),
            ..Default::default()
        };
        let dec = MteResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            MteResourceDecision::Halt { mt_id, evidence } => {
                assert_eq!(mt_id, "MT-7");
                assert_eq!(evidence.dimension, CapDimension::WallMs);
                assert!(evidence.at_or_over_cap);
            }
            other => panic!("expected Halt, got {other:?}"),
        }
    }

    #[test]
    fn under_warn_threshold_allows() {
        let caps = MteResourceCapsV1::for_mt("MT-1").with_memory_bytes(1_000_000);
        let usage = ResourceUsageV1 {
            memory_bytes: Some(100_000),
            ..Default::default()
        };
        let dec = MteResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        assert_eq!(dec, MteResourceDecision::Allow);
    }

    #[test]
    fn near_cap_throttles() {
        let caps = MteResourceCapsV1::for_mt("MT-2").with_cpu_ms(1000);
        let usage = ResourceUsageV1 {
            cpu_ms: Some(850),
            ..Default::default()
        };
        let dec = MteResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        match dec {
            MteResourceDecision::Throttle { mt_id, evidence } => {
                assert_eq!(mt_id, "MT-2");
                assert!(!evidence.at_or_over_cap);
            }
            other => panic!("expected Throttle, got {other:?}"),
        }
    }

    #[test]
    fn no_caps_allows_anything() {
        let caps = MteResourceCapsV1::for_mt("MT-3");
        let usage = ResourceUsageV1 {
            wall_ms: Some(9_999_999_999),
            ..Default::default()
        };
        let dec = MteResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        assert_eq!(dec, MteResourceDecision::Allow);
    }

    #[test]
    fn evidence_includes_observed_and_cap() {
        let caps = MteResourceCapsV1::for_mt("MT-x").with_output_bytes(1024);
        let usage = ResourceUsageV1 {
            output_bytes: Some(2048),
            ..Default::default()
        };
        let dec = MteResourceCapEvaluator::new(&caps).evaluate(&run(), &usage);
        let ev = dec.evidence().unwrap();
        assert_eq!(ev.observed, 2048);
        assert_eq!(ev.cap, 1024);
        assert_eq!(ev.dimension, CapDimension::OutputBytes);
    }

    #[test]
    fn builder_chains_compose() {
        let caps = MteResourceCapsV1::for_mt("MT-9")
            .with_cpu_ms(500)
            .with_wall_ms(2000)
            .with_memory_bytes(64 * 1024 * 1024)
            .with_output_bytes(1024 * 1024)
            .with_file_descriptors(128)
            .with_child_processes(4);
        let sandbox = caps.to_sandbox_caps();
        assert_eq!(sandbox.cpu_ms, Some(500));
        assert_eq!(sandbox.wall_ms, Some(2000));
        assert_eq!(sandbox.memory_bytes, Some(64 * 1024 * 1024));
        assert_eq!(sandbox.output_bytes, Some(1024 * 1024));
        assert_eq!(sandbox.file_descriptors, Some(128));
        // child_processes is MTE-layer extension, not in sandbox caps yet.
        assert_eq!(caps.child_processes, Some(4));
    }
}
