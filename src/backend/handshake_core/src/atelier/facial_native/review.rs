use crate::atelier::facial::FacialIngestAnalysisRow;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const FACIAL_REVIEW_SESSION_SCHEMA_ID: &str = "hsk.atelier.facial_review.session@1";
pub const FACIAL_REVIEW_CLAIM_SCHEMA_ID: &str = "hsk.atelier.facial_review.claim@1";
pub const FACIAL_REVIEW_DECISION_SCHEMA_ID: &str = "hsk.atelier.facial_review.decision@1";
pub const FACIAL_REVIEW_STATUS_SCHEMA_ID: &str = "hsk.atelier.facial_review.status@1";
pub const FACIAL_REVIEW_MONTAGE_SCHEMA_ID: &str = "hsk.atelier.facial_review.montage@1";
pub const FACIAL_REVIEW_EXPORT_SCHEMA_ID: &str = "hsk.atelier.facial_review.export@1";

const DEFAULT_CLAIM_TTL_SECONDS: u64 = 3600;

#[derive(Clone, Debug, PartialEq)]
pub struct BuildFacialReviewSessionRequest {
    pub batch_id: String,
    pub analysis_sha256: String,
    pub receipt_sha256: String,
    pub created_by: String,
    pub created_at_utc: String,
    pub shard_count: usize,
    pub claim_ttl_seconds: Option<u64>,
    pub rows: Vec<FacialIngestAnalysisRow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewItem {
    pub item_id: String,
    pub stable_image_id: String,
    pub id_basis: String,
    pub source_ref: String,
    pub file_name: String,
    pub lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub shard: usize,
    pub review_recommendation: String,
    pub quality_band: String,
    pub quality_score: u8,
    pub duplicate_group_id: String,
    pub duplicate_role: String,
    pub identity_verdict: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewSessionArtifact {
    pub schema_id: String,
    pub session_id: String,
    pub batch_id: String,
    pub analysis_sha256: String,
    pub receipt_sha256: String,
    pub created_by: String,
    pub created_at_utc: String,
    pub shard_count: usize,
    pub claim_ttl_seconds: u64,
    pub item_count: usize,
    pub items: Vec<FacialReviewItem>,
    pub lineage_refs: serde_json::Value,
    pub session_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FacialReviewClaimRequest {
    pub actor: String,
    pub claimed_at_utc: String,
    pub shard: Option<usize>,
    pub claim_ttl_seconds: Option<u64>,
    pub steal_expired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewClaimReceipt {
    pub schema_id: String,
    pub claim_id: String,
    pub session_id: String,
    pub actor: String,
    pub shard: usize,
    pub claimed_at_utc: String,
    pub expires_at_utc: String,
    pub status: String,
    pub item_ids: Vec<String>,
    pub stable_image_ids: Vec<String>,
    pub work_items: Vec<FacialReviewItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_from_claim_id: Option<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FacialReviewDecisionRequest {
    pub actor: String,
    pub decided_at_utc: String,
    pub item_id: String,
    pub claim_id: String,
    pub decision: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewDecisionReceipt {
    pub schema_id: String,
    pub decision_id: String,
    pub session_id: String,
    pub claim_id: String,
    pub actor: String,
    pub decided_at_utc: String,
    pub item_id: String,
    pub stable_image_id: String,
    pub entered_decision: String,
    pub canonical_decision: String,
    pub reason: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub review_recommendation: String,
    pub quality_band: String,
    pub duplicate_group_id: String,
    pub duplicate_role: String,
    pub identity_verdict: String,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialReviewShardStatus {
    pub shard: usize,
    pub total: usize,
    pub decided: usize,
    pub claimed_by: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewStatusArtifact {
    pub schema_id: String,
    pub session_id: String,
    pub item_count: usize,
    pub decided_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub hold_count: usize,
    pub undecided_count: usize,
    pub active_claim_count: usize,
    pub expired_claim_count: usize,
    pub per_shard: Vec<FacialReviewShardStatus>,
    pub per_actor_decisions: BTreeMap<String, usize>,
    pub decision_conflicts: Vec<serde_json::Value>,
    pub status_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildFacialReviewMontageRequest {
    pub requested_by: String,
    pub page: usize,
    pub columns: usize,
    pub rows: usize,
    pub tile_width: u32,
    pub tile_height: u32,
    pub decision_filter: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewMontageTile {
    pub tile_id: String,
    pub tile_index: usize,
    pub row: usize,
    pub col: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub item_id: String,
    pub stable_image_id: String,
    pub source_ref: String,
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub review_recommendation: String,
    pub decision: String,
    pub argus_selector: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewMontageArtifact {
    pub schema_id: String,
    pub session_id: String,
    pub requested_by: String,
    pub page: usize,
    pub page_count: usize,
    pub matched_count: usize,
    pub tile_count: usize,
    pub grid: serde_json::Value,
    pub artifact_kind: String,
    pub tiles: Vec<FacialReviewMontageTile>,
    pub montage_sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildFacialReviewExportRequest {
    pub requested_by: String,
    pub exported_at_utc: String,
    pub dataset_name: String,
    pub repeats: u32,
    pub allow_partial: bool,
    pub output_root_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewExportedFile {
    pub item_id: String,
    pub stable_image_id: String,
    pub source_ref: String,
    pub output_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub decision_id: String,
    pub duplicate_group_id: String,
    pub quality_band: String,
    pub identity_verdict: String,
    pub copy_mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialReviewExportProblem {
    pub item_id: String,
    pub stable_image_id: String,
    pub source_ref: String,
    pub problem: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialReviewExportManifest {
    pub schema_id: String,
    pub session_id: String,
    pub requested_by: String,
    pub exported_at_utc: String,
    pub dataset_name: String,
    pub repeats: u32,
    pub output_root_ref: String,
    pub source_mutation: bool,
    pub copy_mode: String,
    pub funnel: serde_json::Value,
    pub lineage: serde_json::Value,
    pub exported_files: Vec<FacialReviewExportedFile>,
    pub problems: Vec<FacialReviewExportProblem>,
    pub manifest_sha256: String,
}

pub fn build_review_session(
    request: BuildFacialReviewSessionRequest,
) -> Result<FacialReviewSessionArtifact, String> {
    let batch_id = require_clean_ref("batch_id", &request.batch_id)?;
    let analysis_sha256 = require_hashish("analysis_sha256", &request.analysis_sha256)?;
    let receipt_sha256 = require_hashish("receipt_sha256", &request.receipt_sha256)?;
    let created_by = require_clean_ref("created_by", &request.created_by)?;
    parse_utc("created_at_utc", &request.created_at_utc)?;
    if request.rows.is_empty() {
        return Err("facial review session requires at least one analysis row".to_owned());
    }
    let shard_count = request.shard_count.clamp(1, 256);
    let claim_ttl_seconds = request
        .claim_ttl_seconds
        .unwrap_or(DEFAULT_CLAIM_TTL_SECONDS)
        .max(1);

    let mut items = request
        .rows
        .iter()
        .map(review_item_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|left, right| {
        (
            left.stable_image_id.as_str(),
            left.item_id.as_str(),
            left.source_ref.as_str(),
        )
            .cmp(&(
                right.stable_image_id.as_str(),
                right.item_id.as_str(),
                right.source_ref.as_str(),
            ))
    });
    for (index, item) in items.iter_mut().enumerate() {
        item.shard = index % shard_count;
    }

    let payload = serde_json::json!({
        "schema_id": FACIAL_REVIEW_SESSION_SCHEMA_ID,
        "batch_id": batch_id,
        "analysis_sha256": analysis_sha256,
        "receipt_sha256": receipt_sha256,
        "created_by": created_by,
        "created_at_utc": request.created_at_utc,
        "shard_count": shard_count,
        "claim_ttl_seconds": claim_ttl_seconds,
        "items": items,
    });
    let session_sha256 = json_sha256(&payload)?;
    let session_id = format!("facial-review-session-{}", &session_sha256[..16]);

    Ok(FacialReviewSessionArtifact {
        schema_id: FACIAL_REVIEW_SESSION_SCHEMA_ID.to_owned(),
        session_id,
        batch_id,
        analysis_sha256,
        receipt_sha256,
        created_by,
        created_at_utc: request.created_at_utc,
        shard_count,
        claim_ttl_seconds,
        item_count: items.len(),
        items,
        lineage_refs: serde_json::json!({
            "analysis_sha256": request.analysis_sha256,
            "receipt_sha256": request.receipt_sha256,
            "source_stage": "facial_ingest_analysis",
            "review_stage": "session_initialized",
        }),
        session_sha256,
    })
}

pub fn claim_review_shard(
    session: &FacialReviewSessionArtifact,
    existing_claims: &[FacialReviewClaimReceipt],
    decisions: &[FacialReviewDecisionReceipt],
    request: FacialReviewClaimRequest,
) -> Result<FacialReviewClaimReceipt, String> {
    let actor = require_clean_ref("actor", &request.actor)?;
    let claimed_at = parse_utc("claimed_at_utc", &request.claimed_at_utc)?;
    let decided = effective_decisions(decisions);
    let active_claims = active_claims_by_shard(existing_claims, claimed_at);
    let expired_claims = expired_claims_by_shard(existing_claims, claimed_at);
    let ttl_seconds = request
        .claim_ttl_seconds
        .unwrap_or(session.claim_ttl_seconds)
        .max(1);

    let shard = match request.shard {
        Some(shard) => {
            if shard >= session.shard_count {
                return Err(format!(
                    "facial review shard {shard} out of range for {} shards",
                    session.shard_count
                ));
            }
            if active_claims.contains_key(&shard) {
                return Err(format!("facial review shard {shard} is actively claimed"));
            }
            shard
        }
        None => (0..session.shard_count)
            .find(|shard| {
                !active_claims.contains_key(shard)
                    && session.items.iter().any(|item| {
                        item.shard == *shard && !decided.contains_key(&item.stable_image_id)
                    })
            })
            .ok_or_else(|| "no claimable facial review shards remain".to_owned())?,
    };
    let recovered_from_claim_id = expired_claims
        .get(&shard)
        .map(|claim| claim.claim_id.clone());
    if recovered_from_claim_id.is_some() && !request.steal_expired {
        return Err(format!(
            "facial review shard {shard} has an expired claim; set steal_expired to recover it"
        ));
    }
    let expires_at = claimed_at + Duration::seconds(ttl_seconds.min(i64::MAX as u64) as i64);
    let work_items = session
        .items
        .iter()
        .filter(|item| item.shard == shard && !decided.contains_key(&item.stable_image_id))
        .cloned()
        .collect::<Vec<_>>();
    if work_items.is_empty() {
        return Err(format!("facial review shard {shard} has no undecided work"));
    }
    let item_ids = work_items
        .iter()
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    let stable_image_ids = work_items
        .iter()
        .map(|item| item.stable_image_id.clone())
        .collect::<Vec<_>>();
    let claim_id = format!(
        "facial-review-claim-{}",
        &stable_hash(&format!(
            "{}|{}|{}|{}",
            session.session_id, actor, shard, request.claimed_at_utc
        ))[..16]
    );
    let mut receipt = FacialReviewClaimReceipt {
        schema_id: FACIAL_REVIEW_CLAIM_SCHEMA_ID.to_owned(),
        claim_id,
        session_id: session.session_id.clone(),
        actor,
        shard,
        claimed_at_utc: request.claimed_at_utc,
        expires_at_utc: expires_at.to_rfc3339(),
        status: "active".to_owned(),
        item_ids,
        stable_image_ids,
        work_items,
        recovered_from_claim_id,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = serializable_sha256(&receipt)?;
    Ok(receipt)
}

pub fn record_review_decision(
    session: &FacialReviewSessionArtifact,
    claim: &FacialReviewClaimReceipt,
    request: FacialReviewDecisionRequest,
) -> Result<FacialReviewDecisionReceipt, String> {
    if claim.session_id != session.session_id {
        return Err("facial review decision claim belongs to a different session".to_owned());
    }
    let actor = require_clean_ref("actor", &request.actor)?;
    if actor != claim.actor {
        return Err("facial review decision actor does not own the claim".to_owned());
    }
    if request.claim_id != claim.claim_id {
        return Err("facial review decision claim_id does not match claim receipt".to_owned());
    }
    let decided_at = parse_utc("decided_at_utc", &request.decided_at_utc)?;
    let expires_at = parse_utc("claim.expires_at_utc", &claim.expires_at_utc)?;
    if decided_at > expires_at {
        return Err("facial review decision uses an expired claim".to_owned());
    }
    let item = resolve_review_item(session, &request.item_id)?;
    if !claim.stable_image_ids.contains(&item.stable_image_id) {
        return Err("facial review decision item is not in the claimed shard".to_owned());
    }
    let canonical_decision = normalize_decision(&request.decision)?;
    let reason = require_clean_ref("reason", &request.reason)?;
    let mut tags = request
        .tags
        .iter()
        .map(|tag| normalize_tag(tag))
        .collect::<Result<BTreeSet<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();
    tags.sort();
    let notes = request
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let decision_id = format!(
        "facial-review-decision-{}",
        &stable_hash(&format!(
            "{}|{}|{}|{}|{}|{}",
            session.session_id,
            claim.claim_id,
            item.stable_image_id,
            actor,
            canonical_decision,
            request.decided_at_utc
        ))[..16]
    );
    let mut receipt = FacialReviewDecisionReceipt {
        schema_id: FACIAL_REVIEW_DECISION_SCHEMA_ID.to_owned(),
        decision_id,
        session_id: session.session_id.clone(),
        claim_id: claim.claim_id.clone(),
        actor,
        decided_at_utc: request.decided_at_utc,
        item_id: item.item_id.clone(),
        stable_image_id: item.stable_image_id.clone(),
        entered_decision: request.decision,
        canonical_decision,
        reason,
        tags,
        notes,
        source_ref: item.source_ref.clone(),
        content_hash: item.content_hash.clone(),
        review_recommendation: item.review_recommendation.clone(),
        quality_band: item.quality_band.clone(),
        duplicate_group_id: item.duplicate_group_id.clone(),
        duplicate_role: item.duplicate_role.clone(),
        identity_verdict: item.identity_verdict.clone(),
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = serializable_sha256(&receipt)?;
    Ok(receipt)
}

pub fn build_review_status(
    session: &FacialReviewSessionArtifact,
    claims: &[FacialReviewClaimReceipt],
    decisions: &[FacialReviewDecisionReceipt],
    now_utc: &str,
) -> Result<FacialReviewStatusArtifact, String> {
    let now = parse_utc("now_utc", now_utc)?;
    let effective = effective_decisions(decisions);
    let active_claims = active_claims_by_shard(claims, now);
    let expired_claims = expired_claims_by_shard(claims, now);
    let mut per_actor_decisions = BTreeMap::<String, usize>::new();
    for decision in decisions {
        *per_actor_decisions
            .entry(decision.actor.clone())
            .or_insert(0) += 1;
    }
    let mut accepted_count = 0usize;
    let mut rejected_count = 0usize;
    let mut hold_count = 0usize;
    for decision in effective.values() {
        match decision.as_str() {
            "accept" => accepted_count += 1,
            "reject" => rejected_count += 1,
            "hold" => hold_count += 1,
            _ => {}
        }
    }
    let mut per_shard = Vec::new();
    for shard in 0..session.shard_count {
        let shard_items = session
            .items
            .iter()
            .filter(|item| item.shard == shard)
            .collect::<Vec<_>>();
        let decided = shard_items
            .iter()
            .filter(|item| effective.contains_key(&item.stable_image_id))
            .count();
        let claimed_by = active_claims
            .get(&shard)
            .map(|claim| vec![claim.actor.clone()])
            .unwrap_or_default();
        per_shard.push(FacialReviewShardStatus {
            shard,
            total: shard_items.len(),
            decided,
            claimed_by,
        });
    }
    let decision_conflicts = decision_conflicts(decisions);
    let decided_count = effective.len();
    let mut artifact = FacialReviewStatusArtifact {
        schema_id: FACIAL_REVIEW_STATUS_SCHEMA_ID.to_owned(),
        session_id: session.session_id.clone(),
        item_count: session.item_count,
        decided_count,
        accepted_count,
        rejected_count,
        hold_count,
        undecided_count: session.item_count.saturating_sub(decided_count),
        active_claim_count: active_claims.len(),
        expired_claim_count: expired_claims.len(),
        per_shard,
        per_actor_decisions,
        decision_conflicts,
        status_sha256: String::new(),
    };
    artifact.status_sha256 = serializable_sha256(&artifact)?;
    Ok(artifact)
}

pub fn build_review_montage(
    session: &FacialReviewSessionArtifact,
    decisions: &[FacialReviewDecisionReceipt],
    request: BuildFacialReviewMontageRequest,
) -> Result<FacialReviewMontageArtifact, String> {
    let requested_by = require_clean_ref("requested_by", &request.requested_by)?;
    if request.columns == 0 || request.rows == 0 || request.columns > 20 || request.rows > 20 {
        return Err("facial review montage grid must be between 1 and 20 rows/columns".to_owned());
    }
    if request.tile_width == 0
        || request.tile_height == 0
        || request.tile_width > 4096
        || request.tile_height > 4096
    {
        return Err("facial review montage tile dimensions must be 1..=4096".to_owned());
    }
    let decision_filter = request
        .decision_filter
        .as_deref()
        .map(normalize_decision_filter)
        .transpose()?;
    let effective = effective_decisions(decisions);
    let mut matched = session
        .items
        .iter()
        .filter(|item| {
            decision_filter.as_deref().is_none_or(|filter| {
                effective
                    .get(&item.stable_image_id)
                    .map(String::as_str)
                    .unwrap_or("undecided")
                    == filter
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    matched.sort_by(|left, right| {
        (
            left.shard,
            left.stable_image_id.as_str(),
            left.item_id.as_str(),
        )
            .cmp(&(
                right.shard,
                right.stable_image_id.as_str(),
                right.item_id.as_str(),
            ))
    });
    if matched.is_empty() {
        return Err("facial review montage has no matching items".to_owned());
    }
    let per_page = request.columns * request.rows;
    let page_count = matched.len().div_ceil(per_page);
    if request.page >= page_count {
        return Err(format!(
            "facial review montage page {} out of range ({page_count} pages)",
            request.page
        ));
    }
    let page_start = request.page * per_page;
    let page_items = &matched[page_start..(page_start + per_page).min(matched.len())];
    let tiles = page_items
        .iter()
        .enumerate()
        .map(|(tile_index, item)| {
            let row = tile_index / request.columns;
            let col = tile_index % request.columns;
            FacialReviewMontageTile {
                tile_id: format!(
                    "facial-review-tile-{}-p{}-t{}",
                    &session.session_sha256[..12],
                    request.page,
                    tile_index
                ),
                tile_index,
                row,
                col,
                x: (col as u32) * request.tile_width,
                y: (row as u32) * request.tile_height,
                width: request.tile_width,
                height: request.tile_height,
                item_id: item.item_id.clone(),
                stable_image_id: item.stable_image_id.clone(),
                source_ref: item.source_ref.clone(),
                file_name: item.file_name.clone(),
                content_hash: item.content_hash.clone(),
                review_recommendation: item.review_recommendation.clone(),
                decision: effective
                    .get(&item.stable_image_id)
                    .cloned()
                    .unwrap_or_else(|| "undecided".to_owned()),
                argus_selector: format!("argus://facial-review/{}", item.stable_image_id),
            }
        })
        .collect::<Vec<_>>();
    let grid = serde_json::json!({
        "columns": request.columns,
        "rows": request.rows,
        "tile_width": request.tile_width,
        "tile_height": request.tile_height,
        "decision_filter": decision_filter,
        "coordinate_space": "logical_tile_grid",
    });
    let mut artifact = FacialReviewMontageArtifact {
        schema_id: FACIAL_REVIEW_MONTAGE_SCHEMA_ID.to_owned(),
        session_id: session.session_id.clone(),
        requested_by,
        page: request.page,
        page_count,
        matched_count: matched.len(),
        tile_count: tiles.len(),
        grid,
        artifact_kind: "tile_map_manifest".to_owned(),
        tiles,
        montage_sha256: String::new(),
    };
    artifact.montage_sha256 = serializable_sha256(&artifact)?;
    Ok(artifact)
}

pub fn build_review_export_manifest(
    session: &FacialReviewSessionArtifact,
    decisions: &[FacialReviewDecisionReceipt],
    request: BuildFacialReviewExportRequest,
) -> Result<FacialReviewExportManifest, String> {
    let requested_by = require_clean_ref("requested_by", &request.requested_by)?;
    parse_utc("exported_at_utc", &request.exported_at_utc)?;
    let dataset_name = normalize_dataset_name(&request.dataset_name)?;
    let output_root_ref = require_clean_ref("output_root_ref", &request.output_root_ref)?;
    let repeats = request.repeats.clamp(1, 10_000);
    let effective = effective_decision_receipts(decisions);
    let undecided = session
        .items
        .iter()
        .filter(|item| !effective.contains_key(&item.stable_image_id))
        .collect::<Vec<_>>();
    if !undecided.is_empty() && !request.allow_partial {
        return Err(format!(
            "{} facial review items are undecided; set allow_partial to export anyway",
            undecided.len()
        ));
    }

    let mut exported_files = Vec::new();
    let mut problems = Vec::new();
    for item in &session.items {
        let Some(decision) = effective.get(&item.stable_image_id) else {
            if request.allow_partial {
                problems.push(export_problem(item, "undecided_skipped"));
            }
            continue;
        };
        match decision.canonical_decision.as_str() {
            "accept" if item.duplicate_role == "duplicate" => {
                problems.push(export_problem(item, "accepted_duplicate_skipped"));
            }
            "accept" => exported_files.push(FacialReviewExportedFile {
                item_id: item.item_id.clone(),
                stable_image_id: item.stable_image_id.clone(),
                source_ref: item.source_ref.clone(),
                output_ref: format!(
                    "{}/{}_{}/{}",
                    output_root_ref.trim_end_matches('/'),
                    repeats,
                    dataset_name,
                    safe_file_name(&item.file_name)
                ),
                content_hash: item.content_hash.clone(),
                decision_id: decision.decision_id.clone(),
                duplicate_group_id: item.duplicate_group_id.clone(),
                quality_band: item.quality_band.clone(),
                identity_verdict: item.identity_verdict.clone(),
                copy_mode: "copy_plan_only_source_read_only".to_owned(),
            }),
            "reject" => problems.push(export_problem(item, "rejected_not_exported")),
            "hold" => problems.push(export_problem(item, "hold_not_exported")),
            _ => {}
        }
    }
    exported_files.sort_by(|left, right| left.output_ref.cmp(&right.output_ref));
    problems.sort_by(|left, right| {
        (
            left.problem.as_str(),
            left.stable_image_id.as_str(),
            left.source_ref.as_str(),
        )
            .cmp(&(
                right.problem.as_str(),
                right.stable_image_id.as_str(),
                right.source_ref.as_str(),
            ))
    });
    let accepted_count = effective
        .values()
        .filter(|decision| decision.canonical_decision == "accept")
        .count();
    let rejected_count = effective
        .values()
        .filter(|decision| decision.canonical_decision == "reject")
        .count();
    let hold_count = effective
        .values()
        .filter(|decision| decision.canonical_decision == "hold")
        .count();
    let decisions_sha256 = serializable_sha256(&decisions)?;
    let mut manifest = FacialReviewExportManifest {
        schema_id: FACIAL_REVIEW_EXPORT_SCHEMA_ID.to_owned(),
        session_id: session.session_id.clone(),
        requested_by,
        exported_at_utc: request.exported_at_utc,
        dataset_name,
        repeats,
        output_root_ref,
        source_mutation: false,
        copy_mode: "manifest_only_no_source_mutation".to_owned(),
        funnel: serde_json::json!({
            "source_candidates": session.item_count,
            "decided": effective.len(),
            "accepted": accepted_count,
            "rejected": rejected_count,
            "hold": hold_count,
            "undecided": session.item_count.saturating_sub(effective.len()),
            "deduped_accepted": exported_files.len(),
            "exported": exported_files.len(),
            "export_problems": problems.len(),
            "allow_partial": request.allow_partial,
        }),
        lineage: serde_json::json!({
            "source": {
                "analysis_sha256": session.analysis_sha256,
                "receipt_sha256": session.receipt_sha256,
            },
            "session": {
                "session_id": session.session_id,
                "session_sha256": session.session_sha256,
            },
            "decisions": {
                "decision_receipt_count": decisions.len(),
                "decision_receipts_sha256": decisions_sha256,
            },
            "stages": [
                "source",
                "candidates",
                "decided",
                "deduped",
                "exported_dataset_manifest",
            ],
        }),
        exported_files,
        problems,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = serializable_sha256(&manifest)?;
    Ok(manifest)
}

fn review_item_from_row(row: &FacialIngestAnalysisRow) -> Result<FacialReviewItem, String> {
    let item_id = require_clean_ref("row.item_id", &row.item_id)?;
    let source_ref = require_clean_ref("row.source_ref", &row.source_ref)?;
    let file_name = require_clean_ref("row.file_name", &row.file_name)?;
    let content_hash = row
        .content_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let (basis_kind, basis_value) = content_hash
        .as_deref()
        .map(|hash| {
            (
                "content_hash_plus_source_ref",
                format!("{hash}|{source_ref}"),
            )
        })
        .unwrap_or_else(|| ("source_ref", source_ref.clone()));
    let stable_image_id = format!(
        "facial-review-image-{}",
        stable_hash(&format!("{basis_kind}:{basis_value}"))
    );
    Ok(FacialReviewItem {
        item_id,
        stable_image_id,
        id_basis: basis_kind.to_owned(),
        source_ref,
        file_name,
        lane: row.lane.clone(),
        content_hash,
        shard: 0,
        review_recommendation: row.review_recommendation.clone(),
        quality_band: row.quality_band.clone(),
        quality_score: row.quality_score,
        duplicate_group_id: row.duplicate_group_id.clone(),
        duplicate_role: row.duplicate_role.clone(),
        identity_verdict: row.identity_verdict.clone(),
    })
}

fn resolve_review_item<'a>(
    session: &'a FacialReviewSessionArtifact,
    id: &str,
) -> Result<&'a FacialReviewItem, String> {
    let needle = id.trim();
    if needle.is_empty() {
        return Err("facial review item id must not be empty".to_owned());
    }
    let matches = session
        .items
        .iter()
        .filter(|item| {
            item.item_id == needle
                || item.stable_image_id == needle
                || item
                    .content_hash
                    .as_deref()
                    .is_some_and(|hash| hash == needle)
                || item.stable_image_id.ends_with(needle)
        })
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(format!("unknown facial review item id: {needle}")),
        1 => Ok(matches[0]),
        count => Err(format!(
            "ambiguous facial review item id {needle}: {count} matches"
        )),
    }
}

fn effective_decisions(decisions: &[FacialReviewDecisionReceipt]) -> BTreeMap<String, String> {
    let mut ordered = decisions.to_vec();
    ordered.sort_by(|left, right| {
        (
            left.decided_at_utc.as_str(),
            left.decision_id.as_str(),
            left.actor.as_str(),
        )
            .cmp(&(
                right.decided_at_utc.as_str(),
                right.decision_id.as_str(),
                right.actor.as_str(),
            ))
    });
    let mut effective = BTreeMap::new();
    for decision in ordered {
        effective.insert(decision.stable_image_id, decision.canonical_decision);
    }
    effective
}

fn effective_decision_receipts(
    decisions: &[FacialReviewDecisionReceipt],
) -> BTreeMap<String, FacialReviewDecisionReceipt> {
    let mut ordered = decisions.to_vec();
    ordered.sort_by(|left, right| {
        (
            left.decided_at_utc.as_str(),
            left.decision_id.as_str(),
            left.actor.as_str(),
        )
            .cmp(&(
                right.decided_at_utc.as_str(),
                right.decision_id.as_str(),
                right.actor.as_str(),
            ))
    });
    let mut effective = BTreeMap::new();
    for decision in ordered {
        effective.insert(decision.stable_image_id.clone(), decision);
    }
    effective
}

fn active_claims_by_shard(
    claims: &[FacialReviewClaimReceipt],
    now: DateTime<Utc>,
) -> BTreeMap<usize, FacialReviewClaimReceipt> {
    let mut active = BTreeMap::new();
    for claim in claims {
        if parse_utc("claim.expires_at_utc", &claim.expires_at_utc)
            .map(|expires| expires > now)
            .unwrap_or(false)
        {
            active.insert(claim.shard, claim.clone());
        }
    }
    active
}

fn expired_claims_by_shard(
    claims: &[FacialReviewClaimReceipt],
    now: DateTime<Utc>,
) -> BTreeMap<usize, FacialReviewClaimReceipt> {
    let mut expired = BTreeMap::new();
    for claim in claims {
        if parse_utc("claim.expires_at_utc", &claim.expires_at_utc)
            .map(|expires| expires <= now)
            .unwrap_or(false)
        {
            expired.insert(claim.shard, claim.clone());
        }
    }
    expired
}

fn decision_conflicts(decisions: &[FacialReviewDecisionReceipt]) -> Vec<serde_json::Value> {
    let mut by_item = BTreeMap::<String, Vec<&FacialReviewDecisionReceipt>>::new();
    for decision in decisions {
        by_item
            .entry(decision.stable_image_id.clone())
            .or_default()
            .push(decision);
    }
    let mut conflicts = Vec::new();
    for (stable_image_id, item_decisions) in by_item {
        let distinct = item_decisions
            .iter()
            .map(|decision| decision.canonical_decision.as_str())
            .collect::<BTreeSet<_>>();
        if distinct.len() > 1 {
            conflicts.push(serde_json::json!({
                "stable_image_id": stable_image_id,
                "history": item_decisions
                    .iter()
                    .map(|decision| serde_json::json!({
                        "decision_id": decision.decision_id,
                        "actor": decision.actor,
                        "decision": decision.canonical_decision,
                        "decided_at_utc": decision.decided_at_utc,
                    }))
                    .collect::<Vec<_>>(),
            }));
        }
    }
    conflicts
}

fn normalize_decision(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "accept" | "accepted" | "pass" | "passed" | "keep" => Ok("accept".to_owned()),
        "reject" | "rejected" | "fail" | "failed" | "cull" => Ok("reject".to_owned()),
        "hold" | "held" | "unsure" | "review" => Ok("hold".to_owned()),
        other => Err(format!(
            "unsupported facial review decision {other}; expected pass/reject/unsure or accept/reject/hold"
        )),
    }
}

fn normalize_decision_filter(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("undecided") {
        Ok("undecided".to_owned())
    } else {
        normalize_decision(trimmed)
    }
}

fn normalize_tag(raw: &str) -> Result<String, String> {
    let tag = raw.trim().to_ascii_lowercase().replace(' ', "-");
    if tag.is_empty() {
        return Err("facial review decision tag must not be empty".to_owned());
    }
    if tag.contains(['/', '\\']) {
        return Err("facial review decision tag must not contain path separators".to_owned());
    }
    Ok(tag)
}

fn normalize_dataset_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("facial review export dataset_name must not be empty".to_owned());
    }
    if name.contains(char::is_whitespace) || name.contains(['/', '\\']) {
        return Err(
            "facial review export dataset_name must not contain whitespace or slashes".to_owned(),
        );
    }
    Ok(name.to_owned())
}

fn safe_file_name(raw: &str) -> String {
    Path::new(raw)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("image")
        .replace(['/', '\\'], "_")
}

fn export_problem(item: &FacialReviewItem, problem: &str) -> FacialReviewExportProblem {
    FacialReviewExportProblem {
        item_id: item.item_id.clone(),
        stable_image_id: item.stable_image_id.clone(),
        source_ref: item.source_ref.clone(),
        problem: problem.to_owned(),
    }
}

fn require_clean_ref(field: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(format!("{field} must not be empty or padded"));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err(format!("{field} must not contain newlines"));
    }
    Ok(trimmed.to_owned())
}

fn require_hashish(field: &str, value: &str) -> Result<String, String> {
    let value = require_clean_ref(field, value)?;
    if value.len() < 16 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("{field} must be a hex hash-like value"));
    }
    Ok(value)
}

fn parse_utc(field: &str, value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| format!("{field} must be RFC3339 UTC-compatible: {err}"))
}

fn serializable_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes =
        serde_json::to_vec(value).map_err(|err| format!("serialize hash payload: {err}"))?;
    Ok(sha256_hex(&bytes))
}

fn json_sha256(value: &serde_json::Value) -> Result<String, String> {
    serializable_sha256(value)
}

fn stable_hash(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
