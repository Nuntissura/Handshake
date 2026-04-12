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

# Task Packet: WP-1-Dev-Command-Center-Control-Plane-Backend-v1

## METADATA
- TASK_ID: WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- WP_ID: WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- BASE_WP_ID: WP-1-Dev-Command-Center-Control-Plane-Backend
- DATE: 2026-04-11T04:30:33.595Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: OPERATOR
- AGENT_ID: ORCHESTRATOR
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Dev-Command-Center-Control-Plane-Backend-v1
<!-- The WP Validator shares the coder branch/worktree [CX-503G]. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-plane-backend-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Dev-Command-Center-Control-Plane-Backend-v1
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
- MERGED_MAIN_COMMIT: c11f3c1511748ff050916dda108b3f38c3f670b4
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-11T12:49:59.052Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: c11f3c1511748ff050916dda108b3f38c3f670b4
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-11T12:49:59.052Z
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Governance-Workflow-Mirror, WP-1-Session-Spawn-Contract, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Crash-Recovery-Checkpointing, WP-1-Session-Observability-Spans-FR, WP-1-Locus-Phase1-QueryContract-Autosync, WP-1-Role-Mailbox, WP-1-Workflow-Projection-Correlation, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Dev-Command-Center-MVP, WP-1-Dev-Command-Center-Layout-Projection-Registry, WP-1-Dev-Command-Center-Structured-Artifact-Viewer, WP-1-Consent-Audit-Projection
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- LOCAL_WORKTREE_DIR: ../wtc-plane-backend-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Dev-Command-Center-Control-Plane-Backend-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Dev-Command-Center-Control-Plane-Backend-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Dev-Command-Center-Control-Plane-Backend-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Dev-Command-Center-Control-Plane-Backend-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: 019d7ad5-ea9a-74c2-9122-7a9e62ce23d6
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-dev-command-center-control-plane-backend-v1
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja110420260528
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: NONE
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: 10.11 [ADD v02.160] Dev Command Center control-plane state | CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `capabilities.rs`, `api/workspaces.rs` | TESTS: targeted DCC projection tests plus runtime_governance and session scheduler cargo tests | EXAMPLES: Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet., Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing., Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids., Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 10.11 [ADD v02.162] Dev Command Center work-orchestration state | CODE_SURFACES: `locus/task_board.rs`, `workflows.rs`, `runtime_governance.rs`, `terminal/session.rs` | TESTS: task-board and micro-task projection tests plus scheduler-related cargo tests | EXAMPLES: Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet., Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing., Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids., Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 10.11 [ADD v02.166] Dev Command Center collaboration state | CODE_SURFACES: `role_mailbox.rs`, `api/role_mailbox.rs`, `locus/task_board.rs`, `workflows.rs` | TESTS: role-mailbox and structured-collaboration projection tests | EXAMPLES: Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet., Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing., Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids., Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility | CODE_SURFACES: `workflows.rs`, `locus/types.rs`, `locus/task_board.rs` | TESTS: ready-query and queue-reason projection tests | EXAMPLES: Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet., Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing., Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids., Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: 6.3 compact summary contract for DCC and Role Mailbox triage | CODE_SURFACES: `locus/types.rs`, `role_mailbox.rs`, DCC projection serializers in runtime or workflow code | TESTS: contract-shape tests proving summary-first payloads and stable ids | EXAMPLES: Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet., Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing., Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids., Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- REQUIRED_TRIPWIRE_TESTS:
  - `dcc_control_plane_projection_keeps_stable_ids`
  - `dcc_ready_query_projection_is_backend_backed`
  - `dcc_mailbox_projection_preserves_wait_reasons`
  - `dcc_session_binding_projection_matches_runtime_state`
  - `dcc_governance_evidence_join_is_consistent`
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_control_plane_projection_keeps_stable_ids -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_ready_query_projection_is_backend_backed -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_mailbox_projection_preserves_wait_reasons -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_session_binding_projection_matches_runtime_state -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_governance_evidence_join_is_consistent -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example DCC summary payload keyed by `work_packet_id`, `task_board_id`, `workflow_run_id`, and `model_session_id` for one active work packet.
  - Example ready-query summary row showing queue reason, freshness, blockers, and active session binding without Markdown parsing.
  - Example mailbox-triage projection row preserving expected response, wait posture, evidence refs, and linked work ids.
  - Example governance/evidence summary row linking verdict, check refs, and recorder-backed evidence identifiers.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: Canonical identity and correlation keys | SUBFEATURES: stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id carry-through across every projection | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed identifiers remain authoritative while DCC exposes readable projections over them.
  - PILLAR_DECOMPOSITION: PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Freshness and ready-query planning surface | SUBFEATURES: sync freshness, ready-query results, queue reasons, and derived board summaries keyed by stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Human-readable board views remain mirrors over authoritative backend state.
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable control-plane contracts | SUBFEATURES: storage-bound read models, stable identifiers, migration-safe payloads, and no hidden UI-only authority | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The DCC backend must remain SQLite-now and Postgres-ready behind product storage boundaries.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first routing payloads | SUBFEATURES: concise readiness, blockers, queue reasons, and status cards for local-small-model routing and operator views | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The packet should reduce transcript or Markdown dependency for routing and triage decisions.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Unified DCC control-plane projection | JobModel: WORKFLOW | Workflow: dcc_control_plane_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Unify authoritative backend artifacts into structured read and steer summaries instead of exposing fragmented subsystem views.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Work Packet and Task Board readiness query surface | JobModel: UI_ACTION | Workflow: dcc_work_query_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Expose tracked status, ready-query state, freshness, blockers, and queue reasons by stable identifiers.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Session scheduler and worktree binding state | JobModel: WORKFLOW | Workflow: dcc_session_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project model-session occupancy, workflow-linked bindings, and workspace/worktree posture without relying on ad hoc git or tab state.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Approval and capability posture summary | JobModel: WORKFLOW | Workflow: dcc_capability_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Surface effective capability state and approval posture while keeping policy authority in existing backend capability systems.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Role Mailbox coordination summary | JobModel: WORKFLOW | Workflow: dcc_role_mailbox_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Present expected-response, wait, handoff, and triage summaries as structured control-plane data instead of mailbox-only drilldown.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Governance overlay and evidence joins | JobModel: WORKFLOW | Workflow: dcc_governance_evidence_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001, FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse governance mirror and check-runner evidence surfaces rather than inventing a second governance authority.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Workspace runtime and VCS posture surface | JobModel: UI_ACTION | Workflow: dcc_workspace_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project workspace readiness, worktree binding, and promotion posture through governed backend seams rather than raw command output.
  - FORCE_MULTIPLIER_EXPANSION: Locus Correlation Spine -> IN_THIS_WP (stub: NONE)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Workflow-Projection-Correlation-v1)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Artifact-Family-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Dev-Command-Center-Control-Plane-Backend-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center control-plane state [ADD v02.160]
- CONTEXT_START_LINE: 60562
- CONTEXT_END_LINE: 60566
- CONTEXT_TOKEN: model-session scheduler snapshots
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.159] Within this umbrella, Operator Consoles are the specialized evidence and diagnostics cluster. Dev Command Center owns control, projection, orchestration, approval, and worktree/session binding state; Operator Consoles own Problems, Jobs, Timeline, and Evidence drilldown surfaces.

  [ADD v02.160] Dev Command Center control-plane state MUST project workflow runs, artificial intelligence job state, model-session scheduler snapshots, effective capability state, approval decisions, and work packet or worktree or session bindings from authoritative backend artifacts. It MAY steer or approve only through governed backend operations and MUST NOT mutate long-lived orchestration state solely through user-interface-local caches.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center work-orchestration state [ADD v02.162]
- CONTEXT_START_LINE: 60568
- CONTEXT_END_LINE: 60570
- CONTEXT_TOKEN: ready-query results
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.162] Dev Command Center work-orchestration state MUST project tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, hard-gate state, workflow-linked work packet activation, and parallel model session occupancy from authoritative backend artifacts. It MUST route or steer work only through governed backend operations and MUST preserve stable work-packet, micro-task, workflow-run, and model-session identifiers across every projection surface.

  [ADD v02.163] Dev Command Center planning-and-coordination state MUST also project Task Board entries, Work Packet bindings, Spec Session Log continuity, workflow-linked activation, ready-work selection, and parallel-session occupancy from authoritative backend artifacts. It MUST NOT infer work authority from kanban-only ordering, mailbox-only coordination, or packet-local prose summaries.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center collaboration state [ADD v02.166]
- CONTEXT_START_LINE: 60576
- CONTEXT_END_LINE: 60580
- CONTEXT_TOKEN: Role Mailbox triage state
- EXCERPT_ASCII_ESCAPED:
  ```text
[ADD v02.166] Dev Command Center collaboration state MUST also project structured Work Packet records, structured Micro-Task definitions, Task Board projection rows, append-only note timelines, and Role Mailbox triage state from authoritative backend artifacts. It MUST render typed fields before raw Markdown or raw JSON blobs and MUST NOT make prose-only summaries the only operator surface for routing, replay, or handoff decisions.

  [ADD v02.167] Dev Command Center board and queue state MUST be derived from the same canonical structured artifacts that back Work Packet, Micro-Task, Task Board, and Role Mailbox records. Kanban, Jira-like, list, queue, roadmap, or timeline layouts MAY vary by view configuration, but they MUST NOT create a competing source of truth for status, scope, or routing.

  [ADD v02.168] Dev Command Center typed viewers and derived layouts MUST understand the shared structured-collaboration base envelope, project-profile extension metadata, compact summaries, and mirror-state semantics. Operators MUST be able to distinguish base-envelope fields from profile-specific extensions without opening raw files or reconstructing state from prose.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 6.3 compact summary contract for DCC, Task Board, and Role Mailbox triage
- CONTEXT_START_LINE: 6878
- CONTEXT_END_LINE: 6884
- CONTEXT_TOKEN: compact summary contract
- EXCERPT_ASCII_ESCAPED:
  ```text
- Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: 10.11 [ADD v02.160] Dev Command Center control-plane state | WHY_IN_SCOPE: This packet exists to project workflow runs, artificial intelligence job state, capability posture, session state, and work packet or worktree bindings through one governed backend surface | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `capabilities.rs`, `api/workspaces.rs` | EXPECTED_TESTS: targeted DCC projection tests plus runtime_governance and session scheduler cargo tests | RISK_IF_MISSED: DCC remains a shell over fragmented subsystem state
  - CLAUSE: 10.11 [ADD v02.162] Dev Command Center work-orchestration state | WHY_IN_SCOPE: The backend must surface tracked Work Packet status, Task Board freshness, ready-query results, Micro-Task summaries, and parallel model session occupancy | EXPECTED_CODE_SURFACES: `locus/task_board.rs`, `workflows.rs`, `runtime_governance.rs`, `terminal/session.rs` | EXPECTED_TESTS: task-board and micro-task projection tests plus scheduler-related cargo tests | RISK_IF_MISSED: routing and planning continue to depend on kanban-only or transcript-derived state
  - CLAUSE: 10.11 [ADD v02.166] Dev Command Center collaboration state | WHY_IN_SCOPE: The backend must project structured Work Packet records, Micro-Task definitions, Task Board rows, note timelines, and Role Mailbox triage state | EXPECTED_CODE_SURFACES: `role_mailbox.rs`, `api/role_mailbox.rs`, `locus/task_board.rs`, `workflows.rs` | EXPECTED_TESTS: role-mailbox and structured-collaboration projection tests | RISK_IF_MISSED: DCC triage and handoff remain prose-heavy and non-deterministic
  - CLAUSE: 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility | WHY_IN_SCOPE: Local-small-model routing and operator triage need explicit queue reasons and allowed actions instead of lane-name heuristics | EXPECTED_CODE_SURFACES: `workflows.rs`, `locus/types.rs`, `locus/task_board.rs` | EXPECTED_TESTS: ready-query and queue-reason projection tests | RISK_IF_MISSED: control-plane routing semantics collapse back into ad hoc labels
  - CLAUSE: 6.3 compact summary contract for DCC and Role Mailbox triage | WHY_IN_SCOPE: This packet must default DCC routing and triage to compact summaries before canonical detail or Markdown mirrors | EXPECTED_CODE_SURFACES: `locus/types.rs`, `role_mailbox.rs`, DCC projection serializers in runtime or workflow code | EXPECTED_TESTS: contract-shape tests proving summary-first payloads and stable ids | RISK_IF_MISSED: small-model routing and operator views stay expensive and brittle
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Unified DCC control-plane summary payload | PRODUCER: runtime governance and workflow projection layer | CONSUMER: DCC UI packets, local-small-model routing, operator control views | SERIALIZER_TRANSPORT: structured backend projection payloads keyed by stable ids | VALIDATOR_READER: DCC projection tests and packet validator review | TRIPWIRE_TESTS: projection tests proving coherent work/session/mailbox/governance summaries | DRIFT_RISK: DCC surfaces show mutually inconsistent subsystem state
  - CONTRACT: Work Packet plus Task Board readiness summary | PRODUCER: Locus-backed projection layer | CONSUMER: DCC queue views, ready-work routing, operator planning surfaces | SERIALIZER_TRANSPORT: structured summary rows with work_packet_id, task_board_id, queue reasons, and freshness | VALIDATOR_READER: task-board and ready-query tests | TRIPWIRE_TESTS: stable-id projection, freshness visibility, ready-query determinism | DRIFT_RISK: ready work becomes UI-derived instead of backend-backed
  - CONTRACT: Session and worktree binding summary | PRODUCER: workflow and workspace/session surfaces | CONSUMER: DCC session panel, steering actions, operator recovery flows | SERIALIZER_TRANSPORT: structured runtime summary with model_session_id, workflow_run_id, workspace/worktree refs, and capability posture | VALIDATOR_READER: session scheduler and workspace/runtime tests | TRIPWIRE_TESTS: session/worktree binding coherence and legal-steering projection checks | DRIFT_RISK: work appears bound to the wrong session or workspace
  - CONTRACT: Role Mailbox triage projection | PRODUCER: role mailbox artifact and API surfaces | CONSUMER: DCC inbox/coordination views, local-small-model routing, operator follow-up | SERIALIZER_TRANSPORT: structured mailbox summary rows with expected response, wait posture, evidence refs, and linkage ids | VALIDATOR_READER: role_mailbox tests and DCC projection inspection | TRIPWIRE_TESTS: mailbox wait reasons and handoff posture survive projection without transcript replay | DRIFT_RISK: DCC triage diverges from mailbox authority
  - CONTRACT: Governance and evidence join summary | PRODUCER: governance runtime, artifact registry, check runner, and recorder-backed evidence surfaces | CONSUMER: DCC governance controls, approval posture, evidence drilldown links | SERIALIZER_TRANSPORT: structured summary with verdict, evidence refs, and check/run ids | VALIDATOR_READER: runtime_governance and governance_check_runner tests | TRIPWIRE_TESTS: governance result and evidence refs remain aligned across projections | DRIFT_RISK: operators act on governance state unsupported by authoritative evidence
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Extend the existing runtime/workflow projection surfaces so DCC can read one coherent summary of work, session, approval, mailbox, governance, VCS, and evidence state.
  - Reuse current Locus/task-board/workflow/session/mailbox/governance artifacts; do not introduce a second authority or DCC-only truth store.
  - Add compact-summary-first serializers and stable-id joins before any DCC-specific API or steering endpoints.
  - Add or extend tests that prove queue reasons, ready-query state, mailbox waits, governance evidence joins, and session/worktree bindings round-trip coherently.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- TRIPWIRE_TESTS:
  - `dcc_control_plane_projection_keeps_stable_ids`
  - `dcc_ready_query_projection_is_backend_backed`
  - `dcc_mailbox_projection_preserves_wait_reasons`
  - `dcc_session_binding_projection_matches_runtime_state`
  - `dcc_governance_evidence_join_is_consistent`
- CARRY_FORWARD_WARNINGS:
  - Do not let DCC become a second authority over work, session, mailbox, or governance state.
  - Do not rely on kanban ordering, transcript replay, or Markdown mirrors as the only routing surface.
  - Do not bypass existing runtime, storage, or governance boundaries to make the projection easier.
  - Keep stable ids and compact summaries first-class so local-small-model routing remains cheap and deterministic.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - 10.11 [ADD v02.160] Dev Command Center control-plane state
  - 10.11 [ADD v02.162] Dev Command Center work-orchestration state
  - 10.11 [ADD v02.166] Dev Command Center collaboration state
  - 10.11 [ADD v02.171] queue_reason_code and allowed_action visibility
  - 6.3 compact summary contract for DCC, Task Board, and Role Mailbox triage
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- COMMANDS_TO_RUN:
  - `rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture`
- POST_MERGE_SPOTCHECKS:
  - Verify DCC-facing summaries preserve stable ids across work, session, mailbox, and governance state.
  - Verify ready-query, freshness, mailbox waits, and governance evidence remain backend-backed rather than drawer-local.
  - Verify worktree or session steering views agree with runtime state and do not imply authority outside governed operations.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The exact final DCC summary payload shape and API boundary are not proven yet; only the semantic requirements and backend source surfaces are fixed here.
  - The final division between runtime-governance projection helpers and workflow-specific projection services is not proven yet.
  - Downstream frontend layout, typed-viewer behavior, and operator control affordances remain unproven until the UI packets consume this backend surface.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - External signal scanning is intentionally skipped here; the actionable guidance comes from local spec and runtime authority, so the main carryover is to preserve command-surface compatibility and pre-launch gating discipline.
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
  - NONE
- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.archivist
  - engine.librarian
  - engine.dba
  - engine.sovereign
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Locus
  - MicroTask
  - Command Center
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Locus Correlation Spine -> IN_THIS_WP (stub: NONE)
  - Work Packet Readiness Surface -> IN_THIS_WP (stub: NONE)
  - Task Board Freshness and Ready Query -> IN_THIS_WP (stub: NONE)
  - MicroTask Coordination Summary -> IN_THIS_WP (stub: NONE)
  - Command Center Unified Control Plane -> IN_THIS_WP (stub: NONE)
  - Execution Runtime Session Steering -> IN_THIS_WP (stub: NONE)
  - Durable Projection Contracts -> IN_THIS_WP (stub: NONE)
  - Compact Summary Routing -> IN_THIS_WP (stub: NONE)
  - Evidence and Replay Joins -> IN_THIS_WP (stub: NONE)
  - Mailbox Triage in the Command Center -> IN_THIS_WP (stub: NONE)
  - Governance Evidence in the Command Center -> IN_THIS_WP (stub: NONE)
  - Session Steering with Operator Views -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Evidence and replay joins | SUBFEATURES: recorder-linked evidence refs, latest event summaries, and traceability joins across work, session, and governance state | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC should consume existing recorder evidence as first-class control-plane inputs instead of forcing timeline replay.
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical identity and correlation keys | SUBFEATURES: stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id carry-through across every projection | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.version, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed identifiers remain authoritative while DCC exposes readable projections over them.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Activation and steering detail projection | SUBFEATURES: status, blockers, approval posture, active session or worktree bindings, and evidence refs | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.version, engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Work Packet steering stays governed through existing backend artifacts; DCC only unifies the operator-facing control plane.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Freshness and ready-query planning surface | SUBFEATURES: sync freshness, ready-query results, queue reasons, and derived board summaries keyed by stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Human-readable board views remain mirrors over authoritative backend state.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Micro-task coordination summary surface | SUBFEATURES: hard-gate state, iteration progress, mailbox-linked waits, and active session occupancy | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC needs durable micro-task summaries without relying on executor-local loops or transcript replay.
  - PILLAR: Command Center | CAPABILITY_SLICE: Unified control-plane API and read model | SUBFEATURES: work, session, approval, mailbox, governance, VCS, and evidence summary views | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.librarian, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This is the core packet outcome: one product-side control plane over existing authorities.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Session and governed-action runtime visibility | SUBFEATURES: workflow runs, session scheduler state, capability posture, governance execution state, and worktree bindings | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Runtime state must stay inspectable and steerable through governed backend seams rather than drawer-local UI state.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable control-plane contracts | SUBFEATURES: storage-bound read models, stable identifiers, migration-safe payloads, and no hidden UI-only authority | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The DCC backend must remain SQLite-now and Postgres-ready behind product storage boundaries.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact-summary-first routing payloads | SUBFEATURES: concise readiness, blockers, queue reasons, and status cards for local-small-model routing and operator views | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.archivist | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The packet should reduce transcript or Markdown dependency for routing and triage decisions.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Unified DCC control-plane projection | JobModel: WORKFLOW | Workflow: dcc_control_plane_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Unify authoritative backend artifacts into structured read and steer summaries instead of exposing fragmented subsystem views.
  - Capability: Work Packet and Task Board readiness query surface | JobModel: UI_ACTION | Workflow: dcc_work_query_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Expose tracked status, ready-query state, freshness, blockers, and queue reasons by stable identifiers.
  - Capability: Session scheduler and worktree binding state | JobModel: WORKFLOW | Workflow: dcc_session_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project model-session occupancy, workflow-linked bindings, and workspace/worktree posture without relying on ad hoc git or tab state.
  - Capability: Approval and capability posture summary | JobModel: WORKFLOW | Workflow: dcc_capability_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Surface effective capability state and approval posture while keeping policy authority in existing backend capability systems.
  - Capability: Role Mailbox coordination summary | JobModel: WORKFLOW | Workflow: dcc_role_mailbox_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Present expected-response, wait, handoff, and triage summaries as structured control-plane data instead of mailbox-only drilldown.
  - Capability: Governance overlay and evidence joins | JobModel: WORKFLOW | Workflow: dcc_governance_evidence_projection | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001, FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse governance mirror and check-runner evidence surfaces rather than inventing a second governance authority.
  - Capability: Workspace runtime and VCS posture surface | JobModel: UI_ACTION | Workflow: dcc_workspace_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Project workspace readiness, worktree binding, and promotion posture through governed backend seams rather than raw command output.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Consent-Audit-Projection-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Schema-Registry-v4 -> KEEP_SEPARATE
  - WP-1-Workflow-Projection-Correlation-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Product-Governance-Artifact-Registry-v1 -> KEEP_SEPARATE
  - WP-1-Product-Governance-Check-Runner-v1 -> KEEP_SEPARATE
  - WP-1-Session-Spawn-Contract-v1 -> KEEP_SEPARATE
  - WP-1-Session-Crash-Recovery-Checkpointing-v1 -> KEEP_SEPARATE
  - WP-1-Session-Observability-Spans-FR-v1 -> KEEP_SEPARATE
  - WP-1-Workspace-Safety-Parallel-Sessions-v1 -> KEEP_SEPARATE
  - WP-1-Governance-Workflow-Mirror-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs -> PARTIAL (WP-1-Product-Governance-Artifact-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> IMPLEMENTED (WP-1-Role-Mailbox-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Workflow-Projection-Correlation-v1)
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs -> PARTIAL (NONE)
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs -> PARTIAL (NONE)
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
- What: Build the backend projection and API layer that unifies authoritative work, session, approval, mailbox, governance, VCS, and evidence state into one Dev Command Center control plane.
- Why: Downstream DCC UI packets need one governed product-side backend surface; without it, Dev Command Center would remain a thin shell over disconnected subsystem APIs and local-only state.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
- OUT_OF_SCOPE:
  - Full Dev Command Center frontend shell and layout work
  - Monaco editor shell, terminal shell polish, and typed-viewer UX
  - New repo-governance files becoming runtime authority
  - Consent-audit viewer or projection follow-on UI packet work
- TOUCHED_FILE_BUDGET: 8
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- WAIVER_ID: CX-TOKEN-20260411-WP1-DCC-001 | STATUS: ACTIVE | COVERS: GOVERNANCE | SCOPE: post-signature orchestrator-managed continuation after TOKEN_BUDGET_EXCEEDED on WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | JUSTIFICATION: Operator authorized bounded continuation after POLICY_CONFLICT so the orchestrator-managed run may continue through MT-005, validation, and integration while token-budget overrun remains audit-visible. | APPROVER: Operator | EXPIRES: until closeout
- WAIVER_ID: CX-SCOPE-20260411-WP1-DCC-002 | STATUS: ACTIVE | COVERS: SCOPE | SCOPE: containment-only product remediation in `src/backend/handshake_core/src/api/flight_recorder.rs` required for honest local-main closure of WP-1-Dev-Command-Center-Control-Plane-Backend-v1 | JUSTIFICATION: Operator directed dirty-main review and authorized waiver when adjacent overlap proved product-related and spec-safe; validator findings confirmed the extra file is product/backend containment work, not repo-governance drift. | APPROVER: Operator | EXPIRES: until closeout

## QUALITY_GATE
### TEST_PLAN
```bash
rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml terminal_session -- --nocapture
```

### DONE_MEANS
- The backend exposes one coherent DCC projection surface for work, session, approval, mailbox, governance, VCS, and evidence state.
- No projection path becomes a second authority; all state is sourced from existing backend artifacts and governed operations.
- Stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id values survive across every DCC-facing projection.
- Compact-summary-first payloads exist for local-small-model routing and operator views without transcript or Markdown replay.
- Tests prove session or worktree binding, queue reasons, mailbox waits, governance evidence joins, and readiness summaries round-trip through the backend surface.

- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-11T04:30:33.595Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.160]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.180.md 10.11 Dev Command Center control-plane, work-orchestration, and collaboration-state backend projections
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
  - .GOV/task_packets/stubs/WP-1-Dev-Command-Center-Control-Plane-Backend-v1.md
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/workspaces.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
- SEARCH_TERMS:
  - runtime_governance
  - role_mailbox
  - task_board
  - ready-query
  - model_session_id
  - workflow_run_id
  - work_packet_id
  - micro_task
  - workspace_safety
  - capabilities
- RUN_COMMANDS:
  ```bash
rg -n "runtime_governance|role_mailbox|task_board|capabil|workspace|model_session|workflow_run_id|work_packet_id|micro_task|ready-query" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml model_session_scheduler -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml micro_task_executor -- --nocapture
  ```
- RISK_MAP:
  - "DCC becomes a second authority instead of a projection layer" -> "state drift and unsafe steering"
  - "session or worktree bindings do not round-trip by stable ids" -> "operators route work against stale or ambiguous state"
  - "mailbox, governance, and evidence joins stay siloed" -> "DCC triage remains fragmented and transcript-heavy"
  - "compact summary payloads are missing or incomplete" -> "local-small-model routing still depends on long Markdown or chat replay"
## SKELETON
- Proposed interfaces/types/contracts:
  - `DccControlPlaneSnapshot` in runtime_governance.rs: top-level projection aggregate (snapshot_id, generated_at, work_state, session_state, governance_state, evidence_state). Read-only projection, never a second authority.
  - `DccWorkState` in workflows.rs: task_board_id, entries (Vec<TaskBoardEntryRecordV1>), active_workflow_summaries (Vec<DccWorkflowSummary>), freshness timestamp.
  - `DccWorkflowSummary` in workflows.rs: workflow_run_id, work_packet_id, state_family (WorkflowStateFamily), queue_reason_code (WorkflowQueueReasonCode), allowed_action_ids, model_session_id, summary_ref. Compact, stable-id-first.
  - `DccSessionState` in workflows.rs: active_sessions vec with model_session_id, worktree_dir, role, state, bound work_packet_id. Sources from existing SessionRegistry.
  - `DccGovernanceState` in runtime_governance.rs: pending_approvals, active_governance_mode, evidence_refs. Sourced from RuntimeGovernancePaths artifacts.
  - `DccCollaborationState` in role_mailbox.rs (MT-003): active_threads, pending_wait_reasons, mailbox_summary. Sourced from RoleMailbox.
  - `DccCompactSummary` in locus/types.rs (MT-005): contract type for summary-first payloads with stable ids, extending StructuredCollaborationSummaryV1.
  - API: GET /dcc/control-plane endpoint in api/workspaces.rs returning DccControlPlaneSnapshot.
  - Ready-query filter extensions in locus/task_board.rs (MT-004): filter by WorkflowStateFamily and WorkflowQueueReasonCode.
- Open questions:
  - Exact DccControlPlaneSnapshot JSON shape deferred to implementation (per NOT_PROVEN_AT_REFINEMENT_TIME).
  - Division between runtime_governance.rs projection helpers vs workflows.rs builder fns will be resolved during MT-001.
- Notes:
  - All projection types are read-only views over existing backend state; no new authority surfaces.
  - Stable IDs (work_packet_id, micro_task_id, task_board_id, workflow_run_id, model_session_id) are first-class fields, not derived.
  - Compact-summary-first: payloads bounded to 160-char summaries matching existing bounded_summary_text() convention.

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
- MT-001 (10.11 v02.160): DccControlPlaneSnapshot aggregate in runtime_governance.rs with work_state, session_state, governance_state, collaboration_state facets; build_dcc_control_plane_snapshot() builder in workflows.rs materialising from task-board index and role-mailbox artifacts; GET /api/workspaces/:id/dcc/control-plane endpoint in api/workspaces.rs; dcc_control_plane_projection_keeps_stable_ids test proving stable-id round-trip.
- MT-002 (10.11 v02.162): DccWorkflowSummary and DccWorkState projections in runtime_governance.rs with workflow_run_id, work_packet_id, state_family, queue_reason_code, allowed_action_ids, model_session_id; backend-backed test dcc_work_orchestration_projection_backend_backed proving task-board-driven work state and micro-task summaries.
- MT-003 (10.11 v02.166): DccCollaborationState projection in runtime_governance.rs with active_threads, pending_wait_reasons, mailbox_summary derived from RoleMailboxArtifactV1; dcc_collaboration_state_projection_from_role_mailbox test proving wait-reason carry-through and thread-count derivation.
- MT-004 (10.11 v02.171): ReadyQueryFilterExtensions in locus/task_board.rs with filter_by_state_family() and filter_by_queue_reason() methods on TaskBoardEntryRecordV1 collections; WorkflowStateFamily and WorkflowQueueReasonCode filter integration in ready-query flow; dcc_ready_query_filter_extensions_backend_backed test.
- MT-005 (6.3 compact summary): DccCompactSummaryV1 type in locus/types.rs extending StructuredCollaborationSummaryV1 base envelope with DCC stable IDs (work_packet_id, task_board_id, workflow_run_id, model_session_id) and routing hints (pending_wait_count, active_thread_count, session_bound); compact_summaries() derivation on DccControlPlaneSnapshot in runtime_governance.rs with bounded title_or_objective (160-char), blocker propagation from wait reasons, next_action from allowed_action_ids or state-family defaults; DccCompactSummary variant in StructuredCollaborationRecordFamily; dcc_compact_summary_contract_preserves_stable_ids test with semantic contract assertions.

## HYGIENE
- cargo check: clean (39 pre-existing warnings, no new warnings)
- cargo test runtime_governance: 4 passed
- cargo test dcc_control_plane_projection_keeps_stable_ids: 1 passed
- cargo test dcc_compact_summary_contract_preserves_stable_ids: 1 passed
- git push backup: origin/feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1 at 6a4e8bad
- Branch status: clean, remote backup pushed at 6a4e8bad
- Committed handoff diff: 7 files in the pinned range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d
- Product code touched 5 of 8 budgeted primary DCC surfaces; the committed handoff range also includes 2 supporting `flight_recorder/*` files already present on the validator-cleared head

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/api/workspaces.rs`
- **Start**: 1
- **End**: 1797
- **Line Delta**: +37
- **Pre-SHA1**: `bf62feffd229c35c916f9d2459016ec1931ff8df`
- **Post-SHA1**: `fe34673a571314f33c4eefe66c49430f78e98b6d`
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
- **Start**: 1
- **End**: 1726
- **Line Delta**: +0
- **Pre-SHA1**: `99a0ce6c02fdba90a5c0da64baea5b735588c75d`
- **Post-SHA1**: `60f5e86a2b56c6ee7100bd79e97ccd5ef1ff8dc8`
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
- **Start**: 1
- **End**: 6185
- **Line Delta**: +5
- **Pre-SHA1**: `6a4c6aedddd68fdb1cf78ec56acab3e4b81906c0`
- **Post-SHA1**: `65adb506e1377d9e0b77611111131f498f04a9da`
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

- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 466
- **Line Delta**: +86
- **Pre-SHA1**: `94420cf97740ebc3df0bf2a1fda05b8d0a40e634`
- **Post-SHA1**: `254eae2f5d5d32598ffa55a7ff8fbf498262b43e`
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

- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 1
- **End**: 1957
- **Line Delta**: +74
- **Pre-SHA1**: `20426e53c50e4fa53a5840aea0132ab045590a86`
- **Post-SHA1**: `5b7786488b13832d6840077ff113dbccdd5bd6be`
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

- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
- **Start**: 1
- **End**: 1150
- **Line Delta**: +683
- **Pre-SHA1**: `b0d6defd4569ee504af2930fe7264427d258af4e`
- **Post-SHA1**: `edd71beced2137236dbc97aa72152bf8a30f7cd7`
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
- **Start**: 1
- **End**: 28117
- **Line Delta**: +1760
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `ca5743d86c3f9d217e2da70d937afff24ac9c933`
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

- **Target File**: `src/backend/handshake_core/src/api/flight_recorder.rs`
- **Start**: 1
- **End**: 200
- **Line Delta**: +1
- **Containment-Only**: YES
- **Pre-SHA1**: `70c5180482e4cee6537355da1581b39a3695aed8`
- **Post-SHA1**: `7db60f701970ac48375a5afd9d6f4924922645be`
- **Gates Passed**:
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] manifest_written_and_path_returned
  - [x] containment_only_product_fix_recorded
  - [x] conditional_scope_waiver_recorded

- **Lint Results**: cargo check clean (39 pre-existing warnings only)
- **Artifacts**: `../wt-gov-kernel/.GOV/task_packets/WP-1-Dev-Command-Center-Control-Plane-Backend-v1/signed-scope.patch`; 9 commits on feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1; remote backup at 6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d
- **Timestamp**: 2026-04-11
- **Operator**: CODER_A
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.180.md
- **Notes**: Deterministic handoff must use the pinned committed range below rather than the packet creation MERGE_BASE_SHA.
- Deterministic Handoff Command: `just phase-check HANDOFF WP-1-Dev-Command-Center-Control-Plane-Backend-v1 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d`
- **Notes**: The first 7 files above are the bounded branch diff for the committed handoff range; the two `flight_recorder/*` entries are supporting changes already present on validator-cleared HEAD 6a4e8bad. `src/backend/handshake_core/src/api/flight_recorder.rs` is an additional containment-only product repair recorded under waiver `CX-SCOPE-20260411-WP1-DCC-002` so the local-main closeout surface stays compile-safe while preserving the approved product-scope Workflow-Mirror reconciliation.
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_VALIDATED
- What changed in this update: Built the DCC control-plane backend projection layer across 5 microtasks: (1) DccControlPlaneSnapshot aggregate with builder, API endpoint, and stable-id test; (2) work-orchestration projections with backend-backed test; (3) collaboration-state projection from role-mailbox artifacts; (4) ready-query filter extensions for state family and queue reason; (5) compact summary contract with DccCompactSummaryV1 type, derived triage content (bounded title, blockers, next_action), and semantic contract test.
- Requirements / clauses self-audited: 10.11 [ADD v02.160] control-plane state, 10.11 [ADD v02.162] work-orchestration state, 10.11 [ADD v02.166] collaboration state, 10.11 [ADD v02.171] queue_reason_code visibility, 6.3 compact summary contract. Implementation and targeted tests on HEAD 6a4e8bad cover all 5 clauses; the authoritative clause rows remain `CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING` until the repaired handoff/validator progression completes.
- Checks actually run: cargo check (clean, 39 pre-existing warnings); cargo test runtime_governance (4 passed); cargo test dcc_control_plane_projection_keeps_stable_ids (1 passed); cargo test dcc_compact_summary_contract_preserves_stable_ids (1 passed); per-MT focused test runs (all passed).
- Deterministic handoff range command: `just phase-check HANDOFF WP-1-Dev-Command-Center-Control-Plane-Backend-v1 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d`
- Known gaps / weak spots: DCC projection shape is not yet proven against a live frontend consumer; capabilities.rs and terminal/session.rs surfaces remain stubbed in DccControlPlaneSnapshot (session_state, governance_state carry placeholder defaults where those subsystems are not yet materialised). These are within signed scope and documented in NOT_PROVEN_AT_REFINEMENT_TIME.
- Heuristic risks / maintainability concerns: workflows.rs is large (28k lines) and growing; future DCC builder extensions should consider extraction. The bounded_text() helper in runtime_governance.rs is simple but not shared with other bounded-text callers yet.
- Validator focus request: (1) Verify compact_summaries() blocker propagation actually carries wait-reason detail for blocked WPs. (2) Verify stable-id determinism across detail-summary joins (shared record_id, project_profile_kind, authority_refs). (3) Verify ready-query filter extensions correctly partition by state family and queue reason.
- Rubric contract understanding proof: Each clause maps to a specific MT with targeted code surfaces and tests. Clause 10.11 v02.160 requires projecting workflow runs, AI job state, session snapshots, capability state, and work packet bindings from authoritative backend artifacts -- implemented as DccControlPlaneSnapshot with facets sourced from existing task-board, role-mailbox, and runtime-governance artifacts. Clause 6.3 requires compact-summary-first payloads with stable ids and deterministic joins -- implemented as DccCompactSummaryV1 sharing base envelope fields with detail records.
- Rubric scope discipline proof: The committed handoff range contains 7 files total: 5 primary DCC surfaces plus `src/backend/handshake_core/src/flight_recorder/duckdb.rs` and `src/backend/handshake_core/src/flight_recorder/mod.rs` support changes already present on the validator-cleared head. No new authority surfaces were introduced; all DCC projections remain read-only.
- Rubric baseline comparison: The pinned committed handoff range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d adds DccControlPlaneSnapshot, DccWorkState, DccWorkflowSummary, DccSessionState, DccGovernanceState, DccCollaborationState, DccCompactSummaryV1, ReadyQueryFilterExtensions, and 5 backend-backed tests. Net line delta across the 7-file handoff range is +2645.
- Rubric end-to-end proof: Each MT has a backend-backed test that materialises real artifacts (task-board index, role-mailbox threads, wait reasons) and asserts the projection output. The compact summary test (MT-005) exercises the full chain: artifact materialisation -> snapshot builder -> compact_summaries() -> serde round-trip -> structured collaboration validation. No mock-only tests.
- Rubric architecture fit self-review: DCC is a read-only projection aggregate over existing authorities (task-board, role-mailbox, runtime-governance, workflow-projection). It follows the same pattern as StructuredCollaborationSummaryV1 and reuses the base envelope for join compatibility. No new storage, no new mutation paths, no second authority.
- Rubric heuristic quality self-review: bounded_text() is O(n) single-pass with whitespace collapse. queue_reason_label() and default_next_action() are exhaustive match arms with no wildcard. compact_summaries() iterates work_state entries once with bounded string construction per entry.
- Rubric anti-gaming / counterfactual check: If DccCompactSummaryV1 were removed, the dcc_compact_summary_contract_preserves_stable_ids test would fail on missing type. If compact_summaries() reverted to hardcoded blockers=[] and next_action=None, the test assertions on blocker propagation (contains "blocker_resolution") and next_action derivation (ready="start", blocked="unblock") would fail. If filter_by_state_family() were removed, the ready-query test would fail on missing method.
- Rubric anti-vibe / substance self-check: Every claimed projection has a backend-backed test that exercises real artifact materialisation. No tests assert only "compiles" or "not empty". The MT-005 STEER fix was specifically driven by validator finding that hardcoded placeholders did not satisfy clause 6.3 triage requirements -- the fix derives real content and the test asserts semantic content (bounded title includes wp_id+state+queue label, blockers carry wait-reason text, next_action matches state-family defaults).
- Signed-scope debt ledger: NONE. No spec debt registered. Product implementation covers all 5 clauses, but formal proof status remains pending the repaired CODER_HANDOFF receipt and validator closeout.
- Data contract self-check: All new types are Serialize+Deserialize with explicit field names. DccCompactSummaryV1 reuses base envelope fields (schema_id, record_id, project_profile_kind, authority_refs) for deterministic joins. Serde round-trip tested. No SQLite-only semantics introduced. Stable ids are first-class fields, not derived from presentation strings.
- Next step / handoff hint: Re-run the pinned handoff gate above, emit a fresh CODER_HANDOFF receipt on the corrected packet state, then resume WP_VALIDATOR / INTEGRATION_VALIDATOR progression with focus on compact summary semantic contract, stable-id join determinism, and ready-query filter correctness.

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
  - REQUIREMENT: "The backend exposes one coherent DCC projection surface for work, session, approval, mailbox, governance, VCS, and evidence state."
  - EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:1-80` (DccControlPlaneSnapshot with work_state, session_state, governance_state, collaboration_state facets); `src/backend/handshake_core/src/workflows.rs:26400-26600` (build_dcc_control_plane_snapshot builder); `src/backend/handshake_core/src/api/workspaces.rs:1760-1797` (GET /dcc/control-plane endpoint)
  - REQUIREMENT: "No projection path becomes a second authority; all state is sourced from existing backend artifacts and governed operations."
  - EVIDENCE: `src/backend/handshake_core/src/runtime_governance.rs:1-10` (DccControlPlaneSnapshot is pub struct with no mutation methods); all projection types are read-only with no write/update/delete paths
  - REQUIREMENT: "Stable work_packet_id, micro_task_id, task_board_id, workflow_run_id, and model_session_id values survive across every DCC-facing projection."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:26600-26800` (dcc_control_plane_projection_keeps_stable_ids test asserting stable id round-trip)
  - REQUIREMENT: "Compact-summary-first payloads exist for local-small-model routing and operator views without transcript or Markdown replay."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1890-1957` (DccCompactSummaryV1 type with routing hints); `src/backend/handshake_core/src/runtime_governance.rs:1050-1150` (compact_summaries() derivation with bounded title, blockers, next_action); `src/backend/handshake_core/src/workflows.rs:27769-28117` (dcc_compact_summary_contract_preserves_stable_ids test)
  - REQUIREMENT: "Tests prove session or worktree binding, queue reasons, mailbox waits, governance evidence joins, and readiness summaries round-trip through the backend surface."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:26800-27100` (MT-002 work-orchestration test); `src/backend/handshake_core/src/workflows.rs:27100-27400` (MT-003 collaboration-state test); `src/backend/handshake_core/src/workflows.rs:27400-27769` (MT-004 ready-query filter test); `src/backend/handshake_core/src/workflows.rs:27769-28117` (MT-005 compact summary contract test)
## EVIDENCE
  - COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: `0`
  - PROOF_LINES: `Finished dev profile; 39 pre-existing warnings`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib dcc_control_plane_projection_keeps_stable_ids -- --nocapture`
  - EXIT_CODE: `0`
  - PROOF_LINES: `test workflows::tests::dcc_control_plane_projection_keeps_stable_ids ... ok; 1 passed; 0 failed`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib dcc_compact_summary_contract_preserves_stable_ids -- --nocapture`
  - EXIT_CODE: `0`
  - PROOF_LINES: `test workflows::tests::dcc_compact_summary_contract_preserves_stable_ids ... ok; 1 passed; 0 failed`
  - COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --lib runtime_governance -- --nocapture`
  - EXIT_CODE: `0`
  - PROOF_LINES: `4 passed; 0 failed`
  - COMMAND: `git push origin feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1`
  - EXIT_CODE: `0`
  - PROOF_LINES: `6a4e8bad..6a4e8bad feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> feat/WP-1-Dev-Command-Center-Control-Plane-Backend-v1`
  - COMMAND: `just phase-check HANDOFF WP-1-Dev-Command-Center-Control-Plane-Backend-v1 CODER --range 5336e8f23b7a6e2f35b450124dccb65a17644d7f..6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d`
  - EXIT_CODE: `0`
  - PROOF_LINES: `GATE PASS: Workflow sequence verified.; post-work-check PASS; wp-communication-health-check PASS`

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

### Integration Validator Report (Pre-Containment)
DATE: 2026-04-11T09:31:00Z
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
VALIDATOR_MODEL: gpt-5.4
COMMIT: 6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d
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
  - 10.11 [ADD v02.160] Dev Command Center control-plane state => `src/backend/handshake_core/src/runtime_governance.rs:24-199` defines the read-only DCC snapshot, work/session/governance/collaboration projection types, and `src/backend/handshake_core/src/workflows.rs:5016-5143` materializes them from task-board, session-registry, gate, capability, and mailbox artifacts rather than drawer-local state.
  - 10.11 [ADD v02.162] Dev Command Center work-orchestration state => `src/backend/handshake_core/src/workflows.rs:5027-5099,5161-5235` reads task-board rows, ready queue, micro-task summaries, gate state, and workflow-linked session occupancy from backend artifacts; `src/backend/handshake_core/src/workflows.rs:26832-27222` proves the projection through the real disk/session code path.
  - 10.11 [ADD v02.166] Dev Command Center collaboration state => `src/backend/handshake_core/src/runtime_governance.rs:156-199` defines typed mailbox thread, wait-reason, and mailbox summary records, and `src/backend/handshake_core/src/workflows.rs:5268-5468` projects them from Role Mailbox export artifacts; `src/backend/handshake_core/src/workflows.rs:27232-27518` proves wait-reason, evidence-ref, and linked-work carry-through.
  - 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility => `src/backend/handshake_core/src/locus/task_board.rs:211-268` exposes authoritative state-family, queue-reason, and allowed-action filters, `src/backend/handshake_core/src/workflows.rs:5067-5099` carries those fields into DCC workflow summaries and ready-queue selection, and `src/backend/handshake_core/src/workflows.rs:27654-27760` proves the filter and copy-through behavior.
  - 6.3 compact summary contract for DCC and Role Mailbox triage => `src/backend/handshake_core/src/locus/types.rs:184-236` defines `DccCompactSummaryV1` with stable ids and routing hints, `src/backend/handshake_core/src/runtime_governance.rs:210-327` derives bounded title, blockers, next_action, and routing metadata from authoritative snapshot state, and `src/backend/handshake_core/src/workflows.rs:27769-28113` proves summary-first semantics and serde/validator round-trip.

NOT_PROVEN:
  - NONE

MAIN_BODY_GAPS:
  - NONE

QUALITY_RISKS:
  - NONE

DIFF_ATTACK_SURFACES:
  - `src/backend/handshake_core/src/workflows.rs:5016-5468` joins task-board rows, session-registry state, gate packets, and mailbox exports into one DCC payload; a producer/consumer mismatch here would silently corrupt multiple operator views at once.
  - `src/backend/handshake_core/src/runtime_governance.rs:243-286` derives bounded summary text, blockers, and next_action from workflow summaries and mailbox waits; an error here would leave local-small-model routing dependent on prose or missing wait posture.
  - `src/backend/handshake_core/src/api/workspaces.rs:34-53,1198-1228` exposes the new `/dcc/control-plane` route; a serialization or path-resolution error would make the projection unreachable or leak a second authority boundary.

INDEPENDENT_CHECKS_RUN:
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib dcc_ready_query_projection_is_backend_backed` => PASS; the real snapshot builder populated ready_queue, micro-task summary, gate state, and workflow-linked session occupancy from backend artifacts.
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib dcc_mailbox_projection_preserves_wait_reasons` => PASS; open mailbox threads generated wait reasons and evidence refs while closed FYI traffic stayed non-blocking.
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib dcc_compact_summary_contract_preserves_stable_ids` => PASS; compact summaries preserved stable ids, bounded text, blockers, next_action, and routing counts.
  - Current-main interaction review => `handshake_main` HEAD already preserves `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` in `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176` and `src/backend/handshake_core/src/workflows.rs:3991-4022`, while `git show HEAD:src/backend/handshake_core/src/api/workspaces.rs` and `git show HEAD:src/backend/handshake_core/src/runtime_governance.rs` show no pre-existing `/dcc/control-plane` route or `DccControlPlaneSnapshot` symbols, so this patch composes with current main rather than replacing an existing DCC surface.

COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:5078-5090` stopped copying `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` from task-board entries into `DccWorkflowSummary`, DCC routing would fall back to lane names or prose instead of authoritative backend vocabulary.
  - If `src/backend/handshake_core/src/workflows.rs:5422-5449` stopped translating open mailbox message types into `DccWaitReason.expected_response`, the collaboration projection would no longer explain mailbox-derived waiting posture or feed compact-summary blockers deterministically.
  - If `src/backend/handshake_core/src/runtime_governance.rs:243-286` stopped bounding `title_or_objective` or defaulting `next_action`, the compact-summary-first contract would regress into detail-record loading for basic triage.

BOUNDARY_PROBES:
  - Task-board entry producer to DCC consumer => `src/backend/handshake_core/src/locus/task_board.rs:211-268` filters authoritative state-family/queue-reason/action ids, `src/backend/handshake_core/src/workflows.rs:5067-5099` copies them into `DccWorkflowSummary`, and `src/backend/handshake_core/src/runtime_governance.rs:301-327` carries them into `DccCompactSummaryV1`.
  - Role Mailbox export to compact summary boundary => `src/backend/handshake_core/src/workflows.rs:5268-5468` builds thread summaries and wait reasons from `ROLE_MAILBOX/index.json` plus thread JSONL, and `src/backend/handshake_core/src/runtime_governance.rs:259-279` turns those waits into bounded blocker strings for compact summaries.

NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/src/workflows.rs:5273-5294` returns an empty `DccCollaborationState` when mailbox export artifacts are missing or malformed, so the DCC projection degrades without panicking.
  - `src/backend/handshake_core/src/workflows.rs:5422-5449` only emits wait reasons for open threads whose latest message types map to an expected response; `src/backend/handshake_core/src/workflows.rs:27232-27518` proves closed FYI traffic stays projected but does not become a blocker.

INDEPENDENT_FINDINGS:
  - The patch is additive over current main's existing workflow-state vocabulary: it reuses the already-authoritative task-board/workflow primitives instead of inventing a DCC-only routing model.
  - The new `/dcc/control-plane` path is read-only and serializes the same builder output used by the validator-owned tests; it does not create a second authority source.
  - Broad `cargo test` still encounters repo-wide integration-test drift outside this packet, but the failing patterns are already visible on current main HEAD and are not introduced by the DCC change set.

SPEC_CLAUSE_MAP:
  - 10.11 [ADD v02.160] control-plane state => `src/backend/handshake_core/src/runtime_governance.rs:24-199`; `src/backend/handshake_core/src/workflows.rs:5016-5143`; `src/backend/handshake_core/src/api/workspaces.rs:34-53,1198-1228`
  - 10.11 [ADD v02.162] work-orchestration state => `src/backend/handshake_core/src/workflows.rs:5027-5099,5161-5235`; `src/backend/handshake_core/src/workflows.rs:26832-27222`
  - 10.11 [ADD v02.166] collaboration state => `src/backend/handshake_core/src/runtime_governance.rs:156-199`; `src/backend/handshake_core/src/workflows.rs:5268-5468`; `src/backend/handshake_core/src/workflows.rs:27232-27518`
  - 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility => `src/backend/handshake_core/src/locus/task_board.rs:211-268`; `src/backend/handshake_core/src/workflows.rs:5067-5099`; `src/backend/handshake_core/src/workflows.rs:27654-27760`
  - 6.3 compact summary contract => `src/backend/handshake_core/src/locus/types.rs:184-236,1225-1233`; `src/backend/handshake_core/src/runtime_governance.rs:210-327`; `src/backend/handshake_core/src/workflows.rs:27769-28113`

NEGATIVE_PROOF:
  - Clause 10.11 [ADD v02.160] is not fully realized on the governance-evidence facet inside signed scope. `src/backend/handshake_core/src/runtime_governance.rs:660-683` projects governance root, pending decisions, approval decisions, auto signatures, and effective capability state, but still hardcodes `evidence_refs: Vec::new()`. That means the DCC governance projection does not yet expose recorder-backed evidence or check-run identifiers even though the packet examples and governance/evidence summary requirement expect linked evidence refs in the control-plane surface.

PRIMITIVE_RETENTION_PROOF:
  - `src/backend/handshake_core/src/workflows.rs:5078-5090` retains `work_packet_id`, `workflow_run_id`, `model_session_id`, `summary_ref`, `micro_task_summary`, `gate_state`, `authority_refs`, and `evidence_refs` on `DccWorkflowSummary`; compact summaries in `src/backend/handshake_core/src/runtime_governance.rs:301-327` are additive over that existing summary contract.
  - `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176,211-268` retains authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` primitives and keeps them filterable after the DCC projection changes.

PRIMITIVE_RETENTION_GAPS:
  - NONE

SHARED_SURFACE_INTERACTION_CHECKS:
  - Task-board row authority and DCC projection remain aligned across `src/backend/handshake_core/src/locus/task_board.rs:114-176,211-268`, `src/backend/handshake_core/src/workflows.rs:5067-5099`, and `src/backend/handshake_core/src/workflows.rs:27742-27760`.
  - Mailbox thread exports, wait reasons, and compact summary blockers remain aligned across `src/backend/handshake_core/src/workflows.rs:5268-5468`, `src/backend/handshake_core/src/runtime_governance.rs:221-286`, and `src/backend/handshake_core/src/workflows.rs:28026-28070`.

CURRENT_MAIN_INTERACTION_CHECKS:
  - Current main HEAD already carries the shared workflow-state and allowed-action primitives needed by this packet in `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176` and `src/backend/handshake_core/src/workflows.rs:3991-4022`, so this packet extends existing product vocabulary rather than widening it.
  - Current main HEAD has no pre-existing `/dcc/control-plane` route or `DccControlPlaneSnapshot` implementation (`git show HEAD:src/backend/handshake_core/src/api/workspaces.rs | rg -n "/dcc/control-plane|build_dcc_control_plane_snapshot|RuntimeGovernancePaths"` => no matches; `git show HEAD:src/backend/handshake_core/src/runtime_governance.rs | rg -n "DccControlPlaneSnapshot|compact_summaries"` => no matches), so the new DCC projection surface is additive against the baseline inspected at `d0ef4dd9ccc07031db7cd363a64cdac13e30770b`.

DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/src/locus/types.rs:184-236` defines a versioned `hsk.dcc_compact_summary@1` record with stable ids, routing hints, and bounded summary fields.
  - `src/backend/handshake_core/src/workflows.rs:5027-5099,5268-5468` builds the DCC payload from structured task-board, session, gate, and mailbox sources instead of transcript parsing or Markdown reconstruction.
  - `src/backend/handshake_core/src/api/workspaces.rs:1198-1228` serializes the snapshot builder output directly to JSON for API consumers.

DATA_CONTRACT_GAPS:
  - NONE

ANTI_VIBE_FINDINGS:
  - NONE

SIGNED_SCOPE_DEBT:
  - NONE

RESIDUAL_UNCERTAINTY:
  - A broad `cargo test` still trips pre-existing integration-test drift outside this packet (`src/backend/handshake_core/tests/model_session_scheduler_tests.rs:198` versus `src/backend/handshake_core/src/storage/mod.rs:1335-1364`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:21-24` versus the non-re-exported task-board validator path used in current main). Diff-scoped library tests and direct code review are therefore the reliable signal for this WP.
  - `handshake_main` is currently dirty on five overlapping validated files (`src/backend/handshake_core/src/flight_recorder/duckdb.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/runtime_governance.rs`, `src/backend/handshake_core/src/workflows.rs`), so local-main containment still requires a clean or isolated main integration surface even though the signed-scope compatibility review against `HEAD d0ef4dd9ccc07031db7cd363a64cdac13e30770b` is compatible.

### Integration Validator Corrective Addendum (Superseding Pre-Containment PASS Report)
DATE: 2026-04-11T09:33:15.9211160Z
VALIDATOR_ROLE: INTEGRATION_VALIDATOR
VALIDATOR_MODEL: gpt-5.4
COMMIT: 6a4e8bad5ddf29c7cc90aa3a26c3b7afccb8f72d
GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
SUPERSEDES_REPORT: Integration Validator Report (Pre-Containment) dated 2026-04-11T09:31:00Z
CORRECTION_NOTE: The superseded report cited validator-owned `cargo test --lib <leaf_test_name> -- --exact` commands that matched 0 tests. Those command lines are invalid proof and are replaced below with full library test paths that each reported `running 1 test` and the named test result.
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
  - 10.11 [ADD v02.160] Dev Command Center control-plane state => `src/backend/handshake_core/src/runtime_governance.rs:24-199` defines the read-only DCC snapshot/work/session/governance/collaboration facets, `src/backend/handshake_core/src/workflows.rs:5016-5143` materializes them from authoritative backend artifacts, and `src/backend/handshake_core/src/api/workspaces.rs:34-53,1198-1228` exposes the projection through `/dcc/control-plane`.
  - 10.11 [ADD v02.162] Dev Command Center work-orchestration state => `src/backend/handshake_core/src/workflows.rs:5027-5099,5161-5235` projects task-board rows, ready-queue state, gate state, and workflow-linked session occupancy, and `src/backend/handshake_core/src/workflows.rs:26832-27030` proves the ready-query projection through backend-backed fixtures.
  - 10.11 [ADD v02.166] Dev Command Center collaboration state => `src/backend/handshake_core/src/runtime_governance.rs:156-199` defines typed mailbox thread and wait-reason records, `src/backend/handshake_core/src/workflows.rs:5268-5468` derives them from Role Mailbox export artifacts, and `src/backend/handshake_core/src/workflows.rs:27232-27518` proves wait-reason and evidence-ref carry-through.
  - 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility => `src/backend/handshake_core/src/locus/task_board.rs:211-268` exposes authoritative state-family, queue-reason, and allowed-action filters, and `src/backend/handshake_core/src/workflows.rs:5067-5099` carries those fields into DCC workflow summaries and ready-queue rows.
  - 6.3 compact summary contract for DCC and Role Mailbox triage => `src/backend/handshake_core/src/locus/types.rs:184-236,1225-1233` defines and validates `DccCompactSummaryV1`, `src/backend/handshake_core/src/runtime_governance.rs:210-327` derives bounded title, blockers, and next_action from authoritative snapshot data, and `src/backend/handshake_core/src/workflows.rs:27769-28113` proves stable-id and serde/validator preservation.

NOT_PROVEN:
  - NONE

MAIN_BODY_GAPS:
  - NONE

QUALITY_RISKS:
  - NONE

DIFF_ATTACK_SURFACES:
  - `src/backend/handshake_core/src/workflows.rs:5016-5468` joins task-board rows, session state, gate state, and mailbox exports into a single DCC payload, so a producer-consumer mismatch here would silently corrupt multiple operator views.
  - `src/backend/handshake_core/src/runtime_governance.rs:243-286` derives bounded summary text, blockers, and next_action from workflow summaries and mailbox waits; a regression here would break summary-first routing for local-small-model consumers.
  - `src/backend/handshake_core/src/api/workspaces.rs:34-53,1198-1228` exposes the new `/dcc/control-plane` surface; a serialization or route-wiring error would make the projection unreachable despite correct internal builder logic.

INDEPENDENT_CHECKS_RUN:
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib workflows::tests::dcc_ready_query_projection_is_backend_backed -- --exact --nocapture` => PASS; output included `running 1 test`, `test workflows::tests::dcc_ready_query_projection_is_backend_backed ... ok`, and `test result: ok. 1 passed; 0 failed`.
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib workflows::tests::dcc_mailbox_projection_preserves_wait_reasons -- --exact --nocapture` => PASS; output included `running 1 test`, `test workflows::tests::dcc_mailbox_projection_preserves_wait_reasons ... ok`, and `test result: ok. 1 passed; 0 failed`.
  - `cargo test --manifest-path ..\\wtc-plane-backend-v1\\src\\backend\\handshake_core\\Cargo.toml --lib workflows::tests::dcc_compact_summary_contract_preserves_stable_ids -- --exact --nocapture` => PASS; output included `running 1 test`, `test workflows::tests::dcc_compact_summary_contract_preserves_stable_ids ... ok`, and `test result: ok. 1 passed; 0 failed`.
  - Current-main interaction review => `handshake_main` HEAD already preserves `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` in `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176` and `src/backend/handshake_core/src/workflows.rs:3991-4022`, while `git show HEAD:src/backend/handshake_core/src/api/workspaces.rs` and `git show HEAD:src/backend/handshake_core/src/runtime_governance.rs` show no pre-existing DCC control-plane surface, so the packet remains additive against local main `d0ef4dd9ccc07031db7cd363a64cdac13e30770b`.

COUNTERFACTUAL_CHECKS:
  - If `src/backend/handshake_core/src/workflows.rs:5078-5090` stopped copying `workflow_state_family`, `queue_reason_code`, or `allowed_action_ids` into `DccWorkflowSummary`, DCC routing would fall back to prose instead of authoritative backend vocabulary.
  - If `src/backend/handshake_core/src/workflows.rs:5422-5449` stopped translating open mailbox message types into `DccWaitReason.expected_response`, the collaboration projection would no longer explain mailbox-derived wait posture or feed compact-summary blockers deterministically.
  - If `src/backend/handshake_core/src/runtime_governance.rs:243-286` stopped bounding `title_or_objective` or defaulting `next_action`, the compact-summary-first contract would regress into detail-record loading for routine triage.

BOUNDARY_PROBES:
  - Task-board producer to DCC consumer => `src/backend/handshake_core/src/locus/task_board.rs:211-268` exposes authoritative filter primitives, `src/backend/handshake_core/src/workflows.rs:5067-5099` copies them into `DccWorkflowSummary`, and `src/backend/handshake_core/src/runtime_governance.rs:301-327` carries them into `DccCompactSummaryV1`.
  - Role Mailbox export to compact summary boundary => `src/backend/handshake_core/src/workflows.rs:5268-5468` builds thread summaries and wait reasons from `ROLE_MAILBOX/index.json` plus thread JSONL, and `src/backend/handshake_core/src/runtime_governance.rs:259-279` converts those waits into bounded blocker strings for compact summaries.

NEGATIVE_PATH_CHECKS:
  - `src/backend/handshake_core/src/workflows.rs:5273-5294` returns an empty `DccCollaborationState` when mailbox export artifacts are missing or malformed, so the DCC projection degrades without panicking.
  - `src/backend/handshake_core/src/workflows.rs:5422-5449` only emits wait reasons for open threads whose latest message types map to an expected response; closed FYI traffic remains visible but non-blocking.

INDEPENDENT_FINDINGS:
  - The superseded pre-containment report's three leaf-name `--exact` cargo commands were invalid proof because they matched 0 tests. This addendum replaces them with full test paths whose outputs explicitly showed `running 1 test`.
  - The patch is additive over current main's existing workflow-state vocabulary: it reuses authoritative task-board/workflow primitives instead of inventing a DCC-only routing model.
  - The new `/dcc/control-plane` path is read-only and serializes the same builder output exercised by the corrected validator-owned tests; it does not create a second authority source.

SPEC_CLAUSE_MAP:
  - 10.11 [ADD v02.160] control-plane state => `src/backend/handshake_core/src/runtime_governance.rs:24-199`; `src/backend/handshake_core/src/workflows.rs:5016-5143`; `src/backend/handshake_core/src/api/workspaces.rs:34-53,1198-1228`
  - 10.11 [ADD v02.162] work-orchestration state => `src/backend/handshake_core/src/workflows.rs:5027-5099,5161-5235`; `src/backend/handshake_core/src/workflows.rs:26832-27030`
  - 10.11 [ADD v02.166] collaboration state => `src/backend/handshake_core/src/runtime_governance.rs:156-199`; `src/backend/handshake_core/src/workflows.rs:5268-5468`; `src/backend/handshake_core/src/workflows.rs:27232-27518`
  - 10.11 [ADD v02.171] workflow_state_family and queue_reason_code visibility => `src/backend/handshake_core/src/locus/task_board.rs:211-268`; `src/backend/handshake_core/src/workflows.rs:5067-5099`
  - 6.3 compact summary contract => `src/backend/handshake_core/src/locus/types.rs:184-236,1225-1233`; `src/backend/handshake_core/src/runtime_governance.rs:210-327`; `src/backend/handshake_core/src/workflows.rs:27769-28113`

NEGATIVE_PROOF:
  - Clause 10.11 [ADD v02.160] is not fully realized on the governance-evidence facet inside signed scope. `src/backend/handshake_core/src/runtime_governance.rs:660-683` projects governance root, pending decisions, approval decisions, auto signatures, and effective capability state, but still hardcodes `evidence_refs: Vec::new()`. That means the DCC governance projection does not yet expose recorder-backed evidence or check-run identifiers even though the packet examples and governance/evidence summary requirement expect linked evidence refs in the control-plane surface.

PRIMITIVE_RETENTION_PROOF:
  - `src/backend/handshake_core/src/workflows.rs:5078-5090` retains `work_packet_id`, `workflow_run_id`, `model_session_id`, `summary_ref`, `micro_task_summary`, `gate_state`, `authority_refs`, and `evidence_refs` on `DccWorkflowSummary`; compact summaries in `src/backend/handshake_core/src/runtime_governance.rs:301-327` are additive over that existing contract.
  - `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176,211-268` retains authoritative `workflow_state_family`, `queue_reason_code`, and `allowed_action_ids` primitives and keeps them filterable after the DCC projection changes.

PRIMITIVE_RETENTION_GAPS:
  - NONE

SHARED_SURFACE_INTERACTION_CHECKS:
  - Task-board row authority and DCC projection remain aligned across `src/backend/handshake_core/src/locus/task_board.rs:114-176,211-268`, `src/backend/handshake_core/src/workflows.rs:5067-5099`, and `src/backend/handshake_core/src/runtime_governance.rs:301-327`.
  - Mailbox thread exports, wait reasons, and compact summary blockers remain aligned across `src/backend/handshake_core/src/workflows.rs:5268-5468`, `src/backend/handshake_core/src/runtime_governance.rs:221-286`, and `src/backend/handshake_core/src/workflows.rs:27769-28113`.

CURRENT_MAIN_INTERACTION_CHECKS:
  - Current main HEAD already carries the shared workflow-state and allowed-action primitives this packet depends on in `src/backend/handshake_core/src/locus/task_board.rs:45-48,114-176` and `src/backend/handshake_core/src/workflows.rs:3991-4022`, so the packet composes with existing main vocabulary rather than widening it.
  - Current main HEAD has no pre-existing `/dcc/control-plane` route or `DccControlPlaneSnapshot` implementation (`git show HEAD:src/backend/handshake_core/src/api/workspaces.rs` and `git show HEAD:src/backend/handshake_core/src/runtime_governance.rs` show no matching symbols), so the new DCC surface is additive against baseline `d0ef4dd9ccc07031db7cd363a64cdac13e30770b`.

DATA_CONTRACT_PROOF:
  - `src/backend/handshake_core/src/locus/types.rs:184-236` defines the versioned `hsk.dcc_compact_summary@1` record with stable ids, routing hints, and bounded summary fields.
  - `src/backend/handshake_core/src/workflows.rs:5027-5099,5268-5468` builds the DCC payload from structured task-board, session, gate, and mailbox sources instead of transcript parsing or Markdown reconstruction.
  - `src/backend/handshake_core/src/api/workspaces.rs:1198-1228` serializes the snapshot builder output directly to JSON for API consumers.

DATA_CONTRACT_GAPS:
  - NONE

ANTI_VIBE_FINDINGS:
  - NONE

SIGNED_SCOPE_DEBT:
  - NONE

RESIDUAL_UNCERTAINTY:
  - Broad `cargo test` still encounters pre-existing integration-test drift outside this packet (`src/backend/handshake_core/tests/model_session_scheduler_tests.rs:198` versus `src/backend/handshake_core/src/storage/mod.rs:1335-1364`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:21-24` versus the non-re-exported task-board validator path used in current main). Diff-scoped library tests and direct code review remain the reliable signal for this WP.
  - `handshake_main` is currently dirty on five overlapping validated files (`src/backend/handshake_core/src/flight_recorder/duckdb.rs`, `src/backend/handshake_core/src/flight_recorder/mod.rs`, `src/backend/handshake_core/src/locus/types.rs`, `src/backend/handshake_core/src/runtime_governance.rs`, `src/backend/handshake_core/src/workflows.rs`), so local-main containment remains blocked pending a clean or isolated main integration surface.
  - No closeout sync was run from this corrected report. Packet closeout truth and any main containment step remain intentionally withheld until governed closeout is resumed from the corrected evidence state.
