# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

- Requirements:
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
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 4a5534925af14bf994344182fb5c863cacba6741
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-28T21:08:01.328Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 1d874cda0c384d7484f0cc792f63617ff062dd29
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-04-28T21:08:01.328Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
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
- WP_VALIDATOR_OF_RECORD: WP_VALIDATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- INTEGRATION_VALIDATOR_OF_RECORD: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
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
Verdict: PASS
Blockers: NONE
Next: NONE
## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: v02.181 Projection-surface discipline | CODE_SURFACES: workflows.rs, locus/types.rs, locus/task_board.rs, role_mailbox.rs, runtime_governance.rs | TESTS: dcc_software_delivery_projection_surface_keeps_runtime_authority | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Software-delivery overlay runtime truth specialization | CODE_SURFACES: locus/types.rs, locus/task_board.rs, workflows.rs | TESTS: task_board_software_delivery_projection_cannot_override_runtime_truth | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Software-delivery closeout derivation | CODE_SURFACES: workflows.rs, runtime_governance.rs, locus/types.rs | TESTS: closeout_projection_requires_gate_evidence_and_owner_truth | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Software-delivery overlay extension records and lifecycle semantics | CODE_SURFACES: workflows.rs, locus/types.rs, role_mailbox.rs | TESTS: projection_surface_exposes_claim_and_queued_instruction_ids | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Role Mailbox authority boundary | CODE_SURFACES: role_mailbox.rs, api/role_mailbox.rs, workflows.rs | TESTS: role_mailbox_software_delivery_triage_remains_advisory | EXAMPLES: Example software-delivery projection summary for one work_packet_id showing DCC, Task Board, and Role Mailbox values plus canonical runtime truth., Example Task Board row with stale mirror state but runtime validator-gate blocker winning., Example Role Mailbox thread where latest reply is advisory and linked closeout remains blocked until governed action resolution., Example queued steer-next instruction showing queued, injected, expired, and rejected lifecycle states by queued_instruction_id., Example recovery posture row linking checkpoint_id, parent checkpoint lineage, stale binding state, and legal recover action. | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
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
- WAIVER_ID: CX-573F-20260427-WP1-FLIGHT-RECORDER-BASELINE-BRACE | STATUS: ACTIVE | COVERS: SCOPE, ENVIRONMENT, PROOF | SCOPE: WP-1-Software-Delivery-Projection-Surface-Discipline-v1 MT-001 may include the minimal `src/backend/handshake_core/src/flight_recorder/mod.rs` brace repair around `FlightRecorderEventType::SessionCascadeCancel` needed to restore baseline compile/test proof | JUSTIFICATION: Operator explicitly granted waiver after Coder proved the baseline compile failure blocks MT-001 proof and the file is outside signed scope; continuing inside the orchestrator-managed WP avoids a separate unblocker while keeping the exception narrow and auditable | APPROVER: Operator (chat: "waiver granted, continue with orchestrator managed wp.", 2026-04-27) | EXPIRES: when MT-001 proof and WP Validator review complete or this WP reaches terminal closeout
- WAIVER_ID: CX-573F-20260427-WP1-WORKFLOWS-BASELINE-BRACE | STATUS: ACTIVE | COVERS: SCOPE, ENVIRONMENT, PROOF | SCOPE: WP-1-Software-Delivery-Projection-Surface-Discipline-v1 MT-001 may include the minimal `src/backend/handshake_core/src/workflows.rs` brace repair after `task_board_projection_preserves_updated_at_then_wp_id_order` around line 25229 needed to restore baseline compile/test proof | JUSTIFICATION: Operator explicitly granted waiver after Coder proved the clean baseline workflows.rs compile failure blocks MT-001 proof and is outside the first flight_recorder waiver; continuing inside the orchestrator-managed WP keeps the exception narrow and auditable | APPROVER: Operator (chat: "waiver granted for WP-1 workflows.rs clean-baseline brace repair; continue orchestrator-managed WP", 2026-04-27) | EXPIRES: when MT-001 proof and WP Validator review complete or this WP reaches terminal closeout
- WAIVER_ID: CX-573F-20260427-WP1-WORKFLOWS-CHECKPOINT-COMPILE | STATUS: ACTIVE | COVERS: SCOPE, ENVIRONMENT, PROOF | SCOPE: WP-1-Software-Delivery-Projection-Surface-Discipline-v1 MT-001 may include only the minimal `src/backend/handshake_core/src/workflows.rs` compile restoration around `create_session_checkpoint` lines 5293, 5303, and 5305: bring `SessionCheckpoint` into scope and map JSON serialization failures into the existing `WorkflowError`/`StorageError` path without changing checkpoint semantics, runtime authority, projection surfaces, or unrelated workflow behavior | JUSTIFICATION: Operator granted a conditional waiver only if Orchestrator judged the repair good code and Master Spec compliant; Orchestrator judged it compliant because Master Spec v02.181 requires checkpoint-backed recovery posture and immutable checkpoint lineage, and this repair restores the existing session-checkpoint path rather than introducing a new authority source | APPROVER: Operator (chat: "waiver granted for this wp but only if you judge this to be correct vs master spec. continue orchestrator managed wp and use my waiver only if its good code and complies with master spec.", 2026-04-27) plus Orchestrator judgment checkpoint #1027 | EXPIRES: when MT-001 proof and WP Validator review complete or this WP reaches terminal closeout

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
- Whole-WP closeout range regenerated by Orchestrator mechanical handoff repair: `45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- **MERGE_BASE_SHA**: `45afc8867f08f7c2f8edfe71ab750fe92ab28866`
- **COMMITTED_TARGET_HEAD_SHA**: `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`
- **Artifacts**: `.GOV/task_packets/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/signed-scope.patch`

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 1
- **End**: 6185
- **Line Delta**: 5
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/flight_recorder/mod.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/locus/mod.rs`
- **Start**: 1
- **End**: 6
- **Line Delta**: 1
- **Pre-SHA1**: `e3a47c5cea8bba4bdb60865fe229a2ddcd7da9eb`
- **Post-SHA1**: `9bc4663719c8e2062f9bbe5da286a6f6c0a2e75b`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/locus/mod.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/locus/task_board.rs`
- **Start**: 1
- **End**: 435
- **Line Delta**: 55
- **Pre-SHA1**: `94420cf97740ebc3df0bf2a1fda05b8d0a40e634`
- **Post-SHA1**: `c88148fe2c885efcc1af12a02a7da5a638b18927`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/locus/task_board.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/locus/types.rs`
- **Start**: 1
- **End**: 3515
- **Line Delta**: 1632
- **Pre-SHA1**: `20426e53c50e4fa53a5840aea0132ab045590a86`
- **Post-SHA1**: `1235e07c536796decb348e3141957ef701c93d72`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for IntVal-FAIL remediation candidate `eb59e981`; adds `GovernedActionPreviewV1` + `GovernedActionEligibility` + `derive_governed_action_preview(s)` helpers and the `governed_action_previews` field on `SoftwareDeliveryProjectionSurfaceV1`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/locus/types.rs
- **Timestamp**: 2026-04-28T18:08:00Z
- **Operator**: CODER:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4 after Integration Validator FAIL on candidate 2d6c40193cbf28ef244198e9b09ccbe855a2dd54 flagged the missing governed-action preview payload contract.

- **Target File**: `src/backend/handshake_core/src/role_mailbox.rs`
- **Start**: 1
- **End**: 1882
- **Line Delta**: 67
- **Pre-SHA1**: `90e05dd16ffaad51b0388bb230a8265c471c0593`
- **Post-SHA1**: `b408a045264945c04ce425e75d5c53f3281c0e2f`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/role_mailbox.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
- **Start**: 1
- **End**: 785
- **Line Delta**: 318
- **Pre-SHA1**: `b0d6defd4569ee504af2930fe7264427d258af4e`
- **Post-SHA1**: `d8bf5ced710b779ad03bdf57c5ac8df54fd540ef`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/runtime_governance.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- **Start**: 1
- **End**: 1277
- **Line Delta**: 54
- **Pre-SHA1**: `e8b673477c97e800f09b9d469276969d48b0be08`
- **Post-SHA1**: `f7833c5efbeaa08a8c1f6bdb834361d1cb4fb1d5`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/storage/locus_sqlite.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 1
- **End**: 27048
- **Line Delta**: 691
- **Pre-SHA1**: `292b63d2c0da2ccd5dfd1505461575223096d6d5`
- **Post-SHA1**: `2ad9e7ff1ff78efd6649fe99c44d52da5fc8d7e9`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for current candidate `4dee21bd`; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/src/workflows.rs
- **Timestamp**: 2026-04-28T10:32:00Z
- **Operator**: ORCHESTRATOR:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd.

- **Target File**: `src/backend/handshake_core/tests/micro_task_executor_tests.rs`
- **Start**: 1
- **End**: 6493
- **Line Delta**: 3323
- **Pre-SHA1**: `d0d8c79a208ac5f9152ff28769f02f04d5dd0af7`
- **Post-SHA1**: `b2492a10aecfd753708aebeb6f3bb602ea43b338`
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
- **Lint Results**: regenerated mechanical whole-WP range evidence for IntVal-FAIL remediation candidate `eb59e981`; adds the exact tripwire test `projection_surface_previews_governed_action_before_mutation` covering the packet's governed-action preview contract row; validator performs judgment separately.
- **Artifacts**: src/backend/handshake_core/tests/micro_task_executor_tests.rs
- **Timestamp**: 2026-04-28T18:08:00Z
- **Operator**: CODER:WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.181.md
- **Notes**: Whole-WP committed candidate manifest regenerated mechanically from evaluated git range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4 after Integration Validator FAIL on candidate 2d6c40193cbf28ef244198e9b09ccbe855a2dd54 flagged the missing `projection_surface_previews_governed_action_before_mutation` exact tripwire.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS: DONE_VALIDATED
- What changed in this update:
  - locus/types.rs: SoftwareDeliveryProjectionSurfaceV1 schema, derive_software_delivery_projection_surface, validate_software_delivery_projection_surface_authority, mirror_state_is_advisory_only, plus two new pure predicates is_governed_action_id_allowed_for_workflow_family and is_registered_governed_action_id mirroring the canonical workflow-family action matrix.
  - locus/task_board.rs: TaskBoardEntryRecordV1 docstring declaring mirror_state/lane_id/status as advisory display state for software_delivery profile.
  - role_mailbox.rs: RoleMailboxThread docstring declaring mailbox chronology advisory-only; only thread_id reaches the projection surface.
  - tests/micro_task_executor_tests.rs: dcc_software_delivery_projection_surface_keeps_runtime_authority tripwire test plus import path move for validate_task_board_entry_authoritative_fields into locus::task_board::{}.
  - flight_recorder/mod.rs: WAIVER CX-573F-20260427-WP1-FLIGHT-RECORDER-BASELINE-BRACE -- mechanical brace repair around SessionCascadeCancel arms.
  - workflows.rs: WAIVER CX-573F-20260427-WP1-WORKFLOWS-BASELINE-BRACE -- closing brace after task_board_projection_preserves_updated_at_then_wp_id_order; WAIVER CX-573F-20260427-WP1-WORKFLOWS-CHECKPOINT-COMPILE -- narrow create_session_checkpoint compile restoration (SessionCheckpoint import + serde_json error mapping via WorkflowError::Storage(StorageError::Serialization(format!(...)))).
- Requirements / clauses self-audited: v02.181 Projection-surface discipline (MT-001 clause); see EVIDENCE_MAPPING for per-bullet file:line evidence.
- Checks actually run: `cargo check --lib` PASS (39 pre-existing dead-code warnings, 0 errors); `cargo test --test micro_task_executor_tests dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact` 1 passed 0 failed.
- Known gaps / weak spots: see WEAK_SPOTS below.
- Heuristic risks / maintainability concerns: action matrix is now duplicated 3x (workflows.rs:3376, storage/locus_sqlite.rs:203, locus/types.rs governed_action_ids_for_family). Consolidation is out of MT-001 scope but is a maintainability risk if action sets drift. Validator should confirm the duplicates remain identical or recommend a follow-on consolidation.
- Validator focus request: confirm (a) projection surface authority validator rejects every advisory mutation; (b) status-specific actions stay unregistered; (c) waiver-scoped repairs do not change checkpoint semantics or Flight Recorder payload schemas; (d) the four MTs that remain DECLARED (MT-002..MT-005) are not implicitly closed by MT-001.
- Rubric contract understanding proof: MT-001 contract is a tripwire that proves canonical runtime truth wins over Task Board / Role Mailbox / packet-prose / GOV-mirror display state for one software-delivery work item. The new SoftwareDeliveryProjectionSurfaceV1 lifts authoritative fields verbatim from StructuredCollaborationSummaryV1 and demotes board mirror_state/lane_id/status_text and mailbox thread ids to advisory pass-through. The targeted test exercises the conflict case explicitly.
- Rubric scope discipline proof: signed scope was locus/types.rs, locus/task_board.rs, role_mailbox.rs, tests/micro_task_executor_tests.rs (plus runtime_governance.rs and workflows.rs per CODE_SURFACES). All other product edits are confined to the three packet-recorded waivers (flight_recorder brace repair, workflows.rs:25229 brace repair, workflows.rs create_session_checkpoint compile restoration). No edit touched checkpoint fields, ids, registry update, Flight Recorder payload, recovery semantics, runtime authority, or projection surfaces beyond what MT-001 added.
- Rubric baseline comparison: pre-edit baseline failed cargo check --lib with three errors (E0422 SessionCheckpoint missing import; two E0277 serde_json::to_string conversions) plus a baseline missing brace in workflows.rs and a flight_recorder baseline missing brace. Post-edit baseline passes cargo check --lib and the targeted test runs. The three baseline blockers are the basis for the three Operator waivers.
- Rubric end-to-end proof: targeted test dcc_software_delivery_projection_surface_keeps_runtime_authority runs derive_software_delivery_projection_surface against a canonical Validation/ValidationWait summary plus a stale_advisory_board_entry (mirror_state=Stale, lane_id=done, workflow_state_family=Done, status="Done (mirror)") and a mixed mailbox thread id list (with empty + duplicates). Asserts the projection surface lifts canonical Validation/ValidationWait/allowed_action_ids/status/blockers/next_action/summary_ref/authority_refs/evidence_refs verbatim, deduplicates and trims advisory mailbox thread ids, and refuses to project a misaligned board (different work_packet_id) or non-software_delivery profile.
- Rubric architecture fit self-review: predicates are pure functions with no state, no I/O, and no allocation beyond a static slice; live next to existing structured-collaboration helpers in locus/types.rs. The module re-export at locus/mod.rs:5 (`pub use types::*`) carries them to `workflows::locus::*` automatically -- no mod.rs edit was required. Test import for validate_task_board_entry_authoritative_fields was moved into the locus::task_board::{} group rather than re-exported through mod.rs (per orchestrator decision).
- Rubric heuristic quality self-review: action matrix duplication noted above is the main heuristic risk. The exclusion of status-specific actions (start_work_packet, start_micro_task) is documented in the predicate docstrings and matches the orchestrator constraint. No fuzzy thresholds, ML, or natural-language classification -- predicates are exact-string membership tests against finite static slices.
- Rubric anti-gaming / counterfactual check: if the mirror_state branch were removed from validate_software_delivery_projection_surface_authority, the dcc_software_delivery_projection_surface_keeps_runtime_authority test would still pass because its mirror_state assertion runs on derive output, not validate output. Validator should add a focused counterfactual probe of the validate path. If is_registered_governed_action_id were widened to accept "start_work_packet", the test wouldn't catch it because that string is not in the test corpus -- Validator should boundary-probe with both registered and unregistered ids.
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check: every diff line is either a public predicate, a struct field exposing a canonical posture, an authority validator branch, an advisory-pass-through field, a test assertion, a docstring declaring authority discipline, or a packet-recorded waiver repair. No drive-by refactors, no renames, no test "improvements" outside MT-001 scope.
- Signed-scope debt ledger:
  - DEBT: Action matrix is duplicated 3x. Consolidation is out of MT-001 scope; recommend a follow-on RGF for matrix unification.
  - DEBT: MT-002..MT-005 remain DECLARED. MT-001 only proves the projection-surface tripwire; the closeout, claim/lease, queued-instruction, and mailbox-authority clauses still need their own MTs.
- Data contract self-check: SoftwareDeliveryProjectionSurfaceV1 carries canonical schema_id `hsk.software_delivery_projection_surface@1` and SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND constant; advisory fields are explicitly named `advisory_*` to make their non-authoritative status self-evident at every call site. SHA1s and line ranges are recorded in VALIDATION manifest above.
- Next step / handoff hint: Validator should run `cargo test --test micro_task_executor_tests dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact` independently, audit the three waiver repairs against the recorded waiver scopes, and confirm canonical/advisory boundaries in derive_software_delivery_projection_surface and validate_software_delivery_projection_surface_authority. WP_VALIDATOR session is the next actor; integration validator gates remain ahead.

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
- REQUIREMENT: "DCC, Task Board, and Role Mailbox projections for one software-delivery work item expose the same runtime truth by stable identifiers."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1979` (derive_software_delivery_projection_surface joins canonical summary, board entry, and mailbox thread ids by `record_id`/`work_packet_id`)
  - EVIDENCE: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3247` (dcc_software_delivery_projection_surface_keeps_runtime_authority asserts a single projection surface drives DCC + board + mailbox views)
- REQUIREMENT: "Projection rows expose validator-gate, governed-action, claim/lease, queued-instruction, checkpoint/recovery, evidence, stale, and closeout posture from canonical runtime records."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1835` (SoftwareDeliveryProjectionSurfaceV1 struct exposes workflow_state_family, queue_reason_code, allowed_action_ids, status, blockers, next_action, summary_ref, authority_refs, evidence_refs from canonical summary)
- REQUIREMENT: "Board lane, unread mailbox state, transcript order, packet prose, and repo /.GOV mirrors cannot authorize validation, ownership, recovery, queued steering, or closeout."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:2049` (validate_software_delivery_projection_surface_authority asserts authoritative fields equal canonical summary; advisory fields cannot mutate them)
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1898` (mirror_state_is_advisory_only marks Stale/AdvisoryEdit/NormalizationRequired as advisory)
  - EVIDENCE: `src/backend/handshake_core/src/locus/task_board.rs:25` (TaskBoardEntryRecordV1 docstring declares mirror_state/lane_id/status as advisory display state)
  - EVIDENCE: `src/backend/handshake_core/src/role_mailbox.rs:223` (RoleMailboxThread docstring declares mailbox chronology advisory-only; only thread_id reaches projection)
- REQUIREMENT: "Governed action previews name target records and action ids before mutation."
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1946` (is_governed_action_id_allowed_for_workflow_family validates a next_action against the family allowlist)
  - EVIDENCE: `src/backend/handshake_core/src/locus/types.rs:1959` (is_registered_governed_action_id rejects status-specific actions like start_work_packet/start_micro_task that are not part of any family allowlist)
  - EVIDENCE: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:965` and `:974` (next_action assertions exercise both predicates against canonical workflow_state_family)
- REQUIREMENT: "Tests include a conflict case where DCC, Task Board, and Role Mailbox advisory states disagree and runtime truth wins."
  - EVIDENCE: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3247` (dcc_software_delivery_projection_surface_keeps_runtime_authority builds a stale_advisory_board_entry with mirror_state=Stale and workflow_state_family=Done and asserts canonical Validation/ValidationWait wins)
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- COMMAND: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib`
  - EXIT_CODE: `0`
  - PROOF_LINES: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 23.08s` ; `warning: \`handshake_core\` (lib) generated 39 warnings` (all pre-existing dead-code lints)
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact`
  - EXIT_CODE: `0`
  - PROOF_LINES: `running 1 test` ; `test dcc_software_delivery_projection_surface_keeps_runtime_authority ... ok` ; `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.00s`
- COMMAND: `just phase-check HANDOFF WP-1-Software-Delivery-Projection-Surface-Discipline-v1 CODER --range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..2d6c40193cbf28ef244198e9b09ccbe855a2dd54`
  - EXIT_CODE: `0`
  - PROOF_LINES: `OK | gate-check passed` ; `OK | post-work-check passed` ; `OK | role-mailbox-export-check passed` ; `OK | wp-communication-health-check passed` ; `OK | phase-check HANDOFF passed for WP-1-Software-Delivery-Projection-Surface-Discipline-v1`
- COMMAND: `git rev-parse HEAD`
  - EXIT_CODE: `0`
  - PROOF_LINES: `2d6c40193cbf28ef244198e9b09ccbe855a2dd54` (current WP-1 final candidate on feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1)

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

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T20:50:12Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-201125
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: eb59e9819c7cc2729c169c723dab3932d5d7b9d4
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4
MAIN_MERGE_COMMIT: 4a5534925af14bf994344182fb5c863cacba6741
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
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
MERGED_MAIN_COMMIT: 4a5534925af14bf994344182fb5c863cacba6741
MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-28T20:50:12Z
VALIDATOR_RISK_TIER: HIGH
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline.
- Software-delivery overlay runtime truth specialization.
- Software-delivery closeout derivation.
- Software-delivery overlay extension records and lifecycle semantics.
- Role Mailbox authority boundary.
- Governed action preview payload.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- NONE
SPEC_CLAUSE_MAP:
- v02.181 Projection-surface discipline (`Handshake_Master_Spec_v02.181.md:48046`, `Handshake_Master_Spec_v02.181.md:6917`): implemented by `src/backend/handshake_core/src/locus/types.rs:1944`, `src/backend/handshake_core/src/locus/types.rs:1997`, `src/backend/handshake_core/src/locus/types.rs:2259`, `src/backend/handshake_core/src/locus/types.rs:2297`, and `src/backend/handshake_core/src/workflows.rs:4682`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3240`.
- Software-delivery overlay runtime truth specialization (`Handshake_Master_Spec_v02.181.md:6917`-`Handshake_Master_Spec_v02.181.md:6920`): implemented by `src/backend/handshake_core/src/locus/task_board.rs:30`, `src/backend/handshake_core/src/locus/task_board.rs:203`, and `src/backend/handshake_core/src/locus/types.rs:2339`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3850`.
- Software-delivery closeout derivation (`Handshake_Master_Spec_v02.181.md:7032`-`Handshake_Master_Spec_v02.181.md:7036`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:2568`, and `src/backend/handshake_core/src/locus/types.rs:2716`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3961`.
- Software-delivery overlay extension records and lifecycle semantics (`Handshake_Master_Spec_v02.181.md:7038`-`Handshake_Master_Spec_v02.181.md:7048`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:3151`, `src/backend/handshake_core/src/locus/types.rs:3239`, and `src/backend/handshake_core/src/workflows.rs:5101`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:4765`.
- Role Mailbox authority boundary (`Handshake_Master_Spec_v02.181.md:10661`): implemented by `src/backend/handshake_core/src/role_mailbox.rs:227`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, and `src/backend/handshake_core/src/role_mailbox.rs:1707`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081`.
- Governed action preview payload (`packet.md:459`, `Handshake_Master_Spec_v02.181.md:6993`-`Handshake_Master_Spec_v02.181.md:6999`): implemented by `src/backend/handshake_core/src/locus/types.rs:1858`, `src/backend/handshake_core/src/locus/types.rs:1895`, `src/backend/handshake_core/src/locus/types.rs:2167`, `src/backend/handshake_core/src/locus/types.rs:2218`, and `src/backend/handshake_core/src/locus/types.rs:2297`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`.
NEGATIVE_PROOF:
- No signed packet product requirement remained unimplemented in candidate `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- v02.181 also describes broader software-delivery control-plane health/backpressure and push-oriented operator alerts (`Handshake_Master_Spec_v02.181.md:7058`-`Handshake_Master_Spec_v02.181.md:7059`); this signed product delta does not add those broader alert/backpressure semantics, and the reviewed product-code evidence remains limited to projection/authority surfaces such as `src/backend/handshake_core/src/locus/types.rs:1944`, `src/backend/handshake_core/src/locus/types.rs:2259`, and `src/backend/handshake_core/src/workflows.rs:5101`.
DIFF_ATTACK_SURFACES:
- Producer/consumer drift between `SoftwareDeliveryProjectionSurfaceV1` producers and DCC, Task Board, Role Mailbox consumers.
- Serialized `GovernedActionPreviewV1` payload drift before mutation.
- Authority inversion risk where stale board lane/status or mailbox chronology overrides canonical runtime fields.
- Closeout spoofing through noncanonical validator-gate, claim/lease, queued-instruction, owner packet, or checkpoint refs.
- Current-main containment risk if the feature branch is merged outside signed range discipline.
INDEPENDENT_CHECKS_RUN:
- `just check-notifications WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` and `just ack-notifications ...` returned no pending notifications before verdict and before merge commit.
- `just phase-check STARTUP WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` passed.
- `just -f ../wt-gov-kernel/justfile artifact-hygiene-check` passed.
- `just validator-git-hygiene` passed.
- `git merge-tree --write-tree HEAD eb59e9819c7cc2729c169c723dab3932d5d7b9d4` produced clean tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`, and `git diff --check HEAD 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` passed.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib` passed on candidate and committed local `main`.
- Exact tests passed on candidate and committed local `main`: `projection_surface_previews_governed_action_before_mutation`, `dcc_software_delivery_projection_surface_keeps_runtime_authority`, `task_board_software_delivery_projection_cannot_override_runtime_truth`, `closeout_projection_requires_gate_evidence_and_owner_truth`, `projection_surface_exposes_claim_and_queued_instruction_ids`, `role_mailbox_software_delivery_triage_remains_advisory`, `locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait`, `locus_sync_task_board_validation_reports_authority_scope_drift`, and `production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests --no-run` passed on candidate and committed local `main`.
- `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR` passed.
COUNTERFACTUAL_CHECKS:
- If `derive_governed_action_previews()` stopped sourcing canonical allowed action ids, DCC/Task Board/mailbox consumers could preview actions not authorized by runtime truth.
- If `derive_software_delivery_projection_surface()` stopped embedding `governed_action_previews`, downstream software-delivery surfaces would not receive the packet-required preview payload before mutation.
- If `enforce_software_delivery_task_board_projection_authority()` did not compare canonical state/queue/action/status fields, stale mirrors could override runtime truth.
- If `build_software_delivery_overlay_triage_row()` wrote back to runtime records, mailbox chronology could become hidden authority.
- If `RuntimeGovernancePaths::is_canonical_validator_gate_ref` accepted substring matches, spoofed validator-gate, claim/lease, or queued-instruction refs could satisfy authority checks.
BOUNDARY_PROBES:
- Producer/consumer boundary: `src/backend/handshake_core/src/workflows.rs:4682` delegates to the same projection derivation consumed by DCC, Task Board, and Role Mailbox projection paths.
- Authority boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3403` builds a tampered projection from board mirror fields and proves validator rejection.
- Profile boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3441`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3674`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6240` prove non-software-delivery summaries/projections do not retain software-delivery surfaces.
- Mailbox boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081` snapshots canonical overlay files before mutating advisory triage and proves no on-disk authority drift.
- Current-main boundary: contained local main commit is `4a5534925af14bf994344182fb5c863cacba6741`.
NEGATIVE_PATH_CHECKS:
- `fabricated_action` returns no preview (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3670`).
- Registered but out-of-family `approve` yields `IneligibleOutOfFamily` for a Validation-family canonical summary (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3649`).
- Non-software-delivery canonical summaries yield no preview and no projection surface (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3674`).
- Stale Task Board authority fields are rejected while advisory mirror/lane/status fields remain non-authoritative (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751`).
- Non-software-delivery lifecycle emission clears stale workflow-run lifecycle records (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:6240`).
INDEPENDENT_FINDINGS:
- NONE
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
PRIMITIVE_RETENTION_PROOF:
- `SoftwareDeliveryProjectionSurfaceV1` remains the shared projection primitive at `src/backend/handshake_core/src/locus/types.rs:1944`.
- `GovernedActionPreviewV1` remains the preview payload primitive at `src/backend/handshake_core/src/locus/types.rs:1895`.
- Closeout reference validation remains rooted in `RuntimeGovernancePaths` helpers at `src/backend/handshake_core/src/runtime_governance.rs:319`.
PRIMITIVE_RETENTION_GAPS:
- NONE
SHARED_SURFACE_INTERACTION_CHECKS:
- `src/backend/handshake_core/src/locus/types.rs:2259` derives one software-delivery projection from canonical summary plus advisory board/mailbox inputs.
- `src/backend/handshake_core/src/workflows.rs:5101` materializes `projection_surface.json` from canonical summary, claim/lease records, queued-instruction records, workflow lifecycle, and gate posture.
- `src/backend/handshake_core/src/locus/task_board.rs:203` enforces Task Board projection authority against canonical state.
- `src/backend/handshake_core/src/role_mailbox.rs:1707` builds mailbox triage from the already-derived projection surface instead of mailbox chronology.
- `src/backend/handshake_core/src/storage/locus_sqlite.rs:285` applies canonical action/queue overrides to SQLite progress metadata.
CURRENT_MAIN_INTERACTION_CHECKS:
- Current local `main` contains the signed product surface at `4a5534925af14bf994344182fb5c863cacba6741`.
- Candidate worktree was clean at `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- Contained merge tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` was limited to the signed product/test paths before harmonization.
- Post-harmonization committed main exact tests and `cargo check --lib` passed.
DATA_CONTRACT_PROOF:
- `GovernedActionPreviewV1` serializes `action_request_id`, target record refs, eligibility, blockers, and evidence refs (`src/backend/handshake_core/src/locus/types.rs:1895`) and is embedded into `SoftwareDeliveryProjectionSurfaceV1` (`src/backend/handshake_core/src/locus/types.rs:1997`).
- The preview tripwire round-trips the projection payload and proves direct preview derivation is read-only (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3716`).
- `SoftwareDeliveryProjectionSurfaceV1` carries stable ids for workflow, model session, Task Board, claim/lease, queued-instruction, authority, and evidence refs (`src/backend/handshake_core/src/locus/types.rs:1944`).
DATA_CONTRACT_GAPS:
- NONE

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T16:05:17Z
- ACTOR_ROLE: INTEGRATION_VALIDATOR
- ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
- REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-160034
- CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
- CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
- CANDIDATE_HEAD: 4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd
- HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd
- Verdict: FAIL
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: FAIL
- TEST_VERDICT: NOT_RUN
- CODE_REVIEW_VERDICT: FAIL
- HEURISTIC_REVIEW_VERDICT: FAIL
- SPEC_ALIGNMENT_VERDICT: PARTIAL
- ENVIRONMENT_VERDICT: FAIL
- DISPOSITION: NONE
- LEGAL_VERDICT: FAIL
- SPEC_CONFIDENCE: PARTIAL_DIFF_SCOPED
- WORKFLOW_VALIDITY: INVALID
- SCOPE_VALIDITY: OUT_OF_SCOPE
- PROOF_COMPLETENESS: NOT_PROVEN
- INTEGRATION_READINESS: NOT_READY
- DOMAIN_GOAL_COMPLETION: PARTIAL
- MECHANICAL_TRACK_VERDICT: FAIL
- SPEC_RETENTION_TRACK_VERDICT: PARTIAL
- MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- VALIDATOR_RISK_TIER: HIGH
- FAILURE_SUMMARY:
  - Candidate `.cargo/config.toml:4` sets `target-dir = "../Handshake Artifacts/handshake-cargo-target"`.
  - `AGENTS.md:56` requires all build/test/tool outputs to live under `../Handshake_Artifacts/` with the `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, and `handshake-tool/` subfolders.
  - This is a blocking governance/environment violation in the merge candidate. Running Cargo tests from this candidate would write proof outputs into the wrong artifact root, so Integration Validator stopped before test execution and did not merge.
- REMEDIATION_REQUIRED:
  - Restore `.cargo/config.toml:4` to `target-dir = "../Handshake_Artifacts/handshake-cargo-target"`.
  - Re-run the final candidate proof from the repaired branch using the corrected artifact root: `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib`, the exact WP regression tests named in this packet, and `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR`.
  - Do not delete any `../Handshake Artifacts/` directory or other output directory unless the Operator explicitly authorizes cleanup under the repo destructive-cleanup rules.
- CLAUSES_REVIEWED:
  - v02.181 Projection-surface discipline: master spec requires DCC, Task Board, and Role Mailbox projections to explain the same runtime truth without mailbox chronology or Markdown mirrors becoming authority (`Handshake_Master_Spec_v02.181.md:48046`). Candidate implements the core projection surface in `src/backend/handshake_core/src/locus/types.rs:1867`, derives it from canonical summary plus advisory board/mailbox inputs at `src/backend/handshake_core/src/locus/types.rs:2077`, validates canonical equality at `src/backend/handshake_core/src/locus/types.rs:2153`, and covers the conflict case at `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3237`.
  - Software-delivery overlay runtime truth specialization: master spec requires authoritative work meaning to resolve through canonical structured records instead of packet prose, board ordering, mailbox chronology, or side ledgers (`Handshake_Master_Spec_v02.181.md:6917`). Candidate declares Task Board row authority boundaries at `src/backend/handshake_core/src/locus/task_board.rs:35` and enforces software-delivery row authority against canonical state at `src/backend/handshake_core/src/locus/task_board.rs:203`.
  - Software-delivery closeout derivation: master spec requires closeout from canonical workflow state, validator-gate posture, governed action resolutions, and evidence refs rather than packet checklist surgery (`Handshake_Master_Spec_v02.181.md:7034`). Candidate adds canonical validator-gate and owner-packet path checks at `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, and `src/backend/handshake_core/src/locus/types.rs:3194`.
  - Software-delivery overlay extension records and lifecycle semantics: master spec requires stable-id claim/lease and queued-instruction records (`Handshake_Master_Spec_v02.181.md:7040`, `Handshake_Master_Spec_v02.181.md:7042`). Candidate adds claim/lease and queued-instruction canonical refs at `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, and `src/backend/handshake_core/src/runtime_governance.rs:500`; overlay projection derivation/validation lives at `src/backend/handshake_core/src/locus/types.rs:2965` and `src/backend/handshake_core/src/locus/types.rs:3053`.
  - Role Mailbox authority boundary: master spec requires mailbox summaries/threads to remain evidence and projection only, not direct mutation of validator posture, claim/lease posture, queued follow-up state, or closeout state (`Handshake_Master_Spec_v02.181.md:10661`). Candidate declares mailbox chronology advisory at `src/backend/handshake_core/src/role_mailbox.rs:230` and builds a stable-id advisory overlay triage row at `src/backend/handshake_core/src/role_mailbox.rs:1681` and `src/backend/handshake_core/src/role_mailbox.rs:1707`.
- NOT_PROVEN:
  - v02.181 Projection-surface discipline: candidate not cleared because blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
  - Software-delivery overlay runtime truth specialization: candidate not cleared because blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
  - Software-delivery closeout derivation: candidate not cleared because blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
  - Software-delivery overlay extension records and lifecycle semantics: candidate not cleared because blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
  - Role Mailbox authority boundary: candidate not cleared because blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
  - PASS readiness is not proven because the candidate's Cargo target directory violates the required artifact root before Integration Validator can run test proof.
  - The product clauses above were reviewed at line level but were not cleared for PASS because the merge candidate includes a blocking out-of-scope root tooling change.
- MAIN_BODY_GAPS:
  - `.cargo/config.toml:4` must use `../Handshake_Artifacts/handshake-cargo-target`, not `../Handshake Artifacts/handshake-cargo-target`.
- QUALITY_RISKS:
  - A wrong artifact root can fragment build/test proof and make future validator evidence non-comparable across worktrees.
- DIFF_ATTACK_SURFACES:
  - Root build-output routing via `.cargo/config.toml`.
  - Projection-surface authority joins across `locus/types.rs`, `locus/task_board.rs`, `workflows.rs`, and `role_mailbox.rs`.
  - Runtime canonical path validation in `runtime_governance.rs`.
  - Micro-task packet/progress parity between workflow artifact emission and SQLite progress metadata.
- INDEPENDENT_CHECKS_RUN:
  - `just integration-validator-context-brief WP-1-Software-Delivery-Projection-Surface-Discipline-v1` => `CONTEXT_STATUS: OK`, `CLOSEOUT_READINESS: READY`, candidate branch/worktree/head/range matched the Orchestrator instruction.
  - `git status --short --branch` and `git rev-parse HEAD` in `../wtc-surface-discipline-v1` => feature branch clean at `4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd`.
  - `git diff --name-status 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd` => `.cargo/config.toml` is in the candidate range.
  - `rg -n "target-dir" -- .cargo/config.toml` in candidate worktree => line 4 points at `../Handshake Artifacts/handshake-cargo-target`.
  - `rg -n "Build/test/tool outputs MUST live|Handshake_Artifacts" -- AGENTS.md` in `handshake_main` => line 56 requires `../Handshake_Artifacts/`.
  - `just check-notifications ... INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` before verdict => no pending notifications.
- COUNTERFACTUAL_CHECKS:
  - If `.cargo/config.toml:4` remains pointed at `../Handshake Artifacts/handshake-cargo-target`, any Cargo proof generated after merge bypasses the repo-mandated `../Handshake_Artifacts/handshake-cargo-target` root and violates the build/test/tool-output guardrail even if product tests pass.
  - If Integration Validator runs tests before repairing `.cargo/config.toml:4`, the validation evidence itself is produced through the wrong artifact root and cannot support a legal PASS verdict.
  - If the candidate's product code were merged with the wrong target root, future worktrees would inherit divergent artifact placement and validator reruns would no longer share the repo's canonical external artifact root.
- BOUNDARY_PROBES:
  - Repo-control boundary: root `.cargo/config.toml` affects every Cargo proof path and is not one of the packet's product code surfaces.
  - Product/runtime boundary: candidate uses product-owned `.handshake/gov` runtime paths for projection artifacts while repo `/.GOV/**` remains evidence/projection input only.
  - Mailbox boundary: candidate exposes `SoftwareDeliveryOverlayTriageRowV1` as advisory stable-id navigation and does not give mailbox chronology direct mutation authority.
- NEGATIVE_PATH_CHECKS:
  - Candidate artifact root mismatch checked directly against `AGENTS.md:56`; result is a blocking FAIL.
  - Cargo tests intentionally not run from the unrepaired candidate because doing so would write outputs to the wrong artifact root.
  - Notification queue checked before verdict; no newer instruction superseded this FAIL finding.
- INDEPENDENT_FINDINGS:
  - The software-delivery product code appears to address the main projection-surface authority model at line level, but the candidate cannot be accepted because it carries a root Cargo output-path regression.
  - The WP Validator PASS did not catch the `.cargo/config.toml` artifact-root regression.
- RESIDUAL_UNCERTAINTY:
  - After `.cargo/config.toml` is repaired, Integration Validator still needs to run the required Cargo proof and exact WP regression tests from the corrected artifact root before any PASS can be considered.
  - I did not finish a whole-WP PASS-grade runtime test sweep because the environment/root-output violation must be fixed first.
- SPEC_CLAUSE_MAP:
  - v02.181 Projection-surface discipline => `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/locus/types.rs:2077`, `src/backend/handshake_core/src/locus/types.rs:2153`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3237`.
  - Software-delivery overlay runtime truth specialization => `src/backend/handshake_core/src/locus/task_board.rs:35`, `src/backend/handshake_core/src/locus/task_board.rs:203`.
  - Software-delivery closeout derivation => `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:3194`.
  - Software-delivery overlay extension records and lifecycle semantics => `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:2965`, `src/backend/handshake_core/src/locus/types.rs:3053`.
  - Role Mailbox authority boundary => `src/backend/handshake_core/src/role_mailbox.rs:230`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, `src/backend/handshake_core/src/role_mailbox.rs:1707`.
- NEGATIVE_PROOF:
  - The broader repo output-root requirement is not implemented in this candidate: `.cargo/config.toml:4` points at `../Handshake Artifacts/handshake-cargo-target` while `AGENTS.md:56` requires `../Handshake_Artifacts/handshake-cargo-target`.
- ANTI_VIBE_FINDINGS:
  - Blocking: the candidate contains a root tooling change that contradicts repo guardrails and is not justified by the packet's product-code scope.
- SIGNED_SCOPE_DEBT:
  - `.cargo/config.toml` artifact-root mutation is outside the packet's listed product code surfaces and must be reverted or explicitly governed before revalidation.
- PRIMITIVE_RETENTION_PROOF:
  - `SoftwareDeliveryProjectionSurfaceV1` remains present at `src/backend/handshake_core/src/locus/types.rs:1867`.
  - Canonical governed action family registry remains present at `src/backend/handshake_core/src/locus/types.rs:1975`.
  - Role Mailbox advisory triage row remains present at `src/backend/handshake_core/src/role_mailbox.rs:1681`.
- PRIMITIVE_RETENTION_GAPS:
  - Not fully proven because Integration Validator stopped before running the runtime test suite from a compliant artifact root.
- SHARED_SURFACE_INTERACTION_CHECKS:
  - `workflows.rs` applies micro-task packet canonical overrides before artifact write at `src/backend/handshake_core/src/workflows.rs:3643` and before structured registry validation at `src/backend/handshake_core/src/workflows.rs:13401`.
  - `storage/locus_sqlite.rs` applies progress metadata canonical overrides at `src/backend/handshake_core/src/storage/locus_sqlite.rs:288` and defines the override helper at `src/backend/handshake_core/src/storage/locus_sqlite.rs:304`.
  - `role_mailbox.rs` consumes the projection surface type instead of deriving authority from mailbox chronology at `src/backend/handshake_core/src/role_mailbox.rs:1707`.
- CURRENT_MAIN_INTERACTION_CHECKS:
  - Current local main baseline in the context brief was `660a1d5befa8ca083864730f8622e664b9c3eeef`; no merge was attempted.
  - Candidate `.cargo/config.toml` diverges from current main's expected artifact root and must be repaired before merge-tree or runtime proof can support PASS.
- DATA_CONTRACT_PROOF:
  - Candidate code exposes stable-id, JSON-serializable projection records in `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, and runtime path helpers in `src/backend/handshake_core/src/runtime_governance.rs:178`.
- DATA_CONTRACT_GAPS:
  - The LLM-first data contract was not fully revalidated through runtime tests because the candidate's Cargo artifact root is noncompliant.

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T16:14:30Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-160034
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: 4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: NOT_RUN
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: PARTIAL
ENVIRONMENT_VERDICT: FAIL
DISPOSITION: NONE
LEGAL_VERDICT: FAIL
SPEC_CONFIDENCE: PARTIAL_DIFF_SCOPED
WORKFLOW_VALIDITY: INVALID
SCOPE_VALIDITY: OUT_OF_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: PARTIAL
MECHANICAL_TRACK_VERDICT: FAIL
SPEC_RETENTION_TRACK_VERDICT: PARTIAL
MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
MERGED_MAIN_COMMIT: NONE
MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
VALIDATOR_RISK_TIER: HIGH
FAILURE_SUMMARY:
- Candidate `.cargo/config.toml:4` sets `target-dir = "../Handshake Artifacts/handshake-cargo-target"`.
- `AGENTS.md:56` requires all build/test/tool outputs to live under `../Handshake_Artifacts/`, including `handshake-cargo-target/`.
- This is a blocking governance/environment violation; Integration Validator stopped before Cargo proof and did not merge.
REMEDIATION_REQUIRED:
- Restore `.cargo/config.toml:4` to `target-dir = "../Handshake_Artifacts/handshake-cargo-target"`.
- Re-run final proof from the repaired branch using the corrected artifact root, including `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib`, the exact WP regression tests named in this packet, and `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR`.
- Do not delete `../Handshake Artifacts/` or any other output directory unless the Operator explicitly authorizes cleanup under repo destructive-cleanup rules.
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline: reviewed `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/locus/types.rs:2077`, `src/backend/handshake_core/src/locus/types.rs:2153`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3237`.
- Software-delivery overlay runtime truth specialization: reviewed `src/backend/handshake_core/src/locus/task_board.rs:35` and `src/backend/handshake_core/src/locus/task_board.rs:203`.
- Software-delivery closeout derivation: reviewed `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, and `src/backend/handshake_core/src/locus/types.rs:3194`.
- Software-delivery overlay extension records and lifecycle semantics: reviewed `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:2965`, and `src/backend/handshake_core/src/locus/types.rs:3053`.
- Role Mailbox authority boundary: reviewed `src/backend/handshake_core/src/role_mailbox.rs:230`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, and `src/backend/handshake_core/src/role_mailbox.rs:1707`.
NOT_PROVEN:
- v02.181 Projection-surface discipline: not cleared because the blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
- Software-delivery overlay runtime truth specialization: not cleared because the blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
- Software-delivery closeout derivation: not cleared because the blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
- Software-delivery overlay extension records and lifecycle semantics: not cleared because the blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
- Role Mailbox authority boundary: not cleared because the blocking `.cargo/config.toml` artifact-root violation prevents legal Integration Validator test proof.
MAIN_BODY_GAPS:
- `.cargo/config.toml:4` must use `../Handshake_Artifacts/handshake-cargo-target`, not `../Handshake Artifacts/handshake-cargo-target`.
QUALITY_RISKS:
- Wrong Cargo artifact routing fragments validation proof and makes future validator evidence non-comparable across worktrees.
DIFF_ATTACK_SURFACES:
- Root build-output routing via `.cargo/config.toml`.
- Projection-surface authority joins across `locus/types.rs`, `locus/task_board.rs`, `workflows.rs`, and `role_mailbox.rs`.
- Runtime canonical path validation in `runtime_governance.rs`.
- Micro-task packet/progress parity between workflow artifact emission and SQLite progress metadata.
INDEPENDENT_CHECKS_RUN:
- `just integration-validator-context-brief WP-1-Software-Delivery-Projection-Surface-Discipline-v1` => context OK and candidate branch/worktree/head/range matched assignment.
- `git status --short --branch` and `git rev-parse HEAD` in `../wtc-surface-discipline-v1` => feature branch clean at `4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd`.
- `git diff --name-status 45afc8867f08f7c2f8edfe71ab750fe92ab28866..4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd` => `.cargo/config.toml` is in the candidate range.
- `rg -n "target-dir" -- .cargo/config.toml` => candidate line 4 points at `../Handshake Artifacts/handshake-cargo-target`.
- `rg -n "Build/test/tool outputs MUST live|Handshake_Artifacts" -- AGENTS.md` => line 56 requires `../Handshake_Artifacts/`.
- `just check-notifications ... INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` => no pending notifications before verdict.
COUNTERFACTUAL_CHECKS:
- If `.cargo/config.toml:4` remains pointed at `../Handshake Artifacts/handshake-cargo-target`, Cargo proof bypasses the repo-mandated artifact root even if tests pass.
- If Integration Validator runs tests before repairing `.cargo/config.toml:4`, the validation evidence itself is produced through the wrong artifact root and cannot support PASS.
- If the candidate is merged with the wrong target root, future worktrees inherit divergent artifact placement and validator reruns cannot use the canonical external artifact root.
BOUNDARY_PROBES:
- Repo-control boundary: root `.cargo/config.toml` affects every Cargo proof path and is outside the packet product-code surfaces.
- Product/runtime boundary: candidate uses product-owned `.handshake/gov` runtime paths for projection artifacts while repo `/.GOV/**` remains evidence/projection input only.
- Mailbox boundary: candidate exposes `SoftwareDeliveryOverlayTriageRowV1` as advisory stable-id navigation and does not give mailbox chronology direct mutation authority.
NEGATIVE_PATH_CHECKS:
- Candidate artifact-root mismatch checked directly against `AGENTS.md:56`; result is blocking FAIL.
- Cargo tests intentionally not run from the unrepaired candidate because doing so would write outputs to the wrong artifact root.
- Notification queue checked before verdict; no newer instruction superseded this FAIL finding.
INDEPENDENT_FINDINGS:
- Product code appears to address the main projection-surface authority model at line level, but the candidate cannot be accepted while carrying the root Cargo output-path regression.
- The WP Validator PASS did not catch the `.cargo/config.toml` artifact-root regression.
RESIDUAL_UNCERTAINTY:
- After `.cargo/config.toml` is repaired, Integration Validator still needs to run Cargo proof and exact WP regression tests from the corrected artifact root before any PASS can be considered.
- I did not finish a whole-WP PASS-grade runtime test sweep because the environment/root-output violation must be fixed first.
SPEC_CLAUSE_MAP:
- v02.181 Projection-surface discipline => `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/locus/types.rs:2077`, `src/backend/handshake_core/src/locus/types.rs:2153`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3237`.
- Software-delivery overlay runtime truth specialization => `src/backend/handshake_core/src/locus/task_board.rs:35`, `src/backend/handshake_core/src/locus/task_board.rs:203`.
- Software-delivery closeout derivation => `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:3194`.
- Software-delivery overlay extension records and lifecycle semantics => `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:2965`, `src/backend/handshake_core/src/locus/types.rs:3053`.
- Role Mailbox authority boundary => `src/backend/handshake_core/src/role_mailbox.rs:230`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, `src/backend/handshake_core/src/role_mailbox.rs:1707`.
NEGATIVE_PROOF:
- The broader repo output-root requirement is not implemented in this candidate: `.cargo/config.toml:4` points at `../Handshake Artifacts/handshake-cargo-target` while `AGENTS.md:56` requires `../Handshake_Artifacts/handshake-cargo-target`.
ANTI_VIBE_FINDINGS:
- Blocking: the candidate contains a root tooling change that contradicts repo guardrails and is not justified by packet product-code scope.
SIGNED_SCOPE_DEBT:
- `.cargo/config.toml` artifact-root mutation is outside the packet's listed product code surfaces and must be reverted or explicitly governed before revalidation.
PRIMITIVE_RETENTION_PROOF:
- `SoftwareDeliveryProjectionSurfaceV1` remains present at `src/backend/handshake_core/src/locus/types.rs:1867`.
- Canonical governed action family registry remains present at `src/backend/handshake_core/src/locus/types.rs:1975`.
- Role Mailbox advisory triage row remains present at `src/backend/handshake_core/src/role_mailbox.rs:1681`.
PRIMITIVE_RETENTION_GAPS:
- Not fully proven because Integration Validator stopped before running the runtime test suite from a compliant artifact root.
SHARED_SURFACE_INTERACTION_CHECKS:
- `workflows.rs` applies micro-task packet canonical overrides before artifact write at `src/backend/handshake_core/src/workflows.rs:3643` and before structured registry validation at `src/backend/handshake_core/src/workflows.rs:13401`.
- `storage/locus_sqlite.rs` applies progress metadata canonical overrides at `src/backend/handshake_core/src/storage/locus_sqlite.rs:288` and defines the override helper at `src/backend/handshake_core/src/storage/locus_sqlite.rs:304`.
- `role_mailbox.rs` consumes the projection surface type instead of deriving authority from mailbox chronology at `src/backend/handshake_core/src/role_mailbox.rs:1707`.
CURRENT_MAIN_INTERACTION_CHECKS:
- Current local main baseline in the context brief was `660a1d5befa8ca083864730f8622e664b9c3eeef`; no merge was attempted.
- Candidate `.cargo/config.toml` diverges from current main's expected artifact root and must be repaired before merge-tree or runtime proof can support PASS.
DATA_CONTRACT_PROOF:
- Candidate code exposes stable-id, JSON-serializable projection records in `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, and runtime path helpers in `src/backend/handshake_core/src/runtime_governance.rs:178`.
DATA_CONTRACT_GAPS:
- The LLM-first data contract was not fully revalidated through runtime tests because the candidate's Cargo artifact root is noncompliant.

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T20:48:12Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-201125
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: eb59e9819c7cc2729c169c723dab3932d5d7b9d4
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4
MAIN_MERGE_COMMIT: 4a5534925af14bf994344182fb5c863cacba6741
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
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
MERGED_MAIN_COMMIT: 4a5534925af14bf994344182fb5c863cacba6741
MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-28T20:48:12Z
VALIDATOR_RISK_TIER: HIGH
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline.
- Software-delivery overlay runtime truth specialization.
- Software-delivery closeout derivation.
- Software-delivery overlay extension records and lifecycle semantics.
- Role Mailbox authority boundary.
- Governed action preview payload.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- Existing Rust warnings remain outside this diff-scoped proof; they did not block `cargo check --lib`, the exact packet tripwires, or the integration-test compile target.
SPEC_CLAUSE_MAP:
- v02.181 Projection-surface discipline (`Handshake_Master_Spec_v02.181.md:48046`, `Handshake_Master_Spec_v02.181.md:6917`): implemented by `src/backend/handshake_core/src/locus/types.rs:1944`, `src/backend/handshake_core/src/locus/types.rs:1997`, `src/backend/handshake_core/src/locus/types.rs:2259`, `src/backend/handshake_core/src/locus/types.rs:2297`, and `src/backend/handshake_core/src/workflows.rs:4682`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3240`.
- Software-delivery overlay runtime truth specialization (`Handshake_Master_Spec_v02.181.md:6917`-`Handshake_Master_Spec_v02.181.md:6920`): implemented by `src/backend/handshake_core/src/locus/task_board.rs:30`, `src/backend/handshake_core/src/locus/task_board.rs:203`, and `src/backend/handshake_core/src/locus/types.rs:2339`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3850`.
- Software-delivery closeout derivation (`Handshake_Master_Spec_v02.181.md:7032`-`Handshake_Master_Spec_v02.181.md:7036`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:2568`, and `src/backend/handshake_core/src/locus/types.rs:2716`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3961`.
- Software-delivery overlay extension records and lifecycle semantics (`Handshake_Master_Spec_v02.181.md:7038`-`Handshake_Master_Spec_v02.181.md:7048`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:3151`, `src/backend/handshake_core/src/locus/types.rs:3239`, and `src/backend/handshake_core/src/workflows.rs:5101`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:4765`.
- Role Mailbox authority boundary (`Handshake_Master_Spec_v02.181.md:10661`): implemented by `src/backend/handshake_core/src/role_mailbox.rs:227`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, and `src/backend/handshake_core/src/role_mailbox.rs:1707`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081`.
- Governed action preview payload (`packet.md:459`, `Handshake_Master_Spec_v02.181.md:6993`-`Handshake_Master_Spec_v02.181.md:6999`): implemented by `src/backend/handshake_core/src/locus/types.rs:1858`, `src/backend/handshake_core/src/locus/types.rs:1895`, `src/backend/handshake_core/src/locus/types.rs:2167`, `src/backend/handshake_core/src/locus/types.rs:2218`, and `src/backend/handshake_core/src/locus/types.rs:2297`; proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`.
NEGATIVE_PROOF:
- No signed packet product requirement remained unimplemented in candidate `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- v02.181 also describes broader software-delivery control-plane health/backpressure and push-oriented operator alerts (`Handshake_Master_Spec_v02.181.md:7058`-`Handshake_Master_Spec_v02.181.md:7059`); this candidate does not implement those broader alerting/backpressure surfaces, which are outside the signed packet scope.
- The signed backend range does not add frontend widgets; it exposes backend projection/control fields for later UI consumers.
INDEPENDENT_CHECKS_RUN:
- `just check-notifications WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` and `just ack-notifications ...` returned no pending notifications before verdict and before merge commit.
- `just phase-check STARTUP WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` passed.
- `just -f ../wt-gov-kernel/justfile artifact-hygiene-check` passed.
- `just validator-git-hygiene` passed.
- `git merge-tree --write-tree HEAD eb59e9819c7cc2729c169c723dab3932d5d7b9d4` produced clean tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`, and `git diff --check HEAD 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` passed.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib` passed on candidate and committed local `main`.
- Exact tests passed on candidate and committed local `main`: `projection_surface_previews_governed_action_before_mutation`, `dcc_software_delivery_projection_surface_keeps_runtime_authority`, `task_board_software_delivery_projection_cannot_override_runtime_truth`, `closeout_projection_requires_gate_evidence_and_owner_truth`, `projection_surface_exposes_claim_and_queued_instruction_ids`, `role_mailbox_software_delivery_triage_remains_advisory`, `locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait`, `locus_sync_task_board_validation_reports_authority_scope_drift`, and `production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests --no-run` passed on candidate and committed local `main`.
- `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR` passed.
COUNTERFACTUAL_CHECKS:
- If governed-action previews stopped sourcing canonical allowed action ids, DCC/Task Board/mailbox consumers could preview actions not authorized by runtime truth.
- If projection derivation stopped embedding `governed_action_previews`, downstream software-delivery surfaces would not receive the packet-required preview payload before mutation.
- If Task Board validation did not compare canonical state/queue/action/status fields, stale mirrors could override runtime truth.
- If Role Mailbox triage wrote back to runtime records, mailbox chronology could become hidden authority.
- If closeout reference checks accepted substring matches, spoofed validator-gate, claim/lease, or queued-instruction refs could satisfy authority checks.
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
PRIMITIVE_RETENTION_GAPS:
- NONE
DATA_CONTRACT_GAPS:
- NONE

## ORCHESTRATOR_REMEDIATION_EVIDENCE
- Scope: post-Integration-Validator artifact-root remediation after terminal FAIL on candidate `4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd`.
- Candidate head now under review: `2d6c40193cbf28ef244198e9b09ccbe855a2dd54`.
- Command: `just phase-check HANDOFF WP-1-Software-Delivery-Projection-Surface-Discipline-v1 CODER --range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..2d6c40193cbf28ef244198e9b09ccbe855a2dd54`
  - Exit code: `0`
  - Artifact: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/2026-04-28T17-02-53-129Z.log`
- Signed-scope note: `.cargo/config.toml` is recorded as `Containment-Only: YES` because `2d6c4019` restores the file to the baseline artifact root; it is absent from the final `45afc886..2d6c4019` candidate diff but present in the signed patch artifact as remediation containment evidence from `4dee21bd..2d6c4019`.

## INTEGRATION_VALIDATOR_FINAL_REVIEW_2026-04-28T17-35Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-172625
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: 2d6c40193cbf28ef244198e9b09ccbe855a2dd54
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..2d6c40193cbf28ef244198e9b09ccbe855a2dd54
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: FAIL
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: PARTIAL
ENVIRONMENT_VERDICT: PARTIAL
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: INCOMPLETE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
VALIDATOR_RISK_TIER: HIGH
FAILURE_SUMMARY:
- The artifact-root remediation is present: `.cargo/config.toml:4` now routes Cargo output to `../Handshake_Artifacts/handshake-cargo-target`, and `.cargo/config.toml` is absent from the final `45afc886..2d6c4019` candidate diff.
- The packet-required tripwire `projection_surface_previews_governed_action_before_mutation` is not implemented in the candidate code or tests. `rg` finds the name only in the packet, and the targeted integration-test invocation runs `0 tests`.
- The packet data contract still requires a governed action preview payload with `action_request_id`, target record refs, eligibility, blockers, and evidence refs before mutation. I found no corresponding preview payload type, builder, or test in the reviewed product surfaces.
- Therefore the candidate does not fully satisfy the signed packet contract and must not be merged.
REMEDIATION_REQUIRED:
- Implement the governed-action preview payload required by the packet contract row: include `action_request_id`, target record refs, eligibility, blockers, and evidence refs; expose it before any mutation path used by DCC quick actions, Task Board row actions, or mailbox escalation controls.
- Add and run the packet-required exact tripwire `projection_surface_previews_governed_action_before_mutation`; it must fail if preview construction mutates canonical runtime state or skips policy/evidence gates.
- Keep `.cargo/config.toml:4` at `target-dir = "../Handshake_Artifacts/handshake-cargo-target"` and rerun final proof from the corrected artifact root.
- Re-run `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib`, the WP integration-test target regressions, and `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR`.
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline: `src/backend/handshake_core/src/locus/types.rs:1867`, `src/backend/handshake_core/src/locus/types.rs:2077`, `src/backend/handshake_core/src/locus/types.rs:2153`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3237`.
- Software-delivery overlay runtime truth specialization: `src/backend/handshake_core/src/locus/task_board.rs:35`, `src/backend/handshake_core/src/locus/task_board.rs:203`, `src/backend/handshake_core/src/locus/types.rs:2297`.
- Software-delivery closeout derivation: `src/backend/handshake_core/src/runtime_governance.rs:330`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:3194`.
- Software-delivery overlay extension records and lifecycle semantics: `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:2965`, `src/backend/handshake_core/src/locus/types.rs:3053`, `src/backend/handshake_core/src/workflows.rs:5101`.
- Role Mailbox authority boundary: `src/backend/handshake_core/src/role_mailbox.rs:230`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, `src/backend/handshake_core/src/role_mailbox.rs:1707`.
- Governed action preview payload: NOT IMPLEMENTED / NOT PROVEN; packet references at lines 228, 237, and 459 remain unmatched by candidate code.
NOT_PROVEN:
- Governed action preview payload before mutation is not proven because the required test is absent and the searched product surfaces contain no preview payload implementation.
- Full packet semantic proof is not proven because the packet-required tripwire suite includes a zero-test target.
DIFF_ATTACK_SURFACES:
- Contract row drift between packet-required governed-action preview payload and implemented projection surfaces.
- False positive test proof when a named `--exact` tripwire runs zero tests.
- Producer/consumer mismatch between DCC/Task Board/mailbox action controls and the runtime projection payload they are required to inspect before mutation.
- Runtime canonical path and stable-id checks across projection, closeout, claim/lease, queued-instruction, and mailbox triage surfaces.
INDEPENDENT_CHECKS_RUN:
- `just integration-validator-context-brief WP-1-Software-Delivery-Projection-Surface-Discipline-v1` => context OK, candidate head `2d6c4019`, range `45afc886..2d6c4019`, main compatibility compatible.
- `just check-notifications ... INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` and `just ack-notifications ...` => no pending notifications before verdict.
- `just phase-check STARTUP ...`, `just phase-check VERDICT ...`, and `just phase-check CLOSEOUT ...` => all PASS; CLOSEOUT artifact `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-closeout/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/2026-04-28T17-35-02-766Z.log`.
- `rg -n "target-dir|Handshake_Artifacts|Handshake Artifacts" -- .cargo/config.toml AGENTS.md` in candidate => `.cargo/config.toml:4` uses `../Handshake_Artifacts/handshake-cargo-target`.
- `git diff --name-status 4dee21bd97b35e4b7591dd0a39e2d3e34dd706cd..2d6c40193cbf28ef244198e9b09ccbe855a2dd54` => `.cargo/config.toml` remediation only.
- `git diff --check 45afc8867f08f7c2f8edfe71ab750fe92ab28866..2d6c40193cbf28ef244198e9b09ccbe855a2dd54` => PASS.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib` => PASS with warnings.
- Raw `cargo test --manifest-path src/backend/handshake_core/Cargo.toml dcc_software_delivery_projection_surface_keeps_runtime_authority -- --exact` => FAIL while compiling unrelated lib-test targets (`FlightRecorderEvent::with_activity_span_id` and `ModelSession` fixture fields), before the integration test ran.
- Targeted integration-test invocations with `--test micro_task_executor_tests` passed for the implemented packet tests and independent probes, but `projection_surface_previews_governed_action_before_mutation` ran `0 tests`.
- `rg -n "projection_surface_previews_governed_action_before_mutation|previews_governed|governed_action_before_mutation|preview.*governed"` over candidate `src/` and `tests/` => no candidate code/test hit.
- `rg -n "preview|Preview|action_request_id|eligibility|required evidence|required_evidence|governed_action"` over reviewed product surfaces => no governed-action preview payload matching the packet contract.
COUNTERFACTUAL_CHECKS:
- If `projection_surface_previews_governed_action_before_mutation` remains absent, a future candidate can claim packet proof while the named exact tripwire executes zero tests.
- If DCC/Task Board/mailbox actions do not consume a preview payload with target refs, eligibility, blockers, and evidence refs before mutation, UI action controls can skip the policy/evidence gate the packet explicitly requires.
- If `derive_software_delivery_projection_surface_with_overlay` stopped rejecting foreign-WP claim/lease or queued-instruction records, stable-id overlay posture could be projected for the wrong work item.
- If `build_software_delivery_overlay_triage_row` wrote back to canonical records, mailbox chronology could become hidden authority; the candidate tests currently protect this boundary.
BOUNDARY_PROBES:
- Producer/consumer: packet contract requires a governed-action preview payload for DCC/Task Board/mailbox consumers; searched candidate producer surfaces do not define it.
- Writer/reader: `apply_software_delivery_projection_surface_lifecycle` writes canonical projection artifacts from runtime summaries and overlay records; mailbox triage reads a clone-only advisory row.
- Authority boundary: Task Board row validation flags authority-field drift while allowing advisory mirror/lane/status drift.
NEGATIVE_PATH_CHECKS:
- Missing-test negative path: exact candidate invocation for `projection_surface_previews_governed_action_before_mutation` in the integration-test target returned zero tests.
- Preview-payload negative path: searched for `action_request_id`, preview eligibility, blockers, and evidence refs in the reviewed product surfaces; no implementation was present.
- Raw Cargo exact-command negative path: the packet's command form currently compiles unrelated lib-test targets and fails before reaching the WP integration test.
INDEPENDENT_FINDINGS:
- Blocking: packet-required governed-action preview test and payload are absent.
- Non-blocking for this verdict but residual: raw `cargo test <name> -- --exact` remains unsafe as packet proof because unrelated lib-test compilation fails; the WP integration-test target is the only runnable narrow proof observed in this review.
SPEC_CLAUSE_MAP:
- PASS-PARTIAL: v02.181 projection-surface discipline is implemented for canonical field lifting and advisory board/mailbox state at `locus/types.rs:1867`, `locus/types.rs:2077`, and `locus/types.rs:2153`.
- PASS-PARTIAL: Task Board stale mirror authority rejection is implemented at `locus/types.rs:2297` and production helper wiring at `locus/task_board.rs:203`.
- PASS-PARTIAL: Closeout canonical gate/owner proof is implemented at `runtime_governance.rs:369`, `runtime_governance.rs:390`, and `locus/types.rs:3194`.
- PASS-PARTIAL: Claim/lease and queued-instruction stable-id overlay projection is implemented at `locus/types.rs:2965` and `locus/types.rs:3053`.
- PASS-PARTIAL: Role Mailbox triage remains advisory at `role_mailbox.rs:1707`.
- FAIL: Governed action preview payload before mutation from packet line 459 is not implemented/proven.
NEGATIVE_PROOF:
- At least one signed requirement is not fully implemented: `projection_surface_previews_governed_action_before_mutation` is required by packet lines 224-238 and the governed action preview payload contract at line 459, but no matching test or implementation exists in candidate product code.
RESIDUAL_UNCERTAINTY:
- I did not merge or run post-merge spotchecks because the missing preview contract is a merge blocker.
- The targeted WP tests that exist pass, but they do not cover the absent governed-action preview contract.

## CODER_REMEDIATION_EVIDENCE_2026-04-28T18-10Z
- Scope: post-Integration-Validator preview-contract remediation after terminal FAIL on candidate `2d6c40193cbf28ef244198e9b09ccbe855a2dd54`. Adds the missing governed-action preview payload type plus the packet-required exact tripwire `projection_surface_previews_governed_action_before_mutation`.
- Candidate head now under review: `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- Command: `just phase-check HANDOFF WP-1-Software-Delivery-Projection-Surface-Discipline-v1 CODER --range 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4`
  - Exit code: `0`
  - Artifact: `../gov_runtime/roles_shared/GATE_OUTPUTS/phase-check-handoff/WP-1-Software-Delivery-Projection-Surface-Discipline-v1/2026-04-28T18-10-17-878Z.log`
- Verification: cargo check --lib PASS; the new tripwire `projection_surface_previews_governed_action_before_mutation` PASS via `cargo test --test micro_task_executor_tests ... -- --exact`; all 4 prior packet-required tripwires PASS; full projection regression set (15 tests including production_finalize_emits_software_delivery_projection_surface_with_overlay) PASS; integration target `--no-run` compiles; merge-tree vs origin/main reports 0 conflict markers.
- Signed-scope note: only `src/backend/handshake_core/src/locus/types.rs` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs` are touched in the `45afc886..eb59e981` candidate diff; `.cargo/config.toml` is preserved at the IntVal-required `../Handshake_Artifacts/handshake-cargo-target` artifact root.

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T19:36:00Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-191829
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: eb59e9819c7cc2729c169c723dab3932d5d7b9d4
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4
Verdict: FAIL
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PARTIAL
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: FAIL
DISPOSITION: NONE
LEGAL_VERDICT: FAIL
SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
WORKFLOW_VALIDITY: BLOCKED
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: COMPLETE
MECHANICAL_TRACK_VERDICT: FAIL
SPEC_RETENTION_TRACK_VERDICT: PASS
MAIN_CONTAINMENT_STATUS: NOT_REQUIRED
MERGED_MAIN_COMMIT: NONE
MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
VALIDATOR_RISK_TIER: HIGH
FAILURE_SUMMARY:
- Candidate `eb59e9819c7cc2729c169c723dab3932d5d7b9d4` satisfies the signed product-code projection-surface contract under independent review: the prior missing governed-action preview payload and exact tripwire are now present and passing.
- The hard pre-merge artifact hygiene gate fails in `handshake_main`: `just validator-git-hygiene` reports `Handshake Artifacts: noncanonical sibling artifact root; use Handshake_Artifacts and remove or quarantine this root after review (D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake Artifacts)`.
- Per Integration Validator protocol section "Artifact Hygiene Pre-Merge Check (HARD)", I did not merge or sync main while that gate was failing.
- This is not a Coder remediation item. It is a repo-governance/environment cleanup blocker for the Orchestrator: quarantine or remove the noncanonical sibling artifact root, rerun `just validator-git-hygiene`, then resume Integration Validator containment.
REMEDIATION_REQUIRED:
- Orchestrator should review `../Handshake Artifacts/handshake-cargo-target` and either quarantine it outside the active worktree sibling root or remove it through an approved/sanctioned cleanup path.
- After cleanup, rerun `just validator-git-hygiene`, `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR`, and `just phase-check CLOSEOUT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1`.
- If hygiene remains clean, apply the contained current-main merge tree proven by `git merge-tree --write-tree --merge-base 45afc8867f08f7c2f8edfe71ab750fe92ab28866 --name-only main eb59e9819c7cc2729c169c723dab3932d5d7b9d4`, which produced tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` with a diff limited to signed product/test files and `git diff --check main 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` PASS.
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline.
- Software-delivery overlay runtime truth specialization.
- Software-delivery closeout derivation.
- Software-delivery overlay extension records and lifecycle semantics.
- Role Mailbox authority boundary.
- Governed action preview payload.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- Artifact hygiene blocks merge readiness, but no product-code quality risk was found inside the signed range.
SPEC_CLAUSE_MAP:
- v02.181 Projection-surface discipline: `src/backend/handshake_core/src/locus/types.rs:1944`, `src/backend/handshake_core/src/locus/types.rs:1997`, `src/backend/handshake_core/src/locus/types.rs:2265`, `src/backend/handshake_core/src/workflows.rs:4682`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3240`.
- Software-delivery overlay runtime truth specialization: `src/backend/handshake_core/src/locus/task_board.rs:37`, `src/backend/handshake_core/src/locus/types.rs:2340`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3850`.
- Software-delivery closeout derivation: `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:2568`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3961`.
- Software-delivery overlay extension records and lifecycle semantics: `src/backend/handshake_core/src/locus/types.rs:3162`, `src/backend/handshake_core/src/locus/types.rs:3240`, `src/backend/handshake_core/src/workflows.rs:5100`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:4765`.
- Role Mailbox authority boundary: `src/backend/handshake_core/src/role_mailbox.rs:1676`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, `src/backend/handshake_core/src/role_mailbox.rs:1707`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081`.
- Governed action preview payload: `src/backend/handshake_core/src/locus/types.rs:1859`, `src/backend/handshake_core/src/locus/types.rs:1895`, `src/backend/handshake_core/src/locus/types.rs:2167`, `src/backend/handshake_core/src/locus/types.rs:2218`, `src/backend/handshake_core/src/locus/types.rs:2297`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`.
NEGATIVE_PROOF:
- Product-code negative proof: no signed product requirement remained unimplemented in candidate `eb59e981` after the preview remediation review.
- Merge-readiness negative proof: the hard artifact hygiene requirement is not satisfied because `../Handshake Artifacts/` exists as a noncanonical sibling artifact root while canonical output must use `../Handshake_Artifacts/`.
DIFF_ATTACK_SURFACES:
- Producer/consumer drift between the runtime projection producer and DCC, Task Board, and Role Mailbox consumers.
- Serialized payload drift for `SoftwareDeliveryProjectionSurfaceV1` and the new `GovernedActionPreviewV1` field.
- False positive exact-test proof if `projection_surface_previews_governed_action_before_mutation` ran zero tests.
- Current-main containment risk because the assigned candidate worktree is in the kernel-backed repo and direct branch merge would include unrelated governance/root diffs; only the signed range may be contained.
- Artifact-root hygiene regression risk from the prior `Handshake Artifacts` vs `Handshake_Artifacts` split.
INDEPENDENT_CHECKS_RUN:
- `git -C ../wtc-surface-discipline-v1 rev-parse HEAD` => `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- `git -C ../wtc-surface-discipline-v1 diff --name-status 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4` => nine signed Rust/test files; no `.GOV/**`, `AGENTS.md`, `justfile`, or `.cargo/config.toml` in the signed range.
- `rg -n "projection_surface_previews_governed_action_before_mutation|GovernedActionPreview|action_request_id|eligibility" -- src/backend/handshake_core/src/locus/types.rs src/backend/handshake_core/tests/micro_task_executor_tests.rs` => preview type, builder, embedded projection list, and exact tripwire present.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib` => PASS with existing warnings.
- Exact packet tripwires all PASS with one test each: `projection_surface_previews_governed_action_before_mutation`, `dcc_software_delivery_projection_surface_keeps_runtime_authority`, `task_board_software_delivery_projection_cannot_override_runtime_truth`, `closeout_projection_requires_gate_evidence_and_owner_truth`, `projection_surface_exposes_claim_and_queued_instruction_ids`, and `role_mailbox_software_delivery_triage_remains_advisory`.
- Independent probes not required by the coder handoff PASS: `locus_sync_task_board_validation_reports_authority_scope_drift` and `production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests --no-run` => PASS.
- `git merge-tree --write-tree --merge-base 45afc8867f08f7c2f8edfe71ab750fe92ab28866 --name-only main eb59e9819c7cc2729c169c723dab3932d5d7b9d4` => clean contained tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`.
- `git diff --name-status main 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` => seven signed product/test files only; `git diff --check main 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` => PASS.
- `just check-notifications ... INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` and `just ack-notifications ...` => no pending notifications before verdict.
- `just phase-check STARTUP ...`, `just phase-check VERDICT ...`, and `just phase-check CLOSEOUT ...` => PASS.
- `just validator-git-hygiene` => FAIL on the noncanonical sibling artifact root `../Handshake Artifacts`.
COUNTERFACTUAL_CHECKS:
- If `GovernedActionPreviewV1` were removed from `locus/types.rs`, the projection payload would no longer carry `action_request_id`, target refs, eligibility, blockers, and evidence refs before mutation.
- If `derive_governed_action_previews` stopped sourcing `canonical.allowed_action_ids`, DCC/Task Board/mailbox consumers could see actions not authorized by canonical runtime truth.
- If `derive_software_delivery_projection_surface` stopped embedding `governed_action_previews`, the remediated preview payload would exist as an orphan helper rather than as the shared projection record.
- If `build_software_delivery_overlay_triage_row` wrote back to canonical records instead of cloning advisory stable ids, mailbox chronology could become hidden authority.
- If containment used direct branch merge instead of the explicit signed-range merge base, unrelated `.GOV/**`, root, and historical product diffs from the kernel-backed candidate repo could enter `main`.
BOUNDARY_PROBES:
- Producer/consumer boundary: verified `SoftwareDeliveryProjectionSurfaceV1` now embeds preview rows and the production wrapper consumes the same projection surface.
- Writer/reader boundary: verified the preview tripwire snapshots canonical bytes before/after derivation and requires serde round-trip retention.
- Profile boundary: independent negative test `production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary` passed.
- Authority boundary: independent test `locus_sync_task_board_validation_reports_authority_scope_drift` passed, and Role Mailbox triage remains advisory.
- Repo-governance boundary: direct branch diff from kernel-backed candidate to `main` was rejected as unsafe; explicit signed merge-base containment produced a clean signed-surface tree.
NEGATIVE_PATH_CHECKS:
- Unregistered governed action `fabricated_action` returns `None` in the preview tripwire.
- Out-of-family registered action `approve` yields `IneligibleOutOfFamily` for a Validation-family canonical summary.
- Non-software-delivery canonical summary yields no preview and no projection surface.
- Forged preview target refs diverge from canonical authority refs in the tripwire.
- Artifact hygiene negative path failed as expected on stale/noncanonical `../Handshake Artifacts`.
INDEPENDENT_FINDINGS:
- Blocking: pre-merge artifact hygiene is not clean because `../Handshake Artifacts` exists beside the canonical `../Handshake_Artifacts`.
- Non-blocking for product code: candidate `eb59e981` satisfies the product-code and test contract I reviewed.
- Non-blocking containment note: current-main containment is possible through the explicit signed merge base and stays inside the signed product/test file surface, but I did not apply it because hygiene failed first.
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
PRIMITIVE_RETENTION_PROOF:
- `SoftwareDeliveryProjectionSurfaceV1` remains present at `src/backend/handshake_core/src/locus/types.rs:1944`.
- `GovernedActionPreviewV1` remains present at `src/backend/handshake_core/src/locus/types.rs:1895`.
- Role Mailbox advisory triage remains present at `src/backend/handshake_core/src/role_mailbox.rs:1681`.
PRIMITIVE_RETENTION_GAPS:
- NONE
SHARED_SURFACE_INTERACTION_CHECKS:
- `src/backend/handshake_core/src/workflows.rs:4682` calls the shared projection derivation wrapper for DCC, Task Board, and Role Mailbox projection consumers.
- `src/backend/handshake_core/src/role_mailbox.rs:1707` builds mailbox triage from the already-derived projection surface rather than mailbox chronology.
- `src/backend/handshake_core/src/locus/types.rs:2297` embeds governed-action previews in the same projection surface consumed by downstream projection readers.
CURRENT_MAIN_INTERACTION_CHECKS:
- Explicit signed merge-base containment against current `main` produced tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`.
- `git diff --name-status main 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` stayed inside signed product/test files.
- `git diff --check main 56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` passed.
DATA_CONTRACT_PROOF:
- `GovernedActionPreviewV1` is JSON-serializable and round-tripped by the exact tripwire; required fields include `action_request_id`, `target_record_refs`, `eligibility`, `blockers`, and `evidence_refs`.
- `SoftwareDeliveryProjectionSurfaceV1` embeds `governed_action_previews` so downstream projection readers receive the preview payload before mutation.
DATA_CONTRACT_GAPS:
- No product-code data contract gap found for this candidate. The remaining gap is environment/governance hygiene.
RESIDUAL_UNCERTAINTY:
- I did not run post-merge tests because no merge was performed after the hard hygiene gate failed.
- I did not quarantine or delete `../Handshake Artifacts` because destructive cleanup of a noncanonical sibling artifact root requires Orchestrator/Operator-directed cleanup policy outside this validator verdict.

### INTEGRATION_VALIDATOR_REPORT - 2026-04-28T20:18:48Z
ACTOR_ROLE: INTEGRATION_VALIDATOR
ACTOR_SESSION: integration_validator:wp-1-software-delivery-projection-surface-discipline-v1
REPOMEM_SESSION: INTEGRATION_VALIDATOR-20260428-201125
CANDIDATE_BRANCH: feat/WP-1-Software-Delivery-Projection-Surface-Discipline-v1
CANDIDATE_WORKTREE: ../wtc-surface-discipline-v1
CANDIDATE_HEAD: eb59e9819c7cc2729c169c723dab3932d5d7b9d4
HANDOFF_RANGE: 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4
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
MECHANICAL_TRACK_VERDICT: PASS
SPEC_RETENTION_TRACK_VERDICT: PASS
MAIN_CONTAINMENT_STATUS: MERGE_PENDING
MERGED_MAIN_COMMIT: NONE
MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-04-28T20:18:31Z
VALIDATOR_RISK_TIER: HIGH
CLAUSES_REVIEWED:
- v02.181 Projection-surface discipline.
- Software-delivery overlay runtime truth specialization.
- Software-delivery closeout derivation.
- Software-delivery overlay extension records and lifecycle semantics.
- Role Mailbox authority boundary.
- Governed action preview payload.
NOT_PROVEN:
- NONE
MAIN_BODY_GAPS:
- NONE
QUALITY_RISKS:
- Existing Rust warnings remain, including unused variables in `src/backend/handshake_core/src/workflows.rs:4479` and `src/backend/handshake_core/src/workflows.rs:4480`; they are not introduced as blocking runtime behavior in this diff-scoped review.
SPEC_CLAUSE_MAP:
- v02.181 Projection-surface discipline (`Handshake_Master_Spec_v02.181.md:48046`, `Handshake_Master_Spec_v02.181.md:6917`): implemented by `src/backend/handshake_core/src/locus/types.rs:1944`, `src/backend/handshake_core/src/locus/types.rs:1997`, `src/backend/handshake_core/src/locus/types.rs:2259`, `src/backend/handshake_core/src/locus/types.rs:2297`, `src/backend/handshake_core/src/workflows.rs:4682`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3240`.
- Software-delivery overlay runtime truth specialization (`Handshake_Master_Spec_v02.181.md:6917`-`Handshake_Master_Spec_v02.181.md:6920`): implemented by `src/backend/handshake_core/src/locus/task_board.rs:30`, `src/backend/handshake_core/src/locus/task_board.rs:203`, `src/backend/handshake_core/src/locus/types.rs:2339`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751` and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3850`.
- Software-delivery closeout derivation (`Handshake_Master_Spec_v02.181.md:7032`-`Handshake_Master_Spec_v02.181.md:7036`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:319`, `src/backend/handshake_core/src/runtime_governance.rs:369`, `src/backend/handshake_core/src/locus/types.rs:2568`, `src/backend/handshake_core/src/locus/types.rs:2716`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3961`.
- Software-delivery overlay extension records and lifecycle semantics (`Handshake_Master_Spec_v02.181.md:7038`-`Handshake_Master_Spec_v02.181.md:7048`): implemented by `src/backend/handshake_core/src/runtime_governance.rs:409`, `src/backend/handshake_core/src/runtime_governance.rs:440`, `src/backend/handshake_core/src/runtime_governance.rs:468`, `src/backend/handshake_core/src/runtime_governance.rs:500`, `src/backend/handshake_core/src/locus/types.rs:3151`, `src/backend/handshake_core/src/locus/types.rs:3239`, `src/backend/handshake_core/src/workflows.rs:5101`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:4765`.
- Role Mailbox authority boundary (`Handshake_Master_Spec_v02.181.md:10661`): implemented by `src/backend/handshake_core/src/role_mailbox.rs:227`, `src/backend/handshake_core/src/role_mailbox.rs:1681`, `src/backend/handshake_core/src/role_mailbox.rs:1707`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081`.
- Governed action preview payload (`packet.md:459`, `Handshake_Master_Spec_v02.181.md:6993`-`Handshake_Master_Spec_v02.181.md:6999`): implemented by `src/backend/handshake_core/src/locus/types.rs:1858`, `src/backend/handshake_core/src/locus/types.rs:1895`, `src/backend/handshake_core/src/locus/types.rs:2167`, `src/backend/handshake_core/src/locus/types.rs:2218`, `src/backend/handshake_core/src/locus/types.rs:2297`, and proven by `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`.
NEGATIVE_PROOF:
- Product-code negative proof: no signed packet product requirement remained unimplemented in candidate `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- Required broader-scope negative proof: v02.181 also describes software-delivery control-plane health/backpressure and push-oriented operator alerts (`Handshake_Master_Spec_v02.181.md:7058`-`Handshake_Master_Spec_v02.181.md:7059`); this candidate does not implement those broader alerting/backpressure surfaces. That is outside the signed CLAUSE_PROOF_PLAN/DONE_MEANS for this WP and is not a blocker for this diff-scoped PASS.
- UI rendering negative proof: `UI_UX_SPEC` names DCC/Task Board/Role Mailbox controls, but this signed backend range does not add frontend widgets; it exposes the required backend projection fields and preview payload for later UI consumers.
DIFF_ATTACK_SURFACES:
- Producer/consumer drift between `SoftwareDeliveryProjectionSurfaceV1` producers and DCC, Task Board, Role Mailbox consumers.
- Serialized preview payload drift for `GovernedActionPreviewV1` fields before mutation.
- Authority inversion risk where stale board lane/status or mailbox chronology could override canonical runtime fields.
- Closeout spoofing risk via noncanonical validator-gate, owner packet, checkpoint, claim/lease, or queued-instruction refs.
- Current-main containment risk if the feature branch is merged outside the signed range discipline.
INDEPENDENT_CHECKS_RUN:
- `just check-notifications WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` and `just ack-notifications ...` returned no pending notifications before verdict.
- `just phase-check STARTUP WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR integration_validator:wp-1-software-delivery-projection-surface-discipline-v1` passed.
- `just -f ../wt-gov-kernel/justfile artifact-hygiene-check` passed, reporting canonical artifact root `../Handshake_Artifacts` and no reclaimable external dirs.
- `just validator-git-hygiene` passed.
- `git fetch origin main`; `git rev-parse HEAD origin/main FETCH_HEAD` all returned `660a1d5befa8ca083864730f8622e664b9c3eeef`.
- `git -C ../wtc-surface-discipline-v1 rev-parse HEAD` returned `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- `git -C ../wtc-surface-discipline-v1 diff --name-status 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4` showed nine Rust/test files and no `.GOV/**`, `AGENTS.md`, `justfile`, or `.cargo/config.toml`.
- `git -C ../wtc-surface-discipline-v1 diff --check 45afc8867f08f7c2f8edfe71ab750fe92ab28866..eb59e9819c7cc2729c169c723dab3932d5d7b9d4` passed.
- `git merge-tree --write-tree HEAD eb59e9819c7cc2729c169c723dab3932d5d7b9d4` returned clean tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --lib` passed in `../wtc-surface-discipline-v1` with existing warnings.
- Exact tests passed: `projection_surface_previews_governed_action_before_mutation`, `dcc_software_delivery_projection_surface_keeps_runtime_authority`, `task_board_software_delivery_projection_cannot_override_runtime_truth`, `closeout_projection_requires_gate_evidence_and_owner_truth`, `projection_surface_exposes_claim_and_queued_instruction_ids`, `role_mailbox_software_delivery_triage_remains_advisory`, `locus_mt_progress_workflow_parity_with_emitted_packet_and_mailbox_wait`, `locus_sync_task_board_validation_reports_authority_scope_drift`, and `production_clears_workflow_run_lifecycle_record_for_non_software_delivery_summary`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test micro_task_executor_tests --no-run` passed.
- `just phase-check VERDICT WP-1-Software-Delivery-Projection-Surface-Discipline-v1 INTEGRATION_VALIDATOR` passed.
- `just phase-check CLOSEOUT WP-1-Software-Delivery-Projection-Surface-Discipline-v1` passed.
COUNTERFACTUAL_CHECKS:
- If `derive_governed_action_previews` stopped sourcing `canonical.allowed_action_ids`, DCC/Task Board/mailbox consumers could preview actions not authorized by runtime truth.
- If `derive_software_delivery_projection_surface` stopped embedding `governed_action_previews`, the preview payload would exist as an orphan helper and downstream projection surfaces would not see it before mutation.
- If `validate_software_delivery_projection_surface_authority` did not compare canonical state/queue/action/status fields, stale Task Board mirrors could silently override runtime truth.
- If `build_software_delivery_overlay_triage_row` wrote back to runtime records, Role Mailbox chronology could become a hidden authority path.
- If `RuntimeGovernancePaths::is_canonical_validator_gate_ref`, `is_canonical_claim_lease_record_ref`, or `is_canonical_queued_instruction_record_ref` accepted substring matches, spoofed closeout/overlay refs could satisfy authority checks.
BOUNDARY_PROBES:
- Producer/consumer boundary: `src/backend/handshake_core/src/workflows.rs:4682` delegates to the same projection derivation used by DCC, Task Board, and Role Mailbox projection consumers.
- Authority boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3403` builds a tampered projection from board mirror fields and proves validator rejection.
- Profile boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3441`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3674`, and `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6240` prove non-software-delivery summaries/projections do not retain software-delivery surfaces.
- Mailbox boundary: `src/backend/handshake_core/tests/micro_task_executor_tests.rs:6081` snapshots canonical overlay files before mutating the advisory triage row and proves no on-disk authority drift.
- Current-main boundary: merge-tree against current `main` produced tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc` without conflicts.
NEGATIVE_PATH_CHECKS:
- `fabricated_action` returns no preview (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3670`).
- Registered but out-of-family `approve` yields `IneligibleOutOfFamily` for a Validation-family canonical summary (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3649`).
- Non-software-delivery canonical summaries yield no preview and no projection surface (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3674`).
- Stale Task Board authority fields are rejected while advisory mirror/lane/status fields remain non-authoritative (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3751`).
- Non-software-delivery lifecycle emission clears stale workflow-run lifecycle records (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:6240`).
INDEPENDENT_FINDINGS:
- NONE
ANTI_VIBE_FINDINGS:
- NONE
SIGNED_SCOPE_DEBT:
- NONE
PRIMITIVE_RETENTION_PROOF:
- `SoftwareDeliveryProjectionSurfaceV1` remains the shared projection primitive at `src/backend/handshake_core/src/locus/types.rs:1944`.
- `GovernedActionPreviewV1` remains the preview payload primitive at `src/backend/handshake_core/src/locus/types.rs:1895`.
- Closeout posture remains the derived runtime primitive at `src/backend/handshake_core/src/locus/types.rs:2583`.
- Role Mailbox advisory triage remains projection-only at `src/backend/handshake_core/src/role_mailbox.rs:1681`.
PRIMITIVE_RETENTION_GAPS:
- NONE
SHARED_SURFACE_INTERACTION_CHECKS:
- `src/backend/handshake_core/src/locus/types.rs:2259` derives one software-delivery projection from canonical summary plus advisory board/mailbox inputs.
- `src/backend/handshake_core/src/workflows.rs:5101` materializes `projection_surface.json` from canonical summary, claim/lease records, queued-instruction records, workflow lifecycle, and gate posture.
- `src/backend/handshake_core/src/locus/task_board.rs:203` enforces Task Board projection authority against canonical state.
- `src/backend/handshake_core/src/role_mailbox.rs:1707` builds mailbox triage from the already-derived projection surface instead of mailbox chronology.
- `src/backend/handshake_core/src/storage/locus_sqlite.rs:285` applies canonical action/queue overrides to SQLite progress metadata.
CURRENT_MAIN_INTERACTION_CHECKS:
- Current `main`, `origin/main`, and `FETCH_HEAD` all resolved to `660a1d5befa8ca083864730f8622e664b9c3eeef` before verdict.
- `git merge-tree --write-tree HEAD eb59e9819c7cc2729c169c723dab3932d5d7b9d4` produced clean tree `56e52e7058fbf4e3b39df5feb9b4fa9b6e77ebcc`.
- Candidate worktree was clean at `eb59e9819c7cc2729c169c723dab3932d5d7b9d4`.
- `git diff --check` on the signed handoff range passed.
DATA_CONTRACT_PROOF:
- `GovernedActionPreviewV1` serializes `action_request_id`, target record refs, eligibility, blockers, and evidence refs (`src/backend/handshake_core/src/locus/types.rs:1895`) and is embedded into `SoftwareDeliveryProjectionSurfaceV1` (`src/backend/handshake_core/src/locus/types.rs:1997`).
- The exact preview tripwire round-trips the projection payload and proves direct preview derivation is read-only (`src/backend/handshake_core/tests/micro_task_executor_tests.rs:3489`, `src/backend/handshake_core/tests/micro_task_executor_tests.rs:3716`).
- `SoftwareDeliveryProjectionSurfaceV1` carries stable ids for workflow, model session, Task Board, claim/lease, queued-instruction, authority, and evidence refs (`src/backend/handshake_core/src/locus/types.rs:1944`).
DATA_CONTRACT_GAPS:
- NONE
RESIDUAL_UNCERTAINTY:
- Rust warnings remain pre-existing or outside this WP's runtime proof; I did not convert them to hard failures.
- I did not run full `micro_task_executor_tests` runtime because the packet/thread record identifies unrelated pre-existing failures; I ran exact packet tripwires plus `--no-run`.
- I did not exercise a frontend renderer for the UI_UX_SPEC controls; the reviewed candidate exposes backend projection/control fields for those surfaces.
