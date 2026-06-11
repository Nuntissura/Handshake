//! WP-KERNEL-009 MemoryGraphAndClaims storage (MT-113..MT-128).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 and the
//! WP-009 contract field `translated_memory_system_spec`. This module is the
//! PostgreSQL surface for the MemoryGraph layer (`knowledge_memory_*`,
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
//! Pattern follows `storage/knowledge_crdt.rs`: free async functions over
//! `&sqlx::PgPool` rather than widening the legacy `Database` trait. There is
//! NO in-memory, SQLite, or fixture fallback: without PostgreSQL every function
//! fails closed with a typed `StorageError`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{StorageError, StorageResult};

/// Mint a `<PREFIX>-<32 hex>` id matching the `knowledge_memory_*` CHECKs.
/// Uuidv7 is time-ordered; `.simple()` is exactly 32 lowercase hex chars.
fn new_memory_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7().simple())
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

const ONTOLOGY_TERM_COLUMNS: &str = r#"
    term_id, workspace_id, term_kind, term_key, normalized_label,
    maps_to_edge_type, maps_to_entity_kind, lifecycle_state, retirement_reason,
    superseded_by_term_id, observation_count, promotion_threshold,
    operator_approved, promotion_receipt_event_id, detection_provenance,
    first_seen_in_run, last_seen_in_run, created_at, updated_at
"#;

fn ontology_term_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<MemoryOntologyTerm> {
    Ok(MemoryOntologyTerm {
        term_id: row.get("term_id"),
        workspace_id: row.get("workspace_id"),
        term_kind: MemoryOntologyTermKind::from_db(row.get::<String, _>("term_kind").as_str())?,
        term_key: row.get("term_key"),
        normalized_label: row.get("normalized_label"),
        maps_to_edge_type: row.get("maps_to_edge_type"),
        maps_to_entity_kind: row.get("maps_to_entity_kind"),
        lifecycle_state: MemoryOntologyLifecycle::from_db(
            row.get::<String, _>("lifecycle_state").as_str(),
        )?,
        retirement_reason: row
            .get::<Option<String>, _>("retirement_reason")
            .map(|value| MemoryOntologyRetirementReason::from_db(&value))
            .transpose()?,
        superseded_by_term_id: row.get("superseded_by_term_id"),
        observation_count: row.get("observation_count"),
        promotion_threshold: row.get("promotion_threshold"),
        operator_approved: row.get("operator_approved"),
        promotion_receipt_event_id: row.get("promotion_receipt_event_id"),
        detection_provenance: row.get("detection_provenance"),
        first_seen_in_run: row.get("first_seen_in_run"),
        last_seen_in_run: row.get("last_seen_in_run"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// Upsert a probationary ontology term on its stable identity
/// (workspace, kind, key). Re-derivation by a later run increments the
/// observation count and refreshes provenance/last_seen, WITHOUT moving the
/// lifecycle state (promotion is a separate receipt-backed step).
pub async fn upsert_memory_ontology_term(
    pool: &PgPool,
    new: NewMemoryOntologyTerm,
) -> StorageResult<MemoryOntologyTerm> {
    if new.maps_to_edge_type.is_some() && new.maps_to_entity_kind.is_some() {
        return Err(StorageError::Validation(
            "ontology term cannot map to both an edge type and an entity kind",
        ));
    }
    let term_id = new_memory_id("KMO");
    let sql = format!(
        r#"
        INSERT INTO knowledge_memory_ontology_terms (
            term_id, workspace_id, term_kind, term_key, normalized_label,
            maps_to_edge_type, maps_to_entity_kind, observation_count,
            promotion_threshold, operator_approved, detection_provenance,
            first_seen_in_run, last_seen_in_run
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, 1, $8, $9, $10, $11, $11
        )
        ON CONFLICT (workspace_id, term_kind, term_key) DO UPDATE SET
            normalized_label = EXCLUDED.normalized_label,
            observation_count = knowledge_memory_ontology_terms.observation_count + 1,
            operator_approved =
                knowledge_memory_ontology_terms.operator_approved OR EXCLUDED.operator_approved,
            detection_provenance = EXCLUDED.detection_provenance,
            last_seen_in_run = EXCLUDED.last_seen_in_run,
            updated_at = NOW()
        RETURNING {ONTOLOGY_TERM_COLUMNS}
        "#
    );
    let row = sqlx::query(&sql)
        .bind(&term_id)
        .bind(&new.workspace_id)
        .bind(new.term_kind.as_str())
        .bind(&new.term_key)
        .bind(&new.normalized_label)
        .bind(&new.maps_to_edge_type)
        .bind(&new.maps_to_entity_kind)
        .bind(new.promotion_threshold)
        .bind(new.operator_approved)
        .bind(&new.detection_provenance)
        .bind(&new.seen_in_run)
        .fetch_one(pool)
        .await?;
    ontology_term_from_row(&row)
}

/// Fetch one ontology term by id.
pub async fn get_memory_ontology_term(
    pool: &PgPool,
    term_id: &str,
) -> StorageResult<Option<MemoryOntologyTerm>> {
    let sql = format!(
        "SELECT {ONTOLOGY_TERM_COLUMNS} FROM knowledge_memory_ontology_terms WHERE term_id = $1"
    );
    let row = sqlx::query(&sql).bind(term_id).fetch_optional(pool).await?;
    row.as_ref().map(ontology_term_from_row).transpose()
}

/// List ontology terms for a workspace, optionally filtered to one kind and/or
/// lifecycle state, newest first, bounded by `limit`.
pub async fn list_memory_ontology_terms(
    pool: &PgPool,
    workspace_id: &str,
    term_kind: Option<MemoryOntologyTermKind>,
    lifecycle_state: Option<MemoryOntologyLifecycle>,
    limit: i64,
) -> StorageResult<Vec<MemoryOntologyTerm>> {
    let sql = format!(
        r#"
        SELECT {ONTOLOGY_TERM_COLUMNS} FROM knowledge_memory_ontology_terms
        WHERE workspace_id = $1
          AND ($2::text IS NULL OR term_kind = $2)
          AND ($3::text IS NULL OR lifecycle_state = $3)
        ORDER BY created_at DESC, term_id DESC
        LIMIT $4
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(workspace_id)
        .bind(term_kind.map(|kind| kind.as_str()))
        .bind(lifecycle_state.map(|state| state.as_str()))
        .bind(limit)
        .fetch_all(pool)
        .await?;
    rows.iter().map(ontology_term_from_row).collect()
}

/// MT-119/MT-120: promote a probationary term to stable, backed by an
/// EventLedger receipt. Fails closed (`Validation`) if the term has not cleared
/// its promotion gate, (`Conflict`) if it is not probationary, and the DB
/// trigger independently refuses a stable row without a receipt.
pub async fn promote_memory_ontology_term(
    pool: &PgPool,
    term_id: &str,
    promotion_receipt_event_id: &str,
) -> StorageResult<MemoryOntologyTerm> {
    let current = get_memory_ontology_term(pool, term_id)
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
    let sql = format!(
        r#"
        UPDATE knowledge_memory_ontology_terms
           SET lifecycle_state = 'stable',
               promotion_receipt_event_id = $2,
               updated_at = NOW()
         WHERE term_id = $1
        RETURNING {ONTOLOGY_TERM_COLUMNS}
        "#
    );
    let row = sqlx::query(&sql)
        .bind(term_id)
        .bind(promotion_receipt_event_id)
        .fetch_one(pool)
        .await?;
    ontology_term_from_row(&row)
}

/// Retire an ontology term with a reason (and optional supersessor).
pub async fn retire_memory_ontology_term(
    pool: &PgPool,
    term_id: &str,
    reason: MemoryOntologyRetirementReason,
    superseded_by_term_id: Option<&str>,
) -> StorageResult<MemoryOntologyTerm> {
    if superseded_by_term_id.is_some() && reason != MemoryOntologyRetirementReason::Superseded {
        return Err(StorageError::Validation(
            "superseded_by_term_id requires the 'superseded' retirement reason",
        ));
    }
    let current = get_memory_ontology_term(pool, term_id)
        .await?
        .ok_or(StorageError::NotFound("memory ontology term"))?;
    if current.lifecycle_state == MemoryOntologyLifecycle::Retired {
        return Err(StorageError::Conflict("ontology term is already retired"));
    }
    let sql = format!(
        r#"
        UPDATE knowledge_memory_ontology_terms
           SET lifecycle_state = 'retired',
               retirement_reason = $2,
               superseded_by_term_id = $3,
               updated_at = NOW()
         WHERE term_id = $1
        RETURNING {ONTOLOGY_TERM_COLUMNS}
        "#
    );
    let row = sqlx::query(&sql)
        .bind(term_id)
        .bind(reason.as_str())
        .bind(superseded_by_term_id)
        .fetch_one(pool)
        .await?;
    ontology_term_from_row(&row)
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

fn ontology_alias_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<MemoryOntologyAlias> {
    Ok(MemoryOntologyAlias {
        alias_id: row.get("alias_id"),
        term_id: row.get("term_id"),
        workspace_id: row.get("workspace_id"),
        alias_surface: row.get("alias_surface"),
        alias_norm_key: row.get("alias_norm_key"),
        alias_source: MemoryOntologyAliasSource::from_db(
            row.get::<String, _>("alias_source").as_str(),
        )?,
        created_at: row.get("created_at"),
    })
}

/// Add an alias for a term. The (workspace, alias_norm_key) uniqueness means a
/// normalized spelling resolves to exactly one canonical term.
pub async fn add_memory_ontology_alias(
    pool: &PgPool,
    term_id: &str,
    workspace_id: &str,
    alias_surface: &str,
    alias_norm_key: &str,
    alias_source: MemoryOntologyAliasSource,
) -> StorageResult<MemoryOntologyAlias> {
    let alias_id = new_memory_id("KMA");
    let row = sqlx::query(
        r#"
        INSERT INTO knowledge_memory_ontology_aliases (
            alias_id, term_id, workspace_id, alias_surface, alias_norm_key, alias_source
        ) VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING alias_id, term_id, workspace_id, alias_surface, alias_norm_key,
                  alias_source, created_at
        "#,
    )
    .bind(&alias_id)
    .bind(term_id)
    .bind(workspace_id)
    .bind(alias_surface)
    .bind(alias_norm_key)
    .bind(alias_source.as_str())
    .fetch_one(pool)
    .await?;
    ontology_alias_from_row(&row)
}

/// Resolve an alias surface (by its normalized key) to its canonical term.
pub async fn resolve_memory_ontology_alias(
    pool: &PgPool,
    workspace_id: &str,
    alias_norm_key: &str,
) -> StorageResult<Option<MemoryOntologyTerm>> {
    // `t.*` projects exactly the term columns (unambiguous after the JOIN);
    // `ontology_term_from_row` reads them by name.
    let row = sqlx::query(
        r#"
        SELECT t.* FROM knowledge_memory_ontology_terms t
        JOIN knowledge_memory_ontology_aliases a ON a.term_id = t.term_id
        WHERE a.workspace_id = $1 AND a.alias_norm_key = $2
        "#,
    )
    .bind(workspace_id)
    .bind(alias_norm_key)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(ontology_term_from_row).transpose()
}

/// List all aliases for a term.
pub async fn list_memory_ontology_aliases(
    pool: &PgPool,
    term_id: &str,
) -> StorageResult<Vec<MemoryOntologyAlias>> {
    let rows = sqlx::query(
        r#"
        SELECT alias_id, term_id, workspace_id, alias_surface, alias_norm_key,
               alias_source, created_at
        FROM knowledge_memory_ontology_aliases
        WHERE term_id = $1
        ORDER BY created_at ASC, alias_id ASC
        "#,
    )
    .bind(term_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(ontology_alias_from_row).collect()
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
            // An unsupported fact can be re-grounded (evidence returns) or
            // promoted by an operator.
            Unsupported => matches!(to, Source | Derived | ModelSuggested | OperatorApproved),
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

const FACT_COLUMNS: &str = r#"
    fact_id, workspace_id, claim_id, subject_entity_id, predicate_key,
    predicate_term_id, object_entity_id, object_literal, qualifiers,
    authority_label, extractor_version, created_in_run, created_at, updated_at
"#;

fn fact_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<MemoryFact> {
    Ok(MemoryFact {
        fact_id: row.get("fact_id"),
        workspace_id: row.get("workspace_id"),
        claim_id: row.get("claim_id"),
        subject_entity_id: row.get("subject_entity_id"),
        predicate_key: row.get("predicate_key"),
        predicate_term_id: row.get("predicate_term_id"),
        object_entity_id: row.get("object_entity_id"),
        object_literal: row.get("object_literal"),
        qualifiers: row.get("qualifiers"),
        authority_label: MemoryClaimAuthorityLabel::from_db(
            row.get::<String, _>("authority_label").as_str(),
        )?,
        extractor_version: row.get("extractor_version"),
        created_in_run: row.get("created_in_run"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// Create a memory fact attached to an existing backing claim.
pub async fn create_memory_fact(pool: &PgPool, new: NewMemoryFact) -> StorageResult<MemoryFact> {
    let fact_id = new_memory_id("KMF");
    let (object_entity_id, object_literal) = match &new.object {
        MemoryFactObject::Entity { entity_id } => (Some(entity_id.clone()), None),
        MemoryFactObject::Literal { value } => (None, Some(value.clone())),
    };
    let sql = format!(
        r#"
        INSERT INTO knowledge_memory_facts (
            fact_id, workspace_id, claim_id, subject_entity_id, predicate_key,
            predicate_term_id, object_entity_id, object_literal, qualifiers,
            authority_label, extractor_version, created_in_run
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING {FACT_COLUMNS}
        "#
    );
    let row = sqlx::query(&sql)
        .bind(&fact_id)
        .bind(&new.workspace_id)
        .bind(&new.claim_id)
        .bind(&new.subject_entity_id)
        .bind(&new.predicate_key)
        .bind(&new.predicate_term_id)
        .bind(&object_entity_id)
        .bind(&object_literal)
        .bind(&new.qualifiers)
        .bind(new.authority_label.as_str())
        .bind(&new.extractor_version)
        .bind(&new.created_in_run)
        .fetch_one(pool)
        .await?;
    fact_from_row(&row)
}

/// Fetch one fact by id.
pub async fn get_memory_fact(pool: &PgPool, fact_id: &str) -> StorageResult<Option<MemoryFact>> {
    let sql = format!("SELECT {FACT_COLUMNS} FROM knowledge_memory_facts WHERE fact_id = $1");
    let row = sqlx::query(&sql).bind(fact_id).fetch_optional(pool).await?;
    row.as_ref().map(fact_from_row).transpose()
}

/// Fetch the fact backed by a given claim, if any.
pub async fn get_memory_fact_by_claim(
    pool: &PgPool,
    claim_id: &str,
) -> StorageResult<Option<MemoryFact>> {
    let sql = format!("SELECT {FACT_COLUMNS} FROM knowledge_memory_facts WHERE claim_id = $1");
    let row = sqlx::query(&sql)
        .bind(claim_id)
        .fetch_optional(pool)
        .await?;
    row.as_ref().map(fact_from_row).transpose()
}

/// List facts for a workspace, newest first, bounded by `limit`.
pub async fn list_memory_facts(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<MemoryFact>> {
    let sql = format!(
        r#"
        SELECT {FACT_COLUMNS} FROM knowledge_memory_facts
        WHERE workspace_id = $1
        ORDER BY created_at DESC, fact_id DESC
        LIMIT $2
        "#
    );
    let rows = sqlx::query(&sql)
        .bind(workspace_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    rows.iter().map(fact_from_row).collect()
}

/// MT-125: re-label a fact's authority, enforcing the legal label-transition
/// table. An illegal transition is a typed `Conflict` (e.g. silently demoting
/// an operator-approved fact to model_suggested).
pub async fn set_memory_fact_authority_label(
    pool: &PgPool,
    fact_id: &str,
    to: MemoryClaimAuthorityLabel,
) -> StorageResult<MemoryFact> {
    let current = get_memory_fact(pool, fact_id)
        .await?
        .ok_or(StorageError::NotFound("memory fact"))?;
    if !current.authority_label.can_transition_to(to) {
        return Err(StorageError::Conflict(
            "illegal memory fact authority label transition",
        ));
    }
    let sql = format!(
        r#"
        UPDATE knowledge_memory_facts
           SET authority_label = $2, updated_at = NOW()
         WHERE fact_id = $1
        RETURNING {FACT_COLUMNS}
        "#
    );
    let row = sqlx::query(&sql)
        .bind(fact_id)
        .bind(to.as_str())
        .fetch_one(pool)
        .await?;
    fact_from_row(&row)
}
