use crate::atelier::facial_native::common::FacialNativeImageContext;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const HASH_FEATURE_ID: &str = "imagededup:hash_duplicates";
pub const REMOVE_FEATURE_ID: &str = "imagededup:remove_candidates";
pub const SOURCE_FAMILY: &str = "imagededup";
pub const HASH_SOURCE: &str = "imagededup_native_content_hash_exact_v1";
pub const HASH_METHOD: &str = "content_hash_exact";
pub const REMOVE_SOURCE: &str = "imagededup_native_remove_candidates_v1";

#[derive(Clone, Debug)]
pub struct DedupeAssignment {
    pub group_key: String,
    pub group_id: String,
    pub group_size: usize,
    pub role: String,
    pub source: String,
    pub reason: String,
}

pub fn exact_hash_assignments(
    contexts: &[FacialNativeImageContext],
) -> BTreeMap<String, DedupeAssignment> {
    let mut by_key = BTreeMap::<String, Vec<&FacialNativeImageContext>>::new();
    for ctx in contexts {
        let key = ctx
            .content_hash
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("content_hash:{value}"))
            .unwrap_or_else(|| format!("singleton:{}", ctx.item_id));
        by_key.entry(key).or_default().push(ctx);
    }

    let mut assignments = BTreeMap::new();
    for (group_key, mut members) in by_key {
        members.sort_by(compare_keep_preference);
        let group_size = members.len();
        let representative_id = members
            .first()
            .map(|ctx| ctx.item_id.as_str())
            .unwrap_or_default()
            .to_owned();
        let group_id = format!("facial-dedupe-{}", stable_hash(&group_key));
        for ctx in members {
            let has_hash = ctx.has_content_hash();
            let role = if group_size <= 1 {
                "singleton"
            } else if ctx.item_id == representative_id {
                "representative"
            } else {
                "duplicate"
            }
            .to_owned();
            assignments.insert(
                ctx.item_id.clone(),
                DedupeAssignment {
                    group_key: group_key.clone(),
                    group_id: group_id.clone(),
                    group_size,
                    role,
                    source: if has_hash {
                        HASH_SOURCE.to_owned()
                    } else {
                        "imagededup_native_missing_hash_singleton_v1".to_owned()
                    },
                    reason: if has_hash {
                        "exact_content_hash_group".to_owned()
                    } else {
                        "missing_content_hash_singleton".to_owned()
                    },
                },
            );
        }
    }
    assignments
}

pub fn assignment_payload(ctx: &FacialNativeImageContext, assignment: &DedupeAssignment) -> Value {
    json!({
        "feature_id": HASH_FEATURE_ID,
        "source_family": SOURCE_FAMILY,
        "source": assignment.source,
        "method": HASH_METHOD,
        "path": ctx.source_ref,
        "group_key": assignment.group_key,
        "group_id": assignment.group_id,
        "group_size": assignment.group_size,
        "role": assignment.role,
        "reason": assignment.reason,
    })
}

pub fn group_summary(
    contexts: &[FacialNativeImageContext],
    assignments: &BTreeMap<String, DedupeAssignment>,
) -> Value {
    let mut groups = BTreeMap::<String, Vec<&FacialNativeImageContext>>::new();
    for ctx in contexts {
        if let Some(assignment) = assignments.get(&ctx.item_id) {
            if assignment.group_size > 1 {
                groups
                    .entry(assignment.group_key.clone())
                    .or_default()
                    .push(ctx);
            }
        }
    }

    let mut output = BTreeMap::new();
    let mut duplicate_members = 0usize;
    for (group_key, mut members) in groups {
        members.sort_by(|left, right| left.source_ref.cmp(&right.source_ref));
        duplicate_members += members.len();
        let paths = members
            .iter()
            .map(|ctx| ctx.source_ref.clone())
            .collect::<Vec<_>>();
        let total_size = members
            .iter()
            .map(|ctx| ctx.byte_len.max(0) as u64)
            .sum::<u64>();
        let best_keep = members
            .iter()
            .max_by(|left, right| {
                left.byte_len
                    .cmp(&right.byte_len)
                    .then_with(|| right.source_ref.cmp(&left.source_ref))
            })
            .map(|ctx| ctx.source_ref.clone())
            .unwrap_or_default();
        output.insert(
            group_key.clone(),
            json!({
                "group_key": group_key,
                "type": "sha256_content_hash",
                "paths": paths,
                "count": members.len(),
                "type_size_total": total_size,
                "best_keep": best_keep,
                "method": "exact_content_hash_group",
            }),
        );
    }

    json!({
        "feature": HASH_FEATURE_ID,
        "source_family": SOURCE_FAMILY,
        "source": HASH_SOURCE,
        "method": HASH_METHOD,
        "count": output.len(),
        "duplicates_found": duplicate_members,
        "coverage_percent": if contexts.is_empty() {
            0.0
        } else {
            (duplicate_members as f64) * 100.0 / contexts.len() as f64
        },
        "groups": output,
    })
}

pub fn remove_candidates(
    contexts: &[FacialNativeImageContext],
    assignments: &BTreeMap<String, DedupeAssignment>,
) -> Value {
    let mut by_group = BTreeMap::<String, Vec<&FacialNativeImageContext>>::new();
    for ctx in contexts {
        if let Some(assignment) = assignments.get(&ctx.item_id) {
            if assignment.group_size > 1 {
                by_group
                    .entry(assignment.group_key.clone())
                    .or_default()
                    .push(ctx);
            }
        }
    }

    let mut remove_list = Vec::new();
    let mut component_id = 0usize;
    for (_group_key, mut members) in by_group {
        members.sort_by(|left, right| {
            right
                .byte_len
                .cmp(&left.byte_len)
                .then_with(|| left.source_ref.cmp(&right.source_ref))
        });
        let Some(keeper) = members.first().copied() else {
            continue;
        };
        component_id += 1;
        let keeper_score = keep_score(keeper);
        for candidate in members.into_iter().skip(1) {
            let remove_score = keep_score(candidate);
            remove_list.push(json!({
                "path": candidate.source_ref,
                "action": "review_remove_candidate",
                "keep": keeper.source_ref,
                "component_id": component_id,
                "similarity_to_keep": 100.0,
                "decision": {
                    "keep_score": keeper_score,
                    "remove_score": remove_score,
                    "score_delta": keeper_score - remove_score,
                    "reason": "exact_content_hash_duplicate_keep_largest_then_path",
                },
            }));
        }
    }
    remove_list.sort_by(|left, right| {
        left.get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .cmp(
                right
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
    });

    json!({
        "feature": REMOVE_FEATURE_ID,
        "source_family": SOURCE_FAMILY,
        "source": REMOVE_SOURCE,
        "method": "exact_content_hash_review_recommendation",
        "count": remove_list.len(),
        "remove_list": remove_list,
        "policy": {
            "non_destructive": true,
            "action_is_recommendation_only": true,
            "component_scoring": "largest_file_then_path",
        },
        "images_scanned": contexts.len(),
    })
}

fn keep_score(ctx: &FacialNativeImageContext) -> f64 {
    let decoded = if ctx.is_decoded() { 50.0 } else { 0.0 };
    let size = if ctx.byte_len <= 0 {
        0.0
    } else {
        ((ctx.byte_len as f64).ln() * 5.0).clamp(0.0, 100.0)
    };
    let dimensions = ctx
        .image_width
        .zip(ctx.image_height)
        .map(|(w, h)| ((w as f64 * h as f64).ln() * 3.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    decoded + size + dimensions
}

fn compare_keep_preference(
    left: &&FacialNativeImageContext,
    right: &&FacialNativeImageContext,
) -> std::cmp::Ordering {
    right
        .byte_len
        .cmp(&left.byte_len)
        .then_with(|| left.source_ref.cmp(&right.source_ref))
        .then_with(|| left.item_id.cmp(&right.item_id))
}

fn stable_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}
