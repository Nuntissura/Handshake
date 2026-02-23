# Task Packet: WP-1-Loom-MVP-v1

## METADATA
- TASK_ID: WP-1-Loom-MVP-v1
- WP_ID: WP-1-Loom-MVP-v1
- BASE_WP_ID: WP-1-Loom-MVP (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-22T15:55:22.397Z
- MERGE_BASE_SHA: 0f5aaf67c6676c1552e948d5efd152ff2ac6b28c
- REQUESTOR: ilja
- AGENT_ID: codex-orchestrator
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A (AGENTIC_MODE=NO)
- ORCHESTRATION_STARTED_AT_UTC: N/A (AGENTIC_MODE=NO)
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Validated (PASS)
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja220220261648
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: Coder A, orchestrator does NOT spawn agents. Coder can use agents.
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Loom-MVP-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement Loom MVP per Master Spec anchors (LoomBlock + LoomEdge graph, import/dedup, cache-tiered browsing + preview jobs, Loom views, Tier-1 search, Flight Recorder events).
- Why: Loom is the Phase 1 "heaper-style unit of meaning" substrate; this WP unblocks downstream WPs in the build order.
- IN_SCOPE_PATHS:
  - src/
  - app/
  - tests/
  - .GOV/task_packets/WP-1-Loom-MVP-v1.md (append-only: STATUS_HANDOFF/EVIDENCE/EVIDENCE_MAPPING/VALIDATION)
  - src/backend/handshake_core/Cargo.lock
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
- OUT_OF_SCOPE:
  - Any non-Loom features not referenced by SPEC_ANCHOR (e.g., Lens extraction tiers, Handshake Stage UI).
  - Search tiers beyond Tier-1 baseline (Tier-2 hybrid + Tier-3 semantic) unless explicitly required by the Tier-1 anchor.
  - Implementing AI tag suggestion workflows (but ensure FR event types exist; emit only for implemented workflows).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- [WAIVER-2026-02-23-LOOM-UI-SURFACES]
  - Date: 2026-02-23
  - Scope: Defer `LM-LINK-002`, `LM-LINK-003`, `LM-TAG-004`, `LM-BACK-001`, `LM-BACK-002`, `LM-VIEW-004` (UI/editor surfaces in `app/`).
  - Justification: WP-1-Loom-MVP-v1 is backend-first; UI/editor integration is deferred to a follow-up WP.
  - Proposed follow-up WP: WP-2-Loom-UI
  - Expiry: before Loom UI release
  - Approver: Operator (chat authorization 2026-02-23: "i will grant the waivers")
- [WAIVER-2026-02-23-LOOM-CACHE-SYNC-STREAMING]
  - Date: 2026-02-23
  - Scope: Defer `LM-CACHE-001`, `LM-CACHE-002`, `LM-CACHE-004`, `LM-CACHE-005` (replication defaults, per-device cache tier config, replication-state query, video streaming).
  - Justification: Sync/replication + streaming behavior requires additional infra + UI and is deferred to a follow-up WP.
  - Proposed follow-up WP: WP-2-Loom-CacheSync
  - Expiry: before multi-device Loom rollout
  - Approver: Operator (chat authorization 2026-02-23: "i will grant the waivers")
- [WAIVER-2026-02-23-LOOM-SHADOW-INDEX]
  - Date: 2026-02-23
  - Scope: Defer `LM-MEDIA-001` step 6 (index LoomBlock in Shadow Workspace Section 2.3.8).
  - Justification: Shadow Workspace indexing path is deferred to a follow-up WP.
  - Proposed follow-up WP: WP-2-Loom-ShadowIndex
  - Expiry: before Tier-3/embeddings work
  - Approver: Operator (chat authorization 2026-02-23: "i will grant the waivers")
- [WAIVER-2026-02-23-LOOM-PG-REL-SEARCH]
  - Date: 2026-02-23
  - Scope: Defer `LM-SEARCH-002` (PostgreSQL relationship-filtered search including backlink depth within query).
  - Justification: Tier-2/PG relationship search is deferred; this WP provides Tier-1 baseline search + API surfaces.
  - Proposed follow-up WP: WP-2-Loom-SearchPG
  - Expiry: before enabling Postgres tier-2 graph-filtered search
  - Approver: Operator (chat authorization 2026-02-23: "i will grant the waivers")

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Loom-MVP-v1
# Coder: add repo-specific build/test commands used (unit + integration) here.
# Optional hygiene:
# just cargo-clean
just post-work WP-1-Loom-MVP-v1 --range 0f5aaf67c6676c1552e948d5efd152ff2ac6b28c..HEAD
```

### DONE_MEANS
- LoomBlock entity + storage match spec anchor (fields + derived fields); CRUD paths exist for create/update/delete.
- LoomEdge entity + storage match spec anchor; edge create/delete works for mention/tag/backlink semantics and preserves `source_anchor` offsets.
- Import path computes SHA-256 content_hash for each imported asset and enforces workspace-scoped dedup; emits FR-EVT-LOOM-006 on dedup hit.
- Cache-tiered asset browsing works per spec anchor; LoomBlocks reference assets deterministically (no client-trusted paths).
- Tier-1 preview generation job exists (bounded concurrency; cancellable) and updates preview status; emits FR-EVT-LOOM-007.
- Loom Views API exists per spec anchor and emits FR-EVT-LOOM-011 with required fields.
- Tier-1 (SQLite FTS5) search API exists per spec anchor and emits FR-EVT-LOOM-012 with required fields.
- Flight Recorder events are implemented and emitted for implemented workflows: at minimum FR-EVT-LOOM-001..007, FR-EVT-LOOM-011..012; event type shapes exist for the full FR-EVT-LOOM-001..012 anchor.
- `just pre-work WP-1-Loom-MVP-v1` and `just post-work WP-1-Loom-MVP-v1 --range <merge_base>..HEAD` run clean; evidence is appended to this packet.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.134.md (recorded_at: 2026-02-22T15:55:22.397Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.134.md 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
  - Handshake_Master_Spec_v02.134.md 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
  - Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 6 Media & File Management: Cache-Tiered Asset Browsing
  - Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 7 Loom Views: Browsing Projections
  - Handshake_Master_Spec_v02.134.md 10.12 Loom Integration Spec 9.3 Three-Tier Search Architecture
  - Handshake_Master_Spec_v02.134.md 11.5.12 FR-EVT-LOOM-001..012 (Loom Surface Events) (Normative) [ADD v02.130]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet(s):
  - .GOV/task_packets/stubs/WP-1-Loom-MVP-v1.md (stub; not activated)
- This packet:
  - Activates Loom MVP into an official packet under `.GOV/task_packets/`.
  - Preserves Loom MVP intent from the stub; updates SPEC_BASELINE to v02.134 and adds concrete DONE_MEANS/BOOTSTRAP/E2E plan.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Loom-MVP-v1.md
  - Handshake_Master_Spec_v02.134.md
- SEARCH_TERMS:
  - "LoomBlock"
  - "LoomEdge"
  - "FR-EVT-LOOM"
  - "loom_block_created"
  - "loom_edge_created"
  - "loom_dedup_hit"
  - "content_hash"
  - "sha256"
  - "preview_status"
  - "FTS5"
  - "search_loom_blocks"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Loom-MVP-v1
  # implement...
  just post-work WP-1-Loom-MVP-v1 --range 0f5aaf67c6676c1552e948d5efd152ff2ac6b28c..HEAD
  ```
- RISK_MAP:
  - "import_dos" -> "bounded concurrency + cancellation on hash/preview/index work; rate-limit large imports"
  - "edge_token_integrity" -> "validate offsets + UUIDs; do not trust client-provided derived edges"
  - "graph_spam" -> "bound auto-creation of missing @mentions/#tags/backlinks per edit/import"
  - "preview_sandbox" -> "capability-gate preview tooling; outputs must be controlled artifacts"

## SKELETON
- Proposed interfaces/types/contracts (no logic):
  - Storage entities (backend; `src/backend/handshake_core/src/storage/*`):
    - `LoomBlock`, `NewLoomBlock`, `LoomBlockUpdate`
    - `LoomBlockContentType`: `note|file|annotated_file|tag_hub|journal`
    - `PreviewStatus`: `none|pending|generated|failed`
    - `LoomBlockDerived` (stored as JSON; selected fields materialized for queryability)
    - `LoomEdge`, `NewLoomEdge`
    - `LoomEdgeType`: `mention|tag|sub_tag|parent|ai_suggested`
    - `LoomSourceAnchor`: `{ document_id, block_id, offset_start, offset_end }` (nullable on edge)
    - `LoomImportRequest` + `LoomImportResult`:
      - request carries file bytes + `original_filename?` + `mime?` (client paths are forbidden)
      - result returns `{ dedup_hit, existing_block_id?, block_id, asset_id?, content_hash }`
    - `LoomViewType`: `all|unlinked|sorted|pins`
    - `LoomViewFilters`: `{ content_type?, mime?, date_from?, date_to?, tag_ids?, mention_ids?, layout? }`
    - `LoomViewResponse`:
      - `all|unlinked|pins`: flat `blocks[]`
      - `sorted`: `groups[]` keyed by `{ edge_type, target_block_id }` + `blocks[]`
    - `LoomBlockSearchQuery`, `LoomBlockSearchResult` (FTS5 Tier-1 baseline)
  - Storage trait extensions (backend; `storage::Database`):
    - LoomBlock CRUD: `create|get|list|update|delete`
    - LoomEdge ops: `create|delete|list_for_block` (for backlinks + groupings)
    - Import primitive: `import_loom_asset(...)`:
      - server computes SHA-256 `content_hash`
      - workspace-scoped dedup returns existing block (no new block created) and emits FR-EVT-LOOM-006
      - on non-dedup: creates Asset + LoomBlock + enqueues preview job + emits FR-EVT-LOOM-001
    - Views: `query_loom_view(workspace_id, view_type, filters, pagination)`
    - Search: `search_loom_blocks(workspace_id, query, filters, pagination)` per **[LM-SEARCH-001]**
  - SQL schema (new migration; SQLite + Postgres):
    - `assets` (minimal Loom-needed subset of Section 2.2.3.1):
      - `asset_id (PK)`, `workspace_id (FK)`, `kind`, `mime`, `original_filename?`, `content_hash (sha256 hex)`, `size_bytes`, `created_at`
      - `classification`, `exportable`, `proxy_asset_id?`, `is_proxy_of?`
      - unique: `(workspace_id, content_hash)`
    - `loom_blocks`:
      - `block_id (PK)`, `workspace_id (FK)`
      - `content_type`, `document_id? (FK documents)`, `asset_id? (FK assets)`
      - `title?`, `original_filename?`, `content_hash?`
      - `pinned`, `journal_date?`, `imported_at?`, `created_at`, `updated_at`
      - derived/materialized: `backlink_count`, `mention_count`, `tag_count`, `thumbnail_asset_id?`, `proxy_asset_id?`, `preview_status`, `derived_json`
      - indexes: `(workspace_id, updated_at DESC)`, `(workspace_id, pinned)`
    - `loom_edges`:
      - `edge_id (PK)`, `workspace_id (FK)`
      - `source_block_id (FK loom_blocks)`, `target_block_id (FK loom_blocks)`, `edge_type`, `created_by`, `created_at`, `crdt_site_id?`
      - source_anchor columns: `source_document_id?`, `source_text_block_id?`, `offset_start?`, `offset_end?`
      - indexes: `(workspace_id, target_block_id)`, `(workspace_id, source_block_id)`
    - `loom_blocks_fts` (SQLite FTS5 virtual table) + triggers to keep in sync with LoomBlock title + doc text + derived full_text_index
  - API routes (backend; new `src/backend/handshake_core/src/api/loom.rs` + `models` request/response types):
    - LoomBlock:
      - `POST /workspaces/:workspace_id/loom/blocks`
      - `GET /workspaces/:workspace_id/loom/blocks/:block_id`
      - `PATCH /workspaces/:workspace_id/loom/blocks/:block_id` (metadata-only; emits FR-EVT-LOOM-002)
      - `DELETE /workspaces/:workspace_id/loom/blocks/:block_id` (emits FR-EVT-LOOM-003)
    - LoomEdge:
      - `POST /workspaces/:workspace_id/loom/edges` (emits FR-EVT-LOOM-004)
      - `DELETE /workspaces/:workspace_id/loom/edges/:edge_id` (emits FR-EVT-LOOM-005)
    - Import + assets:
      - `POST /workspaces/:workspace_id/loom/import` (bytes -> asset -> block; dedup emits FR-EVT-LOOM-006)
      - `GET /workspaces/:workspace_id/assets/:asset_id` (metadata)
      - `GET /workspaces/:workspace_id/assets/:asset_id/content` (stream by server-resolved path; no client paths)
      - `GET /workspaces/:workspace_id/assets/:asset_id/thumbnail` (Tier-1 preview if available)
    - Views + search:
      - `GET /workspaces/:workspace_id/loom/views/:view_type` (emits FR-EVT-LOOM-011)
      - `GET /workspaces/:workspace_id/loom/search?q=...` (Tier-1 FTS5; emits FR-EVT-LOOM-012)
  - Preview generation job (mechanical; bounded + cancellable):
    - New `JobKind::LoomPreviewGenerate` + protocol_id `hsk.loom.preview_generate@v1`
    - `job_inputs` include `{ workspace_id, block_id, asset_id, content_hash, requested_tier: 1 }`
    - job updates `preview_status` + `thumbnail_asset_id` and emits FR-EVT-LOOM-007
  - Flight Recorder events (backend; extend `flight_recorder`):
    - Add event types + payload validators for FR-EVT-LOOM-001..012 (shapes for all; emit only for implemented workflows)
    - Emission points:
      - create/update/delete block: 001/002/003
      - create/delete edge: 004/005
      - import dedup hit: 006
      - preview generated: 007
      - view queried: 011
      - search executed: 012
    - Trust boundary: server-derived IDs/hashes/counts; do not accept client provenance as truth
- Open questions / assumptions:
  - `source_anchor.offset_*` units: confirm UTF-16 code units (frontend JS/ProseMirror) vs Unicode scalar offsets; document chosen contract and validate consistently.
  - Asset binary storage path scheme: `data/workspaces/{workspace_id}/assets/original/{content_hash}` (preferred for dedup) vs `{asset_id}`; confirm with Operator/Validator expectations.
  - "Sorted" view response: confirm grouping requirements for tags vs mentions (two group axes vs a unified group key).
  - Preview tooling baseline on Windows: confirm whether `ffmpeg` is the approved minimal dependency for thumbnails (images/videos) and what to use for PDFs.
- Notes:
  - END_TO_END_CLOSURE_PLAN is applicable and already defined in `## END_TO_END_CLOSURE_PLAN [CX-E2E-001]`.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client->server (import + edits) and job->apply (preview/index updates)
- SERVER_SOURCES_OF_TRUTH:
  - Server recomputes SHA-256 content_hash (dedup key) and rejects/ignores any client-supplied hashes.
  - Server derives and validates LoomEdges (incl. source_anchor offsets) from canonical block content/operations.
  - Preview outputs are produced by server-side jobs and stored as assets; client never supplies filesystem paths.
- REQUIRED_PROVENANCE_FIELDS:
  - workspace_id, block_id, asset_id, content_hash, created_by/updated_by, import_source, job_id, duration_ms
- VERIFICATION_PLAN:
  - Tests cover: dedup correctness (same content_hash => stable existing_block_id) and that preview/search/view paths do not accept client-trusted derived state.
  - Flight Recorder events include correlation identifiers and required fields; evidence is appended in this packet.
- ERROR_TAXONOMY_PLAN:
  - Distinguish stale client state (retryable) vs malformed/forged payload (reject) vs true server errors (surface + log).
- UI_GUARDRAILS:
  - Show preview/index status; disable actions that depend on missing assets; prevent "apply" on stale versions when applicable.
- VALIDATOR_ASSERTIONS:
  - Spec anchors are implemented; trust boundary is enforced (server recomputes/derives critical fields); required FR events are emitted for implemented workflows.

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/Cargo.lock`
- **Start**: 1
- **End**: 4863
- **Line Delta**: 85
- **Pre-SHA1**: `b2cf6fcafbc9429de3b755a587dc787f741eec4d`
- **Post-SHA1**: `92340f26f9e36720e560986930f856ee57e1a8e0`
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
- **End**: 47
- **Line Delta**: 2
- **Pre-SHA1**: `ece7b24c551a0374bfd19babb39b9eeabbe3fbaf`
- **Post-SHA1**: `10749aa276af644d3bd41c6c37b7a1bf1d05c8cc`
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

- **Target File**: `src/backend/handshake_core/migrations/0013_loom_mvp.down.sql`
- **Start**: 1
- **End**: 5
- **Line Delta**: 5
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `678ab7701f680ad084df9f080256b8b686998a14`
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

- **Target File**: `src/backend/handshake_core/migrations/0013_loom_mvp.sql`
- **Start**: 1
- **End**: 95
- **Line Delta**: 95
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `c47beec6c93e40ff962f97fcb49e20327c897193`
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

- **Target File**: `src/backend/handshake_core/src/api/loom.rs`
- **Start**: 1
- **End**: 1095
- **Line Delta**: 1095
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `90948ea1547a3cd84480eec1ebfe5769ac97f34c`
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

- **Target File**: `src/backend/handshake_core/src/api/mod.rs`
- **Start**: 1
- **End**: 41
- **Line Delta**: 3
- **Pre-SHA1**: `47997872f7716ef6ad03601ecac4cf91d851f3da`
- **Post-SHA1**: `f21dd5ae8b5d2a2605ef19afcef1f2fccaea6cbe`
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

- **Target File**: `src/backend/handshake_core/src/capabilities.rs`
- **Start**: 1
- **End**: 673
- **Line Delta**: 5
- **Pre-SHA1**: `f23df3d836f0eeb3fed539080678ae7003ba1193`
- **Post-SHA1**: `0418da12a9816b2b87bd1af7d1fe0ab14d54d2a2`
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
- **End**: 1380
- **Line Delta**: 12
- **Pre-SHA1**: `a2052209cb68b398c77d558eb6e075a854c299b7`
- **Post-SHA1**: `c6e6342642bec2b9501737405e00d90bb2bde2b2`
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
- **End**: 4526
- **Line Delta**: 317
- **Pre-SHA1**: `9800c3816675c821e14572b4fb179d27c3828563`
- **Post-SHA1**: `ec88b8d28bd6170c38208c4f1bde3706709ed406`
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
- **End**: 38
- **Line Delta**: 1
- **Pre-SHA1**: `79723ac3493861294b0325385fc8061ae89b20cf`
- **Post-SHA1**: `cad7ab9b5c4a7f15eaef75f05e9ae3733203411a`
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

- **Target File**: `src/backend/handshake_core/src/loom_fs.rs`
- **Start**: 1
- **End**: 35
- **Line Delta**: 35
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `a5fa14bb612e83624ad8b6f5b54e81733f1847bb`
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

- **Target File**: `src/backend/handshake_core/src/storage/loom.rs`
- **Start**: 1
- **End**: 364
- **Line Delta**: 364
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `85f9734420d560785a1018b9fec037e0a0c37948`
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
- **End**: 1698
- **Line Delta**: 98
- **Pre-SHA1**: `fa76c2c5e02a08be156ee24d114024eaebdf6659`
- **Post-SHA1**: `f94ce4989159248e7e7d0bb8b9f7fe8186ef2285`
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
- **End**: 3567
- **Line Delta**: 1177
- **Pre-SHA1**: `725645ae9c54231873f14f29a5e1f9ea24bf5ba9`
- **Post-SHA1**: `d6346c2c953094f5064300b7253d97580d2320c2`
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
- **End**: 4288
- **Line Delta**: 1451
- **Pre-SHA1**: `0b8715ab4d5cb77f68d0a2100c0d620c8c343660`
- **Post-SHA1**: `6d743f9c8a3910d8eb302957290965dfa1659197`
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
- **End**: 17144
- **Line Delta**: 252
- **Pre-SHA1**: `ea7311db8cd29ff32d1cd59fa5da843b00f8dec7`
- **Post-SHA1**: `0fb7b3b8c92e0fe1454bea5a5664146af6644aca`
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

- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.134.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Ready for Dev
- What changed in this update: Official packet created + filled (scope, anchors, done_means, bootstrap); refinement is signed; prepare recorded to Coder-A with worktree `P:\\Handshake\\Handshake Worktrees\\wt-WP-1-Loom-MVP-v1` on branch `feat/WP-1-Loom-MVP-v1`.
- Next step / handoff hint: Coder-A starts from the WP worktree, runs `just pre-work WP-1-Loom-MVP-v1`, implements against SPEC_ANCHOR list, and appends EVIDENCE_MAPPING + EVIDENCE + STATUS_HANDOFF updates here.
- Current WP_STATUS: In Progress (Coder-A claimed; SKELETON drafted; awaiting approval)
- Current WP_STATUS: In Progress (implementation + hygiene staging in feat/WP-1-Loom-MVP-v1; portable Loom schema + API + job + FR events + Tier-1 search; no DB triggers)
- Touched (non-.GOV): src/backend/handshake_core/migrations/0013_loom_mvp.sql, src/backend/handshake_core/src/storage/loom.rs, src/backend/handshake_core/src/storage/sqlite.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/src/api/loom.rs, src/backend/handshake_core/src/workflows.rs, src/backend/handshake_core/src/flight_recorder/mod.rs
- SQL stance: migration remains portable; SQLite-only FTS5 is created/maintained at runtime with explicit writes (no triggers) to keep future PostgreSQL migration feasible.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`
- REQUIREMENT: "LoomBlock entity + storage match spec anchor (fields + derived fields); CRUD paths exist for create/update/delete."
- EVIDENCE: `src/backend/handshake_core/src/storage/loom.rs:166`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:124`
- REQUIREMENT: "LoomEdge entity + storage match spec anchor; edge create/delete works for mention/tag/backlink semantics and preserves source_anchor offsets."
- EVIDENCE: `src/backend/handshake_core/src/storage/loom.rs:282`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:331`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:1838`
- REQUIREMENT: "Import path computes SHA-256 content_hash for each imported asset and enforces workspace-scoped dedup; emits FR-EVT-LOOM-006 on dedup hit."
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:461`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:493`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:968`
- REQUIREMENT: "Cache-tiered asset browsing works per spec anchor; LoomBlocks reference assets deterministically (no client-trusted paths)."
- EVIDENCE: `src/backend/handshake_core/src/loom_fs.rs:14`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:549`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:641`
- REQUIREMENT: "Tier-1 preview generation job exists (bounded concurrency; cancellable) and updates preview status; emits FR-EVT-LOOM-007."
- EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:941`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10523`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10526`
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:10726`
- REQUIREMENT: "Loom Views API exists per spec anchor and emits FR-EVT-LOOM-011 with required fields."
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:784`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:814`
- REQUIREMENT: "Tier-1 (SQLite FTS5) search API exists per spec anchor and emits FR-EVT-LOOM-012 with required fields."
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:405`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:418`
- EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2275`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:849`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:889`
- REQUIREMENT: "Flight Recorder events are implemented and emitted for implemented workflows: at minimum FR-EVT-LOOM-001..007, FR-EVT-LOOM-011..012; event type shapes exist for the full FR-EVT-LOOM-001..012 anchor."
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:1671`
- EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:1797`
- EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:1011`
- REQUIREMENT: "Scope waivers recorded (backend-first): defer LM-LINK-002/003, LM-TAG-004, LM-BACK-001/002, LM-VIEW-004, LM-CACHE-001/004/005, LM-MEDIA-001 step 6, LM-SEARCH-002."
- EVIDENCE: `.GOV/task_packets/WP-1-Loom-MVP-v1.md:66`
- REQUIREMENT: "Scope waivers recorded (backend-first): defer LM-CACHE-002 (Tier 2 / Tier 3 per-device config)."
- EVIDENCE: `.GOV/task_packets/WP-1-Loom-MVP-v1.md:66`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `just pre-work WP-1-Loom-MVP-v1`
- EXIT_CODE: 0
- PROOF_LINES: `Pre-work validation PASSED`
- COMMAND: `cargo test --lib` (run in `src/backend/handshake_core`)
- EXIT_CODE: 0
- PROOF_LINES: `test api::loom::tests::import_dedup_emits_fr_evt_loom_006 ... ok`
- PROOF_LINES: `test api::loom::tests::view_and_search_emit_events ... ok`
- PROOF_LINES: `test result: ok. 183 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 64.35s`
- COMMAND: `just post-work WP-1-Loom-MVP-v1`
- EXIT_CODE: 0
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`
- PROOF_LINES: `? ROLE_MAILBOX_EXPORT_GATE PASS`
- COMMAND: `cargo test --lib` (run in `src/backend/handshake_core`)
- EXIT_CODE: 0
- PROOF_LINES: `test api::loom::tests::view_and_search_emit_events ... ok`
- PROOF_LINES: `test api::loom::tests::import_dedup_emits_fr_evt_loom_006 ... ok`
- PROOF_LINES: `test result: ok. 183 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.97s`
- COMMAND: `just post-work WP-1-Loom-MVP-v1`
- EXIT_CODE: 0
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`
- PROOF_LINES: `? ROLE_MAILBOX_EXPORT_GATE PASS`
- COMMAND: `just validator-error-codes`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_010431.validator-error-codes.log`
- LOG_SHA256: `33bed78bce3f524657f1e7f5221816a4358677d33bd953c7f84fa9286f3cba64`
- PROOF_LINES: `validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected.`
- COMMAND: `just post-work WP-1-Loom-MVP-v1`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_010431.post-work.log`
- LOG_SHA256: `04172bcc8ace2485b13438be289d1d12f43adde5958805a48ff082ead08b813a`
- PROOF_LINES: `Diff selection: staged (staged changes present)`
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`
- PROOF_LINES: `Manifest[3]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\migrations\0013_loom_mvp.down.sql`
- PROOF_LINES: `Manifest[4]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\migrations\0013_loom_mvp.sql`
- PROOF_LINES: `Manifest[5]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\src\api\loom.rs`
- PROOF_LINES: `Manifest[11]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\src\loom_fs.rs`
- PROOF_LINES: `Manifest[12]: Could not load HEAD version (new file or not tracked): src\backend\handshake_core\src\storage\loom.rs`
- PROOF_LINES: `? ROLE_MAILBOX_EXPORT_GATE PASS`
- COMMAND: `cargo test --lib` (run in `src/backend/handshake_core`)
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_010431.cargo-test-lib.log`
- LOG_SHA256: `07572dadc6ce762ac7758843a218b8c8faf3d1f6a337a07ddbb9145f147178f1`
- PROOF_LINES: `running 183 tests`
- PROOF_LINES: `test api::loom::tests::view_and_search_emit_events ... ok`
- PROOF_LINES: `test api::loom::tests::import_dedup_emits_fr_evt_loom_006 ... ok`
- PROOF_LINES: `test flight_recorder::duckdb::tests::test_fr_evt_shapes_persisted ... ok`
- PROOF_LINES: `test flight_recorder::duckdb::tests::test_record_and_list_events ... ok`
- PROOF_LINES: `test storage::tests::migrations_are_replay_safe_sqlite ... ok`
- PROOF_LINES: `test storage::tests::migrations_are_replay_safe_postgres ... ok`
- PROOF_LINES: `test storage::tests::migrations_can_undo_to_baseline_sqlite ... ok`
- PROOF_LINES: `test storage::tests::migrations_can_undo_to_baseline_postgres ... ok`
- PROOF_LINES: `test workflows::tests::workflow_persists_node_history_and_outputs ... ok`
- PROOF_LINES: `test workflows::tests::terminal_job_enforces_capability ... ok`
- PROOF_LINES: `test workflows::tests::terminal_job_runs_when_authorized ... ok`
- PROOF_LINES: `test result: ok. 183 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 47.89s`
- COMMAND: `just validator-error-codes`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_051633.validator-error-codes.log`
- LOG_SHA256: `33bed78bce3f524657f1e7f5221816a4358677d33bd953c7f84fa9286f3cba64`
- PROOF_LINES: `validator-error-codes: PASS - no stringly errors or nondeterminism patterns detected.`
- COMMAND: `just post-work WP-1-Loom-MVP-v1`
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_051633.post-work.log`
- LOG_SHA256: `68d3c84a22a970eb47d27087d14069c0e515d88d43ea44545606489a12dbe7c1`
- PROOF_LINES: `Diff selection: staged (staged changes present)`
- PROOF_LINES: `Post-work validation PASSED (deterministic manifest gate; not tests) with warnings`
- PROOF_LINES: `? ROLE_MAILBOX_EXPORT_GATE PASS`
- COMMAND: `cargo test --lib` (run in `src/backend/handshake_core`)
- EXIT_CODE: 0
- LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/20260223_051633.cargo-test-lib.log`
- LOG_SHA256: `6beb6464b238368b4c32684dc74763f0616e26713ee0cf660d40feb56c710224`
- PROOF_LINES: `running 183 tests`
- PROOF_LINES: `test api::loom::tests::view_and_search_emit_events ... ok`
- PROOF_LINES: `test api::loom::tests::import_dedup_emits_fr_evt_loom_006 ... ok`
- PROOF_LINES: `test result: ok. 183 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 53.43s`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### 2026-02-23 VALIDATION REPORT - WP-1-Loom-MVP-v1
Verdict: PASS

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Loom-MVP-v1`; not tests): PASS
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): YES

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Loom-MVP-v1.md (**Status:** Validated (PASS))
- Spec Target: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.134.md
- Refinement: .GOV/refinements/WP-1-Loom-MVP-v1.md (Loom Integration anchors 6/7/9.3 + FR-EVT-LOOM-001..012; LM-* MUST list)
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md (active=WP-1-Loom-MVP-v1)

Files Checked:
- .GOV/task_packets/WP-1-Loom-MVP-v1.md
- .GOV/refinements/WP-1-Loom-MVP-v1.md
- .GOV/roles_shared/SPEC_CURRENT.md
- .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
- src/backend/handshake_core/migrations/0013_loom_mvp.sql
- src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- src/backend/handshake_core/src/storage/loom.rs
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/storage/sqlite.rs
- src/backend/handshake_core/src/storage/postgres.rs
- src/backend/handshake_core/src/api/loom.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/src/loom_fs.rs

Findings:
- PASS: LoomBlock schema + CRUD implemented. Evidence mapping: `src/backend/handshake_core/src/storage/loom.rs:166`, `src/backend/handshake_core/src/api/loom.rs:124`.
- PASS: LoomEdge types + create/delete with source_anchor offsets. Evidence mapping: `src/backend/handshake_core/src/storage/loom.rs:282`, `src/backend/handshake_core/src/api/loom.rs:331`, `src/backend/handshake_core/src/storage/sqlite.rs:1838`.
- PASS: LM-LINK-004 (mention to non-existent target auto-creates LoomBlock by UUID) implemented via `ensure_edge_target_exists` for `LoomEdgeType::Mention`. Evidence: `src/backend/handshake_core/src/api/loom.rs:282`.
- PASS: LM-TAG-003 (SUB_TAG) and LM-TAG-005 (AI_SUGGESTED edge type exists) are supported in the data model. Evidence: `src/backend/handshake_core/src/storage/loom.rs:211`.
- PASS: LM-MEDIA-001 steps 1-5,7 + LM-MEDIA-002 workspace-scoped dedup implemented (SHA-256 content_hash; dedup hit emits FR-EVT-LOOM-006). Evidence mapping: `src/backend/handshake_core/src/api/loom.rs:461`, `src/backend/handshake_core/src/api/loom.rs:493`, `src/backend/handshake_core/src/api/loom.rs:968`.
- PASS: LM-CACHE-003 preview generation is a background mechanical job with bounded concurrency + cancellation; emits FR-EVT-LOOM-007. Evidence mapping: `src/backend/handshake_core/src/workflows.rs:10523`, `src/backend/handshake_core/src/workflows.rs:10726`.
- PASS: LM-VIEW-001/002/003 implemented as backend views API with required filters. Evidence: `src/backend/handshake_core/src/storage/loom.rs:308`, `src/backend/handshake_core/src/api/loom.rs:784`.
- PASS: LM-SEARCH-001 backend-agnostic search trait + Tier-1 SQLite FTS5 implementation. Evidence mapping: `src/backend/handshake_core/src/storage/sqlite.rs:405`, `src/backend/handshake_core/src/api/loom.rs:849`.
- PASS: FR-EVT-LOOM-001..012 event shapes/validators exist; implemented workflows emit required events. Evidence mapping: `src/backend/handshake_core/src/flight_recorder/mod.rs:1671`, `src/backend/handshake_core/src/api/loom.rs:1011`.
- WAIVERS (explicit Operator approval 2026-02-23) recorded under `## WAIVERS GRANTED` for deferred MUSTs: LM-LINK-002, LM-LINK-003, LM-TAG-004, LM-BACK-001, LM-BACK-002, LM-VIEW-004, LM-CACHE-001, LM-CACHE-002, LM-CACHE-004, LM-CACHE-005, LM-MEDIA-001 step 6, LM-SEARCH-002.

Tests:
- `just pre-work WP-1-Loom-MVP-v1`: PASS (see `## EVIDENCE` proof lines).
- `just post-work WP-1-Loom-MVP-v1`: PASS (see `## EVIDENCE` proof lines; warnings only for new files).
- `cd src/backend/handshake_core; cargo test --lib`: PASS (183 tests; see `## EVIDENCE` proof lines + log sha256).

Risks and Suggested Actions:
- Deferred by waiver: UI/editor mention+tag rendering, backlinks UI, view grid/list toggle (follow-up WP: WP-2-Loom-UI).
- Deferred by waiver: multi-device cache replication defaults, per-device tier config persistence, streaming video, replication-state queries (follow-up WP: WP-2-Loom-CacheSync).
- Deferred by waiver: Shadow Workspace indexing on import/update (follow-up WP: WP-2-Loom-ShadowIndex).
- Deferred by waiver: Postgres relationship-filtered search including backlink depth (follow-up WP: WP-2-Loom-SearchPG).

Reason for PASS:
- DONE_MEANS + SPEC_ANCHOR requirements are implemented and mapped to evidence; all remaining binding MUSTs are explicitly waived by Operator and recorded in-packet; tests and deterministic manifest gate are PASS.

