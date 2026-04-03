# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).
- Packet metadata is the authoritative lifecycle truth. `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, and `BUILD_ORDER.md` are projections and must reconcile to this header.
- Active packet rule: the packet mapped by `BASE_WP_ID` in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` is the current contract. Any other official packet with the same `BASE_WP_ID` is older history and must be tracked as `SUPERSEDED` on the Task Board.
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just pre-work` enforces alignment.

---

# Task Packet: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1

## METADATA
- TASK_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- WP_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- BASE_WP_ID: WP-1-Postgres-Structured-Collaboration-Artifact-Parity
- DATE: 2026-04-03T23:39:59.457Z
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
- SESSION_HOST_PREFERENCE: VSCODE_EXTENSION_TERMINAL
- SESSION_HOST_FALLBACK: CLI_ESCALATION_WINDOW
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_PLUGIN_FIRST_WITH_2TRY_ESCALATION
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-artifact-parity-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01. Allowed: NONE | LLM_FIRST_DATA_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Ready for Dev
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NONE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Capability-Boundary-Refactor, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Dual-Backend-Tests
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- LOCAL_WORKTREE_DIR: ../wtc-artifact-parity-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja040420260134
- PACKET_FORMAT_VERSION: 2026-04-01

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/migrations/ | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/workflows.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Conformance requirement 11 structured-record readability [ADD v02.166] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics
  - the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields
  - a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/mod.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/postgres.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/sqlite.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/locus_sqlite.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/locus/types.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/locus/task_board.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/tests.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/migrations/ (migration/sql surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: canonical structured-collaboration Postgres parity | SUBFEATURES: portable schema coverage, Postgres query implementations, update/sync wiring, dual-backend proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability objective
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: tracked work-packet and micro-task storage parity | SUBFEATURES: work-packet readers, micro-task metadata/status/list readers, bounded task-board sync path | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity is bounded to canonical structured work-state paths
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: compact summary and base-envelope parity | SUBFEATURES: `project_profile_kind`, `mirror_state`, `workflow_state_family`, `queue_reason_code`, `allowed_action_ids` | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: local-small-model consumers depend on these fields staying portable
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PostgreSQL work-packet structured row and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: canonical work-packet state must stop being SQLite-only
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PostgreSQL micro-task metadata, status-row, and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: micro-task summary and workflow-state projections depend on these readers
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PostgreSQL task-board update and bounded sync parity | JobModel: WORKFLOW | Workflow: locus_task_board_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: a bounded task-board path must be portable for parity to be honest
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: dual-backend structured-collaboration conformance proofs | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validators need executable proof that PostgreSQL left blanket capability denial behind
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- DATA_CONTRACT_RULES:
  - Keep persisted and emitted structure SQL-backed and PostgreSQL-ready; do not introduce fresh SQLite-only semantics unless the packet or spec explicitly requires them.
  - Prefer explicit machine-readable fields, enums, ids, relations, and provenance over presentation-only strings, overloaded text blobs, or parser-only implied meaning.
  - Preserve stable ids, explicit relations, backlink-friendly fields, provenance anchors, and retrieval-friendly summaries so Loom and graph/search consumers can traverse the data without reparsing UI text.
- VALIDATOR_DATA_PROOF_HINTS:
  - Prove the touched data surfaces remain SQLite-now and PostgreSQL-ready or justify any backend-specific semantics explicitly.
  - Prove the emitted or persisted shapes stay LLM-first readable and parseable with stable field names and explicit structured values.
  - Prove Loom-facing ids, relations, provenance anchors, and retrieval fields remain explicit where the packet touches them.
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
- DIRECT REVIEW CONTRACT:
  - For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` packets with `PACKET_FORMAT_VERSION >= 2026-03-21`, `COMMUNICATION_CONTRACT` MUST be `DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE` MUST be `HANDOFF_VERDICT_BLOCKING`.
  - Required structured receipts for the coder <-> WP validator lane:
    - `VALIDATOR_KICKOFF` (`WP_VALIDATOR -> CODER`)
    - `CODER_INTENT` (`CODER -> WP_VALIDATOR`, correlated to kickoff)
    - `CODER_HANDOFF` (`CODER -> WP_VALIDATOR`)
    - `VALIDATOR_REVIEW` (`WP_VALIDATOR -> CODER`, correlated to handoff)
  - For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with a shared `correlation_id` / `ack_for` chain.
  - Review-tracked receipt appends auto-write notifications for the explicit target role and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`.
  - `just wp-thread-append` remains valid for soft coordination only. It does not satisfy the required direct-review contract by itself.
  - `just wp-communication-health-check WP-{ID} KICKOFF|HANDOFF|VERDICT` is the machine gate for this contract.
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
- CONTEXT_START_LINE: 3243
- CONTEXT_END_LINE: 3250
- CONTEXT_TOKEN: Storage Backend Portability Architecture [CX-DBP-001]
- EXCERPT_ASCII_ESCAPED:
  ```text
### 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]

  **What**
  Defines four mandatory architectural pillars for ensuring database backend flexibility:
  - single storage API
  - portable schema and migrations
  - rebuildable indexes
  - dual-backend testing
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Canonical structured collaboration artifact family [ADD v02.167]
- CONTEXT_START_LINE: 6817
- CONTEXT_END_LINE: 6838
- CONTEXT_TOKEN: Canonical structured collaboration artifact family
- EXCERPT_ASCII_ESCAPED:
  ```text
**Canonical structured collaboration artifact family** [ADD v02.167]

  - The canonical file standard for Work Packets, Micro-Tasks, and Task Board projections SHALL be versioned JSON documents.
  - Recommended portable Phase 1 layout:
    - `.handshake/gov/work_packets/{wp_id}/packet.json`
    - `.handshake/gov/work_packets/{wp_id}/summary.json`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/packet.json`
    - `.handshake/gov/micro_tasks/{wp_id}/{mt_id}/summary.json`
    - `.handshake/gov/task_board/index.json`
  - Every canonical structured collaboration record MUST expose a schema identifier, schema version, stable record identifier, updated timestamp, project profile kind, and references to note or evidence artifacts.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Base structured schema and project-profile extension contract [ADD v02.168]
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6861
- CONTEXT_TOKEN: Base structured schema and project-profile extension contract
- EXCERPT_ASCII_ESCAPED:
  ```text
**Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope.
  - At minimum that base envelope MUST expose:
    - `schema_id`
    - `schema_version`
    - `record_id`
    - `record_kind`
    - `project_profile_kind`
    - `updated_at`
    - `mirror_state`
    - `authority_refs`
    - `evidence_refs`
  - Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports MUST remain field-equivalent at the base-envelope level.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
- CONTEXT_START_LINE: 6929
- CONTEXT_END_LINE: 6988
- CONTEXT_TOKEN: Project-agnostic workflow state, queue reason, and governed action contract
- EXCERPT_ASCII_ESCAPED:
  ```text
**Project-agnostic workflow state, queue reason, and governed action contract** [ADD v02.171]

  - Every canonical Work Packet, Micro-Task, Task Board projection row, and Dev Command Center queue row SHALL expose:
    - `workflow_state_family`
    - `queue_reason_code`
    - `allowed_action_ids`
  - `workflow_state_family` MUST stay low-cardinality and project-agnostic.
  - `queue_reason_code` MUST explain why the record is currently routed or grouped where it is.
  - Board position, queue order, and mailbox thread order MUST NOT become substitutes for `workflow_state_family` or `queue_reason_code`.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.15.10 Conformance requirement 11 structured-record readability [ADD v02.166]
- CONTEXT_START_LINE: 7367
- CONTEXT_END_LINE: 7373
- CONTEXT_TOKEN: Canonical Work Packet, Micro-Task, and Task Board state MUST be readable as structured records
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.166] 11. Canonical Work Packet, Micro-Task, and Task Board state MUST be readable as structured records without requiring Markdown parsing as the only machine-readable path.
  [ADD v02.166] 12. Append-only plan, blocker, handoff, review, and decision notes MUST preserve note type, summary, author, and time metadata even when the long-form body is stored in Markdown sidecars.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] | WHY_IN_SCOPE: PostgreSQL is still an honest denial for canonical structured work-state storage instead of a real backend participant | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/migrations/ | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: portability remains theoretical and future backend migration debt keeps compounding
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | WHY_IN_SCOPE: canonical Work Packet, Micro-Task, and Task Board records must stop being effectively SQLite-only | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: PostgreSQL may store data but not the canonical portable artifact family the spec actually governs
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | WHY_IN_SCOPE: work-packet and micro-task rows must preserve base-envelope semantics such as `project_profile_kind` and `mirror_state` across backends | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: consumers can parse different field semantics depending on backend
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | WHY_IN_SCOPE: portable structured records must preserve `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` on PostgreSQL as well as SQLite | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | RISK_IF_MISSED: Task Board and queue consumers remain backend-sensitive and semantically lossy
  - CLAUSE: Conformance requirement 11 structured-record readability [ADD v02.166] | WHY_IN_SCOPE: canonical Work Packet, Micro-Task, and Task Board state must be readable as structured records on PostgreSQL without falling back to Markdown-only truth | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | RISK_IF_MISSED: structured collaboration remains effectively single-backend or mirror-dependent
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: structured work-packet row and list readers | PRODUCER: storage/postgres.rs and storage/sqlite.rs | CONSUMER: workflows.rs and Locus readers | SERIALIZER_TRANSPORT: SQL row mapping into typed structs | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | DRIFT_RISK: PostgreSQL can diverge on field presence or semantics while still returning a row
  - CONTRACT: structured micro-task metadata, status-row, and list readers | PRODUCER: storage/postgres.rs and storage/sqlite.rs | CONSUMER: workflows.rs and task-board summary code | SERIALIZER_TRANSPORT: SQL row mapping and JSON metadata payloads | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | DRIFT_RISK: micro-task workflow-state or summary semantics drift by backend
  - CONTRACT: task-board update and bounded sync path | PRODUCER: workflows.rs plus backend storage implementations | CONSUMER: task-board projection readers and Command Center style views | SERIALIZER_TRANSPORT: SQL update plus structured projection export | VALIDATOR_READER: storage/tests.rs and task-board tests | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | DRIFT_RISK: task-board truth remains SQLite-only or semantically inconsistent
  - CONTRACT: structured-collaboration schema and migration coverage | PRODUCER: numbered migrations and storage implementations | CONSUMER: SQLite/PostgreSQL runtime boot plus validators | SERIALIZER_TRANSPORT: migration framework and sqlx schema | VALIDATOR_READER: storage/tests.rs and migration checks | TRIPWIRE_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | DRIFT_RISK: backend portability claims fail at schema bootstrap rather than at query time
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add or update portable schema and migration coverage for the structured-collaboration tables needed by the packet.
  - Implement PostgreSQL structured work-packet and micro-task readers plus the bounded task-board update or sync path using the same semantic contract as SQLite.
  - Replace denial-focused tests with dual-backend parity and negative-path assertions for any still-unsupported narrow operations.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
- CARRY_FORWARD_WARNINGS:
  - Do not widen the top-level storage boundary to compensate for missing parity; keep any new behavior inside the existing governed contract shape or the downstream boundary-refactor packet.
  - Do not claim full PostgreSQL Locus parity; keep remaining unsupported operations explicit, narrow, and tested.
  - Do not preserve SQLite-only task-board sync by silently downgrading the packet to read-only parity.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013]
  - Canonical structured collaboration artifact family [ADD v02.167]
  - Base structured schema and project-profile extension contract [ADD v02.168]
  - Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]
  - Conformance requirement 11 structured-record readability [ADD v02.166]
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - confirm PostgreSQL no longer reports blanket structured-collaboration capability denial on the claimed paths
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Full PostgreSQL parity for every Locus operation remains outside this packet.
  - Historical data migration complexity and performance characteristics are not proven at refinement time.
  - The exact final bounded update or sync surface may narrow during coding if a specific sub-operation is shown to be non-portable within the signed scope.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - The current spec and completed structured-collaboration packets already define the canonical record family, base envelope, summary contract, and workflow-state vocabulary.
  - The missing work is backend implementation parity plus dual-backend proof, not discovery of a new external pattern.
- GITHUB_PROJECT_DECISIONS:
  - NONE
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE
- SOURCE_SCAN_DECISIONS:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Locus
  - MicroTask
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - portable migration coverage plus PostgreSQL structured readers -> IN_THIS_WP (stub: NONE)
  - task-board sync plus workflow-state and queue-reason contract -> IN_THIS_WP (stub: NONE)
  - compact summary/base-envelope parity plus local-small-model consumers -> IN_THIS_WP (stub: NONE)
  - dual-backend proof plus capability-denial removal -> IN_THIS_WP (stub: NONE)
  - work-packet row parity plus summary-field preservation -> IN_THIS_WP (stub: NONE)
  - micro-task metadata parity plus task-board summary hydration -> IN_THIS_WP (stub: NONE)
  - PostgreSQL reader parity plus bounded writer parity -> IN_THIS_WP (stub: NONE)
  - base-envelope field parity plus contract-hardening validators -> IN_THIS_WP (stub: NONE)
  - `project_profile_kind` plus `mirror_state` parity -> IN_THIS_WP (stub: NONE)
  - workflow-state triplet parity plus task-board projection -> IN_THIS_WP (stub: NONE)
  - denial-removal plus negative-path explicitness for remaining unsupported ops -> IN_THIS_WP (stub: NONE)
  - migration bootstrap plus full-suite backend proof -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: canonical structured-collaboration Postgres parity | SUBFEATURES: portable schema coverage, Postgres query implementations, update/sync wiring, dual-backend proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability objective
  - PILLAR: Locus | CAPABILITY_SLICE: tracked work-packet and micro-task storage parity | SUBFEATURES: work-packet readers, micro-task metadata/status/list readers, bounded task-board sync path | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity is bounded to canonical structured work-state paths
  - PILLAR: MicroTask | CAPABILITY_SLICE: canonical micro-task parity | SUBFEATURES: metadata payloads, status rows, list readers, workflow-state preservation | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.archivist, engine.librarian, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: compact summaries and metadata must stay field-equivalent across backends
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: bounded task-board update or sync parity | SUBFEATURES: update path, sync eligibility, canonical row semantics, workflow-state and queue-reason preservation | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: parity is only honest if the claimed runtime projection path stops being SQLite-only
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: compact summary and base-envelope parity | SUBFEATURES: `project_profile_kind`, `mirror_state`, `workflow_state_family`, `queue_reason_code`, `allowed_action_ids` | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: local-small-model consumers depend on these fields staying portable
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: PostgreSQL work-packet structured row and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: canonical work-packet state must stop being SQLite-only
  - Capability: PostgreSQL micro-task metadata, status-row, and list readers | JobModel: WORKFLOW | Workflow: structured_collaboration_runtime | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: micro-task summary and workflow-state projections depend on these readers
  - Capability: PostgreSQL task-board update and bounded sync parity | JobModel: WORKFLOW | Workflow: locus_task_board_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: a bounded task-board path must be portable for parity to be honest
  - Capability: dual-backend structured-collaboration conformance proofs | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: validators need executable proof that PostgreSQL left blanket capability denial behind
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Dual-Backend-Tests-v2 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs -> NOT_PRESENT (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs -> IMPLEMENTED (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs -> PARTIAL (WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1)
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
- What: Implement real PostgreSQL support for canonical Work Packet, Micro-Task, and Task Board structured artifact readers and bounded sync or update paths instead of blanket capability denial.
- Why: PostgreSQL still cannot participate in canonical structured work-state persistence, so backend portability is honest but incomplete and Locus or Task Board truth remains effectively SQLite-only.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - full PostgreSQL parity for every Locus operation
  - mailbox or structured-artifact viewer UI work
  - CRDT or realtime multi-user conflict resolution beyond the bounded structured record contract claimed here
  - storage-boundary refactoring unrelated to making PostgreSQL a real structured-collaboration backend
- TOUCHED_FILE_BUDGET: 9
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
```

### DONE_MEANS
- PostgreSQL no longer blanket-denies the canonical structured-collaboration flows claimed by this packet.
- The same base-envelope, summary, and workflow-state semantics round-trip on SQLite and PostgreSQL for work-packet and micro-task records.
- At least one bounded task-board update or sync path becomes portable on both backends, and any remaining unsupported operations are explicit and narrow.
- Dual-backend tests fail if PostgreSQL regresses back to blanket denial or semantic drift.

- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-StorageTraits
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-03T23:39:59.457Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md storage portability plus canonical structured collaboration artifact family, base envelope, workflow-state contract, and structured-record readability [CX-DBP-001]/[ADD v02.167]/[ADD v02.168]/[ADD v02.171]/[ADD v02.166]
- Codex: .GOV/codex/Handshake_Codex_v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/locus/task_board.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/migrations/
- SEARCH_TERMS:
  - supports_structured_collab_artifacts
  - structured_collab_work_packet_row
  - structured_collab_micro_task_metadata
  - structured_collab_micro_task_status_rows
  - structured_collab_micro_task_rows
  - locus_task_board_update_work_packet
  - ensure_locus_sqlite
  - workflow_state_family
  - queue_reason_code
  - allowed_action_ids
- RUN_COMMANDS:
  ```bash
rg -n "supports_structured_collab_artifacts|structured_collab_work_packet_row|structured_collab_work_packet_rows|structured_collab_micro_task_metadata|structured_collab_micro_task_status_rows|structured_collab_micro_task_rows|locus_task_board_update_work_packet|ensure_locus_sqlite" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Postgres rows drift on base-envelope or workflow-state fields" -> "portable readers and small-model consumers see inconsistent structured truth"
  - "task-board sync remains SQLite-only while readers are implemented" -> "parity claim becomes false because projection truth still diverges by backend"
  - "migration coverage is not portable" -> "backend portability law is violated even if local tests pass"
  - "packet runs in parallel with boundary refactor on overlapping files" -> "implementation races and invalid governance truth"
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
- **Target File**: `N/A (fill after implementation)`
- **Start**: N/A
- **End**: N/A
- **Line Delta**: N/A
- **Pre-SHA1**: `N/A`
- **Post-SHA1**: `N/A`
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
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS:
- What changed in this update:
- Requirements / clauses self-audited:
- Checks actually run:
- Known gaps / weak spots:
- Heuristic risks / maintainability concerns:
- Validator focus request:
- Rubric contract understanding proof:
- Rubric scope discipline proof:
- Rubric baseline comparison:
- Rubric end-to-end proof:
- Rubric architecture fit self-review:
- Rubric heuristic quality self-review:
- Rubric anti-gaming / counterfactual check:
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check:
- Signed-scope debt ledger:
- Data contract self-check:
- Next step / handoff hint:

## MERGE_PROGRESSION_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, PASS closure is two-step and must stay explicit:
  - validator PASS append before merge-to-main:
    - set `**Status:** Done`
    - set `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - keep `MERGED_MAIN_COMMIT: NONE`
    - keep `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
    - project Task Board `## Done` token as `[MERGE_PENDING]`
  - after merge-to-main authority finishes and local `main` actually contains the approved closure commit:
    - set `**Status:** Validated (PASS)`
    - set `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - record `MERGED_MAIN_COMMIT: <main-contained sha>`
    - record `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - project Task Board `## Done` token as `[VALIDATED]`
- `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` must use `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`.
- Historical packet variants older than `PACKET_FORMAT_VERSION 2026-03-25` keep their legacy closure semantics and must not be rewritten in place only to satisfy this newer merge-progression model.

## SIGNED_SCOPE_COMPATIBILITY_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-26`, final-lane PASS clearance must record current-`main` compatibility explicitly before `just validator-gate-commit`:
  - if the signed packet still covers the required integration work against current local `main`, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NOT_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: N/A`
  - if current local `main` introduces adjacent shared-surface work outside the signed packet, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: ADJACENT_SCOPE_REQUIRED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: <new WP id / packet id / audit id / short governed rationale>`
  - if compatibility cannot be checked honestly, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: BLOCKED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NONE`
    - `PACKET_WIDENING_EVIDENCE: N/A`
- `CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN` is legal only before final-lane compatibility review starts.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `N/A (fill during implementation)`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, merge progression truth is part of closure law:
  - `**Status:** Done` means PASS is recorded but main containment is still pending and requires:
    - `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - `MERGED_MAIN_COMMIT: NONE`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
  - `**Status:** Validated (PASS)` is reserved for closures already contained in local `main` and requires:
    - `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - `MERGED_MAIN_COMMIT: <main-contained sha>`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
  - `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` require `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, every appended governed validation report MUST also include these completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
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
- For `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, every appended governed validation report MUST also include:
  - `ANTI_VIBE_FINDINGS:`
    - `- NONE` only when the validator found no shallow easy-surface work, no weakly justified implementation, and no vibe-coded behavior inside signed scope
    - otherwise list each anti-vibe finding explicitly
  - `SIGNED_SCOPE_DEBT:`
    - `- NONE` only when no signed-scope debt, cleanup IOU, or "fix later" residue was accepted
    - otherwise list each signed-scope debt item explicitly
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, every appended governed validation report MUST also include:
  - `DATA_CONTRACT_PROOF:`
    - list concrete code paths, emitted artifacts, or storage/query surfaces proving the active data contract was reviewed
  - `DATA_CONTRACT_GAPS:`
    - `- NONE` only when no SQL-portability, LLM-parseability, or Loom-intertwined gap remains inside signed scope
    - otherwise list each remaining gap explicitly
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, `LEGAL_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `LEGAL_VERDICT=PASS` is legal only when `DATA_CONTRACT_PROOF` is present and `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.
