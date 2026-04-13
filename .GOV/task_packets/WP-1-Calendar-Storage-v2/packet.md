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

# Task Packet: WP-1-Calendar-Storage-v2

## METADATA
- TASK_ID: WP-1-Calendar-Storage-v2
- WP_ID: WP-1-Calendar-Storage-v2
- BASE_WP_ID: WP-1-Calendar-Storage
- DATE: 2026-04-13T09:54:43.222Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
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
- ACTIVATION_MANAGER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Optional but authoritative when Activation Manager launch or repair resumes from the packet. -->
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: claude-opus-4-6
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
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
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
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Calendar-Storage-v2
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Calendar-Storage-v2
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-calendar-storage-v2
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Storage-v2
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Storage-v2
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Calendar-Storage-v2
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Storage-v2
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Storage-v2
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Calendar-Storage-v2
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Calendar-Storage-v2
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Calendar-Storage-v2
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Migration-Framework, WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Storage-Capability-Boundary-Refactor
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Lens, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration, WP-1-Calendar-Law-Compliance-Tests, WP-1-Calendar-Correlation-Export, WP-1-Calendar-Mailbox-Correlation
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Calendar-Storage-v2
- LOCAL_WORKTREE_DIR: ../wtc-calendar-storage-v2
- REMOTE_BACKUP_BRANCH: feat/WP-1-Calendar-Storage-v2
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Calendar-Storage-v2
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Storage-v2
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Storage-v2/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Storage-v2/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Calendar-Storage-v2/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja130420261117
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [HSK-CAL-WRITE-GATE] mutation governance | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: temporal invariants (2.1.1) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: recurrence invariants (2.1.2) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013] | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs; ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: CalendarEvent and ActivitySpan join semantics (11.9.3) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
  - targeted validator review of calendar row shape, migration truth, and dual-backend behavior
- CANONICAL_CONTRACT_EXAMPLES:
  - a calendar source row carrying sync-state and governed write-context metadata
  - a calendar event row proving time-window query shape and provider-payload preservation
  - a same-source/same-external-id upsert path that stays idempotent across both backends
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql (migration/sql surface)
  - IN_SCOPE_PATH: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.down.sql (migration/sql surface)
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Calendar | CAPABILITY_SLICE: deterministic time-window query substrate | SUBFEATURES: overlap queries, source filtering, canonical window shape | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Downstream correlation and Lens consumers depend on this query surface.
  - PILLAR_DECOMPOSITION: PILLAR: Flight Recorder | CAPABILITY_SLICE: provenance-ready calendar rows | SUBFEATURES: job/workflow-linked write context, stable mutation back-links | PRIMITIVES_FEATURES: PRIM-CalendarEvent, PRIM-CalendarSource | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet preserves storage truth so later FR emitters can link calendar writes to governed execution.
  - PILLAR_DECOMPOSITION: PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: governed mutation persistence | SUBFEATURES: write-context columns, source sync-state durability, workflow-facing row updates | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarSyncStateStage, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The runtime workflow remains downstream, but the durable storage substrate is in scope here.
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable calendar migrations and tests | SUBFEATURES: DB-agnostic DDL, replay safety, dual-backend conformance | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the main portability obligation of the packet.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical calendar retrieval substrate | SUBFEATURES: stable IDs, provenance-preserving rows, queryable source sync state | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet prepares the backend truth later retrieval/scope-hint layers consume.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: calendar scope-hint and policy compilation | SUBFEATURES: time-range selection, source trust posture, policy-profile attachment | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet only prepares the rows/query surface that policy compilation will later consume.
  - PILLAR_DECOMPOSITION: PILLAR: Flight Recorder | CAPABILITY_SLICE: calendar-to-activity correlation and export | SUBFEATURES: overlap joins, debug/export bundles, activity annotation | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Correlation-Export-v1 | NOTES: Storage/query substrate is in scope here; correlation/export workflow is not.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar source and event persistence | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: This packet owns the storage abstraction, not a runtime-invocable workflow.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar write-context and provenance durability | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Existing row metadata already persists job/workflow/actor provenance and must remain aligned across both backends.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar time-window query substrate | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Query substrate only; downstream UI/workflow surfaces remain separate.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar sync workflow orchestration | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation, calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Sync-Engine-v1 | Notes: This packet prepares the storage layer that sync orchestration will consume.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar scope-hint and policy projection | JobModel: WORKFLOW | Workflow: calendar_policy_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: Scope-hint/policy projection is downstream of the storage substrate.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: calendar lens query consumption | JobModel: UI_ACTION | Workflow: calendar_lens_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: User-facing calendar views remain downstream.
  - FORCE_MULTIPLIER_EXPANSION: portable calendar schema plus dual-backend conformance stays the root blocker remover -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: calendar sync orchestration consumes the same storage truth without adding shadow schemas -> NEW_STUB (stub: WP-1-Calendar-Sync-Engine-v1)
  - FORCE_MULTIPLIER_EXPANSION: activity correlation and export reuse the time-window substrate -> NEW_STUB (stub: WP-1-Calendar-Correlation-Export-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.down.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Calendar-Storage-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.4 Mutation governance (Hard Invariant) [ilja251220250127]
- CONTEXT_START_LINE: 55894
- CONTEXT_END_LINE: 55899
- CONTEXT_TOKEN: calendar_mutation
- EXCERPT_ASCII_ESCAPED:
  ```text
- **[HSK-CAL-WRITE-GATE]:** Direct database writes to `calendar_events` are **PROHIBITED** from the API layer or UI components.
  - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - Every successful mutation MUST emit a `Flight Recorder` span of type `calendar_mutation` with a back-link to the `job_id`.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.4 Mutation and governance rules
- CONTEXT_START_LINE: 55912
- CONTEXT_END_LINE: 55920
- CONTEXT_TOKEN: Patch-sets are the only write primitive
- EXCERPT_ASCII_ESCAPED:
  ```text
- **No direct UI writes:** UI gestures emit jobs; only the host applies patches after validation and gates.
  - **Patch-sets are the only write primitive:** all calendar writes (local or external) are expressed as validated patch-sets with:
    - preconditions (`expected_etag`, `expected_local_rev`)
    - effect (`set`, `unset`, `append`, `remove`)
    - provenance (`job_id`, `client_op_id`, `idempotency_key`)
  - **External writes are explicitly gated:** any write that leaves the device requires capability + user confirmation unless the source is configured as `auto_export=true`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1 Raw entities
- CONTEXT_START_LINE: 55927
- CONTEXT_END_LINE: 55966
- CONTEXT_TOKEN: CalendarSource (RawContent)
- EXCERPT_ASCII_ESCAPED:
  ```text
CalendarEvent (RawContent)
  - id (RID)
  - workspace_id
  - source_id (CalendarSource.id, e.g. "local", "google:...", "ics:...")
  - external_id (nullable; provider-specific event id)
  ...
  - attendees[] (ParticipantRef)
  - links[] (EntityLinkRef -> doc, canvas, task, mail_thread, etc.)
  - created_by (User/Agent RID)

  CalendarSource (RawContent)
  - id: "local:<id>" | "google:<account_id>:<calendar_id>" | "ics:<url>" | ...
  - type: "local" | "google" | "ics" | "caldav" | "other"
  ...
  - capability_profile_id: which jobs/agents may touch this source
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1.1 Temporal invariants
- CONTEXT_START_LINE: 55976
- CONTEXT_END_LINE: 55995
- CONTEXT_TOKEN: Canonical storage
- EXCERPT_ASCII_ESCAPED:
  ```text
Handshake must treat time as a **deterministic, lossless** domain.
  - **Canonical storage:** store `start_ts_utc` and `end_ts_utc` as UTC instants, and also store the originating `tzid` ...
  Required fields (additions to `CalendarEvent`):
  - `tzid: string`
  - `start_ts_utc: timestamp`
  - `end_ts_utc: timestamp`
  - `start_local: string?`
  - `end_local: string?`
  - `was_floating: bool`
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.1.2 Recurrence invariants
- CONTEXT_START_LINE: 55996
- CONTEXT_END_LINE: 56025
- CONTEXT_TOKEN: CalendarEventOverride
- EXCERPT_ASCII_ESCAPED:
  ```text
- **RRULE is source-of-truth:** store RRULE + `DTSTART` semantics + exceptions (`EXDATE`, `RDATE`) without lossy \u201cflattening\u201d.
  Required fields / structures:
  - `rrule: string?`
  - `rdate: string[]?`
  - `exdate: string[]?`
  - `series_id: string?`
  - `instance_key: string?`
  - `is_override: bool`
  CalendarEventOverride
  - id
  - series_id
  - instance_key
  - patch_set (start/end/title/attendees/etc)
  - created_by (human | job_id)
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.0.5 Never-lose-data rule
- CONTEXT_START_LINE: 55921
- CONTEXT_END_LINE: 55925
- CONTEXT_TOKEN: source_payload
- EXCERPT_ASCII_ESCAPED:
  ```text
- Preserve the original provider payload in `source_payload` (encrypted-at-rest if needed).
  - If parsing fails, store the raw record with `parse_status="failed"` and surface it as \u201cunparsed event\u201d, never drop it.
  ```

#### ANCHOR 7
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3 Storage and indexing
- CONTEXT_START_LINE: 56066
- CONTEXT_END_LINE: 56121
- CONTEXT_TOKEN: CREATE TABLE calendar_sources
- EXCERPT_ASCII_ESCAPED:
  ```text
- Relational table `calendar_events` with indices on `(workspace_id, start_ts, end_ts)` and full-text on `title`, `description`, `location`.
  CREATE TABLE calendar_sources (
      id TEXT PRIMARY KEY NOT NULL,
      ...
  );
  CREATE TABLE calendar_events (
      id TEXT PRIMARY KEY NOT NULL,
      ...
      FOREIGN KEY (source_id) REFERENCES calendar_sources(id)
  );
  ```

#### ANCHOR 8
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3.13.1 Pillar 2: Portable Schema & Migrations [CX-DBP-011]
- CONTEXT_START_LINE: 3282
- CONTEXT_END_LINE: 3296
- CONTEXT_TOKEN: Pillar 2: Portable Schema
- EXCERPT_ASCII_ESCAPED:
  ```text
**Pillar 2: Portable Schema & Migrations [CX-DBP-011]**
  All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.
  - FORBIDDEN: `strftime()`, SQLite datetime functions
  - FORBIDDEN: SQLite placeholder syntax `?1`, `?2`
  - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics
  - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - REQUIRED: Schema definitions are pure DDL (no data transforms)
  ```

#### ANCHOR 9
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a72.3.13.1 Pillar 4: Dual-Backend Testing Early [CX-DBP-013]
- CONTEXT_START_LINE: 3314
- CONTEXT_END_LINE: 3325
- CONTEXT_TOKEN: Pillar 4: Dual-Backend Testing
- EXCERPT_ASCII_ESCAPED:
  ```text
**Pillar 4: Dual-Backend Testing Early [CX-DBP-013]**
  - REQUIRED: Storage layer tests parameterized for both backends
  - REQUIRED: CI pipeline includes PostgreSQL test variant
  - REQUIRED: New storage features tested against both backends before merge
  - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  ```

#### ANCHOR 10
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a710.4.0 Scope and positioning
- CONTEXT_START_LINE: 55799
- CONTEXT_END_LINE: 55806
- CONTEXT_TOKEN: backend force multiplier
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.155] In Phase 1, Calendar is also a backend force multiplier: `CalendarSourceSyncState`, `CalendarSource.write_policy`, `CalendarEvent.export_mode`, `capability_profile_id`, and `CalendarScopeHint` are canonical backend contracts for sync recovery, consent posture, AI-job mutation discipline, and scope-hint routing. These contracts MUST remain portable across SQLite-now / PostgreSQL-ready storage...
  ```

#### ANCHOR 11
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a711.9.3 CalendarEvent and ActivitySpan Join Semantics
- CONTEXT_START_LINE: 70743
- CONTEXT_END_LINE: 70772
- CONTEXT_TOKEN: CalendarEvent and ActivitySpan Join
- EXCERPT_ASCII_ESCAPED:
  ```text
A calendar block is a time window; activity is a set of spans.
  Overlap definition:
  - Represent all spans as half-open intervals: `[start_ts, end_ts)`.
  - A span \u201cbelongs\u201d to an event if:
    - `span.start_ts < event.end_ts` AND `span.end_ts > event.start_ts` (any overlap)
  ```

#### ANCHOR 12
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md \u00a710.4.2.1 CalendarScopeHint
- CONTEXT_START_LINE: 57452
- CONTEXT_END_LINE: 57468
- CONTEXT_TOKEN: CalendarScopeHint
- EXCERPT_ASCII_ESCAPED:
  ```text
CalendarScopeHint (DerivedContent, ephemeral)
  - time_range: [start_ts, end_ts)
  - active_event_id?: CalendarEvent.id
  - source: (active_event | manual_override | none)
  - policy_profile_id?: string
  - projection: (minimal | full | analytics_only)
  ...
  - trust_level: (local_authoritative | external_import | unknown)
  - confidence: float
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: [HSK-CAL-WRITE-GATE] mutation governance | WHY_IN_SCOPE: storage must preserve governed write-context truth and must not create an easier bypass path while v2 realigns existing code | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: calendar writes remain technically possible but governance truth becomes harder to prove
  - CLAUSE: temporal invariants (2.1.1) | WHY_IN_SCOPE: v2 must prove the existing row models and migrations still preserve the required time fields and query semantics | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: silent time-window drift and user-visible corruption
  - CLAUSE: recurrence invariants (2.1.2) | WHY_IN_SCOPE: v2 must decide whether the current storage shape is sufficient or needs code changes to preserve recurrence semantics under the current packet scope | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: recurring-event behavior looks complete while drift stays latent
  - CLAUSE: portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013] | WHY_IN_SCOPE: this is a backend blocker whose value comes from remaining portable and validated across SQLite and Postgres | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs; ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | RISK_IF_MISSED: storage appears finished while Postgres drift or migration debt survives
  - CLAUSE: CalendarEvent and ActivitySpan join semantics (11.9.3) | WHY_IN_SCOPE: the packet owns the overlap-query substrate that downstream correlation packets consume | EXPECTED_CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | EXPECTED_TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | RISK_IF_MISSED: later correlation/export packets inherit a broken substrate
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `CalendarSource` and `CalendarSourceUpsert` row contract | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, storage/tests.rs | SERIALIZER_TRANSPORT: sqlx row mapping and JSON sync-state payload | VALIDATOR_READER: calendar_storage_tests.rs plus storage/tests.rs | TRIPWIRE_TESTS: calendar storage conformance suite | DRIFT_RISK: source sync-state and write-policy fields drift between structs and SQL
  - CONTRACT: `CalendarEvent` and `CalendarEventUpsert` row contract | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, storage/tests.rs | SERIALIZER_TRANSPORT: sqlx row mapping and JSON attendees/links/provider payload columns | VALIDATOR_READER: calendar_storage_tests.rs plus storage/tests.rs | TRIPWIRE_TESTS: calendar storage conformance suite | DRIFT_RISK: migrations and row mappers silently disagree about recurrence, payload, or provenance fields
  - CONTRACT: `CalendarEventWindowQuery` overlap semantics | PRODUCER: storage/calendar.rs | CONSUMER: sqlite.rs, postgres.rs, downstream correlation, Lens, and policy packets | SERIALIZER_TRANSPORT: query inputs and SQL overlap filters | VALIDATOR_READER: storage/tests.rs | TRIPWIRE_TESTS: storage calendar query tests | DRIFT_RISK: later consumers assume half-open overlap semantics that the storage layer no longer actually enforces
  - CONTRACT: calendar migration 0015 <-> runtime structs/tests | PRODUCER: migrations/0015_calendar_storage.sql | CONSUMER: storage/calendar.rs; sqlite.rs; postgres.rs; tests | SERIALIZER_TRANSPORT: SQL DDL | VALIDATOR_READER: calendar_storage_tests.rs | TRIPWIRE_TESTS: migration plus calendar storage test plan | DRIFT_RISK: the migration keeps legacy shapes while structs/tests evolve separately
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Audit the existing calendar migration, structs, storage trait methods, backend implementations, and tests against the signed refinement instead of starting greenfield.
  - Patch the smallest truthful set of storage-model, migration, and test changes needed to align current code to the v2 scope.
  - Re-run the calendar-specific and storage-wide proof commands from the product worktree until they pass cleanly.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
- CARRY_FORWARD_WARNINGS:
  - Do not act on the deleted-branch and greenfield story from the rejected first draft; the real code already exists.
  - Do not widen into Lens, sync orchestration, policy integration, correlation export, or mailbox correlation.
  - Do not silently mint new PRIM IDs or appendix claims that are not already present in the current spec.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - current code reality versus the signed v2 scope
  - temporal and recurrence storage truth
  - portable migration plus dual-backend conformance
  - governed write-context and provenance preservation
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  - cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
- POST_MERGE_SPOTCHECKS:
  - verify the validated calendar storage changes are present on `main`
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - This refinement does not prove whether the current JSON-based attendees/links/provider-payload storage is sufficient under the final validator interpretation or whether a narrower row-shape change is still required.
  - This refinement does not prove that the monolithic `Database` trait is the final acceptable boundary shape under current storage-capability guidance; coder and validators must settle that on real code, not prose.
  - This refinement does not prove any downstream Lens, sync, policy, correlation, or mailbox consumer behavior.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
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
  - PRIM-SqliteDatabase
  - PRIM-PostgresDatabase
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_EXPOSED:
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.dba
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: NEEDS_STUBS
- PILLARS_TOUCHED:
  - Flight Recorder
  - Calendar
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - Flight Recorder: WP-1-Calendar-Correlation-Export-v1
  - Calendar: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
  - Execution / Job Runtime: WP-1-Calendar-Sync-Engine-v1
  - LLM-friendly data: WP-1-Calendar-Lens-v3, WP-1-Calendar-Policy-Integration-v1
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_RESOLUTIONS:
  - portable calendar schema plus dual-backend conformance stays the root blocker remover -> IN_THIS_WP (stub: NONE)
  - governed write-context columns preserve Flight Recorder back-linkability -> IN_THIS_WP (stub: NONE)
  - source sync-state durability unblocks workflow-driven recovery and retries -> IN_THIS_WP (stub: NONE)
  - canonical time-window query shape becomes the shared substrate for later consumers -> IN_THIS_WP (stub: NONE)
  - calendar sync orchestration consumes the same storage truth without adding shadow schemas -> NEW_STUB (stub: WP-1-Calendar-Sync-Engine-v1)
  - calendar lens uses the same time-window/query contract instead of bespoke fetch rules -> NEW_STUB (stub: WP-1-Calendar-Lens-v3)
  - scope-hint and policy projection read governed calendar substrate -> NEW_STUB (stub: WP-1-Calendar-Policy-Integration-v1)
  - activity correlation and export reuse the time-window substrate -> NEW_STUB (stub: WP-1-Calendar-Correlation-Export-v1)
- STUB_WP_IDS: WP-1-Calendar-Lens-v3, WP-1-Calendar-Sync-Engine-v1, WP-1-Calendar-Policy-Integration-v1, WP-1-Calendar-Law-Compliance-Tests-v1, WP-1-Calendar-Correlation-Export-v1, WP-1-Calendar-Mailbox-Correlation-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS
- DECOMPOSITION_ROWS:
  - PILLAR: Calendar | CAPABILITY_SLICE: durable source and event storage | SUBFEATURES: source CRUD, event upsert, source-scoped cleanup | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceUpsert, PRIM-CalendarEvent, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the direct storage baseline already present in v1 code and still owned by v2.
  - PILLAR: Calendar | CAPABILITY_SLICE: deterministic time-window query substrate | SUBFEATURES: overlap queries, source filtering, canonical window shape | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Downstream correlation and Lens consumers depend on this query surface.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: provenance-ready calendar rows | SUBFEATURES: job/workflow-linked write context, stable mutation back-links | PRIMITIVES_FEATURES: PRIM-CalendarEvent, PRIM-CalendarSource | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet preserves storage truth so later FR emitters can link calendar writes to governed execution.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: governed mutation persistence | SUBFEATURES: write-context columns, source sync-state durability, workflow-facing row updates | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarSyncStateStage, PRIM-CalendarEventUpsert | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The runtime workflow remains downstream, but the durable storage substrate is in scope here.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: portable calendar migrations and tests | SUBFEATURES: DB-agnostic DDL, replay safety, dual-backend conformance | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the main portability obligation of the packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: canonical calendar retrieval substrate | SUBFEATURES: stable IDs, provenance-preserving rows, queryable source sync state | PRIMITIVES_FEATURES: PRIM-CalendarSource, PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This packet prepares the backend truth later retrieval/scope-hint layers consume.
  - PILLAR: Calendar | CAPABILITY_SLICE: sync orchestration and provider adapters | SUBFEATURES: pull/push workflows, conflict resolution, mutation workflows | PRIMITIVES_FEATURES: PRIM-CalendarMutation, PRIM-CalendarSyncInput, PRIM-CalendarSourceSyncState | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Sync-Engine-v1 | NOTES: Existing storage rows unblock this work, but the workflow and adapter layer is out of scope here.
  - PILLAR: Calendar | CAPABILITY_SLICE: calendar lens and projection consumers | SUBFEATURES: agenda/timeline views, user-facing filters, drill-down queries | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Lens-v3 | NOTES: Lens remains a separate user-facing packet.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: calendar scope-hint and policy compilation | SUBFEATURES: time-range selection, source trust posture, policy-profile attachment | PRIMITIVES_FEATURES: PRIM-CalendarSourceSyncState, PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Policy-Integration-v1 | NOTES: This packet only prepares the rows/query surface that policy compilation will later consume.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: calendar-to-activity correlation and export | SUBFEATURES: overlap joins, debug/export bundles, activity annotation | PRIMITIVES_FEATURES: PRIM-CalendarEventWindowQuery | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: NEW_STUB | STUB: WP-1-Calendar-Correlation-Export-v1 | NOTES: Storage/query substrate is in scope here; correlation/export workflow is not.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: NEEDS_STUBS
- ALIGNMENT_ROWS:
  - Capability: calendar source and event persistence | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: This packet owns the storage abstraction, not a runtime-invocable workflow.
  - Capability: calendar write-context and provenance durability | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Existing row metadata already persists job/workflow/actor provenance and must remain aligned across both backends.
  - Capability: calendar time-window query substrate | JobModel: NONE | Workflow: N/A | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Query substrate only; downstream UI/workflow surfaces remain separate.
  - Capability: calendar sync workflow orchestration | JobModel: AI_JOB | Workflow: calendar_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: calendar_mutation, calendar_sync_result | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Sync-Engine-v1 | Notes: This packet prepares the storage layer that sync orchestration will consume.
  - Capability: calendar scope-hint and policy projection | JobModel: WORKFLOW | Workflow: calendar_policy_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Policy-Integration-v1 | Notes: Scope-hint/policy projection is downstream of the storage substrate.
  - Capability: calendar lens query consumption | JobModel: UI_ACTION | Workflow: calendar_lens_query | ToolSurface: UI_ONLY | ModelExposure: OPERATOR_ONLY | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: NEW_STUB | Stub: WP-1-Calendar-Lens-v3 | Notes: User-facing calendar views remain downstream.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Calendar-Storage-v2 -> EXPAND_IN_THIS_WP
  - WP-1-Calendar-Lens-v3 -> KEEP_SEPARATE
  - WP-1-Calendar-Sync-Engine-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Policy-Integration-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Law-Compliance-Tests-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Correlation-Export-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Mailbox-Correlation-v1 -> KEEP_SEPARATE
  - WP-1-Calendar-Storage-v1 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql -> IMPLEMENTED (WP-1-Calendar-Storage-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs -> IMPLEMENTED (WP-1-Calendar-Storage-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> IMPLEMENTED (WP-1-Calendar-Storage-v1)
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs -> IMPLEMENTED (WP-1-Calendar-Storage-v1)
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
- What: Realign and validate the already-landed calendar storage implementation in `../handshake_main` against the current v02.180 calendar/storage contract, keeping portable SQLite/Postgres behavior while closing the governed validation gap left by v1.
- Why: Calendar storage is the highest-value backend blocker in BUILD_ORDER. The code exists, but until it is truthfully aligned to the current spec/governance workflow and validated, every downstream calendar packet remains blocked on unstable substrate truth.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.down.sql
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
- OUT_OF_SCOPE:
  - Calendar Lens UI and user-facing projection work
  - Sync-engine workflow orchestration and provider adapters
  - Capability/consent policy enforcement flows
  - Calendar correlation/export workflows
  - Mailbox-correlation product logic
  - New repo-governance tooling or protocol changes unrelated to this packet
- TOUCHED_FILE_BUDGET: 8
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml run_calendar_storage_conformance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
```

### DONE_MEANS
- The calendar storage code under `../handshake_main` is truthfully aligned to the current v02.180 packet scope rather than the stale v1 assumptions.
- SQLite and Postgres calendar storage tests pass from the product worktree under the packet's chosen test plan.
- The storage boundary, migrations, row models, and conformance tests agree on the same governed calendar source/event contract.
- WP Validator and Integration Validator pass, and the validated code is integrated into `main`.

- PRIMITIVES_EXPOSED:
  - PRIM-CalendarSource
  - PRIM-CalendarSourceProviderType
  - PRIM-CalendarSourceWritePolicy
  - PRIM-CalendarSourceSyncState
  - PRIM-CalendarSourceUpsert
  - PRIM-CalendarSyncStateStage
  - PRIM-CalendarEvent
  - PRIM-CalendarEventStatus
  - PRIM-CalendarEventVisibility
  - PRIM-CalendarEventExportMode
  - PRIM-CalendarEventUpsert
  - PRIM-CalendarEventWindowQuery
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-13T09:54:43.222Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md \u00a710.4 Calendar + \u00a72.3 Storage and indexing + \u00a72.3.13.1 Portability Pillars
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
  - ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/tests.rs
  - ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs
  - ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql
- SEARCH_TERMS:
  - upsert_calendar_source
  - upsert_calendar_event
  - query_calendar_events
  - CalendarSourceSyncState
  - CalendarEventWindowQuery
  - provider_payload_json
  - attendees_json
  - links_json
- RUN_COMMANDS:
  ```bash
rg -n "upsert_calendar_source|upsert_calendar_event|query_calendar_events|CalendarSourceSyncState|CalendarEventWindowQuery|provider_payload_json|attendees_json|links_json" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage
  ```
- RISK_MAP:
  - "v2 keeps pretending the packet is greenfield" -> "coder and validator act on false premises and miss real code-regression risk"
  - "schema and row-model drift remain hidden behind passing legacy tests" -> "calendar substrate looks done while current spec semantics still diverge"
  - "trait-boundary expectations stay unresolved" -> "future storage packets keep duplicating or bypassing calendar access patterns"
  - "dual-backend coverage regresses during v2 alignment" -> "Postgres readiness erodes while SQLite still passes"
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
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
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
  - LOG_PATH: `.handshake/logs/WP-1-Calendar-Storage-v2/<name>.log` (recommended; not committed)
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
