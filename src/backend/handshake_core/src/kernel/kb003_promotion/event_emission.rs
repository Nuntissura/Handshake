//! MT-044: Typed `Kb003EventEnvelope` emission for promotion lifecycle.
//!
//! Acceptance (MT-044): "Validation Preflight: preflight descriptors, tools,
//! capabilities, policy mode, paths, and budget. Missing tools produce
//! BLOCKED/UNSUPPORTED, not silent skip."
//!
//! The Batch D validation runner already encodes BLOCKED/UNSUPPORTED via
//! `ValidationStatus`. This module is the promotion-side mirror: every
//! promotion lifecycle event is *typed* and travels through the
//! `Kb003EventEnvelope`, so a downstream consumer (EventLedger, projection
//! rebuilder, audit replay) never has to guess the event shape or origin.
//!
//! Style template: `kernel::validation::report.rs` uses const-string event
//! types from `kb003_schemas`. We do the same here so the EventLedger sees a
//! consistent KB003 event vocabulary.

use uuid::Uuid;

use crate::kernel::kb003_schemas::{
    Kb003EventEnvelope, EVENT_KB003_PROMOTION_DECIDED, EVENT_KB003_PROMOTION_RECEIPT_ISSUED,
    EVENT_KB003_PROMOTION_REJECTED,
};

use super::decision::PromotionDecisionV1;
use super::receipt::PromotionReceiptV1;

/// Build the `Kb003EventEnvelope` for an accepted decision.
pub fn build_promotion_decided_event(
    decision: &PromotionDecisionV1,
    actor: &str,
    session: &str,
    task: &str,
    artifact_refs: Vec<String>,
) -> Kb003EventEnvelope {
    let run_id = parse_run_id(&decision.sandbox_run_id);
    let mut env = Kb003EventEnvelope::new(
        EVENT_KB003_PROMOTION_DECIDED,
        run_id,
        actor,
        session,
        task,
        &decision.decided_at_utc.to_rfc3339(),
    );
    env.artifact_refs = artifact_refs;
    env
}

/// Build the `Kb003EventEnvelope` for a rejected decision.
pub fn build_promotion_rejected_event(
    decision: &PromotionDecisionV1,
    actor: &str,
    session: &str,
    task: &str,
    artifact_refs: Vec<String>,
) -> Kb003EventEnvelope {
    let run_id = parse_run_id(&decision.sandbox_run_id);
    let mut env = Kb003EventEnvelope::new(
        EVENT_KB003_PROMOTION_REJECTED,
        run_id,
        actor,
        session,
        task,
        &decision.decided_at_utc.to_rfc3339(),
    );
    env.artifact_refs = artifact_refs;
    env
}

/// Build the `Kb003EventEnvelope` for a receipt issuance. The receipt id is
/// added to the envelope's `artifact_refs` so downstream consumers can locate
/// the durable receipt without joining tables.
pub fn build_promotion_receipt_issued_event(
    receipt: &PromotionReceiptV1,
    actor: &str,
    session: &str,
    task: &str,
) -> Kb003EventEnvelope {
    let run_id = parse_run_id(&receipt.decision.sandbox_run_id);
    let mut env = Kb003EventEnvelope::new(
        EVENT_KB003_PROMOTION_RECEIPT_ISSUED,
        run_id,
        actor,
        session,
        task,
        &receipt.issued_at_utc.to_rfc3339(),
    );
    let mut refs = vec![format!("kb003://promotion_receipt/{}", receipt.receipt_id)];
    refs.extend(receipt.artifact_refs.clone());
    env.artifact_refs = refs;
    env
}

/// Sandbox run ids carry the `SBX-<uuid>` prefix; the envelope's `run_id` is a
/// raw `Uuid`. Strip the prefix and fall back to a deterministic zero-uuid if
/// the string is malformed so envelope construction never panics.
fn parse_run_id(sandbox_run_id: &str) -> Uuid {
    let trimmed = sandbox_run_id.trim().trim_start_matches("SBX-");
    Uuid::parse_str(trimmed).unwrap_or(Uuid::nil())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_promotion::decision::{PromotionDecisionV1, PromotionRejectionReason};

    #[test]
    fn accepted_event_uses_decided_type() {
        let d = PromotionDecisionV1::accepted("SBX-1", "VR-1");
        let env = build_promotion_decided_event(&d, "actor", "ses", "task", vec!["a".into()]);
        assert_eq!(env.event_type, EVENT_KB003_PROMOTION_DECIDED);
        assert_eq!(env.artifact_refs, vec!["a"]);
        assert_eq!(env.actor_id, "actor");
        assert_eq!(env.session_id, "ses");
        assert_eq!(env.task_id, "task");
    }

    #[test]
    fn rejected_event_uses_rejected_type() {
        let r = PromotionRejectionReason::MissingApproval {
            missing_field: "operator_id".into(),
        };
        let d = PromotionDecisionV1::rejected("SBX-1", "VR-1", r);
        let env = build_promotion_rejected_event(&d, "actor", "ses", "task", vec![]);
        assert_eq!(env.event_type, EVENT_KB003_PROMOTION_REJECTED);
    }

    #[test]
    fn receipt_issued_event_includes_receipt_handle() {
        let d = PromotionDecisionV1::accepted("SBX-1", "VR-1");
        let r = PromotionReceiptV1::new(d, "IK-1", None, None, vec!["kb003://x/1".into()]);
        let env = build_promotion_receipt_issued_event(&r, "actor", "ses", "task");
        assert_eq!(env.event_type, EVENT_KB003_PROMOTION_RECEIPT_ISSUED);
        assert!(env.artifact_refs.iter().any(|s| s.contains(&r.receipt_id)));
        assert!(env.artifact_refs.iter().any(|s| s == "kb003://x/1"));
    }

    #[test]
    fn malformed_sandbox_run_id_does_not_panic() {
        let d = PromotionDecisionV1::accepted("not-a-uuid", "VR-1");
        let env = build_promotion_decided_event(&d, "a", "s", "t", vec![]);
        assert_eq!(env.run_id, Uuid::nil());
    }
}
