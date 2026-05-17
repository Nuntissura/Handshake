//! Validation status taxonomy.
//!
//! WP-KERNEL-003 packet acceptance requires every validation outcome to carry
//! one of seven typed verdicts:
//!
//! - `Pass`                  — descriptor evaluated; criterion satisfied.
//! - `Fail { reason }`       — descriptor evaluated; criterion violated. The
//!   reason is non-empty by construction (constructor enforces it).
//! - `Blocked { reason }`    — descriptor could not evaluate because a
//!   prerequisite was absent (missing artifact, missing adapter, missing
//!   policy decision). Distinct from `Error` (which is a defect).
//! - `AdvisoryOnly { note }` — descriptor is informational; never blocks
//!   promotion. Carries the advisory note for surfaces.
//! - `Unsupported { adapter }` — declared adapter (sandbox tier, language
//!   plugin, browser engine, etc.) is not available in this build/host.
//!   Carries the adapter name so reports can render it.
//! - `SkippedWithReason { reason }` — operator/policy explicitly skipped the
//!   descriptor (feature flag, scope narrowing). Distinct from `Unsupported`.
//! - `Error { code, detail }` — descriptor evaluator itself failed
//!   (panic-trapped, IO error, schema mismatch). Promotion treats this as a
//!   hard block by default.
//!
//! All payload-carrying variants are constructed via helpers that reject
//! empty strings; this is what the packet acceptance "FAIL status carries a
//! reason / UNSUPPORTED carries adapter name" rule depends on.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    Pass,
    Fail { reason: String },
    Blocked { reason: String },
    AdvisoryOnly { note: String },
    Unsupported { adapter: String },
    SkippedWithReason { reason: String },
    Error { code: String, detail: String },
}

/// Lightweight error returned by status constructors when callers attempt to
/// build a payload-carrying variant with empty payload fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBuildError(pub &'static str);

impl std::fmt::Display for StatusBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "validation status build error: {}", self.0)
    }
}

impl std::error::Error for StatusBuildError {}

impl ValidationStatus {
    pub fn pass() -> Self {
        Self::Pass
    }

    pub fn fail(reason: impl Into<String>) -> Result<Self, StatusBuildError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(StatusBuildError("Fail.reason must not be empty"));
        }
        Ok(Self::Fail { reason })
    }

    pub fn blocked(reason: impl Into<String>) -> Result<Self, StatusBuildError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(StatusBuildError("Blocked.reason must not be empty"));
        }
        Ok(Self::Blocked { reason })
    }

    pub fn advisory(note: impl Into<String>) -> Result<Self, StatusBuildError> {
        let note = note.into();
        if note.trim().is_empty() {
            return Err(StatusBuildError("AdvisoryOnly.note must not be empty"));
        }
        Ok(Self::AdvisoryOnly { note })
    }

    pub fn unsupported(adapter: impl Into<String>) -> Result<Self, StatusBuildError> {
        let adapter = adapter.into();
        if adapter.trim().is_empty() {
            return Err(StatusBuildError("Unsupported.adapter must not be empty"));
        }
        Ok(Self::Unsupported { adapter })
    }

    pub fn skipped(reason: impl Into<String>) -> Result<Self, StatusBuildError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(StatusBuildError(
                "SkippedWithReason.reason must not be empty",
            ));
        }
        Ok(Self::SkippedWithReason { reason })
    }

    pub fn error(code: impl Into<String>, detail: impl Into<String>) -> Result<Self, StatusBuildError> {
        let code = code.into();
        let detail = detail.into();
        if code.trim().is_empty() {
            return Err(StatusBuildError("Error.code must not be empty"));
        }
        if detail.trim().is_empty() {
            return Err(StatusBuildError("Error.detail must not be empty"));
        }
        Ok(Self::Error { code, detail })
    }

    /// Promotion-gate view: does this status, by default policy, block the
    /// promotion gate from accepting the candidate?
    pub fn blocks_promotion(&self) -> bool {
        matches!(
            self,
            Self::Fail { .. } | Self::Blocked { .. } | Self::Error { .. }
        )
    }

    /// Short tag used in receipts/projections.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail { .. } => "FAIL",
            Self::Blocked { .. } => "BLOCKED",
            Self::AdvisoryOnly { .. } => "ADVISORY_ONLY",
            Self::Unsupported { .. } => "UNSUPPORTED",
            Self::SkippedWithReason { .. } => "SKIPPED_WITH_REASON",
            Self::Error { .. } => "ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seven_variants_tag_distinctly() {
        let s = vec![
            ValidationStatus::pass(),
            ValidationStatus::fail("descriptor X violated").unwrap(),
            ValidationStatus::blocked("missing adapter").unwrap(),
            ValidationStatus::advisory("informational only").unwrap(),
            ValidationStatus::unsupported("HardIsolation").unwrap(),
            ValidationStatus::skipped("operator scope-narrowing").unwrap(),
            ValidationStatus::error("E_IO", "could not read artifact").unwrap(),
        ];
        let tags: Vec<&str> = s.iter().map(|x| x.tag()).collect();
        assert_eq!(
            tags,
            vec![
                "PASS",
                "FAIL",
                "BLOCKED",
                "ADVISORY_ONLY",
                "UNSUPPORTED",
                "SKIPPED_WITH_REASON",
                "ERROR",
            ]
        );
    }

    #[test]
    fn fail_rejects_empty_reason() {
        assert!(ValidationStatus::fail("").is_err());
        assert!(ValidationStatus::fail("   ").is_err());
        // Non-empty reason succeeds and is carried verbatim.
        let s = ValidationStatus::fail("hash mismatch").unwrap();
        if let ValidationStatus::Fail { reason } = s {
            assert_eq!(reason, "hash mismatch");
        } else {
            panic!("expected Fail");
        }
    }

    #[test]
    fn unsupported_carries_adapter_name() {
        assert!(ValidationStatus::unsupported("").is_err());
        let s = ValidationStatus::unsupported("Firecracker").unwrap();
        if let ValidationStatus::Unsupported { adapter } = s {
            assert_eq!(adapter, "Firecracker");
        } else {
            panic!("expected Unsupported");
        }
    }

    #[test]
    fn blocks_promotion_classifies_correctly() {
        assert!(!ValidationStatus::pass().blocks_promotion());
        assert!(ValidationStatus::fail("x").unwrap().blocks_promotion());
        assert!(ValidationStatus::blocked("y").unwrap().blocks_promotion());
        assert!(ValidationStatus::error("E", "d").unwrap().blocks_promotion());
        assert!(!ValidationStatus::advisory("note").unwrap().blocks_promotion());
        assert!(!ValidationStatus::unsupported("a").unwrap().blocks_promotion());
        assert!(!ValidationStatus::skipped("r").unwrap().blocks_promotion());
    }

    #[test]
    fn serde_round_trip_keeps_taxonomy_stable() {
        let s = ValidationStatus::unsupported("HardIsolation").unwrap();
        let j = serde_json::to_string(&s).unwrap();
        // tagged enum: kind="UNSUPPORTED", adapter="HardIsolation"
        assert!(j.contains("\"kind\":\"UNSUPPORTED\""), "got {j}");
        assert!(j.contains("\"adapter\":\"HardIsolation\""), "got {j}");
        let back: ValidationStatus = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
