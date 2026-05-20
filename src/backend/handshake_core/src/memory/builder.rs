use std::collections::{BTreeMap, BTreeSet};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::capsule::{
    CapsuleAuditEntry, CapsuleAuditLog, MemoryCapsule, MemoryCapsuleError, RetrievalPolicy,
    TaskType,
};
use super::policy_table::{CapsulePolicyTable, RETRIEVAL_SCORING_FORMULA_V0};
use crate::ace::{
    FemsEntityRef, FemsSourceRef, FemsSourceRefKind, MemoryPack, MemoryPackBudgets,
    MemoryPackDeterminismMode, MemoryPackItem, MemoryPolicy,
};
use crate::kernel::fems_mt_handoff_memory_context::{
    project_fems_mt_handoff_memory_context, FemsMtHandoffItemKind, FemsMtHandoffMemoryContextV1,
    FemsMtHandoffMemoryItemV1,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildContext {
    pub task_type: TaskType,
    pub query: String,
    pub role_id: String,
    pub session_id: String,
    pub override_policy: Option<RetrievalPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievedItem {
    pub item_id: String,
    pub memory_class: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub summary: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured: Option<Value>,
    pub trust_level: String,
    pub confidence: f64,
    pub scope_refs: Vec<FemsEntityRef>,
    pub source_refs: Vec<FemsSourceRef>,
    pub score: f64,
    pub score_breakdown: BTreeMap<String, f64>,
    pub capsule_bytes: u64,
    pub token_estimate: u32,
    pub pinned: bool,
}

pub trait FemsRetriever {
    fn retrieve(&self, query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError>;
}

pub struct CapsuleBuilder<'a> {
    fems: &'a dyn FemsRetriever,
    policy_table: &'a CapsulePolicyTable,
}

impl<'a> CapsuleBuilder<'a> {
    pub fn new(fems: &'a dyn FemsRetriever, policy_table: &'a CapsulePolicyTable) -> Self {
        Self { fems, policy_table }
    }

    pub fn build(&self, ctx: BuildContext) -> Result<MemoryCapsule, BuilderError> {
        let policy = match ctx.override_policy {
            Some(policy) => {
                if policy.task_type != ctx.task_type {
                    return Err(BuilderError::PolicyTaskTypeMismatch {
                        context: ctx.task_type,
                        policy: policy.task_type,
                    });
                }
                policy
            }
            None => self.policy_table.policy_for(ctx.task_type),
        };
        validate_policy(&policy)?;

        let retrieved = self.fems.retrieve(&ctx.query, policy.top_k)?;
        for item in &retrieved {
            validate_retrieved_item(item)?;
        }
        let (pack, audit) = build_pack_and_audit(&retrieved, &policy)?;
        let mut capsule = MemoryCapsule::new(ctx.task_type, pack, policy)?;
        capsule.audit = audit;
        Ok(capsule)
    }
}

fn validate_policy(policy: &RetrievalPolicy) -> Result<(), BuilderError> {
    if policy.top_k == 0 {
        return Err(BuilderError::InvalidPolicy {
            field: "top_k",
            message: "top_k must be greater than zero",
        });
    }
    if policy.capsule_budget_bytes == 0 {
        return Err(BuilderError::InvalidPolicy {
            field: "capsule_budget_bytes",
            message: "capsule budget bytes must be greater than zero",
        });
    }
    if policy.scoring_formula_version != RETRIEVAL_SCORING_FORMULA_V0 {
        return Err(BuilderError::InvalidPolicy {
            field: "scoring_formula_version",
            message: "scoring formula version is not supported by this builder",
        });
    }
    Ok(())
}

fn validate_retrieved_item(item: &RetrievedItem) -> Result<(), BuilderError> {
    if item.item_id.trim().is_empty() {
        return Err(invalid_retrieved_item(
            item,
            "item_id",
            "item id must not be empty",
        ));
    }
    if !item.score.is_finite() {
        return Err(invalid_retrieved_item(
            item,
            "score",
            "score must be finite",
        ));
    }
    if !item.confidence.is_finite() {
        return Err(invalid_retrieved_item(
            item,
            "confidence",
            "confidence must be finite",
        ));
    }
    if item.capsule_bytes == 0 {
        return Err(invalid_retrieved_item(
            item,
            "capsule_bytes",
            "capsule byte cost must be greater than zero",
        ));
    }
    for (name, value) in &item.score_breakdown {
        if !value.is_finite() {
            return Err(BuilderError::InvalidRetrievedItem {
                item_id: item.item_id.clone(),
                field: "score_breakdown",
                message: format!("score breakdown component {name} must be finite"),
            });
        }
    }
    Ok(())
}

fn invalid_retrieved_item(
    item: &RetrievedItem,
    field: &'static str,
    message: &'static str,
) -> BuilderError {
    BuilderError::InvalidRetrievedItem {
        item_id: item.item_id.clone(),
        field,
        message: message.to_string(),
    }
}

fn build_pack_and_audit(
    retrieved: &[RetrievedItem],
    policy: &RetrievalPolicy,
) -> Result<(MemoryPack, CapsuleAuditLog), BuilderError> {
    let mut candidates = retrieved.to_vec();
    candidates.sort_by(|left, right| compare_retrieved_items(left, right));

    let mut included_ids = BTreeSet::new();
    let mut used_bytes = 0u64;
    let mut included_items = Vec::new();

    for item in &candidates {
        let include = if item.pinned {
            true
        } else if included_items.len() >= policy.top_k as usize {
            false
        } else {
            used_bytes.saturating_add(item.capsule_bytes) <= policy.capsule_budget_bytes
        };

        if include {
            used_bytes = used_bytes.saturating_add(item.capsule_bytes);
            included_ids.insert(item.item_id.clone());
            included_items.push(item.clone());
        }
    }

    let audit = CapsuleAuditLog {
        entries: candidates
            .iter()
            .map(|item| audit_entry_for(item, included_ids.contains(&item.item_id)))
            .collect(),
    };
    let mut pack = memory_pack_from_items(included_items, policy)?;
    pack.memory_pack_hash = pack
        .compute_hash()
        .map_err(|error| BuilderError::MemoryPackHash(error.to_string()))?;

    Ok((pack, audit))
}

fn compare_retrieved_items(left: &RetrievedItem, right: &RetrievedItem) -> std::cmp::Ordering {
    right
        .pinned
        .cmp(&left.pinned)
        .then_with(|| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| left.item_id.cmp(&right.item_id))
}

fn memory_pack_from_items(
    included_items: Vec<RetrievedItem>,
    policy: &RetrievalPolicy,
) -> Result<MemoryPack, BuilderError> {
    let token_estimate = included_items
        .iter()
        .fold(0u32, |sum, item| sum.saturating_add(item.token_estimate));
    let mut max_items_per_type = BTreeMap::new();
    let mut scope_refs_by_key = BTreeMap::new();

    for item in &included_items {
        *max_items_per_type
            .entry(item.item_type.clone())
            .or_insert(0u32) += 1;
        for scope_ref in &item.scope_refs {
            scope_refs_by_key.insert(scope_ref_key(scope_ref), scope_ref.clone());
        }
    }

    Ok(MemoryPack {
        schema_version: "memory_pack.v1".to_string(),
        pack_id: format!("memory-pack-{}", Uuid::now_v7()),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        determinism_mode: MemoryPackDeterminismMode::Strict,
        memory_policy: MemoryPolicy::WorkspaceScoped,
        scope_refs: scope_refs_by_key.into_values().collect(),
        budgets: MemoryPackBudgets {
            max_tokens: policy.capsule_budget_bytes.min(u64::from(u32::MAX)) as u32,
            max_items: policy.top_k,
            max_items_per_type,
        },
        items: included_items
            .into_iter()
            .map(memory_pack_item_from)
            .collect(),
        token_estimate,
        memory_pack_hash: String::new(),
        warnings: Vec::new(),
    })
}

fn memory_pack_item_from(item: RetrievedItem) -> MemoryPackItem {
    MemoryPackItem {
        memory_id: item.item_id,
        memory_class: item.memory_class,
        item_type: item.item_type,
        summary: item.summary,
        content: item.content,
        structured: item.structured,
        trust_level: item.trust_level,
        confidence: item.confidence,
        scope_refs: item.scope_refs,
        source_refs: item.source_refs,
        last_verified_at: None,
    }
}

fn audit_entry_for(item: &RetrievedItem, included: bool) -> CapsuleAuditEntry {
    let mut score_breakdown = item.score_breakdown.clone();
    score_breakdown.insert("capsule_bytes".to_string(), item.capsule_bytes as f64);

    CapsuleAuditEntry {
        item_id: item.item_id.clone(),
        source_uri: source_uri_for_item(item),
        included,
        suppression_reason: if included {
            None
        } else {
            Some("budget".to_string())
        },
        score: item.score,
        score_breakdown,
        pinned: item.pinned,
    }
}

fn source_uri_for_item(item: &RetrievedItem) -> String {
    item.source_refs
        .first()
        .map(source_uri_for_ref)
        .unwrap_or_else(|| format!("memory://item/{}", item.item_id))
}

fn source_uri_for_ref(source_ref: &FemsSourceRef) -> String {
    let mut uri = format!(
        "fems://source/{}/{}",
        source_ref_kind_slug(source_ref.kind),
        source_ref.id
    );
    if let Some(selector) = &source_ref.selector {
        uri.push('#');
        uri.push_str(selector.trim_start_matches('#'));
    }
    uri
}

fn source_ref_kind_slug(kind: FemsSourceRefKind) -> &'static str {
    match kind {
        FemsSourceRefKind::Span => "span",
        FemsSourceRefKind::JobStep => "job_step",
        FemsSourceRefKind::Artifact => "artifact",
        FemsSourceRefKind::Entity => "entity",
        FemsSourceRefKind::DocBlock => "doc_block",
        FemsSourceRefKind::Kv => "kv",
    }
}

fn scope_ref_key(scope_ref: &FemsEntityRef) -> String {
    format!(
        "{}:{}:{}",
        scope_ref.artefact_type, scope_ref.artefact_id, scope_ref.selector
    )
}

#[derive(Debug, Clone)]
pub struct FemsMtHandoffRetriever {
    context: FemsMtHandoffMemoryContextV1,
}

impl FemsMtHandoffRetriever {
    pub fn new(context: FemsMtHandoffMemoryContextV1) -> Self {
        Self { context }
    }
}

impl FemsRetriever for FemsMtHandoffRetriever {
    fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        let projection =
            project_fems_mt_handoff_memory_context(&self.context).map_err(|errors| {
                FemsError::new(format!(
                    "FEMS MT handoff context validation failed: {:?}",
                    errors
                ))
            })?;
        let boosted_ids = projection
            .boosted_item_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        projection
            .selected_item_ids
            .iter()
            .take(top_k as usize)
            .map(|item_id| {
                let item = self
                    .context
                    .carried_items
                    .iter()
                    .find(|candidate| candidate.item_id == *item_id)
                    .ok_or_else(|| {
                        FemsError::new(format!(
                            "FEMS MT handoff projection referenced missing item {item_id}"
                        ))
                    })?;
                Ok(retrieved_item_from_handoff(
                    item,
                    boosted_ids.contains(item_id),
                ))
            })
            .collect()
    }
}

fn retrieved_item_from_handoff(item: &FemsMtHandoffMemoryItemV1, pinned: bool) -> RetrievedItem {
    let score = f64::from(item.base_score_x100) / 100.0;
    let item_type = handoff_item_kind_slug(item.kind).to_string();
    let capsule_bytes = u64::from(item.token_count).saturating_mul(4);

    RetrievedItem {
        item_id: item.item_id.clone(),
        memory_class: "fems_handoff".to_string(),
        item_type,
        summary: item.memory_ref.clone(),
        content: format!("{} {}", item.memory_ref, item.provenance_ref),
        structured: None,
        trust_level: "fems_handoff".to_string(),
        confidence: score,
        scope_refs: Vec::new(),
        source_refs: vec![FemsSourceRef {
            kind: FemsSourceRefKind::Artifact,
            id: item.provenance_ref.clone(),
            hash: None,
            selector: Some(item.memory_ref.clone()),
            created_at: None,
            classification: None,
        }],
        score,
        score_breakdown: BTreeMap::from([
            (
                "base_score_x100".to_string(),
                f64::from(item.base_score_x100),
            ),
            ("token_count".to_string(), f64::from(item.token_count)),
        ]),
        capsule_bytes,
        token_estimate: item.token_count,
        pinned,
    }
}

fn handoff_item_kind_slug(kind: FemsMtHandoffItemKind) -> &'static str {
    match kind {
        FemsMtHandoffItemKind::MemoryPackItem => "memory_pack_item",
        FemsMtHandoffItemKind::InsightCheckpoint => "insight_checkpoint",
        FemsMtHandoffItemKind::FailedAttempt => "failed_attempt",
        FemsMtHandoffItemKind::RecommendedProceduralItem => "recommended_procedural_item",
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("FEMS retrieval failed: {message}")]
pub struct FemsError {
    pub message: String,
}

impl FemsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuilderError {
    #[error("{0}")]
    Fems(#[from] FemsError),
    #[error("retrieval policy task type {policy:?} does not match build context {context:?}")]
    PolicyTaskTypeMismatch { context: TaskType, policy: TaskType },
    #[error("invalid retrieval policy {field}: {message}")]
    InvalidPolicy {
        field: &'static str,
        message: &'static str,
    },
    #[error("invalid retrieved item {item_id} {field}: {message}")]
    InvalidRetrievedItem {
        item_id: String,
        field: &'static str,
        message: String,
    },
    #[error("{0}")]
    Capsule(#[from] MemoryCapsuleError),
    #[error("memory pack hash failed: {0}")]
    MemoryPackHash(String),
}
