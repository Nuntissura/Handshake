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
- MERGE_BASE_SHA: f85d767d8ae8a56121f224f6e12ed2df6f973d6b
- MERGE_BASE_NOTE: authoritative reviewable closeout surface is the full merge-base candidate `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`, preserving the bounded eight-file migration/runtime/storage/workflow parity bundle for final-lane containment
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
- **Status:** In Progress
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: cc00f1187e233b0bf228522e9f716d3f9ef2478e
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-04T21:20:00Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
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
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-postgres-structured-collaboration-artifact-parity-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-postgres-structured-collaboration-artifact-parity-v1
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
Next: ORCHESTRATOR advances verdict progression and integration closeout from the authoritative completed direct-review lane.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/migrations/ | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/workflows.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Base structured schema and project-profile extension contract [ADD v02.168] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/locus/task_board.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board; cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collab | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Conformance requirement 11 structured-record readability [ADD v02.166] | CODE_SURFACES: src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab; cargo test --manifest-path src/backend/handshake_core/Cargo.toml task_board | EXAMPLES: the same canonical work-packet row round-trips on SQLite and PostgreSQL with matching `project_profile_kind`, `mirror_state`, and summary semantics, the same canonical micro-task metadata and status rows round-trip on SQLite and PostgreSQL with matching workflow-state and queue-reason fields, a bounded task-board projection path reads or updates the same authoritative structured state on SQLite and PostgreSQL | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
- WAIVER_ID: CX-573F-20260404-PARITY-MAIN-STORAGE-TESTS | STATUS: ACTIVE | COVERS: SCOPE | SCOPE: authoritative-main assessment of already-performed `src/backend/handshake_core/src/storage/tests.rs` drift during parity v1 contained-main accounting | JUSTIFICATION: Operator granted a conditional waiver on 2026-04-04 for already-performed out-of-scope work if it is correct versus the Master Spec; both CODER and INTEGRATION_VALIDATOR later assessed the authoritative-main `storage/tests.rs` drift as correct enough and non-semantic outside test-facing/backend-proof hooks. | APPROVER: Operator (chat, 2026-04-04) | EXPIRES: when WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 closeout is resolved
- WAIVER_ID: CX-573F-20260404-PARITY-TOKEN-BUDGET-CONTINUATION | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED / POLICY_CONFLICT on WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 during crash-recovery finish pass | JUSTIFICATION: Operator explicitly instructed autonomous continuation until this WP is finished and governance is satisfied after the prior orchestrator-managed parallel run had already exceeded the governed token budget. This waiver authorizes bounded continuation without pretending the budget overrun did not occur. | APPROVER: Operator (chat, 2026-04-04) | EXPIRES: when WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 reaches an honest closeout verdict

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
- **Target File**: `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.down.sql`
- **Start**: 1
- **End**: 4
- **Line Delta**: 4
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `10f91d67f4986604a02ab7aec3b7bd21d15c558b`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate includes the bounded numbered rollback pair from the real merge-base, not only the final storage-hook repair
- **Target File**: `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.sql`
- **Start**: 1
- **End**: 36
- **Line Delta**: 36
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `b184d1c31073ce7beb88f67c4bba4358423ba280`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate includes the bounded numbered schema bootstrap pair from the real merge-base, not only the final storage-hook repair
- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 1
- **End**: 1169
- **Line Delta**: -4
- **Pre-SHA1**: `a3bdbe81c302f8fdbefd260bff808c12b2181ee8`
- **Post-SHA1**: `6e47ee37dea39cfaa04a6dbcd650242b4aca1ca0`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate still carries the earlier SQLite-side bounded locus helper narrowing that validators already reviewed on the same packet surface
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 2085
- **Line Delta**: 160
- **Pre-SHA1**: `931b2f54ed60b3415e588a23b076b670e0419d74`
- **Post-SHA1**: `70cc848c40debdcfcdaa4442c1644781b38d6ad9`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate still carries the earlier shared test-hook and capability seam changes from the real merge-base
- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 19
- **End**: 6619
- **Line Delta**: 1036
- **Pre-SHA1**: `d1b3b82d78a9fe77716cb7762449b8c2cc6ace88`
- **Post-SHA1**: `1ed62dcea80beb0242ad25a889d31a7b14692888`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate still carries the earlier bounded runtime bootstrap, row reader, and parity proof deltas from the real merge-base
- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 6377
- **Line Delta**: 146
- **Pre-SHA1**: `1b2af0d384fc9bd7d5f04679cb60a63c062bed59`
- **Post-SHA1**: `5736f281a5531b3fd9b8a5e5912b3ea8d3fd23b4`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate still carries the earlier SQLite-side artifact and row-shape alignment from the real merge-base
- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 1
- **End**: 3291
- **Line Delta**: -416
- **Pre-SHA1**: `be0a61ca3a6f05cc015729a901ab952d911e5b6f`
- **Post-SHA1**: `9363f58aedbf4e96cc50fcb02be33292f2942fbd`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] current_file_matches_preimage
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate now truthfully records the accepted authoritative-main test-surface drift inside the full merge-base candidate, not only the final incremental repair
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 24777
- **Line Delta**: 110
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `9cc3dda93d199334bca2aeaf9a3234319f344bc0`
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
- **Lint Results**: refreshed deterministic manifest for committed full candidate range `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`; final-lane proof commands recorded below
- **Artifacts**: refreshed full signed candidate surface at `.GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch`
- **Timestamp**: 2026-04-04T21:20:00Z
- **Operator**: `orchestrator:wp-1-postgres-structured-collaboration-artifact-parity-v1-recovery`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: full closeout candidate still carries the earlier chronology-preserving task-board projection and structured artifact emission repairs from the real merge-base
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: Done; committed head `8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af` is the current closeout candidate, the authoritative merge-base candidate surface is `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`, and the lane is ready for renewed `INTEGRATION_VALIDATOR` closeout once the packet-declared backup branch resolves to this head in `handshake_main`.
- What changed in this update: Rehydrated the full committed parity candidate into authoritative packet truth instead of describing only the last four-file repair. From the real merge-base, the closeout candidate carries the bounded `0016` migration pair, the `src/backend/handshake_core/src/storage/locus_sqlite.rs` helper narrowing, the chronology-preserving and structured-artifact repairs in `src/backend/handshake_core/src/workflows.rs`, and the final storage-hook alignment in `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/storage/postgres.rs`, `src/backend/handshake_core/src/storage/sqlite.rs`, and `src/backend/handshake_core/src/storage/tests.rs`. The signed patch artifact has been refreshed to that full candidate surface so closeout truth matches the actual committed head.
- Requirements / clauses self-audited: Storage Backend Portability Architecture [CX-DBP-001] and Dual-Backend Testing Early [CX-DBP-013]; Canonical structured collaboration artifact family [ADD v02.167]; Base structured schema and project-profile extension contract [ADD v02.168]; Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]; Conformance requirement 11 structured-record readability [ADD v02.166].
- Checks actually run: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::sqlite_persists_mutation_traceability_metadata_on_writes -- --exact --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::postgres_persists_mutation_traceability_metadata_on_writes -- --exact --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields -- --exact --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib postgres_structured_collab`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib workflows::tests::task_board_projection_preserves_updated_at_then_wp_id_order -- --exact --nocapture`; `git diff --name-only f85d767d8ae8a56121f224f6e12ed2df6f973d6b 8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af -- src/backend/handshake_core`; `git diff --numstat f85d767d8ae8a56121f224f6e12ed2df6f973d6b 8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af -- src/backend/handshake_core`; `git diff --check f85d767d8ae8a56121f224f6e12ed2df6f973d6b 8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af -- src/backend/handshake_core`.
- Known gaps / weak spots: No new in-scope product blocker remains from this repair. The packet still intentionally leaves broader Postgres Locus dependency/query-ready/get-status/get-progress/sync-task-board operations explicit `NotImplemented`, because the signed scope remains bounded to the canonical artifact/work-packet/micro-task path and its test proof surface.
- Heuristic risks / maintainability concerns: Future shared storage tests should keep using trait-level backend hooks instead of per-test backend downcasts, or the packet-owned test surface will drift against authoritative `main` again without changing product semantics. The Loom graph capability/perf metadata now lives on the shared backend trait for test intent clarity; future backend-specific performance expectations should update those trait hooks rather than hardcoding backend checks in tests.
- Validator focus request: Recheck the full merge-base candidate surface on `8e90cc8`: the packet-declared signed patch artifact must now match the actual committed head across the `0016` migration pair, `storage/locus_sqlite.rs`, `storage/mod.rs`, `storage/postgres.rs`, `storage/sqlite.rs`, `storage/tests.rs`, and `workflows.rs`, and current-local-main compatibility should remain honest against baseline `cc00f1187e233b0bf228522e9f716d3f9ef2478e` without widening.
- Rubric contract understanding proof: The packet does not ask for generic Postgres enablement; it asks for parity on the canonical structured collaboration storage and emitted artifact surfaces, including bounded task-board projection, while preserving scope discipline and truthful unsupported paths.
- Rubric scope discipline proof: The authoritative closeout candidate is the full signed packet surface from merge-base `f85d767d8ae8a56121f224f6e12ed2df6f973d6b` to head `8e90cc87eb262b1d56039ce0f581d9d7f9b4b2af`: `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.down.sql`, `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.sql`, `src/backend/handshake_core/src/storage/locus_sqlite.rs`, `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/storage/postgres.rs`, `src/backend/handshake_core/src/storage/sqlite.rs`, `src/backend/handshake_core/src/storage/tests.rs`, and `src/backend/handshake_core/src/workflows.rs`. No additional product file outside that signed surface is being introduced by this governance recovery repair.
- Rubric baseline comparison: Before this packet truth repair, the packet and signed patch artifact had collapsed onto only the last four-file storage-hook update, so final-lane closeout could not honestly compare the actual committed head against the declared clean-room surface. After this repair, the packet, manifest, and signed patch artifact all describe the same full candidate already reviewed across the packet's bounded parity history, and the remaining blocker is mechanical final-lane reachability plus closeout rerun rather than undisclosed product scope.
- Rubric end-to-end proof: Both exact storage traceability tests pass through the new shared hooks, the exact Postgres parity proof still passes, the broader `postgres_structured_collab` filter still resolves to that proof and passes, and the exact task-board ordering proof still passes on the committed head.
- Rubric architecture fit self-review: The repair keeps backend-specific behavior in the storage trait/backends rather than in duplicated test SQL. The tests now express intent through backend-neutral hooks, which is the right seam for shared storage conformance behavior.
- Rubric heuristic quality self-review: I aligned with authoritative `main` instead of inventing a branch-local test abstraction. The shared trait hooks are narrow, test-only where appropriate, and avoid dragging packet semantics into production callers.
- Rubric anti-gaming / counterfactual check: If the shared `MutationTraceabilityRow` or backend hook methods were absent, the exact SQLite/Postgres traceability tests would either fail to compile or fall back to duplicated backend SQL again. If the storage tests still depended on backend downcasts and inline SQL only, the previously blocking authoritative-main drift would still exist after this closeout turn.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: The containment claim is backed by exact storage traceability tests, exact Postgres/task-board parity proofs, and a deterministic `post-work --range` pass on the committed head, not by a claim that the files merely "look aligned." The remaining unsupported operations are still called out explicitly rather than hidden behind vague success paths.
- Signed-scope debt ledger: NONE within signed scope. Remaining unsupported Postgres Locus operations and tables are outside the validator-approved narrow parity seam and stay explicitly unsupported for now.
- Data contract self-check: The emitted packet, micro-task, and task-board artifacts are still built from the same tracked structures and compact metadata envelopes across both backends. I did not introduce any new SQLite-only record shape or backend-specific artifact schema.
- Next step / handoff hint: Hand the WP to `INTEGRATION_VALIDATOR` for renewed final-lane closeout on committed head `8e90cc8`, focused on the full merge-base candidate surface, refreshed signed-scope patch artifact, packet-declared backup-branch reachability, and truthful current-main compatibility against `cc00f1187e233b0bf228522e9f716d3f9ef2478e`.

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
- REQUIREMENT: "Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013]"
- EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1926`, `src/backend/handshake_core/src/storage/postgres.rs:164`, `src/backend/handshake_core/src/storage/postgres.rs:1615`
- REQUIREMENT: "Canonical structured collaboration artifact family [ADD v02.167]"
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2570`, `src/backend/handshake_core/src/workflows.rs:3604`, `src/backend/handshake_core/src/workflows.rs:3842`
- REQUIREMENT: "Base structured schema and project-profile extension contract [ADD v02.168]"
- EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:948`, `src/backend/handshake_core/src/workflows.rs:4468`
- REQUIREMENT: "Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171]"
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:4250`, `src/backend/handshake_core/src/workflows.rs:4285`, `src/backend/handshake_core/src/workflows.rs:6596`
- REQUIREMENT: "Conformance requirement 11 structured-record readability [ADD v02.166]"
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:23921`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- PROOF_LINES: existing unrelated dead-code warnings remained; the handshake_core crate completed `cargo check` with the new Postgres parity seam in place
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib postgres_structured_collab`
- EXIT_CODE: 0
- PROOF_LINES: `test workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib migrations_are_replay_safe_postgres`
- EXIT_CODE: 0
- PROOF_LINES: `test storage::tests::migrations_are_replay_safe_postgres ... ok` (rerun as unchanged-migration regression coverage; no numbered migration file changed in this repair)
- COMMAND: `git diff --check -- src/backend/handshake_core/src/storage/postgres.rs`
- EXIT_CODE: 0
- PROOF_LINES: git reported no whitespace or patch-shape violations; only CRLF normalization warnings were emitted during staging
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::sqlite_persists_mutation_traceability_metadata_on_writes -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test storage::tests::sqlite_persists_mutation_traceability_metadata_on_writes ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::postgres_persists_mutation_traceability_metadata_on_writes -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test storage::tests::postgres_persists_mutation_traceability_metadata_on_writes ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib postgres_structured_collab`
- EXIT_CODE: 0
- PROOF_LINES: `test workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields ... ok`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib workflows::tests::task_board_projection_preserves_updated_at_then_wp_id_order -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test workflows::tests::task_board_projection_preserves_updated_at_then_wp_id_order ... ok`
- COMMAND: `git diff --check -- src/backend/handshake_core/src/storage/mod.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/tests.rs`
- EXIT_CODE: 0
- PROOF_LINES: git reported no whitespace or patch-shape violations across the full eight-file closeout candidate surface; only CRLF normalization warnings were emitted during checkout/staging
- COMMAND: `just post-work WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 --range f85d767d8ae8a56121f224f6e12ed2df6f973d6b..HEAD`
- EXIT_CODE: 0
- PROOF_LINES: `post-work-check: PASS`; `wp-communication-health-check: PASS`; `RESULT: PASS`

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

### 2026-04-04T08:25:20.648Z | INTEGRATION_VALIDATOR PASS REPORT
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-postgres-structured-collaboration-artifact-parity-v1
COMMITTED_RANGE: f85d767d8ae8a56121f224f6e12ed2df6f973d6b..c2af3df1ba52bc762c6c5542b19753eda65a5438
SIGNED_PATCH_ARTIFACT: .GOV/task_packets/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/signed-scope.patch
REVIEW_EXCHANGE_PROOF: CODER `CODER_HANDOFF` at `2026-04-04T07:59:07.557Z`, WP_VALIDATOR `VALIDATOR_REVIEW` `PASS` at `2026-04-04T08:19:33.949Z`, and INTEGRATION_VALIDATOR `VALIDATOR_RESPONSE` `PASS` at `2026-04-04T08:25:20.648Z` are all recorded under `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1/RECEIPTS.jsonl` on correlation `review:WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1:coder_handoff:mnk1jscx:c3c115`.
Verdict: PASS
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
CLAUSES_REVIEWED:
- Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] reviewed against the committed range and confirmed by the Postgres schema bootstrap at `src/backend/handshake_core/src/storage/postgres.rs:164`, the backend activation path at `src/backend/handshake_core/src/storage/postgres.rs:1614`, the shared task-board ordering seam at `src/backend/handshake_core/src/workflows.rs:3140`, and the Postgres replay and undo tripwires at `src/backend/handshake_core/src/storage/tests.rs:3175` and `src/backend/handshake_core/src/storage/tests.rs:3253`.
- Canonical structured collaboration artifact family [ADD v02.167] reviewed against the committed range and confirmed by the work-packet, micro-task, and task-board artifact materialization paths at `src/backend/handshake_core/src/workflows.rs:3613`, `src/backend/handshake_core/src/workflows.rs:4553`, `src/backend/handshake_core/src/workflows.rs:4614`, and the exact parity proof at `src/backend/handshake_core/src/workflows.rs:23944`.
- Base structured schema and project-profile extension contract [ADD v02.168] reviewed against the committed range and confirmed by base-envelope field emission at `src/backend/handshake_core/src/workflows.rs:4553`, `src/backend/handshake_core/src/workflows.rs:4614`, and the Postgres artifact assertions for `project_profile_kind` and `mirror_state` at `src/backend/handshake_core/src/workflows.rs:23992`, `src/backend/handshake_core/src/workflows.rs:23993`, `src/backend/handshake_core/src/workflows.rs:23998`, and `src/backend/handshake_core/src/workflows.rs:23999`.
- Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] reviewed against the committed range and confirmed by task-board row projection at `src/backend/handshake_core/src/workflows.rs:3681`, `src/backend/handshake_core/src/workflows.rs:3693`, `src/backend/handshake_core/src/workflows.rs:3694`, `src/backend/handshake_core/src/workflows.rs:3695`, the work-packet and micro-task artifact emitters at `src/backend/handshake_core/src/workflows.rs:4559`, `src/backend/handshake_core/src/workflows.rs:4560`, `src/backend/handshake_core/src/workflows.rs:4561`, `src/backend/handshake_core/src/workflows.rs:4620`, `src/backend/handshake_core/src/workflows.rs:4621`, and `src/backend/handshake_core/src/workflows.rs:4622`, plus the shared ordering proof at `src/backend/handshake_core/src/workflows.rs:24015`.
- Conformance requirement 11 structured-record readability [ADD v02.166] reviewed against the committed range and confirmed by the structured JSON artifact assertions at `src/backend/handshake_core/src/workflows.rs:23992` through `src/backend/handshake_core/src/workflows.rs:24008` and the chronology-preserving task-board projection proof at `src/backend/handshake_core/src/workflows.rs:24015`.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- NONE
VALIDATOR_RISK_TIER: HIGH
DIFF_ATTACK_SURFACES:
- Shared task-board ordering can drift from persisted chronology and silently rewrite canonical row order, `display_order`, and downstream canonical hashes.
- Postgres schema bootstrap can drift from the numbered migration pair and create false parity that only works in one boot path.
- Canonical work-packet, micro-task, and task-board artifact fields can diverge by backend even when a row still exists.
- Signed-scope closeout can overclaim broader PostgreSQL Locus parity if unsupported operations stop failing explicitly.
INDEPENDENT_CHECKS_RUN:
- `git diff --check 814078e..c2af3df -- src/backend/handshake_core/src/workflows.rs` => clean; the final repair is scoped to `workflows.rs` only and introduces no patch-shape violations.
- `cargo test --manifest-path ..\\wtc-artifact-parity-v1\\src\\backend\\handshake_core\\Cargo.toml --lib workflows::tests::task_board_projection_preserves_updated_at_then_wp_id_order -- --exact --nocapture` => PASS; chronology now beats lexical `wp_id` order in emitted task-board rows.
- `cargo test --manifest-path ..\\wtc-artifact-parity-v1\\src\\backend\\handshake_core\\Cargo.toml --lib workflows::tests::postgres_structured_collab_artifacts_materialize_parity_fields -- --exact --nocapture` => PASS; Postgres emits the same bounded structured-collaboration fields as SQLite.
- `cargo test --manifest-path ..\\wtc-artifact-parity-v1\\src\\backend\\handshake_core\\Cargo.toml --lib postgres_structured_collab` => PASS; the packet-scoped Postgres tripwire still resolves to the exact artifact proof.
- `cargo test --manifest-path ..\\wtc-artifact-parity-v1\\src\\backend\\handshake_core\\Cargo.toml --lib storage::tests::migrations_are_replay_safe_postgres -- --exact --nocapture` => PASS; the numbered migration pair replays cleanly on PostgreSQL.
- `cargo test --manifest-path ..\\wtc-artifact-parity-v1\\src\\backend\\handshake_core\\Cargo.toml --lib storage::tests::migrations_can_undo_to_baseline_postgres -- --exact --nocapture` => PASS; the numbered migration pair unwinds cleanly back to baseline.
COUNTERFACTUAL_CHECKS:
- If `src/backend/handshake_core/src/workflows.rs:3140` reverts to `ORDER BY wp_id ASC`, `emit_task_board_projection_artifacts` will again derive canonical row order and `display_order` from lexical ids instead of persisted chronology.
- If `src/backend/handshake_core/src/storage/postgres.rs:164` or `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.sql:4` stop agreeing on the bounded `work_packets` and `micro_tasks` schema, one bootstrap path will pass while the other silently diverges.
- If `src/backend/handshake_core/src/workflows.rs:4553` or `src/backend/handshake_core/src/workflows.rs:4614` stop emitting the shared base-envelope fields, the exact Postgres parity proof at `src/backend/handshake_core/src/workflows.rs:23944` will lose `project_profile_kind`, `mirror_state`, `workflow_state_family`, or `queue_reason_code`.
BOUNDARY_PROBES:
- Storage-to-artifact boundary checked at `src/backend/handshake_core/src/workflows.rs:3140` -> `src/backend/handshake_core/src/workflows.rs:3613`; both SQLite and Postgres now feed task-board projection through the same chronology-preserving `ORDER BY updated_at ASC, wp_id ASC`.
- Schema-bootstrap boundary checked between `src/backend/handshake_core/src/storage/postgres.rs:164`, `src/backend/handshake_core/src/storage/postgres.rs:1614`, and `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.sql:4`; the bounded tables and fields match the runtime bootstrap and numbered migration pair.
NEGATIVE_PATH_CHECKS:
- `src/backend/handshake_core/src/workflows.rs:24015` exercises the prior failure mode directly by making lexical `wp_id` order disagree with chronological creation order and proving the emitted task-board rows still preserve chronology.
- `src/backend/handshake_core/src/storage/postgres.rs:956` keeps broader unsupported Locus operations explicit `NotImplemented`, so this packet does not silently widen into unrelated Postgres parity claims.
INDEPENDENT_FINDINGS:
- The final repair on `c2af3df` is genuinely narrow; only `src/backend/handshake_core/src/workflows.rs` changed relative to `814078e`, and the earlier Postgres migration pair plus runtime `SELECT EXISTS(...)` repair remain intact underneath it.
- No blocking product findings remain inside the signed bounded scope after the chronology fix.
RESIDUAL_UNCERTAINTY:
- `postgres_structured_collab` still resolves to a single exact proof test, so breadth confidence for the bounded scope still relies on the migration replay or undo tripwires plus direct code inspection.
- Cargo emitted non-fatal shared-target object-copy warnings during validation because another build lane had touched the shared target tree, but the validator-owned commands completed successfully.
SPEC_CLAUSE_MAP:
- Storage Backend Portability Architecture [CX-DBP-001] plus Dual-Backend Testing Early [CX-DBP-013] => `src/backend/handshake_core/src/storage/postgres.rs:164`, `src/backend/handshake_core/src/storage/postgres.rs:1614`, `src/backend/handshake_core/src/workflows.rs:3140`, `src/backend/handshake_core/src/storage/tests.rs:3175`, `src/backend/handshake_core/src/storage/tests.rs:3253`.
- Canonical structured collaboration artifact family [ADD v02.167] => `src/backend/handshake_core/src/workflows.rs:3613`, `src/backend/handshake_core/src/workflows.rs:4553`, `src/backend/handshake_core/src/workflows.rs:4614`, `src/backend/handshake_core/src/workflows.rs:23944`.
- Base structured schema and project-profile extension contract [ADD v02.168] => `src/backend/handshake_core/src/workflows.rs:4553`, `src/backend/handshake_core/src/workflows.rs:4614`, `src/backend/handshake_core/src/workflows.rs:23992`, `src/backend/handshake_core/src/workflows.rs:23993`, `src/backend/handshake_core/src/workflows.rs:23998`, `src/backend/handshake_core/src/workflows.rs:23999`.
- Project-agnostic workflow state, queue reason, and governed action contract [ADD v02.171] => `src/backend/handshake_core/src/workflows.rs:3681`, `src/backend/handshake_core/src/workflows.rs:3693`, `src/backend/handshake_core/src/workflows.rs:3694`, `src/backend/handshake_core/src/workflows.rs:3695`, `src/backend/handshake_core/src/workflows.rs:4559`, `src/backend/handshake_core/src/workflows.rs:4560`, `src/backend/handshake_core/src/workflows.rs:4561`, `src/backend/handshake_core/src/workflows.rs:4620`, `src/backend/handshake_core/src/workflows.rs:4621`, `src/backend/handshake_core/src/workflows.rs:4622`, `src/backend/handshake_core/src/workflows.rs:24015`.
- Conformance requirement 11 structured-record readability [ADD v02.166] => `src/backend/handshake_core/src/workflows.rs:23992`, `src/backend/handshake_core/src/workflows.rs:23993`, `src/backend/handshake_core/src/workflows.rs:23998`, `src/backend/handshake_core/src/workflows.rs:23999`, `src/backend/handshake_core/src/workflows.rs:24003`, `src/backend/handshake_core/src/workflows.rs:24004`, `src/backend/handshake_core/src/workflows.rs:24015`.
NEGATIVE_PROOF:
- Broader PostgreSQL Locus operation parity is still not fully implemented. The validator confirmed that `AddDependency`, `RemoveDependency`, `QueryReady`, `GetWpStatus`, `GetMtProgress`, and `SyncTaskBoard` remain explicit `NotImplemented` at `src/backend/handshake_core/src/storage/postgres.rs:956`, so this packet proves bounded canonical artifact parity only and does not overclaim wider Postgres support.
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
DATA_CONTRACT_PROOF:
- The numbered migration contract for bounded structured-collaboration tables was reviewed at `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.sql:4` and `src/backend/handshake_core/migrations/0016_locus_structured_collaboration.down.sql:3`, then revalidated by `src/backend/handshake_core/src/storage/tests.rs:3175` and `src/backend/handshake_core/src/storage/tests.rs:3253`.
- The emitted LLM-readable artifact contract was reviewed through the structured JSON assertions at `src/backend/handshake_core/src/workflows.rs:23992` through `src/backend/handshake_core/src/workflows.rs:24008`.
- The task-board chronology and row-order contract was reviewed through `src/backend/handshake_core/src/workflows.rs:3140` and proved by `src/backend/handshake_core/src/workflows.rs:24015`.
DATA_CONTRACT_GAPS:
- NONE
