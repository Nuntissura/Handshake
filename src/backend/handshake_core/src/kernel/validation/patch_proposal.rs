//! MT-031: PatchProposal contract.
//!
//! Acceptance: proposals without base refs or target ranges cannot enter
//! validation. The constructor returns a typed error in both cases so the
//! runner never sees an under-specified candidate.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRange {
    /// Workspace-relative path the proposal claims to edit.
    pub path: String,
    /// 1-based inclusive start line.
    pub start_line: u32,
    /// 1-based inclusive end line.
    pub end_line: u32,
}

impl TargetRange {
    pub fn new(path: impl Into<String>, start_line: u32, end_line: u32) -> Self {
        Self {
            path: path.into(),
            start_line,
            end_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposal {
    pub proposal_id: String,
    pub base_ref: String,
    pub target_ranges: Vec<TargetRange>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchProposalError {
    EmptyProposalId,
    EmptyBaseRef,
    NoTargetRanges,
    InvalidRange { path: String, start: u32, end: u32 },
    EmptyTargetPath,
}

impl std::fmt::Display for PatchProposalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProposalId => write!(f, "PatchProposal.proposal_id must not be empty"),
            Self::EmptyBaseRef => write!(f, "PatchProposal.base_ref must not be empty"),
            Self::NoTargetRanges => {
                write!(f, "PatchProposal.target_ranges must contain at least one range")
            }
            Self::InvalidRange { path, start, end } => write!(
                f,
                "PatchProposal target_range for '{path}' has invalid bounds {start}..{end}"
            ),
            Self::EmptyTargetPath => write!(f, "PatchProposal target_range.path must not be empty"),
        }
    }
}

impl std::error::Error for PatchProposalError {}

impl PatchProposal {
    pub fn new(
        proposal_id: impl Into<String>,
        base_ref: impl Into<String>,
        target_ranges: Vec<TargetRange>,
    ) -> Result<Self, PatchProposalError> {
        let proposal_id = proposal_id.into();
        if proposal_id.trim().is_empty() {
            return Err(PatchProposalError::EmptyProposalId);
        }
        let base_ref = base_ref.into();
        if base_ref.trim().is_empty() {
            return Err(PatchProposalError::EmptyBaseRef);
        }
        if target_ranges.is_empty() {
            return Err(PatchProposalError::NoTargetRanges);
        }
        for r in &target_ranges {
            if r.path.trim().is_empty() {
                return Err(PatchProposalError::EmptyTargetPath);
            }
            if r.start_line == 0 || r.end_line < r.start_line {
                return Err(PatchProposalError::InvalidRange {
                    path: r.path.clone(),
                    start: r.start_line,
                    end: r.end_line,
                });
            }
        }
        Ok(Self {
            proposal_id,
            base_ref,
            target_ranges,
            summary: None,
        })
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn covers(&self, path: &str, line: u32) -> bool {
        self.target_ranges
            .iter()
            .any(|r| r.path == path && line >= r.start_line && line <= r.end_line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_base_ref_rejected() {
        let r = PatchProposal::new("p1", "", vec![TargetRange::new("a.rs", 1, 5)]);
        assert_eq!(r.unwrap_err(), PatchProposalError::EmptyBaseRef);
    }

    #[test]
    fn missing_target_ranges_rejected() {
        let r = PatchProposal::new("p1", "main", vec![]);
        assert_eq!(r.unwrap_err(), PatchProposalError::NoTargetRanges);
    }

    #[test]
    fn invalid_range_rejected() {
        let r = PatchProposal::new("p1", "main", vec![TargetRange::new("a.rs", 0, 5)]);
        assert!(matches!(r.unwrap_err(), PatchProposalError::InvalidRange { .. }));

        let r = PatchProposal::new("p1", "main", vec![TargetRange::new("a.rs", 5, 4)]);
        assert!(matches!(r.unwrap_err(), PatchProposalError::InvalidRange { .. }));
    }

    #[test]
    fn well_formed_proposal_accepted_and_covers_query_works() {
        let p = PatchProposal::new(
            "p1",
            "abc123",
            vec![TargetRange::new("src/a.rs", 10, 20)],
        )
        .unwrap();
        assert!(p.covers("src/a.rs", 15));
        assert!(!p.covers("src/a.rs", 21));
        assert!(!p.covers("src/b.rs", 15));
    }
}
