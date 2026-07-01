//! Native Facial-derived Ingest analysis.
//!
//! MT-019 ports the first high-value Facial app capability into Handshake as a
//! native, headless analysis artifact. This module does not shell out to
//! `facial.exe`, and it does not pretend to provide ArcFace/YuNet parity before
//! those model assets are wired into Handshake. Rows that need a real model are
//! explicitly marked unavailable; metadata-only quality lanes are labeled as
//! degraded metadata rather than full pixel/model parity.

use super::facial_native::plugins::{ediffiqa, facet, imagededup, python_ofiq};
use super::facial_native::{
    build_facial_native_run_report, facial_feature_registry, FacialNativeImageContext,
    FacialNativeRunItem, FacialNativeRunReport, FacialNativeRunRequest,
};
use super::facial_native::{identity, landmarks, models};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub const FACIAL_INGEST_ANALYSIS_SCHEMA_ID: &str = "hsk.atelier.facial_ingest_analysis@1";
pub const FACIAL_INGEST_ANALYSIS_RECEIPT_SCHEMA_ID: &str =
    "hsk.atelier.facial_ingest_analysis_receipt@1";

const SUPPORTED_PROFILE_TOKENS: [&str; 4] = ["quality", "dedupe", "identity", "review"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialIngestAnalysisItem {
    pub item_id: String,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path_hint: Option<String>,
    pub file_name: String,
    pub byte_len: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub lane: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateFacialIngestAnalysisRequest {
    pub batch_id: String,
    pub profile: String,
    pub requested_by: String,
    pub items: Vec<FacialIngestAnalysisItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialIngestAnalysisRow {
    pub item_id: String,
    pub source_ref: String,
    pub file_name: String,
    pub lane: String,
    pub byte_len: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub decode_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub megapixels: Option<f64>,
    pub quality_source: String,
    #[serde(default = "legacy_quality_source_family")]
    pub quality_source_family: String,
    #[serde(default = "legacy_quality_feature_id")]
    pub quality_feature_id: String,
    #[serde(default = "legacy_quality_method")]
    pub quality_method: String,
    pub quality_score: u8,
    pub quality_band: String,
    pub headshot_candidate: bool,
    #[serde(default = "legacy_quality_metrics")]
    pub quality_metrics: serde_json::Value,
    #[serde(default = "legacy_ofiq_quality")]
    pub ofiq_quality: serde_json::Value,
    pub duplicate_group_id: String,
    #[serde(default = "legacy_duplicate_group_key")]
    pub duplicate_group_key: String,
    pub duplicate_group_size: usize,
    pub duplicate_role: String,
    pub dedupe_source: String,
    #[serde(default = "legacy_dedupe_source_family")]
    pub dedupe_source_family: String,
    #[serde(default = "legacy_dedupe_feature_id")]
    pub dedupe_feature_id: String,
    #[serde(default = "legacy_dedupe_method")]
    pub dedupe_method: String,
    #[serde(default = "legacy_dedupe_record")]
    pub dedupe_record: serde_json::Value,
    pub identity_proxy_key: String,
    pub identity_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_source_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_feature_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_method: Option<String>,
    pub identity_verdict: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landmark_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_required_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_count_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_box: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_frac: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_crop_sharpness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw_estimate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eyes_open: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ear_left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ear_right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landmark_conf_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_error: Option<String>,
    #[serde(default = "legacy_identity_record")]
    pub identity_record: serde_json::Value,
    pub review_recommendation: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialCapabilityMapRow {
    pub capability: String,
    pub source_feature_key: String,
    pub facial_source_family: String,
    pub native_field: String,
    pub artifact_contract: String,
    pub handshake_status: String,
    pub native_route: String,
    pub provenance_note: String,
    pub required_config_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialIngestAnalysisSummary {
    pub item_count: usize,
    pub decoded_count: usize,
    pub duplicate_group_count: usize,
    pub duplicate_item_count: usize,
    pub quality_band_counts: BTreeMap<String, usize>,
    pub review_recommendation_counts: BTreeMap<String, usize>,
    pub profile: String,
    pub profile_tokens: Vec<String>,
    pub quality_source: String,
    pub identity_source: String,
    #[serde(default = "legacy_identity_provenance")]
    pub identity_provenance: serde_json::Value,
    pub dedupe_source: String,
    #[serde(default = "legacy_native_feature_outputs")]
    pub native_feature_outputs: serde_json::Value,
    pub capability_map: Vec<FacialCapabilityMapRow>,
    pub native_run: FacialNativeRunReport,
}

fn legacy_quality_source_family() -> String {
    "legacy_unknown".to_owned()
}

fn legacy_quality_feature_id() -> String {
    "legacy:quality_proxy".to_owned()
}

fn legacy_quality_method() -> String {
    "legacy_schema_v1_missing_quality_method".to_owned()
}

fn legacy_quality_metrics() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "quality_metrics_field_added_after_initial_v1_artifacts",
    })
}

fn legacy_ofiq_quality() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "ofiq_quality_field_added_after_initial_v1_artifacts",
    })
}

fn legacy_duplicate_group_key() -> String {
    "legacy_missing_duplicate_group_key".to_owned()
}

fn legacy_dedupe_source_family() -> String {
    "legacy_unknown".to_owned()
}

fn legacy_dedupe_feature_id() -> String {
    "legacy:dedupe_proxy".to_owned()
}

fn legacy_dedupe_method() -> String {
    "legacy_schema_v1_missing_dedupe_method".to_owned()
}

fn legacy_dedupe_record() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "dedupe_record_field_added_after_initial_v1_artifacts",
    })
}

fn legacy_identity_record() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "identity_record_field_added_after_initial_v1_artifacts",
    })
}

fn legacy_identity_provenance() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "identity_provenance_field_added_after_initial_v1_artifacts",
    })
}

fn legacy_native_feature_outputs() -> serde_json::Value {
    serde_json::json!({
        "status": "legacy_missing",
        "reason": "native_feature_outputs_field_added_after_initial_v1_artifacts",
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialIngestAnalysisExport {
    pub schema_id: String,
    pub batch_id: String,
    pub profile: String,
    pub profile_tokens: Vec<String>,
    pub requested_by: String,
    pub item_count: usize,
    pub rows: Vec<FacialIngestAnalysisRow>,
    pub summary: FacialIngestAnalysisSummary,
    pub analysis_json: serde_json::Value,
    pub analysis_sha256: String,
    pub receipt_json: serde_json::Value,
    pub receipt_sha256: String,
    pub content_hash: String,
}

pub fn generate_facial_ingest_analysis(
    request: GenerateFacialIngestAnalysisRequest,
) -> Result<FacialIngestAnalysisExport, String> {
    let batch_id = require_ref("batch_id", &request.batch_id)?;
    let requested_by = require_ref("requested_by", &request.requested_by)?;
    if request.items.is_empty() {
        return Err("facial ingest analysis requires at least one canonical item".to_owned());
    }
    if request.items.len() > 50_000 {
        return Err("facial ingest analysis item_count must be <= 50000".to_owned());
    }
    let profile_tokens = normalize_profile_tokens(&request.profile)?;
    let profile = profile_tokens.join("+");

    for item in &request.items {
        require_ref("item_id", &item.item_id)?;
        require_ref("item.source_ref", &item.source_ref)?;
        require_ref("item.file_name", &item.file_name)?;
    }

    let contexts = request
        .items
        .iter()
        .map(|item| {
            let image = inspect_image_ref(&item.source_ref, item.local_path_hint.as_deref());
            FacialNativeImageContext {
                item_id: item.item_id.clone(),
                source_ref: item.source_ref.clone(),
                local_path_hint: item.local_path_hint.clone(),
                file_name: item.file_name.clone(),
                lane: item.lane.clone(),
                byte_len: item.byte_len,
                content_hash: item.content_hash.clone(),
                decode_status: image.decode_status,
                image_width: image.width,
                image_height: image.height,
                megapixels: image.megapixels,
            }
        })
        .collect::<Vec<_>>();
    let dedupe_assignments = imagededup::exact_hash_assignments(&contexts);
    let identity_config = models::discover_identity_model_config_from_env();
    let identity_runtime = identity::FacialIdentityRuntime::load(&identity_config);

    let mut rows = Vec::with_capacity(request.items.len());
    for ctx in &contexts {
        let quality = facet::quality_score(ctx);
        let quality_band = facet::quality_band(quality);
        let assignment = dedupe_assignments
            .get(&ctx.item_id)
            .ok_or_else(|| format!("missing dedupe assignment for {}", ctx.item_id))?;
        let quality_metrics = facet::quality_metrics(ctx, quality);
        let ofiq_quality = python_ofiq::vector_quality(ctx, quality);
        let dedupe_record = imagededup::assignment_payload(ctx, assignment);
        let identity_basis = ctx
            .content_hash
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(ctx.source_ref.as_str())
            .to_owned();
        let identity_proxy_key = format!("facial-identity-proxy-{}", stable_hash(&identity_basis));
        let identity_analysis = identity::analyze_identity(
            ctx,
            &identity_config,
            &identity_runtime,
            identity_proxy_key.clone(),
        );
        let (review_recommendation, reasons) = review_recommendation(
            ctx,
            quality,
            quality_band,
            &assignment.role,
            &identity_analysis.verdict,
        );
        rows.push(FacialIngestAnalysisRow {
            item_id: ctx.item_id.clone(),
            source_ref: ctx.source_ref.clone(),
            file_name: ctx.file_name.clone(),
            lane: ctx.lane.clone(),
            byte_len: ctx.byte_len,
            content_hash: ctx.content_hash.clone(),
            decode_status: ctx.decode_status.clone(),
            image_width: ctx.image_width,
            image_height: ctx.image_height,
            megapixels: ctx.megapixels,
            quality_source: facet::QUALITY_SOURCE.to_owned(),
            quality_source_family: facet::QUALITY_SOURCE_FAMILY.to_owned(),
            quality_feature_id: facet::QUALITY_FEATURE_ID.to_owned(),
            quality_method: facet::QUALITY_METHOD.to_owned(),
            quality_score: quality,
            quality_band: quality_band.to_owned(),
            headshot_candidate: quality >= 68 && ctx.is_decoded(),
            quality_metrics,
            ofiq_quality,
            duplicate_group_id: assignment.group_id.clone(),
            duplicate_group_key: assignment.group_key.clone(),
            duplicate_group_size: assignment.group_size,
            duplicate_role: assignment.role.clone(),
            dedupe_source: assignment.source.clone(),
            dedupe_source_family: imagededup::SOURCE_FAMILY.to_owned(),
            dedupe_feature_id: imagededup::HASH_FEATURE_ID.to_owned(),
            dedupe_method: imagededup::HASH_METHOD.to_owned(),
            dedupe_record,
            identity_proxy_key,
            identity_source: identity_analysis.source,
            identity_source_family: Some(identity_analysis.source_family),
            identity_feature_id: Some(identity_analysis.feature_id),
            identity_method: Some(identity_analysis.method),
            identity_verdict: identity_analysis.verdict,
            identity_model_sha256: identity_analysis.model_sha256,
            detector_source: identity_analysis.detector_source,
            detector_model_sha256: identity_analysis.detector_model_sha256,
            landmark_model_sha256: identity_analysis.landmark_model_sha256,
            identity_threshold: identity_analysis.threshold,
            identity_required_margin: identity_analysis.required_margin,
            identity_count_threshold: identity_analysis.count_threshold,
            face_box: identity_analysis.face_box,
            face_frac: identity_analysis.face_frac,
            face_score: identity_analysis.face_score,
            face_crop_sharpness: identity_analysis.face_crop_sharpness,
            yaw_estimate: identity_analysis.yaw_estimate,
            yaw_ratio: identity_analysis.yaw_ratio,
            eyes_open: identity_analysis.eyes_open,
            ear_left: identity_analysis.ear_left,
            ear_right: identity_analysis.ear_right,
            landmark_conf_min: identity_analysis.landmark_conf_min,
            embedding_sha256: identity_analysis.embedding_sha256,
            embedding_dimensions: identity_analysis.embedding_dimensions,
            identity_error: identity_analysis.error,
            identity_record: identity_analysis.record,
            review_recommendation,
            reasons,
        });
    }

    let native_feature_outputs =
        native_feature_outputs(&contexts, &dedupe_assignments, &rows, &identity_config);
    let summary = summarize_rows(
        &batch_id,
        &profile,
        &requested_by,
        &profile_tokens,
        &rows,
        native_feature_outputs,
        &identity_config,
    )?;
    let analysis_json = serde_json::json!({
        "schema_id": FACIAL_INGEST_ANALYSIS_SCHEMA_ID,
        "batch_id": batch_id,
        "profile": profile,
        "profile_tokens": profile_tokens,
        "item_count": rows.len(),
        "summary": analysis_summary_json(&summary),
        "rows": rows,
    });
    let analysis_sha256 = json_sha256(&analysis_json)?;
    let receipt_json = serde_json::json!({
        "schema_id": FACIAL_INGEST_ANALYSIS_RECEIPT_SCHEMA_ID,
        "analysis_schema_id": FACIAL_INGEST_ANALYSIS_SCHEMA_ID,
        "native_extension_schema_id": summary.native_run.schema_id,
        "batch_id": batch_id,
        "profile": profile,
        "profile_tokens": profile_tokens,
        "requested_by": requested_by,
        "item_count": summary.item_count,
        "decoded_count": summary.decoded_count,
        "duplicate_group_count": summary.duplicate_group_count,
        "duplicate_item_count": summary.duplicate_item_count,
        "quality_source": summary.quality_source,
        "identity_source": summary.identity_source,
        "identity_provenance": summary.identity_provenance,
        "dedupe_source": summary.dedupe_source,
        "native_run": summary.native_run,
        "analysis_sha256": analysis_sha256,
        "capability_map": summary.capability_map,
    });
    let receipt_sha256 = json_sha256(&receipt_json)?;
    let content_hash = stable_hash(&format!(
        "{}|{}|{}|{}|{}",
        FACIAL_INGEST_ANALYSIS_SCHEMA_ID, batch_id, profile, summary.item_count, analysis_sha256
    ));

    let rows = analysis_json
        .get("rows")
        .cloned()
        .ok_or_else(|| "facial analysis JSON missing rows".to_owned())
        .and_then(|value| {
            serde_json::from_value::<Vec<FacialIngestAnalysisRow>>(value)
                .map_err(|err| format!("facial analysis JSON row reparse failed: {err}"))
        })?;
    let profile_tokens = summary.profile_tokens.clone();
    Ok(FacialIngestAnalysisExport {
        schema_id: FACIAL_INGEST_ANALYSIS_SCHEMA_ID.to_owned(),
        batch_id,
        profile,
        profile_tokens,
        requested_by,
        item_count: summary.item_count,
        rows,
        summary,
        analysis_json,
        analysis_sha256,
        receipt_json,
        receipt_sha256,
        content_hash,
    })
}

fn normalize_profile_tokens(raw: &str) -> Result<Vec<String>, String> {
    if raw.trim().is_empty() || raw.trim() != raw {
        return Err("facial profile must not be empty or padded".to_owned());
    }
    let mut seen = BTreeMap::<&'static str, bool>::new();
    for token in raw
        .split(|ch: char| ch == '+' || ch == ',' || ch == ';' || ch == '|' || ch.is_whitespace())
        .filter(|token| !token.is_empty())
    {
        let normalized = token.trim().to_ascii_lowercase().replace('-', "_");
        let canonical = match normalized.as_str() {
            "quality" | "quality_proxy" => "quality",
            "dedupe" | "duplicate" | "duplicates" => "dedupe",
            "identity" | "identity_gate" => "identity",
            "review" | "review_assist" => "review",
            other => return Err(format!("unsupported facial profile token: {other}")),
        };
        seen.insert(canonical, true);
    }
    let tokens = SUPPORTED_PROFILE_TOKENS
        .iter()
        .filter(|token| seen.contains_key(**token))
        .map(|token| (*token).to_owned())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err("facial profile did not contain any supported tokens".to_owned());
    }
    Ok(tokens)
}

fn require_ref(field: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(format!("{field} must not be empty or padded"));
    }
    Ok(trimmed.to_owned())
}

#[derive(Clone, Debug)]
struct ImageProbe {
    decode_status: String,
    width: Option<u32>,
    height: Option<u32>,
    megapixels: Option<f64>,
}

fn inspect_image_ref(source_ref: &str, local_path_hint: Option<&str>) -> ImageProbe {
    if let Some(local_path_hint) = local_path_hint {
        return inspect_image_path(local_path_hint, "resolved_ref");
    }
    if source_ref.contains("://") {
        return ImageProbe {
            decode_status: "external_ref_not_decoded".to_owned(),
            width: None,
            height: None,
            megapixels: None,
        };
    }
    inspect_image_path(source_ref, "local_path")
}

fn inspect_image_path(path_ref: &str, status_prefix: &str) -> ImageProbe {
    let path = Path::new(path_ref);
    if !path.is_file() {
        return ImageProbe {
            decode_status: format!("{status_prefix}_missing_file"),
            width: None,
            height: None,
            megapixels: None,
        };
    }
    match image::image_dimensions(path) {
        Ok((width, height)) => ImageProbe {
            decode_status: "decoded".to_owned(),
            width: Some(width),
            height: Some(height),
            megapixels: Some(
                ((width as f64) * (height as f64) / 1_000_000.0 * 100.0).round() / 100.0,
            ),
        },
        Err(err) => ImageProbe {
            decode_status: format!("{status_prefix}_decode_failed:{}", image_error_kind(&err)),
            width: None,
            height: None,
            megapixels: None,
        },
    }
}

fn image_error_kind(err: &image::ImageError) -> &'static str {
    match err {
        image::ImageError::IoError(_) => "io",
        image::ImageError::Decoding(_) => "decoding",
        image::ImageError::Encoding(_) => "encoding",
        image::ImageError::Parameter(_) => "parameter",
        image::ImageError::Limits(_) => "limits",
        image::ImageError::Unsupported(_) => "unsupported",
    }
}

fn review_recommendation(
    ctx: &FacialNativeImageContext,
    score: u8,
    quality_band: &str,
    duplicate_role: &str,
    identity_verdict: &str,
) -> (String, Vec<String>) {
    let mut reasons = Vec::new();
    reasons.push(format!("quality_score={score}"));
    reasons.push(format!("quality_band={quality_band}"));
    reasons.push(format!("decode_status={}", ctx.decode_status));
    reasons.push(format!("quality_source={}", facet::QUALITY_SOURCE));
    reasons.push(format!("identity_verdict={identity_verdict}"));
    if duplicate_role != "singleton" {
        reasons.push(format!("duplicate_role={duplicate_role}"));
    }
    if !ctx.has_content_hash() {
        reasons.push("missing_content_hash_limits_dedupe".to_owned());
    }

    let recommendation = if matches!(quality_band, "reject" | "weak") {
        "cull"
    } else if duplicate_role == "duplicate"
        || !ctx.is_decoded()
        || quality_band == "usable"
        || matches!(
            identity_verdict,
            identity::IDENTITY_VERDICT_MODEL_UNAVAILABLE
        )
    {
        "review"
    } else {
        "keep"
    };
    (recommendation.to_owned(), reasons)
}

fn native_feature_outputs(
    contexts: &[FacialNativeImageContext],
    dedupe_assignments: &BTreeMap<String, imagededup::DedupeAssignment>,
    rows: &[FacialIngestAnalysisRow],
    identity_config: &models::FacialIdentityModelConfig,
) -> serde_json::Value {
    let mut quality_band_counts = BTreeMap::<String, usize>::new();
    for row in rows {
        *quality_band_counts
            .entry(row.quality_band.clone())
            .or_insert(0) += 1;
    }
    let ofiq_schema = rows
        .first()
        .and_then(|row| row.ofiq_quality.get("schema").cloned())
        .unwrap_or_else(|| {
            serde_json::json!({
                "version": "0.2-handshake-native",
                "dimension_count": 0,
                "dimensions": [],
            })
        });
    let ofiq_missing_source_dimensions = rows
        .first()
        .and_then(|row| row.ofiq_quality.get("missing_source_dimensions").cloned())
        .unwrap_or_else(|| serde_json::json!([]));
    let identity_records = rows
        .iter()
        .map(|row| row.identity_record.clone())
        .collect::<Vec<_>>();

    serde_json::json!({
        "facet:quality_pass": {
            "feature_id": facet::QUALITY_FEATURE_ID,
            "source_family": facet::QUALITY_SOURCE_FAMILY,
            "source": facet::QUALITY_SOURCE,
            "method": facet::QUALITY_METHOD,
            "status": "metadata_only_degraded",
            "count": rows.len(),
            "quality_band_counts": quality_band_counts,
            "limitations": [
                "metadata_only_quality_not_pixel_analysis_parity",
                "no_real_face_detector",
                "no_landmark_eye_sharpness",
            ],
        },
        "python-ofiq:setup_data": {
            "feature_id": python_ofiq::SETUP_FEATURE_ID,
            "source_family": python_ofiq::SOURCE_FAMILY,
            "source": python_ofiq::SOURCE,
            "method": python_ofiq::METHOD,
            "status": "metadata_only_degraded",
            "schema": ofiq_schema.clone(),
            "missing_source_dimensions": ofiq_missing_source_dimensions.clone(),
            "thresholds": {
                "scalar_quality_headshot_min": 68.0,
                "vector_quality_gap_tolerance": 25.0,
                "quality_score_range": [0.0, 100.0],
            },
        },
        "python-ofiq:scalar_quality": {
            "feature_id": python_ofiq::SCALAR_FEATURE_ID,
            "source_family": python_ofiq::SOURCE_FAMILY,
            "source": python_ofiq::SOURCE,
            "method": python_ofiq::METHOD,
            "status": "metadata_only_degraded",
            "count": rows.len(),
        },
        "python-ofiq:vector_quality": {
            "feature_id": python_ofiq::VECTOR_FEATURE_ID,
            "source_family": python_ofiq::SOURCE_FAMILY,
            "source": python_ofiq::SOURCE,
            "method": python_ofiq::METHOD,
            "status": "metadata_only_degraded",
            "count": rows.len(),
            "schema": ofiq_schema,
            "missing_source_dimensions": ofiq_missing_source_dimensions,
        },
        "imagededup:hash_duplicates": imagededup::group_summary(contexts, dedupe_assignments),
        "imagededup:remove_candidates": imagededup::remove_candidates(contexts, dedupe_assignments),
        "identity_gate:arcface_embedding": identity::native_feature_output(identity_config, &identity_records),
        "identity_gate:pipnet_landmarks": landmarks::landmark_feature_output(identity_config),
        "ediffiqa": {
            "source_family": ediffiqa::SOURCE_FAMILY,
            "status": "unavailable",
            "features": ediffiqa::unavailable_records(),
        },
    })
}

fn summarize_rows(
    batch_id: &str,
    profile: &str,
    requested_by: &str,
    profile_tokens: &[String],
    rows: &[FacialIngestAnalysisRow],
    native_feature_outputs: serde_json::Value,
    identity_config: &models::FacialIdentityModelConfig,
) -> Result<FacialIngestAnalysisSummary, String> {
    let mut quality_band_counts = BTreeMap::new();
    let mut review_recommendation_counts = BTreeMap::new();
    let mut identity_source_counts = BTreeMap::<String, usize>::new();
    for row in rows {
        *quality_band_counts
            .entry(row.quality_band.clone())
            .or_insert(0) += 1;
        *review_recommendation_counts
            .entry(row.review_recommendation.clone())
            .or_insert(0) += 1;
        *identity_source_counts
            .entry(row.identity_source.clone())
            .or_insert(0) += 1;
    }
    let duplicate_group_count = rows
        .iter()
        .filter(|row| row.duplicate_group_size > 1 && row.duplicate_role == "representative")
        .count();
    let duplicate_item_count = rows
        .iter()
        .filter(|row| row.duplicate_group_size > 1)
        .count();
    let native_run = build_facial_native_run_report(FacialNativeRunRequest {
        batch_id: batch_id.to_owned(),
        profile: profile.to_owned(),
        requested_by: requested_by.to_owned(),
        profile_tokens: profile_tokens.to_vec(),
        items: rows
            .iter()
            .map(|row| FacialNativeRunItem {
                item_id: row.item_id.clone(),
                source_ref: row.source_ref.clone(),
                lane: row.lane.clone(),
                decode_status: row.decode_status.clone(),
                content_hash: row.content_hash.clone(),
            })
            .collect(),
    })?;

    let identity_source = if identity_source_counts.len() == 1 {
        identity_source_counts
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| identity::IDENTITY_SOURCE_NO_MODEL.to_owned())
    } else {
        "mixed_identity_sources".to_owned()
    };
    let row_identity_records = rows
        .iter()
        .map(|row| row.identity_record.clone())
        .collect::<Vec<_>>();
    let identity_provenance = identity_config.to_summary_json(&row_identity_records);

    Ok(FacialIngestAnalysisSummary {
        item_count: rows.len(),
        decoded_count: rows
            .iter()
            .filter(|row| row.decode_status == "decoded")
            .count(),
        duplicate_group_count,
        duplicate_item_count,
        quality_band_counts,
        review_recommendation_counts,
        profile: profile.to_owned(),
        profile_tokens: profile_tokens.to_vec(),
        quality_source: facet::QUALITY_SOURCE.to_owned(),
        identity_source,
        identity_provenance,
        dedupe_source: imagededup::HASH_SOURCE.to_owned(),
        native_feature_outputs,
        capability_map: capability_map(),
        native_run,
    })
}

fn analysis_summary_json(summary: &FacialIngestAnalysisSummary) -> serde_json::Value {
    serde_json::json!({
        "item_count": summary.item_count,
        "decoded_count": summary.decoded_count,
        "duplicate_group_count": summary.duplicate_group_count,
        "duplicate_item_count": summary.duplicate_item_count,
        "quality_band_counts": summary.quality_band_counts,
        "review_recommendation_counts": summary.review_recommendation_counts,
        "profile": summary.profile,
        "profile_tokens": summary.profile_tokens,
        "quality_source": summary.quality_source,
        "identity_source": summary.identity_source,
        "identity_provenance": summary.identity_provenance,
        "dedupe_source": summary.dedupe_source,
        "native_feature_outputs": summary.native_feature_outputs,
        "capability_map": summary.capability_map,
        "native_run": summary.native_run,
    })
}

fn capability_map() -> Vec<FacialCapabilityMapRow> {
    facial_feature_registry()
        .into_iter()
        .map(|feature| FacialCapabilityMapRow {
            capability: feature.capability,
            source_feature_key: feature.feature_id,
            facial_source_family: feature.source_family,
            native_field: feature.native_field,
            artifact_contract: feature.artifact_contract,
            handshake_status: feature.status,
            native_route: feature.native_route,
            provenance_note: feature.provenance_note,
            required_config_keys: feature.required_config_keys,
            unavailable_reason: feature.unavailable_reason,
        })
        .collect()
}

fn json_sha256(value: &serde_json::Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|err| format!("serialize json failed: {err}"))?;
    Ok(sha256_hex(&bytes))
}

fn stable_hash(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(
        item_id: &str,
        file_name: &str,
        content_hash: Option<&str>,
    ) -> FacialIngestAnalysisItem {
        item_with_size(item_id, file_name, content_hash, 2_000_000)
    }

    fn item_with_size(
        item_id: &str,
        file_name: &str,
        content_hash: Option<&str>,
        byte_len: i64,
    ) -> FacialIngestAnalysisItem {
        FacialIngestAnalysisItem {
            item_id: item_id.to_owned(),
            source_ref: format!("dataset://source/{file_name}"),
            local_path_hint: None,
            file_name: file_name.to_owned(),
            byte_len,
            content_hash: content_hash.map(ToOwned::to_owned),
            lane: "pending".to_owned(),
        }
    }

    #[test]
    fn facial_ingest_analysis_groups_duplicates_and_scores_quality() {
        let export = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality+dedupe+identity".to_owned(),
            requested_by: "facial-agent-019".to_owned(),
            items: vec![
                item("item-a", "a.jpg", Some("hash-1")),
                item("item-b", "b.jpg", Some("hash-1")),
                item("item-c", "c.png", Some("hash-2")),
            ],
        })
        .expect("facial analysis export");

        assert_eq!(export.schema_id, FACIAL_INGEST_ANALYSIS_SCHEMA_ID);
        assert_eq!(export.profile, "quality+dedupe+identity");
        assert_eq!(export.summary.item_count, 3);
        assert_eq!(export.summary.duplicate_group_count, 1);
        assert_eq!(export.summary.duplicate_item_count, 2);
        assert_eq!(export.rows[0].duplicate_role, "representative");
        assert_eq!(export.rows[1].duplicate_role, "duplicate");
        assert_eq!(export.rows[1].review_recommendation, "review");
        assert_eq!(
            export.rows[0].quality_source,
            "facet_native_metadata_only_v1"
        );
        assert_eq!(export.rows[0].quality_source_family, "facet");
        assert_eq!(export.rows[0].quality_feature_id, "facet:quality_pass");
        assert_eq!(
            export.rows[0].ofiq_quality["source"].as_str(),
            Some("python_ofiq_native_metadata_only_v1")
        );
        assert_eq!(
            export.rows[0].dedupe_source,
            "imagededup_native_content_hash_exact_v1"
        );
        assert_eq!(
            export.rows[0].dedupe_feature_id,
            "imagededup:hash_duplicates"
        );
        assert_eq!(export.rows[0].identity_source, "handshake_proxy_no_model");
        assert_eq!(export.rows[0].identity_verdict, "proxy_unverified");
        assert!(export
            .summary
            .capability_map
            .iter()
            .any(|row| row.source_feature_key == "imagededup:hash_duplicates"));
        assert!(export
            .summary
            .capability_map
            .iter()
            .any(|row| row.source_feature_key == "python-ofiq:vector_quality"));
        assert_eq!(
            export.summary.native_feature_outputs["ediffiqa"]["status"].as_str(),
            Some("unavailable")
        );
        assert_eq!(
            export.summary.native_feature_outputs["imagededup:remove_candidates"]["policy"]
                ["non_destructive"]
                .as_bool(),
            Some(true)
        );
        assert!(export
            .summary
            .capability_map
            .iter()
            .any(|row| row.source_feature_key == "identity_gate:arcface_embedding"));
        let arcface_row = export
            .summary
            .capability_map
            .iter()
            .find(|row| row.source_feature_key == "identity_gate:arcface_embedding")
            .expect("ArcFace row from native registry");
        assert!(arcface_row.unavailable_reason.is_none());
        assert_eq!(arcface_row.handshake_status, "runtime_gated_model_backed");
        assert_eq!(
            arcface_row.native_route,
            "atelier.facial.identity.arcface_runtime_gated"
        );
        assert!(export
            .summary
            .native_run
            .selected_feature_ids
            .contains(&"identity_gate:arcface_embedding".to_owned()));
        assert!(export
            .summary
            .native_run
            .selected_feature_ids
            .contains(&"identity_gate:pipnet_landmarks".to_owned()));
        assert_eq!(export.summary.native_run.requested_by, "facial-agent-019");
        assert_eq!(export.summary.native_run.decoded_count, 0);
        assert_eq!(
            export.summary.native_run.run_status,
            "native_partial_degraded"
        );
        assert_eq!(
            export.analysis_json["summary"]["native_run"]["run_id"].as_str(),
            Some(export.summary.native_run.run_id.as_str())
        );
        assert_eq!(
            export.analysis_json["summary"]["identity_provenance"]["proxy_unverified_row_count"]
                .as_u64(),
            Some(3)
        );
        assert!(export.analysis_json["summary"]["capability_map"]
            .as_array()
            .is_some());
        assert_eq!(
            export.receipt_json["native_run"]["run_id"].as_str(),
            Some(export.summary.native_run.run_id.as_str())
        );
        assert_eq!(
            export.receipt_json["native_extension_schema_id"].as_str(),
            Some("hsk.atelier.facial_native.run@1")
        );
        assert_eq!(
            export.analysis_json["schema_id"],
            FACIAL_INGEST_ANALYSIS_SCHEMA_ID
        );
        assert_eq!(
            export.receipt_json["schema_id"],
            FACIAL_INGEST_ANALYSIS_RECEIPT_SCHEMA_ID
        );
    }

    #[test]
    fn facial_ingest_legacy_v1_rows_deserialize_with_compat_defaults() {
        let legacy_row = serde_json::json!({
            "item_id": "legacy-a",
            "source_ref": "dataset://source/legacy-a.jpg",
            "file_name": "legacy-a.jpg",
            "lane": "pending",
            "byte_len": 1234,
            "content_hash": "legacy-hash",
            "decode_status": "probe_unavailable",
            "quality_source": "handshake_native_proxy_v1",
            "quality_score": 42,
            "quality_band": "weak",
            "headshot_candidate": false,
            "duplicate_group_id": "facial-dedupe-legacy",
            "duplicate_group_size": 1,
            "duplicate_role": "singleton",
            "dedupe_source": "content_hash_exact",
            "identity_proxy_key": "facial-identity-proxy-legacy",
            "identity_source": "handshake_proxy_no_model",
            "identity_verdict": "proxy_unverified",
            "review_recommendation": "review",
            "reasons": ["legacy_artifact"],
        });

        let row: FacialIngestAnalysisRow =
            serde_json::from_value(legacy_row).expect("legacy @1 row remains readable");

        assert_eq!(row.quality_source_family, "legacy_unknown");
        assert_eq!(row.quality_feature_id, "legacy:quality_proxy");
        assert_eq!(
            row.quality_metrics["status"].as_str(),
            Some("legacy_missing")
        );
        assert_eq!(row.dedupe_source_family, "legacy_unknown");
        assert_eq!(row.dedupe_feature_id, "legacy:dedupe_proxy");
        assert_eq!(row.dedupe_record["status"].as_str(), Some("legacy_missing"));
        assert_eq!(
            row.identity_record["status"].as_str(),
            Some("legacy_missing")
        );
    }

    #[test]
    fn facial_ingest_dedupe_representative_matches_remove_candidate_keeper() {
        let export = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality+dedupe".to_owned(),
            requested_by: "facial-agent-019".to_owned(),
            items: vec![
                item_with_size("item-a", "a.jpg", Some("same-hash"), 1_000_000),
                item_with_size("item-b", "b.jpg", Some("same-hash"), 2_000_000),
            ],
        })
        .expect("facial analysis export");

        let representative = export
            .rows
            .iter()
            .find(|row| row.duplicate_role == "representative")
            .expect("representative row");
        assert_eq!(representative.item_id, "item-b");
        assert_eq!(
            export.summary.native_feature_outputs["imagededup:remove_candidates"]["remove_list"][0]
                ["keep"]
                .as_str(),
            Some("dataset://source/b.jpg")
        );
    }

    #[test]
    fn facial_ingest_analysis_rejects_unknown_profile_tokens() {
        let err = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality+magic".to_owned(),
            requested_by: "facial-agent-019".to_owned(),
            items: vec![item("item-a", "a.jpg", Some("hash-1"))],
        })
        .expect_err("unknown profile token should fail");
        assert!(err.contains("unsupported facial profile token"));
    }

    #[test]
    fn facial_ingest_analysis_continues_on_missing_local_files() {
        let export = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality review".to_owned(),
            requested_by: "facial-agent-019".to_owned(),
            items: vec![FacialIngestAnalysisItem {
                item_id: "item-local".to_owned(),
                source_ref: "missing-local-file.jpg".to_owned(),
                local_path_hint: None,
                file_name: "missing-local-file.jpg".to_owned(),
                byte_len: 0,
                content_hash: None,
                lane: "pending".to_owned(),
            }],
        })
        .expect("missing local files are per-row diagnostics, not fatal");
        assert_eq!(export.profile, "quality+review");
        assert_eq!(export.rows[0].decode_status, "local_path_missing_file");
        assert_eq!(export.rows[0].quality_band, "reject");
        assert!(!export.rows[0].headshot_candidate);
        assert_eq!(export.rows[0].review_recommendation, "cull");
        assert!(export.rows[0]
            .reasons
            .iter()
            .any(|reason| reason == "missing_content_hash_limits_dedupe"));
    }

    #[test]
    fn facial_ingest_analysis_hash_is_actor_stable_and_receipt_is_actor_specific() {
        let request = GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality+dedupe+identity".to_owned(),
            requested_by: "facial-agent-a".to_owned(),
            items: vec![item("item-a", "a.jpg", Some("hash-1"))],
        };
        let export_a = generate_facial_ingest_analysis(request.clone()).expect("export a");
        let export_b = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            requested_by: "facial-agent-b".to_owned(),
            ..request
        })
        .expect("export b");
        assert_eq!(export_a.analysis_sha256, export_b.analysis_sha256);
        assert_eq!(export_a.content_hash, export_b.content_hash);
        assert_eq!(
            export_a.summary.native_run.run_hash,
            export_b.summary.native_run.run_hash
        );
        assert_ne!(
            export_a.summary.native_run.run_id,
            export_b.summary.native_run.run_id
        );
        assert_ne!(
            export_a.summary.native_run.requested_by,
            export_b.summary.native_run.requested_by
        );
        assert_ne!(export_a.receipt_sha256, export_b.receipt_sha256);
    }

    #[test]
    fn facial_ingest_analysis_decodes_resolved_artifact_hint_and_labels_source_refs() {
        const PNG_1X1: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4,
            0, 9, 251, 3, 253, 167, 164, 37, 219, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        let path =
            std::env::temp_dir().join(format!("hsk-facial-mt019-{}.png", std::process::id()));
        std::fs::write(&path, PNG_1X1).expect("write png fixture");
        let before_hash = sha256_hex(&std::fs::read(&path).expect("read png before analysis"));
        let export = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-00000000f019".to_owned(),
            profile: "quality".to_owned(),
            requested_by: "facial-agent-019".to_owned(),
            items: vec![
                FacialIngestAnalysisItem {
                    item_id: "item-artifact".to_owned(),
                    source_ref: "artifact://.handshake/artifacts/L1/test/payload".to_owned(),
                    local_path_hint: Some(path.to_string_lossy().into_owned()),
                    file_name: "artifact.png".to_owned(),
                    byte_len: PNG_1X1.len() as i64,
                    content_hash: Some("hash-artifact".to_owned()),
                    lane: "pending".to_owned(),
                },
                FacialIngestAnalysisItem {
                    item_id: "item-source".to_owned(),
                    source_ref: "source://operator/import/source-only.png".to_owned(),
                    local_path_hint: None,
                    file_name: "source-only.png".to_owned(),
                    byte_len: 1024,
                    content_hash: Some("hash-source".to_owned()),
                    lane: "pending".to_owned(),
                },
            ],
        })
        .expect("artifact/source refs are analyzable");
        let after_hash = sha256_hex(&std::fs::read(&path).expect("read png after analysis"));
        let _ = std::fs::remove_file(path);
        assert_eq!(before_hash, after_hash);
        assert_eq!(export.rows[0].decode_status, "decoded");
        assert_eq!(export.rows[0].image_width, Some(1));
        assert_eq!(export.rows[0].image_height, Some(1));
        assert_eq!(export.rows[1].decode_status, "external_ref_not_decoded");
        assert_eq!(export.summary.decoded_count, 1);
    }
}
