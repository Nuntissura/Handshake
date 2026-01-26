# Task Packet: WP-1-AI-Ready-Data-Architecture-v1

## METADATA
- TASK_ID: WP-1-AI-Ready-Data-Architecture-v1
- WP_ID: WP-1-AI-Ready-Data-Architecture-v1
- BASE_WP_ID: WP-1-AI-Ready-Data-Architecture
- DATE: 2026-01-25T22:27:10.491Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja250120262250

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Phase 1 AI-Ready Data Architecture baseline per Master Spec Section 2.3.14: Bronze/Silver/Gold storage mapping, content-aware chunking, embedding model registry/versioning, hybrid retrieval config/interfaces, quality SLO metrics, and Flight Recorder DATA event schemas + validation enforcement (FR-EVT-DATA-001..015).
- Why: Prevent a "shadow retrieval pipeline" by making retrieval artifacts reproducible, auditable, and visible in Flight Recorder; unblock Phase 1 roadmap item "AI-Ready Data Architecture" and downstream RAG features.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.118.md
  - docs/SPEC_CURRENT.md
  - docs/SIGNATURE_AUDIT.md
  - docs/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
  - docs/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md
  - src/backend/handshake_core/src/ai_ready_data/
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/migrations/
  - src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs
- OUT_OF_SCOPE:
  - Any Phase 2+ ingestion expansion (Docling pipelines, pack builders, cloud bundle sharing).
  - Any UI work that is not strictly required to view existing Flight Recorder events in Operator Consoles.
  - Any spec edits beyond the approved v02.118 enrichment (Tree-sitter chunking + Shadow Workspace root mapping + FR-EVT-DATA-015 query hashing).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- Waiver ID: WP-1-AI-Ready-Data-Architecture-v1-WAIVER-001
  - Date: 2026-01-26
  - Policy: CX-573F
  - Scope: Allow Master Spec enrichment (v02.118) + add Tree-sitter parsing deps (Cargo.toml/Cargo.lock) to satisfy AST-aware chunking determinism requirements.
  - Justification: Master Spec requires AST-aware code chunking at syntactic boundaries; Tree-sitter is required to make boundaries deterministic and avoid heuristic-only splits.
  - Granted By: Operator (ilja) USER_SIGNATURE ilja260120260102
  - Expiry: On WP closure (validation complete).

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-AI-Ready-Data-Architecture-v1

# Targeted backend tests (add/update as needed):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Formatting / lint (HIGH risk):
cargo fmt --all -- --check
cargo clippy --all-targets --all-features

# Governance and workflow gates:
just validator-scan
just validator-spec-regression
just cargo-clean
just post-work WP-1-AI-Ready-Data-Architecture-v1
```

### DONE_MEANS
- Bronze/Silver/Gold storage artifacts exist and are wired to workspace storage mapping per Master Spec 2.3.14.5 and 2.3.14.5.3 (Bronze immutable; Silver derived; clear separation of raw vs derived vs indexes).
- Chunking strategies exist and are deterministic per Master Spec 2.3.14.6 (code: AST-aware; documents: header-recursive); chunk IDs and metadata are persisted.
- Embedding pipeline exists with model versioning and re-embedding triggers per Master Spec 2.3.14.7 and 2.3.14.7.4 (no silent drift; compatibility enforced).
- Hybrid retrieval config/interfaces exist per Master Spec 2.3.14.8.2-2.3.14.8.3 and 2.3.14.8.5 (vector + keyword weights; candidate counts); retrieval integrates with existing routing (StoreKind ShadowWsLexical/ShadowWsVector) without breaking existing flows.
- Quality thresholds/SLOs are encoded and checked per Master Spec 2.3.14.14.4; at least one mechanical validation path emits quality/pollution signals deterministically.
- Flight Recorder DATA events FR-EVT-DATA-001..015 are emitted and schema-validated per Master Spec 2.3.14.17.4 and 11.5.5; malformed DATA events are rejected; query text is hashed per 2.6.6.7.14.6; embedding vectors are never logged.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes and `just post-work WP-1-AI-Ready-Data-Architecture-v1` returns PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.117.md (recorded_at: 2026-01-25T22:27:10.491Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14, 2.3.14.5, 2.3.14.5.3, 2.3.14.6, 2.3.14.7, 2.3.14.7.4, 2.3.14.8.2-2.3.14.8.3, 2.3.14.8.5, 2.3.14.14.4, 2.3.14.17.1, 2.3.14.17.4, 11.5.5, 2.6.6.7.14.6 (plus Roadmap pointer: 7.6.3 Phase 1)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet artifacts:
  - docs/task_packets/stubs/WP-1-AI-Ready-Data-Architecture-v1.md (stub; not executable)
- Changes in this v1 activation:
  - Activated as an official task packet anchored to SPEC_CURRENT (v02.117) and the signed refinement file.
  - No prior official packet exists for this Base WP; this is the first executable packet (no v2/v3 lineage).

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.118.md
  - docs/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
  - docs/task_packets/stubs/WP-1-AI-Ready-Data-Architecture-v1.md
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/ace/mod.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "FlightRecorderEventType"
  - "RecorderError::InvalidEvent"
  - "validate_ace_validation_payload"
  - "normalized_query_hash"
  - "ShadowWsLexical"
  - "ShadowWsVector"
  - "StoreKind::ShadowWs"
  - "RetrievalTrace"
  - "QueryPlan"
  - "data_bronze_created"
  - "data_silver_created"
  - "data_retrieval_executed"
  - "QualitySLOs"
  - "chunking_strategy"
  - "embedding"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-AI-Ready-Data-Architecture-v1
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just post-work WP-1-AI-Ready-Data-Architecture-v1
  ```
- RISK_MAP:
  - "telemetry privacy regression" -> "Flight Recorder payload/schema validation"
  - "embedding vector leakage" -> "Flight Recorder data events (must never log vectors)"
  - "non-deterministic chunking" -> "chunking pipeline (unstable ids/traces)"
  - "index/model drift" -> "retrieval quality + re-embedding triggers"
  - "schema drift / event rejection" -> "DATA events fail validation and disappear from Operator Consoles"

## SKELETON
- Proposed interfaces/types/contracts:
  - `handshake_core::ai_ready_data` module tree (new):
    - `paths` (Shadow Workspace root + Bronze/Silver/Gold path mapping)
    - `chunking` (Tree-sitter AST-aware chunking for `code/*`; header-recursive for docs)
    - `models` (Bronze/Silver/Embedding record types + deterministic IDs)
    - `embeddings` (model registry/versioning + compatibility + re-embed triggers)
    - `retrieval` (Hybrid query/config + adapter hooks for existing ACE routing)
    - `quality` (QualitySLO thresholds + deterministic checks + event emission helpers)
    - `storage` (SQLite metadata persistence + artifact path references)
  - Shadow Workspace root mapping (Phase 1, normative; Spec v02.118):
    - Root: `data/workspaces/{workspace_id}/workspace/`
    - Bronze: `workspace/raw/` (immutable raw artifacts)
    - Silver: `workspace/derived/` (chunks/embeddings/metadata artifacts)
    - Gold: `workspace/indexes/`, `workspace/graph/` (index + graph artifacts)
  - Deterministic chunking + IDs (Spec 2.3.14.6):
    - Strategy IDs: `code_ast_treesitter_v1`, `document_header_recursive_v1`
    - Chunk IDs derived from stable inputs (bronze_ref + strategy_id + chunk_index + byte/line range + content_hash)
    - Chunk metadata MUST include file path + location (line/byte ranges) for traceability
  - Tree-sitter requirement (Spec v02.118 hard requirement):
    - Use Tree-sitter parsing (dedicated parser) to compute syntactic boundaries for `code/*` (no heuristic-only fallback for conformance)
    - Initial grammars: Rust, Python, TypeScript/JavaScript, Go, SQL (expandable)
  - Embedding model registry (Spec 2.3.14.7 + 2.3.14.7.4):
    - Persist `(model_id, model_version, dimensions, is_default)` and track model used per Silver record
    - Enforce compatibility: query embeddings and index embeddings MUST use the same model/version
  - Hybrid retrieval config/interfaces (Spec 2.3.14.8.2-2.3.14.8.5):
    - `VectorIndexConfig` (HNSW params), `KeywordIndexConfig` (BM25 params), `HybridQuery` weights + candidate counts
    - Integrate with existing routing: `StoreKind::ShadowWsLexical` / `StoreKind::ShadowWsVector` without breaking flows
  - Flight Recorder DATA events (Spec 11.5.5; FR-EVT-DATA-001..015):
    - Add `FlightRecorderEventType` variants for DATA and strict payload validators (reject malformed events; reject unexpected keys)
    - Privacy: DATA schemas MUST log `query_hash` only (use `QueryPlan::compute_normalized_query_hash()` / `RetrievalTrace.normalized_query_hash`); never log plaintext queries or embedding vectors
- Storage + migrations:
    - New SQLite tables for Bronze/Silver/model registry/embedding metadata (no vectors in telemetry; vectors never logged)
    - Filesystem artifacts stored under the Shadow Workspace root; DB stores stable IDs + hashes + artifact refs
- Locked decisions (implementation):
  - Tree-sitter coverage (Phase 1 minimum): Rust + TypeScript/JavaScript; other languages best-effort. Parse failure: emit `data_validation_failed` and do not do heuristic chunking as a fallback (lexical retrieval may continue).
  - Gold indexing format: file-based artifacts under `workspace/indexes/` and `workspace/graph/`; no DB-embedded vector index work in this WP unless explicitly required by the specific spec section being implemented.
- Notes:
  - Spec v02.118 deltas to enforce in code: Tree-sitter parser requirement, Shadow Workspace root mapping under `data/workspaces/{workspace_id}/workspace/`, and FR-EVT-DATA-015 logs `query_hash` (not plaintext).

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.118.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (BOOTSTRAP complete; awaiting SKELETON APPROVAL)
- What changed in this update: Claimed packet (CODER_MODEL/CODER_REASONING_STRENGTH) and moved Status -> In Progress.
- Next step / handoff hint: Review/approve SKELETON; after approval I will update SKELETON in this packet + docs-only checkpoint commit, then start implementation in-scope.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
