//! WP-KERNEL-005 MT-156: STEER feedback from visual mismatch.
//!
//! Converts visual threshold breaches in a validated
//! [`VisualDebuggingLoopV1`](crate::kernel::visual_debugging_loop::VisualDebuggingLoopV1)
//! into actionable, durable STEER feedback records -- never a silent failure
//! and never generic prose. One record per `(loop_id, evidence_id)` breach is
//! persisted in the embedded SurrealDB store (table
//! `atelier_visual_steer_feedback`) and mirrored through the Atelier
//! EventLedger so downstream roles can route the STEER receipt.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use surrealdb::types::{Datetime, SurrealValue};

use crate::kernel::visual_debugging_loop::{validate_visual_debugging_loop, VisualDebuggingLoopV1};

use super::{atelier_event_sql, AtelierError, AtelierResult, AtelierStore};

pub mod visual_steer_event_family {
    /// A visual threshold breach was converted into a STEER feedback record
    /// (MT-156).
    pub const VISUAL_STEER_FEEDBACK_RECORDED: &str = "atelier.visual_steer.feedback_recorded";

    pub const ALL: &[&str] = &[VISUAL_STEER_FEEDBACK_RECORDED];
}

/// Persisted STEER feedback record for one visual threshold breach (MT-156).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualSteerFeedbackRecord {
    pub feedback_id: String,
    pub loop_id: String,
    pub evidence_id: String,
    pub wp_id: String,
    pub mismatch_basis_points: i32,
    pub threshold_basis_points: i32,
    /// Role the STEER receipt is routed to (always `VALIDATOR` today).
    pub target_role: String,
    /// Receipt kind (always `STEER`; enforced by the schema's literal field
    /// type).
    pub receipt_kind: String,
    pub code_diff_ref: String,
    pub visual_diff_ref: String,
    /// Concrete, actionable instruction naming the breach and the refs to act
    /// on -- never generic prose.
    pub next_action: String,
    pub created_at_utc: DateTime<Utc>,
}

/// One `atelier_visual_steer_feedback` row as the store returns it.
#[derive(SurrealValue)]
struct VisualSteerFeedbackRow {
    feedback_id: String,
    loop_id: String,
    evidence_id: String,
    wp_id: String,
    mismatch_basis_points: i32,
    threshold_basis_points: i32,
    target_role: String,
    receipt_kind: String,
    code_diff_ref: String,
    visual_diff_ref: String,
    next_action: String,
    created_at_utc: Datetime,
}

impl From<VisualSteerFeedbackRow> for VisualSteerFeedbackRecord {
    fn from(row: VisualSteerFeedbackRow) -> Self {
        VisualSteerFeedbackRecord {
            feedback_id: row.feedback_id,
            loop_id: row.loop_id,
            evidence_id: row.evidence_id,
            wp_id: row.wp_id,
            mismatch_basis_points: row.mismatch_basis_points,
            threshold_basis_points: row.threshold_basis_points,
            target_role: row.target_role,
            receipt_kind: row.receipt_kind,
            code_diff_ref: row.code_diff_ref,
            visual_diff_ref: row.visual_diff_ref,
            next_action: row.next_action,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(Clone, SurrealValue)]
struct VisualSteerFeedbackBindings {
    feedback_id: String,
    loop_id: String,
    evidence_id: String,
    wp_id: String,
    mismatch_basis_points: i32,
    threshold_basis_points: i32,
    target_role: String,
    receipt_kind: String,
    code_diff_ref: String,
    visual_diff_ref: String,
    next_action: String,
}

#[derive(SurrealValue)]
struct LoopIdBinding {
    loop_id: String,
}

/// Upsert one STEER feedback record keyed on its deterministic
/// `feedback_id` (which encodes `loop_id` + `evidence_id`) and append its
/// event in the same atomic statement. The SET form deliberately omits
/// `created_at_utc` so the schema default stamps it on first write and an
/// upsert replay preserves it, matching the former conflict-update contract.
const RECORD_VISUAL_STEER_FEEDBACK_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_visual_steer_feedback', $domain.feedback_id); ",
    atelier_event_sql!(),
    " RETURN (UPSERT $rid SET \
         feedback_id = $domain.feedback_id, \
         loop_id = $domain.loop_id, \
         evidence_id = $domain.evidence_id, \
         wp_id = $domain.wp_id, \
         mismatch_basis_points = $domain.mismatch_basis_points, \
         threshold_basis_points = $domain.threshold_basis_points, \
         target_role = $domain.target_role, \
         receipt_kind = $domain.receipt_kind, \
         code_diff_ref = $domain.code_diff_ref, \
         visual_diff_ref = $domain.visual_diff_ref, \
         next_action = $domain.next_action \
       RETURN AFTER)[0]; };"
);

const LIST_VISUAL_STEER_FEEDBACK_STATEMENT: &str =
    "SELECT feedback_id, loop_id, evidence_id, wp_id, mismatch_basis_points, \
            threshold_basis_points, target_role, receipt_kind, code_diff_ref, \
            visual_diff_ref, next_action, created_at_utc \
     FROM atelier_visual_steer_feedback \
     WHERE loop_id = $loop_id \
     ORDER BY created_at_utc DESC, feedback_id ASC;";

impl AtelierStore {
    /// Convert every visual threshold breach in `loop_config` into a durable,
    /// actionable STEER feedback record (MT-156).
    ///
    /// The loop is first validated against the full MT-046 visual-debugging
    /// loop contract; an invalid loop is rejected (no silent failure). For each
    /// evidence artifact whose `mismatch_basis_points` exceeds the configured
    /// `max_pixel_diff_basis_points`, one record is upserted keyed on
    /// `(loop_id, evidence_id)` and one `VISUAL_STEER_FEEDBACK_RECORDED` event
    /// is written in the same atomic statement as the record. Records are
    /// written per breach; a replay converges because every write is an
    /// idempotent upsert. A loop without breaches records nothing and returns
    /// an empty list.
    pub async fn record_visual_steer_feedback(
        &self,
        loop_config: &VisualDebuggingLoopV1,
    ) -> AtelierResult<Vec<VisualSteerFeedbackRecord>> {
        if let Err(errors) = validate_visual_debugging_loop(loop_config) {
            let detail = errors
                .iter()
                .map(|error| format!("{}: {}", error.field, error.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AtelierError::Validation(format!(
                "visual steer feedback rejected an invalid visual debugging loop: {detail}"
            )));
        }

        let threshold = loop_config.threshold_config.max_pixel_diff_basis_points;
        let breaches: Vec<_> = loop_config
            .evidence_artifacts
            .iter()
            .filter(|artifact| artifact.mismatch_basis_points > threshold)
            .collect();
        if breaches.is_empty() {
            return Ok(Vec::new());
        }

        let steering = &loop_config.validator_steering;
        let mut recorded = Vec::with_capacity(breaches.len());
        for artifact in breaches {
            let feedback_id = format!("steer-{}-{}", loop_config.loop_id, artifact.evidence_id);
            let next_action = format!(
                "Visual mismatch {} bps exceeds threshold {} bps for evidence {}: apply the \
                 code diff at {} and re-run the visual loop against {}",
                artifact.mismatch_basis_points,
                threshold,
                artifact.evidence_id,
                steering.code_diff_ref,
                artifact.visual_diff_artifact_ref,
            );
            let bindings = VisualSteerFeedbackBindings {
                feedback_id: feedback_id.clone(),
                loop_id: loop_config.loop_id.clone(),
                evidence_id: artifact.evidence_id.clone(),
                wp_id: artifact.wp_id.clone(),
                mismatch_basis_points: artifact.mismatch_basis_points as i32,
                threshold_basis_points: threshold as i32,
                target_role: steering.target_role.clone(),
                receipt_kind: steering.receipt_kind.clone(),
                code_diff_ref: steering.code_diff_ref.clone(),
                visual_diff_ref: artifact.visual_diff_artifact_ref.clone(),
                next_action,
            };
            let row: Option<VisualSteerFeedbackRow> = self
                .write_with_event(
                    RECORD_VISUAL_STEER_FEEDBACK_STATEMENT,
                    bindings,
                    visual_steer_event_family::VISUAL_STEER_FEEDBACK_RECORDED,
                    "atelier_visual_steer_feedback",
                    &feedback_id,
                    json!({
                        "feedback_id": feedback_id,
                        "loop_id": loop_config.loop_id,
                        "evidence_id": artifact.evidence_id,
                        "wp_id": artifact.wp_id,
                        "mismatch_basis_points": artifact.mismatch_basis_points,
                        "threshold_basis_points": threshold,
                        "target_role": steering.target_role,
                        "receipt_kind": steering.receipt_kind,
                        "schema": "hsk.atelier.visual_steer_feedback@1",
                    }),
                )
                .await?;
            let record: VisualSteerFeedbackRecord = row
                .ok_or_else(|| {
                    AtelierError::Internal(
                        "recording visual steer feedback returned no row".to_owned(),
                    )
                })?
                .into();
            recorded.push(record);
        }
        Ok(recorded)
    }

    /// List the STEER feedback records for one visual debugging loop, newest
    /// first (MT-156).
    pub async fn list_visual_steer_feedback_for_loop(
        &self,
        loop_id: &str,
    ) -> AtelierResult<Vec<VisualSteerFeedbackRecord>> {
        let bindings = LoopIdBinding {
            loop_id: loop_id.to_owned(),
        };
        let rows: Vec<VisualSteerFeedbackRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_VISUAL_STEER_FEEDBACK_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows
            .into_iter()
            .map(VisualSteerFeedbackRecord::from)
            .collect())
    }
}
