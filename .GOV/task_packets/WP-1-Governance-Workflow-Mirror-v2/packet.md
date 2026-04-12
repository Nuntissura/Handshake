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

# Task Packet: WP-1-Governance-Workflow-Mirror-v2

## METADATA
- TASK_ID: WP-1-Governance-Workflow-Mirror-v2
- WP_ID: WP-1-Governance-Workflow-Mirror-v2
- BASE_WP_ID: WP-1-Governance-Workflow-Mirror
- DATE: 2026-04-12T03:02:27.572Z
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Governance-Workflow-Mirror-v2
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Governance-Workflow-Mirror-v2
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-workflow-mirror-v2
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Governance-Workflow-Mirror-v2
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Governance-Workflow-Mirror-v2
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Governance-Workflow-Mirror-v2
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Governance-Workflow-Mirror-v2
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Governance-Workflow-Mirror-v2
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Governance-Workflow-Mirror-v2
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Governance-Workflow-Mirror-v2
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Governance-Workflow-Mirror-v2
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
- **Status:** In Progress
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Product-Governance-Check-Runner, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Workflow-Projection-Correlation, WP-1-Role-Mailbox
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Workflow-Transition-Automation-Registry, WP-1-Governance-Pack
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Governance-Workflow-Mirror-v2
- LOCAL_WORKTREE_DIR: ../wtc-workflow-mirror-v2
- REMOTE_BACKUP_BRANCH: feat/WP-1-Governance-Workflow-Mirror-v2
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Governance-Workflow-Mirror-v2
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v2
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v2/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v2/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v2/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja120420260458
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: MT-001 FAIL; remediation in progress on re-anchored substrate
Blockers: MT-001 validator fail on `review:WP-1-Governance-Workflow-Mirror-v2:review_request:mnv8x6en:1f43b7`; previous candidate wrote display-path refs into canonical `task_board_id` fields and was proven on a stale branch substrate. The worktree has been re-anchored to clean `c11f3c1511748ff050916dda108b3f38c3f670b4`; coder must reapply MT-001 there before MT-002.
Next: CODER remediates MT-001 on the re-anchored `../wtc-workflow-mirror-v2` branch, proves canonical `task_board_id` versus path-style `task_board_ref` separation, and returns a governed review request/handoff with exact proof.

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: 2.3.15 tracked work-packet `gates.pre_work`, `task_packet_path`, and `task_board_status` remain canonical workflow fields | CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `locus/types.rs` | TESTS: runtime-governance/workflow projection tests proving per-WP gate files and stable id carry-through | EXAMPLES: Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs., Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref., Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary., Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: 2.6.8.8 Spec Session Log + 2.6.8.9 integration hooks | CODE_SURFACES: `role_mailbox.rs`, workflow-mirror adapter in `workflows.rs` or adjacent runtime-governance service | TESTS: session-log tests proving append/query behavior and stable `spec_id`/`task_board_id`/`work_packet_id` linkage | EXAMPLES: Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs., Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref., Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary., Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: 7.5.4.8 hard repo/runtime boundary | CODE_SURFACES: `runtime_governance.rs`, any new workflow-mirror service, boundary tests | TESTS: negative-path tests proving `.GOV/` access is rejected and runtime roots stay under `.handshake/gov/` | EXAMPLES: Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs., Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref., Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary., Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: 7.5.4.9 Governance Check Runner additive overlay rule and storage boundary | CODE_SURFACES: `governance_check_runner.rs`, `governance_artifact_registry.rs`, `storage/mod.rs`, workflow-mirror linkage surfaces | TESTS: check-linkage tests proving result/evidence refs are persisted and projected without direct SQLite bypass | EXAMPLES: Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs., Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref., Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary., Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: 11.5.4 `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001` | CODE_SURFACES: `flight_recorder/mod.rs`, workflow-mirror update paths in `workflows.rs` or adjacent service | TESTS: FR payload tests validating event kind, refs, and idempotency behavior | EXAMPLES: Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs., Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref., Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary., Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
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
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs
- REQUIRED_TRIPWIRE_TESTS:
  - `governance_workflow_mirror_gate_transition_emits_fr_event`
  - `test_gov_work_packet_activated_payload_validation`
  - `governance_workflow_mirror_spec_session_log_enforces_runtime_artifact_boundary`
  - `workflows -- --nocapture`
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_workflow_mirror_gate_transition_emits_fr_event -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml test_gov_work_packet_activated_payload_validation -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_workflow_mirror_spec_session_log_enforces_runtime_artifact_boundary -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example per-WP gate artifact at `.handshake/gov/validator_gates/WP-1-Governance-Workflow-Mirror-v2.json` with stable `wp_id`, verdict, gate-state ref, and evidence/check refs.
  - Example activation-traceability record linking `base_wp_id`, `work_packet_id`, `task_board_id`, `active_packet_ref`, and runtime-owned traceability ref.
  - Example task-board projection row whose structured fields include `task_board_id`, `work_packet_id`, gate summary, and activation summary.
  - Example workflow-facing `SpecSessionLogEntry` record covering a gate transition and a stub activation event with linked runtime artifacts only.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: Canonical WP/workflow identity | SUBFEATURES: stable `task_board_id`, `work_packet_id`, `workflow_run_id`, `model_session_id` carry-through into mirror artifacts | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus remains canonical; the mirror stores foreign keys and summaries only.
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable storage contracts | SUBFEATURES: no direct SQLite outside storage, deterministic record shapes, upgrade-safe persistence | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Current trait gaps are implementation debt, not a reason to bypass the boundary.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: Structured-first governance state | SUBFEATURES: JSON-first gate artifacts, structured task-board/traceability summaries, bounded on-demand Markdown | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This directly supports local-small-model planning and replay.
  - PILLAR_DECOMPOSITION: PILLAR: RAG | CAPABILITY_SLICE: Session-log retrieval substrate | SUBFEATURES: workflow-facing Spec Session Log append/query with linked artifacts and stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This WP should preserve RAG-ready indexing semantics already named by the spec.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Per-WP validator gate mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Add runtime-owned gate artifacts under `.handshake/gov/validator_gates/` keyed by canonical WP ids.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Activation traceability mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_activation | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Record base->active packet mapping in a narrowly scoped overlay artifact without treating runtime as repo authority.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Workflow-facing Spec Session Log | JobModel: WORKFLOW | Workflow: governance_spec_session_log | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse current `spec_session_log_entries` substrate but expose workflow-facing append/query semantics.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Governed check linkage | JobModel: MECHANICAL_TOOL | Workflow: governance_check_runner | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse CheckRunner and registry foundations; mirror stores summaries/evidence refs, not raw tool execution state.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Runtime governance query/projection surface | JobModel: UI_ACTION | Workflow: runtime_governance_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extend current task-board/work-packet projection surface instead of introducing a parallel query stack.
  - FORCE_MULTIPLIER_EXPANSION: Locus + Activation Traceability -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: Locus Keys + Storage Boundary -> IN_THIS_WP (stub: NONE)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Schema-Registry-v4)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Workflow-Projection-Correlation-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Governance-Workflow-Mirror-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.3.15 Locus Work Tracking System (tracked work packet gate/task packet linkage)
- CONTEXT_START_LINE: 6134
- CONTEXT_END_LINE: 6143
- CONTEXT_TOKEN: validator_gates/{WP_ID}.json
- EXCERPT_ASCII_ESCAPED:
  ```text
// Gate status (Validator integration)
  gates: {
    pre_work: GateStatus;            // From .handshake/gov/validator_gates/{WP_ID}.json
    post_work: GateStatus;
  };

  // Task Packet reference
  task_packet_path?: string;         // ".handshake/gov/task_packets/WP-1-Auth.md"
  task_board_status: TaskBoardStatus;
  };
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.6.8.8 Spec Session Log (Task Board + Work Packets)
- CONTEXT_START_LINE: 10511
- CONTEXT_END_LINE: 10543
- CONTEXT_TOKEN: SpecSessionLogEntry
- EXCERPT_ASCII_ESCAPED:
  ```text
Task Board items and Work Packets together form a Spec Session Log that runs in parallel to the Flight Recorder. Flight Recorder remains the authoritative system log; the Spec Session Log captures human-facing planning state and is used for context offload.

  pub struct SpecSessionLogEntry {
      pub entry_id: String,
      pub spec_id: String,
      pub task_board_id: String,
      pub work_packet_id: Option<String>,
      pub event_type: String,
      pub governance_mode: GovernanceMode,
      pub actor: String,
      pub timestamp: DateTime<Utc>,
      pub summary: String,
      pub linked_artifacts: Vec<ArtifactHandle>,
  }

  Rules:
  - Every Task Board or Work Packet change MUST emit a SpecSessionLogEntry stored in the workspace and indexed for RAG.
  - The Spec Session Log MUST NOT replace Flight Recorder; it is a parallel, human-facing ledger.
  - Spec Session Log entries MUST reference the same spec_id and work_packet_id used in SpecIntent and WorkPacketBinding.
  - SpecSessionLogEntry.entry_id MUST be unique within the workspace.
  - SpecSessionLogEntry.governance_mode MUST match the active mode at the time of the event; mode transitions require a dedicated entry.
  - [ADD v02.163] Task Board entries and Work Packet artifacts MUST remain separately queryable coordination surfaces: Task Board is the human-readable mirror, while Work Packet artifacts preserve scoped execution contracts, workflow-linked activation, and session binding state.
  - [ADD v02.163] Dev Command Center, Locus Work Tracking, Workflow Engine, and Model Session Orchestration projections MUST preserve stable task_board_id, work_packet_id, workflow_run_id, and model_session_id values so parallel-model planning never depends on manual board interpretation.
  - [ADD v02.166] Work Packet and Task Board artifacts SHOULD expose canonical structured representations that are cheap to filter, route, and replay. Human-readable Markdown mirrors MAY remain, but they MUST be derived from or reconciled against the same stable identifiers and field values.
  - [ADD v02.166] Local-small-model planning and execution SHOULD read bounded structured fields first and only load long-form notes or Markdown mirrors on demand.

  #### 2.6.8.9 Integration Hooks (Normative)

  - Flight Recorder logs every router decision, refinement pass, signature, gate outcome, and spec status change.
    - Governance gate transitions MUST emit `FR-EVT-GOV-GATES-001`.
    - Stub activation (stub -> official packet + traceability mapping) MUST emit `FR-EVT-GOV-WP-001`.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.8 Repo/runtime boundary (HARD)
- CONTEXT_START_LINE: 31897
- CONTEXT_END_LINE: 31901
- CONTEXT_TOKEN: Runtime governance state MUST live in product-owned storage
- EXCERPT_ASCII_ESCAPED:
  ```text
**Repo/runtime boundary (HARD)**
  - `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
  - `docs/` MAY exist as a temporary compatibility bundle only (non-authoritative governance state).
  - Handshake product runtime MUST NOT read from or write to `/.GOV/` (hard boundary; enforce via CI/gates).
  - Runtime governance state MUST live in product-owned storage. Handshake default: `.handshake/gov/` (configurable). This directory contains runtime governance state only.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.9 Governance Check Runner: Bounded Execution Contract
- CONTEXT_START_LINE: 31907
- CONTEXT_END_LINE: 31951
- CONTEXT_TOKEN: Additive Overlay Rule
- EXCERPT_ASCII_ESCAPED:
  ```text
**Purpose**
  Define a bounded, observable execution layer for imported governance checks so that Handshake validates software-delivery workflows through capability-gated, recorder-visible, product-owned execution instead of raw shell bypass.

  **Definitions**
  - **CheckDescriptor**: a typed execution record derived from a `GovernanceArtifactRegistryEntry` with `kind=Checks` or `kind=Rubrics`. It carries the check identifier, required capabilities, timeout_ms, input schema, and version provenance from the registry.
  - **CheckResult**: a typed result contract with exactly five variants:
    - `PASS` -- check completed and all assertions satisfied
    - `FAIL` -- check completed and one or more assertions failed
    - `BLOCKED` -- check could not execute due to capability denial or precondition failure
    - `ADVISORY_ONLY` -- check completed but findings are informational and do not gate progress
    - `UNSUPPORTED` -- check kind or descriptor version is not executable in the current runtime
  - **CheckRunner**: the product service that executes a `CheckDescriptor` through the Unified Tool Surface Contract and produces a `CheckResult` with evidence.

  **Execution Lifecycle**
  The CheckRunner MUST implement a three-phase bounded lifecycle:
  1. **PreCheck**: validate `CheckDescriptor` schema, verify required capabilities through `CapabilityGate`, and confirm `timeout_ms` is within runtime bounds. Failure here MUST produce `CheckResult::Blocked` immediately without proceeding to execution.
  2. **Check**: invoke the check body through the `governance.check.run` tool surface. Execution is bounded by `timeout_ms`. A timeout or execution error MUST produce `CheckResult::Blocked`.
  3. **PostCheck**: capture the raw result, map it to the `CheckResult` enum, store evidence artifacts with content hash, and emit Flight Recorder events.

  **Tool Surface**
  The `governance.check.run` tool_id MUST be registered under the Unified Tool Surface Contract (6.0.2) with:
  - `side_effect: GOVERNED_WRITE`
  - Required capabilities declared in the `CheckDescriptor`
  - Input schema: `{ check_id: string, descriptor_ref: string, input_args: object }`
  - Output schema: `CheckResult` JSON

  **Flight Recorder Events**
  Every check execution MUST emit the following FR events:
  - `FR-EVT-GOV-CHECK-001` (`governance.check.started`): payload includes `check_id`, `session_id`, `check_descriptor_hash`
  - `FR-EVT-GOV-CHECK-002` (`governance.check.completed`): payload includes `check_id`, `session_id`, `result_status`, `duration_ms`, `evidence_artifact_id`
  - `FR-EVT-GOV-CHECK-003` (`governance.check.blocked`): payload includes `check_id`, `session_id`, `blocked_reason`

  FR events MUST be emitted for all result variants including `BLOCKED` and `UNSUPPORTED`. Silent skip is prohibited.

  **Additive Overlay Rule**
  Imported governance checks MUST extend the product governance surface additively. No imported check MAY:
  - overwrite or disable native Handshake governance rules
  - modify base-envelope structured collaboration records
  - acquire capabilities beyond those declared in its `CheckDescriptor`

  **Unsupported Checks**
  A check descriptor with an unrecognized `kind`, unsupported schema version, or missing required execution surface MUST return `CheckResult::Unsupported` with an explicit reason string. Silent skip is prohibited.

  **Storage**
  All `CheckDescriptor` and `CheckResult` persistence MUST go through the `Database` trait boundary. No direct SQLite calls outside the storage module are permitted.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 7.5.4.11 Product Governance Snapshot (HARD)
- CONTEXT_START_LINE: 46952
- CONTEXT_END_LINE: 46999
- CONTEXT_TOKEN: wp_gate_summaries
- EXCERPT_ASCII_ESCAPED:
  ```text
**Purpose**
  Provide a deterministic, leak-safe snapshot of the current governance state for a product/repo so a fresh agent (or auditor) can reconstruct "what is true" without relying on chat history.

  **Definition**
  A "Product Governance Snapshot" is a machine-readable JSON export derived ONLY from canonical governance artifacts (no repo scan; no extras):
  - `.GOV/spec/SPEC_CURRENT.md`
  - resolved spec file referenced inside it (e.g., `Handshake_Master_Spec_v02.125.md`)
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`
  - `.GOV/validator_gates/*.json` (if present)

  **Output location (HARD)**
  - Default path: `.GOV/roles_shared/runtime/PRODUCT_GOVERNANCE_SNAPSHOT.json`
  - The export MUST be deterministic for a given set of input files.
  - The export MUST NOT include wall-clock timestamps.
  - The export MAY include the current git HEAD sha (if available) as provenance.
  - The output bytes MUST be `JSON.stringify(obj, null, 2) + "\n"` (force `\n` newlines; no locale formatting).

  **Determinism (HARD)**
  - Generator MUST enforce stable ordering:
    - `inputs` sorted by `path` (ascending).
    - `task_board.entries` sorted by `wp_id` (ascending).
    - `traceability.mappings` sorted by `base_wp_id` (ascending).
    - `signatures.consumed` sorted by `signature` (ascending).
    - `gates.validator.wp_gate_summaries` sorted by `wp_id` (ascending) if present.
  - Generator MUST avoid locale/time dependent formatting (no wall clock calls).

  **Minimum schema (normative)**
  ProductGovernanceSnapshot
  - schema_version: "hsk.product_governance_snapshot@0.1"
  - spec: { spec_target: string, spec_sha1: string }
  - git: { head_sha?: string } (generator SHOULD default to `git: {}`; omit head_sha unless explicitly enabled)
  - inputs: [{ path: string, sha256: string }]
  - task_board: { entries: [{ wp_id: string, status_token: string }] }
  - traceability: { mappings: [{ base_wp_id: string, active_packet_path: string }] }
  - signatures: { consumed: [{ signature: string, purpose: string, wp_id?: string }] }
  - gates: { orchestrator: { last_refinement?: string, last_signature?: string, last_prepare?: string }, validator: { wp_gate_summaries?: [{ wp_id: string, verdict?: string, status?: string, gates_passed?: string[] }] } }
    - `wp_gate_summaries` MUST be a list (not a map/object) and MUST omit timestamps and raw logs/bodies.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 11.5.4 FR-EVT-GOV-GATES-001 / FR-EVT-GOV-WP-001
- CONTEXT_START_LINE: 67220
- CONTEXT_END_LINE: 67249
- CONTEXT_TOKEN: traceability_registry_ref
- EXCERPT_ASCII_ESCAPED:
  ```text
// FR-EVT-GOV-GATES-001
  interface GovernanceGateTransitionEvent extends FlightRecorderEventBase {
    type: 'gov_gate_transition';

    spec_id: string | null;
    work_packet_id: string | null;

    role: 'operator' | 'orchestrator' | 'coder' | 'validator' | 'system';
    gate_kind: 'orchestrator' | 'validator';
    gate: string;                     // e.g. REPORT_PRESENTED, USER_ACKNOWLEDGED, WP_APPENDED, COMMITTED, REFINE_RECORDED, SIGNATURE_RECORDED, PREPARE_RECORDED
    verdict?: 'PASS' | 'FAIL' | null;  // REQUIRED for REPORT_PRESENTED; otherwise null/omitted

    gate_state_ref: string;           // e.g. .handshake/gov/validator_gates/WP-1-Example-v1.json (or other artifact handle)
    idempotency_key: string;
  }

  // FR-EVT-GOV-WP-001
  interface WorkPacketActivatedEvent extends FlightRecorderEventBase {
    type: 'gov_work_packet_activated';

    spec_id: string | null;
    base_wp_id: string;
    work_packet_id: string;

    stub_packet_ref: string;          // e.g. .handshake/gov/task_packets/stubs/WP-...md
    active_packet_ref: string;         // e.g. .handshake/gov/task_packets/WP-...md

    traceability_registry_ref: string; // e.g. .handshake/gov/WP_TRACEABILITY_REGISTRY.md
    task_board_ref: string;            // e.g. .handshake/gov/TASK_BOARD.md
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: 2.3.15 tracked work-packet `gates.pre_work`, `task_packet_path`, and `task_board_status` remain canonical workflow fields | WHY_IN_SCOPE: The workflow mirror must extend these fields with runtime-owned validator-gate and activation artifacts without replacing Locus authority | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, `workflows.rs`, `locus/types.rs` | EXPECTED_TESTS: runtime-governance/workflow projection tests proving per-WP gate files and stable id carry-through | RISK_IF_MISSED: split-brain state between Locus and runtime mirror
  - CLAUSE: 2.6.8.8 Spec Session Log + 2.6.8.9 integration hooks | WHY_IN_SCOPE: Gate transitions and stub activation must append human-facing ledger entries and remain separately queryable from Flight Recorder | EXPECTED_CODE_SURFACES: `role_mailbox.rs`, workflow-mirror adapter in `workflows.rs` or adjacent runtime-governance service | EXPECTED_TESTS: session-log tests proving append/query behavior and stable `spec_id`/`task_board_id`/`work_packet_id` linkage | RISK_IF_MISSED: operators and models lose the required parallel planning ledger
  - CLAUSE: 7.5.4.8 hard repo/runtime boundary | WHY_IN_SCOPE: The product runtime mirror must be product-owned and must not read/write `/.GOV/` | EXPECTED_CODE_SURFACES: `runtime_governance.rs`, any new workflow-mirror service, boundary tests | EXPECTED_TESTS: negative-path tests proving `.GOV/` access is rejected and runtime roots stay under `.handshake/gov/` | RISK_IF_MISSED: the implementation violates a hard spec boundary
  - CLAUSE: 7.5.4.9 Governance Check Runner additive overlay rule and storage boundary | WHY_IN_SCOPE: This WP should reuse typed check execution/results and persist summaries through existing boundaries, not invent a side channel | EXPECTED_CODE_SURFACES: `governance_check_runner.rs`, `governance_artifact_registry.rs`, `storage/mod.rs`, workflow-mirror linkage surfaces | EXPECTED_TESTS: check-linkage tests proving result/evidence refs are persisted and projected without direct SQLite bypass | RISK_IF_MISSED: governed check state drifts from the runtime mirror or bypasses the storage contract
  - CLAUSE: 11.5.4 `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001` | WHY_IN_SCOPE: The workflow mirror is the runtime surface that must emit those governance events on state change | EXPECTED_CODE_SURFACES: `flight_recorder/mod.rs`, workflow-mirror update paths in `workflows.rs` or adjacent service | EXPECTED_TESTS: FR payload tests validating event kind, refs, and idempotency behavior | RISK_IF_MISSED: governance transitions become invisible to the authoritative system log
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `.handshake/gov/validator_gates/{wp_id}.json` gate artifact | PRODUCER: workflow-mirror runtime service | CONSUMER: task-board/work-packet projections, Command Center governance views, validator logic | SERIALIZER_TRANSPORT: deterministic JSON file under runtime governance root | VALIDATOR_READER: runtime-governance path helpers plus projection tests | TRIPWIRE_TESTS: per-WP isolation, deterministic serialization, verdict transition event emission | DRIFT_RISK: parallel WPs collide or UI reads stale/malformed gate state
  - CONTRACT: runtime activation-traceability record | PRODUCER: stub-activation workflow path | CONSUMER: task-board mirror, FR event payload, operator audit views | SERIALIZER_TRANSPORT: structured runtime artifact plus optional human-readable mirror | VALIDATOR_READER: workflow projection and FR payload tests | TRIPWIRE_TESTS: base->active mapping correctness, stable refs, idempotent re-activation handling | DRIFT_RISK: active packet lineage becomes ambiguous
  - CONTRACT: workflow-facing Spec Session Log entry | PRODUCER: gate transition and activation mirror updates | CONSUMER: RAG/context offload, operator timelines, workflow replay/debug | SERIALIZER_TRANSPORT: database-backed record with stable structured fields and linked artifact refs | VALIDATOR_READER: role-mailbox/session-log query tests | TRIPWIRE_TESTS: append/query by `spec_id` + `task_board_id` + `work_packet_id`, no mailbox-body leakage | DRIFT_RISK: human-facing ledger loses continuity or leaks unrelated mailbox data
  - CONTRACT: governed check run linkage | PRODUCER: CheckRunner + workflow-mirror linkage layer | CONSUMER: runtime governance views, WP gate summaries, validator workflows | SERIALIZER_TRANSPORT: database row plus evidence/hash refs in mirror summaries | VALIDATOR_READER: storage trait tests and projection tests | TRIPWIRE_TESTS: result/evidence refs survive persistence and projection without raw tool-log duplication | DRIFT_RISK: mirror shows check status unsupported by stored evidence
  - CONTRACT: task-board/work-packet mirror summaries | PRODUCER: workflow projection layer | CONSUMER: Command Center, local/cloud models, operator audits | SERIALIZER_TRANSPORT: structured projection JSON plus human-readable Markdown mirror | VALIDATOR_READER: workflow/task-board projection tests | TRIPWIRE_TESTS: stable ids, gate summary presence, activation summary presence, no second task-board authority | DRIFT_RISK: human-readable mirrors and structured records diverge silently
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Start from current `main`, not from the historical branch snapshot.
  - Port only the missing branch-only workflow-mirror slice: `GovGateTransition`, structured gate/activation summary types, and the projection wiring that consumes them.
  - Preserve the current-main `gov_work_packet_activated` path and the existing workflow-facing Spec Session Log seam in `role_mailbox.rs`.
  - Extend task-board/work-packet projections and FR emission paths without replaying stale whole-file versions from the historical branch.
  - Add deterministic tests and negative boundary checks after the minimal port is in place.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs
- TRIPWIRE_TESTS:
  - `governance_workflow_mirror_gate_transition_emits_fr_event`
  - `test_gov_work_packet_activated_payload_validation`
  - `governance_workflow_mirror_spec_session_log_enforces_runtime_artifact_boundary`
  - `workflows -- --nocapture`
- CARRY_FORWARD_WARNINGS:
  - Do not widen base Locus families to absorb overlay-only workflow mirror facts.
  - Do not create a second task-board authority; extend the current runtime task-board/work-packet projection surface.
  - Do not let runtime read or write `/.GOV/`.
  - Do not overwrite current-main `SessionSpawnRequested`, `SessionCheckpointCreated`, or `WorkspaceIsolationDenied` behavior by replaying stale branch code.
  - Keep structured-first records canonical and Markdown mirrors derived or reconciled.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - 2.3.15 tracked work-packet gate/task-packet/task-board linkage
  - 2.6.8.8 Spec Session Log + 2.6.8.9 integration hooks
  - 7.5.4.8 repo/runtime hard boundary
  - 7.5.4.9 CheckRunner additive overlay/storage boundary
  - 11.5.4 `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001`
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs
  - ../handshake_main/src/backend/handshake_core/src/governance_check_runner.rs
  - ../handshake_main/src/backend/handshake_core/src/governance_artifact_registry.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
- COMMANDS_TO_RUN:
  - `rg -n "validator_gates|WP_TRACEABILITY_REGISTRY|spec_session_log|FR-EVT-GOV|GovernanceCheckRun" ../handshake_main/src/backend/handshake_core/src`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml runtime_governance -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml governance_check_runner -- --nocapture`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture`
- POST_MERGE_SPOTCHECKS:
  - Verify runtime-generated gate refs stay under `.handshake/gov/validator_gates/`.
  - Verify activation traceability and task-board/work-packet projections agree on the same WP/task-board/workflow ids.
  - Verify Spec Session Log entries exist for gate transitions and activation without leaking mailbox message bodies.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The exact final runtime artifact format for activation traceability (single structured artifact vs structured artifact plus Markdown mirror) is not proven in code yet, only bounded semantically by this refinement.
  - The storage-backed implementation depth for `create_governance_check_run` and `list_governance_check_runs` is not proven; only the trait seam and structs exist today.
  - The final Command Center operator surface is not proven; this refinement only establishes the backend/runtime visibility requirements.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE. Local authoritative evidence was sufficient and external current-signal scanning would not materially improve this WP.
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
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
  - RAG
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Flight Recorder + Gate Mirror -> IN_THIS_WP (stub: NONE)
  - Locus + Activation Traceability -> IN_THIS_WP (stub: NONE)
  - Runtime Governance + Durable Gate Storage -> IN_THIS_WP (stub: NONE)
  - SQLite-now Contract + Postgres-ready Shapes -> IN_THIS_WP (stub: NONE)
  - Structured Gate Summaries + Small-Model Routing -> IN_THIS_WP (stub: NONE)
  - Session Log Retrieval + Workflow Replay -> IN_THIS_WP (stub: NONE)
  - Flight Recorder + Idempotent Gate Transitions -> IN_THIS_WP (stub: NONE)
  - Runtime Projection + Structured Session Log -> IN_THIS_WP (stub: NONE)
  - Locus Keys + Storage Boundary -> IN_THIS_WP (stub: NONE)
  - Structured Retrieval + Gate Evidence -> IN_THIS_WP (stub: NONE)
  - Command Center Consumer Readiness -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Canonical WP/workflow identity | SUBFEATURES: stable `task_board_id`, `work_packet_id`, `workflow_run_id`, `model_session_id` carry-through into mirror artifacts | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus remains canonical; the mirror stores foreign keys and summaries only.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Governance gate and activation telemetry | SUBFEATURES: `FR-EVT-GOV-GATES-001`, `FR-EVT-GOV-WP-001`, check-run linkage | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Recorder visibility is a hard requirement.
  - PILLAR: Work packets (product, not repo) | CAPABILITY_SLICE: Activation-bound runtime packet mirror | SUBFEATURES: active packet refs, activation mappings, check summary attachment | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Reuse current runtime work-packet projections rather than creating new packet primitives.
  - PILLAR: Task board (product, not repo) | CAPABILITY_SLICE: Human-readable governance mirror | SUBFEATURES: gate summaries, activation visibility, stable task-board routing | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: The task board stays the human-readable mirror, not the canonical work-state authority.
  - PILLAR: Command Center | CAPABILITY_SLICE: Operator governance inspection | SUBFEATURES: WP filter, verdict filter, activation card, last check run, session-log timeline | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Backend-first in this WP, but visibility requirements are real.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Governed runtime updates | SUBFEATURES: mirror synchronization, check-run recording, bounded mutation points | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Runtime updates must stay behind product-owned services and database traits.
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Durable storage contracts | SUBFEATURES: no direct SQLite outside storage, deterministic record shapes, upgrade-safe persistence | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.dba, engine.version | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Current trait gaps are implementation debt, not a reason to bypass the boundary.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Structured-first governance state | SUBFEATURES: JSON-first gate artifacts, structured task-board/traceability summaries, bounded on-demand Markdown | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This directly supports local-small-model planning and replay.
  - PILLAR: RAG | CAPABILITY_SLICE: Session-log retrieval substrate | SUBFEATURES: workflow-facing Spec Session Log append/query with linked artifacts and stable ids | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.archivist, engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: This WP should preserve RAG-ready indexing semantics already named by the spec.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Per-WP validator gate mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_sync | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Add runtime-owned gate artifacts under `.handshake/gov/validator_gates/` keyed by canonical WP ids.
  - Capability: Activation traceability mirror | JobModel: WORKFLOW | Workflow: governance_workflow_mirror_activation | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Record base->active packet mapping in a narrowly scoped overlay artifact without treating runtime as repo authority.
  - Capability: Workflow-facing Spec Session Log | JobModel: WORKFLOW | Workflow: governance_spec_session_log | ToolSurface: COMMAND_CENTER | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-GATES-001, FR-EVT-GOV-WP-001 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse current `spec_session_log_entries` substrate but expose workflow-facing append/query semantics.
  - Capability: Governed check linkage | JobModel: MECHANICAL_TOOL | Workflow: governance_check_runner | ToolSurface: UNIFIED_TOOL_SURFACE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-GOV-CHECK-001, FR-EVT-GOV-CHECK-002, FR-EVT-GOV-CHECK-003 | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Reuse CheckRunner and registry foundations; mirror stores summaries/evidence refs, not raw tool execution state.
  - Capability: Runtime governance query/projection surface | JobModel: UI_ACTION | Workflow: runtime_governance_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extend current task-board/work-packet projection surface instead of introducing a parallel query stack.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Governance-Workflow-Mirror-v2 -> EXPAND_IN_THIS_WP
  - WP-1-Product-Governance-Artifact-Registry-v1 -> REUSE_EXISTING
  - WP-1-Product-Governance-Check-Runner-v1 -> REUSE_EXISTING
  - WP-1-Structured-Collaboration-Schema-Registry-v4 -> REUSE_EXISTING
  - WP-1-Workflow-Projection-Correlation-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Role-Mailbox-v1 -> REUSE_EXISTING
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs -> IMPLEMENTED (NONE)
  - ../handshake_main/src/backend/handshake_core/src/governance_artifact_registry.rs -> IMPLEMENTED (WP-1-Product-Governance-Artifact-Registry-v1)
  - ../handshake_main/src/backend/handshake_core/src/governance_check_runner.rs -> IMPLEMENTED (WP-1-Product-Governance-Check-Runner-v1)
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> IMPLEMENTED (WP-1-Structured-Collaboration-Schema-Registry-v4)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> IMPLEMENTED (WP-1-Role-Mailbox-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Product-Governance-Check-Runner-v1)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Workflow-Projection-Correlation-v1)
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs -> PARTIAL (NONE)
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
- What: Finish the missing workflow-mirror parity slice on current `main`: add the gate-transition FR event and payload validation, restore structured gate/activation summary types, and wire those summaries into the current workflow/task-board projection path without replaying stale whole-file branch snapshots.
- Why: Current product already contains workflow-facing session-log support and the activation-side `gov_work_packet_activated` path, but it still lacks the gate-transition and structured-summary half of the original workflow-mirror design. This remediation closes that specific gap while preserving later mainline runtime behavior.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
- OUT_OF_SCOPE:
  - Any runtime code that reads from or writes to `/.GOV/`
  - Replacing Locus as the canonical work-tracking authority
  - Replaying the historical `v1` branch wholesale onto current `main`
  - A broad repo-governance mirror beyond validator gates, activation traceability, and workflow-facing session-log state
  - New Master Spec text, appendix updates, or packet creation in this turn
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
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib governance_workflow_mirror_gate_transition_emits_fr_event -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib test_gov_work_packet_activated_payload_validation -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture
```

### DONE_MEANS
- Runtime-owned validator-gate artifacts exist per WP without collisions across parallel WPs.
- Current-main activation traceability behavior stays intact and continues to point `traceability_registry_ref` at the shared runtime registry.
- Gate transitions append workflow-facing Spec Session Log entries and emit the required `gov_gate_transition` / `FR-EVT-GOV-GATES-001` event.
- Structured gate-summary and activation-summary data are visible through the current workflow/task-board projection surfaces without creating a second authority.
- No implementation path reads from or writes to `/.GOV/`.

- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-12T03:02:27.572Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- SPEC_ANCHOR_PRIMARY: 2.3.15 tracked work-packet gates/task-packet linkage + 2.6.8.8 Spec Session Log + 7.5.4.8 repo/runtime boundary + 7.5.4.9 CheckRunner + 11.5.4 FR governance events
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
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs
  - ../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - validator_gates
  - gov_gate_transition
  - GovGateTransition
  - WorkflowMirrorGateSummary
  - WorkflowMirrorActivationSummary
  - activation_summary
  - gate_summary
  - spec_session_log
  - task_board_id
  - work_packet_id
- RUN_COMMANDS:
  ```bash
rg -n "gov_gate_transition|GovGateTransition|WorkflowMirrorGateSummary|WorkflowMirrorActivationSummary|activation_summary|gate_summary" ../handshake_main/src/backend/handshake_core/src ../wtc-workflow-mirror-v1/src/backend/handshake_core/src
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib governance_workflow_mirror_gate_transition_emits_fr_event -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml --lib test_gov_work_packet_activated_payload_validation -- --exact --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox -- --nocapture
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml workflows -- --nocapture
  ```
- RISK_MAP:
  - "whole-file replay from historical v1 overwrites later current-main runtime behavior" -> "regression in session spawn/checkpoint/workspace isolation flows"
  - "mirror widens into a general repo-governance clone" -> "split authority and boundary violation"
  - "gate state not keyed by canonical WP/task-board ids" -> "parallel-WP collisions and untrustworthy UI state"
  - "session-log seam is reimplemented instead of reused" -> "workflow mirror drifts from the already-contained current-main behavior"
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
  - LOG_PATH: `.handshake/logs/WP-1-Governance-Workflow-Mirror-v2/<name>.log` (recommended; not committed)
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

## LIVE_EXECUTION_LOG

- [2026-04-12 06:35:18 Europe/Brussels] [ORCHESTRATOR] [NOTE] [TOPOLOGY] Pushed backup branch origin/feat/WP-1-Governance-Workflow-Mirror-v2 at fe4ebbd7 before topology repair, then removed the stale shared-repo worktree and recreated ../wtc-workflow-mirror-v2 as a clean handshake_main-based branch at c11f3c1511748ff050916dda108b3f38c3f670b4 with the same WP branch name and a fresh .GOV junction back to wt-gov-kernel.
