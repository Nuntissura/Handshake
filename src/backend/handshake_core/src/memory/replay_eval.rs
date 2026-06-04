//! MT-163: deterministic FEMS acceptance replay evaluation.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::{
    bitemporal::{AsOfQuery, BitemporalIndex, BitemporalItem},
    builder::{BuildContext, CapsuleBuilder, FemsError, FemsRetriever, RetrievedItem},
    capsule::MemoryCapsule,
    ipc::MemoryCapsuleIpcStore,
    persistence::CapsuleRecord,
    policy_table::CapsulePolicyTable,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayRequest {
    pub capsule_id: Uuid,
    pub replay_as_of: AsOfQuery,
    pub expected_source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayDifference {
    pub items_present_now_absent_then: Vec<Uuid>,
    pub items_absent_now_present_then: Vec<Uuid>,
    pub items_with_different_scores: Vec<(Uuid, f64, f64)>,
}

impl ReplayDifference {
    pub fn is_empty(&self) -> bool {
        self.items_present_now_absent_then.is_empty()
            && self.items_absent_now_present_then.is_empty()
            && self.items_with_different_scores.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaySnapshot {
    pub item_ids: Vec<Uuid>,
    pub item_scores: Vec<(Uuid, f64)>,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayResult {
    pub replayed_capsule: MemoryCapsule,
    pub original_snapshot: ReplaySnapshot,
    pub replay_difference: ReplayDifference,
    pub replay_succeeded: bool,
    pub replayed_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ReplayError {
    #[error("capsule not found: {capsule_id}")]
    CapsuleNotFound { capsule_id: Uuid },
    #[error("source hash mismatch: expected {expected}, got {got}")]
    HashMismatch { expected: String, got: String },
    #[error("capsule record read failed: {message}")]
    RecordRead { message: String },
    #[error("bitemporal accessor error: {message}")]
    Accessor { message: String },
    #[error("bitemporal item {item_id} payload is not replayable: {message}")]
    InvalidBitemporalPayload { item_id: Uuid, message: String },
    #[error("capsule record item id is not a UUID: {item_id}: {message}")]
    InvalidRecordItemId { item_id: String, message: String },
    #[error("capsule replay build failed: {message}")]
    Builder { message: String },
}

/// Source for the original capsule record used in replay.
pub trait CapsuleRecordReader {
    fn read_record(&self, capsule_id: Uuid) -> Result<CapsuleRecord, ReplayError>;
}

impl<T> CapsuleRecordReader for T
where
    T: MemoryCapsuleIpcStore + ?Sized,
{
    fn read_record(&self, capsule_id: Uuid) -> Result<CapsuleRecord, ReplayError> {
        self.get_capsule_record(capsule_id)
            .map_err(|error| ReplayError::RecordRead {
                message: error.to_string(),
            })?
            .ok_or(ReplayError::CapsuleNotFound { capsule_id })
    }
}

/// Source for as-of memory items used by the replay. Implementations must
/// return only items visible at the supplied bi-temporal query.
pub trait BitemporalAccessor {
    fn items_visible_at(&self, query: &AsOfQuery) -> Result<Vec<BitemporalItem>, ReplayError>;
}

impl BitemporalAccessor for BitemporalIndex {
    fn items_visible_at(&self, query: &AsOfQuery) -> Result<Vec<BitemporalItem>, ReplayError> {
        BitemporalIndex::items_visible_at(self, query)
            .map(|items| items.into_iter().cloned().collect())
            .map_err(|error| ReplayError::Accessor {
                message: error.to_string(),
            })
    }
}

pub struct ReplayEvaluator;

impl ReplayEvaluator {
    pub fn replay(
        req: ReplayRequest,
        recorder: &dyn CapsuleRecordReader,
        bitemporal: &dyn BitemporalAccessor,
        _builder: &CapsuleBuilder<'_>,
    ) -> Result<ReplayResult, ReplayError> {
        // A CapsuleBuilder is bound to the retriever it was constructed with.
        // Replay must use only the frozen bitemporal state, so the public
        // contract accepts the builder handle while the actual rebuild below
        // constructs a scoped builder over ReplayFrozenFemsRetriever.
        let record = recorder.read_record(req.capsule_id)?;
        if record.capsule_source_hash != req.expected_source_hash {
            return Err(ReplayError::HashMismatch {
                expected: req.expected_source_hash.clone(),
                got: record.capsule_source_hash.clone(),
            });
        }

        let original_snapshot = snapshot_from_record(&record)?;
        let replay_items = bitemporal
            .items_visible_at(&req.replay_as_of)?
            .into_iter()
            .map(retrieved_item_from_bitemporal)
            .collect::<Result<Vec<_>, ReplayError>>()?;

        let frozen_fems = ReplayFrozenFemsRetriever::new(replay_items);
        let policy_table = CapsulePolicyTable;
        let replay_builder = CapsuleBuilder::new(&frozen_fems, &policy_table);
        let replayed_capsule = replay_builder
            .build(BuildContext {
                task_type: record.task_type,
                query: replay_query(&record, &req.replay_as_of),
                role_id: record.role_id.clone(),
                session_id: record.session_id.clone(),
                override_policy: Some(record.policy.clone()),
            })
            .map_err(|error| ReplayError::Builder {
                message: error.to_string(),
            })?;

        let replayed_snapshot = snapshot_from_capsule(&replayed_capsule)?;
        let replay_difference = diff_snapshots(&original_snapshot, &replayed_snapshot);
        let replay_succeeded = record.capsule_source_hash == replayed_capsule.source_hash
            && replay_difference.is_empty();

        Ok(ReplayResult {
            replayed_capsule,
            original_snapshot,
            replay_difference,
            replay_succeeded,
            replayed_at_utc: Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
struct ReplayFrozenFemsRetriever {
    items: Vec<RetrievedItem>,
}

impl ReplayFrozenFemsRetriever {
    fn new(items: Vec<RetrievedItem>) -> Self {
        Self { items }
    }
}

impl FemsRetriever for ReplayFrozenFemsRetriever {
    fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        let mut items = self.items.clone();
        items.sort_by(compare_retrieved_items_for_replay);
        items.truncate(top_k as usize);
        Ok(items)
    }
}

fn compare_retrieved_items_for_replay(
    left: &RetrievedItem,
    right: &RetrievedItem,
) -> std::cmp::Ordering {
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

fn replay_query(record: &CapsuleRecord, query: &AsOfQuery) -> String {
    format!(
        "replay:{}:{}:{}",
        record.capsule_id,
        query.as_of_world_time.to_rfc3339(),
        query.as_of_recorded_time.to_rfc3339()
    )
}

fn retrieved_item_from_bitemporal(item: BitemporalItem) -> Result<RetrievedItem, ReplayError> {
    let item_id = item.item_id;
    let mut payload = item.payload;
    if let Value::Object(object) = &mut payload {
        object
            .entry("item_id".to_string())
            .or_insert_with(|| Value::String(item_id.to_string()));
    }

    let retrieved: RetrievedItem =
        serde_json::from_value(payload).map_err(|error| ReplayError::InvalidBitemporalPayload {
            item_id,
            message: error.to_string(),
        })?;
    if retrieved.item_id != item_id.to_string() {
        return Err(ReplayError::InvalidBitemporalPayload {
            item_id,
            message: format!(
                "payload item_id {} does not match bitemporal item id {}",
                retrieved.item_id, item_id
            ),
        });
    }
    Ok(retrieved)
}

fn snapshot_from_record(record: &CapsuleRecord) -> Result<ReplaySnapshot, ReplayError> {
    snapshot_from_audit_entries(
        record
            .audit_log
            .entries
            .iter()
            .filter(|entry| entry.included)
            .map(|entry| (entry.item_id.as_str(), entry.score)),
        record.capsule_source_hash.clone(),
    )
}

fn snapshot_from_capsule(capsule: &MemoryCapsule) -> Result<ReplaySnapshot, ReplayError> {
    snapshot_from_audit_entries(
        capsule
            .audit
            .entries
            .iter()
            .filter(|entry| entry.included)
            .map(|entry| (entry.item_id.as_str(), entry.score)),
        capsule.source_hash.clone(),
    )
}

fn snapshot_from_audit_entries<'a>(
    entries: impl Iterator<Item = (&'a str, f64)>,
    source_hash: String,
) -> Result<ReplaySnapshot, ReplayError> {
    let mut scores = BTreeMap::new();
    for (item_id, score) in entries {
        let item_uuid = parse_record_item_id(item_id)?;
        scores.insert(item_uuid, score);
    }

    Ok(ReplaySnapshot {
        item_ids: scores.keys().copied().collect(),
        item_scores: scores.into_iter().collect(),
        source_hash,
    })
}

fn parse_record_item_id(item_id: &str) -> Result<Uuid, ReplayError> {
    Uuid::parse_str(item_id).map_err(|error| ReplayError::InvalidRecordItemId {
        item_id: item_id.to_string(),
        message: error.to_string(),
    })
}

fn diff_snapshots(original: &ReplaySnapshot, replayed: &ReplaySnapshot) -> ReplayDifference {
    let original_ids: BTreeSet<Uuid> = original.item_ids.iter().copied().collect();
    let replayed_ids: BTreeSet<Uuid> = replayed.item_ids.iter().copied().collect();

    let items_present_now_absent_then = replayed_ids.difference(&original_ids).copied().collect();
    let items_absent_now_present_then = original_ids.difference(&replayed_ids).copied().collect();

    let original_scores: BTreeMap<Uuid, f64> = original.item_scores.iter().copied().collect();
    let replayed_scores: BTreeMap<Uuid, f64> = replayed.item_scores.iter().copied().collect();

    let mut items_with_different_scores = Vec::new();
    for id in original_ids.intersection(&replayed_ids) {
        let original_score = original_scores.get(id).copied().unwrap_or(0.0);
        let replayed_score = replayed_scores.get(id).copied().unwrap_or(0.0);
        if (original_score - replayed_score).abs() > 1e-9 {
            items_with_different_scores.push((*id, original_score, replayed_score));
        }
    }

    ReplayDifference {
        items_present_now_absent_then,
        items_absent_now_present_then,
        items_with_different_scores,
    }
}
