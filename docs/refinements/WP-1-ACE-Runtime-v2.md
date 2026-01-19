## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, you MUST include REASON_NO_ENRICHMENT and SPEC_EXCERPTS for every anchor.
- If ENRICHMENT_NEEDED=YES, you MUST include the full Proposed Spec Enrichment text (verbatim Markdown) that could be copy-pasted into the Master Spec.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-ACE-Runtime-v2
- CREATED_AT: 2026-01-18
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- SPEC_TARGET_SHA1: CF2F5305FC8EEC517D577D87365BD9C072A99B0F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja180120261659
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-ACE-Runtime-v2

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (ACE-RAG-001 requirements are present in Master Spec Main Body; this is remediation/revalidation work due to prior packet/spec drift).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Spec requires logging to Flight Recorder for each retrieval-backed model call (QueryPlan id+hash, RetrievalTrace id+hash, normalized_query_hash, cache hits/misses, rerank/diversity metadata).
- No explicit Flight Recorder event IDs are enumerated in the anchored text; implement using the existing logging/event plumbing for model calls/retrieval as appropriate.

### RED_TEAM_ADVISORY (security failure modes)
- If query normalization is not deterministic, `normalized_query_hash` becomes non-replayable and retrieval caching/drift detection can be bypassed or become flaky.
- If budgets are not enforced strictly, retrieval can cause prompt stuffing / token exhaustion and can leak more context than intended.
- If validators/guards are not applied to every retrieval-backed call, hidden retrieval paths can bypass budget/freshness/drift invariants.
- If RetrievalTrace does not persist route/candidates/selection inputs, replay mode becomes unverifiable and auditing is hollow.
- If cache keys are not validated in strict mode, stale/incorrect evidence can be reused without detection.

### PRIMITIVES (traits/structs/enums)
- `QueryPlan` (Derived schema)
- `RetrievalTrace` (Derived schema)
- `AceRuntimeValidator` (trait)
- Guard implementations (minimum): `RetrievalBudgetGuard`, `ContextPackFreshnessGuard`, `IndexDriftGuard`, `CacheKeyGuard`
- Determinism primitives: `normalized_query_hash`, determinism mode `(strict|replay)`

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: The spec provides explicit schemas, deterministic normalization algorithm requirements, a concrete validator trait, required guards, logging requirements, and named conformance tests (T-ACE-RAG-001..007).
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.113 already defines ACE-RAG-001 requirements (schemas, algorithms, validator trait, logging requirements, and conformance tests).

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.5 (New typed objects)
- CONTEXT_START_LINE: 6854
- CONTEXT_END_LINE: 6951
- CONTEXT_TOKEN: QueryPlan (Derived)
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.5 New typed objects (schemas)

  ```text
  QueryPlan (Derived)
  - plan_id: UUID
  - created_at: Timestamp
  - query_text: string
  - query_kind: (fact_lookup | summarize | compare | transform | export | unknown)
  - route[]: RouteStep
  - budgets: RetrievalBudgets
  - filters: RetrievalFilters
  - determinism_mode: (strict | replay)
  - policy_id: string
  - version: int

  RouteStep
  - store: (context_packs | knowledge_graph | shadow_ws_lexical | shadow_ws_vector | local_web_cache | bounded_read_only)
  - purpose: string
  - max_candidates: int
  - required: bool

  RetrievalBudgets
  - max_total_evidence_tokens: int
  - max_snippets_total: int
  - max_snippets_per_source: int
  - max_candidates_total: int
  - max_read_tokens: int
  - max_tool_calls: int
  - max_rerank_candidates: int
  - tool_delta_inline_char_limit: int

  RetrievalTrace (Derived; persisted per model call)
  - trace_id: UUID
  - query_plan_id: UUID
  - normalized_query_hash: Hash
  - route_taken[]: {store, reason, cache_hit?: bool}
  - candidates[]: RetrievalCandidate
  - rerank: {used: bool, method: string, inputs_hash: Hash, outputs_hash: Hash}
  - diversity: {used: bool, method: string, lambda?: float}
  - selected[]: {ref, final_rank: int, final_score: float, why: string}
  - spans[]: {ref: SourceRef, selector: string, start: int, end: int, token_estimate: int}
  - budgets_applied: RetrievalBudgets
  - truncation_flags[]?: string
  - warnings[]?: string
  - errors[]?: string
  ```
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.6(B) (Query normalization; deterministic)
- CONTEXT_START_LINE: 6955
- CONTEXT_END_LINE: 6972
- CONTEXT_TOKEN: normalized_query_hash = sha256(normalize(query_text))
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.6 Required behavior (normative algorithms)

  **B) Query normalization (deterministic)**
  The runtime MUST compute `normalized_query_hash = sha256(normalize(query_text))`, where `normalize()`:
  - trims leading/trailing whitespace,
  - collapses internal whitespace runs to single spaces,
  - NFC normalizes unicode,
  - lowercases using Unicode casefold,
  - strips control characters.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.11 (Validators; required guards)
- CONTEXT_START_LINE: 7125
- CONTEXT_END_LINE: 7164
- CONTEXT_TOKEN: pub trait AceRuntimeValidator
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.11 Validators (Normative Traits)

  The runtime MUST implement the `AceRuntimeValidator` trait. All retrieval operations MUST be validated by a pipeline of these guards.

  ```rust
  /// HSK-TRAIT-002: ACE Runtime Validator
  #[async_trait]
  pub trait AceRuntimeValidator: Send + Sync {
      fn name(&self) -> &str;
      async fn validate_plan(&self, plan: &QueryPlan) -> Result<(), AceError>;
      async fn validate_trace(&self, trace: &RetrievalTrace) -> Result<(), AceError>;
  }
  ```

  **Required Implementations:**
  1) **RetrievalBudgetGuard**
  2) **ContextPackFreshnessGuard**
  3) **IndexDriftGuard**
  4) **CacheKeyGuard**
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.12 (Logging requirements)
- CONTEXT_START_LINE: 7167
- CONTEXT_END_LINE: 7174
- CONTEXT_TOKEN: MUST log to Flight Recorder
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.12 Logging requirements (extends \\u00A72.6.6.7.12)

  For each retrieval-backed model call, the runtime MUST log to Flight Recorder:
  - `QueryPlan` (id + hash)
  - `normalized_query_hash`
  - `RetrievalTrace` (id + hash)
  - cache hits/misses per stage
  - rerank metadata (method + inputs_hash + outputs_hash)
  - diversity metadata (method + lambda)
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.113.md 2.6.6.7.14.13 (Conformance tests)
- CONTEXT_START_LINE: 7181
- CONTEXT_END_LINE: 7202
- CONTEXT_TOKEN: T-ACE-RAG-001
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.13 Conformance tests (minimum set; extends \\u00A72.6.6.7.12)

  T-ACE-RAG-001 Query normalization determinism
  - Same input string variations (whitespace/unicode) MUST yield identical normalized_query_hash.

  T-ACE-RAG-002 Strict ranking determinism
  - Under strict mode, identical inputs MUST yield identical candidate order and selection, including tie-break behavior.

  T-ACE-RAG-003 Replay persistence correctness
  - Under replay mode, replay MUST re-use persisted candidate list + rerank order and produce identical selected ids/hashes.

  T-ACE-RAG-004 ContextPack freshness invalidation
  - If any underlying source_hash changes, a previously built pack MUST be marked stale and MUST NOT receive pack_score=1.0.

  T-ACE-RAG-005 Budget enforcement
  - Evidence token ceilings and per-source caps MUST never be exceeded; truncation MUST be deterministic and logged.

  T-ACE-RAG-006 Drift detection
  - Corrupt an embedding record source_hash and verify IndexDriftGuard triggers (fail or degraded output per policy).

  T-ACE-RAG-007 Cache key invalidation
  - Change any CacheKey component (budgets/policy/toolchain_hash/source hash) and verify cache miss + recomputation.
  ```
