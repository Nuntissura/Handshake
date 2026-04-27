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

# Task Packet: WP-1-Software-Delivery-Projection-Surface-Discipline-v1

## METADATA
- TASK_ID: WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- WP_ID: WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- BASE_WP_ID: WP-1-Software-Delivery-Projection-Surface-Discipline
- DATE: 2026-04-27T16:46:05.420Z
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
- CODER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: claude-opus-4-7
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
- CODER_RESUME_COMMAND: just coder-next WP-1-Software-Delivery-Projection-Surface-Discipline-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.5
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-surface-discipline-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Software-Delivery-Projection-Surface-Discipline-v1
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
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-Control-Plane-Backend, WP-1-Project-Agnostic-Workflow-State-Registry, WP-1-Structured-Collaboration-Artifact-Family, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Role-Mailbox
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: YES
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- LOCAL_WORKTREE_DIR: ../wtc-surface-discipline-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja270420261840
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Projection-surface discipline | CODE_SURFACES: workflows.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs, runtime_governance.rs | TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Software-delivery overlay runtime truth specialization | CODE_SURFACES: locus/types.rs, locus/task_board.rs, workflows.rs | TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Software-delivery closeout derivation | CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Software-delivery overlay extension records and lifecycle semantics | CODE_SURFACES: workflows.rs, locus/types.rs, role_mailbox.rs | TESTS: projection_surface_exposes_claim_and_queued_instruction_ids | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Role Mailbox authority boundary | CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs, workflows.rs | TESTS: role_mailbox_software_delivery_triage_remains_advisory | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - `dcc_software_delivery_projection_surface_keeps_runtime_authority`
  - `task_board_software_delivery_projection_cannot_override_runtime_truth`
  - `role_mailbox_software_delivery_triage_remains_advisory`
  - `projection_surface_previews_governed_action_before_mutation`
  - `closeout_projection_requires_gate_evidence_and_owner_truth`
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml projection_surface_previews_governed_action_before_mutation -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact`
- CANONICAL_CONTRACT_EXAMPLES:
  - Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth.
  - Example Task Board row with stale mirror state but runtime validator-gate blocker winning.
  - Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution.
  - Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id.
  - Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action.
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: Software-delivery runtime truth substrate | SUBFEATURES: canonical structured records, workflow-state family, queue-reason code, allowed action ids, gate posture, checkpoint/evidence refs | PRIMITIVES_FEATURES: FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.librarian, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed records remain the join target for DCC, Task Board, and Role Mailbox projections.
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact summary first projection | SUBFEATURES: compact software-delivery summary rows, authority refs, evidence refs, next action previews, linked mailbox and gate ids | PRIMITIVES_FEATURES: FEAT-LLM-FRIENDLY-DATA | MECHANICAL: engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Local-small-model routing should consume compact structured records before raw Markdown mirrors.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Software-delivery projection discipline snapshot | JobModel: UI_ACTION | Workflow: dcc_software_delivery_projection_snapshot | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: work_packet_gated, task_board_synced | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC should read one runtime-backed projection that carries validator-gate, claim/lease, queued-instruction, recovery, closeout, and authority refs.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Task Board runtime-authority projection guard | JobModel: UI_ACTION | Workflow: locus_task_board_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: task_board_status_changed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Board rows can group or sort work, but validation, ownership, recovery, and closeout posture must come from runtime fields.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Role Mailbox linked-authority preview | JobModel: UI_ACTION | Workflow: role_mailbox_software_delivery_triage | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Mailbox replies and unread order stay advisory until governed action or transcription updates linked runtime records.
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Workflow-backed overlay lifecycle projection | JobModel: WORKFLOW | Workflow: software_delivery_overlay_lifecycle | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: workflow_gate_transition | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: start, steer, cancel, close, and recover must expose explicit runtime states and stable ids before mutation.
  - FORCE_MULTIPLIER_EXPANSION: DCC snapshot plus Locus runtime truth -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: Runtime storage plus projection field provenance -> IN_THIS_WP (stub: NONE)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v4)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Software-Delivery-Projection-Surface-Discipline-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 10.11 Dev Command Center integration [ADD v02.181]
- CONTEXT_START_LINE: 6678
- CONTEXT_END_LINE: 6680
- CONTEXT_TOKEN: mirror freshness alone
- EXCERPT_ASCII_ESCAPED:
  ```text
- DCC is the canonical operator/developer surface to **view** Locus WPs/MTs and bind a **worktree-backed workspace** to a `wp_id`/`mt_id`/`session_id` context.
  - DCC MUST NOT become an alternate authority for work status; it MUST read/write via `locus_*` operations and treat `.handshake/gov/TASK_BOARD.md` as the human-readable mirror.
  - [ADD v02.181] For `project_profile_kind=software_delivery`, Dev Command Center SHOULD project work contract state, workflow-binding state, pending governed actions, validator-gate posture, checkpoint lineage, evidence readiness, claim/lease posture, queued follow-up instructions, binding health, stale detection, and backpressure posture from canonical runtime records. Dev Command Center MAY start, steer, cancel, close, or recover those records only through governed actions or workflow-backed control-plane mutations, and it MUST NOT infer authority from layout position, unread state, or mirror freshness alone.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 compact summary and structured projection contract
- CONTEXT_START_LINE: 6878
- CONTEXT_END_LINE: 6888
- CONTEXT_TOKEN: compact summary contract
- EXCERPT_ASCII_ESCAPED:
  ```text
- Dev Command Center, Task Board derived layouts, Role Mailbox triage, and local-small-model planning flows SHOULD default to the compact summary contract first and load canonical detail records or Markdown sidecars only on demand.

  **Task Board and Role Mailbox structured projections** [ADD v02.168]

  - Task Board projection rows SHOULD be serialized as `record_kind=task_board_entry` records that reuse the same base envelope and add only board-specific fields such as `task_board_id`, `work_packet_id`, `lane_id`, `display_order`, and optional `view_ids`.
  - Role Mailbox exports SHOULD reuse the same base envelope for thread indexes and append-only thread lines. Message-level records SHOULD add only mailbox-specific fields such as `thread_id`, `message_type`, `from_role`, `to_roles`, `expected_response`, and `expires_at`.
  - When a collaboration artifact supports both canonical detail and compact summary representations, both records MUST share the same `record_id`, `project_profile_kind`, and authoritative references so deterministic joins remain possible without transcript reconstruction or Markdown parsing.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 software-delivery overlay runtime truth specialization [ADD v02.181]
- CONTEXT_START_LINE: 6915
- CONTEXT_END_LINE: 6925
- CONTEXT_TOKEN: authoritative work meaning
- EXCERPT_ASCII_ESCAPED:
  ```text
**Software-delivery overlay runtime truth specialization** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative work meaning MUST resolve through canonical structured records instead of packet prose, board ordering, mailbox chronology, or side-ledger files.
  - Software-delivery structured collaboration state MUST preserve, at minimum, canonical truth for scoped work contract semantics, workflow binding semantics, governed action request/resolution posture, validator-gate posture, and checkpoint/evidence references.
  - Readable task-packet Markdown, Task Board mirrors, and mailbox summaries MAY remain source artifacts and human-readable projections, but they MUST NOT act as the mutable operational ledger for software-delivery execution.
  - Software-delivery-specific fields SHOULD remain profile extensions or profile-specialized records over the shared base envelope so the shared parser, compact summary contract, and validator surface stay reusable across project kinds.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.8 software-delivery closeout, overlay records, and control-plane behaviors [ADD v02.181]
- CONTEXT_START_LINE: 7032
- CONTEXT_END_LINE: 7058
- CONTEXT_TOKEN: closeout_pending
- EXCERPT_ASCII_ESCAPED:
  ```text
**Software-delivery closeout derivation** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, authoritative closeout MUST be derived from canonical workflow state, validator-gate posture, governed action resolutions, and evidence references rather than from packet-local checklist surgery, board reshuffling, or manual side-ledger convergence.
  - Human-readable closeout sections, packets, and board badges MAY be synchronized after authoritative closeout becomes true, but they MUST NOT define closeout legality on their own.

  **Software-delivery overlay extension records and lifecycle semantics** [ADD v02.181]

  - When `project_profile_kind=software_delivery` requires bounded temporary ownership, takeover policy, or steer-next behavior, canonical runtime state SHOULD expose `GovernanceClaimLeaseRecord` and `GovernanceQueuedInstructionRecord` or equivalent stable overlay records keyed by `work_packet_id`, `workflow_run_id`, `workflow_binding_id`, `model_session_id`, or other canonical runtime identifiers.
  - Software-delivery workflow bindings SHOULD preserve explicit states `created`, `queued`, `claimed`, `node_active`, `approval_wait`, `validation_wait`, `closeout_pending`, `settled`, `failed`, and `canceled`. `approval_wait` requires unresolved governed actions, `validation_wait` requires active validator-gate records, and `closeout_pending` is derived from canonical runtime truth rather than packet prose.

  **Software-delivery overlay control-plane behaviors** [ADD v02.181]

  - For `project_profile_kind=software_delivery`, start, steer, cancel, close, and recover MUST resolve through workflow-backed governed actions and canonical runtime records instead of repo ledgers, mailbox chronology, or transcript-only intent.
  - Software-delivery control-plane state SHOULD preserve health posture, stale-detection posture, backpressure posture, and operator-alert posture by stable runtime identifiers. Under load or blocked authority, the system MUST surface backpressure explicitly instead of silently dropping or reordering control-plane intent.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md 2.6.8.10 Role Mailbox authority boundary [ADD v02.173]
- CONTEXT_START_LINE: 7061
- CONTEXT_END_LINE: 7069
- CONTEXT_TOKEN: mailbox chronology
- EXCERPT_ASCII_ESCAPED:
  ```text
**Role Mailbox message contract, thread lifecycle, and authority boundary** [ADD v02.173]

  - Role Mailbox SHALL separate thread lifecycle from message delivery state. At minimum, thread lifecycle MUST distinguish `open`, `awaiting_response`, `waiting_on_linked_authority`, `escalated`, `resolved`, `expired`, and `archived`, while message delivery MUST distinguish `queued`, `delivered`, `acknowledged`, `replied`, `ignored`, `failed`, and `dead_lettered`.
  - Every actionable mailbox message SHOULD expose a bounded action-request envelope that declares allowed responses, due or expiry posture, optional snooze posture, and the stable linked record identifiers that the message refers to.
  - Replying to a mailbox thread MUST NOT silently mutate Work Packet, Micro-Task, Task Board, or Locus authority. Any linked change MUST resolve through a governed action, transition rule, or explicit transcription into the authoritative artifact.
  - Local-small-model and cloud-model routing MAY consume compact mailbox summaries, but mailbox chronology, unread badges, or transcript order MUST NOT become substitutes for workflow state, dependency state, or completion evidence.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.181.md Phase 1 roadmap projection-surface discipline coverage
- CONTEXT_START_LINE: 48042
- CONTEXT_END_LINE: 48048
- CONTEXT_TOKEN: Projection-surface discipline
- EXCERPT_ASCII_ESCAPED:
  ```text
- [ADD v02.181] Software-delivery runtime truth: at least one workflow-backed software-delivery work item exposes product-owned runtime state and linked governed actions by stable identifiers instead of relying on packet prose, mailbox order, or Markdown mirrors as the operational authority surface.
  - [ADD v02.181] Validator-gate and closeout posture: at least one workflow-backed software-delivery work item exposes validator-gate summaries, evidence-linked gate posture, and derived closeout posture by stable identifiers without requiring packet surgery to explain why the item may proceed or close.
  - [ADD v02.181] Projection-surface discipline: Dev Command Center, Task Board, and Role Mailbox projections for at least one software-delivery work item explain the same underlying state without turning repo `/.GOV/**`, Markdown mirrors, or mailbox chronology into authority.
  - [ADD v02.181] Overlay coordination records: at least one software-delivery work item exposes overlay claim/lease state and queued steering or follow-up state by stable identifiers so actor ownership, takeover legality, and deferred steering are visible without transcript reconstruction.
  - [ADD v02.181] Overlay lifecycle and recovery posture: at least one software-delivery work item exposes checkpoint-backed recovery posture plus workflow-backed start/steer/cancel/close/recover semantics by stable identifiers so restart-safe replay and control decisions remain inspectable.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Projection-surface discipline | WHY_IN_SCOPE: This is the exact stub target: DCC, Task Board, and Role Mailbox projections must explain one underlying software-delivery runtime state without making mirrors or chronology authoritative | EXPECTED_CODE_SURFACES: workflows.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs, runtime_governance.rs | EXPECTED_TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | RISK_IF_MISSED: Display state silently becomes operational authority.
  - CLAUSE: Software-delivery overlay runtime truth specialization | WHY_IN_SCOPE: Software-delivery work meaning must resolve through canonical structured records rather than packet prose, board ordering, mailbox chronology, or side ledgers | EXPECTED_CODE_SURFACES: locus/types.rs, locus/task_board.rs, workflows.rs | EXPECTED_TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | RISK_IF_MISSED: Board or packet edits can mask invalid runtime state.
  - CLAUSE: Software-delivery closeout derivation | WHY_IN_SCOPE: Closeout must be derived from workflow, validator-gate, governed-action, owner, and evidence truth | EXPECTED_CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | EXPECTED_TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | RISK_IF_MISSED: Work can look complete while gates or evidence are unresolved.
  - CLAUSE: Software-delivery overlay extension records and lifecycle semantics | WHY_IN_SCOPE: Claim/lease and queued instruction posture must be explicit and stable-id backed | EXPECTED_CODE_SURFACES: workflows.rs, locus/types.rs, role_mailbox.rs | EXPECTED_TESTS: projection_surface_exposes_claim_and_queued_instruction_ids | RISK_IF_MISSED: Ownership and steer-next intent are inferred from comments or mailbox order.
  - CLAUSE: Role Mailbox authority boundary | WHY_IN_SCOPE: Mailbox replies and triage rows may inform linked work but cannot mutate authoritative state without governed action or transcription | EXPECTED_CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs, workflows.rs | EXPECTED_TESTS: role_mailbox_software_delivery_triage_remains_advisory | RISK_IF_MISSED: A reply or unread badge can substitute for workflow state or completion evidence.
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: Software-delivery projection summary payload | PRODUCER: DCC/workflow projection builder | CONSUMER: DCC UI, Task Board comparison view, Role Mailbox triage, validators | SERIALIZER_TRANSPORT: structured runtime summary keyed by work_packet_id and workflow_run_id | VALIDATOR_READER: DCC projection tests | TRIPWIRE_TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | DRIFT_RISK: Surfaces disagree about status, owner, gate, or closeout.
  - CONTRACT: Task Board projection row authority fields | PRODUCER: Locus task-board projection layer | CONSUMER: Task Board derived layouts and DCC planning views | SERIALIZER_TRANSPORT: TaskBoardEntryRecordV1 plus software-delivery profile extension | VALIDATOR_READER: task-board projection tests | TRIPWIRE_TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | DRIFT_RISK: Lane placement or mirror state becomes hidden authority.
  - CONTRACT: Role Mailbox linked-authority triage row | PRODUCER: Role Mailbox export/projection layer | CONSUMER: Role Mailbox triage and DCC collaboration inbox | SERIALIZER_TRANSPORT: structured mailbox index/thread records with linked work ids and action_request_id | VALIDATOR_READER: role-mailbox projection tests | TRIPWIRE_TESTS: role_mailbox_software_delivery_triage_remains_advisory | DRIFT_RISK: Mailbox chronology or summary text mutates linked work.
  - CONTRACT: Governed action preview payload | PRODUCER: workflow/governed action registry and DCC projection layer | CONSUMER: DCC quick actions, Task Board row actions, mailbox escalation controls | SERIALIZER_TRANSPORT: preview record with action_request_id, target record refs, eligibility, blockers, and evidence refs | VALIDATOR_READER: governed action projection tests | TRIPWIRE_TESTS: projection_surface_previews_governed_action_before_mutation | DRIFT_RISK: UI actions skip policy, approval, or evidence gates.
  - CONTRACT: Closeout and recovery posture payload | PRODUCER: workflow runtime, validator-gate records, checkpoint lineage | CONSUMER: DCC close/recover controls, Task Board badges, Role Mailbox follow-up | SERIALIZER_TRANSPORT: structured closeout/recovery summary with gate_record_id, checkpoint_id, unresolved blockers, and authority refs | VALIDATOR_READER: closeout/recovery tests | TRIPWIRE_TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | DRIFT_RISK: Recovery or closeout depends on transcript reconstruction or packet surgery.
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Inspect current DCC snapshot, DccCompactSummaryV1, TaskBoardEntryRecordV1, Role Mailbox export, workflow-state-family, queue-reason, and allowed-action code before adding new fields.
  - Define the minimal software-delivery profile extension or summary payload needed to carry validator-gate, governed-action, claim/lease, queued-instruction, recovery, evidence, and closeout posture.
  - Wire DCC, Task Board, and Role Mailbox projections to read the same runtime-backed fields by stable identifiers.
  - Add tests with intentional DCC/Task Board/mailbox advisory disagreement where canonical runtime truth wins.
  - Keep repo /.GOV mirrors, Markdown packet prose, board lanes, and mailbox chronology as readable/advisory inputs only.
- HOT_FILES:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- TRIPWIRE_TESTS:
  - `dcc_software_delivery_projection_surface_keeps_runtime_authority`
  - `task_board_software_delivery_projection_cannot_override_runtime_truth`
  - `role_mailbox_software_delivery_triage_remains_advisory`
  - `projection_surface_previews_governed_action_before_mutation`
  - `closeout_projection_requires_gate_evidence_and_owner_truth`
- CARRY_FORWARD_WARNINGS:
  - Do not create a second DCC-only or board-only truth store.
  - Do not treat mailbox replies, unread badges, transcript order, packet prose, or mirror freshness as authority.
  - Keep stable identifiers and authority_refs/evidence_refs visible enough for validators to inspect.
  - If implementation discovers missing base schema support, report bounded spec/stub need rather than silently broadening scope.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - v02.181 projection-surface discipline.
  - v02.181 software-delivery overlay runtime truth specialization.
  - v02.181 closeout derivation.
  - v02.181 overlay extension records and lifecycle semantics.
  - v02.173 Role Mailbox authority boundary.
- FILES_TO_READ:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- COMMANDS_TO_RUN:
  - `rg -n "build_dcc_control_plane_snapshot|DccCompactSummaryV1|TaskBoardEntryRecordV1|queue_reason_code|allowed_action_ids|role_mailbox|validator_gate|claim|lease|queued|closeout|checkpoint" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact`
  - `cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact`
  - `just gov-check`
- POST_MERGE_SPOTCHECKS:
  - Verify DCC, Task Board, and Role Mailbox projection rows expose the same canonical work_packet_id and workflow_run_id.
  - Verify a stale Task Board mirror and advisory mailbox reply cannot override validator-gate or closeout blockers.
  - Verify governed action previews expose target record ids and blockers before mutation.
  - Verify recovery and queued steering posture remain stable-id backed and do not require transcript replay.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Exact final field names for claim/lease, queued instruction, validator-gate, recovery, and closeout projection payloads are not proven until implementation inspects current struct boundaries.
  - Whether the best landing surface is a software-delivery profile extension, DccCompactSummaryV1 extension, or workflow projection helper split is not proven yet.
  - No product tests have been run in this pre-signature refinement pass.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Local evidence shows a partial implementation base: DCC snapshot/compact-summary tests, Task Board structured projection rows, Role Mailbox structured export, and workflow-state-family or queue-reason contracts already exist, so the WP should extend those seams rather than introduce a new projection store.
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
  - engine.librarian
  - engine.dba
  - engine.sovereign
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NO_CHANGE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Locus
  - MicroTask
  - Command Center
  - Execution / Job Runtime
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - DCC snapshot plus Locus runtime truth -> IN_THIS_WP (stub: NONE)
  - Task Board rows plus governed action previews -> IN_THIS_WP (stub: NONE)
  - Role Mailbox triage plus linked runtime authority -> IN_THIS_WP (stub: NONE)
  - Validator-gate posture plus closeout derivation -> IN_THIS_WP (stub: NONE)
  - Claim lease plus queued steering state -> IN_THIS_WP (stub: NONE)
  - Compact summary plus local model routing -> IN_THIS_WP (stub: NONE)
  - Runtime storage plus projection field provenance -> IN_THIS_WP (stub: NONE)
  - Recovery checkpoint lineage plus stale binding detection -> IN_THIS_WP (stub: NONE)
  - Backpressure posture plus operator alert projection -> IN_THIS_WP (stub: NONE)
  - MicroTask hard gates plus mailbox wait reasons -> IN_THIS_WP (stub: NONE)
  - Governed action resolution plus UI quick actions -> IN_THIS_WP (stub: NONE)
  - Stable projection ids plus cross-surface consistency checks -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: Software-delivery runtime truth substrate | SUBFEATURES: canonical structured records, workflow-state family, queue-reason code, allowed action ids, gate posture, checkpoint/evidence refs | PRIMITIVES_FEATURES: FEAT-LOCUS-WORK-TRACKING | MECHANICAL: engine.librarian, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Locus-backed records remain the join target for DCC, Task Board, and Role Mailbox projections.
  - PILLAR: MicroTask | CAPABILITY_SLICE: Mailbox-linked execution wait projection | SUBFEATURES: micro-task summary, hard-gate state, mailbox wait reason, verifier outcome refs, active session occupancy | PRIMITIVES_FEATURES: FEAT-MICRO-TASK-EXECUTOR | MECHANICAL: engine.context, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Micro-task waits must be visible through compact state rather than transcript replay.
  - PILLAR: Command Center | CAPABILITY_SLICE: Projection discipline and governed action preview | SUBFEATURES: DCC snapshot fields for validator-gate, claim/lease, queued follow-up, recovery, closeout, stale detection, backpressure, and authority refs | PRIMITIVES_FEATURES: FEAT-DEV-COMMAND-CENTER | MECHANICAL: engine.librarian, engine.context, engine.version, engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: DCC remains the projection/control surface, not the authority.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Workflow-backed start/steer/cancel/close/recover semantics | SUBFEATURES: governed action resolution, workflow binding state, validator-gate phase, checkpoint recovery posture, stable action request ids | PRIMITIVES_FEATURES: FEAT-WORKFLOW-ENGINE | MECHANICAL: engine.dba, engine.sovereign, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Control-plane changes must resolve through workflow-backed runtime records.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: Compact summary first projection | SUBFEATURES: compact software-delivery summary rows, authority refs, evidence refs, next action previews, linked mailbox and gate ids | PRIMITIVES_FEATURES: FEAT-LLM-FRIENDLY-DATA | MECHANICAL: engine.context, engine.librarian | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Local-small-model routing should consume compact structured records before raw Markdown mirrors.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Software-delivery projection discipline snapshot | JobModel: UI_ACTION | Workflow: dcc_software_delivery_projection_snapshot | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: work_packet_gated, task_board_synced | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: DCC should read one runtime-backed projection that carries validator-gate, claim/lease, queued-instruction, recovery, closeout, and authority refs.
  - Capability: Task Board runtime-authority projection guard | JobModel: UI_ACTION | Workflow: locus_task_board_projection | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: task_board_status_changed | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Board rows can group or sort work, but validation, ownership, recovery, and closeout posture must come from runtime fields.
  - Capability: Role Mailbox linked-authority preview | JobModel: UI_ACTION | Workflow: role_mailbox_software_delivery_triage | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Mailbox replies and unread order stay advisory until governed action or transcription updates linked runtime records.
  - Capability: Workflow-backed overlay lifecycle projection | JobModel: WORKFLOW | Workflow: software_delivery_overlay_lifecycle | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: VISIBLE | FlightRecorder: workflow_gate_transition | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: start, steer, cancel, close, and recover must expose explicit runtime states and stable ids before mutation.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Software-Delivery-Projection-Surface-Discipline-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Dev-Command-Center-Control-Plane-Backend-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Structured-Collaboration-Schema-Registry-v4 -> EXPAND_IN_THIS_WP
  - WP-1-Role-Mailbox-v1 -> EXPAND_IN_THIS_WP
  - WP-1-Project-Agnostic-Workflow-State-Registry-v1 -> EXPAND_IN_THIS_WP
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> PARTIAL (WP-1-Dev-Command-Center-Control-Plane-Backend-v1)
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Structured-Collaboration-Schema-Registry-v4)
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> PARTIAL (WP-1-Structured-Collaboration-Artifact-Family-v1)
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs -> PARTIAL (WP-1-Role-Mailbox-v1)
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs -> PARTIAL (WP-1-Project-Agnostic-Workflow-State-Registry-v1)
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: YES
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS
- GUI_REFERENCE_DECISIONS:
  - DCC and Task Board action preview <- NONE (IN_THIS_WP)
  - Role Mailbox triage <- NONE (IN_THIS_WP)
- HANDSHAKE_GUI_ADVICE:
  - Surface: Dev Command Center | Control: Authority source inspector | Type: icon button | Why: Operators need to see whether a status came from runtime, Task Board mirror, or mailbox advisory state | Microcopy: Runtime truth | Tooltip: Show canonical record ids and evidence refs for this status
  - Surface: Task Board | Control: Lane action preview | Type: icon button | Why: Drag or row actions must preview transition legality before state changes | Microcopy: Preview move | Tooltip: Show transition rule, target workflow state, and blockers
  - Surface: Role Mailbox | Control: Reply authority indicator | Type: status chip | Why: Mailbox replies can be local or governed-linked and must not be confused | Microcopy: Mailbox local or Governed | Tooltip: Explain whether this reply can affect linked work
- HIDDEN_GUI_REQUIREMENTS:
  - Mutation controls remain disabled when canonical runtime fields are absent or stale even if the visible mirror suggests work is ready.
  - Cross-surface conflict state must name DCC, Task Board, and Role Mailbox values while marking canonical runtime state as winning.
  - Closeout controls must show unresolved gate/evidence/owner/action blockers before allowing a close request.
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - Keep projection rows compact but include expandable authority refs and evidence refs.
  - Store action preview payloads as structured data so validators can assert legal transitions without screenshot inspection.
  - Emit one test fixture where DCC, Task Board, and Role Mailbox disagree and runtime truth wins.
## SCOPE
- What: Implement and prove the software-delivery projection-discipline contract that keeps Dev Command Center, Task Board, and Role Mailbox as projections over one runtime-backed truth.
- Why: v02.181 forbids DCC layout position, Task Board mirrors, Role Mailbox chronology, packet prose, or repo /.GOV artifacts from becoming hidden authority for validation, ownership, recovery, queued steering, or closeout posture.
- IN_SCOPE_PATHS:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/api/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/src/runtime_governance.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- OUT_OF_SCOPE:
  - Cosmetic UI redesign or broad layout registry work.
  - New external research or third-party workflow framework integration.
  - Making repo /.GOV mirrors, packet prose, board lanes, or mailbox chronology canonical product runtime authority.
  - Official packet creation, signature recording, coder launch, or validator launch during this pre-signature refinement pass.
- TOUCHED_FILE_BUDGET: 7
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (Do not use `## WAIVERS GRANTED` to continue after token-cost overrun. Token budget and token-ledger drift are diagnostic-only cost telemetry and must be surfaced mechanically in audits/dossiers instead of requiring a continuation waiver.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact
  just gov-check
```

### DONE_MEANS
- DCC, Task Board, and Role Mailbox projections for one software-delivery work item expose the same runtime truth by stable identifiers.
- Projection rows expose validator-gate, governed-action, claim/lease, queued-instruction, checkpoint/recovery, evidence, stale, and closeout posture from canonical runtime records.
- Board lane, unread mailbox state, transcript order, packet prose, and repo /.GOV mirrors cannot authorize validation, ownership, recovery, queued steering, or closeout.
- Governed action previews name target records and action ids before mutation.
- Tests include a conflict case where DCC, Task Board, and Role Mailbox advisory states disagree and runtime truth wins.

- PRIMITIVES_EXPOSED:
  - NONE
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-27T16:46:05.420Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.181]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.181.md v02.181 projection-surface discipline and software-delivery runtime truth specialization
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
  - .GOV/task_packets/stubs/WP-1-Software-Delivery-Projection-Surface-Discipline-v1.md
  - .GOV/spec/Handshake_Master_Spec_v02.181.md
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs
  - ../handshake_main/src/backend/handshake_core/src/role_mailbox.rs
  - ../handshake_main/src/backend/handshake_core/tests/micro_task_executor_tests.rs
- SEARCH_TERMS:
  - build_dcc_control_plane_snapshot
  - DccCompactSummaryV1
  - TaskBoardEntryRecordV1
  - queue_reason_code
  - allowed_action_ids
  - role_mailbox
  - validator_gate
  - claim lease queued instruction closeout checkpoint
- RUN_COMMANDS:
  ```bash
rg -n "build_dcc_control_plane_snapshot|DccCompactSummaryV1|TaskBoardEntryRecordV1|queue_reason_code|allowed_action_ids|role_mailbox|validator_gate|claim|lease|queued|closeout|checkpoint" ../handshake_main/src/backend/handshake_core/src ../handshake_main/src/backend/handshake_core/tests
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml task_board_software_delivery_projection_cannot_override_runtime_truth -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml role_mailbox_software_delivery_triage_remains_advisory -- --exact
  cargo test --manifest-path ../handshake_main/src/backend/handshake_core/Cargo.toml closeout_projection_requires_gate_evidence_and_owner_truth -- --exact
  just gov-check
  ```
- RISK_MAP:
  - "Projection rows become a second authority" -> "Operators can close, validate, or reroute work from stale display state."
  - "Mailbox chronology is treated as completion" -> "Completion or ownership can be inferred without governed action proof."
  - "Task Board lane placement outranks runtime truth" -> "Validation and closeout posture can drift from gate/evidence records."
  - "Stable ids are missing from summaries" -> "DCC, Task Board, and mailbox views cannot prove they describe the same work item."
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- Principle: prefer enumerating "too many" controls early, consolidate later.
- For `PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1`, this section is copied from the signed refinement and should not drift.
- Include minimalistic in-UI explainers (prefer hover tooltips), and ensure tooltips are accessible (hover + keyboard focus; dismissible; avoid violating WCAG 1.4.13).
- UI_SURFACES:
  - Dev Command Center work item detail and queue rows.
  - Task Board derived software-delivery planning rows.
  - Role Mailbox triage rows linked to work packets or micro-tasks.
  - Projection field-provenance and governed-action preview panels.
- UI_CONTROLS (buttons/dropdowns/inputs):
  - Control: Action preview trigger | Type: icon button | Tooltip:Preview target records, governed action id, and required evidence before mutation | Notes: Must not mutate directly.
  - Control: Authority source inspector | Type: icon button | Tooltip:Show canonical runtime fields behind this visible status | Notes: Surfaces field provenance and mirror state.
  - Control: Projection surface switcher | Type: segmented control | Tooltip:Compare DCC, Task Board, and Role Mailbox views of the same work item | Notes: Useful validator spotcheck surface.
  - Control: Claim or lease posture filter | Type: menu | Tooltip:Filter work by claimed, leased, expired, or takeover-eligible posture | Notes: Reads runtime overlay fields only.
  - Control: Queued follow-up filter | Type: menu | Tooltip:Show queued, injected, expired, or rejected steering instructions | Notes: Prevents transcript-only steer-next handling.
  - Control: Closeout eligibility badge | Type: status chip | Tooltip:Explain unresolved gate, evidence, owner, or action blockers | Notes: Badge must be derived, not authoritative.
- UI_STATES (empty/loading/error):
  - Empty state explains that no software-delivery runtime projection exists for this work item yet and offers no fake status.
  - Loading state preserves the last verified timestamp and blocks mutation controls until authority refs load.
  - Error state distinguishes missing canonical runtime record, stale mirror, mailbox-only advisory state, and unresolved governed action.
  - Conflict state shows DCC, Task Board, and Role Mailbox disagreement with canonical runtime state winning.
- UI_MICROCOPY_NOTES (labels, helper text, hover explainers):
  - Use labels such as Runtime truth, Projection only, Mailbox advisory, Mirror stale, Governed action required, Closeout blocked, and Claim expired.
  - Avoid wording that implies board lane, unread count, packet checklist, or mailbox order is itself authority.
  - Every quick action should name the target governed action or field before the operator confirms it.
- UI_ACCESSIBILITY_NOTES:
  - Tooltips must work on hover and keyboard focus; be dismissible; do not obscure content (WCAG 1.4.13).
  - Status chips must not rely on color alone; include accessible labels for stale, blocked, advisory, and authoritative states.
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
  - LOG_PATH: `.handshake/logs/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/<name>.log` (recommended; not committed)
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
