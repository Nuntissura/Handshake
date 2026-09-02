//! WP-KERNEL-009 project wiki compile layer — "knowledge as a compile target"
//! (Karpathy LLM-wiki pattern; operator decisions DEC-004/DEC-007; Master Spec
//! 11-shared-dev-platform §10.12 Section 17 [LM-PWIKI-001..013]).
//!
//! Raw authority (code-index symbols/references/file lens, knowledge entities
//! and edges, rich documents — all PostgreSQL/EventLedger) is the "source
//! code"; the compiled wiki is the regenerable "binary". Three engines:
//!
//! * [`compiler`] — MT-241 `ProjectWikiBootstrapCompiler`: bootstrap-compiles
//!   a navigable, typed (module/concept/flow/entity/decision + index), CITED
//!   project wiki from existing authority into the EXISTING
//!   `knowledge_wiki_projections` store (LM-PWIKI-005: no parallel store),
//!   with token-aware clustering for large repos (LM-PWIKI-004) and
//!   EventLedger compile receipts (LM-PWIKI-012).
//! * [`drift`] — MT-242 `WikiProjectionDriftAndStaleness`: every compiled page
//!   is stamped with the EventLedger source version + the exact cited-source
//!   set (ids + content hashes, LM-PWIKI-006); the drift checker diffs current
//!   authority against stamps and flags exactly the stale pages with concrete
//!   reasons (LM-PWIKI-007); every page-serve path attaches a machine-readable
//!   [`WikiStalenessVerdict`] fail-closed (LM-PWIKI-008).
//! * [`fanout`] — MT-243 `WikiIncrementalIngestFanOut`: one changed source
//!   regenerates exactly the pages whose stamps cite it (set equality with the
//!   drift result, LM-PWIKI-010), refreshes page links in the same pass,
//!   bounded by an explicit budget with LOUD EventLedger truncation receipts
//!   (LM-PWIKI-011), idempotent re-run.
//!
//! SHIP-TOGETHER GUARD (LM-PWIKI-009): the bootstrap compiler and the
//! staleness layer are one delivery unit. Structurally enforced:
//! migration 0300's `chk_knowledge_wiki_projections_stamp_guard` makes a typed
//! page without a stamp impossible at the database, [`WikiCompileStamp`] is a
//! REQUIRED (non-`Option`) argument of the page-write storage path, and the
//! serve paths refuse to call any unstamped page fresh
//! ([`WikiStalenessVerdict::Unstamped`]).
//!
//! The compiled wiki is NEVER authority: deleting every generated page leaves
//! authority byte-identical (LM-PWIKI-001; proven by negative test).

pub mod compiler;
pub mod drift;
pub mod fanout;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::storage::knowledge::KnowledgeRichDocument;
use crate::storage::knowledge::KnowledgeEntity;
use crate::storage::LoomBlock;

/// Stamp/compiler version token recorded in every compile stamp; bump when the
/// hash derivations below change so old stamps are comparable on their own
/// version's terms.
pub const WIKI_COMPILER_VERSION: &str = "project_wiki_compiler_v1";

/// Stamp schema version token.
pub const WIKI_STAMP_VERSION: &str = "wiki_stamp_v1";

/// Default per-page token budget for token-aware clustering (LM-PWIKI-004).
/// A module page whose estimated render exceeds this is split into
/// deterministic `(part N)` pages instead of failing or silently truncating.
pub const DEFAULT_PAGE_TOKEN_BUDGET: usize = 4_000;

/// Hard floor/ceiling for the per-page token budget (DoS guard: a hostile
/// budget of 1 would explode the page count; an enormous one defeats
/// clustering).
pub const MIN_PAGE_TOKEN_BUDGET: usize = 256;
pub const MAX_PAGE_TOKEN_BUDGET: usize = 200_000;

/// Hard cap on pages a single bootstrap compile may emit (unbounded-compile
/// DoS guard). Exceeding it fails the compile LOUDLY (typed error + no partial
/// silent wiki), never truncates silently.
pub const MAX_BOOTSTRAP_PAGES: usize = 2_000;

/// Default / maximum fan-out budgets (MT-243, LM-PWIKI-011).
pub const DEFAULT_FANOUT_BUDGET: usize = 25;
pub const MAX_FANOUT_BUDGET: usize = 200;

/// Typed compiled wiki page kinds (LM-PWIKI-002). Projection metadata only —
/// never a new authority entity type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiPageType {
    Module,
    Concept,
    Flow,
    Entity,
    Decision,
    Index,
}

impl WikiPageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Concept => "concept",
            Self::Flow => "flow",
            Self::Entity => "entity",
            Self::Decision => "decision",
            Self::Index => "index",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "module" => Some(Self::Module),
            "concept" => Some(Self::Concept),
            "flow" => Some(Self::Flow),
            "entity" => Some(Self::Entity),
            "decision" => Some(Self::Decision),
            "index" => Some(Self::Index),
            _ => None,
        }
    }
}

/// The kind of an authority record a compiled page cites. Citations are
/// precise authority ids + content hashes — NEVER loose file-path strings
/// (LM-PWIKI-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitedSourceKind {
    /// A `knowledge_sources` row (KSRC-…); hash = its `content_hash`.
    Source,
    /// A `knowledge_entities` row (KEN-…); hash = [`entity_content_hash`].
    Entity,
    /// A `loom_blocks` row; hash = [`loom_block_content_hash`].
    LoomBlock,
    /// A `knowledge_rich_documents` row (KRD-…); hash = its `content_sha256`.
    RichDocument,
}

impl CitedSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Entity => "entity",
            Self::LoomBlock => "loom_block",
            Self::RichDocument => "rich_document",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "source" => Some(Self::Source),
            "entity" => Some(Self::Entity),
            "loom_block" => Some(Self::LoomBlock),
            "rich_document" => Some(Self::RichDocument),
            _ => None,
        }
    }
}

/// One citation in a compile stamp: the authority record the page compiled
/// from, with the content hash it had at compile time (LM-PWIKI-006).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitedSource {
    pub kind: CitedSourceKind,
    /// Stable authority id (KSRC-/KEN-/KRD-/loom block id).
    pub id: String,
    /// Lowercase sha256 hex of the cited record's content at compile time.
    pub content_hash: String,
    /// For entity citations: the definition/evidence span id (KSP-…) the page
    /// renders, as a navigation anchor. Spans are immutable evidence rows;
    /// change detection rides `content_hash`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// For entity citations: the owning `knowledge_sources` id, so a reader
    /// can jump from the citation to the source record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// The MT-242 compile stamp (LM-PWIKI-006): EventLedger source version + the
/// exact cited-source set the page compiled from. Stored as
/// `knowledge_wiki_projections.compile_stamp`; REQUIRED for every compiled
/// page (ship-together guard).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiCompileStamp {
    /// Stamp schema version ([`WIKI_STAMP_VERSION`]).
    pub stamp_version: String,
    /// `MAX(kernel_event_ledger.event_sequence)` observed when the compile
    /// read authority — the EventLedger source version. Monotonic; never
    /// wall-clock.
    pub ledger_version: i64,
    /// Compiler version token ([`WIKI_COMPILER_VERSION`]).
    pub compiler_version: String,
    /// The exact cited-source set (ids + content hashes).
    pub cited_sources: Vec<CitedSource>,
}

impl WikiCompileStamp {
    pub fn new(ledger_version: i64, mut cited_sources: Vec<CitedSource>) -> Self {
        // Deterministic stamp: citations sorted by (kind, id), deduplicated.
        cited_sources.sort_by(|a, b| {
            (a.kind.as_str(), a.id.as_str()).cmp(&(b.kind.as_str(), b.id.as_str()))
        });
        cited_sources.dedup_by(|a, b| a.kind == b.kind && a.id == b.id);
        Self {
            stamp_version: WIKI_STAMP_VERSION.to_string(),
            ledger_version,
            compiler_version: WIKI_COMPILER_VERSION.to_string(),
            cited_sources,
        }
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }

    /// Parse a stamp from its stored JSONB. Returns `None` for missing or
    /// malformed stamps (the caller must then treat the page as
    /// [`WikiStalenessVerdict::Unstamped`] — fail closed, never fresh).
    pub fn from_value(value: Option<&Value>) -> Option<Self> {
        value.and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Why one cited source makes a page stale (LM-PWIKI-007: a concrete reason —
/// which source, which version delta).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiStaleReason {
    /// Which cited source moved.
    pub kind: CitedSourceKind,
    pub id: String,
    /// The content hash the page compiled from.
    pub stamped_content_hash: String,
    /// The source's CURRENT content hash, or `None` when the cited record was
    /// deleted/retired from authority.
    pub current_content_hash: Option<String>,
    /// `source_changed` | `source_deleted`.
    pub change: WikiSourceChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiSourceChange {
    SourceChanged,
    SourceDeleted,
}

/// The machine-readable staleness verdict attached to EVERY served wiki page
/// (LM-PWIKI-008, fail-closed). Mirrors the MT-106/MT-107 code-index
/// [`crate::knowledge_code_index::staleness::StalenessVerdict`] pattern:
/// a tagged enum with a stable machine `label()`.
///
/// THE STALE-BADGE CONTRACT for the Notes UI and retrieval consumers:
/// ```json
/// { "state": "fresh",
///   "stamp_ledger_version": 123, "current_ledger_version": 130 }
/// { "state": "stale",
///   "stamp_ledger_version": 123, "current_ledger_version": 130,
///   "reasons": [ { "kind": "entity", "id": "KEN-…",
///                  "stamped_content_hash": "…64hex…",
///                  "current_content_hash": "…64hex…|null",
///                  "change": "source_changed|source_deleted" } ] }
/// { "state": "unstamped" }
/// ```
/// * `fresh`     — every cited source's current hash equals the stamp.
/// * `stale`     — at least one cited source changed/was deleted; `reasons`
///                 lists exactly which, with the stamped vs current hashes and
///                 the EventLedger version delta visible via the two version
///                 fields.
/// * `unstamped` — the page predates stamping (e.g. a legacy row created
///                 before migration 0300). FORBIDDEN to treat as fresh; the
///                 Notes UI must badge it as untrusted/regenerate-needed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WikiStalenessVerdict {
    Fresh {
        stamp_ledger_version: i64,
        current_ledger_version: i64,
    },
    Stale {
        stamp_ledger_version: i64,
        current_ledger_version: i64,
        reasons: Vec<WikiStaleReason>,
    },
    Unstamped,
}

impl WikiStalenessVerdict {
    pub fn is_fresh(&self) -> bool {
        matches!(self, Self::Fresh { .. })
    }

    /// Stable machine label for receipts/badges.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fresh { .. } => "fresh",
            Self::Stale { .. } => "stale",
            Self::Unstamped => "unstamped",
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic content-hash derivations (compile-time stamping and drift-time
// recompute MUST be the same functions — single source of truth).
// ---------------------------------------------------------------------------

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Content hash of a knowledge entity citation. Folds the entity's identity,
/// display surface, lifecycle, and — dominantly — the owning source's content
/// hash, so editing the underlying file/document flips every symbol/concept
/// entity citation anchored to it. Deliberately EXCLUDES wall-clock fields
/// (`updated_at`) and run ids (LM-PWIKI-006: no wall-clock heuristics;
/// re-indexing identical content must NOT flag pages stale).
pub fn entity_content_hash(entity: &KnowledgeEntity, source_content_hash: Option<&str>) -> String {
    sha256_hex(
        format!(
            "wiki_entity_v1|{}|{}|{}|{}|{}",
            entity.entity_kind.as_str(),
            entity.entity_key,
            entity.display_name,
            entity.lifecycle_state.as_str(),
            source_content_hash.unwrap_or("-"),
        )
        .as_bytes(),
    )
}

/// Content hash of a Loom block citation: content-bearing fields only (no
/// `updated_at`, so a metadata touch that does not change content does not
/// flag pages stale). Field-level variant shared by the compile-time stamper
/// (`&LoomBlock`) and the drift-time recompute (`WikiLoomBlockState`) so the
/// two sides can never diverge.
#[allow(clippy::too_many_arguments)]
pub fn loom_block_state_content_hash(
    title: Option<&str>,
    content_type: &str,
    full_text_index: Option<&str>,
    document_id: Option<&str>,
    asset_id: Option<&str>,
    content_hash: Option<&str>,
) -> String {
    sha256_hex(
        format!(
            "wiki_loom_block_v1|{}|{}|{}|{}|{}|{}",
            title.unwrap_or(""),
            content_type,
            full_text_index.unwrap_or(""),
            document_id.unwrap_or(""),
            asset_id.unwrap_or(""),
            content_hash.unwrap_or(""),
        )
        .as_bytes(),
    )
}

/// [`loom_block_state_content_hash`] over a full authority [`LoomBlock`].
pub fn loom_block_content_hash(block: &LoomBlock) -> String {
    loom_block_state_content_hash(
        block.title.as_deref(),
        block.content_type.as_str(),
        block.derived.full_text_index.as_deref(),
        block.document_id.as_deref(),
        block.asset_id.as_deref(),
        block.content_hash.as_deref(),
    )
}

/// Content hash of a rich-document citation: the document's own canonical
/// content hash (already sha256 over canonical JSON).
pub fn rich_document_content_hash(document: &KnowledgeRichDocument) -> String {
    document.content_sha256.clone()
}

/// Conservative token estimate for budget-aware clustering: ~4 chars/token
/// (the ubiquitous BPE heuristic), minimum 1 per non-empty text.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.chars().count().div_ceil(4)
    }
}

/// Errors of the project wiki compile layer.
#[derive(Debug, thiserror::Error)]
pub enum WikiCompileError {
    #[error("wiki compile validation: {0}")]
    Validation(String),
    #[error("wiki compile would exceed the page cap ({0} pages > {MAX_BOOTSTRAP_PAGES}); refusing unbounded compile")]
    PageCapExceeded(usize),
    #[error("storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),
    #[error("kernel event error: {0}")]
    Kernel(#[from] crate::kernel::KernelError),
}

pub type WikiCompileResult<T> = Result<T, WikiCompileError>;
