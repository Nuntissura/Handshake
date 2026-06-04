//! MT-032: Candidate range truth.
//!
//! Acceptance: unexpected file edits are rejected before promotion. Given a
//! `PatchProposal` (declared targets) and a list of `ObservedEdit` records
//! (what the candidate actually changed), `verify_candidate_ranges` rejects
//! any edit whose path is not in the declared targets or whose line range
//! lies outside the declared range for that path.

use serde::{Deserialize, Serialize};

use super::patch_proposal::PatchProposal;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedEdit {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

impl ObservedEdit {
    pub fn new(path: impl Into<String>, start_line: u32, end_line: u32) -> Self {
        Self {
            path: path.into(),
            start_line,
            end_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeViolation {
    UndeclaredPath { path: String },
    OutOfDeclaredRange { path: String, start: u32, end: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RangeVerification {
    pub violations: Vec<RangeViolation>,
}

impl RangeVerification {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Verify every observed edit lies inside the declared target ranges of the
/// proposal. Returns the (possibly empty) list of violations; callers reject
/// the candidate when `!result.is_clean()`.
pub fn verify_candidate_ranges(
    proposal: &PatchProposal,
    observed: &[ObservedEdit],
) -> RangeVerification {
    let mut violations = Vec::new();
    for edit in observed {
        // Find any declared range matching this path.
        let path_declared = proposal.target_ranges.iter().any(|r| r.path == edit.path);
        if !path_declared {
            violations.push(RangeViolation::UndeclaredPath {
                path: edit.path.clone(),
            });
            continue;
        }
        // For declared paths, the entire observed range must lie inside at
        // least one declared range.
        let inside = proposal.target_ranges.iter().any(|r| {
            r.path == edit.path && edit.start_line >= r.start_line && edit.end_line <= r.end_line
        });
        if !inside {
            violations.push(RangeViolation::OutOfDeclaredRange {
                path: edit.path.clone(),
                start: edit.start_line,
                end: edit.end_line,
            });
        }
    }
    RangeVerification { violations }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::validation::patch_proposal::TargetRange;

    fn proposal() -> PatchProposal {
        PatchProposal::new(
            "p1",
            "main",
            vec![
                TargetRange::new("src/a.rs", 10, 20),
                TargetRange::new("src/b.rs", 1, 50),
            ],
        )
        .unwrap()
    }

    #[test]
    fn unexpected_path_rejected() {
        let v = verify_candidate_ranges(&proposal(), &[ObservedEdit::new("src/c.rs", 1, 5)]);
        assert!(!v.is_clean());
        assert!(matches!(
            v.violations[0],
            RangeViolation::UndeclaredPath { .. }
        ));
    }

    #[test]
    fn out_of_declared_range_rejected() {
        let v = verify_candidate_ranges(&proposal(), &[ObservedEdit::new("src/a.rs", 5, 25)]);
        assert!(!v.is_clean());
        assert!(matches!(
            v.violations[0],
            RangeViolation::OutOfDeclaredRange { .. }
        ));
    }

    #[test]
    fn in_range_edits_pass_clean() {
        let v = verify_candidate_ranges(
            &proposal(),
            &[
                ObservedEdit::new("src/a.rs", 12, 18),
                ObservedEdit::new("src/b.rs", 1, 50),
            ],
        );
        assert!(v.is_clean());
    }
}
