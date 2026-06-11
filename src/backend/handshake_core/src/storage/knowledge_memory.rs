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
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<FactConflictCandidate>> {
    let rows = sqlx::query(
        r#"
        SELECT
            a.subject_entity_id AS subject_entity_id,
            a.predicate_key     AS predicate_key,
            a.fact_id           AS fact_id_a,
            a.claim_id          AS claim_id_a,
            COALESCE(a.object_entity_id, a.object_literal) AS object_a,
            b.fact_id           AS fact_id_b,
            b.claim_id          AS claim_id_b,
            COALESCE(b.object_entity_id, b.object_literal) AS object_b
        FROM knowledge_memory_facts a
        JOIN knowledge_memory_facts b
          ON a.workspace_id = b.workspace_id
         AND a.subject_entity_id = b.subject_entity_id
         AND a.predicate_key = b.predicate_key
         AND a.fact_id < b.fact_id
        WHERE a.workspace_id = $1
          AND COALESCE(a.object_entity_id, a.object_literal)
              IS DISTINCT FROM COALESCE(b.object_entity_id, b.object_literal)
        ORDER BY a.subject_entity_id, a.predicate_key, a.fact_id, b.fact_id
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| FactConflictCandidate {
            subject_entity_id: row.get("subject_entity_id"),
            predicate_key: row.get("predicate_key"),
            fact_id_a: row.get("fact_id_a"),
            claim_id_a: row.get("claim_id_a"),
            object_a: row.get("object_a"),
            fact_id_b: row.get("fact_id_b"),
            claim_id_b: row.get("claim_id_b"),
            object_b: row.get("object_b"),
            candidate_reason: "symbolic_subject_predicate_object_mismatch".to_string(),
        })
        .collect())
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

fn detection_job_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<ConflictDetectionJob> {
    Ok(ConflictDetectionJob {
        job_id: row.get("job_id"),
        workspace_id: row.get("workspace_id"),
        detection_kind: ConflictDetectionKind::from_db(
            row.get::<String, _>("detection_kind").as_str(),
        )?,
        job_state: row.get("job_state"),
        candidates_scanned: row.get("candidates_scanned"),
        conflicts_found: row.get("conflicts_found"),
        search_parameters: row.get("search_parameters"),
        detection_receipt_event_id: row.get("detection_receipt_event_id"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    })
}

/// Record a completed conflict-detection job and link the conflict ids it
/// found, in one transaction. `conflict_ids` are existing
/// knowledge_claim_conflicts rows (produced by the detection pass).
pub async fn record_conflict_detection_job(
    pool: &PgPool,
    workspace_id: &str,
    detection_kind: ConflictDetectionKind,
    candidates_scanned: i32,
    search_parameters: Value,
    conflict_ids: &[String],
    detection_receipt_event_id: Option<&str>,
) -> StorageResult<ConflictDetectionJob> {
    let job_id = new_memory_id("KCDJ");
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        r#"
        INSERT INTO knowledge_memory_conflict_detection_jobs (
            job_id, workspace_id, detection_kind, job_state, candidates_scanned,
            conflicts_found, search_parameters, detection_receipt_event_id,
            completed_at
        ) VALUES ($1, $2, $3, 'completed', $4, $5, $6, $7, NOW())
        RETURNING job_id, workspace_id, detection_kind, job_state,
                  candidates_scanned, conflicts_found, search_parameters,
                  detection_receipt_event_id, created_at, completed_at
        "#,
    )
    .bind(&job_id)
    .bind(workspace_id)
    .bind(detection_kind.as_str())
    .bind(candidates_scanned)
    .bind(conflict_ids.len() as i32)
    .bind(&search_parameters)
    .bind(detection_receipt_event_id)
    .fetch_one(&mut *tx)
    .await?;

    for conflict_id in conflict_ids {
        sqlx::query(
            r#"
            INSERT INTO knowledge_memory_conflict_detection_findings (job_id, conflict_id)
            VALUES ($1, $2)
            "#,
        )
        .bind(&job_id)
        .bind(conflict_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    detection_job_from_row(&row)
}

/// List the conflict ids a detection job found.
pub async fn list_conflict_detection_findings(
    pool: &PgPool,
    job_id: &str,
) -> StorageResult<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT conflict_id FROM knowledge_memory_conflict_detection_findings
        WHERE job_id = $1 ORDER BY conflict_id ASC
        "#,
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>("conflict_id"))
        .collect())
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

fn resolution_job_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<ConflictResolutionJob> {
    Ok(ConflictResolutionJob {
        job_id: row.get("job_id"),
        workspace_id: row.get("workspace_id"),
        conflict_id: row.get("conflict_id"),
        outcome: ConflictResolutionOutcome::from_db(row.get::<String, _>("outcome").as_str())?,
        kept_claim_id: row.get("kept_claim_id"),
        discarded_claim_id: row.get("discarded_claim_id"),
        resolution_detail: row.get("resolution_detail"),
        resolution_receipt_event_id: row.get("resolution_receipt_event_id"),
        created_at: row.get("created_at"),
    })
}

/// Record a conflict-resolution job. Validates the kept/discarded claim shape
/// against the chosen outcome before insert (the DB CHECK is the backstop).
#[allow(clippy::too_many_arguments)]
pub async fn record_conflict_resolution_job(
    pool: &PgPool,
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
    let row = sqlx::query(
        r#"
        INSERT INTO knowledge_memory_conflict_resolution_jobs (
            job_id, workspace_id, conflict_id, outcome, kept_claim_id,
            discarded_claim_id, resolution_detail, resolution_receipt_event_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING job_id, workspace_id, conflict_id, outcome, kept_claim_id,
                  discarded_claim_id, resolution_detail,
                  resolution_receipt_event_id, created_at
        "#,
    )
    .bind(&job_id)
    .bind(workspace_id)
    .bind(conflict_id)
    .bind(outcome.as_str())
    .bind(kept_claim_id)
    .bind(discarded_claim_id)
    .bind(&resolution_detail)
    .bind(resolution_receipt_event_id)
    .fetch_one(pool)
    .await?;
    resolution_job_from_row(&row)
}

/// List resolution jobs for a conflict (newest first).
pub async fn list_conflict_resolution_jobs(
    pool: &PgPool,
    conflict_id: &str,
) -> StorageResult<Vec<ConflictResolutionJob>> {
    let rows = sqlx::query(
        r#"
        SELECT job_id, workspace_id, conflict_id, outcome, kept_claim_id,
               discarded_claim_id, resolution_detail, resolution_receipt_event_id,
               created_at
        FROM knowledge_memory_conflict_resolution_jobs
        WHERE conflict_id = $1
        ORDER BY created_at DESC, job_id DESC
        "#,
    )
    .bind(conflict_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(resolution_job_from_row).collect()
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
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<EntityCooccurrence>> {
    let rows = sqlx::query(
        r#"
        SELECT esa.entity_id AS entity_id_a,
               esb.entity_id AS entity_id_b,
               esa.span_id   AS shared_span_id
        FROM knowledge_entity_spans esa
        JOIN knowledge_entity_spans esb
          ON esa.span_id = esb.span_id
         AND esa.entity_id < esb.entity_id
        JOIN knowledge_entities ea ON ea.entity_id = esa.entity_id
        JOIN knowledge_entities eb ON eb.entity_id = esb.entity_id
        WHERE ea.workspace_id = $1 AND eb.workspace_id = $1
        ORDER BY esa.entity_id, esb.entity_id, esa.span_id
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| EntityCooccurrence {
            entity_id_a: row.get("entity_id_a"),
            entity_id_b: row.get("entity_id_b"),
            shared_span_id: row.get("shared_span_id"),
        })
        .collect())
}

/// A directed-or-undirected edge endpoint pair from `knowledge_edges`, used to
/// build connected components (only non-retired edges count toward
/// connectivity). Returned as (source, target) pairs.
pub async fn list_active_edge_endpoints(
    pool: &PgPool,
    workspace_id: &str,
) -> StorageResult<Vec<(String, String)>> {
    let rows = sqlx::query(
        r#"
        SELECT source_entity_id, target_entity_id
        FROM knowledge_edges
        WHERE workspace_id = $1 AND lifecycle_state <> 'retired'
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("source_entity_id"),
                row.get::<String, _>("target_entity_id"),
            )
        })
        .collect())
}

/// Undirected degree of an entity in the non-retired edge graph (number of
/// edges touching it as source or target). The hub-suppression input.
pub async fn entity_edge_degree(pool: &PgPool, entity_id: &str) -> StorageResult<i64> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) AS degree
        FROM knowledge_edges
        WHERE lifecycle_state <> 'retired'
          AND (source_entity_id = $1 OR target_entity_id = $1)
        "#,
    )
    .bind(entity_id)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>("degree"))
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

fn bridge_decision_from_row(row: &sqlx::postgres::PgRow) -> StorageResult<BridgeDecisionRecord> {
    Ok(BridgeDecisionRecord {
        decision_id: row.get("decision_id"),
        workspace_id: row.get("workspace_id"),
        entity_id_a: row.get("entity_id_a"),
        entity_id_b: row.get("entity_id_b"),
        decision: BridgeDecision::from_db(row.get::<String, _>("decision").as_str())?,
        degree_a: row.get("degree_a"),
        degree_b: row.get("degree_b"),
        hub_degree_threshold: row.get("hub_degree_threshold"),
        evidence_span_id: row.get("evidence_span_id"),
        bridge_edge_id: row.get("bridge_edge_id"),
        created_at: row.get("created_at"),
    })
}

/// Record one bridge-evaluation decision.
#[allow(clippy::too_many_arguments)]
pub async fn record_bridge_decision(
    pool: &PgPool,
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
    let row = sqlx::query(
        r#"
        INSERT INTO knowledge_memory_bridge_decisions (
            decision_id, workspace_id, entity_id_a, entity_id_b, decision,
            degree_a, degree_b, hub_degree_threshold, evidence_span_id,
            bridge_edge_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING decision_id, workspace_id, entity_id_a, entity_id_b, decision,
                  degree_a, degree_b, hub_degree_threshold, evidence_span_id,
                  bridge_edge_id, created_at
        "#,
    )
    .bind(&decision_id)
    .bind(workspace_id)
    .bind(entity_id_a)
    .bind(entity_id_b)
    .bind(decision.as_str())
    .bind(degree_a)
    .bind(degree_b)
    .bind(hub_degree_threshold)
    .bind(evidence_span_id)
    .bind(bridge_edge_id)
    .fetch_one(pool)
    .await?;
    bridge_decision_from_row(&row)
}

/// List bridge decisions for a workspace, newest first.
pub async fn list_bridge_decisions(
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<Vec<BridgeDecisionRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT decision_id, workspace_id, entity_id_a, entity_id_b, decision,
               degree_a, degree_b, hub_degree_threshold, evidence_span_id,
               bridge_edge_id, created_at
        FROM knowledge_memory_bridge_decisions
        WHERE workspace_id = $1
        ORDER BY created_at DESC, decision_id DESC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter().map(bridge_decision_from_row).collect()
}
