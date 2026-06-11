//! WP-KERNEL-005 MT-156: STEER feedback from visual mismatch.
//!
//! Converts visual threshold breaches in a validated
//! [`VisualDebuggingLoopV1`](crate::kernel::visual_debugging_loop::VisualDebuggingLoopV1)
//! into actionable, durable STEER feedback records -- never a silent failure
//! and never generic prose. One record per `(loop_id, evidence_id)` breach is
//! persisted in PostgreSQL (table `atelier_visual_steer_feedback`, migration
//! `0129_atelier_visual_steer_retention.sql`) and mirrored through the Atelier
//! EventLedger so downstream roles can route the STEER receipt.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use crate::kernel::visual_debugging_loop::{
    validate_visual_debugging_loop, VisualDebuggingLoopV1,
};

use super::{AtelierError, AtelierResult, AtelierStore};

pub mod visual_steer_event_family {
    /// A visual threshold breach was converted into a STEER feedback record
    /// (MT-156).
    pub const VISUAL_STEER_FEEDBACK_RECORDED: &str =
        "atelier.visual_steer.feedback_recorded";

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
    /// Receipt kind (always `STEER`; enforced by a DB check constraint).
    pub receipt_kind: String,
    pub code_diff_ref: String,
    pub visual_diff_ref: String,
    /// Concrete, actionable instruction naming the breach and the refs to act
    /// on -- never generic prose.
    pub next_action: String,
    pub created_at_utc: DateTime<Utc>,
}

const VISUAL_STEER_FEEDBACK_COLUMNS: &str =
    "feedback_id, loop_id, evidence_id, wp_id, mismatch_basis_points, \
     threshold_basis_points, target_role, receipt_kind, code_diff_ref, \
     visual_diff_ref, next_action, created_at_utc";

fn visual_steer_feedback_from_row(row: &sqlx::postgres::PgRow) -> VisualSteerFeedbackRecord {
    VisualSteerFeedbackRecord {
        feedback_id: row.get("feedback_id"),
        loop_id: row.get("loop_id"),
        evidence_id: row.get("evidence_id"),
        wp_id: row.get("wp_id"),
        mismatch_basis_points: row.get("mismatch_basis_points"),
        threshold_basis_points: row.get("threshold_basis_points"),
        target_role: row.get("target_role"),
        receipt_kind: row.get("receipt_kind"),
        code_diff_ref: row.get("code_diff_ref"),
        visual_diff_ref: row.get("visual_diff_ref"),
        next_action: row.get("next_action"),
        created_at_utc: row.get("created_at_utc"),
    }
}

impl AtelierStore {
    /// Convert every visual threshold breach in `loop_config` into a durable,
    /// actionable STEER feedback record (MT-156).
    ///
    /// The loop is first validated against the full MT-046 visual-debugging
    /// loop contract; an invalid loop is rejected (no silent failure). For each
    /// evidence artifact whose `mismatch_basis_points` exceeds the configured
    /// `max_pixel_diff_basis_points`, one record is upserted keyed on
    /// `(loop_id, evidence_id)` and one `VISUAL_STEER_FEEDBACK_RECORDED` event
    /// is emitted in the same transaction. A loop without breaches records
    /// nothing and returns an empty list.
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
        let mut tx = self.pool().begin().await?;
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
            let row = sqlx::query(&format!(
                r#"INSERT INTO atelier_visual_steer_feedback
                     (feedback_id, loop_id, evidence_id, wp_id, mismatch_basis_points,
                      threshold_basis_points, target_role, receipt_kind, code_diff_ref,
                      visual_diff_ref, next_action)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                   ON CONFLICT (loop_id, evidence_id) DO UPDATE SET
                     wp_id = EXCLUDED.wp_id,
                     mismatch_basis_points = EXCLUDED.mismatch_basis_points,
                     threshold_basis_points = EXCLUDED.threshold_basis_points,
                     target_role = EXCLUDED.target_role,
                     receipt_kind = EXCLUDED.receipt_kind,
                     code_diff_ref = EXCLUDED.code_diff_ref,
                     visual_diff_ref = EXCLUDED.visual_diff_ref,
                     next_action = EXCLUDED.next_action
                   RETURNING {VISUAL_STEER_FEEDBACK_COLUMNS}"#
            ))
            .bind(&feedback_id)
            .bind(&loop_config.loop_id)
            .bind(&artifact.evidence_id)
            .bind(&artifact.wp_id)
            .bind(artifact.mismatch_basis_points as i32)
            .bind(threshold as i32)
            .bind(&steering.target_role)
            .bind(&steering.receipt_kind)
            .bind(&steering.code_diff_ref)
            .bind(&artifact.visual_diff_artifact_ref)
            .bind(&next_action)
            .fetch_one(&mut *tx)
            .await?;
            let record = visual_steer_feedback_from_row(&row);

            self.record_event_in_tx(
                &mut tx,
                visual_steer_event_family::VISUAL_STEER_FEEDBACK_RECORDED,
                "atelier_visual_steer_feedback",
                &record.feedback_id,
                json!({
                    "feedback_id": record.feedback_id,
                    "loop_id": record.loop_id,
                    "evidence_id": record.evidence_id,
                    "wp_id": record.wp_id,
                    "mismatch_basis_points": record.mismatch_basis_points,
                    "threshold_basis_points": record.threshold_basis_points,
                    "target_role": record.target_role,
                    "receipt_kind": record.receipt_kind,
                    "schema": "hsk.atelier.visual_steer_feedback@1",
                }),
            )
            .await?;
            recorded.push(record);
        }
        tx.commit().await?;
        Ok(recorded)
    }

    /// List the STEER feedback records for one visual debugging loop, newest
    /// first (MT-156).
    pub async fn list_visual_steer_feedback_for_loop(
        &self,
        loop_id: &str,
    ) -> AtelierResult<Vec<VisualSteerFeedbackRecord>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {VISUAL_STEER_FEEDBACK_COLUMNS}
               FROM atelier_visual_steer_feedback
               WHERE loop_id = $1
               ORDER BY created_at_utc DESC, feedback_id ASC"#
        ))
        .bind(loop_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(visual_steer_feedback_from_row).collect())
    }
}
