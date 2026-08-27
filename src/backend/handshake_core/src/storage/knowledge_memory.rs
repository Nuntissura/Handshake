//! WP-KERNEL-009 MemoryGraphAndClaims storage (MT-113..MT-128).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 and the
//! WP-009 contract field `translated_memory_system_spec`. This module is the
//! storage surface for the MemoryGraph layer (`knowledge_memory_*`,
//! migrations 0240-0259): the ontology of schema terms, the memory-fact
//! records (S/P/O claims that reuse the claim lifecycle), the ontology/alias
//! links, conflict-detection / conflict-resolution agent-job records, bridge
//! edges, and claim authority labels.
//!
//! Design: the MemoryGraph EXTENDS the committed knowledge substrate (entities
//! 0135, edges 0136, claims 0137, spans 0134, passages 0138). It REUSES the
//! claim lifecycle (proposed/accepted/conflicted/retired), the claim conflict
//! table + EventLedger-backed resolution, and the deterministic edge derivation
//! rather than duplicating them. A `MemoryFact` is a structured subject/
//! predicate/object view *backed by* a `knowledge_claims` row, so the claim's
//! evidence-span requirement, transition guard (0200), and conflict machinery
//! all hold for every memory fact for free.
//!
//! Pattern follows `storage/knowledge_crdt.rs`: free async functions over the
//! embedded SurrealDB store (`&SurrealStorage`) rather than widening the legacy
//! `Database` trait. There is NO in-memory, SQLite, or fixture fallback:
//! without the durable store every function fails closed with a typed
//! `StorageError`.
//!
//! WP-KERNEL-012 MT-136 note on foreign keys: every id that PostgreSQL held as
//! an FK column is a RECORD LINK here (`record<table>` with
//! `ASSERT record::exists($value)`), so a fact that cites a missing claim,
//! entity or ontology term is rejected by the store exactly as the relational
//! FK rejected it. The public structs keep their `String` shape; the link keys
//! are unwrapped on read and rebuilt on write.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

use super::surreal::{SurrealStorage, SurrealStorageError};
use super::{StorageError, StorageResult};

const WORKSPACES_TABLE: &str = "workspaces";
const ONTOLOGY_TERMS_TABLE: &str = "knowledge_memory_ontology_terms";
const CLAIMS_TABLE: &str = "knowledge_claims";
const ENTITIES_TABLE: &str = "knowledge_entities";
const INDEX_RUNS_TABLE: &str = "knowledge_index_runs";
const KERNEL_EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";
const CLAIM_CONFLICTS_TABLE: &str = "knowledge_claim_conflicts";
const DETECTION_JOBS_TABLE: &str = "knowledge_memory_conflict_detection_jobs";
const SPANS_TABLE: &str = "knowledge_spans";
const EDGES_TABLE: &str = "knowledge_edges";

/// Mint a `<PREFIX>-<32 hex>` id matching the `knowledge_memory_*` CHECKs.
/// Uuidv7 is time-ordered; `.simple()` is exactly 32 lowercase hex chars.
fn new_memory_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
}

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

/// The plain id behind a record link.
///
/// `RecordIdKey` has no `Display` that yields a bare id (its SurrealQL
/// rendering would quote the value), so the key is destructured instead.
fn record_key(record_id: RecordId, reason: &'static str) -> StorageResult<String> {
    let RecordIdKey::String(key) = record_id.key else {
        return Err(StorageError::Conflict(reason));
    };
    Ok(key)
}

fn optional_record_key(
    record_id: Option<RecordId>,
    reason: &'static str,
) -> StorageResult<Option<String>> {
    record_id.map(|id| record_key(id, reason)).transpose()
}

fn link(table: &'static str, id: &str) -> RecordId {
    RecordId::new(table, id)
}

fn optional_link(table: &'static str, id: Option<&str>) -> Option<RecordId> {
    id.map(|value| RecordId::new(table, value))
}

fn narrow_i32(value: i64, reason: &'static str) -> StorageResult<i32> {
    i32::try_from(value).map_err(|_| StorageError::Validation(reason))
}

// ===========================================================================
// MT-113 MemoryOntologySchema
// ===========================================================================

/// What class of ontology object a term names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOntologyTermKind {
    EntityClass,
    RelationClass,
    Attribute,
    ExtractionPattern,
}

impl MemoryOntologyTermKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryOntologyTermKind::EntityClass => "entity_class",
            MemoryOntologyTermKind::RelationClass => "relation_class",
            MemoryOntologyTermKind::Attribute => "attribute",
            MemoryOntologyTermKind::ExtractionPattern => "extraction_pattern",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "entity_class" => Ok(MemoryOntologyTermKind::EntityClass),
            "relation_class" => Ok(MemoryOntologyTermKind::RelationClass),
            "attribute" => Ok(MemoryOntologyTermKind::Attribute),
            "extraction_pattern" => Ok(MemoryOntologyTermKind::ExtractionPattern),
            _ => Err(StorageError::Validation(
                "invalid memory ontology term_kind",
            )),
        }
    }
}

/// Lifecycle of an ontology term: probationary terms are not yet stable
/// retrieval ontology. Mirrors the claim lifecycle discipline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOntologyLifecycle {
    Probationary,
    Stable,
    Retired,
}

impl MemoryOntologyLifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryOntologyLifecycle::Probationary => "probationary",
            MemoryOntologyLifecycle::Stable => "stable",
            MemoryOntologyLifecycle::Retired => "retired",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "probationary" => Ok(MemoryOntologyLifecycle::Probationary),
            "stable" => Ok(MemoryOntologyLifecycle::Stable),
            "retired" => Ok(MemoryOntologyLifecycle::Retired),
            _ => Err(StorageError::Validation(
                "invalid memory ontology lifecycle_state",
            )),
        }
    }

    /// Legal forward transitions (the same table the 0240 trigger enforces).
    pub fn can_transition_to(&self, to: MemoryOntologyLifecycle) -> bool {
        use MemoryOntologyLifecycle::*;
        matches!(
            (self, to),
            (Probationary, Stable) | (Probationary, Retired) | (Stable, Retired)
        )
    }
}

/// Why an ontology term was retired (reuses the claim retirement vocabulary).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOntologyRetirementReason {
    Rejected,
    Superseded,
    Stale,
    OperatorRetired,
}

impl MemoryOntologyRetirementReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryOntologyRetirementReason::Rejected => "rejected",
            MemoryOntologyRetirementReason::Superseded => "superseded",
            MemoryOntologyRetirementReason::Stale => "stale",
            MemoryOntologyRetirementReason::OperatorRetired => "operator_retired",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "rejected" => Ok(MemoryOntologyRetirementReason::Rejected),
            "superseded" => Ok(MemoryOntologyRetirementReason::Superseded),
            "stale" => Ok(MemoryOntologyRetirementReason::Stale),
            "operator_retired" => Ok(MemoryOntologyRetirementReason::OperatorRetired),
            _ => Err(StorageError::Validation(
                "invalid memory ontology retirement_reason",
            )),
        }
    }
}

/// A stable-schema-memory ontology term row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryOntologyTerm {
    pub term_id: String,
    pub workspace_id: String,
    pub term_kind: MemoryOntologyTermKind,
    pub term_key: String,
    pub normalized_label: String,
    pub maps_to_edge_type: Option<String>,
    pub maps_to_entity_kind: Option<String>,
    pub lifecycle_state: MemoryOntologyLifecycle,
    pub retirement_reason: Option<MemoryOntologyRetirementReason>,
    pub superseded_by_term_id: Option<String>,
    pub observation_count: i32,
    pub promotion_threshold: i32,
    pub operator_approved: bool,
    pub promotion_receipt_event_id: Option<String>,
    pub detection_provenance: Value,
    pub first_seen_in_run: Option<String>,
    pub last_seen_in_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryOntologyTerm {
    /// Whether this term has cleared its promotion gate: either an operator
    /// approved it, or its observation count met the frequency threshold
    /// (MT-120 promotion rule, evaluated read-side; promotion still requires a
    /// receipt via [`promote_memory_ontology_term`]).
    pub fn is_promotable(&self) -> bool {
        self.lifecycle_state == MemoryOntologyLifecycle::Probationary
            && (self.operator_approved || self.observation_count >= self.promotion_threshold)
    }
}

/// Upsert payload for a [`MemoryOntologyTerm`].
#[derive(Clone, Debug)]
pub struct NewMemoryOntologyTerm {
    pub workspace_id: String,
    pub term_kind: MemoryOntologyTermKind,
    pub term_key: String,
    pub normalized_label: String,
    pub maps_to_edge_type: Option<String>,
    pub maps_to_entity_kind: Option<String>,
    pub promotion_threshold: i32,
    pub operator_approved: bool,
    pub detection_provenance: Value,
    pub seen_in_run: Option<String>,
}

/// Stored `knowledge_memory_ontology_terms` projection.
#[derive(SurrealValue)]
struct OntologyTermRecord {
    term_id: String,
    workspace_id: RecordId,
    term_kind: String,
    term_key: String,
    normalized_label: String,
    maps_to_edge_type: Option<String>,
    maps_to_entity_kind: Option<String>,
    lifecycle_state: String,
    retirement_reason: Option<String>,
    superseded_by_term_id: Option<RecordId>,
    observation_count: i64,
    promotion_threshold: i64,
    operator_approved: bool,
    promotion_receipt_event_id: Option<RecordId>,
    detection_provenance: Value,
    first_seen_in_run: Option<RecordId>,
    last_seen_in_run: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

impl OntologyTermRecord {
    fn into_term(self) -> StorageResult<MemoryOntologyTerm> {
        Ok(MemoryOntologyTerm {
            term_id: self.term_id,
            workspace_id: record_key(
                self.workspace_id,
                "ontology term workspace link is not a string key",
            )?,
            term_kind: MemoryOntologyTermKind::from_db(self.term_kind.as_str())?,
            term_key: self.term_key,
            normalized_label: self.normalized_label,
            maps_to_edge_type: self.maps_to_edge_type,
            maps_to_entity_kind: self.maps_to_entity_kind,
            lifecycle_state: MemoryOntologyLifecycle::from_db(self.lifecycle_state.as_str())?,
            retirement_reason: self
                .retirement_reason
                .map(|value| MemoryOntologyRetirementReason::from_db(&value))
                .transpose()?,
            superseded_by_term_id: optional_record_key(
                self.superseded_by_term_id,
                "ontology term supersession link is not a string key",
            )?,
            observation_count: narrow_i32(
                self.observation_count,
                "ontology term observation_count is out of range",
            )?,
            promotion_threshold: narrow_i32(
                self.promotion_threshold,
                "ontology term promotion_threshold is out of range",
            )?,
            operator_approved: self.operator_approved,
            promotion_receipt_event_id: optional_record_key(
                self.promotion_receipt_event_id,
                "ontology term promotion receipt link is not a string key",
            )?,
            detection_provenance: self.detection_provenance,
            first_seen_in_run: optional_record_key(
                self.first_seen_in_run,
                "ontology term first_seen_in_run link is not a string key",
            )?,
            last_seen_in_run: optional_record_key(
                self.last_seen_in_run,
                "ontology term last_seen_in_run link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
            updated_at: self.updated_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct OntologyTermUpsertBindings {
    term_id: String,
    workspace: RecordId,
    term_kind: String,
    term_key: String,
    normalized_label: String,
    maps_to_edge_type: Option<String>,
    maps_to_entity_kind: Option<String>,
    promotion_threshold: i64,
    operator_approved: bool,
    detection_provenance: Value,
    seen_in_run: Option<RecordId>,
}

#[derive(SurrealValue)]
struct TermIdBindings {
    term_id: String,
}

#[derive(SurrealValue)]
struct TermListBindings {
    workspace: RecordId,
    term_kind: Option<String>,
    lifecycle_state: Option<String>,
    limit: i64,
}

#[derive(SurrealValue)]
struct TermPromoteBindings {
    term_id: String,
    receipt: RecordId,
}

#[derive(SurrealValue)]
struct TermRetireBindings {
    term_id: String,
    reason: String,
    superseded_by: Option<RecordId>,
}

/// Upsert a probationary ontology term on its stable identity
/// (workspace, kind, key). Re-derivation by a later run increments the
/// observation count and refreshes provenance/last_seen, WITHOUT moving the
/// lifecycle state (promotion is a separate receipt-backed step).
pub async fn upsert_memory_ontology_term(
    storage: &SurrealStorage,
    new: NewMemoryOntologyTerm,
) -> StorageResult<MemoryOntologyTerm> {
    if new.maps_to_edge_type.is_some() && new.maps_to_entity_kind.is_some() {
        return Err(StorageError::Validation(
            "ontology term cannot map to both an edge type and an entity kind",
        ));
    }
    let term_id = new_memory_id("KMO");
    let bindings = OntologyTermUpsertBindings {
        term_id,
        workspace: link(WORKSPACES_TABLE, &new.workspace_id),
        term_kind: new.term_kind.as_str().to_owned(),
        term_key: new.term_key.clone(),
        normalized_label: new.normalized_label.clone(),
        maps_to_edge_type: new.maps_to_edge_type.clone(),
        maps_to_entity_kind: new.maps_to_entity_kind.clone(),
        promotion_threshold: i64::from(new.promotion_threshold),
        operator_approved: new.operator_approved,
        detection_provenance: new.detection_provenance.clone(),
        seen_in_run: optional_link(INDEX_RUNS_TABLE, new.seen_in_run.as_deref()),
    };
    // ONE statement, so the existence test and the write cannot interleave:
    // two runs re-deriving the same (workspace, kind, key) still converge on a
    // single row with a single increment, which is what
    // `ON CONFLICT ... observation_count + 1` guaranteed. Promotion state is
    // still untouched here - only the counter, provenance and last_seen move.
    let rows: Vec<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "IF array::len((SELECT VALUE term_id FROM \
                         knowledge_memory_ontology_terms WHERE workspace_id = $workspace \
                         AND term_kind = $term_kind AND term_key = $term_key)) = 0 { \
                           CREATE type::record('knowledge_memory_ontology_terms', $term_id) \
                           CONTENT { term_id: $term_id, workspace_id: $workspace, \
                             term_kind: $term_kind, term_key: $term_key, \
                             normalized_label: $normalized_label, \
                             maps_to_edge_type: $maps_to_edge_type, \
                             maps_to_entity_kind: $maps_to_entity_kind, \
                             observation_count: 1, \
                             promotion_threshold: $promotion_threshold, \
                             operator_approved: $operator_approved, \
                             detection_provenance: $detection_provenance, \
                             first_seen_in_run: $seen_in_run, \
                             last_seen_in_run: $seen_in_run } \
                         } ELSE { \
                           UPDATE knowledge_memory_ontology_terms SET \
                             normalized_label = $normalized_label, \
                             observation_count = observation_count + 1, \
                             operator_approved = operator_approved OR $operator_approved, \
                             detection_provenance = $detection_provenance, \
                             last_seen_in_run = $seen_in_run, \
                             updated_at = time::now() \
                           WHERE workspace_id = $workspace AND term_kind = $term_kind \
                             AND term_key = $term_key RETURN AFTER \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "ontology term upsert produced no row".to_owned(),
        ))?
        .into_term()
}

/// Fetch one ontology term by id.
pub async fn get_memory_ontology_term(
    storage: &SurrealStorage,
    term_id: &str,
) -> StorageResult<Option<MemoryOntologyTerm>> {
    let bindings = TermIdBindings {
        term_id: term_id.to_owned(),
    };
    let record: Option<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_memory_ontology_terms WHERE term_id = $term_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(OntologyTermRecord::into_term).transpose()
}

/// List ontology terms for a workspace, optionally filtered to one kind and/or
/// lifecycle state, newest first, bounded by `limit`.
pub async fn list_memory_ontology_terms(
    storage: &SurrealStorage,
    workspace_id: &str,
    term_kind: Option<MemoryOntologyTermKind>,
    lifecycle_state: Option<MemoryOntologyLifecycle>,
    limit: i64,
) -> StorageResult<Vec<MemoryOntologyTerm>> {
    // `NONE` is the embedded store's "no filter" value, reproducing the
    // `$2::text IS NULL OR column = $2` predicate without string assembly.
    let bindings = TermListBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
        term_kind: term_kind.map(|kind| kind.as_str().to_owned()),
        lifecycle_state: lifecycle_state.map(|state| state.as_str().to_owned()),
        limit,
    };
    let records: Vec<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_memory_ontology_terms \
                         WHERE workspace_id = $workspace \
                         AND ($term_kind = NONE OR term_kind = $term_kind) \
                         AND ($lifecycle_state = NONE OR lifecycle_state = $lifecycle_state) \
                         ORDER BY created_at DESC, term_id DESC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(OntologyTermRecord::into_term)
        .collect()
}

/// MT-119/MT-120: promote a probationary term to stable, backed by an
/// EventLedger receipt. Fails closed (`Validation`) if the term has not cleared
/// its promotion gate, (`Conflict`) if it is not probationary, and the DB
/// trigger independently refuses a stable row without a receipt.
pub async fn promote_memory_ontology_term(
    storage: &SurrealStorage,
    term_id: &str,
    promotion_receipt_event_id: &str,
) -> StorageResult<MemoryOntologyTerm> {
    let current = get_memory_ontology_term(storage, term_id)
        .await?
        .ok_or(StorageError::NotFound("memory ontology term"))?;
    if current.lifecycle_state != MemoryOntologyLifecycle::Probationary {
        return Err(StorageError::Conflict(
            "only probationary ontology terms can be promoted",
        ));
    }
    if !current.is_promotable() {
        return Err(StorageError::Validation(
            "ontology term has not met its promotion threshold or operator approval",
        ));
    }
    // The `probationary` guard is carried into the WHERE clause so a concurrent
    // promotion cannot be applied twice; the pre-read above stays only to keep
    // the typed Conflict/Validation errors the callers already distinguish.
    let bindings = TermPromoteBindings {
        term_id: term_id.to_owned(),
        receipt: link(KERNEL_EVENT_LEDGER_TABLE, promotion_receipt_event_id),
    };
    let rows: Vec<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_memory_ontology_terms SET lifecycle_state = 'stable', \
                         promotion_receipt_event_id = $receipt, updated_at = time::now() \
                         WHERE term_id = $term_id AND lifecycle_state = 'probationary' \
                         RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Conflict(
            "only probationary ontology terms can be promoted",
        ))?
        .into_term()
}

/// Retire an ontology term with a reason (and optional supersessor).
pub async fn retire_memory_ontology_term(
    storage: &SurrealStorage,
    term_id: &str,
    reason: MemoryOntologyRetirementReason,
    superseded_by_term_id: Option<&str>,
) -> StorageResult<MemoryOntologyTerm> {
    if superseded_by_term_id.is_some() && reason != MemoryOntologyRetirementReason::Superseded {
        return Err(StorageError::Validation(
            "superseded_by_term_id requires the 'superseded' retirement reason",
        ));
    }
    let current = get_memory_ontology_term(storage, term_id)
        .await?
        .ok_or(StorageError::NotFound("memory ontology term"))?;
    if current.lifecycle_state == MemoryOntologyLifecycle::Retired {
        return Err(StorageError::Conflict("ontology term is already retired"));
    }
    let bindings = TermRetireBindings {
        term_id: term_id.to_owned(),
        reason: reason.as_str().to_owned(),
        superseded_by: optional_link(ONTOLOGY_TERMS_TABLE, superseded_by_term_id),
    };
    let rows: Vec<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_memory_ontology_terms SET lifecycle_state = 'retired', \
                         retirement_reason = $reason, superseded_by_term_id = $superseded_by, \
                         updated_at = time::now() \
                         WHERE term_id = $term_id AND lifecycle_state != 'retired' RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Conflict("ontology term is already retired"))?
        .into_term()
}

/// Source of an ontology alias.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOntologyAliasSource {
    Extraction,
    Operator,
    Spec,
    Import,
}

impl MemoryOntologyAliasSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryOntologyAliasSource::Extraction => "extraction",
            MemoryOntologyAliasSource::Operator => "operator",
            MemoryOntologyAliasSource::Spec => "spec",
            MemoryOntologyAliasSource::Import => "import",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "extraction" => Ok(MemoryOntologyAliasSource::Extraction),
            "operator" => Ok(MemoryOntologyAliasSource::Operator),
            "spec" => Ok(MemoryOntologyAliasSource::Spec),
            "import" => Ok(MemoryOntologyAliasSource::Import),
            _ => Err(StorageError::Validation("invalid ontology alias_source")),
        }
    }
}

/// An alias row mapping an alternate spelling onto a canonical term.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryOntologyAlias {
    pub alias_id: String,
    pub term_id: String,
    pub workspace_id: String,
    pub alias_surface: String,
    pub alias_norm_key: String,
    pub alias_source: MemoryOntologyAliasSource,
    pub created_at: DateTime<Utc>,
}

/// Stored `knowledge_memory_ontology_aliases` projection.
#[derive(SurrealValue)]
struct OntologyAliasRecord {
    alias_id: String,
    term_id: RecordId,
    workspace_id: RecordId,
    alias_surface: String,
    alias_norm_key: String,
    alias_source: String,
    created_at: Datetime,
}

impl OntologyAliasRecord {
    fn into_alias(self) -> StorageResult<MemoryOntologyAlias> {
        Ok(MemoryOntologyAlias {
            alias_id: self.alias_id,
            term_id: record_key(self.term_id, "ontology alias term link is not a string key")?,
            workspace_id: record_key(
                self.workspace_id,
                "ontology alias workspace link is not a string key",
            )?,
            alias_surface: self.alias_surface,
            alias_norm_key: self.alias_norm_key,
            alias_source: MemoryOntologyAliasSource::from_db(self.alias_source.as_str())?,
            created_at: self.created_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct AliasCreate {
    alias_id: String,
    term_id: RecordId,
    workspace_id: RecordId,
    alias_surface: String,
    alias_norm_key: String,
    alias_source: String,
}

#[derive(SurrealValue)]
struct AliasResolveBindings {
    workspace: RecordId,
    alias_norm_key: String,
}

#[derive(SurrealValue)]
struct AliasTermBindings {
    term: RecordId,
}

/// Add an alias for a term. The (workspace, alias_norm_key) uniqueness means a
/// normalized spelling resolves to exactly one canonical term.
///
/// `term_id` is a record link with `ASSERT record::exists`, so an alias for a
/// term that does not exist is refused by the store, not by a client check.
pub async fn add_memory_ontology_alias(
    storage: &SurrealStorage,
    term_id: &str,
    workspace_id: &str,
    alias_surface: &str,
    alias_norm_key: &str,
    alias_source: MemoryOntologyAliasSource,
) -> StorageResult<MemoryOntologyAlias> {
    let alias_id = new_memory_id("KMA");
    let content = AliasCreate {
        alias_id: alias_id.clone(),
        term_id: link(ONTOLOGY_TERMS_TABLE, term_id),
        workspace_id: link(WORKSPACES_TABLE, workspace_id),
        alias_surface: alias_surface.to_owned(),
        alias_norm_key: alias_norm_key.to_owned(),
        alias_source: alias_source.as_str().to_owned(),
    };
    // A plain CREATE, exactly like the relational INSERT: the
    // `(workspace_id, alias_norm_key)` UNIQUE index is the server-side guard
    // that a normalized spelling maps to one canonical term, and a violation
    // surfaces as a database error rather than being swallowed here.
    let rows: Vec<OntologyAliasRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_memory_ontology_aliases', $alias_id) \
                         CONTENT { alias_id: $alias_id, term_id: $term_id, \
                           workspace_id: $workspace_id, alias_surface: $alias_surface, \
                           alias_norm_key: $alias_norm_key, alias_source: $alias_source };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "memory ontology alias insert produced no row".to_owned(),
        ))?
        .into_alias()
}

/// Resolve an alias surface (by its normalized key) to its canonical term.
pub async fn resolve_memory_ontology_alias(
    storage: &SurrealStorage,
    workspace_id: &str,
    alias_norm_key: &str,
) -> StorageResult<Option<MemoryOntologyTerm>> {
    // The relational JOIN becomes a link traversal: `term_id` IS the term
    // record, so `term_id.*` returns the whole term row without a join key.
    let bindings = AliasResolveBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
        alias_norm_key: alias_norm_key.to_owned(),
    };
    let record: Option<OntologyTermRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT VALUE term_id.* FROM knowledge_memory_ontology_aliases \
                         WHERE workspace_id = $workspace AND alias_norm_key = $alias_norm_key;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(OntologyTermRecord::into_term).transpose()
}

/// List all aliases for a term.
pub async fn list_memory_ontology_aliases(
    storage: &SurrealStorage,
    term_id: &str,
) -> StorageResult<Vec<MemoryOntologyAlias>> {
    let bindings = AliasTermBindings {
        term: link(ONTOLOGY_TERMS_TABLE, term_id),
    };
    let records: Vec<OntologyAliasRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT alias_id, term_id, workspace_id, alias_surface, alias_norm_key, \
                         alias_source, created_at FROM knowledge_memory_ontology_aliases \
                         WHERE term_id = $term ORDER BY created_at ASC, alias_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(OntologyAliasRecord::into_alias)
        .collect()
}

// ===========================================================================
// MT-114 MemoryFactSchema  (+ MT-125 ClaimAuthorityLabels vocabulary)
// ===========================================================================

/// Fact-level authority label (MT-125): where a fact's authority comes from.
/// `source` (extracted verbatim from a source span), `derived` (computed from
/// other facts), `model_suggested` (LLM-proposed, not yet operator-approved),
/// `operator_approved` (an operator accepted it), `deprecated` / `superseded`
/// (no longer current), `unsupported` (no surviving evidence).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClaimAuthorityLabel {
    Source,
    Derived,
    ModelSuggested,
    OperatorApproved,
    Deprecated,
    Superseded,
    Unsupported,
}

impl MemoryClaimAuthorityLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryClaimAuthorityLabel::Source => "source",
            MemoryClaimAuthorityLabel::Derived => "derived",
            MemoryClaimAuthorityLabel::ModelSuggested => "model_suggested",
            MemoryClaimAuthorityLabel::OperatorApproved => "operator_approved",
            MemoryClaimAuthorityLabel::Deprecated => "deprecated",
            MemoryClaimAuthorityLabel::Superseded => "superseded",
            MemoryClaimAuthorityLabel::Unsupported => "unsupported",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "source" => Ok(MemoryClaimAuthorityLabel::Source),
            "derived" => Ok(MemoryClaimAuthorityLabel::Derived),
            "model_suggested" => Ok(MemoryClaimAuthorityLabel::ModelSuggested),
            "operator_approved" => Ok(MemoryClaimAuthorityLabel::OperatorApproved),
            "deprecated" => Ok(MemoryClaimAuthorityLabel::Deprecated),
            "superseded" => Ok(MemoryClaimAuthorityLabel::Superseded),
            "unsupported" => Ok(MemoryClaimAuthorityLabel::Unsupported),
            _ => Err(StorageError::Validation(
                "invalid memory fact authority_label",
            )),
        }
    }

    /// Whether a transition from this label to `to` is allowed (MT-125). An
    /// `operator_approved` label is sticky: it cannot silently drop back to a
    /// model-suggested or source label (only an operator action deprecates it).
    /// `unsupported` is reachable from any non-operator label (evidence loss),
    /// and `deprecated`/`superseded` are reachable from any live label.
    pub fn can_transition_to(&self, to: MemoryClaimAuthorityLabel) -> bool {
        use MemoryClaimAuthorityLabel::*;
        if *self == to {
            return true;
        }
        match self {
            // Operator approval is authoritative; only deprecation/supersession
            // moves it off, never a downgrade to a weaker source label.
            OperatorApproved => matches!(to, Deprecated | Superseded | Unsupported),
            // Terminal-ish end states: only supersede a deprecated fact, only
            // deprecate a superseded one (both already "not current").
            Deprecated => matches!(to, Superseded),
            Superseded => matches!(to, Deprecated),
            // Live extraction labels can be promoted, deprecated, superseded,
            // or marked unsupported.
            Source | Derived | ModelSuggested => matches!(
                to,
                OperatorApproved | Deprecated | Superseded | Unsupported | Source | Derived
            ),
            // Unsupported is not a probationary label. Once a fact has lost its
            // evidence basis, a later re-grounding must create a fresh
            // evidence-backed fact rather than mutating this row into stable
            // retrieval authority.
            Unsupported => matches!(to, Deprecated | Superseded),
        }
    }
}

/// A structured subject/predicate/object memory fact backed 1:1 by a
/// knowledge_claims row (the lifecycle + evidence + conflict authority).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryFact {
    pub fact_id: String,
    pub workspace_id: String,
    pub claim_id: String,
    pub subject_entity_id: String,
    pub predicate_key: String,
    pub predicate_term_id: Option<String>,
    pub object_entity_id: Option<String>,
    pub object_literal: Option<String>,
    pub qualifiers: Value,
    pub authority_label: MemoryClaimAuthorityLabel,
    pub extractor_version: String,
    pub created_in_run: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The object of a fact: another entity (relationship) XOR a literal
/// (attribute).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MemoryFactObject {
    Entity { entity_id: String },
    Literal { value: String },
}

/// Create-fact payload. The backing `claim_id` MUST already exist (created via
/// `KnowledgeStore::create_knowledge_claim`, which enforces the REQUIRED
/// evidence spans). The fact attaches structure to that authority row.
#[derive(Clone, Debug)]
pub struct NewMemoryFact {
    pub workspace_id: String,
    pub claim_id: String,
    pub subject_entity_id: String,
    pub predicate_key: String,
    pub predicate_term_id: Option<String>,
    pub object: MemoryFactObject,
    pub qualifiers: Value,
    pub authority_label: MemoryClaimAuthorityLabel,
    pub extractor_version: String,
    pub created_in_run: Option<String>,
}

/// Stored `knowledge_memory_facts` projection.
///
/// `claim_id`, `subject_entity_id`, `object_entity_id` and `predicate_term_id`
/// are record links, so the store itself refuses a fact whose backing claim,
/// subject or object no longer exists - the guarantee the relational FKs gave.
#[derive(SurrealValue)]
struct MemoryFactRecord {
    fact_id: String,
    workspace_id: RecordId,
    claim_id: RecordId,
    subject_entity_id: RecordId,
    predicate_key: String,
    predicate_term_id: Option<RecordId>,
    object_entity_id: Option<RecordId>,
    object_literal: Option<String>,
    qualifiers: Value,
    authority_label: String,
    extractor_version: String,
    created_in_run: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

impl MemoryFactRecord {
    fn into_fact(self) -> StorageResult<MemoryFact> {
        Ok(MemoryFact {
            fact_id: self.fact_id,
            workspace_id: record_key(
                self.workspace_id,
                "memory fact workspace link is not a string key",
            )?,
            claim_id: record_key(self.claim_id, "memory fact claim link is not a string key")?,
            subject_entity_id: record_key(
                self.subject_entity_id,
                "memory fact subject link is not a string key",
            )?,
            predicate_key: self.predicate_key,
            predicate_term_id: optional_record_key(
                self.predicate_term_id,
                "memory fact predicate term link is not a string key",
            )?,
            object_entity_id: optional_record_key(
                self.object_entity_id,
                "memory fact object link is not a string key",
            )?,
            object_literal: self.object_literal,
            qualifiers: self.qualifiers,
            authority_label: MemoryClaimAuthorityLabel::from_db(self.authority_label.as_str())?,
            extractor_version: self.extractor_version,
            created_in_run: optional_record_key(
                self.created_in_run,
                "memory fact created_in_run link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
            updated_at: self.updated_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct MemoryFactCreate {
    fact_id: String,
    workspace_id: RecordId,
    claim_id: RecordId,
    subject_entity_id: RecordId,
    predicate_key: String,
    predicate_term_id: Option<RecordId>,
    object_entity_id: Option<RecordId>,
    object_literal: Option<String>,
    qualifiers: Value,
    authority_label: String,
    extractor_version: String,
    created_in_run: Option<RecordId>,
}

#[derive(SurrealValue)]
struct FactIdBindings {
    fact_id: String,
}

#[derive(SurrealValue)]
struct FactClaimBindings {
    claim: RecordId,
}

#[derive(SurrealValue)]
struct FactListBindings {
    workspace: RecordId,
    limit: i64,
}

#[derive(SurrealValue)]
struct FactScopeBindings {
    workspace: RecordId,
    scope_terms: Vec<RecordId>,
    scope_entities: Vec<RecordId>,
    limit: i64,
}

/// Create a memory fact attached to an existing backing claim.
pub async fn create_memory_fact(
    storage: &SurrealStorage,
    new: NewMemoryFact,
) -> StorageResult<MemoryFact> {
    let fact_id = new_memory_id("KMF");
    let (object_entity_id, object_literal) = match &new.object {
        MemoryFactObject::Entity { entity_id } => (Some(entity_id.clone()), None),
        MemoryFactObject::Literal { value } => (None, Some(value.clone())),
    };
    let content = MemoryFactCreate {
        fact_id: fact_id.clone(),
        workspace_id: link(WORKSPACES_TABLE, &new.workspace_id),
        claim_id: link(CLAIMS_TABLE, &new.claim_id),
        subject_entity_id: link(ENTITIES_TABLE, &new.subject_entity_id),
        predicate_key: new.predicate_key.clone(),
        predicate_term_id: optional_link(ONTOLOGY_TERMS_TABLE, new.predicate_term_id.as_deref()),
        object_entity_id: optional_link(ENTITIES_TABLE, object_entity_id.as_deref()),
        object_literal,
        qualifiers: new.qualifiers.clone(),
        authority_label: new.authority_label.as_str().to_owned(),
        extractor_version: new.extractor_version.clone(),
        created_in_run: optional_link(INDEX_RUNS_TABLE, new.created_in_run.as_deref()),
    };
    let rows: Vec<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_memory_facts', $fact_id) CONTENT { \
                           fact_id: $fact_id, workspace_id: $workspace_id, claim_id: $claim_id, \
                           subject_entity_id: $subject_entity_id, \
                           predicate_key: $predicate_key, \
                           predicate_term_id: $predicate_term_id, \
                           object_entity_id: $object_entity_id, \
                           object_literal: $object_literal, qualifiers: $qualifiers, \
                           authority_label: $authority_label, \
                           extractor_version: $extractor_version, \
                           created_in_run: $created_in_run };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "memory fact insert produced no row".to_owned(),
        ))?
        .into_fact()
}

/// Fetch one fact by id.
pub async fn get_memory_fact(
    storage: &SurrealStorage,
    fact_id: &str,
) -> StorageResult<Option<MemoryFact>> {
    let bindings = FactIdBindings {
        fact_id: fact_id.to_owned(),
    };
    let record: Option<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_memory_facts WHERE fact_id = $fact_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(MemoryFactRecord::into_fact).transpose()
}

/// Fetch the fact backed by a given claim, if any.
pub async fn get_memory_fact_by_claim(
    storage: &SurrealStorage,
    claim_id: &str,
) -> StorageResult<Option<MemoryFact>> {
    let bindings = FactClaimBindings {
        claim: link(CLAIMS_TABLE, claim_id),
    };
    let record: Option<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT * FROM knowledge_memory_facts WHERE claim_id = $claim;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    record.map(MemoryFactRecord::into_fact).transpose()
}

/// List facts for a workspace, newest first, bounded by `limit`.
pub async fn list_memory_facts(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<MemoryFact>> {
    let bindings = FactListBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
        limit,
    };
    let records: Vec<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_memory_facts WHERE workspace_id = $workspace \
                         ORDER BY created_at DESC, fact_id DESC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(MemoryFactRecord::into_fact)
        .collect()
}

/// List ONLY the facts whose schema matches an in-scope id set
/// (adversarial-v2 MT-131): the scope predicate is pushed into the SQL
/// (`predicate_term_id` / `object_entity_id` = ANY(scope)) so the row cap
/// applies to IN-SCOPE facts. The previous capped unordered load filtered in
/// memory had a recall gap: in-scope facts beyond the cap were invisible.
pub async fn list_memory_facts_in_schema_scope(
    storage: &SurrealStorage,
    workspace_id: &str,
    in_scope_ids: &[String],
    limit: i64,
) -> StorageResult<Vec<MemoryFact>> {
    // The scope stays IN the query so the row cap applies to in-scope facts, as
    // the `= ANY($2)` predicate did. The two columns are links into different
    // tables, so the one id list has to be projected onto both link tables; a
    // caller-supplied id that names neither simply matches nothing.
    let bindings = FactScopeBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
        scope_terms: in_scope_ids
            .iter()
            .map(|id| link(ONTOLOGY_TERMS_TABLE, id))
            .collect(),
        scope_entities: in_scope_ids
            .iter()
            .map(|id| link(ENTITIES_TABLE, id))
            .collect(),
        limit,
    };
    let records: Vec<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM knowledge_memory_facts WHERE workspace_id = $workspace \
                         AND (predicate_term_id IN $scope_terms \
                              OR object_entity_id IN $scope_entities) \
                         ORDER BY created_at DESC, fact_id DESC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(MemoryFactRecord::into_fact)
        .collect()
}

/// MT-125: re-label a fact's authority, enforcing the legal label-transition
/// table. An illegal transition is a typed `Conflict` (e.g. silently demoting
/// an operator-approved fact to model_suggested).
pub async fn set_memory_fact_authority_label(
    storage: &SurrealStorage,
    fact_id: &str,
    to: MemoryClaimAuthorityLabel,
) -> StorageResult<MemoryFact> {
    let current = get_memory_fact(storage, fact_id)
        .await?
        .ok_or(StorageError::NotFound("memory fact"))?;
    if !current.authority_label.can_transition_to(to) {
        return Err(StorageError::Conflict(
            "illegal memory fact authority label transition",
        ));
    }
    // The observed label is carried into the WHERE clause so a concurrent
    // re-label that changed it under us fails closed instead of overwriting a
    // transition this call never validated.
    let bindings = FactRelabelBindings {
        fact_id: fact_id.to_owned(),
        authority_label: to.as_str().to_owned(),
        expected_label: current.authority_label.as_str().to_owned(),
    };
    let rows: Vec<MemoryFactRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "UPDATE knowledge_memory_facts SET authority_label = $authority_label, \
                         updated_at = time::now() WHERE fact_id = $fact_id \
                         AND authority_label = $expected_label RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Conflict(
            "memory fact authority label changed concurrently",
        ))?
        .into_fact()
}

#[derive(SurrealValue)]
struct FactRelabelBindings {
    fact_id: String,
    authority_label: String,
    expected_label: String,
}

// ===========================================================================
// MT-121 ConflictCandidateSearch
// ===========================================================================

/// A pair of facts that assert the SAME (subject, predicate) but with a
/// DIFFERENT object — i.e. a symbolic conflict candidate. The pair is ordered
/// deterministically by fact_id so the same two facts always produce the same
/// candidate (idempotent search; no duplicate (a,b)/(b,a)).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactConflictCandidate {
    pub subject_entity_id: String,
    pub predicate_key: String,
    pub fact_id_a: String,
    pub claim_id_a: String,
    pub object_a: String,
    pub fact_id_b: String,
    pub claim_id_b: String,
    pub object_b: String,
    /// Why these are candidates (the symbolic key class).
    pub candidate_reason: String,
}

/// MT-121: find symbolic conflict candidates in a workspace — facts that share
/// a (subject_entity_id, predicate_key) symbolic key but disagree on the object
/// (entity object id or literal). This is the deterministic candidate search
/// the ConflictDetectionJob (MT-122) runs; semantic/embedding candidates are a
/// future extension noted in the contract ("embedding/vector-like evidence
/// where available"). The self-join is ordered (a.fact_id < b.fact_id) so each
/// unordered pair appears once.
pub async fn find_fact_conflict_candidates(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FactConflictCandidate>> {
    // The embedded store has no self-join, so the ordered nested loop the
    // relational self-join performed is done here instead. The store still
    // supplies the ORDER, which is what makes the pairing deterministic: rows
    // arrive sorted by (subject, predicate, fact_id), so walking i < j inside
    // each (subject, predicate) run emits exactly the same unordered pairs, in
    // exactly the same sequence, and the same `limit` cut. Only the row cap
    // moves client-side; the candidate set and its order are unchanged.
    let bindings = ConflictCandidateBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
    };
    let rows: Vec<ConflictCandidateRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT subject_entity_id, predicate_key, fact_id, claim_id, \
                         object_entity_id, object_literal FROM knowledge_memory_facts \
                         WHERE workspace_id = $workspace \
                         ORDER BY subject_entity_id ASC, predicate_key ASC, fact_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in rows {
        facts.push(row.into_candidate_fact()?);
    }

    let limit = limit.max(0) as usize;
    let mut candidates = Vec::new();
    let mut group_start = 0usize;
    while group_start < facts.len() {
        let mut group_end = group_start + 1;
        while group_end < facts.len()
            && facts[group_end].subject_entity_id == facts[group_start].subject_entity_id
            && facts[group_end].predicate_key == facts[group_start].predicate_key
        {
            group_end += 1;
        }
        for left in group_start..group_end {
            for right in (left + 1)..group_end {
                if facts[left].object == facts[right].object {
                    continue;
                }
                if candidates.len() >= limit {
                    return Ok(candidates);
                }
                candidates.push(FactConflictCandidate {
                    subject_entity_id: facts[left].subject_entity_id.clone(),
                    predicate_key: facts[left].predicate_key.clone(),
                    fact_id_a: facts[left].fact_id.clone(),
                    claim_id_a: facts[left].claim_id.clone(),
                    object_a: facts[left].object.clone(),
                    fact_id_b: facts[right].fact_id.clone(),
                    claim_id_b: facts[right].claim_id.clone(),
                    object_b: facts[right].object.clone(),
                    candidate_reason: "symbolic_subject_predicate_object_mismatch".to_string(),
                });
            }
        }
        group_start = group_end;
    }
    Ok(candidates)
}

#[derive(SurrealValue)]
struct ConflictCandidateBindings {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct ConflictCandidateRow {
    subject_entity_id: RecordId,
    predicate_key: String,
    fact_id: String,
    claim_id: RecordId,
    object_entity_id: Option<RecordId>,
    object_literal: Option<String>,
}

/// One side of a candidate pair, with the object already collapsed the way
/// `COALESCE(object_entity_id, object_literal)` collapsed it.
struct CandidateFact {
    subject_entity_id: String,
    predicate_key: String,
    fact_id: String,
    claim_id: String,
    object: String,
}

impl ConflictCandidateRow {
    fn into_candidate_fact(self) -> StorageResult<CandidateFact> {
        let object = match (
            optional_record_key(
                self.object_entity_id,
                "memory fact object link is not a string key",
            )?,
            self.object_literal,
        ) {
            (Some(entity_id), _) => entity_id,
            (None, Some(literal)) => literal,
            (None, None) => {
                return Err(StorageError::Conflict(
                    "memory fact has neither an object entity nor an object literal",
                ))
            }
        };
        Ok(CandidateFact {
            subject_entity_id: record_key(
                self.subject_entity_id,
                "memory fact subject link is not a string key",
            )?,
            predicate_key: self.predicate_key,
            fact_id: self.fact_id,
            claim_id: record_key(self.claim_id, "memory fact claim link is not a string key")?,
            object,
        })
    }
}

// ===========================================================================
// MT-122 ConflictDetectionAgentJob  (typed job record + findings)
// ===========================================================================

/// The conflict class a detection pass searched for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDetectionKind {
    Symbolic,
    Temporal,
    Alias,
    StaleSource,
    Granularity,
    Semantic,
}

impl ConflictDetectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictDetectionKind::Symbolic => "symbolic",
            ConflictDetectionKind::Temporal => "temporal",
            ConflictDetectionKind::Alias => "alias",
            ConflictDetectionKind::StaleSource => "stale_source",
            ConflictDetectionKind::Granularity => "granularity",
            ConflictDetectionKind::Semantic => "semantic",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "symbolic" => Ok(ConflictDetectionKind::Symbolic),
            "temporal" => Ok(ConflictDetectionKind::Temporal),
            "alias" => Ok(ConflictDetectionKind::Alias),
            "stale_source" => Ok(ConflictDetectionKind::StaleSource),
            "granularity" => Ok(ConflictDetectionKind::Granularity),
            "semantic" => Ok(ConflictDetectionKind::Semantic),
            _ => Err(StorageError::Validation("invalid conflict detection_kind")),
        }
    }
}

/// A typed conflict-detection job record (NOT a spawned LLM agent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConflictDetectionJob {
    pub job_id: String,
    pub workspace_id: String,
    pub detection_kind: ConflictDetectionKind,
    pub job_state: String,
    pub candidates_scanned: i32,
    pub conflicts_found: i32,
    pub search_parameters: Value,
    pub detection_receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Stored `knowledge_memory_conflict_detection_jobs` projection.
#[derive(SurrealValue)]
struct DetectionJobRecord {
    job_id: String,
    workspace_id: RecordId,
    detection_kind: String,
    job_state: String,
    candidates_scanned: i64,
    conflicts_found: i64,
    search_parameters: Value,
    detection_receipt_event_id: Option<RecordId>,
    created_at: Datetime,
    completed_at: Option<Datetime>,
}

impl DetectionJobRecord {
    fn into_job(self) -> StorageResult<ConflictDetectionJob> {
        Ok(ConflictDetectionJob {
            job_id: self.job_id,
            workspace_id: record_key(
                self.workspace_id,
                "conflict detection job workspace link is not a string key",
            )?,
            detection_kind: ConflictDetectionKind::from_db(self.detection_kind.as_str())?,
            job_state: self.job_state,
            candidates_scanned: narrow_i32(
                self.candidates_scanned,
                "conflict detection candidates_scanned is out of range",
            )?,
            conflicts_found: narrow_i32(
                self.conflicts_found,
                "conflict detection conflicts_found is out of range",
            )?,
            search_parameters: self.search_parameters,
            detection_receipt_event_id: optional_record_key(
                self.detection_receipt_event_id,
                "conflict detection receipt link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
            completed_at: self.completed_at.map(Datetime::into_inner),
        })
    }
}

#[derive(SurrealValue)]
struct DetectionJobBindings {
    job_id: String,
    workspace: RecordId,
    detection_kind: String,
    candidates_scanned: i64,
    conflicts_found: i64,
    search_parameters: Value,
    receipt: Option<RecordId>,
    conflicts: Vec<RecordId>,
}

#[derive(SurrealValue)]
struct DetectionFindingsBindings {
    job: RecordId,
}

#[derive(SurrealValue)]
struct DetectionFindingRow {
    conflict_id: RecordId,
}

/// Record a completed conflict-detection job and link the conflict ids it
/// found, in one transaction. `conflict_ids` are existing
/// knowledge_claim_conflicts rows (produced by the detection pass).
///
/// Job row and finding rows still land atomically: both writes live inside ONE
/// top-level block statement, which the embedded store executes in a single
/// implicit transaction, so a job can never be observed without the findings it
/// claims to have produced. The findings loop became a `FOR` over the bound id
/// list rather than N round trips. A block (not `BEGIN ... COMMIT`) is used
/// deliberately: the seam reads result set 0, and an explicit `BEGIN` occupies
/// that slot with its own empty result.
pub async fn record_conflict_detection_job(
    storage: &SurrealStorage,
    workspace_id: &str,
    detection_kind: ConflictDetectionKind,
    candidates_scanned: i32,
    search_parameters: Value,
    conflict_ids: &[String],
    detection_receipt_event_id: Option<&str>,
) -> StorageResult<ConflictDetectionJob> {
    let job_id = new_memory_id("KCDJ");
    let bindings = DetectionJobBindings {
        job_id,
        workspace: link(WORKSPACES_TABLE, workspace_id),
        detection_kind: detection_kind.as_str().to_owned(),
        candidates_scanned: i64::from(candidates_scanned),
        conflicts_found: conflict_ids.len() as i64,
        search_parameters,
        receipt: optional_link(KERNEL_EVENT_LEDGER_TABLE, detection_receipt_event_id),
        conflicts: conflict_ids
            .iter()
            .map(|id| link(CLAIM_CONFLICTS_TABLE, id))
            .collect(),
    };
    let records: Vec<DetectionJobRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "{ \
                           LET $job = (CREATE \
                             type::record('knowledge_memory_conflict_detection_jobs', $job_id) \
                             CONTENT { job_id: $job_id, workspace_id: $workspace, \
                               detection_kind: $detection_kind, job_state: 'completed', \
                               candidates_scanned: $candidates_scanned, \
                               conflicts_found: $conflicts_found, \
                               search_parameters: $search_parameters, \
                               detection_receipt_event_id: $receipt, \
                               completed_at: time::now() }); \
                           FOR $conflict IN $conflicts { \
                             CREATE knowledge_memory_conflict_detection_findings CONTENT { \
                               job_id: type::record('knowledge_memory_conflict_detection_jobs', \
                                                   $job_id), \
                               conflict_id: $conflict }; \
                           }; \
                           RETURN $job; \
                         };",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .next()
        .ok_or(StorageError::Database(
            "conflict detection job insert produced no row".to_owned(),
        ))?
        .into_job()
}

/// List the conflict ids a detection job found.
pub async fn list_conflict_detection_findings(
    storage: &SurrealStorage,
    job_id: &str,
) -> StorageResult<Vec<String>> {
    let bindings = DetectionFindingsBindings {
        job: link(DETECTION_JOBS_TABLE, job_id),
    };
    let rows: Vec<DetectionFindingRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT conflict_id FROM knowledge_memory_conflict_detection_findings \
                         WHERE job_id = $job ORDER BY conflict_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .map(|row| {
            record_key(
                row.conflict_id,
                "conflict detection finding link is not a string key",
            )
        })
        .collect()
}

// ===========================================================================
// MT-123 ConflictResolutionAgentJob  (typed job record)
// ===========================================================================

/// A conflict resolution outcome (translated-spec ConflictResolutionJob).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionOutcome {
    Discard,
    Refine,
    TemporalQualify,
    GranularityQualify,
    Merge,
}

impl ConflictResolutionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictResolutionOutcome::Discard => "discard",
            ConflictResolutionOutcome::Refine => "refine",
            ConflictResolutionOutcome::TemporalQualify => "temporal_qualify",
            ConflictResolutionOutcome::GranularityQualify => "granularity_qualify",
            ConflictResolutionOutcome::Merge => "merge",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "discard" => Ok(ConflictResolutionOutcome::Discard),
            "refine" => Ok(ConflictResolutionOutcome::Refine),
            "temporal_qualify" => Ok(ConflictResolutionOutcome::TemporalQualify),
            "granularity_qualify" => Ok(ConflictResolutionOutcome::GranularityQualify),
            "merge" => Ok(ConflictResolutionOutcome::Merge),
            _ => Err(StorageError::Validation(
                "invalid conflict resolution outcome",
            )),
        }
    }

    /// Whether this outcome requires both a kept and a discarded claim.
    fn requires_discarded(&self) -> bool {
        matches!(
            self,
            ConflictResolutionOutcome::Discard | ConflictResolutionOutcome::Merge
        )
    }
}

/// A typed conflict-resolution job record. The resolution is receipt-backed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConflictResolutionJob {
    pub job_id: String,
    pub workspace_id: String,
    pub conflict_id: String,
    pub outcome: ConflictResolutionOutcome,
    pub kept_claim_id: Option<String>,
    pub discarded_claim_id: Option<String>,
    pub resolution_detail: Value,
    pub resolution_receipt_event_id: String,
    pub created_at: DateTime<Utc>,
}

/// Stored `knowledge_memory_conflict_resolution_jobs` projection.
#[derive(SurrealValue)]
struct ResolutionJobRecord {
    job_id: String,
    workspace_id: RecordId,
    conflict_id: RecordId,
    outcome: String,
    kept_claim_id: Option<RecordId>,
    discarded_claim_id: Option<RecordId>,
    resolution_detail: Value,
    resolution_receipt_event_id: RecordId,
    created_at: Datetime,
}

impl ResolutionJobRecord {
    fn into_job(self) -> StorageResult<ConflictResolutionJob> {
        Ok(ConflictResolutionJob {
            job_id: self.job_id,
            workspace_id: record_key(
                self.workspace_id,
                "conflict resolution workspace link is not a string key",
            )?,
            conflict_id: record_key(
                self.conflict_id,
                "conflict resolution conflict link is not a string key",
            )?,
            outcome: ConflictResolutionOutcome::from_db(self.outcome.as_str())?,
            kept_claim_id: optional_record_key(
                self.kept_claim_id,
                "conflict resolution kept-claim link is not a string key",
            )?,
            discarded_claim_id: optional_record_key(
                self.discarded_claim_id,
                "conflict resolution discarded-claim link is not a string key",
            )?,
            resolution_detail: self.resolution_detail,
            resolution_receipt_event_id: record_key(
                self.resolution_receipt_event_id,
                "conflict resolution receipt link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct ResolutionJobCreate {
    job_id: String,
    workspace_id: RecordId,
    conflict_id: RecordId,
    outcome: String,
    kept_claim_id: Option<RecordId>,
    discarded_claim_id: Option<RecordId>,
    resolution_detail: Value,
    resolution_receipt_event_id: RecordId,
}

#[derive(SurrealValue)]
struct ConflictIdBindings {
    conflict: RecordId,
}

/// Record a conflict-resolution job. Validates the kept/discarded claim shape
/// against the chosen outcome before insert (the DB CHECK is the backstop).
#[allow(clippy::too_many_arguments)]
pub async fn record_conflict_resolution_job(
    storage: &SurrealStorage,
    workspace_id: &str,
    conflict_id: &str,
    outcome: ConflictResolutionOutcome,
    kept_claim_id: Option<&str>,
    discarded_claim_id: Option<&str>,
    resolution_detail: Value,
    resolution_receipt_event_id: &str,
) -> StorageResult<ConflictResolutionJob> {
    if kept_claim_id.is_none() {
        return Err(StorageError::Validation(
            "conflict resolution requires a kept claim",
        ));
    }
    if outcome.requires_discarded() && discarded_claim_id.is_none() {
        return Err(StorageError::Validation(
            "discard/merge resolution requires a discarded claim",
        ));
    }
    let job_id = new_memory_id("KCRJ");
    let content = ResolutionJobCreate {
        job_id: job_id.clone(),
        workspace_id: link(WORKSPACES_TABLE, workspace_id),
        conflict_id: link(CLAIM_CONFLICTS_TABLE, conflict_id),
        outcome: outcome.as_str().to_owned(),
        kept_claim_id: optional_link(CLAIMS_TABLE, kept_claim_id),
        discarded_claim_id: optional_link(CLAIMS_TABLE, discarded_claim_id),
        resolution_detail,
        resolution_receipt_event_id: link(KERNEL_EVENT_LEDGER_TABLE, resolution_receipt_event_id),
    };
    let rows: Vec<ResolutionJobRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_memory_conflict_resolution_jobs', $job_id) \
                         CONTENT { job_id: $job_id, workspace_id: $workspace_id, \
                           conflict_id: $conflict_id, outcome: $outcome, \
                           kept_claim_id: $kept_claim_id, \
                           discarded_claim_id: $discarded_claim_id, \
                           resolution_detail: $resolution_detail, \
                           resolution_receipt_event_id: $resolution_receipt_event_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "conflict resolution job insert produced no row".to_owned(),
        ))?
        .into_job()
}

/// List resolution jobs for a conflict (newest first).
pub async fn list_conflict_resolution_jobs(
    storage: &SurrealStorage,
    conflict_id: &str,
) -> StorageResult<Vec<ConflictResolutionJob>> {
    let bindings = ConflictIdBindings {
        conflict: link(CLAIM_CONFLICTS_TABLE, conflict_id),
    };
    let records: Vec<ResolutionJobRecord> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT job_id, workspace_id, conflict_id, outcome, kept_claim_id, \
                         discarded_claim_id, resolution_detail, resolution_receipt_event_id, \
                         created_at FROM knowledge_memory_conflict_resolution_jobs \
                         WHERE conflict_id = $conflict ORDER BY created_at DESC, job_id DESC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    records
        .into_iter()
        .map(ResolutionJobRecord::into_job)
        .collect()
}

// ===========================================================================
// MT-124 BridgeEdgeGenerator  (storage helpers + decision log)
// ===========================================================================

/// An (entity_a, entity_b, shared_span) co-occurrence: two DISTINCT entities
/// detected from the SAME span. Ordered (entity_id_a < entity_id_b) so each
/// unordered pair appears once per shared span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityCooccurrence {
    pub entity_id_a: String,
    pub entity_id_b: String,
    pub shared_span_id: String,
}

/// Find entity pairs that co-occur in evidence (share a detection span). These
/// are the raw bridge candidates: co-occurrence is the evidence a bridge needs
/// (the translated-spec rule "only when evidence supports the bridge"). The
/// ordered self-join over `knowledge_entity_spans` yields one row per
/// (entity_a < entity_b, span).
pub async fn find_entity_cooccurrences(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<EntityCooccurrence>> {
    // Same self-join situation as `find_fact_conflict_candidates`: the store
    // has no self-join, so the ordered nested loop is done here. The workspace
    // filter is a link traversal (`entity_id.workspace_id`) rather than a join
    // onto `knowledge_entities`, and the store still supplies the ordering that
    // makes the pairing deterministic, so the emitted set, its order and the
    // `limit` cut are unchanged. Only the row cap moves client-side.
    let bindings = CooccurrenceBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
    };
    let rows: Vec<EntitySpanRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT entity_id, span_id FROM knowledge_entity_spans \
                         WHERE entity_id.workspace_id = $workspace \
                         ORDER BY span_id ASC, entity_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;

    let mut spans: Vec<(String, String)> = Vec::with_capacity(rows.len());
    for row in rows {
        spans.push((
            record_key(row.span_id, "entity span link is not a string key")?,
            record_key(row.entity_id, "entity span entity link is not a string key")?,
        ));
    }

    let mut pairs = Vec::new();
    let mut group_start = 0usize;
    while group_start < spans.len() {
        let mut group_end = group_start + 1;
        while group_end < spans.len() && spans[group_end].0 == spans[group_start].0 {
            group_end += 1;
        }
        for left in group_start..group_end {
            for right in (left + 1)..group_end {
                let (span_id, entity_a) = &spans[left];
                let (_, entity_b) = &spans[right];
                if entity_a >= entity_b {
                    continue;
                }
                pairs.push(EntityCooccurrence {
                    entity_id_a: entity_a.clone(),
                    entity_id_b: entity_b.clone(),
                    shared_span_id: span_id.clone(),
                });
            }
        }
        group_start = group_end;
    }
    // The relational statement ordered by (entity_a, entity_b, span) before
    // applying LIMIT, so the ordering has to be restored before the cut.
    pairs.sort_by(|left, right| {
        left.entity_id_a
            .cmp(&right.entity_id_a)
            .then_with(|| left.entity_id_b.cmp(&right.entity_id_b))
            .then_with(|| left.shared_span_id.cmp(&right.shared_span_id))
    });
    pairs.truncate(limit.max(0) as usize);
    Ok(pairs)
}

#[derive(SurrealValue)]
struct CooccurrenceBindings {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct EntitySpanRow {
    entity_id: RecordId,
    span_id: RecordId,
}

#[derive(SurrealValue)]
struct EdgeEndpointRow {
    source_entity_id: RecordId,
    target_entity_id: RecordId,
}

#[derive(SurrealValue)]
struct EdgeDegreeBindings {
    entity: RecordId,
}

/// A directed-or-undirected edge endpoint pair from `knowledge_edges`, used to
/// build connected components (only non-retired edges count toward
/// connectivity). Returned as (source, target) pairs.
pub async fn list_active_edge_endpoints(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<(String, String)>> {
    let bindings = CooccurrenceBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
    };
    let rows: Vec<EdgeEndpointRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT source_entity_id, target_entity_id FROM knowledge_edges \
                         WHERE workspace_id = $workspace AND lifecycle_state != 'retired';",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .map(|row| {
            Ok((
                record_key(row.source_entity_id, "edge source link is not a string key")?,
                record_key(row.target_entity_id, "edge target link is not a string key")?,
            ))
        })
        .collect()
}

/// Undirected degree of an entity in the non-retired edge graph (number of
/// edges touching it as source or target). The hub-suppression input.
pub async fn entity_edge_degree(storage: &SurrealStorage, entity_id: &str) -> StorageResult<i64> {
    let bindings = EdgeDegreeBindings {
        entity: link(ENTITIES_TABLE, entity_id),
    };
    // `count()` is aggregated by the store, so the degree is still computed
    // server-side rather than by loading every incident edge.
    let degree: Option<i64> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT VALUE count() FROM knowledge_edges \
                         WHERE lifecycle_state != 'retired' \
                         AND (source_entity_id = $entity OR target_entity_id = $entity) \
                         GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(degree.unwrap_or(0))
}

/// The outcome of a bridge evaluation for one candidate pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeDecision {
    Bridged,
    SuppressedHub,
    SuppressedNoEvidence,
    SuppressedConnected,
}

impl BridgeDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeDecision::Bridged => "bridged",
            BridgeDecision::SuppressedHub => "suppressed_hub",
            BridgeDecision::SuppressedNoEvidence => "suppressed_no_evidence",
            BridgeDecision::SuppressedConnected => "suppressed_connected",
        }
    }

    fn from_db(value: &str) -> StorageResult<Self> {
        match value {
            "bridged" => Ok(BridgeDecision::Bridged),
            "suppressed_hub" => Ok(BridgeDecision::SuppressedHub),
            "suppressed_no_evidence" => Ok(BridgeDecision::SuppressedNoEvidence),
            "suppressed_connected" => Ok(BridgeDecision::SuppressedConnected),
            _ => Err(StorageError::Validation("invalid bridge decision")),
        }
    }
}

/// A recorded bridge-evaluation decision (auditable "why did/didn't a bridge
/// appear").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeDecisionRecord {
    pub decision_id: String,
    pub workspace_id: String,
    pub entity_id_a: String,
    pub entity_id_b: String,
    pub decision: BridgeDecision,
    pub degree_a: i32,
    pub degree_b: i32,
    pub hub_degree_threshold: i32,
    pub evidence_span_id: Option<String>,
    pub bridge_edge_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Stored `knowledge_memory_bridge_decisions` projection.
#[derive(SurrealValue)]
struct BridgeDecisionRow {
    decision_id: String,
    workspace_id: RecordId,
    entity_id_a: RecordId,
    entity_id_b: RecordId,
    decision: String,
    degree_a: i64,
    degree_b: i64,
    hub_degree_threshold: i64,
    evidence_span_id: Option<RecordId>,
    bridge_edge_id: Option<RecordId>,
    created_at: Datetime,
}

impl BridgeDecisionRow {
    fn into_record(self) -> StorageResult<BridgeDecisionRecord> {
        Ok(BridgeDecisionRecord {
            decision_id: self.decision_id,
            workspace_id: record_key(
                self.workspace_id,
                "bridge decision workspace link is not a string key",
            )?,
            entity_id_a: record_key(
                self.entity_id_a,
                "bridge decision entity_a link is not a string key",
            )?,
            entity_id_b: record_key(
                self.entity_id_b,
                "bridge decision entity_b link is not a string key",
            )?,
            decision: BridgeDecision::from_db(self.decision.as_str())?,
            degree_a: narrow_i32(self.degree_a, "bridge decision degree_a is out of range")?,
            degree_b: narrow_i32(self.degree_b, "bridge decision degree_b is out of range")?,
            hub_degree_threshold: narrow_i32(
                self.hub_degree_threshold,
                "bridge decision hub_degree_threshold is out of range",
            )?,
            evidence_span_id: optional_record_key(
                self.evidence_span_id,
                "bridge decision evidence span link is not a string key",
            )?,
            bridge_edge_id: optional_record_key(
                self.bridge_edge_id,
                "bridge decision edge link is not a string key",
            )?,
            created_at: self.created_at.into_inner(),
        })
    }
}

#[derive(SurrealValue)]
struct BridgeDecisionCreate {
    decision_id: String,
    workspace_id: RecordId,
    entity_id_a: RecordId,
    entity_id_b: RecordId,
    decision: String,
    degree_a: i64,
    degree_b: i64,
    hub_degree_threshold: i64,
    evidence_span_id: Option<RecordId>,
    bridge_edge_id: Option<RecordId>,
}

/// Record one bridge-evaluation decision.
#[allow(clippy::too_many_arguments)]
pub async fn record_bridge_decision(
    storage: &SurrealStorage,
    workspace_id: &str,
    entity_id_a: &str,
    entity_id_b: &str,
    decision: BridgeDecision,
    degree_a: i32,
    degree_b: i32,
    hub_degree_threshold: i32,
    evidence_span_id: Option<&str>,
    bridge_edge_id: Option<&str>,
) -> StorageResult<BridgeDecisionRecord> {
    let decision_id = new_memory_id("KBR");
    let content = BridgeDecisionCreate {
        decision_id: decision_id.clone(),
        workspace_id: link(WORKSPACES_TABLE, workspace_id),
        entity_id_a: link(ENTITIES_TABLE, entity_id_a),
        entity_id_b: link(ENTITIES_TABLE, entity_id_b),
        decision: decision.as_str().to_owned(),
        degree_a: i64::from(degree_a),
        degree_b: i64::from(degree_b),
        hub_degree_threshold: i64::from(hub_degree_threshold),
        evidence_span_id: optional_link(SPANS_TABLE, evidence_span_id),
        bridge_edge_id: optional_link(EDGES_TABLE, bridge_edge_id),
    };
    let rows: Vec<BridgeDecisionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "CREATE type::record('knowledge_memory_bridge_decisions', $decision_id) \
                         CONTENT { decision_id: $decision_id, workspace_id: $workspace_id, \
                           entity_id_a: $entity_id_a, entity_id_b: $entity_id_b, \
                           decision: $decision, degree_a: $degree_a, degree_b: $degree_b, \
                           hub_degree_threshold: $hub_degree_threshold, \
                           evidence_span_id: $evidence_span_id, \
                           bridge_edge_id: $bridge_edge_id };",
                        content,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .ok_or(StorageError::Database(
            "bridge decision insert produced no row".to_owned(),
        ))?
        .into_record()
}

/// List bridge decisions for a workspace, newest first.
pub async fn list_bridge_decisions(
    storage: &SurrealStorage,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<BridgeDecisionRecord>> {
    let bindings = FactListBindings {
        workspace: link(WORKSPACES_TABLE, workspace_id),
        limit,
    };
    let rows: Vec<BridgeDecisionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT decision_id, workspace_id, entity_id_a, entity_id_b, decision, \
                         degree_a, degree_b, hub_degree_threshold, evidence_span_id, \
                         bridge_edge_id, created_at FROM knowledge_memory_bridge_decisions \
                         WHERE workspace_id = $workspace \
                         ORDER BY created_at DESC, decision_id DESC LIMIT $limit;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .map(BridgeDecisionRow::into_record)
        .collect()
}
