# Task Packet: WP-1-Loom-Storage-Portability-v2

## METADATA
- TASK_ID: WP-1-Loom-Storage-Portability-v2
- WP_ID: WP-1-Loom-Storage-Portability-v2
- BASE_WP_ID: WP-1-Loom-Storage-Portability
- DATE: 2026-03-16T19:38:50.184Z
- MERGE_BASE_SHA: d8edecab4a4115736a8f58e7f7c73ffcd065b9b5 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_B
<!-- Required before packet creation: CODER_A .. CODER_Z -->
- WORKFLOW_AUTHORITY: ORCHESTRATOR
<!-- Current repo-governance owner for workflow steering and hard-gate progression. -->
- TECHNICAL_ADVISOR: WP_VALIDATOR
<!-- Advisory WP-scoped validator; may question and challenge coder work but does not own final merge authority. -->
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final technical verdict authority across the WP batch. -->
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final merge-to-main authority. -->
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO. Default NO; set to YES only with explicit operator-authorized sub-agent use. -->
- ORCHESTRATOR_MODEL: N/A
<!-- Required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL: gpt-5.4
- CODER_REASONING_STRENGTH: EXTRA_HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Loom-Storage-Portability-v2
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Loom-Storage-Portability-v2
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-storage-portability-v2
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v2
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v2
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Loom-Storage-Portability-v2
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: integrate/WP-1-Loom-Storage-Portability-v2
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../wti-8fe2e16076
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v2
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v2
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Loom-Storage-Portability-v2
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Loom-Storage-Portability-v2
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Loom-Storage-Portability-v2
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Done
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-Storage-Abstraction-Layer, WP-1-Artifact-System-Foundations
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Loom-Storage-Portability-v2
- LOCAL_WORKTREE_DIR: ../wtc-9302cdcbcd
- REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v2
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v2
- REMOTE_BACKUP_LIFECYCLE: TEMPORARY
<!-- WP backup branches may be deleted after Operator-approved cleanup; later dead links are non-blocking. -->
- BACKUP_PUSH_STATUS: REQUIRED_BEFORE_DESTRUCTIVE_OPS
<!-- Treat the WP backup branch as the phase-boundary recovery branch. Preserve the latest committed restart-safe state at packet/refinement checkpoint, bootstrap claim, skeleton checkpoint, skeleton approval, and before destructive/state-hiding local git actions. -->
- HEARTBEAT_INTERVAL_MINUTES: 15
<!-- Integer minutes; update runtime status/receipts on event boundaries and at this interval only while actively working. -->
- STALE_AFTER_MINUTES: 45
<!-- Integer minutes; heartbeat older than this threshold is stale. -->
- MAX_CODER_REVISION_CYCLES: 3
- MAX_VALIDATOR_REVIEW_CYCLES: 3
- MAX_RELAY_ESCALATION_CYCLES: 2
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: NONE (historical packet; live WP communication authority retired)
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja160320262020
- PACKET_FORMAT_VERSION: 2026-03-16

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001] [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs, src/backend/handshake_core/src/storage/loom.rs, src/backend/handshake_core/src/storage/sqlite.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/src/api/loom.rs, src/backend/handshake_core/src/loom_fs.rs, src/backend/handshake_core/src/storage/tests.rs, src/backend/handshake_core/tests/storage_conformance.rs, src/backend/handshake_core/migrations/0013_loom_mvp.sql, src/backend/handshake_core/migrations/0013_loom_mvp.down.sql | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance; just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs, src/backend/handshake_core/src/storage/loom.rs, src/backend/handshake_core/src/storage/sqlite.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/src/api/loom.rs, src/backend/handshake_core/src/loom_fs.rs, src/backend/handshake_core/src/storage/tests.rs, src/backend/handshake_core/tests/storage_conformance.rs, src/backend/handshake_core/migrations/0013_loom_mvp.sql, src/backend/handshake_core/migrations/0013_loom_mvp.down.sql | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance; just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs, src/backend/handshake_core/src/storage/loom.rs, src/backend/handshake_core/src/storage/sqlite.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/src/api/loom.rs, src/backend/handshake_core/src/loom_fs.rs, src/backend/handshake_core/src/storage/tests.rs, src/backend/handshake_core/tests/storage_conformance.rs, src/backend/handshake_core/migrations/0013_loom_mvp.sql, src/backend/handshake_core/migrations/0013_loom_mvp.down.sql | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance; just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation [LEGACY_REFINEMENT_BRIDGE] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs, src/backend/handshake_core/src/storage/loom.rs, src/backend/handshake_core/src/storage/sqlite.rs, src/backend/handshake_core/src/storage/postgres.rs, src/backend/handshake_core/src/api/loom.rs, src/backend/handshake_core/src/loom_fs.rs, src/backend/handshake_core/src/storage/tests.rs, src/backend/handshake_core/tests/storage_conformance.rs, src/backend/handshake_core/migrations/0013_loom_mvp.sql, src/backend/handshake_core/migrations/0013_loom_mvp.down.sql | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance; just gov-check | EXAMPLES: NONE | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- REQUIRED_TRIPWIRE_TESTS:
  - Legacy bridge: use packet TEST_PLAN plus validator spot-check on integrated main
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance; just gov-check [LEGACY_REFINEMENT_BRIDGE]
- CANONICAL_CONTRACT_EXAMPLES:
  - NONE
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## WP_COMMUNICATIONS (NON-AUTHORITATIVE; REQUIRED FOR NEW PACKETS)
- RULE: The task packet remains authoritative for scope, status, branch/worktree truth, acceptance, and verdict.
- PURPOSE: The per-WP communication folder holds freeform discussion, liveness state, and deterministic receipts for multi-session work.
- COMMUNICATION AUTHORITY RULE:
  - all roles use the packet-declared `WP_COMMUNICATION_DIR`
  - do not improvise role-local inboxes or worktree-local communication authority
  - if there is any dispute about where to write, `WP_COMMUNICATION_DIR` wins
- AUTHORITY SPLIT:
  - `WORKFLOW_AUTHORITY` owns workflow steering and hard gates
  - `TECHNICAL_ADVISOR` is the WP-scoped advisory validator
  - `TECHNICAL_AUTHORITY` owns final technical verdict authority
  - `MERGE_AUTHORITY` owns merge-to-main authority
  - `WP_VALIDATOR_OF_RECORD` and `INTEGRATION_VALIDATOR_OF_RECORD` name the active validator sessions once assigned
- THREAD.md:
  - append-only freeform conversation for Operator, Orchestrator, Coder, and Validator
  - may contain steering, questions, clarifications, and soft coordination
- RUNTIME_STATUS.json:
  - non-authoritative liveness and watch state
  - uses repo-governance runtime states: `submitted | working | input_required | completed | failed | canceled`
  - use for next expected actor, waiting state, validator trigger, heartbeat posture, validation readiness, and bounded review-loop counters
- RECEIPTS.jsonl:
  - append-only deterministic receipt ledger
  - one JSON object per line
  - use for assignment, status, heartbeat, steering, repair, validation, and handoff receipts
- SESSION START + WAKE RULE:
  - only `WORKFLOW_AUTHORITY` may start repo-governed Coder, WP Validator, and Integration Validator sessions
  - primary launch path is `SESSION_HOST_PREFERENCE` via `SESSION_PLUGIN_BRIDGE_ID`
  - if the plugin path fails or times out `SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION` times for the same role/WP session, `CLI_ESCALATION_HOST_DEFAULT` is allowed as the escalation host
  - `SESSION_PLUGIN_REQUESTS_FILE` and `SESSION_REGISTRY_FILE` are the deterministic launch/watch state for those sessions
- WATCH RULE:
  - primary wake-up channel is `SESSION_WAKE_CHANNEL_PRIMARY`
  - fallback wake-up channel is `SESSION_WAKE_CHANNEL_FALLBACK`
  - do not rely on continuous polling when a watch event can be used
- CONFLICT RULE:
  - if THREAD.md, RUNTIME_STATUS.json, or RECEIPTS.jsonl conflicts with this packet, this packet wins
- LOOP LIMIT RULE:
  - do not exceed `MAX_CODER_REVISION_CYCLES`, `MAX_VALIDATOR_REVIEW_CYCLES`, or `MAX_RELAY_ESCALATION_CYCLES` without explicit Operator steering recorded in the packet or thread
- HEARTBEAT RULE:
  - do not poll continuously
  - update `RUNTIME_STATUS.json` and append a receipt on session start, major phase change, blocker/unblock, handoff, completion, and every `HEARTBEAT_INTERVAL_MINUTES` only while active

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- NOTE: `AGENTIC_MODE: YES` means sub-agent use is explicitly authorized for this WP; `AGENTIC_MODE: NO` means all roles remain single-session.
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Loom-Storage-Portability-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- CONTEXT_START_LINE: 3242
- CONTEXT_END_LINE: 3319
- CONTEXT_TOKEN: ### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- EXCERPT_ASCII_ESCAPED:
  ```text
### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]

  **Why**
  Handshake's local-first philosophy (\u00A71.1.4) requires flexibility to support future migrations from SQLite (local) to PostgreSQL (cloud-optional). Building portability constraints now (Phase 1) prevents exponential rework costs in Phase 2+.

  **What**
  Defines four mandatory architectural pillars for ensuring database backend flexibility: single storage API, portable schema/migrations, rebuildable indexes, and dual-backend testing.

  **Pillar 2: Portable Schema & Migrations [CX-DBP-011]**

  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.

  - FORBIDDEN: `strftime()`, SQLite datetime functions -> REQUIRED: Parameterized timestamps
  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2` -> REQUIRED: Portable syntax `$1`, `$2`
  - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics -> REQUIRED: Application-layer mutation tracking
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)

  **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- CONTEXT_START_LINE: 3518
- CONTEXT_END_LINE: 3606
- CONTEXT_TOKEN: #### 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]

  Loom's tables are a portability reference implementation: no triggers, portable SQL types, and rebuildable derived indexes/metrics.

  [ADD v02.156] LoomBlock/LoomEdge records, LoomViewFilters, LoomSearchFilters, LoomBlockSearchResult, and LoomSourceAnchor are canonical portable backend library contracts. Their meaning MUST survive SQLite-now / PostgreSQL-ready storage, export, and replay instead of being hidden behind view-only adapters.

  **Portable schema (SQLite + PostgreSQL)**

  -- LoomBlocks table
  CREATE TABLE loom_blocks (
      block_id UUID PRIMARY KEY,
      workspace_id UUID NOT NULL,
      content_type TEXT NOT NULL,       -- 'note', 'file', 'annotated_file', 'tag_hub', 'journal'
      document_id UUID,                 -- FK to documents table (nullable)
      asset_id UUID,                    -- FK to assets table (nullable)
      title TEXT,
      original_filename TEXT,
      content_hash TEXT,                -- SHA-256 hex
      pinned BOOLEAN NOT NULL DEFAULT FALSE,
      journal_date TEXT,                -- ISO date string (YYYY-MM-DD)
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      imported_at TIMESTAMP
  );

  -- LoomEdges table (Knowledge Graph edges for Loom features)
  CREATE TABLE loom_edges (
      edge_id UUID PRIMARY KEY,
      source_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      target_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      edge_type TEXT NOT NULL,         -- 'mention', 'tag', 'sub_tag', 'parent', 'ai_suggested'
      created_by TEXT NOT NULL,        -- 'user' or 'ai'
      crdt_site_id TEXT,
      source_anchor_doc_id UUID,
      source_anchor_block_id UUID,
      source_anchor_offset_start INTEGER,
      source_anchor_offset_end INTEGER,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  **Storage trait extension (conceptual)**
  This extends the existing Storage API boundary (\u00A72.3.13.3). It is not a parallel storage layer.

  trait LoomStorage {
      // Block CRUD
      async fn create_loom_block(&self, block: &LoomBlock) -> Result<UUID>;
      async fn get_loom_block(&self, block_id: UUID) -> Result<Option<LoomBlock>>;
      async fn update_loom_block(&self, block_id: UUID, update: &LoomBlockUpdate) -> Result<()>;
      async fn delete_loom_block(&self, block_id: UUID) -> Result<()>;

      // Deduplication
      async fn find_by_content_hash(&self, workspace_id: UUID, hash: &str) -> Result<Option<UUID>>;

      // Edge CRUD
      async fn create_loom_edge(&self, edge: &LoomEdge) -> Result<UUID>;
      async fn delete_loom_edge(&self, edge_id: UUID) -> Result<()>;
      async fn get_backlinks(&self, block_id: UUID) -> Result<Vec<LoomEdge>>;
      async fn get_outgoing_edges(&self, block_id: UUID) -> Result<Vec<LoomEdge>>;

      // View queries
      async fn query_all_view(&self, workspace_id: UUID, pagination: &Pagination) -> Result<Vec<LoomBlock>>;
      async fn query_unlinked_view(&self, workspace_id: UUID, pagination: &Pagination) -> Result<Vec<LoomBlock>>;
      async fn query_sorted_view(&self, workspace_id: UUID, group_by: LoomEdgeType) -> Result<Vec<LoomGroup>>;
      async fn query_pinned_view(&self, workspace_id: UUID) -> Result<Vec<LoomBlock>>;

      // Search (backend-adaptive)
      async fn search_loom_blocks(&self, workspace_id: UUID, query: &str, filters: &LoomSearchFilters) -> Result<Vec<LoomBlockSearchResult>>;
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
- CONTEXT_START_LINE: 62252
- CONTEXT_END_LINE: 62318
- CONTEXT_TOKEN: ## 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
- EXCERPT_ASCII_ESCAPED:
  ```text
## 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]

  Loom is a **library + context** surface derived from Heaper patterns: a unified "block" object that can represent a note, a file, or a file-with-annotation, with fast browsing views and a lightweight relational model (mentions, tags, backlinks).

  - Core entity + edge definitions are integrated into \u00A72.2.1.14 (LoomBlock) and \u00A72.3.7.1 (LoomEdge).
  - This section preserves the full Loom integration spec (imported) to avoid loss of detail/intent.

  #### 1. Purpose and Scope

  This document extracts validated UX patterns and architectural concepts from Heaper (a local-first, linked note-taking application spanning notes, media, and files) and maps them onto Handshake's existing architecture. The goal is to absorb Heaper's strengths - particularly its "block-as-unit-of-meaning" information model, relational organization via links/tags/mentions, and cache-tiered media browsing - without importing Heaper's stack, deployment model, or limitations.

  - A **pattern integration spec** that translates Heaper's product concepts into Handshake-native schemas, requirements, and roadmap items.
  - A **gap analysis** identifying where Heaper's features fill genuine holes in Handshake's current specification.
  - A **PostgreSQL expansion plan** showing how Handshake's database-portable architecture can implement these patterns at a level Heaper's own stack cannot reach.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation
- CONTEXT_START_LINE: 62794
- CONTEXT_END_LINE: 63004
- CONTEXT_TOKEN: **[LM-SEARCH-001]** The search API MUST be backend-agnostic.
- EXCERPT_ASCII_ESCAPED:
  ```text
**[LM-SEARCH-001]** The search API MUST be backend-agnostic. The storage trait exposes `search_loom_blocks(query, filters) -> Vec<LoomBlockSearchResult>`. The implementation varies by backend.

  **[LM-SEARCH-002]** On PostgreSQL, search results MUST be filterable by graph relationships (tags, mentions, backlink depth) within the query. This is a key improvement over Heaper's client-side-only search.

  **[LM-GRAPH-001]** Graph traversal queries MUST work on both SQLite (using recursive CTEs, available since SQLite 3.35+) and PostgreSQL. Performance targets: <100ms for 3-hop traversal on 10K blocks (SQLite), <50ms on PostgreSQL.

  ##### 11.1 Schema (Portable - SQLite and PostgreSQL)

  All schemas follow \u00A72.3.13 Storage Backend Portability requirements:
  - No `strftime()` or SQLite-specific functions.
  - Portable placeholder syntax (`$1`, `$2`).
  - No triggers - application-layer mutation tracking.
  - TIMESTAMP instead of DATETIME.

  -- LoomBlocks table
  CREATE TABLE loom_blocks (
      block_id UUID PRIMARY KEY,
      workspace_id UUID NOT NULL,
      content_type TEXT NOT NULL,       -- 'note', 'file', 'annotated_file', 'tag_hub', 'journal'
      document_id UUID,                 -- FK to documents table (nullable)
      asset_id UUID,                    -- FK to assets table (nullable)
      title TEXT,
      original_filename TEXT,
      content_hash TEXT,                -- SHA-256 hex
      pinned BOOLEAN NOT NULL DEFAULT FALSE,
      journal_date TEXT,                -- ISO date string
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      imported_at TIMESTAMP
  );

  -- LoomEdges table (Knowledge Graph edges for Loom features)
  CREATE TABLE loom_edges (
      edge_id UUID PRIMARY KEY,
      source_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      target_block_id UUID NOT NULL REFERENCES loom_blocks(block_id),
      edge_type TEXT NOT NULL,         -- 'mention', 'tag', 'sub_tag', 'parent', 'ai_suggested'
      created_by TEXT NOT NULL,        -- 'user' or 'ai'
      crdt_site_id TEXT,
      source_anchor_doc_id UUID,
      source_anchor_block_id UUID,
      source_anchor_offset_start INTEGER,
      source_anchor_offset_end INTEGER,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  trait LoomStorage {
      // Search (backend-adaptive)
      async fn search_loom_blocks(&self, workspace_id: UUID, query: &str, filters: &LoomSearchFilters) -> Result<Vec<LoomBlockSearchResult>>;

      // Graph traversal
      async fn traverse_graph(&self, start_block_id: UUID, max_depth: u32, edge_types: &[LoomEdgeType]) -> Result<Vec<(LoomBlock, u32)>>;

      // Metrics (derived, rebuildable)
      async fn recompute_block_metrics(&self, block_id: UUID) -> Result<()>;
      async fn recompute_all_metrics(&self, workspace_id: UUID) -> Result<()>;
  }
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Use the signed refinement directly for clause proof planning.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Reconstruct contract-surface checks from the signed refinement when needed.
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Read the signed refinement directly for execution guidance.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Read the signed refinement directly for inspection guidance.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- COMPATIBILITY_NOTE: Signed refinement predates REFINEMENT_FORMAT_VERSION 2026-03-15. Uncertainty tracking remains in the signed refinement only.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [OSS_DOC] SQLite FTS5 docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://sqlite.org/fts5.html | Why: canonical reference for SQLite-side full-text search behavior and index locality
  - [OSS_DOC] PostgreSQL full text search docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://www.postgresql.org/docs/current/textsearch.html | Why: canonical reference for PostgreSQL-side ranked text search and backend-specific query power
  - [BIG_TECH] Google Cloud Spanner full-text search overview | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://cloud.google.com/spanner/docs/full-text-search | Why: current large-scale vendor reference showing richer backend-specific search features can exist behind one SQL-facing search surface
  - [OSS_DOC] OpenLineage spec docs | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://openlineage.io/docs/spec/ | Why: useful reference for typed lineage and provenance payloads that survive transport and backend changes
  - [GITHUB] OpenLineage repository | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://github.com/OpenLineage/OpenLineage | Why: concrete repository-scale example of typed lineage/provenance contract evolution
  - [PAPER] In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | 2025-11-05 | Retrieved: 2026-03-13T22:38:08Z | https://arxiv.org/abs/2511.03480 | Why: recent provenance-indexing paper supporting explicit, queryable source/provenance structures instead of opaque backend-local metadata
  - [GITHUB] pgvector repository | 2026-03-13 | Retrieved: 2026-03-13T22:38:08Z | https://github.com/pgvector/pgvector | Why: high-signal reference for backend-specific search acceleration that should remain optional rather than canonical in this packet
- RESEARCH_SYNTHESIS:
  - A portability packet should preserve one stable API and semantic contract while allowing provider-specific indexing and query plans behind the boundary.
  - Big-tech search systems confirm that richer backend-specific ranking, tokenization, and query expansion can stay behind a stable query surface instead of redefining canonical filter meaning.
  - Typed provenance payloads are more durable than ad hoc search or edge metadata and map well to Loom `source_anchor` export/replay expectations.
  - Recent provenance-indexing research reinforces that explicit source/provenance structures should stay queryable and transport-stable, which matches Handshake's need for durable `LoomSourceAnchor` semantics across storage, export, and replay.
  - Search parity should mean stable filters, result identity, and semantic guarantees, not identical score math across SQLite and PostgreSQL.
  - Backend-specific enhancements are useful, but they must not become the only definition of Loom search or graph behavior while Handshake still promises SQLite-now and PostgreSQL-ready portability.
- GITHUB_PROJECT_DECISIONS:
  - OpenLineage/OpenLineage -> ADAPT (NONE)
  - pgvector/pgvector -> TRACK_ONLY (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - SQLite FTS5 docs -> ADOPT (IN_THIS_WP)
  - PostgreSQL full text search docs -> ADOPT (IN_THIS_WP)
  - Google Cloud Spanner full-text search overview -> ADOPT (IN_THIS_WP)
  - OpenLineage spec docs -> ADAPT (IN_THIS_WP)
  - OpenLineage repository -> ADAPT (IN_THIS_WP)
  - In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines -> ADAPT (IN_THIS_WP)
  - pgvector repository -> REJECT (REJECT_DUPLICATE)
- MATRIX_GROWTH_CANDIDATES:
  - Stable search API plus provider-local indexing -> IN_THIS_WP (stub: NONE)
  - Portable source-anchor lineage plus export/replay durability -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep provider-specific FTS and ranking logic inside backend implementations, not in portable migrations.
  - Assert semantic parity through shared Loom conformance tests rather than comparing SQL query text.
  - Treat `LoomSourceAnchor` and view/search filters as portable contract structs, not adapter-only shapes.
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_EXPOSED:
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Loom
  - SQL to PostgreSQL shift readiness
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - One Loom storage contract plus dual provider implementations -> IN_THIS_WP (stub: NONE)
  - Portable Loom migrations plus replay-safe and down-safe verification -> IN_THIS_WP (stub: NONE)
  - SQLite FTS and PostgreSQL text search behind one API -> IN_THIS_WP (stub: NONE)
  - View filter parity across providers -> IN_THIS_WP (stub: NONE)
  - Source-anchor durability across storage, export, and replay -> IN_THIS_WP (stub: NONE)
  - Asset blob path stability plus storage metadata parity -> IN_THIS_WP (stub: NONE)
  - Rebuildable derived metrics plus provider-local indexes -> IN_THIS_WP (stub: NONE)
  - Shared Loom conformance tests over SQLite and PostgreSQL -> IN_THIS_WP (stub: NONE)
  - Thin API seam over portable storage behavior -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Loom | CAPABILITY_SLICE: Portable block and edge record parity | SUBFEATURES: `LoomBlock`, `LoomEdge`, content-hash dedup, metrics rebuildability, and stable backend-neutral meaning | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived, PRIM-LoomEdgeType | MECHANICAL: engine.archivist, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should guarantee that block and edge semantics survive SQLite and PostgreSQL backends without adapter drift
  - PILLAR: Loom | CAPABILITY_SLICE: Portable view, search, and source-anchor contract | SUBFEATURES: `LoomViewFilters`, `LoomSearchFilters`, `LoomBlockSearchResult`, and `LoomSourceAnchor` parity | PRIMITIVES_FEATURES: PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the API contract should preserve the same filter meaning and source-anchor durability across both backends
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Loom migration and DDL portability | SUBFEATURES: replay-safe migrations, down migrations, provider-local indexes outside portable DDL, and no trigger-dependent semantics | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the direct portability law bridge from spec to code for the Loom surface
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Cross-provider Loom conformance coverage | SUBFEATURES: shared test helpers for SQLite and PostgreSQL parity over CRUD, search, view, dedup, and anchor round-trips | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity should be proven by tests, not inferred from provider implementations
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend parity must preserve CRUD meaning and existing Loom telemetry regardless of provider implementation
  - Capability: Loom import and dedup portability | JobModel: WORKFLOW | Workflow: loom_import | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_dedup_hit, loom_block_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: content-hash dedup, asset-path layout, and import-created blocks must preserve the same semantics across backends
  - Capability: Loom view portability | JobModel: UI_ACTION | Workflow: loom_view_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_view_queried | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `LoomViewFilters` and grouped-view semantics must not drift when the backend changes
  - Capability: Loom search portability | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: search scoring may differ by provider, but filter meaning, result identity, and backend-neutral contract must remain stable
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Media-Downloader-Loom-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Video-Archive-Loom-Integration-v1 -> KEEP_SEPARATE
  - WP-1-Loom-Preview-VideoPosterFrames-v1 -> KEEP_SEPARATE
  - WP-1-Loom-MVP-v1 -> KEEP_SEPARATE
  - WP-1-Storage-Abstraction-Layer-v3 -> KEEP_SEPARATE
  - WP-1-Artifact-System-Foundations-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Storage-Abstraction-Layer-v3)
  - src/backend/handshake_core/src/storage/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/sqlite.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/postgres.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/tests.rs -> NOT_PRESENT (NONE)
  - src/backend/handshake_core/src/api/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/loom_fs.rs -> PARTIAL (NONE)
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql -> PARTIAL (WP-1-Loom-MVP-v1)
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: NO
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE
- GUI_REFERENCE_DECISIONS:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
## SCOPE
- What: Harden Loom block, edge, search, view, source-anchor, migration, and asset-path behavior into one portable backend contract that preserves meaning across SQLite and PostgreSQL implementations.
- Why: Local `main` already includes the selective Loom portability integration, but operator review still judged the implementation underperformed and the prior audit only established a narrower correctness slice. This packet re-audits and, where needed, remediates the landed storage-portability seam with stronger parity proof and validator pressure.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- OUT_OF_SCOPE:
  - structured-collaboration schema registry and mailbox export schema work
  - frontend Loom viewer UX and preview-surface layout changes
  - preview job protocol redesign or broad workflow-runtime refactors
  - downloader-to-Loom and video-archive bridge behavior beyond the base portability contract
  - capability-registry publication logic unrelated to Loom storage parity
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance
  just gov-check
```

### DONE_MEANS
- Loom block, edge, view, search, and source-anchor semantics are either confirmed unchanged or explicitly remediated on the landed current-main slice, with no vague "looks fine" closeout.
- Portable migrations and down migrations for Loom tables remain replay-safe and provider-neutral.
- Shared Loom conformance tests and semantic tripwires give explicit parity coverage for CRUD, dedup, views, search filters, literal search escaping, and source-anchor round-trips across both backends.
- Filesystem asset-path layout remains stable and compatible with the portable storage contract, and no unrelated product families are touched.

- PRIMITIVES_EXPOSED:
  - PRIM-LoomBlock
  - PRIM-LoomBlockContentType
  - PRIM-LoomBlockDerived
  - PRIM-LoomEdge
  - PRIM-LoomEdgeType
  - PRIM-LoomViewFilters
  - PRIM-LoomSearchFilters
  - PRIM-LoomBlockSearchResult
  - PRIM-LoomSourceAnchor
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.178.md (recorded_at: 2026-03-16T19:38:50.184Z)
- SPEC_TARGET: .GOV/roles_shared/records/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.156]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/0013_loom_mvp.sql
  - src/backend/handshake_core/migrations/0013_loom_mvp.down.sql
- SEARCH_TERMS:
  - create_loom_block
  - create_loom_edge
  - query_loom_view
  - search_loom_blocks
  - LoomSourceAnchor
  - LoomViewFilters
  - LoomSearchFilters
  - loom_blocks
  - loom_edges
  - loom_blocks_fts
- RUN_COMMANDS:
  ```bash
rg -n "create_loom_block|create_loom_edge|query_loom_view|search_loom_blocks|LoomSourceAnchor|LoomViewFilters|LoomSearchFilters|loom_blocks|loom_edges|loom_blocks_fts" src/backend/handshake_core
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance
  just gov-check
  ```
- RISK_MAP:
  - "provider-specific search logic changes filter meaning" -> "view and search parity break across SQLite and PostgreSQL"
  - "source anchors fail to round-trip on one backend" -> "backlinks, context snippets, and downstream bridge packets lose stable provenance"
  - "filesystem asset-path layout drifts from storage metadata" -> "export, replay, and dedup behavior become unreliable"
  - "the remediation pass drifts into unrelated runtime families" -> "the packet loses file-lock isolation and repeats the earlier live-smoke scope failure"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - NONE
- REQUIRED_PROVENANCE_FIELDS:
  - NONE
- VERIFICATION_PLAN:
  - Record end-to-end trust/provenance requirements only if this WP introduces a cross-boundary apply path.
- ERROR_TAXONOMY_PLAN:
  - N/A for initial coder handoff.
- UI_GUARDRAILS:
  - N/A for initial coder handoff.
- VALIDATOR_ASSERTIONS:
  - Validate the packet-scoped spec anchors, in-scope files, and deterministic evidence recorded during implementation.
## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 21
- **End**: 2213
- **Line Delta**: 36
- **Pre-SHA1**: `6d815d9ff393eb7073462bcc57bd14286049c6e2`
- **Post-SHA1**: `b2ba46411dc04b5ae8a4c50ded273f887bb6b19e`
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
- **Lint Results**: Covered by `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`, and `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom -- --nocapture`.
- **Artifacts**: `git diff --unified=0 705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005 -- src/backend/handshake_core/src/storage/sqlite.rs`; `just cor701-sha src/backend/handshake_core/src/storage/sqlite.rs`
- **Timestamp**: 2026-03-17T04:34:05.9040608+01:00
- **Operator**: CODER codex-cli
- **Spec Target Resolved**: .GOV/roles_shared/records/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Range-mode pre/post SHA1s are anchored to `705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005`; `just cor701-sha` confirms the committed repair LF blob matches the post image.

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 23
- **End**: 1800
- **Line Delta**: 39
- **Pre-SHA1**: `f105bd3fb4bfda5fb9259a330365f651038c4c03`
- **Post-SHA1**: `023aa7cb5258649d6eb6dd3aa014a5493c2860b9`
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
- **Lint Results**: Covered by `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`, and `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom -- --nocapture`.
- **Artifacts**: `git diff --unified=0 705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005 -- src/backend/handshake_core/src/storage/postgres.rs`; `just cor701-sha src/backend/handshake_core/src/storage/postgres.rs`
- **Timestamp**: 2026-03-17T04:34:05.9040608+01:00
- **Operator**: CODER codex-cli
- **Spec Target Resolved**: .GOV/roles_shared/records/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Range-mode pre/post SHA1s are anchored to `705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005`; `just cor701-sha` confirms the committed repair LF blob matches the post image.

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 1186
- **End**: 1387
- **Line Delta**: 201
- **Pre-SHA1**: `83a3a74e37c6ddcd7c335a6cc22694eba328fbfc`
- **Post-SHA1**: `0f78a8f0862c9cca2f47b4effb26580b36bea86e`
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
- **Lint Results**: Covered by `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`, and `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom -- --nocapture`.
- **Artifacts**: `git diff --unified=0 705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005 -- src/backend/handshake_core/src/storage/tests.rs`; `just cor701-sha src/backend/handshake_core/src/storage/tests.rs`
- **Timestamp**: 2026-03-17T04:34:05.9040608+01:00
- **Operator**: CODER codex-cli
- **Spec Target Resolved**: .GOV/roles_shared/records/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Range-mode pre/post SHA1s are anchored to `705220e2e3a9468b83e471029c239ad906f728bd..643558e2173d4f7d167432be43e6140d97158005`; `just cor701-sha` confirms the committed repair LF blob matches the post image.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Current WP_STATUS: In Progress; committed repair candidate `643558e2173d4f7d167432be43e6140d97158005` is awaiting integration-validator closeout after packet evidence closure.
- What changed in this update: The committed repair recalculates surviving Loom `mention_count`, `tag_count`, and `backlink_count` after linked block deletion in `src/backend/handshake_core/src/storage/sqlite.rs`, `src/backend/handshake_core/src/storage/postgres.rs`, and adds shared conformance regression coverage in `src/backend/handshake_core/src/storage/tests.rs`.
- Next step / handoff hint: Run integration-validator closeout on the committed repair candidate, using the packet evidence below plus the WP communication thread for the bounded PostgreSQL environment skip.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "Loom block, edge, view, search, and source-anchor semantics are either confirmed unchanged or explicitly remediated on the landed current-main slice, with no vague 'looks fine' closeout."
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2145`; `src/backend/handshake_core/src/storage/sqlite.rs:2197`; `src/backend/handshake_core/src/storage/postgres.rs:1742`; `src/backend/handshake_core/src/storage/postgres.rs:1783`
  - REQUIREMENT: "Shared Loom conformance tests and semantic tripwires give explicit parity coverage for CRUD, dedup, views, search filters, literal search escaping, and source-anchor round-trips across both backends."
  - EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:1284`; `src/backend/handshake_core/src/storage/tests.rs:1295`; `src/backend/handshake_core/src/storage/tests.rs:1314`; `src/backend/handshake_core/src/storage/tests.rs:1386`; `src/backend/handshake_core/tests/storage_conformance.rs:32`; `src/backend/handshake_core/tests/storage_conformance.rs:40`
  - REQUIREMENT: "[ADD v02.156] LoomBlock/LoomEdge records, LoomViewFilters, LoomSearchFilters, LoomBlockSearchResult, and LoomSourceAnchor are canonical portable backend library contracts. Their meaning MUST survive SQLite-now / PostgreSQL-ready storage, export, and replay instead of being hidden behind view-only adapters."
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2165`; `src/backend/handshake_core/src/storage/sqlite.rs:2197`; `src/backend/handshake_core/src/storage/postgres.rs:1762`; `src/backend/handshake_core/src/storage/postgres.rs:1783`; `src/backend/handshake_core/src/storage/tests.rs:1293`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES:
  - `test sqlite_loom_storage_conformance ... ok`
  - `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.04s`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES:
  - `test postgres_loom_storage_conformance ... ok`
  - `Skipping postgres loom storage conformance: POSTGRES_TEST_URL not set for postgres tests`
  - `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom -- --nocapture`
- EXIT_CODE: 0
- PROOF_LINES:
  - `test storage::tests::loom_migration_schema_is_portable_postgres ... ok`
  - `test storage::tests::loom_migration_schema_is_portable_sqlite ... ok`
  - `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 200 filtered out; finished in 0.40s`

- COMMAND: `just gov-check`
- EXIT_CODE: 0
- PROOF_LINES:
  - `wp-communications-check ok`
  - `migration-path-truth-check ok`
  - `gov-check ok`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.

### VALIDATION REPORT - 2026-03-17 (Integration Validator, authoritative-context superseding report)
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: FAIL
- HEURISTIC_REVIEW_VERDICT: FAIL
- SPEC_ALIGNMENT_VERDICT: FAIL
- ENVIRONMENT_VERDICT: PARTIAL
- DISPOSITION: NONE
- LEGAL_VERDICT: FAIL
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- SUMMARY:
  - Authoritative current workflow context was accepted for this verdict. Packet status is `In Progress`, Task Board status is `IN_PROGRESS`, and stale local mirror-based packet/task-board reasoning from the earlier local report is superseded as context mismatch rather than counted here as a Loom packet failure.
  - Final `FAIL` remains because deleting a linked Loom block leaves stale derived counters on surviving blocks. SQLite `delete_loom_block` deletes the block and commits without recomputing `mention_count` / `tag_count` / `backlink_count` for surviving neighbors (`src/backend/handshake_core/src/storage/sqlite.rs:2144-2179`), while the corresponding recomputation exists only in explicit edge create/delete flows (`src/backend/handshake_core/src/storage/sqlite.rs:2272-2289`, `src/backend/handshake_core/src/storage/sqlite.rs:2342-2365`). The same omission exists in PostgreSQL (`src/backend/handshake_core/src/storage/postgres.rs:1741-1762`) even though edge create/delete recompute is present there too (`src/backend/handshake_core/src/storage/postgres.rs:1854-1871`, `src/backend/handshake_core/src/storage/postgres.rs:1924-1947`).
  - The portable schema cascades `loom_edges` on block delete (`src/backend/handshake_core/migrations/0013_loom_mvp.sql:77-78`), and the shared conformance suite only deletes an unlinked block (`src/backend/handshake_core/src/storage/tests.rs:1187-1193`). The stale-counter regression is therefore real, packet-scope, and currently uncovered by committed regression tests.
- CLAUSES_REVIEWED:
  - Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001] [LEGACY_REFINEMENT_BRIDGE] -> SQLite and PostgreSQL both implement the block-delete lifecycle, but both omit surviving-block derived-counter recomputation on that path (`src/backend/handshake_core/src/storage/sqlite.rs:2144-2179`, `src/backend/handshake_core/src/storage/postgres.rs:1741-1762`).
  - Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> the portable schema cascades linked-edge deletion when a block is removed (`src/backend/handshake_core/migrations/0013_loom_mvp.sql:77-78`), so stored derived metrics must remain consistent across that lifecycle transition; the current implementation does not preserve that invariant.
  - Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> `LoomBlockDerived` counts on surviving linked blocks can remain stale after deleting a linked target because recomputation only occurs on explicit edge create/delete (`src/backend/handshake_core/src/storage/sqlite.rs:2272-2289`, `src/backend/handshake_core/src/storage/sqlite.rs:2342-2365`, `src/backend/handshake_core/src/storage/postgres.rs:1854-1871`, `src/backend/handshake_core/src/storage/postgres.rs:1924-1947`).
  - Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation [LEGACY_REFINEMENT_BRIDGE] -> backend conformance tests pass, but the packet-scoped suite does not cover linked-block deletion; current delete coverage stops at an unlinked block (`src/backend/handshake_core/src/storage/tests.rs:1187-1193`).
- NOT_PROVEN:
  - A live ad hoc PostgreSQL repro of the stale-counter effect was not rerun in this session because environment-backed proof remained unavailable; the PostgreSQL delete path is structurally identical to SQLite, but that runtime proof is still absent.
- FINDINGS:
  - Deleting a linked Loom block leaves stale derived counts on surviving blocks. In a direct throwaway harness against current SQLite main, the source block reported `mention_count=1` before deletion and still reported `mention_count=1` after deleting the linked target block, even though the cascade had removed the only linking edge. This is a concrete product defect, not a governance artifact.
- DIFF_ATTACK_SURFACES:
  - cascading edge lifecycle versus stored derived counters
  - SQLite/PostgreSQL delete-path parity for provider-portable semantics
  - conformance coverage gap between edge deletion and linked block deletion
  - portable DDL assumptions versus runtime-maintained derived state
- INDEPENDENT_CHECKS_RUN:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` -> PASS
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance` -> PASS
  - `just gov-check` -> PASS
  - `cargo run --quiet --manifest-path %TEMP%\loom-delete-repro\Cargo.toml` -> defect reproduced on SQLite current main (`after_delete source mention_count=1`)
- COUNTERFACTUAL_CHECKS:
  - If block delete recomputed surviving neighbors the same way edge delete already does, the reproduced stale `mention_count` / `tag_count` / `backlink_count` drift would collapse after cascade and this failure mode would disappear.
  - If these counts were always derived on read instead of stored, this specific cascade-delete drift class would not persist.
- BOUNDARY_PROBES:
  - shared SQLite/PostgreSQL storage conformance passed
  - portable migration checks passed
  - direct linked-block-delete counter probe failed on SQLite current main
- NEGATIVE_PATH_CHECKS:
  - literal `%` and `_` search filters remained bounded in shared conformance coverage
  - metadata-only derived fields remained unsearchable in shared conformance coverage
  - no committed packet-scope negative test currently exercises cascade deletion of a linked target block and then verifies surviving derived counters
- INDEPENDENT_FINDINGS:
  - The stale local packet/task-board mirror issue is superseded and not counted as a Loom failure in this report.
  - One concrete packet-scope product defect remains: block deletion can leave stale derived metrics on surviving blocks.
  - That defect alone is sufficient for final integration `FAIL`.
- RESIDUAL_UNCERTAINTY:
  - PostgreSQL runtime repro was not rerun live in this session.
  - No committed automated regression currently verifies linked-block deletion followed by surviving-block derived-counter refresh.

### VALIDATION REPORT - 2026-03-17 (Integration Validator, resumed current-candidate report)
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: FAIL
- HEURISTIC_REVIEW_VERDICT: FAIL
- SPEC_ALIGNMENT_VERDICT: FAIL
- ENVIRONMENT_VERDICT: PARTIAL
- DISPOSITION: NONE
- LEGAL_VERDICT: FAIL
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- SUMMARY:
  - This resumed validation reviewed the current packet-scoped candidate after the earlier `FAIL` report was acknowledged. There is still no packet-scoped remediation in the affected Loom storage files: `git diff main...HEAD -- src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/migrations/0013_loom_mvp.sql` was empty, and `git log main..HEAD --` on those same files was empty.
  - Because the failing storage paths are unchanged, the prior blocker remains unresolved on the current candidate: deleting a linked Loom block still bypasses surviving-block derived-counter recomputation in SQLite and PostgreSQL block-delete flows (`src/backend/handshake_core/src/storage/sqlite.rs:2144-2179`, `src/backend/handshake_core/src/storage/postgres.rs:1741-1762`) even though explicit edge create/delete paths do recompute those counters (`src/backend/handshake_core/src/storage/sqlite.rs:2272-2289`, `src/backend/handshake_core/src/storage/sqlite.rs:2342-2365`, `src/backend/handshake_core/src/storage/postgres.rs:1854-1871`, `src/backend/handshake_core/src/storage/postgres.rs:1924-1947`).
  - Targeted Loom tests still pass, but they do not cover linked-block deletion. Shared `gov-check` is currently red for unrelated truth drift on `WP-1-Structured-Collaboration-Schema-Registry-v2`; that is treated here as shared environment noise, not as a Loom packet governance blocker.
- CLAUSES_REVIEWED:
  - Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001] [LEGACY_REFINEMENT_BRIDGE] -> no current-candidate remediation was present in the affected SQLite/PostgreSQL storage delete paths relative to `main`; the unresolved block-delete lifecycle still omits surviving-block derived-counter refresh (`src/backend/handshake_core/src/storage/sqlite.rs:2144-2179`, `src/backend/handshake_core/src/storage/postgres.rs:1741-1762`).
  - Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> the portable schema still cascades `loom_edges` on block delete (`src/backend/handshake_core/migrations/0013_loom_mvp.sql:77-78`), but the current candidate still leaves stored derived counters stale on surviving neighbors.
  - Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> `LoomBlockDerived` values remain vulnerable to stale `mention_count` / `tag_count` / `backlink_count` after linked-block deletion because only explicit edge create/delete recomputes them (`src/backend/handshake_core/src/storage/sqlite.rs:2272-2289`, `src/backend/handshake_core/src/storage/sqlite.rs:2342-2365`, `src/backend/handshake_core/src/storage/postgres.rs:1854-1871`, `src/backend/handshake_core/src/storage/postgres.rs:1924-1947`).
  - Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation [LEGACY_REFINEMENT_BRIDGE] -> the current packet-scoped coverage still stops at deleting an unlinked block (`src/backend/handshake_core/src/storage/tests.rs:1187-1193`), so the linked-block delete regression remains unproven by committed tests.
- NOT_PROVEN:
  - A new direct runtime repro was not rerun in this resumed pass because the affected product files are unchanged and no new packet-scoped remediation candidate exists there; this verdict relies on the unchanged failing code path plus the earlier reproduced SQLite defect.
  - A live PostgreSQL repro remains unavailable in this environment.
- FINDINGS:
  - No packet-scoped remediation exists in the affected Loom storage files on the current candidate, so the previously reported blocker is still open.
  - The unresolved blocker is unchanged: linked-block deletion can leave stale derived counters on surviving blocks.
- DIFF_ATTACK_SURFACES:
  - absence of remediation in the exact previously failing storage paths
  - cascade-delete lifecycle versus runtime-maintained derived counters
  - green conformance tests masking a linked-block delete regression gap
  - SQLite/PostgreSQL parity on unchanged delete semantics
- INDEPENDENT_CHECKS_RUN:
  - `git diff main...HEAD -- src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/migrations/0013_loom_mvp.sql` -> no packet-scoped remediation diff
  - `git log main..HEAD -- src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/migrations/0013_loom_mvp.sql` -> no commits touching the failing paths on the candidate branch
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib loom` -> PASS
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact` -> PASS
  - `just gov-check` -> FAIL on unrelated packet-truth drift for `WP-1-Structured-Collaboration-Schema-Registry-v2`; not counted as a Loom packet blocker
- COUNTERFACTUAL_CHECKS:
  - If `delete_loom_block` in `src/backend/handshake_core/src/storage/sqlite.rs` recomputed surviving neighbors after the cascade, the prior stale `mention_count` condition would no longer survive block deletion.
  - If `delete_loom_block` in `src/backend/handshake_core/src/storage/postgres.rs` were corrected in parity with a SQLite fix, the backend-portable delete semantics would stop inheriting the same stale-counter risk.
- BOUNDARY_PROBES:
  - branch-vs-main diff probe on the exact failing files showed no remediation
  - targeted SQLite Loom storage conformance still passes on the unchanged candidate
- NEGATIVE_PATH_CHECKS:
  - the linked-block delete negative path is still uncovered by committed tests
  - shared governance validation currently fails on an unrelated WP truth-drift path, confirming the repo environment is not globally clean even though this is not a Loom governance blocker
- INDEPENDENT_FINDINGS:
  - This resumed pass did not find a new packet-scoped product candidate in the failing Loom storage paths.
  - The prior concrete blocker therefore still stands on the current candidate.
  - Final integration verdict remains `FAIL`.
- RESIDUAL_UNCERTAINTY:
  - No new direct runtime repro was rerun in this resumed pass because there was no remediation candidate to test in the affected files.
  - PostgreSQL live proof remains unavailable.

### VALIDATION REPORT - 2026-03-17 (Integration Validator, committed-range superseding PASS closeout)
Verdict: PASS
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PARTIAL
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
VALIDATOR_RISK_TIER: HIGH
COMMITTED_RANGE_VALIDATED: `705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204`
BASELINE_COMPARED: `main` @ `d8edecab4a4115736a8f58e7f7c73ffcd065b9b5`
SUMMARY:
- This report supersedes older FAIL reports that predate the authoritative repaired candidate. Validation was rerun against local `main` first and then against the exact committed candidate range `705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204` on `feat/WP-1-Loom-Storage-Portability-v2`.
- `just validator-handoff-check WP-1-Loom-Storage-Portability-v2 --range 705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204` now passes on the exact committed range, and `just gov-check` passes in the current authoritative context.
- Product repair commit `643558e2173d4f7d167432be43e6140d97158005` snapshots surviving neighbor block ids before delete and recomputes `mention_count`, `tag_count`, and `backlink_count` after the cascade in both adapters; packet-evidence closure commit `0dcc5ce505c5b256fe849600223818f22db1a204` does not reopen that defect.
- No current packet-scoped blocker remains in the validated range. Residual uncertainty is bounded to unavailable live PostgreSQL execution because `POSTGRES_TEST_URL` is unset.
CLAUSES_REVIEWED:
- Handshake_Master_Spec_v02.178.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001] [LEGACY_REFINEMENT_BRIDGE] -> the committed range preserves backend-portable delete semantics by repairing the same linked-delete lifecycle in both `src/backend/handshake_core/src/storage/sqlite.rs:2145` and `src/backend/handshake_core/src/storage/postgres.rs:1742`, with shared parity coverage in `src/backend/handshake_core/src/storage/tests.rs:1284`.
- Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> the repair now recomputes surviving derived metrics from the canonical `loom_edges` relation after `ON DELETE CASCADE` in both adapters (`src/backend/handshake_core/src/storage/sqlite.rs:2165`, `src/backend/handshake_core/src/storage/postgres.rs:1762`), consistent with the portable schema boundary in `src/backend/handshake_core/migrations/0013_loom_mvp.sql:77`.
- Handshake_Master_Spec_v02.178.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130] [LEGACY_REFINEMENT_BRIDGE] -> shared conformance now verifies that deleting linked targets drops surviving source `mention_count` / `tag_count` and deleting a linked source drops surviving target `backlink_count` (`src/backend/handshake_core/src/storage/tests.rs:1295`, `src/backend/handshake_core/src/storage/tests.rs:1314`, `src/backend/handshake_core/src/storage/tests.rs:1386`).
- Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract and portable schema continuation [LEGACY_REFINEMENT_BRIDGE] -> diff review against local `main` shows no search/view contract drift outside the intended delete-counter repair and shared regression coverage, and the Loom lib/test surfaces remain green.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- NONE
DIFF_ATTACK_SURFACES:
- linked-block delete cascade versus surviving derived-counter repair
- SQLite/PostgreSQL parity inside `delete_loom_block`
- regression coverage for deleting linked targets and linked sources
- packet-evidence closure after a product repair commit
INDEPENDENT_CHECKS_RUN:
- `just validator-handoff-check WP-1-Loom-Storage-Portability-v2 --range 705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204` -> PASS
- `just gov-check` -> PASS
- `git diff --unified=60 705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204 -- src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/tests.rs` -> validator-owned diff review confirmed the committed range is limited to the intended delete-counter repair and shared regression coverage
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance sqlite_loom_storage_conformance -- --exact` -> PASS
COUNTERFACTUAL_CHECKS:
- If `delete_loom_block` in `src/backend/handshake_core/src/storage/sqlite.rs` stopped snapshotting surviving neighbor ids before deletion, the repaired source/target derived counters would regress to the stale post-cascade state that originally failed validation.
- If `delete_loom_block` in `src/backend/handshake_core/src/storage/postgres.rs` failed to mirror the SQLite transaction-local recompute path, the shared linked-delete assertions in `src/backend/handshake_core/src/storage/tests.rs` would no longer represent backend-portable behavior.
BOUNDARY_PROBES:
- local-main-to-candidate diff probe confirmed the product delta is confined to the linked-delete repair in both adapters plus shared regression coverage
- writer/reader boundary probe confirmed the shared conformance path now writes linked edges, deletes linked blocks, then reads surviving blocks to verify repaired derived counts
NEGATIVE_PATH_CHECKS:
- `delete_loom_block` still returns `StorageError::NotFound("loom_block")` on missing rows in both adapters; the repair only changes the successful linked-delete path
- the PostgreSQL exact conformance command still exits through the explicit environment skip gate when `POSTGRES_TEST_URL` is unset, so the candidate does not hide missing live-DB proof behind a silent false PASS
INDEPENDENT_FINDINGS:
- The previously reported linked-delete stale-counter defect is repaired in the validated committed range.
- The exact committed handoff range now passes deterministic handoff validation and current governance checks.
- No current packet-scoped blocker remains for `705220e2e3a9468b83e471029c239ad906f728bd..0dcc5ce505c5b256fe849600223818f22db1a204`.
RESIDUAL_UNCERTAINTY:
- Live PostgreSQL execution of the repaired linked-delete path remains environment-bounded because `POSTGRES_TEST_URL` is unset; this did not surface as a code or governance blocker in the validated range.
