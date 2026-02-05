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
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja250120262250

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Phase 1 AI-Ready Data Architecture baseline per Master Spec Section 2.3.14: Bronze/Silver/Gold storage mapping, content-aware chunking, embedding model registry/versioning, hybrid retrieval config/interfaces, quality SLO metrics, and Flight Recorder DATA event schemas + validation enforcement (FR-EVT-DATA-001..015).
- Why: Prevent a "shadow retrieval pipeline" by making retrieval artifacts reproducible, auditable, and visible in Flight Recorder; unblock Phase 1 roadmap item "AI-Ready Data Architecture" and downstream RAG features.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.118.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
  - .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.3.14, 2.3.14.5, 2.3.14.5.3, 2.3.14.6, 2.3.14.7, 2.3.14.7.4, 2.3.14.8.2-2.3.14.8.3, 2.3.14.8.5, 2.3.14.14.4, 2.3.14.17.1, 2.3.14.17.4, 11.5.5, 2.6.6.7.14.6 (plus Roadmap pointer: 7.6.3 Phase 1)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet artifacts:
  - .GOV/task_packets/stubs/WP-1-AI-Ready-Data-Architecture-v1.md (stub; not executable)
- Changes in this v1 activation:
  - Activated as an official task packet anchored to SPEC_CURRENT (v02.117) and the signed refinement file.
  - No prior official packet exists for this Base WP; this is the first executable packet (no v2/v3 lineage).

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.118.md
  - .GOV/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
  - .GOV/task_packets/stubs/WP-1-AI-Ready-Data-Architecture-v1.md
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
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/.gitignore`
- **Start**: 1
- **End**: 3
- **Line Delta**: 1
- **Pre-SHA1**: `e5736e1e6484dc30b070801ba5b03f5b5cada890`
- **Post-SHA1**: `3079df40bde7977e94bd2796527623a51daaf62b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-1520f3e49e0b87a59bdf609eb0d335a6d91bbdc9bd850038c909c9d37813aa21.json`
- **Start**: 1
- **End**: 12
- **Line Delta**: 12
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `0a0272a68552922a428c6fc24cf7c8b8a5c6f471`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-20a5f019ca56f0ca94a6eece13f050c4315b2b41adc6f65f91cab147445750ac.json`
- **Start**: 1
- **End**: 194
- **Line Delta**: 194
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `cb1e0baf767101562de239cbb93fa96de717694f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-288620813036d55ff86030d609f979083873e579e1cdf0d5f8abe7dbdf6dfadd.json`
- **Start**: 1
- **End**: 110
- **Line Delta**: 110
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `aa9e119f49aa8a49da1d508fdb34b7446a52f498`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-5741f7c645f8786259a5eafb3df0225f21e0825d87010b8fab2ddc14f8a23b0e.json`
- **Start**: 1
- **End**: 194
- **Line Delta**: 194
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `8ec9e223437bd2ca4c0401d732b99125d74e69f0`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-5bb844805a39ba1f1237cb0877e9a5e39642051bf8543e944fddbeb7e0c5393c.json`
- **Start**: 1
- **End**: 194
- **Line Delta**: 194
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `5dca68b9ba2af2fee32716a611bd730f6f3e3796`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-5d2a0b569ea81015b2849184add5837a9ae17af773306e35aa5b6765514c8a1e.json`
- **Start**: 1
- **End**: 194
- **Line Delta**: 194
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `00a10eea58acd3377ceaca28eab6e30dd4ecc035`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-660564921bf169d7355b724614f213f71f5aa77e46ca171df7ba1f125a233877.json`
- **Start**: 1
- **End**: 32
- **Line Delta**: 32
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `792bb4a7f46b1c537079c73ae63139fef1f36e9e`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-8b2dab9dcb02ba7839a125045ed7f9688890001f90d20acd9ac2f6f3c8e9b0fa.json`
- **Start**: 1
- **End**: 12
- **Line Delta**: 12
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `d36dde257ee9d90eb84e1e8f9073956d48f75924`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-8bd192b5a9f89d1fb3a625cf7d8647adc9eb506c461768505e4127cbec458452.json`
- **Start**: 1
- **End**: 12
- **Line Delta**: 12
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `00bd32094701d7a335947ce9fe88b04c54e3ec56`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-a9b1d00eb0cda8b18fbf056ec04a044be21c2396b42a596c97c384ba49c34190.json`
- **Start**: 1
- **End**: 110
- **Line Delta**: 110
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `3b7dbdd3f5943884cb22fd319db9f7deceb1a5f7`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-acc2c9328f08160177013e15899e56c0ff5c98284fefeac4e290c1621ae2d73a.json`
- **Start**: 1
- **End**: 62
- **Line Delta**: 62
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `34fe2753a1518bfbe449b8319ef3cca6d32e96d6`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-d38feb7821a8fc9638120f9e592f6330813a7606507d24a47ae5cce711ac8caf.json`
- **Start**: 1
- **End**: 110
- **Line Delta**: 110
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `1f98a3cc2178b1f16139af7e7179b709f1dbd458`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/.sqlx/query-eb373c67e2823e2ab6530fccd3bcd49316be7cdc06506f412acd1246138349d5.json`
- **Start**: 1
- **End**: 12
- **Line Delta**: 12
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `b44da300bd5449e3b7a7de76c0ffd3ac59bca6d0`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/Cargo.lock`
- **Start**: 1
- **End**: 4821
- **Line Delta**: 102
- **Pre-SHA1**: `bee0328b6956b7c5e9faffe5ae29096282d53a5c`
- **Post-SHA1**: `13fc0b040e20a83b62173e0bad81eafbc7ff2846`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/Cargo.toml`
- **Start**: 1
- **End**: 45
- **Line Delta**: 4
- **Pre-SHA1**: `36ff6a115d1f519ca63c0e18d4e0f22e53cac92b`
- **Post-SHA1**: `eeb61a7a751e70d2facd9dbdae31389715e1d9ca`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/migrations/0012_ai_ready_data_arch.down.sql`
- **Start**: 1
- **End**: 5
- **Line Delta**: 5
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `2b72188ac2ecd6f4853d77c3f8e48873153ed994`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/migrations/0012_ai_ready_data_arch.sql`
- **Start**: 1
- **End**: 78
- **Line Delta**: 78
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `3d30adc86be7cfd818b492e14e6a6522ee76558c`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/chunking.rs`
- **Start**: 1
- **End**: 369
- **Line Delta**: 369
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `93b9e6cad299dd702044de424012b5774595cfbd`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/embedding.rs`
- **Start**: 1
- **End**: 94
- **Line Delta**: 94
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `be570281ab70df293e561a8c0ab5158339b193a6`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/indexing.rs`
- **Start**: 1
- **End**: 166
- **Line Delta**: 166
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `50c2c7de16bb678095336a96c19bc1b1f2022421`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/mod.rs`
- **Start**: 1
- **End**: 41
- **Line Delta**: 41
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `df3d5ccd8ed032cba663c3c2c6959f4038d2b524`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/paths.rs`
- **Start**: 1
- **End**: 96
- **Line Delta**: 96
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `ed188078787ff4dfd31c50bca04782b010d3b6ac`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/pipeline.rs`
- **Start**: 1
- **End**: 1379
- **Line Delta**: 1379
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `a604cea6eb1f714b0a022670ec8ad47cf490d6cf`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/quality.rs`
- **Start**: 1
- **End**: 43
- **Line Delta**: 43
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `e824a8b320ce20dda714660125677c42730a871f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/records.rs`
- **Start**: 1
- **End**: 255
- **Line Delta**: 255
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `8fba3e073310111acabf51d77c2926a3962b283f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/ai_ready_data/retrieval.rs`
- **Start**: 1
- **End**: 201
- **Line Delta**: 201
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `f6a50b87053dbaf3df6bd16fd6350155361ec540`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 1
- **End**: 1203
- **Line Delta**: 23
- **Pre-SHA1**: `74026c5abd314be17ac1a7b66e521264289cb9c4`
- **Post-SHA1**: `cdcc07e1ceb4877e7837660e57788791844bd1d3`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 2598
- **Line Delta**: 0
- **Pre-SHA1**: `2cb6602880f20518f77ba59b44889299c1e383be`
- **Post-SHA1**: `1ae6968a8762918b4f5eac3ce0ddcf69f3a711c5`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 5334
- **Line Delta**: 61
- **Pre-SHA1**: `752ffae100d803a47163ad807bb1d734e47e5800`
- **Post-SHA1**: `e6528eb704a36dd5b817e75c83213370bff3a4b4`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 1
- **End**: 35
- **Line Delta**: 1
- **Pre-SHA1**: `16ee13bac7ed06e25865aa6fd72edcedd4d0027c`
- **Post-SHA1**: `95112ff112450323fe0a7e1ccac00650cde77f3b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 974
- **Line Delta**: 51
- **Pre-SHA1**: `cf8dfbc0af01b2678c12a3d7ead7d5b228f88d4d`
- **Post-SHA1**: `a93b2e02e73f861bf075e9a9a02f16bfb1a5e792`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1
- **End**: 2302
- **Line Delta**: 768
- **Pre-SHA1**: `398dda479680210845a77331c5288eadabbfdd71`
- **Post-SHA1**: `c89b52efcdcb7e2bf8597fea208163079be42f28`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 2647
- **Line Delta**: 774
- **Pre-SHA1**: `84bab7838d8366d273e0e3fee4e3f3c09027cf10`
- **Post-SHA1**: `58c017cecd8275202c4728d27f4579da50b8db74`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs`
- **Start**: 1
- **End**: 386
- **Line Delta**: 57
- **Pre-SHA1**: `17c6f8bcbc4a80dec06dc68b19a03d301293cad6`
- **Post-SHA1**: `5ca87633112dd2d2e9c97c299dbce3a5cdc45224`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 1
- **End**: 447
- **Line Delta**: 29
- **Pre-SHA1**: `a2764e4993a27d0e07f0bc175a8e4ad59e49a89e`
- **Post-SHA1**: `d72fd69702b3f6810eec62f8f39148bc5e288a3f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (BOOTSTRAP complete; awaiting SKELETON APPROVAL)
- What changed in this update: Claimed packet (CODER_MODEL/CODER_REASONING_STRENGTH) and moved Status -> In Progress.
- Next step / handoff hint: Review/approve SKELETON; after approval I will update SKELETON in this packet + docs-only checkpoint commit, then start implementation in-scope.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### 2026-01-26 - VALIDATION REPORT - WP-1-AI-Ready-Data-Architecture-v1
Verdict: FAIL

Scope Inputs:
- Reviewed commit: 47d6c96fbf2ba7fe5d045803092d2169fbd7b0f8
- Task Packet: .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md (Status: In Progress)
- Spec: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.118.md (anchors: 2.3.14.5/6/7.4/8.2-8.5/14.4/17.4, 11.5.5, 2.6.6.7.14.6)

Files Checked:
- Handshake_Master_Spec_v02.118.md
- .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md
- .GOV/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
- src/backend/handshake_core/src/ai_ready_data/paths.rs
- src/backend/handshake_core/src/ai_ready_data/chunking.rs
- src/backend/handshake_core/src/ai_ready_data/pipeline.rs
- src/backend/handshake_core/src/ai_ready_data/indexing.rs
- src/backend/handshake_core/src/ai_ready_data/retrieval.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/src/flight_recorder/duckdb.rs
- src/backend/handshake_core/migrations/0012_ai_ready_data_arch.sql
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs

Findings (satisfied evidence):
- Shadow Workspace root mapping implemented: src/backend/handshake_core/src/ai_ready_data/paths.rs:3, src/backend/handshake_core/src/ai_ready_data/paths.rs:32
- Tree-sitter chunking present (Rust + TS/JS): src/backend/handshake_core/src/ai_ready_data/chunking.rs:101, src/backend/handshake_core/src/ai_ready_data/chunking.rs:112
- DATA event types + validators added (strict key sets): src/backend/handshake_core/src/flight_recorder/mod.rs:37, src/backend/handshake_core/src/flight_recorder/mod.rs:671, src/backend/handshake_core/src/flight_recorder/mod.rs:971
- DuckDB event_type parsing supports DATA strings: src/backend/handshake_core/src/flight_recorder/duckdb.rs:721
- Query hashing posture enforced in AI-ready pipeline + tests ensure no plaintext query field:
  - src/backend/handshake_core/src/ai_ready_data/pipeline.rs:1057
  - src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs:320
  - src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs:328
- Migration introduces AI-ready tables: src/backend/handshake_core/migrations/0012_ai_ready_data_arch.sql:22
- Storage trait extended for bronze/silver operations: src/backend/handshake_core/src/storage/mod.rs:846

REASON FOR FAIL:
1) Missing integration with the existing Shadow Workspace retrieval path (task packet intent: cover real runtime retrieval, not only an unused helper pipeline).
   - No data_retrieval_executed / data_context_assembled emission exists in the current workflow retrieval codepath: 0 hits in src/backend/handshake_core/src/workflows.rs (as of commit reviewed).
   - The only DATA retrieval/context emission path is AiReadyDataPipeline (src/backend/handshake_core/src/ai_ready_data/pipeline.rs:228, src/backend/handshake_core/src/ai_ready_data/pipeline.rs:1057), which currently has no call sites in runtime code (exported only at src/backend/handshake_core/src/lib.rs:2).

2) Flight Recorder schema enforcement is too lenient for optional numeric fields (spec uses `?: number`; validator currently accepts null when present).
   - Spec: optional confidence?: number at Handshake_Master_Spec_v02.118.md:52508; optional rerank_ms?: number at Handshake_Master_Spec_v02.118.md:52443.
   - Code allows null: src/backend/handshake_core/src/flight_recorder/mod.rs:1054 (rerank_ms), src/backend/handshake_core/src/flight_recorder/mod.rs:1216 (confidence).

Tests (Validator-run in WP worktree):
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check: PASS
- cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml --all-targets --all-features: PASS (warnings present)

Risks & Suggested Actions:
- Wire DATA-009/010 emission into the real ShadowWs retrieval/context assembly path (likely src/backend/handshake_core/src/workflows.rs) using normalized query hashing per src/backend/handshake_core/src/ace/mod.rs:437.
- Tighten validators: change rerank_ms/confidence to reject null (use require_number when key present).
- Process: update packet scope/waiver coverage if keeping .GOV/roles_shared/OSS_REGISTER.md and src/backend/handshake_core/.sqlx/** as required by the new deps/sqlx changes.

### 2026-01-26 - VALIDATION REPORT - WP-1-AI-Ready-Data-Architecture-v1 (REVALIDATION)
Verdict: PASS

Scope Inputs:
- Reviewed commit: 4c9975c00cd3b836bdcfc1ba4138b52a321d2825
- Task Packet: .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md (**Status:** In Progress)
- Spec: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.118.md (key conformance points: DATA-009/010 runtime emission, query_hash-only privacy posture, strict payload validation)

Files Checked:
- .GOV/roles_shared/SPEC_CURRENT.md
- Handshake_Master_Spec_v02.118.md
- .GOV/task_packets/WP-1-AI-Ready-Data-Architecture-v1.md
- .GOV/refinements/WP-1-AI-Ready-Data-Architecture-v1.md
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs
- src/backend/handshake_core/tests/micro_task_executor_tests.rs

Findings (satisfied evidence):
1) ShadowWs runtime integration now emits the required DATA retrieval/context events:
   - request_id sourced from QueryPlan.plan_id: src/backend/handshake_core/src/workflows.rs:3883
   - data_retrieval_executed emitted with query_hash = RetrievalTrace.normalized_query_hash: src/backend/handshake_core/src/workflows.rs:3993
   - data_context_assembled emitted and shares the same request_id: src/backend/handshake_core/src/workflows.rs:4041

2) Privacy posture (hash-only) is enforced in runtime emission and tests:
   - No plaintext query field is emitted in data_retrieval_executed payload: src/backend/handshake_core/src/workflows.rs:3993
   - Pipeline/test coverage ensures query_hash matches sha256(normalize(query_text)) and no plaintext query leaks: src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs:314

3) Flight Recorder schema enforcement tightened per spec for optional numeric fields (reject null when present):
   - rerank_ms: src/backend/handshake_core/src/flight_recorder/mod.rs:1053
   - confidence: src/backend/handshake_core/src/flight_recorder/mod.rs:1215
   - Test proofs: src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs:333, src/backend/handshake_core/tests/ai_ready_data_arch_tests.rs:370

4) End-to-end proof that ShadowWs micro-task workflow now emits both DATA events:
   - src/backend/handshake_core/tests/micro_task_executor_tests.rs:153

Mechanical Checks:
- just validator-scan: PASS
- just validator-spec-regression: PASS
- just validator-dal-audit: PASS
- just validator-git-hygiene: PASS
- just validator-traceability: PASS

Tests (Validator-run in WP worktree):
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS (warnings present)
- cargo fmt --all -- --check (from src/backend/handshake_core): PASS
- cargo clippy --all-targets --all-features (from src/backend/handshake_core): PASS (warnings present)

Notes / Residual Risk (non-blocking):
- Scope hygiene: src/backend/handshake_core/tests/micro_task_executor_tests.rs was modified to assert DATA emission; consider adding it to IN_SCOPE_PATHS or recording an explicit waiver if strict scoping is required for tests under src/backend/handshake_core/tests/.


