//! WP-KERNEL-005 MT-155/MT-157: durable visual-diff baseline + diff-request +
//! comparison-result store for the kernel visual debugging loop.
//!
//! MT-155 (Visual Diff Baseline Contract): screenshot baselines and the
//! standalone diff-request schema (threshold + metadata) persist in
//! PostgreSQL (tables from migration 0124, applied by the kernel migration
//! runner `Database::run_migrations`) instead of living only as embedded
//! fields of the in-memory [`crate::kernel::visual_debugging_loop`]
//! projection. A diff request binds EITHER a registered baseline row OR the
//! previous screenshot artifact ref — the "baseline-or-previous" comparison
//! contract.
//!
//! MT-157 (Pixel Versus Structural Comparison): a computed
//! [`VisualDiffComputationV1`] (units compared/differing, mismatch basis
//! points, threshold verdict, outcome) persists against its request so the
//! result fields are durable, re-readable evidence. The persisted
//! `comparison_mode` must match the request's mode; `manual` results park in
//! `manual_review_required` until an operator verdict is recorded.
//!
//! Every record emits its `kernel.visual_diff.*` EventLedger family in the
//! same transaction (same pattern as
//! `diagnostics::bundle_manifest::record_kernel_diagnostic_bundle_manifest`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::atelier::{AtelierError, AtelierResult, AtelierStore, reject_legacy_runtime_ref};

use super::visual_debugging_loop::{
    VisualComparisonMode, VisualDebuggingThresholdConfigV1, VisualDiffComputationV1,
    VisualDiffOutcome,
};

/// Stable schema id stamped on every persisted baseline row.
pub const KERNEL_VISUAL_DIFF_BASELINE_SCHEMA: &str = "hsk.kernel.visual_diff_baseline@1";
/// Stable schema id stamped on every persisted diff-request row.
pub const KERNEL_VISUAL_DIFF_REQUEST_SCHEMA: &str = "hsk.kernel.visual_diff_request@1";
/// Stable schema id stamped on every persisted comparison-result row.
pub const KERNEL_VISUAL_DIFF_RESULT_SCHEMA: &str = "hsk.kernel.visual_diff_result@1";

pub mod kernel_visual_diff_event_family {
    pub const BASELINE_RECORDED: &str = "kernel.visual_diff.baseline_recorded";
    pub const REQUEST_RECORDED: &str = "kernel.visual_diff.request_recorded";
    pub const RESULT_RECORDED: &str = "kernel.visual_diff.result_recorded";

    pub const ALL: &[&str] = &[BASELINE_RECORDED, REQUEST_RECORDED, RESULT_RECORDED];
}

const BASELINE_REF_PREFIX: &str = "artifact://baselines/";
const SCREENSHOT_REF_PREFIX: &str = "artifact://screenshots/";
const THRESHOLD_CONFIG_REF_PREFIX: &str = "packet://";

/// New screenshot baseline registration for a GUI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewVisualDiffBaseline {
    pub surface_id: String,
    pub baseline_ref: String,
    pub content_sha256: String,
    pub captured_by: String,
    pub captured_at_utc: DateTime<Utc>,
}

/// Persisted screenshot baseline row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDiffBaselineRecord {
    pub baseline_id: Uuid,
    pub surface_id: String,
    pub baseline_ref: String,
    pub content_sha256: String,
    pub captured_by: String,
    pub captured_at_utc: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

/// The reference side of a diff request: a registered baseline row or the
/// previous screenshot artifact (the "baseline-or-previous" contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum VisualDiffReference {
    Baseline { baseline_id: Uuid },
    PreviousScreenshot { previous_screenshot_ref: String },
}

/// New standalone diff request with thresholds and metadata (MT-155).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewVisualDiffRequest {
    pub surface_id: String,
    pub reference: VisualDiffReference,
    pub candidate_screenshot_ref: String,
    pub comparison_mode: VisualComparisonMode,
    pub threshold_config: VisualDebuggingThresholdConfigV1,
    pub metadata: Value,
    pub requested_by: String,
}

/// Persisted diff request row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDiffRequestRecord {
    pub request_id: Uuid,
    pub surface_id: String,
    pub reference: VisualDiffReference,
    pub candidate_screenshot_ref: String,
    pub comparison_mode: VisualComparisonMode,
    pub threshold_config: VisualDebuggingThresholdConfigV1,
    pub metadata: Value,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Persisted comparison result row (MT-157 result fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDiffResultRecord {
    pub result_id: Uuid,
    pub request_id: Uuid,
    pub computation: VisualDiffComputationV1,
    pub computed_at_utc: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

fn validate_new_baseline(new: &NewVisualDiffBaseline) -> AtelierResult<()> {
    require_token("surface_id", &new.surface_id)?;
    require_token("captured_by", &new.captured_by)?;
    reject_legacy_runtime_ref("baseline_ref", &new.baseline_ref)?;
    if !new.baseline_ref.starts_with(BASELINE_REF_PREFIX) {
        return Err(AtelierError::Validation(format!(
            "baseline_ref must start with {BASELINE_REF_PREFIX}; got {}",
            new.baseline_ref
        )));
    }
    let is_sha256 = new
        .content_sha256
        .strip_prefix("sha256:")
        .is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        });
    if !is_sha256 {
        return Err(AtelierError::Validation(
            "content_sha256 must be a sha256:<64 lowercase hex> token".into(),
        ));
    }
    Ok(())
}

fn validate_new_request(new: &NewVisualDiffRequest) -> AtelierResult<()> {
    require_token("surface_id", &new.surface_id)?;
    require_token("requested_by", &new.requested_by)?;
    reject_legacy_runtime_ref("candidate_screenshot_ref", &new.candidate_screenshot_ref)?;
    if !new
        .candidate_screenshot_ref
        .starts_with(SCREENSHOT_REF_PREFIX)
    {
        return Err(AtelierError::Validation(format!(
            "candidate_screenshot_ref must start with {SCREENSHOT_REF_PREFIX}; got {}",
            new.candidate_screenshot_ref
        )));
    }
    if let VisualDiffReference::PreviousScreenshot {
        previous_screenshot_ref,
    } = &new.reference
    {
        reject_legacy_runtime_ref("previous_screenshot_ref", previous_screenshot_ref)?;
        if !previous_screenshot_ref.starts_with(SCREENSHOT_REF_PREFIX) {
            return Err(AtelierError::Validation(format!(
                "previous_screenshot_ref must start with {SCREENSHOT_REF_PREFIX}; got \
                 {previous_screenshot_ref}"
            )));
        }
    }
    let threshold = &new.threshold_config;
    reject_legacy_runtime_ref("threshold_config_ref", &threshold.threshold_config_ref)?;
    if !threshold
        .threshold_config_ref
        .starts_with(THRESHOLD_CONFIG_REF_PREFIX)
    {
        return Err(AtelierError::Validation(format!(
            "threshold_config_ref must start with {THRESHOLD_CONFIG_REF_PREFIX} (thresholds are \
             configured from the task packet or refinement); got {}",
            threshold.threshold_config_ref
        )));
    }
    if threshold.max_pixel_diff_basis_points == 0 {
        return Err(AtelierError::Validation(
            "max_pixel_diff_basis_points must be positive".into(),
        ));
    }
    if threshold.max_layout_shift_basis_points == 0 {
        return Err(AtelierError::Validation(
            "max_layout_shift_basis_points must be positive".into(),
        ));
    }
    if !new.metadata.is_object() {
        return Err(AtelierError::Validation(
            "metadata must be a JSON object".into(),
        ));
    }
    Ok(())
}

fn require_token(field: &str, value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn baseline_from_row(row: &sqlx::postgres::PgRow) -> VisualDiffBaselineRecord {
    VisualDiffBaselineRecord {
        baseline_id: row.get("baseline_id"),
        surface_id: row.get("surface_id"),
        baseline_ref: row.get("baseline_ref"),
        content_sha256: row.get("content_sha256"),
        captured_by: row.get("captured_by"),
        captured_at_utc: row.get("captured_at_utc"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn request_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<VisualDiffRequestRecord> {
    let mode_token: String = row.get("comparison_mode");
    let comparison_mode = VisualComparisonMode::from_token(&mode_token).ok_or_else(|| {
        AtelierError::Validation(format!("unknown comparison_mode token: {mode_token}"))
    })?;
    let baseline_id: Option<Uuid> = row.get("baseline_id");
    let previous_screenshot_ref: Option<String> = row.get("previous_screenshot_ref");
    let reference = match (baseline_id, previous_screenshot_ref) {
        (Some(baseline_id), None) => VisualDiffReference::Baseline { baseline_id },
        (None, Some(previous_screenshot_ref)) => VisualDiffReference::PreviousScreenshot {
            previous_screenshot_ref,
        },
        _ => {
            return Err(AtelierError::Validation(
                "diff request row must carry exactly one of baseline_id / \
                 previous_screenshot_ref"
                    .into(),
            ));
        }
    };
    let pixel: i32 = row.get("max_pixel_diff_basis_points");
    let layout: i32 = row.get("max_layout_shift_basis_points");
    let structural: i32 = row.get("structural_mismatch_limit");
    Ok(VisualDiffRequestRecord {
        request_id: row.get("request_id"),
        surface_id: row.get("surface_id"),
        reference,
        candidate_screenshot_ref: row.get("candidate_screenshot_ref"),
        comparison_mode,
        threshold_config: VisualDebuggingThresholdConfigV1 {
            threshold_config_ref: row.get("threshold_config_ref"),
            max_pixel_diff_basis_points: pixel as u32,
            max_layout_shift_basis_points: layout as u32,
            structural_mismatch_limit: structural as u32,
        },
        metadata: row.get("metadata_json"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn result_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<VisualDiffResultRecord> {
    let mode_token: String = row.get("comparison_mode");
    let comparison_mode = VisualComparisonMode::from_token(&mode_token).ok_or_else(|| {
        AtelierError::Validation(format!("unknown comparison_mode token: {mode_token}"))
    })?;
    let outcome_token: String = row.get("outcome");
    let outcome = VisualDiffOutcome::from_token(&outcome_token).ok_or_else(|| {
        AtelierError::Validation(format!("unknown outcome token: {outcome_token}"))
    })?;
    let units_compared: i64 = row.get("units_compared");
    let units_differing: i64 = row.get("units_differing");
    let mismatch_basis_points: i32 = row.get("mismatch_basis_points");
    Ok(VisualDiffResultRecord {
        result_id: row.get("result_id"),
        request_id: row.get("request_id"),
        computation: VisualDiffComputationV1 {
            comparison_mode,
            units_compared: units_compared as u64,
            units_differing: units_differing as u64,
            mismatch_basis_points: mismatch_basis_points as u32,
            threshold_exceeded: row.get("threshold_exceeded"),
            outcome,
        },
        computed_at_utc: row.get("computed_at_utc"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const BASELINE_COLUMNS: &str = "baseline_id, surface_id, baseline_ref, content_sha256, \
                                captured_by, captured_at_utc, created_at_utc";
const REQUEST_COLUMNS: &str = "request_id, surface_id, baseline_id, previous_screenshot_ref, \
                               candidate_screenshot_ref, comparison_mode, threshold_config_ref, \
                               max_pixel_diff_basis_points, max_layout_shift_basis_points, \
                               structural_mismatch_limit, metadata_json, requested_by, \
                               created_at_utc";
const RESULT_COLUMNS: &str = "result_id, request_id, comparison_mode, units_compared, \
                              units_differing, mismatch_basis_points, threshold_exceeded, \
                              outcome, computed_at_utc, created_at_utc";

impl AtelierStore {
    /// Register a screenshot baseline for a GUI surface, emitting the
    /// `kernel.visual_diff.baseline_recorded` EventLedger family in the same
    /// transaction.
    pub async fn record_visual_diff_baseline(
        &self,
        new: &NewVisualDiffBaseline,
    ) -> AtelierResult<VisualDiffBaselineRecord> {
        validate_new_baseline(new)?;

        let baseline_id = Uuid::now_v7();
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            "INSERT INTO kernel_visual_diff_baseline
                 (baseline_id, surface_id, baseline_ref, content_sha256,
                  captured_by, captured_at_utc)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING {BASELINE_COLUMNS}"
        ))
        .bind(baseline_id)
        .bind(&new.surface_id)
        .bind(&new.baseline_ref)
        .bind(&new.content_sha256)
        .bind(&new.captured_by)
        .bind(new.captured_at_utc)
        .fetch_one(&mut *tx)
        .await?;
        let baseline = baseline_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            kernel_visual_diff_event_family::BASELINE_RECORDED,
            "kernel_visual_diff_baseline",
            &baseline.baseline_id.to_string(),
            serde_json::json!({
                "schema": KERNEL_VISUAL_DIFF_BASELINE_SCHEMA,
                "baseline_id": baseline.baseline_id,
                "surface_id": baseline.surface_id,
                "baseline_ref": baseline.baseline_ref,
                "content_sha256": baseline.content_sha256,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(baseline)
    }

    /// Fetch a registered baseline by id, if recorded.
    pub async fn get_visual_diff_baseline(
        &self,
        baseline_id: Uuid,
    ) -> AtelierResult<Option<VisualDiffBaselineRecord>> {
        let row = sqlx::query(&format!(
            "SELECT {BASELINE_COLUMNS}
             FROM kernel_visual_diff_baseline
             WHERE baseline_id = $1"
        ))
        .bind(baseline_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(baseline_from_row))
    }

    /// Latest baseline for a surface (newest capture wins) — the "baseline"
    /// side of the baseline-or-previous comparison contract.
    pub async fn latest_visual_diff_baseline_for_surface(
        &self,
        surface_id: &str,
    ) -> AtelierResult<Option<VisualDiffBaselineRecord>> {
        let row = sqlx::query(&format!(
            "SELECT {BASELINE_COLUMNS}
             FROM kernel_visual_diff_baseline
             WHERE surface_id = $1
             ORDER BY captured_at_utc DESC, baseline_id DESC
             LIMIT 1"
        ))
        .bind(surface_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(baseline_from_row))
    }

    /// Persist a standalone visual-diff request (threshold + metadata),
    /// emitting the `kernel.visual_diff.request_recorded` EventLedger family
    /// in the same transaction. A baseline reference must point at a
    /// registered baseline row.
    pub async fn record_visual_diff_request(
        &self,
        new: &NewVisualDiffRequest,
    ) -> AtelierResult<VisualDiffRequestRecord> {
        validate_new_request(new)?;

        let (baseline_id, previous_screenshot_ref) = match &new.reference {
            VisualDiffReference::Baseline { baseline_id } => (Some(*baseline_id), None),
            VisualDiffReference::PreviousScreenshot {
                previous_screenshot_ref,
            } => (None, Some(previous_screenshot_ref.clone())),
        };
        if let Some(baseline_id) = baseline_id {
            if self.get_visual_diff_baseline(baseline_id).await?.is_none() {
                return Err(AtelierError::Validation(format!(
                    "diff request references unknown baseline {baseline_id}"
                )));
            }
        }

        let request_id = Uuid::now_v7();
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            "INSERT INTO kernel_visual_diff_request
                 (request_id, surface_id, baseline_id, previous_screenshot_ref,
                  candidate_screenshot_ref, comparison_mode, threshold_config_ref,
                  max_pixel_diff_basis_points, max_layout_shift_basis_points,
                  structural_mismatch_limit, metadata_json, requested_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12)
             RETURNING {REQUEST_COLUMNS}"
        ))
        .bind(request_id)
        .bind(&new.surface_id)
        .bind(baseline_id)
        .bind(previous_screenshot_ref.as_deref())
        .bind(&new.candidate_screenshot_ref)
        .bind(new.comparison_mode.as_token())
        .bind(&new.threshold_config.threshold_config_ref)
        .bind(new.threshold_config.max_pixel_diff_basis_points as i32)
        .bind(new.threshold_config.max_layout_shift_basis_points as i32)
        .bind(new.threshold_config.structural_mismatch_limit as i32)
        .bind(&new.metadata)
        .bind(&new.requested_by)
        .fetch_one(&mut *tx)
        .await?;
        let request = request_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            kernel_visual_diff_event_family::REQUEST_RECORDED,
            "kernel_visual_diff_request",
            &request.request_id.to_string(),
            serde_json::json!({
                "schema": KERNEL_VISUAL_DIFF_REQUEST_SCHEMA,
                "request_id": request.request_id,
                "surface_id": request.surface_id,
                "comparison_mode": request.comparison_mode.as_token(),
                "reference": request.reference,
                "threshold_config_ref": request.threshold_config.threshold_config_ref,
                "max_pixel_diff_basis_points":
                    request.threshold_config.max_pixel_diff_basis_points,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(request)
    }

    /// Fetch a visual-diff request by id, if recorded.
    pub async fn get_visual_diff_request(
        &self,
        request_id: Uuid,
    ) -> AtelierResult<Option<VisualDiffRequestRecord>> {
        let row = sqlx::query(&format!(
            "SELECT {REQUEST_COLUMNS}
             FROM kernel_visual_diff_request
             WHERE request_id = $1"
        ))
        .bind(request_id)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(request_from_row).transpose()
    }

    /// Persist a computed comparison result against its request (MT-157
    /// result fields), emitting the `kernel.visual_diff.result_recorded`
    /// EventLedger family in the same transaction. The computation's mode
    /// must match the request's persisted mode.
    pub async fn record_visual_diff_result(
        &self,
        request_id: Uuid,
        computation: &VisualDiffComputationV1,
        computed_at_utc: DateTime<Utc>,
    ) -> AtelierResult<VisualDiffResultRecord> {
        let request = self
            .get_visual_diff_request(request_id)
            .await?
            .ok_or_else(|| {
                AtelierError::Validation(format!(
                    "visual diff result references unknown request {request_id}"
                ))
            })?;
        if request.comparison_mode != computation.comparison_mode {
            return Err(AtelierError::Validation(format!(
                "comparison mode mismatch: request is {}, result is {}",
                request.comparison_mode.as_token(),
                computation.comparison_mode.as_token()
            )));
        }

        let result_id = Uuid::now_v7();
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            "INSERT INTO kernel_visual_diff_result
                 (result_id, request_id, comparison_mode, units_compared,
                  units_differing, mismatch_basis_points, threshold_exceeded,
                  outcome, computed_at_utc)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING {RESULT_COLUMNS}"
        ))
        .bind(result_id)
        .bind(request_id)
        .bind(computation.comparison_mode.as_token())
        .bind(computation.units_compared as i64)
        .bind(computation.units_differing as i64)
        .bind(computation.mismatch_basis_points as i32)
        .bind(computation.threshold_exceeded)
        .bind(computation.outcome.as_token())
        .bind(computed_at_utc)
        .fetch_one(&mut *tx)
        .await?;
        let result = result_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            kernel_visual_diff_event_family::RESULT_RECORDED,
            "kernel_visual_diff_request",
            &request_id.to_string(),
            serde_json::json!({
                "schema": KERNEL_VISUAL_DIFF_RESULT_SCHEMA,
                "result_id": result.result_id,
                "request_id": request_id,
                "comparison_mode": result.computation.comparison_mode.as_token(),
                "units_compared": result.computation.units_compared,
                "units_differing": result.computation.units_differing,
                "mismatch_basis_points": result.computation.mismatch_basis_points,
                "threshold_exceeded": result.computation.threshold_exceeded,
                "outcome": result.computation.outcome.as_token(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Results recorded for a request, newest first.
    pub async fn list_visual_diff_results_for_request(
        &self,
        request_id: Uuid,
    ) -> AtelierResult<Vec<VisualDiffResultRecord>> {
        let rows = sqlx::query(&format!(
            "SELECT {RESULT_COLUMNS}
             FROM kernel_visual_diff_result
             WHERE request_id = $1
             ORDER BY created_at_utc DESC, result_id DESC"
        ))
        .bind(request_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(result_from_row).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_baseline() -> NewVisualDiffBaseline {
        NewVisualDiffBaseline {
            surface_id: "dcc.session_panel".to_string(),
            baseline_ref: "artifact://baselines/dcc.session_panel/v1.png".to_string(),
            content_sha256: format!("sha256:{}", "a".repeat(64)),
            captured_by: "unit-test".to_string(),
            captured_at_utc: Utc::now(),
        }
    }

    fn sample_request() -> NewVisualDiffRequest {
        NewVisualDiffRequest {
            surface_id: "dcc.session_panel".to_string(),
            reference: VisualDiffReference::PreviousScreenshot {
                previous_screenshot_ref: "artifact://screenshots/dcc.session_panel/prev.png"
                    .to_string(),
            },
            candidate_screenshot_ref: "artifact://screenshots/dcc.session_panel/cand.png"
                .to_string(),
            comparison_mode: VisualComparisonMode::PixelDiff,
            threshold_config: VisualDebuggingThresholdConfigV1 {
                threshold_config_ref: "packet://WP-GUI/visual-thresholds".to_string(),
                max_pixel_diff_basis_points: 250,
                max_layout_shift_basis_points: 100,
                structural_mismatch_limit: 0,
            },
            metadata: json!({ "trigger": "post_commit" }),
            requested_by: "unit-test".to_string(),
        }
    }

    #[test]
    fn baseline_validation_accepts_complete_input() {
        validate_new_baseline(&sample_baseline()).expect("valid baseline");
    }

    #[test]
    fn baseline_validation_rejects_bad_ref_and_hash() {
        let mut bad_ref = sample_baseline();
        bad_ref.baseline_ref = "artifact://screenshots/not-a-baseline.png".to_string();
        assert!(validate_new_baseline(&bad_ref).is_err());

        let mut bad_hash = sample_baseline();
        bad_hash.content_sha256 = "sha256:short".to_string();
        assert!(validate_new_baseline(&bad_hash).is_err());

        let mut gov_ref = sample_baseline();
        gov_ref.baseline_ref = "artifact://baselines/.GOV/spec.png".to_string();
        assert!(validate_new_baseline(&gov_ref).is_err());
    }

    #[test]
    fn request_validation_rejects_bad_refs_thresholds_and_metadata() {
        let mut bad_candidate = sample_request();
        bad_candidate.candidate_screenshot_ref = "artifact://baselines/wrong.png".to_string();
        assert!(validate_new_request(&bad_candidate).is_err());

        let mut bad_previous = sample_request();
        bad_previous.reference = VisualDiffReference::PreviousScreenshot {
            previous_screenshot_ref: "file:c/temp/prev.png".to_string(),
        };
        assert!(validate_new_request(&bad_previous).is_err());

        let mut zero_threshold = sample_request();
        zero_threshold.threshold_config.max_pixel_diff_basis_points = 0;
        assert!(validate_new_request(&zero_threshold).is_err());

        let mut bad_threshold_ref = sample_request();
        bad_threshold_ref.threshold_config.threshold_config_ref =
            "artifact://thresholds".to_string();
        assert!(validate_new_request(&bad_threshold_ref).is_err());

        let mut bad_metadata = sample_request();
        bad_metadata.metadata = json!(["not", "an", "object"]);
        assert!(validate_new_request(&bad_metadata).is_err());
    }
}
