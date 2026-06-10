//! WP-KERNEL-005 MT-191 / MT-192 / MT-193: Flight Recorder workflow event
//! kinds for the model-workflow diagnostics (DCC) surface.
//!
//! Defines the typed FR-EVT-* event kinds that WP-KERNEL-005 adds on top of
//! the WP-KERNEL-004 registry ([`super::fr_event_registry::FrEventId`]):
//!
//!  * MT-191 -- tool calls, proposals, apply decisions
//!    (`FR-EVT-TOOL-CALL`, `FR-EVT-TOOL-PROPOSAL`,
//!    `FR-EVT-TOOL-APPLY-DECISION`).
//!  * MT-192 -- visual capture, validation, recovery
//!    (`FR-EVT-VISUAL-CAPTURE`, `FR-EVT-VISUAL-VALIDATION`,
//!    `FR-EVT-VISUAL-RECOVERY`).
//!  * MT-193 -- build/package guard and stale-doc detection
//!    (`FR-EVT-BUILD-GUARD`, `FR-EVT-PACKAGE-GUARD`,
//!    `FR-EVT-STALE-DOC-DETECTED`).
//!
//! These kinds intentionally live in their own WP-KERNEL-005 namespace rather
//! than extending [`super::fr_event_registry::FrEventId`]: that enum is
//! byte-locked to the WP-KERNEL-004 governance manifest
//! (`FR_EVENT_REGISTRY.json`) by the MT-198 alignment test, and governance
//! files are read-only for product microtasks. The persistence half (typed
//! PostgreSQL rows + EventLedger emission) lives in
//! [`crate::atelier::dcc_flight_recorder`].
//!
//! Canonical-string rules mirror the MT-198 registry: `UPPER-KEBAB-CASE`
//! after the `FR-EVT-` prefix, exact-match lookups (no case folding, no
//! whitespace tolerance), and a typed [`UnknownWorkflowEventKind`] error for
//! anything unregistered.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Work-packet identifier responsible for landing these event kinds.
pub const FR_WORKFLOW_EVENT_KINDS_ADDED_IN_WP: &str = "WP-KERNEL-005";

/// Exhaustive enum of the WP-KERNEL-005 Flight Recorder workflow event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrWorkflowEventKind {
    // ----- MT-191: tool calls / proposals / apply decisions -----
    /// A tool call was executed on behalf of a model session.
    ToolCall,
    /// A tool proposed a change (diff/plan) awaiting an apply decision.
    ToolProposal,
    /// An apply decision (approved / rejected / deferred) was taken on a
    /// proposal.
    ToolApplyDecision,
    // ----- MT-192: visual capture / validation / recovery -----
    /// A visual capture (screenshot/snapshot) was produced as evidence.
    VisualCapture,
    /// A visual validation verdict was recorded over a capture.
    VisualValidation,
    /// A visual-loop recovery action was taken after a failed validation.
    VisualRecovery,
    // ----- MT-193: build/package guard + stale-doc detection -----
    /// A build guard verdict was recorded for a build run.
    BuildGuard,
    /// A package guard verdict was recorded for a packaging run.
    PackageGuard,
    /// A stale generated-doc surface was detected.
    StaleDocDetected,
}

impl FrWorkflowEventKind {
    /// Canonical string id. Stable wire format; never change after a variant
    /// ships -- add a new variant instead.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolCall => "FR-EVT-TOOL-CALL",
            Self::ToolProposal => "FR-EVT-TOOL-PROPOSAL",
            Self::ToolApplyDecision => "FR-EVT-TOOL-APPLY-DECISION",
            Self::VisualCapture => "FR-EVT-VISUAL-CAPTURE",
            Self::VisualValidation => "FR-EVT-VISUAL-VALIDATION",
            Self::VisualRecovery => "FR-EVT-VISUAL-RECOVERY",
            Self::BuildGuard => "FR-EVT-BUILD-GUARD",
            Self::PackageGuard => "FR-EVT-PACKAGE-GUARD",
            Self::StaleDocDetected => "FR-EVT-STALE-DOC-DETECTED",
        }
    }

    /// Ordered slice of every registered variant.
    pub fn all() -> &'static [FrWorkflowEventKind] {
        &[
            Self::ToolCall,
            Self::ToolProposal,
            Self::ToolApplyDecision,
            Self::VisualCapture,
            Self::VisualValidation,
            Self::VisualRecovery,
            Self::BuildGuard,
            Self::PackageGuard,
            Self::StaleDocDetected,
        ]
    }

    /// Reverse lookup: canonical id string -> typed variant. Exact match
    /// only; lower-case, missing prefix, and padded ids are rejected.
    pub fn from_str_id(s: &str) -> Result<Self, UnknownWorkflowEventKind> {
        for &kind in Self::all() {
            if kind.as_str() == s {
                return Ok(kind);
            }
        }
        Err(UnknownWorkflowEventKind(s.to_string()))
    }

    /// The WP-KERNEL-005 microtask that owns this event kind.
    pub fn mt_owner(self) -> &'static str {
        match self {
            Self::ToolCall | Self::ToolProposal | Self::ToolApplyDecision => "MT-191",
            Self::VisualCapture | Self::VisualValidation | Self::VisualRecovery => "MT-192",
            Self::BuildGuard | Self::PackageGuard | Self::StaleDocDetected => "MT-193",
        }
    }

    /// Owning subsystem for diagnostics-panel filtering.
    pub fn subsystem(self) -> &'static str {
        match self {
            Self::ToolCall | Self::ToolProposal | Self::ToolApplyDecision => "tool_workflow",
            Self::VisualCapture | Self::VisualValidation | Self::VisualRecovery => {
                "visual_workflow"
            }
            Self::BuildGuard | Self::PackageGuard | Self::StaleDocDetected => "build_workflow",
        }
    }

    /// Required payload fields for the event kind. The persistence layer
    /// ([`crate::atelier::dcc_flight_recorder`]) rejects an emission whose
    /// payload is missing any of these as a non-empty string, so an event of
    /// this kind can never land hollow.
    pub fn required_payload_fields(self) -> &'static [&'static str] {
        match self {
            Self::ToolCall => &["tool_id", "status"],
            Self::ToolProposal => &["tool_id", "proposal_id"],
            Self::ToolApplyDecision => &["proposal_id", "decision"],
            Self::VisualCapture => &["capture_ref"],
            Self::VisualValidation => &["capture_ref", "verdict"],
            Self::VisualRecovery => &["capture_ref", "recovery_action"],
            Self::BuildGuard => &["build_id", "verdict"],
            Self::PackageGuard => &["package_id", "verdict"],
            Self::StaleDocDetected => &["doc_ref", "staleness_kind"],
        }
    }
}

impl fmt::Display for FrWorkflowEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FrWorkflowEventKind {
    type Err = UnknownWorkflowEventKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_id(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown FrWorkflowEventKind string: {0}")]
pub struct UnknownWorkflowEventKind(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn round_trip_every_variant() {
        for kind in FrWorkflowEventKind::all() {
            let s = kind.as_str();
            let back = FrWorkflowEventKind::from_str_id(s).expect("canonical id must round-trip");
            assert_eq!(*kind, back, "round-trip failed for {s}");
        }
    }

    #[test]
    fn ids_have_no_duplicates_and_canonical_shape() {
        let mut seen = HashSet::new();
        for kind in FrWorkflowEventKind::all() {
            let id = kind.as_str();
            assert!(seen.insert(id), "duplicate id: {id}");
            assert!(id.starts_with("FR-EVT-"), "id missing prefix: {id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'),
                "id contains non-canonical character: {id}"
            );
        }
    }

    #[test]
    fn unknown_lowercase_and_padded_ids_are_rejected() {
        assert!(FrWorkflowEventKind::from_str_id("FR-EVT-DOES-NOT-EXIST").is_err());
        assert!(FrWorkflowEventKind::from_str_id("fr-evt-tool-call").is_err());
        assert!(FrWorkflowEventKind::from_str_id("FR-EVT-TOOL-CALL ").is_err());
        assert!(FrWorkflowEventKind::from_str_id(" FR-EVT-TOOL-CALL").is_err());
    }

    #[test]
    fn mt_owner_partition_is_total_and_correct() {
        for kind in FrWorkflowEventKind::all() {
            let owner = kind.mt_owner();
            assert!(
                matches!(owner, "MT-191" | "MT-192" | "MT-193"),
                "unexpected owner {owner} for {kind}"
            );
        }
        assert_eq!(FrWorkflowEventKind::ToolApplyDecision.mt_owner(), "MT-191");
        assert_eq!(FrWorkflowEventKind::VisualRecovery.mt_owner(), "MT-192");
        assert_eq!(FrWorkflowEventKind::StaleDocDetected.mt_owner(), "MT-193");
    }

    #[test]
    fn every_kind_documents_required_payload_fields() {
        for kind in FrWorkflowEventKind::all() {
            assert!(
                !kind.required_payload_fields().is_empty(),
                "{kind} must require at least one payload field"
            );
        }
    }

    #[test]
    fn no_collision_with_wp004_registry_ids() {
        // WP-KERNEL-005 kinds must not shadow a WP-KERNEL-004 FrEventId.
        use super::super::fr_event_registry::FrEventId;
        for kind in FrWorkflowEventKind::all() {
            assert!(
                FrEventId::from_str_id(kind.as_str()).is_err(),
                "{} collides with a WP-KERNEL-004 registry id",
                kind.as_str()
            );
        }
    }
}
