//! Validation descriptor trait + allowlist registry.
//!
//! A `ValidationDescriptor` is the *definition* of a deterministic check the
//! validation runner can evaluate against a candidate. Descriptors are
//! registered into an allowlist; the runner refuses to execute any descriptor
//! whose `name()` is not present in the allowlist. This is what prevents an
//! ad-hoc check from sneaking into a run.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::status::ValidationStatus;

/// Static intent classification of a descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DescriptorKind {
    /// Decides promotion-gate outcome (blocks on Fail/Blocked/Error).
    Gating,
    /// Informational only (cannot block promotion even on FAIL).
    Advisory,
}

/// Definition of a deterministic check.
pub trait ValidationDescriptor: Send + Sync {
    /// Stable identifier, e.g. `"no_sandbox_escape"`, `"artifact_hashes_valid"`.
    fn name(&self) -> &'static str;
    /// Gating vs advisory classification.
    fn kind(&self) -> DescriptorKind;
    /// Evaluate against an opaque candidate handle. Implementations return a
    /// typed `ValidationStatus`. Panics are caller-trapped; this trait does
    /// not unwind.
    fn evaluate(&self, candidate: &dyn DescriptorInput) -> ValidationStatus;
}

/// Opaque handle the runner passes into a descriptor. Real production
/// descriptors will downcast to a richer candidate model; for MT-scope the
/// trait surface is intentionally minimal so descriptors stay testable.
pub trait DescriptorInput {
    /// Implementations can stash structured context behind this method. The
    /// default implementation returns `None` so the trait is object-safe and
    /// trivially mockable in tests.
    fn lookup(&self, _key: &str) -> Option<&str> {
        None
    }
}

/// Allowlist of descriptor names the runner is permitted to invoke.
#[derive(Debug, Clone, Default)]
pub struct DescriptorAllowlist {
    names: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorAdmissionError {
    NotInAllowlist { name: String },
}

impl std::fmt::Display for DescriptorAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInAllowlist { name } => {
                write!(f, "descriptor '{name}' is not in the allowlist")
            }
        }
    }
}

impl std::error::Error for DescriptorAdmissionError {}

impl DescriptorAllowlist {
    pub fn new<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            names: iter.into_iter().map(Into::into).collect(),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Admit a descriptor if its `name()` is in the allowlist, else return a
    /// typed admission error.
    pub fn admit<'d>(
        &self,
        descriptor: &'d dyn ValidationDescriptor,
    ) -> Result<&'d dyn ValidationDescriptor, DescriptorAdmissionError> {
        if self.contains(descriptor.name()) {
            Ok(descriptor)
        } else {
            Err(DescriptorAdmissionError::NotInAllowlist {
                name: descriptor.name().to_string(),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Concrete descriptors used by the MVP runner.
// ---------------------------------------------------------------------------

/// Descriptor: candidate produced no sandbox-escape evidence.
pub struct NoSandboxEscape;
impl ValidationDescriptor for NoSandboxEscape {
    fn name(&self) -> &'static str {
        "no_sandbox_escape"
    }
    fn kind(&self) -> DescriptorKind {
        DescriptorKind::Gating
    }
    fn evaluate(&self, candidate: &dyn DescriptorInput) -> ValidationStatus {
        match candidate.lookup("sandbox_escape_evidence") {
            None | Some("") => ValidationStatus::pass(),
            Some(reason) => ValidationStatus::fail(format!("sandbox escape evidence: {reason}"))
                .expect("non-empty reason"),
        }
    }
}

/// Descriptor: declared artifact hashes are valid (present + correct shape).
pub struct ArtifactHashesValid;
impl ValidationDescriptor for ArtifactHashesValid {
    fn name(&self) -> &'static str {
        "artifact_hashes_valid"
    }
    fn kind(&self) -> DescriptorKind {
        DescriptorKind::Gating
    }
    fn evaluate(&self, candidate: &dyn DescriptorInput) -> ValidationStatus {
        match candidate.lookup("artifact_hashes") {
            None => ValidationStatus::blocked("no artifact_hashes recorded for candidate")
                .expect("non-empty reason"),
            Some(value) if value.len() < 16 => {
                ValidationStatus::fail("artifact_hashes payload truncated/invalid")
                    .expect("non-empty reason")
            }
            Some(_) => ValidationStatus::pass(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapInput(HashMap<&'static str, String>);
    impl DescriptorInput for MapInput {
        fn lookup(&self, key: &str) -> Option<&str> {
            self.0.get(key).map(|s| s.as_str())
        }
    }

    fn input(pairs: &[(&'static str, &str)]) -> MapInput {
        MapInput(
            pairs
                .iter()
                .map(|(k, v)| (*k, (*v).to_string()))
                .collect(),
        )
    }

    #[test]
    fn allowlist_rejects_unknown_descriptor() {
        let allow = DescriptorAllowlist::new(["no_sandbox_escape"]);
        let d = NoSandboxEscape;
        assert!(allow.admit(&d).is_ok());

        let d2 = ArtifactHashesValid;
        let err = allow.admit(&d2).unwrap_err();
        assert_eq!(
            err,
            DescriptorAdmissionError::NotInAllowlist {
                name: "artifact_hashes_valid".to_string()
            }
        );
    }

    #[test]
    fn no_sandbox_escape_fail_carries_reason() {
        let d = NoSandboxEscape;
        let candidate = input(&[("sandbox_escape_evidence", "wrote outside workspace root")]);
        let status = d.evaluate(&candidate);
        match status {
            ValidationStatus::Fail { reason } => {
                assert!(reason.contains("wrote outside workspace root"));
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    #[test]
    fn artifact_hashes_valid_blocked_when_missing() {
        let d = ArtifactHashesValid;
        let candidate = input(&[]);
        assert!(matches!(
            d.evaluate(&candidate),
            ValidationStatus::Blocked { .. }
        ));
    }
}
