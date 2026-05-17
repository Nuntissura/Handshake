//! MT-065 Lane Wake Receipt.
//!
//! Acceptance (MT-065.json): "wake/settlement event includes receipt refs
//! and reason."
//!
//! A KB003 lane (sandbox, validation, promotion) can enter a "wait" state
//! whenever a blocked reason (`dcc_kb003_blocked_reasons`) requires upstream
//! action. When that upstream action arrives (capability granted, artifact
//! produced, operator approval) the lane *wakes* and continues. When the
//! lane reaches a terminal status it *settles*. Both transitions must
//! produce typed, receipt-bearing events so a no-context reviewer can read
//! the lane history from durable storage without scraping chat.
//!
//! This module declares the receipt + the DCC display row. The receipt is
//! the durable record; the DCC row is the operator-visible projection.
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::dcc_kb003_blocked_reasons::{BlockedLane, BlockedReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LaneWakeEvent {
    Wake,
    Settle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneWakeReceiptV1 {
    pub schema_version: String,
    pub receipt_id: String,
    pub lane: BlockedLane,
    pub event: LaneWakeEvent,
    /// Receipt refs that justify the wake/settlement (e.g. promotion
    /// receipt id, denial id, validation report ref).
    pub receipt_refs: Vec<String>,
    /// Typed reason for the wake/settle (the blocked reason being resolved,
    /// or `None` if the lane is settling cleanly without a prior block).
    pub prior_blocked_reason: Option<BlockedReason>,
    /// Operator-readable short rationale.
    pub rationale_short: String,
    pub recorded_at_utc: DateTime<Utc>,
}

impl LaneWakeReceiptV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_lane_wake_receipt@1";

    pub fn wake(
        lane: BlockedLane,
        receipt_refs: Vec<String>,
        prior_blocked_reason: BlockedReason,
        rationale_short: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            receipt_id: format!("LWR-{}", Uuid::now_v7()),
            lane,
            event: LaneWakeEvent::Wake,
            receipt_refs,
            prior_blocked_reason: Some(prior_blocked_reason),
            rationale_short: rationale_short.into(),
            recorded_at_utc: Utc::now(),
        }
    }

    pub fn settle(
        lane: BlockedLane,
        receipt_refs: Vec<String>,
        rationale_short: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            receipt_id: format!("LWR-{}", Uuid::now_v7()),
            lane,
            event: LaneWakeEvent::Settle,
            receipt_refs,
            prior_blocked_reason: None,
            rationale_short: rationale_short.into(),
            recorded_at_utc: Utc::now(),
        }
    }

    /// Acceptance helper: wake events must include both a reason and at
    /// least one receipt ref; settle events must include at least one
    /// receipt ref (the terminal record).
    pub fn is_well_formed(&self) -> bool {
        if self.receipt_refs.is_empty() {
            return false;
        }
        match self.event {
            LaneWakeEvent::Wake => self.prior_blocked_reason.is_some() && !self.rationale_short.is_empty(),
            LaneWakeEvent::Settle => !self.rationale_short.is_empty(),
        }
    }
}

/// DCC display row for the lane wake/settlement timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003LaneWakeRowV1 {
    pub lane: BlockedLane,
    pub event: LaneWakeEvent,
    pub receipt_id: String,
    pub receipt_refs: Vec<String>,
    pub prior_blocked_tag: Option<String>,
    pub rationale_short: String,
    pub recorded_at_utc: DateTime<Utc>,
}

impl DccKb003LaneWakeRowV1 {
    pub fn from_receipt(r: &LaneWakeReceiptV1) -> Self {
        Self {
            lane: r.lane,
            event: r.event,
            receipt_id: r.receipt_id.clone(),
            receipt_refs: r.receipt_refs.clone(),
            prior_blocked_tag: r.prior_blocked_reason.as_ref().map(|b| b.tag().to_string()),
            rationale_short: r.rationale_short.clone(),
            recorded_at_utc: r.recorded_at_utc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_receipt_includes_reason_and_refs() {
        let reason = BlockedReason::MissingApproval { missing_field: "operator_id".into() };
        let r = LaneWakeReceiptV1::wake(
            BlockedLane::Promotion,
            vec!["kb003://promotion_receipt/PR-1".into()],
            reason,
            "operator approval received",
        );
        assert!(r.is_well_formed());
        assert_eq!(r.event, LaneWakeEvent::Wake);
        assert!(r.prior_blocked_reason.is_some());
        assert!(r.receipt_id.starts_with("LWR-"));
    }

    #[test]
    fn settle_receipt_well_formed_without_prior_reason() {
        let r = LaneWakeReceiptV1::settle(
            BlockedLane::Sandbox,
            vec!["kb003://sandbox_run/SBX-1".into()],
            "sandbox run completed",
        );
        assert!(r.is_well_formed());
        assert_eq!(r.event, LaneWakeEvent::Settle);
        assert!(r.prior_blocked_reason.is_none());
    }

    #[test]
    fn wake_receipt_requires_receipt_refs() {
        let reason = BlockedReason::AdapterUnavailable { adapter_kind: "x".into(), host_detail: "y".into() };
        let mut r = LaneWakeReceiptV1::wake(BlockedLane::Sandbox, vec![], reason, "x");
        r.receipt_refs.clear();
        assert!(!r.is_well_formed());
    }

    #[test]
    fn dcc_row_projects_receipt_tag() {
        let reason = BlockedReason::PolicyDenied {
            capability: "NETWORK".into(),
            policy_version_id: "POL-1@1".into(),
            denial_id: "DEN-1".into(),
        };
        let r = LaneWakeReceiptV1::wake(
            BlockedLane::Sandbox,
            vec!["kb003://denial/DEN-1".into()],
            reason,
            "operator granted network capability",
        );
        let row = DccKb003LaneWakeRowV1::from_receipt(&r);
        assert_eq!(row.prior_blocked_tag.as_deref(), Some("BLOCKED_POLICY_DENIED"));
        assert_eq!(row.lane, BlockedLane::Sandbox);
    }
}
