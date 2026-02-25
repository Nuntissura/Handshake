//! ACE Runtime Module (§2.6.6.7.14)
//!
//! Implements the Retrieval Correctness & Efficiency contract (ACE-RAG-001).
//! This module provides:
//! - QueryPlan and RetrievalTrace schemas
//! - AceRuntimeValidator trait and 4 required Guards
//! - Deterministic retrieval algorithms
//!
//! Spec Reference: Handshake_Master_Spec_v02.113 §2.6.6.7.14

pub mod validators;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types (ACE-001 through ACE-006)
// ============================================================================

/// ACE Runtime errors with stable error codes for debugging.
/// See the governance runbook for error resolution guidance.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AceError {
    #[error("ACE-001: Budget exceeded: {field} ({actual} > {max})")]
    BudgetExceeded {
        field: String,
        actual: u32,
        max: u32,
    },

    #[error("ACE-002: Context pack stale: pack_id={pack_id}, source_hash mismatch")]
    ContextPackStale { pack_id: Uuid },

    #[error("ACE-003: Index drift detected: {details}")]
    IndexDrift { details: String },

    #[error("ACE-004: Cache key missing in strict mode: stage={stage}")]
    CacheKeyMissing { stage: String },

    #[error("ACE-005: Query plan required but not provided")]
    QueryPlanRequired,

    #[error("ACE-006: Validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("ACE-007: Provenance missing for evidence: {candidate_id}")]
    ProvenanceMissing { candidate_id: String },

    #[error("ACE-008: Truncation required but flag not set: span at {source_id}")]
    TruncationFlagMissing { source_id: String },

    #[error("ACE-009: Determinism violation: {reason}")]
    DeterminismViolation { reason: String },

    #[error("ACE-010: Inline delta exceeds limit: {actual} > {limit} chars")]
    InlineDeltaExceeded { actual: u32, limit: u32 },

    #[error("ACE-011: Compaction schema violation: {reason}")]
    CompactionSchemaViolation { reason: String },

    #[error("ACE-012: Memory promotion blocked: {reason}")]
    MemoryPromotionBlocked { reason: String },

    #[error("ACE-013: Cloud leakage blocked: {reason}")]
    CloudLeakageBlocked { reason: String },

    #[error(
        "ACE-014: Prompt injection detected: pattern={pattern}, offset={offset}, context={context}"
    )]
    PromptInjectionDetected {
        pattern: String,
        offset: usize,
        context: String,
    },

    #[error("ACE-015: Job boundary violation: {field} changed from {original} to {current}")]
    JobBoundaryViolation {
        field: String,
        original: String,
        current: String,
    },

    #[error("ACE-016: Local payload violation: {reason}")]
    LocalPayloadViolation { reason: String },
}

// ============================================================================
// Core Types (foundational - to be extended in separate WPs)
// ============================================================================

/// SHA256 hash as hex string
pub type Hash = String;

/// Reference to a source document with its content hash for drift detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SourceRef {
    pub source_id: Uuid,
    pub source_hash: Hash,
}

impl SourceRef {
    pub fn new(source_id: Uuid, source_hash: Hash) -> Self {
        Self {
            source_id,
            source_hash,
        }
    }

    /// Create a canonical string representation for tie-breaking
    pub fn canonical_id(&self) -> String {
        format!("source:{}:{}", self.source_id, self.source_hash)
    }
}

/// Reference to an entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityRef {
    pub entity_id: Uuid,
    pub entity_type: String,
}

impl EntityRef {
    pub fn new(entity_id: Uuid, entity_type: String) -> Self {
        Self {
            entity_id,
            entity_type,
        }
    }

    /// Create a canonical string representation for tie-breaking
    pub fn canonical_id(&self) -> String {
        format!("entity:{}:{}", self.entity_id, self.entity_type)
    }
}

/// Handle to a stored artifact (ContextPack payload, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ArtifactHandle {
    pub artifact_id: Uuid,
    pub path: String,
}

impl ArtifactHandle {
    pub fn new(artifact_id: Uuid, path: String) -> Self {
        Self { artifact_id, path }
    }

    /// Create a canonical string representation for tie-breaking
    pub fn canonical_id(&self) -> String {
        format!("artifact:{}:{}", self.artifact_id, self.path)
    }
}

// ============================================================================
// QueryPlan Types (§2.6.6.7.14.5)
// ============================================================================

/// Classification of query intent for routing decisions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueryKind {
    FactLookup,
    Summarize,
    Compare,
    Transform,
    Export,
    #[default]
    Unknown,
}

/// Determinism mode for retrieval operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismMode {
    /// Retrieval MUST be deterministic (or deterministic approximation with fixed seed)
    #[default]
    Strict,
    /// Retrieval MAY be approximate, but candidate list and selection inputs are persisted
    Replay,
}

/// Data store types for retrieval routing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StoreKind {
    /// Mechanical compaction artifacts (preferred)
    ContextPacks,
    /// Knowledge graph (prefilter candidate entity sets)
    KnowledgeGraph,
    /// Shadow workspace lexical search (high-precision)
    ShadowWsLexical,
    /// Shadow workspace vector search (semantic recall)
    ShadowWsVector,
    /// Local web cache (only if cached or external fetch allowed)
    LocalWebCache,
    /// Bounded read escalation (resolve ambiguity)
    BoundedReadOnly,
}

/// A step in the retrieval route
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteStep {
    /// Which store to query
    pub store: StoreKind,
    /// Human-readable purpose (logged, not executed as code)
    pub purpose: String,
    /// Maximum candidates to retrieve from this step
    pub max_candidates: u32,
    /// If true, failure is a hard error, not a silent skip
    pub required: bool,
}

impl RouteStep {
    pub fn new(
        store: StoreKind,
        purpose: impl Into<String>,
        max_candidates: u32,
        required: bool,
    ) -> Self {
        Self {
            store,
            purpose: purpose.into(),
            max_candidates,
            required,
        }
    }
}

/// Budget constraints for retrieval operations (§2.6.6.7.14.5)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalBudgets {
    /// Maximum total evidence tokens in final context
    pub max_total_evidence_tokens: u32,
    /// Maximum total snippets across all sources
    pub max_snippets_total: u32,
    /// Maximum snippets from a single source
    pub max_snippets_per_source: u32,
    /// Maximum candidates to consider total
    pub max_candidates_total: u32,
    /// Maximum tokens per bounded read
    pub max_read_tokens: u32,
    /// Maximum tool calls during retrieval
    pub max_tool_calls: u32,
    /// Maximum candidates for reranking
    pub max_rerank_candidates: u32,
    /// Character limit for inline tool delta output
    pub tool_delta_inline_char_limit: u32,
}

impl Default for RetrievalBudgets {
    fn default() -> Self {
        Self {
            max_total_evidence_tokens: 4000,
            max_snippets_total: 20,
            max_snippets_per_source: 3,
            max_candidates_total: 100,
            max_read_tokens: 500,
            max_tool_calls: 5,
            max_rerank_candidates: 50,
            tool_delta_inline_char_limit: 2000,
        }
    }
}

impl RetrievalBudgets {
    /// Compute a deterministic hash of the budgets for cache key
    pub fn compute_hash(&self) -> Hash {
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if budgets are valid (non-zero where required)
    pub fn validate(&self) -> Result<(), AceError> {
        if self.max_total_evidence_tokens == 0 {
            return Err(AceError::ValidationFailed {
                message: "max_total_evidence_tokens must be > 0".to_string(),
            });
        }
        if self.max_read_tokens == 0 {
            return Err(AceError::ValidationFailed {
                message: "max_read_tokens must be > 0".to_string(),
            });
        }
        Ok(())
    }
}

/// Minimum trust level for evidence
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Low,
    #[default]
    Medium,
    High,
}

/// Time range filter for retrieval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// View mode for Lens-style retrieval + output (spec Addendum 2.4)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViewMode {
    #[default]
    Nsfw,
    Sfw,
}

/// Content tier for a candidate/result (spec §11.2)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContentTier {
    Sfw,
    AdultSoft,
    AdultExplicit,
}

/// Projection kind marker for SFW-projected output (spec §11.3)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectionKind {
    Sfw,
}

/// Filters for retrieval operations (§2.6.6.7.14.5)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RetrievalFilters {
    /// Whether external web fetches are allowed
    pub allow_external_fetch: bool,
    /// Minimum trust level for evidence
    pub trust_min: TrustLevel,
    /// View mode (SFW/NSFW). In SFW, unknown content_tier is default-deny (hard drop).
    pub view_mode: ViewMode,
    /// Allowlist of content tiers (None = all allowed)
    pub content_tier_allowlist: Option<Vec<String>>,
    /// Allowlist of consent profiles (None = all allowed)
    pub consent_profile_allowlist: Option<Vec<String>>,
    /// Allowlist of entity types (None = all allowed)
    pub entity_types_allowlist: Option<Vec<String>>,
    /// Time range filter
    pub time_range: Option<TimeRange>,
}

impl RetrievalFilters {
    /// Compute a deterministic hash of the filters for cache key
    pub fn compute_hash(&self) -> Hash {
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Query plan for retrieval operations (§2.6.6.7.14.5)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Unique identifier for this plan
    pub plan_id: Uuid,
    /// When the plan was created
    pub created_at: DateTime<Utc>,
    /// The original query text
    pub query_text: String,
    /// Classification of query intent
    pub query_kind: QueryKind,
    /// Ordered list of retrieval steps
    pub route: Vec<RouteStep>,
    /// Budget constraints
    pub budgets: RetrievalBudgets,
    /// Filter constraints
    pub filters: RetrievalFilters,
    /// Determinism mode (strict or replay)
    pub determinism_mode: DeterminismMode,
    /// Policy identifier for this retrieval
    pub policy_id: String,
    /// Plan version for tracking changes
    pub version: u32,
}

impl QueryPlan {
    /// Create a new query plan
    pub fn new(query_text: String, query_kind: QueryKind, policy_id: String) -> Self {
        Self {
            plan_id: Uuid::new_v4(),
            created_at: Utc::now(),
            query_text,
            query_kind,
            route: Vec::new(),
            budgets: RetrievalBudgets::default(),
            filters: RetrievalFilters::default(),
            determinism_mode: DeterminismMode::Strict,
            policy_id,
            version: 1,
        }
    }

    /// Create default routing policy per §2.6.6.7.14.6
    pub fn with_default_route(mut self) -> Self {
        self.route = vec![
            RouteStep::new(
                StoreKind::ContextPacks,
                "Primary: mechanical compactions",
                20,
                false,
            ),
            RouteStep::new(
                StoreKind::KnowledgeGraph,
                "Secondary: entity prefilter",
                50,
                false,
            ),
            RouteStep::new(
                StoreKind::ShadowWsLexical,
                "Tertiary: high-precision lexical",
                30,
                false,
            ),
            RouteStep::new(
                StoreKind::ShadowWsVector,
                "Quaternary: semantic recall",
                30,
                false,
            ),
            RouteStep::new(StoreKind::LocalWebCache, "Cached web content", 10, false),
            RouteStep::new(
                StoreKind::BoundedReadOnly,
                "Escalation: resolve ambiguity",
                5,
                false,
            ),
        ];
        self
    }

    /// Compute normalized query hash per §2.6.6.7.14.6(B)
    pub fn compute_normalized_query_hash(&self) -> Hash {
        let normalized = normalize_query(&self.query_text);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Normalize a query string per §2.6.6.7.14.6(B)
/// - Trims leading/trailing whitespace
/// - Collapses internal whitespace runs to single spaces
/// - NFC normalizes unicode
/// - Lowercases using Unicode casefold
/// - Strips control characters
///
/// INVARIANT: Uses ASCII-only whitespace definition for collapse to ensure
/// total determinism across all environments [CX-573E].
pub fn normalize_query(query: &str) -> String {
    use caseless::default_case_fold_str;
    use unicode_normalization::UnicodeNormalization;

    // ASCII whitespace - fixed definition for deterministic behavior
    fn is_ascii_ws(c: char) -> bool {
        matches!(c, ' ' | '\t' | '\n' | '\r')
    }

    // Step 1: NFC normalize, convert whitespace to space, strip non-whitespace control chars
    // Per spec §2.6.6.7.14.6(B):
    // - "collapses internal whitespace runs to single spaces" (whitespace includes \t \n \r)
    // - "strips control characters" (non-whitespace control chars like NUL, BEL, etc.)
    let normalized: String = query
        .nfc() // NFC normalize unicode
        .filter_map(|c| {
            if c.is_whitespace() {
                // Convert all Unicode whitespace (including \t \n \r) to ASCII space
                Some(' ')
            } else if c.is_control() {
                // Strip non-whitespace control characters (NUL, BEL, BS, etc.)
                None
            } else {
                Some(c)
            }
        })
        .collect();

    // Step 2: Apply Unicode casefold (spec: "lowercases using Unicode casefold")
    let casefolded = default_case_fold_str(&normalized);

    // Step 3: Deterministic ASCII-only whitespace collapse
    let mut result = String::with_capacity(casefolded.len());
    let mut prev_was_space = true; // Starts true to trim leading whitespace

    for c in casefolded.chars() {
        if is_ascii_ws(c) {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    // Trim trailing space if present
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

// ============================================================================
// RetrievalCandidate Types (§2.6.6.7.14.5)
// ============================================================================

/// Kind of candidate reference
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    SourceRef,
    EntityRef,
    ArtifactHandle,
}

/// Scores for a retrieval candidate
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CandidateScores {
    /// Lexical (BM25/TF-IDF) score
    pub lexical: Option<f64>,
    /// Vector similarity score
    pub vector: Option<f64>,
    /// Graph traversal score
    pub graph: Option<f64>,
    /// ContextPack score (1.0 for fresh, 0.0 otherwise)
    pub pack: Option<f64>,
    /// Trust adjustment factor
    pub trust_adjust: Option<f64>,
}

impl CandidateScores {
    /// Compute base score per §2.6.6.7.14.6(D)
    /// base_score = pack_score + trust_adjust + max(lexical, vector, graph)
    pub fn compute_base_score(&self) -> f64 {
        let pack = self.pack.unwrap_or(0.0);
        let trust = self.trust_adjust.unwrap_or(0.0);
        let max_retrieval = self
            .lexical
            .unwrap_or(0.0)
            .max(self.vector.unwrap_or(0.0))
            .max(self.graph.unwrap_or(0.0));
        pack + trust + max_retrieval
    }
}

/// Union type for candidate references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CandidateRef {
    Source(SourceRef),
    Entity(EntityRef),
    Artifact(ArtifactHandle),
}

impl CandidateRef {
    /// Get canonical ID string for tie-breaking
    pub fn canonical_id(&self) -> String {
        match self {
            CandidateRef::Source(r) => r.canonical_id(),
            CandidateRef::Entity(r) => r.canonical_id(),
            CandidateRef::Artifact(r) => r.canonical_id(),
        }
    }
}

/// A candidate retrieved during search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    /// Stable identifier for this candidate
    pub candidate_id: String,
    /// Kind of reference
    pub kind: CandidateKind,
    /// The actual reference
    pub candidate_ref: CandidateRef,
    /// Content tier classification (None = unknown/unclassified; default-deny in SFW)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_tier: Option<ContentTier>,
    /// Which store produced this candidate
    pub store: StoreKind,
    /// Individual score components
    pub scores: CandidateScores,
    /// Computed base score (deterministic)
    pub base_score: f64,
    /// Stable tie-break key (canonical_id of ref)
    pub tiebreak: String,
}

impl RetrievalCandidate {
    /// Create a new candidate from a source reference
    pub fn from_source(source: SourceRef, store: StoreKind, scores: CandidateScores) -> Self {
        let tiebreak = source.canonical_id();
        let base_score = scores.compute_base_score();
        Self {
            candidate_id: Uuid::new_v4().to_string(),
            kind: CandidateKind::SourceRef,
            candidate_ref: CandidateRef::Source(source),
            content_tier: None,
            store,
            scores,
            base_score,
            tiebreak,
        }
    }

    /// Create a new candidate from an entity reference
    pub fn from_entity(entity: EntityRef, store: StoreKind, scores: CandidateScores) -> Self {
        let tiebreak = entity.canonical_id();
        let base_score = scores.compute_base_score();
        Self {
            candidate_id: Uuid::new_v4().to_string(),
            kind: CandidateKind::EntityRef,
            candidate_ref: CandidateRef::Entity(entity),
            content_tier: None,
            store,
            scores,
            base_score,
            tiebreak,
        }
    }

    /// Create a new candidate from an artifact handle
    pub fn from_artifact(
        artifact: ArtifactHandle,
        store: StoreKind,
        scores: CandidateScores,
    ) -> Self {
        let tiebreak = artifact.canonical_id();
        let base_score = scores.compute_base_score();
        Self {
            candidate_id: Uuid::new_v4().to_string(),
            kind: CandidateKind::ArtifactHandle,
            candidate_ref: CandidateRef::Artifact(artifact),
            content_tier: None,
            store,
            scores,
            base_score,
            tiebreak,
        }
    }
}

// ============================================================================
// RetrievalTrace Types (§2.6.6.7.14.5)
// ============================================================================

/// Record of a route step taken during retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTaken {
    /// Which store was queried
    pub store: StoreKind,
    /// Reason for taking this route
    pub reason: String,
    /// Whether this was a cache hit
    pub cache_hit: Option<bool>,
}

/// Reranking metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RerankInfo {
    /// Whether reranking was used
    pub used: bool,
    /// Reranking method identifier
    pub method: String,
    /// Hash of rerank inputs (candidate list)
    pub inputs_hash: Hash,
    /// Hash of rerank outputs (ordered list)
    pub outputs_hash: Hash,
}

/// Diversity/de-duplication metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiversityInfo {
    /// Whether diversity filtering was used
    pub used: bool,
    /// Diversity method identifier (e.g., "mmr")
    pub method: String,
    /// Lambda parameter for MMR
    pub lambda: Option<f64>,
}

/// A selected evidence item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedEvidence {
    /// Reference to the selected candidate
    pub candidate_ref: CandidateRef,
    /// Final rank after reranking/diversity
    pub final_rank: u32,
    /// Final score
    pub final_score: f64,
    /// Reason for selection
    pub why: String,
}

/// An extracted span from a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanExtraction {
    /// Reference to the source
    pub source_ref: SourceRef,
    /// Selector used to extract the span
    pub selector: String,
    /// Start offset in characters
    pub start: u32,
    /// End offset in characters
    pub end: u32,
    /// Estimated token count
    pub token_estimate: u32,
}

/// Complete trace of a retrieval operation (§2.6.6.7.14.5)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalTrace {
    /// Unique identifier for this trace
    pub trace_id: Uuid,
    /// Reference to the query plan used
    pub query_plan_id: Uuid,
    /// Normalized query hash for caching
    pub normalized_query_hash: Hash,
    /// Routes taken during retrieval
    pub route_taken: Vec<RouteTaken>,
    /// All candidates considered
    pub candidates: Vec<RetrievalCandidate>,
    /// Reranking metadata
    pub rerank: RerankInfo,
    /// Diversity metadata
    pub diversity: DiversityInfo,
    /// Final selected evidence
    pub selected: Vec<SelectedEvidence>,
    /// Extracted spans
    pub spans: Vec<SpanExtraction>,
    /// Budgets that were applied
    pub budgets_applied: RetrievalBudgets,
    /// Filters that were applied (must include view_mode for trace auditability)
    #[serde(default)]
    pub filters_applied: RetrievalFilters,
    /// Projection markers (spec Addendum 11.3)
    #[serde(default)]
    pub projection_applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_kind: Option<ProjectionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_ruleset_id: Option<String>,
    /// Flags for truncated content
    pub truncation_flags: Vec<String>,
    /// Non-fatal warnings
    pub warnings: Vec<String>,
    /// Fatal errors (retrieval may have degraded)
    pub errors: Vec<String>,
}

impl RetrievalTrace {
    /// Create a new trace for a query plan
    pub fn new(query_plan: &QueryPlan) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            query_plan_id: query_plan.plan_id,
            normalized_query_hash: query_plan.compute_normalized_query_hash(),
            route_taken: Vec::new(),
            candidates: Vec::new(),
            rerank: RerankInfo::default(),
            diversity: DiversityInfo::default(),
            selected: Vec::new(),
            spans: Vec::new(),
            budgets_applied: query_plan.budgets.clone(),
            filters_applied: query_plan.filters.clone(),
            projection_applied: false,
            projection_kind: None,
            projection_ruleset_id: None,
            truncation_flags: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Enforce ViewMode semantics for strict SFW output (spec 11.2/11.3).
    ///
    /// - In `ViewMode::Sfw`, strict-drop any candidate/selected/span that is not `content_tier=sfw`.
    /// - `content_tier=None` is treated as non-sfw (default-deny) and dropped.
    /// - Projection is non-destructive: evidence links for remaining items are preserved.
    /// - When SFW is active, projection labeling fields are set (spec Addendum 11.3).
    pub fn apply_view_mode_hard_drop(&mut self) {
        if self.filters_applied.view_mode != ViewMode::Sfw {
            return;
        }

        self.projection_applied = true;
        self.projection_kind = Some(ProjectionKind::Sfw);
        self.projection_ruleset_id = Some("viewmode_sfw_hard_drop@v1".to_string());

        let before_candidates = self.candidates.len();
        self.candidates
            .retain(|c| c.content_tier == Some(ContentTier::Sfw));

        let allowed_candidate_refs: std::collections::HashSet<String> = self
            .candidates
            .iter()
            .map(|c| c.candidate_ref.canonical_id())
            .collect();

        let allowed_source_refs: std::collections::HashSet<String> = self
            .candidates
            .iter()
            .filter_map(|c| match &c.candidate_ref {
                CandidateRef::Source(s) => Some(s.canonical_id()),
                _ => None,
            })
            .collect();

        let before_selected = self.selected.len();
        self.selected.retain(|s| {
            let key = s.candidate_ref.canonical_id();
            allowed_candidate_refs.contains(&key)
        });

        let before_spans = self.spans.len();
        self.spans
            .retain(|s| allowed_source_refs.contains(&s.source_ref.canonical_id()));

        let dropped_candidates = before_candidates.saturating_sub(self.candidates.len());
        let dropped_selected = before_selected.saturating_sub(self.selected.len());
        let dropped_spans = before_spans.saturating_sub(self.spans.len());

        if dropped_candidates > 0 || dropped_selected > 0 || dropped_spans > 0 {
            self.warnings.push(format!(
                "view_mode_sfw_hard_drop:dropped_candidates={dropped_candidates},dropped_selected={dropped_selected},dropped_spans={dropped_spans}"
            ));
        }
    }

    /// Get total token estimate for all spans
    pub fn total_span_tokens(&self) -> u32 {
        self.spans.iter().map(|s| s.token_estimate).sum()
    }

    /// Get total selected evidence count
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    /// Count snippets per source (for budget enforcement)
    pub fn snippets_per_source(&self) -> std::collections::HashMap<Uuid, u32> {
        let mut counts = std::collections::HashMap::new();
        for span in &self.spans {
            *counts.entry(span.source_ref.source_id).or_insert(0) += 1;
        }
        counts
    }

    /// Check if any span exceeds max_read_tokens without truncation flag
    pub fn find_untruncated_oversized_spans(&self, max_tokens: u32) -> Vec<&SpanExtraction> {
        self.spans
            .iter()
            .filter(|span| {
                span.token_estimate > max_tokens
                    && !self
                        .truncation_flags
                        .iter()
                        .any(|f| f.contains(&span.source_ref.source_id.to_string()))
            })
            .collect()
    }
}

// ============================================================================
// CacheKey Types (§2.6.6.7.14.9)
// ============================================================================

/// Kind of cacheable artifact
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheKind {
    RetrievalCandidates,
    RerankOrder,
    Spans,
    PromptEnvelope,
    ContextSnapshot,
}

/// Cache key for retrieval artifacts (§2.6.6.7.14.9)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheKey {
    /// Kind of cached artifact
    pub cache_kind: CacheKind,
    /// Determinism mode
    pub determinism_mode: DeterminismMode,
    /// Policy identifier
    pub policy_id: String,
    /// Hash of normalized query
    pub query_hash: Hash,
    /// Hash of scope inputs
    pub scope_inputs_hash: Hash,
    /// Hash of budgets
    pub budgets_hash: Hash,
    /// Hash of filters
    pub filters_hash: Hash,
    /// Hash of toolchain (tool_id, tool_version, config_hash)
    pub toolchain_hash: Hash,
    /// Hash of sources (sorted source_id, source_hash pairs)
    pub sources_hash: Hash,
}

impl CacheKey {
    /// Create a cache key from a query plan
    pub fn from_plan(
        plan: &QueryPlan,
        cache_kind: CacheKind,
        scope_inputs_hash: Hash,
        toolchain_hash: Hash,
        sources_hash: Hash,
    ) -> Self {
        Self {
            cache_kind,
            determinism_mode: plan.determinism_mode,
            policy_id: plan.policy_id.clone(),
            query_hash: plan.compute_normalized_query_hash(),
            scope_inputs_hash,
            budgets_hash: plan.budgets.compute_hash(),
            filters_hash: plan.filters.compute_hash(),
            toolchain_hash,
            sources_hash,
        }
    }

    /// Compute a single hash representing this cache key
    pub fn compute_full_hash(&self) -> Hash {
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }
}

// ============================================================================
// ContextPack Types (§2.6.6.7.14.7) - foundational, extended in separate WP
// ============================================================================

/// Record of a built context pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackRecord {
    /// Unique pack identifier
    pub pack_id: Uuid,
    /// Target source at build time
    pub target: SourceRef,
    /// Handle to the pack artifact
    pub pack_artifact: ArtifactHandle,
    /// Hashes of underlying sources at build time (spec §2.6.6.7.14.7).
    #[serde(default)]
    pub source_hashes: Vec<Hash>,
    /// Underlying sources at build time (sorted by source_id for determinism)
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    /// When the pack was created
    pub created_at: DateTime<Utc>,
    /// Builder metadata
    pub builder: ContextPackBuilder,
    /// Canonical JSON hash of the payload artifact
    pub payload_hash: Hash,
    /// Pack version
    pub version: u32,
}

/// Builder metadata for a context pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackBuilder {
    pub tool_id: String,
    pub tool_version: String,
    pub config_hash: Hash,
}

impl ContextPackRecord {
    /// Check if the pack is stale by comparing source hashes
    pub fn is_stale(&self, current_sources: &[SourceRef]) -> bool {
        // Preferred path: compare (source_id -> source_hash) deterministically.
        if !self.source_refs.is_empty() {
            if self.source_refs.len() != current_sources.len() {
                return true;
            }

            let mut old_sorted: Vec<&SourceRef> = self.source_refs.iter().collect();
            old_sorted.sort_by_key(|s| s.source_id);

            let mut current_sorted: Vec<&SourceRef> = current_sources.iter().collect();
            current_sorted.sort_by_key(|s| s.source_id);

            for (old, current) in old_sorted.iter().zip(current_sorted.iter()) {
                if old.source_id != current.source_id || old.source_hash != current.source_hash {
                    return true;
                }
            }

            // Enforce record's source_hashes[] (spec field) if present.
            if !self.source_hashes.is_empty() {
                let expected_hashes: Vec<Hash> = old_sorted
                    .iter()
                    .map(|s| (*s).source_hash.clone())
                    .collect();
                if expected_hashes != self.source_hashes {
                    return true;
                }
            }

            return false;
        }

        // Hash-only fallback (supports legacy records) - safe only for single-source packs.
        if self.source_hashes.len() == 1 && current_sources.len() == 1 {
            return self.source_hashes[0] != current_sources[0].source_hash;
        }

        true
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextPackConstraintSeverity {
    Hard,
    Soft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackFactV1 {
    pub fact_id: String,
    pub text: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackConstraintV1 {
    pub constraint_id: String,
    pub text: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    pub severity: ContextPackConstraintSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackOpenLoopV1 {
    pub loop_id: String,
    pub question: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackAnchorV1 {
    pub anchor_id: String,
    pub source_ref: SourceRef,
    pub excerpt_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackCoverageV1 {
    pub scanned_selectors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_selectors: Option<Vec<String>>,
}

/// Canonical JSON payload persisted in `pack_artifact` (spec §2.6.6.7.14.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackPayloadV1 {
    pub synopsis: String,
    #[serde(default)]
    pub facts: Vec<ContextPackFactV1>,
    #[serde(default)]
    pub constraints: Vec<ContextPackConstraintV1>,
    #[serde(default)]
    pub open_loops: Vec<ContextPackOpenLoopV1>,
    #[serde(default)]
    pub anchors: Vec<ContextPackAnchorV1>,
    pub coverage: ContextPackCoverageV1,
}

impl ContextPackPayloadV1 {
    /// Enforce provenance binding per spec §2.6.6.7.14.7.
    /// - facts: missing SourceRefs => confidence=0 (downgrade)
    /// - constraints/open_loops: missing SourceRefs => dropped
    pub fn enforce_provenance_binding(&mut self) {
        for fact in &mut self.facts {
            if fact.source_refs.is_empty() {
                fact.confidence = 0.0;
            }
        }
        self.constraints.retain(|c| !c.source_refs.is_empty());
        self.open_loops.retain(|l| !l.source_refs.is_empty());
    }

    pub fn compute_payload_hash(&self) -> Result<Hash, serde_json::Error> {
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut h = Sha256::new();
        h.update(&bytes);
        Ok(hex::encode(h.finalize()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackFreshnessPolicyV1 {
    pub regen_allowed: bool,
    pub regen_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextPackFreshnessDecision {
    Fresh {
        pack_id: Uuid,
    },
    Stale {
        pack_id: Uuid,
        reason: String,
    },
    Regenerated {
        old_pack_id: Uuid,
        new_pack_id: Uuid,
    },
}

// ============================================================================
// Re-exports
// ============================================================================

pub use validators::{
    // Core trait and pipeline
    AceRuntimeValidator,
    // Flight Recorder logging types (§2.6.6.7.14.12)
    AceValidationPayload,
    // New 8 security guards (§2.6.6.7.11.1-8)
    ArtifactHandleOnlyGuard,
    // Original 4 guards (§2.6.6.7.14.11)
    CacheKeyGuard,
    CacheMarker,
    CloudLeakageGuard,
    CompactionSchemaGuard,
    // Content-aware validation types [HSK-ACE-VAL-100]
    ContentClassification,
    ContentResolver,
    ContextDeterminismGuard,
    ContextPackFreshnessGuard,
    IndexDriftGuard,
    JobBoundaryRoutingGuard,
    LocalPayloadGuard,
    MemoryPromotionGuard,
    PromptInjectionGuard,
    ResolvedSnippet,
    RetrievalBudgetGuard,
    SecurityValidationResult,
    SecurityViolation,
    SecurityViolationType,
    SensitivityLevel,
    ValidatorPipeline,
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// T-ACE-RAG-001: Query normalization determinism
    /// Same input string variations (whitespace/unicode) MUST yield identical normalized_query_hash
    #[test]
    fn test_query_normalization_determinism() {
        let variations = vec![
            "  What is the capital of France?  ",
            "what is the capital of france?",
            "WHAT IS THE CAPITAL OF FRANCE?",
            "What  is   the    capital     of      France?",
            "What\tis\nthe\r\ncapital of France?",
        ];

        let expected = normalize_query(variations[0]);
        for variant in &variations {
            let normalized = normalize_query(variant);
            assert_eq!(
                normalized, expected,
                "Normalization mismatch: {:?} -> {:?} (expected {:?})",
                variant, normalized, expected
            );
        }

        // All should produce the same hash
        let plan1 = QueryPlan::new(
            variations[0].to_string(),
            QueryKind::FactLookup,
            "test".to_string(),
        );
        let plan2 = QueryPlan::new(
            variations[1].to_string(),
            QueryKind::FactLookup,
            "test".to_string(),
        );
        assert_eq!(
            plan1.compute_normalized_query_hash(),
            plan2.compute_normalized_query_hash()
        );
    }

    /// T-ACE-RAG-001b: Unicode casefold correctness
    /// Validates that normalize_query uses proper Unicode casefold, not just to_lowercase.
    /// Key difference: casefold converts ß → ss, to_lowercase keeps ß as ß.
    #[test]
    fn test_unicode_casefold_correctness() {
        // German ß (Eszett) should casefold to "ss"
        let with_eszett = "Straße"; // "Street" in German
        let with_ss = "strasse";
        assert_eq!(
            normalize_query(with_eszett),
            normalize_query(with_ss),
            "Unicode casefold must convert ß to ss (ß should equal ss)"
        );

        // Verify ß explicitly becomes ss
        let normalized = normalize_query("ß");
        assert_eq!(normalized, "ss", "ß must casefold to ss");

        // Turkish dotless i (ı) and dotted I (İ) - casefold behavior
        // Standard casefold: İ → i̇ (with combining dot), ı → ı
        // (Note: actual behavior depends on caseless crate implementation)
        let turkish_i = normalize_query("I");
        assert_eq!(turkish_i, "i", "ASCII I should casefold to i");

        // Control characters should be stripped (not converted to space)
        let with_control = "hello\x00\x07\x08world"; // NUL, BEL, BS
        let without_control = "helloworld";
        assert_eq!(
            normalize_query(with_control),
            normalize_query(without_control),
            "Non-whitespace control characters must be stripped"
        );

        // Whitespace control chars (tab, newline) should become spaces then collapse
        let with_ws_control = "hello\t\n\rworld";
        let expected = "hello world";
        assert_eq!(
            normalize_query(with_ws_control),
            expected,
            "Whitespace control chars (\\t \\n \\r) should become space and collapse"
        );
    }

    /// T-ACE-RAG-002: Strict ranking determinism
    /// Under strict mode, identical inputs MUST yield identical candidate order
    #[test]
    fn test_strict_ranking_determinism() {
        let source1 = SourceRef::new(Uuid::nil(), "hash1".to_string());
        let source2 = SourceRef::new(Uuid::from_u128(1), "hash2".to_string());

        let scores1 = CandidateScores {
            lexical: Some(0.8),
            pack: Some(1.0),
            ..Default::default()
        };

        let scores2 = CandidateScores {
            lexical: Some(0.8),
            pack: Some(1.0),
            ..Default::default()
        };

        let candidate1 = RetrievalCandidate::from_source(
            source1.clone(),
            StoreKind::ContextPacks,
            scores1.clone(),
        );
        let candidate2 = RetrievalCandidate::from_source(
            source2.clone(),
            StoreKind::ContextPacks,
            scores2.clone(),
        );

        // Same base scores, tiebreak by canonical_id
        assert_eq!(candidate1.base_score, candidate2.base_score);
        assert!(candidate1.tiebreak < candidate2.tiebreak); // Nil UUID < 00..001

        // Sorting should be deterministic
        let mut candidates = [candidate2.clone(), candidate1.clone()];
        candidates.sort_by(|a, b| {
            b.base_score
                .partial_cmp(&a.base_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.tiebreak.cmp(&b.tiebreak))
        });
        assert_eq!(candidates[0].tiebreak, candidate1.tiebreak);
        assert_eq!(candidates[1].tiebreak, candidate2.tiebreak);
    }

    /// T-ACE-RAG-005: Budget enforcement
    /// Evidence token ceilings and per-source caps MUST never be exceeded
    #[test]
    fn test_budget_validation() {
        let mut budgets = RetrievalBudgets::default();
        assert!(budgets.validate().is_ok());

        // Zero max_total_evidence_tokens should fail
        budgets.max_total_evidence_tokens = 0;
        assert!(budgets.validate().is_err());

        budgets.max_total_evidence_tokens = 4000;
        budgets.max_read_tokens = 0;
        assert!(budgets.validate().is_err());
    }

    #[test]
    fn test_cache_key_hashing() {
        let plan = QueryPlan::new(
            "test query".to_string(),
            QueryKind::FactLookup,
            "policy1".to_string(),
        );
        let key1 = CacheKey::from_plan(
            &plan,
            CacheKind::RetrievalCandidates,
            "scope1".to_string(),
            "toolchain1".to_string(),
            "sources1".to_string(),
        );
        let key2 = CacheKey::from_plan(
            &plan,
            CacheKind::RetrievalCandidates,
            "scope1".to_string(),
            "toolchain1".to_string(),
            "sources1".to_string(),
        );

        // Same inputs -> same hash
        assert_eq!(key1.compute_full_hash(), key2.compute_full_hash());

        // Different inputs -> different hash
        let key3 = CacheKey::from_plan(
            &plan,
            CacheKind::RetrievalCandidates,
            "scope2".to_string(), // Different!
            "toolchain1".to_string(),
            "sources1".to_string(),
        );
        assert_ne!(key1.compute_full_hash(), key3.compute_full_hash());
    }

    #[test]
    fn test_retrieval_trace_metrics() {
        let plan = QueryPlan::new(
            "test".to_string(),
            QueryKind::FactLookup,
            "policy".to_string(),
        );
        let mut trace = RetrievalTrace::new(&plan);

        let source = SourceRef::new(Uuid::new_v4(), "hash".to_string());
        trace.spans.push(SpanExtraction {
            source_ref: source.clone(),
            selector: "test".to_string(),
            start: 0,
            end: 100,
            token_estimate: 50,
        });
        trace.spans.push(SpanExtraction {
            source_ref: source.clone(),
            selector: "test2".to_string(),
            start: 100,
            end: 200,
            token_estimate: 75,
        });

        assert_eq!(trace.total_span_tokens(), 125);
        let per_source = trace.snippets_per_source();
        assert_eq!(per_source.get(&source.source_id), Some(&2));
    }

    #[test]
    fn test_context_pack_staleness() {
        let source1 = SourceRef::new(Uuid::from_u128(1), "hash1".to_string());
        let source2 = SourceRef::new(Uuid::from_u128(2), "hash2".to_string());
        let pack = ContextPackRecord {
            pack_id: Uuid::new_v4(),
            target: source1.clone(),
            pack_artifact: ArtifactHandle::new(Uuid::new_v4(), "/path".to_string()),
            source_hashes: vec![source1.source_hash.clone(), source2.source_hash.clone()],
            source_refs: vec![source2.clone(), source1.clone()],
            created_at: Utc::now(),
            builder: ContextPackBuilder {
                tool_id: "builder".to_string(),
                tool_version: "1.0".to_string(),
                config_hash: "config".to_string(),
            },
            payload_hash: "payload_hash".to_string(),
            version: 1,
        };

        // Same hashes -> not stale
        assert!(!pack.is_stale(&[source1.clone(), source2.clone()]));

        // Different hashes -> stale
        assert!(pack.is_stale(&[
            source1.clone(),
            SourceRef::new(source2.source_id, "hash3".to_string()),
        ]));

        // Different count -> stale
        assert!(pack.is_stale(&[source1.clone()]));
    }

    #[test]
    fn test_context_pack_provenance_binding_enforcement() {
        let source_ref = SourceRef::new(Uuid::new_v4(), "hash".to_string());

        let mut payload = ContextPackPayloadV1 {
            synopsis: "synopsis".to_string(),
            facts: vec![ContextPackFactV1 {
                fact_id: "fact-1".to_string(),
                text: "text".to_string(),
                source_refs: Vec::new(),
                confidence: 1.0,
            }],
            constraints: vec![ContextPackConstraintV1 {
                constraint_id: "c-1".to_string(),
                text: "must".to_string(),
                source_refs: Vec::new(),
                severity: ContextPackConstraintSeverity::Hard,
            }],
            open_loops: vec![ContextPackOpenLoopV1 {
                loop_id: "l-1".to_string(),
                question: "q?".to_string(),
                source_refs: Vec::new(),
            }],
            anchors: vec![ContextPackAnchorV1 {
                anchor_id: "a-1".to_string(),
                source_ref: source_ref.clone(),
                excerpt_hint: "hint".to_string(),
            }],
            coverage: ContextPackCoverageV1 {
                scanned_selectors: vec!["sel-1".to_string()],
                skipped_selectors: None,
            },
        };

        payload.enforce_provenance_binding();

        assert_eq!(payload.facts.len(), 1);
        assert_eq!(payload.facts[0].confidence, 0.0);
        assert!(payload.constraints.is_empty());
        assert!(payload.open_loops.is_empty());
    }

    /// T-ACE-RAG-003: Replay persistence correctness
    /// Under replay mode, replay MUST re-use persisted candidate list + rerank order
    /// and produce identical selected ids/hashes.
    #[test]
    fn test_replay_persistence_correctness() -> Result<(), Box<dyn std::error::Error>> {
        use sha2::{Digest, Sha256};

        // Helper to compute trace hash
        fn compute_trace_hash(trace: &RetrievalTrace) -> Result<String, serde_json::Error> {
            let json = serde_json::to_string(trace)?;
            let mut hasher = Sha256::new();
            hasher.update(json.as_bytes());
            Ok(hex::encode(hasher.finalize()))
        }

        // 1. Create QueryPlan in Replay mode
        let mut plan = QueryPlan::new(
            "test replay query".to_string(),
            QueryKind::FactLookup,
            "replay_policy".to_string(),
        );
        plan.determinism_mode = DeterminismMode::Replay;

        // 2. Build RetrievalTrace with candidates and rerank info
        let mut trace = RetrievalTrace::new(&plan);

        // Use fixed UUIDs for deterministic testing (infallible from_u128)
        let uuid1 = Uuid::from_u128(1);
        let uuid2 = Uuid::from_u128(2);

        let source1 = SourceRef::new(uuid1, "hash_source1".to_string());
        let source2 = SourceRef::new(uuid2, "hash_source2".to_string());

        let scores1 = CandidateScores {
            lexical: Some(0.9),
            pack: Some(1.0),
            ..Default::default()
        };
        let scores2 = CandidateScores {
            lexical: Some(0.8),
            pack: Some(1.0),
            ..Default::default()
        };

        // Add candidates with deterministic IDs (override the random UUIDs)
        let mut candidate1 =
            RetrievalCandidate::from_source(source1.clone(), StoreKind::ContextPacks, scores1);
        candidate1.candidate_id = "candidate_001".to_string();

        let mut candidate2 =
            RetrievalCandidate::from_source(source2.clone(), StoreKind::ContextPacks, scores2);
        candidate2.candidate_id = "candidate_002".to_string();

        trace.candidates.push(candidate1);
        trace.candidates.push(candidate2);

        // Add selected evidence
        trace.selected.push(SelectedEvidence {
            candidate_ref: CandidateRef::Source(source1.clone()),
            final_rank: 0,
            final_score: 1.9,
            why: "highest_combined_score".to_string(),
        });
        trace.selected.push(SelectedEvidence {
            candidate_ref: CandidateRef::Source(source2.clone()),
            final_rank: 1,
            final_score: 1.8,
            why: "second_highest_score".to_string(),
        });

        // Add rerank metadata (per §2.6.6.7.14.6(E) - replay mode persists rerank order)
        trace.rerank = RerankInfo {
            used: true,
            method: "cross_encoder_v1".to_string(),
            inputs_hash: "rerank_inputs_hash_abc123".to_string(),
            outputs_hash: "rerank_outputs_hash_def456".to_string(),
        };

        // Add diversity metadata
        trace.diversity = DiversityInfo {
            used: true,
            method: "mmr".to_string(),
            lambda: Some(0.7),
        };

        // 3. Capture original state before serialization (persistence)
        let original_candidate_ids: Vec<_> = trace
            .candidates
            .iter()
            .map(|c| c.candidate_id.clone())
            .collect();
        let mut original_selected_refs = Vec::new();
        for s in &trace.selected {
            original_selected_refs.push(serde_json::to_string(&s.candidate_ref)?);
        }
        let original_rerank_inputs = trace.rerank.inputs_hash.clone();
        let original_rerank_outputs = trace.rerank.outputs_hash.clone();
        let original_trace_hash = compute_trace_hash(&trace)?;

        // 4. Serialize (simulating persistence to Flight Recorder / storage)
        let serialized = serde_json::to_string(&trace)?;

        // 5. Deserialize (simulating replay load)
        let replayed: RetrievalTrace = serde_json::from_str(&serialized)?;

        // 6. Verify candidate IDs are IDENTICAL
        let replayed_candidate_ids: Vec<_> = replayed
            .candidates
            .iter()
            .map(|c| c.candidate_id.clone())
            .collect();
        assert_eq!(
            original_candidate_ids, replayed_candidate_ids,
            "T-ACE-RAG-003: Candidate IDs must be identical after replay"
        );

        // 7. Verify selected evidence refs are IDENTICAL
        let mut replayed_selected_refs = Vec::new();
        for s in &replayed.selected {
            replayed_selected_refs.push(serde_json::to_string(&s.candidate_ref)?);
        }
        assert_eq!(
            original_selected_refs, replayed_selected_refs,
            "T-ACE-RAG-003: Selected evidence refs must be identical after replay"
        );

        // 8. Verify rerank hashes are IDENTICAL (per §2.6.6.7.14.6(E))
        assert_eq!(
            original_rerank_inputs, replayed.rerank.inputs_hash,
            "T-ACE-RAG-003: Rerank inputs_hash must be identical after replay"
        );
        assert_eq!(
            original_rerank_outputs, replayed.rerank.outputs_hash,
            "T-ACE-RAG-003: Rerank outputs_hash must be identical after replay"
        );

        // 9. Verify full trace hash is IDENTICAL
        let replayed_trace_hash = compute_trace_hash(&replayed)?;
        assert_eq!(
            original_trace_hash, replayed_trace_hash,
            "T-ACE-RAG-003: Full trace hash must be identical after replay"
        );

        // 10. Verify determinism mode is preserved
        assert_eq!(
            replayed.query_plan_id, trace.query_plan_id,
            "T-ACE-RAG-003: Query plan ID must be preserved"
        );

        Ok(())
    }
}
