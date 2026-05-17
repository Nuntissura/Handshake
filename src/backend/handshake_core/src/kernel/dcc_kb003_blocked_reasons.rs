//! MT-060 Blocked Reason Taxonomy.
//!
//! Acceptance (MT-060.json): "each blocked reason has retry/escalate/gate
//! semantics."
//!
//! A "blocked" reason in KB003 is any typed condition that prevents a
//! sandbox run, validation descriptor, or promotion decision from making
//! forward progress without operator or upstream intervention. The taxonomy
//! covers the three KB003 lanes (sandbox, validation, promotion) and binds
//! each variant to:
//!
//! - `BlockedDisposition::Retry`     — the same actor may retry after a
//!   bounded delay; the lane stays open.
//! - `BlockedDisposition::Escalate`  — a different actor (operator, gate
//!   owner) must act; the lane goes to wait state.
//! - `BlockedDisposition::Gate`      — a hard gate must be unblocked
//!   upstream (capability grant, missing artifact, descriptor update).
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits required.
//! The display row [`DccKb003BlockedReasonRowV1`] is the DCC contract.

use serde::{Deserialize, Serialize};

/// Lane the blocked reason originated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockedLane {
    Sandbox,
    Validation,
    Promotion,
}

/// What the operator/orchestrator should do with the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockedDisposition {
    /// Bounded retry by the same actor.
    Retry,
    /// Different actor must act (operator, gate owner, integration validator).
    Escalate,
    /// Upstream gate must be unblocked (capability, artifact, descriptor).
    Gate,
}

/// Typed blocked reason. Each variant carries enough load-bearing detail for
/// a no-context model to know what to do without joining other tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockedReason {
    // ------------------------- Sandbox lane -------------------------
    /// Sandbox adapter (process, container, microVM) not available on host.
    AdapterUnavailable { adapter_kind: String, host_detail: String },
    /// Policy denied a required capability.
    PolicyDenied { capability: String, policy_version_id: String, denial_id: String },
    /// Workspace materializer reported a boundary violation.
    WorkspaceBoundaryViolation { workspace_id: String, attempted_path: String },
    /// Sandbox resource cap (cpu, memory, disk, wall-time) hit.
    ResourceCapExceeded { cap_kind: String, observed: String, limit: String },
    /// Cancellation propagated from operator/upstream before completion.
    Cancelled { upstream_reason: String },

    // ------------------------ Validation lane -----------------------
    /// Descriptor referenced an artifact that is not present.
    MissingArtifact { expected_artifact_ref: String, descriptor_id: String },
    /// Required tool/adapter for the descriptor is unsupported on host.
    UnsupportedAdapter { adapter: String, descriptor_id: String },
    /// Operator/policy explicitly skipped this descriptor.
    SkippedByPolicy { descriptor_id: String, reason: String },

    // ------------------------ Promotion lane ------------------------
    /// Operator approval evidence missing.
    MissingApproval { missing_field: String },
    /// Validation report blocks promotion.
    ValidationFailure { validation_run_id: String, blocking_outcomes: Vec<String> },
    /// Idempotency conflict (replay collision with different payload).
    IdempotencyConflict { idempotency_key: String },
    /// Downstream durable-storage write failed; retryable.
    StorageWriteFailure { storage_error: String },
}

impl BlockedReason {
    /// Lane the reason originates from.
    pub fn lane(&self) -> BlockedLane {
        match self {
            Self::AdapterUnavailable { .. }
            | Self::PolicyDenied { .. }
            | Self::WorkspaceBoundaryViolation { .. }
            | Self::ResourceCapExceeded { .. }
            | Self::Cancelled { .. } => BlockedLane::Sandbox,
            Self::MissingArtifact { .. }
            | Self::UnsupportedAdapter { .. }
            | Self::SkippedByPolicy { .. } => BlockedLane::Validation,
            Self::MissingApproval { .. }
            | Self::ValidationFailure { .. }
            | Self::IdempotencyConflict { .. }
            | Self::StorageWriteFailure { .. } => BlockedLane::Promotion,
        }
    }

    /// Disposition: retry / escalate / gate. Acceptance criterion: every
    /// variant returns a non-default disposition.
    pub fn disposition(&self) -> BlockedDisposition {
        match self {
            // Transient or recoverable — bounded retry.
            Self::ResourceCapExceeded { .. } => BlockedDisposition::Retry,
            Self::StorageWriteFailure { .. } => BlockedDisposition::Retry,
            // Operator/owner must act.
            Self::Cancelled { .. } => BlockedDisposition::Escalate,
            Self::MissingApproval { .. } => BlockedDisposition::Escalate,
            Self::ValidationFailure { .. } => BlockedDisposition::Escalate,
            Self::IdempotencyConflict { .. } => BlockedDisposition::Escalate,
            Self::SkippedByPolicy { .. } => BlockedDisposition::Escalate,
            // Upstream gate (capability, artifact, adapter) must be unblocked.
            Self::AdapterUnavailable { .. } => BlockedDisposition::Gate,
            Self::PolicyDenied { .. } => BlockedDisposition::Gate,
            Self::WorkspaceBoundaryViolation { .. } => BlockedDisposition::Gate,
            Self::MissingArtifact { .. } => BlockedDisposition::Gate,
            Self::UnsupportedAdapter { .. } => BlockedDisposition::Gate,
        }
    }

    /// Short stable tag for receipts / projections.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::AdapterUnavailable { .. } => "BLOCKED_ADAPTER_UNAVAILABLE",
            Self::PolicyDenied { .. } => "BLOCKED_POLICY_DENIED",
            Self::WorkspaceBoundaryViolation { .. } => "BLOCKED_WORKSPACE_BOUNDARY_VIOLATION",
            Self::ResourceCapExceeded { .. } => "BLOCKED_RESOURCE_CAP_EXCEEDED",
            Self::Cancelled { .. } => "BLOCKED_CANCELLED",
            Self::MissingArtifact { .. } => "BLOCKED_MISSING_ARTIFACT",
            Self::UnsupportedAdapter { .. } => "BLOCKED_UNSUPPORTED_ADAPTER",
            Self::SkippedByPolicy { .. } => "BLOCKED_SKIPPED_BY_POLICY",
            Self::MissingApproval { .. } => "BLOCKED_MISSING_APPROVAL",
            Self::ValidationFailure { .. } => "BLOCKED_VALIDATION_FAILURE",
            Self::IdempotencyConflict { .. } => "BLOCKED_IDEMPOTENCY_CONFLICT",
            Self::StorageWriteFailure { .. } => "BLOCKED_STORAGE_WRITE_FAILURE",
        }
    }

    /// Operator-facing single-line summary.
    pub fn rationale_short(&self) -> String {
        match self {
            Self::AdapterUnavailable { adapter_kind, host_detail } => {
                format!("adapter '{adapter_kind}' unavailable on host: {host_detail}")
            }
            Self::PolicyDenied { capability, policy_version_id, denial_id } => {
                format!("policy {policy_version_id} denied '{capability}' (denial {denial_id})")
            }
            Self::WorkspaceBoundaryViolation { workspace_id, attempted_path } => {
                format!("workspace {workspace_id} boundary violated by '{attempted_path}'")
            }
            Self::ResourceCapExceeded { cap_kind, observed, limit } => {
                format!("{cap_kind} cap exceeded (observed={observed} limit={limit})")
            }
            Self::Cancelled { upstream_reason } => format!("cancelled: {upstream_reason}"),
            Self::MissingArtifact { expected_artifact_ref, descriptor_id } => {
                format!("descriptor {descriptor_id} requires missing artifact {expected_artifact_ref}")
            }
            Self::UnsupportedAdapter { adapter, descriptor_id } => {
                format!("descriptor {descriptor_id} needs unsupported adapter '{adapter}'")
            }
            Self::SkippedByPolicy { descriptor_id, reason } => {
                format!("descriptor {descriptor_id} skipped by policy: {reason}")
            }
            Self::MissingApproval { missing_field } => {
                format!("operator approval missing field '{missing_field}'")
            }
            Self::ValidationFailure { validation_run_id, blocking_outcomes } => format!(
                "validation run {validation_run_id} blocks promotion: {}",
                blocking_outcomes.join(",")
            ),
            Self::IdempotencyConflict { idempotency_key } => {
                format!("idempotency key '{idempotency_key}' collides with prior payload")
            }
            Self::StorageWriteFailure { storage_error } => {
                format!("storage write failure: {storage_error}")
            }
        }
    }
}

/// DCC operator-facing row for a single blocked reason. The frontend renders
/// these via the existing dcc-* IPC surface; no app/** edits required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003BlockedReasonRowV1 {
    pub lane: BlockedLane,
    pub disposition: BlockedDisposition,
    pub tag: String,
    pub rationale_short: String,
    pub reason: BlockedReason,
}

impl DccKb003BlockedReasonRowV1 {
    pub fn from_reason(reason: BlockedReason) -> Self {
        Self {
            lane: reason.lane(),
            disposition: reason.disposition(),
            tag: reason.tag().to_string(),
            rationale_short: reason.rationale_short(),
            reason,
        }
    }
}

/// Top-level DCC overlay enumerating every active blocked reason across all
/// three lanes for an operator view. Constructed by `DccKb003RollupV1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003BlockedReasonOverlayV1 {
    pub overlay_family_id: String,
    pub rows: Vec<DccKb003BlockedReasonRowV1>,
}

impl DccKb003BlockedReasonOverlayV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.blocked_reasons@1";

    pub fn new(rows: Vec<DccKb003BlockedReasonRowV1>) -> Self {
        Self {
            overlay_family_id: Self::FAMILY_ID.to_string(),
            rows,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn rows_for_lane(&self, lane: BlockedLane) -> impl Iterator<Item = &DccKb003BlockedReasonRowV1> {
        self.rows.iter().filter(move |r| r.lane == lane)
    }

    pub fn rows_for_disposition(&self, d: BlockedDisposition) -> impl Iterator<Item = &DccKb003BlockedReasonRowV1> {
        self.rows.iter().filter(move |r| r.disposition == d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_variants() -> Vec<BlockedReason> {
        vec![
            BlockedReason::AdapterUnavailable { adapter_kind: "microvm".into(), host_detail: "no kvm".into() },
            BlockedReason::PolicyDenied { capability: "NETWORK".into(), policy_version_id: "POL-1@1".into(), denial_id: "DEN-1".into() },
            BlockedReason::WorkspaceBoundaryViolation { workspace_id: "WSP-1".into(), attempted_path: "/etc/x".into() },
            BlockedReason::ResourceCapExceeded { cap_kind: "wall_time_s".into(), observed: "120".into(), limit: "60".into() },
            BlockedReason::Cancelled { upstream_reason: "operator_abort".into() },
            BlockedReason::MissingArtifact { expected_artifact_ref: "ART-1".into(), descriptor_id: "DESC-1".into() },
            BlockedReason::UnsupportedAdapter { adapter: "axe-core".into(), descriptor_id: "DESC-2".into() },
            BlockedReason::SkippedByPolicy { descriptor_id: "DESC-3".into(), reason: "feature flag off".into() },
            BlockedReason::MissingApproval { missing_field: "operator_id".into() },
            BlockedReason::ValidationFailure { validation_run_id: "VR-1".into(), blocking_outcomes: vec!["FAIL".into()] },
            BlockedReason::IdempotencyConflict { idempotency_key: "IK-1".into() },
            BlockedReason::StorageWriteFailure { storage_error: "conn refused".into() },
        ]
    }

    #[test]
    fn every_variant_has_lane_disposition_tag_and_rationale() {
        for v in all_variants() {
            // lane is one of the three.
            let lane = v.lane();
            assert!(matches!(
                lane,
                BlockedLane::Sandbox | BlockedLane::Validation | BlockedLane::Promotion
            ));
            // disposition is one of three (not silently default).
            let d = v.disposition();
            assert!(matches!(
                d,
                BlockedDisposition::Retry | BlockedDisposition::Escalate | BlockedDisposition::Gate
            ));
            // tag is non-empty and BLOCKED_ prefixed.
            assert!(v.tag().starts_with("BLOCKED_"), "tag={}", v.tag());
            // rationale_short is non-empty.
            assert!(!v.rationale_short().is_empty());
        }
    }

    #[test]
    fn tags_are_unique_across_variants() {
        let mut tags: Vec<&'static str> = all_variants().iter().map(|v| v.tag()).collect();
        let original = tags.len();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), original, "duplicate blocked tag detected");
    }

    #[test]
    fn overlay_partitions_by_lane_and_disposition() {
        let rows: Vec<_> = all_variants().into_iter().map(DccKb003BlockedReasonRowV1::from_reason).collect();
        let overlay = DccKb003BlockedReasonOverlayV1::new(rows);
        assert!(!overlay.is_empty());
        assert!(overlay.rows_for_lane(BlockedLane::Sandbox).count() >= 1);
        assert!(overlay.rows_for_lane(BlockedLane::Validation).count() >= 1);
        assert!(overlay.rows_for_lane(BlockedLane::Promotion).count() >= 1);
        assert!(overlay.rows_for_disposition(BlockedDisposition::Retry).count() >= 1);
        assert!(overlay.rows_for_disposition(BlockedDisposition::Escalate).count() >= 1);
        assert!(overlay.rows_for_disposition(BlockedDisposition::Gate).count() >= 1);
    }

    #[test]
    fn family_id_is_versioned_and_namespaced() {
        assert!(DccKb003BlockedReasonOverlayV1::FAMILY_ID.starts_with("hsk.dcc.kb003."));
        assert!(DccKb003BlockedReasonOverlayV1::FAMILY_ID.contains('@'));
    }
}
