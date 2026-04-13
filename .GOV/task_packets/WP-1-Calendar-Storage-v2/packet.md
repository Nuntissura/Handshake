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
- MERGE_BASE_SHA: e1243008365566d4cde3c707f1b6078b5837fdcd
<!-- git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence. -->
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
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Calendar-Storage-v2
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
- MERGED_MAIN_COMMIT: 066cc18dcc401d413de5e66073ec84c7a2a0b3db
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-13T14:23:17.657Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 066cc18dcc401d413de5e66073ec84c7a2a0b3db
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-13T14:23:17.657Z
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
- WP_VALIDATOR_OF_RECORD: wp_validator:wp-1-calendar-storage-v2
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-calendar-storage-v2
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
Verdict: PASS
Blockers: NONE
Next: NONE
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: [HSK-CAL-WRITE-GATE] mutation governance | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/mod.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: temporal invariants (2.1.1) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: recurrence invariants (2.1.2) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013] | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/migrations/0015_calendar_storage.sql; ../handshake_main/src/backend/handshake_core/src/storage/tests.rs; ../handshake_main/src/backend/handshake_core/tests/calendar_storage_tests.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml calendar_storage_tests | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: CalendarEvent and ActivitySpan join semantics (11.9.3) | CODE_SURFACES: ../handshake_main/src/backend/handshake_core/src/storage/calendar.rs; ../handshake_main/src/backend/handshake_core/src/storage/sqlite.rs; ../handshake_main/src/backend/handshake_core/src/storage/postgres.rs | TESTS: cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml storage | EXAMPLES: a calendar source row carrying sync-state and governed write-context metadata, a calendar event row proving time-window query shape and provider-payload preservation, a same-source/same-external-id upsert path that stays idempotent across both backends | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
- **Target File**: `src/backend/handshake_core/src/storage/calendar.rs`
- **Start**: 152
- **End**: 318
- **Line Delta**: 10
- **Pre-SHA1**: `9fbd02c81fd0f17cdea6b1bedde2da83797b2e24`
- **Post-SHA1**: `8bb59c03345db33024a84ba46e217d35ad577590`
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
- **Lint Results**: cargo check --lib PASS (0 errors)
- **Artifacts**: `.GOV/Audits/smoketest/WP-1-Calendar-Storage-v2-CANDIDATE_TARGET-066cc18d.patch`, d0832fe0
- **Timestamp**: 2026-04-13T11:12:51Z
- **Operator**: CODER
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- **Notes**: Added 5 provenance fields to CalendarSource (line 155) and CalendarEvent (line 313)

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1071
- **End**: 4296
- **Line Delta**: 40
- **Pre-SHA1**: `8bd60b245b4f3c5729bd2fb8248260cf9bcc6c24`
- **Post-SHA1**: `7a1938f0d8fd42e34668767d389314228ae4e068`
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
- **Notes**: 8 edit sites: map_calendar_source_row, map_calendar_event_row, 3 source SELECT/RETURNING, 3 event SELECT/RETURNING

- **Target File**: `src/backend/handshake_core/src/storage/postgres.rs`
- **Start**: 1619
- **End**: 4737
- **Line Delta**: 40
- **Pre-SHA1**: `482ec755adabdb751605a91c2ff9648dcd0e7533`
- **Post-SHA1**: `793ae16cf037731e209c8a9e55c941f35b8bd167`
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
- **Notes**: 8 edit sites mirroring sqlite.rs: map_calendar_source, map_calendar_event, 3 source SELECT/RETURNING, 3 event SELECT/RETURNING

- **Target File**: `src/backend/handshake_core/src/storage/tests.rs`
- **Start**: 2209
- **End**: 2525
- **Line Delta**: 139
- **Pre-SHA1**: `eb46c0ca165706357d9de294bfb95560d01c5f0d`
- **Post-SHA1**: `590629941d7518b67a3c44bec3453ef5b08e7891`
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
- **Notes**: HUMAN provenance assertions (lines 2212, 2309); workflow-backed AI provenance round-trip with non-None last_job_id/last_workflow_id proving governed read paths (get/list/query) on both backends (lines 2393-2525)
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_VALIDATED
- What changed in this update: (1) Surfaced 5 provenance columns (last_job_id, last_workflow_id, last_actor_id, edit_event_id, last_actor_kind) in CalendarSource and CalendarEvent return types across both backends. (2) Added workflow-backed/job-backed provenance round-trip test (commit cfd7a388) proving last_job_id, last_workflow_id, edit_event_id, and last_actor_kind survive governed read paths (upsert RETURNING, get_calendar_source, list_calendar_sources, query_calendar_events) on both SQLite and PostgreSQL when using WriteContext::ai with non-None job_id and workflow_id. This addresses the validator REVIEW_RESPONSE blocker from correlation review:WP-1-Calendar-Storage-v2:coder_handoff:mnx4u19s:c5d92d.
- Requirements / clauses self-audited: MT-001 [HSK-CAL-WRITE-GATE] mutation governance -- provenance columns present in migration 0015, written via MutationMetadata during upserts, but NOT surfaced in Rust return types or SELECT queries. This was the central gap. MT-002 through MT-005 assessed as already aligned: temporal fields (start_ts_utc, end_ts_utc, tzid, all_day, was_floating), recurrence fields (rrule, rdate, exdate, is_recurring, series_id, instance_key, is_override), half-open overlap query semantics, and migration portability all match spec v02.180.
- Checks actually run: [post-review-response, cfd7a388] cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib (PASS, 0 errors, 34 warnings pre-existing); cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests (PASS, 2/2: sqlite_calendar_storage_conformance + postgres_calendar_storage_conformance, both now include workflow-backed provenance assertions); cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage (PASS, 25/25)
- Known gaps / weak spots: (1) FlightRecorder emission hooks for calendar mutations are absent in the storage layer -- spec-gap, not in WP scope per validator steer. (2) Two pre-existing test files (micro_task_executor_tests.rs, model_session_scheduler_tests.rs) fail to compile when building all test targets -- they reference APIs from unpushed handshake_main commits not yet on origin/main. Not introduced by this WP and not in scope. (3) created_by field on CalendarEvent is nullable but has no spec-mandated default; existing behavior preserved.
- Heuristic risks / maintainability concerns: The 8-site-per-backend edit pattern (map function + 3 source queries + 3 event queries + query builder) is mechanically repetitive. Any future column additions must touch all 8 sites in both backends. The trait abstraction prevents drift between backends but the manual SQL column lists are a maintenance surface.
- Validator focus request: Verify the 5 provenance columns appear in all 8 SELECT/RETURNING sites per backend. Verify the test assertions cover both HUMAN-context (actor_kind="HUMAN", job_id=None, workflow_id=None) AND AI/workflow-context (actor_kind="AI", job_id=Some(...), workflow_id=Some(...)) round-trips through upsert RETURNING, get, list, and query read paths. Confirm MT-002 through MT-005 are truthfully already-aligned rather than overlooked.
- Rubric contract understanding proof: The packet clause for MT-001 requires [HSK-CAL-WRITE-GATE] mutation governance alignment. The spec v02.180 section 2.0.4 mandates that every mutation carries WriteContext provenance through to persisted rows. The gap was that the SQL writes (via MutationMetadata) stored provenance but the Rust return types and SELECT queries did not read it back, making provenance invisible to callers.
- Rubric scope discipline proof: Only 4 files changed, all in src/backend/handshake_core/src/storage/. No trait changes (mod.rs untouched), no migration changes (0015 already correct), no downstream sync/policy/FR changes per validator steer. Diff is 229 insertions, 0 deletions (99 in d0832fe0 + 130 in cfd7a388).
- Rubric baseline comparison: Branch rebased onto origin/main (e1243008). Pre-existing baseline issues (truncated workflows.rs and flight_recorder/mod.rs) resolved by the rebase. The WP branch diff against origin/main shows only the 4 storage files.
- Rubric end-to-end proof: cargo test --test calendar_storage_tests exercises the full upsert->query->assert cycle for both SQLite and PostgreSQL backends, including HUMAN-context provenance assertions (None job/workflow) AND workflow-backed AI-context provenance assertions (non-None job_id, workflow_id, actor_kind=AI). Both backends pass.
- Rubric architecture fit self-review: Changes follow the existing pattern: struct fields in calendar.rs, row.get() in map functions, column names in SQL strings. No new abstractions, no trait changes, no API surface changes. The storage layer remains the single source of truth for column projection.
- Rubric heuristic quality self-review: The 5-field block (last_job_id, last_workflow_id, last_actor_id, edit_event_id, last_actor_kind) is inserted at the same position in every edit site: after the last domain field, before created_at/updated_at. This is consistent with the migration column order and the existing provenance pattern in non-calendar storage methods.
- Rubric anti-gaming / counterfactual check: If the provenance columns were NOT added to the return types, callers would receive CalendarSource/CalendarEvent structs with default/zero values for provenance fields, silently losing audit trail data. The test assertions would not exist, so the gap would be invisible. The conformance test now fails if provenance is not read back.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: Every claim above maps to a specific file:line or test assertion. No vague "improved" or "aligned" statements without evidence.
- Signed-scope debt ledger: No signed-scope debt. All 4 files are within the packet's CODE_SURFACES for MT-001.
- Data contract self-check: CalendarSource and CalendarEvent structs now match the 0015_calendar_storage.sql column set exactly. The Upsert variants intentionally omit provenance (it is injected by the storage layer via WriteContext/MutationMetadata, not caller-supplied).
- Next step / handoff hint: WP Validator should verify MT-001 provenance alignment and assess whether MT-002 through MT-005 require code changes or are truthfully already-aligned. If all MTs clear, proceed to Integration Validator for final merge authority.

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
  - REQUIREMENT: "The calendar storage code is truthfully aligned to the current v02.180 packet scope"
  - EVIDENCE: `src/backend/handshake_core/src/storage/calendar.rs:155` (CalendarSource provenance fields), `src/backend/handshake_core/src/storage/calendar.rs:313` (CalendarEvent provenance fields)
  - REQUIREMENT: "SQLite and Postgres calendar storage tests pass from the product worktree"
  - EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:2212` (source HUMAN provenance assertions), `src/backend/handshake_core/src/storage/tests.rs:2309` (event HUMAN provenance assertions)
  - REQUIREMENT: "Workflow-backed/job-backed WriteContext provenance survives governed read paths on both backends"
  - EVIDENCE: `src/backend/handshake_core/src/storage/tests.rs:2441` (AI source upsert provenance assertions), `src/backend/handshake_core/src/storage/tests.rs:2449` (AI source get provenance assertions), `src/backend/handshake_core/src/storage/tests.rs:2456` (AI source list provenance assertions), `src/backend/handshake_core/src/storage/tests.rs:2497` (AI event upsert provenance assertions), `src/backend/handshake_core/src/storage/tests.rs:2513` (AI event query provenance assertions)
  - REQUIREMENT: "The storage boundary, migrations, row models, and conformance tests agree on the same governed calendar source/event contract"
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:1074` (map_calendar_source_row provenance), `src/backend/handshake_core/src/storage/postgres.rs:1622` (map_calendar_source provenance), `src/backend/handshake_core/src/storage/sqlite.rs:1121` (map_calendar_event_row provenance), `src/backend/handshake_core/src/storage/postgres.rs:1669` (map_calendar_event provenance)
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib`
  - EXIT_CODE: `0`
  - PROOF_LINES: `Finished dev profile [unoptimized + debuginfo] target(s) in 59.84s`
  - SESSION: post-visibility-fix (066cc18d)
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests`
  - EXIT_CODE: `0`
  - PROOF_LINES: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s`
  - SESSION: post-visibility-fix (066cc18d)
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage`
  - EXIT_CODE: `0`
  - PROOF_LINES: `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 282 filtered out; finished in 56.34s`
  - SESSION: post-visibility-fix (066cc18d)

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

### 2026-04-13 INTEGRATION VALIDATION REPORT - WP-1-Calendar-Storage-v2 (Historical Remediation Snapshot)
Historical_Verdict: FAIL
Historical_Validated_at: 2026-04-13T13:22:55.1894314Z
Historical_VALIDATION_CONTEXT: OK
Historical_GOVERNANCE_VERDICT: PASS
Historical_TEST_VERDICT: PARTIAL
Historical_CODE_REVIEW_VERDICT: FAIL
Historical_HEURISTIC_REVIEW_VERDICT: FAIL
Historical_SPEC_ALIGNMENT_VERDICT: FAIL
Historical_ENVIRONMENT_VERDICT: PASS
Historical_DISPOSITION: NONE
Historical_LEGAL_VERDICT: FAIL
Historical_SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
Historical_WORKFLOW_VALIDITY: VALID
Historical_SCOPE_VALIDITY: IN_SCOPE
Historical_PROOF_COMPLETENESS: PARTIAL
Historical_INTEGRATION_READINESS: NOT_READY
Historical_DOMAIN_GOAL_COMPLETION: INCOMPLETE
Historical_MECHANICAL_TRACK_VERDICT: FAIL
Historical_SPEC_RETENTION_TRACK_VERDICT: FAIL
Historical_MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
Historical_MERGED_MAIN_COMMIT: NONE
Historical_MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A

Historical_CLAUSES_REVIEWED:
- [HSK-CAL-WRITE-GATE] mutation governance: FAIL on live `main`. The schema persists provenance columns in `src/backend/handshake_core/migrations/0015_calendar_storage.sql:29-33` and `src/backend/handshake_core/migrations/0015_calendar_storage.sql:69-73`, and both backends bind them on write in `src/backend/handshake_core/src/storage/sqlite.rs:3761-3765`, `src/backend/handshake_core/src/storage/sqlite.rs:4044-4048`, `src/backend/handshake_core/src/storage/sqlite.rs:4201-4205`, `src/backend/handshake_core/src/storage/postgres.rs:4207-4211`, `src/backend/handshake_core/src/storage/postgres.rs:4485-4489`, and `src/backend/handshake_core/src/storage/postgres.rs:4642-4646`. But the returned/read calendar shapes still omit those fields in `src/backend/handshake_core/src/storage/calendar.rs:142-156` and `src/backend/handshake_core/src/storage/calendar.rs:279-307`, the row mappers omit them in `src/backend/handshake_core/src/storage/sqlite.rs:1042-1076`, `src/backend/handshake_core/src/storage/sqlite.rs:1080-1118`, `src/backend/handshake_core/src/storage/postgres.rs:1590-1624`, and `src/backend/handshake_core/src/storage/postgres.rs:1628-1665`, and the live source/event `RETURNING` or read projections omit them in `src/backend/handshake_core/src/storage/sqlite.rs:3706-3733`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4179`, `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`, `src/backend/handshake_core/src/storage/postgres.rs:4275-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`.
- temporal invariants (2.1.1): CONFIRMED. Temporal fields remain explicit in runtime structs at `src/backend/handshake_core/src/storage/calendar.rs:284-290` and `src/backend/handshake_core/src/storage/calendar.rs:320-326`, the migration keeps the same columns in `src/backend/handshake_core/migrations/0015_calendar_storage.sql:47-53`, and the conformance test exercises provider/local time-window behavior in `src/backend/handshake_core/src/storage/tests.rs:2235-2239`, `src/backend/handshake_core/src/storage/tests.rs:2272-2278`, `src/backend/handshake_core/src/storage/tests.rs:2319-2325`, and `src/backend/handshake_core/src/storage/tests.rs:2346-2367`.
- recurrence invariants (2.1.2): CONFIRMED. Recurrence fields remain explicit in `src/backend/handshake_core/src/storage/calendar.rs:294-300` and `src/backend/handshake_core/src/storage/calendar.rs:330-336`, the migration keeps the recurrence columns in `src/backend/handshake_core/migrations/0015_calendar_storage.sql:57-62`, and the conformance test exercises recurring/provider override data in `src/backend/handshake_core/src/storage/tests.rs:2245-2251` and `src/backend/handshake_core/src/storage/tests.rs:2282-2288`.
- portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013]: CONFIRMED. The packet still targets a portable migration surface at `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`, and the dual-backend harness remains live in `src/backend/handshake_core/tests/calendar_storage_tests.rs:7-27`.
- CalendarEvent and ActivitySpan join semantics (11.9.3): CONFIRMED for the owned half-open overlap substrate. SQLite enforces `start_ts_utc < window_end` and `end_ts_utc > window_start` in `src/backend/handshake_core/src/storage/sqlite.rs:4256-4264`, Postgres mirrors it in `src/backend/handshake_core/src/storage/postgres.rs:4697-4704`, and the conformance suite proves the wide and narrow window cases in `src/backend/handshake_core/src/storage/tests.rs:2346-2367`.

Historical_NOT_PROVEN:
- [HSK-CAL-WRITE-GATE] mutation governance is not satisfied on live `main`. The review thread cites MT-001 PASS on commit `066cc18d`, but that commit is not present in this checkout (`git branch --all --contains 066cc18d` failed with `malformed object name 066cc18d`), and the live calendar source/event read paths listed above still omit the five provenance fields.

Historical_MAIN_BODY_GAPS:
- Restore governed provenance to the live calendar source/event read contract on both backends by updating `src/backend/handshake_core/src/storage/calendar.rs:142-156` and `src/backend/handshake_core/src/storage/calendar.rs:279-307`, the SQLite mappers/projections at `src/backend/handshake_core/src/storage/sqlite.rs:1035-1118`, `src/backend/handshake_core/src/storage/sqlite.rs:3706-3733`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, and `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, and the Postgres counterparts at `src/backend/handshake_core/src/storage/postgres.rs:1583-1665`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4179`, `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`, `src/backend/handshake_core/src/storage/postgres.rs:4275-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`.
- Repair the stale packet evidence block at `.GOV/task_packets/WP-1-Calendar-Storage-v2/packet.md:1021-1027`. It currently cites `src/backend/handshake_core/src/storage/calendar.rs:155` and `src/backend/handshake_core/src/storage/tests.rs:2441-2513` as calendar provenance proof, but those lines presently point to `created_at` / generic workspace-document-block-canvas mutation tests rather than calendar source/event read-path coverage.

Historical_QUALITY_RISKS:
- The live code persists provenance on write but drops it at the producer/consumer boundary for returned calendar rows, creating a silent data-contract split that happy-path conformance tests can miss.

Historical_VALIDATOR_RISK_TIER: HIGH

Historical_DIFF_ATTACK_SURFACES:
- Migration columns and write bindings versus runtime `CalendarSource`/`CalendarEvent` return shapes.
- Backend `RETURNING` / `SELECT` / query projections versus row-mapper expectations on both SQLite and Postgres.
- Shared query substrate consumed by downstream policy/correlation packets, where hidden provenance drift would survive green overlap tests.

Historical_INDEPENDENT_CHECKS_RUN:
- `git branch --all --contains 066cc18d` => failed with `malformed object name 066cc18d`; the cited MT-001 review-loop commit is not present in this checkout.
- Side-by-side source inspection of `src/backend/handshake_core/src/storage/calendar.rs`, `src/backend/handshake_core/src/storage/sqlite.rs`, and `src/backend/handshake_core/src/storage/postgres.rs` => live source/event return structs, mappers, and read projections still omit `last_actor_kind`, `last_actor_id`, `last_job_id`, `last_workflow_id`, and `edit_event_id`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests sqlite_calendar_storage_conformance -- --exact` => PASS (`1 passed; 0 failed`).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::sqlite_rejects_ai_writes_without_context_with_hsk_403_silent_edit --lib -- --exact` => PASS (`1 passed; 0 failed`).

Historical_COUNTERFACTUAL_CHECKS:
- If the source projections at `src/backend/handshake_core/src/storage/sqlite.rs:3706-3733`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4179`, `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`, and `src/backend/handshake_core/src/storage/postgres.rs:4275-4302` remain unchanged, calendar-source callers can never observe governed provenance even though the migration stores it.
- If the event projections at `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696` remain unchanged, downstream calendar-window consumers inherit provenance-blind events and cannot prove governed lineage from the returned data.

Historical_BOUNDARY_PROBES:
- Compared `src/backend/handshake_core/migrations/0015_calendar_storage.sql:29-33` and `src/backend/handshake_core/migrations/0015_calendar_storage.sql:69-73` against the runtime structs at `src/backend/handshake_core/src/storage/calendar.rs:142-156` and `src/backend/handshake_core/src/storage/calendar.rs:279-307`, then against the backend mappers at `src/backend/handshake_core/src/storage/sqlite.rs:1035-1118` and `src/backend/handshake_core/src/storage/postgres.rs:1583-1665`. The write-side and schema-side provenance contract exists; the returned/read-side contract does not.

Historical_NEGATIVE_PATH_CHECKS:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::sqlite_rejects_ai_writes_without_context_with_hsk_403_silent_edit --lib -- --exact` => PASS, confirming the existing negative-path guard still rejects AI writes without job/workflow context while the calendar provenance gap remains independent.

Historical_INDEPENDENT_FINDINGS:
- MT-001 is not present on live `main`, regardless of the WP thread's passing coder/WP-validator loop on `066cc18d`.
- MT-002 through MT-005 are already aligned in the live code and did not reveal additional product-code work in this lane.
- The packet's current EVIDENCE block is stale and points at unrelated or no-longer-correct line anchors, so it cannot be reused as final proof.

Historical_RESIDUAL_UNCERTAINTY:
- I did not rerun the Postgres variant in this lane. The live-code inspection supports the same clause conclusions on both backends, but only the SQLite conformance spot-check was re-executed here.
- The exact contents of `066cc18d` remain unknown in this checkout because the commit object is absent locally.

Historical_SPEC_CLAUSE_MAP:
- [HSK-CAL-WRITE-GATE] mutation governance => FAIL on live `main`: persisted in `src/backend/handshake_core/migrations/0015_calendar_storage.sql:29-33` and `src/backend/handshake_core/migrations/0015_calendar_storage.sql:69-73`, but omitted from runtime/read surfaces in `src/backend/handshake_core/src/storage/calendar.rs:142-156`, `src/backend/handshake_core/src/storage/calendar.rs:279-307`, `src/backend/handshake_core/src/storage/sqlite.rs:1035-1118`, `src/backend/handshake_core/src/storage/sqlite.rs:3706-3733`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, `src/backend/handshake_core/src/storage/postgres.rs:1583-1665`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4179`, `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`, `src/backend/handshake_core/src/storage/postgres.rs:4275-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`.
- temporal invariants (2.1.1) => `src/backend/handshake_core/src/storage/calendar.rs:284-290`, `src/backend/handshake_core/src/storage/calendar.rs:320-326`, `src/backend/handshake_core/migrations/0015_calendar_storage.sql:47-53`, and `src/backend/handshake_core/src/storage/tests.rs:2235-2239`, `src/backend/handshake_core/src/storage/tests.rs:2272-2278`, `src/backend/handshake_core/src/storage/tests.rs:2319-2325`, `src/backend/handshake_core/src/storage/tests.rs:2346-2367`.
- recurrence invariants (2.1.2) => `src/backend/handshake_core/src/storage/calendar.rs:294-300`, `src/backend/handshake_core/src/storage/calendar.rs:330-336`, `src/backend/handshake_core/migrations/0015_calendar_storage.sql:57-62`, and `src/backend/handshake_core/src/storage/tests.rs:2245-2251`, `src/backend/handshake_core/src/storage/tests.rs:2282-2288`.
- portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013] => `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88` and `src/backend/handshake_core/tests/calendar_storage_tests.rs:7-27`.
- CalendarEvent and ActivitySpan join semantics (11.9.3) => `src/backend/handshake_core/src/storage/sqlite.rs:4256-4264`, `src/backend/handshake_core/src/storage/postgres.rs:4697-4704`, and `src/backend/handshake_core/src/storage/tests.rs:2346-2367`.

Historical_NEGATIVE_PROOF:
- `.GOV/task_packets/WP-1-Calendar-Storage-v2/packet.md:1021-1027` is not valid live proof for MT-001. In the current checkout, `src/backend/handshake_core/src/storage/calendar.rs:155` is `created_at`, not a provenance field, and `src/backend/handshake_core/src/storage/tests.rs:2441-2513` covers generic mutation-traceability rows for workspaces/documents/blocks/canvases rather than calendar source/event read paths.

Historical_ANTI_VIBE_FINDINGS:
- The packet's current evidence block overclaims live calendar provenance proof by citing unrelated line anchors. This must be corrected in the next loop instead of being carried forward as if it were still true.

Historical_SIGNED_SCOPE_DEBT:
- Outstanding signed-scope debt remains on MT-001 until the live calendar read contract exposes the five governed provenance fields and the packet evidence block is updated to the real calendar-specific anchors.

Historical_PRIMITIVE_RETENTION_PROOF:
- `CalendarSourceWritePolicy`, `CalendarSourceSyncState`, and the calendar source retrieval surface remain present in `src/backend/handshake_core/src/storage/calendar.rs:147-155`, `src/backend/handshake_core/src/storage/calendar.rs:165-172`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, and `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`.
- `CalendarEvent` temporal/recurrence/query primitives remain present in `src/backend/handshake_core/src/storage/calendar.rs:284-305`, `src/backend/handshake_core/src/storage/calendar.rs:320-338`, `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`.

Historical_PRIMITIVE_RETENTION_GAPS:
- The governed mutation-provenance primitive is not retained in the returned/read `CalendarSource` and `CalendarEvent` shapes on live `main`; the schema stores it, but the runtime contract does not expose it.

Historical_SHARED_SURFACE_INTERACTION_CHECKS:
- Compared migration `0015` against SQLite source/event mappers and projections: `src/backend/handshake_core/migrations/0015_calendar_storage.sql:29-33`, `src/backend/handshake_core/migrations/0015_calendar_storage.sql:69-73`, `src/backend/handshake_core/src/storage/sqlite.rs:1035-1118`, `src/backend/handshake_core/src/storage/sqlite.rs:3706-3733`, `src/backend/handshake_core/src/storage/sqlite.rs:3780-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, and `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`.
- Compared the same contract boundary on Postgres in `src/backend/handshake_core/src/storage/postgres.rs:1583-1665`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4179`, `src/backend/handshake_core/src/storage/postgres.rs:4226-4253`, `src/backend/handshake_core/src/storage/postgres.rs:4275-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, and `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`.

Historical_CURRENT_MAIN_INTERACTION_CHECKS:
- Current `main` source consumers call `list_calendar_sources` / `get_calendar_source` through `src/backend/handshake_core/src/storage/sqlite.rs:3774-3864` and `src/backend/handshake_core/src/storage/postgres.rs:4220-4312`, which only project the fields available on `CalendarSource` at `src/backend/handshake_core/src/storage/calendar.rs:142-156`; provenance is therefore inaccessible to current main callers.
- Current `main` event-window consumers call `query_calendar_events` through `src/backend/handshake_core/src/storage/sqlite.rs:4215-4274` and `src/backend/handshake_core/src/storage/postgres.rs:4656-4715`; overlap semantics are intact, but returned `CalendarEvent` values still omit provenance because the current main shapes and mappers at `src/backend/handshake_core/src/storage/calendar.rs:279-307`, `src/backend/handshake_core/src/storage/sqlite.rs:1079-1118`, and `src/backend/handshake_core/src/storage/postgres.rs:1627-1665` do not carry it.

Historical_DATA_CONTRACT_PROOF:
- Reviewed the active calendar data contract across the migration DDL, runtime source/event shapes, backend projections, and conformance suite at `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`, `src/backend/handshake_core/src/storage/calendar.rs:142-172`, `src/backend/handshake_core/src/storage/calendar.rs:279-338`, `src/backend/handshake_core/src/storage/sqlite.rs:1035-1118`, `src/backend/handshake_core/src/storage/sqlite.rs:3706-3807`, `src/backend/handshake_core/src/storage/sqlite.rs:3829-3856`, `src/backend/handshake_core/src/storage/sqlite.rs:3980-4011`, `src/backend/handshake_core/src/storage/sqlite.rs:4223-4255`, `src/backend/handshake_core/src/storage/postgres.rs:1583-1665`, `src/backend/handshake_core/src/storage/postgres.rs:4152-4253`, `src/backend/handshake_core/src/storage/postgres.rs:4275-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4420-4452`, `src/backend/handshake_core/src/storage/postgres.rs:4664-4696`, `src/backend/handshake_core/src/storage/tests.rs:2235-2367`, and `src/backend/handshake_core/tests/calendar_storage_tests.rs:7-27`.

Historical_DATA_CONTRACT_GAPS:
- Governed provenance is persisted but not exposed in the live calendar source/event read contract on either backend.

Historical_REMEDIATION_REQUIRED:
1. Add the five governed provenance fields to the live `CalendarSource` and `CalendarEvent` read contract in `src/backend/handshake_core/src/storage/calendar.rs`, and extend the SQLite/Postgres row mappers to decode them.
2. Add those same five fields to every calendar-source and calendar-event `RETURNING` / read projection on both backends: source upsert/list/get plus event upsert and `query_calendar_events`.
3. Replace the stale packet evidence anchors at `.GOV/task_packets/WP-1-Calendar-Storage-v2/packet.md:1021-1027` with the real calendar-specific line references after the product fix lands.
4. Re-run calendar storage proof with real calendar provenance assertions on both HUMAN and AI/workflow contexts across upsert `RETURNING`, get, list, and query read paths, then re-enter final-lane validation.

### 2026-04-13 INTEGRATION VALIDATION REPORT - WP-1-Calendar-Storage-v2 (PASS; Merge Pending)
Verdict: PASS
Validated_at: 2026-04-13T14:07:27.7982077Z
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
MAIN_CONTAINMENT_STATUS: MERGE_PENDING
MERGED_MAIN_COMMIT: NONE
MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Calendar-Storage-v2/packet.md` (status at review time: `In Progress`)
- Reviewed diff: `e1243008365566d4cde3c707f1b6078b5837fdcd..066cc18dcc401d413de5e66073ec84c7a2a0b3db`
- Current main baseline: `e1243008365566d4cde3c707f1b6078b5837fdcd`
- Prepared worktree: `../wtc-calendar-storage-v2`

CLAUSES_REVIEWED:
- `[HSK-CAL-WRITE-GATE] mutation governance` -> `src/backend/handshake_core/src/storage/calendar.rs:142-159`; `src/backend/handshake_core/src/storage/calendar.rs:280-315`; `src/backend/handshake_core/src/storage/sqlite.rs:1042-1078`; `src/backend/handshake_core/src/storage/sqlite.rs:1085-1125`; `src/backend/handshake_core/src/storage/sqlite.rs:3671-3746`; `src/backend/handshake_core/src/storage/sqlite.rs:3818-3879`; `src/backend/handshake_core/src/storage/sqlite.rs:3954-4040`; `src/backend/handshake_core/src/storage/sqlite.rs:4114-4202`; `src/backend/handshake_core/src/storage/sqlite.rs:4289-4293`; `src/backend/handshake_core/src/storage/postgres.rs:1590-1626`; `src/backend/handshake_core/src/storage/postgres.rs:1633-1673`; `src/backend/handshake_core/src/storage/postgres.rs:4117-4192`; `src/backend/handshake_core/src/storage/postgres.rs:4264-4325`; `src/backend/handshake_core/src/storage/postgres.rs:4395-4481`; `src/backend/handshake_core/src/storage/postgres.rs:4555-4643`; `src/backend/handshake_core/src/storage/postgres.rs:4730-4734`; `src/backend/handshake_core/src/storage/tests.rs:2212-2216`; `src/backend/handshake_core/src/storage/tests.rs:2309-2312`; `src/backend/handshake_core/src/storage/tests.rs:2439-2459`; `src/backend/handshake_core/src/storage/tests.rs:2499-2518`
- `temporal invariants (2.1.1)` -> `src/backend/handshake_core/src/storage/calendar.rs:289-295`; `src/backend/handshake_core/src/storage/calendar.rs:330-336`; `src/backend/handshake_core/migrations/0015_calendar_storage.sql:47-53`; `src/backend/handshake_core/src/storage/tests.rs:2355-2376`
- `recurrence invariants (2.1.2)` -> `src/backend/handshake_core/src/storage/calendar.rs:299-305`; `src/backend/handshake_core/src/storage/calendar.rs:339-346`; `src/backend/handshake_core/migrations/0015_calendar_storage.sql:57-63`; `src/backend/handshake_core/src/storage/tests.rs:2250-2251`; `src/backend/handshake_core/src/storage/tests.rs:2287-2288`
- `portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013]` -> `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`; `src/backend/handshake_core/tests/calendar_storage_tests.rs:1-27`; `src/backend/handshake_core/src/storage/tests.rs:2123-2518`
- `CalendarEvent and ActivitySpan join semantics (11.9.3)` -> `src/backend/handshake_core/src/storage/calendar.rs:354-359`; `src/backend/handshake_core/src/storage/sqlite.rs:4300-4302`; `src/backend/handshake_core/src/storage/postgres.rs:4741-4743`; `src/backend/handshake_core/src/storage/tests.rs:2355-2376`

NOT_PROVEN:
- NONE

MAIN_BODY_GAPS:
- NONE

QUALITY_RISKS:
- NONE

VALIDATOR_RISK_TIER: HIGH

DIFF_ATTACK_SURFACES:
- Provenance drift between the signed migration columns, runtime storage models, row mappers, and backend `RETURNING` / `SELECT` projections.
- Dual-backend divergence where SQLite and Postgres expose different source/event row shapes after the same write-context mutation.
- Half-open window-query regression that could silently break downstream correlation consumers while happy-path event upserts still pass.

INDEPENDENT_CHECKS_RUN:
- `just phase-check HANDOFF WP-1-Calendar-Storage-v2 CODER --range e1243008365566d4cde3c707f1b6078b5837fdcd..066cc18dcc401d413de5e66073ec84c7a2a0b3db --verbose` from `../wtc-calendar-storage-v2` => `PASS`
- `git -C ../wtc-calendar-storage-v2 merge-base --is-ancestor e1243008365566d4cde3c707f1b6078b5837fdcd 066cc18dcc401d413de5e66073ec84c7a2a0b3db` => `ANCESTOR_OK`
- `git diff --stat e1243008365566d4cde3c707f1b6078b5837fdcd 066cc18dcc401d413de5e66073ec84c7a2a0b3db` => `4 files changed, 229 insertions(+); scope limited to calendar storage models/backends/tests`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test calendar_storage_tests` from `../wtc-calendar-storage-v2` => `test result: ok. 2 passed; 0 failed`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib storage` from `../wtc-calendar-storage-v2` => `test result: ok. 25 passed; 0 failed`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::sqlite_rejects_ai_writes_without_context_with_hsk_403_silent_edit --lib -- --exact` from `../wtc-calendar-storage-v2` => `test result: ok. 1 passed; 0 failed`

COUNTERFACTUAL_CHECKS:
- If `map_calendar_source_row()` in `src/backend/handshake_core/src/storage/sqlite.rs:1042-1078` or `map_calendar_source()` in `src/backend/handshake_core/src/storage/postgres.rs:1590-1626` stopped decoding `last_job_id`, `last_workflow_id`, `last_actor_id`, `edit_event_id`, and `last_actor_kind`, the AI source round-trip assertions in `src/backend/handshake_core/src/storage/tests.rs:2439-2459` would stop proving governed provenance survives `upsert -> get -> list`.
- If the event query projections in `src/backend/handshake_core/src/storage/sqlite.rs:4289-4293` or `src/backend/handshake_core/src/storage/postgres.rs:4730-4734` dropped any provenance column, the workflow-backed event assertions in `src/backend/handshake_core/src/storage/tests.rs:2513-2518` would fail even though writes still persist metadata.
- If the half-open predicates in `src/backend/handshake_core/src/storage/sqlite.rs:4300-4302` or `src/backend/handshake_core/src/storage/postgres.rs:4741-4743` were widened or inverted, the narrow-window query assertions in `src/backend/handshake_core/src/storage/tests.rs:2367-2376` would stop matching the provider event boundary.

BOUNDARY_PROBES:
- Compared the schema contract in `src/backend/handshake_core/migrations/0015_calendar_storage.sql:29-33` and `src/backend/handshake_core/migrations/0015_calendar_storage.sql:69-73` against the runtime source/event shapes in `src/backend/handshake_core/src/storage/calendar.rs:142-159` and `src/backend/handshake_core/src/storage/calendar.rs:280-315`, then against the SQLite/Postgres row mappers in `src/backend/handshake_core/src/storage/sqlite.rs:1042-1125` and `src/backend/handshake_core/src/storage/postgres.rs:1590-1673`.
- Probed the producer/consumer boundary for both backends by matching source/event upsert/list/get/query projections at `src/backend/handshake_core/src/storage/sqlite.rs:3671-3746`, `src/backend/handshake_core/src/storage/sqlite.rs:3818-3879`, `src/backend/handshake_core/src/storage/sqlite.rs:3954-4040`, `src/backend/handshake_core/src/storage/sqlite.rs:4114-4202`, `src/backend/handshake_core/src/storage/sqlite.rs:4289-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4117-4192`, `src/backend/handshake_core/src/storage/postgres.rs:4264-4325`, `src/backend/handshake_core/src/storage/postgres.rs:4395-4481`, `src/backend/handshake_core/src/storage/postgres.rs:4555-4643`, and `src/backend/handshake_core/src/storage/postgres.rs:4730-4743` to the dual-backend conformance harness in `src/backend/handshake_core/tests/calendar_storage_tests.rs:1-27`.

NEGATIVE_PATH_CHECKS:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::sqlite_rejects_ai_writes_without_context_with_hsk_403_silent_edit --lib -- --exact` from `../wtc-calendar-storage-v2` passed, confirming the governed AI-without-context rejection path still fails closed while the provenance visibility fix is present.
- The migration still enforces `last_actor_kind != 'AI' OR last_job_id IS NOT NULL` at `src/backend/handshake_core/migrations/0015_calendar_storage.sql:33` and `src/backend/handshake_core/migrations/0015_calendar_storage.sql:73`, so the read-surface expansion did not weaken the write gate.

INDEPENDENT_FINDINGS:
- The committed candidate fully closes MT-001 without widening beyond the signed storage surfaces; the reviewed range remains limited to `src/backend/handshake_core/src/storage/calendar.rs`, `src/backend/handshake_core/src/storage/sqlite.rs`, `src/backend/handshake_core/src/storage/postgres.rs`, and `src/backend/handshake_core/src/storage/tests.rs`.
- MT-002 through MT-005 were already aligned at the recorded `main` baseline and remain aligned after the candidate range; the candidate only adds provenance visibility plus one workflow-backed test visibility fix.
- `CURRENT_MAIN_COMPATIBILITY_STATUS=COMPATIBLE` is truthful because local `main` stayed at `e1243008365566d4cde3c707f1b6078b5837fdcd` and the candidate head is a direct descendant of that baseline.

RESIDUAL_UNCERTAINTY:
- This report is diff-scoped and merge-pending. I have not yet recorded a post-merge `main` spotcheck in this appended report; the containment step must still land on local `main` before the packet can truthfully end `Validated (PASS)`.
- Downstream Lens, sync, policy, correlation, and mailbox consumers remain explicitly outside this packet and were not revalidated here; this PASS only covers the signed calendar storage substrate.

SPEC_CLAUSE_MAP:
- `[HSK-CAL-WRITE-GATE] mutation governance` => `src/backend/handshake_core/src/storage/calendar.rs:142-159`; `src/backend/handshake_core/src/storage/calendar.rs:280-315`; `src/backend/handshake_core/src/storage/sqlite.rs:1042-1078`; `src/backend/handshake_core/src/storage/sqlite.rs:1085-1125`; `src/backend/handshake_core/src/storage/sqlite.rs:3671-3746`; `src/backend/handshake_core/src/storage/sqlite.rs:3818-3879`; `src/backend/handshake_core/src/storage/sqlite.rs:3954-4040`; `src/backend/handshake_core/src/storage/sqlite.rs:4114-4202`; `src/backend/handshake_core/src/storage/sqlite.rs:4289-4293`; `src/backend/handshake_core/src/storage/postgres.rs:1590-1626`; `src/backend/handshake_core/src/storage/postgres.rs:1633-1673`; `src/backend/handshake_core/src/storage/postgres.rs:4117-4192`; `src/backend/handshake_core/src/storage/postgres.rs:4264-4325`; `src/backend/handshake_core/src/storage/postgres.rs:4395-4481`; `src/backend/handshake_core/src/storage/postgres.rs:4555-4643`; `src/backend/handshake_core/src/storage/postgres.rs:4730-4734`; `src/backend/handshake_core/src/storage/tests.rs:2212-2216`; `src/backend/handshake_core/src/storage/tests.rs:2309-2312`; `src/backend/handshake_core/src/storage/tests.rs:2439-2459`; `src/backend/handshake_core/src/storage/tests.rs:2499-2518`
- `temporal invariants (2.1.1)` => `src/backend/handshake_core/src/storage/calendar.rs:289-295`; `src/backend/handshake_core/src/storage/calendar.rs:330-336`; `src/backend/handshake_core/migrations/0015_calendar_storage.sql:47-53`; `src/backend/handshake_core/src/storage/tests.rs:2355-2376`
- `recurrence invariants (2.1.2)` => `src/backend/handshake_core/src/storage/calendar.rs:299-305`; `src/backend/handshake_core/src/storage/calendar.rs:339-346`; `src/backend/handshake_core/migrations/0015_calendar_storage.sql:57-63`; `src/backend/handshake_core/src/storage/tests.rs:2250-2251`; `src/backend/handshake_core/src/storage/tests.rs:2287-2288`
- `portable schema and migrations [CX-DBP-011] plus dual-backend testing [CX-DBP-013]` => `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`; `src/backend/handshake_core/tests/calendar_storage_tests.rs:1-27`; `src/backend/handshake_core/src/storage/tests.rs:2123-2518`
- `CalendarEvent and ActivitySpan join semantics (11.9.3)` => `src/backend/handshake_core/src/storage/calendar.rs:354-359`; `src/backend/handshake_core/src/storage/sqlite.rs:4300-4302`; `src/backend/handshake_core/src/storage/postgres.rs:4741-4743`; `src/backend/handshake_core/src/storage/tests.rs:2355-2376`

NEGATIVE_PROOF:
- The validated candidate does not widen the signed storage substrate beyond provenance-read alignment: `src/backend/handshake_core/src/storage/calendar.rs:165-177` and `src/backend/handshake_core/src/storage/calendar.rs:354-359` still expose the same `CalendarSourceUpsert`, `CalendarEventUpsert`, and `CalendarEventWindowQuery` contract shape, while `src/backend/handshake_core/src/storage/sqlite.rs:3739-3743` / `src/backend/handshake_core/src/storage/postgres.rs:4185-4189` only add provenance columns to existing source/event projections rather than introducing new storage operations.

ANTI_VIBE_FINDINGS:
- NONE

SIGNED_SCOPE_DEBT:
- NONE

PRIMITIVE_RETENTION_PROOF:
- `CalendarSourceWritePolicy` and `CalendarSourceSyncState` remain first-class storage primitives at `src/backend/handshake_core/src/storage/calendar.rs:47-70`; `src/backend/handshake_core/src/storage/calendar.rs:123-139`; `src/backend/handshake_core/src/storage/calendar.rs:147-154`; `src/backend/handshake_core/src/storage/calendar.rs:165-178`
- `CalendarEventVisibility`, `CalendarEventExportMode`, and `CalendarEventWindowQuery` remain first-class calendar query primitives at `src/backend/handshake_core/src/storage/calendar.rs:215-271`; `src/backend/handshake_core/src/storage/calendar.rs:297-305`; `src/backend/handshake_core/src/storage/calendar.rs:337-359`

PRIMITIVE_RETENTION_GAPS:
- NONE

SHARED_SURFACE_INTERACTION_CHECKS:
- Verified the shared migration/runtime/test contract across `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`, `src/backend/handshake_core/src/storage/calendar.rs:142-159`, `src/backend/handshake_core/src/storage/calendar.rs:280-359`, `src/backend/handshake_core/src/storage/sqlite.rs:1042-1125`, `src/backend/handshake_core/src/storage/postgres.rs:1590-1673`, `src/backend/handshake_core/src/storage/tests.rs:2123-2518`, and `src/backend/handshake_core/tests/calendar_storage_tests.rs:1-27`.
- Verified both backends preserve the same governed row contract at the storage boundary by comparing source/event `RETURNING` and query projections in `src/backend/handshake_core/src/storage/sqlite.rs:3671-3746`, `src/backend/handshake_core/src/storage/sqlite.rs:3818-3879`, `src/backend/handshake_core/src/storage/sqlite.rs:3954-4040`, `src/backend/handshake_core/src/storage/sqlite.rs:4114-4202`, `src/backend/handshake_core/src/storage/sqlite.rs:4289-4302`, `src/backend/handshake_core/src/storage/postgres.rs:4117-4192`, `src/backend/handshake_core/src/storage/postgres.rs:4264-4325`, `src/backend/handshake_core/src/storage/postgres.rs:4395-4481`, `src/backend/handshake_core/src/storage/postgres.rs:4555-4643`, and `src/backend/handshake_core/src/storage/postgres.rs:4730-4743`.

CURRENT_MAIN_INTERACTION_CHECKS:
- Current `main` compatibility is additive rather than widening: `git -C ../wtc-calendar-storage-v2 merge-base --is-ancestor e1243008365566d4cde3c707f1b6078b5837fdcd 066cc18dcc401d413de5e66073ec84c7a2a0b3db` returned `ANCESTOR_OK`, so the reviewed candidate sits directly on top of the recorded `main` baseline without extra packet scope.
- The current main caller surfaces that downstream code uses are the same source/event entrypoints extended by the candidate: `src/backend/handshake_core/src/storage/sqlite.rs:3789-3842`; `src/backend/handshake_core/src/storage/sqlite.rs:3842-3895`; `src/backend/handshake_core/src/storage/sqlite.rs:4250-4302`; `src/backend/handshake_core/src/storage/postgres.rs:4235-4288`; `src/backend/handshake_core/src/storage/postgres.rs:4288-4340`; `src/backend/handshake_core/src/storage/postgres.rs:4691-4743`. The candidate preserves those function boundaries and only widens the returned row shape with the signed provenance fields.

DATA_CONTRACT_PROOF:
- Reviewed the active calendar data contract across DDL, runtime structs, backend row mappers/projections, and the dual-backend conformance suite at `src/backend/handshake_core/migrations/0015_calendar_storage.sql:1-88`; `src/backend/handshake_core/src/storage/calendar.rs:142-159`; `src/backend/handshake_core/src/storage/calendar.rs:280-359`; `src/backend/handshake_core/src/storage/sqlite.rs:1042-1125`; `src/backend/handshake_core/src/storage/sqlite.rs:3671-3746`; `src/backend/handshake_core/src/storage/sqlite.rs:3818-3879`; `src/backend/handshake_core/src/storage/sqlite.rs:3954-4040`; `src/backend/handshake_core/src/storage/sqlite.rs:4114-4202`; `src/backend/handshake_core/src/storage/sqlite.rs:4289-4302`; `src/backend/handshake_core/src/storage/postgres.rs:1590-1673`; `src/backend/handshake_core/src/storage/postgres.rs:4117-4192`; `src/backend/handshake_core/src/storage/postgres.rs:4264-4325`; `src/backend/handshake_core/src/storage/postgres.rs:4395-4481`; `src/backend/handshake_core/src/storage/postgres.rs:4555-4643`; `src/backend/handshake_core/src/storage/postgres.rs:4730-4743`; `src/backend/handshake_core/src/storage/tests.rs:2123-2518`; `src/backend/handshake_core/tests/calendar_storage_tests.rs:1-27`

DATA_CONTRACT_GAPS:
- NONE
