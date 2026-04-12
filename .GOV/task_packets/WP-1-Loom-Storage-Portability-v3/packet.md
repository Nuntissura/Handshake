# Task Packet: WP-1-Loom-Storage-Portability-v3

## METADATA
- TASK_ID: WP-1-Loom-Storage-Portability-v3
- WP_ID: WP-1-Loom-Storage-Portability-v3
- BASE_WP_ID: WP-1-Loom-Storage-Portability
- DATE: 2026-03-19T08:32:34.441Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Loom-Storage-Portability-v3
<!-- Validator roles keep distinct local branches/worktrees, but they mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Loom-Storage-Portability-v3
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-storage-portability-v3
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v3
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v3
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Loom-Storage-Portability-v3
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v3
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v3
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Loom-Storage-Portability-v3
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Loom-Storage-Portability-v3
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Loom-Storage-Portability-v3
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Blocked
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
- LOCAL_BRANCH: feat/WP-1-Loom-Storage-Portability-v3
- LOCAL_WORKTREE_DIR: ../wtc-storage-portability-v3
- REMOTE_BACKUP_BRANCH: feat/WP-1-Loom-Storage-Portability-v3
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Loom-Storage-Portability-v3
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Loom-Storage-Portability-v3/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-loom-storage-portability-v3
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- USER_SIGNATURE: ilja190320260922
- PACKET_FORMAT_VERSION: 2026-03-18

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: FAIL
Blockers: LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED. The 2026-03-21 product-vs-spec audit did not find a fresh Loom defect comparable to the Schema Registry gaps, but it also did not support keeping this v3 packet as live validated closure under current governance law.
Next: NONE. Do not resume or re-prepare this packet in place. Any future Loom portability remediation moves to `WP-1-Loom-Storage-Portability-v4`.

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [LM-GRAPH-001] Graph traversal with recursive CTEs on both backends | CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | TESTS: storage_conformance::loom_traverse_graph_depth_limit, storage_conformance::loom_traverse_graph_cycle_detection | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [LM-SEARCH-002] PostgreSQL search filterable by graph relationships | CODE_SURFACES: storage/loom.rs (LoomSearchFilters), postgres.rs (search_loom_blocks) | TESTS: storage_conformance::loom_search_graph_filter_postgres | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 2.3.13.7 get_backlinks and get_outgoing_edges | CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | TESTS: storage_conformance::loom_directional_edge_queries | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 2.3.13.7 recompute_block_metrics and recompute_all_metrics | CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | TESTS: storage_conformance::loom_metrics_recompute_idempotent | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: [CX-DBP-011] Portable schema and migrations | CODE_SURFACES: migrations/ | TESTS: cargo test -p handshake_core loom migration | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 2.3.13.7 LoomSourceAnchor export/replay durability | CODE_SURFACES: storage/loom.rs, sqlite.rs, postgres.rs | TESTS: storage_conformance::loom_source_anchor_round_trip | EXAMPLES: Golden traverse_graph result for a known 3-hop graph on 10+ blocks, Golden LoomSourceAnchor JSON that must round-trip identically through both backends, Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test -p handshake_core loom
  - cargo test -p handshake_core --test storage_conformance
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - storage_conformance::loom_traverse_graph_depth_limit -- proves traverse_graph stops at max_depth on both backends
  - storage_conformance::loom_traverse_graph_cycle_detection -- proves cycle detection prevents infinite loops
  - storage_conformance::loom_traverse_graph_edge_type_filter -- proves edge_types parameter filters correctly
  - storage_conformance::loom_search_graph_filter_postgres -- proves LM-SEARCH-002 graph-relationship filtering on PostgreSQL
  - storage_conformance::loom_directional_edge_queries -- proves get_backlinks and get_outgoing_edges return correct subsets
  - storage_conformance::loom_metrics_recompute_idempotent -- proves recompute is idempotent and matches fresh computation
  - storage_conformance::loom_source_anchor_round_trip -- proves LoomSourceAnchor survives write/read/export/reimport on both backends
- CANONICAL_CONTRACT_EXAMPLES:
  - Golden traverse_graph result for a known 3-hop graph on 10+ blocks
  - Golden LoomSourceAnchor JSON that must round-trip identically through both backends
  - Golden LoomSearchFilters with graph-relationship fields for PostgreSQL-specific test
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
- REFINEMENT_FILE: .GOV/task_packets/WP-1-Loom-Storage-Portability-v3/refinement.md
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
- SPEC_ANCHOR: Handshake_Master_Spec_v02.178.md Loom search API backend-agnostic contract, graph traversal, and metrics
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
- CLAUSE_ROWS:
  - CLAUSE: [LM-GRAPH-001] Graph traversal with recursive CTEs on both backends | WHY_IN_SCOPE: spec requires traverse_graph with performance targets; method is completely absent from current code | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_traverse_graph_depth_limit, storage_conformance::loom_traverse_graph_cycle_detection | RISK_IF_MISSED: graph navigation and backlink exploration have no backend-portable foundation
  - CLAUSE: [LM-SEARCH-002] PostgreSQL search filterable by graph relationships | WHY_IN_SCOPE: spec requires PostgreSQL search to support tag/mention/backlink-depth filtering; current LoomSearchFilters lacks these fields | EXPECTED_CODE_SURFACES: storage/loom.rs (LoomSearchFilters), postgres.rs (search_loom_blocks) | EXPECTED_TESTS: storage_conformance::loom_search_graph_filter_postgres | RISK_IF_MISSED: PostgreSQL search lacks spec-mandated graph-aware filtering capability
  - CLAUSE: 2.3.13.7 get_backlinks and get_outgoing_edges | WHY_IN_SCOPE: spec defines two directional edge query methods; current code only has list_loom_edges_for_block | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_directional_edge_queries | RISK_IF_MISSED: downstream consumers cannot distinguish incoming from outgoing edges portably
  - CLAUSE: 2.3.13.7 recompute_block_metrics and recompute_all_metrics | WHY_IN_SCOPE: spec requires rebuildable derived metrics methods; absent from trait and all implementations | EXPECTED_CODE_SURFACES: storage/mod.rs (trait), sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_metrics_recompute_idempotent | RISK_IF_MISSED: derived metrics become migration-coupled state instead of rebuildable
  - CLAUSE: [CX-DBP-011] Portable schema and migrations | WHY_IN_SCOPE: any new Loom DDL must follow portable SQL rules; existing migrations must stay replay-safe | EXPECTED_CODE_SURFACES: migrations/ | EXPECTED_TESTS: cargo test -p handshake_core loom migration | RISK_IF_MISSED: new methods introduce SQLite-specific or Postgres-specific schema that breaks portability
  - CLAUSE: 2.3.13.7 LoomSourceAnchor export/replay durability | WHY_IN_SCOPE: spec requires source anchors to survive export and replay across backends; no conformance test exists | EXPECTED_CODE_SURFACES: storage/loom.rs, sqlite.rs, postgres.rs | EXPECTED_TESTS: storage_conformance::loom_source_anchor_round_trip | RISK_IF_MISSED: source-anchor provenance is silently lost on backend swap or export/reimport
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Storage trait LoomStorage methods | PRODUCER: sqlite.rs, postgres.rs | CONSUMER: api/loom.rs, workflows.rs | SERIALIZER_TRANSPORT: in-process Rust trait dispatch | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_* | DRIFT_RISK: trait method signatures drift between trait declaration and provider implementations
  - CONTRACT: LoomSearchFilters struct | PRODUCER: api/loom.rs (from HTTP params) | CONSUMER: sqlite.rs, postgres.rs (search_loom_blocks) | SERIALIZER_TRANSPORT: serde JSON | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_search_filter_parity | DRIFT_RISK: graph-relationship filter fields added to struct but not wired in one backend
  - CONTRACT: LoomSourceAnchor struct | PRODUCER: api/loom.rs (edge creation) | CONSUMER: sqlite.rs, postgres.rs (edge storage), export/replay paths | SERIALIZER_TRANSPORT: serde JSON to DB columns | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_source_anchor_round_trip | DRIFT_RISK: anchor fields lost during serialization on one backend
  - CONTRACT: traverse_graph result shape | PRODUCER: sqlite.rs, postgres.rs | CONSUMER: api/loom.rs, future graph navigation UI | SERIALIZER_TRANSPORT: in-process Vec<(LoomBlock, u32)> | VALIDATOR_READER: storage_conformance tests | TRIPWIRE_TESTS: storage_conformance::loom_traverse_graph_* | DRIFT_RISK: depth values or cycle handling differs between backends
  - CONTRACT: Loom migration DDL | PRODUCER: migrations/*.sql | CONSUMER: sqlx migrate | SERIALIZER_TRANSPORT: SQL files | VALIDATOR_READER: migration replay test | TRIPWIRE_TESTS: cargo test migration_replay | DRIFT_RISK: new columns or indexes use provider-specific syntax
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add traverse_graph, recompute_block_metrics, recompute_all_metrics, get_backlinks, get_outgoing_edges to Storage trait in mod.rs
  - Implement all 5 methods in sqlite.rs using recursive CTEs and application-layer logic
  - Implement all 5 methods in postgres.rs using recursive CTEs with PostgreSQL optimizations
  - Add graph-relationship filter fields to LoomSearchFilters in loom.rs
  - Wire graph-filtered search in postgres.rs search_loom_blocks implementation
  - Add API endpoints for graph traversal and metrics recomputation in api/loom.rs
  - Write shared Loom conformance tests covering all new methods and source-anchor round-trips
  - Verify all existing Loom tests still pass
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- TRIPWIRE_TESTS:
  - cargo test -p handshake_core loom
  - cargo test -p handshake_core --test storage_conformance
- CARRY_FORWARD_WARNINGS:
  - v2 passed validator but failed operator code inspection; v3 must close gaps with actual implementation, not narrative
  - Do not widen scope beyond Loom storage/API surface; keep file-lock isolation
  - Performance targets from spec: traverse_graph <100ms 3-hop on 10K blocks (SQLite), <50ms (PostgreSQL)
  - LM-SEARCH-002 graph filtering is PostgreSQL-only; SQLite search keeps existing behavior
  - All new methods must follow portable SQL rules (no strftime, no SQLite triggers, $1/$2 placeholders)
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - [LM-GRAPH-001] traverse_graph exists on trait + both implementations with recursive CTE
  - [LM-SEARCH-002] PostgreSQL search_loom_blocks accepts and uses graph-relationship filters
  - 2.3.13.7 get_backlinks and get_outgoing_edges exist as separate trait methods
  - 2.3.13.7 recompute_block_metrics and recompute_all_metrics exist on trait + both implementations
  - [CX-DBP-011] Any new migrations follow portable SQL rules
  - LoomSourceAnchor round-trip proof in conformance tests
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core loom
  - cargo test -p handshake_core --test storage_conformance
  - rg -n "traverse_graph|recompute_block_metrics|recompute_all_metrics|get_backlinks|get_outgoing_edges" src/backend/handshake_core/src/storage/
- POST_MERGE_SPOTCHECKS:
  - Verify traverse_graph recursive CTE syntax is portable (no provider-specific extensions)
  - Verify LoomSearchFilters graph fields are present in struct definition
  - Verify conformance tests run against both SQLite and PostgreSQL
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Exact traverse_graph performance on 10K-block datasets has not been benchmarked; spec targets are stated but actual performance depends on index strategy and query plan
  - LM-SEARCH-002 graph-filtered search may require additional Postgres indexes beyond current schema; exact index requirements are implementation-dependent
  - LoomSourceAnchor round-trip fidelity through external export/reimport paths (beyond direct DB write/read) depends on export format decisions not yet finalized in the codebase
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [OSS_DOC] SQLite FTS5 docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://sqlite.org/fts5.html | Why: canonical reference for SQLite-side full-text search behavior and index locality
  - [OSS_DOC] PostgreSQL full text search docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://www.postgresql.org/docs/current/textsearch.html | Why: canonical reference for PostgreSQL-side ranked text search and backend-specific query power
  - [BIG_TECH] Google Cloud Spanner full-text search overview | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://cloud.google.com/spanner/docs/full-text-search | Why: current large-scale vendor reference showing richer backend-specific search features can exist behind one SQL-facing search surface
  - [OSS_DOC] OpenLineage spec docs | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://openlineage.io/docs/spec/ | Why: useful reference for typed lineage and provenance payloads that survive transport and backend changes
  - [GITHUB] OpenLineage repository | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://github.com/OpenLineage/OpenLineage | Why: concrete repository-scale example of typed lineage/provenance contract evolution
  - [PAPER] In-Memory Indexing and Querying of Provenance in Data Preparation Pipelines | 2025-11-05 | Retrieved: 2026-03-19T05:55:23Z | https://arxiv.org/abs/2511.03480 | Why: recent provenance-indexing paper supporting explicit, queryable source/provenance structures instead of opaque backend-local metadata
  - [GITHUB] pgvector repository | 2026-03-13 | Retrieved: 2026-03-19T05:55:23Z | https://github.com/pgvector/pgvector | Why: high-signal reference for backend-specific search acceleration that should remain optional rather than canonical in this packet
  - [OSS_DOC] SQLite recursive CTE docs | 2026-03-19 | Retrieved: 2026-03-19T05:55:23Z | https://sqlite.org/lang_with.html | Why: canonical reference for recursive CTE syntax and performance characteristics on SQLite 3.35+
  - [OSS_DOC] PostgreSQL recursive CTE docs | 2026-03-19 | Retrieved: 2026-03-19T05:55:23Z | https://www.postgresql.org/docs/current/queries-with.html | Why: canonical reference for recursive CTE graph traversal performance on PostgreSQL
- RESEARCH_SYNTHESIS:
  - A portability packet should preserve one stable API and semantic contract while allowing provider-specific indexing and query plans behind the boundary.
  - Big-tech search systems confirm that richer backend-specific ranking, tokenization, and query expansion can stay behind a stable query surface instead of redefining canonical filter meaning.
  - Typed provenance payloads are more durable than ad hoc search or edge metadata and map well to Loom `source_anchor` export/replay expectations.
  - Recursive CTEs are well-supported on both SQLite (3.35+) and PostgreSQL with predictable depth-tracking semantics, making graph traversal portable without provider-specific branching.
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
  - SQLite recursive CTE docs -> ADOPT (IN_THIS_WP)
  - PostgreSQL recursive CTE docs -> ADOPT (IN_THIS_WP)
  - OpenLineage spec docs -> ADAPT (IN_THIS_WP)
  - pgvector repository -> REJECT (REJECT_DUPLICATE)
- MATRIX_GROWTH_CANDIDATES:
  - Stable search API plus provider-local indexing with graph filtering -> IN_THIS_WP (stub: NONE)
  - Portable graph traversal with recursive CTEs -> IN_THIS_WP (stub: NONE)
  - Portable source-anchor lineage plus export/replay durability -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Keep provider-specific FTS and ranking logic inside backend implementations, not in portable migrations.
  - Assert semantic parity through shared Loom conformance tests rather than comparing SQL query text.
  - Treat `LoomSourceAnchor` and view/search filters as portable contract structs, not adapter-only shapes.
  - Use WITH RECURSIVE for graph traversal on both backends with explicit depth counter and max_depth guard.
  - Implement rebuildable metrics as application-layer recomputation, not migration-coupled state.
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
  - Graph traversal with portable recursive CTEs -> IN_THIS_WP (stub: NONE)
  - Rebuildable derived metrics plus provider-local indexes -> IN_THIS_WP (stub: NONE)
  - Directional edge queries (backlinks + outgoing) -> IN_THIS_WP (stub: NONE)
  - SQLite FTS and PostgreSQL text search behind one API with graph filtering -> IN_THIS_WP (stub: NONE)
  - View filter parity across providers -> IN_THIS_WP (stub: NONE)
  - Source-anchor durability across storage, export, and replay -> IN_THIS_WP (stub: NONE)
  - Shared Loom conformance tests over SQLite and PostgreSQL -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Loom | CAPABILITY_SLICE: Portable block and edge record parity | SUBFEATURES: `LoomBlock`, `LoomEdge`, content-hash dedup, metrics rebuildability, and stable backend-neutral meaning | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived, PRIM-LoomEdgeType | MECHANICAL: engine.archivist, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the packet should guarantee that block and edge semantics survive SQLite and PostgreSQL backends without adapter drift
  - PILLAR: Loom | CAPABILITY_SLICE: Graph traversal and metrics recomputation | SUBFEATURES: `traverse_graph` with recursive CTE, `recompute_block_metrics`, `recompute_all_metrics`, `get_backlinks`, `get_outgoing_edges` | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomBlockDerived | MECHANICAL: engine.archivist, engine.dba, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: these are spec-required trait methods currently missing from both the trait definition and all implementations
  - PILLAR: Loom | CAPABILITY_SLICE: Portable view, search, and source-anchor contract | SUBFEATURES: `LoomViewFilters`, `LoomSearchFilters`, `LoomBlockSearchResult`, and `LoomSourceAnchor` parity; graph-filtered search on PostgreSQL | PRIMITIVES_FEATURES: PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomBlockSearchResult, PRIM-LoomSourceAnchor | MECHANICAL: engine.librarian, engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the API contract should preserve the same filter meaning and source-anchor durability across both backends; PostgreSQL search must support graph-relationship filters per LM-SEARCH-002
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Loom migration and DDL portability | SUBFEATURES: replay-safe migrations, down migrations, provider-local indexes outside portable DDL, and no trigger-dependent semantics | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomSourceAnchor | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the direct portability law bridge from spec to code for the Loom surface
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Cross-provider Loom conformance coverage | SUBFEATURES: shared test helpers for SQLite and PostgreSQL parity over CRUD, search, view, dedup, graph traversal, metrics, and anchor round-trips | PRIMITIVES_FEATURES: PRIM-LoomBlock, PRIM-LoomEdge, PRIM-LoomViewFilters, PRIM-LoomSearchFilters, PRIM-LoomSourceAnchor | MECHANICAL: engine.archivist, engine.librarian, engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity should be proven by tests, not inferred from provider implementations
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Loom block and edge storage parity | JobModel: UI_ACTION | Workflow: loom_surface_crud | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_block_created, loom_block_updated, loom_block_deleted, loom_edge_created, loom_edge_deleted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend parity must preserve CRUD meaning and existing Loom telemetry regardless of provider implementation
  - Capability: Loom graph traversal | JobModel: UI_ACTION | Workflow: loom_graph_traverse | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_graph_traversed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: recursive CTE traversal must meet spec performance targets on both backends; results drive backlink navigation and graph exploration
  - Capability: Loom metrics recomputation | JobModel: MECHANICAL_TOOL | Workflow: loom_metrics_recompute | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_metrics_recomputed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: rebuildable derived metrics must not become migration-coupled state
  - Capability: Loom import and dedup portability | JobModel: WORKFLOW | Workflow: loom_import | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_dedup_hit, loom_block_created | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: content-hash dedup, asset-path layout, and import-created blocks must preserve the same semantics across backends
  - Capability: Loom view portability | JobModel: UI_ACTION | Workflow: loom_view_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: loom_view_queried | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `LoomViewFilters` and grouped-view semantics must not drift when the backend changes
  - Capability: Loom search portability with graph filtering | JobModel: UI_ACTION | Workflow: loom_search | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: loom_search_executed | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: PostgreSQL search must support LM-SEARCH-002 graph-relationship filtering; SQLite search preserves stable filter meaning
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Media-Downloader-Loom-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Video-Archive-Loom-Integration-v1 -> KEEP_SEPARATE
  - WP-1-Loom-Preview-VideoPosterFrames-v1 -> KEEP_SEPARATE
  - WP-1-Loom-MVP-v1 -> KEEP_SEPARATE
  - WP-1-Storage-Abstraction-Layer-v3 -> KEEP_SEPARATE
  - WP-1-Artifact-System-Foundations-v1 -> KEEP_SEPARATE
  - WP-1-Loom-Storage-Portability-v2 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Loom-Storage-Portability-v2)
  - src/backend/handshake_core/src/storage/mod.rs -> IMPLEMENTED (WP-1-Loom-Storage-Portability-v2)
  - src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Storage-Abstraction-Layer-v3)
  - src/backend/handshake_core/src/storage/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/sqlite.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/postgres.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/storage/tests.rs -> NOT_PRESENT (NONE)
  - src/backend/handshake_core/src/api/loom.rs -> PARTIAL (WP-1-Loom-MVP-v1)
  - src/backend/handshake_core/src/loom_fs.rs -> PARTIAL (NONE)
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
- What: Implement the 5 missing Loom storage trait methods (traverse_graph, recompute_block_metrics, recompute_all_metrics, get_backlinks, get_outgoing_edges), add LM-SEARCH-002 graph-filtered search on PostgreSQL, and prove LoomSourceAnchor export/replay round-trip parity across both backends with shared conformance tests.
- Why: v2 passed validator but operator code inspection against spec v02.178 revealed 5 missing trait methods, incomplete graph-filtered search, and absent source-anchor round-trip proof. This v3 closes those concrete spec compliance gaps.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests/storage_conformance.rs
  - src/backend/handshake_core/migrations/
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
cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
```

### DONE_MEANS
- `traverse_graph` trait method exists on Storage trait with SQLite (recursive CTE) and PostgreSQL implementations meeting spec performance targets (<100ms 3-hop on 10K blocks SQLite, <50ms PostgreSQL).
- `recompute_block_metrics` and `recompute_all_metrics` trait methods exist on Storage trait with both backend implementations.
- `get_backlinks` and `get_outgoing_edges` trait methods exist as separate directional edge queries on Storage trait with both backend implementations.
- PostgreSQL `search_loom_blocks` supports graph-relationship filtering (tags, mentions, backlink depth) per LM-SEARCH-002.
- `LoomSourceAnchor` round-trip tests prove export/replay durability across both backends in shared conformance suite.
- Shared Loom conformance tests cover graph traversal, metrics recomputation, directional edges, graph-filtered search, and source-anchor round-trips.
- No unrelated product families are touched; changes stay inside Loom storage/API/test surface.

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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-19T08:32:34.441Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.156]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.178.md 2.3.13.7 Loom Storage Trait + Portable Schema (Example) [ADD v02.130]
- Codex: .GOV/codex/Handshake_Codex_v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Roadmap/spec/repo audit basis:
  - Base WP traceability anchor: `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` marks `WP-1-Loom-Storage-Portability-v3` as the active remediation packet for `WP-1-Loom-Storage-Portability`.
  - Governing Main Body scope is unchanged across variants: portable Loom storage trait behavior, portable schema/migrations, backend-parity search/view semantics, source-anchor durability, and dual-backend conformance remain mandatory.
  - Current repo-code audit baseline for v3 is the signed refinement plus direct inspection of current `main`/merge-base state; prior validator PASS claims are treated as non-authoritative where code/spec inspection disproved closure.
- Prior packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v1.md`
  - Preserved into v3: the base WP intent to make Loom storage semantics portable across SQLite-now / PostgreSQL-ready backends without redefining canonical search, edge, or source-anchor meaning.
  - Preserved into v3: the packet family focus on storage trait, backend adapters, API surface, filesystem artifact layout, migrations, and conformance tests.
  - Changed in v3: v1 closure claims are no longer trusted as sufficient proof because later direct code inspection found missing graph-traversal and metrics APIs plus unproven portability obligations.
- Prior packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v2.md`
  - Preserved into v3: the remediation posture against the v1 false-closeout and the shared-surface emphasis on storage trait boundaries, migration portability, and backend conformance tests.
  - Changed in v3: the missing requirements are promoted into explicit clause-closure rows and semantic-proof assets so the coder/validator must prove parity instead of relying on generic Loom test passes.
  - Changed in v3: the signed refinement narrows the active repo-code gap set to six concrete unclosed obligations: `traverse_graph`, `recompute_block_metrics`, `recompute_all_metrics`, `get_backlinks`, `get_outgoing_edges`, PostgreSQL graph-filtered search per `LM-SEARCH-002`, and LoomSourceAnchor round-trip proof across backends/export-replay.
- Carry-forward verdict:
  - No prior governing requirement is dropped in v3.
  - v3 supersedes v1/v2 closure claims but preserves their valid scope while replacing under-proven or incorrect completion assumptions with explicit current-main gap statements and test/example obligations.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.178.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/loom.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/loom_fs.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - traverse_graph
  - recompute_block_metrics
  - recompute_all_metrics
  - get_backlinks
  - get_outgoing_edges
  - create_loom_block
  - create_loom_edge
  - query_loom_view
  - search_loom_blocks
  - LoomSourceAnchor
  - LoomSearchFilters
  - loom_blocks
  - loom_edges
  - loom_blocks_fts
- RUN_COMMANDS:
  ```bash
rg -n "traverse_graph|recompute_block_metrics|recompute_all_metrics|get_backlinks|get_outgoing_edges|create_loom_block|search_loom_blocks|LoomSourceAnchor|LoomSearchFilters" src/backend/handshake_core
  cargo test -p handshake_core loom
  cargo test -p handshake_core --test storage_conformance
  just gov-check
  ```
- RISK_MAP:
  - "traverse_graph recursive CTE hits cycle or exceeds depth" -> "unbounded result sets or infinite loops in graph navigation"
  - "provider-specific search logic changes filter meaning" -> "view and search parity break across SQLite and PostgreSQL"
  - "source anchors fail to round-trip on one backend" -> "backlinks, context snippets, and downstream bridge packets lose stable provenance"
  - "LM-SEARCH-002 graph-filtered search widens Postgres search beyond portable contract" -> "Postgres-only search semantics become de facto requirement"
  - "the remediation pass drifts into unrelated runtime families" -> "the packet loses file-lock isolation and repeats the earlier live-smoke scope failure"
## SKELETON
- Proposed interfaces/types/contracts: Add a portable Loom graph-traversal method to the storage trait, implement it with recursive CTEs on both SQLite and PostgreSQL, and extend the shared Loom conformance suite to prove depth limits, cycle handling, and edge-type filtering across both backends before moving to later MTs for graph-filtered search, directional edge queries, metrics recomputation, migrations, and LoomSourceAnchor durability.
- Open questions: The first MT establishes the traversal contract, but duplicates, cycle revisit policy, and result ordering must be kept deterministic across both SQL backends so later search/filter features build on the same semantics.
- Notes: First executable MT is `MT-001 [LM-GRAPH-001] Graph traversal with recursive CTEs on both backends`. Keep the first pass storage-only; do not spread into unrelated Loom viewer or preview-runtime surfaces.

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
- Added the missing portable Loom graph-contract methods to the shared storage trait and implemented them on both SQLite and PostgreSQL: `get_backlinks`, `get_outgoing_edges`, `traverse_graph`, `recompute_block_metrics`, and `recompute_all_metrics`.
- Added `backlink_depth` to the public Loom search filters/API contract, added graph-traversal + metrics-recompute endpoints in `api/loom.rs`, and clamped graph depth at the API boundary so recursive search/traversal stays bounded.
- Reworked the new backend traversal filters to use parameterized edge-type filters on both SQLite and PostgreSQL rather than string-built `IN (...)` clauses.
- Added shared conformance helpers/tests for graph depth limiting, cycle detection, edge-type filtering, directional edge queries, metrics recomputation, and LoomSourceAnchor export/replay durability across both backends.
- Added explicit 10K-block, 3-hop traversal performance probes on both backends and wired them into `tests/storage_conformance.rs` so the packet's `<100ms` SQLite / `<50ms` PostgreSQL target is exercised directly.
- Tightened the PostgreSQL search test input so the negative-path assertion proves the intended behavior: depth-1 filtering must not match an indirect tag path, while depth-3 may.

## HYGIENE
- Captured deterministic COR-701 LF-blob SHA pairs for every staged product file with `just cor701-sha ...`.
- Re-ran the Loom-scoped library, migration, API route, SQLite/PostgreSQL conformance, and SQLite/PostgreSQL traversal-performance commands from `src/backend/handshake_core`, storing full logs under `.handshake/logs/WP-1-Loom-Storage-Portability-v3/`.
- Kept the repair diff staged to 5 product files while preserving the earlier WP branch work; cumulative WP scope now spans 7 tracked Loom files including the new explicit traversal-performance test target.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/api/loom.rs`
- **Start**: 1
- **End**: 1508
- **Line Delta**: 408
- **Pre-SHA1**: `5c27b06d591058c14795ecefd1cafe3affcd5b18`
- **Post-SHA1**: `1c4f67e690bfaec4790e5b14669342de41206bad`
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
- **Lint Results**: Covered by Loom-scoped Rust test runs plus the dedicated `graph_traversal_and_metrics_routes_work` API test
- **Artifacts**: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_lib.log`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_api_routes.log`
- **Timestamp**: 2026-03-20T13:21:49.9252887+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Adds graph traversal + metrics recompute endpoints, clamps recursive depth at the API boundary, and proves the route surface with a dedicated handler-level test.
- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 2104
- **End**: 2183
- **Line Delta**: -7
- **Pre-SHA1**: `b7785113422f232f97a30d2b3e455cfebfa8a250`
- **Post-SHA1**: `d1b3b82d78a9fe77716cb7762449b8c2cc6ace88`
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
- **Lint Results**: Covered by Loom-scoped Rust test runs including real PostgreSQL conformance
- **Artifacts**: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_postgres.log`
- **Timestamp**: 2026-03-20T13:21:49.9252887+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Keeps the PostgreSQL traversal behavior but replaces raw string-built edge-type SQL with parameterized array binding for the recursive CTE path.
- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 2579
- **End**: 2667
- **Line Delta**: 11
- **Pre-SHA1**: `3f08f1d9c74a08ae920709ee123bd333dffc3e7f`
- **Post-SHA1**: `1b2af0d384fc9bd7d5f04679cb60a63c062bed59`
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
- **Lint Results**: Covered by Loom-scoped Rust test runs including SQLite conformance
- **Artifacts**: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_sqlite.log`
- **Timestamp**: 2026-03-20T13:21:49.9252887+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Mirrors the same traversal-filter hardening on SQLite by binding edge-type filters through `json_each(...)` instead of raw string interpolation.
- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 21
- **End**: 2225
- **Line Delta**: 218
- **Pre-SHA1**: `4f08a9f76a9951f27abf5b4fa5a11dcb4624cb24`
- **Post-SHA1**: `be0a61ca3a6f05cc015729a901ab952d911e5b6f`
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
- **Lint Results**: Covered by Loom-scoped Rust test runs including SQLite/PostgreSQL conformance and explicit traversal-performance probes
- **Artifacts**: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_sqlite.log`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_postgres.log`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_sqlite.log`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_postgres.log`
- **Timestamp**: 2026-03-20T13:21:49.9252887+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Adds the 10K-block traversal performance fixture/probe while retaining the shared graph, metrics, and source-anchor conformance coverage.
- **Target File**: `src/backend/handshake_core/tests/storage_conformance.rs`
- **Start**: 1
- **End**: 80
- **Line Delta**: 24
- **Pre-SHA1**: `654cec765edf820ee71d4b3663823110f9341632`
- **Post-SHA1**: `a3dd58a5209abdaf7647622d0320dcda44b6131a`
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
- **Lint Results**: Covered by dedicated SQLite/PostgreSQL traversal-performance targets
- **Artifacts**: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_sqlite.log`; `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_postgres.log`
- **Timestamp**: 2026-03-20T14:21:00+01:00
- **Operator**: Orchestrator
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.178.md
- **Notes**: Exposes the 10K-block traversal probes as explicit integration-test targets for both backends.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: Done; committed handoff validation passed at `7aa995b`, validator gates are closed PASS, the branch is pushed to `origin/feat/WP-1-Loom-Storage-Portability-v3`, and the finalized Loom product scope is integrated on `main` via selective commit `e867469`.
- What changed in this update: Added graph traversal + metrics recompute endpoints in `api/loom.rs`, clamped recursive depth at the API boundary, replaced raw string-built traversal edge-type SQL with parameterized filters on both backends, and added explicit 10K-block traversal-performance probes plus dedicated test targets for SQLite and PostgreSQL.
- Requirements / clauses self-audited: `[LM-GRAPH-001]` -> shared trait + both backend recursive CTE implementations remain intact, API traversal surface now exists, and 10K-block timing probes are added; `[LM-SEARCH-002]` -> API/filter contract keeps `backlink_depth`, now bounded at the API entry point, and PostgreSQL recursive graph filtering remains proven; `2.3.13.7 get_backlinks and get_outgoing_edges` -> unchanged backend/query proof remains valid; `2.3.13.7 recompute_block_metrics and recompute_all_metrics` -> API recompute endpoints now exercise the backend methods directly; `[CX-DBP-011] Portable schema and migrations` -> migration portability remains green; `2.3.13.7 LoomSourceAnchor export/replay durability` -> prior conformance proof remains green.
- Checks actually run: `cargo test loom --lib -- --nocapture`; `cargo test migration --lib -- --nocapture`; `cargo test graph_traversal_and_metrics_routes_work --lib -- --nocapture`; `cargo test --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`; `$env:POSTGRES_TEST_URL='postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus'; cargo test --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`; `cargo test --test storage_conformance sqlite_loom_traversal_performance_target -- --exact --nocapture`; `$env:POSTGRES_TEST_URL='postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus'; cargo test --test storage_conformance postgres_loom_traversal_performance_target -- --exact --nocapture`
- Known gaps / weak spots: The traversal performance proof is a warmed median-of-three probe on a deterministic 10K-block chain fixture, not a full production benchmark matrix. It closes the packet threshold directly, but broader real-world graph-shape variance still lives outside this WP.
- Heuristic risks / maintainability concerns: The recursive traversal/search logic now lives in two backend-specific implementations; semantic drift between SQLite and PostgreSQL is the main long-term risk, so the shared conformance helpers are now the critical guardrail.
- Validator focus request: Re-run adversarial review on the new API route surface (`traverse_loom_graph`, `recompute_loom_block_metrics`, `recompute_all_loom_metrics`), recursion depth fences, parameterized edge-type filtering parity, and the explicit 10K-block timing probe on both backends.
- Rubric contract understanding proof: This packet closes a backend portability contract, not a single backend feature. A pass requires parity proof across both storage engines plus evidence that the recursive SQL remains bounded, deterministic, and spec-portable.
- Rubric scope discipline proof: The staged repair diff is limited to 5 product files, and the cumulative WP branch scope remains within 7 Loom-tracked files including the new traversal-performance test target. No unrelated product files were staged for this repair commit.
- Rubric baseline comparison: Before this repair, the branch still lacked API endpoints for traversal/metrics recompute, accepted unbounded recursive depth through the public API, and had no explicit 10K-block timing proof for the packet threshold. After this repair, those gaps are closed in product code and tests.
- Rubric end-to-end proof: The handler-level API test now creates Loom blocks/edges, traverses the graph through the new route, forces stale metrics, and proves both block-scoped and workspace-scoped recomputation restore derived counts; the dedicated SQLite/PostgreSQL traversal-performance tests prove the 3-hop 10K-block threshold directly.
- Rubric architecture fit self-review: The change extends the shared storage abstraction first and wires backend-specific SQL underneath it, which matches the packet's portability-first architecture instead of hiding behavior in API-local helpers.
- Rubric heuristic quality self-review: The remaining maintainability risk is keeping the two backend traversal SQL paths aligned. The compensating guard is now stronger: both the shared conformance suite and the explicit 10K-block performance probes exercise the same contract on both providers, and the traversal filter inputs are parameterized rather than string-built.
- Rubric anti-gaming / counterfactual check: If `src/backend/handshake_core/src/api/loom.rs:933` `traverse_loom_graph(...)` or `src/backend/handshake_core/src/api/loom.rs:967` / `:986` recompute handlers are removed, `graph_traversal_and_metrics_routes_work` fails. If `src/backend/handshake_core/src/storage/tests.rs:1313` `loom_traverse_graph_meets_performance_target(...)` or `src/backend/handshake_core/tests/storage_conformance.rs:56` / `:64` are removed, the packet no longer carries any explicit 10K-block timing proof. If the new parameter binding at `src/backend/handshake_core/src/storage/postgres.rs:2121` or `src/backend/handshake_core/src/storage/sqlite.rs:2603` is reverted to string interpolation, the repaired traversal filter hardening is lost.
- Next step / handoff hint: NONE. WP is closed; downstream Loom packets may now consume the completed storage portability contract.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "[LM-GRAPH-001] Graph traversal with recursive CTEs on both backends"
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1684; src/backend/handshake_core/src/storage/postgres.rs:2093; src/backend/handshake_core/src/storage/sqlite.rs:2568; src/backend/handshake_core/src/storage/tests.rs:849; src/backend/handshake_core/src/storage/tests.rs:918; src/backend/handshake_core/src/storage/tests.rs:955`
  - REQUIREMENT: "API endpoints for graph traversal and metrics recomputation in api/loom.rs"
  - EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:86; src/backend/handshake_core/src/api/loom.rs:121; src/backend/handshake_core/src/api/loom.rs:125; src/backend/handshake_core/src/api/loom.rs:933; src/backend/handshake_core/src/api/loom.rs:967; src/backend/handshake_core/src/api/loom.rs:986; src/backend/handshake_core/src/api/loom.rs:1319`
  - REQUIREMENT: "[LM-SEARCH-002] PostgreSQL search filterable by graph relationships"
  - EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:1017; src/backend/handshake_core/src/api/loom.rs:1042; src/backend/handshake_core/src/storage/loom.rs:358; src/backend/handshake_core/src/storage/postgres.rs:2710; src/backend/handshake_core/src/storage/postgres.rs:2737; src/backend/handshake_core/src/storage/postgres.rs:2756; src/backend/handshake_core/src/storage/postgres.rs:2779; src/backend/handshake_core/src/storage/tests.rs:1044; src/backend/handshake_core/src/storage/tests.rs:1062`
  - REQUIREMENT: "2.3.13.7 get_backlinks and get_outgoing_edges"
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1674; src/backend/handshake_core/src/storage/mod.rs:1679; src/backend/handshake_core/src/storage/postgres.rs:2025; src/backend/handshake_core/src/storage/postgres.rs:2059; src/backend/handshake_core/src/storage/sqlite.rs:2496; src/backend/handshake_core/src/storage/sqlite.rs:2532; src/backend/handshake_core/src/storage/tests.rs:1014; src/backend/handshake_core/src/storage/tests.rs:1015`
  - REQUIREMENT: "2.3.13.7 recompute_block_metrics and recompute_all_metrics"
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1691; src/backend/handshake_core/src/storage/mod.rs:1696; src/backend/handshake_core/src/api/loom.rs:967; src/backend/handshake_core/src/api/loom.rs:986; src/backend/handshake_core/src/storage/postgres.rs:2196; src/backend/handshake_core/src/storage/postgres.rs:2242; src/backend/handshake_core/src/storage/sqlite.rs:2665; src/backend/handshake_core/src/storage/sqlite.rs:2711; src/backend/handshake_core/src/api/loom.rs:1450; src/backend/handshake_core/src/api/loom.rs:1470`
  - REQUIREMENT: "[CX-DBP-011] Portable schema and migrations"
  - EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:2093; src/backend/handshake_core/src/storage/sqlite.rs:2568; src/backend/handshake_core/src/storage/tests.rs:1759`
  - REQUIREMENT: "`traverse_graph` performance targets (<100ms SQLite, <50ms PostgreSQL on 3-hop / 10K blocks)"
  - EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:27; src/backend/handshake_core/src/storage/tests.rs:1313; src/backend/handshake_core/src/storage/tests.rs:1336; src/backend/handshake_core/src/storage/tests.rs:1348; src/backend/handshake_core/tests/storage_conformance.rs:56; src/backend/handshake_core/tests/storage_conformance.rs:64`
  - REQUIREMENT: "2.3.13.7 LoomSourceAnchor export/replay durability"
  - EVIDENCE: `src/backend/handshake_core/src/storage/loom.rs:274; src/backend/handshake_core/src/storage/postgres.rs:308; src/backend/handshake_core/src/storage/sqlite.rs:334; src/backend/handshake_core/src/storage/tests.rs:1082; src/backend/handshake_core/src/storage/tests.rs:1127; src/backend/handshake_core/src/storage/tests.rs:1381`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `cargo test loom --lib -- --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_lib.log`
  - LOG_SHA256: `1CFDC7E6CB03EC4F7E0FCBA4F54952650DAE33493AF28A0643A3643D1C1FC838`
  - PROOF_LINES: `test api::loom::tests::graph_traversal_and_metrics_routes_work ... ok`; `test api::loom::tests::view_and_search_emit_events ... ok`; `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 200 filtered out; finished in 0.83s`
  - COMMAND: `cargo test migration --lib -- --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/migration_lib.log`
  - LOG_SHA256: `09BC50A6E63EDAB7EB742F06202CC065BA7DA555C6DC218A8856826A7E080915`
  - PROOF_LINES: `test storage::tests::migrations_are_replay_safe_postgres ... ok`; `test storage::tests::migrations_are_replay_safe_sqlite ... ok`; `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 200 filtered out; finished in 0.03s`
  - COMMAND: `cargo test graph_traversal_and_metrics_routes_work --lib -- --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_api_routes.log`
  - LOG_SHA256: `2125DF16A84C9C15F8BB5125B2180E37CEFFC251DFB76510AC9714B9AED8749F`
  - PROOF_LINES: `test api::loom::tests::graph_traversal_and_metrics_routes_work ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 206 filtered out; finished in 0.27s`
  - COMMAND: `cargo test --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_sqlite.log`
  - LOG_SHA256: `4974E306E73E033AFCDAAED0156D17426CD31648F7B030AB815BB4CD5133F593`
  - PROOF_LINES: `test sqlite_loom_storage_conformance ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.06s`
  - COMMAND: `$env:POSTGRES_TEST_URL='postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus'; cargo test --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/storage_conformance_postgres.log`
  - LOG_SHA256: `A6AA0A7480AD1FC31128CE0ED705770A1E6D78F2A29D2E1488E5BFE29C0728D8`
  - PROOF_LINES: `test postgres_loom_storage_conformance ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 23.50s`
  - COMMAND: `cargo test --test storage_conformance sqlite_loom_traversal_performance_target -- --exact --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_sqlite.log`
  - LOG_SHA256: `CC0A5FC913AB2FE95B08523C2B0CF6ECDEE1AFC87CA59D2D09A188499A590904`
  - PROOF_LINES: `test sqlite_loom_traversal_performance_target ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 1.58s`
  - COMMAND: `$env:POSTGRES_TEST_URL='postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus'; cargo test --test storage_conformance postgres_loom_traversal_performance_target -- --exact --nocapture`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-Loom-Storage-Portability-v3/loom_perf_postgres.log`
  - LOG_SHA256: `B619CE07056BD1E574C778EAB55E095540B3EA660CE211DE916B8846B32DA625`
  - PROOF_LINES: `test postgres_loom_traversal_performance_target ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 38.45s`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
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
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, every appended governed validation report MUST also include:
  - `MAIN_BODY_GAPS:`
    - `- NONE` only when no main-body requirement remains unproven, partial, or weakly evidenced
    - otherwise list each unresolved MUST/SHOULD gap explicitly
  - `QUALITY_RISKS:`
    - `- NONE` only when no material maintainability, brittleness, ambiguity, or heuristic-quality risk remains
    - otherwise list each residual code-quality risk explicitly
  - `VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH`
    - validator-assigned risk tier; MUST NOT be lower than the packet `RISK_TIER`
  - `DIFF_ATTACK_SURFACES:`
    - list the failure surfaces the validator derived from reading the diff directly
  - `INDEPENDENT_CHECKS_RUN:`
    - list validator-owned checks that were not copied from coder evidence, formatted as `what => observed`
  - `COUNTERFACTUAL_CHECKS:`
    - list concrete code-path / symbol counterfactuals in the form `If X were removed or altered, Y would break`
    - naming a test only is insufficient; name the file, symbol, or code path
  - `BOUNDARY_PROBES:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record the validator's interface / producer-consumer / storage / contract boundary checks
  - `NEGATIVE_PATH_CHECKS:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record invalid, missing, adversarial, or failure-path checks the validator ran
  - `INDEPENDENT_FINDINGS:`
    - list what the validator learned independently, even if the conclusion is baseline confirmation
  - `RESIDUAL_UNCERTAINTY:`
    - list remaining uncertainty explicitly; for `VALIDATOR_RISK_TIER=HIGH`, `- NONE` is illegal
  - `SPEC_CLAUSE_MAP:`
    - map each packet requirement to `file:line` evidence proving it is implemented
    - entries must include concrete code references (file paths, line numbers, or symbol names)
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.

### WP_VALIDATOR Advisory Report - 2026-03-20T13:50:26.6877986+01:00 - commit 8c4055a
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PARTIAL
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: PARTIAL
- HEURISTIC_REVIEW_VERDICT: PARTIAL
- SPEC_ALIGNMENT_VERDICT: PARTIAL
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: NONE
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- COMMIT_REVIEWED: `8c4055a487428ef7c957ed57611f1e55c23bae9d`
- BASELINE_REVIEWED: first parent of `8c4055a`
- CLAUSES_REVIEWED:
  - `[LM-GRAPH-001] Graph traversal with recursive CTEs on both backends` -> `src/backend/handshake_core/src/storage/mod.rs:1684-1690` declares the trait method; `src/backend/handshake_core/src/storage/sqlite.rs:2568-2662` and `src/backend/handshake_core/src/storage/postgres.rs:2093-2193` implement recursive CTE traversal with cycle guards; `src/backend/handshake_core/src/storage/tests.rs:849-976` proves depth limit, cycle suppression, and edge-type filtering. Performance target proof remains unclosed.
  - `[LM-SEARCH-002] PostgreSQL search filterable by graph relationships` -> `src/backend/handshake_core/src/api/loom.rs:835-869` threads `backlink_depth`; `src/backend/handshake_core/src/storage/loom.rs:348-359` extends `LoomSearchFilters`; `src/backend/handshake_core/src/storage/postgres.rs:2710-2793` applies the recursive filter; `src/backend/handshake_core/src/storage/tests.rs:1038-1074` proves direct-vs-indirect tag behavior on PostgreSQL.
  - `2.3.13.7 get_backlinks and get_outgoing_edges` -> `src/backend/handshake_core/src/storage/mod.rs:1674-1683`, `src/backend/handshake_core/src/storage/sqlite.rs:2496-2566`, `src/backend/handshake_core/src/storage/postgres.rs:2025-2090`, and `src/backend/handshake_core/src/storage/tests.rs:979-1025`.
  - `2.3.13.7 recompute_block_metrics and recompute_all_metrics` -> `src/backend/handshake_core/src/storage/mod.rs:1691-1696`, `src/backend/handshake_core/src/storage/sqlite.rs:2665-2744`, `src/backend/handshake_core/src/storage/postgres.rs:2196-2275`, and `src/backend/handshake_core/src/storage/tests.rs:808-846`.
  - `[CX-DBP-011] Portable schema and migrations` -> no migration files changed in `8c4055a`; recursive SQL additions stay in storage adapters, and the targeted portability test `cargo test --lib storage::tests::loom_migration_schema_is_portable_sqlite -- --exact --nocapture` passed.
  - `2.3.13.7 LoomSourceAnchor export/replay durability` -> `src/backend/handshake_core/src/storage/tests.rs:1077-1149` verifies JSON export/replay plus storage readback via `get_outgoing_edges`; backend edge readers are `src/backend/handshake_core/src/storage/sqlite.rs:2496-2566` and `src/backend/handshake_core/src/storage/postgres.rs:2025-2090`.
- NOT_PROVEN:
  - `packet.md:616` requires `traverse_graph` to meet explicit performance targets (`<100ms` SQLite, `<50ms` PostgreSQL for 3-hop/10K-block traversal). The reviewed diff and tests prove functional correctness, not timing.
  - `packet.md:393` requires API endpoints for graph traversal and metrics recomputation in `api/loom.rs`. The reviewed diff only adds `backlink_depth` to search query handling; no traversal/metrics routes were added.
- MAIN_BODY_GAPS:
  - The explicit `traverse_graph` performance target remains unproven against the packet/spec threshold.
- QUALITY_RISKS:
  - `src/backend/handshake_core/src/storage/postgres.rs:2104-2176` and `src/backend/handshake_core/src/storage/sqlite.rs:2579-2651` build `edge_type_filter` by formatting enum strings into raw SQL. Current values are enum-bounded by `src/backend/handshake_core/src/storage/loom.rs:208-239`, but the pattern is brittle for future edge-type expansion and harder to audit than parameterized composition.
  - `src/backend/handshake_core/src/api/loom.rs:845-868` and `src/backend/handshake_core/src/storage/postgres.rs:2710-2793` accept any `u32` `backlink_depth` with no packet-defined clamp, leaving worst-case recursive search cost unmeasured.
- VALIDATOR_RISK_TIER: HIGH
- DIFF_ATTACK_SURFACES:
  - Producer/consumer drift between HTTP query params, `LoomSearchFilters`, and PostgreSQL recursive search behavior.
  - Trait expansion drift between `storage/mod.rs`, `sqlite.rs`, and `postgres.rs`.
  - Recursive CTE depth/cycle handling on both backends.
  - Persistence/readback drift for `LoomSourceAnchor` across create, fetch, export, and replay.
  - Portable-SQL claims versus backend-specific SQL construction.
- INDEPENDENT_CHECKS_RUN:
  - `cargo test --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture` with `POSTGRES_TEST_URL=postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus` => PASS; independently exercised PostgreSQL graph traversal, directional edge queries, metrics recomputation, graph-filter negative path, and source-anchor round trip.
  - `cargo test --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture` => PASS; independently exercised the same Loom conformance surface on SQLite.
  - `cargo test --lib storage::tests::loom_migration_schema_is_portable_sqlite -- --exact --nocapture` => PASS; no portability regression was introduced in the SQLite migration contract.
  - `rg -n "LoomSearchFilters \\{" src/backend/handshake_core` plus line inspection => `backlink_depth` has one product producer (`src/backend/handshake_core/src/api/loom.rs:863-869`), the contract struct (`src/backend/handshake_core/src/storage/loom.rs:348-359`), and the PostgreSQL consumer (`src/backend/handshake_core/src/storage/postgres.rs:2710-2793`); no stale constructor omission was found.
  - `rg -n "100ms|50ms|benchmark|bench|Instant::now\\(|elapsed\\(|loom_traverse_graph" src/backend/handshake_core/src src/backend/handshake_core/tests` => no traversal-specific timing assertion or benchmark exists, so the packet's performance clause is not independently proven.
- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/storage/postgres.rs:2710-2793` stopped threading `backlink_depth` into the recursive `EXISTS` filter, the PostgreSQL-only negative path at `src/backend/handshake_core/src/storage/tests.rs:1038-1054` would no longer distinguish direct from indirect tag matches.
  - If the cycle guards in `src/backend/handshake_core/src/storage/sqlite.rs:2613-2616` or `src/backend/handshake_core/src/storage/postgres.rs:2138-2141` were removed, the cycle test at `src/backend/handshake_core/src/storage/tests.rs:918-950` would re-emit the starting block or loop through the AI-suggested back edge.
  - If the new trait methods at `src/backend/handshake_core/src/storage/mod.rs:1674-1696` existed without backend implementations, the conformance calls at `src/backend/handshake_core/src/storage/tests.rs:855-975`, `1014-1015`, `1127`, and `1466-1764` would fail per backend.
  - If `src/backend/handshake_core/src/storage/tests.rs:1126-1139` did not read the replayed edge back through `get_outgoing_edges`, the new directional-edge surface would not prove that stored source anchors survive the writer/reader boundary.
- BOUNDARY_PROBES:
  - HTTP query -> contract struct -> backend query: inspected `src/backend/handshake_core/src/api/loom.rs:835-869`, `src/backend/handshake_core/src/storage/loom.rs:348-359`, and `src/backend/handshake_core/src/storage/postgres.rs:2710-2793` to confirm `backlink_depth` crosses the producer/consumer boundary without silent drop.
  - Edge write -> edge read -> JSON replay: inspected `src/backend/handshake_core/src/storage/tests.rs:1110-1145` plus backend edge readers `src/backend/handshake_core/src/storage/sqlite.rs:2496-2566` and `src/backend/handshake_core/src/storage/postgres.rs:2025-2090` to confirm `source_anchor` survives storage and retrieval.
- NEGATIVE_PATH_CHECKS:
  - PostgreSQL negative path rerun: `postgres_loom_storage_conformance` passed while `src/backend/handshake_core/src/storage/tests.rs:1038-1054` requires `backlink_depth: Some(1)` to return no result for an indirect tag path.
  - Static absence probe: repo search found no traversal benchmark/timing assertion, which is negative evidence against the explicit performance closure claim in `packet.md:616`.
- INDEPENDENT_FINDINGS:
  - The six packet clauses are implemented at the storage/test layer and independently exercised on both backends.
  - The PostgreSQL-only graph-filter feature is wired end-to-end from the API parameter layer into the recursive search query.
  - The reviewed diff proves behavioral portability far better than v2, but it still does not prove the explicit performance acceptance threshold.
  - The packet handoff brief still overstates API-surface closure; `api/loom.rs` only gained search-param plumbing in this diff.
- RESIDUAL_UNCERTAINTY:
  - The packet requires traversal timing targets on 10K-block graphs, but no benchmark or timing harness was reviewed, so production-scale performance remains unknown.
  - `backlink_depth` has no upper bound in the reviewed API/query path, so hostile or accidental large-depth recursive queries remain unmeasured.
  - The source-anchor proof is conformance-level JSON export/replay, not a full external artifact export/import workflow.
- SPEC_CLAUSE_MAP:
  - `[LM-GRAPH-001] Graph traversal with recursive CTEs on both backends` => `src/backend/handshake_core/src/storage/mod.rs:1684-1690`, `src/backend/handshake_core/src/storage/sqlite.rs:2568-2662`, `src/backend/handshake_core/src/storage/postgres.rs:2093-2193`, `src/backend/handshake_core/src/storage/tests.rs:849-976`
  - `[LM-SEARCH-002] PostgreSQL search filterable by graph relationships` => `src/backend/handshake_core/src/api/loom.rs:835-869`, `src/backend/handshake_core/src/storage/loom.rs:348-359`, `src/backend/handshake_core/src/storage/postgres.rs:2710-2793`, `src/backend/handshake_core/src/storage/tests.rs:1038-1074`
  - `2.3.13.7 get_backlinks and get_outgoing_edges` => `src/backend/handshake_core/src/storage/mod.rs:1674-1683`, `src/backend/handshake_core/src/storage/sqlite.rs:2496-2566`, `src/backend/handshake_core/src/storage/postgres.rs:2025-2090`, `src/backend/handshake_core/src/storage/tests.rs:979-1025`
  - `2.3.13.7 recompute_block_metrics and recompute_all_metrics` => `src/backend/handshake_core/src/storage/mod.rs:1691-1696`, `src/backend/handshake_core/src/storage/sqlite.rs:2665-2744`, `src/backend/handshake_core/src/storage/postgres.rs:2196-2275`, `src/backend/handshake_core/src/storage/tests.rs:808-846`
  - `[CX-DBP-011] Portable schema and migrations` => `src/backend/handshake_core/src/storage/tests.rs:1759-1764` plus validator-owned check `cargo test --lib storage::tests::loom_migration_schema_is_portable_sqlite -- --exact --nocapture`
  - `2.3.13.7 LoomSourceAnchor export/replay durability` => `src/backend/handshake_core/src/storage/tests.rs:1077-1149`, `src/backend/handshake_core/src/storage/sqlite.rs:2496-2566`, `src/backend/handshake_core/src/storage/postgres.rs:2025-2090`
- NEGATIVE_PROOF:
  - `packet.md:616` requires `traverse_graph` implementations "meeting spec performance targets (<100ms 3-hop on 10K blocks SQLite, <50ms PostgreSQL)". The reviewed diff and tests contain functional traversal assertions only (`src/backend/handshake_core/src/storage/tests.rs:849-976`) and no benchmark/timing proof, so that acceptance clause is not fully implemented/proven.
  - `packet.md:393` requires API endpoints for graph traversal and metrics recomputation in `api/loom.rs`. The reviewed route table at `src/backend/handshake_core/src/api/loom.rs:72-90` still exposes blocks, edges, import/assets, views, and search only; commit `8c4055a` adds `backlink_depth` to search query handling at `src/backend/handshake_core/src/api/loom.rs:845-868`, but no traversal/metrics endpoints.

### WP_VALIDATOR Advisory Report - 2026-03-20T15:20:41.5401475+01:00 - commit 7aa995b
- REVIEW_SCOPE: Advisory diff-scoped review of commit `7aa995b0dd2a2fca5bb713dd12ce400a9a0a1214` versus first parent `8c4055a487428ef7c957ed57611f1e55c23bae9d`, with the current packet and committed PREPARE worktree as authority.
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: NONE
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- VALIDATOR_RISK_TIER: HIGH
- ADVISORY_VERDICT: PASS
- FINDINGS_COUNT: 0
- CLAUSES_REVIEWED:
  - `[LM-GRAPH-001] Graph traversal with recursive CTEs on both backends` -> `src/backend/handshake_core/src/api/loom.rs:121`; `src/backend/handshake_core/src/api/loom.rs:933`; `src/backend/handshake_core/src/storage/postgres.rs:2098-2135`; `src/backend/handshake_core/src/storage/sqlite.rs:2573-2624`; `src/backend/handshake_core/src/storage/tests.rs:1313`; `src/backend/handshake_core/tests/storage_conformance.rs:56`; `src/backend/handshake_core/tests/storage_conformance.rs:64`
  - `[LM-SEARCH-002] PostgreSQL search filterable by graph relationships` -> `src/backend/handshake_core/src/storage/loom.rs:360`; `src/backend/handshake_core/src/api/loom.rs:1017-1042`; `src/backend/handshake_core/src/storage/postgres.rs:2703-2775`; `src/backend/handshake_core/src/storage/tests.rs:1046-1064`
  - `2.3.13.7 get_backlinks and get_outgoing_edges` -> `src/backend/handshake_core/src/storage/mod.rs:1674-1679`; `src/backend/handshake_core/src/storage/postgres.rs:2025-2059`; `src/backend/handshake_core/src/storage/sqlite.rs:2496-2532`; `src/backend/handshake_core/src/storage/tests.rs:1016-1017`
  - `2.3.13.7 recompute_block_metrics and recompute_all_metrics` -> `src/backend/handshake_core/src/api/loom.rs:967-993`; `src/backend/handshake_core/src/storage/mod.rs:1691-1696`; `src/backend/handshake_core/src/storage/postgres.rs:2189-2235`; `src/backend/handshake_core/src/storage/sqlite.rs:2676-2722`; `src/backend/handshake_core/src/storage/tests.rs:822-842`
  - `[CX-DBP-011] Portable schema and migrations` -> `src/backend/handshake_core/src/storage/tests.rs:3136-3148`; `src/backend/handshake_core/src/storage/tests.rs:3215-3239`; `src/backend/handshake_core/tests/storage_conformance.rs:32`; `src/backend/handshake_core/tests/storage_conformance.rs:40`
  - `2.3.13.7 LoomSourceAnchor export/replay durability` -> `src/backend/handshake_core/src/storage/loom.rs:293-305`; `src/backend/handshake_core/src/storage/tests.rs:1079-1149`; `src/backend/handshake_core/src/storage/sqlite.rs:2496-2532`; `src/backend/handshake_core/src/storage/postgres.rs:2025-2059`
- NOT_PROVEN:
  - NONE
- MAIN_BODY_GAPS:
  - NONE
- QUALITY_RISKS:
  - NONE
- DIFF_ATTACK_SURFACES:
  - HTTP route exposure for traversal plus block-scoped and workspace-scoped metrics recomputation.
  - Recursive-depth bounding at the API boundary for both traversal and graph-filtered search.
  - Backend SQL hardening for edge-type traversal filters on PostgreSQL and SQLite.
  - Shared-storage conformance and explicit 10K-block traversal target coverage.
- INDEPENDENT_CHECKS_RUN:
  - `just validator-handoff-check WP-1-Loom-Storage-Portability-v3` => PASS; reran committed `pre-work` and `post-work --rev HEAD` against `7aa995b`, confirming the manifest now matches the final five-file repair diff.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib api::loom::tests::graph_traversal_and_metrics_routes_work -- --exact --nocapture` => PASS; exercised `traverse_loom_graph`, `recompute_loom_block_metrics`, and `recompute_all_loom_metrics` on the final committed API surface.
  - `$env:POSTGRES_TEST_URL='postgres://fastfocus:fastfocus@127.0.0.1:55434/fastfocus'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture` => PASS; re-exercised PostgreSQL graph traversal, directional edge queries, metrics recomputation, backlink-depth negative path, and LoomSourceAnchor round-trip on the final head.
  - `rg -n "MAX_LOOM_GRAPH_DEPTH|traverse_loom_graph|recompute_loom_block_metrics|recompute_all_loom_metrics|ANY\\(\\$4::TEXT\\[\\]\\)|json_each\\(\\$4\\)" src/backend/handshake_core/src/api/loom.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/sqlite.rs` => confirmed route exposure, depth clamp at the API boundary, and parameterized edge-type filters on both backends.
- COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/api/loom.rs:791` or `src/backend/handshake_core/src/api/loom.rs:1040-1042` stopped clamping depth to `MAX_LOOM_GRAPH_DEPTH`, the oversize-depth request at `src/backend/handshake_core/src/api/loom.rs:1496` would stop proving bounded traversal/search behavior.
  - If `src/backend/handshake_core/src/api/loom.rs:933`; `src/backend/handshake_core/src/api/loom.rs:967`; or `src/backend/handshake_core/src/api/loom.rs:986` were removed, `api::loom::tests::graph_traversal_and_metrics_routes_work` would fail on the missing route handlers.
  - If `src/backend/handshake_core/src/storage/postgres.rs:2121-2135` or `src/backend/handshake_core/src/storage/sqlite.rs:2603-2624` reverted away from bound edge-type filters, traversal filter hardening and backend parity would be lost even though the rest of the recursive CTE shape remained.
- BOUNDARY_PROBES:
  - HTTP route -> storage trait -> backend traversal: `src/backend/handshake_core/src/api/loom.rs:933-1042` feeds the trait methods at `src/backend/handshake_core/src/storage/mod.rs:1674-1696`, which land in the backend CTE implementations at `src/backend/handshake_core/src/storage/postgres.rs:2098-2235` and `src/backend/handshake_core/src/storage/sqlite.rs:2573-2722`.
  - Storage write -> readback -> replay boundary: `src/backend/handshake_core/src/storage/tests.rs:1079-1149` confirms that `source_anchor` written with a Loom edge survives retrieval through `get_outgoing_edges` and export/replay on both providers.
- NEGATIVE_PATH_CHECKS:
  - PostgreSQL negative-path rerun: `postgres_loom_storage_conformance` passed while `src/backend/handshake_core/src/storage/tests.rs:1046-1064` requires `backlink_depth: Some(1)` to reject an indirect tag path.
  - Oversize-depth API probe: `src/backend/handshake_core/src/api/loom.rs:1496` exercises a request above `MAX_LOOM_GRAPH_DEPTH` and still passes through the clamped traversal/search path.
- INDEPENDENT_FINDINGS:
  - The repair commit closes the two earlier partial findings: the traversal/metrics API routes now exist, and the committed post-work manifest now matches the actual five-file repair diff.
  - The backend traversal filter hardening is real on both providers; the final code binds filter values instead of formatting raw edge-type SQL.
  - The final head keeps the portability contract concentrated inside the shared storage abstraction and conformance surface rather than scattering backend-specific behavior into API-local shims.
- RESIDUAL_UNCERTAINTY:
  - This remains a `HIGH`-risk shared storage surface, so post-merge spotchecks on both providers are still prudent even after the committed diff-scoped PASS.
  - I did not rerun every 10K-block performance probe in the validator pass; the packet already carries committed evidence for those exact targets, and this validator rerun focused on the repaired API and backend-hardening surfaces.
- SPEC_CLAUSE_MAP:
  - `[LM-GRAPH-001] Graph traversal with recursive CTEs on both backends` -> `src/backend/handshake_core/src/api/loom.rs:121`; `src/backend/handshake_core/src/api/loom.rs:933`; `src/backend/handshake_core/src/storage/postgres.rs:2098-2135`; `src/backend/handshake_core/src/storage/sqlite.rs:2573-2624`; `src/backend/handshake_core/src/storage/tests.rs:1313`; `src/backend/handshake_core/tests/storage_conformance.rs:56`; `src/backend/handshake_core/tests/storage_conformance.rs:64`
  - `[LM-SEARCH-002] PostgreSQL search filterable by graph relationships` -> `src/backend/handshake_core/src/storage/loom.rs:360`; `src/backend/handshake_core/src/api/loom.rs:1017-1042`; `src/backend/handshake_core/src/storage/postgres.rs:2703-2775`; `src/backend/handshake_core/src/storage/tests.rs:1046-1064`
  - `2.3.13.7 get_backlinks and get_outgoing_edges` -> `src/backend/handshake_core/src/storage/mod.rs:1674-1679`; `src/backend/handshake_core/src/storage/postgres.rs:2025-2059`; `src/backend/handshake_core/src/storage/sqlite.rs:2496-2532`; `src/backend/handshake_core/src/storage/tests.rs:1016-1017`
  - `2.3.13.7 recompute_block_metrics and recompute_all_metrics` -> `src/backend/handshake_core/src/api/loom.rs:967-993`; `src/backend/handshake_core/src/storage/mod.rs:1691-1696`; `src/backend/handshake_core/src/storage/postgres.rs:2189-2235`; `src/backend/handshake_core/src/storage/sqlite.rs:2676-2722`; `src/backend/handshake_core/src/storage/tests.rs:822-842`
  - `[CX-DBP-011] Portable schema and migrations` -> `src/backend/handshake_core/src/storage/tests.rs:3136-3148`; `src/backend/handshake_core/src/storage/tests.rs:3215-3239`; `src/backend/handshake_core/tests/storage_conformance.rs:32`; `src/backend/handshake_core/tests/storage_conformance.rs:40`
  - `2.3.13.7 LoomSourceAnchor export/replay durability` -> `src/backend/handshake_core/src/storage/loom.rs:293-305`; `src/backend/handshake_core/src/storage/tests.rs:1079-1149`; `src/backend/handshake_core/src/storage/sqlite.rs:2496-2532`; `src/backend/handshake_core/src/storage/postgres.rs:2025-2059`
- NEGATIVE_PROOF:
  - Out-of-scope downstream Loom bridge requirements remain open: this packet proves storage/API portability only and does not implement the later downloader/archive bridge behavior tracked under `WP-1-Media-Downloader-Loom-Bridge` and `WP-1-Video-Archive-Loom-Integration`.

VALIDATION REPORT - WP-1-Loom-Storage-Portability-v3
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PARTIAL
HEURISTIC_REVIEW_VERDICT: PARTIAL
SPEC_ALIGNMENT_VERDICT: FAIL
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: FAIL
SPEC_CONFIDENCE: POST_MERGE_RECHECKED
WORKFLOW_VALIDITY: INVALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: PARTIAL
VALIDATOR_RISK_TIER: HIGH

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Loom-Storage-Portability-v3/packet.md` (status: `Blocked`)
- Spec: `Handshake_Master_Spec_v02.178.md`
- Governance basis: `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT.md`

CLAUSES_REVIEWED:
- `[LM-GRAPH-001] graph traversal and backend portability surfaces` -> `src/backend/handshake_core/src/storage/mod.rs`; `src/backend/handshake_core/src/storage/sqlite.rs`; `src/backend/handshake_core/src/storage/postgres.rs`
- `LM-SEARCH-002 and source-anchor durability` -> `src/backend/handshake_core/src/storage/loom.rs`; `src/backend/handshake_core/src/api/loom.rs`; `src/backend/handshake_core/src/storage/tests.rs`
- `dual-backend conformance evidence` -> `src/backend/handshake_core/tests/storage_conformance.rs`

NOT_PROVEN:
- The 2026-03-21 audit did not re-prove full Loom portability closure; it only found no fresh concrete Loom failure in the audited slice.
- Historical v3 closure remains blocked by current workflow law because it predates the completion-layer threshold and cannot stand as PASS-shaped closure in place.

MAIN_BODY_GAPS:
- Full current-spec portability closure was not re-established during the governance hardening pass.
- The packet’s historical closure claim is broader than the narrower audited evidence now relied on.

QUALITY_RISKS:
- The old v3 packet narrative compresses narrower audited evidence into a stronger closure claim than current governance allows.
- Further remediation could create speculative churn unless the next packet is narrowed to concrete current-main gaps only.

DIFF_ATTACK_SURFACES:
- Storage trait parity between SQLite and PostgreSQL
- Graph traversal/search boundary from API to backend query
- Source-anchor export/replay durability across backend readers

INDEPENDENT_CHECKS_RUN:
- `cargo test --test storage_conformance postgres_loom_storage_conformance -- --exact --nocapture` => PASS during the audit on the reviewed PostgreSQL slice
- `cargo test --test storage_conformance sqlite_loom_storage_conformance -- --exact --nocapture` => PASS during the audit on the reviewed SQLite slice
- Direct audit review of the integrated Loom route/storage surface => no fresh Loom defect isolated, but full closure remained narrower than the packet narrative

COUNTERFACTUAL_CHECKS:
- If recursive traversal or directional edge readers drift between `storage/mod.rs`, `sqlite.rs`, and `postgres.rs`, the dual-backend conformance surface would stop proving portability.
- If the API/search boundary stops carrying graph-filter semantics into the backend query path, the PostgreSQL-specific graph-filter proof would collapse.

BOUNDARY_PROBES:
- The audit inspected the API -> storage-trait -> backend-query path for graph traversal/search and the storage write -> readback -> replay path for Loom source anchors.

NEGATIVE_PATH_CHECKS:
- Existing negative-path coverage for graph-filter depth behavior and cycle handling passed in the audited slice, but the audit did not elevate that narrower proof into a fresh full-portability PASS.

INDEPENDENT_FINDINGS:
- The audited Loom slice looked materially real rather than narrative-only.
- The governance failure here is primarily proof-shape and closure-authority drift, not a newly isolated Loom implementation collapse.

RESIDUAL_UNCERTAINTY:
- This was not a full renewed dual-backend portability signoff.
- Future refinement must decide whether the next Loom packet is code remediation, proof-only closure, or explicit archival handling.

SPEC_CLAUSE_MAP:
- `graph traversal and directional edge portability` -> `src/backend/handshake_core/src/storage/mod.rs`; `src/backend/handshake_core/src/storage/sqlite.rs`; `src/backend/handshake_core/src/storage/postgres.rs`
- `graph-filter search and source-anchor durability` -> `src/backend/handshake_core/src/api/loom.rs`; `src/backend/handshake_core/src/storage/loom.rs`; `src/backend/handshake_core/src/storage/tests.rs`

NEGATIVE_PROOF:
- The governance kernel no longer accepts the old v3 closure shape as sufficient proof of current-spec completion; a new remediation/proof packet is required before Loom portability can be presented as current validated closure again.

REASON FOR FAIL:
- Governance reclassification on 2026-03-24: this packet is retained as historical audit evidence only. It must not be resumed or treated as validated closure because the current workflow law blocks legacy PASS-shaped closure in place, and the 2026-03-21 audit provided only a narrower “no fresh Loom failure found” judgment rather than a new full-closure signoff.
