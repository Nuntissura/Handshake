//! MT-157: FEMS bi-temporal indexing — `valid_from`/`valid_until` (world
//! time) + `recorded_at`/`invalidated_at` (system time) on MemoryItem.
//!
//! Bi-temporal queries reconstruct exactly which memories existed AND were
//! considered valid at a given instant. Ports the Graphiti/Zep temporal
//! knowledge graph pattern.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{Database, StorageError};

pub const MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE: &str = "memory_bitemporal_item";
pub const MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_TYPE: &str = "memory_bitemporal_manifest";
pub const MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_ID: &str = "memory_bitemporal_manifest_v1";
pub const MEMORY_BITEMPORAL_SOURCE_COMPONENT: &str = "memory_bitemporal_index";
pub const MEMORY_BITEMPORAL_EVENT_SCHEMA_ID: &str = "hsk.memory.bitemporal_item_event@1";

const MEMORY_BITEMPORAL_MANIFEST_SCHEMA_ID: &str = "hsk.memory.bitemporal_manifest_pointer@1";
const MEMORY_BITEMPORAL_TASK_RUN_ID: &str = "memory-bitemporal-index";
const MEMORY_BITEMPORAL_SESSION_RUN_ID: &str = "memory-bitemporal-index";

/// Bi-temporal payload attached to every persisted MemoryItem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitemporalStamps {
    /// World time: when the fact was true in the world.
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    /// System time: when we recorded / invalidated this fact.
    pub recorded_at: DateTime<Utc>,
    pub invalidated_at: Option<DateTime<Utc>>,
}

impl BitemporalStamps {
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            valid_from: now,
            valid_until: None,
            recorded_at: now,
            invalidated_at: None,
        }
    }

    /// Returns true if this item is visible at the given AsOfQuery.
    pub fn visible_at(&self, query: &AsOfQuery) -> bool {
        // World axis: valid_from <= as_of_world_time < (valid_until OR +inf)
        if self.valid_from > query.as_of_world_time {
            return false;
        }
        if let Some(until) = self.valid_until {
            if query.as_of_world_time >= until {
                return false;
            }
        }
        // System axis: recorded_at <= as_of_recorded_time < (invalidated_at OR +inf)
        if self.recorded_at > query.as_of_recorded_time {
            return false;
        }
        if let Some(invalidated) = self.invalidated_at {
            if query.as_of_recorded_time >= invalidated {
                return false;
            }
        }
        true
    }
}

/// As-of query exposes both temporal axes independently. Callers cannot
/// accidentally collapse to single-temporal queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsOfQuery {
    pub as_of_world_time: DateTime<Utc>,
    pub as_of_recorded_time: DateTime<Utc>,
}

impl AsOfQuery {
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            as_of_world_time: now,
            as_of_recorded_time: now,
        }
    }
}

/// Bi-temporally stamped item used by [`BitemporalIndex`]. Mirror of
/// MemoryItem with stamps inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitemporalItem {
    pub item_id: Uuid,
    pub stamps: BitemporalStamps,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BitemporalLedgerAction {
    Record,
    Invalidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BitemporalLedgerEventPayload {
    schema_id: String,
    action: BitemporalLedgerAction,
    item: BitemporalItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BitemporalManifestPointerPayload {
    schema_id: String,
    item_id: Uuid,
    item_event_id: Option<String>,
    item_event_sequence: Option<i64>,
}

/// In-memory bi-temporal index. The Postgres adapter below persists the same
/// item shape into `kernel_event_ledger`; this type keeps unit coverage of the
/// filter semantics without requiring a database.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct BitemporalIndex {
    pub items: Vec<BitemporalItem>,
}

impl BitemporalIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, item: BitemporalItem) {
        self.items.push(item);
    }

    /// Invalidate an item at a given system time. Item rows are NOT
    /// deleted — bitemporal semantics require they remain visible to
    /// queries with `as_of_recorded_time < invalidated_at`.
    pub fn invalidate(&mut self, item_id: Uuid, invalidated_at: DateTime<Utc>) -> bool {
        for item in &mut self.items {
            if item.item_id == item_id {
                item.stamps.invalidated_at = Some(invalidated_at);
                return true;
            }
        }
        false
    }

    /// Filter items by AsOfQuery.
    pub fn items_visible_at(
        &self,
        query: &AsOfQuery,
    ) -> Result<Vec<&BitemporalItem>, BitemporalError> {
        Ok(self
            .items
            .iter()
            .filter(|item| item.stamps.visible_at(query))
            .collect())
    }
}

/// Postgres-backed bitemporal memory index over the real append-only
/// `kernel_event_ledger`. It deliberately keeps `memory_item` nonexistent:
/// the item aggregate stores bitemporal snapshots and the manifest aggregate
/// stores item pointers so replay can enumerate items without a side table.
#[derive(Clone)]
pub struct PostgresBitemporalMemoryIndex {
    db: Arc<dyn Database>,
}

impl PostgresBitemporalMemoryIndex {
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    pub async fn record_item(&self, item: BitemporalItem) -> Result<KernelEvent, BitemporalError> {
        validate_item(&item)?;
        self.append_item_event_with_manifest(BitemporalLedgerAction::Record, item)
            .await
    }

    pub async fn invalidate_item(
        &self,
        item_id: Uuid,
        invalidated_at: DateTime<Utc>,
    ) -> Result<bool, BitemporalError> {
        let Some(mut item) = self.latest_item(item_id).await? else {
            return Ok(false);
        };
        item.stamps.invalidated_at = Some(invalidated_at);
        validate_item(&item)?;

        self.append_item_event_with_manifest(BitemporalLedgerAction::Invalidate, item)
            .await?;
        Ok(true)
    }

    pub async fn items_visible_at(
        &self,
        query: &AsOfQuery,
    ) -> Result<Vec<BitemporalItem>, BitemporalError> {
        let mut visible = Vec::new();
        for item_id in self.manifest_item_ids().await? {
            if let Some(item) = self
                .latest_item_as_of(item_id, query.as_of_recorded_time)
                .await?
            {
                validate_item(&item)?;
                if item.stamps.visible_at(query) {
                    visible.push(item);
                }
            }
        }
        Ok(visible)
    }

    async fn latest_item(&self, item_id: Uuid) -> Result<Option<BitemporalItem>, BitemporalError> {
        let events = self
            .db
            .list_kernel_events_for_aggregate(
                MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
                &item_id.to_string(),
            )
            .await
            .map_err(storage_error)?;

        let mut latest: Option<(i64, BitemporalItem)> = None;
        for event in events {
            let Some(payload) = decode_item_event(&event)? else {
                continue;
            };
            if payload.item.item_id != item_id {
                continue;
            }
            let replace = match latest.as_ref() {
                None => true,
                Some((sequence, _)) => event.event_sequence > *sequence,
            };
            if replace {
                latest = Some((event.event_sequence, payload.item));
            }
        }
        Ok(latest.map(|(_, item)| item))
    }

    async fn latest_item_as_of(
        &self,
        item_id: Uuid,
        as_of_recorded_time: DateTime<Utc>,
    ) -> Result<Option<BitemporalItem>, BitemporalError> {
        let events = self
            .db
            .list_kernel_events_for_aggregate(
                MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
                &item_id.to_string(),
            )
            .await
            .map_err(storage_error)?;

        let mut latest: Option<(DateTime<Utc>, i64, BitemporalItem)> = None;
        for event in events {
            let Some(payload) = decode_item_event(&event)? else {
                continue;
            };
            if payload.item.item_id != item_id {
                continue;
            }
            let Some(effective_time) = event_effective_recorded_time(&payload) else {
                continue;
            };
            if effective_time > as_of_recorded_time {
                continue;
            }
            let replace = match latest.as_ref() {
                None => true,
                Some((latest_time, latest_sequence, _)) => {
                    effective_time > *latest_time
                        || (effective_time == *latest_time
                            && event.event_sequence > *latest_sequence)
                }
            };
            if replace {
                latest = Some((effective_time, event.event_sequence, payload.item));
            }
        }
        Ok(latest.map(|(_, _, item)| item))
    }

    async fn manifest_item_ids(&self) -> Result<Vec<Uuid>, BitemporalError> {
        let events = self
            .db
            .list_kernel_events_for_aggregate(
                MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_TYPE,
                MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_ID,
            )
            .await
            .map_err(storage_error)?;

        let mut item_ids = Vec::new();
        for event in events {
            if event.aggregate_type != MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_TYPE {
                continue;
            }
            if schema_id(&event.payload) != Some(MEMORY_BITEMPORAL_MANIFEST_SCHEMA_ID) {
                continue;
            }
            let pointer: BitemporalManifestPointerPayload = serde_json::from_value(event.payload)
                .map_err(|err| {
                BitemporalError::Serialization {
                    message: err.to_string(),
                }
            })?;
            if pointer.schema_id != MEMORY_BITEMPORAL_MANIFEST_SCHEMA_ID {
                continue;
            }
            if !item_ids.contains(&pointer.item_id) {
                item_ids.push(pointer.item_id);
            }
        }
        Ok(item_ids)
    }

    async fn append_item_event_with_manifest(
        &self,
        action: BitemporalLedgerAction,
        item: BitemporalItem,
    ) -> Result<KernelEvent, BitemporalError> {
        let payload = serde_json::to_value(BitemporalLedgerEventPayload {
            schema_id: MEMORY_BITEMPORAL_EVENT_SCHEMA_ID.to_string(),
            action,
            item: item.clone(),
        })
        .map_err(|err| BitemporalError::Serialization {
            message: err.to_string(),
        })?;
        let payload_digest = sha256_json(&payload)?;
        let action_label = match action {
            BitemporalLedgerAction::Record => "record",
            BitemporalLedgerAction::Invalidate => "invalidate",
        };
        let item_idempotency_key = format!(
            "memory-bitemporal:{action_label}:{}:{payload_digest}",
            item.item_id
        );

        let item_event = NewKernelEvent::builder(
            MEMORY_BITEMPORAL_TASK_RUN_ID,
            MEMORY_BITEMPORAL_SESSION_RUN_ID,
            KernelEventType::ArtifactStored,
            KernelActor::System(MEMORY_BITEMPORAL_SOURCE_COMPONENT.to_string()),
        )
        .aggregate(
            MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE,
            item.item_id.to_string(),
        )
        .idempotency_key(item_idempotency_key.clone())
        .source_component(MEMORY_BITEMPORAL_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| BitemporalError::EventBuild {
            message: err.to_string(),
        })?;

        let manifest_payload = serde_json::to_value(BitemporalManifestPointerPayload {
            schema_id: MEMORY_BITEMPORAL_MANIFEST_SCHEMA_ID.to_string(),
            item_id: item.item_id,
            item_event_id: None,
            item_event_sequence: None,
        })
        .map_err(|err| BitemporalError::Serialization {
            message: err.to_string(),
        })?;

        let manifest_event = NewKernelEvent::builder(
            MEMORY_BITEMPORAL_TASK_RUN_ID,
            MEMORY_BITEMPORAL_SESSION_RUN_ID,
            KernelEventType::ArtifactStored,
            KernelActor::System(MEMORY_BITEMPORAL_SOURCE_COMPONENT.to_string()),
        )
        .aggregate(
            MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_TYPE,
            MEMORY_BITEMPORAL_MANIFEST_AGGREGATE_ID,
        )
        .idempotency_key(format!(
            "memory-bitemporal:manifest:{}:{}:{}",
            item.item_id, action_label, payload_digest
        ))
        .source_component(MEMORY_BITEMPORAL_SOURCE_COMPONENT)
        .payload(manifest_payload)
        .build()
        .map_err(|err| BitemporalError::EventBuild {
            message: err.to_string(),
        })?;

        let events = self
            .db
            .append_kernel_event_pair_atomic_with_causation(item_event, manifest_event)
            .await
            .map_err(storage_error)?;
        events
            .into_iter()
            .next()
            .ok_or_else(|| BitemporalError::Storage {
                message: "atomic bitemporal append returned no item event".to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BitemporalError {
    #[error("invalid temporal window: valid_until <= valid_from")]
    InvalidWorldWindow {
        valid_until: DateTime<Utc>,
        valid_from: DateTime<Utc>,
    },
    #[error("invalid system window: invalidated_at <= recorded_at")]
    InvalidSystemWindow {
        invalidated_at: DateTime<Utc>,
        recorded_at: DateTime<Utc>,
    },
    #[error("kernel event build failed: {message}")]
    EventBuild { message: String },
    #[error("bitemporal serialization failed: {message}")]
    Serialization { message: String },
    #[error("bitemporal storage failed: {message}")]
    Storage { message: String },
}

fn decode_item_event(
    event: &KernelEvent,
) -> Result<Option<BitemporalLedgerEventPayload>, BitemporalError> {
    if event.aggregate_type != MEMORY_BITEMPORAL_ITEM_AGGREGATE_TYPE {
        return Ok(None);
    }
    if schema_id(&event.payload) != Some(MEMORY_BITEMPORAL_EVENT_SCHEMA_ID) {
        return Ok(None);
    }
    let payload: BitemporalLedgerEventPayload = serde_json::from_value(event.payload.clone())
        .map_err(|err| BitemporalError::Serialization {
            message: err.to_string(),
        })?;
    if payload.schema_id != MEMORY_BITEMPORAL_EVENT_SCHEMA_ID {
        return Ok(None);
    }
    Ok(Some(payload))
}

fn event_effective_recorded_time(payload: &BitemporalLedgerEventPayload) -> Option<DateTime<Utc>> {
    match payload.action {
        BitemporalLedgerAction::Record => Some(payload.item.stamps.recorded_at),
        BitemporalLedgerAction::Invalidate => payload.item.stamps.invalidated_at,
    }
}

fn schema_id(payload: &Value) -> Option<&str> {
    payload.get("schema_id").and_then(Value::as_str)
}

fn validate_item(item: &BitemporalItem) -> Result<(), BitemporalError> {
    if let Some(valid_until) = item.stamps.valid_until {
        if valid_until <= item.stamps.valid_from {
            return Err(BitemporalError::InvalidWorldWindow {
                valid_until,
                valid_from: item.stamps.valid_from,
            });
        }
    }
    if let Some(invalidated_at) = item.stamps.invalidated_at {
        if invalidated_at <= item.stamps.recorded_at {
            return Err(BitemporalError::InvalidSystemWindow {
                invalidated_at,
                recorded_at: item.stamps.recorded_at,
            });
        }
    }
    Ok(())
}

fn sha256_json(value: &Value) -> Result<String, BitemporalError> {
    let bytes = serde_json::to_vec(value).map_err(|err| BitemporalError::Serialization {
        message: err.to_string(),
    })?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn storage_error(err: StorageError) -> BitemporalError {
    BitemporalError::Storage {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    fn item(id: u128, stamps: BitemporalStamps) -> BitemporalItem {
        BitemporalItem {
            item_id: Uuid::from_u128(id),
            stamps,
            payload: json!({"id": id}),
        }
    }

    #[test]
    fn item_visible_within_world_and_system_windows() {
        let stamps = BitemporalStamps {
            valid_from: at(100),
            valid_until: Some(at(200)),
            recorded_at: at(50),
            invalidated_at: Some(at(180)),
        };
        let query = AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(100),
        };
        assert!(stamps.visible_at(&query));
    }

    #[test]
    fn item_not_visible_after_invalidation() {
        let stamps = BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: Some(at(150)),
        };
        // Query after invalidation — not visible.
        let q_after = AsOfQuery {
            as_of_world_time: at(160),
            as_of_recorded_time: at(160),
        };
        assert!(!stamps.visible_at(&q_after));
        // Query before invalidation — still visible.
        let q_before = AsOfQuery {
            as_of_world_time: at(140),
            as_of_recorded_time: at(140),
        };
        assert!(stamps.visible_at(&q_before));
    }

    #[test]
    fn item_not_visible_before_recorded() {
        let stamps = BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        };
        let q = AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(40),
        };
        assert!(!stamps.visible_at(&q));
    }

    #[test]
    fn item_not_visible_after_world_expiry() {
        let stamps = BitemporalStamps {
            valid_from: at(100),
            valid_until: Some(at(200)),
            recorded_at: at(50),
            invalidated_at: None,
        };
        let q = AsOfQuery {
            as_of_world_time: at(250),
            as_of_recorded_time: at(100),
        };
        assert!(!stamps.visible_at(&q));
    }

    #[test]
    fn index_returns_only_visible_items() {
        let mut idx = BitemporalIndex::new();
        idx.insert(item(
            1,
            BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(50),
                invalidated_at: None,
            },
        ));
        idx.insert(item(
            2,
            BitemporalStamps {
                valid_from: at(300),
                valid_until: None,
                recorded_at: at(250),
                invalidated_at: None,
            },
        ));
        let q = AsOfQuery {
            as_of_world_time: at(200),
            as_of_recorded_time: at(150),
        };
        let visible = idx.items_visible_at(&q).unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].item_id, Uuid::from_u128(1));
    }

    #[test]
    fn invalidate_does_not_remove_item() {
        let mut idx = BitemporalIndex::new();
        idx.insert(item(
            1,
            BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(50),
                invalidated_at: None,
            },
        ));
        assert!(idx.invalidate(Uuid::from_u128(1), at(150)));
        assert_eq!(idx.items.len(), 1);
        // Visible before invalidation
        let q_before = AsOfQuery {
            as_of_world_time: at(120),
            as_of_recorded_time: at(120),
        };
        assert_eq!(idx.items_visible_at(&q_before).unwrap().len(), 1);
        // Not visible after invalidation
        let q_after = AsOfQuery {
            as_of_world_time: at(160),
            as_of_recorded_time: at(160),
        };
        assert_eq!(idx.items_visible_at(&q_after).unwrap().len(), 0);
    }

    #[test]
    fn replay_reconstruction_returns_identical_pack_for_same_query() {
        let mut idx = BitemporalIndex::new();
        idx.insert(item(
            1,
            BitemporalStamps {
                valid_from: at(100),
                valid_until: None,
                recorded_at: at(50),
                invalidated_at: None,
            },
        ));
        let q = AsOfQuery {
            as_of_world_time: at(120),
            as_of_recorded_time: at(120),
        };
        let a = idx
            .items_visible_at(&q)
            .unwrap()
            .into_iter()
            .map(|it| it.item_id)
            .collect::<Vec<_>>();
        let b = idx
            .items_visible_at(&q)
            .unwrap()
            .into_iter()
            .map(|it| it.item_id)
            .collect::<Vec<_>>();
        assert_eq!(a, b);
    }

    #[test]
    fn now_query_is_consistent() {
        // Sanity: AsOfQuery::now constructs both axes at the same instant.
        let q = AsOfQuery::now();
        assert!(
            (q.as_of_world_time - q.as_of_recorded_time)
                .num_milliseconds()
                .abs()
                < 1
        );
    }

    #[test]
    fn stamps_now_helper_sets_recorded_close_to_now() {
        let stamps = BitemporalStamps::now();
        let drift = (Utc::now() - stamps.recorded_at).num_seconds().abs();
        assert!(drift <= 1);
    }

    #[allow(dead_code)]
    fn ensure_bitemporal_duration_unused_is_kept() {
        // Documentation hook: Duration is part of chrono's surface even
        // though we never embed it directly. This avoids "unused import"
        // sweeps later if we add Duration-based helpers.
        let _ = Duration::seconds(0);
    }
}
