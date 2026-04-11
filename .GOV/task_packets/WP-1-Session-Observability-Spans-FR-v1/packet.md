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

# Task Packet: WP-1-Session-Observability-Spans-FR-v1

## METADATA
- TASK_ID: WP-1-Session-Observability-Spans-FR-v1
- WP_ID: WP-1-Session-Observability-Spans-FR-v1
- BASE_WP_ID: WP-1-Session-Observability-Spans-FR
- DATE: 2026-04-08T22:46:49.759Z
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
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
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
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Session-Observability-Spans-FR-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Session-Observability-Spans-FR-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-spans-fr-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Session-Observability-Spans-FR-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Session-Observability-Spans-FR-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Session-Observability-Spans-FR-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Session-Observability-Spans-FR-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Session-Observability-Spans-FR-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Session-Observability-Spans-FR-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Session-Observability-Spans-FR-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Session-Observability-Spans-FR-v1
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
- MERGED_MAIN_COMMIT: a42b682d446ce602d44a6fde6d25a801fcdbbe33
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-09T13:20:25.815Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: a42b682d446ce602d44a6fde6d25a801fcdbbe33
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-09T13:20:25.815Z
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
- BUILD_ORDER_DEPENDS_ON: WP-1-FR-ModelSessionId, WP-1-ModelSession-Core-Scheduler, WP-1-Session-Spawn-Contract
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-Control-Plane-Backend
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Session-Observability-Spans-FR-v1
- LOCAL_WORKTREE_DIR: ../wtc-spans-fr-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Session-Observability-Spans-FR-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Session-Observability-Spans-FR-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Session-Observability-Spans-FR-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: WP_VALIDATOR:WP-1-Session-Observability-Spans-FR-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-session-observability-spans-fr-v1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja090420260043
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: ORCHESTRATOR advances verdict progression and integration closeout from the authoritative completed direct-review lane.
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: 4.3.9.18.2 Span Binding (Normative) | CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs` | TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`, A `model_run` row within that session emits an `activity_span_id` nested under the session span, A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree, `session.completed` totals align with runtime token/cost aggregation for that session | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 4.3.9.18.4 Flight Recorder Events (Session Lifecycle) | CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/workflows.rs` | TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`, A `model_run` row within that session emits an `activity_span_id` nested under the session span, A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree, `session.completed` totals align with runtime token/cost aggregation for that session | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 11.5 schema registry must include subsystem event families | CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/flight_recorder/duckdb.rs` | TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`, A `model_run` row within that session emits an `activity_span_id` nested under the session span, A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree, `session.completed` totals align with runtime token/cost aggregation for that session | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 11.9.1.X Session-Scoped Observability Requirements | CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs` | TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml` | EXAMPLES: Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`, A `model_run` row within that session emits an `activity_span_id` nested under the session span, A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree, `session.completed` totals align with runtime token/cost aggregation for that session | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test model_run_spawn_announce_back_event_is_emitted_for_parented_completion --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just gov-check`
- CANONICAL_CONTRACT_EXAMPLES:
  - Root session creation emits `session.created` with `model_session_id` and stable `session_span_id`
  - A `model_run` row within that session emits an `activity_span_id` nested under the session span
  - A nested tool call reuses the session id and carries a child `activity_span_id` rather than dropping out of the span tree
  - `session.completed` totals align with runtime token/cost aggregation for that session
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: session-scoped observability query substrate | SUBFEATURES: model_session_id plus span ids in recorder rows, filterable session timelines, stable nested identifiers for debugging and replay-adjacent analysis | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightEvent, PRIM-ActivitySpanBinding | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: makes session telemetry structurally useful to both operators and local/cloud model workflows
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: session lifecycle Flight Recorder events | JobModel: WORKFLOW | Workflow: model_session_lifecycle | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-SESS-001..005 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: lifecycle creation/state/complete/message/budget events become first-class recorder rows for every ModelSession
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: ModelSessionSpan plus model_run/tool ActivitySpan binding | JobModel: AI_JOB | Workflow: model_run execution plus nested tool calls | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: model_session_id plus session_span_id/activity_span_id on session/model/tool rows | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: existing builder/storage fields are finally driven by live runtime binding
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: session-scoped timeline/cost query substrate | JobModel: NONE | Workflow: flight_recorder session timeline query | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: query by model_session_id with nested span structure available for reconstruction | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend query substrate only; DCC rendering is explicitly downstream
  - FORCE_MULTIPLIER_EXPANSION: model_run event emission + nested tool-call activity spans + existing `model_session_id` correlation -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: spawn/announce-back session links + session span correlation ids + existing scheduler/spawn event families -> IN_THIS_WP (stub: NONE)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Session-Observability-Spans-FR-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 4.3.9.18.2 Span Binding (Normative)
- CONTEXT_START_LINE: 32710
- CONTEXT_END_LINE: 32734
- CONTEXT_TOKEN: ModelSessionSpanBinding
- EXCERPT_ASCII_ESCAPED:
  ```text
##### 4.3.9.18.2 Span Binding (Normative)

  - Every `ModelSession` MUST create a `ModelSessionSpan` at session creation and close it at session completion/cancellation.
  - Every `model_run` job within a session MUST create an `ActivitySpan` nested under the session's `ModelSessionSpan`.
  - Tool calls within a model run MUST create child `ActivitySpan`s under the model run span.

  ```yaml
  # ADD v02.137
  ModelSessionSpanBinding:
    session_id: string
    model_session_span_id: string
    parent_model_session_span_id: string | null  # null for root sessions; parent session's span for children

  ActivitySpanBinding:
    activity_span_id: string
    model_session_span_id: string
    job_id: string
    model_id: ModelId
    start_time: string
    end_time: string | null
    token_count: int | null
    cost_usd: number | null
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 4.3.9.18.4 Flight Recorder Events (Session Lifecycle)
- CONTEXT_START_LINE: 32745
- CONTEXT_END_LINE: 32769
- CONTEXT_TOKEN: FR-EVT-SESS-001
- EXCERPT_ASCII_ESCAPED:
  ```text
##### 4.3.9.18.4 Flight Recorder Events (Session Lifecycle)

  ```yaml
  # ADD v02.137
  FR-EVT-SESS-001:
    event_type: "session.created"
    payload: { session_id, model_id, backend, role, wp_id, mt_id, memory_policy, spawn_depth }

  FR-EVT-SESS-002:
    event_type: "session.state_change"
    payload: { session_id, from_state, to_state, reason }

  FR-EVT-SESS-003:
    event_type: "session.completed"
    payload: { session_id, total_tokens, total_cost_usd, duration_ms, messages_count }

  FR-EVT-SESS-004:
    event_type: "session.message"
    payload: { session_id, message_id, role, content_hash, token_count }
    # NOTE: content is stored as artifact; event carries hash only (INV-SESS-002)

  FR-EVT-SESS-005:
    event_type: "session.budget_warning"
    payload: { session_id, budget_type, current_value, threshold_value }
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.5 Schema Registry Event Family Coverage
- CONTEXT_START_LINE: 66867
- CONTEXT_END_LINE: 66881
- CONTEXT_TOKEN: Schema registry must include subsystem event families
- EXCERPT_ASCII_ESCAPED:
  ```text
- **Schema registry must include subsystem event families:** The Flight Recorder schema validator MUST include and validate Locus event families defined in \u00a72.3.15.6 (FR-EVT-WP-*, FR-EVT-MT-*, FR-EVT-DEP-*, FR-EVT-TB-*, FR-EVT-SYNC-*, FR-EVT-QUERY-*), and Multi-Session event families defined in \u00a74.3.9.13/\u00a74.3.9.15/\u00a74.3.9.18 (FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*). Unknown event IDs MUST be rejected.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.9.1.X Session-Scoped Observability Requirements [ADD v02.137]
- CONTEXT_START_LINE: 70713
- CONTEXT_END_LINE: 70731
- CONTEXT_TOKEN: Timeline view:
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 11.9.1.X Session-Scoped Observability Requirements [ADD v02.137]

  ##### 11.9.1.X.1 ModelSessionSpan Binding to ModelSession

  Every `ModelSession` (\u00a74.3.9.12) MUST be associated with exactly one `ModelSessionSpan` (distinct from the operator `SessionSpan`). The span:
  - opens at session creation,
  - closes at session completion/cancellation/failure,
  - contains all `ActivitySpan`s for `model_run` jobs and tool calls within the session.

  ##### 11.9.1.X.2 Cross-Session Correlation

  When sessions communicate (announce-back, Role Mailbox), the spans MUST carry correlation IDs so that the full conversation across parent and child sessions can be reconstructed in a timeline view.

  ##### 11.9.1.X.3 UI Surface Requirements

  - **Timeline view:** Sessions displayed as horizontal swim-lanes with nested activity spans.
  - **Cost overlay:** Token cost per span, aggregated per session.
  - **Filter:** By session_id, role, model, WP, time range.
  - **Deep-link:** From any span \u2192 Flight Recorder events \u2192 artifacts \u2192 session message thread.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: 4.3.9.18.2 Span Binding (Normative) | WHY_IN_SCOPE: this packet exists to bind live `ModelSession` / `model_run` / tool execution to session and activity spans | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs` | EXPECTED_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: session and activity spans remain decorative fields with no runtime truth
  - CLAUSE: 4.3.9.18.4 Flight Recorder Events (Session Lifecycle) | WHY_IN_SCOPE: the spec-mandated lifecycle family is still absent in product code | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/workflows.rs` | EXPECTED_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: DCC/backend consumers get no canonical session creation, transition, completion, message, or budget-warning rows
  - CLAUSE: 11.5 schema registry must include subsystem event families | WHY_IN_SCOPE: any newly added lifecycle rows must remain part of the canonical schema validation surface | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/flight_recorder/duckdb.rs` | EXPECTED_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: unknown or malformed lifecycle events can drift silently into persisted history
  - CLAUSE: 11.9.1.X Session-Scoped Observability Requirements | WHY_IN_SCOPE: this packet is the backend substrate that makes timeline, cost, filter, and deep-link reconstruction possible | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs` | EXPECTED_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml` | RISK_IF_MISSED: session-scoped observability remains incomplete even if individual rows exist
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `FR-EVT-SESS-001..005` event family registration and payload schemas | PRODUCER: workflow runtime plus Flight Recorder event constructors | CONSUMER: recorder schema validator, DuckDB persistence, API/query readers, downstream DCC backend | SERIALIZER_TRANSPORT: canonical `FlightRecorderEvent` envelope persisted through DuckDB | VALIDATOR_READER: schema validation and session lifecycle tests | TRIPWIRE_TESTS: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: lifecycle rows exist in one layer but not another, causing silent replay/query breakage
  - CONTRACT: ModelSessionSpanBinding / ActivitySpanBinding runtime semantics | PRODUCER: workflow/session execution helpers | CONSUMER: Flight Recorder event stamping, DuckDB row consumers, downstream session timeline reconstruction | SERIALIZER_TRANSPORT: runtime-bound ids copied into `session_span_id` / `activity_span_id` event fields | VALIDATOR_READER: span propagation tests and code inspection | TRIPWIRE_TESTS: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: spans are generated inconsistently or rebound per event, breaking hierarchy
  - CONTRACT: `model_session_id` plus session/activity span propagation across session/model/tool events | PRODUCER: workflow runtime emitters | CONSUMER: query filters, validators, DCC backend, operators | SERIALIZER_TRANSPORT: `FlightRecorderEventBase` stored in DuckDB and exposed through API filters | VALIDATOR_READER: existing `model_session_id` tripwires plus new span tests | TRIPWIRE_TESTS: `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`, `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` | DRIFT_RISK: correlation keys drift between event families and break cross-row session reconstruction
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Add the missing lifecycle event family (`FR-EVT-SESS-001..005`) to the existing Flight Recorder schema/validation surface in `flight_recorder/mod.rs`.
  - Introduce or wire the runtime session/activity span binding substrate in the existing workflow execution path; reuse current builder hooks instead of creating a second observability layer.
  - Extend storage/query proof and scheduler-session tests so lifecycle emission and span propagation are covered without weakening existing `model_session_id` or spawn/scheduler tripwires.
- HOT_FILES:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- TRIPWIRE_TESTS:
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
- CARRY_FORWARD_WARNINGS:
  - Do not replace `model_session_id` with span ids; it remains the primary session-wide correlation key.
  - Do not add a new observability store, sidecar pipeline, or duplicate runtime truth surface.
  - Do not regress the already-landed scheduler/spawn/model_session_id foundations while wiring lifecycle and span telemetry.
  - Prefer extending the existing recorder/runtime/test surfaces over adding new narrow checks or helper commands unless a real blind spot remains.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Confirm `FR-EVT-SESS-001..005` exists in the canonical schema/validation surface and is actually emitted by workflow/session runtime paths.
  - Confirm every lifecycle row carries `model_session_id` and coherent `session_span_id` values.
  - Confirm model runs and nested tool calls stamp `activity_span_id` consistently under the correct session span.
  - Confirm the existing DuckDB/query/API surfaces remain the single backend truth for session reconstruction.
- FILES_TO_READ:
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- COMMANDS_TO_RUN:
  - `rg -n "session.created|session.state_change|session.completed|session.message|session.budget_warning|with_activity_span|with_session_span|model_session_id" src/backend/handshake_core/src/flight_recorder/mod.rs src/backend/handshake_core/src/flight_recorder/duckdb.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Confirm the downstream DCC backend packet can query session timelines without asking for a second recorder surface.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The exact in-memory persistence shape for span bindings is not proven yet; it may live as explicit typed records or equivalent runtime state as long as the emitted Flight Recorder semantics remain identical and there is no second truth store.
  - Downstream DCC rendering, swim-lane layout, and deep-link UX remain unproven until the control-plane backend and UI packets consume this substrate.
  - Cross-session mailbox/announce-back correlation beyond the current packet scope is not fully proven at refinement time.
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
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.context
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Command Center
  - Execution / Job Runtime
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - ModelSession lifecycle events + ModelSessionSpan binding + session cost aggregation -> IN_THIS_WP (stub: NONE)
  - model_run event emission + nested tool-call activity spans + existing `model_session_id` correlation -> IN_THIS_WP (stub: NONE)
  - session timeline query substrate + DCC control-plane backend consumer -> IN_THIS_WP (stub: NONE)
  - lifecycle budget warnings + session completion totals + existing runtime token/cost accounting -> IN_THIS_WP (stub: NONE)
  - spawn/announce-back session links + session span correlation ids + existing scheduler/spawn event families -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: session lifecycle event family | SUBFEATURES: `session.created`, `session.state_change`, `session.completed`, `session.message`, `session.budget_warning` payload validation and emission | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightEvent, PRIM-ModelSession | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: fills the still-missing `FR-EVT-SESS-001..005` family on top of the existing scheduler/spawn foundation
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: ModelSessionSpan and ActivitySpan runtime binding | SUBFEATURES: open/close session span, bind model_run activity span, bind nested tool-call child spans, reuse ids across emitted events | PRIMITIVES_FEATURES: PRIM-ModelSession, PRIM-ModelSessionSpanBinding, PRIM-ActivitySpanBinding, PRIM-SessionRegistry | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: binds the live workflow runtime to the span contract already declared in spec
  - PILLAR: Command Center | CAPABILITY_SLICE: backend timeline/cost projection inputs | SUBFEATURES: session timeline reconstruction inputs, per-session aggregate cost payloads, deep-linkable span ids | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-ModelSessionSpanBinding | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: backend projection surface only; DCC rendering remains downstream
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: session-scoped observability query substrate | SUBFEATURES: model_session_id plus span ids in recorder rows, filterable session timelines, stable nested identifiers for debugging and replay-adjacent analysis | PRIMITIVES_FEATURES: PRIM-FlightRecorderEventBase, PRIM-FlightEvent, PRIM-ActivitySpanBinding | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: makes session telemetry structurally useful to both operators and local/cloud model workflows
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: session lifecycle Flight Recorder events | JobModel: WORKFLOW | Workflow: model_session_lifecycle | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: FR-EVT-SESS-001..005 | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: lifecycle creation/state/complete/message/budget events become first-class recorder rows for every ModelSession
  - Capability: ModelSessionSpan plus model_run/tool ActivitySpan binding | JobModel: AI_JOB | Workflow: model_run execution plus nested tool calls | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: model_session_id plus session_span_id/activity_span_id on session/model/tool rows | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: existing builder/storage fields are finally driven by live runtime binding
  - Capability: session-scoped timeline/cost query substrate | JobModel: NONE | Workflow: flight_recorder session timeline query | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: PLANNED | FlightRecorder: query by model_session_id with nested span structure available for reconstruction | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: backend query substrate only; DCC rendering is explicitly downstream
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Session-Observability-Spans-FR-v1 -> EXPAND_IN_THIS_WP
  - WP-1-FR-ModelSessionId-v1 -> KEEP_SEPARATE
  - WP-1-ModelSession-Core-Scheduler-v1 -> KEEP_SEPARATE
  - WP-1-Session-Spawn-Contract-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs -> PARTIAL (WP-1-FR-ModelSessionId-v1)
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs -> IMPLEMENTED (WP-1-FR-ModelSessionId-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs -> PARTIAL (WP-1-ModelSession-Core-Scheduler-v1)
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
- What: Implement the missing session lifecycle Flight Recorder family and wire live `ModelSessionSpan` / `ActivitySpan` binding through the existing workflow runtime. Reuse the current recorder, DuckDB store, and query API so session timelines can be reconstructed from canonical FR rows.
- Why: The refactor foundations already landed `model_session_id`, scheduler events, and spawn events, but the spec-required lifecycle/span observability contract is still missing in product code. DCC backend work depends on this substrate being real rather than implied.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- OUT_OF_SCOPE:
  - DCC rendering, swim-lane UI, and operator-facing controls
  - Stage or Calendar span projections
  - replay/audit features beyond the existing Flight Recorder truth surface
  - any new observability store, sidecar pipeline, or appendix/spec expansion
- TOUCHED_FILE_BUDGET: 6
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- WAIVER_ID: CX-573F-20260409-SPANS-FR-TOKEN-BUDGET-CONTINUATION | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED / POLICY_CONFLICT on WP-1-Session-Observability-Spans-FR-v1 during crash recovery and governance repair | JUSTIFICATION: Operator explicitly instructed autonomous continuation until this WP is validated, governance is repaired in place, and the closure is integrated into main after the prior orchestrator-managed run was interrupted by process loss and had already exceeded the governed token budget. This waiver authorizes bounded continuation without pretending the budget overrun did not occur. | APPROVER: Operator (chat, 2026-04-09) | EXPIRES: when WP-1-Session-Observability-Spans-FR-v1 reaches an honest closeout verdict

## QUALITY_GATE
### TEST_PLAN
```bash
rg -n "session.created|session.state_change|session.completed|session.message|session.budget_warning|with_activity_span|with_session_span|model_session_id" src/backend/handshake_core/src/flight_recorder/mod.rs src/backend/handshake_core/src/flight_recorder/duckdb.rs src/backend/handshake_core/src/workflows.rs src/backend/handshake_core/tests/model_session_scheduler_tests.rs
  cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test model_run_spawn_announce_back_event_is_emitted_for_parented_completion --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
```

### DONE_MEANS
- `FR-EVT-SESS-001..005` is registered, shape-validated, and emitted through the existing Flight Recorder surface
- session lifecycle rows carry `model_session_id` and stable `session_span_id`
- model runs and nested tool calls stamp coherent `activity_span_id` values under the correct session span
- the existing DuckDB/query/API substrate remains the single backend truth for session timeline reconstruction
- tests cover lifecycle emission and span propagation without regressing the adjacent scheduler/spawn/model_session_id foundations

- PRIMITIVES_EXPOSED:
  - PRIM-ModelSession
  - PRIM-ModelSessionSpanBinding
  - PRIM-ActivitySpanBinding
  - PRIM-FlightRecorderEventBase
  - PRIM-FlightEvent
  - PRIM-SessionRegistry
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-08T22:46:49.759Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md 4.3.9.18 Session Observability: ActivitySpan and ModelSessionSpan Binding (Normative) [ADD v02.137]
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
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
  - .GOV/task_packets/stubs/WP-1-Session-Observability-Spans-FR-v1.md
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- SEARCH_TERMS:
  - session.created
  - session.state_change
  - session.completed
  - session.message
  - session.budget_warning
  - with_activity_span
  - with_session_span
  - model_session_id
- RUN_COMMANDS:
  ```bash
just phase-check STARTUP WP-1-Session-Observability-Spans-FR-v1 CODER
  just launch-coder WP-1-Session-Observability-Spans-FR-v1
  just session-send CODER WP-1-Session-Observability-Spans-FR-v1 "Implement the packet exactly as written; extend existing Flight Recorder/runtime surfaces and avoid introducing a second observability store."
  just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER
  just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 WP_VALIDATOR
  ```
- RISK_MAP:
  - "Lifecycle rows emit without stable session span ids" -> "session timeline becomes fragmented and downstream DCC consumers cannot reconstruct the session correctly"
  - "Nested tool-call paths skip activity span propagation" -> "child work disappears from observability and cost/debug traces become misleading"
  - "Span binding lands outside the existing recorder/query path" -> "truth drifts between runtime state and persisted Flight Recorder evidence"
  - "Adding spans regresses the `model_session_id` hard rule" -> "existing session-scoped queries and validation tripwires fail"
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
- Ran: `just check-notifications WP-1-Session-Observability-Spans-FR-v1 CODER CODER:WP-1-Session-Observability-Spans-FR-v1`.
- Ran: `just ack-notifications WP-1-Session-Observability-Spans-FR-v1 CODER CODER:WP-1-Session-Observability-Spans-FR-v1`.
- Ran: `just phase-check STARTUP WP-1-Session-Observability-Spans-FR-v1 CODER`.
- Ran: `just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER --range bf3e7f81..65cf306c` to evaluate only the committed validator-repair diff.
- Kept unrelated governance drift (`justfile`, `.GOV/docs_repo/`) out of WP proof by using the explicit committed range instead of the dirty worktree.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/api/flight_recorder.rs`
- **Start**: 48
- **End**: 397
- **Line Delta**: 127
- **Pre-SHA1**: `787ad782b29bcb62be07a75b565feecfd2269cfe`
- **Post-SHA1**: `466ea3731839025d215ad4d66986123dd9326221`
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
- **Start**: 250
- **End**: 955
- **Line Delta**: 28
- **Pre-SHA1**: `b28ae647329979ee53d1285c48d4b49e74d9e9be`
- **Post-SHA1**: `0e934418a0ea1ebcfdac5ff0a4347455b36b22c9`
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
- **Start**: 161
- **End**: 5965
- **Line Delta**: 404
- **Pre-SHA1**: `11fc877b0073c0ef6147c00377e72debf445009b`
- **Post-SHA1**: `da393de41df525fcc0021a757bb28b3a20d12ce0`
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
- **Target File**: `src/backend/handshake_core/src/mcp/gate.rs`
- **Start**: 17
- **End**: 1896
- **Line Delta**: 26
- **Pre-SHA1**: `7dfdca609427853c2640e83c44fc988049cad11f`
- **Post-SHA1**: `8df7555aef0238e5d2fc47b599f4824200d2bd74`
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
- **Start**: 28
- **End**: 6903
- **Line Delta**: 354
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `fcc32d5f94c0315c4e6c7bd7e21481a05b45bf51`
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
- **Target File**: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
- **Start**: 20
- **End**: 1489
- **Line Delta**: 484
- **Pre-SHA1**: `7a773197bbfaf518d942fbb77f0d50ded4da9576`
- **Post-SHA1**: `f5f8743c4839d63e808a29983b1e7d0aebc52154`
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
- **Lint Results**:
- Not run; packet proof is anchored on the targeted cargo tests plus the committed-range handoff gate.
- **Artifacts**: `../wt-gov-kernel/.GOV/task_packets/WP-1-Session-Observability-Spans-FR-v1/signed-scope.patch`
- `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-startup/WP-1-Session-Observability-Spans-FR-v1/2026-04-09T02-28-40-151Z.log`
- `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Session-Observability-Spans-FR-v1/2026-04-09T08-12-40-973Z.log`
- **Timestamp**: 2026-04-09T08:12:41.0015242Z
- **Operator**: N/A
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- **Notes**:
- Manifest refreshed for committed validator-repair range `bf3e7f81..65cf306c`; rerun the handoff gate against this committed slice.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: In Progress (validator-review repairs committed at `65cf306c` on top of `4ba26a4`; packet proof refreshed for rehandoff).
- What changed in this update: Repaired the validator findings by moving `finalize_model_run_after_terminal` ahead of terminal job publication, preserving `model_session_id` through the API filter/serialization surface, and switching `session.budget_warning` to session-total token accounting while keeping the previously committed lifecycle/span implementation intact.
- Requirements / clauses self-audited: `4.3.9.18.2`, `4.3.9.18.4`, `11.5`, and `11.9.1.X`, plus each signed `DONE_MEANS` bullet in this packet.
- Checks actually run: `cargo test list_events_preserves_model_session_id_filter_and_payload --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`; `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests`; `just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER --range bf3e7f81..65cf306c`.
- Known gaps / weak spots: I did not rerun the full packet `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` matrix in this repair lane; the strongest proof is focused on the validator-reported lifecycle ordering, API `model_session_id` passthrough, session-total budget accounting, and one nested MCP tool-call session path.
- Heuristic risks / maintainability concerns: `mcp/gate.rs` derives `parent_span_id` from `job_id` when present, so future tool-call contexts that bypass a model-run `job_id` will not inherit the same parent-child span relationship automatically.
- Validator focus request: Recheck the committed-range manifest against `bf3e7f81..65cf306c` and pay extra attention to terminal lifecycle ordering in `workflows.rs`, API `model_session_id` passthrough in `api/flight_recorder.rs`, session-total budget warning accounting in `workflows.rs`, and the MCP child-span propagation in `mcp/gate.rs`.
- Rubric contract understanding proof: This handoff records implementation facts, raw proof, and self-critique only; it does not claim a validator verdict and it scopes review to the committed WP diff instead of the dirty governance worktree.
- Rubric scope discipline proof: The product diff stays inside the six signed paths and does not introduce a second observability store, UI work, or spec widening.
- Rubric baseline comparison: Before this repair commit, the validator could still catch terminal-state publication before `session.completed`, the API adapter dropped `model_session_id`, and `session.budget_warning` reflected only the latest assistant message. After `65cf306c`, those review findings align with the same backend truth path already established in the earlier lifecycle/span work.
- Rubric end-to-end proof: The focused scheduler test drives a model session to completion, performs a nested MCP tool call, and now also proves that `session.completed` and `session.budget_warning` reflect the full session totals while `model_session_id`, `session_span_id`, `activity_span_id`, and `parent_span_id` stay coherent across the same timeline.
- Rubric architecture fit self-review: The change extends the existing Flight Recorder, DuckDB, workflow, and MCP gate surfaces instead of layering a parallel observability subsystem.
- Rubric heuristic quality self-review: The most credible proof is the combination of schema validation, the API filter regression test, and end-to-end query assertions; the weakest area is still that the tool-call proof exercises one representative MCP path rather than every error variant.
- Rubric anti-gaming / counterfactual check: If the workflow still marked the job terminal before `finalize_model_run_after_terminal`, or if the API adapter still forced `model_session_id: None`, the refreshed focused tests would not be satisfiable by superficial event emission alone.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: - NONE
- Signed-scope debt ledger: - NONE
- Data contract self-check: No new external API or second storage contract was added; `model_session_id`, `session_span_id`, and `activity_span_id` stay on the existing recorder, DuckDB, and `EventFilter` path.
- Next step / handoff hint: Review repair commit `65cf306c` on top of `4ba26a4` against committed range `bf3e7f81..65cf306c`, with validator focus on lifecycle ordering, API `model_session_id` passthrough, and session-total budget accounting.

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
  - REQUIREMENT: "FR-EVT-SESS-001..005 is registered, shape-validated, and emitted through the existing Flight Recorder surface"
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:162`; `src/backend/handshake_core/src/workflows.rs:2120`; `src/backend/handshake_core/src/flight_recorder/mod.rs:5787`
  - REQUIREMENT: "session lifecycle rows carry model_session_id and stable session_span_id through recorder, persistence, and query reconstruction"
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:364`; `src/backend/handshake_core/src/workflows.rs:2120`; `src/backend/handshake_core/src/flight_recorder/duckdb.rs:920`
  - REQUIREMENT: "model runs and nested tool calls stamp coherent activity_span_id values and terminal lifecycle ordering remains observable in the session timeline"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2115`; `src/backend/handshake_core/src/workflows.rs:6368`; `src/backend/handshake_core/src/mcp/gate.rs:336`; `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1086`
  - REQUIREMENT: "the API query surface preserves model_session_id filtering and serialization on top of the existing backend truth path"
  - EVIDENCE: `src/backend/handshake_core/src/api/flight_recorder.rs:48`; `src/backend/handshake_core/src/api/flight_recorder.rs:180`; `src/backend/handshake_core/src/api/flight_recorder.rs:344`
  - REQUIREMENT: "tests cover lifecycle emission, API passthrough, and session-total budget/span propagation without regressing adjacent scheduler foundations"
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:5787`; `src/backend/handshake_core/src/api/flight_recorder.rs:344`; `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1086`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `just phase-check STARTUP WP-1-Session-Observability-Spans-FR-v1 CODER`
  - EXIT_CODE: 0
  - LOG_PATH: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-startup/WP-1-Session-Observability-Spans-FR-v1/2026-04-09T02-28-40-151Z.log`
  - PROOF_LINES: `RESULT: PASS`; `WHY: STARTUP phase checks passed.`
- COMMAND: `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- START_UTC: `2026-04-09T08:09:36.6342309Z`
- END_UTC: `2026-04-09T08:09:38.1088608Z`
- PROOF_LINES: `test flight_recorder::tests::session_lifecycle_event_payloads_are_validated ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.00s`
- COMMAND: `cargo test list_events_preserves_model_session_id_filter_and_payload --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- START_UTC: `2026-04-09T08:09:27.9901968Z`
- END_UTC: `2026-04-09T08:09:32.1236966Z`
- PROOF_LINES: `test api::flight_recorder::tests::list_events_preserves_model_session_id_filter_and_payload ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.29s`
- COMMAND: `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml`
- EXIT_CODE: 0
- START_UTC: `2026-04-09T08:09:42.7533715Z`
- END_UTC: `2026-04-09T08:09:43.8086557Z`
- PROOF_LINES: `test session_scheduler_event_payloads_are_validated ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s`
- COMMAND: `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests`
- EXIT_CODE: 0
- START_UTC: `2026-04-09T08:09:48.9623479Z`
- END_UTC: `2026-04-09T08:09:50.0554398Z`
- PROOF_LINES: `test session_observability_spans_bind_model_runs_and_tool_calls ... ok`; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.49s`
- COMMAND: `just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER --range bf3e7f81..65cf306c`
- EXIT_CODE: 0
- START_UTC: `2026-04-09T08:12:32.8756613Z`
- END_UTC: `2026-04-09T08:12:41.0015242Z`
- LOG_PATH: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Session-Observability-Spans-FR-v1/2026-04-09T08-12-40-973Z.log`
- PROOF_LINES: `RESULT: PASS`; `WHY: HANDOFF phase checks passed.`

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

### Integration Validator Report (Merge-Pending Closeout)
DATE: 2026-04-09T11:14:30Z
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
VALIDATOR_MODEL: gpt-5.4
COMMIT: 65cf306c
COMMITTED_RANGE: bf3e7f81..65cf306c
GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
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
VALIDATOR_RISK_TIER: HIGH
Verdict: PASS

CLAUSES_REVIEWED:
  - 4.3.9.18.2 Span Binding (Normative) => `src/backend/handshake_core/src/workflows.rs:2117-2129`, `src/backend/handshake_core/src/mcp/gate.rs:335-352`, and `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1086-1398` prove root-session span stamping, model-run activity span binding, and nested tool-call child activity spans.
  - 4.3.9.18.4 Flight Recorder Events (Session Lifecycle) => `src/backend/handshake_core/src/workflows.rs:2141-2298` emits `session.created`, `session.completed`, and `session.budget_warning` through the canonical recorder path, and `src/backend/handshake_core/src/flight_recorder/mod.rs:5787` validates the lifecycle payload family.
  - 11.5 schema registry must include subsystem event families => `src/backend/handshake_core/src/flight_recorder/mod.rs:364-432` exposes `model_session_id`, `activity_span_id`, and `session_span_id`, while `src/backend/handshake_core/src/flight_recorder/duckdb.rs:250-288` and `:563-724` persist and reconstruct the same family without introducing a parallel observability store.
  - 11.9.1.X Session-Scoped Observability Requirements => `src/backend/handshake_core/src/flight_recorder/duckdb.rs:663-665` filters by `model_session_id`, `src/backend/handshake_core/src/api/flight_recorder.rs:182-188` and `:253-263` preserve filter and payload projection, and `:344-392` proves the API round-trip on the repaired slice.

NOT_PROVEN:
  - NONE

MAIN_BODY_GAPS:
  - NONE

QUALITY_RISKS:
  - NONE

DIFF_ATTACK_SURFACES:
  - `run_and_finalize_workflow_job` plus `finalize_model_run_after_terminal` at `src/backend/handshake_core/src/workflows.rs:6813-6881` is ordering-sensitive: if terminal state becomes externally observable before the final lifecycle emitter runs, `session.completed` can disappear from the session timeline.
  - `src/backend/handshake_core/src/api/flight_recorder.rs:58-62`, `:182-188`, and `:253-263` is the only API adapter path for `model_session_id`; dropping either the filter or projection quietly breaks session-scoped query fidelity.
  - `src/backend/handshake_core/src/mcp/gate.rs:335-352` is the tool-call producer boundary for child activity spans; losing either the tool span or parent model-run span would flatten the session tree.
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs:679-727` remains column-position-sensitive for `model_session_id`, `activity_span_id`, and `session_span_id`; a mismatched select/order mapping would corrupt reconstructed timelines.

INDEPENDENT_CHECKS_RUN:
  - `cargo test list_events_preserves_model_session_id_filter_and_payload --manifest-path src/backend/handshake_core/Cargo.toml` => PASS on the repaired slice; API filter and payload passthrough preserved.
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` => PASS; canonical lifecycle payload family remains schema-valid.
  - `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` => PASS; adjacent scheduler payload family remains intact.
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests` => PASS; end-to-end session span, activity span, and budget-warning semantics are green.
  - Integration final-lane review on `handshake_main` confirmed committed target visibility and no current-main compatibility drift for `bf3e7f81..65cf306c`.

COUNTERFACTUAL_CHECKS:
  - If `finalize_model_run_after_terminal` at `src/backend/handshake_core/src/workflows.rs:6368` stopped emitting the final lifecycle rows before `run_and_finalize_workflow_job` exposes terminal job state at `:6813-6881`, the session timeline assertion at `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1321` would stop finding `session.completed`.
  - If `model_session_id` stopped flowing through `src/backend/handshake_core/src/api/flight_recorder.rs:188` or `:263`, the regression test at `:344-392` would fail because the filtered query would either return the wrong row set or drop the session identifier from the response.
  - If `tool_call_activity_span_id` or the parent `model_run_activity_span_id` stopped being applied in `src/backend/handshake_core/src/mcp/gate.rs:335-352`, the nested tool-call assertions at `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1388-1398` would fail.

BOUNDARY_PROBES:
  - Producer/consumer chain check: `src/backend/handshake_core/src/workflows.rs:2141-2298` emits lifecycle rows, `src/backend/handshake_core/src/flight_recorder/mod.rs:5787` validates them, `src/backend/handshake_core/src/flight_recorder/duckdb.rs:563-724` stores/loads them, and `src/backend/handshake_core/src/api/flight_recorder.rs:253-263` exposes them without dropping session/span identifiers.
  - Runtime/tool boundary check: `src/backend/handshake_core/src/mcp/gate.rs:335-352` records tool-call spans using the same canonical span helpers declared in `src/backend/handshake_core/src/flight_recorder/mod.rs:921-929`, so tool events stay inside the session tree rather than creating a side channel.

NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1346-1375` now proves `session.budget_warning` carries total session tokens rather than the latest assistant message token count only.
  - `src/backend/handshake_core/src/api/flight_recorder.rs:380-392` explicitly queries with `model_session_id: Some("sess-keep")` and returns exactly one matching row, proving non-matching session rows are excluded.
  - `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1208`, `:1321`, and `:1364` prove `session.created` keeps `activity_span_id = None` while terminal lifecycle events stay bound to the session span, preventing accidental activity-span leakage.

INDEPENDENT_FINDINGS:
  - The repaired slice closes the exact three WP-validator findings without widening beyond the signed recorder/runtime/query surfaces: lifecycle ordering, API `model_session_id` passthrough, and total-token budget warning semantics.
  - `model_session_id` remains the primary session correlation key; `session_span_id` and `activity_span_id` add hierarchical observability on the same canonical recorder path instead of creating a second telemetry surface.

RESIDUAL_UNCERTAINTY:
  - Full backend current-main test coverage was not rerun from `handshake_main`; the closeout proof remains intentionally diff-scoped plus compatibility-inspected rather than full-suite exhaustive.
  - The formal closeout path still depends on the consolidation-era docs guardrail accepting the tracked `docs_repo` architecture note as the authoritative session-control document surface.

SPEC_CLAUSE_MAP:
  - 4.3.9.18.2 Span Binding (Normative) => `src/backend/handshake_core/src/workflows.rs:2117-2129`, `src/backend/handshake_core/src/mcp/gate.rs:335-352`, `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:1208-1398`
  - 4.3.9.18.4 Flight Recorder Events (Session Lifecycle) => `src/backend/handshake_core/src/workflows.rs:2141-2298`, `src/backend/handshake_core/src/flight_recorder/mod.rs:5787`
  - 11.5 schema registry must include subsystem event families => `src/backend/handshake_core/src/flight_recorder/mod.rs:364-432`, `src/backend/handshake_core/src/flight_recorder/duckdb.rs:250-288`, `:563-724`, `:955-958`
  - 11.9.1.X Session-Scoped Observability Requirements => `src/backend/handshake_core/src/flight_recorder/duckdb.rs:663-665`, `src/backend/handshake_core/src/api/flight_recorder.rs:182-188`, `:253-263`, `:344-392`

NEGATIVE_PROOF:
  - `src/backend/handshake_core/src/api/flight_recorder.rs:182-263` still exposes a flat `Vec<FlightEvent>` query surface and does not materialize a first-class session-span tree or session-summary endpoint. The packet delivers the signed backend substrate for reconstruction, but that richer product-facing aggregation remains unimplemented in the touched API surface.

PRIMITIVE_RETENTION_PROOF:
  - `FlightRecorderEvent` in `src/backend/handshake_core/src/flight_recorder/mod.rs:362-367` retains the existing recorder envelope and adds session/span identifiers as additive optional fields.
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs:679-727` retains the existing event row contract (`event_id`, `trace_id`, actor, workflow/job/model ids, payload) while extending it with the session/span columns required by this packet.
  - The lifecycle helper in `src/backend/handshake_core/src/workflows.rs:2123-2129` remains additive and still builds on the canonical `FlightRecorderEvent` builder surface instead of introducing a second event primitive.

PRIMITIVE_RETENTION_GAPS:
  - NONE

SHARED_SURFACE_INTERACTION_CHECKS:
  - `src/backend/handshake_core/src/workflows.rs:2141-2298` feeds the same canonical validator and DuckDB storage path in `src/backend/handshake_core/src/flight_recorder/mod.rs` and `src/backend/handshake_core/src/flight_recorder/duckdb.rs`; no shadow event family or side store was introduced.
  - `src/backend/handshake_core/src/api/flight_recorder.rs:58-62`, `:182-188`, and `:253-263` reuse the existing `EventFilter` and `FlightEvent` adapter surface with additive `model_session_id` support only.
  - `src/backend/handshake_core/src/mcp/gate.rs:335-352` uses the shared span helpers from `src/backend/handshake_core/src/flight_recorder/mod.rs:921-929`, so tool-call events remain compatible with the runtime/session recorder contract.

CURRENT_MAIN_INTERACTION_CHECKS:
  - Final-lane review on `handshake_main` validated the imported target commit `65cf306c` against current local `main` and found no signed-scope compatibility drift for `bf3e7f81..65cf306c`.
  - `src/backend/handshake_core/src/api/flight_recorder.rs` and `src/backend/handshake_core/src/flight_recorder/duckdb.rs` consume the new session/span fields as additive `Option<String>` data, so current-main callers do not require a contract rewrite.
  - `run_and_finalize_workflow_job` and `finalize_model_run_after_terminal` in `src/backend/handshake_core/src/workflows.rs:6368` and `:6813-6881` preserve the existing job dispatch / terminal reason flow while repairing the final lifecycle ordering.

DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs:250-288` and `:563-724` keep `model_session_id`, `activity_span_id`, and `session_span_id` as explicit SQL columns rather than burying them in presentation-only text.
  - `src/backend/handshake_core/src/api/flight_recorder.rs:38-62` and `:253-263` expose the same identifiers as explicit JSON properties in `FlightEvent` and `EventFilter`.
  - `src/backend/handshake_core/src/flight_recorder/mod.rs:5787` validates structured lifecycle payload objects, preserving LLM-readable and queryable event semantics on the canonical recorder path.

DATA_CONTRACT_GAPS:
  - NONE

ANTI_VIBE_FINDINGS:
  - NONE

SIGNED_SCOPE_DEBT:
  - NONE
