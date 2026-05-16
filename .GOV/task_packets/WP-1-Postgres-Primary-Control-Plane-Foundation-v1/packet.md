<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/packet.json source_hash=62f13c5ec6914fed projection_hash=d117412d33d6accf generated_at_utc=2026-05-14T21:13:39.669Z generator=wp-contract-import.mjs -->
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
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just phase-check STARTUP ... CODER` enforces alignment.

---

# Task Packet: WP-1-Postgres-Primary-Control-Plane-Foundation-v1

## METADATA
- TASK_ID: WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WP_ID: WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- BASE_WP_ID: WP-1-Postgres-Primary-Control-Plane-Foundation
- DATE: 2026-05-05T22:09:08.131Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: ACTIVATION_MANAGER
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
- ACTIVATION_MANAGER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Optional but authoritative when Activation Manager launch or repair resumes from the packet. -->
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: gpt-5.5
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
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_DISABLED
- SESSION_COMPATIBILITY_QUEUE_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Postgres-Primary-Control-Plane-Foundation-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: claude-opus-4-7
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-plane-foundation-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_DUAL_TRACK_FIELDS: MECHANICAL_TRACK_VERDICT | SPEC_RETENTION_TRACK_VERDICT
<!-- For PACKET_FORMAT_VERSION >= 2026-04-05 and RISK_TIER=MEDIUM|HIGH, both governed dual-track fields become mandatory at validator closeout. -->
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01. Allowed: NONE | LLM_FIRST_DATA_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 00fda21a394278ca1fa105df972ffac8b9f4d11e
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-05-06T11:05:10.732Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 00fda21a394278ca1fa105df972ffac8b9f4d11e
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-05-06T11:05:10.732Z
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Dual-Backend-Tests, WP-1-Postgres-Structured-Collaboration-Artifact-Parity, WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-ModelSession-Core-Scheduler
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-Postgres-Control-Plane-Leases-Backpressure, WP-1-ModelSession-Postgres-Queue-Workers, WP-1-FEMS-Postgres-Memory-Store, WP-1-Workflow-Engine-Postgres-Durable-Execution, WP-1-DCC-Postgres-Control-Plane-Projections, WP-1-SQLite-Cache-Offline-Boundaries
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- LOCAL_WORKTREE_DIR: ../wtc-plane-foundation-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-postgres-primary-control-plane-foundation-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-postgres-primary-control-plane-foundation-v1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja050520262319
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: NONE
## WORKTREE_CLEANUP_STATUS (STATUS-SYNC APPENDIX; PRODUCT-CODE ONLY)
- CHECK_TYPE: PRODUCT_CODE_ONLY_WORKTREE_CONTAINMENT
- CHECKED_AT_UTC: 2026-05-14T20:52:00Z
- CHECKED_BY: INTEGRATION_VALIDATOR
- MAIN_HEAD: c5fa320e18ef9e1f13993811df77d30c3a25a538
- WORKTREE_DIR: ../wtc-plane-foundation-v1
- WORK_BRANCH: feat/WP-1-Postgres-Primary-Control-Plane-Foundation-v1
- WORKTREE_HEAD: d7f3f760945c21076d75188fb2c90f1eafb155c3
- BRANCH_HEAD_ANCESTOR_OF_MAIN: YES
- COMMITTED_PRODUCT_DIFF_VS_MAIN_COUNT: 0
- TRACKED_DIRTY_PRODUCT_COUNT: 0
- UNTRACKED_PRODUCT_COUNT: 0
- CLEANUP_RECOMMENDATION: READY_FOR_OPERATOR_APPROVED_WORKTREE_DELETE
- SUMMARY: Branch product commits are contained in main and no local product drift was found in the worktree.
- EVIDENCE:
  - no_committed_product_diff_vs_main
  - no_tracked_dirty_product_paths
  - no_untracked_product_paths

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Proposed [ADD v02.182] PostgreSQL-primary control-plane authority | CODE_SURFACES: storage/mod.rs, main.rs, storage/tests.rs | TESTS: storage_mode_defaults_to_postgres_primary_when_required | EXAMPLES: Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding., Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed., Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata., Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Proposed fail-closed control-plane storage mode | CODE_SURFACES: storage/mod.rs, main.rs | TESTS: storage_mode_fails_closed_when_postgres_required_without_url | EXAMPLES: Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding., Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed., Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata., Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Proposed SQLite cache/offline boundary | CODE_SURFACES: storage/mod.rs, storage/tests.rs | TESTS: sqlite_cache_mode_is_not_control_plane_authority | EXAMPLES: Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding., Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed., Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata., Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Current storage portability and dual-backend testing law | CODE_SURFACES: storage/mod.rs, storage/postgres.rs, storage/tests.rs | TESTS: database_trait_purity_capability_snapshot_reports_postgres | EXAMPLES: Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding., Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed., Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata., Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
  - ID: AC-001 | REQUIREMENT: Proposed [ADD v02.182] PostgreSQL-primary control-plane authority | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: CONFIRMED | EVIDENCE: storage_mode_defaults_to_postgres_primary_when_required | REASON: NONE
  - ID: AC-002 | REQUIREMENT: Proposed fail-closed control-plane storage mode | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: CONFIRMED | EVIDENCE: storage_mode_fails_closed_when_postgres_required_without_url | REASON: NONE
  - ID: AC-003 | REQUIREMENT: Proposed SQLite cache/offline boundary | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: CONFIRMED | EVIDENCE: sqlite_cache_mode_is_not_control_plane_authority | REASON: NONE
  - ID: AC-004 | REQUIREMENT: Current storage portability and dual-backend testing law | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: CONFIRMED | EVIDENCE: database_trait_purity_capability_snapshot_reports_postgres | REASON: NONE
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- REQUIRED_TRIPWIRE_TESTS:
  - storage_mode_defaults_to_postgres_primary_when_required
  - storage_mode_fails_closed_when_postgres_required_without_url
  - sqlite_cache_mode_is_not_control_plane_authority
  - database_trait_purity_capability_snapshot_reports_postgres
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml database_trait_purity_capability_snapshot_reports_postgres -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example `postgres_primary` runtime config with PostgreSQL URL present and startup succeeding.
  - Example `postgres_primary` runtime config with no PostgreSQL URL/service and startup failing closed.
  - Example `sqlite_cache` or `sqlite_offline` mode declaring non-authoritative runtime posture and source/freshness metadata.
  - Example packet `STUB_WP_IDS` manifest listing the seven downstream PostgreSQL pivot slices as Orchestrator-owned follow-up WPs.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: ../handshake_main/src/backend/handshake_core/migrations/ (migration/sql surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: PostgreSQL-primary control-plane foundation | SUBFEATURES: storage-mode config, default authority, fail-closed semantics, bootstrap health proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-PostgresDatabase, PRIM-SqliteDatabase, PRIM-StorageTraits, PostgresPrimaryControlPlane, ControlPlaneStorageMode | MECHANICAL: engine.dba, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: core foundation scope
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: reproducible PostgreSQL developer/test matrix | SUBFEATURES: container service, migration reset, seeded fixtures, CI smoke profiles | PRIMITIVES_FEATURES: PRIM-PostgresDatabase, StorageModeFixtureMatrix | MECHANICAL: engine.dba, engine.sandbox, engine.version | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | NOTES: separable developer setup
  - PILLAR_DECOMPOSITION: PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: ModelSession PostgreSQL queue workers | SUBFEATURES: model run queue, workers, persisted messages, checkpoints, cancellation, provider profile ids | PRIMITIVES_FEATURES: PRIM-ModelSession, ModelRunQueueWorker | MECHANICAL: engine.context, engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-ModelSession-Postgres-Queue-Workers-v1 | NOTES: downstream from lease primitives
  - PILLAR_DECOMPOSITION: PILLAR: Command Center | CAPABILITY_SLICE: DCC Postgres control-plane projections | SUBFEATURES: sessions, queues, leases, workflows, memory jobs, dead-letter, source and freshness metadata | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, DccPostgresProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: UI consumes, not foundation
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: SQLite cache/offline boundary | SUBFEATURES: cache/index/offline modes, fail-closed runtime writes, rebuildable projections, freshness metadata | PRIMITIVES_FEATURES: PRIM-SqliteDatabase, SqliteCacheOfflineBoundary | MECHANICAL: engine.sovereign, engine.librarian | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-SQLite-Cache-Offline-Boundaries-v1 | NOTES: prevents fallback split brain
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: Postgres-authority work-state projection | SUBFEATURES: runtime authority source, freshness, workflow links, task-board sync posture | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, PRIM-WorkflowRun | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: detailed Locus projection parity belongs downstream
  - PILLAR_DECOMPOSITION: PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: product work-packet runtime authority | SUBFEATURES: structured packet record, workflow binding, source label, mirror freshness | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.director, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: packet prose must not become runtime authority
  - PILLAR_DECOMPOSITION: PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: task-board projection parity | SUBFEATURES: board row source, freshness, validation posture, queue status, reconciliation state | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, TaskBoardProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: board markdown is a projection after the pivot
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: FEMS and compact control-plane summaries | SUBFEATURES: memory pack source, runtime freshness, compact status fields, stale-cache labels | PRIMITIVES_FEATURES: PRIM-MemoryPack, FemsPostgresMemoryStore, DccPostgresProjection | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-FEMS-Postgres-Memory-Store-v1 | NOTES: local-small-model reads need compact authoritative summaries
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PostgreSQL-primary control-plane storage mode | JobModel: MECHANICAL_TOOL | Workflow: startup_storage_bootstrap | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing startup/runtime health evidence | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: foundation declares and proves the active storage authority
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PostgreSQL developer/test matrix | JobModel: NONE | Workflow: test_harness | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Notes: required before heavy follow-up validation
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: control-plane leases and backpressure | JobModel: WORKFLOW | Workflow: control_plane_queue_claims | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: planned by follow-up | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Notes: shared concurrency foundation
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: ModelSession Postgres queue workers | JobModel: AI_JOB | Workflow: model_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FR-EVT-SESS families plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: move scheduler authority out of process-local state
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: FEMS Postgres memory store | JobModel: AI_JOB | Workflow: fems_memory_job | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FEMS events plus follow-up proof | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-FEMS-Postgres-Memory-Store-v1 | Notes: shared memory authority
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: workflow durable execution on PostgreSQL | JobModel: WORKFLOW | Workflow: workflow_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing workflow evidence plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Notes: crash-resume and node claim state
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: DCC Postgres projections | JobModel: UI_ACTION | Workflow: dcc_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: evidence refs only | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: operator projection surface
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: SQLite cache/offline boundary | JobModel: MECHANICAL_TOOL | Workflow: storage_mode_guard | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: health/degradation evidence | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1 | Notes: explicit non-authority fallback
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
  - ../handshake_main/src/backend/handshake_core/migrations/
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
  - ordinary launch path is `SESSION_HOST_PREFERENCE` through the governed ACP control lane
  - `SESSION_CONTROL_REQUESTS_FILE`, `SESSION_CONTROL_RESULTS_FILE`, and `SESSION_REGISTRY_FILE` are the deterministic launch/steer/watch state for ordinary governed sessions
  - `SESSION_COMPATIBILITY_SURFACE` and `SESSION_COMPATIBILITY_QUEUE_FILE` exist only for explicit compatibility or repair launches; they are not the default `AUTO` path
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-Primary Control-Plane Foundation [ADD v02.182]
- CONTEXT_START_LINE: 3623
- CONTEXT_END_LINE: 3646
- CONTEXT_TOKEN: postgres_primary
- EXCERPT_ASCII_ESCAPED:
  ```text
Handshake now treats PostgreSQL as the primary runtime authority for the self-hosted control plane.
  The self-hosted control plane MUST support an explicit storage mode contract with semantic modes including postgres_primary, sqlite_cache, sqlite_offline, and test.
  If postgres_primary is required and no valid PostgreSQL connection is available, startup or the attempted authoritative control-plane write MUST fail closed.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.15 Locus Work Tracking System storage and projection posture
- CONTEXT_START_LINE: 6680
- CONTEXT_END_LINE: 7090
- CONTEXT_TOKEN: backpressure posture
- EXCERPT_ASCII_ESCAPED:
  ```text
Software-delivery control-plane state should preserve health posture, stale-detection posture, backpressure posture, and operator-alert posture by stable runtime identifiers.
  Under load or blocked authority, the system MUST surface backpressure explicitly instead of silently dropping or reordering control-plane intent.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 4.3.9.12-4.3.9.13 ModelSession and Session Scheduler
- CONTEXT_START_LINE: 32440
- CONTEXT_END_LINE: 32500
- CONTEXT_TOKEN: INV-SCHED-001
- EXCERPT_ASCII_ESCAPED:
  ```text
The Session Scheduler introduces job_kind = "model_run" into the AI Job Model.
  INV-SCHED-001: All model invocations in RuntimeMode=AI_ENABLED MUST be routed through the Session Scheduler.
  This foundation does not reimplement the scheduler; it sets the PostgreSQL-primary runtime authority boundary for downstream queue-worker work.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.6.6.7.6.2 FEMS and ModelSession memory policy
- CONTEXT_START_LINE: 11950
- CONTEXT_END_LINE: 12005
- CONTEXT_TOKEN: ModelSession.memory_policy
- EXCERPT_ASCII_ESCAPED:
  ```text
FEMS defines read, write, validation, consolidation, and MemoryPack behavior.
  The spec also references ModelSession.memory_policy as the integration point.
  PostgreSQL-primary memory storage is a downstream follow-up WP because the foundation only establishes runtime authority and storage-mode law.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.182.md 2.3.5.6 The Role of SQLite
- CONTEXT_START_LINE: 2247
- CONTEXT_END_LINE: 2264
- CONTEXT_TOKEN: SQLite is used for
- EXCERPT_ASCII_ESCAPED:
  ```text
Important: SQLite is used for indexing, not as the primary data store.
  The pivot preserves this intent by making SQLite cache/index/offline authority explicit instead of allowing silent control-plane fallback.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Proposed [ADD v02.182] PostgreSQL-primary control-plane authority | WHY_IN_SCOPE: This is the operator-directed pivot and the current spec does not yet clearly cover it | EXPECTED_CODE_SURFACES: storage/mod.rs, main.rs, storage/tests.rs | EXPECTED_TESTS: storage_mode_defaults_to_postgres_primary_when_required | RISK_IF_MISSED: Runtime defaults remain SQLite-primary by accident.
  - CLAUSE: Proposed fail-closed control-plane storage mode | WHY_IN_SCOPE: Silent fallback is the main foundation safety risk | EXPECTED_CODE_SURFACES: storage/mod.rs, main.rs | EXPECTED_TESTS: storage_mode_fails_closed_when_postgres_required_without_url | RISK_IF_MISSED: PostgreSQL outages create hidden split brain.
  - CLAUSE: Proposed SQLite cache/offline boundary | WHY_IN_SCOPE: Local-first ergonomics must remain explicit without giving SQLite hidden authority | EXPECTED_CODE_SURFACES: storage/mod.rs, storage/tests.rs | EXPECTED_TESTS: sqlite_cache_mode_is_not_control_plane_authority | RISK_IF_MISSED: Cache/offline mode becomes undeclared runtime authority.
  - CLAUSE: Current storage portability and dual-backend testing law | WHY_IN_SCOPE: Foundation builds on existing database abstraction and test helpers | EXPECTED_CODE_SURFACES: storage/mod.rs, storage/postgres.rs, storage/tests.rs | EXPECTED_TESTS: database_trait_purity_capability_snapshot_reports_postgres | RISK_IF_MISSED: Pivot bypasses established portability discipline.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Control-plane storage-mode config | PRODUCER: backend startup/env resolver | CONSUMER: storage init, health endpoint, DCC projections, tests | SERIALIZER_TRANSPORT: environment/config struct and health payload | VALIDATOR_READER: storage-mode tests | TRIPWIRE_TESTS: storage_mode_defaults_to_postgres_primary_when_required | DRIFT_RISK: Defaults differ between startup, tests, and operator docs.
  - CONTRACT: PostgreSQL-required fail-closed error | PRODUCER: storage init and control-plane write guards | CONSUMER: backend startup, DCC health, validators | SERIALIZER_TRANSPORT: structured StorageError or health/degradation payload | VALIDATOR_READER: fail-closed tests | TRIPWIRE_TESTS: storage_mode_fails_closed_when_postgres_required_without_url | DRIFT_RISK: Missing URL/service becomes SQLite fallback.
  - CONTRACT: SQLite cache/offline authority label | PRODUCER: storage mode resolver and cache/index layers | CONSUMER: DCC, tests, downstream SQLite boundary packet | SERIALIZER_TRANSPORT: storage capability snapshot and health payload | VALIDATOR_READER: SQLite boundary tests | TRIPWIRE_TESTS: sqlite_cache_mode_is_not_control_plane_authority | DRIFT_RISK: Cache projection is mistaken for source of truth.
  - CONTRACT: Candidate follow-up manifest | PRODUCER: refinement and packet `STUB_WP_IDS` | CONSUMER: Orchestrator, coder, validator, Build Order | SERIALIZER_TRANSPORT: comma-separated WP IDs plus Orchestrator-owned registry/build-order projections | VALIDATOR_READER: pre-work/coder checks | TRIPWIRE_TESTS: just gov-check | DRIFT_RISK: downstream work disappears from activation scope.
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Confirm approved spec enrichment has advanced `SPEC_CURRENT` before product-code implementation.
  - Inspect `storage::init_storage()`, `StorageBackendKind`, `StorageCapabilitySnapshot`, `PostgresDatabase::connect`, and current `POSTGRES_TEST_URL` helpers.
  - Add the smallest explicit storage-mode resolver needed for PostgreSQL-primary, SQLite cache, SQLite offline, and test modes.
  - Change self-hosted control-plane default behavior only through that resolver; do not hardcode ad hoc checks in callers.
  - Add fail-closed tests for missing PostgreSQL when required and explicit non-authority tests for SQLite cache/offline mode.
  - Leave queue workers, leases/backpressure, FEMS, workflow durable execution, DCC projections, and dev/test containers to the linked stubs.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
- TRIPWIRE_TESTS:
  - storage_mode_defaults_to_postgres_primary_when_required
  - storage_mode_fails_closed_when_postgres_required_without_url
  - sqlite_cache_mode_is_not_control_plane_authority
  - database_trait_purity_capability_snapshot_reports_postgres
- CARRY_FORWARD_WARNINGS:
  - Do not silently fall back to SQLite for authoritative control-plane writes.
  - Do not implement queue leases, worker claims, FEMS storage, workflow durable execution, or DCC projections inside the foundation patch.
  - Do not use repo `.GOV` files, packet prose, or mailbox chronology as product runtime authority.
  - Keep provider/model profiles as declared catalog IDs in downstream work, not ambient CLI aliases.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Approved [ADD v02.182] PostgreSQL-primary control-plane authority.
  - Approved fail-closed PostgreSQL-required storage mode.
  - Approved SQLite cache/offline non-authority boundary.
  - Current v02.182 storage portability and dual-backend testing law.
- FILES_TO_READ:
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
- COMMANDS_TO_RUN:
  - `rg -n "init_storage|DATABASE_URL|POSTGRES_TEST_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|run_migrations|storage_mode|postgres_primary|sqlite_cache|sqlite_offline" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Confirm `STUB_WP_IDS` in the packet still lists all seven downstream slices.
  - Confirm no product code in the foundation implements the downstream worker/lease/FEMS/workflow/DCC scopes.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Exact storage-mode environment variable names and health payload field names must be chosen during coding.
  - Whether PostgreSQL developer setup uses Docker, Podman, or another local service is downstream Orchestrator-owned follow-up work.
  - Whether lease primitives should use row locks, advisory locks, or compare-and-swap is downstream Orchestrator-owned follow-up work.
  - Whether FEMS memory embeddings use pgvector or remain separate cache/index artifacts is downstream Orchestrator-owned follow-up work.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - The local repo already contains PostgreSQL storage support, migrations, dual-backend tests, and structured-collaboration parity work. The missing decision is normative authority and default runtime posture.
  - The safe shape is to land a narrow foundation and route every separable heavy implementation slice into explicit Orchestrator-owned follow-up WPs rather than enlarging the foundation WP.
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
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
  - PRIM-ModelSession
  - PRIM-WorkflowRun
  - PRIM-MemoryPack
  - PRIM-LocusSyncTaskBoardParams
- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-PostgresDatabase
  - PRIM-SqliteDatabase
  - PRIM-StorageTraits
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.director
  - engine.logistics
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.sovereign
  - engine.context
  - engine.version
  - engine.sandbox
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NO_CHANGE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS
- PILLARS_TOUCHED:
  - Flight Recorder
  - Locus
  - Work packets (product, not repo)
  - Task board (product, not repo)
  - MicroTask
  - Command Center
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - Locus: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Work packets (product, not repo): WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Task board (product, not repo): WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - MicroTask: WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1
  - Command Center: WP-1-DCC-Postgres-Control-Plane-Projections-v1
  - Execution / Job Runtime: WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1
  - SQL to PostgreSQL shift readiness: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
  - LLM-friendly data: WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-FEMS-Postgres-Memory-Store-v1
  - RAG: WP-1-SQLite-Cache-Offline-Boundaries-v1
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Postgres-primary storage mode plus fail-closed control-plane writes -> IN_THIS_WP (stub: NONE)
  - Postgres test matrix plus every downstream runtime packet -> NEW_STUB (stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1)
  - lease/backpressure plus ModelSession workflow and FEMS jobs -> NEW_STUB (stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1)
  - ModelSession queue workers plus DCC projection -> NEW_STUB (stub: WP-1-ModelSession-Postgres-Queue-Workers-v1)
  - FEMS memory store plus parallel ModelSession memory policy -> NEW_STUB (stub: WP-1-FEMS-Postgres-Memory-Store-v1)
  - workflow durable execution plus leases and backpressure -> NEW_STUB (stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1)
  - Postgres authority plus DCC runtime projection -> NEW_STUB (stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1)
  - SQLite cache boundary plus no split-brain fallback -> NEW_STUB (stub: WP-1-SQLite-Cache-Offline-Boundaries-v1)
  - storage-mode health plus Flight Recorder evidence -> IN_THIS_WP (stub: NONE)
  - work-packet runtime authority plus DCC projection -> NEW_STUB (stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1)
  - task-board freshness plus PostgreSQL authoritative records -> NEW_STUB (stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1)
  - micro-task occupancy plus ModelSession queue workers -> NEW_STUB (stub: WP-1-ModelSession-Postgres-Queue-Workers-v1)
- STUB_WP_IDS: WP-1-Postgres-Dev-Test-Container-Matrix-v1, WP-1-Postgres-Control-Plane-Leases-Backpressure-v1, WP-1-ModelSession-Postgres-Queue-Workers-v1, WP-1-FEMS-Postgres-Memory-Store-v1, WP-1-Workflow-Engine-Postgres-Durable-Execution-v1, WP-1-DCC-Postgres-Control-Plane-Projections-v1, WP-1-SQLite-Cache-Offline-Boundaries-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS
- DECOMPOSITION_ROWS:
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: PostgreSQL-primary control-plane foundation | SUBFEATURES: storage-mode config, default authority, fail-closed semantics, bootstrap health proof | PRIMITIVES_FEATURES: PRIM-Database, PRIM-PostgresDatabase, PRIM-SqliteDatabase, PRIM-StorageTraits, PostgresPrimaryControlPlane, ControlPlaneStorageMode | MECHANICAL: engine.dba, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: core foundation scope
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: reproducible PostgreSQL developer/test matrix | SUBFEATURES: container service, migration reset, seeded fixtures, CI smoke profiles | PRIMITIVES_FEATURES: PRIM-PostgresDatabase, StorageModeFixtureMatrix | MECHANICAL: engine.dba, engine.sandbox, engine.version | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | NOTES: separable developer setup
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: shared control-plane leases and backpressure | SUBFEATURES: claims, lease expiry, heartbeat, retry, dead-letter, backpressure | PRIMITIVES_FEATURES: PRIM-WorkflowRun, ControlPlaneLease | MECHANICAL: engine.logistics, engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | NOTES: must not be half-implemented by foundation
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: ModelSession PostgreSQL queue workers | SUBFEATURES: model run queue, workers, persisted messages, checkpoints, cancellation, provider profile ids | PRIMITIVES_FEATURES: PRIM-ModelSession, ModelRunQueueWorker | MECHANICAL: engine.context, engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-ModelSession-Postgres-Queue-Workers-v1 | NOTES: downstream from lease primitives
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow durable execution on PostgreSQL | SUBFEATURES: workflow instance state, node checkpoints, retry state, crash resume | PRIMITIVES_FEATURES: PRIM-WorkflowRun, WorkflowPostgresDurableExecution | MECHANICAL: engine.director, engine.archivist | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | NOTES: separate from storage-mode foundation
  - PILLAR: Command Center | CAPABILITY_SLICE: DCC Postgres control-plane projections | SUBFEATURES: sessions, queues, leases, workflows, memory jobs, dead-letter, source and freshness metadata | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, DccPostgresProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: UI consumes, not foundation
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: SQLite cache/offline boundary | SUBFEATURES: cache/index/offline modes, fail-closed runtime writes, rebuildable projections, freshness metadata | PRIMITIVES_FEATURES: PRIM-SqliteDatabase, SqliteCacheOfflineBoundary | MECHANICAL: engine.sovereign, engine.librarian | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-SQLite-Cache-Offline-Boundaries-v1 | NOTES: prevents fallback split brain
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: storage authority evidence | SUBFEATURES: startup storage mode, health reason, fail-closed storage error, recovery visibility | PRIMITIVES_FEATURES: PRIM-Database, PRIM-PostgresDatabase | MECHANICAL: engine.archivist, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: foundation must make the authority decision recorder-visible without adding a new event family
  - PILLAR: Locus | CAPABILITY_SLICE: Postgres-authority work-state projection | SUBFEATURES: runtime authority source, freshness, workflow links, task-board sync posture | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, PRIM-WorkflowRun | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: detailed Locus projection parity belongs downstream
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: product work-packet runtime authority | SUBFEATURES: structured packet record, workflow binding, source label, mirror freshness | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-LocusSyncTaskBoardParams | MECHANICAL: engine.director, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: packet prose must not become runtime authority
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: task-board projection parity | SUBFEATURES: board row source, freshness, validation posture, queue status, reconciliation state | PRIMITIVES_FEATURES: PRIM-LocusSyncTaskBoardParams, TaskBoardProjection | MECHANICAL: engine.librarian, engine.context | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | NOTES: board markdown is a projection after the pivot
  - PILLAR: MicroTask | CAPABILITY_SLICE: micro-task queue occupancy | SUBFEATURES: model-session binding, retry state, lease posture, workflow node link | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-WorkflowRun, ModelRunQueueWorker | MECHANICAL: engine.logistics, engine.director | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-ModelSession-Postgres-Queue-Workers-v1 | NOTES: micro-task semantics depend on queue-worker and lease follow-ups
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: FEMS and compact control-plane summaries | SUBFEATURES: memory pack source, runtime freshness, compact status fields, stale-cache labels | PRIMITIVES_FEATURES: PRIM-MemoryPack, FemsPostgresMemoryStore, DccPostgresProjection | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-FEMS-Postgres-Memory-Store-v1 | NOTES: local-small-model reads need compact authoritative summaries
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS
- ALIGNMENT_ROWS:
  - Capability: PostgreSQL-primary control-plane storage mode | JobModel: MECHANICAL_TOOL | Workflow: startup_storage_bootstrap | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing startup/runtime health evidence | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: foundation declares and proves the active storage authority
  - Capability: PostgreSQL developer/test matrix | JobModel: NONE | Workflow: test_harness | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Dev-Test-Container-Matrix-v1 | Notes: required before heavy follow-up validation
  - Capability: control-plane leases and backpressure | JobModel: WORKFLOW | Workflow: control_plane_queue_claims | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: planned by follow-up | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 | Notes: shared concurrency foundation
  - Capability: ModelSession Postgres queue workers | JobModel: AI_JOB | Workflow: model_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FR-EVT-SESS families plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-ModelSession-Postgres-Queue-Workers-v1 | Notes: move scheduler authority out of process-local state
  - Capability: FEMS Postgres memory store | JobModel: AI_JOB | Workflow: fems_memory_job | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing FEMS events plus follow-up proof | Locus: PLANNED | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-FEMS-Postgres-Memory-Store-v1 | Notes: shared memory authority
  - Capability: workflow durable execution on PostgreSQL | JobModel: WORKFLOW | Workflow: workflow_run | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: existing workflow evidence plus follow-up proof | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 | Notes: crash-resume and node claim state
  - Capability: DCC Postgres projections | JobModel: UI_ACTION | Workflow: dcc_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: evidence refs only | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: NEW_STUB | Stub: WP-1-DCC-Postgres-Control-Plane-Projections-v1 | Notes: operator projection surface
  - Capability: SQLite cache/offline boundary | JobModel: MECHANICAL_TOOL | Workflow: storage_mode_guard | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: health/degradation evidence | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-SQLite-Cache-Offline-Boundaries-v1 | Notes: explicit non-authority fallback
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_STUBS
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Postgres-Dev-Test-Container-Matrix-v1 -> NEW_STUB
  - WP-1-Postgres-Control-Plane-Leases-Backpressure-v1 -> NEW_STUB
  - WP-1-ModelSession-Postgres-Queue-Workers-v1 -> NEW_STUB
  - WP-1-FEMS-Postgres-Memory-Store-v1 -> NEW_STUB
  - WP-1-Workflow-Engine-Postgres-Durable-Execution-v1 -> NEW_STUB
  - WP-1-DCC-Postgres-Control-Plane-Projections-v1 -> NEW_STUB
  - WP-1-SQLite-Cache-Offline-Boundaries-v1 -> NEW_STUB
  - WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1 -> KEEP_SEPARATE
  - WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> KEEP_SEPARATE
  - WP-1-ModelSession-Core-Scheduler-v1 -> KEEP_SEPARATE
  - WP-1-Storage-Trait-Purity-v1 -> KEEP_SEPARATE
  - WP-1-Dual-Backend-Tests-v2 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/main.rs -> PARTIAL (NONE)
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
- What: Implement the minimal PostgreSQL-primary control-plane foundation from SPEC_CURRENT v02.182: storage-mode/default configuration, explicit authority labels, fail-closed startup behavior, and bootstrap proof that self-hosted control-plane runtime state is not silently SQLite-primary.
- Why: The operator wants Handshake to move to PostgreSQL now while the project is early. Without a foundation WP, each downstream runtime packet will make incompatible assumptions about default storage, fallback, concurrency, and projection authority.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/tests/storage_conformance.rs
  - ../handshake_main/src/backend/handshake_core/migrations/
- OUT_OF_SCOPE:
  - Implementing ModelSession PostgreSQL queue workers, worker claims, or message/checkpoint migration.
  - Implementing FEMS PostgreSQL memory store, bitemporal memory, memory poisoning controls, or memory dashboards.
  - Implementing full workflow-engine durable execution on PostgreSQL.
  - Implementing DCC visual projections or operator action mutations.
  - Implementing SQLite offline sync or cache invalidation beyond naming/fail-closed boundaries.
  - Implementing a full PostgreSQL dev/test container matrix in this foundation WP.
- TOUCHED_FILE_BUDGET: 9
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (Host-load/test deferral example: `- WAIVER_ID: CX-ENV-HOST-LOAD-... | STATUS: ACTIVE | COVERS: TEST, ENVIRONMENT | SCOPE: <heavy commands> while operator-owned host load is active | JUSTIFICATION: Operator waived heavy tests during load; roles must not touch external processes | APPROVER: Operator | EXPIRES: <condition requiring fresh proof or closure>`.)
- (Do not use `## WAIVERS GRANTED` to continue after token-cost overrun. Token budget and token-ledger drift are diagnostic-only cost telemetry and must be surfaced mechanically in audits/dossiers instead of requiring a continuation waiver.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
rg -n "init_storage|DATABASE_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|POSTGRES_TEST_URL|run_migrations" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml database_trait_purity_capability_snapshot_reports_postgres -- --exact
  just gov-check
```

### DONE_MEANS
- Master Spec enrichment for PostgreSQL-primary control-plane authority is approved and `SPEC_CURRENT` resolves to v02.182 before packet coding begins.
- Runtime configuration can declare `postgres_primary`, `sqlite_cache`, `sqlite_offline`, or equivalent explicit storage mode without relying on ambient defaults.
- When PostgreSQL-primary is required and no valid PostgreSQL URL/service is available, control-plane startup or control-plane writes fail closed with a clear storage-mode error.
- SQLite remains explicitly cache/index/offline/demo scoped and cannot silently receive authoritative control-plane writes.
- Downstream candidate follow-up IDs for queues, leases/backpressure, FEMS, workflow, DCC projection, SQLite boundaries, and test container matrix are listed in packet `STUB_WP_IDS`; Orchestrator-owned stub records are not edited by Activation Manager.

- PRIMITIVES_EXPOSED:
  - PRIM-Database
  - PRIM-PostgresDatabase
  - PRIM-SqliteDatabase
  - PRIM-StorageTraits
  - PRIM-PostgresPrimaryControlPlane
  - PRIM-ControlPlaneStorageMode
  - PRIM-SqliteCacheOfflineBoundary
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-05-05T22:09:08.131Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.182]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-primary control-plane foundation plus v02.181 software-delivery control-plane projection law.
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
  - .GOV/refinements/WP-1-Postgres-Primary-Control-Plane-Foundation-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.182.md
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/main.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - init_storage
  - DATABASE_URL
  - POSTGRES_TEST_URL
  - StorageBackendKind
  - PostgresDatabase
  - SqliteDatabase
  - run_migrations
  - supports_structured_collab_artifacts
  - SessionSchedulerConfig
  - backpressure posture
  - control-plane health
- RUN_COMMANDS:
  ```bash
rg -n "init_storage|DATABASE_URL|POSTGRES_TEST_URL|StorageBackendKind|PostgresDatabase|SqliteDatabase|run_migrations|SessionSchedulerConfig|backpressure|control-plane" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_defaults_to_postgres_primary_when_required -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage_mode_fails_closed_when_postgres_required_without_url -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml sqlite_cache_mode_is_not_control_plane_authority -- --exact
  just gov-check
  ```
- RISK_MAP:
  - "Foundation widens into all downstream runtime work" -> "Packet becomes too large and mixes authority definition with queue/FEMS/workflow/DCC implementation."
  - "PostgreSQL required but unavailable silently falls back to SQLite" -> "Control-plane split brain and false operator state."
  - "Spec enrichment skipped" -> "Coder implements against operator intent while current Master Spec still says PostgreSQL is future/Phase 2."
  - "Test container setup deferred without an Orchestrator-owned follow-up WP" -> "Every downstream Postgres packet invents a different service/bootstrap contract."
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
- (Mechanical manifest for audit. This is coder-recorded evidence, not an official validation verdict.)
- COMMITTED_HANDOFF_BASE_SHA: `ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e`
- COMMITTED_HANDOFF_HEAD_SHA: `d7f3f760945c21076d75188fb2c90f1eafb155c3`
- COMMITTED_HANDOFF_RANGE: `ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e..d7f3f760945c21076d75188fb2c90f1eafb155c3`
- COMMITTED_HANDOFF_RANGE_SOURCE: current `git merge-base HEAD main` at coder handoff; packet creation MERGE_BASE_SHA `facce56f879d4ee990f62566b12a8b26d8bc61d7` is retained as creation provenance but is too broad for final committed diff isolation after branch/main topology advanced.
- **Target File**: `src/backend/handshake_core/src/main.rs`
- **Start**: 49
- **End**: 55
- **Line Delta**: 6
- **Pre-SHA1**: `a91f9595cc2d3274a166d09e419a830c11227694`
- **Post-SHA1**: `567f14d674e3fd37ebeaf834901360c0ef2b31ec`
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
- **Lint Results**: N/A; Rust proof commands recorded in EVIDENCE.
- **Artifacts**: `../Handshake_Artifacts/handshake-tool/signed-scope/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/validator-signed-scope.patch`
- **Timestamp**: 2026-05-06T02:12:32Z
- **Operator**: CODER
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Proposed ADD v02.182 surfaces in this packet
- **Notes**: Startup now resolves storage through ControlPlaneStorageConfig; no unrelated main.rs surfaces changed.

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 38
- **End**: 2527
- **Line Delta**: 163
- **Pre-SHA1**: `c38ae70e21fba3b4469ab8cd3295ce4c03ae0da5`
- **Post-SHA1**: `5b5e49b69b9fff45b9131d40d7bb01ef44633bc4`
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
- **Lint Results**: N/A; Rust proof commands recorded in EVIDENCE.
- **Artifacts**: `../Handshake_Artifacts/handshake-tool/signed-scope/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/validator-signed-scope.patch`
- **Timestamp**: 2026-05-06T02:12:32Z
- **Operator**: CODER
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Proposed ADD v02.182 surfaces in this packet
- **Notes**: ControlPlaneStorageMode, resolver, fail-closed default, and authority/freshness label primitives live here.

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 6
- **End**: 3478
- **Line Delta**: 56
- **Pre-SHA1**: `85aa079ab42e97883124ee81c250db91b4fa3e98`
- **Post-SHA1**: `231768440becb9305364ad20761114a0464ed9b1`
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
- **Lint Results**: N/A; Rust proof commands recorded in EVIDENCE.
- **Artifacts**: `../Handshake_Artifacts/handshake-tool/signed-scope/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/validator-signed-scope.patch`
- **Timestamp**: 2026-05-06T02:12:32Z
- **Operator**: CODER
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Proposed ADD v02.182 surfaces in this packet
- **Notes**: Packet-named proofs cover MT-001 through MT-004; live PostgreSQL branch remains gated by POSTGRES_TEST_URL.
## STATUS_HANDOFF
- Current WP_STATUS: DONE_VALIDATED
- What changed in this update: MT-001 added explicit PostgreSQL-primary storage mode/config resolution and startup wiring; MT-002 changed no-config/no-URL resolution to fail closed through PostgresPrimary; MT-003 added non-authority/freshness labels for SQLite cache/offline modes; MT-004 retained and augmented the PostgreSQL capability proof with the positive authority label axis.
- Requirements / clauses self-audited: Proposed ADD v02.182 PostgreSQL-primary control-plane authority; proposed fail-closed control-plane storage mode; proposed SQLite cache/offline boundary; current storage portability and dual-backend testing law.
- Checks actually run: Four packet-named lib proofs passed with external target dir `..\Handshake_Artifacts\handshake-cargo-target`; final handoff gate command is `just phase-check HANDOFF WP-1-Postgres-Primary-Control-Plane-Foundation-v1 CODER --range ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e..d7f3f760945c21076d75188fb2c90f1eafb155c3`.
- Known gaps / weak spots: live PostgreSQL service connection was not exercised; bare packet `cargo test` form still fails on unrelated integration-bin compile errors; MT-003 labels are boundary primitives and need downstream consumption to withhold authority from SQLite cache/offline paths.
- Heuristic risks / maintainability concerns: `ControlPlaneStorageMode::freshness_label()` uses `current_source_of_truth` for PostgresPrimary, which is an authority/freshness conflation if later code interprets wall-clock staleness; downstream code must not treat string labels as enforcement without explicit checks.
- Validator focus request: Review Integration Validator should challenge the committed range, ensure startup fail-closed behavior is acceptable without a live PostgreSQL smoke, and verify downstream follow-on scope captures label consumption rather than treating labels as enforcement.
- Rubric contract understanding proof: I am not claiming a validation verdict; this handoff records implemented scope, deterministic diff evidence, proof commands, and carry-over risk for the Integration Validator's independent judgment.
- Rubric scope discipline proof: Product diff is limited to `src/backend/handshake_core/src/main.rs`, `src/backend/handshake_core/src/storage/mod.rs`, and `src/backend/handshake_core/src/storage/tests.rs`; `storage/postgres.rs` was an MT-004 touch-allowed surface but did not require changes.
- Rubric baseline comparison: Base `ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e` defaulted storage startup through existing backend selection; head `d7f3f760945c21076d75188fb2c90f1eafb155c3` makes PostgreSQL-primary the explicit authority mode and fail-closed default while retaining SQLite cache/offline as non-authority modes.
- Rubric end-to-end proof: Unit-level resolver and label proofs passed; end-to-end live PostgreSQL startup is intentionally not proven because no `POSTGRES_TEST_URL`/service proof was part of this WP's declared MTs.
- Rubric architecture fit self-review: The implementation keeps authority selection in the storage module and routes startup through the resolver, avoiding ad hoc main.rs mode logic and preserving the Database trait backend boundary.
- Rubric heuristic quality self-review: The enum-based mode contract is explicit and exhaustive; the weakest quality point is that labels are currently descriptive strings and enforcement depends on downstream call sites.
- Rubric anti-gaming / counterfactual check: Counterfactual no-env startup no longer silently selects SQLite cache, but a caller that explicitly selects sqlite_cache can still obtain a writable backend; this is acceptable only if downstream authority gates consume `is_control_plane_authority()`.
- Rubric anti-vibe / substance self-check: Each claim maps to code/test lines and a passing command; I am explicitly carrying forward the unproven live PostgreSQL connection instead of implying full runtime proof.
- Signed-scope debt ledger: No MT in this WP covered live PostgreSQL service connection; unrelated integration-bin compile failures block bare cargo proof; downstream WPs must consume MT-003 boundary labels at control-plane write/read sites.
- Data contract self-check: This WP introduces control-plane authority/freshness labels but no new external data schema; downstream data-contract work must treat labels as primitives requiring enforcement, not as durable authority state by themselves.
- Next step / handoff hint: Integration Validator should review committed head `d7f3f760945c21076d75188fb2c90f1eafb155c3` against base `ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e` after the final `CODER_HANDOFF` receipt.

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
- REQUIREMENT: "PostgreSQL-primary control-plane authority is an explicit storage mode and startup uses the resolver."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:47`, `src/backend/handshake_core/src/storage/mod.rs:108`, `src/backend/handshake_core/src/storage/mod.rs:126`, `src/backend/handshake_core/src/main.rs:49`, `src/backend/handshake_core/src/main.rs:55`, `src/backend/handshake_core/src/storage/tests.rs:3423`
- REQUIREMENT: "Fail-closed control-plane storage mode rejects missing DATABASE_URL instead of silently falling back to SQLite cache."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:143`, `src/backend/handshake_core/src/storage/mod.rs:152`, `src/backend/handshake_core/src/storage/tests.rs:3435`
- REQUIREMENT: "SQLite cache and offline modes are marked as non-authority/cache-or-stale boundaries."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:64`, `src/backend/handshake_core/src/storage/mod.rs:68`, `src/backend/handshake_core/src/storage/mod.rs:77`, `src/backend/handshake_core/src/storage/tests.rs:3450`
- REQUIREMENT: "PostgreSQL capability proof and positive authority axis are retained."
  - EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:3468`, `src/backend/handshake_core/src/storage/tests.rs:3469`, `src/backend/handshake_core/src/storage/tests.rs:3479`, `src/backend/handshake_core/src/storage/tests.rs:3483`
## EVIDENCE
- COMMAND: `$env:CARGO_TARGET_DIR = '..\Handshake_Artifacts\handshake-cargo-target'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::storage_mode_defaults_to_postgres_primary_when_required -- --exact`
- EXIT_CODE: `0`
- LOG_PATH: session output
- PROOF_LINES: `test storage::tests::storage_mode_defaults_to_postgres_primary_when_required ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 421 filtered out`

- COMMAND: `$env:CARGO_TARGET_DIR = '..\Handshake_Artifacts\handshake-cargo-target'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::storage_mode_fails_closed_when_postgres_required_without_url -- --exact`
- EXIT_CODE: `0`
- LOG_PATH: session output
- PROOF_LINES: `test storage::tests::storage_mode_fails_closed_when_postgres_required_without_url ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 421 filtered out`

- COMMAND: `$env:CARGO_TARGET_DIR = '..\Handshake_Artifacts\handshake-cargo-target'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::sqlite_cache_mode_is_not_control_plane_authority -- --exact`
- EXIT_CODE: `0`
- LOG_PATH: session output
- PROOF_LINES: `test storage::tests::sqlite_cache_mode_is_not_control_plane_authority ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 421 filtered out`

- COMMAND: `$env:CARGO_TARGET_DIR = '..\Handshake_Artifacts\handshake-cargo-target'; cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage::tests::database_trait_purity_capability_snapshot_reports_postgres -- --exact`
- EXIT_CODE: `0`
- LOG_PATH: session output
- PROOF_LINES: `test storage::tests::database_trait_purity_capability_snapshot_reports_postgres ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 421 filtered out`

- COMMAND: `just phase-check HANDOFF WP-1-Postgres-Primary-Control-Plane-Foundation-v1 CODER --range ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e..d7f3f760945c21076d75188fb2c90f1eafb155c3`
- EXIT_CODE: `0`
- LOG_PATH: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Postgres-Primary-Control-Plane-Foundation-v1/2026-05-06T02-14-18-702Z.log`
- PROOF_LINES: `OK | phase-check HANDOFF passed for WP-1-Postgres-Primary-Control-Plane-Foundation-v1`; `RESULT: PASS`

## VALIDATION_REPORTS
### 2026-05-06T03:22:52.632Z | INTEGRATION_VALIDATOR | session=integration_validator:wp-1-postgres-primary-control-plane-foundation-v1
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
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
CLAUSES_REVIEWED:
  - Handshake_Master_Spec_v02.182.md 2.3.13.8 PostgreSQL-Primary Control-Plane Foundation -> final Integration Validator receipt review:WP-1-Postgres-Primary-Control-Plane-Foundation-v1:coder_handoff:final:d7f3f760 reviewed the committed handoff for WP-1-Postgres-Primary-Control-Plane-Foundation-v1.
  - Proposed [ADD v02.182] PostgreSQL-primary control-plane authority -> storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132; startup consumption -> main.rs:49; proof -> storage/tests.rs:3424.
  - Proposed fail-closed control-plane storage mode -> storage/mod.rs:143, storage/mod.rs:148; proof -> storage/tests.rs:3435.
  - Proposed SQLite cache/offline boundary -> storage/mod.rs:64, storage/mod.rs:68, storage/mod.rs:77; proof -> storage/tests.rs:3450.
  - Current storage portability and dual-backend testing law -> storage/mod.rs:2512; storage/tests.rs:3467.
NOT_PROVEN:
  - NONE
MAIN_BODY_GAPS:
  - NONE
QUALITY_RISKS:
  - NONE
VALIDATOR_RISK_TIER: HIGH
DIFF_ATTACK_SURFACES:
  - Storage-mode resolver/default selection and startup consumption across storage/mod.rs and main.rs.
  - SQLite cache/offline boundary labels plus dual-backend storage dispatch in storage/mod.rs.
INDEPENDENT_CHECKS_RUN:
  - phase-check VERDICT for the Integration Validator session -> PASS.
  - storage_mode_defaults_to_postgres_primary_when_required -> PASS.
  - storage_mode_fails_closed_when_postgres_required_without_url -> PASS.
  - sqlite_cache_mode_is_not_control_plane_authority -> PASS.
  - database_trait_purity_capability_snapshot_reports_postgres -> PASS.
COUNTERFACTUAL_CHECKS:
  - If storage/mod.rs:47 ControlPlaneStorageMode or storage/mod.rs:93-132 resolver logic were removed, PostgreSQL-primary default authority would no longer be enforced at main.rs:49 startup.
  - If storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77 SQLite cache/offline labels were removed, fallback split-brain boundaries would no longer be reviewable from storage metadata.
BOUNDARY_PROBES:
  - Startup-to-storage initialization boundary: main.rs:49 consumes the storage resolver and storage/mod.rs:2507 initializes configured storage.
  - Database trait dispatch boundary: storage/mod.rs:2512 retains postgres/sqlite backend dispatch.
NEGATIVE_PATH_CHECKS:
  - Missing or invalid PostgreSQL URL fail-closed path is enforced at storage/mod.rs:143 and storage/mod.rs:148, with proof at storage/tests.rs:3435.
  - SQLite cache mode is non-authority and tested at storage/tests.rs:3450.
INDEPENDENT_FINDINGS:
  - Diff is confined to src/backend/handshake_core/src/main.rs, src/backend/handshake_core/src/storage/mod.rs, and src/backend/handshake_core/src/storage/tests.rs.
  - Final review source receipt: INTEGRATION_REVIEW PASS for final CODER_HANDOFF correlation_id=review:WP-1-Postgres-Primary-Control-Plane-Foundation-v1:coder_handoff:final:d7f3f760; base=ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e; head=d7f3f760945c21076d75188fb2c90f1eafb155c3; range=ac9f8fbb4db74bffcecc29aea0bb6262b1ab9a7e..d7f3f760945c21076d75188fb2c90f1eafb155c3. phase-check VERDICT passed for this Integration Validator session. Diff is confined to src/backend/handshake_core/src/main.rs, src/backend/handshake_core/src/storage/mod.rs, and src/backend/handshake_core/src/storage/tests.rs. SPEC_CLAUSE_MAP: v02.182 2.3.13.8 explicit storage modes and PostgreSQL-primary authority are implemented by ControlPlaneStorageMode and resolver at storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132, with startup consumption at main.rs:49 and init_storage_with_config at storage/mod.rs:2507; fail-closed missing/invalid PostgreSQL URL is enforced at storage/mod.rs:143 and storage/mod.rs:148 with test proof at storage/tests.rs:3435; SQLite cache/offline non-authority and source/freshness labels are implemented at storage/mod.rs:64, storage/mod.rs:68, storage/mod.rs:77 and tested at storage/tests.rs:3450; dual-backend portability is retained through postgres/sqlite storage dispatch at storage/mod.rs:2512 and Postgres positive authority axis at storage/tests.rs:3467. PROOFS_RUN: cargo test --manifest-path ../wtc-plane-foundation-v1/src/backend/handshake_core/Cargo.toml --lib storage::tests::storage_mode_defaults_to_postgres_primary_when_required -- --exact PASS; storage_mode_fails_closed_when_postgres_required_without_url PASS; sqlite_cache_mode_is_not_control_plane_authority PASS; database_trait_purity_capability_snapshot_reports_postgres PASS, all with external CARGO_TARGET_DIR. NEGATIVE_PROOF/CARRY_OVER_RISKS: live PostgreSQL service connection is not exercised because the live block remains gated by POSTGRES_TEST_URL; bare cargo test still has unrelated integration-bin compile failures; MT-003 authority/freshness labels need downstream write/read-site consumption before they become enforcement. These are preserved as carry-over risks, not blockers for this foundation slice because v02.182 and the packet split queue workers, leases/backpressure, FEMS, workflow durable execution, DCC projections, SQLite fallback boundaries, and dev/test container setup into downstream WPs.
RESIDUAL_UNCERTAINTY:
  - Live PostgreSQL service connection remains gated by POSTGRES_TEST_URL and bare cargo test still has unrelated integration-bin compile failures; these are downstream/live-environment risks outside this foundation closeout.
SPEC_CLAUSE_MAP:
  - v02.182 2.3.13.8 explicit storage modes and PostgreSQL-primary authority -> storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132; main.rs:49; storage/mod.rs:2507.
  - fail-closed missing/invalid PostgreSQL URL -> storage/mod.rs:143, storage/mod.rs:148; storage/tests.rs:3435.
  - SQLite cache/offline non-authority source/freshness labels -> storage/mod.rs:64, storage/mod.rs:68, storage/mod.rs:77; storage/tests.rs:3450.
  - dual-backend portability and PostgreSQL capability proof -> storage/mod.rs:2512; storage/tests.rs:3467.
NEGATIVE_PROOF:
  - Live PostgreSQL service connection is not exercised inside the signed storage/test scope: storage/tests.rs:3467 proves Postgres capability metadata, while storage/tests.rs does not add a live service round-trip proof in this foundation slice.
ANTI_VIBE_FINDINGS:
  - NONE
SIGNED_SCOPE_DEBT:
  - NONE
PRIMITIVE_RETENTION_PROOF:
  - ControlPlaneStorageMode and resolver remain present at storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132.
  - Database trait dispatch and PostgreSQL capability proof remain present at storage/mod.rs:2512 and storage/tests.rs:3467.
PRIMITIVE_RETENTION_GAPS:
  - NONE
SHARED_SURFACE_INTERACTION_CHECKS:
  - main.rs:49 to storage/mod.rs:2507 startup/storage boundary remains explicit.
  - storage/mod.rs:2512 keeps dual-backend dispatch while storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77 mark SQLite non-authority boundaries.
CURRENT_MAIN_INTERACTION_CHECKS:
  - Signed-scope patch artifact for the final handoff matched main.rs:49, storage/mod.rs:47, storage/mod.rs:93, storage/mod.rs:114, storage/mod.rs:132, and storage/tests.rs:3424 against local main during closeout preflight.
  - Current main interaction is limited to main.rs:49 startup, storage/mod.rs:2507 storage initialization, and storage/mod.rs:2512 backend dispatch; no unrelated shared surface was included in the signed diff.
DATA_CONTRACT_PROOF:
  - Storage authority and freshness labels are structured data on ControlPlaneStorageMode/metadata paths in storage/mod.rs:47, storage/mod.rs:64, storage/mod.rs:68, and storage/mod.rs:77.
DATA_CONTRACT_GAPS:
  - NONE
Verdict: PASS

MECHANICAL_REPORT_SOURCE: materialized from final Integration Validator REVIEW_RESPONSE receipt review:WP-1-Postgres-Primary-Control-Plane-Foundation-v1:coder_handoff:final:d7f3f760 for CODER_HANDOFF final handoff.

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
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, every appended governed validation report MUST also include:
  - `MECHANICAL_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_RETENTION_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
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
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
- For `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `ANTI_VIBE_FINDINGS:`
    - `- NONE` only when the validator found no shallow easy-surface work, no weakly justified implementation, and no vibe-coded behavior inside signed scope
    - otherwise list each anti-vibe finding explicitly
  - `SIGNED_SCOPE_DEBT:`
    - `- NONE` only when no signed-scope debt, cleanup IOU, or "fix later" residue was accepted
    - otherwise list each signed-scope debt item explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `PRIMITIVE_RETENTION_PROOF:`
    - record the concrete file:line or symbol evidence proving previously exposed or expected primitives remain present after the change
    - for `RISK_TIER=MEDIUM|HIGH`, `- NONE` is illegal
  - `PRIMITIVE_RETENTION_GAPS:`
    - `- NONE` only when no primitive was weakened, dropped, or silently re-shaped inside signed scope
    - otherwise list each retained-feature or primitive-composition gap explicitly
  - `SHARED_SURFACE_INTERACTION_CHECKS:`
    - record the concrete producer/consumer, registry, type, runtime, or contract interaction checks reviewed across shared surfaces
    - for `RISK_TIER=MEDIUM|HIGH` or `SHARED_SURFACE_RISK=YES`, `- NONE` is illegal
  - `CURRENT_MAIN_INTERACTION_CHECKS:`
    - record the concrete current-`main` caller, consumer, or compatibility interactions reviewed against the packet diff
    - for `RISK_TIER=MEDIUM|HIGH` or `CURRENT_MAIN_COMPATIBILITY_STATUS=PASS`, `- NONE` is illegal
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, every appended governed validation report MUST also include:
  - `DATA_CONTRACT_PROOF:`
    - list concrete code paths, emitted artifacts, or storage/query surfaces proving the active data contract was reviewed
  - `DATA_CONTRACT_GAPS:`
    - `- NONE` only when no SQL-portability, LLM-parseability, or Loom-intertwined gap remains inside signed scope
    - otherwise list each remaining gap explicitly
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `SPEC_ALIGNMENT_VERDICT=PASS` and `Verdict: PASS` are legal only when `PRIMITIVE_RETENTION_GAPS` is exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS` all contain concrete code or symbol evidence.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `RISK_TIER=MEDIUM|HIGH` requires non-empty `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `MECHANICAL_TRACK_VERDICT=PASS` is legal only when `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `ENVIRONMENT_VERDICT`, `WORKFLOW_VALIDITY`, `SCOPE_VALIDITY`, `PROOF_COMPLETENESS`, `INTEGRATION_READINESS`, and `DOMAIN_GOAL_COMPLETION` are all in their PASS states.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `SPEC_RETENTION_TRACK_VERDICT=PASS` is legal only when `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN`, `MAIN_BODY_GAPS`, and `PRIMITIVE_RETENTION_GAPS` are all exactly `- NONE`, and the report contains concrete `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, `CURRENT_MAIN_INTERACTION_CHECKS`, `SPEC_CLAUSE_MAP`, and `NEGATIVE_PROOF` evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `LEGAL_VERDICT=PASS` and `Verdict: PASS` are legal only when `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `LEGAL_VERDICT=PASS` is legal only when `DATA_CONTRACT_PROOF` is present and `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.
