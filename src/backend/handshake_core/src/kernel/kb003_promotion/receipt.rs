//! MT-042/043: `PromotionReceiptV1` — the durable, content-addressed receipt
//! for a `PromotionDecisionV1`.
//!
//! Acceptance composite:
//! - "Check Runner Adapter: reuse Product Governance Check Runner. No
//!   duplicate check runner is created." — this module carries the receipt
//!   shape that downstream consumers (orchestrator, validator) read instead of
//!   re-running checks. The same `idempotency_key` produces the same receipt
//!   id; replays do not duplicate the runner.
//! - "Validation Result Schema: every non-PASS has typed reason and evidence
//!   refs." — receipt embeds the decision (which embeds the typed rejection
//!   reason from `decision.rs`) and the artifact refs the decision relied on.
//!
//! Hash policy: `Kb003ArtifactClass::PromotionReceipt` declares
//! `HashPolicy::CanonicalJsonSha256`. `payload_hash()` computes the canonical
//! JSON SHA-256 over a deterministic projection of the receipt body so the
//! same logical receipt always hashes to the same value.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::kernel::context_bundle::{canonical_json_bytes, sha256_hex};
use crate::kernel::kb003_artifact_classes::{HashPolicy, Kb003ArtifactClass, metadata_for};
use crate::kernel::kb003_schemas::SCHEMA_KERNEL_PROMOTION_RECEIPT_V1;

use super::decision::{PromotionDecisionV1, PromotionOutcome};

/// Durable receipt for a promotion decision. Schema id
/// `hsk.kernel.promotion_receipt@1`.
///
/// `idempotency_key` is the caller-supplied key that lets retries collapse to
/// the same receipt. `payload_hash` is the canonical-JSON SHA-256 of the
/// receipt body and is used by `Kb003Storage::insert_promotion_receipt` to
/// detect idempotency conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionReceiptV1 {
    pub schema_version: String,
    pub receipt_id: String,
    pub decision: PromotionDecisionV1,
    pub idempotency_key: String,
    pub payload_hash: String,
    pub bundle_id: Option<Uuid>,
    pub bundle_sha256: Option<String>,
    pub artifact_refs: Vec<String>,
    pub event_ledger_event_id: Option<String>,
    pub issued_at_utc: DateTime<Utc>,
    /// H4 fix (KB003 remediation): raw `e.to_string()` for the storage error
    /// when the rejection variant is `PostgresFailure`. NOT included in
    /// `payload_hash` — present only for observability so operators can
    /// inspect what Postgres actually said even though the hash is now
    /// computed from a bucketed `NormalisedStorageErrorKind`. `None` for
    /// every other rejection variant and for accepted decisions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_error_detail: Option<String>,
}

impl PromotionReceiptV1 {
    /// Construct a receipt for `decision`. The caller passes the idempotency
    /// key (typically `format!("PROM-{}-{}", sandbox_run_id, validation_run_id)`).
    /// `bundle_sha256` and `artifact_refs` come from the
    /// `Kb003ArtifactBundleV1` the gate inspected.
    pub fn new(
        decision: PromotionDecisionV1,
        idempotency_key: impl Into<String>,
        bundle_id: Option<Uuid>,
        bundle_sha256: Option<String>,
        artifact_refs: Vec<String>,
    ) -> Self {
        let idempotency_key = idempotency_key.into();
        let receipt_id = format!("PR-{}", Uuid::now_v7());
        let mut receipt = Self {
            schema_version: SCHEMA_KERNEL_PROMOTION_RECEIPT_V1.to_string(),
            receipt_id,
            decision,
            idempotency_key,
            payload_hash: String::new(),
            bundle_id,
            bundle_sha256,
            artifact_refs,
            event_ledger_event_id: None,
            issued_at_utc: Utc::now(),
            storage_error_detail: None,
        };
        receipt.payload_hash = receipt.compute_payload_hash();
        receipt
    }

    /// H4 fix: like `new`, but also captures the raw storage-error string
    /// for observability (only used on the PostgresFailure rejection path
    /// in `PromotionGate::evaluate`). The raw string does NOT participate
    /// in `payload_hash` — the hash is computed from the canonical
    /// projection in `decision::canonical_hash_projection`.
    pub fn new_with_storage_error_detail(
        decision: PromotionDecisionV1,
        idempotency_key: impl Into<String>,
        bundle_id: Option<Uuid>,
        bundle_sha256: Option<String>,
        artifact_refs: Vec<String>,
        storage_error_detail: Option<String>,
    ) -> Self {
        let mut receipt = Self::new(
            decision,
            idempotency_key,
            bundle_id,
            bundle_sha256,
            artifact_refs,
        );
        receipt.storage_error_detail = storage_error_detail;
        receipt
    }

    /// Confirms the artifact class metadata still pins the receipt to the
    /// canonical-JSON hash policy. The gate refuses to issue a receipt if
    /// this invariant breaks.
    pub fn hash_policy() -> HashPolicy {
        metadata_for(Kb003ArtifactClass::PromotionReceipt)
            .expect("PromotionReceipt class registered")
            .hash_policy
    }

    /// Canonical-JSON SHA-256 over the receipt body, excluding `payload_hash`,
    /// `receipt_id`, `issued_at_utc`, AND `decision_id` (so two logically-
    /// equivalent receipts produced at different times — therefore with
    /// different ephemeral UUIDs in those fields — hash identically).
    ///
    /// Round-3 self-scrutiny fix: `decision_id` used to be in the hash. It
    /// is a `format!("PD-{}", Uuid::now_v7())` ephemeral, so two evaluate()
    /// calls with identical inputs produced different hashes and the
    /// idempotency dedup never fired. The previously-committed M-A5 storage
    /// tightening (require `decision_id` match on a hash hit) compounded
    /// the symptom; both are corrected together.
    pub fn compute_payload_hash(&self) -> String {
        // H4 fix (KB003 remediation): the outcome we hash is a DETERMINISTIC
        // projection, not the raw `self.decision.outcome`. The raw outcome
        // embeds ephemeral fields (PolicyDenial.denial_id, MissingArtifact
        // .bundle_id, PostgresFailure.storage_error raw string) that caused
        // two retries of the same logical rejection to produce different
        // payload_hash values → false IdempotencyConflict. The full outcome
        // is still retained on `self.decision.outcome` for observability.
        let canonical_outcome = match &self.decision.outcome {
            PromotionOutcome::Accepted => json!({"kind": "ACCEPTED"}),
            PromotionOutcome::Rejected { reason } => json!({
                "kind": "REJECTED",
                "projection": reason.canonical_hash_projection(),
            }),
        };
        let body = json!({
            "schema_version": self.schema_version,
            "validation_run_id": self.decision.validation_run_id,
            "sandbox_run_id": self.decision.sandbox_run_id,
            "outcome": canonical_outcome,
            "idempotency_key": self.idempotency_key,
            "bundle_sha256": self.bundle_sha256,
            "artifact_refs": self.artifact_refs,
        });
        sha256_hex(&canonical_json_bytes(&body))
    }

    /// Attach the EventLedger event id once it has been recorded.
    pub fn with_event_ledger_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_ledger_event_id = Some(event_id.into());
        self
    }

    /// True iff the receipt is anchored to durable evidence (bundle hash +
    /// at least one artifact ref).
    pub fn is_anchored_to_evidence(&self) -> bool {
        self.bundle_sha256.is_some() && !self.artifact_refs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_promotion::decision::{PromotionDecisionV1, PromotionRejectionReason};

    fn accepted_decision() -> PromotionDecisionV1 {
        PromotionDecisionV1::accepted("SBX-1", "VR-1")
    }

    // MT-042/043 acceptance: receipt hash policy is CanonicalJsonSha256.
    #[test]
    fn hash_policy_matches_class_table() {
        assert_eq!(
            PromotionReceiptV1::hash_policy(),
            HashPolicy::CanonicalJsonSha256
        );
    }

    // Equivalent receipts hash to the same payload_hash even though
    // receipt_id/issued_at_utc differ.
    #[test]
    fn payload_hash_is_stable_across_receipt_instances() {
        let d = accepted_decision();
        let r1 = PromotionReceiptV1::new(
            d.clone(),
            "IK-1",
            None,
            Some("bundle-sha".into()),
            vec!["kb003://x/1".into()],
        );
        let r2 = PromotionReceiptV1::new(
            d,
            "IK-1",
            None,
            Some("bundle-sha".into()),
            vec!["kb003://x/1".into()],
        );
        assert_eq!(r1.payload_hash, r2.payload_hash);
        assert_ne!(r1.receipt_id, r2.receipt_id, "ids are fresh per call");
    }

    // Round-3 regression guard: two FRESH accepted() decisions with the same
    // inputs (different decision_id UUIDs) MUST hash to the same payload_hash.
    // Previously decision_id was in the hash, so idempotency dedup never fired.
    #[test]
    fn fresh_decisions_with_same_inputs_hash_identically() {
        let d1 = PromotionDecisionV1::accepted("SBX-x", "VR-x");
        let d2 = PromotionDecisionV1::accepted("SBX-x", "VR-x");
        assert_ne!(
            d1.decision_id, d2.decision_id,
            "decision_id is ephemeral UUID"
        );
        let r1 = PromotionReceiptV1::new(
            d1,
            "IK-test",
            None,
            Some("bundle-sha".into()),
            vec!["kb003://art/1".into()],
        );
        let r2 = PromotionReceiptV1::new(
            d2,
            "IK-test",
            None,
            Some("bundle-sha".into()),
            vec!["kb003://art/1".into()],
        );
        assert_eq!(
            r1.payload_hash, r2.payload_hash,
            "fresh decisions with same logical inputs must hash identically (idempotency)"
        );
    }

    // Different idempotency_keys => different payload_hash (the key is part
    // of the canonical body so storage idempotency aligns with caller intent).
    #[test]
    fn different_idempotency_key_changes_payload_hash() {
        let d = accepted_decision();
        let r1 = PromotionReceiptV1::new(d.clone(), "IK-1", None, None, vec![]);
        let r2 = PromotionReceiptV1::new(d, "IK-2", None, None, vec![]);
        assert_ne!(r1.payload_hash, r2.payload_hash);
    }

    // Rejection-bearing receipts preserve the typed rejection reason in the
    // hash (so attackers cannot swap reason without changing the hash).
    #[test]
    fn payload_hash_covers_rejection_reason() {
        let r1_dec = PromotionDecisionV1::rejected(
            "SBX-1",
            "VR-1",
            PromotionRejectionReason::ValidationFailure {
                validation_run_id: "VR-1".into(),
                blocking_outcomes: vec!["x".into()],
                report_artifact_ref: None,
            },
        );
        let r2_dec = PromotionDecisionV1::rejected(
            "SBX-1",
            "VR-1",
            PromotionRejectionReason::ValidationFailure {
                validation_run_id: "VR-1".into(),
                blocking_outcomes: vec!["y".into()],
                report_artifact_ref: None,
            },
        );
        let r1 = PromotionReceiptV1::new(r1_dec, "IK-1", None, None, vec![]);
        let r2 = PromotionReceiptV1::new(r2_dec, "IK-1", None, None, vec![]);
        assert_ne!(r1.payload_hash, r2.payload_hash);
    }

    #[test]
    fn evidence_anchoring_check() {
        let d = accepted_decision();
        let unanchored = PromotionReceiptV1::new(d.clone(), "IK", None, None, vec![]);
        assert!(!unanchored.is_anchored_to_evidence());
        let anchored = PromotionReceiptV1::new(
            d,
            "IK",
            None,
            Some("bundle-sha".into()),
            vec!["kb003://x/1".into()],
        );
        assert!(anchored.is_anchored_to_evidence());
    }
}
