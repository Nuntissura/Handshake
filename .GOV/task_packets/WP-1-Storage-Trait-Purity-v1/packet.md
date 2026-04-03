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

# Task Packet: WP-1-Storage-Trait-Purity-v1

## METADATA
- TASK_ID: WP-1-Storage-Trait-Purity-v1
- WP_ID: WP-1-Storage-Trait-Purity-v1
- BASE_WP_ID: WP-1-Storage-Trait-Purity
- DATE: 2026-04-03T00:19:07.793Z
- MERGE_BASE_SHA: d26f46d586d2c44a76dd40ffaadf8603972867c4
- MERGE_BASE_NOTE: committed handoff base for reviewable range `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`; use for deterministic `just post-work --range` evidence
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_A
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Storage-Trait-Purity-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Storage-Trait-Purity-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-trait-purity-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Storage-Trait-Purity-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Storage-Trait-Purity-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Storage-Trait-Purity-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Storage-Trait-Purity-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Storage-Trait-Purity-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Storage-Trait-Purity-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Storage-Trait-Purity-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Storage-Trait-Purity-v1
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
- **Status:** Done
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: MERGE_PENDING
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: bb66b2b06dd72e0a24438fbe2274620984aca69a
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-03T07:24:53.088Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: MEDIUM
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: MEDIUM
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Storage-Trait-Purity-v1
- LOCAL_WORKTREE_DIR: ../wtc-trait-purity-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Storage-Trait-Purity-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Storage-Trait-Purity-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja030420260212
- PACKET_FORMAT_VERSION: 2026-04-01

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: ORCHESTRATOR advances verdict progression and integration closeout from the completed direct-review lane.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: One Storage API [CX-DBP-010] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api/loom.rs; src/backend/handshake_core/src/storage/retention.rs; src/backend/handshake_core/src/storage/locus_sqlite.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier | EXAMPLES: a `Database` trait object backed by SQLite reports explicit backend identity or capability without requiring downcast access, a `Database` trait object backed by PostgreSQL rejects SQLite-only Locus or structured-artifact paths through explicit capability law or terminal error, not concrete-type checks, Loom search records the same `tier_used` semantics using explicit backend identity or capability queries instead of `as_any` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Trait Purity Invariant [CX-DBP-040] | CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src | EXAMPLES: a `Database` trait object backed by SQLite reports explicit backend identity or capability without requiring downcast access, a `Database` trait object backed by PostgreSQL rejects SQLite-only Locus or structured-artifact paths through explicit capability law or terminal error, not concrete-type checks, Loom search records the same `tier_used` semantics using explicit backend identity or capability queries instead of `as_any` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Dual-Backend Testing Early [CX-DBP-013] | CODE_SURFACES: src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | EXAMPLES: a `Database` trait object backed by SQLite reports explicit backend identity or capability without requiring downcast access, a `Database` trait object backed by PostgreSQL rejects SQLite-only Locus or structured-artifact paths through explicit capability law or terminal error, not concrete-type checks, Loom search records the same `tier_used` semantics using explicit backend identity or capability queries instead of `as_any` | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a `Database` trait object backed by SQLite reports explicit backend identity or capability without requiring downcast access
  - a `Database` trait object backed by PostgreSQL rejects SQLite-only Locus or structured-artifact paths through explicit capability law or terminal error, not concrete-type checks
  - Loom search records the same `tier_used` semantics using explicit backend identity or capability queries instead of `as_any`
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/mod.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/sqlite.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/postgres.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/retention.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/locus_sqlite.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/tests.rs (backend data surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: Loom
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLAR_DECOMPOSITION: PILLAR: Flight Recorder | CAPABILITY_SLICE: backend-neutral Loom search telemetry classification | SUBFEATURES: `tier_used` meaning, backend-kind exposure, and recorder payload stability without concrete-type checks | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: touched because current telemetry branches on concrete backend type
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: explicit backend identity and capability query surface | SUBFEATURES: backend_kind, supports(feature), removal of production downcasts | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability boundary the spec already requires
  - PILLAR_DECOMPOSITION: PILLAR: Loom | CAPABILITY_SLICE: backend-tier classification without concrete-type branching | SUBFEATURES: Loom search tier metadata and any backend-sensitive search path logic | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep current search semantics while removing hidden backend inspection
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: SQLite-only locus path expressed as explicit capability | SUBFEATURES: locus operation dispatch, readiness queries, task-board sync helpers | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LocusQueryReadyParams, PRIM-LocusGetWpStatusParams, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: if PostgreSQL does not support a path yet, that must be explicit and testable
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: explicit backend identity and capability queries on `Database` | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal runtime contract used by multiple backend consumers
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: structured collaboration artifact emission backend gate | JobModel: WORKFLOW | Workflow: structured_collaboration_artifact_emission | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: unsupported backend behavior must become explicit instead of requiring a SQLite downcast
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Loom search backend tier classification | JobModel: UI_ACTION | Workflow: NONE | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: LoomSearchExecuted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: preserve current telemetry semantics while removing concrete-type branching
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Locus SQLite-only operation routing via explicit backend feature gate | JobModel: WORKFLOW | Workflow: locus_operation_dispatch | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: make unsupported backend status explicit and deterministic
  - FORCE_MULTIPLIER_EXPANSION: `Database` trait plus Locus operation routing -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: `Database` trait plus Loom search telemetry -> IN_THIS_WP (stub: NONE)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/api/loom.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Storage-Trait-Purity-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 One Storage API [CX-DBP-010]
- CONTEXT_START_LINE: 3260
- CONTEXT_END_LINE: 3277
- CONTEXT_TOKEN: **Pillar 1: One Storage API [CX-DBP-010]**
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 2.3.13.1 Four Portability Pillars [CX-DBP-002]

  **Pillar 1: One Storage API [CX-DBP-010]**

  All database operations MUST flow through a single storage module boundary. No business logic code may directly access database connections.

  - FORBIDDEN: Direct `sqlx::query()` in API handlers
  - FORBIDDEN: Direct `state.pool` or `state.fr_pool` access outside `src/storage/`
  - REQUIRED: All DB operations via `state.storage.*` interface
  - REQUIRED: AppState MUST NOT expose raw `SqlitePool` or `DuckDbConnection`

  **Enforcement:**
  Pre-commit validation checks for direct pool access in API handlers (FAIL on violation).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.3 Trait Purity Invariant [CX-DBP-040]
- CONTEXT_START_LINE: 3361
- CONTEXT_END_LINE: 3368
- CONTEXT_TOKEN: **[CX-DBP-040] Trait Purity Invariant (Normative):**
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 2.3.13.3 Storage API Abstraction Pattern [CX-DBP-021]

  The storage module MUST define a trait-based interface that hides database differences. This contract is MANDATORY for all storage implementations.

  **[CX-DBP-040] Trait Purity Invariant (Normative):**
  The `Database` trait MUST NOT expose any methods that return concrete, backend-specific types (e.g., `SqlitePool`, `PgPool`, `DuckDbConnection`). All implementations MUST encapsulate their internal connection pools.
  - **Violation:** `fn sqlite_pool(&self) -> Option<&SqlitePool>` is strictly FORBIDDEN.
  - **Remediation:** Any service requiring database access (e.g., Janitor, Search) MUST consume the generic `Database` trait methods or be refactored into a trait-compliant operation.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.13.1 Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3313
- CONTEXT_END_LINE: 3323
- CONTEXT_TOKEN: **Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**
- EXCERPT_ASCII_ESCAPED:
  ```text
**Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**

  Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.

  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge

  **Rationale:**
  Testing only against SQLite means PostgreSQL bugs are discovered during Phase 2 migration (expensive). Testing both backends in Phase 1 catches portability issues immediately.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: One Storage API [CX-DBP-010] | WHY_IN_SCOPE: product code still makes behavior decisions by concrete backend type instead of staying behind one storage boundary | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/workflows.rs; src/backend/handshake_core/src/api/loom.rs; src/backend/handshake_core/src/storage/retention.rs; src/backend/handshake_core/src/storage/locus_sqlite.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier | RISK_IF_MISSED: business logic will continue to rely on hidden backend branching and portability claims will remain overstated
  - CLAUSE: Trait Purity Invariant [CX-DBP-040] | WHY_IN_SCOPE: the `Database` trait still exposes `as_any`, which enables backend-specific production logic through downcasts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/mod.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs; src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity; rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src | RISK_IF_MISSED: future call sites can keep escaping the trait boundary without clear audit pressure
  - CLAUSE: Dual-Backend Testing Early [CX-DBP-013] | WHY_IN_SCOPE: the refactor is only valuable if both backends remain explicit and testable after downcast removal | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/storage/tests.rs; src/backend/handshake_core/src/storage/sqlite.rs; src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability; cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier; cargo test --manifest-path src/backend/handshake_core/Cargo.toml | RISK_IF_MISSED: SQLite and Postgres behavior can drift again while still appearing green in narrow local checks
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `Database` trait object boundary | PRODUCER: storage/sqlite.rs and storage/postgres.rs implementations | CONSUMER: workflows.rs, api/loom.rs, storage/retention.rs, storage/locus_sqlite.rs | SERIALIZER_TRANSPORT: in-process trait object | VALIDATOR_READER: storage/tests.rs and source grep tripwires | TRIPWIRE_TESTS: database_trait_purity plus source grep for production downcasts | DRIFT_RISK: concrete backend assumptions can return silently through helper methods or trait escape hatches
  - CONTRACT: explicit backend identity or capability query surface | PRODUCER: `Database` implementations | CONSUMER: Loom search tier selection, structured-artifact emission, Locus routing, retention update paths | SERIALIZER_TRANSPORT: in-process enum or capability methods | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: database_trait_purity; locus_backend_capability; loom_search_backend_tier | DRIFT_RISK: capability names or semantics drift and callers start reintroducing ad hoc backend checks
  - CONTRACT: Locus SQLite-only operation gate | PRODUCER: storage/locus_sqlite.rs | CONSUMER: workflow and task-board coordination paths | SERIALIZER_TRANSPORT: in-process result or terminal error | VALIDATOR_READER: storage/tests.rs and Locus-facing tests | TRIPWIRE_TESTS: locus_backend_capability | DRIFT_RISK: unsupported backend behavior becomes silent or inconsistent instead of explicit
  - CONTRACT: Loom search `tier_used` recorder payload | PRODUCER: api/loom.rs | CONSUMER: Flight Recorder and downstream diagnostics | SERIALIZER_TRANSPORT: Flight Recorder JSON event payload | VALIDATOR_READER: loom tests and event assertions | TRIPWIRE_TESTS: loom_search_backend_tier | DRIFT_RISK: telemetry stays green while semantic backend classification changes unexpectedly
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add explicit backend identity or capability methods to the `Database` trait and remove or isolate `as_any` from production paths.
  - Refactor Loom, workflows, retention, and Locus callers to use the explicit trait surface instead of concrete-type branching.
  - Add regression tests that fail if production downcasts return or if backend-sensitive behavior becomes implicit again.
- HOT_FILES:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
- CARRY_FORWARD_WARNINGS:
  - Do not replace downcasts with hidden helper wrappers that still leak concrete backend types.
  - Do not silently no-op unsupported PostgreSQL paths; fail explicitly through capability law or deterministic terminal error.
  - Do not widen this packet into runtime DDL cleanup or unrelated storage feature work.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - One Storage API boundary remains intact in production code
  - Trait Purity Invariant is satisfied without production `as_any` escape hatches
  - backend-sensitive Locus, Loom, retention, and workflow behavior is explicit and tested
- FILES_TO_READ:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- COMMANDS_TO_RUN:
  - rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify production-source grep stays free of backend downcasts except for test-only or implementation-local code inside backend modules that does not cross the trait boundary
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The final shape of the explicit backend identity or capability API is not proven until coding and validation complete.
  - Full PostgreSQL runtime behavior is not proven at refinement time; it depends on the later coding and dual-backend validation pass.
  - Whether any test-only downcasts remain acceptable behind isolated test helpers will need inspection during implementation and validation.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Current-spec storage-portability law is already stronger than the current product implementation.
  - The missing work is removal of production downcast seams and replacement with explicit backend identity or capability contracts.
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
  - engine.dba
  - engine.context
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Locus
  - Loom
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - `Database` trait plus Locus operation routing -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus Loom search telemetry -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus workflow-side structured artifact emission -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus retention pin updates -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus dual-backend conformance tripwires -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus unsupported-backend failure semantics -> IN_THIS_WP (stub: NONE)
  - `Database` trait plus downstream Stage/media portability tracks -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: backend-neutral Loom search telemetry classification | SUBFEATURES: `tier_used` meaning, backend-kind exposure, and recorder payload stability without concrete-type checks | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: touched because current telemetry branches on concrete backend type
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: explicit backend identity and capability query surface | SUBFEATURES: backend_kind, supports(feature), removal of production downcasts | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits, PRIM-SqliteDatabase, PRIM-PostgresDatabase | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: this is the core portability boundary the spec already requires
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: structured-artifact emission capability gate | SUBFEATURES: workflow-side artifact emission path stops assuming SQLite storage | PRIMITIVES_FEATURES: PRIM-Database, PRIM-StorageTraits | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: runtime behavior must stay explicit when a backend lacks a SQLite-only path
  - PILLAR: Loom | CAPABILITY_SLICE: backend-tier classification without concrete-type branching | SUBFEATURES: Loom search tier metadata and any backend-sensitive search path logic | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LoomStorage | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep current search semantics while removing hidden backend inspection
  - PILLAR: Locus | CAPABILITY_SLICE: SQLite-only locus path expressed as explicit capability | SUBFEATURES: locus operation dispatch, readiness queries, task-board sync helpers | PRIMITIVES_FEATURES: PRIM-Database, PRIM-LocusQueryReadyParams, PRIM-LocusGetWpStatusParams, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.dba, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: if PostgreSQL does not support a path yet, that must be explicit and testable
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: explicit backend identity and capability queries on `Database` | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal runtime contract used by multiple backend consumers
  - Capability: structured collaboration artifact emission backend gate | JobModel: WORKFLOW | Workflow: structured_collaboration_artifact_emission | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: unsupported backend behavior must become explicit instead of requiring a SQLite downcast
  - Capability: Loom search backend tier classification | JobModel: UI_ACTION | Workflow: NONE | ToolSurface: UI_ONLY | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: LoomSearchExecuted | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: preserve current telemetry semantics while removing concrete-type branching
  - Capability: Locus SQLite-only operation routing via explicit backend feature gate | JobModel: WORKFLOW | Workflow: locus_operation_dispatch | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: make unsupported backend status explicit and deterministic
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Storage-Trait-Purity-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Storage-No-Runtime-DDL-v1 -> KEEP_SEPARATE
  - WP-1-Stage-Media-Artifact-Portability-v1 -> KEEP_SEPARATE
  - WP-1-Storage-Abstraction-Layer-v3 -> KEEP_SEPARATE
  - WP-1-Loom-Storage-Portability-v2 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
  - ../handshake_main/src/backend/handshake_core/src/api/loom.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/retention.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/locus_sqlite.rs -> PARTIAL (WP-1-Storage-Trait-Purity-v1)
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
- What: Remove production `Database` downcast escape hatches and replace them with explicit backend identity or capability queries so Loom, Locus, retention, and workflow code remain inside the storage abstraction boundary.
- Why: Current product code still depends on `as_any` and concrete `SqliteDatabase` checks, which violates the current portability law and hides SQLite-only behavior behind implementation details instead of explicit capability semantics.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- OUT_OF_SCOPE:
  - runtime DDL cleanup and migration restructuring
  - broader Stage/media portability work
  - new GUI surfaces or new Flight Recorder event families
  - unrelated storage feature expansion beyond explicit backend identity or capability semantics
- TOUCHED_FILE_BUDGET: 8
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
cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
```

### DONE_MEANS
- Production code no longer depends on `Database` downcasts for behavior decisions.
- Unsupported backend behavior is explicit through backend identity or capability methods, not hidden concrete-type branching.
- Loom, Locus, retention, and workflow-side consumers keep their intended behavior without using `as_any`.
- Dual-backend or backend-sensitive tripwire tests fail if a production downcast seam returns.

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
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-03T00:19:07.793Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md One Storage API plus Trait Purity Invariant [CX-DBP-010]/[CX-DBP-040]
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
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/storage/locus_sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/api/loom.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - as_any
  - downcast_ref
  - SqliteDatabase::pool
  - backend_kind
  - supports(
  - sqlite_pool_for_structured_artifacts
  - locus sqlite
  - tier_used
- RUN_COMMANDS:
  ```bash
rg -n "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Production code keeps `as_any` escape hatches" -> "backend divergence can reappear without audit visibility"
  - "SQLite-only behavior remains implicit" -> "Postgres readiness is overstated and failures surface late"
  - "Loom or Locus semantics drift during the refactor" -> "runtime behavior changes without matching capability gates or tests"
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
- **Target File**: `src/backend/handshake_core/src/api/loom.rs`
- **Start**: 1057
- **End**: 1366
- **Line Delta**: 49
- **Pre-SHA1**: `1c4f67e690bfaec4790e5b14669342de41206bad`
- **Post-SHA1**: `3e3c6bbe3a79c449632ed9ac268ab6408d9f1f45`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/api/loom.rs`
- **Artifacts**: `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/signed-scope.patch`; `git show --stat --summary --format=fuller be50f673a26aead6c2af4cc43037e84b15f3392b`; `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/api/loom.rs`
- **Timestamp**: `2026-04-03T05:21:37.5518914Z`
- **Operator**: `ORCHESTRATOR`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: manifest refreshed after coder proof-semantic repair commit `be50f673a26aead6c2af4cc43037e84b15f3392b`; commit-bounded row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`
- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 17
- **End**: 1147
- **Line Delta**: -21
- **Pre-SHA1**: `a3bdbe81c302f8fdbefd260bff808c12b2181ee8`
- **Post-SHA1**: `a681dcf66bf5311fdb1d2cb6a6589902a50b8d41`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 24
- **End**: 2072
- **Line Delta**: 172
- **Pre-SHA1**: `931b2f54ed60b3415e588a23b076b670e0419d74`
- **Post-SHA1**: `ef003795647e82b956fca7e5683acff3ba6887b9`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/mod.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/mod.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 719
- **End**: 957
- **Line Delta**: 221
- **Pre-SHA1**: `d1b3b82d78a9fe77716cb7762449b8c2cc6ace88`
- **Post-SHA1**: `54513af2b9e21daf9608e0f714e78c08a5993c11`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/postgres.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/postgres.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
- **Target File**: `src/backend/handshake_core/src/storage/retention.rs`
- **Start**: 574
- **End**: 588
- **Line Delta**: -13
- **Pre-SHA1**: `3a089db581a754e8500385c74bac71b08d482012`
- **Post-SHA1**: `5e24dd5644d820b580bbbd6f1b484b5bd04bd4d2`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/retention.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/retention.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1035
- **End**: 1372
- **Line Delta**: 318
- **Pre-SHA1**: `1b2af0d384fc9bd7d5f04679cb60a63c062bed59`
- **Post-SHA1**: `15dc6470ef2cf2acb5db3acd925ae9ac02c532c0`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/sqlite.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/storage/sqlite.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 731
- **End**: 3107
- **Line Delta**: -106
- **Pre-SHA1**: `be0a61ca3a6f05cc015729a901ab952d911e5b6f`
- **Post-SHA1**: `05ee3f5ac744ba8b87ed2d11a3c3f79451093f90`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/storage/tests.rs`
- **Artifacts**: `git show --stat --summary --format=fuller be50f673a26aead6c2af4cc43037e84b15f3392b`; `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/storage/tests.rs`
- **Timestamp**: `2026-04-03T05:21:37.5518914Z`
- **Operator**: `ORCHESTRATOR`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: manifest refreshed after coder proof-semantic repair commit `be50f673a26aead6c2af4cc43037e84b15f3392b`; commit-bounded row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 49
- **End**: 4616
- **Line Delta**: -153
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `8178f2c2b592847293a23ddc8a4eea32ed0ddbd4`
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
- **Lint Results**: commit-window manifest derived from `git diff --unified=0 d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/workflows.rs`
- **Artifacts**: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 518066ae4b382b8f3d9550eac19f316f6798c914 -- src/backend/handshake_core/src/workflows.rs`
- **Timestamp**: `2026-04-03T02:34:09.3837331Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: commit-bounded manifest row for `d26f46d586d2c44a76dd40ffaadf8603972867c4..518066ae4b382b8f3d9550eac19f316f6798c914`
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: `In Progress`; committed product work now ends at `be50f673a26aead6c2af4cc43037e84b15f3392b`, and this packet companion is refreshed to that head for deterministic validator review.
- What changed in this update: the earlier storage capability-law repair from `518066a` remains intact, `54978d0` restored the named storage proof filters, and the latest proof-semantic repair commit `be50f67` moves `loom_search_backend_tier` into `src/backend/handshake_core/src/api/loom.rs` so the signed tripwire now exercises the real Loom API path and asserts the emitted Flight Recorder `tier_used` payload contract. The storage-only graph-filter proof remains in `src/backend/handshake_core/src/storage/tests.rs` under the truthful name `loom_search_graph_filter_backend_support`.
- Requirements / clauses self-audited: One Storage API [CX-DBP-010], Trait Purity Invariant [CX-DBP-040], and Dual-Backend Testing Early [CX-DBP-013] across the signed 8-file storage surface, specifically the capability-gated `Database` methods, Locus/structured-collaboration accessors, Loom tier routing, retention helper usage, and workflow-side generic reads.
- Checks actually run: `just pre-work WP-1-Storage-Trait-Purity-v1`; `git show --stat --summary --format=fuller be50f673a26aead6c2af4cc43037e84b15f3392b`; `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/api/loom.rs src/backend/handshake_core/src/storage/locus_sqlite.rs src/backend/handshake_core/src/storage/mod.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/retention.rs src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/src/workflows.rs`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_backend_tier -- --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml database_trait_purity -- --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml locus_backend_capability -- --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml loom_search_graph_filter_backend_support -- --nocapture`; `POSTGRES_TEST_URL=postgres://postgres:postgres@127.0.0.1:5432/handshake_test cargo test --manifest-path src/backend/handshake_core/Cargo.toml postgres_structured_collab_artifacts_are_capability_denied -- --nocapture`; `cargo test --manifest-path src/backend/handshake_core/Cargo.toml -- --nocapture`.
- Known gaps / weak spots: the packet companion was the only live blocker after `be50f67`; product code is not reopened by this governance repair. The one transient full-suite failure seen with `POSTGRES_TEST_URL` set reran green in isolation, so validator review should treat it as an environment wrinkle unless it reproduces independently.
- Heuristic risks / maintainability concerns: the portability law now lives in small capability methods plus generic trait accessors, so future backend additions can regress silently if they update only one backend impl or only one downstream consumer; validator review should check that the signed 8-file seam stays symmetric.
- Validator focus request: validate the eight manifest rows against `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`, confirm that `loom_search_backend_tier` now proves the Flight Recorder `tier_used` payload contract through `api/loom.rs`, and treat any remaining blocker after gate reruns as either governance-runtime drift or an independently reproduced product issue.
- Rubric contract understanding proof: the signed packet promises removal of production downcast behavior decisions, explicit backend capability law, and preserved Loom/Locus/retention/workflow behavior inside the exact eight-file storage scope; this handoff now points those promises at concrete code lines and commit-window hashes instead of placeholder prose.
- Rubric scope discipline proof: no product files were edited in this governance repair turn; the only intended change is this packet companion, and all manifest evidence is pinned to the committed `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b` eight-file diff.
- Rubric baseline comparison: the repaired packet evidence is anchored to the single committed range `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`; every manifest row, SHA1 pair, and line window was derived from that range rather than from current worktree state or a broader branch diff.
- Rubric end-to-end proof: the strongest end-to-end claim I can defend here is committed-handoff hygiene, not new implementation. The packet now names the exact storage trait/capability seams, the validator-facing manifest covers all eight changed files, and the proof commands in `## EVIDENCE` are commit-aware rather than conversational.
- Rubric architecture fit self-review: the current repair range ending at `be50f67` fits the existing storage architecture because it keeps backend-specific law on the backend impls and keeps consumers on `Database`, while the added named tests keep the signed proof surface honest without widening product scope.
- Rubric heuristic quality self-review: this is mechanically stronger than the prior placeholder packet, but it still depends on honest validator rereads of the capability-law seams; a bad reviewer could over-trust the packet if they skip the underlying code.
- Rubric anti-gaming / counterfactual check: if any manifest row were omitted, `post-work-check` would lose changed-file coverage; if the file:line anchors were replaced with vague prose, the evidence mapping would become non-auditable; if I described the product as "complete" without rerunnable commit-window hashes, the handoff would be pure narration.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: this repair does not pretend packet prose is product proof. The substantive claims are limited to manifest rows, code anchors, and commands actually run; anything I did not rerun in this turn is not presented as fresh evidence.
- Signed-scope debt ledger: `NONE` inside this packet repair. If `wp-coder-handoff` remains blocked after the packet gates go green, that blockage belongs to runtime workflow state outside this packet file.
- Data contract self-check: no new data-contract behavior was introduced in this turn. The packet evidence stays aligned with the active data contract by pointing to the committed Loom/Locus/storage capability surfaces already in the signed 8-file diff.
- Next step / handoff hint: rerun `just post-work WP-1-Storage-Trait-Purity-v1 --range d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b` and `just validator-handoff-check WP-1-Storage-Trait-Purity-v1 --range d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`; if both are green, return the lane to `WP_VALIDATOR`.

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
  - REQUIREMENT: "Production code no longer depends on `Database` downcasts for behavior decisions."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1598`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:19`, `src/backend/handshake_core/src/workflows.rs:1596`, `src/backend/handshake_core/src/api/loom.rs:1057`, `src/backend/handshake_core/src/storage/retention.rs:574`
  - REQUIREMENT: "Unsupported backend behavior is explicit through backend identity or capability methods, not hidden concrete-type branching."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1602`, `src/backend/handshake_core/src/storage/mod.rs:1604`, `src/backend/handshake_core/src/storage/mod.rs:1608`, `src/backend/handshake_core/src/storage/sqlite.rs:1035`, `src/backend/handshake_core/src/storage/postgres.rs:719`
  - REQUIREMENT: "Loom, Locus, retention, and workflow-side consumers keep their intended behavior without using `as_any`."
  - EVIDENCE: `src/backend/handshake_core/src/storage/locus_sqlite.rs:37`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:66`, `src/backend/handshake_core/src/storage/retention.rs:574`, `src/backend/handshake_core/src/api/loom.rs:1057`, `src/backend/handshake_core/src/workflows.rs:2825`
  - REQUIREMENT: "Dual-backend or backend-sensitive tripwire coverage remains in the storage proof surface."
  - EVIDENCE: `src/backend/handshake_core/src/api/loom.rs:1302`, `src/backend/handshake_core/src/storage/tests.rs:3062`, `src/backend/handshake_core/src/storage/tests.rs:3072`, `src/backend/handshake_core/src/storage/tests.rs:3077`, `src/backend/handshake_core/src/storage/tests.rs:3082`, `src/backend/handshake_core/src/storage/sqlite.rs:1218`, `src/backend/handshake_core/src/storage/postgres.rs:803`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `just pre-work WP-1-Storage-Trait-Purity-v1`
  - EXIT_CODE: 0
  - PROOF_LINES: `gate-check: PASS`; `pre-work-check: PASS`; `RESULT: PASS`
  - COMMAND: `git show --stat --summary --format=fuller be50f673a26aead6c2af4cc43037e84b15f3392b`
  - EXIT_CODE: 0
  - PROOF_LINES: `2 files changed, 67 insertions(+), 1 deletion(-)`; `test: align loom tier proof with flight recorder semantics`
  - COMMAND: `git diff --name-only d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/api/loom.rs src/backend/handshake_core/src/storage/locus_sqlite.rs src/backend/handshake_core/src/storage/mod.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/retention.rs src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/src/workflows.rs`
  - EXIT_CODE: 0
  - PROOF_LINES: `src/backend/handshake_core/src/api/loom.rs`; `src/backend/handshake_core/src/storage/locus_sqlite.rs`; `src/backend/handshake_core/src/storage/mod.rs`; `src/backend/handshake_core/src/storage/postgres.rs`; `src/backend/handshake_core/src/storage/retention.rs`; `src/backend/handshake_core/src/storage/sqlite.rs`; `src/backend/handshake_core/src/storage/tests.rs`; `src/backend/handshake_core/src/workflows.rs`
  - COMMAND: `git diff --numstat d26f46d586d2c44a76dd40ffaadf8603972867c4 be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src/api/loom.rs src/backend/handshake_core/src/storage/locus_sqlite.rs src/backend/handshake_core/src/storage/mod.rs src/backend/handshake_core/src/storage/postgres.rs src/backend/handshake_core/src/storage/retention.rs src/backend/handshake_core/src/storage/sqlite.rs src/backend/handshake_core/src/storage/tests.rs src/backend/handshake_core/src/workflows.rs`
  - EXIT_CODE: 0
  - PROOF_LINES: `76	27	src/backend/handshake_core/src/api/loom.rs`; `32	53	src/backend/handshake_core/src/storage/locus_sqlite.rs`; `176	4	src/backend/handshake_core/src/storage/mod.rs`; `223	2	src/backend/handshake_core/src/storage/postgres.rs`; `1	14	src/backend/handshake_core/src/storage/retention.rs`; `320	2	src/backend/handshake_core/src/storage/sqlite.rs`; `302	408	src/backend/handshake_core/src/storage/tests.rs`; `107	260	src/backend/handshake_core/src/workflows.rs`

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

### 2026-04-03T06:30:49.4824785Z | INTEGRATION_VALIDATOR PASS REPORT
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-storage-trait-purity-v1
COMMITTED_RANGE: d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b
SIGNED_PATCH_ARTIFACT: .GOV/task_packets/WP-1-Storage-Trait-Purity-v1/signed-scope.patch
REVIEW_EXCHANGE_PROOF: CODER REVIEW_REQUEST at 2026-04-03T06:13:13.323Z and INTEGRATION_VALIDATOR REVIEW_RESPONSE at 2026-04-03T06:18:35.854Z under ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Storage-Trait-Purity-v1/RECEIPTS.jsonl
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
- One Storage API [CX-DBP-010] reviewed against the committed 8-file range and confirmed through capability-based `Database` methods plus real consumer usage at `src/backend/handshake_core/src/storage/mod.rs:1596-1608`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:36-70`, `src/backend/handshake_core/src/workflows.rs:2823-2829`, and `src/backend/handshake_core/src/api/loom.rs:1050-1068`.
- Trait Purity Invariant [CX-DBP-040] reviewed against the committed range and confirmed by the trait defaults/capability surface at `src/backend/handshake_core/src/storage/mod.rs:1596-1608`, the Postgres backend capability law at `src/backend/handshake_core/src/storage/postgres.rs:717-733`, and the committed be50 tree grep showing no production `as_any` / `downcast_ref::<SqliteDatabase>` / `downcast_ref::<PostgresDatabase>` matches under `src/backend/handshake_core/src`.
- Dual-Backend Testing Early [CX-DBP-013] reviewed against the committed range and confirmed by the explicit Postgres capability-denial proof at `src/backend/handshake_core/src/storage/tests.rs:3036-3049`, the signed tripwires at `src/backend/handshake_core/src/storage/tests.rs:3061-3078`, and the API-level Loom payload proof at `src/backend/handshake_core/src/api/loom.rs:1302-1358`.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- NONE
VALIDATOR_RISK_TIER: MEDIUM
DIFF_ATTACK_SURFACES:
- Producer/consumer drift between `search_loom_blocks` emitting `tier_used` and the validator tripwire that claims to prove the Flight Recorder payload contract.
- Capability-law drift between `Database` trait defaults and backend implementations, especially whether Postgres still denies structured-collaboration artifacts and Locus runtime access explicitly.
- Consumer drift in workflow/Locus paths where a trait cleanup could silently leave one downstream path using backend-specific assumptions.
- Evidence drift between the signed diff, the packet, and the governed signed patch artifact.
INDEPENDENT_CHECKS_RUN:
- `git ls-remote --heads origin feat/WP-1-Storage-Trait-Purity-v1` => remote backup branch points at `be50f673a26aead6c2af4cc43037e84b15f3392b`.
- `git rev-parse --verify "be50f673a26aead6c2af4cc43037e84b15f3392b^{commit}"` => committed target is reachable in the Integration Validator lane.
- `Test-Path $HANDSHAKE_GOV_ROOT/task_packets/WP-1-Storage-Trait-Purity-v1/signed-scope.patch` => signed patch artifact is present in live governance state.
- `git grep -n -E "as_any|downcast_ref::<SqliteDatabase>|downcast_ref::<PostgresDatabase>" be50f673a26aead6c2af4cc43037e84b15f3392b -- src/backend/handshake_core/src` => no production trait-purity escape-hatch matches in the committed tree.
- `git show be50f673a26aead6c2af4cc43037e84b15f3392b:src/backend/handshake_core/src/api/loom.rs` and `git show be50f673a26aead6c2af4cc43037e84b15f3392b:src/backend/handshake_core/src/storage/tests.rs` => the producer sets `tier_used` from `state.storage.loom_search_observability_tier()` and the test reads the emitted payload and asserts it against the same backend-tier source.
COUNTERFACTUAL_CHECKS:
- If `src/backend/handshake_core/src/api/loom.rs:1057-1068` stopped emitting `tier_used` from `state.storage.loom_search_observability_tier()`, `src/backend/handshake_core/src/api/loom.rs:1348-1357` would no longer prove the Flight Recorder payload contract promised by this WP.
- If `src/backend/handshake_core/src/storage/postgres.rs:723-724` were flipped away from explicit capability denial without real artifact parity, `src/backend/handshake_core/src/storage/tests.rs:3036-3049` would stop defending the signed negative-path contract.
- If `src/backend/handshake_core/src/storage/locus_sqlite.rs:47-58` or `src/backend/handshake_core/src/workflows.rs:2827-2829` stopped guarding structured-collaboration access through capability checks, Postgres-backed workflow/Locus reads would silently cross the signed portability boundary.
BOUNDARY_PROBES:
- Producer/consumer boundary checked by reading the emitted Flight Recorder payload path in `src/backend/handshake_core/src/api/loom.rs:1057-1068` and the proving assertion path in `src/backend/handshake_core/src/api/loom.rs:1348-1357`.
- Trait/consumer boundary checked by reading the capability surface in `src/backend/handshake_core/src/storage/mod.rs:1596-1608` and the downstream structured-collab callers in `src/backend/handshake_core/src/storage/locus_sqlite.rs:43-59` and `src/backend/handshake_core/src/workflows.rs:2823-2829`.
NEGATIVE_PATH_CHECKS:
- Postgres negative path checked at `src/backend/handshake_core/src/storage/tests.rs:3036-3049`, where `structured_collab_work_packet_row("WP-TEST")` and `locus_work_packet_exists(...)` both assert `StorageError::NotImplemented("structured collaboration artifacts")`.
- Capability-negative path checked at `src/backend/handshake_core/src/storage/postgres.rs:719-725`, where the backend explicitly reports `supports_locus_runtime() == false` and `supports_structured_collab_artifacts() == false`.
INDEPENDENT_FINDINGS:
- The be50 repair did move the Loom proof from a storage-only fixture path to the real API/Flight Recorder path; the committed tree now proves the emitted `tier_used` payload rather than only a backend helper.
- The signed patch artifact is present in live governance state, so the packet, receipts, and signed-scope artifact are aligned on the same committed range.
- Local `main` still does not contain `be50f67`, which is correct for a `MERGE_PENDING` PASS stage and not a product-scope regression.
RESIDUAL_UNCERTAINTY:
- This PASS report is diff-scoped to `d26f46d586d2c44a76dd40ffaadf8603972867c4..be50f673a26aead6c2af4cc43037e84b15f3392b`; it does not by itself prove contained-main promotion or post-merge spot checks.
- Merge-surface governance state can still block promotion after PASS is recorded, including current-main compatibility recording, Gate 2 progression, active broker runs, or unrelated dirt on local `main`.
SPEC_CLAUSE_MAP:
- One Storage API [CX-DBP-010] => `src/backend/handshake_core/src/storage/mod.rs:1596-1608`, `src/backend/handshake_core/src/storage/locus_sqlite.rs:36-70`, `src/backend/handshake_core/src/workflows.rs:2823-2829`, `src/backend/handshake_core/src/api/loom.rs:1050-1068`.
- Trait Purity Invariant [CX-DBP-040] => `src/backend/handshake_core/src/storage/mod.rs:1596-1608`, `src/backend/handshake_core/src/storage/postgres.rs:717-733`, `src/backend/handshake_core/src/storage/tests.rs:3061-3078`.
- Dual-Backend Testing Early [CX-DBP-013] => `src/backend/handshake_core/src/storage/tests.rs:3036-3078`, `src/backend/handshake_core/src/api/loom.rs:1302-1358`, `src/backend/handshake_core/src/storage/sqlite.rs:1217-1225`.
NEGATIVE_PROOF:
- True Postgres structured-collaboration artifact parity is still not implemented. The validator confirmed that this WP intentionally preserves explicit capability denial instead of adding parity, and the signed contract proves that denial path honestly at `src/backend/handshake_core/src/storage/postgres.rs:723-724` and `src/backend/handshake_core/src/storage/tests.rs:3036-3049`.
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
DATA_CONTRACT_PROOF:
- Emitted Loom/Flight Recorder payload contract reviewed at `src/backend/handshake_core/src/api/loom.rs:1057-1068` and proved by the API-level test at `src/backend/handshake_core/src/api/loom.rs:1348-1357`.
- Structured-collaboration storage/query contract reviewed through the capability-denial backend surface at `src/backend/handshake_core/src/storage/postgres.rs:723-724` and the negative-path proof at `src/backend/handshake_core/src/storage/tests.rs:3036-3049`.
- Signed scope artifact reviewed at `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/signed-scope.patch` and matched to the committed 8-file range.
DATA_CONTRACT_GAPS:
- NONE
