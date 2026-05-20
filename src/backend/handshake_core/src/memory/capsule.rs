use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::ace::{FemsSourceRef, FemsSourceRefKind, MemoryPack, MemoryPackItem};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    ValidatorHbrTestPacket,
    KernelBuilderMtImplementation,
    IntegrationValidatorBatchReview,
    OperatorTriage,
    SwarmHarnessSession,
    ProcessLedgerInspection,
    SelfImprovementLoopEval,
    GeneralRetrieval,
}

impl TaskType {
    pub const ALL: [Self; 8] = [
        Self::ValidatorHbrTestPacket,
        Self::KernelBuilderMtImplementation,
        Self::IntegrationValidatorBatchReview,
        Self::OperatorTriage,
        Self::SwarmHarnessSession,
        Self::ProcessLedgerInspection,
        Self::SelfImprovementLoopEval,
        Self::GeneralRetrieval,
    ];

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DegradationTier {
    Strict,
    Tiered,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalPolicy {
    pub top_k: u32,
    pub capsule_budget_bytes: u64,
    pub task_type: TaskType,
    pub scoring_formula_version: String,
    pub graceful_degradation_tier: DegradationTier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapsuleAuditEntry {
    pub item_id: String,
    pub source_uri: String,
    pub included: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppression_reason: Option<String>,
    pub score: f64,
    pub score_breakdown: BTreeMap<String, f64>,
    pub pinned: bool,
}

impl CapsuleAuditEntry {
    fn suppressed(item_id: String, source_uri: String, reason: String) -> Self {
        Self {
            item_id,
            source_uri,
            included: false,
            suppression_reason: Some(reason),
            score: 0.0,
            score_breakdown: BTreeMap::new(),
            pinned: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CapsuleAuditLog {
    pub entries: Vec<CapsuleAuditEntry>,
}

impl CapsuleAuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry(&self, item_id: &str) -> Option<&CapsuleAuditEntry> {
        self.entries.iter().find(|entry| entry.item_id == item_id)
    }

    fn entry_mut(&mut self, item_id: &str) -> Option<&mut CapsuleAuditEntry> {
        self.entries
            .iter_mut()
            .find(|entry| entry.item_id == item_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryCapsule {
    pub id: Uuid,
    pub task_type: TaskType,
    pub pack: MemoryPack,
    pub policy: RetrievalPolicy,
    pub audit: CapsuleAuditLog,
    pub built_at_utc: DateTime<Utc>,
    pub source_hash: String,
}

impl MemoryCapsule {
    pub fn new(
        task_type: TaskType,
        pack: MemoryPack,
        policy: RetrievalPolicy,
    ) -> Result<Self, MemoryCapsuleError> {
        if policy.task_type != task_type {
            return Err(MemoryCapsuleError::TaskTypeMismatch {
                capsule: task_type,
                policy: policy.task_type,
            });
        }

        let source_hash = source_hash_for_items(&pack.items)?;

        Ok(Self {
            id: Uuid::now_v7(),
            task_type,
            pack,
            policy,
            audit: CapsuleAuditLog::new(),
            built_at_utc: Utc::now(),
            source_hash,
        })
    }

    pub fn suppress_item(
        &mut self,
        item_id: &str,
        reason: impl Into<String>,
    ) -> Result<(), MemoryCapsuleError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(MemoryCapsuleError::EmptySuppressionReason);
        }

        if let Some(entry) = self.audit.entry_mut(item_id) {
            entry.included = false;
            entry.suppression_reason = Some(reason);
            return Ok(());
        }

        let item = self
            .pack
            .items
            .iter()
            .find(|item| item.memory_id == item_id)
            .ok_or_else(|| MemoryCapsuleError::ItemNotFound {
                item_id: item_id.to_string(),
            })?;

        self.audit.entries.push(CapsuleAuditEntry::suppressed(
            item.memory_id.clone(),
            source_uri_for_item(item),
            reason,
        ));
        Ok(())
    }
}

fn source_hash_for_items(items: &[MemoryPackItem]) -> Result<String, MemoryCapsuleError> {
    let mut sortable_items = items
        .iter()
        .map(|item| {
            let value = serde_json::to_value(item)?;
            let key = crate::llm::canonical_json_bytes_nfc(&value);
            Ok((key, item))
        })
        .collect::<Result<Vec<_>, serde_json::Error>>()?;
    sortable_items.sort_by(|left, right| left.0.cmp(&right.0));

    let sorted_items = sortable_items
        .into_iter()
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    let value = json!({
        "schema_version": "memory_capsule.source_hash.v1",
        "items": sorted_items,
    });
    Ok(crate::llm::sha256_hex(
        crate::llm::canonical_json_bytes_nfc(&value).as_slice(),
    ))
}

fn source_uri_for_item(item: &MemoryPackItem) -> String {
    item.source_refs
        .first()
        .map(source_uri_for_ref)
        .unwrap_or_else(|| format!("memory://item/{}", item.memory_id))
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MemoryCapsuleError {
    #[error(
        "memory capsule task type {capsule:?} does not match retrieval policy task type {policy:?}"
    )]
    TaskTypeMismatch { capsule: TaskType, policy: TaskType },
    #[error("memory capsule item not found: {item_id}")]
    ItemNotFound { item_id: String },
    #[error("memory capsule suppression reason cannot be empty")]
    EmptySuppressionReason,
    #[error("memory capsule source hash serialization failed: {0}")]
    SourceHashSerialization(String),
}

impl From<serde_json::Error> for MemoryCapsuleError {
    fn from(value: serde_json::Error) -> Self {
        Self::SourceHashSerialization(value.to_string())
    }
}
