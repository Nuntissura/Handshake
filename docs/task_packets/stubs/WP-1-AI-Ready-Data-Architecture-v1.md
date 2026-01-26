# Task Packet Stub: WP-1-AI-Ready-Data-Architecture-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-AI-Ready-Data-Architecture-v1
- BASE_WP_ID: WP-1-AI-Ready-Data-Architecture
- Created: 2026-01-22
- SPEC_TARGET: docs/SPEC_CURRENT.md (currently Handshake_Master_Spec_v02.117.md)
- ROADMAP_SOURCE: Handshake_Master_Spec_v02.115.md §7.6.3 Phase 1 -> Mechanical Track [ADD v02.115]
- SPEC_ANCHOR_CANDIDATE:
  - Handshake_Master_Spec_v02.115.md §7.6.3 Phase 1 (AI-Ready Data Architecture - Phase 1)
  - Handshake_Master_Spec_v02.115.md §2.3.14 (AI-Ready Data Architecture)

## Roadmap fixed fields (copied from spec; draft)
- Goal:
  - Make the AI-ready data model (Bronze/Silver/Gold) real in the repo so that multiple surfaces can retrieve/work over the same indexed content.
- MUST deliver:
  - Bronze/Silver/Gold storage layers mapped to `workspace/raw/`, `workspace/derived/`, `workspace/indexes/`
  - Content-aware chunking:
    - Code: AST-aware (100-500 tokens)
    - Documents: header-recursive (256-512 tokens)
  - Embedding pipeline with model versioning (default `text-embedding-3-small`, local fallback `bge-small-en-v1.5`)
  - Hybrid search (vector HNSW + keyword BM25) with configurable weights
  - Ingestion validation gates (token count, coherence checks, boundary validation)
  - Flight Recorder wiring for FR-EVT-DATA-001..015 (new event schemas) into existing Flight Recorder
  - Quality SLOs + alerts (MRR >= 0.6, Recall@10 >= 0.8, p95 retrieval <= 500ms)
- Key risks addressed in Phase 1:
  - Retrieval/indexing becomes a shadow subsystem (no shared artifacts, no Flight Recorder evidence).
  - Search quality regressions are invisible without measurable SLOs and logged traces.
- Acceptance criteria:
  - Hybrid search returns results from Monaco, Terminal, and basic docs.
  - Retrieval traces visible in Operator Consoles.
  - FR-EVT-DATA events appear in Flight Recorder.
- Explicitly OUT of scope:
  - Phase 2+ ingestion expansion (Docling pipelines, pack builders, cloud bundle sharing).
- Mechanical Track:
  - YES (ingestion validation jobs + search pipeline + provenance).
- Atelier Track:
  - N/A (unless required by shared retrieval contracts).
- Distillation Track:
  - N/A (unless required by shared retrieval contracts).
- Vertical slice:
  - "Search workspace" over at least Monaco + Terminal + one docs surface, with trace drilldown and reproducible artifacts.

## Why this stub exists
Handshake_Master_Spec_v02.115.md adds a new AI-Ready Data Architecture main-body section (§2.3.14) and Phase 1 roadmap requirements tagged [ADD v02.115]. This stub tracks the Phase 1 work needed to implement the minimum viable Bronze/Silver/Gold + hybrid retrieval + Flight Recorder evidence, so later RAG/ingestion work does not become a shadow pipeline.

## Scope sketch (draft)
- In scope:
  - Implement minimum viable storage/layout and indexing primitives needed for Phase 1 hybrid search + traces.
  - Wire required FR-EVT-DATA events into existing Flight Recorder storage and Operator Console surfaces.
- Out of scope:
  - Full Docling ingestion expansion (Phase 2).
  - Cloud sync or multi-user indexing.

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md` Part 2.5.2).
2. USER_SIGNATURE.
3. Create `docs/refinements/WP-1-AI-Ready-Data-Architecture-v1.md`.
4. Create official task packet via `just create-task-packet WP-1-AI-Ready-Data-Architecture-v1`.
5. Update `docs/WP_TRACEABILITY_REGISTRY.md` to point `WP-1-AI-Ready-Data-Architecture` -> `WP-1-AI-Ready-Data-Architecture-v1`.
6. Update `docs/TASK_BOARD.md` to move `WP-1-AI-Ready-Data-Architecture-v1` out of STUB when activated.
