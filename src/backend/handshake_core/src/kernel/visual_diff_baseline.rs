//! WP-KERNEL-005 MT-155/MT-157: durable visual-diff baseline + diff-request +
//! comparison-result store for the kernel visual debugging loop.
//!
//! MT-155 (Visual Diff Baseline Contract): screenshot baselines and the
//! standalone diff-request schema (threshold + metadata) persist in
//! embedded SurrealDB tables instead of living only as embedded
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
use serde_json::Value as JsonValue;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid, Value};
use uuid::Uuid;

use crate::atelier::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

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
    pub metadata: JsonValue,
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
    pub metadata: JsonValue,
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

#[derive(SurrealValue)]
struct BaselineRow {
    baseline_id: SurrealUuid,
    surface_id: String,
    baseline_ref: String,
    content_sha256: String,
    captured_by: String,
    captured_at_utc: Datetime,
    created_at_utc: Datetime,
}

impl From<BaselineRow> for VisualDiffBaselineRecord {
    fn from(row: BaselineRow) -> Self {
        Self {
            baseline_id: row.baseline_id.into(),
            surface_id: row.surface_id,
            baseline_ref: row.baseline_ref,
            content_sha256: row.content_sha256,
            captured_by: row.captured_by,
            captured_at_utc: row.captured_at_utc.into(),
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct RequestRow {
    request_id: SurrealUuid,
    surface_id: String,
    baseline_id: Option<SurrealUuid>,
    previous_screenshot_ref: Option<String>,
    candidate_screenshot_ref: String,
    comparison_mode: String,
    threshold_config_ref: String,
    max_pixel_diff_basis_points: i64,
    max_layout_shift_basis_points: i64,
    structural_mismatch_limit: i64,
    metadata_json: JsonValue,
    requested_by: String,
    created_at_utc: Datetime,
}

impl TryFrom<RequestRow> for VisualDiffRequestRecord {
    type Error = AtelierError;

    fn try_from(row: RequestRow) -> AtelierResult<Self> {
        let comparison_mode =
            VisualComparisonMode::from_token(&row.comparison_mode).ok_or_else(|| {
                AtelierError::Validation(format!(
                    "unknown comparison_mode token: {}",
                    row.comparison_mode
                ))
            })?;
        let reference = match (row.baseline_id, row.previous_screenshot_ref) {
            (Some(baseline_id), None) => VisualDiffReference::Baseline {
                baseline_id: baseline_id.into(),
            },
            (None, Some(previous_screenshot_ref)) => VisualDiffReference::PreviousScreenshot {
                previous_screenshot_ref,
            },
            _ => {
                return Err(AtelierError::Validation(
                    "diff request row must carry exactly one baseline reference".into(),
                ))
            }
        };
        Ok(Self {
            request_id: row.request_id.into(),
            surface_id: row.surface_id,
            reference,
            candidate_screenshot_ref: row.candidate_screenshot_ref,
            comparison_mode,
            threshold_config: VisualDebuggingThresholdConfigV1 {
                threshold_config_ref: row.threshold_config_ref,
                max_pixel_diff_basis_points: u32::try_from(row.max_pixel_diff_basis_points)
                    .map_err(|_| AtelierError::Validation("pixel threshold out of range".into()))?,
                max_layout_shift_basis_points: u32::try_from(row.max_layout_shift_basis_points)
                    .map_err(|_| {
                        AtelierError::Validation("layout threshold out of range".into())
                    })?,
                structural_mismatch_limit: u32::try_from(row.structural_mismatch_limit).map_err(
                    |_| AtelierError::Validation("structural threshold out of range".into()),
                )?,
            },
            metadata: row.metadata_json,
            requested_by: row.requested_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct ResultRow {
    result_id: SurrealUuid,
    request_id: SurrealUuid,
    comparison_mode: String,
    units_compared: i64,
    units_differing: i64,
    mismatch_basis_points: i64,
    threshold_exceeded: bool,
    outcome: String,
    computed_at_utc: Datetime,
    created_at_utc: Datetime,
}

impl TryFrom<ResultRow> for VisualDiffResultRecord {
    type Error = AtelierError;

    fn try_from(row: ResultRow) -> AtelierResult<Self> {
        let comparison_mode =
            VisualComparisonMode::from_token(&row.comparison_mode).ok_or_else(|| {
                AtelierError::Validation(format!(
                    "unknown comparison_mode token: {}",
                    row.comparison_mode
                ))
            })?;
        let outcome = VisualDiffOutcome::from_token(&row.outcome).ok_or_else(|| {
            AtelierError::Validation(format!("unknown outcome token: {}", row.outcome))
        })?;
        Ok(Self {
            result_id: row.result_id.into(),
            request_id: row.request_id.into(),
            computation: VisualDiffComputationV1 {
                comparison_mode,
                units_compared: u64::try_from(row.units_compared)
                    .map_err(|_| AtelierError::Validation("units_compared out of range".into()))?,
                units_differing: u64::try_from(row.units_differing)
                    .map_err(|_| AtelierError::Validation("units_differing out of range".into()))?,
                mismatch_basis_points: u32::try_from(row.mismatch_basis_points).map_err(|_| {
                    AtelierError::Validation("mismatch basis points out of range".into())
                })?,
                threshold_exceeded: row.threshold_exceeded,
                outcome,
            },
            computed_at_utc: row.computed_at_utc.into(),
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct BaselineBindings {
    record_id: RecordId,
    baseline_id: SurrealUuid,
    surface_id: String,
    baseline_ref: String,
    content_sha256: String,
    captured_by: String,
    captured_at_utc: Datetime,
}

#[derive(Clone, SurrealValue)]
struct RequestBindings {
    record_id: RecordId,
    request_id: SurrealUuid,
    surface_id: String,
    baseline_id: Option<SurrealUuid>,
    previous_screenshot_ref: Option<String>,
    candidate_screenshot_ref: String,
    comparison_mode: String,
    threshold_config_ref: String,
    max_pixel_diff_basis_points: i64,
    max_layout_shift_basis_points: i64,
    structural_mismatch_limit: i64,
    metadata_json: Value,
    requested_by: String,
}

#[derive(Clone, SurrealValue)]
struct ResultBindings {
    record_id: RecordId,
    result_id: SurrealUuid,
    request_id: SurrealUuid,
    comparison_mode: String,
    units_compared: i64,
    units_differing: i64,
    mismatch_basis_points: i64,
    threshold_exceeded: bool,
    outcome: String,
    computed_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct UuidBinding {
    id: SurrealUuid,
}
#[derive(SurrealValue)]
struct SurfaceBinding {
    surface_id: String,
}

const RECORD_BASELINE: &str = concat!(
    "RETURN { LET $row = (CREATE $domain.record_id CONTENT { baseline_id: $domain.baseline_id, surface_id: $domain.surface_id, baseline_ref: $domain.baseline_ref, content_sha256: $domain.content_sha256, captured_by: $domain.captured_by, captured_at_utc: $domain.captured_at_utc } RETURN AFTER)[0]; ",
    atelier_event_sql!(), " RETURN $row; };"
);
const RECORD_REQUEST: &str = concat!(
    "RETURN { LET $row = (CREATE $domain.record_id CONTENT { request_id: $domain.request_id, surface_id: $domain.surface_id, baseline_id: $domain.baseline_id, previous_screenshot_ref: $domain.previous_screenshot_ref, candidate_screenshot_ref: $domain.candidate_screenshot_ref, comparison_mode: $domain.comparison_mode, threshold_config_ref: $domain.threshold_config_ref, max_pixel_diff_basis_points: $domain.max_pixel_diff_basis_points, max_layout_shift_basis_points: $domain.max_layout_shift_basis_points, structural_mismatch_limit: $domain.structural_mismatch_limit, metadata_json: $domain.metadata_json, requested_by: $domain.requested_by } RETURN AFTER)[0]; ",
    atelier_event_sql!(), " RETURN $row; };"
);
const RECORD_RESULT: &str = concat!(
    "RETURN { LET $row = (CREATE $domain.record_id CONTENT { result_id: $domain.result_id, request_id: $domain.request_id, comparison_mode: $domain.comparison_mode, units_compared: $domain.units_compared, units_differing: $domain.units_differing, mismatch_basis_points: $domain.mismatch_basis_points, threshold_exceeded: $domain.threshold_exceeded, outcome: $domain.outcome, computed_at_utc: $domain.computed_at_utc } RETURN AFTER)[0]; ",
    atelier_event_sql!(), " RETURN $row; };"
);

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
        let row: Option<BaselineRow> = self
            .write_with_event(
                RECORD_BASELINE,
                BaselineBindings {
                    record_id: RecordId::new(
                        "kernel_visual_diff_baseline",
                        SurrealUuid::from(baseline_id),
                    ),
                    baseline_id: baseline_id.into(),
                    surface_id: new.surface_id.clone(),
                    baseline_ref: new.baseline_ref.clone(),
                    content_sha256: new.content_sha256.clone(),
                    captured_by: new.captured_by.clone(),
                    captured_at_utc: new.captured_at_utc.into(),
                },
                kernel_visual_diff_event_family::BASELINE_RECORDED,
                "kernel_visual_diff_baseline",
                &baseline_id.to_string(),
                serde_json::json!({
                    "schema": KERNEL_VISUAL_DIFF_BASELINE_SCHEMA,
                    "baseline_id": baseline_id,
                    "surface_id": new.surface_id,
                    "baseline_ref": new.baseline_ref,
                    "content_sha256": new.content_sha256,
                }),
            )
            .await?;
        row.map(Into::into).ok_or_else(|| {
            AtelierError::Internal("recording visual diff baseline returned no row".into())
        })
    }

    /// Fetch a registered baseline by id, if recorded.
    pub async fn get_visual_diff_baseline(
        &self,
        baseline_id: Uuid,
    ) -> AtelierResult<Option<VisualDiffBaselineRecord>> {
        let row: Option<BaselineRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "SELECT baseline_id, surface_id, baseline_ref, content_sha256, captured_by, captured_at_utc, created_at_utc FROM kernel_visual_diff_baseline WHERE baseline_id = $id LIMIT 1;",
                        UuidBinding { id: baseline_id.into() },
                    )
                    .await
                })
            })
            .await?;
        Ok(row.map(Into::into))
    }

    /// Latest baseline for a surface (newest capture wins) — the "baseline"
    /// side of the baseline-or-previous comparison contract.
    pub async fn latest_visual_diff_baseline_for_surface(
        &self,
        surface_id: &str,
    ) -> AtelierResult<Option<VisualDiffBaselineRecord>> {
        let bindings = SurfaceBinding {
            surface_id: surface_id.to_owned(),
        };
        let row: Option<BaselineRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "SELECT baseline_id, surface_id, baseline_ref, content_sha256, captured_by, captured_at_utc, created_at_utc FROM kernel_visual_diff_baseline WHERE surface_id = $surface_id ORDER BY captured_at_utc DESC, baseline_id DESC LIMIT 1;",
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        Ok(row.map(Into::into))
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
        let row: Option<RequestRow> = self
            .write_with_event(
                RECORD_REQUEST,
                RequestBindings {
                    record_id: RecordId::new(
                        "kernel_visual_diff_request",
                        SurrealUuid::from(request_id),
                    ),
                    request_id: request_id.into(),
                    surface_id: new.surface_id.clone(),
                    baseline_id: baseline_id.map(Into::into),
                    previous_screenshot_ref,
                    candidate_screenshot_ref: new.candidate_screenshot_ref.clone(),
                    comparison_mode: new.comparison_mode.as_token().to_owned(),
                    threshold_config_ref: new.threshold_config.threshold_config_ref.clone(),
                    max_pixel_diff_basis_points: i64::from(
                        new.threshold_config.max_pixel_diff_basis_points,
                    ),
                    max_layout_shift_basis_points: i64::from(
                        new.threshold_config.max_layout_shift_basis_points,
                    ),
                    structural_mismatch_limit: i64::from(
                        new.threshold_config.structural_mismatch_limit,
                    ),
                    metadata_json: SurrealValue::into_value(new.metadata.clone()),
                    requested_by: new.requested_by.clone(),
                },
                kernel_visual_diff_event_family::REQUEST_RECORDED,
                "kernel_visual_diff_request",
                &request_id.to_string(),
                serde_json::json!({
                    "schema": KERNEL_VISUAL_DIFF_REQUEST_SCHEMA,
                    "request_id": request_id,
                    "surface_id": new.surface_id,
                    "comparison_mode": new.comparison_mode.as_token(),
                    "reference": new.reference,
                    "threshold_config_ref": new.threshold_config.threshold_config_ref,
                    "max_pixel_diff_basis_points": new.threshold_config.max_pixel_diff_basis_points,
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording visual diff request returned no row".into())
        })?
        .try_into()
    }

    /// Fetch a visual-diff request by id, if recorded.
    pub async fn get_visual_diff_request(
        &self,
        request_id: Uuid,
    ) -> AtelierResult<Option<VisualDiffRequestRecord>> {
        let row: Option<RequestRow> = self.store().with_data_operation(move |ctx| Box::pin(async move {
            ctx.query_first(
                "SELECT request_id, surface_id, baseline_id, previous_screenshot_ref, candidate_screenshot_ref, comparison_mode, threshold_config_ref, max_pixel_diff_basis_points, max_layout_shift_basis_points, structural_mismatch_limit, metadata_json, requested_by, created_at_utc FROM kernel_visual_diff_request WHERE request_id = $id LIMIT 1;",
                UuidBinding { id: request_id.into() },
            ).await
        })).await?;
        row.map(TryInto::try_into).transpose()
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
        let row: Option<ResultRow> = self
            .write_with_event(
                RECORD_RESULT,
                ResultBindings {
                    record_id: RecordId::new(
                        "kernel_visual_diff_result",
                        SurrealUuid::from(result_id),
                    ),
                    result_id: result_id.into(),
                    request_id: request_id.into(),
                    comparison_mode: computation.comparison_mode.as_token().to_owned(),
                    units_compared: i64::try_from(computation.units_compared).map_err(|_| {
                        AtelierError::Validation("units_compared out of range".into())
                    })?,
                    units_differing: i64::try_from(computation.units_differing).map_err(|_| {
                        AtelierError::Validation("units_differing out of range".into())
                    })?,
                    mismatch_basis_points: i64::from(computation.mismatch_basis_points),
                    threshold_exceeded: computation.threshold_exceeded,
                    outcome: computation.outcome.as_token().to_owned(),
                    computed_at_utc: computed_at_utc.into(),
                },
                kernel_visual_diff_event_family::RESULT_RECORDED,
                "kernel_visual_diff_request",
                &request_id.to_string(),
                serde_json::json!({
                    "schema": KERNEL_VISUAL_DIFF_RESULT_SCHEMA,
                    "result_id": result_id,
                    "request_id": request_id,
                    "comparison_mode": computation.comparison_mode.as_token(),
                    "units_compared": computation.units_compared,
                    "units_differing": computation.units_differing,
                    "mismatch_basis_points": computation.mismatch_basis_points,
                    "threshold_exceeded": computation.threshold_exceeded,
                    "outcome": computation.outcome.as_token(),
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording visual diff result returned no row".into())
        })?
        .try_into()
    }

    /// Results recorded for a request, newest first.
    pub async fn list_visual_diff_results_for_request(
        &self,
        request_id: Uuid,
    ) -> AtelierResult<Vec<VisualDiffResultRecord>> {
        let rows: Vec<ResultRow> = self.store().with_data_operation(move |ctx| Box::pin(async move {
            ctx.query_values(
                "SELECT result_id, request_id, comparison_mode, units_compared, units_differing, mismatch_basis_points, threshold_exceeded, outcome, computed_at_utc, created_at_utc FROM kernel_visual_diff_result WHERE request_id = $id ORDER BY created_at_utc DESC, result_id DESC;",
                UuidBinding { id: request_id.into() },
            ).await
        })).await?;
        rows.into_iter().map(TryInto::try_into).collect()
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
