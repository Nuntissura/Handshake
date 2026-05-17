//! MT-041/043: `PromotionDecisionV1` + rejection-reason taxonomy.
//!
//! Acceptance composite (MT-041 + MT-043 + MT-045):
//! - "ValidationDescriptor contract: validation runner rejects undeclared raw
//!   commands" — enforced upstream in `validation::descriptor::DescriptorAllowlist`.
//!   The promotion gate consumes that decision via `ValidationReport` and never
//!   accepts a candidate whose validation report blocks promotion.
//! - "Validation result schema: every non-PASS has typed reason and evidence
//!   refs" — `PromotionRejectionReason` is a tagged enum; every variant either
//!   embeds the reason inline or routes to a structured evidence ref.
//! - "Deterministic check batch: blocking check failure prevents promotion" —
//!   `PromotionOutcome::Rejected` is the only thing the gate emits when the
//!   `ValidationReport` flags any blocking outcome.
//!
//! The full rejection-reason set required by the packet acceptance criteria
//! (per worker prompt):
//! - StaleCandidate            — candidate run already superseded by a newer run.
//! - DuplicateIdempotencyKey   — idempotency key already issued for a different payload.
//! - ValidationFailure         — `ValidationReport` blocks promotion.
//! - PolicyDenial              — sandbox policy denied a required capability.
//! - MissingApproval           — operator approval evidence missing or invalid.
//! - MissingArtifact           — referenced artifact handle is absent from the bundle.
//! - PostgresFailure           — durable storage write failed.
//! - ProjectionRebuildFailure  — DCC projection refresh failed after a decision.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::kb003_schemas::{
    EVENT_KB003_PROMOTION_DECIDED, EVENT_KB003_PROMOTION_REJECTED,
    SCHEMA_KERNEL_PROMOTION_DECISION_V1,
};

/// Typed outcome of `PromotionGate::evaluate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionOutcome {
    Accepted,
    Rejected { reason: PromotionRejectionReason },
}

impl PromotionOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn rejection_reason(&self) -> Option<&PromotionRejectionReason> {
        if let Self::Rejected { reason } = self {
            Some(reason)
        } else {
            None
        }
    }

    /// Short stable tag used in receipts/projections and the rationale_short
    /// column of `PromotionDecisionRowV1`.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Rejected { reason } => reason.tag(),
        }
    }

    /// EventLedger event-type for this outcome.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Accepted => EVENT_KB003_PROMOTION_DECIDED,
            Self::Rejected { .. } => EVENT_KB003_PROMOTION_REJECTED,
        }
    }
}

/// Full rejection-reason taxonomy. Every non-Accepted decision carries one of
/// these variants. Variants embed the load-bearing evidence inline so the
/// receipt can be inspected offline without joining other tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionRejectionReason {
    /// Candidate run was superseded by a newer run before the gate ran.
    StaleCandidate {
        candidate_run_id: String,
        latest_run_id: String,
    },
    /// Idempotency key already issued with a different payload hash.
    DuplicateIdempotencyKey {
        idempotency_key: String,
        existing_payload_hash: String,
        new_payload_hash: String,
    },
    /// `ValidationReport` reported at least one blocking descriptor outcome.
    ValidationFailure {
        validation_run_id: String,
        blocking_outcomes: Vec<String>,
        report_artifact_ref: Option<String>,
    },
    /// Sandbox policy denied a required capability.
    PolicyDenial {
        denial_id: String,
        policy_version_id: String,
        capability: Option<String>,
    },
    /// Operator approval evidence missing, empty, or malformed.
    MissingApproval { missing_field: String },
    /// Referenced artifact handle is not present in the bundle.
    MissingArtifact {
        expected_artifact_ref: String,
        bundle_id: Option<Uuid>,
    },
    /// Durable storage write failed (Postgres or compatible backend).
    PostgresFailure { storage_error: String },
    /// DCC projection refresh failed after the decision was recorded; the
    /// decision still exists in durable storage but the operator surface did
    /// not update. Surfaces as a soft rejection that the orchestrator should
    /// retry.
    ProjectionRebuildFailure { projection_family_id: String, detail: String },
}

impl PromotionRejectionReason {
    /// Short stable tag used in receipts and projections.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::StaleCandidate { .. } => "REJECTED_STALE_CANDIDATE",
            Self::DuplicateIdempotencyKey { .. } => "REJECTED_DUPLICATE_IDEMPOTENCY_KEY",
            Self::ValidationFailure { .. } => "REJECTED_VALIDATION_FAILURE",
            Self::PolicyDenial { .. } => "REJECTED_POLICY_DENIAL",
            Self::MissingApproval { .. } => "REJECTED_MISSING_APPROVAL",
            Self::MissingArtifact { .. } => "REJECTED_MISSING_ARTIFACT",
            Self::PostgresFailure { .. } => "REJECTED_POSTGRES_FAILURE",
            Self::ProjectionRebuildFailure { .. } => "REJECTED_PROJECTION_REBUILD_FAILURE",
        }
    }

    /// Human-readable rationale used by the `rationale_short` column.
    pub fn rationale_short(&self) -> String {
        match self {
            Self::StaleCandidate {
                candidate_run_id,
                latest_run_id,
            } => format!("stale candidate {candidate_run_id}; latest run is {latest_run_id}"),
            Self::DuplicateIdempotencyKey {
                idempotency_key, ..
            } => format!("duplicate idempotency key {idempotency_key}"),
            Self::ValidationFailure {
                validation_run_id,
                blocking_outcomes,
                ..
            } => format!(
                "validation run {} blocked promotion: {}",
                validation_run_id,
                blocking_outcomes.join(",")
            ),
            Self::PolicyDenial {
                denial_id,
                policy_version_id,
                capability,
            } => format!(
                "policy {policy_version_id} denied capability {} (denial {denial_id})",
                capability.as_deref().unwrap_or("<unspecified>")
            ),
            Self::MissingApproval { missing_field } => {
                format!("operator approval missing field {missing_field}")
            }
            Self::MissingArtifact {
                expected_artifact_ref,
                ..
            } => format!("artifact {expected_artifact_ref} not present in bundle"),
            Self::PostgresFailure { storage_error } => {
                format!("postgres write failed: {storage_error}")
            }
            Self::ProjectionRebuildFailure {
                projection_family_id,
                detail,
            } => format!("projection {projection_family_id} rebuild failed: {detail}"),
        }
    }
}

/// Durable promotion decision record. Schema id
/// `hsk.kernel.promotion_decision@1`. The decision id `PD-<uuid>` is stable
/// across replays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionDecisionV1 {
    pub schema_version: String,
    pub decision_id: String,
    pub validation_run_id: String,
    pub sandbox_run_id: String,
    pub outcome: PromotionOutcome,
    pub decided_at_utc: DateTime<Utc>,
}

impl PromotionDecisionV1 {
    pub fn accepted(sandbox_run_id: impl Into<String>, validation_run_id: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_KERNEL_PROMOTION_DECISION_V1.to_string(),
            decision_id: format!("PD-{}", Uuid::now_v7()),
            validation_run_id: validation_run_id.into(),
            sandbox_run_id: sandbox_run_id.into(),
            outcome: PromotionOutcome::Accepted,
            decided_at_utc: Utc::now(),
        }
    }

    pub fn rejected(
        sandbox_run_id: impl Into<String>,
        validation_run_id: impl Into<String>,
        reason: PromotionRejectionReason,
    ) -> Self {
        Self {
            schema_version: SCHEMA_KERNEL_PROMOTION_DECISION_V1.to_string(),
            decision_id: format!("PD-{}", Uuid::now_v7()),
            validation_run_id: validation_run_id.into(),
            sandbox_run_id: sandbox_run_id.into(),
            outcome: PromotionOutcome::Rejected { reason },
            decided_at_utc: Utc::now(),
        }
    }

    pub fn is_accepted(&self) -> bool {
        self.outcome.is_accepted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_outcome_has_no_rejection_reason() {
        let d = PromotionDecisionV1::accepted("SBX-1", "VR-1");
        assert!(d.is_accepted());
        assert_eq!(d.outcome.tag(), "ACCEPTED");
        assert_eq!(d.outcome.event_type(), EVENT_KB003_PROMOTION_DECIDED);
        assert!(d.outcome.rejection_reason().is_none());
        assert_eq!(d.schema_version, SCHEMA_KERNEL_PROMOTION_DECISION_V1);
        assert!(d.decision_id.starts_with("PD-"));
    }

    #[test]
    fn every_rejection_variant_tags_distinctly() {
        let reasons = vec![
            PromotionRejectionReason::StaleCandidate {
                candidate_run_id: "SBX-old".into(),
                latest_run_id: "SBX-new".into(),
            },
            PromotionRejectionReason::DuplicateIdempotencyKey {
                idempotency_key: "IK-1".into(),
                existing_payload_hash: "h-a".into(),
                new_payload_hash: "h-b".into(),
            },
            PromotionRejectionReason::ValidationFailure {
                validation_run_id: "VR-1".into(),
                blocking_outcomes: vec!["no_sandbox_escape".into()],
                report_artifact_ref: Some("ART-1".into()),
            },
            PromotionRejectionReason::PolicyDenial {
                denial_id: "DEN-1".into(),
                policy_version_id: "POL-1@1".into(),
                capability: Some("NETWORK".into()),
            },
            PromotionRejectionReason::MissingApproval {
                missing_field: "operator_id".into(),
            },
            PromotionRejectionReason::MissingArtifact {
                expected_artifact_ref: "kb003://x/abc".into(),
                bundle_id: None,
            },
            PromotionRejectionReason::PostgresFailure {
                storage_error: "deadlock".into(),
            },
            PromotionRejectionReason::ProjectionRebuildFailure {
                projection_family_id: "hsk.dcc.kb003.sandbox_promotion_lane@1".into(),
                detail: "missing source schema".into(),
            },
        ];
        let mut tags: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
        for r in &reasons {
            tags.insert(r.tag());
            assert!(!r.rationale_short().is_empty());
        }
        assert_eq!(tags.len(), 8, "every rejection variant must carry a unique tag");
    }

    // MT-043: every non-PASS rejection records its typed reason (no `Rejected`
    // without a reason is constructible).
    #[test]
    fn rejected_decision_preserves_reason() {
        let r = PromotionRejectionReason::ValidationFailure {
            validation_run_id: "VR-1".into(),
            blocking_outcomes: vec!["no_sandbox_escape".into(), "artifact_hashes_valid".into()],
            report_artifact_ref: Some("ART-1".into()),
        };
        let d = PromotionDecisionV1::rejected("SBX-1", "VR-1", r.clone());
        assert!(!d.is_accepted());
        assert_eq!(d.outcome.tag(), "REJECTED_VALIDATION_FAILURE");
        assert_eq!(d.outcome.event_type(), EVENT_KB003_PROMOTION_REJECTED);
        assert_eq!(d.outcome.rejection_reason(), Some(&r));
        // Rationale carries the blocking outcomes verbatim.
        let rationale = r.rationale_short();
        assert!(rationale.contains("no_sandbox_escape"));
        assert!(rationale.contains("artifact_hashes_valid"));
    }

    #[test]
    fn serde_round_trip_keeps_rejection_taxonomy_stable() {
        let r = PromotionRejectionReason::MissingArtifact {
            expected_artifact_ref: "kb003://sandbox_log/abcd".into(),
            bundle_id: None,
        };
        let d = PromotionDecisionV1::rejected("SBX-1", "VR-1", r);
        let j = serde_json::to_string(&d).unwrap();
        assert!(j.contains("\"kind\":\"REJECTED\""), "got {j}");
        assert!(j.contains("\"reason_kind\":\"MISSING_ARTIFACT\""), "got {j}");
        let back: PromotionDecisionV1 = serde_json::from_str(&j).unwrap();
        assert_eq!(back, d);
    }
}
