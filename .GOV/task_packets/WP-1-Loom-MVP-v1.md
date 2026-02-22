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
- **Status:** In Progress
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
- OUT_OF_SCOPE:
  - Any non-Loom features not referenced by SPEC_ANCHOR (e.g., Lens extraction tiers, Handshake Stage UI).
  - Search tiers beyond Tier-1 baseline (Tier-2 hybrid + Tier-3 semantic) unless explicitly required by the Tier-1 anchor.
  - Implementing AI tag suggestion workflows (but ensure FR event types exist; emit only for implemented workflows).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

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
    - `assets` (minimal Loom-needed subset of §2.2.3.1):
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Ready for Dev
- What changed in this update: Official packet created + filled (scope, anchors, done_means, bootstrap); refinement is signed; prepare recorded to Coder-A with worktree `P:\\Handshake\\Handshake Worktrees\\wt-WP-1-Loom-MVP-v1` on branch `feat/WP-1-Loom-MVP-v1`.
- Next step / handoff hint: Coder-A starts from the WP worktree, runs `just pre-work WP-1-Loom-MVP-v1`, implements against SPEC_ANCHOR list, and appends EVIDENCE_MAPPING + EVIDENCE + STATUS_HANDOFF updates here.
- Current WP_STATUS: In Progress (Coder-A claimed; SKELETON drafted; awaiting approval)

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Loom-MVP-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

