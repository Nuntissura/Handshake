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
- WP_ID: WP-1-AI-Ready-Data-Architecture-v1
- CREATED_AT: 2026-01-25
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.118.md
- SPEC_TARGET_SHA1: eee766ed478513ffa48963c49367ec31ad47fa00
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja250120262250
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-AI-Ready-Data-Architecture-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- GAP-1 (Implementation absent): No repo implementation currently wires AI-Ready Data Architecture (Bronze/Silver/Gold layout, chunking, embeddings, hybrid indexing, validation jobs, and FR-EVT-DATA-001..015 telemetry) end-to-end.
- GAP-2 (Telemetry validation): No existing Flight Recorder DATA event emission/validation exists in code (FR-EVT-DATA-001..015 are Phase 1 scope per spec).

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Data event catalog: FR-EVT-DATA-001..015 MUST be emitted per Section 2.3.14.17.4 and Roadmap Phase 1 remediation callout.
- Validation rules (HARD): Flight Recorder MUST reject malformed DATA events; query text MUST be hashed; embedding vectors MUST NOT be logged (Section 11.5.5).
- Query hashing requirement: normalized_query_hash must be computed as sha256(normalize(query_text)) (Section 2.6.6.7.14.6); FR-EVT-DATA-009 query_hash must follow this privacy posture.

### RED_TEAM_ADVISORY (security failure modes)
- Privacy regression risk: logging plaintext queries or embedding vectors would violate explicit HARD requirements and leak sensitive content via telemetry; ensure schemas + validators make this impossible.
- Data poisoning risk: Bronze is immutable; Silver/Gold are derived and rebuildable. If derived layers are not reproducible/auditable, attackers can introduce untraceable retrieval corruption.
- Capability bypass risk: ingestion/validation/retrieval jobs must respect capability checks and must produce Flight Recorder evidence; otherwise the system can silently exfiltrate or mutate content.

### PRIMITIVES (traits/structs/enums)
- Storage layers: BronzeRecord, SilverRecord, GoldLayerComponents; BronzeId/SilverId/ChunkId identifiers; storage locations workspace/raw, workspace/derived, workspace/indexes, workspace/graph.
- Chunking: ChunkingStrategy, ChunkingParams, BoundaryType; AST-aware code chunking and header-recursive document chunking.
- Embeddings: EmbeddingRecord; EmbeddingModelRegistry; model version compatibility constraints.
- Indexing: VectorIndexConfig (HNSW), KeywordIndexConfig (BM25), HybridQuery (weights + candidate counts).
- Validation: QualitySLOs thresholds; RetrievalQualityMetrics/DataQualityMetrics; ingestion validation job kinds.
- Flight Recorder: FR-EVT-DATA-001..015 (schemas + validation rules), plus normalized_query_hash requirement.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Master Spec v02.117 defines the full AI-Ready Data Architecture requirements in Section 2.3.14 and includes complete FR-EVT-DATA-001..015 schema coverage plus hard validation rules in Section 11.5.5; remaining work is implementation, not spec ambiguity.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Master Spec v02.117 already includes the missing FR-EVT-DATA schemas and the hard validation rules needed to implement this WP deterministically.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14 (AI-Ready Data Architecture)
- CONTEXT_START_LINE: 3252
- CONTEXT_END_LINE: 3274
- CONTEXT_TOKEN: ### 2.3.14 AI-Ready Data Architecture
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.3.14 AI-Ready Data Architecture [ADD v02.115]

  **Spec-ID:** `ai_ready_data_arch_v1`  
  **Version:** v1.0.0  
  **Date:** 2026-01-22  
  **Authority:** Master Spec Main Body (CX-598)  
  **Implements:** Shadow Workspace (\\u00A72.3), Knowledge Graph (\\u00A72.3.7), Storage Traits (\\u00A72.3.13), ACE Runtime (\\u00A72.6.6.7), Model Runtime (\\u00A74), Skill Bank (\\u00A79), Flight Recorder (\\u00A711.5)

  **Abstract**

  This section defines the **AI-Ready Data Architecture** for the Handshake workspace, establishing principles, schemas, algorithms, and validation criteria that ensure all data\\u2014user-created content, ingested external data, and internal system structures\\u2014is optimally structured for consumption by Large Language Models (LLMs), Vision-Language Models (VLMs), AI agents, and mechanical tooling.

  The architecture addresses a fundamental challenge: data structured for traditional software (human UI interaction, CRUD operations, batch analytics) performs poorly when consumed by AI systems. AI requires semantic coherence, rich metadata, relationship awareness, and retrieval-optimized indexing that traditional architectures do not provide.

  **CRITICAL DESIGN PRINCIPLE:** Every piece of data stored in Handshake MUST be retrievable, interpretable, and usable by AI systems without human intervention. Data that cannot be effectively consumed by AI is considered architecturally deficient. This enables the "everything uses everything" force multiplier where all tools and surfaces can access all data.

  ---

  #### 2.3.14.1 Motivation and Scope [ADD v02.115]

  ##### 2.3.14.1.1 Problem Statement

  Traditional software data architectures optimize for:
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.17.1 (MUST Requirements)
- CONTEXT_START_LINE: 4856
- CONTEXT_END_LINE: 4870
- CONTEXT_TOKEN: ##### 2.3.14.17.1 MUST Requirements
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.3.14.17 Conformance Requirements [ADD v02.115]

  ##### 2.3.14.17.1 MUST Requirements

  1. Implement Bronze/Silver/Gold storage layers
  2. Support content-type-aware chunking
  3. Track embedding model versions
  4. Implement hybrid search (vector + keyword)
  5. Enforce metadata completeness
  6. Run validation during ingestion
  7. Log operations to Flight Recorder
  8. Respect capability system
  9. Support core metadata schema
  10. Implement context budget management
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.5 (Storage mapping + Bronze requirements)
- CONTEXT_START_LINE: 3553
- CONTEXT_END_LINE: 3572
- CONTEXT_TOKEN: `workspace/raw/`
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Mapping to Master Spec \\u00A72.3:**

  | Layer | Master Spec Concept | Storage Location |
  |-------|---------------------|------------------|
  | Bronze | RawContent | `workspace/raw/` |
  | Silver | DerivedContent | `workspace/derived/` |
  | Gold | Indexes, Knowledge Graph | `workspace/indexes/`, `workspace/graph/` |

  ##### 2.3.14.5.2 Bronze Layer Specification

  **Purpose:** Preserve original content exactly as created or ingested.

  **Requirements:**

  1. Bronze content MUST NOT be modified after initial storage
  2. Bronze content MUST retain original encoding, formatting, and structure
  3. Bronze content MUST include ingestion metadata
  4. Bronze content MUST be addressable by stable identifier
  5. Bronze content MUST support bulk export for backup/migration
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.5.3 (Silver requirements)
- CONTEXT_START_LINE: 3609
- CONTEXT_END_LINE: 3620
- CONTEXT_TOKEN: ##### 2.3.14.5.3 Silver Layer Specification
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.5.3 Silver Layer Specification

  **Purpose:** Store processed, AI-ready content with embeddings and metadata.

  **Requirements:**

  1. Silver records MUST reference their Bronze source
  2. Silver records MUST include processing metadata (model versions, timestamps)
  3. Silver records MUST be re-generable from Bronze
  4. Silver records SHOULD be updated when processing strategies improve
  5. Silver records MUST include validation status
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.6 (Chunking core requirements + Phase 1 strategies)
- CONTEXT_START_LINE: 3751
- CONTEXT_END_LINE: 3819
- CONTEXT_TOKEN: ##### 2.3.14.6.3 Code Chunking (AST-Aware)
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Core Requirements:**

  1. Chunks MUST be within embedding model token limits
  2. Chunks MUST NOT split semantic units (sentences, functions, paragraphs)
  3. Chunks SHOULD be self-contained (interpretable without external context)
  4. Chunks MUST preserve parent references for context expansion
  5. Chunk boundaries MUST be deterministic (same input \\u2192 same chunks)

  ##### 2.3.14.6.2 Chunking Strategy Selection

  ```typescript
  interface ChunkingStrategy {
    strategy_id: string;
    content_types: ContentType[];
    chunker: Chunker;
    params: ChunkingParams;
  }

  interface ChunkingParams {
    target_size_tokens: number;
    min_size_tokens: number;
    max_size_tokens: number;
    overlap_tokens: number;
    preserve_boundaries: BoundaryType[];
  }

  type BoundaryType = 
    | "sentence" | "paragraph" | "section"
    | "function" | "class" | "module"
    | "page" | "record";
  ```

  ##### 2.3.14.6.3 Code Chunking (AST-Aware)

  **Applicable to:** `code/*` content types

  **Strategy:** Parse code into Abstract Syntax Tree, chunk at function/class boundaries.

  **Requirements:**

  1. Chunks MUST align with syntactic boundaries (functions, classes, methods)
  2. Import statements MUST be included with first chunk OR stored as metadata
  3. Docstrings/comments MUST stay with their associated code
  4. Class methods MAY be chunked separately with class context in metadata
  5. Chunks MUST include file path and location in metadata

  **Parameters by Language:**

  | Language | Target Size | Max Size | Preserve |
  |----------|-------------|----------|----------|
  | Python | 200 tokens | 500 tokens | function, class |
  | TypeScript | 200 tokens | 500 tokens | function, class, interface |
  | Rust | 250 tokens | 600 tokens | fn, impl, struct, enum |
  | Go | 200 tokens | 500 tokens | func, type |
  | SQL | 150 tokens | 400 tokens | CREATE, SELECT |

  ##### 2.3.14.6.4 Document Chunking (Header-Recursive)

  **Applicable to:** `document/markdown`, `document/rst`, `document/asciidoc`

  **Strategy:** Recursively split at header boundaries, maintaining hierarchy.

  **Requirements:**

  1. Chunks MUST align with section headers when present
  2. Section hierarchy MUST be preserved in metadata
  3. Tables and code blocks MUST NOT be split
  4. Lists SHOULD be kept together when under max size
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.7 (Embedding selection + constraint)
- CONTEXT_START_LINE: 3893
- CONTEXT_END_LINE: 3907
- CONTEXT_TOKEN: Critical Constraint:
- EXCERPT_ASCII_ESCAPED:
  ```text
  **Definition:** An embedding is a dense vector representation of content meaning, enabling mathematical similarity comparison.

  **Critical Constraint:** Query embeddings and index embeddings MUST use the same model. Mixing models produces incompatible vector spaces.

  ##### 2.3.14.7.2 Embedding Model Selection

  | Use Case | Recommended Model | Dimensions | Notes |
  |----------|-------------------|------------|-------|
  | General text | `text-embedding-3-small` | 512 | Best cost/performance |
  | High-precision | `text-embedding-3-large` | 3072 | When accuracy > cost |
  | Code | `Qwen3-Embedding` or `jina-code-v2` | 768 | #1 on MTEB-Code |
  | Multilingual | `multilingual-e5-large-instruct` | 1024 | 100+ languages |
  | Local/offline | `bge-small-en-v1.5` | 384 | Good quality, runs locally |
  | Vision | `clip-ViT-L-14` | 768 | Image-text alignment |
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.118.md 2.3.14.7.4 (Model versioning + re-embedding)
- CONTEXT_START_LINE: 3950
- CONTEXT_END_LINE: 3960
- CONTEXT_TOKEN: Model Versioning and Re-embedding
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.7.4 Model Versioning and Re-embedding

  **Requirements:**

  1. Every embedding MUST record model ID and version
  2. System MUST track which model version each Silver record uses
  3. Model upgrades MUST trigger re-embedding of affected content
  4. Queries MUST only search embeddings from compatible model versions
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.8.2-2.3.14.8.3 (HNSW + BM25 config)
- CONTEXT_START_LINE: 4003
- CONTEXT_END_LINE: 4057
- CONTEXT_TOKEN: interface VectorIndexConfig
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.8.2 Vector Index

  **Algorithm:** HNSW (Hierarchical Navigable Small World)

  **Configuration:**

  ```typescript
  interface VectorIndexConfig {
    algorithm: "hnsw";
    M: number;                    // Max connections (default 16)
    ef_construction: number;      // Construction search width (default 200)
    ef_search: number;            // Query search width (default 100)
    metric: "cosine" | "euclidean" | "dot_product";
    dimensions: number;
  }

  const DEFAULT_VECTOR_CONFIG: VectorIndexConfig = {
    algorithm: "hnsw",
    M: 16,
    ef_construction: 200,
    ef_search: 100,
    metric: "cosine",
    dimensions: 512
  };
  ```

  ##### 2.3.14.8.3 Keyword Index

  **Algorithm:** BM25 (Best Match 25)

  **Configuration:**

  ```typescript
  interface KeywordIndexConfig {
    algorithm: "bm25";
    k1: number;                  // Term frequency saturation (default 1.5)
    b: number;                   // Length normalization (default 0.75)
    tokenizer: TokenizerConfig;
    stop_words: string[] | "default" | "none";
    stemmer: "porter" | "snowball" | "none";
  }

  const CODE_KEYWORD_CONFIG: KeywordIndexConfig = {
    algorithm: "bm25",
    k1: 1.2,
    b: 0.5,
    tokenizer: {
      type: "code",
      lowercase: false,
      split_on_case_change: true
    },
    stop_words: "none",
    stemmer: "none"
  };
  ```
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.8.5 (Hybrid query interface)
- CONTEXT_START_LINE: 4081
- CONTEXT_END_LINE: 4101
- CONTEXT_TOKEN: interface HybridQuery
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.8.5 Hybrid Query Interface

  ```typescript
  interface HybridQuery {
    query: string;
    query_embedding?: Float32Array;
    
    weights: {
      vector: number;    // 0.0 - 1.0
      keyword: number;
      graph: number;
    };
    
    retrieval: {
      k: number;
      vector_candidates: number;
      keyword_candidates: number;
      graph_hops: number;
    };
    
    filters?: MetadataFilter;
  ```
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14.14.4 (Quality SLOs)
- CONTEXT_START_LINE: 4734
- CONTEXT_END_LINE: 4755
- CONTEXT_TOKEN: interface QualitySLOs
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.14.4 Quality Thresholds and SLOs

  ```typescript
  interface QualitySLOs {
    // Retrieval
    min_mrr: 0.6;
    min_recall_at_10: 0.8;
    min_ndcg_at_5: 0.7;
    
    // Data
    min_validation_pass_rate: 0.95;
    min_metadata_completeness: 0.99;
    max_stale_records_ratio: 0.05;
    
    // Latency
    max_p95_retrieval_ms: 500;
    max_p99_retrieval_ms: 1000;
    max_indexing_delay_seconds: 5;
    
    // Coverage
    max_orphan_record_ratio: 0.01;
  }
  ```
  ```

#### ANCHOR 11
- SPEC_ANCHOR: Handshake_Master_Spec_v02.118.md 2.3.14.17.4 (FR-EVT-DATA list)
- CONTEXT_START_LINE: 4916
- CONTEXT_END_LINE: 4930
- CONTEXT_TOKEN: FR-EVT-DATA-015
- EXCERPT_ASCII_ESCAPED:
  ```text
  ##### 2.3.14.17.4 Flight Recorder Events

  | Event ID | Event Name | Trigger |
  |----------|------------|---------|
  | FR-EVT-DATA-001 | bronze_record_created | New Bronze stored |
  | FR-EVT-DATA-002 | silver_record_created | New Silver created |
  | FR-EVT-DATA-003 | silver_record_updated | Silver re-processed |
  | FR-EVT-DATA-004 | embedding_computed | Embedding generated |
  | FR-EVT-DATA-005 | embedding_model_changed | Model upgraded |
  | FR-EVT-DATA-006 | index_updated | Index modified |
  | FR-EVT-DATA-007 | index_rebuilt | Full rebuild |
  | FR-EVT-DATA-008 | validation_failed | Chunk failed |
  | FR-EVT-DATA-009 | retrieval_executed | Search executed |
  | FR-EVT-DATA-010 | context_assembled | Context built |
  | FR-EVT-DATA-011 | pollution_alert | Threshold exceeded |
  | FR-EVT-DATA-012 | quality_degradation | Below SLO |
  | FR-EVT-DATA-013 | reembedding_triggered | Re-embed started |
  | FR-EVT-DATA-014 | relationship_extracted | Edge added |
  | FR-EVT-DATA-015 | golden_query_failed | Regression test failed |
  ```

#### ANCHOR 12
- SPEC_ANCHOR: Handshake_Master_Spec_v02.118.md 11.5.5 (DATA event schemas + validation requirements)
- CONTEXT_START_LINE: 52525
- CONTEXT_END_LINE: 52536
- CONTEXT_TOKEN: Flight Recorder MUST reject DATA events
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.5 FR-EVT-DATA-* (AI-Ready Data Architecture Events) [ADD v02.115]

  Events for Bronze/Silver/Gold layer operations, embedding, indexing, retrieval, and validation per \\u00A72.3.14.

  ```ts
  // FR-EVT-DATA-001: Bronze record created
  interface BronzeRecordCreatedEvent extends FlightRecorderEventBase {
    type: 'data_bronze_created';
    bronze_id: string;
    content_type: string;
    content_hash: string;
    size_bytes: number;
    ingestion_source: 'user' | 'connector' | 'system';
    ingestion_method: 'user_create' | 'file_import' | 'api_ingest' | 'connector_sync';
  }

  // FR-EVT-DATA-002: Silver record created
  interface SilverRecordCreatedEvent extends FlightRecorderEventBase {
    type: 'data_silver_created';
    silver_id: string;
    bronze_ref: string;
    chunk_index: number;
    total_chunks: number;
    token_count: number;
    chunking_strategy: string;
    processing_duration_ms: number;
  }

  // FR-EVT-DATA-004: Embedding computed
  interface EmbeddingComputedEvent extends FlightRecorderEventBase {
    type: 'data_embedding_computed';
    silver_id: string;
    model_id: string;
    model_version: string;
    dimensions: number;
    compute_latency_ms: number;
    was_truncated: boolean;
  }

  // FR-EVT-DATA-009: Retrieval executed
  interface RetrievalExecutedEvent extends FlightRecorderEventBase {
    type: 'data_retrieval_executed';
    request_id: string;
    query_hash: string;  // Privacy: hash the query, not plaintext
    query_intent: 'factual_lookup' | 'code_search' | 'similarity_search' | 'relationship_query' | 'temporal_query';
    weights: { vector: number; keyword: number; graph: number };
    results: {
      vector_candidates: number;
      keyword_candidates: number;
      after_fusion: number;
      final_count: number;
    };
    latency: {
      embedding_ms: number;
      vector_search_ms: number;
      keyword_search_ms: number;
      rerank_ms?: number;
      total_ms: number;
    };
    reranking_used: boolean;
  }

  // FR-EVT-DATA-011: Context pollution alert
  interface ContextPollutionAlertEvent extends FlightRecorderEventBase {
    type: 'data_pollution_alert';
    request_id: string;
    pollution_score: number;
    threshold: number;
    metrics: {
      task_relevance_score: number;
      drift_score: number;
      redundancy_score: number;
      stale_content_ratio: number;
    };
    context_size_tokens: number;
  }

  // FR-EVT-DATA-012: Quality degradation alert
  interface QualityDegradationAlertEvent extends FlightRecorderEventBase {
    type: 'data_quality_degradation';
    metric_name: 'mrr' | 'recall_at_10' | 'ndcg_at_5' | 'validation_pass_rate' | 'metadata_completeness' | 'p95_latency';
    current_value: number;
    threshold: number;
    slo_target: number;
  }

  // FR-EVT-DATA-015: Golden query test failed
  interface GoldenQueryFailedEvent extends FlightRecorderEventBase {
    type: 'data_golden_query_failed';
    query: string;
    expected_ids: string[];
    retrieved_ids: string[];
    expected_mrr: number;
    actual_mrr: number;
    regression_from_baseline: boolean;
  }
  ```

  Full event list (see \\u00A72.3.14.12 for triggers):
  - FR-EVT-DATA-001 through FR-EVT-DATA-015

  Validation requirements (HARD):
  - Flight Recorder MUST reject DATA events that do not match schemas above.
  - Query text MUST be hashed (privacy); never log plaintext queries.
  - Embedding vectors MUST NOT be logged (size/privacy); log only metadata.
  ```

#### ANCHOR 13
- SPEC_ANCHOR: Handshake_Master_Spec_v02.118.md 2.6.6.7.14.6 (normalized_query_hash)
- CONTEXT_START_LINE: 10224
- CONTEXT_END_LINE: 10236
- CONTEXT_TOKEN: normalized_query_hash = sha256(normalize(query_text))
- EXCERPT_ASCII_ESCAPED:
  ```text
  ###### 2.6.6.7.14.6 Required behavior (normative algorithms)

  **A) Query planning is mandatory**
  1. For any retrieval-backed model call, the runtime MUST produce a `QueryPlan` before candidate generation.
  2. `QueryPlan.route[]` MUST be derived from:
     - SemanticCatalog (if present),
     - policy/capability constraints,
     - determinism mode,
     - budgets.
  3. If the runtime cannot produce a plan, the call MUST fail with a surfaced error (not silently proceed).

  **B) Query normalization (deterministic)**
  The runtime MUST compute `normalized_query_hash = sha256(normalize(query_text))`, where `normalize()`:
  - trims leading/trailing whitespace,
  - collapses internal whitespace runs to single spaces,
  - NFC normalizes unicode,
  - lowercases using Unicode casefold,
  - strips control characters.
  ```

#### ANCHOR 14
- SPEC_ANCHOR: Handshake_Master_Spec_v02.118.md 7.6.3 Phase 1 (Roadmap pointer for Phase 1 scope)
- CONTEXT_START_LINE: 41154
- CONTEXT_END_LINE: 41162
- CONTEXT_TOKEN: AI-Ready Data Architecture
- EXCERPT_ASCII_ESCAPED:
  ```text
  - [ADD v02.115] **AI-Ready Data Architecture (\\u00A72.3.14) - Phase 1:**
    - Implement Bronze/Silver/Gold storage layers mapped to `workspace/raw/`, `workspace/derived/`, `workspace/indexes/`
    - Implement content-aware chunking for code (AST-aware, 100-500 tokens) and documents (header-recursive, 256-512 tokens)
    - Implement embedding pipeline with model versioning (`text-embedding-3-small` default, `bge-small-en-v1.5` local fallback)
    - Implement hybrid search (vector HNSW + keyword BM25) with configurable weights
    - Implement ingestion validation gates (token count, coherence checks, boundary validation)
    - **[REMEDIATION]** Wire FR-EVT-DATA-001 through FR-EVT-DATA-015 events to existing Flight Recorder (new event schemas for bronze/silver/embedding/retrieval/quality)
    - Implement quality SLOs and alerts (MRR \\u2265 0.6, Recall@10 \\u2265 0.8, p95 retrieval \\u2264 500ms)
    - Acceptance: hybrid search returns results from Monaco, Terminal, and basic docs; retrieval traces visible in Operator Consoles; FR-EVT-DATA events appear in Flight Recorder
  ```

