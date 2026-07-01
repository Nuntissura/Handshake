//! Native Facial-derived Ingest analysis.
//!
//! MT-019 ports the first high-value Facial app capability into Handshake as a
//! native, headless analysis artifact. This module does not shell out to
//! `facial.exe`, and it does not pretend to provide ArcFace/YuNet parity before
//! those model assets are wired into Handshake. Rows that need a real model are
//! explicitly marked as proxy-derived.

use super::facial_native::{
    build_facial_native_run_report, facial_feature_registry, FacialNativeRunItem,
    FacialNativeRunReport, FacialNativeRunRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
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
    pub quality_score: u8,
    pub quality_band: String,
    pub headshot_candidate: bool,
    pub duplicate_group_id: String,
    pub duplicate_group_size: usize,
    pub duplicate_role: String,
    pub dedupe_source: String,
    pub identity_proxy_key: String,
    pub identity_source: String,
    pub identity_verdict: String,
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
    pub dedupe_source: String,
    pub capability_map: Vec<FacialCapabilityMapRow>,
    pub native_run: FacialNativeRunReport,
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

    let group_keys = duplicate_group_keys(&request.items);
    let mut group_sizes: HashMap<String, usize> = HashMap::new();
    for key in group_keys.values() {
        *group_sizes.entry(key.clone()).or_insert(0) += 1;
    }
    let mut first_by_group: HashMap<String, String> = HashMap::new();
    for item in &request.items {
        if let Some(key) = group_keys.get(&item.item_id) {
            first_by_group
                .entry(key.clone())
                .or_insert_with(|| item.item_id.clone());
        }
    }

    let mut rows = Vec::with_capacity(request.items.len());
    for item in request.items {
        let image = inspect_image_ref(&item.source_ref, item.local_path_hint.as_deref());
        let quality = quality_score(&item, &image);
        let quality_band = quality_band(quality);
        let group_key = group_keys
            .get(&item.item_id)
            .cloned()
            .unwrap_or_else(|| format!("item:{}", stable_hash(&item.item_id)));
        let duplicate_group_size = *group_sizes.get(&group_key).unwrap_or(&1);
        let duplicate_role = if duplicate_group_size <= 1 {
            "singleton"
        } else if first_by_group.get(&group_key) == Some(&item.item_id) {
            "representative"
        } else {
            "duplicate"
        };
        let dedupe_source = if item
            .content_hash
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            "content_hash_exact"
        } else {
            "missing_content_hash_singleton"
        };
        let (review_recommendation, reasons) =
            review_recommendation(&item, &image, quality, quality_band, duplicate_role);
        let identity_basis = item
            .content_hash
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(item.source_ref.as_str())
            .to_owned();
        rows.push(FacialIngestAnalysisRow {
            item_id: item.item_id,
            source_ref: item.source_ref,
            file_name: item.file_name,
            lane: item.lane,
            byte_len: item.byte_len,
            content_hash: item.content_hash,
            decode_status: image.decode_status,
            image_width: image.width,
            image_height: image.height,
            megapixels: image.megapixels,
            quality_source: "handshake_native_proxy_v1".to_owned(),
            quality_score: quality,
            quality_band: quality_band.to_owned(),
            headshot_candidate: quality >= 70 && image.width.is_some(),
            duplicate_group_id: format!("facial-dedupe-{}", stable_hash(&group_key)),
            duplicate_group_size,
            duplicate_role: duplicate_role.to_owned(),
            dedupe_source: dedupe_source.to_owned(),
            identity_proxy_key: format!("facial-identity-proxy-{}", stable_hash(&identity_basis)),
            identity_source: "handshake_proxy_no_model".to_owned(),
            identity_verdict: "proxy_unverified".to_owned(),
            review_recommendation,
            reasons,
        });
    }

    let summary = summarize_rows(&batch_id, &profile, &requested_by, &profile_tokens, &rows)?;
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
        .and_then(|value| serde_json::from_value::<Vec<FacialIngestAnalysisRow>>(value).ok())
        .unwrap_or_default();
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

fn duplicate_group_keys(items: &[FacialIngestAnalysisItem]) -> HashMap<String, String> {
    items
        .iter()
        .map(|item| {
            let key = item
                .content_hash
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("content_hash:{value}"))
                .unwrap_or_else(|| format!("singleton:{}", item.item_id));
            (item.item_id.clone(), key)
        })
        .collect()
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

fn quality_score(item: &FacialIngestAnalysisItem, image: &ImageProbe) -> u8 {
    let mut score: i32 = 18;
    if item.byte_len > 0 {
        score += 12;
    }
    if item.byte_len >= 128_000 {
        score += 8;
    }
    if item.byte_len >= 1_000_000 {
        score += 6;
    }
    if item
        .content_hash
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        score += 8;
    }
    match (image.width, image.height) {
        (Some(width), Some(height)) => {
            score += 18;
            let megapixels = (width as f64) * (height as f64) / 1_000_000.0;
            if megapixels >= 0.5 {
                score += 8;
            }
            if megapixels >= 1.5 {
                score += 8;
            }
            let short_side = width.min(height);
            if short_side >= 512 {
                score += 8;
            }
            let ratio = width.max(height) as f64 / width.min(height).max(1) as f64;
            if ratio <= 2.2 {
                score += 5;
            }
        }
        _ => {
            if likely_image_extension(&item.file_name) || likely_image_extension(&item.source_ref) {
                score += 8;
            }
        }
    }
    score.clamp(0, 100) as u8
}

fn likely_image_extension(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".bmp"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn quality_band(score: u8) -> &'static str {
    match score {
        85..=100 => "excellent",
        70..=84 => "good",
        50..=69 => "usable",
        30..=49 => "weak",
        _ => "reject",
    }
}

fn review_recommendation(
    item: &FacialIngestAnalysisItem,
    image: &ImageProbe,
    score: u8,
    quality_band: &str,
    duplicate_role: &str,
) -> (String, Vec<String>) {
    let mut reasons = Vec::new();
    reasons.push(format!("quality_score={score}"));
    reasons.push(format!("quality_band={quality_band}"));
    reasons.push(format!("decode_status={}", image.decode_status));
    reasons.push("identity_verdict=proxy_unverified".to_owned());
    if duplicate_role != "singleton" {
        reasons.push(format!("duplicate_role={duplicate_role}"));
    }
    if item.content_hash.is_none() {
        reasons.push("missing_content_hash_limits_dedupe".to_owned());
    }

    let recommendation = if matches!(quality_band, "reject" | "weak") {
        "cull"
    } else if duplicate_role == "duplicate"
        || image.decode_status != "decoded"
        || quality_band == "usable"
    {
        "review"
    } else {
        "keep"
    };
    (recommendation.to_owned(), reasons)
}

fn summarize_rows(
    batch_id: &str,
    profile: &str,
    requested_by: &str,
    profile_tokens: &[String],
    rows: &[FacialIngestAnalysisRow],
) -> Result<FacialIngestAnalysisSummary, String> {
    let mut quality_band_counts = BTreeMap::new();
    let mut review_recommendation_counts = BTreeMap::new();
    for row in rows {
        *quality_band_counts
            .entry(row.quality_band.clone())
            .or_insert(0) += 1;
        *review_recommendation_counts
            .entry(row.review_recommendation.clone())
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
        quality_source: "handshake_native_proxy_v1".to_owned(),
        identity_source: "handshake_proxy_no_model".to_owned(),
        dedupe_source: "content_hash_exact_or_singleton".to_owned(),
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
        "dedupe_source": summary.dedupe_source,
        "capability_map": summary.capability_map,
        "native_run": analysis_native_run_json(&summary.native_run),
    })
}

fn analysis_native_run_json(native_run: &FacialNativeRunReport) -> serde_json::Value {
    serde_json::json!({
        "schema_id": native_run.schema_id,
        "registry_schema_id": native_run.registry_schema_id,
        "batch_id": native_run.batch_id,
        "profile": native_run.profile,
        "profile_tokens": native_run.profile_tokens,
        "item_count": native_run.item_count,
        "decoded_count": native_run.decoded_count,
        "selected_feature_ids": native_run.selected_feature_ids,
        "run_status": native_run.run_status,
        "status_counts": native_run.status_counts,
        "degraded_reasons": native_run.degraded_reasons,
        "feature_records": native_run.feature_records,
        "run_hash": native_run.run_hash,
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
        FacialIngestAnalysisItem {
            item_id: item_id.to_owned(),
            source_ref: format!("dataset://source/{file_name}"),
            local_path_hint: None,
            file_name: file_name.to_owned(),
            byte_len: 2_000_000,
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
        assert_eq!(export.rows[0].quality_source, "handshake_native_proxy_v1");
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
            .any(|row| row.source_feature_key == "identity_gate:arcface_embedding"));
        let arcface_row = export
            .summary
            .capability_map
            .iter()
            .find(|row| row.source_feature_key == "identity_gate:arcface_embedding")
            .expect("ArcFace row from native registry");
        assert_eq!(
            arcface_row.unavailable_reason.as_deref(),
            Some("arcface_model_not_configured")
        );
        assert_eq!(
            arcface_row.native_route,
            "atelier.facial.identity.arcface_unavailable"
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
            export.analysis_json["summary"]["native_run"]["run_hash"].as_str(),
            Some(export.summary.native_run.run_hash.as_str())
        );
        assert_eq!(
            export.analysis_json["summary"]["native_run"]["run_id"].as_str(),
            None
        );
        assert_eq!(
            export.analysis_json["summary"]["native_run"]["requested_by"].as_str(),
            None
        );
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
        let _ = std::fs::remove_file(path);
        assert_eq!(export.rows[0].decode_status, "decoded");
        assert_eq!(export.rows[0].image_width, Some(1));
        assert_eq!(export.rows[0].image_height, Some(1));
        assert_eq!(export.rows[1].decode_status, "external_ref_not_decoded");
        assert_eq!(export.summary.decoded_count, 1);
    }
}
