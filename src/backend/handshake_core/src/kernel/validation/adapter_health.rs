//! MT-030: Sandbox adapter health projection.
//!
//! Acceptance: unsupported isolation is visible **before** a validation/sandbox
//! run launches. This projection is consumed by the validation pre-flight
//! (and any operator-facing surface) so a missing HardIsolation tier surfaces
//! as `Unsupported { adapter }` rather than silent fallback or runtime failure.
//!
//! The projection is intentionally read-only and adapter-agnostic: it does
//! not import the sandbox adapter modules (Batch B), it consumes a small
//! `AdapterHealthReport` value that adapter implementations produce. This
//! keeps validation independent of which adapter tiers are wired up.

use serde::{Deserialize, Serialize};

use super::status::ValidationStatus;

/// Health of one declared sandbox adapter tier (process / hard-isolation / ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterHealth {
    /// Tier / adapter name, e.g. "process", "hard_isolation_docker",
    /// "hard_isolation_firecracker".
    pub adapter: String,
    /// Whether the host actually supports this adapter right now.
    pub available: bool,
    /// Optional human-readable preflight detail (why unavailable, version, etc).
    pub detail: Option<String>,
}

impl AdapterHealth {
    pub fn available(adapter: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            available: true,
            detail: None,
        }
    }

    pub fn unsupported(adapter: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            available: false,
            detail: Some(detail.into()),
        }
    }
}

/// Full preflight projection across declared adapter tiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdapterHealthReport {
    pub tiers: Vec<AdapterHealth>,
}

impl AdapterHealthReport {
    pub fn new(tiers: Vec<AdapterHealth>) -> Self {
        Self { tiers }
    }

    /// Lookup a tier by name.
    pub fn tier(&self, adapter: &str) -> Option<&AdapterHealth> {
        self.tiers.iter().find(|t| t.adapter == adapter)
    }

    /// Project the health of a declared adapter into a `ValidationStatus`
    /// suitable for the pre-flight gate.
    ///
    /// - Available    -> `Pass`
    /// - Unavailable  -> `Unsupported { adapter }`
    /// - Not declared -> `Blocked { reason }` (we don't know the host stance)
    pub fn project_for(&self, adapter: &str) -> ValidationStatus {
        match self.tier(adapter) {
            Some(t) if t.available => ValidationStatus::pass(),
            Some(_) => ValidationStatus::unsupported(adapter)
                .expect("adapter name from declared tiers is non-empty"),
            None => ValidationStatus::blocked(format!(
                "adapter '{adapter}' not declared in preflight projection"
            ))
            .expect("non-empty reason"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_is_visible_before_run() {
        // Process tier present; hard-isolation tier missing.
        let report = AdapterHealthReport::new(vec![
            AdapterHealth::available("process"),
            AdapterHealth::unsupported("hard_isolation_firecracker", "binary not installed"),
        ]);
        let process = report.project_for("process");
        let hard = report.project_for("hard_isolation_firecracker");
        assert!(matches!(process, ValidationStatus::Pass));
        match hard {
            ValidationStatus::Unsupported { adapter } => {
                assert_eq!(adapter, "hard_isolation_firecracker");
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn undeclared_adapter_is_blocked_not_silent_pass() {
        let report = AdapterHealthReport::new(vec![AdapterHealth::available("process")]);
        let status = report.project_for("kvm_microvm");
        assert!(matches!(status, ValidationStatus::Blocked { .. }));
    }
}
